//! Skins y capas de una cuenta de Microsoft.

use lcore::auth::mc::{Profile, SkinVariant};

use crate::api::types::BridgeError;
use crate::state::{core, on_rt};

#[derive(Debug, Clone)]
pub struct SkinView {
    pub id: String,
    /// `ACTIVE` o `INACTIVE`.
    pub state: String,
    pub url: String,
    /// `true` = brazos delgados (`SLIM`).
    pub slim: bool,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CapeView {
    pub id: String,
    pub active: bool,
    pub url: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProfileView {
    pub id: String,
    pub name: String,
    pub skins: Vec<SkinView>,
    pub capes: Vec<CapeView>,
}

impl From<Profile> for ProfileView {
    fn from(p: Profile) -> Self {
        Self {
            id: p.id,
            name: p.name,
            skins: p
                .skins
                .into_iter()
                .map(|s| SkinView {
                    id: s.id,
                    state: s.state,
                    url: s.url,
                    slim: s.variant.eq_ignore_ascii_case("slim"),
                    alias: s.alias,
                })
                .collect(),
            capes: p
                .capes
                .into_iter()
                .map(|c| CapeView {
                    id: c.id,
                    active: c.state == "ACTIVE",
                    url: c.url,
                    alias: c.alias,
                })
                .collect(),
        }
    }
}

fn variant(slim: bool) -> SkinVariant {
    if slim {
        SkinVariant::Slim
    } else {
        SkinVariant::Classic
    }
}

/// Perfil (skins y capas) de la cuenta `account` o de la activa.
pub async fn get_profile(account: Option<String>) -> Result<ProfileView, BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.profile(account.as_deref()).await?.into()) }).await
}

/// Usa como skin la imagen de una URL pública.
pub async fn set_skin_url(
    account: Option<String>,
    url: String,
    slim: bool,
) -> Result<ProfileView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .set_skin_url(account.as_deref(), &url, variant(slim))
            .await?
            .into())
    })
    .await
}

/// Sube un PNG local como skin.
pub async fn upload_skin(
    account: Option<String>,
    path: String,
    slim: bool,
) -> Result<ProfileView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        Ok(core
            .upload_skin(account.as_deref(), std::path::Path::new(&path), variant(slim))
            .await?
            .into())
    })
    .await
}

/// Vuelve a la skin por defecto.
pub async fn reset_skin(account: Option<String>) -> Result<ProfileView, BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.reset_skin(account.as_deref()).await?.into()) }).await
}

pub async fn show_cape(account: Option<String>, cape_id: String) -> Result<ProfileView, BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.show_cape(account.as_deref(), &cape_id).await?.into()) }).await
}

pub async fn hide_cape(account: Option<String>) -> Result<ProfileView, BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.hide_cape(account.as_deref()).await?.into()) }).await
}
