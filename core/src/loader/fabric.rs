//! Fabric y Quilt: el servidor de metadatos da un perfil JSON con `inheritsFrom`.

use std::time::Duration;

use serde::Deserialize;

use super::LoaderVersion;
use crate::instance::LoaderKind;
use crate::launcher::Core;
use crate::net::check_status;
use crate::{Error, Result};

const LIST_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Fabric,
    Quilt,
}

impl From<LoaderKind> for Flavor {
    fn from(kind: LoaderKind) -> Self {
        match kind {
            LoaderKind::Quilt => Flavor::Quilt,
            _ => Flavor::Fabric,
        }
    }
}

impl Flavor {
    fn base(self) -> &'static str {
        match self {
            Flavor::Fabric => "https://meta.fabricmc.net/v2",
            Flavor::Quilt => "https://meta.quiltmc.org/v3",
        }
    }

    /// Id de versión que generan sus perfiles.
    pub fn version_id(self, mc: &str, loader: &str) -> String {
        match self {
            Flavor::Fabric => format!("fabric-loader-{loader}-{mc}"),
            Flavor::Quilt => format!("quilt-loader-{loader}-{mc}"),
        }
    }
}

#[derive(Deserialize)]
struct Entry {
    loader: EntryLoader,
}

#[derive(Deserialize)]
struct EntryLoader {
    version: String,
    /// Fabric lo indica solo en la última estable; Quilt nunca.
    #[serde(default)]
    stable: Option<bool>,
}

pub async fn versions(core: &Core, flavor: Flavor, mc: &str) -> Result<Vec<LoaderVersion>> {
    let url = format!("{}/versions/loader/{}", flavor.base(), urlencode(mc));
    let entries: Vec<Entry> = core.http().get_json_cached(&url, LIST_TTL).await?;
    if entries.is_empty() {
        return Err(Error::VersionNotFound(format!("{mc} con {flavor:?}")));
    }
    Ok(entries
        .into_iter()
        .map(|e| {
            // Fabric solo marca `stable` en la última estable: una versión sin
            // sufijo de prerelease (`-beta.N`, `-rc.N`) también lo es.
            let stable = e.loader.stable == Some(true) || !e.loader.version.contains('-');
            LoaderVersion {
                version: e.loader.version,
                stable,
            }
        })
        .collect())
}

pub async fn latest_stable(core: &Core, flavor: Flavor, mc: &str) -> Result<String> {
    let list = versions(core, flavor, mc).await?;
    list.iter()
        .find(|v| v.stable)
        .or(list.first())
        .map(|v| v.version.clone())
        .ok_or_else(|| Error::VersionNotFound(format!("{mc} con {flavor:?}")))
}

/// Descarga el perfil si no está y devuelve su id.
pub async fn install_profile(
    core: &Core,
    flavor: Flavor,
    mc: &str,
    loader: &str,
) -> Result<String> {
    let id = flavor.version_id(mc, loader);
    if core.installed_version_json(&id).await?.is_some() {
        return Ok(id);
    }
    if core.http().is_offline() {
        return Err(Error::Offline { url: id });
    }
    let url = format!(
        "{}/versions/loader/{}/{}/profile/json",
        flavor.base(),
        urlencode(mc),
        urlencode(loader)
    );
    let resp = core.http().send(|c| c.get(&url)).await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND
        || resp.status() == reqwest::StatusCode::BAD_REQUEST
    {
        return Err(Error::VersionNotFound(format!(
            "{flavor:?} {loader} para {mc}"
        )));
    }
    let bytes = check_status(resp)?.bytes().await?;
    let info = core.save_version_json(&bytes).await?;
    if info.id != id {
        // Quilt usa a veces otro formato de id: se respeta el del perfil.
        tracing::debug!(expected = %id, got = %info.id, "id de perfil distinto");
    }
    Ok(info.id)
}

fn urlencode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
