//! Contenido de las instancias: mods, resource packs, shaders y datapacks de
//! Modrinth y CurseForge, registro de procedencia (`content.json`) y modpacks.
//! Diseño en `docs/arquitectura.md` y `research/06-lanzamiento.md` §9.

pub mod curseforge;
mod icons;
mod install;
mod manage;
pub mod modpack;
pub mod modrinth;
pub mod types;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub use icons::ContentIcon;
pub use install::{
    IdentifiedContent, IdentifyReport, InstallOutcome, InstalledItem, SkippedItem, UpdateAvailable,
    UpdatedContent,
};
pub use manage::InstalledContent;
pub use modpack::ModpackInstall;
pub use types::{
    ContentKind, ContentQuery, ContentVersion, Dependency, DependencyKind, Page, ProjectHit,
    ReleaseType, SortOrder, Source, VersionFile,
};

use crate::fsutil::{read_json, write_json};
use crate::instance::LoaderKind;
use crate::launcher::Core;
use crate::net::{Http, check_status};
use crate::{Error, Result};
use curseforge::CurseForge;
use modrinth::Modrinth;

/// Valor de `source` de lo que no se sabe de dónde viene.
pub const SOURCE_UNKNOWN: &str = "unknown";
const INDEX_SCHEMA: u32 = 1;

/// Archivo de contenido de una instancia (`content.json`).
///
/// Para los datapacks, `file` es `<mundo>/<archivo>` (viven en
/// `saves/<mundo>/datapacks/`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentFile {
    /// `mod`, `resourcepack`, `shader`, `datapack`…
    #[serde(rename = "type")]
    pub kind: String,
    /// Nombre del archivo sin el sufijo `.disabled`.
    pub file: String,
    pub enabled: bool,
    /// `modrinth`, `curseforge` o `unknown`.
    pub source: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    pub version_number: Option<String>,
    pub sha1: Option<String>,
    pub url: Option<String>,
    pub category: Option<String>,
}

/// `instances/<id>/content.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentIndex {
    pub schema: u32,
    pub files: Vec<ContentFile>,
}

impl Default for ContentIndex {
    fn default() -> Self {
        Self {
            schema: INDEX_SCHEMA,
            files: Vec::new(),
        }
    }
}

impl ContentIndex {
    pub fn find(&self, kind: &str, file: &str) -> Option<&ContentFile> {
        self.files.iter().find(|f| f.kind == kind && f.file == file)
    }

    /// Añade la entrada o sustituye la que tenga el mismo tipo y archivo.
    pub fn upsert(&mut self, entry: ContentFile) {
        match self
            .files
            .iter_mut()
            .find(|f| f.kind == entry.kind && f.file == entry.file)
        {
            Some(slot) => *slot = entry,
            None => self.files.push(entry),
        }
    }

    pub fn remove(&mut self, kind: &str, file: &str) -> Option<ContentFile> {
        let pos = self
            .files
            .iter()
            .position(|f| f.kind == kind && f.file == file)?;
        Some(self.files.remove(pos))
    }

    /// ¿Hay algo instalado de ese proyecto?
    pub fn has_project(&self, source: &str, project_id: &str) -> bool {
        self.files
            .iter()
            .any(|f| f.source == source && f.project_id.as_deref() == Some(project_id))
    }
}

/// Lee `content.json`; si no existe, un índice vacío.
pub async fn load_index(path: &Path) -> Result<ContentIndex> {
    Ok(read_json(path).await?.unwrap_or_default())
}

pub async fn save_index(path: &Path, index: &ContentIndex) -> Result<()> {
    let mut index = index.clone();
    index.schema = INDEX_SCHEMA;
    write_json(path, &index).await
}

/// Serializa las lecturas-modificaciones de cualquier `content.json`.
static INDEX_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Lee, modifica y guarda `content.json` de forma atómica respecto a otras
/// operaciones de este proceso.
pub(crate) async fn update_index<T>(
    path: &Path,
    f: impl FnOnce(&mut ContentIndex) -> Result<T>,
) -> Result<T> {
    let _guard = INDEX_LOCK.lock().await;
    let mut index = load_index(path).await?;
    let out = f(&mut index)?;
    save_index(path, &index).await?;
    Ok(out)
}

/// Construye `base/seg1/seg2?clave=valor…` codificando cada parte.
pub(crate) fn build_url(
    base: &str,
    segments: &[&str],
    params: &[(&str, String)],
) -> Result<String> {
    let mut url = url::Url::parse(base)
        .map_err(|e| Error::InvalidInput(format!("URL base no válida {base:?}: {e}")))?;
    {
        let mut segs = url
            .path_segments_mut()
            .map_err(|_| Error::InvalidInput(format!("URL base no válida: {base:?}")))?;
        segs.pop_if_empty();
        segs.extend(segments);
    }
    if !params.is_empty() {
        url.query_pairs_mut()
            .extend_pairs(params.iter().map(|(k, v)| (*k, v.as_str())));
    }
    Ok(url.to_string())
}

