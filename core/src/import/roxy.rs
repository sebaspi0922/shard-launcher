//! Importador de Roxy Launcher 0.6.0 (docs/migracion.md).
//!
//! Principios: solo lectura sobre `.roxy`, sin secretos y primero simular.
//! Las instancias se localizan listando directorios
//! (`game/accounts/*/instances/*`); solo se abre `instance.json`,
//! `.modrinth_sources.json`, `backups/.backups.json` y el icono dentro de cada
//! instancia, más el contenido compartido (`libraries`, `assets`, `versions`,
//! `java`). Nunca se abre `session.json`, `launcher_account.json` ni
//! `latest_launch.log`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

// Formato de `backups/<id>/backups.json`: ahora vive en el módulo de backups.
pub use crate::backup::{BackupEntry, BackupIndex};
// Se reexporta para no romper `lcore::import::{ContentFile, ContentIndex}`.
pub use crate::content::{ContentFile, ContentIndex};
use crate::events::{Event, EventBus, ProgressUnit};
use crate::fsutil::{now_secs, write_json};
use crate::instance::{
    BackupSettings, GameVersion, INSTANCE_SCHEMA, ImportedFrom, Instance, JavaSettings, LoaderKind,
    WindowSettings, slugify,
};
use crate::launcher::Core;
use crate::paths::{Paths, safe_join, single_segment};
use crate::{Error, Result};

/// Valor de `imported_from.kind` de las instancias que vienen de Roxy.
pub const IMPORT_KIND: &str = "roxy";
/// Registro del último lanzamiento: puede llevar el token en la línea de
/// comandos, así que jamás se abre ni se copia.
const LAUNCH_LOG: &str = "latest_launch.log";
/// Tamaño máximo de un icono aceptado.
const MAX_ICON_BYTES: u64 = 16 * 1024 * 1024;

// ---------------------------------------------------------------------------
// API pública
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RoxyImportOptions {
    /// Raíz de datos de Roxy (por defecto `~/.local/share/.roxy`).
    pub source_root: PathBuf,
    /// Solo calcula el plan; no escribe nada.
    pub dry_run: bool,
    /// Incluye las instancias de la papelera de Roxy (`deletedAt`).
    pub include_trash: bool,
    /// No importa los backups.
    pub skip_backups: bool,
    /// No copia `logs/` ni `crash-reports/`.
    pub skip_logs: bool,
    /// Reimporta aunque la instancia ya se hubiera importado (con id nuevo).
    pub force: bool,
    /// Enlaza (o copia, si no se puede enlazar) librerías, assets, jars de
    /// versión y Java. Con `false` no se importa nada de eso y se volverá a
    /// descargar.
    pub link_shared: bool,
}

impl Default for RoxyImportOptions {
    fn default() -> Self {
        Self {
            source_root: default_source_root(),
            dry_run: false,
            include_trash: false,
            skip_backups: false,
            skip_logs: false,
            force: false,
            link_shared: true,
        }
    }
}

