use std::time::{Duration, SystemTime};

use super::*;
use crate::Options;
use crate::instance::{GameVersion, NewInstance};
use crate::secrets::MemoryStore;
use std::sync::Arc;

fn m(name: &str) -> ModFile {
    ModFile {
        file_name: name.into(),
        enabled: true,
    }
}

fn crash(text: &str) -> DoctorInput {
    DoctorInput {
        crash_text: Some(text.into()),
        ..DoctorInput::default()
    }
}

fn log(text: &str) -> DoctorInput {
    DoctorInput {
        log_tail: Some(text.into()),
        ..DoctorInput::default()
    }
}

fn kinds(d: &[Diagnosis]) -> Vec<&str> {
    d.iter().map(|d| d.kind.as_str()).collect()
}

fn get<'a>(d: &'a [Diagnosis], kind: &str) -> &'a Diagnosis {
    d.iter()
        .find(|x| x.kind == kind)
        .unwrap_or_else(|| panic!("falta {kind}; hay {:?}", kinds(d)))
}

fn param<'a>(d: &'a Diagnosis, key: &str) -> &'a str {
    d.params
        .get(key)
        .unwrap_or_else(|| panic!("falta el parámetro {key}"))
        .as_str()
}

// --- Java y memoria --------------------------------------------------------------

#[test]
fn java_demasiado_antigua() {
    let input = crash(
        "---- Minecraft Crash Report ----\n\
         Exception in thread \"main\" java.lang.UnsupportedClassVersionError: net/fabricmc/loader/impl/launch/knot/KnotClient has been compiled by a more recent version of the Java Runtime (class file version 65.0), this version of the Java Runtime only recognizes class file versions up to 61.0\n",
    );
    let d = analyze(&input);
    let j = get(&d, "java_version_mismatch");
    assert_eq!(j.severity, Severity::Critical);
    assert_eq!(j.category, Category::Java);
    assert_eq!(param(j, "required"), "21");
    assert_eq!(param(j, "current"), "17");
    assert_eq!(j.fix, Some(FixAction::InstallJava { major: 21 }));
    // El "JNI error" típico que acompaña no se confunde con un loader roto.
    assert!(!kinds(&d).contains(&"loader_install_broken"));
}

#[test]
fn java_antigua_formato_major_minor() {
    let d = analyze(&log(
        "Exception in thread \"main\" java.lang.UnsupportedClassVersionError: Unsupported major.minor version 61.0",
    ));
    assert_eq!(param(get(&d, "java_version_mismatch"), "required"), "17");
}

#[test]
fn asm_no_entiende_la_java_no_propone_instalar() {
    let d = analyze(&crash(
        "java.lang.IllegalArgumentException: Unsupported class file major version 65",
    ));
    let j = get(&d, "java_version_mismatch");
    assert_eq!(j.severity, Severity::Warning);
    assert_eq!(j.fix, None);
    assert_eq!(param(j, "class_file_major"), "65");
    let mut input =
        crash("java.lang.IllegalArgumentException: Unsupported class file major version 65");
    input.required_java = Some(17);
    let d = analyze(&input);
    assert_eq!(
        get(&d, "java_version_mismatch").fix,
        Some(FixAction::InstallJava { major: 17 })
    );
}

#[test]
fn sin_memoria_de_heap_sugiere_mas() {
    let mut input = crash(
        "Description: Unexpected error\n\njava.lang.OutOfMemoryError: Java heap space\n\tat a.b.C.d(C.java:1)\n",
    );
    input.memory_mb = 2048;
    input.total_memory_mb = Some(16384);
    let d = analyze(&input);
    let o = get(&d, "out_of_memory");
    assert_eq!(o.category, Category::Memory);
    assert_eq!(param(o, "scope"), "heap");
    assert_eq!(param(o, "suggested"), "3072");
    assert_eq!(o.fix, Some(FixAction::SetMemory { mb: 3072 }));
    assert_eq!(o.confidence, 85);
}

#[test]
fn sin_memoria_no_pasa_del_70_por_ciento_de_la_ram() {
    let mut input = crash("java.lang.OutOfMemoryError: GC overhead limit exceeded");
    input.memory_mb = 6000;
    input.total_memory_mb = Some(8192);
    let d = analyze(&input);
    // 70 % de 8192 = 5734 y ya hay 6000: no hay nada razonable que proponer.
    assert_eq!(get(&d, "out_of_memory").fix, None);
}

#[test]
fn sin_memoria_nativa_sugiere_menos() {
    let mut input = DoctorInput {
        hs_err_text: Some(
            "#\n# There is insufficient memory for the Java Runtime Environment to continue.\n# Native memory allocation (mmap) failed to map 1073741824 bytes for committing reserved memory.\n"
                .into(),
        ),
        ..DoctorInput::default()
    };
    input.memory_mb = 12288;
    input.total_memory_mb = Some(16384);
    let d = analyze(&input);
    let o = get(&d, "out_of_memory");
    assert_eq!(param(o, "scope"), "native");
    assert_eq!(o.fix, Some(FixAction::SetMemory { mb: 9216 }));
    assert!(kinds(&d).iter().filter(|k| **k == "out_of_memory").count() == 1);
}

#[test]
fn no_se_puede_reservar_el_heap() {
    let mut input = log(
        "Error occurred during initialization of VM\nCould not reserve enough space for 8388608KB object heap\n",
    );
    input.memory_mb = 8192;
    let d = analyze(&input);
    let r = get(&d, "reserve_space_failed");
    assert_eq!(param(r, "requested_kb"), "8388608");
    assert_eq!(r.fix, Some(FixAction::SetMemory { mb: 6144 }));
    // Es la causa; no se duplica como argumento de JVM ni como falta de memoria.
    assert_eq!(kinds(&d), vec!["reserve_space_failed"]);
}

#[test]
fn argumento_jvm_desconocido_se_quita() {
    let mut input = log(
        "Unrecognized VM option 'UseConcMarkSweepGC'\nError: Could not create the Java Virtual Machine.\nError: A fatal exception has occurred. Program will exit.\n",
    );
    input.jvm_args = vec!["-Xss1M".into(), "-XX:+UseConcMarkSweepGC".into()];
    let d = analyze(&input);
    assert_eq!(kinds(&d), vec!["bad_jvm_flag"]);
    let f = get(&d, "bad_jvm_flag");
    assert_eq!(f.culprit.as_deref(), Some("-XX:+UseConcMarkSweepGC"));
    assert_eq!(
        f.fix,
        Some(FixAction::RemoveJvmArg {
            arg: "-XX:+UseConcMarkSweepGC".into()
        })
    );
    assert_eq!(param(f, "flag"), "useconcmarksweepgc"); // el texto se compara en minúsculas
}

