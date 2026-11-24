//! Cuentas: lista, cuenta activa (raíl y Skins), panel, login de Microsoft
//! con código de dispositivo y alta offline (accounts/*.dart).
//!
//! `Accounts` corre en el hilo de la UI: copia el estado a `AccountsState`
//! y atiende sus callbacks. El trabajo con el núcleo lo hace un
//! `AccountActions` (`backend::CoreAccounts` en la app, un registro en los
//! tests), que entrega los resultados con [`post`].

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, Image, ModelRc, SharedPixelBuffer, VecModel};

use crate::controller::Controller;
use crate::errors::Failure;
use crate::{AccountChip, AccountItem, AccountsState, LoginPhase, MainWindow, face, i18n, skin};

/// Una cuenta tal como la usa la UI (`AccountView` del bridge).
#[derive(Clone, Debug, PartialEq)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub microsoft: bool,
    /// Textura de la skin (para la cara), si es de Microsoft y se conoce.
    pub skin_url: Option<String>,
    pub active: bool,
}

impl Account {
    pub fn from_core(a: &lcore::auth::Account, active: Option<&str>) -> Self {
        Self {
            id: a.id.clone(),
            name: a.name.clone(),
            microsoft: a.kind == lcore::auth::AccountKind::Msa,
            skin_url: a.skin_url.clone(),
            active: active == Some(a.id.as_str()),
        }
    }
}

/// Cuenta activa: la marcada o, si no hay, la primera (`activeAccountProvider`).
pub fn active(list: &[Account]) -> Option<&Account> {
    list.iter().find(|a| a.active).or(list.first())
}

