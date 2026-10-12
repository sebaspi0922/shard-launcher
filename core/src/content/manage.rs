//! Contenido ya instalado: listar, activar/desactivar y quitar.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::types::ContentKind;
use super::{Content, SOURCE_UNKNOWN, load_index, update_index};
use crate::paths::{safe_join, single_segment};
use crate::{Error, Result};

const DISABLED_SUFFIX: &str = ".disabled";

/// Un archivo de contenido de la instancia, mezcla de disco y `content.json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InstalledContent {
    pub kind: ContentKind,
    /// Nombre sin `.disabled`; en los datapacks, `<mundo>/<archivo>`.
    pub file: String,
    /// Estado real en disco (manda sobre `content.json`).
    pub enabled: bool,
    /// `modrinth`, `curseforge` o `unknown`.
    pub source: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub version_id: Option<String>,
    pub version_number: Option<String>,
    pub sha1: Option<String>,
    pub url: Option<String>,
    pub size: u64,
    pub is_dir: bool,
    /// Está en `content.json`.
    pub tracked: bool,
    pub world: Option<String>,
}

/// Ruta del archivo activo (sin `.disabled`) de una entrada.
pub(crate) fn content_path(game_dir: &Path, kind: ContentKind, file: &str) -> Result<PathBuf> {
    match kind {
        ContentKind::Datapack => {
            let (world, name) = file
                .split_once('/')
                .ok_or_else(|| Error::InvalidPath(format!("datapack sin mundo: {file:?}")))?;
            Ok(game_dir
                .join("saves")
                .join(single_segment(world)?)
                .join("datapacks")
                .join(single_segment(name)?))
        }
        ContentKind::Modpack => Err(Error::InvalidInput(
            "un modpack no es un archivo de contenido".into(),
        )),
        other => {
            let folder = other.folder().expect("tiene carpeta");
            safe_join(&game_dir.join(folder), file)
        }
    }
}

pub(crate) fn disabled_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(DISABLED_SUFFIX);
    path.with_file_name(name)
}

/// Ruta real en disco según esté activo o no.
pub(crate) fn disk_path(
    game_dir: &Path,
    kind: ContentKind,
    file: &str,
    enabled: bool,
) -> Result<PathBuf> {
    let p = content_path(game_dir, kind, file)?;
    Ok(if enabled { p } else { disabled_path(&p) })
}

/// Entrada de la carpeta: ¿es contenido? Devuelve el nombre base y si está activo.
fn classify(kind: ContentKind, name: &str, is_dir: bool) -> Option<(String, bool)> {
    if name.starts_with('.') || name.ends_with(".part") {
        return None;
    }
    let (base, enabled) = match name.strip_suffix(DISABLED_SUFFIX) {
        Some(b) => (b, false),
        None => (name, true),
    };
    let ext = Path::new(base)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let ok = match kind {
        ContentKind::Mod => !is_dir && matches!(ext.as_deref(), Some("jar" | "zip" | "litemod")),
        ContentKind::ResourcePack | ContentKind::Shader | ContentKind::Datapack => {
            is_dir || ext.as_deref() == Some("zip")
        }
        ContentKind::Modpack => false,
    };
    ok.then(|| (base.to_owned(), enabled))
}

struct DiskEntry {
    kind: ContentKind,
    file: String,
    enabled: bool,
    size: u64,
    is_dir: bool,
    world: Option<String>,
}

async fn scan_dir(
    dir: &Path,
    kind: ContentKind,
    world: Option<&str>,
    out: &mut Vec<DiskEntry>,
) -> Result<()> {
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(Error::io(dir, e)),
    };
    while let Some(entry) = entries.next_entry().await.map_err(|e| Error::io(dir, e))? {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(meta) = tokio::fs::metadata(entry.path()).await else {
            continue;
        };
        let Some((base, enabled)) = classify(kind, &name, meta.is_dir()) else {
            continue;
        };
        let file = match world {
            Some(w) => format!("{w}/{base}"),
            None => base,
        };
        // `a.jar` y `a.jar.disabled` a la vez: se queda el activo.
        if let Some(prev) = out.iter_mut().find(|d| d.kind == kind && d.file == file) {
            if enabled && !prev.enabled {
                prev.enabled = true;
                prev.size = meta.len();
                prev.is_dir = meta.is_dir();
            }
            continue;
        }
        out.push(DiskEntry {
            kind,
            file,
            enabled,
            size: if meta.is_dir() { 0 } else { meta.len() },
            is_dir: meta.is_dir(),
            world: world.map(str::to_owned),
        });
    }
    Ok(())
}

async fn scan_disk(game_dir: &Path) -> Result<Vec<DiskEntry>> {
    let mut out = Vec::new();
    for kind in [
        ContentKind::Mod,
        ContentKind::ResourcePack,
        ContentKind::Shader,
    ] {
        let dir = game_dir.join(kind.folder().expect("tiene carpeta"));
        scan_dir(&dir, kind, None, &mut out).await?;
    }
    let saves = game_dir.join("saves");
    if let Ok(mut worlds) = tokio::fs::read_dir(&saves).await {
        while let Ok(Some(w)) = worlds.next_entry().await {
            let name = w.file_name().to_string_lossy().into_owned();
            if single_segment(&name).is_err() || !w.path().is_dir() {
                continue;
            }
            let dir = w.path().join("datapacks");
            scan_dir(&dir, ContentKind::Datapack, Some(&name), &mut out).await?;
        }
    }
    Ok(out)
}

