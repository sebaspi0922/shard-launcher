//! Minecraft Services: token, propiedad del juego, perfil, skins y capas.

use std::path::Path;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::xbox::XboxToken;
use super::{AuthError, Endpoints, join};
use crate::net::{Http, check_status};
use crate::secrets::Secret;
use crate::{Error, Result};

#[derive(Debug, Clone)]
pub struct McToken {
    pub access_token: Secret,
    pub expires_in: u64,
}

#[derive(Deserialize)]
struct LoginResponse {
    access_token: String,
    #[serde(default)]
    expires_in: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// UUID simple (32 hex).
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<Skin>,
    #[serde(default)]
    pub capes: Vec<Cape>,
}

impl Profile {
    pub fn active_skin(&self) -> Option<&Skin> {
        self.skins.iter().find(|s| s.state == "ACTIVE")
    }

    pub fn active_cape(&self) -> Option<&Cape> {
        self.capes.iter().find(|c| c.state == "ACTIVE")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skin {
    pub id: String,
    pub state: String,
    pub url: String,
    /// `CLASSIC` o `SLIM`.
    #[serde(default)]
    pub variant: String,
    #[serde(default)]
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cape {
    pub id: String,
    pub state: String,
    pub url: String,
    #[serde(default)]
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinVariant {
    Classic,
    Slim,
}

impl SkinVariant {
    fn as_str(self) -> &'static str {
        match self {
            SkinVariant::Classic => "classic",
            SkinVariant::Slim => "slim",
        }
    }
}

pub async fn login_with_xbox(http: &Http, ep: &Endpoints, xsts: &XboxToken) -> Result<McToken> {
    let url = join(&ep.minecraft, "authentication/login_with_xbox");
    let body = json!({
        "identityToken": format!("XBL3.0 x={};{}", xsts.uhs, xsts.token.expose()),
    });
    let resp = http.send(|c| c.post(&url).json(&body)).await?;
    if resp.status() == StatusCode::FORBIDDEN {
        return Err(AuthError::AppNotApproved.into());
    }
    let resp = check_status(resp)?;
    let parsed: LoginResponse = resp.json().await?;
    Ok(McToken {
        access_token: Secret::new(parsed.access_token),
        expires_in: parsed.expires_in,
    })
}

/// `true` si la cuenta tiene alguna licencia del juego.
pub async fn owns_game(http: &Http, ep: &Endpoints, token: &Secret) -> Result<bool> {
    #[derive(Deserialize)]
    struct Entitlements {
        #[serde(default)]
        items: Vec<serde_json::Value>,
    }
    let url = join(&ep.minecraft, "entitlements/mcstore");
    let resp = http
        .send(|c| c.get(&url).bearer_auth(token.expose()))
        .await?;
    let parsed: Entitlements = check_status(resp)?.json().await?;
    Ok(!parsed.items.is_empty())
}

/// Perfil del jugador; `None` si la cuenta aún no tiene perfil.
pub async fn profile(http: &Http, ep: &Endpoints, token: &Secret) -> Result<Option<Profile>> {
    let url = join(&ep.minecraft, "minecraft/profile");
    let resp = http
        .send(|c| c.get(&url).bearer_auth(token.expose()))
        .await?;
    if resp.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    Ok(Some(check_status(resp)?.json().await?))
}

pub async fn set_skin_url(
    http: &Http,
    ep: &Endpoints,
    token: &Secret,
    url: &str,
    variant: SkinVariant,
) -> Result<Profile> {
    let endpoint = join(&ep.minecraft, "minecraft/profile/skins");
    let body = json!({ "variant": variant.as_str(), "url": url });
    let resp = http
        .send(|c| c.post(&endpoint).bearer_auth(token.expose()).json(&body))
        .await?;
    Ok(check_status(resp)?.json().await?)
}

pub async fn upload_skin(
    http: &Http,
    ep: &Endpoints,
    token: &Secret,
    file: &Path,
    variant: SkinVariant,
) -> Result<Profile> {
    let bytes = tokio::fs::read(file)
        .await
        .map_err(|e| Error::io(file, e))?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(Error::InvalidInput(format!(
            "{} no es un PNG",
            file.display()
        )));
    }
    let endpoint = join(&ep.minecraft, "minecraft/profile/skins");
    let resp = http
        .send(|c| {
            let part = reqwest::multipart::Part::bytes(bytes.clone())
                .file_name("skin.png")
                .mime_str("image/png")
                .expect("tipo MIME fijo");
            let form = reqwest::multipart::Form::new()
                .text("variant", variant.as_str())
                .part("file", part);
            c.post(&endpoint)
                .bearer_auth(token.expose())
                .multipart(form)
        })
        .await?;
    Ok(check_status(resp)?.json().await?)
}

pub async fn reset_skin(http: &Http, ep: &Endpoints, token: &Secret) -> Result<Profile> {
    let url = join(&ep.minecraft, "minecraft/profile/skins/active");
    let resp = http
        .send(|c| c.delete(&url).bearer_auth(token.expose()))
        .await?;
    Ok(check_status(resp)?.json().await?)
}

pub async fn show_cape(
    http: &Http,
    ep: &Endpoints,
    token: &Secret,
    cape_id: &str,
) -> Result<Profile> {
    let url = join(&ep.minecraft, "minecraft/profile/capes/active");
    let body = json!({ "capeId": cape_id });
    let resp = http
        .send(|c| c.put(&url).bearer_auth(token.expose()).json(&body))
        .await?;
    Ok(check_status(resp)?.json().await?)
}

pub async fn hide_cape(http: &Http, ep: &Endpoints, token: &Secret) -> Result<Profile> {
    let url = join(&ep.minecraft, "minecraft/profile/capes/active");
    let resp = http
        .send(|c| c.delete(&url).bearer_auth(token.expose()))
        .await?;
    Ok(check_status(resp)?.json().await?)
}
