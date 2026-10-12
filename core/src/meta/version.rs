//! JSON de versión (Mojang, perfiles de loaders e instaladores) y su fusión.

use serde::{Deserialize, Serialize};

use super::library::Library;
use super::rules::{Features, Platform, Rule, rules_allow};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherits_from: Option<String>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_class: Option<String>,
    /// Formato antiguo (hasta 1.12): argumentos del juego en una cadena.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecraft_arguments: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloads: Option<VersionDownloads>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_version: Option<JavaVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_time: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<ArgEntry>,
    #[serde(default)]
    pub jvm: Vec<ArgEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgEntry {
    Plain(String),
    Conditional { rules: Vec<Rule>, value: OneOrMany },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub total_size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionDownloads {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<Download>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Download {
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    #[serde(default)]
    pub component: Option<String>,
    pub major_version: u32,
}

impl ArgEntry {
    /// Valores de la entrada si sus reglas la permiten.
    pub fn values(&self, platform: &Platform, features: &Features) -> Vec<String> {
        match self {
            ArgEntry::Plain(s) => vec![s.clone()],
            ArgEntry::Conditional { rules, value } => {
                if !rules_allow(rules, platform, features) {
                    return Vec::new();
                }
                match value {
                    OneOrMany::One(s) => vec![s.clone()],
                    OneOrMany::Many(v) => v.clone(),
                }
            }
        }
    }
}

impl VersionInfo {
    /// Versión mayor de Java requerida (8 si el JSON no lo dice: versiones antiguas).
    pub fn java_major(&self) -> u32 {
        self.java_version.as_ref().map_or(8, |j| j.major_version)
    }

    /// Fusiona este perfil (hijo) sobre `parent` (`inheritsFrom`).
    ///
    /// El hijo manda en id, clase principal y tipo. Las librerías del hijo van
    /// primero y sustituyen a las del padre con el mismo grupo, artefacto y
    /// clasificador. Los argumentos se concatenan: primero los del padre.
    pub fn merge_onto(self, parent: VersionInfo) -> VersionInfo {
        let mut libraries = self.libraries;
        let child_keys: Vec<String> = libraries.iter().map(Library::dedup_key).collect();
        libraries.extend(
            parent
                .libraries
                .into_iter()
                .filter(|l| !child_keys.contains(&l.dedup_key())),
        );

        let arguments = match (parent.arguments, self.arguments) {
            (None, None) => None,
            (Some(a), None) | (None, Some(a)) => Some(a),
            (Some(mut p), Some(c)) => {
                p.game.extend(c.game);
                p.jvm.extend(c.jvm);
                Some(p)
            }
        };

        VersionInfo {
            id: self.id,
            inherits_from: parent.inherits_from,
            kind: self.kind.or(parent.kind),
            main_class: self.main_class.or(parent.main_class),
            minecraft_arguments: self.minecraft_arguments.or(parent.minecraft_arguments),
            arguments,
            libraries,
            asset_index: self.asset_index.or(parent.asset_index),
            assets: self.assets.or(parent.assets),
            downloads: self.downloads.or(parent.downloads),
            java_version: self.java_version.or(parent.java_version),
            release_time: self.release_time.or(parent.release_time),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(path: &str) -> VersionInfo {
        let text = std::fs::read_to_string(format!(
            "{}/tests/fixtures/{path}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn lee_jsons_reales() {
        for v in ["1.8.9", "1.12.2", "1.16.5", "1.20.1", "26.2"] {
            let info = load(&format!("mojang/{v}.json"));
            assert_eq!(info.id, v);
            assert!(info.main_class.is_some());
            assert!(info.downloads.as_ref().unwrap().client.is_some());
        }
        assert_eq!(load("mojang/26.2.json").java_major(), 25);
        assert_eq!(load("mojang/1.20.1.json").java_major(), 17);
        assert_eq!(load("mojang/1.8.9.json").java_major(), 8);
    }

    #[test]
    fn fusion_fabric() {
        let vanilla = load("mojang/26.2.json");
        let fabric = load("fabric/fabric-loader-0.19.3-26.2.json");
        let n_vanilla = vanilla.libraries.len();
        let n_fabric = fabric.libraries.len();
        let merged = fabric.merge_onto(vanilla.clone());
        assert_eq!(merged.id, "fabric-loader-0.19.3-26.2");
        assert_eq!(
            merged.main_class.as_deref(),
            Some("net.fabricmc.loader.impl.launch.knot.KnotClient")
        );
        assert_eq!(merged.java_major(), 25);
        assert_eq!(merged.asset_index, vanilla.asset_index);
        assert!(merged.libraries.len() <= n_vanilla + n_fabric);
        assert!(merged.libraries[0].name.starts_with("org.ow2.asm:asm:"));
        let jvm = &merged.arguments.unwrap().jvm;
        assert!(matches!(jvm.last(), Some(ArgEntry::Plain(s)) if s.starts_with("-DFabricMcEmu")));
    }
}
