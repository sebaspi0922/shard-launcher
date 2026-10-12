//! Ajustes globales (`settings.json`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const SETTINGS_SCHEMA: u32 = 1;

/// Variable de entorno que fija el client ID de Azure sin tocar los ajustes.
pub const MSA_CLIENT_ID_ENV: &str = "LCORE_MSA_CLIENT_ID";

/// Client ID de la app de Azure propia, cuando exista (docs/guia-azure.md).
pub const DEFAULT_MSA_CLIENT_ID: Option<&str> = None;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CloseBehavior {
    #[default]
    Ask,
    Tray,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub schema: u32,
    /// Código de idioma (`es`, `en`); `None` = el del sistema.
    pub language: Option<String>,
    /// Client ID de Azure; tiene prioridad sobre el valor compilado.
    pub msa_client_id: Option<String>,
    pub close_behavior: CloseBehavior,
    pub allow_multiple_games: bool,
    pub default_memory_mb: u32,
    pub default_jvm_args: Vec<String>,
    /// Java elegida a mano por versión mayor.
    pub java_overrides: BTreeMap<u32, PathBuf>,
    pub max_concurrent_downloads: usize,
    pub discord_rpc: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: SETTINGS_SCHEMA,
            language: None,
            msa_client_id: None,
            close_behavior: CloseBehavior::default(),
            allow_multiple_games: false,
            default_memory_mb: 4096,
            default_jvm_args: Vec::new(),
            java_overrides: BTreeMap::new(),
            max_concurrent_downloads: 12,
            discord_rpc: true,
        }
    }
}

impl Settings {
    /// Client ID efectivo: variable de entorno, ajustes o valor compilado.
    pub fn effective_msa_client_id(&self) -> Option<String> {
        std::env::var(MSA_CLIENT_ID_ENV)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| self.msa_client_id.clone().filter(|v| !v.trim().is_empty()))
            .or_else(|| DEFAULT_MSA_CLIENT_ID.map(str::to_owned))
            .map(|v| v.trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campos_ausentes_toman_el_valor_por_defecto() {
        let s: Settings = serde_json::from_str(r#"{"allow_multiple_games": true}"#).unwrap();
        assert!(s.allow_multiple_games);
        assert_eq!(s.default_memory_mb, 4096);
        assert_eq!(s.close_behavior, CloseBehavior::Ask);
    }
}
