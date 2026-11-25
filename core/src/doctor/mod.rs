//! Doctor: diagnóstico de instancias que no arrancan o se cierran solas.
//! `analyze` es
//! una función pura sobre texto (se prueba sin disco); `Core::diagnose` reúne
//! los datos y `Core::apply_fix` (en `fixes`) aplica los arreglos.

mod fixes;
mod signatures;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub use fixes::FixAction;

use crate::instance::LoaderKind;
use crate::launcher::Core;
use crate::{Error, Result};

/// Cuánto se lee del final de `latest.log`.
const LOG_TAIL_BYTES: u64 = 400 * 1024;
/// Máximo que se lee de un crash report o de un `hs_err`.
const REPORT_MAX_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Java,
    Memory,
    JvmArgs,
    ModConflict,
    MissingDependency,
    CorruptedFile,
    GraphicsDriver,
    Loader,
    Account,
    Network,
    System,
    WorldCorruption,
    Unknown,
}

/// El orden de las variantes es el de presentación (lo más grave primero).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnosis {
    /// Identificador único dentro del informe (para `--fix`).
    pub id: String,
    /// Tipo de firma; estable, la UI lo usa para traducir.
    pub kind: String,
    pub category: Category,
    pub severity: Severity,
    pub title: String,
    pub explanation: String,
    /// Porcentaje de confianza (0-100).
    pub confidence: u8,
    pub culprit: Option<String>,
    pub params: BTreeMap<String, String>,
    pub fix: Option<FixAction>,
}

impl Diagnosis {
    pub(crate) fn new(
        kind: &str,
        category: Category,
        severity: Severity,
        confidence: u8,
        title: impl Into<String>,
        explanation: impl Into<String>,
    ) -> Self {
        Self {
            id: kind.to_owned(),
            kind: kind.to_owned(),
            category,
            severity,
            title: title.into(),
            explanation: explanation.into(),
            confidence: confidence.min(100),
            culprit: None,
            params: BTreeMap::new(),
            fix: None,
        }
    }

    pub(crate) fn culprit(mut self, culprit: impl Into<String>) -> Self {
        self.culprit = Some(culprit.into());
        self
    }

    pub(crate) fn param(mut self, key: &str, value: impl ToString) -> Self {
        self.params.insert(key.to_owned(), value.to_string());
        self
    }

    pub(crate) fn fix(mut self, fix: FixAction) -> Self {
        self.fix = Some(fix);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModFile {
    /// Nombre real en disco (`.jar` o `.jar.disabled`).
    pub file_name: String,
    pub enabled: bool,
}

/// Todo lo que `analyze` necesita; no toca el disco.
#[derive(Debug, Clone)]
pub struct DoctorInput {
    pub crash_text: Option<String>,
    pub hs_err_text: Option<String>,
    pub log_tail: Option<String>,
    pub mods: Vec<ModFile>,
    pub loader: LoaderKind,
    /// Memoria efectiva de la instancia.
    pub memory_mb: u32,
    pub total_memory_mb: Option<u64>,
    /// Argumentos JVM efectivos (los de la instancia o los globales).
    pub jvm_args: Vec<String>,
    pub required_java: Option<u32>,
}

impl Default for DoctorInput {
    fn default() -> Self {
        Self {
            crash_text: None,
            hs_err_text: None,
            log_tail: None,
            mods: Vec::new(),
            loader: LoaderKind::Vanilla,
            memory_mb: 4096,
            total_memory_mb: None,
            jvm_args: Vec::new(),
            required_java: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    /// Sin problemas graves ni avisos (los `info` no cuentan).
    pub healthy: bool,
    pub diagnoses: Vec<Diagnosis>,
    pub crash_excerpt: Option<String>,
    pub crash_report_path: Option<PathBuf>,
}

/// Analiza los datos reunidos. Resultado ordenado por gravedad y confianza.
pub fn analyze(input: &DoctorInput) -> Vec<Diagnosis> {
    let ctx = signatures::Ctx::new(input);
    let mut found = signatures::run_text(&ctx);
    if found.is_empty() && ctx.crashed {
        found.push(signatures::unknown_crash(input));
    }
    found.extend(signatures::run_static(input));

    found.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| b.confidence.cmp(&a.confidence))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    // Ids únicos: `kind`, `kind-2`, `kind-3`…
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for d in &mut found {
        let n = seen.entry(d.kind.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            d.id = format!("{}-{}", d.kind, n);
        }
    }
    found
}

/// Línea `Description:` del crash report o primer `Caused by:`; si no hay
/// crash report, lo mismo en el log o el marco problemático de `hs_err`.
pub fn crash_excerpt(input: &DoctorInput) -> Option<String> {
    if !signatures::is_crash(input) {
        return None;
    }
    fn line_with<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
        text.lines().map(str::trim).find(|l| l.starts_with(prefix))
    }
    let from_text = |text: &str| {
        line_with(text, "Description:")
            .or_else(|| line_with(text, "Caused by:"))
            .map(str::to_owned)
    };
    input
        .crash_text
        .as_deref()
        .and_then(from_text)
        .or_else(|| {
            let hs = input.hs_err_text.as_deref()?;
            let idx = hs.find("Problematic frame:")?;
            let frame: Vec<_> = hs[idx..]
                .lines()
                .take(2)
                .map(|l| l.trim_start_matches('#').trim())
                .collect();
            Some(frame.join(" "))
        })
        .or_else(|| input.log_tail.as_deref().and_then(from_text))
}

/// El archivo con la fecha de modificación más reciente que cumpla `accept`.
async fn newest_file(
    dir: &Path,
    since: Option<SystemTime>,
    accept: impl Fn(&str) -> bool,
) -> Result<Option<PathBuf>> {
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::io(dir, e)),
    };
    let mut best: Option<(SystemTime, PathBuf)> = None;
    while let Some(entry) = entries.next_entry().await.map_err(|e| Error::io(dir, e))? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !accept(&name) {
            continue;
        }
        let Ok(meta) = entry.metadata().await else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if since.is_some_and(|s| mtime < s) {
            continue;
        }
        if best.as_ref().is_none_or(|(t, _)| mtime > *t) {
            best = Some((mtime, entry.path()));
        }
    }
    Ok(best.map(|(_, p)| p))
}