#[test]
fn argumento_jvm_fuera_de_la_lista_vacia_o_no() {
    let text =
        "Unrecognized VM option 'UseFancyThing'\nError: Could not create the Java Virtual Machine.";
    let mut input = log(text);
    input.jvm_args = vec!["-Xss1M".into()];
    let d = analyze(&input);
    assert_eq!(
        get(&d, "bad_jvm_flag_unknown").fix,
        Some(FixAction::ClearJvmArgs)
    );

    let d = analyze(&log(text));
    let n = get(&d, "bad_jvm_flag_no_custom_args");
    assert_eq!(n.fix, None);
}

#[test]
fn fallo_generico_de_la_jvm() {
    let mut input = log("Error occurred during initialization of VM\nToo small initial heap\n");
    input.jvm_args = vec!["-Xms1k".into()];
    let d = analyze(&input);
    let f = get(&d, "bad_jvm_flag_unknown");
    assert!(f.confidence < 50);
}

// --- mods ---------------------------------------------------------------------------

const MIXIN_CRASH: &str = "---- Minecraft Crash Report ----\n\
Time: 2026-10-11 12:34:56\n\
Description: Initializing game\n\n\
org.spongepowered.asm.mixin.transformer.throwables.MixinTransformerError: An unexpected critical error was encountered\n\
\tat knot//org.spongepowered.asm.mixin.transformer.MixinProcessor.applyMixins(MixinProcessor.java:392)\n\
Caused by: org.spongepowered.asm.mixin.injection.throwables.InvalidInjectionException: @Inject annotation on onRender could not find any targets matching 'render' in net/minecraft/class_757. [sodium.mixins.json:render.GameRendererMixin from mod sodium]\n";

#[test]
fn mixin_con_mod_conocido() {
    let mut input = crash(MIXIN_CRASH);
    input.loader = LoaderKind::Fabric;
    input.mods = vec![
        m("lithium-fabric-0.12.1+mc1.20.4.jar"),
        m("sodium-fabric-0.5.8+mc1.20.4.jar"),
    ];
    let d = analyze(&input);
    let x = get(&d, "mixin_crash_known");
    assert_eq!(
        x.culprit.as_deref(),
        Some("sodium-fabric-0.5.8+mc1.20.4.jar")
    );
    assert_eq!(
        x.fix,
        Some(FixAction::DisableMod {
            file_name: "sodium-fabric-0.5.8+mc1.20.4.jar".into()
        })
    );
    assert_eq!(param(x, "mod_id"), "sodium");
    assert_eq!(x.category, Category::ModConflict);
    assert!(!kinds(&d).contains(&"unknown_crash"));
}

#[test]
fn mixin_ignora_mods_desactivados() {
    let mut input = crash(MIXIN_CRASH);
    input.mods = vec![ModFile {
        file_name: "sodium-fabric-0.5.8.jar.disabled".into(),
        enabled: false,
    }];
    let d = analyze(&input);
    let x = get(&d, "mixin_crash_unknown");
    assert_eq!(x.fix, None);
    assert_eq!(x.culprit.as_deref(), Some("sodium"));
}

#[test]
fn mixin_por_nombre_del_json() {
    let mut input = crash(
        "Description: Rendering screen\n\nCaused by: org.spongepowered.asm.mixin.throwables.MixinApplyError: Mixin [mixins.betterfoliage.json:client.MixinBlock] from phase [DEFAULT] in config [mixins.betterfoliage.json] FAILED during APPLY\n",
    );
    input.mods = vec![m("BetterFoliage-5.0.2-Fabric.jar")];
    let d = analyze(&input);
    assert_eq!(
        get(&d, "mixin_crash_known").culprit.as_deref(),
        Some("BetterFoliage-5.0.2-Fabric.jar")
    );
}

#[test]
fn dependencia_que_falta_en_fabric() {
    let mut input = crash(
        "net.fabricmc.loader.impl.FormattedException: Incompatible mod set!\n\
         Some of your mods are incompatible with the game or each other!\n\
         A potential solution has been determined:\n\
         \t - Install fabric-api, version 0.91.0 or later.\n\
         Unmet dependency listing:\n\
         \t - Mod 'Mod Menu' (modmenu) 9.0.0 requires version 0.91.0 or later of fabric-api, which is missing!\n",
    );
    input.loader = LoaderKind::Fabric;
    let d = analyze(&input);
    let x = get(&d, "missing_dependency");
    assert_eq!(x.category, Category::MissingDependency);
    assert_eq!(x.culprit.as_deref(), Some("fabric-api"));
    assert_eq!(param(x, "required_by"), "modmenu");
    assert!(x.confidence >= 85);
    // La cabecera genérica no duplica el diagnóstico.
    assert!(!kinds(&d).contains(&"incompatible_mod_set_fabric"));
}

#[test]
fn dependencia_con_version_incorrecta_en_fabric() {
    let d = analyze(&crash(
        "Incompatible mod set!\n - Mod 'Sodium Extra' (sodium-extra) 0.5.4 requires version 0.5.8 or later of mod 'Sodium' (sodium), but only the wrong version is present: 0.5.3!\n",
    ));
    let x = get(&d, "dependency_version_mismatch");
    assert_eq!(x.culprit.as_deref(), Some("sodium"));
    assert_eq!(param(x, "required_by"), "sodium-extra");
}

#[test]
fn dependencia_que_falta_en_forge() {
    let d = analyze(&crash(
        "net.minecraftforge.fml.ModLoadingException: Mod loading has failed\n\
         Missing or unsupported mandatory dependencies:\n\
         \tMod ID: 'geckolib', Requested by: 'alexsmobs', Expected range: '[4.4,)', Actual version: '[MISSING]'\n\
         \tMod ID: 'citadel', Requested by: 'alexsmobs', Expected range: '[2.5,)', Actual version: '[MISSING]'\n",
    ));
    let x = get(&d, "missing_dependency");
    assert_eq!(x.culprit.as_deref(), Some("geckolib"));
    assert_eq!(param(x, "dependencies"), "geckolib, citadel");
    assert_eq!(param(x, "required_by"), "alexsmobs");
    assert!(!kinds(&d).contains(&"forge_mod_resolution_failed"));
}