/// `~/.local/share/.roxy` (o el equivalente del SO).
pub fn default_source_root() -> PathBuf {
    directories::BaseDirs::new()
        .map(|d| d.data_dir().join(".roxy"))
        .unwrap_or_else(|| PathBuf::from(".roxy"))
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ImportPlan {
    pub instances: Vec<PlannedInstance>,
    pub shared: SharedPlan,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlannedInstance {
    pub source_dir: PathBuf,
    pub original_id: String,
    pub name: String,
    /// Id que tendrá (o que ya tiene, si se omite por estar importada). Vacío
    /// si se omite por otra causa.
    pub new_id: String,
    pub mc: String,
    pub loader: String,
    /// Archivos que se copian a `game/` (sin las exclusiones).
    pub files: u64,
    pub bytes: u64,
    pub backups: usize,
    /// `Some` = no se importa, con el motivo.
    pub skipped_reason: Option<String>,
}

/// Contenido compartido que falta en el destino (lo que ya está, con el mismo
/// tamaño, no se cuenta).
#[derive(Debug, Clone, Default, Serialize)]
pub struct SharedPlan {
    pub libraries_files: u64,
    pub libraries_bytes: u64,
    pub assets_files: u64,
    pub assets_bytes: u64,
    pub version_jars: u64,
    /// Versiones de Java que se copiarán (las que no existen en el destino).
    pub java_majors: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportReport {
    pub plan: ImportPlan,
    /// Ids nuevos de las instancias importadas.
    pub imported: Vec<String>,
    pub linked_files: u64,
    pub copied_files: u64,
}

impl Core {
    /// Calcula qué se importaría, sin escribir nada.
    pub async fn plan_roxy_import(&self, opts: &RoxyImportOptions) -> Result<ImportPlan> {
        Ok(self.scan_roxy(opts).await?.0)
    }

    /// Importa desde Roxy. Con `dry_run` solo devuelve el plan.
    pub async fn import_roxy(&self, opts: &RoxyImportOptions) -> Result<ImportReport> {
        let (mut plan, work) = self.scan_roxy(opts).await?;
        let mut report = ImportReport {
            plan: ImportPlan::default(),
            imported: Vec::new(),
            linked_files: 0,
            copied_files: 0,
        };
        if opts.dry_run {
            report.plan = plan;
            return Ok(report);
        }

        let events = self.inner.events.clone();
        let progress = Arc::new(Progress::new(events.clone(), work.total_units));
        let result = self
            .run_roxy_import(opts, work, &mut plan, &mut report, &progress)
            .await;
        events.emit(Event::TaskFinished {
            task_id: progress.task_id,
            ok: result.is_ok(),
            message: result.as_ref().err().map(|e| e.to_string()),
        });
        report.plan = plan;
        result.map(|()| report)
    }

    async fn scan_roxy(&self, opts: &RoxyImportOptions) -> Result<(ImportPlan, Work)> {
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
        let opts = opts.clone();
        let paths = self.paths().clone();
        tokio::task::spawn_blocking(move || scan(&opts, &paths, &existing))
            .await
            .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))?
    }

    async fn run_roxy_import(
        &self,
        opts: &RoxyImportOptions,
        work: Work,
        plan: &mut ImportPlan,
        report: &mut ImportReport,
        progress: &Arc<Progress>,
    ) -> Result<()> {
        for iw in &work.instances {
            let done = self.import_one(iw, opts, progress).await?;
            report.imported.push(iw.new_id.clone());
            report.copied_files += done.copied;
            report.linked_files += done.linked;
            plan.warnings.extend(done.warnings);
        }
        if opts.link_shared {
            let paths = self.paths().clone();
            let shared = work.shared;
            let progress = progress.clone();
            let done = tokio::task::spawn_blocking(move || run_shared(&shared, &paths, &progress))
                .await
                .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))??;
            report.copied_files += done.copied;
            report.linked_files += done.linked;
            plan.warnings.extend(done.warnings);
        }
        Ok(())
    }

    /// Importa una instancia. Si falla, borra lo que se hubiera creado: el
    /// `instance.json` se escribe el último, así que una importación a medias
    /// nunca aparece en el listado.
    async fn import_one(
        &self,
        iw: &Arc<InstWork>,
        opts: &RoxyImportOptions,
        progress: &Arc<Progress>,
    ) -> Result<Done> {
        let inst_dir = self.paths().instance_dir(&iw.new_id)?;
        let backups_dst = self.paths().backups_dir().join(single_segment(&iw.new_id)?);
        let result = self
            .import_one_inner(iw, opts, progress, &inst_dir, &backups_dst)
            .await;
        if result.is_err() {
            let _ = tokio::fs::remove_dir_all(&inst_dir).await;
            let _ = tokio::fs::remove_dir_all(&backups_dst).await;
        }
        result
    }

    async fn import_one_inner(
        &self,
        iw: &Arc<InstWork>,
        opts: &RoxyImportOptions,
        progress: &Arc<Progress>,
        inst_dir: &Path,
        backups_dst: &Path,
    ) -> Result<Done> {
        let job = iw.clone();
        let (idir, bdst, prog) = (
            inst_dir.to_path_buf(),
            backups_dst.to_path_buf(),
            progress.clone(),
        );
        let mut done =
            tokio::task::spawn_blocking(move || copy_instance(&job, &idir, &bdst, &prog))
                .await
                .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))??;

        // Procedencia: `.modrinth_sources.json` -> `content.json`.
        let sources = iw.src_dir.join(".modrinth_sources.json");
        match tokio::fs::read(&sources).await {
            Ok(bytes) => match convert_sources(&bytes) {
                Ok(index) => write_json(&inst_dir.join("content.json"), &index).await?,
                Err(e) => done.warnings.push(format!(
                    "{}: .modrinth_sources.json no es válido ({e}); no se genera content.json",
                    iw.new_id
                )),
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(&sources, e)),
        }

        // Backups: lista convertida.
        if !opts.skip_backups && !iw.backups.is_empty() {
            let index = BackupIndex {
                schema: crate::backup::BACKUP_SCHEMA,
                entries: iw.backups.iter().map(|b| b.entry.clone()).collect(),
            };
            write_json(&backups_dst.join("backups.json"), &index).await?;
        }

        let instance = map_instance(&iw.raw, &iw.original_id, &iw.new_id, &iw.src_dir);
        self.save_instance(&instance).await?;
        Ok(done)
    }
}

// ---------------------------------------------------------------------------
// Formato de origen
// ---------------------------------------------------------------------------

/// `instance.json` de Roxy (claves en camelCase).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RoxyInstance {
    id: String,
    name: String,
    mc_version: String,
    loader_type: String,
    loader_version: Option<String>,
    created_at: i64,
    last_played: Option<i64>,
    playtime_seconds: u64,
    icon_path: Option<String>,
    memory_mb: Option<u32>,
    jvm_args: Option<String>,
    resolution_width: Option<u32>,
    resolution_height: Option<u32>,
    dev_mode: bool,
    backup_auto_enabled: bool,
    backup_interval_hours: Option<u32>,
    backup_max_count: Option<u32>,
    deleted_at: Option<i64>,
}

/// Entrada de `backups/.backups.json` de Roxy.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RoxyBackup {
    id: String,
    created_at: i64,
    size_bytes: u64,
    pinned: bool,
    label: Option<String>,
    automatic: bool,
}

/// Valor de `.modrinth_sources.json` (claves en snake_case).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawSource {
    download_url: Option<String>,
    sha1: Option<String>,
    project_id: Option<String>,
    project_name: Option<String>,
    version_id: Option<String>,
    version_number: Option<String>,
    mod_category: Option<String>,
}

