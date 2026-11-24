//! Datos: `lcore` directo, sin bridge. `seed` escribe en una carpeta de datos
//! las 5 instancias de `app/test/support/fake_bridge.dart` (mismas fechas
//! relativas y tiempos) para que Flutter y GPUI lean lo mismo.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lcore::instance::{GameVersion, Instance, LoaderKind};
use lcore::{Core, Options};

/// Lo que pinta una tarjeta.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub mc: String,
    pub loader: String,
    pub playtime_seconds: u64,
    pub last_played: Option<i64>,
}

impl From<Instance> for Card {
    fn from(i: Instance) -> Self {
        Self {
            id: i.id,
            name: i.name,
            mc: i.game.mc,
            loader: i.game.loader.as_str().to_owned(),
            playtime_seconds: i.playtime_seconds,
            last_played: i.last_played,
        }
    }
}

pub fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

async fn open(dir: &Path) -> lcore::Result<Core> {
    Core::open(Options { data_dir: Some(dir.to_path_buf()), offline: true, ..Default::default() }).await
}

/// `performance` de `<datos>/ui.json` (preferencia de la UI, no del núcleo):
/// fondo quieto, como el modo rendimiento de Flutter.
pub fn performance_mode(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("ui.json"))
        .ok()
        .and_then(|s| serde_json_lite_bool(&s, "performance"))
        .unwrap_or(false)
}

