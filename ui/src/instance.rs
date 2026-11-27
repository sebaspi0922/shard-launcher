//! Detalle de una instancia (instance_detail_page.dart): qué instancia y qué
//! pestaña se ven, la cabecera (paisaje, etiquetas, Jugar), el menú «Más
//! opciones» y «Eliminar instancia».
//!
//! `InstanceDetail` corre en el hilo de la UI y copia el estado a
//! `InstanceState`. Cada pestaña tiene su propio controlador (módulos
//! `inst_content`, `worlds`, `screenshots`, `backups`, `console`,
//! `inst_settings`) que se entera de qué se muestra con [`InstanceDetail::on`].
//! El trabajo con el núcleo lo hace un [`InstanceActions`] ([`CoreInstance`]
//! en la app, `fixtures::Recorder` en los tests) que entrega los resultados
//! con [`post`].

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use slint::ComponentHandle;

use crate::backend::Backend;
use crate::controller::Controller;
use crate::errors::Failure;
use crate::{InstanceState, InstanceTab, MainWindow, Route, i18n};

/// Lo que el detalle necesita de una instancia además de `instances::Record`
/// (que ya lleva nombre, versión, tiempo jugado…): la instancia completa del
/// núcleo y su carpeta de juego.
#[derive(Clone, Debug, PartialEq)]
pub struct Detail {
    pub instance: lcore::instance::Instance,
    pub game_dir: PathBuf,
}

