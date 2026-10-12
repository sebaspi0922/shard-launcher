//! Lista de servidores (`servers.json`) y ping.

use lcore::servers::{NewServer, Server, ServerPatch, ServerStatus};

use crate::api::types::BridgeError;
use crate::state::{core, on_rt};

#[derive(Debug, Clone)]
pub struct ServerView {
    pub id: String,
    pub name: String,
    pub address: String,
    pub instance_id: Option<String>,
    pub account_id: Option<String>,
}

impl From<Server> for ServerView {
    fn from(s: Server) -> Self {
        Self {
            id: s.id,
            name: s.name,
            address: s.address,
            instance_id: s.instance_id,
            account_id: s.account_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServerStatusView {
    pub address: String,
    pub online: bool,
    pub latency_ms: u64,
    pub version_name: String,
    pub players_online: u32,
    pub players_max: u32,
    pub sample: Vec<String>,
    /// MOTD sin códigos de color.
    pub motd: String,
    /// PNG en base64 (sin el prefijo `data:`).
    pub favicon_base64: Option<String>,
    pub error: Option<String>,
}

impl From<ServerStatus> for ServerStatusView {
    fn from(s: ServerStatus) -> Self {
        Self {
            address: s.address,
            online: s.online,
            latency_ms: s.latency_ms,
            version_name: s.version_name,
            players_online: s.players_online,
            players_max: s.players_max,
            sample: s.sample,
            motd: s.motd_plain,
            favicon_base64: s.favicon_png_base64.map(|f| match f.split_once("base64,") {
                Some((_, data)) => data.to_owned(),
                None => f,
            }),
            error: s.error,
        }
    }
}

pub async fn list_servers() -> Result<Vec<ServerView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .list_servers()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    })
    .await
}

pub async fn add_server(
    name: String,
    address: String,
    instance_id: Option<String>,
    account_id: Option<String>,
) -> Result<ServerView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .add_server(NewServer {
                name,
                address,
                instance_id,
                account_id,
            })
            .await?
            .into())
    })
    .await
}

/// Sustituye todos los campos editables (`None` quita instancia o cuenta).
pub async fn update_server(
    id: String,
    name: String,
    address: String,
    instance_id: Option<String>,
    account_id: Option<String>,
) -> Result<ServerView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .update_server(
                &id,
                ServerPatch {
                    name: Some(name),
                    address: Some(address),
                    instance_id: Some(instance_id),
                    account_id: Some(account_id),
                },
            )
            .await?
            .into())
    })
    .await
}

pub async fn remove_server(id: String) -> Result<(), BridgeError> {
    let core = core()?;
    on_rt(async move {
        core.remove_server(&id).await?;
        Ok(())
    })
    .await
}

/// Mueve el servidor a `index` y devuelve la lista nueva.
pub async fn move_server(id: String, index: u32) -> Result<Vec<ServerView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .move_server(&id, index as usize)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    })
    .await
}

/// Añade los servidores de `servers.dat` de la instancia. Devuelve cuántos.
pub async fn import_servers_dat(instance_id: String) -> Result<u32, BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.import_servers_dat(&instance_id).await? as u32) }).await
}

/// Estado de varias direcciones (en paralelo, con caché de 30 s).
pub async fn ping_servers(addresses: Vec<String>) -> Result<Vec<ServerStatusView>, BridgeError> {
    on_rt(async move {
        Ok(lcore::servers::ping_servers(&addresses)
            .await
            .into_iter()
            .map(Into::into)
            .collect())
    })
    .await
}
