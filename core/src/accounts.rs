//! Gestión de cuentas: `accounts.json` (datos públicos) + secretos en el keyring.
//!
//! Secretos por cuenta de Microsoft (usuario = UUID de la cuenta):
//! - `<APP_ID>.msa`: refresh token de Microsoft.
//! - `<APP_ID>.mc`: token de Minecraft en caché con su caducidad (JSON).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::auth::mc::{self, Profile, SkinVariant};
use crate::auth::msa::{self, DeviceCode, MsaTokens};
use crate::auth::{Account, AccountKind, AuthError, Session, UserType, offline, xbox};
use crate::events::Event;
use crate::fsutil::{now_secs, read_json, write_json};
use crate::launcher::Core;
use crate::paths::APP_ID;
use crate::secrets::Secret;
use crate::{Error, Result};

const ACCOUNTS_SCHEMA: u32 = 1;
/// Margen antes de la caducidad para renovar el token de Minecraft.
const MC_TOKEN_MARGIN_SECS: i64 = 300;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountsFile {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub active: Option<String>,
    #[serde(default)]
    pub accounts: Vec<Account>,
}

impl AccountsFile {
    pub fn find(&self, id: &str) -> Option<&Account> {
        self.accounts.iter().find(|a| a.id == id)
    }

    pub fn active_account(&self) -> Option<&Account> {
        self.active.as_deref().and_then(|id| self.find(id))
    }
}

#[derive(Serialize, Deserialize)]
struct CachedMcToken {
    token: String,
    expires_at: i64,
}

fn msa_service() -> String {
    format!("{APP_ID}.msa")
}

fn mc_service() -> String {
    format!("{APP_ID}.mc")
}

impl Core {
    pub async fn accounts(&self) -> Result<AccountsFile> {
        Ok(read_json(&self.paths().accounts_file())
            .await?
            .unwrap_or_default())
    }

    /// Busca por UUID (con o sin guiones), prefijo de UUID o nombre exacto.
    pub async fn find_account(&self, query: &str) -> Result<Account> {
        let file = self.accounts().await?;
        let q = query.replace('-', "").to_lowercase();
        let by_name: Vec<_> = file
            .accounts
            .iter()
            .filter(|a| a.name.eq_ignore_ascii_case(query))
            .collect();
        if by_name.len() == 1 {
            return Ok(by_name[0].clone());
        }
        let by_id: Vec<_> = file
            .accounts
            .iter()
            .filter(|a| !q.is_empty() && a.id.starts_with(&q))
            .collect();
        match by_id.as_slice() {
            [one] => Ok((*one).clone()),
            _ => Err(Error::AccountNotFound(query.to_owned())),
        }
    }

    async fn update_accounts<T>(
        &self,
        f: impl FnOnce(&mut AccountsFile) -> Result<T>,
    ) -> Result<T> {
        let _guard = self.inner.accounts_lock.lock().await;
        let path = self.paths().accounts_file();
        let mut file: AccountsFile = read_json(&path).await?.unwrap_or_default();
        let out = f(&mut file)?;
        file.schema = ACCOUNTS_SCHEMA;
        write_json(&path, &file).await?;
        self.inner.events.emit(Event::AccountsChanged);
        Ok(out)
    }

    async fn client_id(&self) -> Result<String> {
        self.settings()
            .await
            .effective_msa_client_id()
            .ok_or_else(|| AuthError::MissingClientId.into())
    }

    /// Paso 1 del login: pide un código para que el usuario lo introduzca.
    pub async fn begin_msa_login(&self) -> Result<DeviceCode> {
        let client_id = self.client_id().await?;
        msa::request_device_code(self.http(), &self.inner.endpoints, &client_id).await
    }

    /// Paso 2: espera al usuario, completa la cadena y guarda la cuenta.
    pub async fn complete_msa_login(&self, code: &DeviceCode) -> Result<Account> {
        let client_id = self.client_id().await?;
        let tokens =
            msa::poll_device_code(self.http(), &self.inner.endpoints, &client_id, code).await?;
        self.finish_msa(tokens, None).await
    }