/// Lee como mucho `max` bytes del principio del archivo.
async fn read_head(path: &Path, max: u64) -> Result<String> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|e| Error::io(path, e))?;
    let mut buf = Vec::new();
    file.take(max)
        .read_to_end(&mut buf)
        .await
        .map_err(|e| Error::io(path, e))?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Lee los últimos `max` bytes del archivo, sin la primera línea si quedó cortada.
async fn read_tail(path: &Path, max: u64) -> Result<String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| Error::io(path, e))?;
    let len = file.metadata().await.map_err(|e| Error::io(path, e))?.len();
    let start = len.saturating_sub(max);
    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|e| Error::io(path, e))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .await
        .map_err(|e| Error::io(path, e))?;
    let text = String::from_utf8_lossy(&buf);
    if start > 0
        && let Some(nl) = text.find('\n')
    {
        return Ok(text[nl + 1..].to_owned());
    }
    Ok(text.into_owned())
}

async fn list_mods(dir: &Path) -> Result<Vec<ModFile>> {
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(dir, e)),
    };
    let mut mods = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(|e| Error::io(dir, e))? {
        if !entry.file_type().await.is_ok_and(|t| t.is_file()) {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let lower = file_name.to_lowercase();
        if lower.ends_with(".jar") {
            mods.push(ModFile {
                file_name,
                enabled: true,
            });
        } else if lower.ends_with(".jar.disabled") {
            mods.push(ModFile {
                file_name,
                enabled: false,
            });
        }
    }
    mods.sort_by_key(|m| m.file_name.to_lowercase());
    Ok(mods)
}

fn total_memory_mb() -> Option<u64> {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let mb = sys.total_memory() / (1024 * 1024);
    (mb > 0).then_some(mb)
}

impl Core {
    /// Reúne crash reports, logs, mods y ajustes de la instancia y los analiza.
    pub async fn diagnose(&self, instance_id: &str) -> Result<DoctorReport> {
        let instance = self.get_instance(instance_id).await?;
        let instance_dir = self.instance_dir(instance_id)?;
        let game_dir = self.instance_game_dir(instance_id)?;
        let settings = self.settings().await;

        // Solo cuenta lo de la última partida (con margen), si se conoce.
        let since = self.last_launch(instance_id).await.ok().flatten().map(|r| {
            SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs((r.started_at - 60).max(0) as u64)
        });
        let crash_report_path = newest_file(&game_dir.join("crash-reports"), since, |n| {
            n.to_lowercase().ends_with(".txt")
        })
        .await?;
        let crash_text = match &crash_report_path {
            Some(p) => Some(read_head(p, REPORT_MAX_BYTES).await?),
            None => None,
        };
        let hs_err_text = match newest_file(&game_dir, since, |n| {
            n.starts_with("hs_err_pid") && n.ends_with(".log")
        })
        .await?
        {
            Some(p) => Some(read_head(&p, REPORT_MAX_BYTES).await?),
            None => None,
        };
        let mut log_tail = None;
        for candidate in [
            instance_dir.join("logs").join("latest.log"),
            game_dir.join("logs").join("latest.log"),
        ] {
            if tokio::fs::try_exists(&candidate).await.unwrap_or(false) {
                log_tail = Some(read_tail(&candidate, LOG_TAIL_BYTES).await?);
                break;
            }
        }

        let input = DoctorInput {
            crash_text,
            hs_err_text,
            log_tail,
            mods: list_mods(&game_dir.join("mods")).await?,
            loader: instance.game.loader,
            memory_mb: instance.memory_mb(settings.default_memory_mb),
            total_memory_mb: total_memory_mb(),
            jvm_args: instance
                .java
                .jvm_args
                .clone()
                .unwrap_or(settings.default_jvm_args),
            // Sin red: la del JSON de Minecraft ya instalado, si existe.
            required_java: self
                .installed_version_json(&instance.game.mc)
                .await
                .ok()
                .flatten()
                .map(|v| v.java_major()),
        };
        let diagnoses = analyze(&input);
        Ok(DoctorReport {
            healthy: diagnoses.iter().all(|d| d.severity == Severity::Info),
            crash_excerpt: crash_excerpt(&input),
            crash_report_path,
            diagnoses,
        })
    }
}

#[cfg(test)]
mod tests;