#[test]
fn version_incorrecta_en_forge() {
    let d = analyze(&crash(
        "Unsupported installed mandatory dependencies:\n\tMod ID: 'geckolib', Requested by: 'alexsmobs', Expected range: '[4.4,)', Actual version: '3.1.0'\n",
    ));
    let x = get(&d, "forge_dependency_version_mismatch");
    assert_eq!(param(x, "actual"), "3.1.0");
    assert_eq!(param(x, "expected"), "[4.4,)");
    assert!(!kinds(&d).contains(&"missing_dependency"));
}

#[test]
fn forge_rechaza_mods_sin_mas_detalle() {
    let d = analyze(&crash(
        "net.minecraftforge.fml.ModLoadingException: Mod loading has failed\n",
    ));
    assert_eq!(kinds(&d), vec!["forge_mod_resolution_failed"]);
}

#[test]
fn conjunto_incompatible_sin_mas_detalle() {
    let d = analyze(&crash("Incompatible mod set!\nsomething else\n"));
    assert_eq!(kinds(&d), vec!["incompatible_mod_set_fabric"]);
}

#[test]
fn mods_duplicados() {
    let mut input = crash(
        "net.minecraftforge.fml.loading.EarlyLoadingException: Found a duplicate mod jei at [/home/u/instances/x/game/mods/jei-1.20.1-forge-15.3.0.4.jar, /home/u/instances/x/game/mods/jei-1.20.1-forge-15.2.0.27.jar]\n",
    );
    input.mods = vec![
        m("jei-1.20.1-forge-15.2.0.27.jar"),
        m("jei-1.20.1-forge-15.3.0.4.jar"),
        m("other.jar"),
    ];
    let d = analyze(&input);
    let x = get(&d, "duplicate_mods");
    assert_eq!(
        x.fix,
        Some(FixAction::DisableMod {
            file_name: "jei-1.20.1-forge-15.2.0.27.jar".into()
        })
    );
}

#[test]
fn mods_duplicados_por_id_de_fabric() {
    let mut input = crash("Found duplicate mods:\n\tMod ID: 'lithium' ...\n");
    input.mods = vec![m("lithium-fabric-0.11.jar"), m("lithium-fabric-0.12.jar")];
    let d = analyze(&input);
    let x = get(&d, "duplicate_mods");
    assert_eq!(x.culprit.as_deref(), Some("lithium"));
    assert_eq!(
        x.fix,
        Some(FixAction::DisableMod {
            file_name: "lithium-fabric-0.11.jar".into()
        })
    );
}

#[test]
fn archivo_zip_corrupto_con_mod() {
    let mut input = crash(
        "Caused by: java.util.zip.ZipException: error in opening zip file /home/u/instances/x/game/mods/BrokenMod-1.2.jar\n\tat java.base/java.util.zip.ZipFile$Source.<init>\n",
    );
    input.mods = vec![m("BrokenMod-1.2.jar"), m("ok.jar")];
    let d = analyze(&input);
    let x = get(&d, "corrupted_archive");
    assert_eq!(x.culprit.as_deref(), Some("BrokenMod-1.2.jar"));
    assert_eq!(
        x.fix,
        Some(FixAction::DisableMod {
            file_name: "BrokenMod-1.2.jar".into()
        })
    );
}

#[test]
fn archivo_zip_corrupto_sin_mod_en_la_lista() {
    let d = analyze(&crash(
        "java.util.zip.ZipException: zip END header not found\n",
    ));
    let x = get(&d, "corrupted_archive");
    assert_eq!(x.fix, None);
    assert_eq!(x.category, Category::CorruptedFile);
}

#[test]
fn options_txt_ilegible() {
    let d = analyze(&log(
        "[12:00:01] [Render thread/ERROR]: Failed to load options.txt: bad nbt\n",
    ));
    let x = get(&d, "corrupted_options");
    assert_eq!(
        x.fix,
        Some(FixAction::DeleteInstanceFile {
            relative_path: "options.txt".into()
        })
    );
    // Una línea normal que nombra el archivo no dispara nada.
    assert!(analyze(&log("[12:00:01] [main/INFO]: Reading options.txt\n")).is_empty());
}

#[test]
fn servers_dat_ilegible() {
    let d = analyze(&log(
        "[12:00:01] [Render thread/ERROR]: Couldn't load server list\njava.io.EOFException: servers.dat\n",
    ));
    assert_eq!(
        get(&d, "corrupted_servers_dat").fix,
        Some(FixAction::DeleteInstanceFile {
            relative_path: "servers.dat".into()
        })
    );
}

#[test]
fn bloqueo_de_sesion() {
    let d = analyze(&crash(
        "java.io.IOException: Failed to check session lock, aborting\n",
    ));
    let x = get(&d, "session_lock_corrupted");
    assert_eq!(x.fix, Some(FixAction::ClearWorldLocks));
    assert_eq!(x.category, Category::WorldCorruption);
}

#[test]
fn entidad_que_rompe_el_tick() {
    let mut input = crash(
        "---- Minecraft Crash Report ----\nDescription: Ticking entity\n\njava.lang.NullPointerException: Ticking entity\n\n-- Entity being ticked --\nDetails:\n\tEntity Type: alexsmobs:crow (com.github.alexthe666.alexsmobs.entity.EntityCrow)\n\tEntity ID: 1234\n",
    );
    input.mods = vec![m("alexsmobs-1.22.9.jar")];
    let d = analyze(&input);
    let x = get(&d, "world_tick_crash_known");
    assert_eq!(x.culprit.as_deref(), Some("alexsmobs:crow"));
    assert_eq!(param(x, "namespace"), "alexsmobs");
    assert_eq!(param(x, "mod_file"), "alexsmobs-1.22.9.jar");
    assert_eq!(x.fix, None);
}

#[test]
fn error_al_actualizar_el_mundo_sin_entidad() {
    let d = analyze(&crash(
        "Description: Exception ticking world\n\njava.lang.IllegalStateException: x\n",
    ));
    assert_eq!(get(&d, "world_tick_crash_unknown").fix, None);
}

