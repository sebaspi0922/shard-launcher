//! Java gestionada: detección de las instaladas, sondeo de versión e
//! instalación de JRE de Adoptium en `<datos>/java/<major>/` (docs/arquitectura.md §4).

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::task::JoinSet;

use crate::events::{Event, ProgressUnit};
use crate::launcher::Core;
use crate::meta::rules::{Arch, Os, Platform};
use crate::net::{Http, check_status};
use crate::paths::{safe_join, single_segment};
use crate::{Error, Result};

/// API de Adoptium (Eclipse Temurin).
const ADOPTIUM_API: &str = "https://api.adoptium.net";
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
/// Profundidad máxima al buscar `bin/java` dentro de una Java gestionada
/// (en macOS vive en `Contents/Home/bin/java`).
const BIN_SEARCH_DEPTH: u32 = 3;
const DOWNLOAD_ATTEMPTS: u32 = 3;
/// Mínimo de bytes entre dos eventos de progreso de la descarga.
const PROGRESS_STEP: u64 = 128 * 1024;
/// A partir de esta versión se acepta una Java mayor que la pedida
/// (Minecraft moderno arranca bien con una Java más nueva).
const MIN_MAJOR_FOR_NEWER: u32 = 17;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JavaSource {
    /// Elegida a mano en los ajustes.
    Override,
    /// Instalada por el launcher en `<datos>/java/`.
    Managed,
    /// Encontrada en el sistema (`JAVA_HOME`, `PATH`, carpetas habituales).
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JavaInstall {
    /// Ejecutable `bin/java` (`java.exe` en Windows).
    pub path: PathBuf,
    pub major: u32,
    /// Valor de `java.version` (`21.0.5`, `1.8.0_412`).
    pub version: String,
    pub vendor: Option<String>,
    pub source: JavaSource,
}

/// Versión mayor de una cadena `java.version`: `1.8.0_412` → 8, `17.0.2` → 17,
/// `21-ea` → 21.
pub fn parse_major(version: &str) -> Option<u32> {
    let v = version.trim();
    let (first, rest) = leading_number(v)?;
    if first == 1
        && let Some(after) = rest.strip_prefix('.')
    {
        // Esquema antiguo: 1.8.0_412 es Java 8.
        return leading_number(after).map(|(n, _)| n);
    }
    Some(first)
}

fn leading_number(s: &str) -> Option<(u32, &str)> {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let n = s[..end].parse().ok()?;
    Some((n, &s[end..]))
}

/// Ejecuta `java -XshowSettings:properties -version` y lee la versión.
/// La `source` devuelta es `System`; quien llama la ajusta.
pub async fn probe(java: &Path) -> Result<JavaInstall> {
    let mut cmd = Command::new(java);
    cmd.arg("-XshowSettings:properties")
        .arg("-version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = tokio::time::timeout(PROBE_TIMEOUT, cmd.output())
        .await
        .map_err(|_| {
            Error::InvalidInput(format!(
                "{} no respondió en {} s",
                java.display(),
                PROBE_TIMEOUT.as_secs()
            ))
        })?
        .map_err(|e| Error::io(java, e))?;

    // Las propiedades y la versión salen por stderr; se mira también stdout.
    let mut text = String::from_utf8_lossy(&output.stderr).into_owned();
    text.push('\n');
    text.push_str(&String::from_utf8_lossy(&output.stdout));

    let (version, vendor) = parse_probe_output(&text)
        .ok_or_else(|| Error::InvalidInput(format!("{} no es una Java válida", java.display())))?;
    let major = parse_major(&version).ok_or_else(|| {
        Error::InvalidInput(format!(
            "versión de Java no reconocida en {}: {version:?}",
            java.display()
        ))
    })?;
    Ok(JavaInstall {
        path: java.to_path_buf(),
        major,
        version,
        vendor,
        source: JavaSource::System,
    })
}

/// Extrae `(java.version, java.vendor)` de la salida de `-XshowSettings:properties`;
/// si no hay propiedades, la versión sale de la línea `version "x"`.
fn parse_probe_output(text: &str) -> Option<(String, Option<String>)> {
    let mut version = None;
    let mut vendor = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "java.version" if version.is_none() && !value.is_empty() => {
                version = Some(value.to_owned());
            }
            "java.vendor" if vendor.is_none() && !value.is_empty() => {
                vendor = Some(value.to_owned());
            }
            _ => {}
        }
    }
    if version.is_none() {
        version = text.lines().find_map(|line| {
            let rest = &line[line.find("version \"")? + "version \"".len()..];
            let v = &rest[..rest.find('"')?];
            (!v.is_empty()).then(|| v.to_owned())
        });
    }
    version.map(|v| (v, vendor))
}

fn exe_name() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
}

/// `bin/java` de un directorio home de JDK, o `Contents/Home/bin/java` (macOS).
fn java_in_home(home: &Path) -> Option<PathBuf> {
    [
        home.join("bin").join(exe_name()),
        home.join("Contents")
            .join("Home")
            .join("bin")
            .join(exe_name()),
    ]
    .into_iter()
    .find(|p| p.is_file())
}

/// Busca `bin/java` bajando hasta `depth` niveles (sin seguir enlaces).
fn find_java_bin(dir: &Path, depth: u32) -> Option<PathBuf> {
    let bin = dir.join("bin").join(exe_name());
    if bin.is_file() {
        return Some(bin);
    }
    if depth == 0 {
        return None;
    }
    let mut subdirs: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
        .collect();
    subdirs.sort();
    subdirs
        .into_iter()
        .find_map(|sub| find_java_bin(&sub, depth - 1))
}

/// Dónde buscar Java en el sistema. Separado de `Core` para poder probarlo.
#[derive(Debug, Clone, Default)]
pub(crate) struct SystemSearch {
    java_home: Option<OsString>,
    path: Option<OsString>,
    /// Carpetas cuyos hijos son instalaciones de JDK.
    roots: Vec<PathBuf>,
}

impl SystemSearch {
    pub(crate) fn from_env() -> Self {
        Self {
            java_home: std::env::var_os("JAVA_HOME"),
            path: std::env::var_os("PATH"),
            roots: common_roots(),
        }
    }

    /// Ejecutables candidatos, por orden de prioridad y sin repetir rutas.
    fn candidates(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Some(home) = self.java_home.as_ref().filter(|h| !h.is_empty())
            && let Some(bin) = java_in_home(Path::new(home))
        {
            out.push(bin);
        }
        if let Some(path) = &self.path {
            for dir in std::env::split_paths(path) {
                let bin = dir.join(exe_name());
                // En macOS /usr/bin/java es un atajo que abre un diálogo de instalación.
                if cfg!(target_os = "macos") && bin == Path::new("/usr/bin/java") {
                    continue;
                }
                if bin.is_file() {
                    out.push(bin);
                }
            }
        }
        for root in &self.roots {
            let Ok(entries) = std::fs::read_dir(root) else {
                continue;
            };
            let mut homes: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            homes.sort();
            out.extend(homes.iter().filter_map(|h| java_in_home(h)));
        }
        out
    }
}

