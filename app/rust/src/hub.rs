//! Concentrador de eventos: una sola tarea escucha el núcleo, numera las líneas
//! de log y guarda el anillo de cada partida. Los suscriptores de Dart
//! (`api::app::subscribe_events`) leen de aquí.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

use tokio::sync::broadcast::{self, error::RecvError};

use crate::api::types::{log_line, LogLineView, UiEvent};
use crate::state::core;

/// Líneas que se guardan por partida (igual que en el núcleo).
const LOG_CAPACITY: usize = lcore::logs::DEFAULT_LOG_CAPACITY;

/// Evento ya procesado por el concentrador.
#[derive(Clone)]
pub(crate) enum HubEvent {
    Log { game_id: u64, line: LogLineView },
    Other(UiEvent),
}

/// Líneas recientes de una partida.
struct GameLog {
    instance_id: Option<String>,
    lines: VecDeque<LogLineView>,
    next_seq: u64,
}

/// Anillos por partida. Por instancia solo se conserva su partida más reciente.
#[derive(Default)]
pub(crate) struct LogStore {
    games: HashMap<u64, GameLog>,
    latest: HashMap<String, u64>,
    capacity: usize,
}

impl LogStore {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            ..Self::default()
        }
    }

    /// Partida nueva: descarta la anterior de la misma instancia.
    pub(crate) fn started(&mut self, game_id: u64, instance_id: &str) {
        if let Some(old) = self.latest.insert(instance_id.to_owned(), game_id) {
            if old != game_id {
                self.games.remove(&old);
            }
        }
        self.games
            .entry(game_id)
            .or_insert_with(|| GameLog {
                instance_id: None,
                lines: VecDeque::new(),
                next_seq: 0,
            })
            .instance_id = Some(instance_id.to_owned());
    }

    /// Guarda la línea con su número de orden y devuelve la copia numerada.
    pub(crate) fn push(&mut self, game_id: u64, mut line: LogLineView) -> LogLineView {
        let capacity = self.capacity;
        let game = self.games.entry(game_id).or_insert_with(|| GameLog {
            instance_id: None,
            lines: VecDeque::new(),
            next_seq: 0,
        });
        line.seq = game.next_seq;
        game.next_seq += 1;
        if game.lines.len() == capacity {
            game.lines.pop_front();
        }
        game.lines.push_back(line.clone());
        line
    }

    pub(crate) fn backlog(&self, game_id: u64) -> Vec<LogLineView> {
        self.games
            .get(&game_id)
            .map(|g| g.lines.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub(crate) fn latest_game(&self, instance_id: &str) -> Option<u64> {
        self.latest.get(instance_id).copied()
    }

    #[cfg(test)]
    fn instance_of(&self, game_id: u64) -> Option<&str> {
        self.games.get(&game_id)?.instance_id.as_deref()
    }
}

pub(crate) struct Hub {
    pub(crate) tx: broadcast::Sender<HubEvent>,
    logs: Mutex<LogStore>,
}

static HUB: OnceLock<Hub> = OnceLock::new();

/// Crea el concentrador la primera vez (requiere el núcleo iniciado).
pub(crate) fn hub() -> &'static Hub {
    HUB.get_or_init(|| {
        let (tx, _) = broadcast::channel(4096);
        let hub = Hub {
            tx,
            logs: Mutex::new(LogStore::new(LOG_CAPACITY)),
        };
        if let Ok(core) = core() {
            let mut rx = core.subscribe();
            crate::state::runtime().spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(ev) => {
                            let Some(hub) = HUB.get() else { continue };
                            hub.ingest(ev);
                        }
                        Err(RecvError::Lagged(n)) => {
                            tracing::debug!(n, "eventos del núcleo omitidos");
                        }
                        Err(RecvError::Closed) => break,
                    }
                }
            });
        }
        hub
    })
}

impl Hub {
    fn ingest(&self, ev: lcore::events::Event) {
        use lcore::events::Event;
        let out = match ev {
            Event::GameLog {
                game_id,
                stream,
                line,
            } => {
                let line = self
                    .logs
                    .lock()
                    .expect("candado de logs")
                    .push(game_id, log_line(stream, line));
                HubEvent::Log { game_id, line }
            }
            other => {
                if let Event::GameStarted {
                    game_id,
                    instance_id,
                    ..
                } = &other
                {
                    self.logs
                        .lock()
                        .expect("candado de logs")
                        .started(*game_id, instance_id);
                }
                match UiEvent::from_core(other) {
                    Some(ui) => HubEvent::Other(ui),
                    None => return,
                }
            }
        };
        // Sin suscriptores el evento se descarta (el anillo ya lo guardó).
        let _ = self.tx.send(out);
    }
}


pub(crate) fn backlog(game_id: u64) -> Vec<LogLineView> {
    HUB.get()
        .map(|h| h.logs.lock().expect("candado de logs").backlog(game_id))
        .unwrap_or_default()
}

pub(crate) fn latest_game(instance_id: &str) -> Option<u64> {
    HUB.get().and_then(|h| h.logs.lock().expect("candado de logs").latest_game(instance_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(t: &str) -> LogLineView {
        LogLineView {
            text: t.into(),
            stderr: false,
            seq: 0,
        }
    }

    #[test]
    fn el_anillo_numera_y_se_acota() {
        let mut s = LogStore::new(3);
        s.started(7, "a");
        for i in 0..5 {
            let l = s.push(7, line(&format!("l{i}")));
            assert_eq!(l.seq, i);
        }
        let b = s.backlog(7);
        assert_eq!(b.len(), 3);
        assert_eq!(b[0].text, "l2");
        assert_eq!(b[2].seq, 4);
        assert_eq!(s.instance_of(7), Some("a"));
    }

    #[test]
    fn una_partida_nueva_descarta_la_anterior_de_la_instancia() {
        let mut s = LogStore::new(10);
        s.started(1, "a");
        s.push(1, line("x"));
        s.started(2, "b");
        s.push(2, line("y"));
        s.started(3, "a");
        assert!(s.backlog(1).is_empty());
        assert_eq!(s.backlog(2).len(), 1);
        assert_eq!(s.latest_game("a"), Some(3));
        assert_eq!(s.latest_game("b"), Some(2));
    }
}