/// Busca `query` (con o sin `.disabled`, con o sin prefijo `tipo/`) en la lista.
pub(crate) fn resolve_one<'a>(
    list: &'a [InstalledContent],
    query: &str,
) -> Result<&'a InstalledContent> {
    let q = query.strip_suffix(DISABLED_SUFFIX).unwrap_or(query);
    let found: Vec<_> = list
        .iter()
        .filter(|c| c.file == q || format!("{}/{}", c.kind.as_str(), c.file) == q)
        .collect();
    match found.as_slice() {
        [one] => Ok(one),
        [] => Err(Error::InvalidInput(format!(
            "no hay contenido llamado «{query}» en la instancia"
        ))),
        _ => Err(Error::InvalidInput(format!(
            "«{query}» es ambiguo: indica el tipo (p. ej. mod/{q})"
        ))),
    }
}

impl Content {
    /// Archivos de `mods/`, `resourcepacks/`, `shaderpacks/` y los datapacks de
    /// cada mundo, con la procedencia de `content.json` cuando la hay.
    pub async fn list(&self, instance: &str) -> Result<Vec<InstalledContent>> {
        let inst = self.core.find_instance(instance).await?;
        let game_dir = self.core.instance_game_dir(&inst.id)?;
        let index = load_index(&self.index_path(&inst.id)?).await?;
        let disk = scan_disk(&game_dir).await?;
        let mut out: Vec<InstalledContent> = disk
            .into_iter()
            .map(|d| {
                let entry = index.find(d.kind.as_str(), &d.file);
                InstalledContent {
                    kind: d.kind,
                    enabled: d.enabled,
                    source: entry
                        .map(|e| e.source.clone())
                        .unwrap_or_else(|| SOURCE_UNKNOWN.into()),
                    project_id: entry.and_then(|e| e.project_id.clone()),
                    project_name: entry.and_then(|e| e.project_name.clone()),
                    version_id: entry.and_then(|e| e.version_id.clone()),
                    version_number: entry.and_then(|e| e.version_number.clone()),
                    sha1: entry.and_then(|e| e.sha1.clone()),
                    url: entry.and_then(|e| e.url.clone()),
                    size: d.size,
                    is_dir: d.is_dir,
                    tracked: entry.is_some(),
                    world: d.world,
                    file: d.file,
                }
            })
            .collect();
        out.sort_by(|a, b| {
            (a.kind.as_str(), a.file.to_lowercase()).cmp(&(b.kind.as_str(), b.file.to_lowercase()))
        });
        Ok(out)
    }

    /// Activa o desactiva un archivo (`<f>` ↔ `<f>.disabled`).
    pub async fn set_enabled(
        &self,
        instance: &str,
        file: &str,
        enabled: bool,
    ) -> Result<InstalledContent> {
        let inst = self.core.find_instance(instance).await?;
        let game_dir = self.core.instance_game_dir(&inst.id)?;
        let list = self.list(&inst.id).await?;
        let mut item = resolve_one(&list, file)?.clone();
        if item.enabled == enabled {
            return Ok(item);
        }
        let active = content_path(&game_dir, item.kind, &item.file)?;
        let disabled = disabled_path(&active);
        let (from, to) = if enabled {
            (&disabled, &active)
        } else {
            (&active, &disabled)
        };
        if tokio::fs::try_exists(to).await.unwrap_or(false) {
            return Err(Error::InvalidInput(format!(
                "no se puede renombrar: ya existe {}",
                to.display()
            )));
        }
        tokio::fs::rename(from, to)
            .await
            .map_err(|e| Error::io(from, e))?;
        item.enabled = enabled;
        let (kind, name) = (item.kind.as_str(), item.file.clone());
        update_index(&self.index_path(&inst.id)?, |idx| {
            if let Some(e) = idx
                .files
                .iter_mut()
                .find(|f| f.kind == kind && f.file == name)
            {
                e.enabled = enabled;
            }
            Ok(())
        })
        .await?;
        Ok(item)
    }

    /// Borra el archivo y su entrada de `content.json`.
    pub async fn remove(&self, instance: &str, file: &str) -> Result<InstalledContent> {
        let inst = self.core.find_instance(instance).await?;
        let game_dir = self.core.instance_game_dir(&inst.id)?;
        let list = self.list(&inst.id).await?;
        let item = resolve_one(&list, file)?.clone();
        let path = disk_path(&game_dir, item.kind, &item.file, item.enabled)?;
        let result = if item.is_dir {
            tokio::fs::remove_dir_all(&path).await
        } else {
            tokio::fs::remove_file(&path).await
        };
        result.map_err(|e| Error::io(&path, e))?;
        let (kind, name) = (item.kind.as_str(), item.file.clone());
        update_index(&self.index_path(&inst.id)?, |idx| {
            idx.remove(kind, &name);
            Ok(())
        })
        .await?;
        Ok(item)
    }
}
