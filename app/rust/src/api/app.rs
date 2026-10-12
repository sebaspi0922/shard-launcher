//! Arranque del núcleo y stream de eventos.

use std::collections::HashMap;
use std::time::Duration;

use tokio::sync::broadcast::error::RecvError;

use crate::api::types::{log_line, BridgeError, LogLineView, UiEvent};
use crate::frb_generated::StreamSink;
use crate::state::{core, init_core, on_rt};

const LOG_FLUSH: Duration = Duration::from_millis(50);
const LOG_BATCH: usize = 200;

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Abre el núcleo (directorio de datos por defecto si `data_dir` es `None`)
/// y cierra las partidas que terminaron con la app cerrada.
pub async fn start(data_dir: Option<String>) -> Result<String, BridgeError> {
    on_rt(async move {
        let core = init_core(data_dir).await?;
        if let Err(e) = core.reconcile_sessions().await {
            tracing::warn!(error = %e, "no se pudieron revisar las partidas");
        }
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
    let core = core()?;
    let mut rx = core.subscribe();
    crate::state::runtime().spawn(async move {
        let mut pending: HashMap<u64, Vec<LogLineView>> = HashMap::new();
        let mut tick = tokio::time::interval(LOG_FLUSH);
        loop {
            tokio::select! {
                ev = rx.recv() => match ev {
                    Ok(lcore::events::Event::GameLog { game_id, stream, line }) => {
                        let buf = pending.entry(game_id).or_default();
                        buf.push(log_line(stream, line));
                        if buf.len() >= LOG_BATCH {
                            let lines = std::mem::take(buf);
                            if sink.add(UiEvent::GameLog { game_id, lines }).is_err() {
                                break;
                            }
                        }
                    }
                    Ok(other) => {
                        // Antes de un fin de partida, vaciar sus líneas pendientes.
                        flush(&sink, &mut pending);
                        if let Some(ui) = UiEvent::from_core(other) {
                            if sink.add(ui).is_err() {
                                break;
                            }
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
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
