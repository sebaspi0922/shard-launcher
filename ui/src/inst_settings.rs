//! Pestaña Ajustes de la instancia (instance_settings_tab.dart): nombre,
//! memoria, argumentos de la JVM y ventana del juego.
//!
//! [`InstSettings`] corre en el hilo de la UI. El formulario se edita en
//! `InstSettingsState` (estado de vista); en cada cambio Rust lo lee, lo
//! valida (anchos de solo dígitos), decide si hay cambios sin guardar y, al
//! guardar, pide al núcleo `update_instance` con un [`InstSettingsActions`]
//! ([`CoreInstSettings`] en la app, `fixtures::Recorder` en los tests).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use lcore::instance::Instance;
use slint::ComponentHandle;

use crate::backend::Backend;
use crate::controller::Controller;
use crate::errors::Failure;
use crate::fixtures::Recorder;
use crate::instance::{DetailEvent, InstanceDetail};
use crate::{InstSettingsState, InstanceTab, MainWindow, i18n};

/// Ajustes globales que la pestaña muestra cuando la instancia usa los
/// suyos, y la RAM del equipo (tope del deslizador).
#[derive(Clone, Debug, PartialEq)]
pub struct Globals {
    pub memory_mb: u32,
    pub jvm_args: Vec<String>,
    pub total_mb: u64,
}

impl Default for Globals {
    /// Lo que Flutter usa mientras no llegan (`?? 4096`).
    fn default() -> Self {
        Self { memory_mb: 4096, jvm_args: Vec::new(), total_mb: 0 }
    }
}

/// Argumentos de JVM separados por espacios (sin comillas), `splitArgs`.
pub fn split_args(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_owned).collect()
}

/// Campo numérico: solo dígitos y como mucho 5 (`digitsOnly` +
/// `LengthLimitingTextInputFormatter(5)`).
pub fn digits(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).take(5).collect()
}

/// `int.tryParse` de un campo ya filtrado.
fn number(s: &str) -> Option<u32> {
    s.trim().parse().ok()
}

/// El formulario tal como se edita.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub name: String,
    pub custom_memory: bool,
    pub memory_mb: u32,
    pub custom_jvm: bool,
    pub jvm: String,
    pub width: String,
    pub height: String,
    pub fullscreen: bool,
}

/// Lo que se guarda (`None` en memoria/JVM = los globales).
#[derive(Clone, Debug, PartialEq)]
pub struct Values {
    pub name: String,
    pub memory_mb: Option<u32>,
    pub jvm_args: Option<Vec<String>>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fullscreen: bool,
}

impl Values {
    /// Lo que hace `update_instance_settings` del bridge.
    pub fn apply(&self, i: &mut Instance) {
        i.name = self.name.clone();
        i.java.memory_mb = self.memory_mb;
        i.java.jvm_args = self.jvm_args.clone();
        i.window.width = self.width;
        i.window.height = self.height;
        i.window.fullscreen = self.fullscreen;
    }
}

impl Form {
    /// `_load`: la instancia, con la memoria global si no tiene la suya.
    pub fn load(i: &Instance, global_memory: u32) -> Self {
        Self {
            name: i.name.clone(),
            custom_memory: i.java.memory_mb.is_some(),
            memory_mb: i.java.memory_mb.unwrap_or(global_memory),
            custom_jvm: i.java.jvm_args.is_some(),
            jvm: i.java.jvm_args.as_deref().unwrap_or_default().join(" "),
            width: i.window.width.map(|w| w.to_string()).unwrap_or_default(),
            height: i.window.height.map(|h| h.to_string()).unwrap_or_default(),
            fullscreen: i.window.fullscreen,
        }
    }

    pub fn values(&self) -> Values {
        Values {
            name: self.name.trim().to_owned(),
            memory_mb: self.custom_memory.then_some(self.memory_mb),
            jvm_args: self.custom_jvm.then(|| split_args(&self.jvm)),
            width: number(&self.width),
            height: number(&self.height),
            fullscreen: self.fullscreen,
        }
    }

    /// `_dirty`: ¿lo que se guardaría difiere de la instancia?
    pub fn dirty(&self, base: &Instance) -> bool {
        let v = self.values();
        v.name != base.name
            || v.memory_mb != base.java.memory_mb
            || v.jvm_args != base.java.jvm_args
            || v.width != base.window.width
            || v.height != base.window.height
            || v.fullscreen != base.window.fullscreen
    }

