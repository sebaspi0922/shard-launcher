//! Modpacks: `.mrpack` de Modrinth y zip de CurseForge. Crean una instancia,
//! descargan los archivos con SHA-1 y extraen los `overrides` con protección
//! contra zip-slip. Si algo falla, la instancia a medias se borra.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::types::{ContentKind, Source};
use super::{Content, ContentFile, ContentIndex, save_index};
use crate::download::{DownloadTask, Verify, download_all, download_one};
use crate::events::{Event, ProgressUnit};
use crate::install::next_task_id;
use crate::instance::{GameVersion, Instance, LoaderKind, NewInstance};
use crate::paths::{safe_join, single_segment};
use crate::{Error, Result};

/// Servidores de los que se aceptan descargas de un `.mrpack`.
pub const MRPACK_HOSTS: &[&str] = &[
    "cdn.modrinth.com",
    "github.com",
    "raw.githubusercontent.com",
    "gitlab.com",
];
/// Dominios de los que se aceptan descargas de un modpack de CurseForge.
pub const CURSEFORGE_HOST_SUFFIXES: &[&str] = &["forgecdn.net", "curseforge.com"];

/// Máximo de bytes que se leen de `modrinth.index.json` / `manifest.json`.
const MAX_INDEX_BYTES: u64 = 32 * 1024 * 1024;
/// Máximo de bytes descomprimidos de los `overrides` (contra bombas zip).
const MAX_OVERRIDES_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct ModpackInstall {
    pub instance: Instance,
    pub files_total: usize,
    pub files_downloaded: usize,
    pub overrides_extracted: usize,
    pub warnings: Vec<String>,
}

#[derive(Deserialize)]
struct MrIndex {
    #[serde(rename = "formatVersion")]
    format_version: u32,
    game: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    files: Vec<MrFile>,
    #[serde(default)]
    dependencies: HashMap<String, String>,
}

#[derive(Deserialize)]
struct MrFile {
    path: String,
    #[serde(default)]
    hashes: HashMap<String, String>,
    #[serde(default)]
    env: Option<MrEnv>,
    #[serde(default)]
    downloads: Vec<String>,
    #[serde(default, rename = "fileSize")]
    file_size: Option<u64>,
}

#[derive(Deserialize)]
struct MrEnv {
    #[serde(default)]
    client: Option<String>,
}

#[derive(Deserialize)]
struct CfManifest {
    minecraft: CfMinecraft,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    files: Vec<CfFile>,
    #[serde(default)]
    overrides: Option<String>,
}

#[derive(Deserialize)]
struct CfMinecraft {
    version: String,
    #[serde(default, rename = "modLoaders")]
    mod_loaders: Vec<CfLoader>,
}

#[derive(Deserialize)]
struct CfLoader {
    id: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Deserialize)]
struct CfFile {
    #[serde(rename = "projectID")]
    project_id: u64,
    #[serde(rename = "fileID")]
    file_id: u64,
    #[serde(default = "default_true")]
    required: bool,
}

fn default_true() -> bool {
    true
}

/// `dependencies` de un `.mrpack` -> versión de Minecraft y loader.
fn mrpack_game(deps: &HashMap<String, String>) -> Result<GameVersion> {
    let mc = deps
        .get("minecraft")
        .ok_or_else(|| Error::InvalidInput("el modpack no indica la versión de Minecraft".into()))?
        .clone();
    let mut found = Vec::new();
    for (key, kind) in [
        ("fabric-loader", LoaderKind::Fabric),
        ("quilt-loader", LoaderKind::Quilt),
        ("forge", LoaderKind::Forge),
        ("neoforge", LoaderKind::NeoForge),
    ] {
        if let Some(v) = deps.get(key) {
            found.push((kind, v.clone()));
        }
    }
    if found.len() > 1 {
        return Err(Error::InvalidInput(
            "el modpack declara varios loaders a la vez".into(),
        ));
    }
    Ok(match found.pop() {
        Some((loader, version)) => GameVersion {
            mc,
            loader,
            loader_version: Some(version),
        },
        None => GameVersion {
            mc,
            loader: LoaderKind::Vanilla,
            loader_version: None,
        },
    })
}

