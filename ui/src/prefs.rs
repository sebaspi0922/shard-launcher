//! Preferencias de la UI en `<datos>/ui.json` (las mismas que lee y escribe
//! Flutter: `app/lib/core/ui_prefs.dart`). Se leen al arrancar.

use std::path::{Path, PathBuf};

use crate::theme::Style;

/// Tamaño máximo que se lee (como el bridge).
const MAX_BYTES: u64 = 256 * 1024;

/// De dónde sale el color de acento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccentMode {
    /// El propio del estilo.
    #[default]
    Style,
    /// Extraído de la imagen de fondo.
    Auto,
    /// Uno de los presets.
    Preset,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiPrefs {
    pub style: Style,
    pub accent: AccentMode,
    /// ARGB del preset elegido.
    pub accent_preset: u32,
    /// Imagen propia elegida como fondo (`background == "image"`).
    pub background_image: bool,
    /// Copia de la imagen de fondo dentro de la carpeta de datos.
    pub background_path: Option<PathBuf>,
    /// Desenfoque del fondo (0–40).
    pub blur: f64,
    /// Oscurecido del fondo (0–0,85).
    pub dim: f64,
    /// Modo rendimiento: sin desenfoque ni animaciones de fondo.
    pub performance: bool,
    /// `None` = nunca se marcó.
    pub first_run_done: Option<bool>,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            style: Style::Graphite,
            accent: AccentMode::Style,
            accent_preset: 0xFF35D69B,
            background_image: false,
            background_path: None,
            blur: 12.0,
            dim: 0.45,
            performance: false,
            first_run_done: None,
        }
    }
}

impl UiPrefs {
    /// Imagen propia elegida (`usesImage`).
    pub fn uses_image(&self) -> Option<&Path> {
        if self.background_image { self.background_path.as_deref() } else { None }
    }
}

/// `UiPrefs.parse`: archivo dañado = valores por defecto.
pub fn parse(json: &str) -> UiPrefs {
    let d = UiPrefs::default();
    let Ok(serde_json::Value::Object(j)) = serde_json::from_str::<serde_json::Value>(json) else {
        return d;
    };
    let number = |k: &str, def: f64, min: f64, max: f64| j.get(k).and_then(|v| v.as_f64()).map(|v| v.clamp(min, max)).unwrap_or(def);
    // Los archivos anteriores a los estilos no tienen `style`: empiezan con
    // el acento del estilo y conservan su imagen.
    let legacy = !j.contains_key("style");
    UiPrefs {
        style: Style::parse(j.get("style").and_then(|v| v.as_str()).unwrap_or("")),
        accent: match j.get("accent").and_then(|v| v.as_str()) {
            _ if legacy => AccentMode::Style,
            Some("auto") => AccentMode::Auto,
            Some("preset") => AccentMode::Preset,
            _ => AccentMode::Style,
        },
        accent_preset: j.get("accent_preset").and_then(|v| v.as_i64()).map(|v| v as u32).unwrap_or(d.accent_preset),
        background_image: j.get("background").and_then(|v| v.as_str()) == Some("image"),
        background_path: j.get("background_path").and_then(|v| v.as_str()).map(PathBuf::from),
        blur: number("blur", d.blur, 0.0, 40.0),
        dim: number("dim", d.dim, 0.0, 0.85),
        performance: j.get("performance").and_then(|b| b.as_bool()) == Some(true),
        first_run_done: j.get("first_run_done").and_then(|b| b.as_bool()),
    }
}

impl UiPrefs {
    /// `UiPrefs.toJson` (mismas claves y valores que escribe Flutter).
    pub fn to_json(&self) -> serde_json::Value {
        let mut j = serde_json::Map::new();
        j.insert("style".into(), self.style.name().into());
        j.insert(
            "accent".into(),
            match self.accent {
                AccentMode::Style => "style",
                AccentMode::Auto => "auto",
                AccentMode::Preset => "preset",
            }
            .into(),
        );
        j.insert("accent_preset".into(), self.accent_preset.into());
        j.insert("background".into(), if self.background_image { "image" } else { "style" }.into());
        if let Some(p) = &self.background_path {
            j.insert("background_path".into(), p.to_string_lossy().into_owned().into());
        }
        j.insert("blur".into(), self.blur.into());
        j.insert("dim".into(), self.dim.into());
        j.insert("performance".into(), self.performance.into());
        if let Some(done) = self.first_run_done {
            j.insert("first_run_done".into(), done.into());
        }
        serde_json::Value::Object(j)
    }
}

/// Guarda `<datos>/ui.json` (archivo temporal y renombrado, como el bridge).
pub fn save(root: &Path, p: &UiPrefs) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(&p.to_json()).map_err(std::io::Error::other)?;
    let path = root.join("ui.json");
    let tmp = root.join(".ui.json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &path)
}

pub fn load(root: &Path) -> UiPrefs {
    let path = root.join("ui.json");
    let ok = std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= MAX_BYTES);
    if !ok {
        return UiPrefs::default();
    }
    std::fs::read_to_string(&path).map(|s| parse(&s)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guardar_y_leer_da_lo_mismo() {
        let dir = std::env::temp_dir().join(format!("shard-prefs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = UiPrefs {
            style: Style::Landscape,
            accent: AccentMode::Preset,
            accent_preset: 0xFF9B7BFF,
            background_image: true,
            background_path: Some("/d/fondo.png".into()),
            blur: 7.5,
            dim: 0.3,
            performance: true,
            first_run_done: Some(true),
        };
        save(&dir, &p).unwrap();
        assert_eq!(load(&dir), p);
        // Sin imagen ni marca de primer arranque, esas claves no se escriben.
        let j = UiPrefs::default().to_json();
        assert!(j.get("background_path").is_none() && j.get("first_run_done").is_none());
        assert_eq!(j["style"], "graphite");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn modo_rendimiento() {
        assert!(parse(r#"{"first_run_done":true,"performance": true}"#).performance);
        assert!(!parse(r#"{"performance":false}"#).performance);
        assert!(!parse(r#"{"first_run_done":true}"#).performance);
        assert!(!parse("no es json").performance);
    }

    #[test]
    fn apariencia_como_flutter() {
        let p = parse(
            r#"{"style":"aurora","accent":"preset","accent_preset":4283410687,"background":"image",
                "background_path":"/d/fondo.png","blur":99,"dim":-1,"first_run_done":true}"#,
        );
        assert_eq!(p.style, Style::Aurora);
        assert_eq!(p.accent, AccentMode::Preset);
        assert_eq!(p.accent_preset, 0xFF4FA8FF);
        assert_eq!(p.uses_image(), Some(Path::new("/d/fondo.png")));
        assert_eq!((p.blur, p.dim), (40.0, 0.0));
        assert_eq!(p.first_run_done, Some(true));
    }

    #[test]
    fn archivo_antiguo_sin_estilo_usa_el_acento_del_estilo() {
        let p = parse(r#"{"accent":"auto","background":"image","background_path":"/x.jpg"}"#);
        assert_eq!(p.accent, AccentMode::Style);
        assert!(p.uses_image().is_some());
        assert_eq!(p.style, Style::Graphite);
        assert_eq!((p.blur, p.dim), (12.0, 0.45));
    }
}