    /// De token de Microsoft a cuenta guardada.
    async fn finish_msa(&self, tokens: MsaTokens, expected_id: Option<&str>) -> Result<Account> {
        let ep = &self.inner.endpoints;
        let http = self.http();
        let xbl = xbox::authenticate_user(http, ep, &tokens.access_token).await?;
        let xsts = xbox::authorize_xsts(http, ep, &xbl).await?;
        let mc_token = mc::login_with_xbox(http, ep, &xsts).await?;
        let profile = match mc::profile(http, ep, &mc_token.access_token).await? {
            Some(p) => p,
            None => {
                let owns = mc::owns_game(http, ep, &mc_token.access_token)
                    .await
                    .unwrap_or(false);
                return Err(if owns {
                    AuthError::NoProfile
                } else {
                    AuthError::NotEntitled
                }
                .into());
            }
        };
        if let Some(expected) = expected_id
            && expected != profile.id
        {
            return Err(AuthError::Unexpected {
                service: "Minecraft",
                detail: format!("la sesión pertenece a otra cuenta ({})", profile.name),
            }
            .into());
        }

        if let Some(refresh) = &tokens.refresh_token {
            self.secret_set(
                msa_service(),
                profile.id.clone(),
                refresh.expose().to_owned(),
            )
            .await?;
        } else if expected_id.is_none() {
            return Err(AuthError::Unexpected {
                service: "Microsoft",
                detail: "no se recibió refresh token (¿falta offline_access?)".into(),
            }
            .into());
        }
        self.store_mc_token(&profile.id, &mc_token.access_token, mc_token.expires_in)
            .await?;
        self.upsert_msa_account(&profile).await
    }

    async fn store_mc_token(&self, id: &str, token: &Secret, expires_in: u64) -> Result<()> {
        let cached = CachedMcToken {
            token: token.expose().to_owned(),
            expires_at: now_secs() + expires_in as i64,
        };
        let json = serde_json::to_string(&cached).map_err(|e| Error::json("token", e))?;
        self.secret_set(mc_service(), id.to_owned(), json).await
    }

    async fn upsert_msa_account(&self, profile: &Profile) -> Result<Account> {
        let skin_url = profile.active_skin().map(|s| s.url.clone());
        self.update_accounts(|file| {
            let now = now_secs();
            let account = match file.accounts.iter_mut().find(|a| a.id == profile.id) {
                Some(a) => {
                    a.name = profile.name.clone();
                    a.skin_url = skin_url;
                    a.clone()
                }
                None => {
                    let a = Account {
                        id: profile.id.clone(),
                        name: profile.name.clone(),
                        kind: AccountKind::Msa,
                        skin_url,
                        added_at: now,
                        last_used: None,
                    };
                    file.accounts.push(a.clone());
                    a
                }
            };
            if file.active.is_none() {
                file.active = Some(account.id.clone());
            }
            Ok(account)
        })
        .await
    }

    pub async fn add_offline_account(&self, name: &str) -> Result<Account> {
        offline::validate_name(name)?;
        let id = offline::offline_uuid(name).simple().to_string();
        self.update_accounts(|file| {
            if file.find(&id).is_some() {
                return Err(Error::InvalidInput(format!(
                    "ya existe la cuenta offline {name}"
                )));
            }
            let account = Account {
                id: id.clone(),
                name: name.to_owned(),
                kind: AccountKind::Offline,
                skin_url: None,
                added_at: now_secs(),
                last_used: None,
            };
            file.accounts.push(account.clone());
            if file.active.is_none() {
                file.active = Some(id.clone());
            }
            Ok(account)
        })
        .await
    }

    pub async fn set_active_account(&self, id: &str) -> Result<()> {
        self.update_accounts(|file| {
            if file.find(id).is_none() {
                return Err(Error::AccountNotFound(id.to_owned()));
            }
            file.active = Some(id.to_owned());
            Ok(())
        })
        .await
    }

    /// Borra los secretos y quita la cuenta. Si el keyring falla, la cuenta se
    /// queda en la lista para poder reintentar.
    pub async fn remove_account(&self, id: &str) -> Result<Account> {
        let account = self
            .accounts()
            .await?
            .find(id)
            .cloned()
            .ok_or_else(|| Error::AccountNotFound(id.to_owned()))?;
        if account.kind == AccountKind::Msa {
            let a = self.secret_delete(msa_service(), account.id.clone()).await;
            let b = self.secret_delete(mc_service(), account.id.clone()).await;
            a.and(b)?;
        }
        self.update_accounts(|file| {
            let pos = file
                .accounts
                .iter()
                .position(|a| a.id == id)
                .ok_or_else(|| Error::AccountNotFound(id.to_owned()))?;
            let removed = file.accounts.remove(pos);
            if file.active.as_deref() == Some(id) {
                file.active = file.accounts.first().map(|a| a.id.clone());
            }
            Ok(removed)
        })
        .await
    }

