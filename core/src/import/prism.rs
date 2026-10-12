//! Importador de instancias de Prism Launcher / MultiMC.
//!
//! Solo lectura sobre el origen. De cada instancia se abren únicamente
//! `instance.cfg`, `mmc-pack.json` y la carpeta del juego (`.minecraft/` o
//! `minecraft/`). Jamás se abre `accounts.json` (cuentas y tokens), que vive
//! en la raíz de Prism, ni `prismlauncher.cfg`.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::common::{
    MAX_META_BYTES, Progress, Tree, alloc_id, copy_tree, guard_overlap, read_capped, split_jvm_args,
};
use crate::fsutil::now_secs;
use crate::instance::{
    BackupSettings, GameVersion, INSTANCE_SCHEMA, ImportedFrom, Instance, JavaSettings, LoaderKind,
    WindowSettings,
};
use crate::launcher::Core;
use crate::{Error, Result};

/// Valor de `imported_from.kind` de las instancias que vienen de Prism/MultiMC.
pub const IMPORT_KIND: &str = "prism";

// ---------------------------------------------------------------------------
// Inspección
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct PrismInstanceInfo {
    /// Carpeta de la instancia.
    pub dir: PathBuf,
    /// Nombre de la carpeta.
    pub folder: String,
    pub name: String,
    pub icon_key: Option<String>,
    pub game: Option<GameVersion>,
    /// `.minecraft/` o `minecraft/` dentro de la instancia.
    pub game_dir: Option<PathBuf>,
    /// Solo si la instancia sobrescribe la memoria (`OverrideMemory`).
    pub memory_mb: Option<u32>,
    /// Solo si sobrescribe los argumentos de Java (`OverrideJavaArgs`), sin -Xmx/-Xms.
    pub jvm_args: Option<Vec<String>>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub last_played: Option<i64>,
    pub playtime_seconds: u64,
    /// `Some` = no se puede importar, con el motivo.
    pub unusable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrismInfo {
    pub path: PathBuf,
    /// `true` si `path` es una raíz de Prism/MultiMC (o su carpeta `instances`);
    /// `false` si es una sola instancia.
    pub is_root: bool,
    pub instances: Vec<PrismInstanceInfo>,
    pub warnings: Vec<String>,
}

/// Examina una instancia suelta (con `instance.cfg` y `mmc-pack.json`) o una
/// raíz de Prism/MultiMC (con `instances/`).
pub async fn inspect_prism(path: &Path) -> Result<PrismInfo> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || inspect(&path))
        .await
        .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))?
}

fn inspect(path: &Path) -> Result<PrismInfo> {
    let meta = fs::metadata(path).map_err(|e| Error::io(path, e))?;
    if !meta.is_dir() {
        return Err(Error::InvalidPath(format!(
            "{} no es una carpeta",
            path.display()
        )));
    }
    let mut warnings = Vec::new();
    if path.join("instance.cfg").is_file() {
        let folder = folder_name(path);
        return Ok(PrismInfo {
            path: path.to_path_buf(),
            is_root: false,
            instances: vec![read_instance(path, &folder)],
            warnings,
        });
    }
    // Raíz de Prism (`instances/`) o la propia carpeta de instancias.
    let list_dir = if path.join("instances").is_dir() {
        path.join("instances")
    } else {
        path.to_path_buf()
    };
    let mut instances = Vec::new();
    let mut names: Vec<_> = fs::read_dir(&list_dir)
        .map_err(|e| Error::io(&list_dir, e))?
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        // `_LAUNCHER_TEMP`, `.tmp`…: carpetas internas, no instancias.
        .filter(|n| !n.starts_with(['.', '_']))
        .collect();
    names.sort();
    for name in names {
        let dir = list_dir.join(&name);
        if dir.join("instance.cfg").is_file() {
            instances.push(read_instance(&dir, &name));
        } else {
            warnings.push(format!("{name}: sin instance.cfg; se omite"));
        }
    }
    if instances.is_empty() {
        return Err(Error::InvalidInput(format!(
            "{} no es una instancia ni una raíz de Prism/MultiMC (sin instance.cfg ni instances/)",
            path.display()
        )));
    }
    Ok(PrismInfo {
        path: path.to_path_buf(),
        is_root: true,
        instances,
        warnings,
    })
}

