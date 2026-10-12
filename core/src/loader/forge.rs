//! Forge y NeoForge: se descarga el instalador oficial y se ejecuta con
//! `--installClient` sobre el directorio de datos (D6). Los instaladores muy
//! antiguos (con `versionInfo` en `install_profile.json`) no necesitan Java: el
//! perfil y el jar universal se colocan a mano.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::LoaderVersion;
use crate::download::{DownloadTask, Verify, download_one, is_valid};
use crate::fsutil::write_atomic;
use crate::instance::LoaderKind;
use crate::launcher::Core;
use crate::meta::library::Coordinate;
use crate::meta::rules::{Features, Platform};
use crate::net::check_status;
use crate::paths::{safe_join, single_segment};
use crate::{Error, Result};

const LIST_TTL: Duration = Duration::from_secs(30 * 60);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(10 * 60);

const FORGE_MAVEN: &str = "https://maven.minecraftforge.net";
const FORGE_PROMOTIONS: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases";

/// `launcher_profiles.json` mínimo que exigen los instaladores.
const PROFILES_STUB: &str = r#"{"profiles":{},"selectedProfile":null,"clientToken":"","authenticationDatabase":{},"launcherVersion":{"name":"lcore","format":21},"settings":{},"version":3}"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Forge,
    NeoForge,
}

impl From<LoaderKind> for Flavor {
    fn from(kind: LoaderKind) -> Self {
        match kind {
            LoaderKind::NeoForge => Flavor::NeoForge,
            _ => Flavor::Forge,
        }
    }
}

impl Flavor {
    fn name(self) -> &'static str {
        match self {
            Flavor::Forge => "Forge",
            Flavor::NeoForge => "NeoForge",
        }
    }
}

/// Dónde vive un loader para una versión de Minecraft concreta.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Source {
    flavor: Flavor,
    mc: String,
    maven: &'static str,
    /// Ruta del grupo con `/`.
    group: &'static str,
    artifact: &'static str,
    /// La versión del artefacto es `<mc>-<loader>` (Forge y NeoForge 1.20.1) o solo `<loader>`.
    mc_prefixed: bool,
}

impl Source {
    fn new(flavor: Flavor, mc: &str) -> Result<Self> {
        let mc = single_segment(mc)?.to_owned();
        Ok(match flavor {
            Flavor::Forge => Self {
                flavor,
                mc,
                maven: FORGE_MAVEN,
                group: "net/minecraftforge",
                artifact: "forge",
                mc_prefixed: true,
            },
            // NeoForge 1.20.1 todavía era un fork de Forge con su propio artefacto.
            Flavor::NeoForge if mc == "1.20.1" => Self {
                flavor,
                mc,
                maven: NEOFORGE_MAVEN,
                group: "net/neoforged",
                artifact: "forge",
                mc_prefixed: true,
            },
            Flavor::NeoForge => Self {
                flavor,
                mc,
                maven: NEOFORGE_MAVEN,
                group: "net/neoforged",
                artifact: "neoforge",
                mc_prefixed: false,
            },
        })
    }

    fn metadata_url(&self) -> String {
        format!(
            "{}/{}/{}/maven-metadata.xml",
            self.maven, self.group, self.artifact
        )
    }

    /// Versión del artefacto Maven para una versión del loader.
    fn full_version(&self, loader: &str) -> String {
        if self.mc_prefixed {
            format!("{}-{loader}", self.mc)
        } else {
            loader.to_owned()
        }
    }

    fn installer_name(&self, full: &str) -> String {
        format!("{}-{full}-installer.jar", self.artifact)
    }

    fn installer_url(&self, full: &str) -> String {
        format!(
            "{}/{}/{}/{full}/{}",
            self.maven,
            self.group,
            self.artifact,
            self.installer_name(full)
        )
    }
}

// ---------------------------------------------------------------------------
// Listado de versiones
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Promos {
    recommended: Option<String>,
    latest: Option<String>,
}

#[derive(Deserialize)]
struct PromotionsFile {
    #[serde(default)]
    promos: HashMap<String, String>,
}

fn promos_for(file: &PromotionsFile, mc: &str) -> Promos {
    Promos {
        recommended: file.promos.get(&format!("{mc}-recommended")).cloned(),
        latest: file.promos.get(&format!("{mc}-latest")).cloned(),
    }
}

/// Parte numérica de una versión de Forge/NeoForge, sin sufijos (`-1.7.10`, `-beta`).
fn core_of(version: &str) -> &str {
    version.split(['-', '+']).next().unwrap_or(version)
}

/// Todas las `<version>` de un `maven-metadata.xml` (sin duplicados, en el orden del archivo).
fn parse_maven_versions(xml: &str) -> Vec<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"<version>\s*([^<\s]+)\s*</version>").expect("regex"));
    let mut seen = std::collections::HashSet::new();
    re.captures_iter(xml)
        .map(|c| c[1].to_owned())
        .filter(|v| seen.insert(v.clone()))
        .collect()
}

/// Clave de orden (mayor = más nueva): números del núcleo y, a igualdad, sin sufijo gana.
fn sort_key(version: &str) -> (Vec<u64>, Reverse<usize>) {
    let core = core_of(version);
    let nums = core
        .split('.')
        .map(|p| p.parse::<u64>().unwrap_or(0))
        .collect();
    (nums, Reverse(version.len() - core.len()))
}

