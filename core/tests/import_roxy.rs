//! Importador de Roxy contra un árbol falso que imita `~/.local/share/.roxy`.
//! Nunca se toca el Roxy real.

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use lcore::import::{ContentIndex, RoxyImportOptions};
use lcore::{Core, Options};
use serde_json::{Value, json};

const SENTINEL: &str = "SENTINEL_TOKEN_DO_NOT_COPY";

struct Env {
    _tmp: tempfile::TempDir,
    roxy: PathBuf,
    data: PathBuf,
    core: Core,
}

fn write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn instance_json(id: &str, name: &str, extra: Value) -> Value {
    let mut base = json!({
        "id": id,
        "name": name,
        "mcVersion": "1.20.1",
        "loaderType": "fabric",
        "loaderVersion": "0.15.0",
        "createdAt": 1_700_000_000,
        "lastPlayed": 1_700_001_000,
        "playtimeSeconds": 3600,
        "iconPath": null,
        "memoryMb": null,
        "jvmArgs": null,
        "resolutionWidth": null,
        "resolutionHeight": null,
        "devMode": false,
        "backupAutoEnabled": false,
        "backupIntervalHours": null,
        "backupMaxCount": null,
        "deletedAt": null
    });
    base.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    base
}

/// Árbol falso de Roxy con secretos centinela.
fn build_roxy(root: &Path) {
    write(
        root.join("session.json"),
        format!("{{\"mc_access_token\":\"{SENTINEL}\"}}"),
    );
    write(
        root.join("launcher_account.json"),
        format!("{{\"email\":\"{SENTINEL}\"}}"),
    );
    write(
        root.join("local_settings.json"),
        "{\"closeBehavior\":\"quit\"}",
    );

    let accounts = root.join("game/accounts");

    // Cuenta A: instancia normal.
    let main = accounts.join("acc-a/instances/principal-1700000000");
    let main_json = instance_json(
        "principal-1700000000",
        "Principal",
        json!({
            "memoryMb": 12288,
            "jvmArgs": "-Xmx12G -Xms2G -XX:+UseG1GC -XX:MaxGCPauseMillis=50",
            "resolutionWidth": 1280,
            "resolutionHeight": 720,
            "devMode": true,
            "backupAutoEnabled": true,
            "backupIntervalHours": 72,
            "backupMaxCount": 3,
            "iconPath": "icon.png"
        }),
    );
    write(main.join("instance.json"), main_json.to_string());
    write(main.join("icon.png"), "PNG-FAKE");
    write(main.join("mods/sodium.jar"), "sodium-bytes");
    write(main.join("mods/lithium.jar.disabled"), "lithium-bytes");
    write(main.join("mods/modpack-only.jar"), "mp-bytes");
    write(main.join("config/sodium.json"), "{}");
    write(main.join("saves/Mundo1/level.dat"), "level");
    write(main.join("saves/Mundo1/region/r.0.0.mca"), vec![7u8; 4096]);
    write(main.join("options.txt"), "fov:70");
    write(
        main.join("latest_launch.log"),
        format!("java --accessToken {SENTINEL}"),
    );
    write(main.join("logs/latest.log"), "log");
    write(main.join("crash-reports/crash-1.txt"), "crash");
    write(main.join("backups/backup-1700000100.zip"), "zip-bytes-1");
    write(main.join("backups/backup-1700000200.zip"), "zip-bytes-22");
    write(
        main.join("backups/.backups.json"),
        json!([
            {"id": "backup-1700000100", "createdAt": 1_700_000_150, "sizeBytes": 11,
             "pinned": true, "label": "antes de actualizar", "automatic": false},
            {"id": "backup-1700000200", "createdAt": 1_700_000_260, "sizeBytes": 12,
             "pinned": false, "label": null, "automatic": true},
            {"id": "backup-1700000999", "createdAt": 1, "sizeBytes": 1,
             "pinned": false, "label": null, "automatic": true}
        ])
        .to_string(),
    );
    write(
        main.join(".modrinth_sources.json"),
        json!({
            "mods/sodium.jar": {
                "download_url": "https://cdn.modrinth.com/data/AANobbMI/versions/x/sodium.jar",
                "sha1": "aaaa", "project_id": "AANobbMI", "project_name": "Sodium",
                "version_number": "0.5.0", "mod_category": "optimization"
            },
            "mods/lithium.jar.disabled": {
                "download_url": "https://cdn.modrinth.com/data/gvQqBUqZ/versions/y/lithium.jar",
                "sha1": "bbbb", "project_id": "gvQqBUqZ", "project_name": "Lithium",
                "version_number": "0.11.0", "mod_category": null
            },
            "mods/modpack-only.jar": {
                "download_url": null, "sha1": "cccc", "project_id": null,
                "project_name": null, "version_number": null, "mod_category": null
            }
        })
        .to_string(),
    );
    // Enlaces: uno interior (se conserva) y otro que escapa al secreto (se omite).
    std::os::unix::fs::symlink("sodium.json", main.join("config/enlace-interior.json")).unwrap();
    std::os::unix::fs::symlink(
        root.join("session.json"),
        main.join("config/enlace-fuga.json"),
    )
    .unwrap();

    // Cuenta A: instancia con valores nulos y un icono que apunta fuera.
    let vacia = accounts.join("acc-a/instances/vacia-1700000500");
    let vacia_json = instance_json(
        "vacia-1700000500",
        "Vacía",
        json!({"iconPath": root.join("launcher_account.json")}),
    );
    write(vacia.join("instance.json"), vacia_json.to_string());
    write(vacia.join("options.txt"), "x");

    // Cuenta B: instancia en la papelera y otra con loader desconocido.
    let vieja = accounts.join("acc-b/instances/vieja-1600000000");
    let vieja_json = instance_json(
        "vieja-1600000000",
        "Vieja",
        json!({"deletedAt": 1_650_000_000}),
    );
    write(vieja.join("instance.json"), vieja_json.to_string());
    write(vieja.join("mods/old.jar"), "old");
    let rara = accounts.join("acc-b/instances/rara-1700000900");
    write(
        rara.join("instance.json"),
        instance_json("rara-1700000900", "Rara", json!({"loaderType": "mystery"})).to_string(),
    );

    // Contenido compartido.
    let game = root.join("game");
    write(game.join("libraries/org/lib/1.0/lib-1.0.jar"), "lib-bytes");
    write(
        game.join("libraries/net/other/2.0/other-2.0.jar"),
        "other-bytes",
    );
    write(game.join("assets/indexes/32.json"), "{}");
    write(game.join("assets/objects/ab/abcdef0123"), "asset-object");
    write(game.join("assets/skins/skin.png"), "skin-cache");
    write(game.join("versions/1.20.1/1.20.1.jar"), "client-jar");
    write(game.join("versions/1.20.1/extra.txt"), "no se importa");
    write(game.join("java/25/bin/java"), "java-binary");
    write(game.join("java/25/release"), "JAVA_VERSION=25");
    write(game.join("natives/1.20.1/lwjgl.so"), "natives");
}

async fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let roxy = tmp.path().join(".roxy");
    let data = tmp.path().join("data");
    build_roxy(&roxy);
    let core = Core::open(Options {
        data_dir: Some(data.clone()),
        offline: true,
        ..Options::default()
    })
    .await
    .unwrap();
    Env {
        _tmp: tmp,
        roxy,
        data,
        core,
    }
}

fn opts(env: &Env) -> RoxyImportOptions {
    RoxyImportOptions {
        source_root: env.roxy.clone(),
        ..RoxyImportOptions::default()
    }
}

/// Rutas relativas de todo lo que hay bajo `root` (archivos, carpetas y enlaces).
fn tree(root: &Path) -> BTreeSet<String> {
    fn go(dir: &Path, root: &Path, out: &mut BTreeSet<String>) {
        for e in fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            let path = e.path();
            out.insert(path.strip_prefix(root).unwrap().display().to_string());
            if e.file_type().unwrap().is_dir() {
                go(&path, root, out);
            }
        }
    }
    let mut out = BTreeSet::new();
    go(root, root, &mut out);
    out
}

/// Todos los archivos regulares bajo `root` (sin seguir enlaces).
fn files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in fs::read_dir(&dir).unwrap() {
            let e = e.unwrap();
            let ft = e.file_type().unwrap();
            if ft.is_dir() {
                stack.push(e.path());
            } else if ft.is_file() {
                out.push(e.path());
            }
        }
    }
    out
}

fn ino(path: &Path) -> u64 {
    fs::metadata(path).unwrap().ino()
}

