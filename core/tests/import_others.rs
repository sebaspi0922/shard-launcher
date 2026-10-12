//! Importadores de `.minecraft` y de Prism/MultiMC contra árboles falsos.
//! Nunca se toca un launcher real.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use lcore::import::minecraft::infer_game;
use lcore::import::{
    DotMinecraftImport, PrismImport, Section, inspect_dot_minecraft, inspect_prism,
};
use lcore::instance::{GameVersion, LoaderKind};
use lcore::{Core, Options};
use serde_json::{Value, json};

const SENTINEL: &str = "SENTINEL_TOKEN_DO_NOT_COPY";

struct Env {
    _tmp: tempfile::TempDir,
    src: PathBuf,
    data: PathBuf,
    core: Core,
}

fn write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Un archivo con secretos que nadie debe abrir: en Unix, sin permisos de
/// lectura (si algo lo abriera, la importación fallaría).
fn secret(path: impl AsRef<Path>) {
    let path = path.as_ref();
    write(path, format!("{{\"accessToken\":\"{SENTINEL}\"}}"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o000)).unwrap();
    }
}

async fn env(build: impl FnOnce(&Path)) -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("origen");
    let data = tmp.path().join("data");
    build(&src);
    let core = Core::open(Options {
        data_dir: Some(data.clone()),
        offline: true,
        ..Options::default()
    })
    .await
    .unwrap();
    Env {
        _tmp: tmp,
        src,
        data,
        core,
    }
}

/// Todos los archivos bajo `root`, relativos.
fn tree(root: &Path) -> BTreeSet<String> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeSet<String>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if e.file_type().unwrap().is_dir() {
                walk(&p, root, out);
            } else {
                out.insert(p.strip_prefix(root).unwrap().display().to_string());
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(root, root, &mut out);
    out
}

/// Ningún archivo bajo `root` contiene el centinela.
fn assert_no_sentinel(root: &Path) {
    for rel in tree(root) {
        let bytes = fs::read(root.join(&rel)).unwrap();
        assert!(
            !String::from_utf8_lossy(&bytes).contains(SENTINEL),
            "{rel} contiene el centinela"
        );
    }
}

// ---------------------------------------------------------------------------
// .minecraft
// ---------------------------------------------------------------------------

fn version_json(root: &Path, id: &str, inherits: Option<&str>) {
    let mut v = json!({"id": id, "type": "release", "mainClass": "x"});
    if let Some(parent) = inherits {
        v["inheritsFrom"] = json!(parent);
    }
    write(root.join(format!("versions/{id}/{id}.json")), v.to_string());
}

