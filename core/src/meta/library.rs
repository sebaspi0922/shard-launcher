//! Librerías: coordenadas Maven, reglas y resolución para una plataforma.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::rules::{Features, Platform, Rule, rules_allow};

pub const MOJANG_LIBRARIES: &str = "https://libraries.minecraft.net/";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<LibraryDownloads>,
    /// Base Maven (perfiles de Fabric, Quilt y Forge antiguos).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<Rule>>,
    /// Formato antiguo: SO → clasificador con natives (`natives-windows-${arch}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub natives: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extract: Option<Extract>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<Artifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classifiers: Option<BTreeMap<String, Artifact>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artifact {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Coordenada Maven `grupo:artefacto:versión[:clasificador][@extensión]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coordinate {
    pub group: String,
    pub artifact: String,
    pub version: String,
    pub classifier: Option<String>,
    pub extension: String,
}

impl Coordinate {
    pub fn parse(name: &str) -> Option<Self> {
        let (main, extension) = match name.split_once('@') {
            Some((m, e)) => (m, e.to_owned()),
            None => (name, "jar".to_owned()),
        };
        let parts: Vec<&str> = main.split(':').collect();
        let (group, artifact, version, classifier) = match parts.as_slice() {
            [g, a, v] => (g, a, v, None),
            [g, a, v, c] => (g, a, v, Some((*c).to_owned())),
            _ => return None,
        };
        if group.is_empty() || artifact.is_empty() || version.is_empty() {
            return None;
        }
        Some(Self {
            group: (*group).to_owned(),
            artifact: (*artifact).to_owned(),
            version: (*version).to_owned(),
            classifier,
            extension,
        })
    }

    pub fn with_classifier(&self, classifier: &str) -> Self {
        Self {
            classifier: Some(classifier.to_owned()),
            ..self.clone()
        }
    }

    /// Ruta relativa dentro de `libraries/`, con `/`.
    pub fn path(&self) -> String {
        let file = match &self.classifier {
            Some(c) => format!("{}-{}-{c}.{}", self.artifact, self.version, self.extension),
            None => format!("{}-{}.{}", self.artifact, self.version, self.extension),
        };
        format!(
            "{}/{}/{}/{file}",
            self.group.replace('.', "/"),
            self.artifact,
            self.version
        )
    }
}

/// Archivo concreto que hay que tener en `libraries/`.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedLibrary {
    pub name: String,
    /// Relativa a `libraries/`.
    pub path: String,
    /// `None` si lo genera un instalador (Forge/NeoForge) y no se descarga.
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub kind: LibraryKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LibraryKind {
    Classpath,
    /// Natives antiguos que se extraen en el directorio de natives.
    Natives {
        exclude: Vec<String>,
    },
}

impl Library {
    /// Clave para quitar duplicados: grupo, artefacto y clasificador, sin versión.
    pub fn dedup_key(&self) -> String {
        match Coordinate::parse(&self.name) {
            Some(c) => format!(
                "{}:{}:{}",
                c.group,
                c.artifact,
                c.classifier.unwrap_or_default()
            ),
            None => self.name.clone(),
        }
    }

