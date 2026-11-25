//! Gestión de cuentas: `accounts.json` (datos públicos) + secretos en el keyring.
//!
//! Secretos por cuenta de Microsoft (usuario = UUID de la cuenta):
//! - `<APP_ID>.sisu`: sesión SISU completa, incluidos tokens y claves del dispositivo.
//! - `<APP_ID>.msa` y `<APP_ID>.mc`: formato antiguo; solo se leen para pedir
//!   re-login y se borran al eliminar la cuenta.

use std::path::Path;

use minecraft_auth::java::{JavaAuthManager, JavaLaunchSession};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::auth::mc::{self, Profile, SkinVariant};
use crate::auth::msa::{self, DeviceCode};
use crate::auth::{Account, AccountKind, AuthError, Session, UserType, offline, yggdrasil};
use crate::events::Event;
use crate::fsutil::{now_secs, read_json, write_json};
use crate::launcher::Core;
use crate::paths::APP_ID;
use crate::secrets::Secret;
use crate::settings::DEFAULT_YGGDRASIL_BASE_URL;
use crate::{Error, Result};

const ACCOUNTS_SCHEMA: u32 = 1;

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

fn msa_service() -> String {
    format!("{APP_ID}.msa")
}

fn mc_service() -> String {
    format!("{APP_ID}.mc")
}

fn sisu_service() -> String {
    format!("{APP_ID}.sisu")
}

fn shard_service() -> String {
    format!("{APP_ID}.yggdrasil")
}

#[derive(Serialize, Deserialize)]
struct ShardLogin {
    base: String,
    credentials: yggdrasil::Credentials,
}

fn ensure_expected_account(launch: &JavaLaunchSession, expected_id: Option<&str>) -> Result<()> {
    if let Some(expected) = expected_id
        && expected != launch.player_uuid.simple().to_string()
    {
        return Err(AuthError::Unexpected {
            service: "Minecraft",
            detail: format!("la sesión pertenece a otra cuenta ({})", launch.player_name),
        }
        .into());
    }
    Ok(())
}

impl Core {
    pub async fn accounts(&self) -> Result<AccountsFile> {
        Ok(read_json(&self.paths().accounts_file())
            .await?
            .unwrap_or_default())
    }