/// Prefijo de las versiones de NeoForge para una versión de Minecraft.
///
/// - `1.X.Y` → `X.Y.` (`1.21` → `21.0.`, `1.21.1` → `21.1.`).
/// - Versiones por año (`26.2`, `26.2.1`) → `26.2.0.` y `26.2.1.`: el esquema es
///   `<mc>.<compilación>`, con un `0` de parche si Minecraft no lo trae.
///
/// `None` para snapshots, pre-releases y cualquier cosa que no encaje.
fn neoforge_prefix(mc: &str) -> Option<String> {
    static LEGACY: OnceLock<Regex> = OnceLock::new();
    static YEAR: OnceLock<Regex> = OnceLock::new();
    let legacy = LEGACY.get_or_init(|| Regex::new(r"^1\.(\d+)(?:\.(\d+))?$").expect("regex"));
    let year = YEAR.get_or_init(|| Regex::new(r"^(\d+)\.(\d+)(?:\.(\d+))?$").expect("regex"));
    if let Some(c) = legacy.captures(mc) {
        let minor = c.get(2).map_or("0", |m| m.as_str());
        return Some(format!("{}.{minor}.", &c[1]));
    }
    let c = year.captures(mc)?;
    if &c[1] == "1" {
        return None;
    }
    let patch = c.get(3).map_or("0", |m| m.as_str());
    Some(format!("{}.{}.{patch}.", &c[1], &c[2]))
}

/// Versiones del loader para `src.mc` a partir de las del `maven-metadata.xml`,
/// la más nueva primero. Devuelve solo la parte propia del loader.
fn filter_versions(src: &Source, all: &[String]) -> Vec<String> {
    let mut out: Vec<String> = if src.mc_prefixed {
        let prefix = format!("{}-", src.mc);
        all.iter()
            .filter_map(|v| v.strip_prefix(&prefix))
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
            .collect()
    } else {
        let Some(prefix) = neoforge_prefix(&src.mc) else {
            return Vec::new();
        };
        all.iter()
            // `+snapshot-N`/`+pre-N`: compilaciones para snapshots de Minecraft.
            .filter(|v| v.starts_with(&prefix) && !v.contains('+'))
            .cloned()
            .collect()
    };
    out.sort_by_key(|v| Reverse(sort_key(v)));
    out
}

fn is_stable(flavor: Flavor, version: &str, promos: &Promos) -> bool {
    match flavor {
        Flavor::NeoForge => !version.contains(['-', '+']),
        Flavor::Forge => {
            let core = core_of(version);
            promos.recommended.as_deref() == Some(core) || promos.latest.as_deref() == Some(core)
        }
    }
}

/// Lista (más nueva primero) y promociones del loader. Error si no hay ninguna versión.
async fn load(core: &Core, src: &Source) -> Result<(Vec<String>, Promos)> {
    if src.flavor == Flavor::NeoForge && !src.mc_prefixed && neoforge_prefix(&src.mc).is_none() {
        return Err(Error::VersionNotFound(format!(
            "{} con NeoForge (solo hay compilaciones para versiones publicadas)",
            src.mc
        )));
    }
    let xml = fetch_text_cached(core, &src.metadata_url()).await?;
    let list = filter_versions(src, &parse_maven_versions(&xml));
    if list.is_empty() {
        return Err(Error::VersionNotFound(format!(
            "{} con {}",
            src.mc,
            src.flavor.name()
        )));
    }
    let promos = if src.flavor == Flavor::Forge {
        match core
            .http()
            .get_json_cached::<PromotionsFile>(FORGE_PROMOTIONS, LIST_TTL)
            .await
        {
            Ok(file) => promos_for(&file, &src.mc),
            Err(e) => {
                tracing::warn!(error = %e, "sin promociones de Forge; no se marca ninguna estable");
                Promos::default()
            }
        }
    } else {
        Promos::default()
    };
    Ok((list, promos))
}

pub async fn versions(core: &Core, flavor: Flavor, mc: &str) -> Result<Vec<LoaderVersion>> {
    let src = Source::new(flavor, mc)?;
    let (list, promos) = load(core, &src).await?;
    Ok(list
        .into_iter()
        .map(|version| LoaderVersion {
            stable: is_stable(flavor, &version, &promos),
            version,
        })
        .collect())
}

/// Forge: la recomendada, si no la última promocionada, si no la más nueva.
/// NeoForge: la más nueva que no sea beta.
pub async fn latest_stable(core: &Core, flavor: Flavor, mc: &str) -> Result<String> {
    let src = Source::new(flavor, mc)?;
    let (list, promos) = load(core, &src).await?;
    Ok(pick_default(flavor, &list, &promos))
}

fn pick_default(flavor: Flavor, list: &[String], promos: &Promos) -> String {
    let by_core = |core: &Option<String>| {
        core.as_deref()
            .and_then(|c| list.iter().find(|v| core_of(v) == c))
    };
    let chosen = match flavor {
        Flavor::Forge => by_core(&promos.recommended).or_else(|| by_core(&promos.latest)),
        Flavor::NeoForge => None,
    };
    chosen
        .or_else(|| list.iter().find(|v| is_stable(flavor, v, promos)))
        .or(list.first())
        .expect("la lista no está vacía")
        .clone()
}