/// `forge-47.2.0` -> (Forge, `47.2.0`).
fn parse_cf_loader(id: &str) -> Option<(LoaderKind, String)> {
    let (name, version) = id.split_once('-')?;
    let kind = match name.to_ascii_lowercase().as_str() {
        "forge" => LoaderKind::Forge,
        "neoforge" => LoaderKind::NeoForge,
        "fabric" => LoaderKind::Fabric,
        "quilt" => LoaderKind::Quilt,
        _ => return None,
    };
    Some((kind, version.to_owned()))
}

/// Ruta del juego -> (tipo, nombre en `content.json`, activo).
fn classify_path(path: &str) -> Option<(ContentKind, String, bool)> {
    let parts: Vec<&str> = path.split('/').collect();
    let (kind, prefix, name) = match parts.as_slice() {
        ["mods", name] => (ContentKind::Mod, None, *name),
        ["resourcepacks", name] => (ContentKind::ResourcePack, None, *name),
        ["shaderpacks", name] => (ContentKind::Shader, None, *name),
        ["saves", world, "datapacks", name] => (ContentKind::Datapack, Some(*world), *name),
        _ => return None,
    };
    let (base, enabled) = match name.strip_suffix(".disabled") {
        Some(b) => (b, false),
        None => (name, true),
    };
    let file = match prefix {
        Some(w) => format!("{w}/{base}"),
        None => base.to_owned(),
    };
    Some((kind, file, enabled))
}

/// `https://cdn.modrinth.com/data/<proyecto>/versions/<versión>/<archivo>`.
fn modrinth_ids(url: &str) -> Option<(String, String)> {
    let u = url::Url::parse(url).ok()?;
    if u.host_str()? != "cdn.modrinth.com" {
        return None;
    }
    let segs: Vec<&str> = u.path_segments()?.collect();
    match segs.as_slice() {
        ["data", project, "versions", version, _, ..] => {
            Some(((*project).to_owned(), (*version).to_owned()))
        }
        _ => None,
    }
}

fn read_zip_entry(archive: &Path, name: &str) -> Result<Vec<u8>> {
    let file = std::fs::File::open(archive).map_err(|e| Error::io(archive, e))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| {
        Error::InvalidInput(format!("{}: no es un zip válido ({e})", archive.display()))
    })?;
    let entry = zip
        .by_name(name)
        .map_err(|_| Error::InvalidInput(format!("el modpack no contiene {name}")))?;
    let mut buf = Vec::new();
    entry
        .take(MAX_INDEX_BYTES + 1)
        .read_to_end(&mut buf)
        .map_err(|e| Error::io(archive, e))?;
    if buf.len() as u64 > MAX_INDEX_BYTES {
        return Err(Error::InvalidInput(format!("{name} es demasiado grande")));
    }
    Ok(buf)
}

