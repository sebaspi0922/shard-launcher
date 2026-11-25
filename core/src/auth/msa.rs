//! Adaptador del flujo SISU del launcher oficial mediante `minecraft-auth`.
//!
//! El crate espera con el device code dentro de `login_device_code`. El login
//! se ejecuta en una tarea para conservar los dos pasos de la API de Shard.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use minecraft_auth::Error as SisuError;
use minecraft_auth::java::JavaAuthManager;
use minecraft_auth::msa::{MsaApplicationConfig, MsaDeviceCode};
use tokio::sync::{Mutex, oneshot};
use tokio::task::{AbortHandle, JoinHandle};

use super::{AuthError, xbox};
use crate::{Error, Result};

/// Código para que el usuario lo introduzca en `verification_uri`.
///
/// La tarea asociada contiene el `device_code` y no sale de Rust. Soltar la
/// última copia de este valor cancela el sondeo.
#[derive(Clone)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: Option<String>,
    pending: Arc<PendingLogin>,
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

struct PendingLogin {
    task: Mutex<Option<JoinHandle<Result<JavaAuthManager, SisuError>>>>,
    abort: AbortHandle,
}

impl PendingLogin {
    fn new(task: JoinHandle<Result<JavaAuthManager, SisuError>>) -> Self {
        Self {
            abort: task.abort_handle(),
            task: Mutex::new(Some(task)),
        }
    }

    async fn complete(&self) -> Result<JavaAuthManager> {
        let task = self.task.lock().await.take().ok_or_else(|| {
            Error::InvalidInput("este código de inicio de sesión ya se utilizó".into())
        })?;
        match task.await {
            Ok(Ok(manager)) => Ok(manager),
            Ok(Err(error)) => Err(map_error(error)),
            Err(error) if error.is_cancelled() => Err(AuthError::DeviceCodeDeclined.into()),
            Err(error) => Err(AuthError::Unexpected {
                service: "Microsoft",
                detail: format!("la tarea de inicio de sesión falló: {error}"),
            }
            .into()),
        }
    }
}

impl Drop for PendingLogin {
    fn drop(&mut self) {
        self.abort.abort();
    }
}

#[derive(Debug)]
struct DeviceCodeDetails {
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

impl From<&MsaDeviceCode> for DeviceCodeDetails {
    fn from(code: &MsaDeviceCode) -> Self {
        Self {
            user_code: code.user_code.clone(),
            verification_uri: code.verification_uri.clone(),
            expires_in: (code.expire_time_ms.saturating_sub(now_ms()).max(0) as u64).div_ceil(1000),
            interval: (code.interval_ms.max(0) as u64).div_ceil(1000),
        }
    }
}

/// Empieza el login con la aplicación y el dispositivo Win32 del launcher
/// oficial. Devuelve en cuanto Microsoft entrega el código visible.
pub async fn begin_login(client: reqwest::Client) -> Result<DeviceCode> {
    let (code_tx, code_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        JavaAuthManager::builder(client)
            .login_device_code(|code| {
                let _ = code_tx.send(DeviceCodeDetails::from(code));
            })
            .await
    });
    let pending = Arc::new(PendingLogin::new(task));

    match code_rx.await {
        Ok(details) => Ok(DeviceCode {
            user_code: details.user_code,
            verification_uri: details.verification_uri,
            expires_in: details.expires_in,
            interval: details.interval,
            message: None,
            pending,
        }),
        // Si no llegó un código, la tarea terminó: devuelve su error real.
        Err(_) => match pending.complete().await {
            Ok(_) => Err(AuthError::Unexpected {
                service: "Microsoft",
                detail: "el login terminó sin entregar un código".into(),
            }
            .into()),
            Err(error) => Err(error),
        },
    }
}

pub(crate) async fn complete_login(code: &DeviceCode) -> Result<JavaAuthManager> {
    code.pending.complete().await
}

/// Restaura una sesión completa, incluidas las claves del dispositivo. Una
/// configuración distinta no se reutiliza: los tokens de otra app no sirven
/// para SISU y el usuario debe volver a iniciar sesión.
pub fn from_saved_json(client: reqwest::Client, raw: &str) -> Result<JavaAuthManager> {
    let json: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| AuthError::SessionExpired)?;
    let official = MsaApplicationConfig::java_default();
    let config = json.get("application_config");
    let is_official = config
        .and_then(|c| c.get("client_id"))
        .and_then(serde_json::Value::as_str)
        == Some(official.client_id.as_str())
        && config
            .and_then(|c| c.get("scope"))
            .and_then(serde_json::Value::as_str)
            == Some(official.scope.as_str());
    if !is_official {
        return Err(AuthError::LegacyMsaSession.into());
    }
    JavaAuthManager::from_json(client, &json).map_err(|error| match error {
        SisuError::Json(_) | SisuError::InvalidState(_) => AuthError::SessionExpired.into(),
        other => map_error(other),
    })
}

/// Serializa tokens y claves del dispositivo para guardarlos en el keyring.
pub async fn to_saved_json(manager: &JavaAuthManager) -> Result<String> {
    let json = manager.to_json().await.map_err(map_error)?;
    serde_json::to_string(&json).map_err(|e| Error::json("sesión SISU", e))
}