/// GET de texto con caché en disco (30 min); sin red sirve la copia aunque esté vieja.
async fn fetch_text_cached(core: &Core, url: &str) -> Result<String> {
    let hash = hex::encode(Sha256::digest(url.as_bytes()));
    let path = core
        .paths()
        .http_cache_dir()
        .join(format!("{}.xml", &hash[..32]));
    let cached = tokio::fs::read_to_string(&path).await.ok();
    if let Some(body) = &cached {
        let fresh = tokio::fs::metadata(&path)
            .await
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age < LIST_TTL);
        if fresh || core.http().is_offline() {
            return Ok(body.clone());
        }
    } else if core.http().is_offline() {
        return Err(Error::Offline { url: url.into() });
    }

    let fetched: Result<String> = async {
        let resp = core.http().send(|c| c.get(url)).await?;
        Ok(check_status(resp)?.text().await?)
    }
    .await;
    match fetched {
        Ok(body) => {
            if let Err(e) = write_atomic(&path, body.as_bytes()).await {
                tracing::warn!(%url, error = %e, "no se pudo guardar en caché");
            }
            Ok(body)
        }
        Err(e) => match cached {
            Some(body) => {
                tracing::warn!(%url, error = %e, "uso la copia en caché");
                Ok(body)
            }
            None => Err(e),
        },
    }
}

// ---------------------------------------------------------------------------
// Instalación
// ---------------------------------------------------------------------------

/// Instala el loader con su instalador oficial y devuelve el id de versión creado
/// (`1.20.1-forge-47.4.10`, `neoforge-21.1.77`…). Es idempotente: si el perfil y
/// las librerías generadas ya están en disco, no vuelve a ejecutar nada.
pub async fn install(
    core: &Core,
    flavor: Flavor,
    mc: &str,
    loader_version: &str,
    java: &Path,
) -> Result<String> {
    let src = Source::new(flavor, mc)?;
    let loader_version = single_segment(loader_version)?;
    let full = src.full_version(loader_version);
    let installer = ensure_installer(core, &src, &full).await?;
    install_from_jar(core, &src, &full, &installer, java).await
}

/// Id de versión si el loader ya está instalado completo (sin ejecutar Java ni usar la red).
pub async fn installed_id(
    core: &Core,
    flavor: Flavor,
    mc: &str,
    loader_version: &str,
) -> Result<Option<String>> {
    let src = Source::new(flavor, mc)?;
    let full = src.full_version(single_segment(loader_version)?);
    let installer = core
        .paths()
        .installers_dir()
        .join(src.installer_name(&full));
    if !tokio::fs::try_exists(&installer).await.unwrap_or(false) {
        return Ok(None);
    }
    let jar = installer.clone();
    let info = tokio::task::spawn_blocking(move || inspect_installer(&jar))
        .await
        .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))??;
    let complete = matches!(missing_generated(core, &info.id).await?, Some(m) if m.is_empty());
    Ok(complete.then_some(info.id))
}

async fn install_from_jar(
    core: &Core,
    src: &Source,
    full: &str,
    installer: &Path,
    java: &Path,
) -> Result<String> {
    let jar = installer.to_path_buf();
    let info = tokio::task::spawn_blocking(move || inspect_installer(&jar))
        .await
        .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))??;

    // Instaladores antiguos: se colocan los archivos sin ejecutar Java.
    if let Some(legacy) = &info.legacy {
        install_legacy(core, installer, &info.id, legacy).await?;
        return Ok(info.id);
    }

    if matches!(missing_generated(core, &info.id).await?, Some(m) if m.is_empty()) {
        return Ok(info.id);
    }
    run_installer(core, src, full, installer, java).await?;

    match missing_generated(core, &info.id).await? {
        None => Err(Error::InvalidInput(format!(
            "el instalador de {} terminó pero no creó la versión {} (revisa {})",
            src.flavor.name(),
            info.id,
            log_path(core, src, full).display()
        ))),
        Some(missing) if !missing.is_empty() => Err(Error::InvalidInput(format!(
            "el instalador de {} no generó estas librerías: {} (revisa {})",
            src.flavor.name(),
            missing.join(", "),
            log_path(core, src, full).display()
        ))),
        Some(_) => Ok(info.id),
    }
}

fn log_path(core: &Core, src: &Source, full: &str) -> PathBuf {
    core.paths()
        .installers_dir()
        .join(format!("{}-{full}-install.log", src.artifact))
}

/// Descarga el instalador a `installers/` si no está ya.
async fn ensure_installer(core: &Core, src: &Source, full: &str) -> Result<PathBuf> {
    let dest = core.paths().installers_dir().join(src.installer_name(full));
    if tokio::fs::metadata(&dest).await.is_ok_and(|m| m.len() > 0) {
        return Ok(dest);
    }
    let url = src.installer_url(full);
    if core.http().is_offline() {
        return Err(Error::Offline { url });
    }
    // Maven publica el SHA-1 junto al archivo.
    let sha1 = {
        let sha_url = format!("{url}.sha1");
        let resp = core.http().send(|c| c.get(&sha_url)).await?;
        if resp.status().is_success() {
            let text = resp.text().await?;
            text.split_whitespace()
                .next()
                .filter(|h| h.len() == 40 && h.chars().all(|c| c.is_ascii_hexdigit()))
                .map(str::to_owned)
        } else {
            None
        }
    };
    let task = DownloadTask {
        url,
        dest: dest.clone(),
        sha1,
        size: None,
    };
    match download_one(core.http(), &task).await {
        Ok(()) => Ok(dest),
        Err(Error::HttpStatus { status: 404, .. }) => Err(Error::VersionNotFound(format!(
            "{} {full} (no tiene instalador)",
            src.flavor.name()
        ))),
        Err(e) => Err(e),
    }
}

