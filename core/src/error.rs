//! Error común del núcleo. Cada variante tiene un `kind` estable que la UI usa
//! para traducir el mensaje y Doctor para reaccionar.

use std::path::PathBuf;

use crate::auth::AuthError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error de E/S en {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("JSON inválido en {context}: {source}")]
    Json {
        context: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("error de red: {0}")]
    Network(#[from] reqwest::Error),

    #[error("HTTP {status} en {url}")]
    HttpStatus { status: u16, url: String },

    #[error("sin conexión y sin copia en caché de {url}")]
    Offline { url: String },

    #[error("ruta no válida: {0}")]
    InvalidPath(String),

    #[error("almacén de secretos: {0}")]
    Secrets(String),

    #[error(transparent)]
    Auth(#[from] AuthError),

    #[error("el archivo descargado de {url} no coincide (esperado {expected}, obtenido {actual})")]
    HashMismatch {
        url: String,
        expected: String,
        actual: String,
    },

    #[error("la versión {0} no existe")]
    VersionNotFound(String),

    #[error("hace falta Java {major}: instálala con `shard java install {major}`")]
    JavaRequired { major: u32 },

    #[error("ya hay una partida en marcha (activa «permitir varias partidas» para lanzar otra)")]
    GameAlreadyRunning,

    #[error("no existe la instancia {0}")]
    InstanceNotFound(String),

    #[error("no existe la cuenta {0}")]
    AccountNotFound(String),

    #[error(
        "falta la clave de API de CurseForge: guárdala con `shard settings set-curseforge-key`"
    )]
    CurseForgeKeyMissing,

    #[error("{0}")]
    InvalidInput(String),
}

impl Error {
    /// Identificador estable del tipo de error.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Io { .. } => "io",
            Error::Json { .. } => "json",
            Error::Network(_) => "network",
            Error::HttpStatus { .. } => "http_status",
            Error::Offline { .. } => "offline",
            Error::InvalidPath(_) => "invalid_path",
            Error::Secrets(_) => "secrets",
            Error::Auth(e) => e.kind(),
            Error::HashMismatch { .. } => "hash_mismatch",
            Error::VersionNotFound(_) => "version_not_found",
            Error::JavaRequired { .. } => "java_required",
            Error::GameAlreadyRunning => "game_already_running",
            Error::InstanceNotFound(_) => "instance_not_found",
            Error::AccountNotFound(_) => "account_not_found",
            Error::CurseForgeKeyMissing => "curseforge_key_missing",
            Error::InvalidInput(_) => "invalid_input",
        }
    }

    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn json(context: impl Into<String>, source: serde_json::Error) -> Self {
        Error::Json {
            context: context.into(),
            source,
        }
    }
}
