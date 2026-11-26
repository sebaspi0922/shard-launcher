//! Datos de ejemplo de fake_bridge.dart: copias (`sampleBackups`) y sus
//! ajustes (`backupSettings`). Fase 3.

use lcore::backup::BackupEntry;
use lcore::instance::BackupSettings;

use super::NOW;

const H: i64 = 3600;
const D: i64 = 86_400;
const MB: u64 = 1024 * 1024;

/// Desfase con UTC con el que `NOW` es el `kTestNow` de los tests de
/// Flutter (11 oct 2026, 18:30 en la hora local): así las fechas absolutas
/// salen iguales que en los goldens.
pub const UTC_OFFSET: i32 = -22 * 3600;

fn entry(id: &str, ago: i64, mb: u64, pinned: bool, label: Option<&str>, automatic: bool) -> BackupEntry {
    BackupEntry { id: id.into(), created_at: NOW - ago, size_bytes: mb * MB, pinned, label: label.map(Into::into), automatic }
}

/// `sampleBackups()` (de la más reciente a la más antigua).
pub fn sample_backups() -> Vec<BackupEntry> {
    vec![
        entry("backup-4", 3 * H, 1240, false, None, true),
        entry("backup-3", 2 * D, 1190, true, Some("Antes de la actualización 1.21.4"), false),
        entry("backup-2", 3 * D, 1150, false, None, true),
        entry("backup-1", 21 * D, 870, false, Some("Base terminada"), false),
    ]
}

/// `backupSettings` de FakeBridge: automáticas a diario, se conservan 5.
pub fn sample_settings() -> BackupSettings {
    BackupSettings { auto: true, interval_hours: Some(24), max_count: Some(5) }
}
