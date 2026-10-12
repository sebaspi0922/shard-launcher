//! Copias de seguridad de una instancia.

use lcore::backup::{
    BackupEntry, BackupRequest, RestoreMode, DEFAULT_INTERVAL_HOURS, DEFAULT_MAX_AUTOMATIC,
};
use lcore::instance::BackupSettings;

use crate::api::types::BridgeError;
use crate::state::{core, on_rt};

#[derive(Debug, Clone)]
pub struct BackupView {
    pub id: String,
    /// Segundos Unix.
    pub created_at: i64,
    pub size_bytes: u64,
    pub pinned: bool,
    pub label: Option<String>,
    pub automatic: bool,
}

impl From<BackupEntry> for BackupView {
    fn from(b: BackupEntry) -> Self {
        Self {
            id: b.id,
            created_at: b.created_at,
            size_bytes: b.size_bytes,
            pinned: b.pinned,
            label: b.label,
            automatic: b.automatic,
        }
    }
}

/// Ajustes de copias automáticas con los valores por defecto ya aplicados.
#[derive(Debug, Clone)]
pub struct BackupSettingsView {
    pub auto: bool,
    pub interval_hours: u32,
    pub max_count: u32,
}

fn settings_view(s: &BackupSettings) -> BackupSettingsView {
    BackupSettingsView {
        auto: s.auto,
        interval_hours: s.interval_hours.unwrap_or(DEFAULT_INTERVAL_HOURS),
        max_count: s.max_count.unwrap_or(DEFAULT_MAX_AUTOMATIC),
    }
}

#[derive(Debug, Clone)]
pub struct RestoreView {
    pub files: u64,
    pub notes: Vec<String>,
}

/// Del más reciente al más antiguo.
pub async fn list_backups(instance_id: String) -> Result<Vec<BackupView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .list_backups(&instance_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    })
    .await
}

/// Crea una copia manual. Falla si el juego de la instancia está abierto. El
/// progreso llega por eventos con la etapa `backup`.
pub async fn create_backup(
    instance_id: String,
    label: Option<String>,
    pinned: bool,
) -> Result<BackupView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .create_backup(
                &instance_id,
                BackupRequest {
                    label,
                    pinned,
                    automatic: false,
                },
            )
            .await?
            .into())
    })
    .await
}

pub async fn delete_backup(instance_id: String, backup_id: String) -> Result<(), BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.delete_backup(&instance_id, &backup_id).await?) }).await
}

pub async fn set_backup_pinned(
    instance_id: String,
    backup_id: String,
    pinned: bool,
) -> Result<BackupView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .set_backup_pinned(&instance_id, &backup_id, pinned)
            .await?
            .into())
    })
    .await
}

/// `None` o vacía quita la etiqueta.
pub async fn set_backup_label(
    instance_id: String,
    backup_id: String,
    label: Option<String>,
) -> Result<BackupView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .set_backup_label(&instance_id, &backup_id, label)
            .await?
            .into())
    })
    .await
}

/// Restaura todo `game/` o solo `saves/`. Falla si el juego está abierto.
pub async fn restore_backup(
    instance_id: String,
    backup_id: String,
    worlds_only: bool,
) -> Result<RestoreView, BridgeError> {
    let core = core()?;
    let mode = if worlds_only {
        RestoreMode::WorldsOnly
    } else {
        RestoreMode::Full
    };
    on_rt(async move {
        let r = core.restore_backup(&instance_id, &backup_id, mode).await?;
        Ok(RestoreView {
            files: r.files,
            notes: r.notes,
        })
    })
    .await
}

pub async fn get_backup_settings(instance_id: String) -> Result<BackupSettingsView, BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(settings_view(&core.get_instance(&instance_id).await?.backups)) }).await
}

pub async fn update_backup_settings(
    instance_id: String,
    auto: bool,
    interval_hours: u32,
    max_count: u32,
) -> Result<BackupSettingsView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let s = core
            .update_backup_settings(
                &instance_id,
                BackupSettings {
                    auto,
                    interval_hours: Some(interval_hours.max(1)),
                    max_count: Some(max_count.max(1)),
                },
            )
            .await?;
        Ok(settings_view(&s))
    })
    .await
}
