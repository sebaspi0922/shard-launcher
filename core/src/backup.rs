//! Copias de seguridad de instancias.
//!
//! Disco: `backups/<instancia>/backup-<epoch>.zip` + `backups.json`.
//! Contenido de un backup nuevo: `instance.json`, `content.json` (si existe) y
//! `game/**`, sin `game/logs/`, `game/crash-reports/` ni `*.tmp`. De los enlaces
//! simbólicos se siguen los que van a carpetas fuera del juego (p. ej. `saves`
//! enlazado a otro disco), que entran como carpetas normales, y los de archivos
//! dentro del juego; los de archivos fuera se omiten. Los zip heredados de Roxy
//! llevan el directorio entero de la instancia en la raíz (sin `game/`).

use std::collections::HashSet;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::events::{Event, EventBus, ProgressUnit};
use crate::fsutil::{now_secs, read_json, write_json};
use crate::install::next_task_id;
use crate::instance::BackupSettings;
use crate::launcher::Core;
use crate::paths::single_segment;
use crate::{Error, Result};

pub const BACKUP_SCHEMA: u32 = 1;
/// Copias automáticas que se conservan si la instancia no fija `max_count`.
pub const DEFAULT_MAX_AUTOMATIC: u32 = 5;
/// Horas entre copias automáticas si la instancia no fija `interval_hours`.
pub const DEFAULT_INTERVAL_HOURS: u32 = 24;

/// Serializa las lecturas-modificaciones de todos los `backups.json`.
static INDEX_LOCK: Mutex<()> = Mutex::const_new(());

/// Directorios y archivos que no entran en un backup (relativos a `game/`).
const EXCLUDED_TOP_LEVEL: [&str; 2] = ["logs", "crash-reports"];

// ---------------------------------------------------------------------------
// Tipos públicos
// ---------------------------------------------------------------------------

/// Entrada de `backups/<id>/backups.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupEntry {
    /// `backup-<epoch de inicio>`; el zip es `<id>.zip`.
    pub id: String,
    /// Segundos Unix en que terminó la copia.
    pub created_at: i64,
    pub size_bytes: u64,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub automatic: bool,
}