/// `{"<tipo>/<archivo>": {...}}` -> `content.json`.
fn convert_sources(json: &[u8]) -> std::result::Result<ContentIndex, serde_json::Error> {
    let raw: BTreeMap<String, RawSource> = serde_json::from_slice(json)?;
    let files = raw
        .into_iter()
        .map(|(key, src)| {
            let (prefix, name) = key.split_once('/').unwrap_or(("", key.as_str()));
            let (file, enabled) = match name.strip_suffix(".disabled") {
                Some(base) => (base, false),
                None => (name, true),
            };
            let from_modrinth = src.project_id.is_some()
                || src
                    .download_url
                    .as_deref()
                    .is_some_and(|u| u.to_ascii_lowercase().contains("modrinth"));
            ContentFile {
                kind: content_type(prefix).to_owned(),
                file: file.to_owned(),
                enabled,
                source: if from_modrinth { "modrinth" } else { "unknown" }.to_owned(),
                project_id: src.project_id,
                project_name: src.project_name,
                version_id: src.version_id,
                version_number: src.version_number,
                sha1: src.sha1,
                url: src.download_url,
                category: src.mod_category,
            }
        })
        .collect();
    Ok(ContentIndex { schema: 1, files })
}

/// Carpeta de Roxy -> tipo de contenido.
fn content_type(prefix: &str) -> &str {
    match prefix {
        "mods" | "mod" => "mod",
        "resourcepacks" | "resourcepack" => "resourcepack",
        "shaderpacks" | "shaderpack" | "shaders" | "shader" => "shader",
        "datapacks" | "datapack" => "datapack",
        "" => "other",
        other => other,
    }
}

/// Quita el `-<epoch>` final del id de Roxy.
fn strip_epoch(id: &str) -> &str {
    match id.rsplit_once('-') {
        Some((head, tail))
            if !head.is_empty() && tail.len() >= 9 && tail.bytes().all(|b| b.is_ascii_digit()) =>
        {
            head
        }
        _ => id,
    }
}

fn epoch_suffix(id: &str) -> Option<i64> {
    let (_, tail) = id.rsplit_once('-')?;
    (tail.len() >= 9 && tail.bytes().all(|b| b.is_ascii_digit()))
        .then(|| tail.parse().ok())
        .flatten()
}

/// `jvmArgs` (cadena) -> lista, sin `-Xmx`/`-Xms` (los pone la memoria).
/// `null` o en blanco = valor global.
fn split_jvm_args(args: Option<&str>) -> Option<Vec<String>> {
    let args = args.filter(|a| !a.trim().is_empty())?;
    Some(
        args.split_whitespace()
            .filter(|a| !a.starts_with("-Xmx") && !a.starts_with("-Xms"))
            .map(str::to_owned)
            .collect(),
    )
}

fn map_instance(raw: &RoxyInstance, original_id: &str, new_id: &str, src_dir: &Path) -> Instance {
    let created_at = if raw.created_at > 0 {
        raw.created_at
    } else {
        epoch_suffix(original_id)
            .or_else(|| epoch_suffix(&src_dir.file_name().unwrap_or_default().to_string_lossy()))
            .unwrap_or_else(now_secs)
    };
    Instance {
        schema: INSTANCE_SCHEMA,
        id: new_id.to_owned(),
        name: display_name(raw, original_id),
        game: GameVersion {
            mc: raw.mc_version.clone(),
            loader: LoaderKind::parse(&raw.loader_type).unwrap_or(LoaderKind::Vanilla),
            loader_version: raw.loader_version.clone().filter(|v| !v.is_empty()),
        },
        created_at,
        last_played: raw.last_played.filter(|t| *t > 0),
        playtime_seconds: raw.playtime_seconds,
        java: JavaSettings {
            path: None,
            memory_mb: raw.memory_mb,
            jvm_args: split_jvm_args(raw.jvm_args.as_deref()),
        },
        window: WindowSettings {
            width: raw.resolution_width,
            height: raw.resolution_height,
            fullscreen: false,
        },
        backups: BackupSettings {
            auto: raw.backup_auto_enabled,
            interval_hours: raw.backup_interval_hours,
            max_count: raw.backup_max_count,
        },
        dev_console: raw.dev_mode,
        imported_from: Some(ImportedFrom {
            kind: IMPORT_KIND.to_owned(),
            original_id: original_id.to_owned(),
        }),
    }
}

fn display_name(raw: &RoxyInstance, original_id: &str) -> String {
    let name = raw.name.trim();
    if name.is_empty() {
        strip_epoch(original_id).to_owned()
    } else {
        name.to_owned()
    }
}

// ---------------------------------------------------------------------------
// Recorrido del disco
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Kind {
    Dir,
    File,
    Symlink {
        /// Destino tal como está en el origen.
        target: PathBuf,
        /// Si el enlace era absoluto y apuntaba dentro de la raíz copiada:
        /// ruta relativa a esa raíz, para reapuntarlo a la copia.
        inside_abs: Option<PathBuf>,
    },
}

