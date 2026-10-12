//! Instalación de una versión: cliente, librerías, natives antiguos y assets.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Deserialize;

use crate::download::{DownloadTask, Verify, download_all, sha1_file};
use crate::events::{Event, ProgressUnit};
use crate::fsutil::{read_json, write_atomic};
use crate::launcher::Core;
use crate::meta::VersionInfo;
use crate::meta::library::{LibraryKind, ResolvedLibrary};
use crate::meta::rules::{Features, Platform};
use crate::paths::safe_join;
use crate::{Error, Result};

const RESOURCES_URL: &str = "https://resources.download.minecraft.net";

static TASK_IDS: AtomicU64 = AtomicU64::new(1);

pub fn next_task_id() -> u64 {
    TASK_IDS.fetch_add(1, Ordering::Relaxed)
}

/// Resultado de instalar una versión: todo lo necesario para lanzar.
#[derive(Debug, Clone)]
pub struct Installed {
    pub version: VersionInfo,
    pub client_jar: PathBuf,
    /// Librerías del classpath en orden (sin el cliente).
    pub classpath: Vec<PathBuf>,
    pub natives_dir: PathBuf,
    pub assets: AssetsLayout,
}

#[derive(Debug, Clone)]
pub struct AssetsLayout {
    pub index_name: String,
    /// `${assets_root}`.
    pub root: PathBuf,
    /// `${game_assets}`: `assets/virtual/<id>` en versiones muy antiguas.
    pub game_assets: PathBuf,
    /// Índices antiguos (`map_to_resources`): copiar a `<gameDir>/resources`.
    pub map_to_resources: bool,
}

#[derive(Debug, Deserialize)]
struct AssetIndex {
    objects: BTreeMap<String, AssetObject>,
    #[serde(default, rename = "virtual")]
    is_virtual: bool,
    #[serde(default)]
    map_to_resources: bool,
}

#[derive(Debug, Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}

impl Core {
    /// Asegura que la versión (ya con loader) está completa en disco.
    pub async fn install_version(&self, version_id: &str, verify: Verify) -> Result<Installed> {
        let version = self.resolve_version(version_id).await?;
        let platform = Platform::current();
        let paths = self.paths();
        let concurrency = self.settings().await.max_concurrent_downloads;
        let task_id = next_task_id();

        // Cliente: `versions/<id-base>/<id-base>.jar` (la versión con `downloads.client`).
        let base_id = self.base_version_id(version_id).await?;
        let client_jar = paths
            .versions_dir()
            .join(&base_id)
            .join(format!("{base_id}.jar"));
        let client = version
            .downloads
            .as_ref()
            .and_then(|d| d.client.clone())
            .ok_or_else(|| {
                Error::InvalidInput(format!("{version_id} no tiene cliente descargable"))
            })?;

        let libs: Vec<ResolvedLibrary> = version
            .libraries
            .iter()
            .flat_map(|l| l.resolve(&platform, &Features::new()))
            .collect();

        let mut tasks = vec![DownloadTask {
            url: client.url.clone(),
            dest: client_jar.clone(),
            sha1: client.sha1.clone(),
            size: client.size,
        }];
        let mut missing_generated = Vec::new();
        for lib in &libs {
            let dest = safe_join(&paths.libraries_dir(), &lib.path)?;
            match &lib.url {
                Some(url) => tasks.push(DownloadTask {
                    url: url.clone(),
                    dest,
                    sha1: lib.sha1.clone(),
                    size: lib.size,
                }),
                // Las genera el instalador del loader (Forge/NeoForge).
                None if !dest.exists() => missing_generated.push(lib.name.clone()),
                None => {}
            }
        }
        if !missing_generated.is_empty() {
            return Err(Error::InvalidInput(format!(
                "faltan librerías que genera el instalador del loader: {}",
                missing_generated.join(", ")
            )));
        }

        let events = self.inner.events.clone();
        download_all(
            self.http(),
            tasks,
            concurrency,
            verify,
            move |done, total| {
                events.emit(Event::Progress {
                    task_id,
                    stage: "libraries".into(),
                    done,
                    total,
                    unit: ProgressUnit::Files,
                });
            },
        )
        .await?;

        // Como el launcher oficial: el cliente se usa con el nombre de la versión
        // lanzada (`versions/<id>/<id>.jar`). NeoForge/Forge 1.17+ lo necesitan
        // para excluirlo con `-DignoreList=…,${version_name}.jar`.
        let client_jar = if version.id != base_id {
            let target = paths
                .versions_dir()
                .join(&version.id)
                .join(format!("{}.jar", version.id));
            link_or_copy(&client_jar, &target).await?;
            target
        } else {
            client_jar
        };

        let natives_dir = paths.natives_dir().join(&version.id);
        self.extract_natives(&libs, &natives_dir).await?;
        let assets = self.install_assets(&version, verify, task_id).await?;

        let classpath = libs
            .iter()
            .filter(|l| l.kind == LibraryKind::Classpath)
            .map(|l| safe_join(&paths.libraries_dir(), &l.path))
            .collect::<Result<Vec<_>>>()?;
        Ok(Installed {
            version,
            client_jar,
            classpath: dedup(classpath),
            natives_dir,
            assets,
        })
    }

