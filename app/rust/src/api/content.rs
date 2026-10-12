//! Contenido: búsqueda en Modrinth/CurseForge, instalación, gestión de lo
//! instalado, modpacks, clave de CurseForge y caché de iconos.

use std::path::PathBuf;

use lcore::content::{
    ContentIcon, ContentKind, ContentQuery, ContentVersion, InstalledContent, ProjectHit,
    SortOrder, Source,
};
use lcore::instance::LoaderKind;

use crate::api::types::{BridgeError, InstanceView};
use crate::state::{core, on_rt};

/// Tamaño máximo de un icono descargado.
const MAX_ICON_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ProjectView {
    pub source: String,
    pub id: String,
    pub slug: String,
    /// `mod`, `resourcepack`, `shader`, `datapack`, `modpack` o `None`.
    pub kind: Option<String>,
    pub title: String,
    pub description: String,
    pub author: Option<String>,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub categories: Vec<String>,
    pub date_modified: Option<String>,
}

impl From<ProjectHit> for ProjectView {
    fn from(p: ProjectHit) -> Self {
        Self {
            source: p.source.as_str().into(),
            id: p.id,
            slug: p.slug,
            kind: p.kind.map(|k| k.as_str().into()),
            title: p.title,
            description: p.description,
            author: p.author,
            icon_url: p.icon_url,
            downloads: p.downloads,
            follows: p.follows,
            categories: p.categories,
            date_modified: p.date_modified,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchPageView {
    pub items: Vec<ProjectView>,
    pub offset: u32,
    pub limit: u32,
    pub total: u32,
}

#[derive(Debug, Clone)]
pub struct ContentVersionView {
    pub source: String,
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub mc_versions: Vec<String>,
    pub loaders: Vec<String>,
    /// `release`, `beta` o `alpha`.
    pub release: String,
    pub date_published: String,
    pub file_name: Option<String>,
    pub file_url: Option<String>,
    pub file_size: u64,
}

impl From<ContentVersion> for ContentVersionView {
    fn from(v: ContentVersion) -> Self {
        let file = v.primary_file().cloned();
        Self {
            source: v.source.as_str().into(),
            release: match v.release {
                lcore::content::ReleaseType::Release => "release",
                lcore::content::ReleaseType::Beta => "beta",
                lcore::content::ReleaseType::Alpha => "alpha",
            }
            .into(),
            id: v.id,
            project_id: v.project_id,
            name: v.name,
            version_number: v.version_number,
            mc_versions: v.mc_versions,
            loaders: v.loaders,
            date_published: v.date_published,
            file_name: file.as_ref().map(|f| f.filename.clone()),
            file_url: file.as_ref().map(|f| f.url.clone()),
            file_size: file.map(|f| f.size).unwrap_or(0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstalledContentView {
    pub kind: String,
    /// Nombre sin `.disabled`; en los datapacks, `<mundo>/<archivo>`.
    pub file: String,
    pub enabled: bool,
    pub source: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub version_id: Option<String>,
    pub version_number: Option<String>,
    pub size: u64,
    pub is_dir: bool,
    pub tracked: bool,
    pub world: Option<String>,
}

impl From<InstalledContent> for InstalledContentView {
    fn from(c: InstalledContent) -> Self {
        Self {
            kind: c.kind.as_str().into(),
            file: c.file,
            enabled: c.enabled,
            source: c.source,
            project_id: c.project_id,
            project_name: c.project_name,
            version_id: c.version_id,
            version_number: c.version_number,
            size: c.size,
            is_dir: c.is_dir,
            tracked: c.tracked,
            world: c.world,
        }
    }
}

/// Icono de un archivo instalado (solo los que se pudieron resolver).
#[derive(Debug, Clone)]
pub struct ContentIconView {
    pub kind: String,
    pub file: String,
    pub icon_url: String,
}

impl From<ContentIcon> for ContentIconView {
    fn from(i: ContentIcon) -> Self {
        Self {
            kind: i.kind.as_str().into(),
            file: i.file,
            icon_url: i.icon_url,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContentUpdateView {
    pub kind: String,
    pub file: String,
    pub project_id: String,
    pub project_name: Option<String>,
    pub current_version: Option<String>,
    pub latest_version: String,
}

#[derive(Debug, Clone)]
pub struct InstalledItemView {
    pub kind: String,
    pub project_name: String,
    pub version_number: String,
    pub file: String,
    pub dependency: bool,
    pub already_present: bool,
}

#[derive(Debug, Clone)]
pub struct SkippedItemView {
    pub project_id: String,
    pub project_name: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct InstallOutcomeView {
    pub installed: Vec<InstalledItemView>,
    pub skipped: Vec<SkippedItemView>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IdentifyView {
    /// Nombres de los proyectos reconocidos.
    pub identified: Vec<String>,
    /// Archivos que no se reconocieron.
    pub unmatched: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModpackInstallView {
    pub instance: InstanceView,
    pub files_total: u32,
    pub files_downloaded: u32,
    pub warnings: Vec<String>,
}

fn invalid(message: String) -> BridgeError {
    BridgeError {
        kind: "invalid_input".into(),
        message,
    }
}

fn parse_source(s: &str) -> Result<Source, BridgeError> {
    Source::parse(s).ok_or_else(|| invalid(format!("origen desconocido: {s}")))
}

fn parse_kind(s: &str) -> Result<ContentKind, BridgeError> {
    ContentKind::parse(s).ok_or_else(|| invalid(format!("tipo de contenido desconocido: {s}")))
}

fn parse_loader(s: Option<String>) -> Result<Option<LoaderKind>, BridgeError> {
    s.filter(|l| !l.is_empty())
        .map(|l| LoaderKind::parse(&l).ok_or_else(|| invalid(format!("loader desconocido: {l}"))))
        .transpose()
}

/// Busca proyectos. `loader` solo filtra mods y modpacks.
#[allow(clippy::too_many_arguments)]
pub async fn search_content(
    source: String,
    kind: String,
    text: String,
    mc: Option<String>,
    loader: Option<String>,
    sort: String,
    offset: u32,
    limit: u32,
) -> Result<SearchPageView, BridgeError> {
    let core = core()?;
    let query = ContentQuery {
        source: parse_source(&source)?,
        kind: parse_kind(&kind)?,
        text,
        mc: mc.filter(|m| !m.is_empty()),
        loader: parse_loader(loader)?,
        offset,
        limit: limit.clamp(1, 100),
        sort: SortOrder::parse(&sort).unwrap_or_default(),
    };
    on_rt(async move {
        let page = core.search_content(&query).await?;
        Ok(SearchPageView {
            items: page.items.into_iter().map(Into::into).collect(),
            offset: page.offset,
            limit: page.limit,
            total: page.total,
        })
    })
    .await
}

/// Ficha de un proyecto (id o slug).
pub async fn get_project(source: String, project: String) -> Result<ProjectView, BridgeError> {
    let core = core()?;
    let source = parse_source(&source)?;
    on_rt(async move { Ok(core.content().project(source, &project).await?.into()) }).await
}

/// Versiones compatibles (la más nueva primero). `loader` solo si el tipo
/// depende de él.
pub async fn content_versions(
    source: String,
    project: String,
    mc: Option<String>,
    loader: Option<String>,
) -> Result<Vec<ContentVersionView>, BridgeError> {
    let core = core()?;
    let source = parse_source(&source)?;
    let loader = parse_loader(loader)?;
    on_rt(async move {
        let list = core
            .content_versions(source, &project, mc.as_deref().filter(|m| !m.is_empty()), loader)
            .await?;
        Ok(list.into_iter().map(Into::into).collect())
    })
    .await
}

/// Instala un proyecto (y sus dependencias obligatorias) en una instancia.
/// Los datapacks necesitan `world`.
pub async fn install_content(
    instance_id: String,
    source: String,
    project: String,
    version: Option<String>,
    world: Option<String>,
) -> Result<InstallOutcomeView, BridgeError> {
    let core = core()?;
    let source = parse_source(&source)?;
    on_rt(async move {
        let out = core
            .install_content(
                &instance_id,
                source,
                &project,
                version.as_deref(),
                world.as_deref(),
            )
            .await?;
        Ok(InstallOutcomeView {
            installed: out
                .installed
                .into_iter()
                .map(|i| InstalledItemView {
                    kind: i.kind.as_str().into(),
                    project_name: i.project_name,
                    version_number: i.version_number,
                    file: i.file,
                    dependency: i.dependency,
                    already_present: i.already_present,
                })
                .collect(),
            skipped: out
                .skipped
                .into_iter()
                .map(|s| SkippedItemView {
                    project_id: s.project_id,
                    project_name: s.project_name,
                    reason: s.reason,
                })
                .collect(),
            warnings: out.warnings,
        })
    })
    .await
}

/// Iconos de lo instalado (consulta Modrinth/CurseForge; con caché). Los
/// archivos que ninguna fuente reconoce no aparecen.
pub async fn content_icons(instance_id: String) -> Result<Vec<ContentIconView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .content_icons(&instance_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    })
    .await
}

pub async fn list_content(instance_id: String) -> Result<Vec<InstalledContentView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .list_content(&instance_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    })
    .await
}

/// `file` admite el prefijo `tipo/` para evitar ambigüedades.
pub async fn set_content_enabled(
    instance_id: String,
    file: String,
    enabled: bool,
) -> Result<InstalledContentView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .set_content_enabled(&instance_id, &file, enabled)
            .await?
            .into())
    })
    .await
}

pub async fn remove_content(
    instance_id: String,
    file: String,
) -> Result<InstalledContentView, BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.remove_content(&instance_id, &file).await?.into()) }).await
}

pub async fn check_content_updates(
    instance_id: String,
) -> Result<Vec<ContentUpdateView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .check_content_updates(&instance_id)
            .await?
            .into_iter()
            .map(|u| ContentUpdateView {
                kind: u.kind.as_str().into(),
                file: u.file,
                project_id: u.project_id,
                project_name: u.project_name,
                current_version: u.current_version,
                latest_version: u.latest.version_number,
            })
            .collect())
    })
    .await
}

