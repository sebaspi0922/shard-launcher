//! Tipos planos que cruzan a Dart.

use lcore::auth::{Account, AccountKind};
use lcore::events::{Event, LogStream, ProgressUnit};
use lcore::instance::Instance;

/// Error que llega a Dart como excepción. `kind` es estable: la UI traduce por él.
#[derive(Debug, Clone)]
pub struct BridgeError {
    pub kind: String,
    pub message: String,
}

impl BridgeError {
    pub(crate) fn internal(message: String) -> Self {
        Self {
            kind: "internal".into(),
            message,
        }
    }
}

impl From<lcore::Error> for BridgeError {
    fn from(e: lcore::Error) -> Self {
        Self {
            kind: e.kind().into(),
            message: e.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
#[flutter_rust_bridge::frb]
pub struct AccountView {
    pub id: String,
    pub name: String,
    pub microsoft: bool,
    #[frb(default = false)]
    pub shard: bool,
    pub skin_url: Option<String>,
    pub active: bool,
}

impl AccountView {
    pub(crate) fn from_account(a: &Account, active: Option<&str>) -> Self {
        Self {
            id: a.id.clone(),
            name: a.name.clone(),
            microsoft: a.kind == AccountKind::Msa,
            shard: a.kind == AccountKind::Shard,
            skin_url: a.skin_url.clone(),
            active: active == Some(a.id.as_str()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceCodeView {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone)]
pub struct InstanceView {
    pub id: String,
    pub name: String,
    pub mc: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub created_at: i64,
    pub last_played: Option<i64>,
    pub playtime_seconds: u64,
    pub memory_mb: Option<u32>,
    pub jvm_args: Option<Vec<String>>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fullscreen: bool,
    pub game_dir: String,
}

impl InstanceView {
    pub(crate) fn new(i: &Instance, game_dir: String) -> Self {
        Self {
            id: i.id.clone(),
            name: i.name.clone(),
            mc: i.game.mc.clone(),
            loader: i.game.loader.as_str().into(),
            loader_version: i.game.loader_version.clone(),
            created_at: i.created_at,
            last_played: i.last_played,
            playtime_seconds: i.playtime_seconds,
            memory_mb: i.java.memory_mb,
            jvm_args: i.java.jvm_args.clone(),
            width: i.window.width,
            height: i.window.height,
            fullscreen: i.window.fullscreen,
            game_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameView {
    pub game_id: u64,
    pub instance_id: String,
    pub pid: Option<u32>,
    pub started_at: i64,
}

#[derive(Debug, Clone)]
pub struct SettingsView {
    pub language: Option<String>,
    pub close_behavior: String,
    pub allow_multiple_games: bool,
    pub default_memory_mb: u32,
    pub default_jvm_args: Vec<String>,
    pub max_concurrent_downloads: u32,
    pub discord_rpc: bool,
    pub check_updates: bool,
    pub msa_client_id_set: bool,
}

#[derive(Debug, Clone)]
pub struct VersionView {
    pub id: String,
    pub kind: String,
}

/// Eventos del núcleo para la UI.
#[derive(Debug, Clone)]
pub enum UiEvent {
    Progress {
        task_id: u64,
        stage: String,
        done: u64,
        total: u64,
        bytes: bool,
    },
    TaskFinished {
        task_id: u64,
        ok: bool,
        message: Option<String>,
    },
    AccountsChanged,
    GameStarted {
        game_id: u64,
        instance_id: String,
        pid: Option<u32>,
    },
    /// Líneas de log agrupadas para no saturar el canal.
    GameLog {
        game_id: u64,
        lines: Vec<LogLineView>,
    },
    GameCrashed {
        game_id: u64,
        instance_id: String,
        crash_report: Option<String>,
    },
    GameExited {
        game_id: u64,
        instance_id: String,
        code: Option<i32>,
        seconds_played: u64,
    },
}

#[derive(Debug, Clone)]
pub struct LogLineView {
    pub text: String,
    pub stderr: bool,
    /// Número de orden dentro de la partida (detecta huecos y duplicados).
    pub seq: u64,
}

impl UiEvent {
    /// Traduce un evento del núcleo (los de log se agrupan aparte).
    pub(crate) fn from_core(e: Event) -> Option<Self> {
        Some(match e {
            Event::Progress {
                task_id,
                stage,
                done,
                total,
                unit,
            } => UiEvent::Progress {
                task_id,
                stage,
                done,
                total,
                bytes: unit == ProgressUnit::Bytes,
            },
            Event::TaskFinished {
                task_id,
                ok,
                message,
            } => UiEvent::TaskFinished {
                task_id,
                ok,
                message,
            },
            Event::AccountsChanged => UiEvent::AccountsChanged,
            Event::GameStarted {
                game_id,
                instance_id,
                pid,
            } => UiEvent::GameStarted {
                game_id,
                instance_id,
                pid,
            },
            Event::GameLog { .. } => return None,
            Event::GameCrashed {
                game_id,
                instance_id,
                crash_report,
            } => UiEvent::GameCrashed {
                game_id,
                instance_id,
                crash_report: crash_report.map(|p| p.display().to_string()),
            },
            Event::GameExited {
                game_id,
                instance_id,
                code,
                seconds_played,
            } => UiEvent::GameExited {
                game_id,
                instance_id,
                code,
                seconds_played,
            },
        })
    }
}

pub(crate) fn log_line(stream: LogStream, text: String) -> LogLineView {
    LogLineView {
        text,
        stderr: stream == LogStream::Stderr,
        seq: 0,
    }
}