/// GET de un JSON con la cabecera `x-api-key` (sin caché: lleva credenciales).
pub(crate) async fn get_json_keyed<T: DeserializeOwned>(
    http: &Http,
    url: &str,
    key: &str,
) -> Result<T> {
    if http.is_offline() {
        return Err(Error::Offline { url: url.into() });
    }
    let resp = http.send(|c| c.get(url).header("x-api-key", key)).await?;
    let text = check_status(resp)?.text().await?;
    serde_json::from_str(&text).map_err(|e| Error::json(url, e))
}

/// POST con cuerpo JSON y respuesta JSON; `key` añade `x-api-key`.
pub(crate) async fn post_json<T: DeserializeOwned>(
    http: &Http,
    url: &str,
    body: &serde_json::Value,
    key: Option<&str>,
) -> Result<T> {
    if http.is_offline() {
        return Err(Error::Offline { url: url.into() });
    }
    let resp = http
        .send(|c| {
            let req = c.post(url).json(body);
            match key {
                Some(k) => req.header("x-api-key", k),
                None => req,
            }
        })
        .await?;
    let text = check_status(resp)?.text().await?;
    serde_json::from_str(&text).map_err(|e| Error::json(url, e))
}

/// URLs base de las APIs (cambiables para las pruebas).
#[derive(Debug, Clone)]
pub struct ContentEndpoints {
    pub modrinth: String,
    pub curseforge: String,
}

impl Default for ContentEndpoints {
    fn default() -> Self {
        Self {
            modrinth: modrinth::DEFAULT_BASE.into(),
            curseforge: curseforge::DEFAULT_BASE.into(),
        }
    }
}

/// Asa de la gestión de contenido: `core.content()`.
#[derive(Clone)]
pub struct Content {
    pub(crate) core: Core,
    pub(crate) modrinth: Modrinth,
    pub(crate) curseforge: CurseForge,
    /// Servidores extra permitidos al descargar archivos de modpacks (pruebas).
    pub(crate) extra_hosts: Vec<String>,
}

impl Core {
    pub fn content(&self) -> Content {
        Content::new(self.clone(), ContentEndpoints::default())
    }
}

impl Content {
    pub fn new(core: Core, endpoints: ContentEndpoints) -> Self {
        Self {
            modrinth: Modrinth::new(core.http().clone(), endpoints.modrinth),
            curseforge: CurseForge::new(core.clone(), endpoints.curseforge),
            core,
            extra_hosts: Vec::new(),
        }
    }

    /// Cambia las URLs base de Modrinth y CurseForge.
    pub fn with_endpoints(self, endpoints: ContentEndpoints) -> Self {
        Self {
            extra_hosts: self.extra_hosts,
            ..Self::new(self.core, endpoints)
        }
    }

    /// Permite descargar archivos de modpacks también de estos hosts (además de
    /// la lista fija). Pensado para las pruebas con un servidor local.
    pub fn with_extra_download_hosts(mut self, hosts: Vec<String>) -> Self {
        self.extra_hosts = hosts;
        self
    }

    pub fn modrinth(&self) -> &Modrinth {
        &self.modrinth
    }

    pub fn curseforge(&self) -> &CurseForge {
        &self.curseforge
    }

    pub(crate) fn index_path(&self, instance_id: &str) -> Result<PathBuf> {
        Ok(self.core.instance_dir(instance_id)?.join("content.json"))
    }

    /// Busca proyectos.
    pub async fn search(&self, q: &ContentQuery) -> Result<Page<ProjectHit>> {
        match q.source {
            Source::Modrinth => self.modrinth.search(q).await,
            Source::CurseForge => self.curseforge.search(q).await,
        }
    }

    /// Ficha de un proyecto (id o slug).
    pub async fn project(&self, source: Source, project: &str) -> Result<ProjectHit> {
        match source {
            Source::Modrinth => self.modrinth.project(project).await,
            Source::CurseForge => self.curseforge.project(project).await,
        }
    }

    /// Varios proyectos por id.
    pub async fn projects(&self, source: Source, ids: &[String]) -> Result<Vec<ProjectHit>> {
        match source {
            Source::Modrinth => self.modrinth.projects(ids).await,
            Source::CurseForge => self.curseforge.projects(ids).await,
        }
    }