#[tokio::test]
async fn dry_run_no_escribe_nada() {
    let env = env().await;
    let before = tree(&env.data);
    let before_src = tree(&env.roxy);

    let mut o = opts(&env);
    o.dry_run = true;
    o.include_trash = true;
    let report = env.core.import_roxy(&o).await.unwrap();
    let plan = env.core.plan_roxy_import(&o).await.unwrap();

    assert_eq!(tree(&env.data), before, "el dry-run escribió en el destino");
    assert_eq!(tree(&env.roxy), before_src, "se modificó el origen");
    assert!(report.imported.is_empty());
    assert_eq!(report.linked_files + report.copied_files, 0);

    let ids: Vec<_> = plan
        .instances
        .iter()
        .map(|i| (i.original_id.as_str(), i.new_id.as_str()))
        .collect();
    assert_eq!(
        ids,
        [
            ("principal-1700000000", "principal"),
            ("vacia-1700000500", "vacia"),
            ("rara-1700000900", ""),
            ("vieja-1600000000", "vieja"),
        ]
    );
    let principal = &plan.instances[0];
    // 12 archivos: mods(3) config(1+1 enlace interior) saves(2) options icon.png logs crash.
    assert_eq!(principal.files, 3 + 2 + 2 + 1 + 1 + 1 + 1);
    assert_eq!(principal.backups, 2);
    assert!(principal.bytes > 4096);
    assert!(
        plan.instances[2]
            .skipped_reason
            .as_deref()
            .unwrap()
            .contains("mystery")
    );
    assert_eq!(plan.shared.libraries_files, 2);
    assert_eq!(plan.shared.assets_files, 2);
    assert_eq!(plan.shared.version_jars, 1);
    assert_eq!(plan.shared.java_majors, [25]);
    assert!(plan.warnings.iter().any(|w| w.contains("enlace-fuga.json")));
}

