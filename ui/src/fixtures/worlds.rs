//! Datos de ejemplo de fake_bridge.dart: mundos (sampleWorlds) y datapacks
//! (sampleDatapacks, los de «Mundo de verano»). Fase 3.

use lcore::world::{DatapackInfo, Difficulty, GameMode, WorldInfo};

use super::{D, H, NOW};

const MB: u64 = 1024 * 1024;

/// `rules` = (modo, dificultad, hardcore, trucos).
fn world(folder: &str, name: &str, rules: (GameMode, Difficulty, bool, bool), ago: i64, version: &str, mb: u64) -> WorldInfo {
    let (mode, diff, hardcore, cheats) = rules;
    WorldInfo {
        folder: folder.into(),
        name: name.into(),
        game_mode: Some(mode),
        difficulty: Some(diff),
        hardcore: Some(hardcore),
        cheats: Some(cheats),
        last_played_ms: Some((NOW - ago) * 1000),
        version_name: Some(version.into()),
        data_version: None,
        size_bytes: mb * MB,
        icon: None,
        error: None,
    }
}

/// `sampleWorlds()` (de `survival-amigos`).
pub fn sample_worlds() -> Vec<WorldInfo> {
    vec![
        world("Mundo de verano", "Mundo de verano", (GameMode::Survival, Difficulty::Normal, false, false), 2 * H, "1.21.4", 842),
        world("Creativo-pruebas", "Creativo de pruebas", (GameMode::Creative, Difficulty::Peaceful, false, true), 5 * D, "1.21.4", 96),
        world("Hardcore 2", "Hardcore 2", (GameMode::Survival, Difficulty::Hard, true, false), 40 * D, "1.21.1", 311),
    ]
}

/// `sampleDatapacks()`.
pub fn sample_datapacks() -> Vec<DatapackInfo> {
    let p = |name: &str, is_dir, enabled, size_bytes| DatapackInfo { name: name.into(), is_dir, enabled, size_bytes };
    vec![
        p("Terralith_1.21_v2.5.8.zip", false, true, 2_900_000),
        p("mas-aldeas", true, true, 410_000),
        p("sin-phantoms.zip", false, false, 12_000),
    ]
}
