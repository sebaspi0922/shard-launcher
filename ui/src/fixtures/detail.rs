//! Datos de ejemplo de fake_bridge.dart: instancias completas
//! (`fakeInstance` con versión del loader, memoria, argumentos, ventana y
//! carpeta de juego). Fase 3.

use std::path::PathBuf;

use lcore::instance::{GameVersion, INSTANCE_SCHEMA, Instance, JavaSettings, LoaderKind, WindowSettings};

use super::sample_instances;
use crate::instance::Detail;

/// `fakeInstance(...)` de `id` tal como lo da `sampleInstances()`.
pub fn sample_detail(id: &str) -> Option<Detail> {
    let r = sample_instances().into_iter().find(|r| r.id == id)?;
    let loader = LoaderKind::parse(&r.loader).unwrap_or(LoaderKind::Vanilla);
    let (loader_version, java, window) = match id {
        "survival-amigos" => (
            Some("0.16.10".to_owned()),
            JavaSettings {
                path: None,
                memory_mb: Some(6144),
                jvm_args: Some(vec!["-XX:+UseG1GC".into(), "-XX:MaxGCPauseMillis=50".into()]),
            },
            WindowSettings { width: Some(1600), height: Some(900), fullscreen: false },
        ),
        "create-aeronautica" => (Some("47.3.0".to_owned()), JavaSettings::default(), WindowSettings::default()),
        _ => (None, JavaSettings::default(), WindowSettings::default()),
    };
    let instance = Instance {
        schema: INSTANCE_SCHEMA,
        id: r.id.clone(),
        name: r.name,
        game: GameVersion { mc: r.mc, loader, loader_version },
        created_at: r.created_at,
        last_played: r.last_played,
        playtime_seconds: r.playtime_seconds,
        java,
        window,
        // F3 copias: `backupSettings` de FakeBridge.
        backups: super::backups::sample_settings(),
        dev_console: false,
        imported_from: None,
    };
    Some(Detail { instance, game_dir: PathBuf::from(format!("/tmp/shard-test/instances/{id}/minecraft")) })
}