    /// Primera versión de la cadena `inheritsFrom` que trae `downloads.client`.
    async fn base_version_id(&self, version_id: &str) -> Result<String> {
        let mut current = version_id.to_owned();
        for _ in 0..8 {
            let info = match self.installed_version_json(&current).await? {
                Some(i) => i,
                None => return Ok(current),
            };
            if info
                .downloads
                .as_ref()
                .and_then(|d| d.client.as_ref())
                .is_some()
            {
                return Ok(current);
            }
            match info.inherits_from {
                Some(p) => current = p,
                None => return Ok(current),
            }
        }
        Ok(current)
    }

    /// Extrae los natives antiguos (LWJGL 2) una sola vez por versión.
    async fn extract_natives(&self, libs: &[ResolvedLibrary], natives_dir: &Path) -> Result<()> {
        let natives: Vec<(PathBuf, Vec<String>)> = libs
            .iter()
            .filter_map(|l| match &l.kind {
                LibraryKind::Natives { exclude } => Some((l, exclude.clone())),
                LibraryKind::Classpath => None,
            })
            .map(|(l, ex)| Ok((safe_join(&self.paths().libraries_dir(), &l.path)?, ex)))
            .collect::<Result<_>>()?;
        tokio::fs::create_dir_all(natives_dir)
            .await
            .map_err(|e| Error::io(natives_dir, e))?;
        if natives.is_empty() {
            return Ok(());
        }
        let mut stamp = String::new();
        for (jar, _) in &natives {
            stamp.push_str(&sha1_file(jar).await?);
            stamp.push('\n');
        }
        let stamp_path = natives_dir.join(".extracted");
        if tokio::fs::read_to_string(&stamp_path).await.ok().as_deref() == Some(stamp.as_str()) {
            return Ok(());
        }
        let dir = natives_dir.to_path_buf();
        tokio::task::spawn_blocking(move || -> Result<()> {
            for (jar, exclude) in natives {
                extract_zip(&jar, &dir, &exclude)?;
            }
            Ok(())
        })
        .await
        .map_err(|e| Error::InvalidInput(format!("extracción interrumpida: {e}")))??;
        write_atomic(&stamp_path, stamp.as_bytes()).await
    }