    /// Versiones compatibles con `mc` y `loader` (la más nueva primero).
    /// El loader solo se aplica si se pasa: quien llama decide si el tipo de
    /// contenido depende de él (los resource packs y shaders, no).
    pub async fn versions(
        &self,
        source: Source,
        project: &str,
        mc: Option<&str>,
        loader: Option<LoaderKind>,
    ) -> Result<Vec<ContentVersion>> {
        let loaders: Vec<String> = loader
            .map(|l| {
                types::loader_names(l)
                    .into_iter()
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let mut versions = match source {
            Source::Modrinth => {
                self.modrinth
                    .project_versions(project, mc, &loaders)
                    .await?
            }
            Source::CurseForge => {
                self.curseforge
                    .project_versions(project, mc, &loaders)
                    .await?
            }
        };
        versions.retain(|v| is_compatible(v, mc, &loaders));
        versions.sort_by(|a, b| b.date_published.cmp(&a.date_published));
        Ok(versions)
    }

    /// Una versión concreta. CurseForge necesita también el proyecto.
    pub async fn version(
        &self,
        source: Source,
        project: &str,
        version: &str,
    ) -> Result<ContentVersion> {
        match source {
            Source::Modrinth => self.modrinth.version(version).await,
            Source::CurseForge => self.curseforge.version(project, version).await,
        }
    }
}

/// ¿Sirve la versión para ese Minecraft y alguno de esos loaders? Lo que la API
/// no declara (listas vacías) se acepta.
pub(crate) fn is_compatible(v: &ContentVersion, mc: Option<&str>, loaders: &[String]) -> bool {
    let mc_ok = match mc {
        Some(mc) => v.mc_versions.is_empty() || v.mc_versions.iter().any(|x| x == mc),
        None => true,
    };
    let loader_ok = loaders.is_empty()
        || v.loaders.is_empty()
        || v.loaders
            .iter()
            .any(|l| loaders.iter().any(|w| w.eq_ignore_ascii_case(l)));
    mc_ok && loader_ok
}

/// La mejor versión: una release antes que una beta, y la más nueva primero.
pub(crate) fn pick_best(versions: Vec<ContentVersion>) -> Option<ContentVersion> {
    versions.into_iter().max_by(|a, b| {
        a.release
            .rank()
            .cmp(&b.release.rank())
            .then_with(|| a.date_published.cmp(&b.date_published))
    })
}

// Fachada con nombres planos para la CLI y el puente de Flutter.
impl Core {
    pub async fn search_content(&self, q: &ContentQuery) -> Result<Page<ProjectHit>> {
        self.content().search(q).await
    }

    pub async fn content_versions(
        &self,
        source: Source,
        project: &str,
        mc: Option<&str>,
        loader: Option<LoaderKind>,
    ) -> Result<Vec<ContentVersion>> {
        self.content().versions(source, project, mc, loader).await
    }

    pub async fn install_content(
        &self,
        instance: &str,
        source: Source,
        project: &str,
        version: Option<&str>,
        world: Option<&str>,
    ) -> Result<InstallOutcome> {
        self.content()
            .install(instance, source, project, version, world)
            .await
    }

    pub async fn list_content(&self, instance: &str) -> Result<Vec<InstalledContent>> {
        self.content().list(instance).await
    }

    /// Iconos (mejor esfuerzo) de lo instalado: ver `Content::icons`.
    pub async fn content_icons(&self, instance: &str) -> Result<Vec<ContentIcon>> {
        self.content().icons(instance).await
    }

    pub async fn set_content_enabled(
        &self,
        instance: &str,
        file: &str,
        enabled: bool,
    ) -> Result<InstalledContent> {
        self.content().set_enabled(instance, file, enabled).await
    }

    pub async fn remove_content(&self, instance: &str, file: &str) -> Result<InstalledContent> {
        self.content().remove(instance, file).await
    }

    pub async fn check_content_updates(&self, instance: &str) -> Result<Vec<UpdateAvailable>> {
        self.content().check_updates(instance).await
    }

    pub async fn update_content(
        &self,
        instance: &str,
        file: &str,
    ) -> Result<Option<UpdatedContent>> {
        self.content().update(instance, file).await
    }

    pub async fn identify_content(&self, instance: &str) -> Result<IdentifyReport> {
        self.content().identify(instance).await
    }

    pub async fn install_mrpack(
        &self,
        source: &str,
        name: Option<String>,
    ) -> Result<ModpackInstall> {
        self.content().install_mrpack(source, name).await
    }

    pub async fn install_curseforge_pack(
        &self,
        path: &Path,
        name: Option<String>,
    ) -> Result<ModpackInstall> {
        self.content().install_curseforge_pack(path, name).await
    }

    /// Instala un modpack detectando su formato (`.mrpack` o zip de CurseForge).
    pub async fn install_modpack(
        &self,
        source: &str,
        name: Option<String>,
    ) -> Result<ModpackInstall> {
        self.content().install_modpack(source, name).await
    }
}