/// Algo que pasó en el detalle (para los controladores de las pestañas y de
/// los diálogos).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DetailEvent<'a> {
    /// Se muestra `id` en la pestaña `tab` (al abrir o al cambiar de pestaña).
    Shown { id: &'a str, tab: InstanceTab },
    /// Se volvió a la lista.
    Closed,
    /// Llegaron (o cambiaron) los datos completos de la instancia abierta.
    Loaded(&'a Detail),
    /// «Cambiar versión» en el menú.
    ChangeVersion(&'a Detail),
}

/// Lo que el detalle pide fuera de la UI.
pub trait InstanceActions {
    /// Instancia completa → [`InstanceDetail::loaded`].
    fn load(&self, id: &str);
    /// Abrir una carpeta en el gestor de archivos.
    fn open_folder(&self, path: &Path);
    /// Mover a la papelera → [`InstanceDetail::deleted`].
    fn delete(&self, id: &str, name: &str);
}

thread_local! {
    static DETAIL: RefCell<Option<Rc<InstanceDetail>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador del detalle.
pub fn post(f: impl FnOnce(&InstanceDetail) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || with(f));
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// `f` con el controlador del detalle, ya en el hilo de la UI.
pub fn with(f: impl FnOnce(&InstanceDetail)) {
    DETAIL.with(|c| {
        if let Some(d) = c.borrow().as_ref() {
            f(d);
        }
    })
}

type Listener = Box<dyn Fn(&DetailEvent)>;

pub struct InstanceDetail {
    ui: slint::Weak<MainWindow>,
    ctl: Rc<Controller>,
    actions: Box<dyn InstanceActions>,
    /// Instancia abierta (`None` = la lista).
    current: RefCell<Option<String>>,
    tab: Cell<InstanceTab>,
    detail: RefCell<Option<Detail>>,
    /// Tamaño lógico del paisaje de la cabecera.
    art_size: Cell<(f32, f32)>,
    listeners: RefCell<Vec<Listener>>,
}

macro_rules! on {
    ($me:expr, $st:expr, $set:ident, | $d:ident $(, $arg:ident)* | $body:expr) => {{
        let weak = Rc::downgrade(&$me);
        $st.$set(move |$($arg),*| {
            if let Some($d) = weak.upgrade() {
                $body;
            }
        });
    }};
}

impl InstanceDetail {
    pub fn new(ui: &MainWindow, ctl: Rc<Controller>, actions: Box<dyn InstanceActions>) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            ctl: ctl.clone(),
            actions,
            current: RefCell::default(),
            tab: Cell::new(InstanceTab::Settings),
            detail: RefCell::default(),
            art_size: Cell::new((0.0, 0.0)),
            listeners: RefCell::default(),
        });
        DETAIL.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<InstanceState>();
        on!(me, st, on_select_tab, |d, t| d.select_tab(t));
        on!(me, st, on_close, |d| d.close());
        on!(me, st, on_play, |d| {
            if let Some(id) = d.id() {
                d.ctl.play(&id);
            }
        });
        on!(me, st, on_open_folder, |d| {
            let dir = d.detail.borrow().as_ref().map(|x| x.game_dir.clone());
            if let Some(dir) = dir {
                d.actions.open_folder(&dir);
            }
        });
        on!(me, st, on_change_version, |d| {
            let detail = d.detail.borrow().clone();
            if let Some(x) = detail {
                d.emit(&DetailEvent::ChangeVersion(&x));
            }
        });
        on!(me, st, on_diagnose, |d| {
            if let Some(id) = d.id() {
                d.ctl.actions().diagnose(&id);
            }
        });
        on!(me, st, on_delete, |d| {
            if let Some(id) = d.id() {
                let name = d.ctl.model.borrow().by_id(&id).map(|r| r.name.clone()).unwrap_or_default();
                d.actions.delete(&id, &name);
            }
        });
        on!(me, st, on_dismiss_failure, |d| {
            if let Some(id) = d.id() {
                d.ctl.model.borrow_mut().launches.dismiss(&id);
                d.ctl.refresh();
            }
        });
        on!(me, st, on_art_size, |d, w, h| {
            d.art_size.set((w, h));
            d.refresh_art();
        });
        // Abrir desde Instancias, Inicio, la píldora o «Ver registros».
        let weak = Rc::downgrade(&me);
        ctl.set_opener(move |id, tab| {
            if let Some(d) = weak.upgrade() {
                d.open(id, tab);
            }
        });
        let weak = Rc::downgrade(&me);
        ctl.on_refresh(move |_| {
            if let Some(d) = weak.upgrade() {
                d.refresh();
            }
        });
        me
    }

    /// Avisa a `f` de lo que pase en el detalle.
    pub fn on(&self, f: impl Fn(&DetailEvent) + 'static) {
        self.listeners.borrow_mut().push(Box::new(f));
    }

    fn emit(&self, e: &DetailEvent) {
        for f in self.listeners.borrow().iter() {
            f(e);
        }
    }

    /// Instancia abierta.
    pub fn id(&self) -> Option<String> {
        self.current.borrow().clone()
    }

    pub fn tab(&self) -> InstanceTab {
        self.tab.get()
    }

    /// Datos completos de la instancia abierta, si ya llegaron.
    pub fn detail(&self) -> Option<Detail> {
        self.detail.borrow().clone()
    }

    pub fn controller(&self) -> &Rc<Controller> {
        &self.ctl
    }

    pub fn lang(&self) -> String {
        self.ctl.lang()
    }

    /// Abre el detalle de `id` (`/instances/:id?tab=`; sin pestaña, Ajustes
    /// como `parseTab`).
    pub fn open(&self, id: &str, tab: Option<InstanceTab>) {
        let Some(ui) = self.ui.upgrade() else { return };
        let same = self.current.borrow().as_deref() == Some(id);
        if !same {
            *self.current.borrow_mut() = Some(id.to_owned());
            *self.detail.borrow_mut() = None;
            self.actions.load(id);
            if let Some(ui) = self.ui.upgrade() {
                ui.global::<InstanceState>().set_loader_version("".into());
            }
        }
        self.tab.set(tab.unwrap_or(InstanceTab::Settings));
        let st = ui.global::<InstanceState>();
        st.set_confirm_delete(false);
        st.set_id(id.into());
        st.set_tab(self.tab.get());
        if ui.get_route() != Route::Instances {
            ui.set_route(Route::Instances);
            ui.invoke_route_changed(Route::Instances);
        }
        self.refresh();
        self.refresh_art();
        self.emit(&DetailEvent::Shown { id, tab: self.tab.get() });
    }

    pub fn select_tab(&self, tab: InstanceTab) {
        let Some(id) = self.id() else { return };
        self.tab.set(tab);
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<InstanceState>().set_tab(tab);
        }
        self.emit(&DetailEvent::Shown { id: &id, tab });
    }

    /// Vuelve a la lista de Instancias.
    pub fn close(&self) {
        if self.current.borrow_mut().take().is_none() {
            return;
        }
        *self.detail.borrow_mut() = None;
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<InstanceState>().set_id("".into());
        }
        self.emit(&DetailEvent::Closed);
    }

    /// Vuelve a pedir la instancia completa (tras guardar ajustes, cambiar
    /// versión…).
    pub fn reload(&self) {
        if let Some(id) = self.id() {
            self.actions.load(&id);
        }
    }

    /// Resultado de [`InstanceActions::load`].
    pub fn loaded(&self, id: &str, result: Result<Detail, Failure>) {
        if self.id().as_deref() != Some(id) {
            return;
        }
        match result {
            Ok(d) => {
                if let Some(ui) = self.ui.upgrade() {
                    let v = d.instance.game.loader_version.clone().unwrap_or_default();
                    ui.global::<InstanceState>().set_loader_version(v.into());
                }
                *self.detail.borrow_mut() = Some(d.clone());
                self.emit(&DetailEvent::Loaded(&d));
            }
            Err(f) => tracing::warn!(id, error = %f.detail, "no se pudo leer la instancia"),
        }
    }

    /// Resultado de [`InstanceActions::delete`]: aviso y vuelta a la lista.
    pub fn deleted(&self, id: &str, name: &str, result: Result<(), Failure>) {
        let lang = self.lang();
        match result {
            Ok(()) => {
                let msg = i18n::text(&lang, "instanceDeleted").replace("{name}", name);
                self.ctl.toast(&msg, false);
                if self.id().as_deref() == Some(id) {
                    self.close();
                }
            }
            Err(_) => self.ctl.toast(&i18n::text(&lang, "errGeneric"), true),
        }
    }

    /// Copia la cabecera desde el modelo de Instancias.
    fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let Some(id) = self.id() else { return };
        let st = ui.global::<InstanceState>();
        let loading = matches!(self.ctl.model.borrow().load, crate::instances::Load::Loading);
        st.set_loading(loading);
        match self.ctl.instance_item(&id) {
            Some(item) => {
                st.set_found(true);
                st.set_item(item);
                st.set_failure(self.ctl.launch_failure(&id));
            }
            None => st.set_found(false),
        }
    }

    fn refresh_art(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let (w, h) = self.art_size.get();
        if let Some(id) = self.id()
            && w >= 1.0
            && h >= 1.0
        {
            ui.global::<InstanceState>().set_art(self.ctl.landscape(&id, 16.0, w, h));
        }
    }
}

