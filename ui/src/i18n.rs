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

/// Separador decimal del idioma (`NumberFormat` de intl).
pub fn decimal(lang: &str) -> &'static str {
    if lang == "es" { "," } else { "." }
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

    /// Claves de `@tr("…")` en todos los .slint (las de plural aparecen dos
    /// veces con la misma clave).
    fn tr_keys(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                tr_keys(&p, out);
            } else if p.extension().is_some_and(|x| x == "slint") {
                let src = std::fs::read_to_string(&p).unwrap();
                for (i, _) in src.match_indices("@tr(\"") {
                    let rest = &src[i + 5..];
                    let key = &rest[..rest.find('"').unwrap()];
                    out.push((p.display().to_string(), key.to_owned()));
                }
            }
        }
    }

    #[test]
    fn toda_clave_de_la_ui_existe_en_los_dos_arb() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("slint");
        let mut keys = Vec::new();
        tr_keys(&dir, &mut keys);
        assert!(keys.len() > 30, "solo {} claves: ¿cambió la ruta?", keys.len());
        for lang in LANGS {
            for (file, key) in &keys {
                // `<clave>.zero` sale del caso `=0` de un plural.
                let base = key.strip_suffix(".zero").unwrap_or(key);
                assert!(table(lang).contains_key(base), "{file}: «{key}» no está en app_{lang}.arb");
                if key.ends_with(".zero") {
                    assert!(table(lang)[base].contains("=0{"), "{file}: «{base}» no tiene caso =0");
                }
            }
        }
    }

    #[test]
    fn textos_de_rust() {
        assert_eq!(text("es", "loaderCustom"), "Personalizado");
        assert_eq!(text("en", "loaderCustom"), "Custom");
    }
}
