//! Datos de ejemplo de fake_bridge.dart: capturas (sampleScreenshots). Fase 3.

use std::path::PathBuf;

use lcore::world::ScreenshotInfo;

use super::{H, NOW};

/// `kTestNow` es `DateTime(2026, 10, 11, 18, 30)` en hora local: con este
/// desfase, `NOW` se lee como esa hora y las fechas salen como en los
/// goldens («11 oct 2026 13:30»).
pub const UTC_OFFSET: i64 = 1_791_743_400 - NOW;

/// `sampleScreenshots()`: nueve capturas, la primera de hace 5 h y luego
/// cada 19 h. Las rutas no existen: se ven los paisajes.
pub fn sample_screenshots() -> Vec<ScreenshotInfo> {
    (0..9)
        .map(|i| ScreenshotInfo {
            file: format!("2026-10-{:02}_18.0{i}.21.png", 11 - i),
            path: PathBuf::from(format!("/tmp/shard-test/screenshots/shot-{i}.png")),
            size: 2_400_000 + i as u64 * 10_000,
            modified: Some(NOW - (5 + 19 * i) * H),
        })
        .collect()
}
