//! Idioma de la UI. Los textos de Slint van con `@tr` y catálogos generados
//! desde los ARB (build.rs); aquí solo se elige el idioma y se leen las pocas
//! cadenas que hace falta tener en Rust (búsqueda, errores).

use std::collections::HashMap;
use std::sync::OnceLock;

/// Idiomas con traducción. El primero es el de reserva (como `app.dart`).
pub const LANGS: [&str; 2] = ["es", "en"];

/// `es` o `en` a partir del ajuste del núcleo o del idioma del sistema.
pub fn resolve(setting: Option<&str>, system: Option<&str>) -> &'static str {
    let pick = |s: &str| {
        let code = s.split(['_', '.', '-', '@']).next().unwrap_or("").to_ascii_lowercase();
        LANGS.iter().copied().find(|l| *l == code)
    };
    setting.and_then(pick).or_else(|| system.and_then(pick)).unwrap_or(LANGS[0])
}

/// Idioma del sistema según las variables POSIX (en Windows/macOS, `LANG`
/// suele no estar: queda el de reserva hasta la fase de plataforma).
pub fn system_locale() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"].iter().find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
}

/// Cambia el idioma de todos los `@tr` de la UI.
pub fn select(lang: &str) {
    if let Err(e) = slint::select_bundled_translation(lang) {
        tracing::warn!(lang, error = %e, "no se pudo elegir el idioma");
    }
}

fn table(lang: &str) -> &'static HashMap<String, String> {
    static ES: OnceLock<HashMap<String, String>> = OnceLock::new();
    static EN: OnceLock<HashMap<String, String>> = OnceLock::new();
    let (cell, src) = match lang {
        "en" => (&EN, include_str!("../../app/lib/l10n/app_en.arb")),
        _ => (&ES, include_str!("../../app/lib/l10n/app_es.arb")),
    };
    cell.get_or_init(|| crate::arb::parse_arb(src).unwrap_or_default().into_iter().collect())
}

/// Texto simple (sin argumentos ni plurales) de una clave del ARB.
pub fn text(lang: &str, key: &str) -> String {
    table(lang).get(key).cloned().unwrap_or_else(|| key.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idioma_del_ajuste_luego_sistema_luego_espanol() {
        assert_eq!(resolve(Some("en"), Some("es_ES.UTF-8")), "en");
        assert_eq!(resolve(None, Some("en_US.UTF-8")), "en");
        assert_eq!(resolve(None, Some("fr_FR.UTF-8")), "es");
        assert_eq!(resolve(Some("de"), None), "es");
        assert_eq!(resolve(None, None), "es");
    }

    #[test]
    fn textos_de_rust() {
        assert_eq!(text("es", "loaderCustom"), "Personalizado");
        assert_eq!(text("en", "loaderCustom"), "Custom");
    }
}
