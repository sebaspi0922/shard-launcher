//! OAuth 2.0 de Microsoft con device code (tenant `consumers`).

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::{AuthError, Endpoints, join};
use crate::Result;
use crate::net::Http;
use crate::secrets::Secret;

pub const SCOPE: &str = "XboxLive.signin offline_access";
const DEVICE_CODE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// Código para que el usuario lo introduzca en `verification_uri`.
#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    #[serde(default)]
    pub message: Option<String>,
    /// Secreto de un solo uso: no se muestra ni se serializa.
    #[serde(skip_serializing)]
    device_code: String,
}

impl std::fmt::Debug for DeviceCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceCode")
            .field("user_code", &self.user_code)
            .field("verification_uri", &self.verification_uri)
            .field("expires_in", &self.expires_in)
            .field("interval", &self.interval)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub struct MsaTokens {
    pub access_token: Secret,
    pub refresh_token: Option<Secret>,
    pub expires_in: u64,
}

#[derive(Deserialize)]
struct TokenOk {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: u64,
}

#[derive(Deserialize)]
struct TokenErr {
    error: String,
    #[serde(default)]
    error_description: String,
}

pub async fn request_device_code(
    http: &Http,
    ep: &Endpoints,
    client_id: &str,
) -> Result<DeviceCode> {
    let url = join(&ep.login, "consumers/oauth2/v2.0/devicecode");
    let resp = http
        .send(|c| {
            c.post(&url)
                .form(&[("client_id", client_id), ("scope", SCOPE)])
        })
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    if status.is_success() {
        serde_json::from_str(&body).map_err(|e| {
            AuthError::Unexpected {
                service: "Microsoft",
                detail: e.to_string(),
            }
            .into()
        })
    } else {
        Err(msa_error(&body, status.as_u16()).into())
    }
}

/// Sondea hasta que el usuario completa el login, lo rechaza o caduca el código.
/// Se cancela soltando el futuro.
pub async fn poll_device_code(
    http: &Http,
    ep: &Endpoints,
    client_id: &str,
    code: &DeviceCode,
) -> Result<MsaTokens> {
    let url = join(&ep.login, "consumers/oauth2/v2.0/token");
    let deadline = Instant::now() + Duration::from_secs(code.expires_in);
    let mut interval = Duration::from_secs(code.interval);
    loop {
        tokio::time::sleep(interval).await;
        if Instant::now() >= deadline {
            return Err(AuthError::DeviceCodeExpired.into());
        }
        let form = [
            ("grant_type", DEVICE_CODE_GRANT),
            ("client_id", client_id),
            ("device_code", code.device_code.as_str()),
        ];
        let resp = http.send(|c| c.post(&url).form(&form)).await?;
        let status = resp.status();
        let body = resp.text().await?;
        if status.is_success() {
            return parse_tokens(&body);
        }
        let err: TokenErr = serde_json::from_str(&body).map_err(|_| AuthError::Unexpected {
            service: "Microsoft",
            detail: format!("HTTP {status}"),
        })?;
        match err.error.as_str() {
            "authorization_pending" => {}
            "slow_down" => interval += Duration::from_secs(5),
            "expired_token" | "code_expired" => return Err(AuthError::DeviceCodeExpired.into()),
            "authorization_declined" | "access_denied" => {
                return Err(AuthError::DeviceCodeDeclined.into());
            }
            _ => {
                return Err(AuthError::Msa {
                    code: err.error,
                    description: first_line(&err.error_description),
                }
                .into());
            }
        }
    }
}

/// Canjea el refresh token. Microsoft puede devolver uno nuevo (rotación).
pub async fn refresh(
    http: &Http,
    ep: &Endpoints,
    client_id: &str,
    refresh_token: &Secret,
) -> Result<MsaTokens> {
    let url = join(&ep.login, "consumers/oauth2/v2.0/token");
    let form = [
        ("grant_type", "refresh_token"),
        ("client_id", client_id),
        ("refresh_token", refresh_token.expose()),
        ("scope", SCOPE),
    ];
    let resp = http.send(|c| c.post(&url).form(&form)).await?;
    let status = resp.status();
    let body = resp.text().await?;
    if status.is_success() {
        return parse_tokens(&body);
    }
    match serde_json::from_str::<TokenErr>(&body) {
        Ok(err) if err.error == "invalid_grant" => Err(AuthError::SessionExpired.into()),
        _ => Err(msa_error(&body, status.as_u16()).into()),
    }
}

fn parse_tokens(body: &str) -> Result<MsaTokens> {
    let ok: TokenOk = serde_json::from_str(body).map_err(|e| AuthError::Unexpected {
        service: "Microsoft",
        detail: e.to_string(),
    })?;
    Ok(MsaTokens {
        access_token: Secret::new(ok.access_token),
        refresh_token: ok.refresh_token.map(Secret::new),
        expires_in: ok.expires_in,
    })
}

fn msa_error(body: &str, status: u16) -> AuthError {
    match serde_json::from_str::<TokenErr>(body) {
        Ok(err) => AuthError::Msa {
            code: err.error,
            description: first_line(&err.error_description),
        },
        Err(_) => AuthError::Unexpected {
            service: "Microsoft",
            detail: format!("HTTP {status}"),
        },
    }
}

/// Las descripciones de Entra traen trazas y marcas de tiempo tras la primera línea.
fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or_default().trim().to_owned()
}