/// Lo que se necesita saber de un instalador antes de ejecutarlo.
#[derive(Debug, Clone, PartialEq)]
struct InstallerInfo {
    /// Id de la versión que va a crear.
    id: String,
    legacy: Option<Legacy>,
}

/// Instalador antiguo: `install_profile.json` trae el perfil (`versionInfo`) y
/// apunta al jar universal dentro del propio instalador.
#[derive(Debug, Clone, PartialEq)]
struct Legacy {
    version_info: Value,
    /// Versión de Minecraft (`install.minecraft`).
    minecraft: Option<String>,
    /// Entrada del jar universal dentro del instalador (`install.filePath`).
    file_path: String,
    /// Ruta del universal en `libraries/` (de `install.path`).
    lib_path: String,
}

fn read_zip_entry(
    zip: &mut zip::ZipArchive<std::fs::File>,
    name: &str,
    archive: &Path,
) -> Result<Option<Vec<u8>>> {
    use std::io::Read;
    let mut entry = match zip.by_name(name) {
        Ok(e) => e,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(Error::InvalidInput(format!("{}: {e}", archive.display()))),
    };
    let mut buf = Vec::new();
    entry
        .read_to_end(&mut buf)
        .map_err(|e| Error::io(archive, e))?;
    Ok(Some(buf))
}

fn inspect_installer(archive: &Path) -> Result<InstallerInfo> {
    let file = std::fs::File::open(archive).map_err(|e| Error::io(archive, e))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| {
        Error::InvalidInput(format!(
            "el instalador {} no es un zip válido ({e}); bórralo para volver a descargarlo",
            archive.display()
        ))
    })?;
    let ctx = archive.display().to_string();
    let json = |name: &str, bytes: &[u8]| -> Result<Value> {
        serde_json::from_slice(bytes).map_err(|e| Error::json(format!("{ctx}!{name}"), e))
    };

    let profile = match read_zip_entry(&mut zip, "install_profile.json", archive)? {
        Some(b) => Some(json("install_profile.json", &b)?),
        None => None,
    };

    if let Some(profile) = &profile
        && let Some(version_info) = profile.get("versionInfo").filter(|v| v.is_object())
    {
        let text = |v: &Value, key: &str| v.get(key).and_then(Value::as_str).map(str::to_owned);
        let install = profile.get("install").cloned().unwrap_or(Value::Null);
        let bad = |what: &str| {
            Error::InvalidInput(format!(
                "instalador antiguo sin {what}: {}",
                archive.display()
            ))
        };
        let id = text(version_info, "id").ok_or_else(|| bad("versionInfo.id"))?;
        let file_path = text(&install, "filePath").ok_or_else(|| bad("install.filePath"))?;
        let coord = text(&install, "path")
            .and_then(|p| Coordinate::parse(&p))
            .ok_or_else(|| bad("install.path"))?;
        return Ok(InstallerInfo {
            id,
            legacy: Some(Legacy {
                version_info: version_info.clone(),
                minecraft: text(&install, "minecraft"),
                file_path,
                lib_path: coord.path(),
            }),
        });
    }

    // Formato moderno: `version.json` es lo que se escribe en `versions/<id>/`.
    let from_version_json = match read_zip_entry(&mut zip, "version.json", archive)? {
        Some(b) => json("version.json", &b)?
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        None => None,
    };
    let id = from_version_json
        .or_else(|| {
            profile
                .as_ref()
                .and_then(|p| p.get("version"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .ok_or_else(|| {
            Error::InvalidInput(format!(
                "no se encontró el id de versión en el instalador {}",
                archive.display()
            ))
        })?;
    Ok(InstallerInfo { id, legacy: None })
}

/// Librerías del perfil instalado que genera el instalador (`url: None`) y que
/// faltan en disco. `None` si `versions/<id>/<id>.json` no existe.
async fn missing_generated(core: &Core, id: &str) -> Result<Option<Vec<String>>> {
    let Some(info) = core.installed_version_json(id).await? else {
        return Ok(None);
    };
    let platform = Platform::current();
    let libs_dir = core.paths().libraries_dir();
    let mut missing = Vec::new();
    for lib in info
        .libraries
        .iter()
        .flat_map(|l| l.resolve(&platform, &Features::new()))
        .filter(|l| l.url.is_none())
    {
        let dest = safe_join(&libs_dir, &lib.path)?;
        if !tokio::fs::try_exists(&dest).await.unwrap_or(false) {
            missing.push(lib.name);
        }
    }
    Ok(Some(missing))
}

async fn install_legacy(core: &Core, installer: &Path, id: &str, legacy: &Legacy) -> Result<()> {
    let universal = safe_join(&core.paths().libraries_dir(), &legacy.lib_path)?;
    let have_universal = tokio::fs::metadata(&universal)
        .await
        .is_ok_and(|m| m.len() > 0);
    if have_universal && core.installed_version_json(id).await?.is_some() {
        return Ok(());
    }

    if !have_universal {
        let jar = installer.to_path_buf();
        let entry = legacy.file_path.clone();
        let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
            let file = std::fs::File::open(&jar).map_err(|e| Error::io(&jar, e))?;
            let mut zip = zip::ZipArchive::new(file)
                .map_err(|e| Error::InvalidInput(format!("{}: {e}", jar.display())))?;
            read_zip_entry(&mut zip, &entry, &jar)?.ok_or_else(|| {
                Error::InvalidInput(format!("{} no contiene {entry}", jar.display()))
            })
        })
        .await
        .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))??;
        write_atomic(&universal, &bytes).await?;
    }

    // Sin `inheritsFrom` ni cliente propio el perfil no se podría instalar: se
    // apoya en el Minecraft vanilla del que nace.
    let mut profile = legacy.version_info.clone();
    if let Some(obj) = profile.as_object_mut()
        && !obj.contains_key("inheritsFrom")
        && !obj.contains_key("downloads")
        && let Some(mc) = &legacy.minecraft
    {
        obj.insert("inheritsFrom".into(), Value::String(mc.clone()));
    }
    let raw = serde_json::to_vec_pretty(&profile)
        .map_err(|e| Error::json("perfil del instalador antiguo", e))?;
    core.save_version_json(&raw).await?;
    Ok(())
}

