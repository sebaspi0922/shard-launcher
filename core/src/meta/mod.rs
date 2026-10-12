//! Manifiesto de versiones de Mojang y JSON de versión en `versions/<id>/<id>.json`.
//!
//! A diferencia del original, el JSON se guarda en disco: con todo instalado se
//! puede lanzar sin red.

pub mod library;
pub mod rules;
pub mod version;

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::download::sha1_file;
use crate::fsutil::write_atomic;
use crate::launcher::Core;
use crate::paths::single_segment;
use crate::{Error, Result};

pub use version::VersionInfo;

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const MANIFEST_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub latest: Latest,
    pub versions: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub release_time: Option<String>,
}

impl Manifest {
    pub fn find(&self, id: &str) -> Option<&ManifestEntry> {
        self.versions.iter().find(|v| v.id == id)
    }
}

impl Core {
    pub async fn version_manifest(&self) -> Result<Manifest> {
        self.http()
            .get_json_cached(MANIFEST_URL, MANIFEST_TTL)
            .await
    }

    fn version_json_path(&self, id: &str) -> Result<std::path::PathBuf> {
        let id = single_segment(id)?;
        Ok(self
            .paths()
            .versions_dir()
            .join(id)
            .join(format!("{id}.json")))
    }

    /// Lee `versions/<id>/<id>.json` si existe.
    pub async fn installed_version_json(&self, id: &str) -> Result<Option<VersionInfo>> {
        crate::fsutil::read_json(&self.version_json_path(id)?).await
    }

    /// Guarda un JSON de versión (perfil de loader, instalador…).
    pub async fn save_version_json(&self, raw: &[u8]) -> Result<VersionInfo> {
        let info: VersionInfo =
            serde_json::from_slice(raw).map_err(|e| Error::json("JSON de versión", e))?;
        write_atomic(&self.version_json_path(&info.id)?, raw).await?;
        Ok(info)
    }

    /// JSON de una versión de Mojang: de disco si coincide el SHA-1 del
    /// manifiesto (o si no hay red), si no, se descarga.
    pub async fn vanilla_version(&self, id: &str) -> Result<VersionInfo> {
        let path = self.version_json_path(id)?;
        let local_exists = tokio::fs::try_exists(&path).await.unwrap_or(false);

        let manifest = match self.version_manifest().await {
            Ok(m) => Some(m),
            Err(e) if local_exists => {
                tracing::warn!(error = %e, "sin manifiesto; uso el JSON local de {id}");
                None
            }
            Err(e) => return Err(e),
        };
        let entry = match &manifest {
            Some(m) => Some(
                m.find(id)
                    .ok_or_else(|| Error::VersionNotFound(id.to_owned()))?
                    .clone(),
            ),
            None => None,
        };

        if local_exists {
            let up_to_date = match entry.as_ref().and_then(|e| e.sha1.as_ref()) {
                Some(expected) => sha1_file(&path).await?.eq_ignore_ascii_case(expected),
                None => true,
            };
            if up_to_date && let Some(info) = self.installed_version_json(id).await? {
                return Ok(info);
            }
        }

        let entry = entry.expect("sin manifiesto solo se llega con JSON local");
        let resp = self.http().send(|c| c.get(&entry.url)).await?;
        let bytes = crate::net::check_status(resp)?.bytes().await?;
        if let Some(expected) = &entry.sha1 {
            let actual = hex::encode(<sha1::Sha1 as sha1::Digest>::digest(&bytes));
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(Error::HashMismatch {
                    url: entry.url.clone(),
                    expected: expected.clone(),
                    actual,
                });
            }
        }
        self.save_version_json(&bytes).await
    }

    /// Resuelve `id` con toda su cadena `inheritsFrom` ya fusionada.
    pub async fn resolve_version(&self, id: &str) -> Result<VersionInfo> {
        let mut chain = Vec::new();
        let mut current = id.to_owned();
        loop {
            if chain.len() > 8 {
                return Err(Error::InvalidInput(format!(
                    "cadena inheritsFrom demasiado larga en {id}"
                )));
            }
            // Las de Mojang se validan contra el manifiesto; los perfiles de
            // loaders y las versiones importadas se usan tal cual.
            let info = match self.installed_version_json(&current).await? {
                Some(info) if info.inherits_from.is_some() => info,
                Some(info) => match self.vanilla_version(&current).await {
                    Ok(v) => v,
                    Err(Error::VersionNotFound(_)) => info,
                    Err(e) => return Err(e),
                },
                None => self.vanilla_version(&current).await?,
            };
            let parent = info.inherits_from.clone();
            chain.push(info);
            match parent {
                Some(p) => current = p,
                None => break,
            }
        }
        let mut merged = chain.pop().expect("al menos una versión");
        while let Some(child) = chain.pop() {
            merged = child.merge_onto(merged);
        }
        Ok(merged)
    }
}
