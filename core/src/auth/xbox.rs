//! Xbox Live (XBL) y XSTS para Minecraft Services.

use serde::Deserialize;
use serde_json::json;

use super::{AuthError, Endpoints, join};
use crate::Result;
use crate::net::Http;
use crate::secrets::Secret;

#[derive(Debug, Clone)]
pub struct XboxToken {
    pub token: Secret,
    /// User hash (`DisplayClaims.xui[0].uhs`).
    pub uhs: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XboxResponse {
    token: String,
    display_claims: DisplayClaims,
}

#[derive(Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Deserialize)]
struct XboxErrorBody {
    #[serde(rename = "XErr")]
    xerr: u64,
}

pub async fn authenticate_user(
    http: &Http,
    ep: &Endpoints,
    msa_access_token: &Secret,
) -> Result<XboxToken> {
    let body = json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={}", msa_access_token.expose()),
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT",
    });
    post(http, &join(&ep.xbl_user, "user/authenticate"), &body).await
}

pub async fn authorize_xsts(http: &Http, ep: &Endpoints, xbl: &XboxToken) -> Result<XboxToken> {
    let body = json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl.token.expose()],
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT",
    });
    post(http, &join(&ep.xsts, "xsts/authorize"), &body).await
}

async fn post(http: &Http, url: &str, body: &serde_json::Value) -> Result<XboxToken> {
    let resp = http
        .send(|c| c.post(url).header("Accept", "application/json").json(body))
        .await?;
    let status = resp.status();
    let text = resp.text().await?;
    if status.is_success() {
        let parsed: XboxResponse =
            serde_json::from_str(&text).map_err(|e| AuthError::Unexpected {
                service: "Xbox Live",
                detail: e.to_string(),
            })?;
        let uhs = parsed
            .display_claims
            .xui
            .into_iter()
            .next()
            .map(|x| x.uhs)
            .ok_or(AuthError::Unexpected {
                service: "Xbox Live",
                detail: "respuesta sin user hash".into(),
            })?;
        return Ok(XboxToken {
            token: Secret::new(parsed.token),
            uhs,
        });
    }
    if let Ok(err) = serde_json::from_str::<XboxErrorBody>(&text) {
        return Err(AuthError::Xbox {
            xerr: err.xerr,
            message: xerr_message(err.xerr).into(),
        }
        .into());
    }
    Err(AuthError::Unexpected {
        service: "Xbox Live",
        detail: format!("HTTP {status}"),
    }
    .into())
}

/// Explicación de los XErr conocidos (minecraft.wiki, «Microsoft authentication»).
pub fn xerr_message(xerr: u64) -> &'static str {
    match xerr {
        2148916227 => "la cuenta está bloqueada en Xbox",
        2148916233 => "la cuenta no tiene perfil de Xbox; créalo entrando una vez en xbox.com",
        2148916235 => "Xbox Live no está disponible en el país de la cuenta",
        2148916236 | 2148916237 => "la cuenta necesita verificación de adulto en xbox.com",
        2148916238 => "cuenta de menor: un adulto debe añadirla a una familia de Microsoft",
        _ => "error de Xbox Live",
    }
}