/// Lee un booleano de primer nivel sin traer serde_json solo para esto.
fn serde_json_lite_bool(json: &str, key: &str) -> Option<bool> {
    let rest = &json[json.find(&format!("\"{key}\""))? + key.len() + 2..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start();
    if rest.starts_with("true") { Some(true) } else if rest.starts_with("false") { Some(false) } else { None }
}

pub async fn load(dir: PathBuf) -> lcore::Result<Vec<Card>> {
    let core = open(&dir).await?;
    Ok(core.list_instances().await?.into_iter().map(Card::from).collect())
}

/// (id, nombre, mc, loader, hace cuántos segundos, minutos jugados)
const FIXTURES: [(&str, &str, &str, LoaderKind, Option<i64>, u64); 5] = [
    ("survival-amigos", "Survival con amigos", "1.21.4", LoaderKind::Fabric, Some(2 * 3600), 86 * 60 + 12),
    ("create-aeronautica", "Create: Aeronáutica", "1.20.1", LoaderKind::Forge, Some(3 * 86400), 41 * 60),
    ("vanilla-puro", "Vanilla puro", "1.21.4", LoaderKind::Vanilla, Some(86400 + 2 * 3600), 640),
    ("skyblock-tecnico", "Skyblock técnico", "1.21.1", LoaderKind::NeoForge, Some(64 * 86400), 19 * 60 + 45),
    ("quilt-pruebas", "Pruebas de shaders", "1.21.5", LoaderKind::Quilt, None, 0),
];

pub async fn seed(dir: PathBuf) -> lcore::Result<()> {
    let core = open(&dir).await?;
    let now = now_secs();
    for (id, name, mc, loader, ago, minutes) in FIXTURES {
        let mut i = Instance {
            schema: lcore::instance::INSTANCE_SCHEMA,
            id: id.into(),
            name: name.into(),
            game: GameVersion { mc: mc.into(), loader, loader_version: None },
            created_at: now - 200 * 86400,
            last_played: ago.map(|a| now - a),
            playtime_seconds: minutes * 60,
            java: Default::default(),
            window: Default::default(),
            backups: Default::default(),
            dev_console: false,
            imported_from: None,
        };
        i.java.memory_mb = None;
        core.save_instance(&i).await?;
    }
    // Cerrar = a la bandeja, español: así Flutter también crea su icono de
    // bandeja al arrancar (si no, solo lo crea al mandarse a segundo plano).
    core.update_settings(|s| {
        s.close_behavior = lcore::settings::CloseBehavior::Tray;
        s.language = Some("es".into());
    })
    .await?;
    // Flutter: sin primer arranque, tema grafito (el de los goldens).
    std::fs::write(dir.join("ui.json"), br#"{"firstRunDone":true}"#).expect("ui.json");
    Ok(())
}

// --- Formato (core/format.dart + app_es.arb) ---

pub fn loader_name(loader: &str) -> &'static str {
    match loader {
        "vanilla" => "Vanilla",
        "fabric" => "Fabric",
        "quilt" => "Quilt",
        "forge" => "Forge",
        "neoforge" => "NeoForge",
        "labymod" => "LabyMod",
        _ => "Personalizado",
    }
}

pub fn loader_color(loader: &str) -> u32 {
    match loader {
        "vanilla" => 0x6CCB5F,
        "fabric" => 0xDBC59C,
        "quilt" => 0xB28CF5,
        "forge" => 0x8FA6C9,
        "neoforge" => 0xF0954A,
        "labymod" => 0x5BB0F0,
        _ => 0xA0A0A0,
    }
}

pub fn playtime(seconds: u64) -> String {
    let (h, m) = (seconds / 3600, (seconds % 3600) / 60);
    match (h, m) {
        (0, 0) => "0 min".into(),
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, _) if h >= 100 => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

pub fn relative(at: Option<i64>, now: i64) -> String {
    let Some(at) = at else { return "Nunca".into() };
    let d = now - at;
    let (mins, hours, days) = (d / 60, d / 3600, d / 86400);
    if mins < 1 {
        "ahora mismo".into()
    } else if hours < 1 {
        format!("hace {mins} min")
    } else if days < 1 {
        if hours == 1 { "hace 1 hora".into() } else { format!("hace {hours} horas") }
    } else if days < 30 {
        if days == 1 { "ayer".into() } else { format!("hace {days} días") }
    } else if days < 365 {
        let m = days / 30;
        if m == 1 { "hace 1 mes".into() } else { format!("hace {m} meses") }
    } else {
        let y = days / 365;
        if y == 1 { "hace 1 año".into() } else { format!("hace {y} años") }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textos_como_el_golden() {
        let now = 1_000_000_000;
        assert_eq!(playtime((86 * 60 + 12) * 60), "86 h 12 min");
        assert_eq!(playtime(41 * 3600), "41 h");
        assert_eq!(playtime(640 * 60), "10 h 40 min");
        assert_eq!(playtime(0), "0 min");
        assert_eq!(relative(Some(now - 2 * 3600), now), "hace 2 horas");
        assert_eq!(relative(Some(now - 86400 - 7200), now), "ayer");
        assert_eq!(relative(Some(now - 3 * 86400), now), "hace 3 días");
        assert_eq!(relative(Some(now - 64 * 86400), now), "hace 2 meses");
        assert_eq!(relative(None, now), "Nunca");
    }

    #[test]
    fn modo_rendimiento_de_ui_json() {
        assert_eq!(serde_json_lite_bool(r#"{"firstRunDone":true,"performance": true}"#, "performance"), Some(true));
        assert_eq!(serde_json_lite_bool(r#"{"performance":false}"#, "performance"), Some(false));
        assert_eq!(serde_json_lite_bool(r#"{"firstRunDone":true}"#, "performance"), None);
    }

    #[tokio::test]
    async fn sembrar_y_leer_con_lcore() {
        let dir = std::env::temp_dir().join(format!("shard-gpui-test-{}", std::process::id()));
        seed(dir.clone()).await.unwrap();
        let cards = load(dir.clone()).await.unwrap();
        let names: Vec<_> = cards.iter().map(|c| c.name.as_str()).collect();
        // Mismo orden que el golden: última partida primero, «Nunca» al final.
        assert_eq!(
            names,
            ["Survival con amigos", "Vanilla puro", "Create: Aeronáutica", "Skyblock técnico", "Pruebas de shaders"]
        );
        std::fs::remove_dir_all(dir).ok();
    }
}