    async fn install_assets(
        &self,
        version: &VersionInfo,
        verify: Verify,
        task_id: u64,
    ) -> Result<AssetsLayout> {
        let root = self.paths().assets_dir();
        let index_ref = version.asset_index.clone().ok_or_else(|| {
            Error::InvalidInput(format!("{} no tiene índice de assets", version.id))
        })?;
        let index_path = root.join("indexes").join(format!("{}.json", index_ref.id));
        let concurrency = self.settings().await.max_concurrent_downloads;
        download_all(
            self.http(),
            vec![DownloadTask {
                url: index_ref.url.clone(),
                dest: index_path.clone(),
                sha1: index_ref.sha1.clone(),
                size: index_ref.size,
            }],
            1,
            Verify::Full,
            |_, _| {},
        )
        .await?;
        let index: AssetIndex = read_json(&index_path)
            .await?
            .ok_or_else(|| Error::InvalidInput("índice de assets ausente".into()))?;

        let objects = root.join("objects");
        let tasks: Vec<DownloadTask> = index
            .objects
            .values()
            .filter(|o| o.hash.len() > 2 && o.hash.chars().all(|c| c.is_ascii_hexdigit()))
            .map(|o| {
                let prefix = &o.hash[..2];
                DownloadTask {
                    url: format!("{RESOURCES_URL}/{prefix}/{}", o.hash),
                    dest: objects.join(prefix).join(&o.hash),
                    sha1: Some(o.hash.clone()),
                    size: Some(o.size),
                }
            })
            .collect();
        let events = self.inner.events.clone();
        download_all(
            self.http(),
            tasks,
            concurrency,
            verify,
            move |done, total| {
                events.emit(Event::Progress {
                    task_id,
                    stage: "assets".into(),
                    done,
                    total,
                    unit: ProgressUnit::Files,
                });
            },
        )
        .await?;

        let mut game_assets = root.clone();
        if index.is_virtual {
            let virtual_dir = root.join("virtual").join(&index_ref.id);
            for (name, obj) in &index.objects {
                let dest = safe_join(&virtual_dir, name)?;
                if !dest.exists() {
                    copy_object(&objects, &obj.hash, &dest).await?;
                }
            }
            game_assets = virtual_dir;
        }
        Ok(AssetsLayout {
            index_name: index_ref.id,
            root,
            game_assets,
            map_to_resources: index.map_to_resources,
        })
    }

    /// Copia los assets a `<gameDir>/resources` (versiones anteriores a 1.6).
    pub async fn map_assets_to_resources(
        &self,
        assets: &AssetsLayout,
        game_dir: &Path,
    ) -> Result<()> {
        if !assets.map_to_resources {
            return Ok(());
        }
        let index_path = assets
            .root
            .join("indexes")
            .join(format!("{}.json", assets.index_name));
        let index: AssetIndex = read_json(&index_path)
            .await?
            .ok_or_else(|| Error::InvalidInput("índice de assets ausente".into()))?;
        let objects = assets.root.join("objects");
        let resources = game_dir.join("resources");
        for (name, obj) in &index.objects {
            let dest = safe_join(&resources, name)?;
            if !dest.exists() {
                copy_object(&objects, &obj.hash, &dest).await?;
            }
        }
        Ok(())
    }
}

/// Enlace duro (o copia) de `src` en `dst` si `dst` falta o difiere en tamaño.
async fn link_or_copy(src: &Path, dst: &Path) -> Result<()> {
    let src_len = tokio::fs::metadata(src)
        .await
        .map_err(|e| Error::io(src, e))?
        .len();
    if tokio::fs::metadata(dst)
        .await
        .is_ok_and(|m| m.len() == src_len)
    {
        return Ok(());
    }
    if let Some(parent) = dst.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::io(parent, e))?;
    }
    let _ = tokio::fs::remove_file(dst).await;
    if tokio::fs::hard_link(src, dst).await.is_err() {
        tokio::fs::copy(src, dst)
            .await
            .map_err(|e| Error::io(dst, e))?;
    }
    Ok(())
}

async fn copy_object(objects: &Path, hash: &str, dest: &Path) -> Result<()> {
    let src = objects.join(&hash[..2]).join(hash);
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Error::io(parent, e))?;
    }
    tokio::fs::copy(&src, dest)
        .await
        .map(|_| ())
        .map_err(|e| Error::io(&src, e))
}

fn dedup(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    paths
        .into_iter()
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

/// Extrae un zip en `dest` saltando las rutas que empiezan por `exclude` y las inseguras.
pub fn extract_zip(archive: &Path, dest: &Path, exclude: &[String]) -> Result<()> {
    let file = std::fs::File::open(archive).map_err(|e| Error::io(archive, e))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| Error::InvalidInput(format!("{}: {e}", archive.display())))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| Error::InvalidInput(format!("{}: {e}", archive.display())))?;
        let name = entry
            .name()
            .map_err(|e| Error::InvalidInput(format!("{}: {e}", archive.display())))?
            .into_owned();
        if exclude.iter().any(|ex| name.starts_with(ex.as_str())) {
            continue;
        }
        let Some(rel) = entry.enclosed_name() else {
            tracing::warn!(%name, "entrada insegura ignorada");
            continue;
        };
        let out = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| Error::io(&out, e))?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let mut f = std::fs::File::create(&out).map_err(|e| Error::io(&out, e))?;
        std::io::copy(&mut entry, &mut f).map_err(|e| Error::io(&out, e))?;
    }
    Ok(())
}