fn folder_name(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `mmc-pack.json`: solo los componentes.
#[derive(Debug, Default, Deserialize)]
struct Pack {
    #[serde(default)]
    components: Vec<Component>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Component {
    uid: String,
    version: Option<String>,
}

fn read_instance(dir: &Path, folder: &str) -> PrismInstanceInfo {
    let mut info = PrismInstanceInfo {
        dir: dir.to_path_buf(),
        folder: folder.to_owned(),
        name: folder.to_owned(),
        icon_key: None,
        game: None,
        game_dir: None,
        memory_mb: None,
        jvm_args: None,
        width: None,
        height: None,
        last_played: None,
        playtime_seconds: 0,
        unusable_reason: None,
    };

    match read_capped(&dir.join("instance.cfg"), MAX_META_BYTES) {
        Ok(bytes) => apply_cfg(&mut info, &parse_ini(&String::from_utf8_lossy(&bytes))),
        Err(e) => info.unusable_reason = Some(format!("instance.cfg ilegible: {e}")),
    }

    match read_capped(&dir.join("mmc-pack.json"), MAX_META_BYTES) {
        Ok(bytes) => match serde_json::from_slice::<Pack>(&bytes) {
            Ok(pack) => match game_from_components(&pack.components) {
                Some(game) => info.game = Some(game),
                None => {
                    info.unusable_reason
                        .get_or_insert_with(|| "mmc-pack.json sin componente net.minecraft".into());
                }
            },
            Err(e) => {
                info.unusable_reason
                    .get_or_insert_with(|| format!("mmc-pack.json no es válido: {e}"));
            }
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            info.unusable_reason
                .get_or_insert_with(|| "falta mmc-pack.json".into());
        }
        Err(e) => {
            info.unusable_reason
                .get_or_insert_with(|| format!("mmc-pack.json ilegible: {e}"));
        }
    }

    info.game_dir = [".minecraft", "minecraft"]
        .into_iter()
        .map(|n| dir.join(n))
        .find(|p| fs::symlink_metadata(p).is_ok_and(|m| m.is_dir()));
    info
}

/// Pares `clave=valor` de un `instance.cfg` (INI de Qt). Las claves se pasan a
/// minúsculas; las secciones se ignoran.
fn parse_ini(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with([';', '#', '[']) {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(value);
        map.insert(key.trim().to_ascii_lowercase(), value.to_owned());
    }
    map
}

fn apply_cfg(info: &mut PrismInstanceInfo, cfg: &HashMap<String, String>) {
    let text = |k: &str| cfg.get(k).map(|v| v.trim()).filter(|v| !v.is_empty());
    let flag = |k: &str| text(k).is_some_and(|v| v.eq_ignore_ascii_case("true"));
    let number = |k: &str| text(k).and_then(|v| v.parse::<u64>().ok());

    if let Some(name) = text("name") {
        info.name = name.to_owned();
    }
    info.icon_key = text("iconkey").map(str::to_owned);
    // Sin `OverrideMemory` manda la memoria global de Prism, que no se lee.
    if flag("overridememory") {
        info.memory_mb = number("maxmemalloc")
            .and_then(|m| u32::try_from(m).ok())
            .filter(|m| *m > 0);
    }
    if flag("overridejavaargs") {
        info.jvm_args = text("jvmargs").and_then(split_jvm_args);
    }
    if flag("overridewindow") {
        info.width = number("minecraftwinwidth")
            .and_then(|v| u32::try_from(v).ok())
            .filter(|v| *v > 0);
        info.height = number("minecraftwinheight")
            .and_then(|v| u32::try_from(v).ok())
            .filter(|v| *v > 0);
    }
    // `lastLaunchTime` va en milisegundos; `totalTimePlayed`, en segundos.
    info.last_played = number("lastlaunchtime")
        .map(|ms| (ms / 1000) as i64)
        .filter(|t| *t > 0);
    info.playtime_seconds = number("totaltimeplayed").unwrap_or(0);
    // `JavaPath` se ignora a propósito: apunta a una Java de esa máquina.
}

fn game_from_components(components: &[Component]) -> Option<GameVersion> {
    let version = |uid: &str| {
        components
            .iter()
            .find(|c| c.uid == uid)
            .map(|c| c.version.clone().filter(|v| !v.is_empty()))
    };
    let mc = version("net.minecraft")??;
    let (loader, loader_version) = [
        ("net.fabricmc.fabric-loader", LoaderKind::Fabric),
        ("org.quiltmc.quilt-loader", LoaderKind::Quilt),
        ("net.neoforged", LoaderKind::NeoForge),
        ("net.minecraftforge", LoaderKind::Forge),
    ]
    .into_iter()
    .find_map(|(uid, kind)| version(uid).map(|v| (kind, v)))
    .unwrap_or((LoaderKind::Vanilla, None));
    Some(GameVersion {
        mc,
        loader,
        loader_version,
    })
}

// ---------------------------------------------------------------------------
// Importación
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct PrismImport {
    /// Instancia suelta o raíz de Prism/MultiMC.
    pub source: PathBuf,
    /// Instancias a importar, por nombre o por carpeta (sin distinguir
    /// mayúsculas); vacío = todas.
    pub instances: Vec<String>,
    /// Solo calcula el plan.
    pub dry_run: bool,
    /// Reimporta aunque la instancia ya se hubiera importado (con id nuevo).
    pub force: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrismPlanned {
    pub source_dir: PathBuf,
    pub folder: String,
    pub name: String,
    /// Id que tendrá (o que ya tiene, si se omite por estar importada).
    pub new_id: String,
    pub game: Option<GameVersion>,
    pub files: u64,
    pub bytes: u64,
    /// `Some` = no se importa, con el motivo.
    pub skipped_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrismReport {
    pub instances: Vec<PrismPlanned>,
    /// Ids nuevos de las instancias importadas.
    pub imported: Vec<String>,
    pub copied_files: u64,
    pub warnings: Vec<String>,
}

struct InstWork {
    info: PrismInstanceInfo,
    new_id: String,
    original_id: String,
    tree: Tree,
}

impl Core {
    /// Importa instancias de Prism/MultiMC (copia el juego tal cual). Es
    /// idempotente: las ya importadas se omiten salvo `force`.
    pub async fn import_prism(&self, opts: &PrismImport) -> Result<PrismReport> {
        let existing: HashMap<String, String> = self
            .list_instances()
            .await?
            .into_iter()
            .filter_map(|i| {
                i.imported_from
                    .filter(|f| f.kind == IMPORT_KIND)
                    .map(|f| (f.original_id, i.id))
            })
            .collect();
        let (job, paths) = (opts.clone(), self.paths().clone());
        let (mut report, work) = tokio::task::spawn_blocking(move || plan(&job, &paths, &existing))
            .await
            .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))??;
        if opts.dry_run {
            return Ok(report);
        }

        let total = work.iter().map(|w| w.tree.files).sum();
        let progress = Arc::new(Progress::new(self.inner.events.clone(), total));
        let mut outcome = Ok(());
        for iw in work {
            match self.import_prism_instance(&iw, &progress).await {
                Ok(copied) => {
                    report.copied_files += copied;
                    report.imported.push(iw.new_id.clone());
                }
                Err(e) => {
                    outcome = Err(e);
                    break;
                }
            }
        }
        progress.finish(&outcome);
        outcome.map(|()| report)
    }

    /// Si falla borra lo creado: el `instance.json` va el último, así que una
    /// importación a medias nunca aparece en el listado.
    async fn import_prism_instance(&self, iw: &InstWork, progress: &Arc<Progress>) -> Result<u64> {
        let inst_dir = self.paths().instance_dir(&iw.new_id)?;
        let result = self
            .import_prism_instance_inner(iw, progress, &inst_dir)
            .await;
        if result.is_err() {
            let _ = tokio::fs::remove_dir_all(&inst_dir).await;
        }
        result
    }

    async fn import_prism_instance_inner(
        &self,
        iw: &InstWork,
        progress: &Arc<Progress>,
        inst_dir: &Path,
    ) -> Result<u64> {
        let game_dst = inst_dir.join("game");
        let copied = match &iw.info.game_dir {
            Some(src) => {
                let (src, tree, dst, prog) = (
                    src.clone(),
                    iw.tree.clone(),
                    game_dst.clone(),
                    progress.clone(),
                );
                tokio::task::spawn_blocking(move || copy_tree(&src, &dst, &tree, &prog))
                    .await
                    .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))??
            }
            None => {
                tokio::fs::create_dir_all(&game_dst)
                    .await
                    .map_err(|e| Error::io(&game_dst, e))?;
                0
            }
        };

        let info = &iw.info;
        let instance = Instance {
            schema: INSTANCE_SCHEMA,
            id: iw.new_id.clone(),
            name: info.name.clone(),
            game: info.game.clone().ok_or_else(|| {
                Error::InvalidInput(format!("{}: sin versión de Minecraft", info.folder))
            })?,
            created_at: now_secs(),
            last_played: info.last_played,
            playtime_seconds: info.playtime_seconds,
            java: JavaSettings {
                path: None,
                memory_mb: info.memory_mb,
                jvm_args: info.jvm_args.clone(),
            },
            window: WindowSettings {
                width: info.width,
                height: info.height,
                fullscreen: false,
            },
            backups: BackupSettings::default(),
            dev_console: false,
            imported_from: Some(ImportedFrom {
                kind: IMPORT_KIND.to_owned(),
                original_id: iw.original_id.clone(),
            }),
        };
        self.save_instance(&instance).await?;
        Ok(copied)
    }
}

