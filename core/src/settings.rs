//! Ajustes globales (`settings.json`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Dominio de la comunidad pendiente de confirmar.
pub const DEFAULT_YGGDRASIL_BASE_URL: &str = "https://auth.sebaspi.com/authlib-injector";

pub const SETTINGS_SCHEMA: u32 = 1;

/// Obsoleto: el login premium usa SISU y no consulta esta variable. Se conserva
/// para que configuraciones y llamadas antiguas sigan compilando.
pub const MSA_CLIENT_ID_ENV: &str = "SHARD_MSA_CLIENT_ID";

/// Obsoleto: SISU usa la aplicación del launcher oficial, sin client ID de Azure.
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
    /// Obsoleto: ya no se usa para el login SISU. Se conserva al leer y
    /// escribir `settings.json` para no romper instalaciones existentes.
    pub msa_client_id: Option<String>,
    /// Base de Yggdrasil para las nuevas cuentas Shard.
    pub yggdrasil_base_url: Option<String>,
    pub close_behavior: CloseBehavior,
    pub allow_multiple_games: bool,
    pub default_memory_mb: u32,
    pub default_jvm_args: Vec<String>,
    /// Java elegida a mano por versión mayor.
    pub java_overrides: BTreeMap<u32, PathBuf>,
    pub max_concurrent_downloads: usize,
    pub discord_rpc: bool,
    /// La app busca versiones nuevas al arrancar y cada pocas horas.
    pub check_updates: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: SETTINGS_SCHEMA,
            language: None,
            msa_client_id: None,
            yggdrasil_base_url: None,
            close_behavior: CloseBehavior::default(),
            allow_multiple_games: false,
            default_memory_mb: 4096,
            default_jvm_args: Vec::new(),
            java_overrides: BTreeMap::new(),
            max_concurrent_downloads: 12,
            discord_rpc: true,
            check_updates: true,
        }
    }
}

impl Settings {
    /// Obsoleto: solo queda para compatibilidad con el bridge antiguo. El
    /// carril premium no llama a este método.
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
