//! Importador del directorio `.minecraft` del launcher oficial.
//!
//! Solo lectura sobre el origen. De `launcher_profiles.json` se deserializa
//! únicamente `profiles.*.{name,lastVersionId,gameDir,type,lastUsed}`; el resto
//! del archivo (incluida `authenticationDatabase`) se descarta sin guardarse.
//! Nunca se abren `launcher_accounts*.json`, `launcher_msa_credentials*.bin`
//! ni nada fuera de la lista de secciones copiables.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::common::{
    MAX_META_BYTES, Progress, Tree, alloc_id, copy_tree, guard_overlap, read_capped,
};
use crate::fsutil::now_secs;
use crate::instance::{
    BackupSettings, GameVersion, INSTANCE_SCHEMA, ImportedFrom, Instance, JavaSettings, LoaderKind,
    WindowSettings,
};
use crate::launcher::Core;
use crate::paths::single_segment;
use crate::{Error, Result};

/// Valor de `imported_from.kind` de las instancias que vienen de `.minecraft`.
pub const IMPORT_KIND: &str = "dot-minecraft";

// ---------------------------------------------------------------------------
// Secciones
// ---------------------------------------------------------------------------

/// Parte de `.minecraft` que se puede copiar a la instancia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Section {
    #[serde(rename = "saves")]
    Saves,
    #[serde(rename = "mods")]
    Mods,
    #[serde(rename = "config")]
    Config,
    #[serde(rename = "resourcepacks")]
    ResourcePacks,
    #[serde(rename = "shaderpacks")]
    ShaderPacks,
    #[serde(rename = "screenshots")]
    Screenshots,
    #[serde(rename = "options")]
    Options,
    #[serde(rename = "servers")]
    Servers,
}

impl Section {
    pub const ALL: [Section; 8] = [
        Section::Saves,
        Section::Mods,
        Section::Config,
        Section::ResourcePacks,
        Section::ShaderPacks,
        Section::Screenshots,
        Section::Options,
        Section::Servers,
    ];

    /// Nombre estable (el de la CLI y el JSON).
    pub fn as_str(self) -> &'static str {
        match self {
            Section::Saves => "saves",
            Section::Mods => "mods",
            Section::Config => "config",
            Section::ResourcePacks => "resourcepacks",
            Section::ShaderPacks => "shaderpacks",
            Section::Screenshots => "screenshots",
            Section::Options => "options",
            Section::Servers => "servers",
        }
    }

    /// Archivo o carpeta dentro de `.minecraft`.
    pub fn rel_path(self) -> &'static str {
        match self {
            Section::Options => "options.txt",
            Section::Servers => "servers.dat",
            other => other.as_str(),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|sec| s == sec.as_str() || s == sec.rel_path())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SectionInfo {
    pub section: Section,
    pub present: bool,
    pub files: u64,
    pub bytes: u64,
}

// ---------------------------------------------------------------------------
// Inspección
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LauncherProfile {
    /// Clave en `launcher_profiles.json`.
    pub id: String,
    pub name: String,
    pub last_version_id: Option<String>,
    pub game_dir: Option<PathBuf>,
    /// `latest-release`, `latest-snapshot` o `custom`.
    pub kind: Option<String>,
    pub last_used: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InstalledVersion {
    pub id: String,
    pub inherits_from: Option<String>,
    /// Versión y loader deducidos del id y de `inheritsFrom`.
    pub game: GameVersion,
    /// Fecha de modificación del JSON de la versión (segundos Unix).
    pub modified_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DotMinecraftInfo {
    pub path: PathBuf,
    pub profiles: Vec<LauncherProfile>,
    pub versions: Vec<InstalledVersion>,
    pub sections: Vec<SectionInfo>,
    pub warnings: Vec<String>,
}

/// Directorio `.minecraft` por defecto del sistema.
pub fn default_source_root() -> PathBuf {
    let dirs = directories::BaseDirs::new();
    #[cfg(target_os = "windows")]
    {
        dirs.map(|d| d.data_dir().join(".minecraft"))
            .unwrap_or_else(|| PathBuf::from(".minecraft"))
    }
    #[cfg(target_os = "macos")]
    {
        dirs.map(|d| d.data_dir().join("minecraft"))
            .unwrap_or_else(|| PathBuf::from("minecraft"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        dirs.map(|d| d.home_dir().join(".minecraft"))
            .unwrap_or_else(|| PathBuf::from(".minecraft"))
    }
}

/// Examina un `.minecraft`: perfiles, versiones instaladas y secciones
/// copiables con su tamaño.
pub async fn inspect_dot_minecraft(path: &Path) -> Result<DotMinecraftInfo> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || inspect(&path).map(|(info, _)| info))
        .await
        .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))?
}