#[test]
fn verify_error_con_mod_cerca() {
    let mut input = crash(
        "Caused by: java.lang.VerifyError: Bad type on operand stack\n\tat knot//com.example.Thing (badmod-2.0.jar)\n",
    );
    input.mods = vec![m("badmod-2.0.jar")];
    let d = analyze(&input);
    assert_eq!(
        get(&d, "verify_error").culprit.as_deref(),
        Some("badmod-2.0.jar")
    );
}

#[test]
fn optifine_con_nosuchmethoderror() {
    let mut input = crash(
        "Description: Rendering overlay\n\njava.lang.NoSuchMethodError: 'void net.optifine.Config.checkGlError(java.lang.String)'\n",
    );
    input.mods = vec![m("OptiFine_1.20.1_HD_U_I6.jar"), m("OptiFabric-1.14.jar")];
    let d = analyze(&input);
    let x = get(&d, "optifine_incompatibility");
    assert_eq!(
        x.fix,
        Some(FixAction::DisableMod {
            file_name: "OptiFine_1.20.1_HD_U_I6.jar".into()
        })
    );
    // Sin mención a OptiFine, el mismo error no se le atribuye.
    let d = analyze(&crash("java.lang.NoSuchMethodError: 'void a.B.c()'\n"));
    assert!(!kinds(&d).contains(&"optifine_incompatibility"));
}

// --- loader ---------------------------------------------------------------------------

#[test]
fn loader_roto_se_reinstala() {
    let mut input = log(
        "Exception in thread \"main\" java.lang.NoClassDefFoundError: net/fabricmc/loader/impl/launch/knot/KnotClient\n",
    );
    input.loader = LoaderKind::Fabric;
    let d = analyze(&input);
    assert_eq!(
        get(&d, "loader_install_broken").fix,
        Some(FixAction::ReinstallLoader)
    );

    input.loader = LoaderKind::Vanilla;
    let d = analyze(&input);
    assert_eq!(get(&d, "loader_install_broken").fix, None);
}

#[test]
fn mod_de_otro_loader() {
    let mut input = crash("java.lang.NoClassDefFoundError: net/minecraftforge/fml/common/Mod\n");
    input.loader = LoaderKind::Fabric;
    input.mods = vec![
        m("create-forge-0.5.1.jar"),
        m("fabric-api-0.91.0.jar"),
        m("ForgeConfigAPIPort-v8.0.0-1.20.1-Fabric.jar"),
        m("neoforge-only-mod.jar"),
    ];
    let d = analyze(&input);
    let culprits: Vec<_> = d
        .iter()
        .filter(|x| x.kind == "wrong_loader_mod")
        .filter_map(|x| x.culprit.as_deref())
        .collect();
    assert_eq!(culprits.len(), 2);
    assert!(culprits.contains(&"create-forge-0.5.1.jar"));
    assert!(culprits.contains(&"neoforge-only-mod.jar"));
    // Con la causa conocida no se acusa a la instalación del loader.
    assert!(!kinds(&d).contains(&"loader_install_broken"));
    // Ids únicos para poder arreglarlos por separado.
    let ids: Vec<_> = d
        .iter()
        .filter(|x| x.kind == "wrong_loader_mod")
        .map(|x| x.id.as_str())
        .collect();
    assert_eq!(ids, vec!["wrong_loader_mod", "wrong_loader_mod-2"]);
}

#[test]
fn mod_de_fabric_en_forge_salvo_con_connector() {
    let mut input = crash("java.lang.NoClassDefFoundError: net/fabricmc/api/ModInitializer\n");
    input.loader = LoaderKind::Forge;
    input.mods = vec![m("fabric-api-0.91.0.jar")];
    assert!(kinds(&analyze(&input)).contains(&"wrong_loader_mod"));
    input.mods.push(m("connector-1.0.0-beta.jar"));
    assert!(!kinds(&analyze(&input)).contains(&"wrong_loader_mod"));
}

#[test]
fn mod_de_otro_loader_exige_pruebas_y_crash() {
    let mut input = DoctorInput {
        loader: LoaderKind::Fabric,
        mods: vec![m("create-forge-0.5.1.jar")],
        ..DoctorInput::default()
    };
    assert!(analyze(&input).is_empty());
    input.crash_text = Some("Description: Rendering\n\njava.lang.RuntimeException: boom\n".into());
    assert!(!kinds(&analyze(&input)).contains(&"wrong_loader_mod"));
}

#[test]
fn pares_incompatibles() {
    let input = DoctorInput {
        mods: vec![
            m("OptiFine_1.20.1_HD_U_I6.jar"),
            m("sodium-fabric-0.5.8.jar"),
            m("iris-mc1.20.1-1.6.11.jar"),
            m("phosphor-fabric-mc1.19.4-0.8.1.jar"),
            m("starlight-1.1.2+fabric.jar"),
            m("FabricProxy-Lite-2.8.0.jar"),
            m("FabricProxy-1.0.jar"),
        ],
        ..DoctorInput::default()
    };
    let d = analyze(&input);
    assert_eq!(
        kinds(&d)
            .into_iter()
            .filter(|k| k.starts_with("incompatible_pair_"))
            .count(),
        4
    );
    let x = get(&d, "incompatible_pair_optifine_sodium");
    assert_eq!(x.severity, Severity::Warning);
    assert_eq!(
        x.fix,
        Some(FixAction::DisableMod {
            file_name: "OptiFine_1.20.1_HD_U_I6.jar".into()
        })
    );
    assert_eq!(
        get(&d, "incompatible_pair_phosphor_starlight").fix,
        Some(FixAction::DisableMod {
            file_name: "phosphor-fabric-mc1.19.4-0.8.1.jar".into()
        })
    );
    assert_eq!(
        get(&d, "incompatible_pair_fabricproxy_fabricproxy-lite").fix,
        Some(FixAction::DisableMod {
            file_name: "FabricProxy-1.0.jar".into()
        })
    );
    // No hay crash: no se inventa un `unknown_crash`.
    assert!(!kinds(&d).contains(&"unknown_crash"));
}

