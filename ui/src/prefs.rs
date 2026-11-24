//! Preferencias de la UI en `<datos>/ui.json` (las mismas que lee Flutter:
//! `app/rust/src/api/ui_prefs.rs`). De momento solo `performance`.

use std::path::Path;

/// Tamaño máximo que se lee (como el bridge).
const MAX_BYTES: u64 = 256 * 1024;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UiPrefs {
    /// Modo rendimiento: fondo quieto y sin animaciones.
    pub performance: bool,
}

pub fn parse(json: &str) -> UiPrefs {
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    UiPrefs { performance: v.get("performance").and_then(|b| b.as_bool()).unwrap_or(false) }
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
    #[test]
    fn modo_rendimiento() {
        assert!(super::parse(r#"{"firstRunDone":true,"performance": true}"#).performance);
        assert!(!super::parse(r#"{"performance":false}"#).performance);
        assert!(!super::parse(r#"{"firstRunDone":true}"#).performance);
        assert!(!super::parse("no es json").performance);
    }
}