fn plan(
    opts: &PrismImport,
    paths: &crate::paths::Paths,
    existing: &HashMap<String, String>,
) -> Result<(PrismReport, Vec<InstWork>)> {
    let source = fs::canonicalize(&opts.source).map_err(|e| Error::io(&opts.source, e))?;
    guard_overlap(&source, paths.root())?;
    let info = inspect(&source)?;

    // Selección por nombre o carpeta; lo que no case, error.
    let wanted: Vec<String> = opts
        .instances
        .iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    let matches =
        |i: &PrismInstanceInfo, w: &str| i.name.to_lowercase() == w || i.folder.to_lowercase() == w;
    if let Some(missing) = opts
        .instances
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .find(|s| {
            let w = s.to_lowercase();
            !info.instances.iter().any(|i| matches(i, &w))
        })
    {
        let available: Vec<_> = info.instances.iter().map(|i| i.name.as_str()).collect();
        return Err(Error::InvalidInput(format!(
            "no hay una instancia {missing:?} en {} (disponibles: {})",
            source.display(),
            available.join(", ")
        )));
    }

    let mut report = PrismReport {
        instances: Vec::new(),
        imported: Vec::new(),
        copied_files: 0,
        warnings: info.warnings.clone(),
    };
    let mut work = Vec::new();
    let mut reserved = HashSet::new();

    for inst in info.instances {
        if !wanted.is_empty() && !wanted.iter().any(|w| matches(&inst, w)) {
            continue;
        }
        let original_id = fs::canonicalize(&inst.dir)
            .unwrap_or_else(|_| inst.dir.clone())
            .display()
            .to_string();
        let mut planned = PrismPlanned {
            source_dir: inst.dir.clone(),
            folder: inst.folder.clone(),
            name: inst.name.clone(),
            new_id: String::new(),
            game: inst.game.clone(),
            files: 0,
            bytes: 0,
            skipped_reason: inst.unusable_reason.clone(),
        };
        if planned.skipped_reason.is_none()
            && !opts.force
            && let Some(id) = existing.get(&original_id)
        {
            planned.new_id = id.clone();
            planned.skipped_reason = Some(format!("ya importada como {id} (usa --force)"));
        }
        if planned.skipped_reason.is_some() {
            report.instances.push(planned);
            continue;
        }

        let mut tree = Tree::default();
        match &inst.game_dir {
            Some(dir) => tree
                .add_dir(dir, Path::new(""), &mut report.warnings)
                .map_err(|e| Error::io(dir, e))?,
            None => report.warnings.push(format!(
                "{}: no tiene carpeta .minecraft; se crea vacía",
                inst.folder
            )),
        }
        planned.new_id = alloc_id(paths, &inst.name, &mut reserved)?;
        planned.files = tree.files;
        planned.bytes = tree.bytes;
        work.push(InstWork {
            new_id: planned.new_id.clone(),
            original_id,
            info: inst,
            tree,
        });
        report.instances.push(planned);
    }
    Ok((report, work))
}