#[derive(Debug, Clone)]
struct Entry {
    rel: PathBuf,
    kind: Kind,
    size: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Links {
    /// Ignora los enlaces simbólicos.
    Skip,
    /// Los conserva si su destino queda dentro de la raíz; si no, los omite
    /// con un aviso.
    KeepInside,
}

/// Recorre `root` (sin seguir enlaces) en preorden. `include(rel, es_dir)`
/// decide qué se descarta.
fn walk_tree(
    root: &Path,
    include: &dyn Fn(&Path, bool) -> bool,
    links: Links,
    warnings: &mut Vec<String>,
) -> io::Result<Vec<Entry>> {
    let canon_root = fs::canonicalize(root)?;
    let mut out = Vec::new();
    walk_dir(
        root,
        Path::new(""),
        &canon_root,
        include,
        links,
        warnings,
        &mut out,
    )?;
    Ok(out)
}

fn walk_dir(
    root: &Path,
    rel_dir: &Path,
    canon_root: &Path,
    include: &dyn Fn(&Path, bool) -> bool,
    links: Links,
    warnings: &mut Vec<String>,
    out: &mut Vec<Entry>,
) -> io::Result<()> {
    let dir = root.join(rel_dir);
    let mut names: Vec<_> = fs::read_dir(&dir)?
        .map(|e| e.map(|e| e.file_name()))
        .collect::<io::Result<_>>()?;
    names.sort();
    for name in names {
        let Some(name_str) = name.to_str().filter(|n| single_segment(n).is_ok()) else {
            warnings.push(format!(
                "nombre no válido, se omite: {}",
                dir.join(&name).display()
            ));
            continue;
        };
        let rel = rel_dir.join(name_str);
        let path = root.join(&rel);
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        let ft = meta.file_type();
        if !include(&rel, ft.is_dir()) {
            continue;
        }
        if ft.is_symlink() {
            if links == Links::KeepInside {
                symlink_entry(&path, &rel, canon_root, warnings, out)?;
            }
        } else if ft.is_dir() {
            out.push(Entry {
                rel: rel.clone(),
                kind: Kind::Dir,
                size: 0,
            });
            walk_dir(root, &rel, canon_root, include, links, warnings, out)?;
        } else if ft.is_file() {
            out.push(Entry {
                rel,
                kind: Kind::File,
                size: meta.len(),
            });
        }
    }
    Ok(())
}

fn symlink_entry(
    path: &Path,
    rel: &Path,
    canon_root: &Path,
    warnings: &mut Vec<String>,
    out: &mut Vec<Entry>,
) -> io::Result<()> {
    let target = fs::read_link(path)?;
    let abs = if target.is_absolute() {
        target.clone()
    } else {
        path.parent().unwrap_or(Path::new("")).join(&target)
    };
    match fs::canonicalize(&abs) {
        Ok(canon) => match canon.strip_prefix(canon_root) {
            Ok(inside) => out.push(Entry {
                rel: rel.to_path_buf(),
                kind: Kind::Symlink {
                    inside_abs: target.is_absolute().then(|| inside.to_path_buf()),
                    target,
                },
                size: 0,
            }),
            Err(_) => warnings.push(format!(
                "enlace simbólico fuera de la instancia, se omite: {} -> {}",
                rel.display(),
                target.display()
            )),
        },
        Err(_) => warnings.push(format!(
            "enlace simbólico roto, se omite: {} -> {}",
            rel.display(),
            target.display()
        )),
    }
    Ok(())
}

/// Archivos (enlaces incluidos) y bytes.
fn stats(entries: &[Entry]) -> (u64, u64) {
    entries.iter().fold((0, 0), |(f, b), e| match e.kind {
        Kind::Dir => (f, b),
        Kind::File => (f + 1, b + e.size),
        Kind::Symlink { .. } => (f + 1, b),
    })
}

/// Lo que se descarta al copiar una instancia de Roxy.
fn instance_filter(skip_logs: bool) -> impl Fn(&Path, bool) -> bool {
    move |rel, _is_dir| {
        let name = rel.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with(LAUNCH_LOG) {
            return false;
        }
        if rel.components().count() == 1 {
            match name {
                "instance.json" | ".modrinth_sources.json" | "backups" => return false,
                "logs" | "crash-reports" if skip_logs => return false,
                _ => {}
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Plan
// ---------------------------------------------------------------------------

struct InstWork {
    src_dir: PathBuf,
    raw: RoxyInstance,
    original_id: String,
    new_id: String,
    entries: Vec<Entry>,
    backups: Vec<BackupFile>,
    icon_src: Option<PathBuf>,
}

struct BackupFile {
    path: PathBuf,
    name: String,
    /// Tamaño real del zip.
    size: u64,
    entry: BackupEntry,
}

struct LinkJob {
    src: PathBuf,
    dst: PathBuf,
    size: u64,
}

struct JavaJob {
    major: u32,
    src: PathBuf,
    entries: Vec<Entry>,
}

#[derive(Default)]
struct SharedWork {
    links: Vec<LinkJob>,
    java: Vec<JavaJob>,
}

#[derive(Default)]
struct Work {
    instances: Vec<Arc<InstWork>>,
    shared: SharedWork,
    /// Archivos que se van a crear (para el progreso).
    total_units: u64,
}

fn scan(
    opts: &RoxyImportOptions,
    paths: &Paths,
    existing: &HashMap<String, String>,
) -> Result<(ImportPlan, Work)> {
    let root = &opts.source_root;
    let game = root.join("game");
    if !game.is_dir() {
        return Err(Error::InvalidInput(format!(
            "no hay una instalación de Roxy en {}",
            root.display()
        )));
    }
    guard_overlap(root, paths.root())?;

    let mut plan = ImportPlan::default();
    let mut work = Work::default();
    let mut reserved = HashSet::new();

    for (_, account) in list_dirs(&game.join("accounts"), &mut plan.warnings)? {
        for (dir_name, dir) in list_dirs(&account.join("instances"), &mut plan.warnings)? {
            scan_instance(
                &dir_name,
                &dir,
                opts,
                paths,
                existing,
                &mut reserved,
                &mut plan,
                &mut work,
            )?;
        }
    }
    if opts.link_shared {
        scan_shared(&game, paths, &mut plan, &mut work)?;
    }

    work.total_units += work
        .instances
        .iter()
        .map(|i| stats(&i.entries).0 + i.backups.len() as u64)
        .sum::<u64>();
    Ok((plan, work))
}

/// El origen y el destino no pueden contenerse entre sí.
fn guard_overlap(source: &Path, dest: &Path) -> Result<()> {
    let s = fs::canonicalize(source).map_err(|e| Error::io(source, e))?;
    let Ok(d) = fs::canonicalize(dest) else {
        return Ok(());
    };
    if s.starts_with(&d) || d.starts_with(&s) {
        return Err(Error::InvalidInput(format!(
            "el origen ({}) y el directorio de datos ({}) no pueden contenerse",
            source.display(),
            dest.display()
        )));
    }
    Ok(())
}

/// Subdirectorios reales (no enlaces) con nombre válido, ordenados. Si `dir`
/// no existe, lista vacía.
fn list_dirs(dir: &Path, warnings: &mut Vec<String>) -> Result<Vec<(String, PathBuf)>> {
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(dir, e)),
    };
    let mut out = Vec::new();
    for entry in rd {
        let entry = entry.map_err(|e| Error::io(dir, e))?;
        let name = entry.file_name();
        let Some(name) = name.to_str().filter(|n| single_segment(n).is_ok()) else {
            warnings.push(format!(
                "nombre no válido, se omite: {}",
                entry.path().display()
            ));
            continue;
        };
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            out.push((name.to_owned(), entry.path()));
        }
    }
    out.sort();
    Ok(out)
}

/// Id libre: como `unique_instance_id`, pero además descarta los ya reservados
/// en este mismo plan (todavía no existen en disco) y las carpetas de backups.
fn alloc_id(paths: &Paths, base: &str, reserved: &mut HashSet<String>) -> Result<String> {
    let base = slugify(base);
    let mut candidate = base.clone();
    let mut n = 2;
    loop {
        let taken = reserved.contains(&candidate)
            || paths.instance_dir(&candidate)?.exists()
            || paths.backups_dir().join(&candidate).exists();
        if !taken {
            reserved.insert(candidate.clone());
            return Ok(candidate);
        }
        candidate = format!("{base}-{n}");
        n += 1;
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_instance(
    dir_name: &str,
    dir: &Path,
    opts: &RoxyImportOptions,
    paths: &Paths,
    existing: &HashMap<String, String>,
    reserved: &mut HashSet<String>,
    plan: &mut ImportPlan,
    work: &mut Work,
) -> Result<()> {
    let json_path = dir.join("instance.json");
    let skipped = |name: &str, reason: String| PlannedInstance {
        source_dir: dir.to_path_buf(),
        original_id: dir_name.to_owned(),
        name: name.to_owned(),
        new_id: String::new(),
        mc: String::new(),
        loader: String::new(),
        files: 0,
        bytes: 0,
        backups: 0,
        skipped_reason: Some(reason),
    };
    let bytes = match fs::read(&json_path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            plan.warnings
                .push(format!("{dir_name}: no tiene instance.json; se omite"));
            return Ok(());
        }
        Err(e) => {
            plan.instances
                .push(skipped(dir_name, format!("instance.json ilegible: {e}")));
            return Ok(());
        }
    };
    let raw: RoxyInstance = match serde_json::from_slice(&bytes) {
        Ok(r) => r,
        Err(e) => {
            plan.instances
                .push(skipped(dir_name, format!("instance.json inválido: {e}")));
            return Ok(());
        }
    };
    let original_id = if raw.id.trim().is_empty() {
        dir_name.to_owned()
    } else {
        raw.id.clone()
    };
    let name = display_name(&raw, &original_id);
    let loader = LoaderKind::parse(&raw.loader_type);

    let mut planned = PlannedInstance {
        source_dir: dir.to_path_buf(),
        original_id: original_id.clone(),
        name,
        new_id: String::new(),
        mc: raw.mc_version.clone(),
        loader: loader.map_or_else(|| raw.loader_type.clone(), |l| l.as_str().to_owned()),
        files: 0,
        bytes: 0,
        backups: 0,
        skipped_reason: None,
    };

    if raw.deleted_at.is_some() && !opts.include_trash {
        planned.skipped_reason = Some("en la papelera de Roxy (usa --include-trash)".into());
    } else if loader.is_none() {
        planned.skipped_reason = Some(format!("loader desconocido: {:?}", raw.loader_type));
    } else if let Some(id) = existing.get(&original_id).filter(|_| !opts.force) {
        planned.new_id = id.clone();
        planned.skipped_reason = Some(format!("ya importada como {id} (usa --force)"));
    }
    if planned.skipped_reason.is_some() {
        plan.instances.push(planned);
        return Ok(());
    }

    let new_id = alloc_id(paths, strip_epoch(&original_id), reserved)?;
    let filter = instance_filter(opts.skip_logs);
    let entries = walk_tree(dir, &filter, Links::KeepInside, &mut plan.warnings)
        .map_err(|e| Error::io(dir, e))?;
    let (files, bytes) = stats(&entries);
    let backups = if opts.skip_backups {
        Vec::new()
    } else {
        scan_backups(dir, &mut plan.warnings)
    };
    let icon_src = resolve_icon(dir, raw.icon_path.as_deref());

    planned.new_id = new_id.clone();
    planned.files = files;
    planned.bytes = bytes;
    planned.backups = backups.len();
    plan.instances.push(planned);
    work.instances.push(Arc::new(InstWork {
        src_dir: dir.to_path_buf(),
        raw,
        original_id,
        new_id,
        entries,
        backups,
        icon_src,
    }));
    Ok(())
}

/// `backups/backup-*.zip` + `.backups.json`. Los zip sin entrada se
/// describen a partir del archivo; las entradas sin zip se descartan.
fn scan_backups(dir: &Path, warnings: &mut Vec<String>) -> Vec<BackupFile> {
    let bdir = dir.join("backups");
    let Ok(rd) = fs::read_dir(&bdir) else {
        return Vec::new();
    };
    let mut index: HashMap<String, RoxyBackup> = HashMap::new();
    match fs::read(bdir.join(".backups.json")) {
        Ok(bytes) => match serde_json::from_slice::<Vec<RoxyBackup>>(&bytes) {
            Ok(list) => index.extend(list.into_iter().map(|b| (b.id.clone(), b))),
            Err(e) => warnings.push(format!(
                "{}: .backups.json no es válido ({e}); se reconstruye desde los zip",
                dir.display()
            )),
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => warnings.push(format!("{}: .backups.json ilegible ({e})", dir.display())),
    }

    let mut out = Vec::new();
    for entry in rd.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str().filter(|n| single_segment(n).is_ok()) else {
            continue;
        };
        let Some(stem) = name
            .strip_prefix("backup-")
            .and_then(|n| n.strip_suffix(".zip"))
            .map(|s| format!("backup-{s}"))
        else {
            continue;
        };
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let known = index.remove(&stem);
        let entry = match known {
            Some(b) => BackupEntry {
                size_bytes: if b.size_bytes > 0 {
                    b.size_bytes
                } else {
                    meta.len()
                },
                id: stem,
                created_at: b.created_at,
                pinned: b.pinned,
                label: b.label,
                automatic: b.automatic,
            },
            None => BackupEntry {
                created_at: stem
                    .strip_prefix("backup-")
                    .and_then(|e| e.parse().ok())
                    .unwrap_or_else(|| {
                        meta.modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map_or(0, |d| d.as_secs() as i64)
                    }),
                size_bytes: meta.len(),
                id: stem,
                pinned: false,
                label: None,
                automatic: false,
            },
        };
        out.push(BackupFile {
            path,
            name: name.to_owned(),
            size: meta.len(),
            entry,
        });
    }
    out.sort_by(|a, b| (a.entry.created_at, &a.name).cmp(&(b.entry.created_at, &b.name)));
    out
}

/// Icono de la instancia, solo si es un archivo normal dentro de ella.
fn resolve_icon(dir: &Path, icon: Option<&str>) -> Option<PathBuf> {
    let icon = icon?.trim();
    if icon.is_empty() {
        return None;
    }
    let candidate = if Path::new(icon).is_absolute() {
        PathBuf::from(icon)
    } else {
        safe_join(dir, icon).ok()?
    };
    let canon = fs::canonicalize(&candidate).ok()?;
    let root = fs::canonicalize(dir).ok()?;
    let meta = fs::metadata(&canon).ok()?;
    let name = canon.file_name()?.to_str()?;
    (canon.starts_with(&root)
        && meta.is_file()
        && meta.len() <= MAX_ICON_BYTES
        && !name.starts_with(LAUNCH_LOG))
    .then_some(canon)
}

fn scan_shared(game: &Path, paths: &Paths, plan: &mut ImportPlan, work: &mut Work) -> Result<()> {
    let all = |_: &Path, _: bool| true;
    let mut add_tree =
        |src: PathBuf, dst: PathBuf, warnings: &mut Vec<String>| -> Result<(u64, u64)> {
            let mut files = 0;
            let mut bytes = 0;
            if !src.is_dir() {
                return Ok((0, 0));
            }
            let entries =
                walk_tree(&src, &all, Links::Skip, warnings).map_err(|e| Error::io(&src, e))?;
            for e in entries {
                if !matches!(e.kind, Kind::File) {
                    continue;
                }
                let target = dst.join(&e.rel);
                if fs::metadata(&target).is_ok_and(|m| m.is_file() && m.len() == e.size) {
                    continue;
                }
                files += 1;
                bytes += e.size;
                work.shared.links.push(LinkJob {
                    src: src.join(&e.rel),
                    dst: target,
                    size: e.size,
                });
            }
            Ok((files, bytes))
        };

    let (f, b) = add_tree(
        game.join("libraries"),
        paths.libraries_dir(),
        &mut plan.warnings,
    )?;
    plan.shared.libraries_files = f;
    plan.shared.libraries_bytes = b;

    for sub in ["indexes", "objects"] {
        let (f, b) = add_tree(
            game.join("assets").join(sub),
            paths.assets_dir().join(sub),
            &mut plan.warnings,
        )?;
        plan.shared.assets_files += f;
        plan.shared.assets_bytes += b;
    }

    // Jars de versión: `versions/<id>/<id>.jar` (Roxy no guarda el JSON).
    for (id, dir) in list_dirs(&game.join("versions"), &mut plan.warnings)? {
        let jar = dir.join(format!("{id}.jar"));
        let Ok(meta) = fs::symlink_metadata(&jar) else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let dst = paths.versions_dir().join(&id).join(format!("{id}.jar"));
        if fs::metadata(&dst).is_ok_and(|m| m.is_file() && m.len() == meta.len()) {
            continue;
        }
        plan.shared.version_jars += 1;
        work.shared.links.push(LinkJob {
            src: jar,
            dst,
            size: meta.len(),
        });
    }

    // Java: solo las versiones que no existen en el destino.
    for (name, dir) in list_dirs(&game.join("java"), &mut plan.warnings)? {
        let Ok(major) = name.parse::<u32>() else {
            continue;
        };
        if paths.java_dir().join(&name).exists() {
            continue;
        }
        let entries = walk_tree(&dir, &all, Links::KeepInside, &mut plan.warnings)
            .map_err(|e| Error::io(&dir, e))?;
        plan.shared.java_majors.push(major);
        work.shared.java.push(JavaJob {
            major,
            src: dir,
            entries,
        });
    }
    plan.shared.java_majors.sort_unstable();

    work.total_units += work.shared.links.len() as u64
        + work
            .shared
            .java
            .iter()
            .map(|j| stats(&j.entries).0)
            .sum::<u64>();
    Ok(())
}

// ---------------------------------------------------------------------------
// Copia
// ---------------------------------------------------------------------------

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

/// Progreso de la tarea `import` (en archivos).
struct Progress {
    events: EventBus,
    task_id: u64,
    total: u64,
    done: AtomicU64,
}

impl Progress {
    fn new(events: EventBus, total: u64) -> Self {
        Self {
            events,
            task_id: NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed),
            total,
            done: AtomicU64::new(0),
        }
    }

    fn tick(&self, n: u64) {
        let done = self.done.fetch_add(n, Ordering::Relaxed) + n;
        if done / 64 != (done - n) / 64 || done >= self.total {
            self.events.emit(Event::Progress {
                task_id: self.task_id,
                stage: "import".into(),
                done,
                total: self.total.max(done),
                unit: ProgressUnit::Files,
            });
        }
    }
}

#[derive(Default)]
struct Done {
    copied: u64,
    linked: u64,
    warnings: Vec<String>,
}

enum LinkOutcome {
    Linked,
    Copied,
    Skipped,
}

/// Copia un archivo conservando fecha de modificación y permisos (en lo que
/// se pueda).
fn copy_file(src: &Path, dst: &Path) -> io::Result<()> {
    let mut input = fs::File::open(src)?;
    let meta = input.metadata()?;
    let mut out = fs::File::create(dst)?;
    io::copy(&mut input, &mut out)?;
    if let Ok(modified) = meta.modified() {
        let _ = out.set_modified(modified);
    }
    drop(out);
    let _ = fs::set_permissions(dst, meta.permissions());
    Ok(())
}

/// Enlace duro; si falla (otro disco, permisos), copia. No toca un destino que
/// ya existe con el mismo tamaño.
fn link_or_copy(src: &Path, dst: &Path, size: u64) -> io::Result<LinkOutcome> {
    if let Ok(meta) = fs::metadata(dst) {
        if meta.is_file() && meta.len() == size {
            return Ok(LinkOutcome::Skipped);
        }
        fs::remove_file(dst)?;
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::hard_link(src, dst) {
        Ok(()) => Ok(LinkOutcome::Linked),
        Err(_) => {
            copy_file(src, dst)?;
            Ok(LinkOutcome::Copied)
        }
    }
}

#[cfg(unix)]
fn make_symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn make_symlink(_target: &Path, _link: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "enlaces simbólicos no soportados",
    ))
}

/// Copia las entradas de `src_root` a `dst_root`. Devuelve los archivos creados.
fn copy_entries(
    src_root: &Path,
    dst_root: &Path,
    entries: &[Entry],
    progress: &Progress,
) -> Result<u64> {
    fs::create_dir_all(dst_root).map_err(|e| Error::io(dst_root, e))?;
    let mut copied = 0;
    for entry in entries {
        let dst = dst_root.join(&entry.rel);
        match &entry.kind {
            Kind::Dir => fs::create_dir_all(&dst).map_err(|e| Error::io(&dst, e))?,
            Kind::File => {
                let src = src_root.join(&entry.rel);
                copy_file(&src, &dst).map_err(|e| Error::io(&src, e))?;
                copied += 1;
                progress.tick(1);
            }
            Kind::Symlink { target, inside_abs } => {
                let target = match inside_abs {
                    Some(rel) => dst_root.join(rel),
                    None => target.clone(),
                };
                make_symlink(&target, &dst).map_err(|e| Error::io(&dst, e))?;
                copied += 1;
                progress.tick(1);
            }
        }
    }
    Ok(copied)
}

/// Copia `game/`, el icono y los backups de una instancia (todo menos los
/// JSON, que escribe quien llama).
fn copy_instance(
    iw: &InstWork,
    inst_dir: &Path,
    backups_dst: &Path,
    progress: &Progress,
) -> Result<Done> {
    let mut done = Done::default();
    let game_dir = inst_dir.join("game");
    done.copied += copy_entries(&iw.src_dir, &game_dir, &iw.entries, progress)?;

    // Verificación: lo copiado debe coincidir con lo planeado. Las diferencias
    // se avisan (Roxy puede estar escribiendo), no abortan.
    let mut ignored = Vec::new();
    let copied = walk_tree(&game_dir, &|_, _| true, Links::KeepInside, &mut ignored)
        .map_err(|e| Error::io(&game_dir, e))?;
    if stats(&copied) != stats(&iw.entries) {
        let (pf, pb) = stats(&iw.entries);
        let (cf, cb) = stats(&copied);
        done.warnings.push(format!(
            "{}: la copia no coincide con el origen ({cf} archivos/{cb} bytes frente a {pf}/{pb})",
            iw.new_id
        ));
    }

    if let Some(icon) = &iw.icon_src {
        let dst = inst_dir.join("icon.png");
        if let Err(e) = fs::copy(icon, &dst) {
            done.warnings
                .push(format!("{}: no se pudo copiar el icono: {e}", iw.new_id));
        }
    }

    if !iw.backups.is_empty() {
        fs::create_dir_all(backups_dst).map_err(|e| Error::io(backups_dst, e))?;
        for backup in &iw.backups {
            let dst = backups_dst.join(&backup.name);
            match link_or_copy(&backup.path, &dst, backup.size)
                .map_err(|e| Error::io(&backup.path, e))?
            {
                LinkOutcome::Linked => done.linked += 1,
                LinkOutcome::Copied => done.copied += 1,
                LinkOutcome::Skipped => {}
            }
            progress.tick(1);
        }
    }
    Ok(done)
}

/// Enlaza el contenido compartido y copia las versiones de Java que falten.
fn run_shared(shared: &SharedWork, paths: &Paths, progress: &Progress) -> Result<Done> {
    let mut done = Done::default();
    for job in &shared.links {
        match link_or_copy(&job.src, &job.dst, job.size).map_err(|e| Error::io(&job.src, e))? {
            LinkOutcome::Linked => done.linked += 1,
            LinkOutcome::Copied => done.copied += 1,
            LinkOutcome::Skipped => {}
        }
        progress.tick(1);
    }

    for job in &shared.java {
        let java_dir = paths.java_dir();
        let dst = java_dir.join(job.major.to_string());
        if dst.exists() {
            continue;
        }
        // Se copia a un temporal y se renombra: nunca queda una Java a medias.
        let tmp = java_dir.join(format!(".{}.importando", job.major));
        if tmp.exists() {
            fs::remove_dir_all(&tmp).map_err(|e| Error::io(&tmp, e))?;
        }
        done.copied += copy_entries(&job.src, &tmp, &job.entries, progress)?;
        fs::rename(&tmp, &dst).map_err(|e| Error::io(&dst, e))?;
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_sin_epoch() {
        assert_eq!(
            strip_epoch("fabulously-optimized-1785816370"),
            "fabulously-optimized"
        );
        assert_eq!(strip_epoch("mundo-1-20"), "mundo-1-20");
        assert_eq!(strip_epoch("1785816370"), "1785816370");
        assert_eq!(strip_epoch("a-12345678"), "a-12345678");
        assert_eq!(epoch_suffix("x-1785816370"), Some(1_785_816_370));
        assert_eq!(epoch_suffix("x"), None);
    }

    #[test]
    fn jvm_args_sin_memoria() {
        assert_eq!(split_jvm_args(None), None);
        assert_eq!(split_jvm_args(Some("   ")), None);
        assert_eq!(
            split_jvm_args(Some("-Xmx12G -Xms2G  -XX:+UseG1GC -Dx=1")),
            Some(vec!["-XX:+UseG1GC".to_owned(), "-Dx=1".to_owned()])
        );
        assert_eq!(split_jvm_args(Some("-Xmx4G")), Some(vec![]));
    }

    #[test]
    fn procedencia_a_content_json() {
        let json = br#"{
            "mods/sodium.jar": {"download_url": "https://cdn.modrinth.com/data/x/sodium.jar", "sha1": "aa", "project_id": "AANobbMI", "project_name": "Sodium", "version_number": "1.0", "mod_category": "optimization"},
            "mods/lithium.jar.disabled": {"download_url": "https://cdn.modrinth.com/l.jar", "sha1": "bb", "project_id": null, "project_name": null, "version_number": null, "mod_category": null},
            "resourcepacks/pack.zip": {"download_url": null, "sha1": "cc", "project_id": null},
            "shaderpacks/s.zip": {"download_url": "https://example.com/s.zip", "project_id": "P"}
        }"#;
        let index = convert_sources(json).unwrap();
        assert_eq!(index.schema, 1);
        let by_file: HashMap<_, _> = index.files.iter().map(|f| (f.file.as_str(), f)).collect();
        let sodium = by_file["sodium.jar"];
        assert_eq!(
            (sodium.kind.as_str(), sodium.enabled, sodium.source.as_str()),
            ("mod", true, "modrinth")
        );
        assert_eq!(sodium.category.as_deref(), Some("optimization"));
        let lithium = by_file["lithium.jar"];
        assert!(!lithium.enabled);
        assert_eq!(lithium.source, "modrinth"); // por la URL
        assert_eq!(lithium.project_id, None);
        let pack = by_file["pack.zip"];
        assert_eq!(
            (pack.kind.as_str(), pack.source.as_str()),
            ("resourcepack", "unknown")
        );
        assert_eq!(by_file["s.zip"].kind, "shader");
        assert_eq!(by_file["s.zip"].source, "modrinth");
    }
}