    /// Sesión lista para lanzar; renueva el token si hace falta.
    /// `id = None` usa la cuenta activa.
    pub async fn session(&self, id: Option<&str>) -> Result<Session> {
        let account = self.resolve(id).await?;
        let session = match account.kind {
            AccountKind::Offline => Session {
                uuid: account.id.clone(),
                name: account.name.clone(),
                access_token: Secret::new("0"),
                user_type: UserType::Legacy,
            },
            AccountKind::Msa => {
                let token = self.valid_mc_token(&account, false).await?;
                let account = self.find_account(&account.id).await?;
                Session {
                    uuid: account.id,
                    name: account.name,
                    access_token: token,
                    user_type: UserType::Msa,
                }
            }
        };
        let uuid = session.uuid.clone();
        self.update_accounts(|file| {
            if let Some(a) = file.accounts.iter_mut().find(|a| a.id == uuid) {
                a.last_used = Some(now_secs());
            }
            Ok(())
        })
        .await?;
        Ok(session)
    }

    /// Fuerza la renovación completa de la sesión de una cuenta de Microsoft.
    pub async fn refresh_account(&self, id: &str) -> Result<Account> {
        let account = self.find_account(id).await?;
        if account.kind != AccountKind::Msa {
            return Err(AuthError::OfflineAccount.into());
        }
        self.valid_mc_token(&account, true).await?;
        self.find_account(&account.id).await
    }

    async fn resolve(&self, id: Option<&str>) -> Result<Account> {
        match id {
            Some(id) => self.find_account(id).await,
            None => self
                .accounts()
                .await?
                .active_account()
                .cloned()
                .ok_or_else(|| Error::AccountNotFound("(activa)".into())),
        }
    }

    async fn valid_mc_token(&self, account: &Account, force: bool) -> Result<Secret> {
        if !force
            && let Some(raw) = self.secret_get(mc_service(), account.id.clone()).await?
            && let Ok(cached) = serde_json::from_str::<CachedMcToken>(&raw)
            && cached.expires_at - now_secs() > MC_TOKEN_MARGIN_SECS
        {
            return Ok(Secret::new(cached.token));
        }
        let refresh = self
            .secret_get(msa_service(), account.id.clone())
            .await?
            .map(Secret::new)
            .ok_or(AuthError::RefreshTokenMissing)?;
        let client_id = self.client_id().await?;
        let tokens = msa::refresh(self.http(), &self.inner.endpoints, &client_id, &refresh).await?;
        self.finish_msa(tokens, Some(&account.id)).await?;
        let raw = self
            .secret_get(mc_service(), account.id.clone())
            .await?
            .ok_or(AuthError::RefreshTokenMissing)?;
        let cached: CachedMcToken =
            serde_json::from_str(&raw).map_err(|e| Error::json("token", e))?;
        Ok(Secret::new(cached.token))
    }

    async fn msa_token_for(&self, id: Option<&str>) -> Result<(Account, Secret)> {
        let account = self.resolve(id).await?;
        if account.kind != AccountKind::Msa {
            return Err(AuthError::OfflineAccount.into());
        }
        let token = self.valid_mc_token(&account, false).await?;
        Ok((account, token))
    }

    /// Perfil actual (skins y capas) de una cuenta de Microsoft.
    pub async fn profile(&self, id: Option<&str>) -> Result<Profile> {
        let (_, token) = self.msa_token_for(id).await?;
        let profile = mc::profile(self.http(), &self.inner.endpoints, &token)
            .await?
            .ok_or(AuthError::NoProfile)?;
        self.upsert_msa_account(&profile).await?;
        Ok(profile)
    }

    pub async fn set_skin_url(
        &self,
        id: Option<&str>,
        url: &str,
        variant: SkinVariant,
    ) -> Result<Profile> {
        let (_, token) = self.msa_token_for(id).await?;
        let profile =
            mc::set_skin_url(self.http(), &self.inner.endpoints, &token, url, variant).await?;
        self.upsert_msa_account(&profile).await?;
        Ok(profile)
    }

    pub async fn upload_skin(
        &self,
        id: Option<&str>,
        file: &Path,
        variant: SkinVariant,
    ) -> Result<Profile> {
        let (_, token) = self.msa_token_for(id).await?;
        let profile =
            mc::upload_skin(self.http(), &self.inner.endpoints, &token, file, variant).await?;
        self.upsert_msa_account(&profile).await?;
        Ok(profile)
    }

    pub async fn reset_skin(&self, id: Option<&str>) -> Result<Profile> {
        let (_, token) = self.msa_token_for(id).await?;
        let profile = mc::reset_skin(self.http(), &self.inner.endpoints, &token).await?;
        self.upsert_msa_account(&profile).await?;
        Ok(profile)
    }

    pub async fn show_cape(&self, id: Option<&str>, cape_id: &str) -> Result<Profile> {
        let (_, token) = self.msa_token_for(id).await?;
        mc::show_cape(self.http(), &self.inner.endpoints, &token, cape_id).await
    }

    pub async fn hide_cape(&self, id: Option<&str>) -> Result<Profile> {
        let (_, token) = self.msa_token_for(id).await?;
        mc::hide_cape(self.http(), &self.inner.endpoints, &token).await
    }
}