fn common_roots() -> Vec<PathBuf> {
    if cfg!(target_os = "windows") {
        let pf = std::env::var_os("ProgramFiles")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
        ["Java", "Eclipse Adoptium", "Microsoft", "Zulu"]
            .iter()
            .map(|d| pf.join(d))
            .collect()
    } else if cfg!(target_os = "macos") {
        let mut roots = vec![PathBuf::from("/Library/Java/JavaVirtualMachines")];
        if let Some(dirs) = directories::BaseDirs::new() {
            roots.push(dirs.home_dir().join("Library/Java/JavaVirtualMachines"));
        }
        roots
    } else {
        ["/usr/lib/jvm", "/usr/lib64/jvm", "/opt"]
            .iter()
            .map(PathBuf::from)
            .collect()
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    path: PathBuf,
    source: JavaSource,
}

/// Sondea los candidatos en paralelo, descartando duplicados (misma ruta
/// canónica; gana el primero) y los que no son una Java válida. Conserva el orden.
async fn probe_candidates(candidates: Vec<Candidate>) -> Vec<JavaInstall> {
    let mut seen = HashSet::new();
    let mut set = JoinSet::new();
    for (i, cand) in candidates.into_iter().enumerate() {
        let key = std::fs::canonicalize(&cand.path).unwrap_or_else(|_| cand.path.clone());
        if !seen.insert(key) {
            continue;
        }
        set.spawn(async move {
            let found = match probe(&cand.path).await {
                Ok(mut install) => {
                    install.source = cand.source;
                    Some(install)
                }
                Err(e) => {
                    tracing::debug!(java = %cand.path.display(), error = %e, "candidato descartado");
                    None
                }
            };
            (i, found)
        });
    }
    let mut found = Vec::new();
    while let Some(joined) = set.join_next().await {
        if let Ok((i, Some(install))) = joined {
            found.push((i, install));
        }
    }
    found.sort_by_key(|(i, _)| *i);
    found.into_iter().map(|(_, install)| install).collect()
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))?
}

/// `bin/java` de cada Java gestionada, ordenadas por versión mayor.
fn managed_bins(java_dir: &Path) -> Vec<(u32, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(java_dir) else {
        return Vec::new();
    };
    let mut out: Vec<(u32, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let major = e.file_name().to_str()?.parse::<u32>().ok()?;
            let bin = find_java_bin(&e.path(), BIN_SEARCH_DEPTH)?;
            Some((major, bin))
        })
        .collect();
    out.sort();
    out
}

impl Core {
    /// Java gestionada, elegida a mano y del sistema, sin duplicados.
    pub async fn list_java(&self) -> Result<Vec<JavaInstall>> {
        self.list_java_with(&SystemSearch::from_env()).await
    }

    pub(crate) async fn list_java_with(&self, search: &SystemSearch) -> Result<Vec<JavaInstall>> {
        let overrides = self.settings().await.java_overrides;
        let java_dir = self.paths().java_dir();
        let search = search.clone();
        let (managed, system) =
            blocking(move || Ok((managed_bins(&java_dir), search.candidates()))).await?;

        let mut candidates: Vec<Candidate> = overrides
            .into_values()
            .map(|path| Candidate {
                path,
                source: JavaSource::Override,
            })
            .collect();
        candidates.extend(managed.into_iter().map(|(_, path)| Candidate {
            path,
            source: JavaSource::Managed,
        }));
        candidates.extend(system.into_iter().map(|path| Candidate {
            path,
            source: JavaSource::System,
        }));
        Ok(probe_candidates(candidates).await)
    }

    /// Busca una Java para la versión mayor `major`: la elegida a mano, la
    /// gestionada, una del sistema con esa versión exacta y, desde Java 17, la
    /// más baja que sea mayor. Nunca una menor.
    pub async fn find_java(&self, major: u32) -> Result<Option<JavaInstall>> {
        self.find_java_with(major, &SystemSearch::from_env()).await
    }

    pub(crate) async fn find_java_with(
        &self,
        major: u32,
        search: &SystemSearch,
    ) -> Result<Option<JavaInstall>> {
        // 1. Ajuste manual, solo si es exactamente esa versión.
        if let Some(path) = self.settings().await.java_overrides.get(&major) {
            match probe(path).await {
                Ok(mut install) if install.major == major => {
                    install.source = JavaSource::Override;
                    return Ok(Some(install));
                }
                Ok(install) => tracing::warn!(
                    java = %path.display(),
                    found = install.major,
                    "se ignora la Java elegida: esta versión necesita Java {major}"
                ),
                Err(e) => {
                    tracing::warn!(java = %path.display(), error = %e, "se ignora la Java elegida")
                }
            }
        }

        // 2. Java gestionada.
        let java_dir = self.paths().java_dir();
        let managed = {
            let dir = java_dir.clone();
            blocking(move || Ok(managed_bins(&dir))).await?
        };
        if let Some((_, bin)) = managed.iter().find(|(m, _)| *m == major)
            && let Ok(mut install) = probe(bin).await
            && install.major == major
        {
            install.source = JavaSource::Managed;
            return Ok(Some(install));
        }

        // 3. Sistema con versión exacta.
        let search = search.clone();
        let system = blocking(move || Ok(search.candidates())).await?;
        let system = probe_candidates(
            system
                .into_iter()
                .map(|path| Candidate {
                    path,
                    source: JavaSource::System,
                })
                .collect(),
        )
        .await;
        if let Some(install) = system.iter().find(|j| j.major == major) {
            return Ok(Some(install.clone()));
        }

        // 4. Una más nueva (la más baja), solo para versiones modernas.
        if major >= MIN_MAJOR_FOR_NEWER {
            let managed = probe_candidates(
                managed
                    .into_iter()
                    .filter(|(m, _)| *m > major)
                    .map(|(_, path)| Candidate {
                        path,
                        source: JavaSource::Managed,
                    })
                    .collect(),
            )
            .await;
            // La gestionada va primero: gana los empates.
            return Ok(managed
                .into_iter()
                .chain(system)
                .filter(|j| j.major > major)
                .min_by_key(|j| j.major));
        }
        Ok(None)
    }

    /// Instala un JRE de Adoptium en `<datos>/java/<major>/`.
    pub async fn install_java(&self, major: u32) -> Result<JavaInstall> {
        self.install_java_from(ADOPTIUM_API, major).await
    }

    /// La Java adecuada si existe; si no, la instala.
    pub async fn ensure_java(&self, major: u32) -> Result<JavaInstall> {
        match self.find_java(major).await? {
            Some(install) => Ok(install),
            None => self.install_java(major).await,
        }
    }

