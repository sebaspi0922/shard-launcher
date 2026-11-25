//! Clave de API de CurseForge (curseforge_key_dialog.dart y
//! `curseforgeKeyProvider`): saber si hay una, guardarla desde el diálogo
//! (`CurseforgeKey` en `components/curseforge_key.slint`) y borrarla. La
//! clave va al llavero del sistema por el núcleo y nunca vuelve a la UI.
//!
//! Lo usan Contenido (estado vacío de CurseForge) y Ajustes → Integraciones.
//! Quien necesite saber si hay clave se apunta con [`Keys::on_change`].

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use slint::ComponentHandle;

use crate::controller::Controller;
use crate::errors::Failure;
use crate::{CurseforgeKey, MainWindow, i18n};

/// Lo que se pide al núcleo; cada método entrega su resultado con [`post`].
pub trait KeyActions {
    /// ¿Hay clave? → [`Keys::state`].
    fn check(&self);
    /// → [`Keys::saved`].
    fn set(&self, key: &str);
    /// → [`Keys::cleared`].
    fn clear(&self);
}

thread_local! {
    static KEYS: RefCell<Option<Rc<Keys>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador de la clave.
pub fn post(f: impl FnOnce(&Keys) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || {
        KEYS.with(|k| {
            if let Some(k) = k.borrow().as_ref() {
                f(k);
            }
        })
    });
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

type Listener = Box<dyn Fn(bool)>;

pub struct Keys {
    ui: slint::Weak<MainWindow>,
    shell: Rc<Controller>,
    actions: Box<dyn KeyActions>,
    /// `None` = aún sin saber.
    has: Cell<Option<bool>>,
    listeners: RefCell<Vec<Listener>>,
}

impl Keys {
    /// Crea el controlador, lo registra para [`post`] y pregunta si hay clave.
    pub fn new(ui: &MainWindow, shell: Rc<Controller>, actions: Box<dyn KeyActions>) -> Rc<Self> {
        let me = Rc::new(Self { ui: ui.as_weak(), shell, actions, has: Cell::new(None), listeners: RefCell::default() });
        KEYS.with(|k| *k.borrow_mut() = Some(me.clone()));
        let ck = ui.global::<CurseforgeKey>();
        let weak = Rc::downgrade(&me);
        ck.on_open_dialog(move || {
            if let Some(k) = weak.upgrade() {
                k.open_dialog();
            }
        });
        let weak = Rc::downgrade(&me);
        ck.on_save(move |key| {
            if let Some(k) = weak.upgrade() {
                k.save(&key);
            }
        });
        let weak = Rc::downgrade(&me);
        ck.on_clear(move || {
            if let Some(k) = weak.upgrade() {
                k.actions.clear();
            }
        });
        me.actions.check();
        me
    }

    /// `None` mientras no se sabe.
    pub fn has_key(&self) -> Option<bool> {
        self.has.get()
    }

    /// Llama a `f` cada vez que cambia si hay clave.
    pub fn on_change(&self, f: impl Fn(bool) + 'static) {
        self.listeners.borrow_mut().push(Box::new(f));
    }

    pub fn open_dialog(&self) {
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<CurseforgeKey>().set_dialog_open(true);
        }
    }

    fn save(&self, key: &str) {
        let key = key.trim();
        if key.is_empty() {
            return;
        }
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<CurseforgeKey>().set_dialog_open(false);
        }
        self.actions.set(key);
    }

    /// Respuesta de [`KeyActions::check`] (y tras guardar o borrar).
    pub fn state(&self, has: bool) {
        let was = self.has.replace(Some(has));
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<CurseforgeKey>().set_has_key(has);
        }
        if was != Some(has) {
            for f in self.listeners.borrow().iter() {
                f(has);
            }
        }
    }

    pub fn saved(&self, result: Result<(), Failure>) {
        match result {
            Ok(()) => {
                self.shell.toast(&i18n::text(&self.shell.lang(), "cfKeySaved"), false);
                self.state(true);
            }
            Err(f) => self.shell.toast(&f.message, true),
        }
    }

    pub fn cleared(&self, result: Result<(), Failure>) {
        match result {
            Ok(()) => self.state(false),
            Err(f) => self.shell.toast(&f.message, true),
        }
    }
}

/// [`KeyActions`] contra el núcleo (llavero del sistema).
pub struct CoreKeys {
    pub backend: crate::backend::Backend,
    pub lang: String,
}

impl KeyActions for CoreKeys {
    fn check(&self) {
        let core = self.backend.core.clone();
        self.backend.rt.spawn(async move {
            match core.has_curseforge_key().await {
                Ok(has) => post(move |k| k.state(has)),
                // Sin llavero legible se trata como sin clave (Flutter: `?? false`).
                Err(e) => {
                    tracing::warn!(error = %e, "no se pudo saber si hay clave de CurseForge");
                    post(|k| k.state(false));
                }
            }
        });
    }

    fn set(&self, key: &str) {
        let (core, lang, key) = (self.backend.core.clone(), self.lang.clone(), key.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.set_curseforge_key(&key).await.map_err(|e| Failure::new(&lang, &e));
            post(move |k| k.saved(r));
        });
    }

    fn clear(&self) {
        let (core, lang) = (self.backend.core.clone(), self.lang.clone());
        self.backend.rt.spawn(async move {
            let r = core.clear_curseforge_key().await.map_err(|e| Failure::new(&lang, &e));
            post(move |k| k.cleared(r));
        });
    }
}

/// Sin núcleo: sin clave.
pub struct NoKeys;

impl KeyActions for NoKeys {
    fn check(&self) {
        post(|k| k.state(false));
    }
    fn set(&self, _: &str) {}
    fn clear(&self) {}
}