/// Ejecuta `java -jar <instalador> --installClient <datos>`.
async fn run_installer(
    core: &Core,
    src: &Source,
    full: &str,
    installer: &Path,
    java: &Path,
) -> Result<()> {
    let paths = core.paths();
    let mc = &src.mc;

    // El instalador exige el Minecraft vanilla (JSON y jar) en `versions/<mc>/`.
    let vanilla = core.vanilla_version(mc).await?;
    let client = vanilla
        .downloads
        .as_ref()
        .and_then(|d| d.client.as_ref())
        .ok_or_else(|| Error::InvalidInput(format!("{mc} no tiene cliente descargable")))?;
    let client_task = DownloadTask {
        url: client.url.clone(),
        dest: paths.versions_dir().join(mc).join(format!("{mc}.jar")),
        sha1: client.sha1.clone(),
        size: client.size,
    };
    if !is_valid(&client_task, Verify::Size).await {
        download_one(core.http(), &client_task).await?;
    }

    let profiles = paths.root().join("launcher_profiles.json");
    if !tokio::fs::try_exists(&profiles).await.unwrap_or(false) {
        write_atomic(&profiles, PROFILES_STUB.as_bytes()).await?;
    }

    // Rutas absolutas: el instalador corre con otro directorio de trabajo.
    let canon = |p: &Path| std::fs::canonicalize(p).map_err(|e| Error::io(p, e));
    let root = canon(paths.root())?;
    let installers = paths.installers_dir();
    let installer = canon(installer)?;

    let log = log_path(core, src, full);
    let out = std::fs::File::create(&log).map_err(|e| Error::io(&log, e))?;
    let err = out.try_clone().map_err(|e| Error::io(&log, e))?;
    tracing::info!(
        loader = src.flavor.name(),
        version = full,
        "ejecutando el instalador"
    );
    let mut child = tokio::process::Command::new(java)
        .arg("-jar")
        .arg(&installer)
        .arg("--installClient")
        .arg(&root)
        .current_dir(&installers)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| Error::io(java, e))?;

    let status = match tokio::time::timeout(INSTALL_TIMEOUT, child.wait()).await {
        Ok(status) => status.map_err(|e| Error::io(java, e))?,
        Err(_) => {
            let _ = child.kill().await;
            return Err(Error::InvalidInput(format!(
                "el instalador de {} no terminó en {} minutos (revisa {})",
                src.flavor.name(),
                INSTALL_TIMEOUT.as_secs() / 60,
                log.display()
            )));
        }
    };
    if !status.success() {
        let tail = tokio::fs::read_to_string(&log)
            .await
            .map(|t| {
                let lines: Vec<&str> = t.lines().rev().take(5).collect();
                lines.into_iter().rev().collect::<Vec<_>>().join(" | ")
            })
            .unwrap_or_default();
        return Err(Error::InvalidInput(format!(
            "el instalador de {} falló ({status}): {tail} (registro en {})",
            src.flavor.name(),
            log.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::Arc;

    use super::*;
    use crate::Options;
    use crate::secrets::MemoryStore;

    fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            for (name, data) in entries {
                zip.start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(data).unwrap();
            }
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    async fn offline_core(dir: &Path) -> Core {
        Core::open(Options {
            data_dir: Some(dir.to_path_buf()),
            secrets: Some(Arc::new(MemoryStore::default())),
            offline: true,
            ..Options::default()
        })
        .await
        .unwrap()
    }

    #[test]
    fn xml_de_versiones() {
        let xml = r#"<?xml version="1.0"?>
<metadata><versioning><latest>1.20.1-47.4.0</latest><versions>
  <version>1.20.1-47.3.0</version>
  <version> 1.20.1-47.4.0 </version>
  <version>1.20.1-47.3.0</version>
  <version>1.7.10-10.13.4.1614-1.7.10</version>
</versions></versioning></metadata>"#;
        assert_eq!(
            parse_maven_versions(xml),
            [
                "1.20.1-47.3.0",
                "1.20.1-47.4.0",
                "1.7.10-10.13.4.1614-1.7.10"
            ]
        );
    }

    #[test]
    fn filtra_y_ordena_forge() {
        let all: Vec<String> = [
            "1.20.1-47.3.0",
            "1.20.1-47.10.1",
            "1.20.1-47.4.9",
            "1.20-46.0.14",
            "1.7.10-10.13.4.1614-1.7.10",
            "1.7.10-10.13.4.1614",
            "1.7.10_pre4-10.13.0.1150",
        ]
        .map(String::from)
        .into();
        let src = Source::new(Flavor::Forge, "1.20.1").unwrap();
        assert_eq!(filter_versions(&src, &all), ["47.10.1", "47.4.9", "47.3.0"]);
        let src = Source::new(Flavor::Forge, "1.7.10").unwrap();
        // Sin sufijo antes que con sufijo cuando el número coincide.
        assert_eq!(
            filter_versions(&src, &all),
            ["10.13.4.1614", "10.13.4.1614-1.7.10"]
        );
    }

    #[test]
    fn promociones() {
        let file: PromotionsFile = serde_json::from_str(
            r#"{"homepage":"x","promos":{"1.20.1-latest":"47.4.26","1.20.1-recommended":"47.4.10",
               "1.7.10-latest":"10.13.4.1614"}}"#,
        )
        .unwrap();
        let p = promos_for(&file, "1.20.1");
        assert_eq!(p.recommended.as_deref(), Some("47.4.10"));
        assert_eq!(p.latest.as_deref(), Some("47.4.26"));
        assert_eq!(promos_for(&file, "1.7.10").recommended, None);
        assert_eq!(promos_for(&file, "1.99"), Promos::default());

        let list: Vec<String> = ["47.4.26", "47.4.10", "47.3.0"].map(String::from).into();
        assert!(is_stable(Flavor::Forge, "47.4.26", &p));
        assert!(is_stable(Flavor::Forge, "47.4.10", &p));
        assert!(!is_stable(Flavor::Forge, "47.3.0", &p));
        assert_eq!(pick_default(Flavor::Forge, &list, &p), "47.4.10");
        // Sin recomendada: la última promocionada; sin nada: la más nueva.
        let only_latest = Promos {
            recommended: None,
            latest: p.latest.clone(),
        };
        assert_eq!(pick_default(Flavor::Forge, &list, &only_latest), "47.4.26");
        assert_eq!(
            pick_default(Flavor::Forge, &list, &Promos::default()),
            "47.4.26"
        );
        // El número de las promociones casa con versiones con sufijo de rama.
        let old: Vec<String> = ["10.13.4.1614-1.7.10"].map(String::from).into();
        let p = Promos {
            recommended: Some("10.13.4.1614".into()),
            latest: None,
        };
        assert_eq!(pick_default(Flavor::Forge, &old, &p), "10.13.4.1614-1.7.10");
        assert!(is_stable(Flavor::Forge, "10.13.4.1614-1.7.10", &p));
    }

    #[test]
    fn mapeo_minecraft_a_neoforge() {
        for (mc, prefix) in [
            ("1.21", Some("21.0.")),
            ("1.21.1", Some("21.1.")),
            ("1.21.11", Some("21.11.")),
            ("1.20.4", Some("20.4.")),
            ("1.20.2", Some("20.2.")),
            // Versiones por año: `<mc>.<compilación>`, con parche 0 implícito.
            ("26.1", Some("26.1.0.")),
            ("26.2", Some("26.2.0.")),
            ("26.2.1", Some("26.2.1.")),
            ("26.1.2", Some("26.1.2.")),
            ("1.21-pre1", None),
            ("26.3-rc-1", None),
            ("26.4-snapshot-3", None),
            ("24w14a", None),
            ("1", None),
            ("1.x", None),
        ] {
            assert_eq!(neoforge_prefix(mc).as_deref(), prefix, "{mc}");
        }
    }

    #[test]
    fn filtra_neoforge() {
        let all: Vec<String> = [
            "20.4.237",
            "20.4.0-beta",
            "21.0.167",
            "21.1.9",
            "21.1.77",
            "21.1.100-beta",
            "21.10.5",
            "26.1.0.0-alpha.1+snapshot-1",
            "26.1.2.115",
            "26.2.0.89",
            "26.2.0.90",
            "26.3.0.75-beta",
            "26.3.0.76",
        ]
        .map(String::from)
        .into();
        let list = |mc: &str| filter_versions(&Source::new(Flavor::NeoForge, mc).unwrap(), &all);
        assert_eq!(list("1.21.1"), ["21.1.100-beta", "21.1.77", "21.1.9"]);
        assert_eq!(list("1.21"), ["21.0.167"]);
        assert_eq!(list("1.20.4"), ["20.4.237", "20.4.0-beta"]);
        assert_eq!(list("26.2"), ["26.2.0.90", "26.2.0.89"]);
        assert_eq!(list("26.1.2"), ["26.1.2.115"]);
        assert_eq!(list("26.3"), ["26.3.0.76", "26.3.0.75-beta"]);
        // Las compilaciones de snapshots (`+snapshot-N`) no cuentan.
        assert!(list("26.1").is_empty());
        assert!(list("26.4-snapshot-3").is_empty());

        let none = Promos::default();
        assert!(!is_stable(Flavor::NeoForge, "21.1.100-beta", &none));
        assert!(is_stable(Flavor::NeoForge, "21.1.77", &none));
        assert_eq!(
            pick_default(Flavor::NeoForge, &list("1.21.1"), &none),
            "21.1.77"
        );
        // Solo betas: se elige la más nueva.
        assert_eq!(
            pick_default(Flavor::NeoForge, &["20.4.9-beta".to_owned()], &none),
            "20.4.9-beta"
        );
    }

    #[test]
    fn neoforge_1_20_1_usa_el_artefacto_forge() {
        let src = Source::new(Flavor::NeoForge, "1.20.1").unwrap();
        assert_eq!(
            src.metadata_url(),
            "https://maven.neoforged.net/releases/net/neoforged/forge/maven-metadata.xml"
        );
        let all: Vec<String> = ["1.20.1-47.1.106", "1.20.1-47.1.9", "20.2.3"]
            .map(String::from)
            .into();
        assert_eq!(filter_versions(&src, &all), ["47.1.106", "47.1.9"]);
        assert_eq!(
            src.installer_url(&src.full_version("47.1.106")),
            "https://maven.neoforged.net/releases/net/neoforged/forge/1.20.1-47.1.106/forge-1.20.1-47.1.106-installer.jar"
        );
    }

    #[test]
    fn urls_de_instaladores() {
        let f = Source::new(Flavor::Forge, "1.20.1").unwrap();
        assert_eq!(
            f.installer_url(&f.full_version("47.4.10")),
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.10/forge-1.20.1-47.4.10-installer.jar"
        );
        let n = Source::new(Flavor::NeoForge, "1.21.1").unwrap();
        assert_eq!(
            n.installer_url(&n.full_version("21.1.77")),
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.77/neoforge-21.1.77-installer.jar"
        );
        assert!(Source::new(Flavor::Forge, "../x").is_err());
    }

    #[test]
    fn id_del_instalador_moderno() {
        let dir = tempfile::tempdir().unwrap();
        let jar = dir.path().join("forge-installer.jar");
        std::fs::write(
            &jar,
            zip_bytes(&[
                (
                    "install_profile.json",
                    br#"{"spec":1,"version":"1.20.1-forge-otro","json":"/version.json"}"#,
                ),
                (
                    "version.json",
                    br#"{"id":"1.20.1-forge-47.3.0","inheritsFrom":"1.20.1","libraries":[]}"#,
                ),
            ]),
        )
        .unwrap();
        let info = inspect_installer(&jar).unwrap();
        assert_eq!(info.id, "1.20.1-forge-47.3.0");
        assert!(info.legacy.is_none());

        // Sin version.json se usa `version` del perfil.
        std::fs::write(
            &jar,
            zip_bytes(&[("install_profile.json", br#"{"version":"neoforge-21.1.77"}"#)]),
        )
        .unwrap();
        assert_eq!(inspect_installer(&jar).unwrap().id, "neoforge-21.1.77");

        std::fs::write(&jar, zip_bytes(&[("otro.txt", b"x")])).unwrap();
        assert!(inspect_installer(&jar).is_err());
        std::fs::write(&jar, b"no es un zip").unwrap();
        assert!(inspect_installer(&jar).is_err());
    }

    const LEGACY_PROFILE: &str = r#"{
      "install": {
        "profileName": "Forge", "target": "Forge10.13.4.1614",
        "path": "net.minecraftforge:forge:1.7.10-10.13.4.1614-1.7.10",
        "version": "Forge 10.13.4.1614",
        "filePath": "forge-1.7.10-10.13.4.1614-1.7.10-universal.jar",
        "minecraft": "1.7.10"
      },
      "versionInfo": {
        "id": "1.7.10-Forge10.13.4.1614-1.7.10",
        "type": "release",
        "minecraftArguments": "--username ${auth_player_name} --tweakClass cpw.mods.fml.common.launcher.FMLTweaker",
        "mainClass": "net.minecraft.launchwrapper.Launch",
        "libraries": [
          {"name": "net.minecraftforge:forge:1.7.10-10.13.4.1614-1.7.10"},
          {"name": "net.minecraft:launchwrapper:1.12"}
        ]
      }
    }"#;

    #[tokio::test]
    async fn instalador_antiguo_sin_java() {
        let dir = tempfile::tempdir().unwrap();
        let core = offline_core(dir.path()).await;
        let jar = dir
            .path()
            .join("forge-1.7.10-10.13.4.1614-1.7.10-installer.jar");
        std::fs::write(
            &jar,
            zip_bytes(&[
                ("install_profile.json", LEGACY_PROFILE.as_bytes()),
                (
                    "forge-1.7.10-10.13.4.1614-1.7.10-universal.jar",
                    b"contenido del universal",
                ),
            ]),
        )
        .unwrap();

        let info = inspect_installer(&jar).unwrap();
        assert_eq!(info.id, "1.7.10-Forge10.13.4.1614-1.7.10");
        let legacy = info.legacy.as_ref().unwrap();
        assert_eq!(
            legacy.lib_path,
            "net/minecraftforge/forge/1.7.10-10.13.4.1614-1.7.10/forge-1.7.10-10.13.4.1614-1.7.10.jar"
        );

        let src = Source::new(Flavor::Forge, "1.7.10").unwrap();
        // `java` no existe: el camino antiguo no debe ejecutarlo.
        let id = install_from_jar(
            &core,
            &src,
            "1.7.10-10.13.4.1614-1.7.10",
            &jar,
            Path::new("/no/existe/java"),
        )
        .await
        .unwrap();
        assert_eq!(id, "1.7.10-Forge10.13.4.1614-1.7.10");

        let universal = core.paths().libraries_dir().join(&legacy.lib_path);
        assert_eq!(
            std::fs::read(&universal).unwrap(),
            b"contenido del universal"
        );
        let saved = core.installed_version_json(&id).await.unwrap().unwrap();
        assert_eq!(saved.inherits_from.as_deref(), Some("1.7.10"));
        assert_eq!(
            saved.main_class.as_deref(),
            Some("net.minecraft.launchwrapper.Launch")
        );
        assert_eq!(saved.libraries.len(), 2);

        // Idempotente: una segunda pasada no toca nada.
        let before = std::fs::metadata(&universal).unwrap().modified().unwrap();
        let again = install_from_jar(
            &core,
            &src,
            "1.7.10-10.13.4.1614-1.7.10",
            &jar,
            Path::new("/no/existe/java"),
        )
        .await
        .unwrap();
        assert_eq!(again, id);
        assert_eq!(
            std::fs::metadata(&universal).unwrap().modified().unwrap(),
            before
        );
    }

    #[tokio::test]
    async fn librerias_generadas_que_faltan() {
        let dir = tempfile::tempdir().unwrap();
        let core = offline_core(dir.path()).await;
        assert_eq!(missing_generated(&core, "x-1").await.unwrap(), None);
        let json = r#"{"id":"x-1","inheritsFrom":"1.20.1","libraries":[
            {"name":"net.minecraftforge:forge:1.20.1-47.3.0:client",
             "downloads":{"artifact":{"path":"net/minecraftforge/forge/1.20.1-47.3.0/forge-1.20.1-47.3.0-client.jar","url":"","sha1":"","size":1}}},
            {"name":"org.ow2.asm:asm:9.5",
             "downloads":{"artifact":{"path":"org/ow2/asm/asm/9.5/asm-9.5.jar","url":"https://x/asm.jar"}}}]}"#;
        core.save_version_json(json.as_bytes()).await.unwrap();
        assert_eq!(
            missing_generated(&core, "x-1").await.unwrap().unwrap(),
            ["net.minecraftforge:forge:1.20.1-47.3.0:client"]
        );
        let generated = core
            .paths()
            .libraries_dir()
            .join("net/minecraftforge/forge/1.20.1-47.3.0/forge-1.20.1-47.3.0-client.jar");
        std::fs::create_dir_all(generated.parent().unwrap()).unwrap();
        std::fs::write(generated, b"x").unwrap();
        assert!(
            missing_generated(&core, "x-1")
                .await
                .unwrap()
                .unwrap()
                .is_empty()
        );
    }

    /// Instalación real con el Java del sistema (Java 21 en el PATH). Descarga
    /// el cliente, el instalador y las librerías del loader, y luego los assets.
    async fn real_install(flavor: Flavor, mc: &str, expect_prefix: &str) {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::open(Options {
            data_dir: Some(dir.path().to_path_buf()),
            secrets: Some(Arc::new(MemoryStore::default())),
            ..Options::default()
        })
        .await
        .unwrap();
        let java = PathBuf::from("java");
        let version = latest_stable(&core, flavor, mc).await.unwrap();
        eprintln!("{flavor:?} {mc}: instalando {version}");
        let id = install(&core, flavor, mc, &version, &java).await.unwrap();
        eprintln!("id instalado: {id}");
        assert!(id.contains(expect_prefix), "{id}");
        assert_eq!(
            missing_generated(&core, &id).await.unwrap().unwrap(),
            Vec::<String>::new()
        );
        // Segunda llamada: idempotente y sin ejecutar Java.
        let again = install(&core, flavor, mc, &version, Path::new("/no/existe/java"))
            .await
            .unwrap();
        assert_eq!(again, id);
        let installed = core.install_version(&id, Verify::Size).await.unwrap();
        eprintln!(
            "classpath: {} entradas; main class: {:?}",
            installed.classpath.len(),
            installed.version.main_class
        );
        assert!(installed.classpath.iter().all(|p| p.exists()));
    }

    #[tokio::test]
    #[ignore = "red: instala NeoForge de verdad (necesita Java 21 en el PATH)"]
    async fn instala_neoforge_1_21_1() {
        real_install(Flavor::NeoForge, "1.21.1", "neoforge-21.1.").await;
    }

    #[tokio::test]
    #[ignore = "red: instala Forge de verdad (necesita Java 21 en el PATH)"]
    async fn instala_forge_1_20_1() {
        real_install(Flavor::Forge, "1.20.1", "forge").await;
    }
    /// 1.7.10 (instalador con `versionInfo`: sin Java, solo archivos) y 1.12.2 (formato moderno).
    #[tokio::test]
    #[ignore = "red: instala Forge antiguo (necesita Java en el PATH)"]
    async fn instala_forge_antiguo() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::open(Options {
            data_dir: Some(dir.path().to_path_buf()),
            secrets: Some(Arc::new(MemoryStore::default())),
            ..Options::default()
        })
        .await
        .unwrap();
        for mc in ["1.7.10", "1.12.2"] {
            let version = latest_stable(&core, Flavor::Forge, mc).await.unwrap();
            let id = install(&core, Flavor::Forge, mc, &version, Path::new("java"))
                .await
                .unwrap();
            let merged = core.resolve_version(&id).await.unwrap();
            eprintln!(
                "{mc}: {version} -> {id}; {} librerías; main {:?}",
                merged.libraries.len(),
                merged.main_class
            );
            assert!(merged.downloads.is_some());
            assert_eq!(
                missing_generated(&core, &id).await.unwrap().unwrap(),
                Vec::<String>::new()
            );
        }
    }
}
