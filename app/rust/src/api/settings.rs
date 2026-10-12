//! Ajustes globales.

use lcore::settings::CloseBehavior;

use crate::api::types::{BridgeError, SettingsView};
use crate::state::{core, on_rt};

fn view(s: &lcore::settings::Settings) -> SettingsView {
    SettingsView {
        language: s.language.clone(),
        close_behavior: match s.close_behavior {
            CloseBehavior::Ask => "ask",
            CloseBehavior::Tray => "tray",
            CloseBehavior::Quit => "quit",
        }
        .into(),
        allow_multiple_games: s.allow_multiple_games,
        default_memory_mb: s.default_memory_mb,
        default_jvm_args: s.default_jvm_args.clone(),
        max_concurrent_downloads: s.max_concurrent_downloads as u32,
        discord_rpc: s.discord_rpc,
        check_updates: s.check_updates,
        msa_client_id_set: s.effective_msa_client_id().is_some(),
    }
}

pub async fn get_settings() -> Result<SettingsView, BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(view(&core.settings().await)) }).await
}

/// Guarda los ajustes editables desde la UI.
pub async fn save_settings(settings: SettingsView) -> Result<SettingsView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let close = match settings.close_behavior.as_str() {
            "tray" => CloseBehavior::Tray,
            "quit" => CloseBehavior::Quit,
            _ => CloseBehavior::Ask,
        };
        let s = core
            .update_settings(|s| {
                s.language = settings.language;
                s.close_behavior = close;
                s.allow_multiple_games = settings.allow_multiple_games;
                s.default_memory_mb = settings.default_memory_mb.max(512);
                s.default_jvm_args = settings.default_jvm_args;
                s.max_concurrent_downloads =
                    settings.max_concurrent_downloads.clamp(1, 64) as usize;
                s.discord_rpc = settings.discord_rpc;
                s.check_updates = settings.check_updates;
            })
            .await?;
        Ok(view(&s))
    })
    .await
}

/// RAM total del equipo en MB (para el deslizador de memoria).
#[flutter_rust_bridge::frb(sync)]
pub fn system_memory_mb() -> u64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.total_memory() / (1024 * 1024)
}