/// `launcher_profiles.json`: solo el mapa `profiles`.
#[derive(Debug, Default, Deserialize)]
struct ProfilesFile {
    #[serde(default)]
    profiles: BTreeMap<String, RawProfile>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RawProfile {
    name: Option<String>,
    last_version_id: Option<String>,
    game_dir: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    last_used: Option<String>,
}

/// `versions/<id>/<id>.json`: solo `inheritsFrom`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct RawVersion {
    inherits_from: Option<String>,
}

/// Lo que la inspección deja listo para copiar.
struct Scanned {
    sections: Vec<(Section, Tree)>,
}

fn inspect(root: &Path) -> Result<(DotMinecraftInfo, Scanned)> {
    let meta = fs::metadata(root).map_err(|e| Error::io(root, e))?;
    if !meta.is_dir() {
        return Err(Error::InvalidPath(format!(
            "{} no es una carpeta",
            root.display()
        )));
    }
    let mut warnings = Vec::new();
    let profiles = read_profiles(root, &mut warnings);
    let versions = read_versions(root, &mut warnings);

    let mut infos = Vec::new();
    let mut scanned = Vec::new();
    for section in Section::ALL {
        let mut tree = Tree::default();
        let present = tree
            .add_path(root, Path::new(section.rel_path()), &mut warnings)
            .map_err(|e| Error::io(root.join(section.rel_path()), e))?;
        infos.push(SectionInfo {
            section,
            present,
            files: tree.files,
            bytes: tree.bytes,
        });
        if present {
            scanned.push((section, tree));
        }
    }

    if versions.is_empty() && profiles.is_empty() && scanned.is_empty() {
        return Err(Error::InvalidInput(format!(
            "{} no parece un directorio .minecraft (sin versions/, launcher_profiles.json ni contenido)",
            root.display()
        )));
    }
    Ok((
        DotMinecraftInfo {
            path: root.to_path_buf(),
            profiles,
            versions,
            sections: infos,
            warnings,
        },
        Scanned { sections: scanned },
    ))
}

fn read_profiles(root: &Path, warnings: &mut Vec<String>) -> Vec<LauncherProfile> {
    let path = root.join("launcher_profiles.json");
    let bytes = match read_capped(&path, MAX_META_BYTES) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            warnings.push(format!("launcher_profiles.json ilegible: {e}"));
            return Vec::new();
        }
    };
    let file: ProfilesFile = match serde_json::from_slice(&bytes) {
        Ok(f) => f,
        Err(e) => {
            warnings.push(format!("launcher_profiles.json no es válido: {e}"));
            return Vec::new();
        }
    };
    file.profiles
        .into_iter()
        .map(|(id, p)| LauncherProfile {
            name: p
                .name
                .map(|n| n.trim().to_owned())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| id.clone()),
            id,
            last_version_id: p.last_version_id.filter(|v| !v.is_empty()),
            game_dir: p.game_dir.filter(|d| !d.is_empty()).map(PathBuf::from),
            kind: p.kind,
            last_used: p.last_used,
        })
        .collect()
}