/// Abre `path` en el gestor de archivos del sistema (`launchUrl(Uri.file)`).
pub fn open_path(path: &Path) {
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(path).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(path).spawn();
    if let Err(e) = r {
        tracing::warn!(path = %path.display(), error = %e, "no se pudo abrir la carpeta");
    }
}

/// [`InstanceActions`] contra el núcleo.
pub struct CoreInstance(pub Backend);

impl InstanceActions for CoreInstance {
    fn load(&self, id: &str) {
        let core = self.0.core.clone();
        let id = id.to_owned();
        self.0.rt.spawn(async move {
            let r = async {
                let instance = core.get_instance(&id).await?;
                let game_dir = core.instance_game_dir(&id)?;
                Ok::<_, lcore::Error>(Detail { instance, game_dir })
            }
            .await;
            post(move |d| d.loaded(&id, r.map_err(|e| Failure::new(&d.lang(), &e))));
        });
    }

    fn open_folder(&self, path: &Path) {
        open_path(path);
    }

    fn delete(&self, id: &str, name: &str) {
        let b = self.0.clone();
        let (id, name) = (id.to_owned(), name.to_owned());
        self.0.rt.spawn(async move {
            let r = b.core.delete_instance(&id).await.map(|_| ());
            if r.is_ok() {
                b.load();
            }
            post(move |d| d.deleted(&id, &name, r.map_err(|e| Failure::new(&d.lang(), &e))));
        });
    }
}
