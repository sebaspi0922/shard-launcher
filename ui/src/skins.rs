//! Pantalla Skins (skins/skins_page.dart): perfil de la cuenta de Microsoft,
//! texturas, cambiar skin (PNG o URL, clásica o delgada), restablecer y
//! capas. La lógica de qué se puede aplicar está en `Model` (pura); `Skins`
//! la copia a `SkinsState` y pinta el visor con `skin_mesh`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use slint::{ComponentHandle, Image, ModelRc, SharedPixelBuffer, VecModel};

use crate::accounts::{Account, Accounts};
use crate::controller::Controller;
use crate::errors::Failure;
use crate::skin::SkinTexture;
use crate::skin_mesh::{self, Camera, Pixels};
use crate::{AccountChip, CapeItem, LoadState, MainWindow, SkinView, SkinsState, i18n};

#[derive(Clone, Debug, PartialEq)]
pub struct SkinRef {
    pub url: String,
    pub slim: bool,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CapeRef {
    pub id: String,
    pub url: String,
    pub alias: Option<String>,
    pub active: bool,
}

/// Skins y capas de una cuenta (`ProfileView`).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Profile {
    pub skins: Vec<SkinRef>,
    pub capes: Vec<CapeRef>,
}

impl Profile {
    pub fn from_core(p: &lcore::auth::mc::Profile) -> Self {
        Self {
            skins: p
                .skins
                .iter()
                .map(|s| SkinRef { url: s.url.clone(), slim: s.variant.eq_ignore_ascii_case("slim"), active: s.state == "ACTIVE" })
                .collect(),
            capes: p
                .capes
                .iter()
                .map(|c| CapeRef { id: c.id.clone(), url: c.url.clone(), alias: c.alias.clone(), active: c.state == "ACTIVE" })
                .collect(),
        }
    }

    /// La activa o, si ninguna lo está, la primera.
    pub fn active_skin(&self) -> Option<&SkinRef> {
        self.skins.iter().find(|s| s.active).or(self.skins.first())
    }

    pub fn active_cape(&self) -> Option<&CapeRef> {
        self.capes.iter().find(|c| c.active)
    }
}

/// Una textura pedida.
#[derive(Clone, Debug)]
pub enum Tex {
    Loading,
    Ready(Rc<SkinTexture>),
    /// No se pudo descargar o no es una imagen.
    Missing,
}