/// Nombre de cuenta offline válido: 3–16 letras ASCII, dígitos o `_`.
pub fn valid_offline_name(name: &str) -> bool {
    let n = name.trim();
    (3..=16).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// `m:ss` (cuenta atrás del código).
pub fn mmss(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// URL sin `http(s)://` (lo que se muestra bajo el código).
pub fn bare_uri(uri: &str) -> &str {
    uri.strip_prefix("https://").or_else(|| uri.strip_prefix("http://")).unwrap_or(uri)
}

/// Código de dispositivo de Microsoft (sin el secreto, que se queda en el
/// núcleo).
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
}

/// Estado del diálogo de login.
#[derive(Clone, Debug, PartialEq)]
pub enum Login {
    Closed,
    Requesting,
    /// Código a la vista; `started_ms` en el reloj del controlador.
    Code { code: DeviceCode, started_ms: u64 },
    Expired,
    Failed(Failure),
}

impl Login {
    /// Segundos que le quedan al código en `now_ms`.
    pub fn left(&self, now_ms: u64) -> u64 {
        match self {
            Login::Code { code, started_ms } => code.expires_in.saturating_sub(now_ms.saturating_sub(*started_ms) / 1000),
            _ => 0,
        }
    }
}

/// Lo que el controlador pide fuera de la UI.
pub trait AccountActions {
    /// Lista de cuentas → [`Accounts::set_accounts`].
    fn load(&self);
    /// → [`Accounts::activated`].
    fn set_active(&self, id: &str);
    /// → [`Accounts::removed`].
    fn remove(&self, id: &str, name: &str);
    /// → [`Accounts::offline_added`].
    fn add_offline(&self, name: &str);
    /// Pide un código y espera al usuario → [`Accounts::login_code`],
    /// [`Accounts::login_done`] o [`Accounts::login_failed`] con `attempt`.
    fn begin_login(&self, attempt: u64);
    fn cancel_login(&self);
    fn open_url(&self, url: &str);
    /// Cara de la skin → [`Accounts::face_loaded`].
    fn load_face(&self, url: &str);
}

thread_local! {
    static ACCOUNTS: RefCell<Option<Rc<Accounts>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador de cuentas.
pub fn post(f: impl FnOnce(&Accounts) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || {
        ACCOUNTS.with(|c| {
            if let Some(a) = c.borrow().as_ref() {
                f(a);
            }
        })
    });
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// Aviso de cuenta activa nueva (o de su cara).
type ActiveListener = Box<dyn Fn(Option<&Account>, AccountChip)>;

pub struct Accounts {
    ui: slint::Weak<MainWindow>,
    shell: Rc<Controller>,
    actions: Box<dyn AccountActions>,
    /// Reloj monótono en ms (fijo en las escenas de paridad).
    now_ms: Box<dyn Fn() -> u64>,
    list: RefCell<Vec<Account>>,
    loaded: Cell<bool>,
    login: RefCell<Login>,
    attempt: Cell<u64>,
    /// Caras de skin por URL (`None` = no se pudo: cara generada).
    faces: RefCell<HashMap<String, Option<Image>>>,
    tick: slint::Timer,
    items: Rc<VecModel<AccountItem>>,
    listeners: RefCell<Vec<ActiveListener>>,
}

fn rgba_image(px: &[u8], w: u32, h: u32) -> Image {
    Image::from_rgba8(SharedPixelBuffer::clone_from_slice(px, w, h))
}

impl Accounts {
    /// Crea el controlador, lo registra para [`post`] y pide la lista.
    pub fn new(ui: &MainWindow, shell: Rc<Controller>, actions: Box<dyn AccountActions>, now_ms: Box<dyn Fn() -> u64>) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            shell,
            actions,
            now_ms,
            list: RefCell::default(),
            loaded: Cell::new(false),
            login: RefCell::new(Login::Closed),
            attempt: Cell::new(0),
            faces: RefCell::default(),
            tick: slint::Timer::default(),
            items: Rc::new(VecModel::default()),
            listeners: RefCell::default(),
        });
        ACCOUNTS.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<AccountsState>();
        st.set_items(ModelRc::from(me.items.clone()));
        st.set_loading(true);
        st.on_offline_valid(|n| valid_offline_name(&n));
        let weak = Rc::downgrade(&me);
        let with = move |f: Box<dyn Fn(&Accounts)>| {
            let weak = weak.clone();
            move || {
                if let Some(a) = weak.upgrade() {
                    f(&a)
                }
            }
        };
        st.on_add_microsoft(with(Box::new(|a| a.start_login())));
        st.on_login_retry(with(Box::new(|a| a.start_login())));
        st.on_login_cancel(with(Box::new(|a| a.cancel_login())));
        st.on_open_browser(with(Box::new(|a| {
            if let Login::Code { code, .. } = &*a.login.borrow() {
                a.actions.open_url(&code.verification_uri);
            }
        })));
        st.on_copied(with(Box::new(|a| a.toast("copied", false))));
        st.on_add_offline(with(Box::new(|a| {
            if let Some(ui) = a.ui.upgrade() {
                let st = ui.global::<AccountsState>();
                st.set_offline_error("".into());
                st.set_offline_error_detail("".into());
                st.set_offline_busy(false);
                st.set_offline_open(true);
            }
        })));
        let weak = Rc::downgrade(&me);
        st.on_offline_submit(move |name| {
            if let Some(a) = weak.upgrade()
                && valid_offline_name(&name)
            {
                if let Some(ui) = a.ui.upgrade() {
                    ui.global::<AccountsState>().set_offline_busy(true);
                }
                a.actions.add_offline(name.trim());
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_select(move |id| {
            if let Some(a) = weak.upgrade() {
                a.actions.set_active(&id);
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_remove(move |id| {
            if let Some(a) = weak.upgrade() {
                let name = a.list.borrow().iter().find(|x| x.id == id.as_str()).map(|x| x.name.clone()).unwrap_or_default();
                a.actions.remove(&id, &name);
            }
        });
        me.actions.load();
        me
    }

    fn lang(&self) -> String {
        self.shell.lang()
    }

    fn toast(&self, key: &str, error: bool) {
        self.shell.toast(&i18n::text(&self.lang(), key), error);
    }

    fn toast_with(&self, key: &str, arg: &str) {
        self.shell.toast(&fill(&i18n::text(&self.lang(), key), arg), false);
    }

    /// Llama a `f` con la cuenta activa y su cara cada vez que cambian.
    pub fn on_active(&self, f: impl Fn(Option<&Account>, AccountChip) + 'static) {
        let list = self.list.borrow();
        f(active(&list), self.chip(active(&list)));
        self.listeners.borrow_mut().push(Box::new(f));
    }

    /// Vuelve a leer la lista (tras cambiar la skin, eventos del núcleo…).
    pub fn reload(&self) {
        self.actions.load();
    }

    pub fn accounts(&self) -> Vec<Account> {
        self.list.borrow().clone()
    }

    pub fn login_state(&self) -> Login {
        self.login.borrow().clone()
    }

    /// Cara de la cuenta: la de su skin si ya está, si no la generada.
    fn chip(&self, a: Option<&Account>) -> AccountChip {
        let Some(a) = a else { return AccountChip::default() };
        let face = a
            .skin_url
            .as_ref()
            .and_then(|u| self.faces.borrow().get(u).cloned().flatten())
            .unwrap_or_else(|| face::generated_face_image(&a.name));
        AccountChip { present: true, name: a.name.as_str().into(), face }
    }

    /// Lista del núcleo (o el error al leerla: sin cuentas, como Flutter).
    pub fn set_accounts(&self, list: Result<Vec<Account>, Failure>) {
        let list = list.unwrap_or_else(|f| {
            tracing::warn!(detail = %f.detail, "no se pudieron leer las cuentas");
            Vec::new()
        });
        for url in list.iter().filter_map(|a| a.skin_url.clone()) {
            if !self.faces.borrow().contains_key(&url) {
                self.faces.borrow_mut().insert(url.clone(), None);
                self.actions.load_face(&url);
            }
        }
        *self.list.borrow_mut() = list;
        self.loaded.set(true);
        self.refresh();
    }

    pub fn face_loaded(&self, url: &str, face: Option<Vec<u8>>) {
        self.faces.borrow_mut().insert(url.to_owned(), face.map(|px| rgba_image(&px, 8, 8)));
        self.refresh();
    }

    fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let list = self.list.borrow();
        let act = active(&list);
        let rows: Vec<AccountItem> = list
            .iter()
            .map(|a| AccountItem {
                id: a.id.as_str().into(),
                name: a.name.as_str().into(),
                microsoft: a.microsoft,
                active: act.is_some_and(|x| x.id == a.id),
                chip: self.chip(Some(a)),
            })
            .collect();
        self.items.set_vec(rows);
        let st = ui.global::<AccountsState>();
        st.set_loading(!self.loaded.get());
        let chip = self.chip(act);
        ui.set_account(chip.clone());
        for f in self.listeners.borrow().iter() {
            f(act, chip.clone());
        }
    }

    pub fn activated(&self, r: Result<(), Failure>) {
        match r {
            Ok(()) => self.actions.load(),
            Err(f) => {
                tracing::warn!(detail = %f.detail, "no se pudo cambiar de cuenta");
                self.toast("errGeneric", true);
            }
        }
    }

    pub fn removed(&self, name: &str, r: Result<(), Failure>) {
        match r {
            Ok(()) => {
                self.toast_with("accountRemoved", name);
                self.actions.load();
            }
            Err(f) => {
                tracing::warn!(detail = %f.detail, "no se pudo quitar la cuenta");
                self.toast("errGeneric", true);
            }
        }
    }

    pub fn offline_added(&self, r: Result<String, Failure>) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<AccountsState>();
        st.set_offline_busy(false);
        match r {
            Ok(name) => {
                st.set_offline_open(false);
                self.toast_with("accountAdded", &name);
                self.actions.load();
            }
            Err(f) => {
                st.set_offline_error(f.message.into());
                st.set_offline_error_detail(f.detail.into());
            }
        }
    }

    // --- Login de Microsoft -------------------------------------------------

    fn start_login(&self) {
        let attempt = self.attempt.get() + 1;
        self.attempt.set(attempt);
        self.tick.stop();
        *self.login.borrow_mut() = Login::Requesting;
        self.show_login();
        self.actions.begin_login(attempt);
    }

    fn cancel_login(&self) {
        self.attempt.set(self.attempt.get() + 1);
        self.tick.stop();
        self.actions.cancel_login();
        *self.login.borrow_mut() = Login::Closed;
        self.show_login();
    }

    pub fn login_code(&self, attempt: u64, code: DeviceCode) {
        if attempt != self.attempt.get() {
            return;
        }
        *self.login.borrow_mut() = Login::Code { code, started_ms: (self.now_ms)() };
        self.show_login();
        let weak = self.ui.clone();
        self.tick.start(slint::TimerMode::Repeated, Duration::from_secs(1), move || {
            if weak.upgrade().is_some() {
                ACCOUNTS.with(|c| {
                    if let Some(a) = c.borrow().as_ref() {
                        a.on_tick();
                    }
                });
            }
        });
    }

    fn on_tick(&self) {
        let left = self.login.borrow().left((self.now_ms)());
        if left == 0 && matches!(*self.login.borrow(), Login::Code { .. }) {
            self.tick.stop();
            self.actions.cancel_login();
            *self.login.borrow_mut() = Login::Expired;
        }
        self.show_login();
    }

    pub fn login_done(&self, attempt: u64, name: &str) {
        if attempt != self.attempt.get() {
            return;
        }
        self.tick.stop();
        *self.login.borrow_mut() = Login::Closed;
        self.show_login();
        self.toast_with("msSignedIn", name);
        self.actions.load();
    }

    pub fn login_failed(&self, attempt: u64, f: Failure) {
        if attempt != self.attempt.get() || f.kind == "cancelled" {
            return;
        }
        self.tick.stop();
        *self.login.borrow_mut() = if f.kind == "auth_device_code_expired" { Login::Expired } else { Login::Failed(f) };
        self.show_login();
    }

    fn show_login(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<AccountsState>();
        let login = self.login.borrow();
        st.set_login_open(*login != Login::Closed);
        match &*login {
            Login::Closed => {}
            Login::Requesting => st.set_login_phase(LoginPhase::Requesting),
            Login::Expired => st.set_login_phase(LoginPhase::Expired),
            Login::Failed(f) => {
                st.set_login_error(f.message.as_str().into());
                st.set_login_error_detail(f.detail.as_str().into());
                st.set_login_phase(LoginPhase::Failed);
            }
            Login::Code { code, .. } => {
                let left = login.left((self.now_ms)());
                st.set_user_code(code.user_code.as_str().into());
                st.set_verification_uri(code.verification_uri.as_str().into());
                st.set_verification_label(bare_uri(&code.verification_uri).into());
                st.set_expires_label(mmss(left).into());
                st.set_login_left(if code.expires_in == 0 { 0.0 } else { left as f32 / code.expires_in as f32 });
                st.set_login_phase(LoginPhase::Code);
            }
        }
    }
}

/// Pone `arg` en el único `{nombre}` de un texto del ARB.
pub fn fill(template: &str, arg: &str) -> String {
    match (template.find('{'), template.find('}')) {
        (Some(a), Some(b)) if a < b => format!("{}{arg}{}", &template[..a], &template[b + 1..]),
        _ => template.to_owned(),
    }
}

/// Cara 8 × 8 de una textura de skin en PNG (`None` si no es una skin).
pub fn face_from_png(bytes: &[u8]) -> Option<Vec<u8>> {
    skin::SkinTexture::decode(bytes).map(|t| skin::face_rgba(&t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombres_offline() {
        for ok in ["Pruebas_01", "abc", "A_b_C_1234567890", "  Steve  "] {
            assert!(valid_offline_name(ok), "{ok}");
        }
        for bad in ["ab", "A_b_C_12345678901", "con espacio", "ñandú", ""] {
            assert!(!valid_offline_name(bad), "{bad}");
        }
    }

    #[test]
    fn cuenta_atras() {
        assert_eq!(mmss(900), "15:00");
        assert_eq!(mmss(61), "1:01");
        assert_eq!(mmss(0), "0:00");
        let code = DeviceCode { user_code: "X".into(), verification_uri: "https://www.microsoft.com/link".into(), expires_in: 900 };
        let l = Login::Code { code, started_ms: 1_000 };
        assert_eq!(l.left(1_000), 900);
        assert_eq!(l.left(1_999), 900);
        assert_eq!(l.left(2_000), 899);
        assert_eq!(l.left(1_000_000), 0);
        assert_eq!(bare_uri("https://www.microsoft.com/link"), "www.microsoft.com/link");
    }

    #[test]
    fn textos_con_un_argumento() {
        assert_eq!(fill("Cuenta «{name}» quitada", "Pruebas_01"), "Cuenta «Pruebas_01» quitada");
        assert_eq!(fill("sin nada", "x"), "sin nada");
    }

    #[test]
    fn activa_es_la_marcada_o_la_primera() {
        let a = |id: &str, active| Account { id: id.into(), name: id.into(), microsoft: true, skin_url: None, active };
        assert_eq!(active(&[a("1", false), a("2", true)]).unwrap().id, "2");
        assert_eq!(active(&[a("1", false), a("2", false)]).unwrap().id, "1");
        assert!(active(&[]).is_none());
    }
}
