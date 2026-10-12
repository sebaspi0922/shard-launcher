//! Cliente de la API de Modrinth (<https://docs.modrinth.com>).
//! La URL base es inyectable para poder probarlo contra un servidor simulado.

use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use super::types::{
    ContentKind, ContentQuery, ContentVersion, Dependency, DependencyKind, Page, ProjectHit,
    ReleaseType, Source, VersionFile, loader_names,
};
use super::{build_url, post_json};
use crate::Result;
use crate::net::Http;

pub const DEFAULT_BASE: &str = "https://api.modrinth.com/v2";

const TTL_SEARCH: Duration = Duration::from_secs(5 * 60);
const TTL_PROJECT: Duration = Duration::from_secs(30 * 60);
const TTL_VERSIONS: Duration = Duration::from_secs(2 * 60);
const TTL_VERSION: Duration = Duration::from_secs(24 * 3600);
/// Máximo de resultados por página que admite la API.
const MAX_LIMIT: u32 = 100;

#[derive(Debug, Clone)]
pub struct Modrinth {
    http: Http,
    base: String,
}

impl Modrinth {
    pub fn new(http: Http, base: impl Into<String>) -> Self {
        Self {
            http,
            base: base.into().trim_end_matches('/').to_owned(),
        }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    /// `GET /search` con `facets` (tipo, versión de Minecraft y loader).
    pub async fn search(&self, q: &ContentQuery) -> Result<Page<ProjectHit>> {
        let limit = if q.limit == 0 {
            20
        } else {
            q.limit.min(MAX_LIMIT)
        };
        let url = build_url(
            &self.base,
            &["search"],
            &[
                ("query", q.text.clone()),
                ("facets", facets(q)),
                ("index", q.sort.modrinth_index().to_owned()),
                ("offset", q.offset.to_string()),
                ("limit", limit.to_string()),
            ],
        )?;
        let raw: RawSearch = self.http.get_json_cached(&url, TTL_SEARCH).await?;
        Ok(Page {
            items: raw.hits.into_iter().map(RawHit::into_hit).collect(),
            offset: raw.offset,
            limit: raw.limit,
            total: raw.total_hits,
        })
    }

    /// `GET /project/{id|slug}`.
    pub async fn project(&self, id: &str) -> Result<ProjectHit> {
        let url = build_url(&self.base, &["project", id], &[])?;
        let raw: RawProject = self.http.get_json_cached(&url, TTL_PROJECT).await?;
        Ok(raw.into_hit())
    }

    /// `GET /projects?ids=[…]`.
    pub async fn projects(&self, ids: &[String]) -> Result<Vec<ProjectHit>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let url = build_url(
            &self.base,
            &["projects"],
            &[("ids", json!(ids).to_string())],
        )?;
        let raw: Vec<RawProject> = self.http.get_json_cached(&url, TTL_PROJECT).await?;
        Ok(raw.into_iter().map(RawProject::into_hit).collect())
    }

    /// `GET /project/{id}/version?loaders=[…]&game_versions=[…]` (la más nueva primero).
    pub async fn project_versions(
        &self,
        project: &str,
        mc: Option<&str>,
        loaders: &[String],
    ) -> Result<Vec<ContentVersion>> {
        let mut params = vec![("include_changelog", "false".to_owned())];
        if !loaders.is_empty() {
            params.push(("loaders", json!(loaders).to_string()));
        }
        if let Some(mc) = mc {
            params.push(("game_versions", json!([mc]).to_string()));
        }
        let url = build_url(&self.base, &["project", project, "version"], &params)?;
        let raw: Vec<RawVersion> = self.http.get_json_cached(&url, TTL_VERSIONS).await?;
        Ok(raw.into_iter().map(RawVersion::into_version).collect())
    }

    /// `GET /version/{id}`.
    pub async fn version(&self, id: &str) -> Result<ContentVersion> {
        let url = build_url(&self.base, &["version", id], &[])?;
        let raw: RawVersion = self.http.get_json_cached(&url, TTL_VERSION).await?;
        Ok(raw.into_version())
    }

    /// `POST /version_files`: versión de cada SHA-1 conocido por Modrinth.
    pub async fn versions_by_hashes(
        &self,
        hashes: &[String],
    ) -> Result<HashMap<String, ContentVersion>> {
        if hashes.is_empty() {
            return Ok(HashMap::new());
        }
        let url = build_url(&self.base, &["version_files"], &[])?;
        let body = json!({ "hashes": hashes, "algorithm": "sha1" });
        let raw: HashMap<String, RawVersion> = post_json(&self.http, &url, &body, None).await?;
        Ok(into_versions(raw))
    }

