//! Instancias, versiones y lanzamiento.

use lcore::auth::AccountKind;
use lcore::instance::{GameVersion, LoaderKind, NewInstance};
use lcore::launch::LaunchOptions;

use crate::api::types::{BridgeError, GameView, InstanceView, VersionView};
use crate::state::{core, on_rt};

fn view(core: &lcore::Core, i: &lcore::instance::Instance) -> InstanceView {
    let dir = core
        .instance_game_dir(&i.id)
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    InstanceView::new(i, dir)
}

pub async fn list_instances() -> Result<Vec<InstanceView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let list = core.list_instances().await?;
        Ok(list.iter().map(|i| view(&core, i)).collect())
    })
    .await
}

pub async fn create_instance(
    name: String,
    mc: String,
    loader: String,
    loader_version: Option<String>,
) -> Result<InstanceView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let loader = LoaderKind::parse(&loader).ok_or_else(|| BridgeError {
            kind: "invalid_input".into(),
            message: format!("loader desconocido: {loader}"),
        })?;
        let i = core
            .create_instance(NewInstance {
                name,
                game: GameVersion {
                    mc,
                    loader,
                    loader_version,
                },
            })
            .await?;
        Ok(view(&core, &i))
    })
    .await
}

/// Cambia memoria, JVM y ventana. `None` en memoria/JVM = valores globales.
pub async fn update_instance_settings(
    id: String,
    name: String,
    memory_mb: Option<u32>,
    jvm_args: Option<Vec<String>>,
    width: Option<u32>,
    height: Option<u32>,
    fullscreen: bool,
) -> Result<InstanceView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let i = core
            .update_instance(&id, |i| {
                i.name = name;
                i.java.memory_mb = memory_mb;
                i.java.jvm_args = jvm_args;
                i.window.width = width;
                i.window.height = height;
                i.window.fullscreen = fullscreen;
            })
            .await?;
        Ok(view(&core, &i))
    })
    .await
}

pub async fn delete_instance(id: String) -> Result<String, BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.delete_instance(&id).await?.trash_id) }).await
}

/// Versiones de Minecraft (solo releases salvo `snapshots`).
pub async fn list_versions(snapshots: bool) -> Result<Vec<VersionView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let m = core.version_manifest().await?;
        Ok(m.versions
            .into_iter()
            .filter(|v| v.kind == "release" || (snapshots && v.kind == "snapshot"))
            .map(|v| VersionView {
                id: v.id,
                kind: v.kind,
            })
            .collect())
    })
    .await
}

pub async fn list_loader_versions(loader: String, mc: String) -> Result<Vec<String>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let kind = LoaderKind::parse(&loader).ok_or_else(|| BridgeError {
            kind: "invalid_input".into(),
            message: format!("loader desconocido: {loader}"),
        })?;
        Ok(core
            .loader_versions(kind, &mc)
            .await?
            .into_iter()
            .map(|v| v.version)
            .collect())
    })
    .await
}

/// Instala lo necesario y lanza. Devuelve al arrancar el proceso; el resto
/// llega por eventos (`GameLog`, `GameExited`…).
pub async fn launch_instance(
    id: String,
    account: Option<String>,
    server: Option<String>,
) -> Result<GameView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let file = core.accounts().await?;
        let supported = || {
            file.accounts
                .iter()
                .filter(|a| a.kind != AccountKind::Shard)
        };
        let find = |id: &str| supported().find(|a| a.id == id);
        let account = account
            .as_deref()
            .and_then(find)
            .or_else(|| file.active.as_deref().and_then(find))
            .or_else(|| supported().next())
            .ok_or_else(|| lcore::Error::AccountNotFound(String::new()))?;
        // El núcleo busca por nombre antes que por ID.
        let resolved = core.find_account(&account.id).await?;
        if resolved.id != account.id || resolved.kind != account.kind {
            return Err(lcore::Error::AccountNotFound(account.id.clone()).into());
        }
        let info = core
            .launch(
                &id,
                LaunchOptions {
                    account: Some(account.id.clone()),
                    server,
                    ..LaunchOptions::default()
                },
            )
            .await?;
        Ok(GameView {
            game_id: info.game_id,
            instance_id: info.instance_id,
            pid: info.pid,
            started_at: info.started_at,
        })
    })
    .await
}

pub async fn running_games() -> Result<Vec<GameView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .running_games()
            .await
            .into_iter()
            .map(|g| GameView {
                game_id: g.game_id,
                instance_id: g.instance_id,
                pid: g.pid,
                started_at: g.started_at,
            })
            .collect())
    })
    .await
}

/// Cambia la versión de Minecraft y el loader. Sin `loader_version` se usará
/// la última compatible al lanzar.
pub async fn change_instance_version(
    id: String,
    mc: String,
    loader: String,
    loader_version: Option<String>,
) -> Result<InstanceView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let kind = LoaderKind::parse(&loader).ok_or_else(|| BridgeError {
            kind: "invalid_input".into(),
            message: format!("loader desconocido: {loader}"),
        })?;
        if mc.trim().is_empty() {
            return Err(BridgeError {
                kind: "invalid_input".into(),
                message: "falta la versión de Minecraft".into(),
            });
        }
        let i = core
            .update_instance(&id, |i| {
                i.game = GameVersion {
                    mc,
                    loader: kind,
                    loader_version: if kind == LoaderKind::Vanilla {
                        None
                    } else {
                        loader_version
                    },
                };
            })
            .await?;
        Ok(view(&core, &i))
    })
    .await
}

/// Instancia en la papelera.
#[derive(Debug, Clone)]
pub struct TrashedView {
    pub trash_id: String,
    pub id: String,
    pub name: String,
    pub mc: String,
    pub loader: String,
    pub playtime_seconds: u64,
    /// Segundos Unix.
    pub deleted_at: i64,
}

pub async fn list_trash() -> Result<Vec<TrashedView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .list_trash()
            .await?
            .into_iter()
            .map(|t| TrashedView {
                trash_id: t.trash_id,
                id: t.instance.id,
                name: t.instance.name,
                mc: t.instance.game.mc,
                loader: t.instance.game.loader.as_str().into(),
                playtime_seconds: t.instance.playtime_seconds,
                deleted_at: t.deleted_at,
            })
            .collect())
    })
    .await
}

/// Recupera una instancia de la papelera (con otro id si el original está ocupado).
pub async fn restore_instance(trash_id: String) -> Result<InstanceView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let i = core.restore_instance(&trash_id).await?;
        Ok(view(&core, &i))
    })
    .await
}

/// Borra definitivamente una instancia de la papelera.
pub async fn purge_instance(trash_id: String) -> Result<(), BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.purge_instance(&trash_id).await?) }).await
}
