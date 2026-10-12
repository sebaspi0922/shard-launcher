//! Plataforma actual y reglas `rules` de los JSON de versión.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Os {
    Linux,
    Windows,
    Osx,
}

impl Os {
    /// Nombre en los JSON de Mojang.
    pub fn mojang_name(self) -> &'static str {
        match self {
            Os::Linux => "linux",
            Os::Windows => "windows",
            Os::Osx => "osx",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Arch {
    X86_64,
    X86,
    Aarch64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
    /// Versión del SO para las reglas `os.version` (casi solo macOS antiguos).
    pub os_version: String,
}

impl Platform {
    pub fn current() -> Self {
        let os = if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::Osx
        } else {
            Os::Linux
        };
        let arch = if cfg!(target_arch = "aarch64") {
            Arch::Aarch64
        } else if cfg!(target_arch = "x86") {
            Arch::X86
        } else {
            Arch::X86_64
        };
        Self {
            os,
            arch,
            os_version: String::new(),
        }
    }

    pub fn new(os: Os, arch: Arch) -> Self {
        Self {
            os,
            arch,
            os_version: String::new(),
        }
    }

    /// Valor de `${arch}` en las claves `natives` antiguas.
    pub fn arch_bits(&self) -> &'static str {
        match self.arch {
            Arch::X86 => "32",
            _ => "64",
        }
    }

    pub fn classpath_separator(&self) -> &'static str {
        match self.os {
            Os::Windows => ";",
            _ => ":",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub action: RuleAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<OsRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<BTreeMap<String, bool>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OsRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Funciones activas al lanzar (`has_custom_resolution`, `is_quick_play_multiplayer`…).
pub type Features = BTreeMap<String, bool>;

impl Rule {
    fn matches(&self, platform: &Platform, features: &Features) -> bool {
        if let Some(os) = &self.os {
            if let Some(name) = &os.name {
                // Algunos JSON antiguos usan «macos».
                let name = if name == "macos" {
                    "osx"
                } else {
                    name.as_str()
                };
                if name != platform.os.mojang_name() {
                    return false;
                }
            }
            if let Some(arch) = &os.arch {
                let ok = match arch.as_str() {
                    "x86" => platform.arch == Arch::X86,
                    "x86_64" | "amd64" => platform.arch == Arch::X86_64,
                    "arm64" | "aarch64" => platform.arch == Arch::Aarch64,
                    _ => false,
                };
                if !ok {
                    return false;
                }
            }
            if let Some(version) = &os.version {
                match regex::Regex::new(version) {
                    Ok(re) if re.is_match(&platform.os_version) => {}
                    _ => return false,
                }
            }
        }
        if let Some(wanted) = &self.features {
            for (name, value) in wanted {
                if features.get(name).copied().unwrap_or(false) != *value {
                    return false;
                }
            }
        }
        true
    }
}

/// Sin reglas se permite; si hay, manda la última que coincide.
pub fn rules_allow(rules: &[Rule], platform: &Platform, features: &Features) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        if rule.matches(platform, features) {
            allowed = rule.action == RuleAction::Allow;
        }
    }
    allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(json: &str) -> Vec<Rule> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn regla_permitir_menos_osx() {
        let r = rules(r#"[{"action":"allow"},{"action":"disallow","os":{"name":"osx"}}]"#);
        let f = Features::new();
        assert!(rules_allow(&r, &Platform::new(Os::Linux, Arch::X86_64), &f));
        assert!(rules_allow(
            &r,
            &Platform::new(Os::Windows, Arch::X86_64),
            &f
        ));
        assert!(!rules_allow(&r, &Platform::new(Os::Osx, Arch::Aarch64), &f));
    }

    #[test]
    fn arch_y_features() {
        let x86 = rules(r#"[{"action":"allow","os":{"arch":"x86"}}]"#);
        let f = Features::new();
        assert!(!rules_allow(
            &x86,
            &Platform::new(Os::Windows, Arch::X86_64),
            &f
        ));
        assert!(rules_allow(
            &x86,
            &Platform::new(Os::Windows, Arch::X86),
            &f
        ));

        let demo = rules(r#"[{"action":"allow","features":{"is_demo_user":true}}]"#);
        let linux = Platform::new(Os::Linux, Arch::X86_64);
        assert!(!rules_allow(&demo, &linux, &f));
        let mut on = Features::new();
        on.insert("is_demo_user".into(), true);
        assert!(rules_allow(&demo, &linux, &on));
    }
}