/// Extrae `prefixes` (en orden: los posteriores pisan a los anteriores) en
/// `dest`. Devuelve las rutas relativas escritas.
fn extract_overrides(archive: &Path, prefixes: &[String], dest: &Path) -> Result<Vec<String>> {
    let file = std::fs::File::open(archive).map_err(|e| Error::io(archive, e))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| Error::InvalidInput(format!("{}: {e}", archive.display())))?;
    let mut written = Vec::new();
    let mut total = 0u64;
    for prefix in prefixes {
        let prefix_path = Path::new(prefix.trim_matches('/'));
        for i in 0..zip.len() {
            let mut entry = zip
                .by_index(i)
                .map_err(|e| Error::InvalidInput(format!("{}: {e}", archive.display())))?;
            let Some(enclosed) = entry.enclosed_name() else {
                tracing::warn!("entrada insegura del modpack ignorada");
                continue;
            };
            let Ok(rel) = enclosed.strip_prefix(prefix_path) else {
                continue;
            };
            if rel.as_os_str().is_empty() {
                continue;
            }
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            // Defensa extra además de `enclosed_name`.
            let out = safe_join(dest, &rel_str)?;
            if entry.is_dir() {
                std::fs::create_dir_all(&out).map_err(|e| Error::io(&out, e))?;
                continue;
            }
            total = total.saturating_add(entry.size());
            if total > MAX_OVERRIDES_BYTES {
                return Err(Error::InvalidInput(
                    "los overrides del modpack son demasiado grandes".into(),
                ));
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            let mut f = std::fs::File::create(&out).map_err(|e| Error::io(&out, e))?;
            std::io::copy(&mut entry, &mut f).map_err(|e| Error::io(&out, e))?;
            written.push(rel_str);
        }
    }
    Ok(written)
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))?
}

impl Content {
    fn host_allowed(&self, url: &str, exact: &[&str], suffixes: &[&str]) -> bool {
        let Ok(u) = url::Url::parse(url) else {
            return false;
        };
        let Some(host) = u.host_str().map(str::to_ascii_lowercase) else {
            return false;
        };
        if self
            .extra_hosts
            .iter()
            .any(|h| h.eq_ignore_ascii_case(&host))
        {
            return matches!(u.scheme(), "http" | "https");
        }
        u.scheme() == "https"
            && (exact.contains(&host.as_str())
                || suffixes
                    .iter()
                    .any(|s| host == *s || host.ends_with(&format!(".{s}"))))
    }

