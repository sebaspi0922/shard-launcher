//! Datos falsos de `app/test/support/fake_bridge.dart` (los de los goldens),
//! para el arnés de paridad y los tests de interacción. Mismos ids, nombres,
//! versiones, tiempos y fechas relativas a `NOW`.

use crate::instances::Record;

/// «Ahora» de las escenas. Solo importan las diferencias con él.
pub const NOW: i64 = 1_791_822_600;

const H: i64 = 3600;
const D: i64 = 86_400;

fn instance(id: &str, name: &str, mc: &str, loader: &str, ago: Option<i64>, minutes: u64) -> Record {
    Record {
        id: id.into(),
        name: name.into(),
        mc: mc.into(),
        loader: loader.into(),
        created_at: NOW - 200 * D,
        last_played: ago.map(|a| NOW - a),
        playtime_seconds: minutes * 60,
    }
}

/// `sampleInstances()`.
pub fn sample_instances() -> Vec<Record> {
    vec![
        instance("survival-amigos", "Survival con amigos", "1.21.4", "fabric", Some(2 * H), 86 * 60 + 12),
        instance("create-aeronautica", "Create: Aeronáutica", "1.20.1", "forge", Some(3 * D), 41 * 60),
        instance("vanilla-puro", "Vanilla puro", "1.21.4", "vanilla", Some(D + 2 * H), 640),
        instance("skyblock-tecnico", "Skyblock técnico", "1.21.1", "neoforge", Some(64 * D), 19 * 60 + 45),
        instance("quilt-pruebas", "Pruebas de shaders", "1.21.5", "quilt", None, 0),
    ]
}

/// Cuenta activa de `sampleAccounts()` (Microsoft, sin skin: cara generada).
pub const ACTIVE_ACCOUNT: &str = "NadiaK";

/// Escribe con `lcore` las instancias de ejemplo en una carpeta de datos
/// (fechas relativas a la hora real), con «cerrar a la bandeja» y español.
/// Flutter lee la misma carpeta (`SHARD_DATA_DIR`): así `tools/medir.sh`
/// mide las dos apps con los mismos datos.
pub async fn seed(core: &lcore::Core) -> lcore::Result<()> {
    use lcore::instance::{GameVersion, INSTANCE_SCHEMA, Instance, LoaderKind};
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    for r in sample_instances() {
        let loader = match r.loader.as_str() {
            "fabric" => LoaderKind::Fabric,
            "forge" => LoaderKind::Forge,
            "neoforge" => LoaderKind::NeoForge,
            "quilt" => LoaderKind::Quilt,
            _ => LoaderKind::Vanilla,
        };
        let i = Instance {
            schema: INSTANCE_SCHEMA,
            id: r.id,
            name: r.name,
            game: GameVersion { mc: r.mc, loader, loader_version: None },
            created_at: r.created_at - NOW + now,
            last_played: r.last_played.map(|t| t - NOW + now),
            playtime_seconds: r.playtime_seconds,
            java: Default::default(),
            window: Default::default(),
            backups: Default::default(),
            dev_console: false,
            imported_from: None,
        };
        core.save_instance(&i).await?;
    }
    core.update_settings(|s| {
        s.close_behavior = lcore::settings::CloseBehavior::Tray;
        s.language = Some("es".into());
    })
    .await?;
    // Flutter: sin primer arranque (tema grafito, el de los goldens).
    std::fs::write(core.paths().root().join("ui.json"), br#"{"firstRunDone":true}"#)
        .map_err(|source| lcore::Error::Io { path: core.paths().root().join("ui.json"), source })?;
    Ok(())
}
