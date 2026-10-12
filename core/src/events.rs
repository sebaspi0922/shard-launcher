//! Bus de eventos del núcleo (docs/arquitectura.md §3.3). La CLI los imprime
//! y la app los recibe por un stream de flutter_rust_bridge.

use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Progress {
        task_id: u64,
        stage: String,
        done: u64,
        total: u64,
        unit: ProgressUnit,
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
    GameLog {
        game_id: u64,
        stream: LogStream,
        line: String,
    },
    GameCrashed {
        game_id: u64,
        instance_id: String,
        crash_report: Option<std::path::PathBuf>,
    },
    GameExited {
        game_id: u64,
        instance_id: String,
        code: Option<i32>,
        seconds_played: u64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProgressUnit {
    Files,
    Bytes,
}

#[derive(Debug, Clone)]
pub struct EventBus {
    tx: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx }
    }

    /// Publica sin bloquear; si no hay suscriptores el evento se descarta.
    pub fn emit(&self, event: Event) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
