//! Cuentas y skins contra el núcleo (sustituye a `api/accounts.rs` y
//! `api/skins.rs` del bridge): cada acción es una tarea de tokio que entrega
//! su resultado al hilo de la UI con `accounts::post` / `skins::post`.

use std::sync::{Arc, Mutex};

use lcore::auth::mc::SkinVariant;
use tokio::task::AbortHandle;

use crate::accounts::{self, Account, AccountActions, DeviceCode};
use crate::backend::Backend;
use crate::errors::Failure;
use crate::skin::{SkinTexture, secure_texture_url};
use crate::skins::{self, Apply, Profile, SkinActions};
use crate::media;

/// Idioma de los textos de error (el de la UI al arrancar).
fn fail(lang: &str, e: &lcore::Error) -> Failure {
    Failure::new(lang, e)
}

#[derive(Clone)]
pub struct CoreAccounts {
    pub backend: Backend,
    pub lang: String,
    /// Login en curso (se aborta al cancelar).
    login: Arc<Mutex<Option<AbortHandle>>>,
    /// Las lecturas de la lista van de una en una: así llegan a la UI en el
    /// orden en que se leyeron y una vieja no pisa a una nueva.
    loads: Arc<tokio::sync::Mutex<()>>,
}

impl CoreAccounts {
    pub fn new(backend: Backend, lang: &str) -> Self {
        Self { backend, lang: lang.to_owned(), login: Arc::default(), loads: Arc::default() }
    }
}

impl AccountActions for CoreAccounts {
    fn load(&self) {
        let core = self.backend.core.clone();
        let lang = self.lang.clone();
        let loads = self.loads.clone();
        self.backend.rt.spawn(async move {
            let _turn = loads.lock().await;
            let r = core.accounts().await.map(|f| {
                let active = f.active.as_deref();
                f.accounts.iter().map(|a| Account::from_core(a, active)).collect()
            });
            let r = r.map_err(|e| fail(&lang, &e));
            accounts::post(move |a| a.set_accounts(r));
        });
    }

    fn set_active(&self, id: &str) {
        let (core, lang, id) = (self.backend.core.clone(), self.lang.clone(), id.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.set_active_account(&id).await.map_err(|e| fail(&lang, &e));
            accounts::post(move |a| a.activated(r));
        });
    }

    fn remove(&self, id: &str, name: &str) {
        let (core, lang, id, name) = (self.backend.core.clone(), self.lang.clone(), id.to_owned(), name.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.remove_account(&id).await.map(|_| ()).map_err(|e| fail(&lang, &e));
            accounts::post(move |a| a.removed(&name, r));
        });
    }

    fn add_offline(&self, name: &str) {
        let (core, lang, name) = (self.backend.core.clone(), self.lang.clone(), name.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.add_offline_account(&name).await.map(|a| a.name).map_err(|e| fail(&lang, &e));
            accounts::post(move |a| a.offline_added(r));
        });
    }

    fn add_shard(&self, username: &str, password: &str) {
        let (core, lang, user, password) = (self.backend.core.clone(), self.lang.clone(), username.to_owned(), password.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.add_shard_account(&user, &password).await.map(|a| a.name).map_err(|e| fail(&lang, &e));
            accounts::post(move |a| a.offline_added(r));
        });
    }

    fn begin_login(&self, attempt: u64) {
        self.cancel_login();
        let (core, lang) = (self.backend.core.clone(), self.lang.clone());
        let task = self.backend.rt.spawn(async move {
            let code = match core.begin_msa_login().await {
                Ok(c) => c,
                Err(e) => {
                    let f = fail(&lang, &e);
                    accounts::post(move |a| a.login_failed(attempt, f));
                    return;
                }
            };
            let view = DeviceCode { user_code: code.user_code.clone(), verification_uri: code.verification_uri.clone(), expires_in: code.expires_in };
            accounts::post(move |a| a.login_code(attempt, view));
            match core.complete_msa_login(&code).await {
                Ok(acc) => accounts::post(move |a| a.login_done(attempt, &acc.name)),
                Err(e) => {
                    let f = fail(&lang, &e);
                    accounts::post(move |a| a.login_failed(attempt, f));
                }
            }
        });
        *self.login.lock().expect("candado del login") = Some(task.abort_handle());
    }

    fn cancel_login(&self) {
        if let Some(h) = self.login.lock().expect("candado del login").take() {
            h.abort();
        }
    }

    fn open_url(&self, url: &str) {
        if let Err(e) = webbrowser::open(url) {
            tracing::warn!(error = %e, url, "no se pudo abrir el navegador");
        }
    }

    fn load_face(&self, url: &str) {
        let (core, url) = (self.backend.core.clone(), url.to_owned());
        self.backend.rt.spawn(async move {
            let face = media::cached_image(&core, &secure_texture_url(&url)).await.and_then(|b| accounts::face_from_png(&b));
            accounts::post(move |a| a.face_loaded(&url, face));
        });
    }
}