#[tokio::test]
async fn importa_instancias_y_contenido() {
    let env = env().await;
    let report = env.core.import_roxy(&opts(&env)).await.unwrap();
    assert_eq!(report.imported, ["principal", "vacia"]);

    // instance.json mapeado.
    let main = env.core.get_instance("principal").await.unwrap();
    assert_eq!(main.name, "Principal");
    assert_eq!(main.game.mc, "1.20.1");
    assert_eq!(main.game.loader.as_str(), "fabric");
    assert_eq!(main.game.loader_version.as_deref(), Some("0.15.0"));
    assert_eq!(main.created_at, 1_700_000_000);
    assert_eq!(main.last_played, Some(1_700_001_000));
    assert_eq!(main.playtime_seconds, 3600);
    assert_eq!(main.java.memory_mb, Some(12288));
    assert_eq!(
        main.java.jvm_args,
        Some(vec![
            "-XX:+UseG1GC".to_owned(),
            "-XX:MaxGCPauseMillis=50".to_owned()
        ])
    );
    assert_eq!(
        (main.window.width, main.window.height),
        (Some(1280), Some(720))
    );
    assert!(main.dev_console);
    assert!(main.backups.auto);
    assert_eq!(
        (main.backups.interval_hours, main.backups.max_count),
        (Some(72), Some(3))
    );
    let from = main.imported_from.unwrap();
    assert_eq!(
        (from.kind.as_str(), from.original_id.as_str()),
        ("roxy", "principal-1700000000")
    );

    let vacia = env.core.get_instance("vacia").await.unwrap();
    assert_eq!(vacia.java.memory_mb, None);
    assert_eq!(vacia.java.jvm_args, None);
    assert_eq!((vacia.window.width, vacia.window.height), (None, None));
    assert!(!vacia.dev_console && !vacia.backups.auto);

    // Archivos del juego: copiados (inodos distintos), con exclusiones.
    let game = env.core.instance_game_dir("principal").unwrap();
    let src = env
        .roxy
        .join("game/accounts/acc-a/instances/principal-1700000000");
    for rel in [
        "mods/sodium.jar",
        "mods/lithium.jar.disabled",
        "saves/Mundo1/region/r.0.0.mca",
        "options.txt",
        "logs/latest.log",
        "crash-reports/crash-1.txt",
        "icon.png",
    ] {
        assert_eq!(
            fs::read(game.join(rel)).unwrap(),
            fs::read(src.join(rel)).unwrap(),
            "{rel}"
        );
        assert_ne!(
            ino(&game.join(rel)),
            ino(&src.join(rel)),
            "{rel} debe ser copia"
        );
    }
    assert_eq!(
        fs::metadata(game.join("options.txt"))
            .unwrap()
            .modified()
            .unwrap(),
        fs::metadata(src.join("options.txt"))
            .unwrap()
            .modified()
            .unwrap()
    );
    for rel in [
        "instance.json",
        ".modrinth_sources.json",
        "backups",
        "latest_launch.log",
    ] {
        assert!(!game.join(rel).exists(), "{rel} no debe copiarse");
    }
    assert!(
        fs::symlink_metadata(game.join("config/enlace-interior.json"))
            .unwrap()
            .is_symlink()
    );
    assert_eq!(
        fs::read_link(game.join("config/enlace-interior.json")).unwrap(),
        Path::new("sodium.json")
    );
    assert!(fs::symlink_metadata(game.join("config/enlace-fuga.json")).is_err());

    // Icono (el de la instancia nula apuntaba fuera: no se copia).
    let inst_dir = env.core.instance_dir("principal").unwrap();
    assert_eq!(fs::read(inst_dir.join("icon.png")).unwrap(), b"PNG-FAKE");
    assert!(
        !env.core
            .instance_dir("vacia")
            .unwrap()
            .join("icon.png")
            .exists()
    );

    // content.json.
    let content: ContentIndex =
        serde_json::from_slice(&fs::read(inst_dir.join("content.json")).unwrap()).unwrap();
    assert_eq!(content.schema, 1);
    let raw: Value =
        serde_json::from_slice(&fs::read(inst_dir.join("content.json")).unwrap()).unwrap();
    assert_eq!(
        raw["files"],
        json!([
            {"type": "mod", "file": "lithium.jar", "enabled": false, "source": "modrinth",
             "project_id": "gvQqBUqZ", "project_name": "Lithium", "version_number": "0.11.0",
             "sha1": "bbbb", "url": "https://cdn.modrinth.com/data/gvQqBUqZ/versions/y/lithium.jar",
             "category": null},
            {"type": "mod", "file": "modpack-only.jar", "enabled": true, "source": "unknown",
             "project_id": null, "project_name": null, "version_number": null,
             "sha1": "cccc", "url": null, "category": null},
            {"type": "mod", "file": "sodium.jar", "enabled": true, "source": "modrinth",
             "project_id": "AANobbMI", "project_name": "Sodium", "version_number": "0.5.0",
             "sha1": "aaaa", "url": "https://cdn.modrinth.com/data/AANobbMI/versions/x/sodium.jar",
             "category": "optimization"}
        ])
    );
    assert!(
        !env.core
            .instance_dir("vacia")
            .unwrap()
            .join("content.json")
            .exists()
    );

    // Backups: enlazados (mismo inodo) y convertidos.
    let bdir = env.data.join("backups/principal");
    for name in ["backup-1700000100.zip", "backup-1700000200.zip"] {
        assert_eq!(
            ino(&bdir.join(name)),
            ino(&src.join("backups").join(name)),
            "{name}"
        );
    }
    let idx: Value = serde_json::from_slice(&fs::read(bdir.join("backups.json")).unwrap()).unwrap();
    assert_eq!(
        idx,
        json!({"schema": 1, "entries": [
            {"id": "backup-1700000100", "created_at": 1_700_000_150, "size_bytes": 11,
             "pinned": true, "label": "antes de actualizar", "automatic": false},
            {"id": "backup-1700000200", "created_at": 1_700_000_260, "size_bytes": 12,
             "pinned": false, "label": null, "automatic": true}
        ]})
    );
    assert!(!env.data.join("backups/vacia").exists());

    // Contenido compartido: enlaces duros.
    let g = env.roxy.join("game");
    for (rel_src, rel_dst) in [
        (
            "libraries/org/lib/1.0/lib-1.0.jar",
            "libraries/org/lib/1.0/lib-1.0.jar",
        ),
        (
            "libraries/net/other/2.0/other-2.0.jar",
            "libraries/net/other/2.0/other-2.0.jar",
        ),
        ("assets/indexes/32.json", "assets/indexes/32.json"),
        (
            "assets/objects/ab/abcdef0123",
            "assets/objects/ab/abcdef0123",
        ),
        ("versions/1.20.1/1.20.1.jar", "versions/1.20.1/1.20.1.jar"),
    ] {
        assert_eq!(
            ino(&g.join(rel_src)),
            ino(&env.data.join(rel_dst)),
            "{rel_dst}"
        );
    }
    assert!(!env.data.join("assets/skins").exists());
    assert!(!env.data.join("versions/1.20.1/extra.txt").exists());
    assert!(!env.data.join("natives").exists());
    // Java: copia, no enlace.
    let java = env.data.join("java/25/bin/java");
    assert_eq!(fs::read(&java).unwrap(), b"java-binary");
    assert_ne!(ino(&java), ino(&g.join("java/25/bin/java")));
    assert!(
        !env.data.join("java").read_dir().unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with('.'))
    );

    assert_eq!(report.linked_files, 2 + 2 + 2 + 1);
    assert!(report.copied_files > 0);

    // El secreto centinela no aparece en ningún archivo ni nombre del destino.
    for path in files(&env.data) {
        let bytes = fs::read(&path).unwrap();
        assert!(
            !bytes
                .windows(SENTINEL.len())
                .any(|w| w == SENTINEL.as_bytes()),
            "centinela en {}",
            path.display()
        );
    }
    assert!(!tree(&env.data).iter().any(|p| p.contains("session.json")
        || p.contains("launcher_account")
        || p.contains("latest_launch")));

    // El origen sigue intacto.
    assert!(src.join("latest_launch.log").exists());
    assert!(src.join("instance.json").exists());
}

