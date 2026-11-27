//! Datos de ejemplo de fake_bridge.dart: papelera (`sampleTrash`).

use super::{D, H, NOW};
use crate::trash::Trashed;

fn trashed(id: &str, name: &str, mc: &str, loader: &str, ago: i64, playtime_seconds: u64) -> Trashed {
    let deleted_at = NOW - ago;
    Trashed {
        trash_id: format!("{id}~{deleted_at}"),
        id: id.into(),
        name: name.into(),
        mc: mc.into(),
        loader: loader.into(),
        playtime_seconds,
        deleted_at,
    }
}

/// `sampleTrash()`: borradas hace 2 y 19 días.
pub fn sample_trash() -> Vec<Trashed> {
    vec![
        trashed("pruebas-viejas", "Pruebas viejas", "1.20.4", "fabric", 2 * D, 3 * H as u64),
        trashed("hardcore-1", "Hardcore 1", "1.21.1", "vanilla", 19 * D, 41 * 60),
    ]
}