/// `backups/<id>/backups.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupIndex {
    pub schema: u32,
    pub entries: Vec<BackupEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct BackupRequest {
    pub label: Option<String>,
    pub pinned: bool,
    pub automatic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreMode {
    /// Sustituye todo `game/`.
    Full,
    /// Sustituye solo `saves/`.
    WorldsOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupLayout {
    /// `instance.json` + `content.json` + `game/**`.
    Current,
    /// Directorio de instancia de Roxy en la raíz del zip.
    LegacyRoxy,
}

#[derive(Debug, Clone, Serialize)]
pub struct RestoreReport {
    pub backup_id: String,
    pub mode: RestoreMode,
    pub layout: BackupLayout,
    /// Archivos extraídos.
    pub files: u64,
    /// Se aplicó el `content.json` del backup.
    pub content_restored: bool,
    /// El backup traía `instance.json`, pero se conservan los ajustes actuales.
    pub instance_settings_kept: bool,
    pub notes: Vec<String>,
}

// ---------------------------------------------------------------------------
// API sobre `Core`
// ---------------------------------------------------------------------------

impl Core {
    /// Falla si hay una partida de esa instancia en marcha.
    pub(crate) async fn ensure_instance_stopped(&self, instance_id: &str) -> Result<()> {
        if self.is_instance_running(instance_id).await {
            Err(Error::InvalidInput(format!(
                "la instancia {instance_id} tiene el juego en marcha: ciérralo antes de continuar"
            )))
        } else {
            Ok(())
        }
    }

    fn backups_dir_of(&self, instance_id: &str) -> Result<PathBuf> {
        Ok(self
            .paths()
            .backups_dir()
            .join(single_segment(instance_id)?))
    }

    /// Crea un backup. Falla si la instancia tiene el juego en marcha.
    pub async fn create_backup(
        &self,
        instance_id: &str,
        request: BackupRequest,
    ) -> Result<BackupEntry> {
        let instance = self.get_instance(instance_id).await?;
        self.ensure_instance_stopped(&instance.id).await?;
        let label = clean_label(request.label);

        let dir = self.backups_dir_of(&instance.id)?;
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| Error::io(&dir, e))?;
        let epoch = unique_epoch(&dir);
        let id = format!("backup-{epoch}");
        let final_path = dir.join(format!("{id}.zip"));
        let part_path = dir.join(format!("{id}.zip.part"));

        let job = ZipJob {
            instance_dir: self.instance_dir(&instance.id)?,
            game_dir: self.instance_game_dir(&instance.id)?,
            part: part_path,
            dest: final_path.clone(),
            task_id: next_task_id(),
            events: self.inner.events.clone(),
        };
        let task_id = job.task_id;
        if let Err(e) = blocking(move || job.run(), &final_path).await {
            self.inner.events.emit(Event::TaskFinished {
                task_id,
                ok: false,
                message: Some(e.to_string()),
            });
            return Err(e);
        }
        let size_bytes = tokio::fs::metadata(&final_path)
            .await
            .map_err(|e| Error::io(&final_path, e))?
            .len();

        let entry = BackupEntry {
            id,
            created_at: now_secs(),
            size_bytes,
            pinned: request.pinned,
            label,
            automatic: request.automatic,
        };
        {
            let _guard = INDEX_LOCK.lock().await;
            let mut entries = self.load_entries(&instance.id).await?;
            // El zip ya está en disco, así que el listado lo trae como huérfano.
            entries.retain(|e| e.id != entry.id);
            entries.push(entry.clone());
            if request.automatic {
                let max = instance
                    .backups
                    .max_count
                    .unwrap_or(DEFAULT_MAX_AUTOMATIC)
                    .max(1);
                apply_retention(&dir, &mut entries, max as usize).await;
            }
            self.save_entries(&instance.id, entries).await?;
        }
        self.inner.events.emit(Event::TaskFinished {
            task_id,
            ok: true,
            message: None,
        });
        Ok(entry)
    }

    /// Backups de la instancia, del más reciente al más antiguo. Incluye como
    /// manuales y sin fijar los zip que están en disco pero no en `backups.json`.
    pub async fn list_backups(&self, instance_id: &str) -> Result<Vec<BackupEntry>> {
        self.load_entries(instance_id).await
    }

    pub async fn delete_backup(&self, instance_id: &str, backup_id: &str) -> Result<()> {
        let dir = self.backups_dir_of(instance_id)?;
        let zip = backup_zip(&dir, backup_id)?;
        let _guard = INDEX_LOCK.lock().await;
        let mut entries = self.load_entries(instance_id).await?;
        let before = entries.len();
        entries.retain(|e| e.id != backup_id);
        if entries.len() == before {
            return Err(no_backup(backup_id));
        }
        match tokio::fs::remove_file(&zip).await {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(&zip, e)),
        }
        self.save_entries(instance_id, entries).await
    }

    pub async fn set_backup_pinned(
        &self,
        instance_id: &str,
        backup_id: &str,
        pinned: bool,
    ) -> Result<BackupEntry> {
        self.modify_entry(instance_id, backup_id, |e| e.pinned = pinned)
            .await
    }

    /// Cambia la etiqueta; `None` o vacía la quita.
    pub async fn set_backup_label(
        &self,
        instance_id: &str,
        backup_id: &str,
        label: Option<String>,
    ) -> Result<BackupEntry> {
        let label = clean_label(label);
        self.modify_entry(instance_id, backup_id, move |e| e.label = label)
            .await
    }

    /// Guarda los ajustes de backups automáticos de la instancia.
    pub async fn update_backup_settings(
        &self,
        instance_id: &str,
        settings: BackupSettings,
    ) -> Result<BackupSettings> {
        if settings.interval_hours == Some(0) {
            return Err(Error::InvalidInput(
                "el intervalo debe ser de al menos 1 hora".into(),
            ));
        }
        if settings.max_count == Some(0) {
            return Err(Error::InvalidInput(
                "hay que conservar al menos 1 copia automática".into(),
            ));
        }
        let instance = self
            .update_instance(instance_id, |i| i.backups = settings)
            .await?;
        Ok(instance.backups)
    }

    /// Restaura un backup (formato nuevo o heredado de Roxy). Falla si el juego
    /// de la instancia está en marcha.
    pub async fn restore_backup(
        &self,
        instance_id: &str,
        backup_id: &str,
        mode: RestoreMode,
    ) -> Result<RestoreReport> {
        let instance = self.get_instance(instance_id).await?;
        self.ensure_instance_stopped(&instance.id).await?;
        let zip = backup_zip(&self.backups_dir_of(&instance.id)?, backup_id)?;
        if !tokio::fs::try_exists(&zip).await.unwrap_or(false) {
            return Err(no_backup(backup_id));
        }
        let job = RestoreJob {
            backup_id: backup_id.to_owned(),
            archive: zip.clone(),
            instance_dir: self.instance_dir(&instance.id)?,
            game_dir: self.instance_game_dir(&instance.id)?,
            mode,
            task_id: next_task_id(),
            events: self.inner.events.clone(),
        };
        let task_id = job.task_id;
        let report = blocking(move || job.run(), &zip).await;
        self.inner.events.emit(Event::TaskFinished {
            task_id,
            ok: report.is_ok(),
            message: report.as_ref().err().map(|e| e.to_string()),
        });
        report
    }

    /// Una pasada del planificador: crea un backup automático en cada instancia
    /// con `backups.auto` cuyo último automático sea más viejo que su intervalo.
    /// Las instancias con el juego en marcha se saltan.
    pub async fn run_backup_scheduler_once(&self) -> Vec<(String, Result<BackupEntry>)> {
        let mut out = Vec::new();
        let instances = match self.list_instances().await {
            Ok(list) => list,
            Err(e) => {
                tracing::warn!(error = %e, "planificador de backups: no se pudo listar");
                return out;
            }
        };
        let now = now_secs();
        for instance in instances.into_iter().filter(|i| i.backups.auto) {
            if self.ensure_instance_stopped(&instance.id).await.is_err() {
                continue;
            }
            let interval = instance
                .backups
                .interval_hours
                .unwrap_or(DEFAULT_INTERVAL_HOURS)
                .max(1) as i64
                * 3600;
            let newest = match self.list_backups(&instance.id).await {
                Ok(list) => list
                    .iter()
                    .filter(|b| b.automatic)
                    .map(|b| b.created_at)
                    .max(),
                Err(e) => {
                    out.push((instance.id.clone(), Err(e)));
                    continue;
                }
            };
            if newest.is_some_and(|t| now - t < interval) {
                continue;
            }
            let result = self
                .create_backup(
                    &instance.id,
                    BackupRequest {
                        label: None,
                        pinned: false,
                        automatic: true,
                    },
                )
                .await;
            out.push((instance.id.clone(), result));
        }
        out
    }

    /// Lanza el planificador en segundo plano: una pasada ya y otra cada `every`.
    /// Para detenerlo, `abort()` sobre el manejador.
    pub fn spawn_backup_scheduler(&self, every: Duration) -> tokio::task::JoinHandle<()> {
        let core = self.clone();
        tokio::spawn(async move {
            loop {
                for (id, result) in core.run_backup_scheduler_once().await {
                    match result {
                        Ok(entry) => {
                            tracing::info!(instance = %id, backup = %entry.id, "backup automático")
                        }
                        Err(e) => {
                            tracing::warn!(instance = %id, error = %e, "backup automático fallido")
                        }
                    }
                }
                tokio::time::sleep(every).await;
            }
        })
    }

    // -- índice -----------------------------------------------------------

    /// `backups.json` combinado con lo que hay en disco, del más nuevo al más viejo.
    async fn load_entries(&self, instance_id: &str) -> Result<Vec<BackupEntry>> {
        let dir = self.backups_dir_of(instance_id)?;
        let index_path = dir.join("backups.json");
        let known = match read_json::<BackupIndex>(&index_path).await {
            Ok(index) => index.map(|i| i.entries).unwrap_or_default(),
            Err(Error::Json { .. }) => {
                // Índice corrupto: se aparta y se reconstruye desde los zip.
                let aside = dir.join(format!("backups.json.corrupto-{}", now_secs()));
                tracing::warn!(path = %index_path.display(), "backups.json ilegible; se aparta");
                let _ = tokio::fs::rename(&index_path, &aside).await;
                Vec::new()
            }
            Err(e) => return Err(e),
        };
        let mut on_disk = Vec::new();
        match tokio::fs::read_dir(&dir).await {
            Ok(mut rd) => {
                while let Some(item) = rd.next_entry().await.map_err(|e| Error::io(&dir, e))? {
                    let path = item.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("zip") {
                        continue;
                    }
                    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                        continue;
                    };
                    let Ok(meta) = item.metadata().await else {
                        continue;
                    };
                    if meta.is_file() {
                        on_disk.push((stem.to_owned(), meta));
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(&dir, e)),
        }
        let mut entries: Vec<BackupEntry> = Vec::new();
        for (stem, meta) in on_disk {
            if let Some(known) = known.iter().find(|e| e.id == stem) {
                entries.push(known.clone());
                continue;
            }
            let created_at = stem
                .strip_prefix("backup-")
                .and_then(|n| n.parse().ok())
                .or_else(|| {
                    meta.modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                })
                .unwrap_or(0);
            entries.push(BackupEntry {
                id: stem,
                created_at,
                size_bytes: meta.len(),
                pinned: false,
                label: None,
                automatic: false,
            });
        }
        entries.sort_by(|a, b| (b.created_at, &b.id).cmp(&(a.created_at, &a.id)));
        Ok(entries)
    }

    async fn save_entries(&self, instance_id: &str, entries: Vec<BackupEntry>) -> Result<()> {
        let path = self.backups_dir_of(instance_id)?.join("backups.json");
        write_json(
            &path,
            &BackupIndex {
                schema: BACKUP_SCHEMA,
                entries,
            },
        )
        .await
    }

    async fn modify_entry(
        &self,
        instance_id: &str,
        backup_id: &str,
        f: impl FnOnce(&mut BackupEntry),
    ) -> Result<BackupEntry> {
        single_segment(backup_id)?;
        let _guard = INDEX_LOCK.lock().await;
        let mut entries = self.load_entries(instance_id).await?;
        let entry = entries
            .iter_mut()
            .find(|e| e.id == backup_id)
            .ok_or_else(|| no_backup(backup_id))?;
        f(entry);
        let updated = entry.clone();
        self.save_entries(instance_id, entries).await?;
        Ok(updated)
    }
}

fn no_backup(id: &str) -> Error {
    Error::InvalidInput(format!("no existe el backup {id}"))
}

fn backup_zip(dir: &Path, backup_id: &str) -> Result<PathBuf> {
    Ok(dir.join(format!("{}.zip", single_segment(backup_id)?)))
}

fn clean_label(label: Option<String>) -> Option<String> {
    label.map(|l| l.trim().to_owned()).filter(|l| !l.is_empty())
}

/// Epoch de inicio libre en `dir`: nunca repite uno ya dado en este proceso (aunque
/// se haya borrado el backup) ni el de un zip existente.
fn unique_epoch(dir: &Path) -> i64 {
    static LAST: AtomicI64 = AtomicI64::new(0);
    let mut n = now_secs().max(LAST.load(Ordering::Relaxed) + 1);
    while dir.join(format!("backup-{n}.zip")).exists()
        || dir.join(format!("backup-{n}.zip.part")).exists()
    {
        n += 1;
    }
    LAST.fetch_max(n, Ordering::Relaxed);
    n
}

/// Deja como mucho `max` automáticos sin fijar (los más nuevos). Manuales y
/// fijados no se tocan jamás.
async fn apply_retention(dir: &Path, entries: &mut Vec<BackupEntry>, max: usize) {
    let mut candidates: Vec<&BackupEntry> = entries
        .iter()
        .filter(|e| e.automatic && !e.pinned)
        .collect();
    if candidates.len() <= max {
        return;
    }
    candidates.sort_by(|a, b| (a.created_at, &a.id).cmp(&(b.created_at, &b.id)));
    let doomed: Vec<String> = candidates[..candidates.len() - max]
        .iter()
        .map(|e| e.id.clone())
        .collect();
    for id in &doomed {
        let path = dir.join(format!("{id}.zip"));
        match tokio::fs::remove_file(&path).await {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "no se pudo borrar un backup antiguo");
            }
        }
    }
    entries.retain(|e| !doomed.contains(&e.id));
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T> + Send + 'static,
    path: &Path,
) -> Result<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::io(path, io::Error::other(format!("tarea interrumpida: {e}"))))?
}