impl Tex {
    fn ready(&self) -> Option<&Rc<SkinTexture>> {
        match self {
            Tex::Ready(t) => Some(t),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum ProfileLoad {
    #[default]
    Loading,
    Ready(Profile),
    Failed(Failure),
}

/// Qué hace «Aplicar skin».
#[derive(Clone, Debug, PartialEq)]
pub enum Apply {
    Upload { path: String, slim: bool },
    Url { url: String, slim: bool },
    /// Solo cambia el modelo: la misma textura con otros brazos.
    Model { url: String, slim: bool },
    /// El PNG elegido no es una skin válida.
    InvalidPng,
    Nothing,
}

/// Estado de la pantalla (lo que en Flutter es `_SkinsBodyState` + providers).
#[derive(Default)]
pub struct Model {
    pub account: Option<Account>,
    pub accounts_loaded: bool,
    pub profile: ProfileLoad,
    pub textures: HashMap<String, Tex>,
    pub from_url: bool,
    /// PNG local elegido y su textura.
    pub file: Option<String>,
    pub local: Option<Tex>,
    pub url: String,
    /// Modelo elegido; `None` = el de la skin actual.
    pub slim: Option<bool>,
    pub busy: bool,
    /// Capa que se está aplicando (`""` = quitar capa).
    pub cape_busy: Option<String>,
}

impl Model {
    pub fn profile(&self) -> Option<&Profile> {
        match &self.profile {
            ProfileLoad::Ready(p) => Some(p),
            _ => None,
        }
    }

    pub fn slim(&self) -> bool {
        self.slim.or_else(|| self.profile().and_then(|p| p.active_skin()).map(|s| s.slim)).unwrap_or(false)
    }

    fn preview_tex(&self) -> Option<&Rc<SkinTexture>> {
        if self.from_url || self.file.is_none() {
            return None;
        }
        self.local.as_ref().and_then(Tex::ready)
    }

    /// Hay un PNG válido a la vista (en lugar de la skin actual).
    pub fn show_preview(&self) -> bool {
        self.preview_tex().is_some_and(|t| t.valid_skin_size())
    }

    /// El PNG elegido ya se leyó y no sirve.
    pub fn preview_error(&self) -> bool {
        !self.from_url && self.file.is_some() && !matches!(self.local, None | Some(Tex::Loading)) && !self.show_preview()
    }

    fn current(&self) -> Option<&SkinRef> {
        self.profile().and_then(|p| p.active_skin())
    }

    pub fn skin_tex(&self) -> Option<&Tex> {
        self.current().and_then(|s| self.textures.get(&s.url))
    }

    /// Textura que pinta el visor.
    pub fn shown(&self) -> Option<Rc<SkinTexture>> {
        if self.show_preview() {
            return self.preview_tex().cloned();
        }
        self.skin_tex().and_then(Tex::ready).cloned()
    }

    pub fn cape_tex(&self) -> Option<Rc<SkinTexture>> {
        let cape = self.profile()?.active_cape()?;
        self.textures.get(&cape.url).and_then(Tex::ready).cloned()
    }

    pub fn can_apply(&self) -> bool {
        !self.busy
            && ((!self.from_url && self.show_preview())
                || (self.from_url && !self.url.trim().is_empty())
                || self.current().is_some_and(|s| s.slim != self.slim()))
    }

    pub fn plan_apply(&self) -> Apply {
        let slim = self.slim();
        if !self.from_url
            && let Some(path) = &self.file
        {
            return if self.show_preview() { Apply::Upload { path: path.clone(), slim } } else { Apply::InvalidPng };
        }
        if self.from_url && !self.url.trim().is_empty() {
            return Apply::Url { url: self.url.trim().to_owned(), slim };
        }
        match self.current() {
            Some(s) => Apply::Model { url: s.url.clone(), slim },
            None => Apply::Nothing,
        }
    }
}

/// Lo que la pantalla pide fuera de la UI. Cada método entrega su resultado
/// con [`post`].
pub trait SkinActions {
    /// → [`Skins::set_profile`].
    fn load_profile(&self, account: &str);
    /// → [`Skins::texture_loaded`].
    fn load_texture(&self, url: &str);
    /// Diálogo de archivo → [`Skins::file_picked`].
    fn pick_file(&self, filter_label: &str);
    /// → [`Skins::local_loaded`].
    fn load_local(&self, path: &str);
    /// Subir, URL, restablecer o capa → [`Skins::changed`].
    fn apply(&self, account: &str, what: &Apply);
    fn reset(&self, account: &str);
    /// `None` = sin capa → [`Skins::cape_done`].
    fn cape(&self, account: &str, id: Option<&str>);
}

thread_local! {
    static SKINS: RefCell<Option<Rc<Skins>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador de Skins.
pub fn post(f: impl FnOnce(&Skins) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || {
        SKINS.with(|c| {
            if let Some(s) = c.borrow().as_ref() {
                f(s);
            }
        })
    });
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// Clave de la última imagen pintada por vista.
type RenderKey = (i32, u32, u32, i64, i64, i32);

pub struct Skins {
    ui: slint::Weak<MainWindow>,
    shell: Rc<Controller>,
    accounts: Rc<Accounts>,
    actions: Box<dyn SkinActions>,
    pub model: RefCell<Model>,
    revision: std::cell::Cell<i32>,
    renders: RefCell<HashMap<i32, (RenderKey, Image)>>,
    stages: RefCell<HashMap<(u32, u32), Image>>,
    capes: Rc<VecModel<CapeItem>>,
    cape_previews: RefCell<HashMap<(String, u32), Image>>,
}

fn image(p: &Pixels) -> Image {
    Image::from_rgba8(SharedPixelBuffer::clone_from_slice(&p.rgba, p.width, p.height))
}

/// Fondo de «estudio» (SkinStage): `RadialGradient` centrado en (0, −0,2),
/// radio 0,9 del lado corto, del acento al 16 % a la superficie 1. Los dos
/// colores son blancos en Grafito: solo cambia el alfa.
pub fn stage_pixels(w: f64, h: f64, scale: f64, accent: [u8; 3], from_alpha: f64, to_alpha: f64) -> Pixels {
    let (pw, ph) = ((w * scale).round().max(1.0) as u32, (h * scale).round().max(1.0) as u32);
    let (cx, cy) = (w * scale / 2.0, h * scale * 0.4);
    let r = 0.9 * (w.min(h)) * scale;
    let mut rgba = Vec::with_capacity((pw * ph * 4) as usize);
    for y in 0..ph {
        for x in 0..pw {
            let d = ((x as f64 + 0.5 - cx).powi(2) + (y as f64 + 0.5 - cy).powi(2)).sqrt();
            let t = (d / r).min(1.0);
            let a = from_alpha + (to_alpha - from_alpha) * t;
            rgba.extend_from_slice(&[accent[0], accent[1], accent[2], (a * 255.0).round() as u8]);
        }
    }
    Pixels { width: pw, height: ph, rgba }
}

impl Skins {
    pub fn new(ui: &MainWindow, shell: Rc<Controller>, accounts: Rc<Accounts>, actions: Box<dyn SkinActions>) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            shell,
            accounts: accounts.clone(),
            actions,
            model: RefCell::default(),
            revision: Default::default(),
            renders: RefCell::default(),
            stages: RefCell::default(),
            capes: Rc::new(VecModel::default()),
            cape_previews: RefCell::default(),
        });
        SKINS.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<SkinsState>();
        st.set_capes(ModelRc::from(me.capes.clone()));
        let weak = Rc::downgrade(&me);
        st.on_render(move |view, w, h, yaw, pitch, _| weak.upgrade().map(|s| s.render(view, w, h, yaw, pitch)).unwrap_or_default());
        let weak = Rc::downgrade(&me);
        st.on_stage(move |w, h| weak.upgrade().map(|s| s.stage(w, h)).unwrap_or_default());
        let weak = Rc::downgrade(&me);
        st.on_source_changed(move |url| {
            if let Some(s) = weak.upgrade() {
                s.model.borrow_mut().from_url = url;
                s.refresh();
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_url_edited(move |t| {
            if let Some(s) = weak.upgrade() {
                s.model.borrow_mut().url = t.to_string();
                s.refresh();
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_slim_changed(move |v| {
            if let Some(s) = weak.upgrade() {
                s.model.borrow_mut().slim = Some(v);
                s.refresh();
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_pick_file(move || {
            if let Some(s) = weak.upgrade() {
                s.actions.pick_file(&i18n::text(&s.shell.lang(), "skinsPngFiles"));
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_clear_file(move || {
            if let Some(s) = weak.upgrade() {
                let mut m = s.model.borrow_mut();
                m.file = None;
                m.local = None;
                drop(m);
                s.refresh();
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_apply(move || {
            if let Some(s) = weak.upgrade() {
                s.apply();
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_reset(move || {
            if let Some(s) = weak.upgrade()
                && let Some(id) = s.account_id()
            {
                s.model.borrow_mut().busy = true;
                s.refresh();
                s.actions.reset(&id);
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_cape(move |id| {
            if let Some(s) = weak.upgrade() {
                s.select_cape(&id);
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_retry(move || {
            if let Some(s) = weak.upgrade() {
                s.load_profile();
            }
        });
        let weak = Rc::downgrade(&me);
        accounts.on_active(move |a, chip| {
            if let Some(s) = weak.upgrade() {
                s.set_account(a.cloned(), chip);
            }
        });
        me
    }

    fn lang(&self) -> String {
        self.shell.lang()
    }

    fn account_id(&self) -> Option<String> {
        self.model.borrow().account.as_ref().map(|a| a.id.clone())
    }

    /// Cuenta activa nueva (o la misma con otra cara).
    fn set_account(&self, a: Option<Account>, chip: AccountChip) {
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<SkinsState>().set_chip(chip);
        }
        let changed = {
            let mut m = self.model.borrow_mut();
            m.accounts_loaded = true;
            let changed = m.account.as_ref().map(|x| (&x.id, x.microsoft)) != a.as_ref().map(|x| (&x.id, x.microsoft));
            m.account = a;
            if changed {
                m.profile = ProfileLoad::Loading;
                m.file = None;
                m.local = None;
                m.url.clear();
                m.slim = None;
                m.from_url = false;
            }
            changed
        };
        if changed {
            self.load_profile();
        }
        self.refresh();
    }

    fn load_profile(&self) {
        let account = self.model.borrow().account.clone();
        if let Some(a) = account.filter(|a| a.microsoft) {
            self.model.borrow_mut().profile = ProfileLoad::Loading;
            self.refresh();
            self.actions.load_profile(&a.id);
        }
    }

    pub fn set_profile(&self, account: &str, r: Result<Profile, Failure>) {
        if self.account_id().as_deref() != Some(account) {
            return;
        }
        match r {
            Ok(p) => self.use_profile(p),
            Err(f) => {
                self.model.borrow_mut().profile = ProfileLoad::Failed(f);
                self.refresh();
            }
        }
    }

    /// Perfil nuevo: pide las texturas que falten.
    fn use_profile(&self, p: Profile) {
        let mut want: Vec<String> = p.active_skin().map(|s| s.url.clone()).into_iter().collect();
        want.extend(p.capes.iter().map(|c| c.url.clone()));
        {
            let mut m = self.model.borrow_mut();
            m.profile = ProfileLoad::Ready(p);
            want.retain(|u| !m.textures.contains_key(u));
            for u in &want {
                m.textures.insert(u.clone(), Tex::Loading);
            }
        }
        for u in &want {
            self.actions.load_texture(u);
        }
        self.bump();
    }

    pub fn texture_loaded(&self, url: &str, tex: Option<SkinTexture>) {
        self.model.borrow_mut().textures.insert(url.to_owned(), tex.map_or(Tex::Missing, |t| Tex::Ready(Rc::new(t))));
        self.bump();
    }

    pub fn file_picked(&self, path: Option<String>) {
        let Some(path) = path else { return };
        {
            let mut m = self.model.borrow_mut();
            m.file = Some(path.clone());
            m.local = Some(Tex::Loading);
        }
        self.refresh();
        self.actions.load_local(&path);
    }

    pub fn local_loaded(&self, path: &str, tex: Option<SkinTexture>) {
        {
            let mut m = self.model.borrow_mut();
            if m.file.as_deref() != Some(path) {
                return;
            }
            m.local = Some(tex.map_or(Tex::Missing, |t| Tex::Ready(Rc::new(t))));
        }
        self.bump();
    }

    fn apply(&self) {
        let Some(id) = self.account_id() else { return };
        let plan = self.model.borrow().plan_apply();
        match plan {
            Apply::Nothing => {}
            Apply::InvalidPng => self.shell.toast(&i18n::text(&self.lang(), "skinsInvalidPng"), true),
            what => {
                self.model.borrow_mut().busy = true;
                self.refresh();
                self.actions.apply(&id, &what);
            }
        }
    }

    /// Resultado de aplicar o restablecer la skin.
    pub fn changed(&self, reset: bool, r: Result<Profile, Failure>) {
        self.model.borrow_mut().busy = false;
        match r {
            Ok(p) => {
                if !reset {
                    let mut m = self.model.borrow_mut();
                    m.file = None;
                    m.local = None;
                    m.url.clear();
                    m.slim = None;
                }
                self.use_profile(p);
                self.shell.toast(&i18n::text(&self.lang(), if reset { "skinsResetDone" } else { "skinsApplied" }), false);
                // La cara del avatar sale de la cuenta guardada.
                self.accounts.reload();
            }
            Err(f) => {
                self.shell.toast(&f.message, true);
                self.refresh();
            }
        }
    }

    fn select_cape(&self, id: &str) {
        let Some(account) = self.account_id() else { return };
        {
            let mut m = self.model.borrow_mut();
            if m.cape_busy.is_some() {
                return;
            }
            m.cape_busy = Some(id.to_owned());
        }
        self.refresh();
        self.actions.cape(&account, (!id.is_empty()).then_some(id));
    }

    pub fn cape_done(&self, r: Result<Profile, Failure>) {
        self.model.borrow_mut().cape_busy = None;
        match r {
            Ok(p) => self.use_profile(p),
            Err(f) => {
                self.shell.toast(&f.message, true);
                self.refresh();
            }
        }
    }

    /// Cambió lo que pinta el visor.
    fn bump(&self) {
        self.revision.set(self.revision.get().wrapping_add(1));
        self.refresh();
    }

    fn scale(&self) -> f64 {
        self.ui.upgrade().map(|u| u.window().scale_factor()).unwrap_or(1.0) as f64
    }

    fn render(&self, view: SkinView, w: f32, h: f32, yaw: f32, pitch: f32) -> Image {
        let scale = self.scale();
        let m = self.model.borrow();
        let Some(skin) = m.shown() else { return Image::default() };
        let (cam, cape, shadow, slot) = match view {
            SkinView::Main => (Camera { yaw: yaw as f64, pitch: pitch as f64 }, m.cape_tex(), true, 0),
            SkinView::Front => (Camera { yaw: 0.0, pitch: 0.0 }, None, false, 1),
            SkinView::Back => (Camera { yaw: std::f64::consts::PI, pitch: 0.0 }, m.cape_tex(), false, 2),
        };
        let key: RenderKey = (
            slot,
            (w as f64 * scale * 4.0) as u32,
            (h as f64 * scale * 4.0) as u32,
            (cam.yaw * 1e6) as i64,
            (cam.pitch * 1e6) as i64,
            self.revision.get(),
        );
        if let Some((k, img)) = self.renders.borrow().get(&slot)
            && *k == key
        {
            return img.clone();
        }
        let px = skin_mesh::render(&skin, cape.as_deref(), m.slim(), cam, w as f64, h as f64, scale, shadow);
        let img = image(&px);
        self.renders.borrow_mut().insert(slot, (key, img.clone()));
        img
    }

    fn stage(&self, w: f32, h: f32) -> Image {
        let scale = self.scale();
        let key = ((w as f64 * scale * 4.0) as u32, (h as f64 * scale * 4.0) as u32);
        if let Some(img) = self.stages.borrow().get(&key) {
            return img.clone();
        }
        let img = match self.ui.upgrade() {
            Some(ui) => {
                let pal = ui.global::<crate::Palette>();
                let (acc, s1) = (pal.get_accent(), pal.get_surface1());
                let to = s1.alpha() as f64 / 255.0;
                image(&stage_pixels(w as f64, h as f64, scale, [acc.red(), acc.green(), acc.blue()], 0.16, to))
            }
            None => Image::default(),
        };
        self.stages.borrow_mut().insert(key, img.clone());
        img
    }

    fn cape_preview(&self, url: &str, tex: &SkinTexture) -> Image {
        let scale = self.scale();
        let key = (url.to_owned(), (scale * 100.0) as u32);
        if let Some(img) = self.cape_previews.borrow().get(&key) {
            return img.clone();
        }
        let img = image(&skin_mesh::cape_preview(tex, 57.5, 92.0, scale));
        self.cape_previews.borrow_mut().insert(key, img.clone());
        img
    }

    /// Copia el modelo a la UI.
    pub fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<SkinsState>();
        let lang = self.lang();
        let m = self.model.borrow();
        st.set_has_account(m.account.is_some());
        st.set_accounts_loading(!m.accounts_loaded);
        st.set_microsoft(m.account.as_ref().is_some_and(|a| a.microsoft));
        match &m.profile {
            ProfileLoad::Loading => st.set_state(LoadState::Loading),
            ProfileLoad::Ready(_) => st.set_state(LoadState::Ready),
            ProfileLoad::Failed(f) => {
                st.set_error_message(f.message.as_str().into());
                st.set_error_detail(f.detail.as_str().into());
                st.set_state(LoadState::Failed);
            }
        }
        let shown = m.shown().is_some();
        st.set_has_texture(shown);
        st.set_texture_loading(!shown && matches!(m.skin_tex(), Some(Tex::Loading)));
        st.set_preview(m.show_preview());
        st.set_slim(m.slim());
        st.set_from_url(m.from_url);
        let name = m.file.as_deref().map(|f| f.rsplit(['/', '\\']).next().unwrap_or(f)).unwrap_or("");
        st.set_file_name(name.into());
        st.set_preview_error(m.preview_error());
        if st.get_url() != m.url.as_str() {
            st.set_url(m.url.as_str().into());
        }
        st.set_busy(m.busy);
        st.set_can_apply(m.can_apply());
        st.set_cape_busy(m.cape_busy.is_some());
        st.set_revision(self.revision.get());
        let capes = m.profile().map(|p| p.capes.clone()).unwrap_or_default();
        st.set_capes_count(capes.len() as i32);
        let mut rows = vec![CapeItem {
            id: "".into(),
            label: i18n::text(&lang, "skinsNoCape").into(),
            selected: !capes.iter().any(|c| c.active),
            busy: m.cape_busy.as_deref() == Some(""),
            ..Default::default()
        }];
        for c in &capes {
            let tex = m.textures.get(&c.url);
            let preview = tex.and_then(Tex::ready).map(|t| self.cape_preview(&c.url, t));
            rows.push(CapeItem {
                id: c.id.as_str().into(),
                label: c.alias.clone().unwrap_or_else(|| i18n::text(&lang, "skinsCape")).into(),
                selected: c.active,
                busy: m.cape_busy.as_deref() == Some(c.id.as_str()),
                loading: matches!(tex, Some(Tex::Loading)),
                has_preview: preview.is_some(),
                preview: preview.unwrap_or_default(),
            });
        }
        if capes.is_empty() {
            rows.clear();
        }
        self.capes.set_vec(rows);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::{sample_cape, sample_skin};

    fn model() -> Model {
        let mut m = Model {
            profile: ProfileLoad::Ready(Profile {
                skins: vec![SkinRef { url: "s".into(), slim: true, active: true }],
                capes: vec![CapeRef { id: "c1".into(), url: "c".into(), alias: None, active: true }],
            }),
            ..Default::default()
        };
        m.textures.insert("s".into(), Tex::Ready(Rc::new(sample_skin())));
        m
    }

    #[test]
    fn sin_cambios_no_se_puede_aplicar() {
        let m = model();
        assert!(m.slim());
        assert!(!m.can_apply());
        assert_eq!(m.plan_apply(), Apply::Model { url: "s".into(), slim: true });
    }

    #[test]
    fn cambiar_solo_el_modelo() {
        let mut m = model();
        m.slim = Some(false);
        assert!(m.can_apply());
        assert_eq!(m.plan_apply(), Apply::Model { url: "s".into(), slim: false });
        m.busy = true;
        assert!(!m.can_apply());
    }

    #[test]
    fn url_y_png() {
        let mut m = model();
        m.from_url = true;
        m.url = "  ".into();
        assert!(!m.can_apply());
        m.url = " https://x/skin.png ".into();
        assert_eq!(m.plan_apply(), Apply::Url { url: "https://x/skin.png".into(), slim: true });

        let mut m = model();
        m.file = Some("/tmp/a.png".into());
        m.local = Some(Tex::Loading);
        assert!(!m.can_apply() && !m.preview_error());
        // Una capa (64 × 32 pero no de skin… sí mide 64 × 32): vale como plantilla antigua.
        m.local = Some(Tex::Ready(Rc::new(SkinTexture { width: 32, height: 32, argb: vec![0; 1024] })));
        assert!(m.preview_error() && !m.can_apply());
        assert_eq!(m.plan_apply(), Apply::InvalidPng);
        m.local = Some(Tex::Ready(Rc::new(sample_cape(0xFF000000, 0xFFFFFFFF))));
        assert!(m.show_preview() && m.can_apply());
        assert_eq!(m.plan_apply(), Apply::Upload { path: "/tmp/a.png".into(), slim: true });
        // Con «URL» elegido, el PNG no cuenta.
        m.from_url = true;
        assert!(!m.show_preview());
    }

    #[test]
    fn fondo_de_estudio() {
        let p = stage_pixels(100.0, 200.0, 1.0, [255, 255, 255], 0.16, 0.05);
        assert_eq!((p.width, p.height), (100, 200));
        let a = |x: u32, y: u32| p.rgba[((y * 100 + x) * 4 + 3) as usize];
        // Centro en (50, 80): 16 %; lejos: 5 %.
        assert_eq!(a(50, 80), 41);
        assert_eq!(a(0, 199), 13);
    }
}