    /// Busca por identificador local, prefijo o nombre exacto.
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
        let mut file = self.accounts().await?;
        let out = f(&mut file)?;
        self.save_accounts(file).await?;
        Ok(out)
    }

    async fn save_accounts(&self, mut file: AccountsFile) -> Result<()> {
        file.schema = ACCOUNTS_SCHEMA;
        write_json(&self.paths().accounts_file(), &file).await?;
        self.inner.events.emit(Event::AccountsChanged);
        Ok(())
    }

    /// Paso 1 del login SISU: devuelve el código y deja el sondeo en una tarea
    /// cancelable al soltar `DeviceCode`.
    pub async fn begin_msa_login(&self) -> Result<DeviceCode> {
        msa::begin_login(self.http().client().clone()).await
    }

    /// Paso 2: espera al usuario, completa la cadena y guarda la cuenta.
    pub async fn complete_msa_login(&self, code: &DeviceCode) -> Result<Account> {
        let manager = msa::complete_login(code).await?;
        self.finish_msa(manager, None).await
    }

    /// Completa el perfil y guarda toda la sesión, incluidas las claves SISU.
    async fn finish_msa(
        &self,
        manager: JavaAuthManager,
        expected_id: Option<&str>,
    ) -> Result<Account> {
        let launch = match manager.launch_session().await {
            Ok(launch) => launch,
            Err(minecraft_auth::Error::ProfileNotFound) => {
                let entitlements = manager.entitlements().await.map_err(msa::map_error)?;
                return Err(if entitlements.items.is_empty() {
                    AuthError::NotEntitled
                } else {
                    AuthError::NoProfile
                }
                .into());
            }
            Err(error) => return Err(msa::map_error(error)),
        };
        let id = launch.player_uuid.simple().to_string();
        ensure_expected_account(&launch, expected_id)?;

        // El perfil del crate no incluye skins; esta llamada conserva ese dato
        // en `accounts.json` sin rehacer ninguna fase de autenticación.
        let token = Secret::new(launch.access_token);
        let profile = mc::profile(self.http(), &self.inner.endpoints, &token)
            .await?
            .ok_or(AuthError::NoProfile)?;
        self.store_sisu_session(&id, &manager).await?;
        self.upsert_msa_account(&profile).await
    }

    async fn store_sisu_session(&self, id: &str, manager: &JavaAuthManager) -> Result<()> {
        let json = msa::to_saved_json(manager).await?;
        self.secret_set(sisu_service(), id.to_owned(), json).await
    }

    async fn load_sisu_manager(&self, account: &Account) -> Result<JavaAuthManager> {
        if let Some(raw) = self.secret_get(sisu_service(), account.id.clone()).await? {
            return msa::from_saved_json(self.http().client().clone(), &raw);
        }

        let legacy_msa = self
            .secret_get(msa_service(), account.id.clone())
            .await?
            .is_some();
        let legacy_mc = self
            .secret_get(mc_service(), account.id.clone())
            .await?
            .is_some();
        if legacy_msa || legacy_mc {
            Err(AuthError::LegacyMsaSession.into())
        } else {
            Err(AuthError::RefreshTokenMissing.into())
        }
    }

    /// Usa el refresco perezoso del crate y persiste cualquier token renovado.
    async fn stored_msa_launch(&self, account: &Account) -> Result<JavaLaunchSession> {
        let manager = self.load_sisu_manager(account).await?;
        let launch = manager.launch_session().await.map_err(msa::map_error)?;
        ensure_expected_account(&launch, Some(&account.id))?;
        self.store_sisu_session(&account.id, &manager).await?;
        Ok(launch)
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

    pub async fn add_shard_account(&self, username: &str, password: &str) -> Result<Account> {
        let settings = self.settings().await;
        let base = yggdrasil::normalize_base(
            settings
                .yggdrasil_base_url
                .as_deref()
                .unwrap_or(DEFAULT_YGGDRASIL_BASE_URL),
        )?;
        let credentials = yggdrasil::authenticate(self.http(), &base, username, password).await?;
        let profile = &credentials.selected_profile;
        // Un UUID offline puede existir también en Drasl o en otro servidor Yggdrasil.
        let digest = Sha256::digest(format!("{base}:{}", profile.id.simple()).as_bytes());
        let id = format!("shard{}", hex::encode(digest));
        let name = profile.name.clone();
        let login = ShardLogin { base, credentials };
        let json = serde_json::to_string(&login).map_err(|e| Error::json("sesión Shard", e))?;

        let _guard = self.inner.accounts_lock.lock().await;
        let mut file = self.accounts().await?;
        let existing = file.accounts.iter_mut().find(|a| a.id == id);
        let account = if let Some(account) = existing {
            account.name = name;
            account.clone()
        } else {
            let account = Account {
                id: id.clone(),
                name,
                kind: AccountKind::Shard,
                skin_url: None,
                added_at: now_secs(),
                last_used: None,
            };
            file.accounts.push(account.clone());
            account
        };
        if file.active.is_none() {
            file.active = Some(id.clone());
        }

        let service = shard_service();
        let previous = self.secret_get(service.clone(), id.clone()).await?;
        self.secret_set(service.clone(), id.clone(), json).await?;
        if let Err(error) = self.save_accounts(file).await {
            match previous {
                Some(secret) => self.secret_set(service, id, secret).await?,
                None => self.secret_delete(service, id).await?,
            }
            return Err(error);
        }
        Ok(account)
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
        let _guard = self.inner.accounts_lock.lock().await;
        let mut file = self.accounts().await?;
        let pos = file
            .accounts
            .iter()
            .position(|a| a.id == id)
            .ok_or_else(|| Error::AccountNotFound(id.to_owned()))?;
        let account = file.accounts.remove(pos);
        match account.kind {
            AccountKind::Msa => {
                let a = self.secret_delete(msa_service(), account.id.clone()).await;
                let b = self.secret_delete(mc_service(), account.id.clone()).await;
                let c = self.secret_delete(sisu_service(), account.id.clone()).await;
                a.and(b).and(c)?;
            }
            AccountKind::Shard => {
                self.secret_delete(shard_service(), account.id.clone())
                    .await?
            }
            AccountKind::Offline => {}
        }
        if file.active.as_deref() == Some(id) {
            file.active = file.accounts.first().map(|a| a.id.clone());
        }
        self.save_accounts(file).await?;
        Ok(account)
    }

    /// Sesión lista para lanzar; renueva el token si hace falta.
    /// `id = None` usa la cuenta activa.
    pub async fn session(&self, id: Option<&str>) -> Result<Session> {
        self.session_with_base(id).await.map(|(session, _)| session)
    }

    pub(crate) async fn session_with_base(
        &self,
        id: Option<&str>,
    ) -> Result<(Session, Option<String>)> {
        let account = self.resolve(id).await?;
        let mut base = None;
        let session = match account.kind {
            AccountKind::Offline => Session {
                uuid: account.id.clone(),
                name: account.name.clone(),
                access_token: Secret::new("0"),
                user_type: UserType::Legacy,
            },
            AccountKind::Shard => {
                let json = self
                    .secret_get(shard_service(), account.id.clone())
                    .await?
                    .ok_or(AuthError::RefreshTokenMissing)?;
                let mut login: ShardLogin =
                    serde_json::from_str(&json).map_err(|e| Error::json("sesión Shard", e))?;
                if !yggdrasil::validate(self.http(), &login.base, &login.credentials).await? {
                    login.credentials =
                        yggdrasil::refresh(self.http(), &login.base, &login.credentials).await?;
                    let json = serde_json::to_string(&login)
                        .map_err(|e| Error::json("sesión Shard", e))?;
                    self.secret_set(shard_service(), account.id.clone(), json)
                        .await?;
                }
                base = Some(login.base);
                Session {
                    uuid: login.credentials.selected_profile.id.simple().to_string(),
                    name: login.credentials.selected_profile.name,
                    access_token: Secret::new(login.credentials.access_token),
                    user_type: UserType::Mojang,
                }
            }
            AccountKind::Msa => {
                let launch = self.stored_msa_launch(&account).await?;
                Session {
                    uuid: launch.player_uuid.simple().to_string(),
                    name: launch.player_name,
                    access_token: Secret::new(launch.access_token),
                    user_type: UserType::Msa,
                }
            }
        };
        self.update_accounts(|file| {
            if let Some(a) = file.accounts.iter_mut().find(|a| a.id == account.id) {
                a.last_used = Some(now_secs());
                if a.kind == AccountKind::Msa {
                    a.name = session.name.clone();
                }
            }
            Ok(())
        })
        .await?;
        Ok((session, base))
    }

    /// Actualiza el perfil y renueva solo las fases de la sesión que hayan
    /// caducado; la cadencia la controla `minecraft-auth`.
    pub async fn refresh_account(&self, id: &str) -> Result<Account> {
        let account = self.find_account(id).await?;
        if account.kind != AccountKind::Msa {
            return Err(AuthError::OfflineAccount.into());
        }
        let launch = self.stored_msa_launch(&account).await?;
        let token = Secret::new(launch.access_token);
        let profile = mc::profile(self.http(), &self.inner.endpoints, &token)
            .await?
            .ok_or(AuthError::NoProfile)?;
        self.upsert_msa_account(&profile).await
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

    async fn msa_token_for(&self, id: Option<&str>) -> Result<(Account, Secret)> {
        let account = self.resolve(id).await?;
        if account.kind != AccountKind::Msa {
            return Err(AuthError::OfflineAccount.into());
        }
        let launch = self.stored_msa_launch(&account).await?;
        Ok((account, Secret::new(launch.access_token)))
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