#[test]
fn pares_ignoran_desactivados_y_aislados() {
    let input = DoctorInput {
        mods: vec![
            m("OptiFine_1.20.1_HD_U_I6.jar"),
            ModFile {
                file_name: "sodium-fabric-0.5.8.jar.disabled".into(),
                enabled: false,
            },
            m("sodium-extra-fabric-0.5.4.jar"),
            m("iris-flywheel-compat-1.0.jar"),
        ],
        ..DoctorInput::default()
    };
    let d = analyze(&input);
    // Sodium Extra también necesita Sodium y choca con OptiFine, pero los demás no cuentan.
    assert_eq!(kinds(&d), vec!["incompatible_pair_optifine_sodium"]);
    assert!(get(&d, "incompatible_pair_optifine_sodium").params["mods"].contains("sodium-extra"));
}

// --- gráficos y sistema --------------------------------------------------------------------

fn hs_err(frame: &str) -> DoctorInput {
    DoctorInput {
        hs_err_text: Some(format!(
            "#\n# A fatal error has been detected by the Java Runtime Environment:\n#\n#  SIGSEGV (0xb) at pc=0x00007f1a2b3c4d5e, pid=4321, tid=4399\n#\n# JRE version: OpenJDK Runtime Environment (21.0.4+7) (build 21.0.4+7)\n# Problematic frame:\n# {frame}\n#\n"
        )),
        ..DoctorInput::default()
    }
}

#[test]
fn driver_grafico_por_fabricante() {
    for (frame, gpu) in [
        ("C  [nvoglv64.dll+0x1a2b3c]", "NVIDIA"),
        ("C  [libnvidia-glcore.so.535.129.03+0x1d4a3b]", "NVIDIA"),
        ("C  [atio6axx.dll+0x7f3a21]", "AMD"),
        ("C  [radeonsi_dri.so+0x9a1b2c]", "AMD"),
        ("C  [ig9icd64.dll+0x44aa10]", "Intel"),
        ("C  [iris_dri.so+0x5b6c7d]", "Intel"),
    ] {
        let d = analyze(&hs_err(frame));
        let x = get(&d, "native_driver_crash");
        assert_eq!(param(x, "gpu"), gpu, "{frame}");
        assert_eq!(x.category, Category::GraphicsDriver);
        assert!(!kinds(&d).contains(&"unknown_crash"), "{frame}");
    }
    // Biblioteca de gráficos sin fabricante conocido.
    let d = analyze(&hs_err("C  [liblwjgl_opengl.so+0x1234]"));
    assert!(!get(&d, "native_driver_crash").params.contains_key("gpu"));
}

#[test]
fn fallo_nativo_que_no_es_del_driver_queda_como_desconocido() {
    let input = hs_err("V  [libjvm.so+0x8c1f3a]  Symbol::equals(Symbol*) const+0x1a");
    let d = analyze(&input);
    assert_eq!(kinds(&d), vec!["unknown_crash"]);
    let excerpt = crash_excerpt(&input).unwrap();
    assert!(excerpt.starts_with("Problematic frame:"), "{excerpt}");
    assert!(excerpt.contains("libjvm.so"));
    // Un frame de Java tampoco es del driver.
    assert!(
        !kinds(&analyze(&hs_err(
            "J 1234 c2 net.minecraft.Foo.bar()V (12 bytes) @ 0x00007f"
        )))
        .contains(&"native_driver_crash")
    );
}

#[test]
fn firmas_de_una_cadena() {
    for (text, kind, severity) in [
        (
            "GLFW error 65543: WGL: The driver does not appear to support OpenGL.",
            "vanilla_opengl_not_supported",
            Severity::Critical,
        ),
        (
            "java.lang.IllegalStateException: Your graphics card is not compatible with minimum OpenGL version 3.2",
            "vanilla_opengl_not_supported",
            Severity::Critical,
        ),
        (
            "org.lwjgl.LWJGLException: Pixel format not accelerated",
            "pixel_format_not_accelerated",
            Severity::Critical,
        ),
        (
            "java.net.BindException: Address already in use: bind",
            "address_already_in_use",
            Severity::Warning,
        ),
        (
            "java.io.IOException: Too many open files",
            "too_many_open_files",
            Severity::Warning,
        ),
        (
            "java.io.IOException: No space left on device",
            "disk_full",
            Severity::Critical,
        ),
        (
            "java.nio.file.AccessDeniedException: /home/u/instances/x/game/saves/w/level.dat",
            "access_denied_writing",
            Severity::Warning,
        ),
        (
            "java.io.IOException: Access is denied",
            "access_denied_writing",
            Severity::Warning,
        ),
    ] {
        let d = analyze(&crash(text));
        let x = get(&d, kind);
        assert_eq!(x.severity, severity, "{kind}");
        assert_eq!(x.fix, None);
        assert!(!kinds(&d).contains(&"unknown_crash"), "{kind}");
    }
}

#[test]
fn coincidencias_sin_distinguir_mayusculas() {
    let d = analyze(&log("JAVA.NET.BINDEXCEPTION: ADDRESS ALREADY IN USE"));
    assert!(kinds(&d).contains(&"address_already_in_use"));
}

// --- memoria sin crash ---------------------------------------------------------------------

#[test]
fn memoria_casi_igual_a_la_ram() {
    let input = DoctorInput {
        memory_mb: 7800,
        total_memory_mb: Some(8192),
        ..DoctorInput::default()
    };
    let d = analyze(&input);
    let x = get(&d, "memory_exceeds_ram");
    assert_eq!(x.severity, Severity::Warning);
    assert_eq!(x.fix, Some(FixAction::SetMemory { mb: 4096 }));
    let ok = DoctorInput {
        memory_mb: 4096,
        total_memory_mb: Some(8192),
        ..DoctorInput::default()
    };
    assert!(analyze(&ok).is_empty());
    let sin_dato = DoctorInput {
        memory_mb: 65536,
        total_memory_mb: None,
        ..DoctorInput::default()
    };
    assert!(analyze(&sin_dato).is_empty());
}