#[derive(Clone)]
pub struct CoreSkins {
    pub backend: Backend,
    pub lang: String,
}

fn variant(slim: bool) -> SkinVariant {
    if slim { SkinVariant::Slim } else { SkinVariant::Classic }
}

/// Tamaño máximo de un PNG local para la vista previa.
const MAX_LOCAL: u64 = 2 * 1024 * 1024;

impl SkinActions for CoreSkins {
    fn load_profile(&self, account: &str, seq: u64) {
        let (core, lang, account) = (self.backend.core.clone(), self.lang.clone(), account.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.profile(Some(&account)).await.map(|p| Profile::from_core(&p)).map_err(|e| fail(&lang, &e));
            skins::post(move |s| s.set_profile(&account, seq, r));
        });
    }

    fn load_texture(&self, url: &str) {
        let (core, url) = (self.backend.core.clone(), url.to_owned());
        self.backend.rt.spawn(async move {
            let tex = media::cached_image(&core, &secure_texture_url(&url)).await.and_then(|b| SkinTexture::decode(&b));
            skins::post(move |s| s.texture_loaded(&url, tex));
        });
    }

    fn pick_file(&self, filter_label: &str) {
        let label = filter_label.to_owned();
        self.backend.rt.spawn(async move {
            let f = rfd::AsyncFileDialog::new().add_filter(label, &["png", "PNG"]).pick_file().await;
            let path = f.map(|h| h.path().display().to_string());
            skins::post(move |s| s.file_picked(path));
        });
    }

    fn load_local(&self, path: &str) {
        let path = path.to_owned();
        self.backend.rt.spawn(async move {
            let ok = tokio::fs::metadata(&path).await.is_ok_and(|m| m.len() <= MAX_LOCAL);
            let tex = if ok { tokio::fs::read(&path).await.ok().and_then(|b| SkinTexture::decode(&b)) } else { None };
            skins::post(move |s| s.local_loaded(&path, tex));
        });
    }

    fn apply(&self, account: &str, what: &Apply) {
        let (core, lang, account, what) = (self.backend.core.clone(), self.lang.clone(), account.to_owned(), what.clone());
        self.backend.rt.spawn(async move {
            let r = match &what {
                Apply::Upload { path, slim } => core.upload_skin(Some(&account), std::path::Path::new(path), variant(*slim)).await,
                Apply::Url { url, slim } | Apply::Model { url, slim } => core.set_skin_url(Some(&account), url, variant(*slim)).await,
                Apply::InvalidPng | Apply::Nothing => return,
            };
            let r = r.map(|p| Profile::from_core(&p)).map_err(|e| fail(&lang, &e));
            skins::post(move |s| s.changed(&account, false, r));
        });
    }

    fn reset(&self, account: &str) {
        let (core, lang, account) = (self.backend.core.clone(), self.lang.clone(), account.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.reset_skin(Some(&account)).await.map(|p| Profile::from_core(&p)).map_err(|e| fail(&lang, &e));
            skins::post(move |s| s.changed(&account, true, r));
        });
    }

    fn cape(&self, account: &str, id: Option<&str>) {
        let (core, lang, account, id) = (self.backend.core.clone(), self.lang.clone(), account.to_owned(), id.map(str::to_owned));
        self.backend.rt.spawn(async move {
            let r = match &id {
                Some(id) => core.show_cape(Some(&account), id).await,
                None => core.hide_cape(Some(&account)).await,
            };
            let r = r.map(|p| Profile::from_core(&p)).map_err(|e| fail(&lang, &e));
            skins::post(move |s| s.cape_done(&account, r));
        });
    }
}