#[tokio::test]
async fn segunda_ejecucion_no_importa_nada_y_force_reimporta() {
    let env = env().await;
    let first = env.core.import_roxy(&opts(&env)).await.unwrap();
    assert_eq!(first.imported, ["principal", "vacia"]);
    let snapshot = tree(&env.data);

    let second = env.core.import_roxy(&opts(&env)).await.unwrap();
    assert!(second.imported.is_empty());
    assert_eq!(second.linked_files + second.copied_files, 0);
    assert_eq!(
        tree(&env.data),
        snapshot,
        "la segunda ejecución modificó el destino"
    );
    let reasons: Vec<_> = second
        .plan
        .instances
        .iter()
        .map(|i| (i.new_id.as_str(), i.skipped_reason.is_some()))
        .collect();
    assert_eq!(
        reasons,
        [("principal", true), ("vacia", true), ("", true), ("", true)]
    );
    // Lo compartido ya está: nada que hacer.
    assert_eq!(second.plan.shared.libraries_files, 0);
    assert!(second.plan.shared.java_majors.is_empty());

    let mut forced = opts(&env);
    forced.force = true;
    let third = env.core.import_roxy(&forced).await.unwrap();
    assert_eq!(third.imported, ["principal-2", "vacia-2"]);
    let again = env.core.get_instance("principal-2").await.unwrap();
    assert_eq!(
        again.imported_from.unwrap().original_id,
        "principal-1700000000"
    );
    assert!(
        env.core
            .instance_game_dir("principal-2")
            .unwrap()
            .join("mods/sodium.jar")
            .exists()
    );
    assert!(env.data.join("backups/principal-2/backups.json").exists());
    assert_eq!(env.core.list_instances().await.unwrap().len(), 4);
}

#[tokio::test]
async fn papelera_solo_con_include_trash() {
    let env = env().await;
    let plan = env.core.plan_roxy_import(&opts(&env)).await.unwrap();
    let vieja = plan
        .instances
        .iter()
        .find(|i| i.original_id == "vieja-1600000000")
        .unwrap();
    assert!(
        vieja
            .skipped_reason
            .as_deref()
            .unwrap()
            .contains("papelera")
    );

    let mut o = opts(&env);
    o.include_trash = true;
    let report = env.core.import_roxy(&o).await.unwrap();
    assert_eq!(report.imported, ["principal", "vacia", "vieja"]);
    assert!(
        env.core
            .instance_game_dir("vieja")
            .unwrap()
            .join("mods/old.jar")
            .exists()
    );
}

#[tokio::test]
async fn opciones_skip_backups_logs_y_sin_compartido() {
    let env = env().await;
    let mut o = opts(&env);
    o.skip_backups = true;
    o.skip_logs = true;
    o.link_shared = false;
    let report = env.core.import_roxy(&o).await.unwrap();
    assert_eq!(report.imported, ["principal", "vacia"]);
    assert_eq!(report.plan.instances[0].backups, 0);

    let game = env.core.instance_game_dir("principal").unwrap();
    assert!(!game.join("logs").exists() && !game.join("crash-reports").exists());
    assert!(game.join("mods/sodium.jar").exists());
    assert!(!env.data.join("backups").exists());
    for dir in ["libraries", "assets", "versions", "java"] {
        assert!(!env.data.join(dir).exists(), "{dir}");
    }
}

#[tokio::test]
async fn origen_inexistente_o_solapado_falla() {
    let env = env().await;
    let mut o = opts(&env);
    o.source_root = env.roxy.join("no-existe");
    assert_eq!(
        env.core.import_roxy(&o).await.unwrap_err().kind(),
        "invalid_input"
    );

    // Origen que contiene al directorio de datos.
    fs::create_dir_all(env.data.join("game")).unwrap();
    o.source_root = env.data.clone();
    assert_eq!(
        env.core.import_roxy(&o).await.unwrap_err().kind(),
        "invalid_input"
    );
}