    /// Se puede guardar (`_save` no hace nada con el nombre vacío).
    pub fn can_save(&self) -> bool {
        !self.name.trim().is_empty()
    }
}

/// Lo que la pestaña pide fuera de la UI.
pub trait InstSettingsActions {
    /// Ajustes globales y RAM → [`InstSettings::globals_loaded`].
    fn load_globals(&self);
    /// `update_instance` → [`InstSettings::saved`] (y recargar la lista).
    fn save(&self, id: &str, values: &Values);
}

thread_local! {
    static SETTINGS: RefCell<Option<Rc<InstSettings>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador de la pestaña.
pub fn post(f: impl FnOnce(&InstSettings) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || {
        SETTINGS.with(|c| {
            if let Some(s) = c.borrow().as_ref() {
                f(s);
            }
        })
    });
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// Instancia completa abierta en el detalle, si ya llegó.
fn current_detail() -> Option<crate::instance::Detail> {
    let mut out = None;
    crate::instance::with(|d| out = d.detail());
    out
}

pub struct InstSettings {
    ui: slint::Weak<MainWindow>,
    ctl: Rc<Controller>,
    actions: Box<dyn InstSettingsActions>,
    globals: RefCell<Globals>,
    /// Instancia sobre la que se editan los cambios (`_base`).
    base: RefCell<Option<Instance>>,
    form: RefCell<Option<Form>>,
    saving: Cell<bool>,
}

impl InstSettings {
    pub fn new(ui: &MainWindow, detail: &InstanceDetail, actions: Box<dyn InstSettingsActions>) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            ctl: detail.controller().clone(),
            actions,
            globals: RefCell::default(),
            base: RefCell::default(),
            form: RefCell::default(),
            saving: Cell::new(false),
        });
        SETTINGS.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<InstSettingsState>();
        let weak = Rc::downgrade(&me);
        st.on_edited(move || {
            if let Some(s) = weak.upgrade() {
                s.edited();
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_discard(move || {
            if let Some(s) = weak.upgrade() {
                let base = s.base.borrow().clone();
                if let Some(i) = base {
                    s.load(&i);
                }
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_save(move || {
            if let Some(s) = weak.upgrade() {
                s.save();
            }
        });
        let weak = Rc::downgrade(&me);
        detail.on(move |e| {
            let Some(s) = weak.upgrade() else { return };
            match e {
                // La pestaña se crea de nuevo cada vez que se abre: carga la
                // instancia tal como está (sin cambios a medias).
                DetailEvent::Shown { tab: InstanceTab::Settings, .. } => {
                    s.actions.load_globals();
                    match current_detail() {
                        Some(d) => s.load(&d.instance),
                        None => s.forget(),
                    }
                }
                DetailEvent::Closed => s.forget(),
                // `didUpdateWidget`: otra instancia (o formulario vacío) se
                // carga siempre; la misma, si cambió fuera y no hay cambios
                // propios pendientes.
                DetailEvent::Loaded(d) => {
                    let base = s.base.borrow().clone();
                    let other = base.as_ref().is_none_or(|b| b.id != d.instance.id);
                    let pending = s.saving.get() || s.is_dirty();
                    if other || (!pending && base.as_ref() != Some(&d.instance)) {
                        s.load(&d.instance);
                    }
                }
                _ => {}
            }
        });
        me
    }

    /// Formulario actual (tests).
    pub fn form(&self) -> Option<Form> {
        self.form.borrow().clone()
    }

    pub fn is_dirty(&self) -> bool {
        match (&*self.form.borrow(), &*self.base.borrow()) {
            (Some(f), Some(b)) => f.dirty(b),
            _ => false,
        }
    }

    /// Resultado de [`InstSettingsActions::load_globals`].
    pub fn globals_loaded(&self, g: Globals) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<InstSettingsState>();
        st.set_global_memory_mb(g.memory_mb as i32);
        st.set_global_jvm(g.jvm_args.join(" ").into());
        st.set_total_mb(g.total_mb.min(i32::MAX as u64) as i32);
        // Sin memoria propia, el deslizador arranca en la global.
        if let Some(f) = self.form.borrow_mut().as_mut()
            && !f.custom_memory
        {
            f.memory_mb = g.memory_mb;
            st.set_memory_mb(f.memory_mb as i32);
        }
        *self.globals.borrow_mut() = g;
    }

    /// `_load`: el formulario vuelve a ser la instancia.
    fn load(&self, i: &Instance) {
        let form = Form::load(i, self.globals.borrow().memory_mb);
        *self.base.borrow_mut() = Some(i.clone());
        if let Some(ui) = self.ui.upgrade() {
            let st = ui.global::<InstSettingsState>();
            st.set_name(form.name.as_str().into());
            st.set_custom_memory(form.custom_memory);
            st.set_memory_mb(form.memory_mb as i32);
            st.set_custom_jvm(form.custom_jvm);
            st.set_jvm(form.jvm.as_str().into());
            st.set_width(form.width.as_str().into());
            st.set_height(form.height.as_str().into());
            st.set_fullscreen(form.fullscreen);
            st.set_error("".into());
            st.set_error_detail("".into());
            st.set_ready(true);
        }
        *self.form.borrow_mut() = Some(form);
        self.refresh();
    }

    fn forget(&self) {
        *self.base.borrow_mut() = None;
        *self.form.borrow_mut() = None;
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<InstSettingsState>().set_ready(false);
        }
        self.refresh();
    }

    /// Algo cambió en el formulario: leerlo, filtrar los números y, al
    /// pasar la JVM a propia con el campo vacío, rellenarla con la global.
    fn edited(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<InstSettingsState>();
        let prev = self.form.borrow().clone();
        let mut f = Form {
            name: st.get_name().into(),
            custom_memory: st.get_custom_memory(),
            memory_mb: st.get_memory_mb().max(0) as u32,
            custom_jvm: st.get_custom_jvm(),
            jvm: st.get_jvm().into(),
            width: digits(&st.get_width()),
            height: digits(&st.get_height()),
            fullscreen: st.get_fullscreen(),
        };
        if f.width != st.get_width().as_str() {
            st.set_width(f.width.as_str().into());
        }
        if f.height != st.get_height().as_str() {
            st.set_height(f.height.as_str().into());
        }
        if f.custom_jvm && !prev.as_ref().is_some_and(|p| p.custom_jvm) && f.jvm.is_empty() {
            f.jvm = self.globals.borrow().jvm_args.join(" ");
            st.set_jvm(f.jvm.as_str().into());
        }
        *self.form.borrow_mut() = Some(f);
        self.refresh();
    }

    fn save(&self) {
        let form = self.form.borrow().clone();
        let id = current_detail().map(|d| d.instance.id);
        let (Some(form), Some(id)) = (form, id) else { return };
        if !form.can_save() || self.saving.get() {
            return;
        }
        self.saving.set(true);
        if let Some(ui) = self.ui.upgrade() {
            let st = ui.global::<InstSettingsState>();
            st.set_error("".into());
            st.set_error_detail("".into());
        }
        self.refresh();
        self.actions.save(&id, &form.values());
    }

    /// Resultado de [`InstSettingsActions::save`]: el formulario pasa a ser
    /// lo guardado, aviso «Cambios guardados» y el detalle se vuelve a leer.
    pub fn saved(&self, id: &str, result: Result<Instance, Failure>) {
        self.saving.set(false);
        let current = self.base.borrow().as_ref().map(|i| i.id.clone());
        if current.as_deref() != Some(id) {
            self.refresh();
            return;
        }
        match result {
            Ok(i) => {
                self.load(&i);
                self.ctl.toast(&i18n::text(&self.ctl.lang(), "saved"), false);
                crate::instance::with(|d| d.reload());
            }
            Err(f) => {
                if let Some(ui) = self.ui.upgrade() {
                    let st = ui.global::<InstSettingsState>();
                    st.set_error(f.message.into());
                    st.set_error_detail(f.detail.into());
                }
                self.refresh();
            }
        }
    }

    fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<InstSettingsState>();
        st.set_dirty(self.is_dirty());
        st.set_saving(self.saving.get());
        st.set_can_save(self.form.borrow().as_ref().is_some_and(Form::can_save));
    }
}

/// [`InstSettingsActions`] contra el núcleo.
pub struct CoreInstSettings(pub Backend);

impl InstSettingsActions for CoreInstSettings {
    fn load_globals(&self) {
        let core = self.0.core.clone();
        self.0.rt.spawn(async move {
            let s = core.settings().await;
            let total_mb = {
                let mut sys = sysinfo::System::new();
                sys.refresh_memory();
                sys.total_memory() / (1024 * 1024)
            };
            let g = Globals { memory_mb: s.default_memory_mb, jvm_args: s.default_jvm_args, total_mb };
            post(move |x| x.globals_loaded(g));
        });
    }

    fn save(&self, id: &str, values: &Values) {
        let b = self.0.clone();
        let (id, values) = (id.to_owned(), values.clone());
        self.0.rt.spawn(async move {
            let r = b.core.update_instance(&id, |i| values.apply(i)).await;
            if r.is_ok() {
                // Nombre y ajustes nuevos en la lista y en la cabecera.
                b.load();
            }
            post(move |x| {
                let lang = x.ctl.lang();
                x.saved(&id, r.map_err(|e| Failure::new(&lang, &e)));
            });
        });
    }
}

impl InstSettingsActions for Recorder {
    fn load_globals(&self) {
        self.push("settings globals".into());
    }
    fn save(&self, id: &str, v: &Values) {
        self.push(format!(
            "settings save {id} name={} memory={:?} jvm={:?} window={:?}x{:?} fullscreen={}",
            v.name, v.memory_mb, v.jvm_args, v.width, v.height, v.fullscreen
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amigos() -> Instance {
        crate::fixtures::detail::sample_detail("survival-amigos").unwrap().instance
    }

    #[test]
    fn argumentos_y_digitos() {
        assert_eq!(split_args("  -Xss2m   -XX:+UseG1GC\t-Dx=1 \n"), ["-Xss2m", "-XX:+UseG1GC", "-Dx=1"]);
        assert!(split_args("   ").is_empty());
        assert_eq!(digits("1a9b20x"), "1920");
        assert_eq!(digits("1234567"), "12345");
        assert_eq!(digits(""), "");
    }

    #[test]
    fn carga_la_instancia_y_no_hay_cambios() {
        let i = amigos();
        let f = Form::load(&i, 4096);
        assert_eq!(f.name, "Survival con amigos");
        assert!(f.custom_memory);
        assert_eq!(f.memory_mb, 6144);
        assert_eq!(f.jvm, "-XX:+UseG1GC -XX:MaxGCPauseMillis=50");
        assert_eq!((f.width.as_str(), f.height.as_str()), ("1600", "900"));
        assert!(!f.dirty(&i));
        // Sin memoria propia: la global en el deslizador, y sigue sin cambios.
        let mut j = i.clone();
        j.java.memory_mb = None;
        j.java.jvm_args = None;
        j.window = Default::default();
        let f = Form::load(&j, 8192);
        assert_eq!((f.custom_memory, f.memory_mb), (false, 8192));
        assert_eq!((f.custom_jvm, f.jvm.as_str(), f.width.as_str()), (false, "", ""));
        assert!(!f.dirty(&j));
    }

    #[test]
    fn que_cuenta_como_cambio() {
        let i = amigos();
        let base = Form::load(&i, 4096);
        let changed = |m: fn(&mut Form)| {
            let mut f = base.clone();
            m(&mut f);
            f.dirty(&i)
        };
        assert!(!changed(|f| f.name = "  Survival con amigos ".into()), "el nombre se compara sin espacios");
        assert!(changed(|f| f.name = "Otro".into()));
        assert!(changed(|f| f.memory_mb = 4096));
        assert!(changed(|f| f.custom_memory = false));
        assert!(!changed(|f| f.jvm = " -XX:+UseG1GC   -XX:MaxGCPauseMillis=50 ".into()), "mismos argumentos");
        assert!(changed(|f| f.custom_jvm = false));
        assert!(changed(|f| f.width = "".into()));
        assert!(!changed(|f| f.height = "0900".into()), "int.tryParse(\"0900\") = 900");
        assert!(changed(|f| f.fullscreen = true));
        // Memoria global: el valor del deslizador no cuenta.
        let mut f = base.clone();
        f.custom_memory = false;
        let mut g = f.clone();
        g.memory_mb = 1024;
        assert_eq!(f.values(), g.values());
    }

    #[test]
    fn guardar_aplica_lo_del_bridge() {
        let mut i = amigos();
        let mut f = Form::load(&i, 4096);
        f.name = " Nuevo ".into();
        f.custom_memory = false;
        f.jvm = "-Xmx1G".into();
        f.width = "".into();
        f.fullscreen = true;
        assert!(f.can_save());
        f.values().apply(&mut i);
        assert_eq!(i.name, "Nuevo");
        assert_eq!(i.java.memory_mb, None);
        assert_eq!(i.java.jvm_args, Some(vec!["-Xmx1G".to_owned()]));
        assert_eq!((i.window.width, i.window.height, i.window.fullscreen), (None, Some(900), true));
        assert!(!f.dirty(&i));
        f.name = "   ".into();
        assert!(!f.can_save());
    }
}