    /// Trae el modpack a disco: una ruta se usa tal cual; una URL se descarga a
    /// `<datos>/cache/modpacks/`.
    async fn fetch_pack(&self, source: &str) -> Result<PathBuf> {
        if source.starts_with("http://") || source.starts_with("https://") {
            let url = url::Url::parse(source)
                .map_err(|e| Error::InvalidInput(format!("URL no válida: {e}")))?;
            let raw = url
                .path_segments()
                .and_then(|mut s| s.next_back())
                .unwrap_or_default();
            let clean: String = raw
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let name = if clean.is_empty() || clean.starts_with('.') {
                "modpack.zip".to_owned()
            } else {
                clean
            };
            let dir = self.core.paths().root().join("cache").join("modpacks");
            tokio::fs::create_dir_all(&dir)
                .await
                .map_err(|e| Error::io(&dir, e))?;
            let dest = dir.join(format!("{:08x}-{name}", fastrand::u32(..)));
            download_one(
                self.core.http(),
                &DownloadTask {
                    url: source.to_owned(),
                    dest: dest.clone(),
                    sha1: None,
                    size: None,
                },
            )
            .await?;
            Ok(dest)
        } else {
            let path = PathBuf::from(source);
            if !tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_file()) {
                return Err(Error::InvalidInput(format!(
                    "no existe el archivo {}",
                    path.display()
                )));
            }
            Ok(path)
        }
    }

    /// Instala un modpack (ruta o URL) detectando si es de Modrinth o de CurseForge.
    pub async fn install_modpack(
        &self,
        source: &str,
        name: Option<String>,
    ) -> Result<ModpackInstall> {
        let pack = self.fetch_pack(source).await?;
        let (is_mr, is_cf) = {
            let pack = pack.clone();
            blocking(move || {
                let file = std::fs::File::open(&pack).map_err(|e| Error::io(&pack, e))?;
                let mut zip = zip::ZipArchive::new(file).map_err(|e| {
                    Error::InvalidInput(format!("{}: no es un zip válido ({e})", pack.display()))
                })?;
                let is_mr = zip.by_name("modrinth.index.json").is_ok();
                let is_cf = zip.by_name("manifest.json").is_ok();
                Ok((is_mr, is_cf))
            })
            .await?
        };
        match (is_mr, is_cf) {
            (true, _) => self.install_mrpack_file(pack, name).await,
            (false, true) => self.install_curseforge_pack(&pack, name).await,
            _ => Err(Error::InvalidInput(
                "el archivo no es un modpack (falta modrinth.index.json o manifest.json)".into(),
            )),
        }
    }

    /// Instala un `.mrpack` (ruta o URL) en una instancia nueva.
    pub async fn install_mrpack(
        &self,
        source: &str,
        name: Option<String>,
    ) -> Result<ModpackInstall> {
        let pack = self.fetch_pack(source).await?;
        self.install_mrpack_file(pack, name).await
    }

    async fn install_mrpack_file(
        &self,
        pack: PathBuf,
        name: Option<String>,
    ) -> Result<ModpackInstall> {
        let raw = {
            let pack = pack.clone();
            blocking(move || read_zip_entry(&pack, "modrinth.index.json")).await?
        };
        let index: MrIndex =
            serde_json::from_slice(&raw).map_err(|e| Error::json("modrinth.index.json", e))?;
        if index.format_version != 1 {
            return Err(Error::InvalidInput(format!(
                "formatVersion {} no soportado (solo 1)",
                index.format_version
            )));
        }
        if index.game != "minecraft" {
            return Err(Error::InvalidInput(format!(
                "el modpack es para «{}», no para Minecraft",
                index.game
            )));
        }
        let game = mrpack_game(&index.dependencies)?;

        // Todo se valida antes de crear nada.
        let mut warnings = Vec::new();
        let mut planned: Vec<(String, MrFile, String)> = Vec::new();
        let mut seen_paths = HashSet::new();
        for f in index.files {
            safe_join(Path::new("/"), &f.path).map_err(|_| {
                Error::InvalidPath(format!("el modpack trae una ruta insegura: {:?}", f.path))
            })?;
            if !seen_paths.insert(f.path.clone()) {
                return Err(Error::InvalidInput(format!(
                    "el modpack repite la ruta {:?}",
                    f.path
                )));
            }
            if f.env
                .as_ref()
                .and_then(|e| e.client.as_deref())
                .is_some_and(|c| c == "unsupported")
            {
                continue;
            }
            if !f.hashes.contains_key("sha1") {
                return Err(Error::InvalidInput(format!(
                    "{}: el modpack no trae SHA-1",
                    f.path
                )));
            }
            let Some(url) = f
                .downloads
                .iter()
                .find(|u| self.host_allowed(u, MRPACK_HOSTS, &[]))
                .cloned()
            else {
                warnings.push(format!(
                    "{}: se omite, ninguna URL de descarga es de un servidor permitido",
                    f.path
                ));
                continue;
            };
            planned.push((f.path.clone(), f, url));
        }

        let display_name = name
            .or(index.name.clone())
            .or_else(|| pack.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "Modpack".into());
        let instance = self
            .core
            .create_instance(NewInstance {
                name: display_name,
                game,
            })
            .await?;
        let game_dir = self.core.instance_game_dir(&instance.id)?;

        let result = async {
            let mut tasks = Vec::new();
            let mut entries: Vec<(String, ContentFile)> = Vec::new();
            for (path, f, url) in &planned {
                let sha1 = f.hashes.get("sha1").cloned();
                tasks.push(DownloadTask {
                    url: url.clone(),
                    dest: safe_join(&game_dir, path)?,
                    sha1: sha1.clone(),
                    size: f.file_size,
                });
                if let Some((kind, file, enabled)) = classify_path(path) {
                    let (project_id, version_id, source) = match modrinth_ids(url) {
                        Some((p, v)) => (Some(p), Some(v), Source::Modrinth.as_str()),
                        None => (None, None, super::SOURCE_UNKNOWN),
                    };
                    entries.push((
                        path.clone(),
                        ContentFile {
                            kind: kind.as_str().into(),
                            file,
                            enabled,
                            source: source.into(),
                            project_id,
                            project_name: None,
                            version_id,
                            version_number: None,
                            sha1,
                            url: Some(url.clone()),
                            category: None,
                        },
                    ));
                }
            }
            let prefixes = vec!["overrides".to_owned(), "client-overrides".to_owned()];
            self.finish_pack(&instance, &pack, tasks, entries, prefixes)
                .await
        }
        .await;

        match result {
            Ok((downloaded, overrides)) => Ok(ModpackInstall {
                instance,
                files_total: planned.len(),
                files_downloaded: downloaded,
                overrides_extracted: overrides,
                warnings,
            }),
            Err(e) => {
                self.discard_instance(&instance.id).await;
                Err(e)
            }
        }
    }

    /// Instala un modpack de CurseForge (zip con `manifest.json`). Necesita la clave de API.
    pub async fn install_curseforge_pack(
        &self,
        path: &Path,
        name: Option<String>,
    ) -> Result<ModpackInstall> {
        let pack = path.to_path_buf();
        let raw = {
            let pack = pack.clone();
            blocking(move || read_zip_entry(&pack, "manifest.json")).await?
        };
        let manifest: CfManifest =
            serde_json::from_slice(&raw).map_err(|e| Error::json("manifest.json", e))?;
        let loader = manifest
            .minecraft
            .mod_loaders
            .iter()
            .find(|l| l.primary)
            .or_else(|| manifest.minecraft.mod_loaders.first())
            .and_then(|l| parse_cf_loader(&l.id));
        let game = GameVersion {
            mc: manifest.minecraft.version.clone(),
            loader: loader.as_ref().map_or(LoaderKind::Vanilla, |(k, _)| *k),
            loader_version: loader.map(|(_, v)| v),
        };
        let overrides_dir = manifest
            .overrides
            .clone()
            .unwrap_or_else(|| "overrides".into());
        single_segment(&overrides_dir).map_err(|_| {
            Error::InvalidPath(format!("carpeta de overrides no válida: {overrides_dir:?}"))
        })?;

        // Resolver archivos y proyectos antes de crear la instancia.
        let mut warnings = Vec::new();
        let mut planned: Vec<(String, super::ContentVersion, super::ProjectHit)> = Vec::new();
        if !manifest.files.is_empty() {
            if !self.core.has_curseforge_key().await? {
                return Err(Error::CurseForgeKeyMissing);
            }
            let ids: Vec<u64> = manifest.files.iter().map(|f| f.file_id).collect();
            let versions: HashMap<String, super::ContentVersion> = self
                .curseforge
                .files_bulk(&ids)
                .await?
                .into_iter()
                .map(|v| (v.id.clone(), v))
                .collect();
            let mut mod_ids: Vec<String> = manifest
                .files
                .iter()
                .map(|f| f.project_id.to_string())
                .collect();
            mod_ids.sort();
            mod_ids.dedup();
            let projects: HashMap<String, super::ProjectHit> = self
                .curseforge
                .projects(&mod_ids)
                .await?
                .into_iter()
                .map(|p| (p.id.clone(), p))
                .collect();
            for f in &manifest.files {
                let Some(version) = versions.get(&f.file_id.to_string()) else {
                    let msg = format!(
                        "el archivo {} del proyecto {} ya no existe en CurseForge",
                        f.file_id, f.project_id
                    );
                    if f.required {
                        return Err(Error::InvalidInput(msg));
                    }
                    warnings.push(msg);
                    continue;
                };
                let project = projects
                    .get(&f.project_id.to_string())
                    .cloned()
                    .unwrap_or_else(|| super::ProjectHit {
                        source: Source::CurseForge,
                        id: f.project_id.to_string(),
                        slug: String::new(),
                        kind: Some(ContentKind::Mod),
                        title: version.name.clone(),
                        description: String::new(),
                        author: None,
                        icon_url: None,
                        downloads: 0,
                        follows: 0,
                        categories: Vec::new(),
                        date_modified: None,
                    });
                let Some(file) = version.primary_file() else {
                    warnings.push(format!("{}: sin archivo", project.title));
                    continue;
                };
                if single_segment(&file.filename).is_err() {
                    return Err(Error::InvalidPath(format!(
                        "nombre de archivo no válido: {:?}",
                        file.filename
                    )));
                }
                let Some(folder) = project.kind.and_then(ContentKind::folder) else {
                    warnings.push(format!(
                        "{}: tipo de contenido sin carpeta propia; se omite",
                        project.title
                    ));
                    continue;
                };
                if !self.host_allowed(&file.url, &[], CURSEFORGE_HOST_SUFFIXES) {
                    warnings.push(format!(
                        "{}: se omite, la descarga no es de un servidor de CurseForge",
                        file.filename
                    ));
                    continue;
                }
                if file.fallback_url {
                    warnings.push(format!(
                        "{}: sin URL oficial; se usa la del CDN, que puede fallar",
                        file.filename
                    ));
                }
                planned.push((
                    format!("{folder}/{}", file.filename),
                    version.clone(),
                    project,
                ));
            }
        }

        let display_name = name
            .or(manifest.name.clone())
            .or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "Modpack".into());
        let instance = self
            .core
            .create_instance(NewInstance {
                name: display_name,
                game,
            })
            .await?;
        let game_dir = self.core.instance_game_dir(&instance.id)?;

        let result = async {
            let mut tasks = Vec::new();
            let mut entries = Vec::new();
            for (rel, version, project) in &planned {
                let file = version.primary_file().expect("comprobado antes");
                tasks.push(DownloadTask {
                    url: file.url.clone(),
                    dest: safe_join(&game_dir, rel)?,
                    sha1: file.sha1.clone(),
                    size: (file.size > 0).then_some(file.size),
                });
                if let Some((kind, name, enabled)) = classify_path(rel) {
                    entries.push((
                        rel.clone(),
                        ContentFile {
                            kind: kind.as_str().into(),
                            file: name,
                            enabled,
                            source: Source::CurseForge.as_str().into(),
                            project_id: Some(project.id.clone()),
                            project_name: Some(project.title.clone()),
                            version_id: Some(version.id.clone()),
                            version_number: Some(version.version_number.clone()),
                            sha1: file.sha1.clone(),
                            url: Some(file.url.clone()),
                            category: None,
                        },
                    ));
                }
            }
            self.finish_pack(
                &instance,
                &pack,
                tasks,
                entries,
                vec![overrides_dir.clone()],
            )
            .await
        }
        .await;

        match result {
            Ok((downloaded, overrides)) => Ok(ModpackInstall {
                instance,
                files_total: planned.len(),
                files_downloaded: downloaded,
                overrides_extracted: overrides,
                warnings,
            }),
            Err(e) => {
                self.discard_instance(&instance.id).await;
                Err(e)
            }
        }
    }

    /// Descarga, extrae los overrides y escribe `content.json`.
    async fn finish_pack(
        &self,
        instance: &Instance,
        pack: &Path,
        tasks: Vec<DownloadTask>,
        entries: Vec<(String, ContentFile)>,
        override_prefixes: Vec<String>,
    ) -> Result<(usize, usize)> {
        let game_dir = self.core.instance_game_dir(&instance.id)?;
        let task_id = next_task_id();
        let events = self.core.inner.events.clone();
        let concurrency = self.core.settings().await.max_concurrent_downloads;
        let ev = events.clone();
        let downloaded = download_all(
            self.core.http(),
            tasks,
            concurrency,
            Verify::Size,
            move |done, total| {
                ev.emit(Event::Progress {
                    task_id,
                    stage: "modpack".into(),
                    done,
                    total,
                    unit: ProgressUnit::Files,
                });
            },
        )
        .await?;

        let written = {
            let pack = pack.to_path_buf();
            let game_dir = game_dir.clone();
            blocking(move || extract_overrides(&pack, &override_prefixes, &game_dir)).await?
        };
        let overwritten: HashSet<&str> = written.iter().map(String::as_str).collect();
        // Un override que pisa un archivo descargado invalida su procedencia.
        let files: Vec<ContentFile> = entries
            .into_iter()
            .filter(|(path, _)| !overwritten.contains(path.as_str()))
            .map(|(_, e)| e)
            .collect();
        let index = ContentIndex { schema: 1, files };
        save_index(&self.index_path(&instance.id)?, &index).await?;
        events.emit(Event::TaskFinished {
            task_id,
            ok: true,
            message: None,
        });
        Ok((downloaded, written.len()))
    }

    /// Borra del todo la instancia a medio crear (sin pasar por la papelera).
    async fn discard_instance(&self, id: &str) {
        if let Ok(dir) = self.core.instance_dir(id)
            && let Err(e) = tokio::fs::remove_dir_all(&dir).await
        {
            tracing::warn!(error = %e, "no se pudo limpiar la instancia a medias");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_de_modrinth_desde_la_url_del_cdn() {
        assert_eq!(
            modrinth_ids("https://cdn.modrinth.com/data/AANobbMI/versions/ABC123/sodium.jar"),
            Some(("AANobbMI".into(), "ABC123".into()))
        );
        assert_eq!(modrinth_ids("https://github.com/x/y/z.jar"), None);
        assert_eq!(modrinth_ids("https://cdn.modrinth.com/otra/ruta.jar"), None);
        assert_eq!(modrinth_ids("no es una url"), None);
    }

    #[test]
    fn clasifica_rutas_del_juego() {
        use ContentKind::*;
        assert_eq!(
            classify_path("mods/a.jar"),
            Some((Mod, "a.jar".into(), true))
        );
        assert_eq!(
            classify_path("mods/a.jar.disabled"),
            Some((Mod, "a.jar".into(), false))
        );
        assert_eq!(
            classify_path("shaderpacks/s.zip"),
            Some((Shader, "s.zip".into(), true))
        );
        assert_eq!(
            classify_path("saves/Mundo/datapacks/d.zip"),
            Some((Datapack, "Mundo/d.zip".into(), true))
        );
        assert_eq!(classify_path("config/a.toml"), None);
        assert_eq!(classify_path("mods/sub/a.jar"), None);
    }

    #[test]
    fn loader_del_mrpack() {
        let deps = |pairs: &[(&str, &str)]| {
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect::<HashMap<_, _>>()
        };
        let g = mrpack_game(&deps(&[("minecraft", "1.20.1"), ("neoforge", "21.1.1")])).unwrap();
        assert_eq!(
            (g.loader, g.loader_version.as_deref()),
            (LoaderKind::NeoForge, Some("21.1.1"))
        );
        let g = mrpack_game(&deps(&[("minecraft", "1.20.1"), ("quilt-loader", "0.26")])).unwrap();
        assert_eq!(g.loader, LoaderKind::Quilt);
        let g = mrpack_game(&deps(&[("minecraft", "1.8.9")])).unwrap();
        assert_eq!(g.loader, LoaderKind::Vanilla);
        assert!(mrpack_game(&deps(&[("fabric-loader", "1")])).is_err());
        assert!(
            mrpack_game(&deps(&[
                ("minecraft", "1"),
                ("forge", "1"),
                ("neoforge", "1")
            ]))
            .is_err()
        );
    }

    #[test]
    fn loader_del_manifest_de_curseforge() {
        assert_eq!(
            parse_cf_loader("forge-47.2.0"),
            Some((LoaderKind::Forge, "47.2.0".into()))
        );
        assert_eq!(
            parse_cf_loader("fabric-0.15.11"),
            Some((LoaderKind::Fabric, "0.15.11".into()))
        );
        assert_eq!(parse_cf_loader("raro-1"), None);
    }
}
