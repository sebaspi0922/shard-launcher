//! Cuentas de Microsoft y offline.

use std::sync::Mutex;

use lcore::auth::msa::DeviceCode;

use crate::api::types::{AccountView, BridgeError, DeviceCodeView};
use crate::state::{core, on_rt};

/// Código del login en curso; el secreto `device_code` no sale de Rust.
static PENDING: Mutex<Option<DeviceCode>> = Mutex::new(None);

pub async fn list_accounts() -> Result<Vec<AccountView>, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let file = core.accounts().await?;
        let active = file.active.as_deref();
        Ok(file
            .accounts
            .iter()
            .map(|a| AccountView::from_account(a, active))
            .collect())
    })
    .await
}

/// Paso 1: pide el código que el usuario introduce en `verification_uri`.
pub async fn begin_microsoft_login() -> Result<DeviceCodeView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let code = core.begin_msa_login().await?;
        let view = DeviceCodeView {
            user_code: code.user_code.clone(),
            verification_uri: code.verification_uri.clone(),
            expires_in: code.expires_in,
        };
        *PENDING.lock().expect("candado del login") = Some(code);
        Ok(view)
    })
    .await
}

/// Paso 2: espera a que el usuario termine. Se cancela con `cancel_microsoft_login`.
pub async fn complete_microsoft_login() -> Result<AccountView, BridgeError> {
    let core = core()?;
    let code = PENDING
        .lock()
        .expect("candado del login")
        .clone()
        .ok_or_else(|| BridgeError::internal("no hay un inicio de sesión en curso".into()))?;
    on_rt(async move {
        let pending_code = code.user_code.clone();
        let login = core.complete_msa_login(&code);
        let account = tokio::select! {
            r = login => r?,
            _ = wait_cancelled(pending_code) => {
                return Err(BridgeError { kind: "cancelled".into(), message: "inicio de sesión cancelado".into() });
            }
        };
        *PENDING.lock().expect("candado del login") = None;
        let active = core.accounts().await?.active;
        Ok(AccountView::from_account(&account, active.as_deref()))
    })
    .await
}

#[flutter_rust_bridge::frb(sync)]
pub fn cancel_microsoft_login() {
    *PENDING.lock().expect("candado del login") = None;
}

/// Termina cuando el login pendiente deja de ser `user_code`.
async fn wait_cancelled(user_code: String) {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let current = PENDING
            .lock()
            .expect("candado del login")
            .as_ref()
            .map(|c| c.user_code.clone());
        if current.as_deref() != Some(user_code.as_str()) {
            return;
        }
    }
}

pub async fn add_offline_account(name: String) -> Result<AccountView, BridgeError> {
    let core = core()?;
    on_rt(async move {
        let account = core.add_offline_account(&name).await?;
        let active = core.accounts().await?.active;
        Ok(AccountView::from_account(&account, active.as_deref()))
    })
    .await
}

pub async fn set_active_account(id: String) -> Result<(), BridgeError> {
    let core = core()?;
    on_rt(async move { Ok(core.set_active_account(&id).await?) }).await
}

pub async fn remove_account(id: String) -> Result<(), BridgeError> {
    let core = core()?;
    on_rt(async move {
        core.remove_account(&id).await?;
        Ok(())
    })
    .await
}