/// Actualiza un archivo. Devuelve la versión nueva o `None` si ya estaba al día.
pub async fn update_content(
    instance_id: String,
    file: String,
) -> Result<Option<String>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .update_content(&instance_id, &file)
            .await?
            .map(|u| u.to_version))
    })
    .await
}

/// Reconoce en Modrinth (por hash) los archivos de procedencia desconocida.
pub async fn identify_content(instance_id: String) -> Result<IdentifyView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let r = core.identify_content(&instance_id).await?;
        Ok(IdentifyView {
            identified: r.identified.into_iter().map(|i| i.project_name).collect(),
            unmatched: r.unmatched,
        })
    })
    .await
}

/// Instala un modpack (ruta local o URL; `.mrpack` o zip de CurseForge) en una
/// instancia nueva. El progreso llega por eventos con la etapa `modpack`.
pub async fn install_modpack(
    source: String,
    name: Option<String>,
) -> Result<ModpackInstallView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let name = name.filter(|n| !n.trim().is_empty());
        let out = core.install_modpack(&source, name).await?;
        let dir = core
            .instance_game_dir(&out.instance.id)
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        Ok(ModpackInstallView {
            instance: InstanceView::new(&out.instance, dir),
            files_total: out.files_total as u32,
            files_downloaded: out.files_downloaded as u32,
            warnings: out.warnings,
        })
    })
    .await
}