    pub(crate) async fn install_java_from(
        &self,
        api_base: &str,
        major: u32,
    ) -> Result<JavaInstall> {
        let api_base = api_base.trim_end_matches('/');
        let http = self.http();
        if http.is_offline() {
            return Err(Error::Offline {
                url: format!("{api_base}/v3/assets/latest/{major}/hotspot"),
            });
        }
        if major == 0 {
            return Err(Error::InvalidInput(
                "la versión de Java no puede ser 0".into(),
            ));
        }
        let (os, arch) = adoptium_target(&Platform::current());
        let package = fetch_package(http, api_base, major, os, arch).await?;
        let name = single_segment(&package.name)?.to_owned();

        let java_dir = self.paths().java_dir();
        let tmp = java_dir.join(".tmp");
        tokio::fs::create_dir_all(&tmp)
            .await
            .map_err(|e| Error::io(&tmp, e))?;
        let archive = tmp.join(&name);
        let extract = tmp.join(format!("extract-{:08x}", fastrand::u32(..)));
        let task_id = NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed);
        let events = self.inner.events.clone();
        let progress = move |done: u64, total: u64| {
            events.emit(Event::Progress {
                task_id,
                stage: "java".into(),
                done,
                total,
                unit: ProgressUnit::Bytes,
            });
        };

        let result = async {
            download_verified(http, &package, &archive, progress).await?;
            tokio::fs::create_dir_all(&extract)
                .await
                .map_err(|e| Error::io(&extract, e))?;
            {
                let (archive, extract) = (archive.clone(), extract.clone());
                blocking(move || extract_archive(&archive, &extract)).await?;
            }
            let root = single_top_dir(&extract).await?;
            let target = java_dir.join(major.to_string());
            replace_dir(&root, &target, &tmp).await?;

            let bin = {
                let target = target.clone();
                blocking(move || {
                    let bin = find_java_bin(&target, BIN_SEARCH_DEPTH).ok_or_else(|| {
                        Error::InvalidInput(format!(
                            "el archivo de Java no contiene bin/{}",
                            exe_name()
                        ))
                    })?;
                    set_executable(&bin)?;
                    Ok(bin)
                })
                .await?
            };
            let mut install = probe(&bin).await?;
            if install.major != major {
                return Err(Error::InvalidInput(format!(
                    "se pidió Java {major} pero {} es Java {}",
                    bin.display(),
                    install.major
                )));
            }
            install.source = JavaSource::Managed;
            Ok(install)
        }
        .await;

        let _ = tokio::fs::remove_file(&archive).await;
        let _ = tokio::fs::remove_dir_all(&extract).await;
        self.inner.events.emit(Event::TaskFinished {
            task_id,
            ok: result.is_ok(),
            message: result.as_ref().err().map(ToString::to_string),
        });
        result
    }
}

// ---------------------------------------------------------------------------
// Adoptium
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct Asset {
    binary: Binary,
}

#[derive(Debug, Deserialize)]
struct Binary {
    package: Package,
}

#[derive(Debug, Clone, Deserialize)]
struct Package {
    link: String,
    name: String,
    /// SHA-256 en hexadecimal.
    checksum: String,
    #[serde(default)]
    size: Option<u64>,
}

/// `(os, architecture)` en el vocabulario de Adoptium.
fn adoptium_target(platform: &Platform) -> (&'static str, &'static str) {
    let os = match platform.os {
        Os::Linux => "linux",
        Os::Osx => "mac",
        Os::Windows => "windows",
    };
    let arch = match platform.arch {
        Arch::X86_64 => "x64",
        Arch::Aarch64 => "aarch64",
        Arch::X86 => "x32",
    };
    (os, arch)
}

/// Último paquete de Temurin para `major`: JRE y, si no existe, JDK.
async fn fetch_package(
    http: &Http,
    api_base: &str,
    major: u32,
    os: &str,
    arch: &str,
) -> Result<Package> {
    let base = format!("{api_base}/v3/assets/latest/{major}/hotspot");
    let mut last = String::from("sin resultados");
    for image_type in ["jre", "jdk"] {
        let url = url::Url::parse_with_params(
            &base,
            [
                ("architecture", arch),
                ("image_type", image_type),
                ("os", os),
                ("vendor", "eclipse"),
            ],
        )
        .map_err(|e| Error::InvalidInput(format!("URL de Adoptium no válida: {e}")))?;
        let resp = http.send(|c| c.get(url.clone())).await?;
        let status = resp.status();
        if matches!(status.as_u16(), 400 | 404) {
            last = format!("HTTP {}", status.as_u16());
            continue;
        }
        let resp = check_status(resp)?;
        let text = resp.text().await?;
        let assets: Vec<Asset> =
            serde_json::from_str(&text).map_err(|e| Error::json(url.as_str(), e))?;
        if let Some(asset) = assets.into_iter().next() {
            return Ok(asset.binary.package);
        }
        last = "sin resultados".into();
    }
    Err(Error::InvalidInput(format!(
        "Java {major} no está disponible para esta plataforma ({last})"
    )))
}

/// Descarga el paquete comprobando SHA-256 y tamaño. Reintenta errores de red,
/// 5xx y hashes que no coinciden.
async fn download_verified(
    http: &Http,
    package: &Package,
    dest: &Path,
    progress: impl Fn(u64, u64),
) -> Result<()> {
    let mut last_err = None;
    for attempt in 1..=DOWNLOAD_ATTEMPTS {
        match try_download(http, package, dest, &progress).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                let retry = matches!(e, Error::Network(_) | Error::HashMismatch { .. })
                    || matches!(e, Error::HttpStatus { status, .. } if status >= 500);
                tracing::debug!(url = %package.link, attempt, error = %e, "descarga de Java fallida");
                last_err = Some(e);
                if !retry {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(300 * u64::from(attempt))).await;
            }
        }
    }
    Err(last_err.expect("al menos un intento"))
}

async fn try_download(
    http: &Http,
    package: &Package,
    dest: &Path,
    progress: &impl Fn(u64, u64),
) -> Result<()> {
    let mut part_name = dest.file_name().unwrap_or_default().to_os_string();
    part_name.push(format!(".{:08x}.part", fastrand::u32(..)));
    let part = dest.with_file_name(part_name);

    let resp = http.send(|c| c.get(&package.link)).await?;
    let mut resp = check_status(resp)?;
    let total = package.size.or(resp.content_length()).unwrap_or(0);

    let mut file = tokio::fs::File::create(&part)
        .await
        .map_err(|e| Error::io(&part, e))?;
    let mut hasher = Sha256::new();
    let mut written = 0u64;
    let mut reported = 0u64;
    let result = async {
        while let Some(chunk) = resp.chunk().await? {
            hasher.update(&chunk);
            written += chunk.len() as u64;
            file.write_all(&chunk)
                .await
                .map_err(|e| Error::io(&part, e))?;
            if written - reported >= PROGRESS_STEP {
                reported = written;
                progress(written, total.max(written));
            }
        }
        file.flush().await.map_err(|e| Error::io(&part, e))?;
        drop(file);
        progress(written, total.max(written));
        let actual = hex::encode(hasher.finalize());
        if !actual.eq_ignore_ascii_case(package.checksum.trim()) {
            return Err(Error::HashMismatch {
                url: package.link.clone(),
                expected: package.checksum.clone(),
                actual,
            });
        }
        if let Some(size) = package.size
            && written != size
        {
            return Err(Error::HashMismatch {
                url: package.link.clone(),
                expected: format!("{size} bytes"),
                actual: format!("{written} bytes"),
            });
        }
        tokio::fs::rename(&part, dest)
            .await
            .map_err(|e| Error::io(dest, e))
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&part).await;
    }
    result
}