fn build_dot_minecraft(root: &Path) {
    version_json(root, "1.20.1", None);
    version_json(root, "fabric-loader-0.15.11-1.20.1", Some("1.20.1"));
    version_json(root, "quilt-loader-0.26.0-1.20.1", Some("1.20.1"));
    version_json(root, "1.20.1-forge-47.2.0", Some("1.20.1"));
    version_json(root, "neoforge-21.1.77", Some("1.21.1"));
    version_json(root, "OptiFine_1.20.1_HD_U_I6", Some("1.20.1"));
    write(
        root.join("versions/OptiFine_1.20.1_HD_U_I6/OptiFine_1.20.1_HD_U_I6.jar"),
        b"jar",
    );
    fs::create_dir_all(root.join("versions/vacia")).unwrap(); // sin JSON

    write(
        root.join("launcher_profiles.json"),
        json!({
            "profiles": {
                "p1": {"name": "Vanilla", "lastVersionId": "1.20.1", "type": "custom",
                       "lastUsed": "2024-01-01T00:00:00.000Z"},
                "p2": {"name": "Con Fabric", "lastVersionId": "fabric-loader-0.15.11-1.20.1",
                       "gameDir": "/otra/parte", "type": "custom",
                       "lastUsed": "2024-06-01T00:00:00.000Z"},
                "p3": {"lastVersionId": "latest-release", "type": "latest-release",
                       "lastUsed": "2025-01-01T00:00:00.000Z"}
            },
            "authenticationDatabase": {"abc": {"accessToken": SENTINEL, "profiles": {}}},
            "clientToken": SENTINEL,
            "settings": {}
        })
        .to_string(),
    );
    // Cuentas del launcher oficial: no deben abrirse.
    secret(root.join("launcher_accounts.json"));
    secret(root.join("launcher_accounts_microsoft_store.json"));
    secret(root.join("launcher_msa_credentials.bin"));
    secret(root.join("launcher_msa_credentials_microsoft_store.bin"));

    write(root.join("saves/Mundo1/level.dat"), b"level-1");
    write(root.join("saves/Mundo1/region/r.0.0.mca"), vec![7u8; 100]);
    write(root.join("saves/Mundo2/level.dat"), b"level-2");
    fs::create_dir_all(root.join("saves/Vacio/data")).unwrap();
    write(root.join("mods/a.jar"), b"mod-a");
    write(root.join("mods/b.jar"), b"mod-b");
    write(root.join("config/sodium.json"), b"{}");
    write(root.join("resourcepacks/pack.zip"), b"zip");
    write(root.join("shaderpacks/shader.zip"), b"zip");
    write(root.join("screenshots/2024-01-01.png"), b"png");
    write(root.join("options.txt"), b"fov:90\n");
    write(root.join("servers.dat"), b"nbt");
    // Fuera de las secciones: no se copia.
    write(root.join("logs/latest.log"), b"log");
    write(root.join("usercache.json"), format!("[\"{SENTINEL}\"]"));
    write(root.join("versions/1.20.1/1.20.1.jar"), b"vanilla-jar");
}

#[test]
fn deduce_version_y_loader() {
    let g = |id, inherits| infer_game(id, inherits);
    let game = |mc: &str, loader, v: Option<&str>| GameVersion {
        mc: mc.into(),
        loader,
        loader_version: v.map(Into::into),
    };
    use LoaderKind::*;
    assert_eq!(g("1.20.1", None), game("1.20.1", Vanilla, None));
    assert_eq!(g("24w14a", None), game("24w14a", Vanilla, None));
    assert_eq!(
        g("fabric-loader-0.15.11-1.20.1", None),
        game("1.20.1", Fabric, Some("0.15.11"))
    );
    assert_eq!(
        g("fabric-loader-0.16.0-1.21.1", Some("1.21.1")),
        game("1.21.1", Fabric, Some("0.16.0"))
    );
    assert_eq!(
        g("quilt-loader-0.26.0-1.20.1", None),
        game("1.20.1", Quilt, Some("0.26.0"))
    );
    assert_eq!(
        g("1.20.1-forge-47.2.0", None),
        game("1.20.1", Forge, Some("47.2.0"))
    );
    assert_eq!(
        g("1.12.2-forge1.12.2-14.23.5.2860", None),
        game("1.12.2", Forge, Some("14.23.5.2860"))
    );
    assert_eq!(
        g("neoforge-21.1.77", None),
        game("1.21.1", NeoForge, Some("21.1.77"))
    );
    assert_eq!(
        g("neoforge-21.0.5-beta", None),
        game("1.21", NeoForge, Some("21.0.5-beta"))
    );
    assert_eq!(
        g("neoforge-47.1.106", Some("1.20.1")),
        game("1.20.1", NeoForge, Some("47.1.106"))
    );
    // Sin forma de saber la versión de Minecraft: queda como versión propia.
    assert_eq!(
        g("neoforge-99.9.9", None),
        game("neoforge-99.9.9", Custom, Some("neoforge-99.9.9"))
    );
    assert_eq!(
        g("OptiFine_1.20.1_HD_U_I6", Some("1.20.1")),
        game("1.20.1", Custom, Some("OptiFine_1.20.1_HD_U_I6"))
    );
}

