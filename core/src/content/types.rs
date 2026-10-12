//! Tipos comunes a Modrinth y CurseForge.

use serde::{Deserialize, Serialize};

use crate::instance::LoaderKind;

/// Plataforma de la que viene un proyecto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    #[default]
    Modrinth,
    CurseForge,
}

impl Source {
    /// Valor de `source` en `content.json`.
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Modrinth => "modrinth",
            Source::CurseForge => "curseforge",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "modrinth" => Some(Source::Modrinth),
            "curseforge" => Some(Source::CurseForge),
            _ => None,
        }
    }
}

/// Tipo de contenido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentKind {
    #[default]
    Mod,
    ResourcePack,
    Shader,
    Datapack,
    Modpack,
}

impl ContentKind {
    /// Valor de `type` en `content.json`.
    pub fn as_str(self) -> &'static str {
        match self {
            ContentKind::Mod => "mod",
            ContentKind::ResourcePack => "resourcepack",
            ContentKind::Shader => "shader",
            ContentKind::Datapack => "datapack",
            ContentKind::Modpack => "modpack",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "mod" | "mods" => ContentKind::Mod,
            "resourcepack" | "resourcepacks" => ContentKind::ResourcePack,
            "shader" | "shaders" | "shaderpack" | "shaderpacks" => ContentKind::Shader,
            "datapack" | "datapacks" => ContentKind::Datapack,
            "modpack" | "modpacks" => ContentKind::Modpack,
            _ => return None,
        })
    }

    /// Carpeta dentro del directorio del juego; los datapacks viven en cada mundo.
    pub fn folder(self) -> Option<&'static str> {
        match self {
            ContentKind::Mod => Some("mods"),
            ContentKind::ResourcePack => Some("resourcepacks"),
            ContentKind::Shader => Some("shaderpacks"),
            ContentKind::Datapack | ContentKind::Modpack => None,
        }
    }

    /// Si el loader de la instancia filtra las versiones compatibles.
    pub fn uses_loader(self) -> bool {
        matches!(self, ContentKind::Mod | ContentKind::Modpack)
    }

    /// `project_type` de Modrinth.
    pub fn modrinth_type(self) -> &'static str {
        self.as_str()
    }

    /// `classId` de CurseForge.
    pub fn curseforge_class(self) -> u32 {
        match self {
            ContentKind::Mod => 6,
            ContentKind::ResourcePack => 12,
            ContentKind::Shader => 6552,
            ContentKind::Datapack => 6945,
            ContentKind::Modpack => 4471,
        }
    }

    pub fn from_curseforge_class(class: u32) -> Option<Self> {
        [
            ContentKind::Mod,
            ContentKind::ResourcePack,
            ContentKind::Shader,
            ContentKind::Datapack,
            ContentKind::Modpack,
        ]
        .into_iter()
        .find(|k| k.curseforge_class() == class)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

impl SortOrder {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "relevance" => SortOrder::Relevance,
            "downloads" => SortOrder::Downloads,
            "follows" => SortOrder::Follows,
            "newest" => SortOrder::Newest,
            "updated" => SortOrder::Updated,
            _ => return None,
        })
    }

    /// Valor de `index` en la búsqueda de Modrinth.
    pub fn modrinth_index(self) -> &'static str {
        match self {
            SortOrder::Relevance => "relevance",
            SortOrder::Downloads => "downloads",
            SortOrder::Follows => "follows",
            SortOrder::Newest => "newest",
            SortOrder::Updated => "updated",
        }
    }

    /// Valor de `sortField` en CurseForge.
    pub fn curseforge_field(self) -> u32 {
        match self {
            SortOrder::Relevance => 1,
            SortOrder::Downloads => 6,
            SortOrder::Follows => 2,
            SortOrder::Newest => 11,
            SortOrder::Updated => 3,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ContentQuery {
    pub source: Source,
    pub kind: ContentKind,
    pub text: String,
    pub mc: Option<String>,
    /// Solo se aplica a los tipos que dependen del loader (mods y modpacks).
    pub loader: Option<LoaderKind>,
    pub offset: u32,
    pub limit: u32,
    pub sort: SortOrder,
}

#[derive(Debug, Clone, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub offset: u32,
    pub limit: u32,
    pub total: u32,
}

/// Proyecto de una búsqueda o ficha de un proyecto.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProjectHit {
    pub source: Source,
    /// Id del proyecto (numérico en CurseForge).
    pub id: String,
    pub slug: String,
    /// `None` si es un tipo que no se gestiona (plugins, etc.).
    pub kind: Option<ContentKind>,
    pub title: String,
    pub description: String,
    pub author: Option<String>,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub categories: Vec<String>,
    pub date_modified: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseType {
    Release,
    Beta,
    Alpha,
}

impl ReleaseType {
    /// Mayor es mejor: se prefiere una release a una beta.
    pub fn rank(self) -> u8 {
        match self {
            ReleaseType::Release => 2,
            ReleaseType::Beta => 1,
            ReleaseType::Alpha => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VersionFile {
    pub filename: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: u64,
    pub primary: bool,
    /// La URL no la dio la API (CurseForge sin `downloadUrl`): puede fallar.
    pub fallback_url: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Dependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub kind: DependencyKind,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContentVersion {
    pub source: Source,
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub mc_versions: Vec<String>,
    /// En minúsculas: `fabric`, `forge`, `iris`…
    pub loaders: Vec<String>,
    pub release: ReleaseType,
    pub date_published: String,
    pub files: Vec<VersionFile>,
    pub dependencies: Vec<Dependency>,
}

impl ContentVersion {
    /// Archivo principal (o el primero que parezca un `.jar`/`.zip`).
    pub fn primary_file(&self) -> Option<&VersionFile> {
        self.files
            .iter()
            .find(|f| f.primary)
            .or_else(|| {
                self.files.iter().find(|f| {
                    let n = f.filename.to_ascii_lowercase();
                    n.ends_with(".jar") || n.ends_with(".zip")
                })
            })
            .or_else(|| self.files.first())
    }

    pub fn required_dependencies(&self) -> impl Iterator<Item = &Dependency> {
        self.dependencies
            .iter()
            .filter(|d| d.kind == DependencyKind::Required)
    }
}

/// Nombres de loader de Modrinth/CurseForge que acepta una instancia.
/// Quilt también ejecuta los mods de Fabric.
pub fn loader_names(loader: LoaderKind) -> Vec<&'static str> {
    match loader {
        LoaderKind::Fabric => vec!["fabric"],
        LoaderKind::Quilt => vec!["quilt", "fabric"],
        LoaderKind::Forge => vec!["forge"],
        LoaderKind::NeoForge => vec!["neoforge"],
        _ => Vec::new(),
    }
}

/// «Loaders» de Modrinth de los tipos que no dependen del loader del juego.
pub const NON_MOD_LOADERS: &[&str] = &[
    "minecraft",
    "datapack",
    "iris",
    "optifine",
    "canvas",
    "vanilla",
];