// ---------------------------------------------------------------------------
// Extracción
// ---------------------------------------------------------------------------

/// Descomprime un `.zip` o `.tar.gz` en `dest` (que debe existir).
fn extract_archive(archive: &Path, dest: &Path) -> Result<()> {
    let name = archive
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let root = std::fs::canonicalize(dest).map_err(|e| Error::io(dest, e))?;
    if name.ends_with(".zip") {
        extract_zip(archive, &root)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        extract_tar_gz(archive, &root)
    } else {
        Err(Error::InvalidInput(format!(
            "formato de archivo no soportado: {name}"
        )))
    }
}

/// Destino de una entrada del archivo dentro de `root`, creando sus carpetas
/// padre. `None` si la entrada es la propia raíz (`./`). Rechaza `..`, rutas
/// absolutas y cualquier padre que, por enlaces, acabe fuera de `root`.
fn entry_target(root: &Path, rel: &str) -> Result<Option<PathBuf>> {
    if Path::new(rel)
        .components()
        .all(|c| matches!(c, Component::CurDir))
    {
        return Ok(None);
    }
    let out = safe_join(root, rel)?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        ensure_inside(root, parent)?;
    }
    Ok(Some(out))
}

fn ensure_inside(root: &Path, path: &Path) -> Result<()> {
    let real = std::fs::canonicalize(path).map_err(|e| Error::io(path, e))?;
    if real.starts_with(root) {
        Ok(())
    } else {
        Err(Error::InvalidPath(format!(
            "la entrada sale del directorio de destino: {}",
            path.display()
        )))
    }
}

fn make_dir(root: &Path, out: &Path) -> Result<()> {
    std::fs::create_dir_all(out).map_err(|e| Error::io(out, e))?;
    ensure_inside(root, out)
}

/// Crea un enlace simbólico cuyo destino queda dentro de `root`. Solo se
/// admiten destinos relativos con `..` al principio (`../../lib/x`), que es lo
/// que usan los JDK; así el destino se calcula sin seguir otros enlaces.
#[cfg(unix)]
fn make_symlink(root: &Path, link: &Path, target: &str) -> Result<()> {
    let unsafe_link = || {
        Error::InvalidPath(format!(
            "enlace simbólico inseguro: {} -> {target:?}",
            link.display()
        ))
    };
    if target.is_empty() || target.contains('\0') {
        return Err(unsafe_link());
    }
    let parent = link.parent().ok_or_else(unsafe_link)?;
    let mut resolved = std::fs::canonicalize(parent).map_err(|e| Error::io(parent, e))?;
    let mut leading = true;
    for comp in Path::new(target).components() {
        match comp {
            Component::ParentDir if leading => {
                if !resolved.pop() {
                    return Err(unsafe_link());
                }
            }
            Component::Normal(c) => {
                leading = false;
                resolved.push(c);
            }
            Component::CurDir => {}
            _ => return Err(unsafe_link()),
        }
    }
    if !resolved.starts_with(root) {
        return Err(unsafe_link());
    }
    match std::fs::remove_file(link) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(Error::io(link, e)),
    }
    std::os::unix::fs::symlink(target, link).map_err(|e| Error::io(link, e))
}

#[cfg(not(unix))]
fn make_symlink(_root: &Path, link: &Path, _target: &str) -> Result<()> {
    tracing::debug!(link = %link.display(), "se omite un enlace simbólico");
    Ok(())
}

fn extract_tar_gz(archive: &Path, root: &Path) -> Result<()> {
    let file = File::open(archive).map_err(|e| Error::io(archive, e))?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(BufReader::new(file)));
    tar.set_preserve_permissions(true);
    tar.set_unpack_xattrs(false);
    let bad = |e: std::io::Error| Error::io(archive, e);

    for entry in tar.entries().map_err(bad)? {
        let mut entry = entry.map_err(bad)?;
        let rel = entry
            .path()
            .map_err(bad)?
            .to_str()
            .ok_or_else(|| Error::InvalidPath("nombre de entrada no es UTF-8".into()))?
            .to_owned();
        let Some(out) = entry_target(root, &rel)? else {
            continue;
        };
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            make_dir(root, &out)?;
        } else if kind.is_symlink() {
            let target = entry
                .link_name()
                .map_err(bad)?
                .and_then(|t| t.to_str().map(str::to_owned))
                .ok_or_else(|| Error::InvalidPath(format!("enlace sin destino: {rel:?}")))?;
            make_symlink(root, &out, &target)?;
        } else if kind.is_hard_link() {
            let target = entry
                .link_name()
                .map_err(bad)?
                .and_then(|t| t.to_str().map(str::to_owned))
                .ok_or_else(|| Error::InvalidPath(format!("enlace sin destino: {rel:?}")))?;
            let src = safe_join(root, &target)?;
            let _ = std::fs::remove_file(&out);
            std::fs::hard_link(&src, &out).map_err(|e| Error::io(&out, e))?;
        } else if kind.is_file() {
            entry.unpack(&out).map_err(|e| Error::io(&out, e))?;
        }
        // Otros tipos (dispositivos, FIFO, cabeceras globales) no se extraen.
    }
    Ok(())
}

fn extract_zip(archive: &Path, root: &Path) -> Result<()> {
    let file = File::open(archive).map_err(|e| Error::io(archive, e))?;
    let zip_err = |e: zip::result::ZipError| {
        Error::InvalidInput(format!("zip no válido ({}): {e}", archive.display()))
    };
    let mut zip = zip::ZipArchive::new(BufReader::new(file)).map_err(zip_err)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(zip_err)?;
        let rel = entry.name().map_err(zip_err)?.into_owned();
        let Some(out) = entry_target(root, &rel)? else {
            continue;
        };
        if entry.is_dir() {
            make_dir(root, &out)?;
        } else if entry.is_symlink() {
            let mut target = String::new();
            std::io::Read::read_to_string(&mut entry, &mut target)
                .map_err(|e| Error::io(archive, e))?;
            make_symlink(root, &out, &target)?;
        } else {
            let mut dest = File::create(&out).map_err(|e| Error::io(&out, e))?;
            std::io::copy(&mut entry, &mut dest).map_err(|e| Error::io(&out, e))?;
            dest.flush().map_err(|e| Error::io(&out, e))?;
            drop(dest);
            #[cfg(unix)]
            if let Some(mode) = entry.unix_mode() {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&out, std::fs::Permissions::from_mode(mode & 0o777))
                    .map_err(|e| Error::io(&out, e))?;
            }
        }
    }
    Ok(())
}

/// El único directorio de primer nivel del archivo extraído; si hay varias
/// entradas, el propio directorio de extracción.
async fn single_top_dir(extract: &Path) -> Result<PathBuf> {
    let mut entries = tokio::fs::read_dir(extract)
        .await
        .map_err(|e| Error::io(extract, e))?;
    let mut found = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| Error::io(extract, e))?
    {
        found.push(entry);
    }
    if let [only] = found.as_slice()
        && only.file_type().await.is_ok_and(|t| t.is_dir())
    {
        return Ok(only.path());
    }
    Ok(extract.to_path_buf())
}

