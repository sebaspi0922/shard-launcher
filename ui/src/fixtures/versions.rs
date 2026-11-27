//! Datos de ejemplo de fake_bridge.dart: versiones de Minecraft
//! (`listVersions`) y de loaders (`listLoaderVersions`).

use crate::create_instance::Version;

/// `listVersions`: con snapshots, primero `25w14a`.
pub fn sample_versions(snapshots: bool) -> Vec<Version> {
    let releases = [
        "1.21.5", "1.21.4", "1.21.3", "1.21.1", "1.21", "1.20.6", "1.20.4", "1.20.1", "1.19.4", "1.19.2", "1.18.2", "1.16.5",
    ];
    let snap = snapshots.then(|| Version::new("25w14a", "snapshot"));
    snap.into_iter().chain(releases.iter().map(|v| Version::new(v, "release"))).collect()
}

/// `listLoaderVersions` (la misma lista para cualquier loader y versión).
pub fn sample_loader_versions() -> Vec<String> {
    ["0.16.14", "0.16.13", "0.16.12", "0.16.10"].map(String::from).to_vec()
}