#[test]
fn poca_memoria_para_muchos_mods() {
    let mods: Vec<_> = (0..41).map(|i| m(&format!("mod-{i}.jar"))).collect();
    let input = DoctorInput {
        memory_mb: 1024,
        total_memory_mb: Some(16384),
        mods: mods.clone(),
        ..DoctorInput::default()
    };
    let d = analyze(&input);
    let x = get(&d, "low_memory_for_mods");
    assert_eq!(x.severity, Severity::Info);
    assert_eq!(x.fix, Some(FixAction::SetMemory { mb: 4096 }));
    let pocos = DoctorInput {
        memory_mb: 1024,
        mods: mods[..40].to_vec(),
        ..DoctorInput::default()
    };
    assert!(analyze(&pocos).is_empty());
    let suficiente = DoctorInput {
        memory_mb: 2048,
        mods,
        ..DoctorInput::default()
    };
    assert!(analyze(&suficiente).is_empty());
}

// --- crash desconocido, orden y confianza ---------------------------------------------------

#[test]
fn crash_desconocido_con_extracto() {
    let input = crash(
        "---- Minecraft Crash Report ----\nTime: x\nDescription: Rendering screen\n\njava.lang.NullPointerException: Rendering screen\n",
    );
    let d = analyze(&input);
    assert_eq!(kinds(&d), vec!["unknown_crash"]);
    assert_eq!(param(&d[0], "excerpt"), "Description: Rendering screen");
    assert_eq!(d[0].category, Category::Unknown);
    assert_eq!(
        crash_excerpt(&input).as_deref(),
        Some("Description: Rendering screen")
    );
}

#[test]
fn extracto_prefiere_description_y_cae_en_caused_by() {
    let input = crash(
        "java.lang.RuntimeException: x\n\tat a.b\nCaused by: java.lang.IllegalStateException: y\n",
    );
    assert_eq!(
        crash_excerpt(&input).as_deref(),
        Some("Caused by: java.lang.IllegalStateException: y")
    );
    assert_eq!(crash_excerpt(&DoctorInput::default()), None);
    // Sin cierre anómalo no hay extracto aunque el log tenga un `Caused by:`.
    assert_eq!(
        crash_excerpt(&log("Caused by: java.io.IOException: x")),
        None
    );
}

#[test]
fn crash_desconocido_con_aviso_estatico_sigue_apareciendo() {
    let mut input = crash("Description: Rendering screen\n");
    input.mods = vec![
        m("OptiFine_1.20.1_HD_U_I6.jar"),
        m("sodium-fabric-0.5.8.jar"),
    ];
    let d = analyze(&input);
    assert!(kinds(&d).contains(&"unknown_crash"));
    assert!(kinds(&d).contains(&"incompatible_pair_optifine_sodium"));
}

#[test]
fn sin_datos_no_hay_diagnosticos() {
    assert!(analyze(&DoctorInput::default()).is_empty());
    assert!(analyze(&log("[12:00:00] [main/INFO]: Setting user: Steve\n[12:00:03] [Render thread/INFO]: Sound engine started\n")).is_empty());
}

#[test]
fn orden_por_gravedad_y_confianza() {
    let mut input = crash(
        "java.io.IOException: Too many open files\njava.io.IOException: No space left on device\n",
    );
    input.memory_mb = 7900;
    input.total_memory_mb = Some(8192);
    input.mods = (0..45).map(|i| m(&format!("m{i}.jar"))).collect();
    let d = analyze(&input);
    let sev: Vec<_> = d.iter().map(|x| x.severity).collect();
    let mut sorted = sev.clone();
    sorted.sort();
    assert_eq!(sev, sorted);
    assert_eq!(d[0].kind, "disk_full");
    assert_eq!(d.last().unwrap().severity, Severity::Warning);
}

#[test]
fn solo_en_el_log_baja_la_confianza() {
    let text = "java.io.IOException: No space left on device";
    let en_crash = analyze(&crash(text));
    let en_log = analyze(&log(text));
    assert_eq!(get(&en_crash, "disk_full").confidence, 90);
    assert_eq!(get(&en_log, "disk_full").confidence, 75);
}

