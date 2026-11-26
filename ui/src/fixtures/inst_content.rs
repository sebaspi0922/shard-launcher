//! Datos de ejemplo de fake_bridge.dart: contenido instalado
//! (`sampleContent`), actualizaciones (`sampleUpdates`) e iconos de
//! `app/test/fixtures/icons` (los que sirve el puente falso). Fase 3.

use std::path::PathBuf;

use super::Recorder;
use crate::inst_content::{ContentActions, ICON_PX, InstContent, Item, Update, decode_icon, key};

/// `kFixtureIconBase`: URL de los iconos de ejemplo.
pub const ICON_BASE: &str = "https://cdn.modrinth.com/data/test/";

/// `_c(...)`: `kb` en KiB; sin fuente conocida, sin proyecto.
fn c(kind: &str, file: &str, source: &str, name: Option<&str>, version: Option<&str>, kb: u64, enabled: bool) -> Item {
    Item {
        kind: kind.into(),
        file: file.into(),
        enabled,
        source: source.into(),
        project_name: name.map(Into::into),
        version_number: version.map(Into::into),
        size: kb * 1024,
        is_dir: false,
    }
}

/// `sampleContent()` (de `survival-amigos`).
pub fn sample_content() -> Vec<Item> {
    let m = "modrinth";
    vec![
        c("mod", "fabric-api-0.119.2+1.21.4.jar", m, Some("Fabric API"), Some("0.119.2+1.21.4"), 2210, true),
        c("mod", "sodium-fabric-0.6.13+mc1.21.4.jar", m, Some("Sodium"), Some("0.6.13+mc1.21.4"), 1180, true),
        c("mod", "iris-fabric-1.8.8+mc1.21.4.jar", m, Some("Iris Shaders"), Some("1.8.8+mc1.21.4"), 2780, true),
        c("mod", "lithium-fabric-0.14.7+mc1.21.4.jar", m, Some("Lithium"), Some("0.14.7"), 760, true),
        c("mod", "journeymap-1.21.4-6.0.0.jar", "curseforge", Some("JourneyMap"), Some("6.0.0"), 6020, true),
        c("mod", "appleskin-fabric-mc1.21.4-3.0.6.jar", m, Some("AppleSkin"), Some("3.0.6"), 120, false),
        c("mod", "mi-mod-casero.jar", "unknown", None, None, 48, true),
        c("resourcepack", "FreshAnimations_v1.9.3.zip", m, Some("Fresh Animations"), Some("1.9.3"), 5400, true),
        c("shader", "ComplementaryReimagined_r5.4.zip", m, Some("Complementary Shaders - Reimagined"), Some("r5.4"), 900, true),
    ]
}

/// `sampleUpdates()`.
pub fn sample_updates() -> Vec<Update> {
    vec![
        Update {
            kind: "mod".into(),
            file: "sodium-fabric-0.6.13+mc1.21.4.jar".into(),
            project_name: Some("Sodium".into()),
            latest: "0.6.14+mc1.21.4".into(),
        },
        Update {
            kind: "resourcepack".into(),
            file: "FreshAnimations_v1.9.3.zip".into(),
            project_name: Some("Fresh Animations".into()),
            latest: "1.9.4".into(),
        },
    ]
}

/// `_fixtureIconSlug`: proyecto de ejemplo de un archivo (los desconocidos no
/// tienen icono).
fn icon_slug(file: &str) -> Option<&'static str> {
    let f = file.to_lowercase();
    for slug in ["fabric-api", "sodium", "iris", "lithium", "journeymap", "appleskin"] {
        if f.starts_with(slug) {
            return Some(slug);
        }
    }
    if f.starts_with("freshanimations") {
        return Some("fresh-animations");
    }
    if f.starts_with("complementaryreimagined") {
        return Some("complementary-reimagined");
    }
    None
}

/// `contentIcons` del puente falso: `(key, url)` de lo que tiene icono.
pub fn sample_icons(items: &[Item]) -> Vec<(String, String)> {
    items.iter().filter_map(|c| icon_slug(&c.file).map(|s| (key(&c.kind, &c.file), format!("{ICON_BASE}{s}.png")))).collect()
}

/// `fixtureIconPath`: el PNG de `app/test/fixtures/icons` de una URL de
/// ejemplo.
pub fn icon_path(url: &str) -> Option<PathBuf> {
    let name = url.strip_prefix(ICON_BASE)?;
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../app/test/fixtures/icons").join(name);
    p.exists().then_some(p)
}

/// Entrega a la pestaña de `id` todo lo del puente falso (lista,
/// actualizaciones, iconos y sus imágenes), como si el núcleo respondiera.
pub fn deliver(c: &InstContent, id: &str) {
    let items = sample_content();
    let icons = sample_icons(&items);
    c.listed(id, Ok(items));
    c.updates_loaded(id, Ok(sample_updates()));
    c.icons_loaded(id, icons.clone());
    for (_, url) in icons {
        let px = icon_path(&url).and_then(|p| std::fs::read(p).ok()).and_then(|b| decode_icon(&b, ICON_PX));
        c.icon_loaded(&url, px);
    }
}

impl ContentActions for Recorder {
    fn list(&self, id: &str) {
        self.push(format!("content-list {id}"));
    }
    fn icons(&self, id: &str) {
        self.push(format!("content-icons {id}"));
    }
    fn icon_image(&self, url: &str) {
        self.push(format!("content-icon {url}"));
    }
    fn check_updates(&self, id: &str) {
        self.push(format!("content-updates {id}"));
    }
    fn set_enabled(&self, id: &str, key: &str, enabled: bool) {
        self.push(format!("content-enable {id} {key} {enabled}"));
    }
    fn remove(&self, id: &str, key: &str, name: &str) {
        self.push(format!("content-remove {id} {key} {name}"));
    }
    fn update(&self, id: &str, key: &str, name: &str) {
        self.push(format!("content-update {id} {key} {name}"));
    }
    fn update_all(&self, id: &str, keys: &[String]) {
        self.push(format!("content-update-all {id} {}", keys.join(",")));
    }
    fn identify(&self, id: &str) {
        self.push(format!("content-identify {id}"));
    }
    fn browse(&self, id: &str, kind: &str) {
        self.push(format!("content-browse {id} {kind}"));
    }
}