    /// `POST /version_files/update`: la versión más nueva compatible con cada SHA-1.
    pub async fn latest_by_hashes(
        &self,
        hashes: &[String],
        loaders: &[String],
        game_versions: &[String],
    ) -> Result<HashMap<String, ContentVersion>> {
        if hashes.is_empty() {
            return Ok(HashMap::new());
        }
        let url = build_url(&self.base, &["version_files", "update"], &[])?;
        let body = json!({
            "hashes": hashes,
            "algorithm": "sha1",
            "loaders": loaders,
            "game_versions": game_versions,
        });
        let raw: HashMap<String, RawVersion> = post_json(&self.http, &url, &body, None).await?;
        Ok(into_versions(raw))
    }
}

fn into_versions(raw: HashMap<String, RawVersion>) -> HashMap<String, ContentVersion> {
    raw.into_iter()
        .map(|(hash, v)| (hash.to_ascii_lowercase(), v.into_version()))
        .collect()
}

/// Filtros de la búsqueda: `[[A],[B]]` es A y B; los grupos internos son «o».
pub fn facets(q: &ContentQuery) -> String {
    let mut groups: Vec<Vec<String>> =
        vec![vec![format!("project_type:{}", q.kind.modrinth_type())]];
    if let Some(mc) = &q.mc {
        groups.push(vec![format!("versions:{mc}")]);
    }
    if q.kind.uses_loader()
        && let Some(loader) = q.loader
    {
        let names = loader_names(loader);
        if !names.is_empty() {
            groups.push(names.iter().map(|n| format!("categories:{n}")).collect());
        }
    }
    json!(groups).to_string()
}

#[derive(Deserialize)]
struct RawSearch {
    #[serde(default)]
    hits: Vec<RawHit>,
    #[serde(default)]
    offset: u32,
    #[serde(default)]
    limit: u32,
    #[serde(default)]
    total_hits: u32,
}

#[derive(Deserialize)]
struct RawHit {
    project_id: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    project_type: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    icon_url: Option<String>,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    follows: u64,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    date_modified: Option<String>,
}

impl RawHit {
    fn into_hit(self) -> ProjectHit {
        ProjectHit {
            source: Source::Modrinth,
            id: self.project_id,
            slug: self.slug,
            kind: ContentKind::parse(&self.project_type),
            title: self.title,
            description: self.description,
            author: self.author,
            icon_url: self.icon_url.filter(|u| !u.is_empty()),
            downloads: self.downloads,
            follows: self.follows,
            categories: self.categories,
            date_modified: self.date_modified,
        }
    }
}

#[derive(Deserialize)]
struct RawProject {
    id: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    project_type: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    icon_url: Option<String>,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    followers: u64,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    updated: Option<String>,
}

impl RawProject {
    fn into_hit(self) -> ProjectHit {
        ProjectHit {
            source: Source::Modrinth,
            id: self.id,
            slug: self.slug,
            kind: ContentKind::parse(&self.project_type),
            title: self.title,
            description: self.description,
            author: None,
            icon_url: self.icon_url.filter(|u| !u.is_empty()),
            downloads: self.downloads,
            follows: self.followers,
            categories: self.categories,
            date_modified: self.updated,
        }
    }
}

#[derive(Deserialize)]
struct RawVersion {
    id: String,
    project_id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    version_number: String,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    #[serde(default)]
    version_type: String,
    #[serde(default)]
    date_published: String,
    #[serde(default)]
    files: Vec<RawFile>,
    #[serde(default)]
    dependencies: Vec<RawDependency>,
}

#[derive(Deserialize)]
struct RawFile {
    #[serde(default)]
    hashes: HashMap<String, String>,
    url: String,
    filename: String,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    size: u64,
}

#[derive(Deserialize)]
struct RawDependency {
    #[serde(default)]
    version_id: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    dependency_type: String,
}

impl RawVersion {
    fn into_version(self) -> ContentVersion {
        ContentVersion {
            source: Source::Modrinth,
            id: self.id,
            project_id: self.project_id,
            name: self.name,
            version_number: self.version_number,
            mc_versions: self.game_versions,
            loaders: self.loaders,
            release: match self.version_type.as_str() {
                "beta" => ReleaseType::Beta,
                "alpha" => ReleaseType::Alpha,
                _ => ReleaseType::Release,
            },
            date_published: self.date_published,
            files: self
                .files
                .into_iter()
                .map(|f| VersionFile {
                    sha1: f.hashes.get("sha1").map(|h| h.to_ascii_lowercase()),
                    filename: f.filename,
                    url: f.url,
                    size: f.size,
                    primary: f.primary,
                    fallback_url: false,
                })
                .collect(),
            dependencies: self
                .dependencies
                .into_iter()
                .map(|d| Dependency {
                    project_id: d.project_id,
                    version_id: d.version_id,
                    kind: match d.dependency_type.as_str() {
                        "required" => DependencyKind::Required,
                        "incompatible" => DependencyKind::Incompatible,
                        "embedded" => DependencyKind::Embedded,
                        _ => DependencyKind::Optional,
                    },
                })
                .collect(),
        }
    }
}
