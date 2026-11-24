//! Compila la UI Slint y empaqueta las traducciones, que salen de los ARB de
//! la app Flutter (`app/lib/l10n`) para no tener dos fuentes de textos.

#[path = "src/arb.rs"]
#[allow(dead_code)]
mod arb;

use std::path::PathBuf;

const LANGS: [&str; 2] = ["es", "en"];

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let l10n = manifest.join("../app/lib/l10n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("i18n");
    let template = std::fs::read_to_string(l10n.join("app_es.arb")).expect("app_es.arb");
    for lang in LANGS {
        let src = l10n.join(format!("app_{lang}.arb"));
        println!("cargo:rerun-if-changed={}", src.display());
        let arb = std::fs::read_to_string(&src).expect("ARB");
        let po = arb::to_po(&arb, &template).unwrap_or_else(|e| panic!("{}: {e}", src.display()));
        let dir = out.join(lang).join("LC_MESSAGES");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("shard-ui.po"), po).unwrap();
    }
    // Información de depuración para `ElementHandle` (arnés de paridad y
    // tests de interacción). Solo en debug: no pesa en release.
    let debug = std::env::var("PROFILE").as_deref() == Ok("debug");
    let config = slint_build::CompilerConfiguration::new()
        .with_style("fluent-dark".into())
        .with_bundled_translations(out)
        .with_default_translation_context(slint_build::DefaultTranslationContext::None)
        .with_debug_info(debug);
    slint_build::compile_with_config("slint/app.slint", config).expect("UI Slint");
}