/// Pone `new` en `target`: aparta lo anterior, mueve lo nuevo y borra lo viejo.
async fn replace_dir(new: &Path, target: &Path, tmp: &Path) -> Result<()> {
    let old = if tokio::fs::symlink_metadata(target).await.is_ok() {
        let old = tmp.join(format!("old-{:08x}", fastrand::u32(..)));
        tokio::fs::rename(target, &old)
            .await
            .map_err(|e| Error::io(target, e))?;
        Some(old)
    } else {
        None
    };
    if let Err(e) = tokio::fs::rename(new, target).await {
        if let Some(old) = &old {
            let _ = tokio::fs::rename(old, target).await;
        }
        return Err(Error::io(target, e));
    }
    if let Some(old) = old {
        let _ = tokio::fs::remove_dir_all(&old).await;
    }
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(path).map_err(|e| Error::io(path, e))?;
    let mut perms = meta.permissions();
    perms.set_mode(perms.mode() | 0o111);
    std::fs::set_permissions(path, perms).map_err(|e| Error::io(path, e))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher::Options;
    use crate::net::RetryPolicy;
    use crate::secrets::MemoryStore;
    use std::sync::Arc;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn core(dir: &Path) -> Core {
        Core::open(Options {
            data_dir: Some(dir.to_path_buf()),
            secrets: Some(Arc::new(MemoryStore::default())),
            retry: Some(RetryPolicy {
                max_attempts: 2,
                base_delay: Duration::from_millis(1),
                max_delay: Duration::from_millis(5),
            }),
            ..Options::default()
        })
        .await
        .unwrap()
    }

    // --- parse_major / salida del sondeo ---

    #[test]
    fn parse_major_cadenas_reales() {
        let casos = [
            ("1.8.0_412", Some(8)),
            ("1.8.0_292-b10", Some(8)),
            ("1.8.0-internal", Some(8)),
            ("1.7.0_80", Some(7)),
            ("1.8", Some(8)),
            ("9", Some(9)),
            ("9.0.4", Some(9)),
            ("10.0.2", Some(10)),
            ("11.0.22", Some(11)),
            ("11.0.22+7", Some(11)),
            ("16.0.1", Some(16)),
            ("17.0.2", Some(17)),
            ("17.0.2+8-LTS", Some(17)),
            ("21", Some(21)),
            ("21-ea", Some(21)),
            ("21.0.12.1", Some(21)),
            ("25", Some(25)),
            ("25.0.1", Some(25)),
            ("26-ea", Some(26)),
            ("  17.0.1 \n", Some(17)),
            ("10", Some(10)),
            ("", None),
            ("abc", None),
            ("-1", None),
            ("1.", None),
            ("1.x", None),
            ("ea", None),
            ("99999999999", None),
        ];
        for (input, esperado) in casos {
            assert_eq!(parse_major(input), esperado, "{input:?}");
        }
    }

    #[test]
    fn salida_con_propiedades() {
        let text = "Property settings:\n    java.vendor = Eclipse Adoptium\n    java.version = 21.0.5\n    java.version.date = 2024-10-15\n    java.vm.vendor = Eclipse Adoptium\n\nopenjdk version \"21.0.5\" 2024-10-15 LTS\n";
        assert_eq!(
            parse_probe_output(text),
            Some(("21.0.5".into(), Some("Eclipse Adoptium".into())))
        );
    }

    #[test]
    fn salida_solo_con_version() {
        assert_eq!(
            parse_probe_output("java version \"1.8.0_412\"\nJava(TM) SE Runtime Environment\n"),
            Some(("1.8.0_412".into(), None))
        );
        assert_eq!(
            parse_probe_output("openjdk version \"17.0.2\" 2022-01-18\n"),
            Some(("17.0.2".into(), None))
        );
        assert_eq!(parse_probe_output("bash: java: command not found"), None);
        assert_eq!(parse_probe_output(""), None);
    }

    // --- sondeo con un `java` falso ---

    /// Escribe un script ejecutable que imita a `java`. Reintenta si el kernel
    /// responde ETXTBSY (otro hilo lanzó un proceso mientras se escribía).
    #[cfg(unix)]
    fn fake_java(home: &Path, version: &str, vendor: Option<&str>) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let bin = home.join("bin/java");
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        let vendor_line = vendor
            .map(|v| format!("echo '    java.vendor = {v}' >&2\n"))
            .unwrap_or_default();
        let script = format!(
            "#!/bin/sh\necho 'Property settings:' >&2\necho '    java.version = {version}' >&2\n{vendor_line}echo 'openjdk version \"{version}\"' >&2\n"
        );
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin
    }

    #[cfg(unix)]
    async fn probe_retry(bin: &Path) -> Result<JavaInstall> {
        for _ in 0..20 {
            match probe(bin).await {
                Err(Error::Io { source, .. }) if source.raw_os_error() == Some(26) => {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                other => return other,
            }
        }
        probe(bin).await
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn probe_lee_la_version_de_un_java_falso() {
        let dir = tempfile::tempdir().unwrap();
        let bin = fake_java(dir.path(), "21.0.5", Some("Eclipse Adoptium"));
        let install = probe_retry(&bin).await.unwrap();
        assert_eq!(install.major, 21);
        assert_eq!(install.version, "21.0.5");
        assert_eq!(install.vendor.as_deref(), Some("Eclipse Adoptium"));
        assert_eq!(install.path, bin);

        let bin8 = fake_java(&dir.path().join("j8"), "1.8.0_412", None);
        let install = probe_retry(&bin8).await.unwrap();
        assert_eq!(install.major, 8);
        assert_eq!(install.vendor, None);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn probe_rechaza_lo_que_no_es_java() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("java");
        std::fs::write(&fake, "#!/bin/sh\necho hola\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let err = loop {
            match probe(&fake).await {
                Err(Error::Io { source, .. }) if source.raw_os_error() == Some(26) => {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                other => break other.unwrap_err(),
            }
        };
        assert!(matches!(err, Error::InvalidInput(_)), "{err:?}");
        let err = probe(&dir.path().join("no-existe")).await.unwrap_err();
        assert!(matches!(err, Error::Io { .. }), "{err:?}");
    }

    // --- búsqueda ---

    #[cfg(unix)]
    struct Fixture {
        _dir: tempfile::TempDir,
        core: Core,
        data: PathBuf,
        root: PathBuf,
    }

    #[cfg(unix)]
    impl Fixture {
        async fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let data = dir.path().join("data");
            let root = dir.path().join("jvm");
            std::fs::create_dir_all(&root).unwrap();
            Self {
                core: core(&data).await,
                data,
                root,
                _dir: dir,
            }
        }

        /// Solo mira `jvm/` (nada de `PATH` ni `JAVA_HOME` reales).
        fn search(&self) -> SystemSearch {
            SystemSearch {
                java_home: None,
                path: None,
                roots: vec![self.root.clone()],
            }
        }

        fn system(&self, name: &str, version: &str) -> PathBuf {
            fake_java(&self.root.join(name), version, None)
        }

        fn managed(&self, major: u32, sub: &str, version: &str) -> PathBuf {
            fake_java(
                &self.data.join("java").join(major.to_string()).join(sub),
                version,
                None,
            )
        }

        async fn find(&self, major: u32) -> Option<JavaInstall> {
            for _ in 0..20 {
                match self.core.find_java_with(major, &self.search()).await {
                    Ok(found) => return found,
                    Err(_) => tokio::time::sleep(Duration::from_millis(25)).await,
                }
            }
            panic!("find_java falló");
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn find_java_prefiere_la_exacta_y_nunca_una_menor() {
        let f = Fixture::new().await;
        f.system("jdk-8", "1.8.0_412");
        f.system("jdk-17", "17.0.2");
        f.system("jdk-21", "21.0.5");
        f.system("jdk-25", "25");

        assert_eq!(f.find(8).await.unwrap().major, 8);
        let j17 = f.find(17).await.unwrap();
        assert_eq!((j17.major, j17.source), (17, JavaSource::System));
        assert_eq!(f.find(21).await.unwrap().major, 21);
        // Sin la 21 exacta, la más baja que la supere.
        std::fs::remove_dir_all(f.root.join("jdk-21")).unwrap();
        assert_eq!(f.find(21).await.unwrap().major, 25);
        // Java 9..16 no aceptan una mayor.
        assert!(f.find(11).await.is_none());
        // Nunca una menor.
        assert!(f.find(26).await.is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn find_java_acepta_mayor_solo_desde_17() {
        let f = Fixture::new().await;
        f.system("jdk-21", "21.0.5");
        f.system("jdk-25", "25");
        assert_eq!(f.find(17).await.unwrap().major, 21);
        assert_eq!(f.find(18).await.unwrap().major, 21);
        assert!(f.find(8).await.is_none());
        assert!(f.find(16).await.is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn find_java_prefiere_la_gestionada_y_entiende_el_layout_de_macos() {
        let f = Fixture::new().await;
        f.system("jdk-21", "21.0.5");
        let bin = f.managed(21, "Contents/Home", "21.0.9");
        let found = f.find(21).await.unwrap();
        assert_eq!(found.source, JavaSource::Managed);
        assert_eq!(found.version, "21.0.9");
        assert_eq!(found.path, bin);

        // Con una gestionada más nueva y otra del sistema, gana la más baja.
        f.managed(25, "jdk-25-jre", "25.0.1");
        std::fs::remove_dir_all(f.data.join("java/21")).unwrap();
        std::fs::remove_dir_all(f.root.join("jdk-21")).unwrap();
        f.system("jdk-26", "26");
        let found = f.find(21).await.unwrap();
        assert_eq!((found.major, found.source), (25, JavaSource::Managed));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn find_java_override_solo_si_coincide_la_version() {
        let f = Fixture::new().await;
        let custom = fake_java(&f._dir.path().join("custom"), "17.0.9", Some("Zulu"));
        f.system("jdk-17", "17.0.2");
        f.core
            .update_settings(|s| {
                s.java_overrides.insert(17, custom.clone());
                s.java_overrides.insert(21, custom.clone());
            })
            .await
            .unwrap();

        let found = f.find(17).await.unwrap();
        assert_eq!(found.source, JavaSource::Override);
        assert_eq!(found.path, custom);
        // La override de 21 apunta a una 17: se ignora y no hay 21 válida.
        assert!(
            f.find(21)
                .await
                .is_none_or(|j| j.source != JavaSource::Override)
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn list_java_une_fuentes_y_quita_duplicados() {
        let f = Fixture::new().await;
        let real = f.system("jdk-21", "21.0.5");
        std::os::unix::fs::symlink(f.root.join("jdk-21"), f.root.join("java-21-openjdk")).unwrap();
        f.system("jdk-17", "17.0.2");
        f.managed(8, "jre", "1.8.0_412");
        let mut search = f.search();
        // JAVA_HOME apunta a la misma Java que ya está en la carpeta.
        search.java_home = Some(f.root.join("jdk-21").into_os_string());
        // PATH con la misma Java más un directorio sin Java.
        search.path = std::env::join_paths([
            f.root.join("jdk-21/bin"),
            f.root.join("jdk-17/bin"),
            f.root.clone(),
        ])
        .ok();

        let mut list = Vec::new();
        for _ in 0..20 {
            match f.core.list_java_with(&search).await {
                Ok(l) if l.len() == 3 => {
                    list = l;
                    break;
                }
                _ => tokio::time::sleep(Duration::from_millis(25)).await,
            }
        }
        let majors: Vec<_> = list.iter().map(|j| (j.major, j.source)).collect();
        assert_eq!(
            majors,
            vec![
                (8, JavaSource::Managed),
                (21, JavaSource::System),
                (17, JavaSource::System)
            ]
        );
        assert_eq!(list[1].path, real);
    }

    // --- extracción ---

    type TarBuilder = tar::Builder<flate2::write::GzEncoder<Vec<u8>>>;
    type BuildTar = Box<dyn FnOnce(&mut TarBuilder)>;

    fn tar_gz(build: impl FnOnce(&mut tar::Builder<flate2::write::GzEncoder<Vec<u8>>>)) -> Vec<u8> {
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        build(&mut builder);
        builder.into_inner().unwrap().finish().unwrap()
    }

    /// Añade una entrada con el nombre en bruto (el `Builder` no deja poner `..`).
    fn raw_entry(
        b: &mut tar::Builder<flate2::write::GzEncoder<Vec<u8>>>,
        name: &str,
        kind: tar::EntryType,
        link: Option<&str>,
        mode: u32,
        data: &[u8],
    ) {
        let mut h = tar::Header::new_gnu();
        h.as_old_mut().name[..name.len()].copy_from_slice(name.as_bytes());
        h.set_entry_type(kind);
        h.set_mode(mode);
        h.set_size(data.len() as u64);
        if let Some(link) = link {
            h.set_link_name(link).unwrap();
        }
        h.set_cksum();
        b.append(&h, data).unwrap();
    }

    fn write_archive(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn tar_extrae_y_conserva_permisos() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = tar_gz(|b| {
            raw_entry(b, "./", tar::EntryType::Directory, None, 0o755, b"");
            raw_entry(b, "jdk/", tar::EntryType::Directory, None, 0o755, b"");
            raw_entry(
                b,
                "jdk/bin/java",
                tar::EntryType::Regular,
                None,
                0o755,
                b"#!/bin/sh\n",
            );
            raw_entry(
                b,
                "jdk/release",
                tar::EntryType::Regular,
                None,
                0o644,
                b"x=1\n",
            );
            raw_entry(
                b,
                "jdk/legal/a.md",
                tar::EntryType::Regular,
                None,
                0o644,
                b"a",
            );
            raw_entry(
                b,
                "jdk/legal/b.md",
                tar::EntryType::Symlink,
                Some("a.md"),
                0o777,
                b"",
            );
            raw_entry(
                b,
                "jdk/legal/c.md",
                tar::EntryType::Symlink,
                Some("../release"),
                0o777,
                b"",
            );
            raw_entry(
                b,
                "jdk/legal/d.md",
                tar::EntryType::Link,
                Some("jdk/legal/a.md"),
                0o644,
                b"",
            );
        });
        let archive = write_archive(dir.path(), "x.tar.gz", &bytes);
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        extract_archive(&archive, &out).unwrap();

        assert_eq!(std::fs::read(out.join("jdk/release")).unwrap(), b"x=1\n");
        assert_eq!(std::fs::read(out.join("jdk/legal/d.md")).unwrap(), b"a");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(out.join("jdk/bin/java"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111);
            let mode = std::fs::metadata(out.join("jdk/release"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0);
            assert_eq!(std::fs::read(out.join("jdk/legal/b.md")).unwrap(), b"a");
            assert_eq!(std::fs::read(out.join("jdk/legal/c.md")).unwrap(), b"x=1\n");
        }
    }

    #[test]
    fn tar_rechaza_entradas_peligrosas() {
        let malos: Vec<(&str, BuildTar)> = vec![
            (
                "..",
                Box::new(|b| raw_entry(b, "../evil", tar::EntryType::Regular, None, 0o644, b"x")),
            ),
            (
                "interno ..",
                Box::new(|b| {
                    raw_entry(
                        b,
                        "a/../../evil",
                        tar::EntryType::Regular,
                        None,
                        0o644,
                        b"x",
                    )
                }),
            ),
            (
                "absoluta",
                Box::new(|b| raw_entry(b, "/tmp/evil", tar::EntryType::Regular, None, 0o644, b"x")),
            ),
            (
                "dir ..",
                Box::new(|b| raw_entry(b, "../dir/", tar::EntryType::Directory, None, 0o755, b"")),
            ),
        ];
        #[cfg(unix)]
        let malos = {
            let mut malos = malos;
            malos.push((
                "enlace absoluto",
                Box::new(|b| {
                    raw_entry(b, "d/l", tar::EntryType::Symlink, Some("/etc"), 0o777, b"")
                }),
            ));
            malos.push((
                "enlace que escapa",
                Box::new(|b| {
                    raw_entry(
                        b,
                        "d/l",
                        tar::EntryType::Symlink,
                        Some("../../x"),
                        0o777,
                        b"",
                    )
                }),
            ));
            malos.push((
                "enlace con .. intermedio",
                Box::new(|b| {
                    raw_entry(
                        b,
                        "d/l",
                        tar::EntryType::Symlink,
                        Some("a/../../../x"),
                        0o777,
                        b"",
                    )
                }),
            ));
            malos.push((
                "escribir a través de un enlace",
                Box::new(|b| {
                    raw_entry(b, "d/up", tar::EntryType::Symlink, Some(".."), 0o777, b"");
                    raw_entry(
                        b,
                        "d/up/up2",
                        tar::EntryType::Symlink,
                        Some("../../out-evil"),
                        0o777,
                        b"",
                    );
                }),
            ));
            malos.push((
                "enlace duro fuera",
                Box::new(|b| {
                    raw_entry(b, "h", tar::EntryType::Link, Some("../secret"), 0o644, b"")
                }),
            ));
            malos
        };
        for (what, build) in malos {
            let dir = tempfile::tempdir().unwrap();
            let archive = write_archive(dir.path(), "m.tar.gz", &tar_gz(build));
            let out = dir.path().join("sub/out");
            std::fs::create_dir_all(&out).unwrap();
            let err = extract_archive(&archive, &out).unwrap_err();
            assert!(matches!(err, Error::InvalidPath(_)), "{what}: {err:?}");
            assert!(!dir.path().join("evil").exists(), "{what}");
            assert!(!dir.path().join("sub/evil").exists(), "{what}");
            assert!(!dir.path().join("out-evil").exists(), "{what}");
        }
    }

    fn zip_bytes(entries: &[(&str, &[u8], u32)]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, data, mode) in entries {
            let opts = zip::write::SimpleFileOptions::default().unix_permissions(*mode);
            if name.ends_with('/') {
                zip.add_directory(*name, opts).unwrap();
            } else {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(data).unwrap();
            }
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn zip_extrae_y_conserva_permisos() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = zip_bytes(&[
            ("jdk/", b"", 0o755),
            ("jdk/bin/java.exe", b"MZ", 0o755),
            ("jdk/release", b"x=1\n", 0o644),
        ]);
        let archive = write_archive(dir.path(), "x.zip", &bytes);
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        extract_archive(&archive, &out).unwrap();
        assert_eq!(std::fs::read(out.join("jdk/bin/java.exe")).unwrap(), b"MZ");
        assert_eq!(std::fs::read(out.join("jdk/release")).unwrap(), b"x=1\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(out.join("jdk/bin/java.exe"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111);
        }
    }

    #[test]
    fn zip_rechaza_entradas_peligrosas() {
        for (what, name) in [
            ("..", "../evil"),
            ("interno ..", "a/../../evil"),
            ("absoluta", "/tmp/evil"),
            ("barra invertida", "..\\evil"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let archive = write_archive(dir.path(), "m.zip", &zip_bytes(&[(name, b"x", 0o644)]));
            let out = dir.path().join("sub/out");
            std::fs::create_dir_all(&out).unwrap();
            let err = extract_archive(&archive, &out).unwrap_err();
            assert!(matches!(err, Error::InvalidPath(_)), "{what}: {err:?}");
            assert!(!dir.path().join("evil").exists(), "{what}");
            assert!(!dir.path().join("sub/evil").exists(), "{what}");
        }
    }

    #[test]
    fn formato_desconocido() {
        let dir = tempfile::tempdir().unwrap();
        let archive = write_archive(dir.path(), "x.rar", b"x");
        let err = extract_archive(&archive, dir.path()).unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)), "{err:?}");
    }

    // --- instalación contra un servidor simulado ---

    #[cfg(unix)]
    fn fake_jre_tar_gz(version: &str) -> Vec<u8> {
        let script = format!(
            "#!/bin/sh\necho '    java.version = {version}' >&2\necho '    java.vendor = Eclipse Adoptium' >&2\n"
        );
        tar_gz(|b| {
            raw_entry(
                b,
                "jdk-fake-jre/",
                tar::EntryType::Directory,
                None,
                0o755,
                b"",
            );
            // Sin el bit de ejecución en el archivo: lo pone `install_java`.
            raw_entry(
                b,
                "jdk-fake-jre/bin/java",
                tar::EntryType::Regular,
                None,
                0o644,
                script.as_bytes(),
            );
            raw_entry(
                b,
                "jdk-fake-jre/release",
                tar::EntryType::Regular,
                None,
                0o644,
                b"x=1\n",
            );
        })
    }

    fn asset_json(link: &str, name: &str, bytes: &[u8]) -> String {
        serde_json::json!([{
            "binary": { "package": {
                "link": link,
                "name": name,
                "checksum": hex::encode(Sha256::digest(bytes)),
                "size": bytes.len(),
            }},
            "release_name": "jdk-21.0.5+11",
        }])
        .to_string()
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn instala_desde_un_servidor_simulado() {
        let server = MockServer::start().await;
        let bytes = fake_jre_tar_gz("21.0.5");
        Mock::given(method("GET"))
            .and(path("/v3/assets/latest/21/hotspot"))
            .and(query_param("image_type", "jre"))
            .and(query_param("vendor", "eclipse"))
            .respond_with(ResponseTemplate::new(200).set_body_string(asset_json(
                &format!("{}/files/jre.tar.gz", server.uri()),
                "OpenJDK21U-jre_x64_linux_hotspot_21.0.5_11.tar.gz",
                &bytes,
            )))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/files/jre.tar.gz"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let core = core(dir.path()).await;
        let mut events = core.subscribe();
        // Una instalación previa se reemplaza.
        let old = dir.path().join("java/21");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("viejo.txt"), "x").unwrap();

        let mut install = None;
        for _ in 0..20 {
            match core.install_java_from(&server.uri(), 21).await {
                Err(Error::Io { source, .. }) if source.raw_os_error() == Some(26) => {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                other => {
                    install = Some(other.unwrap());
                    break;
                }
            }
        }
        let install = install.unwrap();
        assert_eq!(install.major, 21);
        assert_eq!(install.source, JavaSource::Managed);
        assert_eq!(install.vendor.as_deref(), Some("Eclipse Adoptium"));
        assert_eq!(install.path, dir.path().join("java/21/bin/java"));
        assert!(!old.join("viejo.txt").exists());
        assert!(dir.path().join("java/21/release").exists());
        // Sin restos en .tmp.
        let rest: Vec<_> = std::fs::read_dir(dir.path().join("java/.tmp"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(rest.is_empty(), "{rest:?}");

        let mut last = None;
        let mut finished = false;
        while let Ok(ev) = events.try_recv() {
            match ev {
                Event::Progress {
                    stage,
                    done,
                    total,
                    unit,
                    ..
                } => {
                    assert_eq!((stage.as_str(), unit), ("java", ProgressUnit::Bytes));
                    last = Some((done, total));
                }
                Event::TaskFinished { ok, .. } => finished = ok,
                _ => {}
            }
        }
        let size = fake_jre_tar_gz("21.0.5").len() as u64;
        assert_eq!(last, Some((size, size)));
        assert!(finished);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sin_jre_prueba_con_jdk() {
        let server = MockServer::start().await;
        let bytes = fake_jre_tar_gz("17.0.2");
        Mock::given(method("GET"))
            .and(path("/v3/assets/latest/17/hotspot"))
            .and(query_param("image_type", "jre"))
            .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v3/assets/latest/17/hotspot"))
            .and(query_param("image_type", "jdk"))
            .respond_with(ResponseTemplate::new(200).set_body_string(asset_json(
                &format!("{}/files/jdk.tar.gz", server.uri()),
                "jdk.tar.gz",
                &bytes,
            )))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/files/jdk.tar.gz"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let core = core(dir.path()).await;
        let mut result = None;
        for _ in 0..20 {
            match core.install_java_from(&server.uri(), 17).await {
                Err(Error::Io { source, .. }) if source.raw_os_error() == Some(26) => {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                other => {
                    result = Some(other);
                    break;
                }
            }
        }
        assert_eq!(result.unwrap().unwrap().major, 17);
    }

    #[tokio::test]
    async fn plataforma_sin_java_da_un_error_claro() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let err = core(dir.path())
            .await
            .install_java_from(&server.uri(), 99)
            .await
            .unwrap_err();
        match err {
            Error::InvalidInput(msg) => {
                assert!(msg.contains("Java 99") && msg.contains("404"), "{msg}")
            }
            other => panic!("{other:?}"),
        }
        assert!(!dir.path().join("java/99").exists());
    }

    #[tokio::test]
    async fn checksum_incorrecto_falla_sin_instalar() {
        let server = MockServer::start().await;
        let bytes = b"contenido".to_vec();
        let json = serde_json::json!([{
            "binary": { "package": {
                "link": format!("{}/files/x.tar.gz", server.uri()),
                "name": "x.tar.gz",
                "checksum": hex::encode(Sha256::digest(b"otra cosa")),
                "size": bytes.len(),
            }}
        }]);
        Mock::given(method("GET"))
            .and(path("/v3/assets/latest/21/hotspot"))
            .respond_with(ResponseTemplate::new(200).set_body_string(json.to_string()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/files/x.tar.gz"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let err = core(dir.path())
            .await
            .install_java_from(&server.uri(), 21)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::HashMismatch { .. }), "{err:?}");
        assert!(!dir.path().join("java/21").exists());
        let rest = std::fs::read_dir(dir.path().join("java/.tmp"))
            .unwrap()
            .count();
        assert_eq!(rest, 0);
    }

    #[tokio::test]
    async fn nombre_de_paquete_inseguro() {
        let server = MockServer::start().await;
        let json = asset_json(&format!("{}/f", server.uri()), "../../evil.tar.gz", b"x");
        Mock::given(method("GET"))
            .and(path("/v3/assets/latest/21/hotspot"))
            .respond_with(ResponseTemplate::new(200).set_body_string(json))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let err = core(dir.path())
            .await
            .install_java_from(&server.uri(), 21)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidPath(_)), "{err:?}");
    }

    #[tokio::test]
    async fn sin_conexion_no_instala() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::open(Options {
            data_dir: Some(dir.path().to_path_buf()),
            secrets: Some(Arc::new(MemoryStore::default())),
            offline: true,
            ..Options::default()
        })
        .await
        .unwrap();
        let err = core.install_java(21).await.unwrap_err();
        assert!(matches!(err, Error::Offline { .. }), "{err:?}");
    }

    #[test]
    fn destino_de_adoptium() {
        assert_eq!(
            adoptium_target(&Platform::new(Os::Linux, Arch::X86_64)),
            ("linux", "x64")
        );
        assert_eq!(
            adoptium_target(&Platform::new(Os::Osx, Arch::Aarch64)),
            ("mac", "aarch64")
        );
        assert_eq!(
            adoptium_target(&Platform::new(Os::Windows, Arch::X86)),
            ("windows", "x32")
        );
    }

    /// Instala de verdad Java 21 desde Adoptium (red). `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore = "descarga ~50 MB de Adoptium"]
    async fn instala_java_21_real() {
        let dir = tempfile::tempdir().unwrap();
        let core = core(dir.path()).await;
        let install = core.install_java(21).await.unwrap();
        println!("{install:#?}");
        assert_eq!(install.major, 21);
        assert_eq!(install.source, JavaSource::Managed);
        assert!(install.path.starts_with(dir.path().join("java/21")));
        // La encuentra después como gestionada.
        let found = core.find_java(21).await.unwrap().unwrap();
        assert_eq!(found.source, JavaSource::Managed);
        assert_eq!(found.version, install.version);
    }
}