pub(crate) fn map_error(error: SisuError) -> Error {
    match error {
        SisuError::Http(error) => Error::Network(error),
        SisuError::Json(error) => AuthError::Unexpected {
            service: "Microsoft",
            detail: error.to_string(),
        }
        .into(),
        SisuError::Msa {
            error,
            error_description,
            ..
        } => match error.as_str() {
            "authorization_declined" | "access_denied" => AuthError::DeviceCodeDeclined.into(),
            "expired_token" | "code_expired" => AuthError::DeviceCodeExpired.into(),
            "invalid_grant" => AuthError::SessionExpired.into(),
            _ => AuthError::Msa {
                code: error,
                description: first_line(&error_description),
            }
            .into(),
        },
        SisuError::Xbl { error_code, .. } => AuthError::Xbox {
            xerr: error_code,
            message: xbox::xerr_message(error_code).into(),
        }
        .into(),
        SisuError::MinecraftServices { status, .. }
            if status == reqwest::StatusCode::UNAUTHORIZED =>
        {
            AuthError::SessionExpired.into()
        }
        SisuError::MinecraftServices {
            status,
            error,
            error_message,
        } => AuthError::Unexpected {
            service: "Minecraft",
            detail: format!("HTTP {status}: {error}: {}", first_line(&error_message)),
        }
        .into(),
        SisuError::ProfileNotFound => AuthError::NoProfile.into(),
        SisuError::UnexpectedResponse { endpoint, reason } => AuthError::Unexpected {
            service: endpoint,
            detail: reason,
        }
        .into(),
        SisuError::DeviceCodeTimedOut => AuthError::DeviceCodeExpired.into(),
        SisuError::Cancelled => AuthError::DeviceCodeDeclined.into(),
        SisuError::Webview(detail) => AuthError::Unexpected {
            service: "Microsoft",
            detail,
        }
        .into(),
        SisuError::InvalidState(_) => AuthError::SessionExpired.into(),
    }
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or_default().trim().to_owned()
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("el reloj del sistema es anterior al epoch")
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use minecraft_auth::msa::MsaToken;
    use reqwest::StatusCode;

    use super::*;

    fn manager() -> JavaAuthManager {
        JavaAuthManager::builder(reqwest::Client::new()).login_msa_token(MsaToken {
            expire_time_ms: now_ms() + 60_000,
            access_token: "ACCESS_DE_PRUEBA".into(),
            refresh_token: Some("REFRESH_DE_PRUEBA".into()),
        })
    }

    #[tokio::test]
    async fn sesion_serializada_conserva_configuracion_tokens_y_claves() {
        let json = to_saved_json(&manager()).await.unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["_save_version"], 1);
        assert_eq!(
            value["application_config"]["client_id"],
            MsaApplicationConfig::java_default().client_id
        );
        assert_eq!(value["device_type"], "Win32");
        assert_eq!(value["msa_token"]["refresh_token"], "REFRESH_DE_PRUEBA");
        assert!(value.get("device_key_pair").is_some());
        assert!(value.get("device_id").is_some());

        let restored = from_saved_json(reqwest::Client::new(), &json).unwrap();
        let roundtrip = to_saved_json(&restored).await.unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json).unwrap(),
            serde_json::from_str::<serde_json::Value>(&roundtrip).unwrap()
        );
    }

    #[tokio::test]
    async fn sesion_de_otra_app_o_invalida_pide_relogin() {
        let mut value = manager().to_json().await.unwrap();
        value["application_config"]["client_id"] =
            serde_json::Value::String("00000000-0000-0000-0000-000000000001".into());
        let error = from_saved_json(reqwest::Client::new(), &value.to_string())
            .err()
            .expect("una app distinta debe pedir re-login");
        assert!(matches!(error, Error::Auth(AuthError::LegacyMsaSession)));
        assert_eq!(error.kind(), "auth_session_expired");

        let error = from_saved_json(reqwest::Client::new(), "not json")
            .err()
            .expect("JSON inválido debe pedir re-login");
        assert!(matches!(error, Error::Auth(AuthError::SessionExpired)));
    }

    #[test]
    fn errores_sisu_conservan_la_accion_esperada() {
        let cases = [
            (SisuError::DeviceCodeTimedOut, "auth_device_code_expired"),
            (SisuError::Cancelled, "auth_declined"),
            (
                SisuError::Msa {
                    status: StatusCode::BAD_REQUEST,
                    error: "invalid_grant".into(),
                    error_description: "revoked".into(),
                },
                "auth_session_expired",
            ),
            (
                SisuError::Msa {
                    status: StatusCode::BAD_REQUEST,
                    error: "authorization_declined".into(),
                    error_description: "declined".into(),
                },
                "auth_declined",
            ),
            (SisuError::ProfileNotFound, "auth_no_profile"),
            (
                SisuError::Xbl {
                    status: StatusCode::UNAUTHORIZED,
                    error_code: 2_148_916_233,
                    name: "XboxLive".into(),
                    message: "no profile".into(),
                },
                "auth_xbox",
            ),
        ];
        for (error, kind) in cases {
            assert_eq!(map_error(error).kind(), kind);
        }
    }
}