#[test]
fn fix_action_serializa_con_etiqueta() {
    let f = FixAction::DisableMod {
        file_name: "a.jar".into(),
    };
    assert_eq!(
        serde_json::to_string(&f).unwrap(),
        r#"{"type":"disable_mod","file_name":"a.jar"}"#
    );
    assert_eq!(
        serde_json::to_string(&FixAction::ClearJvmArgs).unwrap(),
        r#"{"type":"clear_jvm_args"}"#
    );
    let back: FixAction = serde_json::from_str(r#"{"type":"set_memory","mb":3072}"#).unwrap();
    assert_eq!(back, FixAction::SetMemory { mb: 3072 });
    let json = serde_json::to_value(Severity::Critical).unwrap();
    assert_eq!(json, "critical");
    assert_eq!(serde_json::to_value(Category::JvmArgs).unwrap(), "jvm_args");
}

// --- diagnose + apply_fix sobre disco -----------------------------------------------------------

async fn core(dir: &std::path::Path) -> Core {
    Core::open(Options {
        data_dir: Some(dir.to_path_buf()),
        secrets: Some(Arc::new(MemoryStore::default())),
        offline: true,
        ..Options::default()
    })
    .await
    .unwrap()
}

fn fabric() -> GameVersion {
    GameVersion {
        mc: "1.20.4".into(),
        loader: LoaderKind::Fabric,
        loader_version: Some("0.15.7".into()),
    }
}

fn write(path: &std::path::Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn set_age(path: &std::path::Path, secs_ago: u64) {
    let f = std::fs::File::options().write(true).open(path).unwrap();
    f.set_modified(SystemTime::now() - Duration::from_secs(secs_ago))
        .unwrap();
}

#[tokio::test]
async fn diagnostica_una_instancia_vacia() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let inst = core
        .create_instance(NewInstance {
            name: "Limpia".into(),
            game: fabric(),
        })
        .await
        .unwrap();
    let report = core.diagnose(&inst.id).await.unwrap();
    assert!(report.crash_report_path.is_none());
    assert!(report.crash_excerpt.is_none());
    assert!(
        report
            .diagnoses
            .iter()
            .all(|d| d.severity == Severity::Info || d.kind == "memory_exceeds_ram")
    );
    assert!(core.diagnose("no-existe").await.is_err());
}

#[tokio::test]
async fn diagnostica_y_arregla() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let inst = core
        .create_instance(NewInstance {
            name: "Mundo roto".into(),
            game: fabric(),
        })
        .await
        .unwrap();
    let game = core.instance_game_dir(&inst.id).unwrap();
    let instance_dir = core.instance_dir(&inst.id).unwrap();

    // Dos crash reports: se analiza el más reciente.
    let old = game.join("crash-reports/crash-2026-10-01_10.00.00-client.txt");
    let new = game.join("crash-reports/crash-2026-10-11_12.34.56-client.txt");
    write(
        &old,
        "Description: Ticking entity\n\tEntity Type: minecraft:zombie (x)\n",
    );
    write(&new, MIXIN_CRASH);
    set_age(&old, 86_400);
    set_age(&new, 60);
    // hs_err antiguo e irrelevante.
    write(&game.join("hs_err_pid111.log"), "# nothing relevant\n");
    // latest.log de la instancia manda sobre el del juego.
    write(
        &instance_dir.join("logs/latest.log"),
        "[12:34:50] [main/INFO]: Loading Minecraft 1.20.4\n[12:34:56] [Render thread/ERROR]: Failed to load options.txt\n",
    );
    write(
        &game.join("logs/latest.log"),
        "java.io.IOException: No space left on device\n",
    );
    write(&game.join("options.txt"), "garbage");
    write(&game.join("mods/sodium-fabric-0.5.8.jar"), "x");
    write(&game.join("mods/lithium-fabric-0.12.1.jar.disabled"), "x");
    write(&game.join("mods/readme.txt"), "x");

    let report = core.diagnose(&inst.id).await.unwrap();
    assert_eq!(report.crash_report_path.as_deref(), Some(new.as_path()));
    assert_eq!(
        report.crash_excerpt.as_deref(),
        Some("Description: Initializing game")
    );
    let k = kinds(&report.diagnoses);
    assert!(k.contains(&"mixin_crash_known"), "{k:?}");
    assert!(k.contains(&"corrupted_options"), "{k:?}");
    assert!(!k.contains(&"disk_full"), "{k:?}");
    assert!(!report.healthy);
    assert_eq!(report.diagnoses[0].severity, Severity::Critical);

    // Mover el mod culpable.
    let mixin = get(&report.diagnoses, "mixin_crash_known").clone();
    let msg = core
        .apply_fix(&inst.id, mixin.fix.as_ref().unwrap())
        .await
        .unwrap();
    assert!(msg.contains("sodium-fabric-0.5.8.jar"));
    assert!(!game.join("mods/sodium-fabric-0.5.8.jar").exists());
    assert!(game.join("mods/sodium-fabric-0.5.8.jar.disabled").exists());

    // Borrar options.txt.
    let opts = get(&report.diagnoses, "corrupted_options").clone();
    core.apply_fix(&inst.id, opts.fix.as_ref().unwrap())
        .await
        .unwrap();
    assert!(!game.join("options.txt").exists());
    assert!(
        core.apply_fix(&inst.id, opts.fix.as_ref().unwrap())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn lee_solo_la_cola_del_log_y_cae_al_log_del_juego() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let inst = core
        .create_instance(NewInstance {
            name: "Log grande".into(),
            game: fabric(),
        })
        .await
        .unwrap();
    let game = core.instance_game_dir(&inst.id).unwrap();

    let mut text = String::from("java.lang.OutOfMemoryError: Java heap space\n");
    text.push_str(&"[12:00:00] [main/INFO]: linea de relleno\n".repeat(14_000)); // > 400 KB
    text.push_str("java.io.IOException: No space left on device\n");
    assert!(text.len() as u64 > LOG_TAIL_BYTES);
    write(&game.join("logs/latest.log"), &text);

    let report = core.diagnose(&inst.id).await.unwrap();
    let k = kinds(&report.diagnoses);
    assert!(k.contains(&"disk_full"), "{k:?}");
    assert!(!k.contains(&"out_of_memory"), "{k:?}");
}

#[tokio::test]
async fn hs_err_mas_reciente_y_modo_de_arranque_fallido() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let inst = core
        .create_instance(NewInstance {
            name: "Driver".into(),
            game: fabric(),
        })
        .await
        .unwrap();
    let game = core.instance_game_dir(&inst.id).unwrap();
    let frame = |lib: &str| {
        format!(
            "# A fatal error has been detected by the Java Runtime Environment:\n# Problematic frame:\n# C  [{lib}+0x10]\n"
        )
    };
    write(&game.join("hs_err_pid100.log"), &frame("libjvm.so"));
    write(
        &game.join("hs_err_pid200.log"),
        &frame("libnvidia-glcore.so.535"),
    );
    set_age(&game.join("hs_err_pid100.log"), 3600);
    let report = core.diagnose(&inst.id).await.unwrap();
    assert_eq!(
        param(get(&report.diagnoses, "native_driver_crash"), "gpu"),
        "NVIDIA"
    );
}