pub async fn has_curseforge_key() -> Result<bool, BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.has_curseforge_key().await?) }).await
}

/// Guarda la clave en el keyring. Nunca se devuelve.
pub async fn set_curseforge_key(key: String) -> Result<(), BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.set_curseforge_key(&key).await?) }).await
}

pub async fn clear_curseforge_key() -> Result<(), BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.clear_curseforge_key().await?) }).await
}

/// FNV-1a de 64 bits (estable) para nombrar los archivos de la caché.
fn fnv64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Descarga (una vez) una imagen remota a `<datos>/cache/icons/` y devuelve su
/// ruta. Solo HTTPS y como mucho 2 MB. `None` si no se pudo.
pub async fn cached_image(url: String) -> Result<Option<String>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        if !url.starts_with("https://") || url.len() > 2048 {
            return Ok(None);
        }
        let dir = core.paths().root().join("cache").join("icons");
        let path: PathBuf = dir.join(format!("{:016x}", fnv64(&url)));
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Ok(Some(path.display().to_string()));
        }
        if core.http().is_offline() {
            return Ok(None);
        }
        let resp = match core.http().send(|c| c.get(url.as_str())).await {
            Ok(r) if r.status().is_success() => r,
            _ => return Ok(None),
        };
        if resp
            .content_length()
            .is_some_and(|n| n as usize > MAX_ICON_BYTES)
        {
            return Ok(None);
        }
        let Ok(bytes) = resp.bytes().await else {
            return Ok(None);
        };
        if bytes.len() > MAX_ICON_BYTES || bytes.is_empty() {
            return Ok(None);
        }
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| BridgeError::internal(format!("{}: {e}", dir.display())))?;
        lcore::fsutil::write_atomic(&path, &bytes).await?;
        Ok(Some(path.display().to_string()))
    })
    .await
}