    /// Archivos de esta librería para la plataforma (vacío si las reglas la excluyen).
    pub fn resolve(&self, platform: &Platform, features: &Features) -> Vec<ResolvedLibrary> {
        if let Some(rules) = &self.rules
            && !rules_allow(rules, platform, features)
        {
            return Vec::new();
        }
        let Some(coord) = Coordinate::parse(&self.name) else {
            tracing::warn!(name = %self.name, "coordenada Maven no válida");
            return Vec::new();
        };
        let mut out = Vec::new();

        let artifact = self.downloads.as_ref().and_then(|d| d.artifact.as_ref());
        match artifact {
            Some(a) => out.push(ResolvedLibrary {
                name: self.name.clone(),
                path: a.path.clone().unwrap_or_else(|| coord.path()),
                url: (!a.url.is_empty()).then(|| a.url.clone()),
                sha1: a.sha1.clone(),
                size: a.size,
                kind: LibraryKind::Classpath,
            }),
            // Sin `downloads.artifact`: o es Maven con base (`url`) o una librería
            // antigua de Mojang. Las de natives antiguos solo tienen clasificadores.
            None if self.natives.is_none() || self.url.is_some() || self.downloads.is_none() => {
                let path = coord.path();
                let base = self.url.as_deref().unwrap_or(MOJANG_LIBRARIES);
                out.push(ResolvedLibrary {
                    name: self.name.clone(),
                    url: Some(format!("{}/{path}", base.trim_end_matches('/'))),
                    path,
                    sha1: self.sha1.clone(),
                    size: self.size,
                    kind: LibraryKind::Classpath,
                });
            }
            None => {}
        }

        if let Some(natives) = &self.natives
            && let Some(classifier) = natives.get(platform.os.mojang_name())
        {
            let classifier = classifier.replace("${arch}", platform.arch_bits());
            let exclude = self.extract.clone().unwrap_or_default().exclude;
            let from_downloads = self
                .downloads
                .as_ref()
                .and_then(|d| d.classifiers.as_ref())
                .and_then(|c| c.get(&classifier));
            let coord = coord.with_classifier(&classifier);
            let resolved = match from_downloads {
                Some(a) => ResolvedLibrary {
                    name: format!("{}:{classifier}", self.name),
                    path: a.path.clone().unwrap_or_else(|| coord.path()),
                    url: Some(a.url.clone()),
                    sha1: a.sha1.clone(),
                    size: a.size,
                    kind: LibraryKind::Natives { exclude },
                },
                None => {
                    let path = coord.path();
                    let base = self.url.as_deref().unwrap_or(MOJANG_LIBRARIES);
                    ResolvedLibrary {
                        name: format!("{}:{classifier}", self.name),
                        url: Some(format!("{}/{path}", base.trim_end_matches('/'))),
                        path,
                        sha1: None,
                        size: None,
                        kind: LibraryKind::Natives { exclude },
                    }
                }
            };
            // Los natives antiguos no son parte del classpath.
            out.retain(|r| r.path != resolved.path);
            out.push(resolved);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::rules::{Arch, Os};
    use super::super::version::VersionInfo;
    use super::*;

    #[test]
    fn rutas_maven() {
        let c = Coordinate::parse("org.lwjgl:lwjgl:3.4.1:natives-linux").unwrap();
        assert_eq!(
            c.path(),
            "org/lwjgl/lwjgl/3.4.1/lwjgl-3.4.1-natives-linux.jar"
        );
        let c = Coordinate::parse("de.oceanlabs.mcp:mcp_config:1.20.1@zip").unwrap();
        assert_eq!(
            c.path(),
            "de/oceanlabs/mcp/mcp_config/1.20.1/mcp_config-1.20.1.zip"
        );
        assert!(Coordinate::parse("a:b").is_none());
    }

    fn resolve_all(version: &str, platform: &Platform) -> Vec<ResolvedLibrary> {
        let text = std::fs::read_to_string(format!(
            "{}/tests/fixtures/mojang/{version}.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let v: VersionInfo = serde_json::from_str(&text).unwrap();
        v.libraries
            .iter()
            .flat_map(|l| l.resolve(platform, &Features::new()))
            .collect()
    }

    #[test]
    fn natives_modernos_por_so() {
        let linux = resolve_all("26.2", &Platform::new(Os::Linux, Arch::X86_64));
        assert!(
            linux
                .iter()
                .any(|l| l.path.ends_with("lwjgl-3.4.1-natives-linux.jar"))
        );
        assert!(!linux.iter().any(|l| l.path.contains("natives-windows")));
        assert!(!linux.iter().any(|l| l.name.contains("java-objc-bridge")));
        assert!(linux.iter().all(|l| l.kind == LibraryKind::Classpath));
        assert!(linux.iter().all(|l| l.url.is_some() && l.sha1.is_some()));

        let mac = resolve_all("26.2", &Platform::new(Os::Osx, Arch::Aarch64));
        assert!(
            mac.iter()
                .any(|l| l.path.ends_with("natives-macos-arm64.jar"))
        );
        assert!(mac.iter().any(|l| l.name.contains("java-objc-bridge")));
    }

    #[test]
    fn natives_antiguos_se_extraen() {
        let linux = resolve_all("1.12.2", &Platform::new(Os::Linux, Arch::X86_64));
        let natives: Vec<_> = linux
            .iter()
            .filter(|l| matches!(l.kind, LibraryKind::Natives { .. }))
            .collect();
        assert!(natives.iter().any(|l| {
            l.path
                .ends_with("lwjgl-platform-2.9.4-nightly-20150209-natives-linux.jar")
        }));
        assert!(natives.iter().all(|l| l.sha1.is_some()));
        // La 2.9.2 es solo para macOS.
        assert!(!linux.iter().any(|l| l.name.contains("2.9.2-nightly")));
        let win = resolve_all("1.8.9", &Platform::new(Os::Windows, Arch::X86_64));
        assert!(win.iter().any(|l| l.path.contains("natives-windows")));
    }
}
