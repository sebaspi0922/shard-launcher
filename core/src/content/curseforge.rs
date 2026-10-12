//! Cliente de la API oficial de CurseForge. La clave la pone el usuario y vive
//! en el keyring (`<APP_ID>.curseforge`, usuario `api-key`); nunca se escribe en
//! JSON ni en logs.

use serde::Deserialize;
use serde_json::json;

use super::types::{
    ContentKind, ContentQuery, ContentVersion, Dependency, DependencyKind, Page, ProjectHit,
    ReleaseType, Source, VersionFile, loader_names,
};
use super::{build_url, get_json_keyed, post_json};
use crate::launcher::Core;
use crate::paths::APP_ID;
use crate::{Error, Result};

pub const DEFAULT_BASE: &str = "https://api.curseforge.com/v1";
/// `gameId` de Minecraft.
pub const GAME_ID: u32 = 432;
const KEY_USER: &str = "api-key";
const PAGE_SIZE: u32 = 50;
/// Máximo de ids por petición masiva.
const BULK_CHUNK: usize = 400;

fn key_service() -> String {
    format!("{APP_ID}.curseforge")
}

impl Core {
    /// Guarda la clave de CurseForge en el keyring.
    pub async fn set_curseforge_key(&self, key: &str) -> Result<()> {
        let key = key.trim();
        if key.is_empty() {
            return Err(Error::InvalidInput("la clave no puede estar vacía".into()));
        }
        self.secret_set(key_service(), KEY_USER.into(), key.into())
            .await
    }

    pub async fn clear_curseforge_key(&self) -> Result<()> {
        self.secret_delete(key_service(), KEY_USER.into()).await
    }

    pub async fn has_curseforge_key(&self) -> Result<bool> {
        Ok(self
            .secret_get(key_service(), KEY_USER.into())
            .await?
            .is_some_and(|k| !k.trim().is_empty()))
    }
}

#[derive(Clone)]
pub struct CurseForge {
    core: Core,
    base: String,
}