#[tokio::test]
async fn inspecciona_dot_minecraft() {
    let e = env(build_dot_minecraft).await;
    let info = inspect_dot_minecraft(&e.src).await.unwrap();

    let ids: Vec<_> = info.versions.iter().map(|v| v.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "1.20.1",
            "1.20.1-forge-47.2.0",
            "OptiFine_1.20.1_HD_U_I6",
            "fabric-loader-0.15.11-1.20.1",
            "neoforge-21.1.77",
            "quilt-loader-0.26.0-1.20.1"
        ]
    );
    let fabric = info
        .versions
        .iter()
        .find(|v| v.id.starts_with("fabric"))
        .unwrap();
    assert_eq!(fabric.game.loader, LoaderKind::Fabric);
    assert_eq!(fabric.inherits_from.as_deref(), Some("1.20.1"));

    assert_eq!(info.profiles.len(), 3);
    let p2 = info.profiles.iter().find(|p| p.id == "p2").unwrap();
    assert_eq!(p2.name, "Con Fabric");
    assert_eq!(p2.game_dir.as_deref(), Some(Path::new("/otra/parte")));
    assert_eq!(
        info.profiles.iter().find(|p| p.id == "p3").unwrap().name,
        "p3"
    );

    let section = |s: Section| info.sections.iter().find(|i| i.section == s).unwrap();
    assert_eq!(
        (section(Section::Saves).files, section(Section::Saves).bytes),
        (3, 7 + 100 + 7)
    );
    assert_eq!(section(Section::Mods).files, 2);
    assert!(section(Section::Options).present);
    assert!(info.sections.iter().all(|s| s.present));

    // La salida no arrastra nada de las cuentas.
    let dump = serde_json::to_string(&info).unwrap();
    assert!(!dump.contains(SENTINEL));

    // Una carpeta cualquiera no es un .minecraft.
    let vacio = e.src.parent().unwrap().join("vacio");
    fs::create_dir_all(&vacio).unwrap();
    assert!(inspect_dot_minecraft(&vacio).await.is_err());
    assert!(
        inspect_dot_minecraft(&e.src.join("no-existe"))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn simulacion_no_escribe() {
    let e = env(build_dot_minecraft).await;
    let report = e
        .core
        .import_dot_minecraft(&DotMinecraftImport {
            dry_run: true,
            ..DotMinecraftImport::new(&e.src)
        })
        .await
        .unwrap();
    assert!(report.imported.is_none());
    assert_eq!(report.plan.new_id, "minecraft-1-20-1");
    assert!(report.plan.files > 10 && report.plan.bytes > 100);
    assert!(!e.data.join("instances").exists());
    assert!(e.core.list_instances().await.unwrap().is_empty());
}

#[tokio::test]
async fn importa_todo_y_no_toca_cuentas() {
    let e = env(build_dot_minecraft).await;
    let report = e
        .core
        .import_dot_minecraft(&DotMinecraftImport {
            name: Some("Mi mundo".into()),
            ..DotMinecraftImport::new(&e.src)
        })
        .await
        .unwrap();
    let id = report.imported.clone().unwrap();
    assert_eq!(id, "mi-mundo");
    // El perfil usado más recientemente con versión instalada es p2 (Fabric).
    assert_eq!(report.plan.game.loader, LoaderKind::Fabric);
    assert_eq!(report.plan.game.mc, "1.20.1");
    assert_eq!(report.plan.game.loader_version.as_deref(), Some("0.15.11"));
    assert!(!report.plan.copies_version_dir);

    let inst = e.core.get_instance(&id).await.unwrap();
    assert_eq!(inst.name, "Mi mundo");
    assert_eq!(inst.game, report.plan.game);
    let from = inst.imported_from.unwrap();
    assert_eq!(from.kind, "dot-minecraft");
    assert_eq!(
        from.original_id,
        fs::canonicalize(&e.src).unwrap().display().to_string()
    );

    let game = e.core.instance_game_dir(&id).unwrap();
    let copied = tree(&game);
    let expected: BTreeSet<String> = [
        "saves/Mundo1/level.dat",
        "saves/Mundo1/region/r.0.0.mca",
        "saves/Mundo2/level.dat",
        "mods/a.jar",
        "mods/b.jar",
        "config/sodium.json",
        "resourcepacks/pack.zip",
        "shaderpacks/shader.zip",
        "screenshots/2024-01-01.png",
        "options.txt",
        "servers.dat",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    assert_eq!(copied, expected);
    assert!(game.join("saves/Vacio/data").is_dir()); // carpetas vacías incluidas
    assert_eq!(fs::read(game.join("mods/a.jar")).unwrap(), b"mod-a");
    assert_eq!(report.copied_files, 11);

    // Es una copia: el origen sigue igual.
    assert!(e.src.join("mods/a.jar").exists());
    assert_ne!(
        fs::canonicalize(game.join("mods/a.jar")).unwrap(),
        fs::canonicalize(e.src.join("mods/a.jar")).unwrap()
    );

    // Nada de las cuentas ni de launcher_profiles.json llega al destino.
    assert_no_sentinel(&e.data);
    assert!(!serde_json::to_string(&report).unwrap().contains(SENTINEL));
    for name in [
        "launcher_accounts.json",
        "launcher_accounts_microsoft_store.json",
        "launcher_msa_credentials.bin",
        "launcher_profiles.json",
        "usercache.json",
    ] {
        assert!(
            !tree(&e.data).iter().any(|f| f.ends_with(name)),
            "{name} copiado"
        );
    }
}

#[tokio::test]
async fn secciones_version_y_loader_a_medida() {
    let e = env(build_dot_minecraft).await;
    let report = e
        .core
        .import_dot_minecraft(&DotMinecraftImport {
            mc_version: Some("1.20.1-forge-47.2.0".into()),
            sections: vec![Section::Saves, Section::Options, Section::Saves],
            ..DotMinecraftImport::new(&e.src)
        })
        .await
        .unwrap();
    let id = report.imported.unwrap();
    assert_eq!(report.plan.name, "Minecraft 1.20.1");
    assert_eq!(report.plan.game.loader, LoaderKind::Forge);
    assert_eq!(report.plan.game.loader_version.as_deref(), Some("47.2.0"));
    let copied = tree(&e.core.instance_game_dir(&id).unwrap());
    assert!(
        copied
            .iter()
            .all(|f| f.starts_with("saves/") || f == "options.txt")
    );
    assert!(copied.contains("options.txt") && copied.contains("saves/Mundo2/level.dat"));

    // Versión de Minecraft escrita a mano + loader forzado; también por nombre de perfil.
    let report = e
        .core
        .import_dot_minecraft(&DotMinecraftImport {
            name: Some("Otra".into()),
            mc_version: Some("1.19.4".into()),
            loader: Some(LoaderKind::Quilt),
            sections: vec![Section::Mods],
            ..DotMinecraftImport::new(&e.src)
        })
        .await
        .unwrap();
    assert_eq!(report.plan.game.mc, "1.19.4");
    assert_eq!(report.plan.game.loader, LoaderKind::Quilt);
    assert_eq!(report.plan.game.loader_version, None);

    let report = e
        .core
        .import_dot_minecraft(&DotMinecraftImport {
            name: Some("Por perfil".into()),
            mc_version: Some("Vanilla".into()),
            sections: vec![Section::Screenshots],
            dry_run: true,
            ..DotMinecraftImport::new(&e.src)
        })
        .await
        .unwrap();
    assert_eq!(report.plan.game.loader, LoaderKind::Vanilla);
    assert_eq!(report.plan.game.mc, "1.20.1");

    // Pedir una sección que no existe avisa.
    fs::remove_dir_all(e.src.join("shaderpacks")).unwrap();
    let report = e
        .core
        .import_dot_minecraft(&DotMinecraftImport {
            sections: vec![Section::ShaderPacks],
            dry_run: true,
            ..DotMinecraftImport::new(&e.src)
        })
        .await
        .unwrap();
    assert!(
        report
            .plan
            .warnings
            .iter()
            .any(|w| w.contains("shaderpacks"))
    );
    assert_eq!(report.plan.files, 0);
}

#[tokio::test]
async fn version_propia_se_copia_a_versions() {
    let e = env(build_dot_minecraft).await;
    let report = e
        .core
        .import_dot_minecraft(&DotMinecraftImport {
            mc_version: Some("OptiFine_1.20.1_HD_U_I6".into()),
            sections: vec![Section::Options],
            ..DotMinecraftImport::new(&e.src)
        })
        .await
        .unwrap();
    assert!(report.plan.copies_version_dir);
    assert_eq!(report.plan.game.loader, LoaderKind::Custom);
    let dir = e.data.join("versions/OptiFine_1.20.1_HD_U_I6");
    assert!(dir.join("OptiFine_1.20.1_HD_U_I6.json").is_file());
    assert_eq!(
        fs::read(dir.join("OptiFine_1.20.1_HD_U_I6.jar")).unwrap(),
        b"jar"
    );
}

#[tokio::test]
async fn sin_perfiles_usa_la_version_mas_nueva_y_rechaza_solapes() {
    let e = env(|root| {
        version_json(root, "1.19.2", None);
        write(root.join("mods/a.jar"), b"x");
    })
    .await;
    let report = e
        .core
        .import_dot_minecraft(&DotMinecraftImport::new(&e.src))
        .await
        .unwrap();
    assert_eq!(report.plan.game.mc, "1.19.2");

    // Sin versiones ni perfiles hay que indicar la versión.
    let e2 = env(|root| write(root.join("mods/a.jar"), b"x")).await;
    let err = e2
        .core
        .import_dot_minecraft(&DotMinecraftImport::new(&e2.src))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("--version"), "{err}");

    // El origen no puede contener los datos de la app.
    let tmp = tempfile::tempdir().unwrap();
    version_json(tmp.path(), "1.20.1", None);
    let core = Core::open(Options {
        data_dir: Some(tmp.path().join("datos")),
        offline: true,
        ..Options::default()
    })
    .await
    .unwrap();
    assert!(
        core.import_dot_minecraft(&DotMinecraftImport::new(tmp.path()))
            .await
            .is_err()
    );
}

// ---------------------------------------------------------------------------
// Prism / MultiMC
// ---------------------------------------------------------------------------

fn pack(components: &[(&str, &str)]) -> String {
    let list: Vec<_> = components
        .iter()
        .map(|(uid, v)| json!({"uid": uid, "version": v, "important": true}))
        .collect();
    json!({"components": list, "formatVersion": 1}).to_string()
}

fn build_prism(root: &Path) {
    // Cuentas y ajustes de Prism: no se abren.
    secret(root.join("accounts.json"));
    secret(root.join("prismlauncher.cfg"));
    write(root.join("instgroups.json"), b"{}");

    let a = root.join("instances/Mi Fabric");
    write(
        a.join("instance.cfg"),
        "[General]\nconfigVersion=1.2\nInstanceType=OneSix\nJavaPath=/usr/lib/jvm/java-17/bin/java\n\
         JvmArgs=-Xmx4G -XX:+UseG1GC -Dfoo=bar\nMaxMemAlloc=6144\nMinMemAlloc=512\n\
         OverrideJavaArgs=true\nOverrideMemory=true\nOverrideWindow=true\nMinecraftWinWidth=1280\n\
         MinecraftWinHeight=720\niconKey=creeper\nname=Mi Fabric\nlastLaunchTime=1700000000000\n\
         totalTimePlayed=7200\n",
    );
    write(
        a.join("mmc-pack.json"),
        pack(&[
            ("net.minecraft", "1.20.1"),
            ("net.fabricmc.intermediary", "1.20.1"),
            ("net.fabricmc.fabric-loader", "0.15.11"),
        ]),
    );
    write(a.join(".minecraft/mods/sodium.jar"), b"sodium");
    write(a.join(".minecraft/saves/Mundo/level.dat"), b"lvl");
    write(a.join(".minecraft/options.txt"), b"fov:70");
    write(a.join(".minecraft/logs/latest.log"), b"log");
    write(a.join(".minecraft/crash-reports/crash.txt"), b"crash");

    let b = root.join("instances/forge-vieja");
    write(
        b.join("instance.cfg"),
        "name=Forge vieja\nMaxMemAlloc=9999\nJvmArgs=-Xmx1G\nOverrideMemory=false\nJavaPath=\"C:\\\\Java\\\\java.exe\"\n",
    );
    write(
        b.join("mmc-pack.json"),
        pack(&[
            ("net.minecraft", "1.12.2"),
            ("net.minecraftforge", "14.23.5.2860"),
        ]),
    );
    write(b.join("minecraft/mods/x.jar"), b"x");

    let c = root.join("instances/Quilt");
    write(c.join("instance.cfg"), "name=Quilt\n");
    write(
        c.join("mmc-pack.json"),
        pack(&[
            ("net.minecraft", "1.20.1"),
            ("org.quiltmc.quilt-loader", "0.26.0"),
        ]),
    );
    write(c.join(".minecraft/mods/q.jar"), b"q");

    let d = root.join("instances/Neo");
    write(d.join("instance.cfg"), "name=Neo\n");
    write(
        d.join("mmc-pack.json"),
        pack(&[("net.minecraft", "1.21.1"), ("net.neoforged", "21.1.77")]),
    );

    // Sin mmc-pack.json: no importable.
    write(root.join("instances/rota/instance.cfg"), "name=Rota\n");
    // Carpetas internas y sueltas.
    write(
        root.join("instances/_LAUNCHER_TEMP/instance.cfg"),
        "name=x\n",
    );
    write(root.join("instances/.tmp/instance.cfg"), "name=y\n");
    write(root.join("instances/sin-cfg/algo.txt"), "z");
    write(root.join("instances/instgroups.json"), b"{}");
}

#[tokio::test]
async fn inspecciona_prism_raiz_e_instancia() {
    let e = env(build_prism).await;
    let info = inspect_prism(&e.src).await.unwrap();
    assert!(info.is_root);
    let folders: Vec<_> = info.instances.iter().map(|i| i.folder.as_str()).collect();
    assert_eq!(
        folders,
        ["Mi Fabric", "Neo", "Quilt", "forge-vieja", "rota"]
    );
    assert!(info.warnings.iter().any(|w| w.contains("sin-cfg")));

    let by = |f: &str| info.instances.iter().find(|i| i.folder == f).unwrap();
    let fabric = by("Mi Fabric");
    assert_eq!(fabric.name, "Mi Fabric");
    assert_eq!(fabric.icon_key.as_deref(), Some("creeper"));
    let game = fabric.game.clone().unwrap();
    assert_eq!(
        (
            game.mc.as_str(),
            game.loader,
            game.loader_version.as_deref()
        ),
        ("1.20.1", LoaderKind::Fabric, Some("0.15.11"))
    );
    assert_eq!(fabric.memory_mb, Some(6144));
    assert_eq!(
        fabric.jvm_args.as_deref(),
        Some(&["-XX:+UseG1GC".to_owned(), "-Dfoo=bar".to_owned()][..])
    );
    assert_eq!((fabric.width, fabric.height), (Some(1280), Some(720)));
    assert_eq!(fabric.last_played, Some(1_700_000_000));
    assert_eq!(fabric.playtime_seconds, 7200);
    assert!(fabric.game_dir.as_ref().unwrap().ends_with(".minecraft"));
    assert!(fabric.unusable_reason.is_none());

    let forge = by("forge-vieja");
    assert_eq!(forge.name, "Forge vieja");
    assert_eq!(forge.game.as_ref().unwrap().loader, LoaderKind::Forge);
    assert_eq!(forge.memory_mb, None); // OverrideMemory=false
    assert_eq!(forge.jvm_args, None);
    assert!(forge.game_dir.as_ref().unwrap().ends_with("minecraft"));
    assert_eq!(by("Quilt").game.as_ref().unwrap().loader, LoaderKind::Quilt);
    let neo = by("Neo").game.clone().unwrap();
    assert_eq!(
        (neo.mc.as_str(), neo.loader, neo.loader_version.as_deref()),
        ("1.21.1", LoaderKind::NeoForge, Some("21.1.77"))
    );
    assert!(
        by("rota")
            .unusable_reason
            .as_deref()
            .unwrap()
            .contains("mmc-pack")
    );

    // Instancia suelta y la carpeta `instances` directamente.
    let single = inspect_prism(&e.src.join("instances/Mi Fabric"))
        .await
        .unwrap();
    assert!(!single.is_root);
    assert_eq!(single.instances.len(), 1);
    assert_eq!(single.instances[0].name, "Mi Fabric");
    let inner = inspect_prism(&e.src.join("instances")).await.unwrap();
    assert!(inner.is_root);
    assert_eq!(inner.instances.len(), 5);

    assert!(!serde_json::to_string(&info).unwrap().contains(SENTINEL));
    let vacio = e.src.join("instances/sin-cfg");
    assert!(inspect_prism(&vacio).await.is_err());
}

#[tokio::test]
async fn importa_prism_completo_e_idempotente() {
    let e = env(build_prism).await;

    let dry = e
        .core
        .import_prism(&PrismImport {
            source: e.src.clone(),
            dry_run: true,
            ..PrismImport::default()
        })
        .await
        .unwrap();
    assert!(dry.imported.is_empty());
    assert!(!e.data.join("instances").exists());
    assert_eq!(dry.instances.len(), 5);
    let fabric = dry
        .instances
        .iter()
        .find(|i| i.folder == "Mi Fabric")
        .unwrap();
    assert_eq!(fabric.new_id, "mi-fabric");
    assert_eq!(fabric.files, 5);
    assert!(dry.instances.iter().any(|i| i.skipped_reason.is_some()));

    let report = e
        .core
        .import_prism(&PrismImport {
            source: e.src.clone(),
            ..PrismImport::default()
        })
        .await
        .unwrap();
    assert_eq!(
        report.imported,
        ["mi-fabric", "neo", "quilt", "forge-vieja"]
    );
    assert_eq!(report.copied_files, 5 + 1 + 1);

    let inst = e.core.get_instance("mi-fabric").await.unwrap();
    assert_eq!(inst.name, "Mi Fabric");
    assert_eq!(inst.game.loader, LoaderKind::Fabric);
    assert_eq!(inst.java.memory_mb, Some(6144));
    assert_eq!(
        inst.java.jvm_args,
        Some(vec!["-XX:+UseG1GC".to_owned(), "-Dfoo=bar".to_owned()])
    );
    assert_eq!(
        (inst.window.width, inst.window.height),
        (Some(1280), Some(720))
    );
    assert_eq!(inst.playtime_seconds, 7200);
    assert_eq!(inst.last_played, Some(1_700_000_000));
    assert_eq!(inst.java.path, None);
    let from = inst.imported_from.unwrap();
    assert_eq!(from.kind, "prism");
    assert!(from.original_id.ends_with("instances/Mi Fabric"));

    // Se copia todo el directorio del juego, logs incluidos.
    let game = e.core.instance_game_dir("mi-fabric").unwrap();
    let copied = tree(&game);
    assert!(copied.contains("mods/sodium.jar") && copied.contains("logs/latest.log"));
    assert!(copied.contains("crash-reports/crash.txt") && copied.contains("saves/Mundo/level.dat"));
    assert_eq!(copied.len(), 5);
    let forge = e.core.get_instance("forge-vieja").await.unwrap();
    assert_eq!(forge.java.memory_mb, None);
    assert!(
        e.core
            .instance_game_dir("forge-vieja")
            .unwrap()
            .join("mods/x.jar")
            .is_file()
    );
    // Sin carpeta de juego: instancia con `game/` vacía.
    assert!(e.core.instance_game_dir("neo").unwrap().is_dir());

    // Cuentas y ajustes de Prism, ni abiertos ni copiados.
    assert_no_sentinel(&e.data);
    assert!(!tree(&e.data).iter().any(|f| f.ends_with("accounts.json")));
    assert!(!serde_json::to_string(&report).unwrap().contains(SENTINEL));

    // Idempotente.
    let again = e
        .core
        .import_prism(&PrismImport {
            source: e.src.clone(),
            ..PrismImport::default()
        })
        .await
        .unwrap();
    assert!(again.imported.is_empty());
    assert!(again.instances.iter().any(|i| {
        i.skipped_reason
            .as_deref()
            .is_some_and(|r| r.contains("ya importada como mi-fabric"))
    }));
    assert_eq!(e.core.list_instances().await.unwrap().len(), 4);

    // Con --force se reimporta con id nuevo.
    let forced = e
        .core
        .import_prism(&PrismImport {
            source: e.src.clone(),
            instances: vec!["mi fabric".into()],
            force: true,
            ..PrismImport::default()
        })
        .await
        .unwrap();
    assert_eq!(forced.imported, ["mi-fabric-2"]);
    assert_eq!(e.core.list_instances().await.unwrap().len(), 5);
}

#[tokio::test]
async fn importa_prism_una_instancia_y_valida_nombres() {
    let e = env(build_prism).await;
    let err = e
        .core
        .import_prism(&PrismImport {
            source: e.src.clone(),
            instances: vec!["Fantasma".into()],
            ..PrismImport::default()
        })
        .await
        .unwrap_err();
    assert!(err.to_string().contains("Fantasma"), "{err}");
    assert!(err.to_string().contains("Mi Fabric"), "{err}");
    assert!(e.core.list_instances().await.unwrap().is_empty());

    // Por carpeta (sin distinguir mayúsculas) y por nombre.
    let report = e
        .core
        .import_prism(&PrismImport {
            source: e.src.clone(),
            instances: vec!["FORGE-VIEJA".into(), "quilt".into()],
            ..PrismImport::default()
        })
        .await
        .unwrap();
    assert_eq!(report.imported, ["quilt", "forge-vieja"]);

    // Una instancia suelta.
    let single = e
        .core
        .import_prism(&PrismImport {
            source: e.src.join("instances/Neo"),
            ..PrismImport::default()
        })
        .await
        .unwrap();
    assert_eq!(single.imported, ["neo"]);
    let neo = e.core.get_instance("neo").await.unwrap();
    assert_eq!(neo.game.loader, LoaderKind::NeoForge);

    // Una instancia sin mmc-pack.json se omite y no falla.
    let broken = e
        .core
        .import_prism(&PrismImport {
            source: e.src.join("instances/rota"),
            ..PrismImport::default()
        })
        .await
        .unwrap();
    assert!(broken.imported.is_empty());
    assert!(broken.instances[0].skipped_reason.is_some());
}

#[tokio::test]
async fn prism_rechaza_origen_dentro_de_los_datos() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().to_path_buf();
    build_prism(&data.join("prism"));
    let core = Core::open(Options {
        data_dir: Some(data.clone()),
        offline: true,
        ..Options::default()
    })
    .await
    .unwrap();
    let err = core
        .import_prism(&PrismImport {
            source: data.join("prism"),
            ..PrismImport::default()
        })
        .await
        .unwrap_err();
    assert!(err.to_string().contains("no pueden contenerse"), "{err}");
}

#[tokio::test]
async fn json_del_informe_es_estable() {
    // La CLI y la app consumen estos nombres.
    let e = env(build_dot_minecraft).await;
    let report = e
        .core
        .import_dot_minecraft(&DotMinecraftImport {
            dry_run: true,
            ..DotMinecraftImport::new(&e.src)
        })
        .await
        .unwrap();
    let v: Value = serde_json::to_value(&report).unwrap();
    assert_eq!(v["plan"]["sections"][0]["section"], "saves");
    assert_eq!(v["plan"]["sections"][3]["section"], "resourcepacks");
    assert!(v["imported"].is_null());
}