#[tokio::test]
async fn arreglos_de_configuracion_de_la_instancia() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let inst = core
        .create_instance(NewInstance {
            name: "Ajustes".into(),
            game: fabric(),
        })
        .await
        .unwrap();

    let msg = core
        .apply_fix(&inst.id, &FixAction::SetMemory { mb: 6144 })
        .await
        .unwrap();
    assert!(msg.contains("6144"));
    assert_eq!(
        core.get_instance(&inst.id).await.unwrap().java.memory_mb,
        Some(6144)
    );
    assert!(
        core.apply_fix(&inst.id, &FixAction::SetMemory { mb: 100 })
            .await
            .is_err()
    );

    // Los argumentos que venían de los ajustes globales pasan a ser propios de la instancia.
    core.update_settings(|s| s.default_jvm_args = vec!["-Xss1M".into(), "-XX:+Bad".into()])
        .await
        .unwrap();
    core.apply_fix(
        &inst.id,
        &FixAction::RemoveJvmArg {
            arg: "-XX:+Bad".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        core.get_instance(&inst.id).await.unwrap().java.jvm_args,
        Some(vec!["-Xss1M".into()])
    );
    let err = core
        .apply_fix(
            &inst.id,
            &FixAction::RemoveJvmArg {
                arg: "-XX:+Bad".into(),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "invalid_input");

    core.apply_fix(&inst.id, &FixAction::ClearJvmArgs)
        .await
        .unwrap();
    assert_eq!(
        core.get_instance(&inst.id).await.unwrap().java.jvm_args,
        Some(vec![])
    );

    // La memoria sale de la propia instancia en el diagnóstico.
    let input_mem = core.diagnose(&inst.id).await.unwrap();
    assert!(input_mem.diagnoses.iter().all(|d| d.kind != "bad_jvm_flag"));
}

#[tokio::test]
async fn desactivar_mod_valida_el_nombre() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let inst = core
        .create_instance(NewInstance {
            name: "Mods".into(),
            game: fabric(),
        })
        .await
        .unwrap();
    let game = core.instance_game_dir(&inst.id).unwrap();
    write(&game.join("mods/a.jar"), "x");
    write(&dir.path().join("fuera.jar"), "x");

    for bad in [
        "../../../../fuera.jar",
        "sub/a.jar",
        "..",
        "",
        "/etc/passwd",
    ] {
        let r = core
            .apply_fix(
                &inst.id,
                &FixAction::DisableMod {
                    file_name: bad.into(),
                },
            )
            .await;
        assert_eq!(r.unwrap_err().kind(), "invalid_path", "{bad:?}");
    }
    assert!(dir.path().join("fuera.jar").exists());
    let r = core
        .apply_fix(
            &inst.id,
            &FixAction::DisableMod {
                file_name: "a.jar.disabled".into(),
            },
        )
        .await;
    assert!(r.is_err());
    let r = core
        .apply_fix(
            &inst.id,
            &FixAction::DisableMod {
                file_name: "no-esta.jar".into(),
            },
        )
        .await;
    assert!(r.is_err());
    core.apply_fix(
        &inst.id,
        &FixAction::DisableMod {
            file_name: "a.jar".into(),
        },
    )
    .await
    .unwrap();
    assert!(game.join("mods/a.jar.disabled").exists());
    // Ya desactivado: no se puede repetir.
    assert!(
        core.apply_fix(
            &inst.id,
            &FixAction::DisableMod {
                file_name: "a.jar".into()
            }
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn borrar_archivos_solo_los_permitidos() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let inst = core
        .create_instance(NewInstance {
            name: "Borrar".into(),
            game: fabric(),
        })
        .await
        .unwrap();
    let game = core.instance_game_dir(&inst.id).unwrap();
    for f in [
        "options.txt",
        "servers.dat",
        "optionsof.txt",
        "optionsshaders.txt",
        "config/sodium-options.json",
        "config/foo/bar.toml",
        "saves/w/level.dat",
        "mods/a.jar",
        "launcher_profiles.json",
    ] {
        write(&game.join(f), "x");
    }
    for ok in [
        "options.txt",
        "servers.dat",
        "optionsof.txt",
        "optionsshaders.txt",
        "config/sodium-options.json",
        "./config/foo/bar.toml",
    ] {
        core.apply_fix(
            &inst.id,
            &FixAction::DeleteInstanceFile {
                relative_path: ok.into(),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{ok}: {e}"));
    }
    assert!(!game.join("options.txt").exists());
    assert!(!game.join("config/foo/bar.toml").exists());
    for bad in [
        "saves/w/level.dat",
        "mods/a.jar",
        "launcher_profiles.json",
        "../instance.json",
        "config/../options.txt",
        "/etc/passwd",
        "config",
        "",
        "config\\x",
    ] {
        let r = core
            .apply_fix(
                &inst.id,
                &FixAction::DeleteInstanceFile {
                    relative_path: bad.into(),
                },
            )
            .await;
        assert!(r.is_err(), "{bad:?} debería rechazarse");
    }
    assert!(game.join("saves/w/level.dat").exists());
    assert!(game.join("mods/a.jar").exists());
    assert!(
        core.instance_dir(&inst.id)
            .unwrap()
            .join("instance.json")
            .exists()
    );
    // Archivo permitido pero inexistente.
    assert!(
        core.apply_fix(
            &inst.id,
            &FixAction::DeleteInstanceFile {
                relative_path: "options.txt".into()
            }
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn limpia_los_bloqueos_de_mundo() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let inst = core
        .create_instance(NewInstance {
            name: "Mundos".into(),
            game: fabric(),
        })
        .await
        .unwrap();
    let game = core.instance_game_dir(&inst.id).unwrap();
    assert!(
        core.apply_fix(&inst.id, &FixAction::ClearWorldLocks)
            .await
            .unwrap()
            .contains("No hay mundos")
    );
    write(&game.join("saves/uno/session.lock"), "x");
    write(&game.join("saves/uno/level.dat"), "x");
    write(&game.join("saves/dos/session.lock"), "x");
    write(&game.join("saves/tres/level.dat"), "x");
    write(&game.join("saves/suelto.txt"), "x");
    let msg = core
        .apply_fix(&inst.id, &FixAction::ClearWorldLocks)
        .await
        .unwrap();
    assert!(msg.contains('2'), "{msg}");
    assert!(!game.join("saves/uno/session.lock").exists());
    assert!(!game.join("saves/dos/session.lock").exists());
    assert!(game.join("saves/uno/level.dat").exists());
    assert!(game.join("saves/tres/level.dat").exists());
}

#[tokio::test]
async fn instalar_java_y_reinstalar_loader() {
    let dir = tempfile::tempdir().unwrap();
    let core = core(dir.path()).await;
    let inst = core
        .create_instance(NewInstance {
            name: "Pendientes".into(),
            game: fabric(),
        })
        .await
        .unwrap();
    // Sin red y sin una Java 9 en el equipo: no se puede instalar.
    let err = core
        .apply_fix(&inst.id, &FixAction::InstallJava { major: 9 })
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "offline");

    // Sin instalación previa del loader no se borra nada.
    let msg = core
        .apply_fix(&inst.id, &FixAction::ReinstallLoader)
        .await
        .unwrap();
    assert!(msg.contains("Pendientes"));
    // Con instalación, se borra su carpeta de `versions/`.
    let id = core.installed_loader_id(&inst.game).await.unwrap();
    let vdir = core.paths().versions_dir().join(&id);
    std::fs::create_dir_all(&vdir).unwrap();
    std::fs::write(vdir.join(format!("{id}.json")), "{}").unwrap();
    let msg = core
        .apply_fix(&inst.id, &FixAction::ReinstallLoader)
        .await
        .unwrap();
    assert!(msg.contains("Se borró"));
    assert!(!vdir.exists());

    assert_eq!(
        core.apply_fix("nada", &FixAction::ClearJvmArgs)
            .await
            .unwrap_err()
            .kind(),
        "instance_not_found"
    );
}