/// `modLoaderType` de CurseForge para un nombre de loader.
pub fn loader_type(name: &str) -> Option<u32> {
    match name {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

/// URL de descarga cuando la API no la da (autor sin permiso de distribución).
pub fn fallback_url(file_id: u64, file_name: &str) -> String {
    format!(
        "https://edge.forgecdn.net/files/{}/{}/{}",
        file_id / 1000,
        file_id % 1000,
        file_name
    )
}

impl CurseForge {
    pub fn new(core: Core, base: impl Into<String>) -> Self {
        Self {
            core,
            base: base.into().trim_end_matches('/').to_owned(),
        }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    async fn api_key(&self) -> Result<String> {
        self.core
            .secret_get(key_service(), KEY_USER.into())
            .await?
            .map(|k| k.trim().to_owned())
            .filter(|k| !k.is_empty())
            .ok_or(Error::CurseForgeKeyMissing)
    }

    async fn get<T: serde::de::DeserializeOwned>(
        &self,
        segments: &[&str],
        params: &[(&str, String)],
    ) -> Result<T> {
        let key = self.api_key().await?;
        let url = build_url(&self.base, segments, params)?;
        get_json_keyed(self.core.http(), &url, &key).await
    }

    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        segments: &[&str],
        body: &serde_json::Value,
    ) -> Result<T> {
        let key = self.api_key().await?;
        let url = build_url(&self.base, segments, &[])?;
        post_json(self.core.http(), &url, body, Some(&key)).await
    }

    /// `GET /mods/search`.
    pub async fn search(&self, q: &ContentQuery) -> Result<Page<ProjectHit>> {
        let limit = if q.limit == 0 {
            20
        } else {
            q.limit.min(PAGE_SIZE)
        };
        let mut params = vec![
            ("gameId", GAME_ID.to_string()),
            ("classId", q.kind.curseforge_class().to_string()),
            ("searchFilter", q.text.clone()),
            ("sortField", q.sort.curseforge_field().to_string()),
            ("sortOrder", "desc".to_owned()),
            ("index", q.offset.to_string()),
            ("pageSize", limit.to_string()),
        ];
        if let Some(mc) = &q.mc {
            params.push(("gameVersion", mc.clone()));
        }
        if q.kind.uses_loader()
            && let Some(loader) = q.loader
            && let Some(t) = loader_names(loader).first().and_then(|n| loader_type(n))
        {
            params.push(("modLoaderType", t.to_string()));
        }
        let raw: RawList<RawMod> = self.get(&["mods", "search"], &params).await?;
        let pagination = raw.pagination.unwrap_or_default();
        Ok(Page {
            items: raw.data.into_iter().map(RawMod::into_hit).collect(),
            offset: pagination.index,
            limit: if pagination.page_size == 0 {
                limit
            } else {
                pagination.page_size
            },
            total: pagination.total_count,
        })
    }

    /// `GET /mods/{id}`; si `project` no es numérico se busca por slug.
    pub async fn project(&self, project: &str) -> Result<ProjectHit> {
        if project.bytes().all(|b| b.is_ascii_digit()) && !project.is_empty() {
            let raw: RawOne<RawMod> = self.get(&["mods", project], &[]).await?;
            return Ok(raw.data.into_hit());
        }
        let params = [
            ("gameId", GAME_ID.to_string()),
            ("slug", project.to_owned()),
            ("pageSize", "1".to_owned()),
        ];
        let raw: RawList<RawMod> = self.get(&["mods", "search"], &params).await?;
        raw.data
            .into_iter()
            .next()
            .map(RawMod::into_hit)
            .ok_or_else(|| Error::InvalidInput(format!("no existe el proyecto {project:?}")))
    }

    /// `POST /mods` (información de varios proyectos).
    pub async fn projects(&self, ids: &[String]) -> Result<Vec<ProjectHit>> {
        let mut out = Vec::new();
        for chunk in ids.chunks(BULK_CHUNK) {
            let ids: Vec<u64> = chunk.iter().filter_map(|i| i.parse().ok()).collect();
            let raw: RawList<RawMod> = self
                .post(&["mods"], &json!({ "modIds": ids, "filterPcOnly": true }))
                .await?;
            out.extend(raw.data.into_iter().map(RawMod::into_hit));
        }
        Ok(out)
    }

    /// `GET /mods/{id}/files` (la más nueva primero). Con varios loaders se
    /// piden por separado y se juntan sin repetir.
    pub async fn project_versions(
        &self,
        mod_id: &str,
        mc: Option<&str>,
        loaders: &[String],
    ) -> Result<Vec<ContentVersion>> {
        let types: Vec<Option<u32>> = if loaders.is_empty() {
            vec![None]
        } else {
            let t: Vec<_> = loaders.iter().filter_map(|l| loader_type(l)).collect();
            if t.is_empty() {
                return Ok(Vec::new());
            }
            t.into_iter().map(Some).collect()
        };
        let mut out: Vec<ContentVersion> = Vec::new();
        for t in types {
            let mut params = vec![("pageSize", PAGE_SIZE.to_string())];
            if let Some(mc) = mc {
                params.push(("gameVersion", mc.to_owned()));
            }
            if let Some(t) = t {
                params.push(("modLoaderType", t.to_string()));
            }
            let raw: RawList<RawFile> = self.get(&["mods", mod_id, "files"], &params).await?;
            for f in raw.data {
                if !out.iter().any(|v| v.id == f.id.to_string()) {
                    out.push(f.into_version());
                }
            }
        }
        out.sort_by(|a, b| b.date_published.cmp(&a.date_published));
        Ok(out)
    }

    /// `GET /mods/{modId}/files/{fileId}`.
    pub async fn version(&self, mod_id: &str, file_id: &str) -> Result<ContentVersion> {
        let raw: RawOne<RawFile> = self.get(&["mods", mod_id, "files", file_id], &[]).await?;
        Ok(raw.data.into_version())
    }

    /// `POST /mods/files` (archivos por id).
    pub async fn files_bulk(&self, ids: &[u64]) -> Result<Vec<ContentVersion>> {
        let mut out = Vec::new();
        for chunk in ids.chunks(BULK_CHUNK) {
            let raw: RawList<RawFile> = self
                .post(&["mods", "files"], &json!({ "fileIds": chunk }))
                .await?;
            out.extend(raw.data.into_iter().map(RawFile::into_version));
        }
        Ok(out)
    }
}

#[derive(Deserialize)]
struct RawOne<T> {
    data: T,
}

#[derive(Deserialize)]
struct RawList<T> {
    #[serde(default = "Vec::new")]
    data: Vec<T>,
    #[serde(default)]
    pagination: Option<RawPagination>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawPagination {
    #[serde(default)]
    index: u32,
    #[serde(default)]
    page_size: u32,
    #[serde(default)]
    total_count: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMod {
    id: u64,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    authors: Vec<RawNamed>,
    #[serde(default)]
    logo: Option<RawLogo>,
    #[serde(default)]
    download_count: f64,
    #[serde(default)]
    thumbs_up_count: u64,
    #[serde(default)]
    class_id: Option<u32>,
    #[serde(default)]
    categories: Vec<RawNamed>,
    #[serde(default)]
    date_modified: Option<String>,
}

#[derive(Deserialize)]
struct RawNamed {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct RawLogo {
    #[serde(default)]
    url: String,
}

impl RawMod {
    fn into_hit(self) -> ProjectHit {
        ProjectHit {
            source: Source::CurseForge,
            id: self.id.to_string(),
            slug: self.slug,
            kind: self.class_id.and_then(ContentKind::from_curseforge_class),
            title: self.name,
            description: self.summary,
            author: self.authors.into_iter().next().map(|a| a.name),
            icon_url: self.logo.map(|l| l.url).filter(|u| !u.is_empty()),
            downloads: self.download_count.max(0.0) as u64,
            follows: self.thumbs_up_count,
            categories: self.categories.into_iter().map(|c| c.name).collect(),
            date_modified: self.date_modified,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFile {
    id: u64,
    mod_id: u64,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    file_name: String,
    #[serde(default = "default_release")]
    release_type: u32,
    #[serde(default)]
    file_date: String,
    #[serde(default)]
    file_length: u64,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    hashes: Vec<RawHash>,
    #[serde(default)]
    dependencies: Vec<RawDependency>,
}

fn default_release() -> u32 {
    1
}

#[derive(Deserialize)]
struct RawHash {
    #[serde(default)]
    value: String,
    #[serde(default)]
    algo: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDependency {
    mod_id: u64,
    #[serde(default)]
    relation_type: u32,
}

impl RawFile {
    fn into_version(self) -> ContentVersion {
        let (url, fallback) = match self.download_url.filter(|u| !u.is_empty()) {
            Some(u) => (u, false),
            None => (fallback_url(self.id, &self.file_name), true),
        };
        let sha1 = self
            .hashes
            .iter()
            .find(|h| h.algo == 1)
            .map(|h| h.value.to_ascii_lowercase());
        let mut mc_versions = Vec::new();
        let mut loaders = Vec::new();
        for gv in &self.game_versions {
            if gv.starts_with(|c: char| c.is_ascii_digit()) {
                mc_versions.push(gv.clone());
            } else {
                let lower = gv.to_ascii_lowercase();
                if loader_type(&lower).is_some() {
                    loaders.push(lower);
                }
            }
        }
        ContentVersion {
            source: Source::CurseForge,
            id: self.id.to_string(),
            project_id: self.mod_id.to_string(),
            name: self.display_name.clone(),
            version_number: self.display_name,
            mc_versions,
            loaders,
            release: match self.release_type {
                2 => ReleaseType::Beta,
                3 => ReleaseType::Alpha,
                _ => ReleaseType::Release,
            },
            date_published: self.file_date,
            files: vec![VersionFile {
                filename: self.file_name,
                url,
                sha1,
                size: self.file_length,
                primary: true,
                fallback_url: fallback,
            }],
            dependencies: self
                .dependencies
                .into_iter()
                .map(|d| Dependency {
                    project_id: Some(d.mod_id.to_string()),
                    version_id: None,
                    kind: match d.relation_type {
                        3 => DependencyKind::Required,
                        5 => DependencyKind::Incompatible,
                        1 | 6 => DependencyKind::Embedded,
                        _ => DependencyKind::Optional,
                    },
                })
                .collect(),
        }
    }
}
