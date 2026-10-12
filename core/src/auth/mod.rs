//! Autenticación: Microsoft (device code) → Xbox Live → XSTS → Minecraft
//! Services, y cuentas offline. Referencia: research/05 §2 y docs/guia-azure.md.

pub mod mc;
pub mod msa;
pub mod offline;
pub mod xbox;

use serde::{Deserialize, Serialize};

use crate::secrets::Secret;

/// Bases de los servicios; se sustituyen en las pruebas.
#[derive(Debug, Clone)]
pub struct Endpoints {
    pub login: String,
    pub xbl_user: String,
    pub xsts: String,
    pub minecraft: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            login: "https://login.microsoftonline.com".into(),
            xbl_user: "https://user.auth.xboxlive.com".into(),
            xsts: "https://xsts.auth.xboxlive.com".into(),
            minecraft: "https://api.minecraftservices.com".into(),
        }
    }
}

impl Endpoints {
    /// Todas las bases apuntando al mismo servidor (pruebas con wiremock).
    pub fn all(base: &str) -> Self {
        Self {
            login: base.into(),
            xbl_user: base.into(),
            xsts: base.into(),
            minecraft: base.into(),
        }
    }
}

pub(crate) fn join(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error(
        "no hay client ID de Azure configurado: usa `shard settings set msa-client-id <id>` (docs/guia-azure.md)"
    )]
    MissingClientId,

    #[error("el código de inicio de sesión caducó; vuelve a empezar")]
    DeviceCodeExpired,

    #[error("se rechazó el inicio de sesión")]
    DeviceCodeDeclined,

    #[error("Microsoft respondió {code}: {description}")]
    Msa { code: String, description: String },

    #[error("la sesión de Microsoft caducó o se revocó; inicia sesión de nuevo")]
    SessionExpired,

    #[error("Xbox Live: {message} (XErr {xerr})")]
    Xbox { xerr: u64, message: String },

    #[error(
        "Minecraft rechazó la app de Azure (403): falta la aprobación de Mojang para este client ID (docs/guia-azure.md §2)"
    )]
    AppNotApproved,

    #[error("esta cuenta de Microsoft no tiene Minecraft: Java Edition")]
    NotEntitled,

    #[error(
        "la cuenta tiene el juego pero aún no ha creado su perfil; entra una vez en minecraft.net o en el launcher oficial"
    )]
    NoProfile,

    #[error("no hay sesión guardada para esta cuenta; inicia sesión de nuevo")]
    RefreshTokenMissing,

    #[error("nombre offline no válido: {0:?} (3-16 caracteres: letras, números y _)")]
    InvalidOfflineName(String),

    #[error("operación no disponible para cuentas offline")]
    OfflineAccount,

    #[error("respuesta inesperada de {service}: {detail}")]
    Unexpected {
        service: &'static str,
        detail: String,
    },
}

impl AuthError {
    pub fn kind(&self) -> &'static str {
        match self {
            AuthError::MissingClientId => "auth_missing_client_id",
            AuthError::DeviceCodeExpired => "auth_device_code_expired",
            AuthError::DeviceCodeDeclined => "auth_declined",
            AuthError::Msa { .. } => "auth_msa",
            AuthError::SessionExpired => "auth_session_expired",
            AuthError::Xbox { .. } => "auth_xbox",
            AuthError::AppNotApproved => "auth_app_not_approved",
            AuthError::NotEntitled => "auth_not_entitled",
            AuthError::NoProfile => "auth_no_profile",
            AuthError::RefreshTokenMissing => "auth_refresh_token_missing",
            AuthError::InvalidOfflineName(_) => "auth_invalid_offline_name",
            AuthError::OfflineAccount => "auth_offline_account",
            AuthError::Unexpected { .. } => "auth_unexpected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Msa,
    Offline,
}

/// Datos públicos de una cuenta (`accounts.json`). Sin tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Account {
    /// UUID de Minecraft en forma simple (32 hex).
    pub id: String,
    pub name: String,
    pub kind: AccountKind,
    pub skin_url: Option<String>,
    pub added_at: i64,
    pub last_used: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserType {
    Msa,
    Legacy,
}

impl UserType {
    /// Valor de `${user_type}` en los argumentos del juego.
    pub fn as_arg(self) -> &'static str {
        match self {
            UserType::Msa => "msa",
            UserType::Legacy => "legacy",
        }
    }
}

/// Lo necesario para lanzar el juego con una cuenta.
#[derive(Debug, Clone)]
pub struct Session {
    pub uuid: String,
    pub name: String,
    pub access_token: Secret,
    pub user_type: UserType,
}