fn zip_error(path: &Path, e: zip::result::ZipError) -> Error {
    match e {
        zip::result::ZipError::Io(e) => Error::io(path, e),
        other => Error::InvalidInput(format!("{}: {other}", path.display())),
    }
}

/// Emite `Progress` cada 32 archivos y al terminar.
struct Ticker {
    events: EventBus,
    task_id: u64,
    stage: &'static str,
    total: u64,
    done: u64,
}

impl Ticker {
    fn tick(&mut self) {
        self.done += 1;
        if self.done.is_multiple_of(32) || self.done >= self.total {
            self.events.emit(Event::Progress {
                task_id: self.task_id,
                stage: self.stage.into(),
                done: self.done,
                total: self.total.max(self.done),
                unit: ProgressUnit::Files,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Crear el zip
// ---------------------------------------------------------------------------

struct ZipJob {
    instance_dir: PathBuf,
    game_dir: PathBuf,
    part: PathBuf,
    dest: PathBuf,
    task_id: u64,
    events: EventBus,
}

enum ItemKind {
    Dir,
    File { size: u64, mode: u32 },
}

struct Item {
    abs: PathBuf,
    /// Nombre dentro del zip (con `/`).
    name: String,
    kind: ItemKind,
}

impl ZipJob {
    fn run(self) -> Result<()> {
        let result = self.build();
        if result.is_err() {
            let _ = fs::remove_file(&self.part);
        }
        result
    }

    fn build(&self) -> Result<()> {
        let mut items = Vec::new();
        // Metadatos de la instancia.
        for file in ["instance.json", "content.json"] {
            let abs = self.instance_dir.join(file);
            match fs::metadata(&abs) {
                Ok(meta) if meta.is_file() => items.push(Item {
                    abs,
                    name: file.to_owned(),
                    kind: ItemKind::File {
                        size: meta.len(),
                        mode: mode_of(&meta),
                    },
                }),
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound && file == "content.json" => {}
                Err(e) => return Err(Error::io(&abs, e)),
            }
        }
        items.push(Item {
            abs: self.game_dir.clone(),
            name: "game/".into(),
            kind: ItemKind::Dir,
        });
        collect_game_items(&self.game_dir, &mut items)?;

        let file = fs::File::create(&self.part).map_err(|e| Error::io(&self.part, e))?;
        let mut zip = zip::ZipWriter::new(io::BufWriter::new(file));
        let mut ticker = Ticker {
            events: self.events.clone(),
            task_id: self.task_id,
            stage: "backup",
            total: items.len() as u64,
            done: 0,
        };
        let base = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for item in &items {
            match item.kind {
                ItemKind::Dir => {
                    zip.add_directory(item.name.as_str(), base)
                        .map_err(|e| zip_error(&self.part, e))?;
                }
                ItemKind::File { size, mode } => {
                    let mut src = match fs::File::open(&item.abs) {
                        Ok(f) => f,
                        // Desapareció entre el listado y la copia.
                        Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                        Err(e) => return Err(Error::io(&item.abs, e)),
                    };
                    let opts = base
                        .unix_permissions(mode)
                        .large_file(size >= u64::from(u32::MAX));
                    zip.start_file(item.name.as_str(), opts)
                        .map_err(|e| zip_error(&self.part, e))?;
                    io::copy(&mut src, &mut zip).map_err(|e| Error::io(&item.abs, e))?;
                }
            }
            ticker.tick();
        }
        let mut writer = zip.finish().map_err(|e| zip_error(&self.part, e))?;
        writer.flush().map_err(|e| Error::io(&self.part, e))?;
        let file = writer
            .into_inner()
            .map_err(|e| Error::io(&self.part, e.into_error()))?;
        file.sync_all().map_err(|e| Error::io(&self.part, e))?;
        drop(file);
        fs::rename(&self.part, &self.dest).map_err(|e| Error::io(&self.dest, e))
    }
}

#[cfg(unix)]
fn mode_of(meta: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o777
}

#[cfg(not(unix))]
fn mode_of(_meta: &fs::Metadata) -> u32 {
    0o644
}

/// Lista ordenada de lo que entra en el backup bajo `game/`.
fn collect_game_items(game_dir: &Path, items: &mut Vec<Item>) -> Result<()> {
    let root = match fs::canonicalize(game_dir) {
        Ok(r) => r,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(Error::io(game_dir, e)),
    };
    let mut visited = HashSet::from([root.clone()]);
    walk(game_dir, "", &root, &mut visited, items)
}

/// `visited`: carpetas canónicas ya recorridas, para no entrar en bucles por
/// enlaces que apuntan a un antecesor.
fn walk(
    dir: &Path,
    prefix: &str,
    root: &Path,
    visited: &mut HashSet<PathBuf>,
    items: &mut Vec<Item>,
) -> Result<()> {
    let mut children = fs::read_dir(dir)
        .map_err(|e| Error::io(dir, e))?
        .collect::<io::Result<Vec<_>>>()
        .map_err(|e| Error::io(dir, e))?;
    children.sort_by_key(|c| c.file_name());
    for child in children {
        let file_name = child.file_name();
        let Some(name) = file_name.to_str() else {
            tracing::warn!(path = %child.path().display(), "nombre no UTF-8: se omite del backup");
            continue;
        };
        let rel = if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}/{name}")
        };
        if (prefix.is_empty() && EXCLUDED_TOP_LEVEL.contains(&name)) || name.ends_with(".tmp") {
            continue;
        }
        let path = child.path();
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(Error::io(&path, e)),
        };
        let zip_name = format!("game/{rel}");
        if meta.file_type().is_symlink() {
            let Ok(target) = fs::canonicalize(&path) else {
                continue;
            };
            let Ok(tmeta) = fs::metadata(&path) else {
                continue;
            };
            if tmeta.is_dir() {
                // Dentro del juego su contenido ya entra por su ruta real.
                if target.starts_with(root) {
                    continue;
                }
                if root.starts_with(&target) || !visited.insert(target.clone()) {
                    tracing::warn!(path = %path.display(), "enlace repetido o en bucle: se omite del backup");
                    continue;
                }
                items.push(Item {
                    abs: path.clone(),
                    name: format!("{zip_name}/"),
                    kind: ItemKind::Dir,
                });
                walk(&path, &rel, root, visited, items)?;
            } else if tmeta.is_file() {
                if !target.starts_with(root) {
                    tracing::warn!(path = %path.display(), "enlace a un archivo fuera de game/: se omite del backup");
                    continue;
                }
                items.push(Item {
                    abs: path,
                    name: zip_name,
                    kind: ItemKind::File {
                        size: tmeta.len(),
                        mode: mode_of(&tmeta),
                    },
                });
            }
        } else if meta.is_dir() {
            items.push(Item {
                abs: path.clone(),
                name: format!("{zip_name}/"),
                kind: ItemKind::Dir,
            });
            walk(&path, &rel, root, visited, items)?;
        } else if meta.is_file() {
            items.push(Item {
                abs: path,
                name: zip_name,
                kind: ItemKind::File {
                    size: meta.len(),
                    mode: mode_of(&meta),
                },
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Restaurar
// ---------------------------------------------------------------------------

struct RestoreJob {
    backup_id: String,
    archive: PathBuf,
    instance_dir: PathBuf,
    game_dir: PathBuf,
    mode: RestoreMode,
    task_id: u64,
    events: EventBus,
}

struct Extracted {
    files: u64,
    /// Había alguna entrada bajo el prefijo (aunque fuera solo el directorio).
    matched: bool,
}

impl RestoreJob {
    fn run(self) -> Result<RestoreReport> {
        let open = || -> Result<zip::ZipArchive<fs::File>> {
            let file = fs::File::open(&self.archive).map_err(|e| Error::io(&self.archive, e))?;
            zip::ZipArchive::new(file).map_err(|e| zip_error(&self.archive, e))
        };
        // Primera pasada: nombres seguros y formato.
        let mut zip = open()?;
        let mut layout = BackupLayout::LegacyRoxy;
        let mut has_instance_json = false;
        let mut has_content = false;
        let mut total = 0u64;
        for i in 0..zip.len() {
            let entry = zip
                .by_index_raw(i)
                .map_err(|e| zip_error(&self.archive, e))?;
            let Some(rel) = entry.enclosed_name() else {
                return Err(Error::InvalidInput(format!(
                    "el backup contiene una ruta insegura ({:?}); no se restaura",
                    String::from_utf8_lossy(entry.name_raw())
                )));
            };
            let rel = path_to_slash(&rel);
            if rel == "game" || rel.starts_with("game/") {
                layout = BackupLayout::Current;
            }
            has_instance_json |= rel == "instance.json";
            has_content |= rel == "content.json";
            total += 1;
        }
        drop(zip);

        let epoch = now_secs();
        let mut notes = Vec::new();
        let mut ticker = Ticker {
            events: self.events.clone(),
            task_id: self.task_id,
            stage: "restore",
            total,
            done: 0,
        };
        let strip = match (layout, self.mode) {
            (BackupLayout::Current, RestoreMode::Full) => "game/",
            (BackupLayout::Current, RestoreMode::WorldsOnly) => "game/saves/",
            (BackupLayout::LegacyRoxy, RestoreMode::Full) => "",
            (BackupLayout::LegacyRoxy, RestoreMode::WorldsOnly) => "saves/",
        };
        let legacy_filter = layout == BackupLayout::LegacyRoxy && self.mode == RestoreMode::Full;

        // Dónde se extrae y qué se sustituye.
        let (target, parent) = match self.mode {
            RestoreMode::Full => (self.game_dir.clone(), self.instance_dir.clone()),
            RestoreMode::WorldsOnly => (self.game_dir.join("saves"), self.game_dir.clone()),
        };
        fs::create_dir_all(&parent).map_err(|e| Error::io(&parent, e))?;
        let target_name = target
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::InvalidPath(target.display().to_string()))?
            .to_owned();
        let staging = parent.join(format!("{target_name}.new-{epoch}"));
        remove_any(&staging)?;
        fs::create_dir_all(&staging).map_err(|e| Error::io(&staging, e))?;

        let extracted = (|| -> Result<(Extracted, Option<Vec<u8>>)> {
            let mut zip = open()?;
            let extracted = extract_filtered(
                &mut zip,
                &self.archive,
                &staging,
                strip,
                |rel| !legacy_filter || !legacy_excluded(rel),
                &mut ticker,
            )?;
            // `content.json` solo en formato nuevo y restauración completa.
            let content =
                if layout == BackupLayout::Current && self.mode == RestoreMode::Full && has_content
                {
                    let mut entry = zip
                        .by_name("content.json")
                        .map_err(|e| zip_error(&self.archive, e))?;
                    let mut buf = Vec::new();
                    entry
                        .read_to_end(&mut buf)
                        .map_err(|e| Error::io(&self.archive, e))?;
                    Some(buf)
                } else {
                    None
                };
            Ok((extracted, content))
        })();
        let (extracted, content) = match extracted {
            Ok(v) => v,
            Err(e) => {
                let _ = fs::remove_dir_all(&staging);
                return Err(e);
            }
        };
        if self.mode == RestoreMode::WorldsOnly && !extracted.matched {
            let _ = fs::remove_dir_all(&staging);
            return Err(Error::InvalidInput(
                "el backup no contiene mundos (saves/)".into(),
            ));
        }

        // Intercambio: lo actual se aparta, lo restaurado entra y solo después se borra lo viejo.
        let old = swap_in(&target, &staging, epoch).inspect_err(|_| {
            let _ = fs::remove_dir_all(&staging);
        })?;
        let mut content_restored = false;
        if let Some(bytes) = content {
            let dest = self.instance_dir.join("content.json");
            if let Err(e) = write_atomic_std(&dest, &bytes) {
                rollback_swap(&target, old.as_deref());
                return Err(e);
            }
            content_restored = true;
        }
        if let Some(old) = &old {
            if self.mode == RestoreMode::Full {
                // Lo que el backup nunca guarda se conserva.
                for keep in EXCLUDED_TOP_LEVEL {
                    let (src, dst) = (old.join(keep), target.join(keep));
                    if src.exists() && !dst.exists() {
                        let _ = fs::rename(&src, &dst);
                    }
                }
            }
            if let Err(e) = fs::remove_dir_all(old) {
                notes.push(format!(
                    "no se pudo borrar la copia previa {}: {e}",
                    old.display()
                ));
            }
        }

        let instance_settings_kept = layout == BackupLayout::Current && has_instance_json;
        match (layout, self.mode) {
            (BackupLayout::Current, RestoreMode::Full) => {
                if instance_settings_kept {
                    notes.push(
                        "se conservan los ajustes actuales de la instancia (instance.json del backup no se aplica)"
                            .into(),
                    );
                }
                if !content_restored {
                    notes.push("el backup no incluye content.json: se conserva el actual".into());
                }
            }
            (BackupLayout::LegacyRoxy, RestoreMode::Full) => notes.push(
                "backup heredado de Roxy: no se aplican instance.json, .modrinth_sources.json ni registros de lanzamiento"
                    .into(),
            ),
            (_, RestoreMode::WorldsOnly) => {
                notes.push("solo se restauró saves/; el resto de la instancia no cambia".into());
            }
        }
        Ok(RestoreReport {
            backup_id: self.backup_id,
            mode: self.mode,
            layout,
            files: extracted.files,
            content_restored,
            instance_settings_kept,
            notes,
        })
    }
}

/// Lo que un zip heredado de Roxy lleva y no es del juego.
fn legacy_excluded(rel: &str) -> bool {
    if rel == "instance.json"
        || rel == ".modrinth_sources.json"
        || rel == "backups"
        || rel.starts_with("backups/")
    {
        return true;
    }
    rel.rsplit('/')
        .next()
        .is_some_and(|name| name.starts_with("latest_launch.log"))
}

fn path_to_slash(p: &Path) -> String {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Extrae en `dest` las entradas bajo `strip` (sin ese prefijo) que cumplan `select`.
/// Una ruta insegura aborta toda la extracción.
fn extract_filtered(
    zip: &mut zip::ZipArchive<fs::File>,
    archive: &Path,
    dest: &Path,
    strip: &str,
    select: impl Fn(&str) -> bool,
    ticker: &mut Ticker,
) -> Result<Extracted> {
    let mut out = Extracted {
        files: 0,
        matched: false,
    };
    let strip_dir = strip.trim_end_matches('/');
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| zip_error(archive, e))?;
        let Some(rel) = entry.enclosed_name() else {
            return Err(Error::InvalidInput(
                "el backup contiene una ruta insegura; no se restaura".into(),
            ));
        };
        let rel = path_to_slash(&rel);
        ticker.tick();
        let rest = if strip.is_empty() {
            rel.as_str()
        } else if rel == strip_dir {
            out.matched = true;
            continue;
        } else if let Some(rest) = rel.strip_prefix(strip) {
            rest
        } else {
            continue;
        };
        out.matched = true;
        if rest.is_empty() || !select(rest) {
            continue;
        }
        if entry.is_symlink() {
            tracing::warn!(entry = %rel, "enlace simbólico del backup omitido");
            continue;
        }
        let target = dest.join(rest);
        if entry.is_dir() {
            fs::create_dir_all(&target).map_err(|e| Error::io(&target, e))?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let mut file = fs::File::create(&target).map_err(|e| Error::io(&target, e))?;
        io::copy(&mut entry, &mut file).map_err(|e| Error::io(&target, e))?;
        drop(file);
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                fs::set_permissions(&target, fs::Permissions::from_mode((mode & 0o777) | 0o600));
        }
        out.files += 1;
    }
    Ok(out)
}

fn remove_any(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => fs::remove_dir_all(path).map_err(|e| Error::io(path, e)),
        Ok(_) => fs::remove_file(path).map_err(|e| Error::io(path, e)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::io(path, e)),
    }
}

/// Aparta `current` a `<current>.old-<epoch>` y pone `new` en su sitio.
/// Devuelve dónde quedó lo anterior (si existía). Si falla, deja todo como estaba.
fn swap_in(current: &Path, new: &Path, epoch: i64) -> Result<Option<PathBuf>> {
    let name = current
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::InvalidPath(current.display().to_string()))?;
    let parent = current
        .parent()
        .ok_or_else(|| Error::InvalidPath(current.display().to_string()))?;
    let mut old = None;
    if fs::symlink_metadata(current).is_ok() {
        let mut dest = parent.join(format!("{name}.old-{epoch}"));
        let mut n = 2;
        while fs::symlink_metadata(&dest).is_ok() {
            dest = parent.join(format!("{name}.old-{epoch}-{n}"));
            n += 1;
        }
        fs::rename(current, &dest).map_err(|e| Error::io(current, e))?;
        old = Some(dest);
    }
    if let Err(e) = fs::rename(new, current) {
        if let Some(old) = &old {
            let _ = fs::rename(old, current);
        }
        return Err(Error::io(current, e));
    }
    Ok(old)
}

/// Deshace [`swap_in`] cuando algo posterior falla.
fn rollback_swap(current: &Path, old: Option<&Path>) {
    let _ = remove_any(current);
    if let Some(old) = old {
        let _ = fs::rename(old, current);
    }
}

fn write_atomic_std(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| Error::InvalidPath(path.display().to_string()))?;
    let tmp = dir.join(format!(".content.{:08x}.tmp", fastrand::u32(..)));
    let result = (|| {
        let mut f = fs::File::create(&tmp).map_err(|e| Error::io(&tmp, e))?;
        f.write_all(bytes).map_err(|e| Error::io(&tmp, e))?;
        f.sync_all().map_err(|e| Error::io(&tmp, e))?;
        fs::rename(&tmp, path).map_err(|e| Error::io(path, e))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

// ---------------------------------------------------------------------------
// Pruebas
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Options;
    use crate::instance::{GameVersion, LoaderKind, NewInstance};
    use crate::launch::GameInfo;
    use crate::secrets::MemoryStore;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    async fn core(dir: &Path) -> Core {
        Core::open(Options {
            data_dir: Some(dir.to_path_buf()),
            secrets: Some(Arc::new(MemoryStore::default())),
            offline: true,
            ..Options::default()
        })
        .await
        .unwrap()
    }

    async fn instance(core: &Core) -> String {
        core.create_instance(NewInstance {
            name: "Pruebas".into(),
            game: GameVersion {
                mc: "1.21".into(),
                loader: LoaderKind::Vanilla,
                loader_version: None,
            },
        })
        .await
        .unwrap()
        .id
    }

    fn put(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    /// Contenido de un árbol como mapa ruta → texto (`None` = directorio).
    fn snapshot(dir: &Path) -> BTreeMap<String, Option<String>> {
        fn go(base: &Path, dir: &Path, out: &mut BTreeMap<String, Option<String>>) {
            for e in fs::read_dir(dir).unwrap() {
                let p = e.unwrap().path();
                let rel = p.strip_prefix(base).unwrap().display().to_string();
                if p.is_dir() {
                    out.insert(rel, None);
                    go(base, &p, out);
                } else {
                    out.insert(rel, Some(fs::read_to_string(&p).unwrap()));
                }
            }
        }
        let mut out = BTreeMap::new();
        go(dir, dir, &mut out);
        out
    }

    fn fake_game(core: &Core, id: &str, instance_id: &str) {
        core.inner.games.try_lock().unwrap().insert(
            id.parse().unwrap(),
            GameInfo {
                game_id: id.parse().unwrap(),
                instance_id: instance_id.to_owned(),
                pid: None,
                started_at: 0,
            },
        );
    }

    fn write_zip(path: &Path, files: &[(&str, &str)]) {
        let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
        for (name, body) in files {
            if name.ends_with('/') {
                zip.add_directory(*name, SimpleFileOptions::default())
                    .unwrap();
            } else {
                zip.start_file(*name, SimpleFileOptions::default()).unwrap();
                zip.write_all(body.as_bytes()).unwrap();
            }
        }
        zip.finish().unwrap();
    }

    fn zip_names(path: &Path) -> Vec<String> {
        let mut zip = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
        (0..zip.len())
            .map(|i| zip.by_index_raw(i).unwrap().name().unwrap().into_owned())
            .collect()
    }

    #[tokio::test]
    async fn crear_excluye_logs_tmp_y_enlaces_externos() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        let game = core.instance_game_dir(&id).unwrap();
        put(&game.join("options.txt"), "fov:70");
        put(&game.join("saves/Mundo/level.dat"), "nbt");
        put(&game.join("logs/latest.log"), "log");
        put(&game.join("crash-reports/c.txt"), "crash");
        put(&game.join("config/x.tmp"), "tmp");
        put(&game.join("config/x.toml"), "ok");
        put(
            &game.join("saves/Mundo/logs/keep.txt"),
            "no es el logs raíz",
        );
        let outside = tmp.path().join("fuera.txt");
        fs::write(&outside, "secreto").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, game.join("enlace-fuera")).unwrap();
            std::os::unix::fs::symlink(game.join("options.txt"), game.join("enlace-dentro"))
                .unwrap();
            // Carpeta de mundos en otro sitio, con un enlace de vuelta a sí misma.
            let mundos = tmp.path().join("mundos-fuera");
            put(&mundos.join("Otro/level.dat"), "nbt");
            std::os::unix::fs::symlink(&mundos, mundos.join("bucle")).unwrap();
            std::os::unix::fs::symlink(&mundos, game.join("saves-externo")).unwrap();
            std::os::unix::fs::symlink(&game, game.join("raiz")).unwrap();
        }

        let mut rx = core.subscribe();
        let entry = core
            .create_backup(
                &id,
                BackupRequest {
                    label: Some("  antes de mods ".into()),
                    pinned: true,
                    automatic: false,
                },
            )
            .await
            .unwrap();
        assert_eq!(entry.label.as_deref(), Some("antes de mods"));
        assert!(entry.pinned && !entry.automatic);
        let zip_path = core
            .paths()
            .backups_dir()
            .join(&id)
            .join(format!("{}.zip", entry.id));
        assert_eq!(entry.size_bytes, fs::metadata(&zip_path).unwrap().len());
        let names = zip_names(&zip_path);
        for expected in [
            "instance.json",
            "game/options.txt",
            "game/saves/Mundo/level.dat",
            "game/config/x.toml",
            "game/saves/Mundo/logs/keep.txt",
        ] {
            assert!(names.contains(&expected.to_owned()), "falta {expected}");
        }
        for bad in ["game/logs/latest.log", "game/crash-reports/c.txt"] {
            assert!(!names.contains(&bad.to_owned()), "sobra {bad}");
        }
        assert!(!names.iter().any(|n| n.ends_with(".tmp")));
        assert!(!names.iter().any(|n| n.contains("enlace-fuera")));
        #[cfg(unix)]
        {
            assert!(names.contains(&"game/enlace-dentro".to_owned()));
            assert!(names.contains(&"game/saves-externo/Otro/level.dat".to_owned()));
            assert!(
                !names
                    .iter()
                    .any(|n| n.contains("/bucle") || n.contains("game/raiz"))
            );
        }
        // Sin restos .part.
        let leftovers: Vec<_> = fs::read_dir(zip_path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".part"))
            .collect();
        assert!(leftovers.is_empty());
        // Eventos de progreso y fin.
        let mut progress = false;
        let mut finished = false;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                Event::Progress { stage, .. } if stage == "backup" => progress = true,
                Event::TaskFinished { ok: true, .. } => finished = true,
                _ => {}
            }
        }
        assert!(progress && finished);
        // Se lista.
        let list = core.list_backups(&id).await.unwrap();
        assert_eq!(list, vec![entry]);
    }

    #[tokio::test]
    async fn ida_y_vuelta_completa_y_solo_mundos() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        let game = core.instance_game_dir(&id).unwrap();
        put(&game.join("options.txt"), "v1");
        put(&game.join("mods/a.jar"), "jar-a");
        put(&game.join("saves/Uno/level.dat"), "uno");
        put(&game.join("saves/Uno/region/r.0.0.mca"), "region");
        put(
            &core.instance_dir(&id).unwrap().join("content.json"),
            "{\"v\":1}",
        );
        let before = snapshot(&game);
        let backup = core
            .create_backup(&id, BackupRequest::default())
            .await
            .unwrap();

        // Se modifica todo.
        put(&game.join("options.txt"), "v2");
        fs::remove_file(game.join("mods/a.jar")).unwrap();
        put(&game.join("mods/b.jar"), "jar-b");
        put(&game.join("saves/Uno/level.dat"), "roto");
        put(&game.join("saves/Dos/level.dat"), "dos");
        put(&game.join("logs/latest.log"), "mi log");
        put(
            &core.instance_dir(&id).unwrap().join("content.json"),
            "{\"v\":2}",
        );
        let mut inst = core.get_instance(&id).await.unwrap();
        inst.name = "Renombrada".into();
        core.save_instance(&inst).await.unwrap();

        // Solo mundos.
        let report = core
            .restore_backup(&id, &backup.id, RestoreMode::WorldsOnly)
            .await
            .unwrap();
        assert_eq!(report.layout, BackupLayout::Current);
        assert_eq!(fs::read_to_string(game.join("options.txt")).unwrap(), "v2");
        assert_eq!(
            fs::read_to_string(game.join("saves/Uno/level.dat")).unwrap(),
            "uno"
        );
        assert!(!game.join("saves/Dos").exists());
        assert!(game.join("mods/b.jar").exists());
        let leftovers: Vec<_> = fs::read_dir(&game)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".old-") || n.contains(".new-"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");

        // Completa.
        let report = core
            .restore_backup(&id, &backup.id, RestoreMode::Full)
            .await
            .unwrap();
        assert!(report.content_restored && report.instance_settings_kept);
        let mut after = snapshot(&game);
        // Los logs actuales se conservan; fuera de eso, el árbol es idéntico al original.
        assert_eq!(
            after.remove("logs/latest.log"),
            Some(Some("mi log".to_owned()))
        );
        after.remove("logs");
        assert_eq!(after, before);
        let inst_dir = core.instance_dir(&id).unwrap();
        assert_eq!(
            fs::read_to_string(inst_dir.join("content.json")).unwrap(),
            "{\"v\":1}"
        );
        // Los ajustes actuales no se tocan.
        assert_eq!(core.get_instance(&id).await.unwrap().name, "Renombrada");
        let stray: Vec<_> = fs::read_dir(&inst_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".old-") || n.contains(".new-"))
            .collect();
        assert!(stray.is_empty(), "{stray:?}");
    }

    #[tokio::test]
    async fn restaura_formato_heredado_de_roxy() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        let game = core.instance_game_dir(&id).unwrap();
        put(&game.join("old.txt"), "viejo");
        let dir = core.paths().backups_dir().join(&id);
        fs::create_dir_all(&dir).unwrap();
        write_zip(
            &dir.join("backup-1700000000.zip"),
            &[
                ("instance.json", "{\"token\":\"x\"}"),
                (".modrinth_sources.json", "{}"),
                ("latest_launch.log", "TOKEN"),
                ("logs/latest_launch.log.1", "TOKEN"),
                ("backups/backup-1.zip", "recursivo"),
                ("options.txt", "fov:90"),
                ("saves/", ""),
                ("saves/Mundo/level.dat", "nbt"),
                ("mods/m.jar", "jar"),
            ],
        );
        // Sin entrada en backups.json: aparece como manual.
        let list = core.list_backups(&id).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].created_at, 1_700_000_000);
        assert!(!list[0].automatic && !list[0].pinned);

        let report = core
            .restore_backup(&id, "backup-1700000000", RestoreMode::Full)
            .await
            .unwrap();
        assert_eq!(report.layout, BackupLayout::LegacyRoxy);
        assert!(!report.content_restored);
        assert!(game.join("options.txt").exists());
        assert!(game.join("saves/Mundo/level.dat").exists());
        assert!(game.join("mods/m.jar").exists());
        assert!(!game.join("old.txt").exists());
        for bad in [
            "instance.json",
            ".modrinth_sources.json",
            "latest_launch.log",
            "logs/latest_launch.log.1",
            "backups",
        ] {
            assert!(!game.join(bad).exists(), "{bad} no debería estar en game/");
        }

        // Solo mundos desde formato heredado.
        put(&game.join("saves/Mundo/level.dat"), "cambiado");
        core.restore_backup(&id, "backup-1700000000", RestoreMode::WorldsOnly)
            .await
            .unwrap();
        assert_eq!(
            fs::read_to_string(game.join("saves/Mundo/level.dat")).unwrap(),
            "nbt"
        );
    }

    #[tokio::test]
    async fn rechaza_zip_slip_sin_tocar_nada() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        let game = core.instance_game_dir(&id).unwrap();
        put(&game.join("intacto.txt"), "sigue");
        let dir = core.paths().backups_dir().join(&id);
        fs::create_dir_all(&dir).unwrap();
        write_zip(
            &dir.join("backup-5.zip"),
            &[("game/ok.txt", "x"), ("game/../../../escape.txt", "mal")],
        );
        for mode in [RestoreMode::Full, RestoreMode::WorldsOnly] {
            let err = core
                .restore_backup(&id, "backup-5", mode)
                .await
                .unwrap_err();
            assert!(err.to_string().contains("insegura"), "{err}");
        }
        assert_eq!(
            fs::read_to_string(game.join("intacto.txt")).unwrap(),
            "sigue"
        );
        assert!(!tmp.path().join("escape.txt").exists());
        assert!(!core.instance_dir(&id).unwrap().join("escape.txt").exists());
        assert!(
            core.restore_backup(&id, "../x", RestoreMode::Full)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn se_niega_con_el_juego_en_marcha() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        let backup = core
            .create_backup(&id, BackupRequest::default())
            .await
            .unwrap();
        fake_game(&core, "7", &id);
        let e = core
            .create_backup(&id, BackupRequest::default())
            .await
            .unwrap_err();
        assert!(e.to_string().contains("en marcha"), "{e}");
        assert!(
            core.restore_backup(&id, &backup.id, RestoreMode::Full)
                .await
                .is_err()
        );
        // Otra instancia no se ve afectada.
        let other = core
            .create_instance(NewInstance {
                name: "Otra".into(),
                game: GameVersion {
                    mc: "1.21".into(),
                    loader: LoaderKind::Vanilla,
                    loader_version: None,
                },
            })
            .await
            .unwrap();
        core.create_backup(&other.id, BackupRequest::default())
            .await
            .unwrap();
    }

    fn auto() -> BackupRequest {
        BackupRequest {
            automatic: true,
            ..BackupRequest::default()
        }
    }

    #[tokio::test]
    async fn retencion_solo_toca_automaticos_sin_fijar() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        core.update_backup_settings(
            &id,
            BackupSettings {
                auto: true,
                interval_hours: None,
                max_count: Some(2),
            },
        )
        .await
        .unwrap();
        let manual = core
            .create_backup(&id, BackupRequest::default())
            .await
            .unwrap();
        let fijado = core
            .create_backup(
                &id,
                BackupRequest {
                    pinned: true,
                    automatic: true,
                    ..BackupRequest::default()
                },
            )
            .await
            .unwrap();
        let mut autos = Vec::new();
        for n in 0..4 {
            let e = core.create_backup(&id, auto()).await.unwrap();
            // Se espacian en el tiempo para que el orden sea determinista.
            let _guard = INDEX_LOCK.lock().await;
            let mut entries = core.load_entries(&id).await.unwrap();
            entries
                .iter_mut()
                .find(|x| x.id == e.id)
                .unwrap()
                .created_at = 1_000 + n;
            core.save_entries(&id, entries).await.unwrap();
            autos.push(e.id);
        }
        let ids: Vec<String> = core
            .list_backups(&id)
            .await
            .unwrap()
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert!(ids.contains(&manual.id) && ids.contains(&fijado.id));
        assert!(ids.contains(&autos[2]) && ids.contains(&autos[3]));
        assert!(!ids.contains(&autos[0]) && !ids.contains(&autos[1]));
        let dir = core.paths().backups_dir().join(&id);
        assert!(!dir.join(format!("{}.zip", autos[0])).exists());
        assert!(dir.join(format!("{}.zip", manual.id)).exists());
        assert_eq!(ids.len(), 4);
        // Fijar uno automático lo salva de la próxima poda.
        core.set_backup_pinned(&id, &autos[2], true).await.unwrap();
        core.create_backup(&id, auto()).await.unwrap();
        core.create_backup(&id, auto()).await.unwrap();
        let ids: Vec<String> = core
            .list_backups(&id)
            .await
            .unwrap()
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert!(ids.contains(&autos[2]));
        assert!(!ids.contains(&autos[3]));
    }

    #[tokio::test]
    async fn etiquetas_fijado_y_borrado() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        let b = core
            .create_backup(&id, BackupRequest::default())
            .await
            .unwrap();
        let b2 = core.set_backup_pinned(&id, &b.id, true).await.unwrap();
        assert!(b2.pinned);
        let b3 = core
            .set_backup_label(&id, &b.id, Some(" hito ".into()))
            .await
            .unwrap();
        assert_eq!(b3.label.as_deref(), Some("hito"));
        let b4 = core
            .set_backup_label(&id, &b.id, Some("  ".into()))
            .await
            .unwrap();
        assert_eq!(b4.label, None);
        assert!(core.set_backup_pinned(&id, "backup-0", true).await.is_err());
        core.delete_backup(&id, &b.id).await.unwrap();
        assert!(core.list_backups(&id).await.unwrap().is_empty());
        assert!(core.delete_backup(&id, &b.id).await.is_err());
        assert!(
            core.update_backup_settings(
                &id,
                BackupSettings {
                    auto: true,
                    interval_hours: Some(0),
                    max_count: None
                }
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn indice_corrupto_se_reconstruye() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        let b = core
            .create_backup(&id, BackupRequest::default())
            .await
            .unwrap();
        let dir = core.paths().backups_dir().join(&id);
        fs::write(dir.join("backups.json"), "{no es json").unwrap();
        let list = core.list_backups(&id).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, b.id);
        assert!(fs::read_dir(&dir).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .contains("corrupto")
        }));
    }

    #[tokio::test]
    async fn planificador_respeta_intervalo_y_juego_en_marcha() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let a = instance(&core).await;
        let b = core
            .create_instance(NewInstance {
                name: "Segunda".into(),
                game: GameVersion {
                    mc: "1.21".into(),
                    loader: LoaderKind::Vanilla,
                    loader_version: None,
                },
            })
            .await
            .unwrap()
            .id;
        // Ninguna tiene backups activados.
        assert!(core.run_backup_scheduler_once().await.is_empty());
        for id in [&a, &b] {
            core.update_backup_settings(
                id,
                BackupSettings {
                    auto: true,
                    interval_hours: Some(6),
                    max_count: None,
                },
            )
            .await
            .unwrap();
        }
        // Primera pasada: ambas sin copia previa.
        let first = core.run_backup_scheduler_once().await;
        assert_eq!(first.len(), 2);
        assert!(
            first
                .iter()
                .all(|(_, r)| r.as_ref().is_ok_and(|e| e.automatic))
        );
        // Inmediatamente después, nada que hacer.
        assert!(core.run_backup_scheduler_once().await.is_empty());

        // Se envejece el de `a` 5 h (aún dentro del intervalo) y el de `b` 7 h.
        {
            let _g = INDEX_LOCK.lock().await;
            for (id, hours) in [(&a, 5), (&b, 7)] {
                let mut entries = core.load_entries(id).await.unwrap();
                entries[0].created_at = now_secs() - hours * 3600;
                core.save_entries(id, entries).await.unwrap();
            }
        }
        let second = core.run_backup_scheduler_once().await;
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].0, b);

        // Con el juego de `a` en marcha se salta aunque le toque.
        {
            let _g = INDEX_LOCK.lock().await;
            let mut entries = core.load_entries(&a).await.unwrap();
            entries[0].created_at = now_secs() - 48 * 3600;
            core.save_entries(&a, entries).await.unwrap();
        }
        fake_game(&core, "9", &a);
        assert!(core.run_backup_scheduler_once().await.is_empty());
    }

    #[tokio::test]
    async fn planificador_en_segundo_plano_se_puede_abortar() {
        let tmp = tempfile::tempdir().unwrap();
        let core = core(tmp.path()).await;
        let id = instance(&core).await;
        core.update_backup_settings(
            &id,
            BackupSettings {
                auto: true,
                ..BackupSettings::default()
            },
        )
        .await
        .unwrap();
        let handle = core.spawn_backup_scheduler(Duration::from_secs(3600));
        for _ in 0..100 {
            if !core.list_backups(&id).await.unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        handle.abort();
        assert_eq!(core.list_backups(&id).await.unwrap().len(), 1);
    }

    #[test]
    fn formato_del_indice_es_compatible() {
        let json = r#"{"schema":1,"entries":[{"id":"backup-1","created_at":2,"size_bytes":3,"pinned":false,"label":null,"automatic":true}]}"#;
        let idx: BackupIndex = serde_json::from_str(json).unwrap();
        assert_eq!(idx.entries[0].created_at, 2);
        assert!(idx.entries[0].automatic);
        assert!(legacy_excluded("latest_launch.log.2"));
        assert!(legacy_excluded("logs/latest_launch.log"));
        assert!(!legacy_excluded("saves/instance.json"));
    }
}