fn read_versions(root: &Path, warnings: &mut Vec<String>) -> Vec<InstalledVersion> {
    let dir = root.join("versions");
    let rd = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            warnings.push(format!("no se pudo leer versions/: {e}"));
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    for entry in rd.flatten() {
        let Some(id) = entry
            .file_name()
            .to_str()
            .filter(|n| single_segment(n).is_ok())
            .map(str::to_owned)
        else {
            continue;
        };
        if !entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let json = dir.join(&id).join(format!("{id}.json"));
        let Ok(meta) = fs::metadata(&json) else {
            continue; // carpeta sin JSON: no es una versión instalada
        };
        let inherits_from = match read_capped(&json, MAX_META_BYTES)
            .map_err(|e| e.to_string())
            .and_then(|b| serde_json::from_slice::<RawVersion>(&b).map_err(|e| e.to_string()))
        {
            Ok(v) => v.inherits_from.filter(|s| !s.is_empty()),
            Err(e) => {
                warnings.push(format!("versions/{id}/{id}.json no es válido: {e}"));
                continue;
            }
        };
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);
        out.push(InstalledVersion {
            game: infer_game(&id, inherits_from.as_deref()),
            id,
            inherits_from,
            modified_at,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Versión y loader a partir del id de una versión instalada.
///
/// Reconoce `fabric-loader-<v>-<mc>`, `quilt-loader-<v>-<mc>`,
/// `<mc>-forge-<v>` (y el antiguo `<mc>-forge<mc>-<v>`) y `neoforge-<v>`.
/// Un id con `inheritsFrom` que no encaja es `Custom`: se conserva el id en
/// `loader_version` para poder copiar esa versión tal cual.
pub fn infer_game(id: &str, inherits_from: Option<&str>) -> GameVersion {
    let custom = |mc: &str| GameVersion {
        mc: mc.to_owned(),
        loader: LoaderKind::Custom,
        loader_version: Some(id.to_owned()),
    };
    for (prefix, loader) in [
        ("fabric-loader-", LoaderKind::Fabric),
        ("quilt-loader-", LoaderKind::Quilt),
    ] {
        if let Some(rest) = id.strip_prefix(prefix)
            && let Some((version, mc)) = rest.split_once('-')
            && !version.is_empty()
        {
            let mc = inherits_from.unwrap_or(mc);
            if !mc.is_empty() {
                return GameVersion {
                    mc: mc.to_owned(),
                    loader,
                    loader_version: Some(version.to_owned()),
                };
            }
        }
    }
    if let Some(version) = id.strip_prefix("neoforge-").filter(|v| !v.is_empty()) {
        return match inherits_from
            .map(str::to_owned)
            .or_else(|| neoforge_mc(version))
        {
            Some(mc) => GameVersion {
                mc,
                loader: LoaderKind::NeoForge,
                loader_version: Some(version.to_owned()),
            },
            None => custom(id),
        };
    }
    if let Some((mc_in_id, rest)) = id.split_once("-forge")
        && !mc_in_id.is_empty()
    {
        let rest = rest.trim_start_matches('-');
        let version = rest
            .strip_prefix(mc_in_id)
            .map_or(rest, |r| r.trim_start_matches('-'));
        if !version.is_empty() {
            return GameVersion {
                mc: inherits_from.unwrap_or(mc_in_id).to_owned(),
                loader: LoaderKind::Forge,
                loader_version: Some(version.to_owned()),
            };
        }
    }
    match inherits_from {
        Some(mc) => custom(mc),
        None => GameVersion {
            mc: id.to_owned(),
            loader: LoaderKind::Vanilla,
            loader_version: None,
        },
    }
}

/// `21.1.77` -> `1.21.1`; `20.2.3` -> `1.20.2`; `21.0.5` -> `1.21`. Solo para
/// la numeración 20.x/21.x de NeoForge; otras (sin `inheritsFrom`) quedan sin mc.
fn neoforge_mc(version: &str) -> Option<String> {
    let mut parts = version.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    if !(19..=21).contains(&major) {
        return None;
    }
    Some(if minor == 0 {
        format!("1.{major}")
    } else {
        format!("1.{major}.{minor}")
    })
}

// ---------------------------------------------------------------------------
// Importación
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DotMinecraftImport {
    /// Carpeta `.minecraft`.
    pub source: PathBuf,
    /// Nombre de la instancia; `None` = `Minecraft <versión>`.
    pub name: Option<String>,
    /// Id de una versión instalada, id o nombre de un perfil, o una versión de
    /// Minecraft. `None` = la del perfil usado más recientemente (o la
    /// versión instalada más nueva).
    pub mc_version: Option<String>,
    /// Fuerza el loader. Si difiere del deducido, se descarta su versión.
    pub loader: Option<LoaderKind>,
    /// Qué copiar; vacío = todo lo que haya.
    pub sections: Vec<Section>,
    /// Solo calcula el plan.
    pub dry_run: bool,
}

impl DotMinecraftImport {
    pub fn new(source: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
            name: None,
            mc_version: None,
            loader: None,
            sections: Vec::new(),
            dry_run: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DotMinecraftPlan {
    pub source: PathBuf,
    pub name: String,
    /// Id que tendrá la instancia.
    pub new_id: String,
    pub game: GameVersion,
    /// Las secciones pedidas (con `present: false` las que no existen).
    pub sections: Vec<SectionInfo>,
    pub files: u64,
    pub bytes: u64,
    /// Se copia también `versions/<id>` (loader `Custom`).
    pub copies_version_dir: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DotMinecraftReport {
    pub plan: DotMinecraftPlan,
    /// Id de la instancia creada; `None` en simulación.
    pub imported: Option<String>,
    pub copied_files: u64,
}

struct Work {
    tree: Tree,
    version_dir: Option<(PathBuf, Tree)>,
}

impl Core {
    /// Importa un `.minecraft` como instancia nueva (copia, no enlaza). Con
    /// `dry_run` solo devuelve el plan.
    pub async fn import_dot_minecraft(
        &self,
        opts: &DotMinecraftImport,
    ) -> Result<DotMinecraftReport> {
        let paths = self.paths().clone();
        let job = opts.clone();
        let (plan, work) = tokio::task::spawn_blocking(move || plan(&job, &paths))
            .await
            .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))??;
        let mut report = DotMinecraftReport {
            plan,
            imported: None,
            copied_files: 0,
        };
        if opts.dry_run {
            return Ok(report);
        }

        let total = work.tree.files + work.version_dir.as_ref().map_or(0, |(_, t)| t.files);
        let progress = std::sync::Arc::new(Progress::new(self.inner.events.clone(), total));
        let inst_dir = self.paths().instance_dir(&report.plan.new_id)?;
        let result = self
            .run_dot_minecraft(&report.plan, work, &inst_dir, &progress)
            .await;
        progress.finish(&result);
        match result {
            Ok(copied) => {
                report.copied_files = copied;
                report.imported = Some(report.plan.new_id.clone());
                Ok(report)
            }
            Err(e) => {
                // El instance.json se escribe el último: no queda nada a medias.
                let _ = tokio::fs::remove_dir_all(&inst_dir).await;
                Err(e)
            }
        }
    }

    async fn run_dot_minecraft(
        &self,
        plan: &DotMinecraftPlan,
        work: Work,
        inst_dir: &Path,
        progress: &std::sync::Arc<Progress>,
    ) -> Result<u64> {
        let (source, game_dir, data_root) = (
            plan.source.clone(),
            inst_dir.join("game"),
            self.paths().root().to_path_buf(),
        );
        let prog = progress.clone();
        let copied = tokio::task::spawn_blocking(move || -> Result<u64> {
            let mut copied = copy_tree(&source, &game_dir, &work.tree, &prog)?;
            if let Some((_, tree)) = &work.version_dir {
                copied += copy_tree(&source, &data_root, tree, &prog)?;
            }
            Ok(copied)
        })
        .await
        .map_err(|e| Error::InvalidInput(format!("tarea interrumpida: {e}")))??;

        let instance = Instance {
            schema: INSTANCE_SCHEMA,
            id: plan.new_id.clone(),
            name: plan.name.clone(),
            game: plan.game.clone(),
            created_at: now_secs(),
            last_played: None,
            playtime_seconds: 0,
            java: JavaSettings::default(),
            window: WindowSettings::default(),
            backups: BackupSettings::default(),
            dev_console: false,
            imported_from: Some(ImportedFrom {
                kind: IMPORT_KIND.to_owned(),
                original_id: plan.source.display().to_string(),
            }),
        };
        self.save_instance(&instance).await?;
        Ok(copied)
    }
}

fn plan(
    opts: &DotMinecraftImport,
    paths: &crate::paths::Paths,
) -> Result<(DotMinecraftPlan, Work)> {
    let source = fs::canonicalize(&opts.source).map_err(|e| Error::io(&opts.source, e))?;
    guard_overlap(&source, paths.root())?;
    let (info, scanned) = inspect(&source)?;
    let mut warnings = info.warnings.clone();

    let game = resolve_game(&info, opts)?;

    // Secciones elegidas, sin repetir y en el orden canónico.
    let wanted: HashSet<Section> = if opts.sections.is_empty() {
        Section::ALL.into_iter().collect()
    } else {
        opts.sections.iter().copied().collect()
    };
    let mut sections = Vec::new();
    let mut tree = Tree::default();
    for section in Section::ALL.into_iter().filter(|s| wanted.contains(s)) {
        match scanned.sections.iter().find(|(s, _)| *s == section) {
            Some((_, t)) => {
                sections.push(SectionInfo {
                    section,
                    present: true,
                    files: t.files,
                    bytes: t.bytes,
                });
                tree.entries.extend(t.entries.iter().cloned());
                tree.files += t.files;
                tree.bytes += t.bytes;
            }
            None => {
                if !opts.sections.is_empty() {
                    warnings.push(format!("no existe {}; se omite", section.rel_path()));
                }
                sections.push(SectionInfo {
                    section,
                    present: false,
                    files: 0,
                    bytes: 0,
                });
            }
        }
    }

    // Una versión Custom (OptiFine, perfiles a mano…) solo existe aquí: se
    // copia su carpeta a `versions/` de la app si aún no está.
    let mut version_dir = None;
    if game.loader == LoaderKind::Custom
        && let Some(id) = game.loader_version.as_deref()
        && single_segment(id).is_ok()
        && info.versions.iter().any(|v| v.id == id)
        && !paths.versions_dir().join(id).exists()
    {
        let rel = PathBuf::from("versions").join(id);
        let mut t = Tree::default();
        t.add_path(&source, &rel, &mut warnings)
            .map_err(|e| Error::io(source.join(&rel), e))?;
        version_dir = Some((rel, t));
    }

    let name = opts
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map_or_else(|| format!("Minecraft {}", game.mc), str::to_owned);
    let new_id = alloc_id(paths, &name, &mut HashSet::new())?;

    let plan = DotMinecraftPlan {
        source,
        name,
        new_id,
        game,
        sections,
        files: tree.files,
        bytes: tree.bytes,
        copies_version_dir: version_dir.is_some(),
        warnings,
    };
    Ok((plan, Work { tree, version_dir }))
}

/// Versión de la instancia: la pedida o, por defecto, la del perfil usado
/// más recientemente (o la versión instalada más nueva).
fn resolve_game(info: &DotMinecraftInfo, opts: &DotMinecraftImport) -> Result<GameVersion> {
    let installed = |id: &str| info.versions.iter().find(|v| v.id == id);
    let from_profile = |p: &LauncherProfile| {
        p.last_version_id
            .as_deref()
            .and_then(installed)
            .map(|v| v.game.clone())
    };

    let mut game = match opts.mc_version.as_deref().map(str::trim) {
        Some(wanted) if !wanted.is_empty() => {
            if let Some(v) = installed(wanted) {
                v.game.clone()
            } else if let Some(game) = info
                .profiles
                .iter()
                .find(|p| p.id == wanted || p.name == wanted)
                .and_then(from_profile)
            {
                game
            } else {
                GameVersion {
                    mc: wanted.to_owned(),
                    loader: LoaderKind::Vanilla,
                    loader_version: None,
                }
            }
        }
        _ => {
            let mut profiles: Vec<&LauncherProfile> = info.profiles.iter().collect();
            profiles.sort_by(|a, b| b.last_used.cmp(&a.last_used));
            profiles
                .into_iter()
                .find_map(from_profile)
                .or_else(|| {
                    info.versions
                        .iter()
                        .max_by_key(|v| v.modified_at)
                        .map(|v| v.game.clone())
                })
                .ok_or_else(|| {
                    Error::InvalidInput(
                        "no se pudo deducir la versión de Minecraft: indícala con --version".into(),
                    )
                })?
        }
    };

    if let Some(loader) = opts.loader
        && loader != game.loader
    {
        game.loader = loader;
        game.loader_version = None;
    }
    Ok(game)
}
