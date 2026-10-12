//! Registro de la última partida de cada instancia (`instances/<id>/.last_launch.json`).
//!
//! Sirve para dos cosas: sumar el tiempo de juego de partidas que terminaron sin
//! que el núcleo lo viera (launcher cerrado durante la partida o `--detach`) y
//! que Doctor mire solo los crash reports de la última partida.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::fsutil::{now_secs, read_json, write_json};
use crate::launcher::Core;

const RECORD_FILE: &str = ".last_launch.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaunchRecord {
    pub started_at: i64,
    #[serde(default)]
    pub pid: Option<u32>,
    pub running: bool,
    #[serde(default)]
    pub ended_at: Option<i64>,
}

impl Core {
    pub async fn last_launch(&self, instance_id: &str) -> Result<Option<LaunchRecord>> {
        read_json(&self.instance_dir(instance_id)?.join(RECORD_FILE)).await
    }

    pub(crate) async fn write_launch_record(
        &self,
        instance_id: &str,
        record: &LaunchRecord,
    ) -> Result<()> {
        write_json(&self.instance_dir(instance_id)?.join(RECORD_FILE), record).await
    }

    /// Marca la partida como terminada (el tiempo ya lo sumó quien la vigilaba).
    pub(crate) async fn finish_launch_record(
        &self,
        instance_id: &str,
        pid: Option<u32>,
    ) -> Result<()> {
        if let Some(mut record) = self.last_launch(instance_id).await?
            && record.running
            && record.pid == pid
        {
            record.running = false;
            record.ended_at = Some(now_secs());
            self.write_launch_record(instance_id, &record).await?;
        }
        Ok(())
    }
    /// `true` si hay una partida de la instancia abierta: lanzada por este proceso
    /// o, según `.last_launch.json`, por otro (`--detach`, otra CLI, la app).
    pub async fn is_instance_running(&self, instance_id: &str) -> bool {
        if self
            .inner
            .games
            .lock()
            .await
            .values()
            .any(|g| g.instance_id == instance_id)
        {
            return true;
        }
        let Ok(Some(record)) = self.last_launch(instance_id).await else {
            return false;
        };
        let Ok(game_dir) = self.instance_game_dir(instance_id) else {
            return false;
        };
        record.running && record.pid.is_some_and(|pid| process_alive(pid, &game_dir))
    }

    /// Suma el tiempo de las partidas que quedaron abiertas en el registro y cuyo
    /// proceso ya no existe. El final se estima con la última escritura del log
    /// del juego. Devuelve cuántas se cerraron.
    pub async fn reconcile_sessions(&self) -> Result<usize> {
        let mut closed = 0;
        for instance in self.list_instances().await? {
            let Some(mut record) = self.last_launch(&instance.id).await? else {
                continue;
            };
            if !record.running {
                continue;
            }
            // Las que vigila este proceso las cierra su tarea al terminar.
            if self
                .inner
                .games
                .lock()
                .await
                .values()
                .any(|g| g.instance_id == instance.id)
            {
                continue;
            }
            let game_dir = self.instance_game_dir(&instance.id)?;
            if record.pid.is_some_and(|pid| process_alive(pid, &game_dir)) {
                continue;
            }
            let ended_at = log_mtime(&game_dir.join("logs").join("latest.log"))
                .await
                .filter(|t| *t >= record.started_at)
                .unwrap_or(record.started_at);
            let seconds = (ended_at - record.started_at).max(0) as u64;
            self.update_instance(&instance.id, |i| {
                i.playtime_seconds += seconds;
                i.last_played = Some(ended_at.max(i.last_played.unwrap_or(0)));
            })
            .await?;
            record.running = false;
            record.ended_at = Some(ended_at);
            self.write_launch_record(&instance.id, &record).await?;
            closed += 1;
        }
        Ok(closed)
    }
}

async fn log_mtime(path: &Path) -> Option<i64> {
    let modified = tokio::fs::metadata(path).await.ok()?.modified().ok()?;
    modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

/// `true` si el proceso existe y es el juego de esa instancia (el pid pudo reutilizarse).
fn process_alive(pid: u32, game_dir: &Path) -> bool {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let pid = Pid::from_u32(pid);
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
    );
    let Some(process) = sys.process(pid) else {
        return false;
    };
    let needle = game_dir.as_os_str();
    process.cmd().iter().any(|arg| arg == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Options;
    use crate::instance::{GameVersion, LoaderKind, NewInstance};
    use crate::secrets::MemoryStore;
    use std::sync::Arc;

    #[tokio::test]
    async fn suma_partidas_huerfanas() {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::open(Options {
            data_dir: Some(dir.path().to_path_buf()),
            secrets: Some(Arc::new(MemoryStore::default())),
            offline: true,
            ..Options::default()
        })
        .await
        .unwrap();
        let inst = core
            .create_instance(NewInstance {
                name: "X".into(),
                game: GameVersion {
                    mc: "1.20.1".into(),
                    loader: LoaderKind::Vanilla,
                    loader_version: None,
                },
            })
            .await
            .unwrap();
        // Un pid que no es el juego (el de este test) no cuenta como vivo.
        let started = now_secs() - 600;
        core.write_launch_record(
            &inst.id,
            &LaunchRecord {
                started_at: started,
                pid: Some(std::process::id()),
                running: true,
                ended_at: None,
            },
        )
        .await
        .unwrap();
        let logs = core.instance_game_dir(&inst.id).unwrap().join("logs");
        std::fs::create_dir_all(&logs).unwrap();
        std::fs::write(logs.join("latest.log"), "x").unwrap();

        assert_eq!(core.reconcile_sessions().await.unwrap(), 1);
        let after = core.get_instance(&inst.id).await.unwrap();
        assert!(
            (590..=700).contains(&after.playtime_seconds),
            "{}",
            after.playtime_seconds
        );
        assert!(!core.last_launch(&inst.id).await.unwrap().unwrap().running);
        // Segunda pasada: nada que hacer.
        assert_eq!(core.reconcile_sessions().await.unwrap(), 0);
    }
}
