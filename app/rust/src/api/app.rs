//! Arranque del núcleo, stream de eventos e histórico de registros.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use tokio::sync::broadcast::error::RecvError;

use crate::api::types::{BridgeError, LogLineView, UiEvent};
use crate::hub::{hub, HubEvent};
use crate::frb_generated::StreamSink;
use crate::state::{core, init_core, on_rt};

const LOG_FLUSH: Duration = Duration::from_millis(50);
const LOG_BATCH: usize = 200;
/// Cada cuánto corre el planificador de backups automáticos.
const BACKUP_SCHEDULER_EVERY: Duration = Duration::from_secs(15 * 60);

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Abre el núcleo (directorio de datos por defecto si `data_dir` es `None`),
/// cierra las partidas que terminaron con la app cerrada, empieza a guardar el
/// histórico de registros y lanza el planificador de backups.
pub async fn start(data_dir: Option<String>) -> Result<String, BridgeError> {
    on_rt(async move {
        let core = init_core(data_dir).await?;
        if let Err(e) = core.reconcile_sessions().await {
            tracing::warn!(error = %e, "no se pudieron revisar las partidas");
        }
        hub();
        static SCHEDULER: OnceLock<()> = OnceLock::new();
        SCHEDULER.get_or_init(|| {
            core.spawn_backup_scheduler(BACKUP_SCHEDULER_EVERY);
        });
        Ok(core.paths().root().display().to_string())
    })
    .await
}

/// Versión del núcleo.
#[flutter_rust_bridge::frb(sync)]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").into()
}

/// Reenvía los eventos del núcleo a Dart. Las líneas de log se agrupan.
pub async fn subscribe_events(sink: StreamSink<UiEvent>) -> Result<(), BridgeError> {
    core()?;
    let mut rx = hub().tx.subscribe();
    crate::state::runtime().spawn(async move {
        let mut pending: HashMap<u64, Vec<LogLineView>> = HashMap::new();
        let mut tick = tokio::time::interval(LOG_FLUSH);
        loop {
            tokio::select! {
                ev = rx.recv() => match ev {
                    Ok(HubEvent::Log { game_id, line }) => {
                        let buf = pending.entry(game_id).or_default();
                        buf.push(line);
                        if buf.len() >= LOG_BATCH {
                            let lines = std::mem::take(buf);
                            if sink.add(UiEvent::GameLog { game_id, lines }).is_err() {
                                break;
                            }
                        }
                    }
                    Ok(HubEvent::Other(ui)) => {
                        // Antes de un fin de partida, vaciar sus líneas pendientes.
                        if !flush(&sink, &mut pending) || sink.add(ui).is_err() {
                            break;
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        // La UI detecta el hueco por `seq` y pide el histórico.
                        tracing::debug!(n, "eventos omitidos hacia la UI");
                    }
                    Err(RecvError::Closed) => break,
                },
                _ = tick.tick() => {
                    if !flush(&sink, &mut pending) {
                        break;
                    }
                }
            }
        }
    });
    Ok(())
}

/// Envía las líneas acumuladas. `false` si Dart ya no escucha.
fn flush(sink: &StreamSink<UiEvent>, pending: &mut HashMap<u64, Vec<LogLineView>>) -> bool {
    for (game_id, lines) in pending.iter_mut() {
        if lines.is_empty() {
            continue;
        }
        let lines = std::mem::take(lines);
        if sink
            .add(UiEvent::GameLog {
                game_id: *game_id,
                lines,
            })
            .is_err()
        {
            return false;
        }
    }
    true
}

/// Últimas líneas (hasta 5000) de una partida, de la más antigua a la más
/// reciente, con su `seq`. Vacío si la partida no se conoce.
#[flutter_rust_bridge::frb(sync)]
pub fn get_log_backlog(game_id: u64) -> Vec<LogLineView> {
    crate::hub::backlog(game_id)
}

/// Partida más reciente de la instancia de la que hay histórico.
#[flutter_rust_bridge::frb(sync)]
pub fn latest_game_for_instance(instance_id: String) -> Option<u64> {
    crate::hub::latest_game(&instance_id)
}
