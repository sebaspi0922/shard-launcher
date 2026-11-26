//! Diálogo «Cambiar versión» (change_version_dialog.dart): versión de
//! Minecraft y loader de la instancia abierta, con copia de seguridad previa
//! opcional. Se abre con [`DetailEvent::ChangeVersion`] (menú «Más
//! opciones»); al terminar avisa, recarga la lista y el detail.
//!
//! El selector es el [`Picker`] de «Nueva instancia».

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use slint::ComponentHandle;

use crate::accounts::fill;
use crate::backend::Backend;
use crate::create_instance::{LOADERS, Picker, Request, Version, core_loader_versions, core_versions, parse_loader, reload_instances, version_line};
use crate::errors::Failure;
use crate::instance::{Detail, DetailEvent, InstanceDetail};
use crate::{ChangeVersionState, MainWindow, i18n};

/// Versión actual de la instancia.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Current {
    pub id: String,
    pub name: String,
    pub mc: String,
    pub loader: String,
    pub loader_version: Option<String>,
}

impl Current {
    pub fn from_detail(d: &Detail) -> Self {
        let i = &d.instance;
        Self {
            id: i.id.clone(),
            name: i.name.clone(),
            mc: i.game.mc.clone(),
            loader: i.game.loader.as_str().to_owned(),
            loader_version: i.game.loader_version.clone(),
        }
    }

    /// Loader con el que abre el selector: el suyo si se puede elegir, si no
    /// vanilla.
    pub fn picker(&self) -> Picker {
        let loader = if LOADERS.contains(&self.loader.as_str()) { self.loader.as_str() } else { "vanilla" };
        Picker::new(Some(self.mc.clone()), loader, self.loader_version.clone())
    }

    /// «1.21.4 · Fabric 0.16.10» (lo que va tras «Ahora:»).
    pub fn label(&self, lang: &str) -> String {
        let line = version_line(lang, &self.mc, &self.loader);
        match &self.loader_version {
            Some(v) => format!("{line} {v}"),
            None => line,
        }
    }
}

/// ¿La elección cambia algo? (`_changed`): otra versión, otro loader u otra
/// versión del loader (salvo en vanilla).
pub fn changed(cur: &Current, p: &Picker) -> bool {
    p.mc.as_deref() != Some(cur.mc.as_str())
        || p.loader != cur.loader
        || (p.loader != "vanilla" && p.loader_version != cur.loader_version)
}

/// Lo que se pide al núcleo al aplicar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub id: String,
    pub mc: String,
    pub loader: String,
    pub loader_version: Option<String>,
    /// Etiqueta de la copia previa («Antes de cambiar desde 1.21.4 · Fabric»);
    /// `None` = sin copia.
    pub backup_label: Option<String>,
}

pub trait ChangeVersionActions {
    /// → [`ChangeVersion::versions_loaded`].
    fn list_versions(&self, snapshots: bool);
    /// → [`ChangeVersion::loader_versions_loaded`].
    fn loader_versions(&self, loader: &str, mc: &str);
    /// Copia (si se pide), cambia, recarga la lista y → [`ChangeVersion::done`].
    fn change(&self, change: &Change, attempt: u64);
}

thread_local! {
    static CHANGE: RefCell<Option<Rc<ChangeVersion>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador del diálogo.
pub fn post(f: impl FnOnce(&ChangeVersion) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || with(f));
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// `f` con el controlador del diálogo, ya en el hilo de la UI.
pub fn with(f: impl FnOnce(&ChangeVersion)) {
    CHANGE.with(|c| {
        if let Some(x) = c.borrow().as_ref() {
            f(x);
        }
    })
}

struct Session {
    current: Current,
    picker: Picker,
    backup: bool,
    busy: bool,
    error: Option<Failure>,
}

pub struct ChangeVersion {
    ui: slint::Weak<MainWindow>,
    detail: Rc<InstanceDetail>,
    actions: Box<dyn ChangeVersionActions>,
    session: RefCell<Option<Session>>,
    attempt: Cell<u64>,
}

impl ChangeVersion {
    pub fn new(ui: &MainWindow, detail: Rc<InstanceDetail>, actions: Box<dyn ChangeVersionActions>) -> Rc<Self> {
        let me = Rc::new(Self { ui: ui.as_weak(), detail: detail.clone(), actions, session: RefCell::default(), attempt: Cell::new(0) });
        CHANGE.with(|c| *c.borrow_mut() = Some(me.clone()));
        let weak = Rc::downgrade(&me);
        detail.on(move |e| {
            if let (Some(c), DetailEvent::ChangeVersion(d)) = (weak.upgrade(), e) {
                c.open(Current::from_detail(d));
            }
        });
        let st = ui.global::<ChangeVersionState>();
        macro_rules! on {
            ($set:ident, | $c:ident $(, $arg:ident)* | $body:expr) => {{
                let weak = Rc::downgrade(&me);
                st.$set(move |$($arg),*| {
                    if let Some($c) = weak.upgrade() {
                        $body;
                    }
                });
            }};
        }
        on!(on_cancel, |c| c.close());
        on!(on_apply, |c| c.apply());
        on!(on_backup_toggled, |c, v| c.update(|s| s.backup = v));
        on!(on_search_edited, |c, t| c.update(|s| s.picker.query = t.to_string()));
        on!(on_snapshots, |c, v| c.update(|s| s.picker.snapshots = v));
        on!(on_select_version, |c, id| c.update(|s| s.picker.select_mc(&id)));
        on!(on_select_loader, |c, l| c.update(|s| s.picker.select_loader(&l)));
        on!(on_select_loader_version, |c, v| c.update(|s| s.picker.loader_version = (!v.is_empty()).then(|| v.to_string())));
        me
    }

    fn lang(&self) -> String {
        self.detail.lang()
    }

    pub fn is_open(&self) -> bool {
        self.session.borrow().is_some()
    }

    /// Abre el diálogo con la versión actual elegida y copia activada.
    pub fn open(&self, current: Current) {
        self.attempt.set(self.attempt.get() + 1);
        let picker = current.picker();
        *self.session.borrow_mut() = Some(Session { current, picker, backup: true, busy: false, error: None });
        if let Some(ui) = self.ui.upgrade() {
            let st = ui.global::<ChangeVersionState>();
            st.set_search("".into());
            st.set_open(true);
        }
        self.update(|_| {});
    }

    pub fn close(&self) {
        self.attempt.set(self.attempt.get() + 1);
        *self.session.borrow_mut() = None;
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<ChangeVersionState>().set_open(false);
        }
    }

    fn update(&self, f: impl FnOnce(&mut Session)) {
        let requests = {
            let mut guard = self.session.borrow_mut();
            let Some(s) = guard.as_mut() else { return };
            f(s);
            s.picker.requests()
        };
        for r in requests {
            match r {
                Request::Versions { snapshots } => self.actions.list_versions(snapshots),
                Request::LoaderVersions { loader, mc } => self.actions.loader_versions(&loader, &mc),
            }
        }
        self.refresh();
    }

    fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let guard = self.session.borrow();
        let Some(s) = guard.as_ref() else { return };
        let lang = self.lang();
        let st = ui.global::<ChangeVersionState>();
        let is_changed = changed(&s.current, &s.picker);
        st.set_name(s.current.name.as_str().into());
        st.set_current(s.current.label(&lang).into());
        st.set_current_loader(s.current.loader.as_str().into());
        st.set_changed(is_changed && s.picker.mc.is_some());
        st.set_target(s.picker.mc.as_deref().map(|mc| version_line(&lang, mc, &s.picker.loader)).unwrap_or_default().into());
        st.set_target_loader(s.picker.loader.as_str().into());
        st.set_picker(s.picker.view(&lang));
        st.set_backup(s.backup);
        st.set_busy(s.busy);
        st.set_can_apply(!s.busy && s.picker.mc.is_some() && is_changed);
        st.set_error(s.error.as_ref().map(|f| f.message.as_str()).unwrap_or_default().into());
        st.set_error_detail(s.error.as_ref().map(|f| f.detail.as_str()).unwrap_or_default().into());
    }

    /// Lo que se pediría al aplicar ahora (`None` = el botón está desactivado).
    pub fn pending(&self) -> Option<Change> {
        let guard = self.session.borrow();
        let s = guard.as_ref()?;
        let mc = s.picker.mc.clone()?;
        if s.busy || !changed(&s.current, &s.picker) {
            return None;
        }
        let lang = self.lang();
        let old = version_line(&lang, &s.current.mc, &s.current.loader);
        Some(Change {
            id: s.current.id.clone(),
            mc,
            loader: s.picker.loader.clone(),
            loader_version: s.picker.chosen_loader_version(),
            backup_label: s.backup.then(|| fill(&i18n::text(&lang, "changeVersionBackupLabel"), &old)),
        })
    }

    fn apply(&self) {
        let Some(change) = self.pending() else { return };
        self.update(|s| {
            s.busy = true;
            s.error = None;
        });
        self.actions.change(&change, self.attempt.get());
    }

    pub fn versions_loaded(&self, snapshots: bool, r: Result<Vec<Version>, Failure>) {
        self.update(|s| s.picker.set_versions(snapshots, r));
    }

    pub fn loader_versions_loaded(&self, loader: &str, mc: &str, r: Result<Vec<String>, Failure>) {
        self.update(|s| s.picker.set_loader_versions(loader, mc, r));
    }

    /// Resultado de [`ChangeVersionActions::change`]: `(mc, loader)` nuevos.
    pub fn done(&self, attempt: u64, r: Result<(String, String), Failure>) {
        if attempt != self.attempt.get() || !self.is_open() {
            // Cerrado antes de terminar: igual cambió; que el detalle lo vea.
            if r.is_ok() {
                self.detail.reload();
            }
            return;
        }
        match r {
            Ok((mc, loader)) => {
                self.close();
                let lang = self.lang();
                let msg = fill(&i18n::text(&lang, "changeVersionDone"), &version_line(&lang, &mc, &loader));
                self.detail.controller().toast(&msg, false);
                self.detail.reload();
            }
            Err(f) => self.update(|s| {
                s.busy = false;
                s.error = Some(f);
            }),
        }
    }
}

/// [`ChangeVersionActions`] contra el núcleo.
pub struct CoreChangeVersion(pub Backend);

impl ChangeVersionActions for CoreChangeVersion {
    fn list_versions(&self, snapshots: bool) {
        let core = self.0.core.clone();
        self.0.rt.spawn(async move {
            let r = core_versions(&core, snapshots).await;
            post(move |c| c.versions_loaded(snapshots, r.map_err(|e| Failure::new(&c.lang(), &e))));
        });
    }

    fn loader_versions(&self, loader: &str, mc: &str) {
        let core = self.0.core.clone();
        let (loader, mc) = (loader.to_owned(), mc.to_owned());
        self.0.rt.spawn(async move {
            let r = core_loader_versions(&core, &loader, &mc).await;
            post(move |c| c.loader_versions_loaded(&loader, &mc, r.map_err(|e| Failure::new(&c.lang(), &e))));
        });
    }

    fn change(&self, change: &Change, attempt: u64) {
        let core = self.0.core.clone();
        let change = change.clone();
        self.0.rt.spawn(async move {
            let r = change_in_core(&core, change).await;
            if r.is_ok() {
                reload_instances(&core).await;
            }
            let r = r.map(|i| (i.game.mc, i.game.loader.as_str().to_owned()));
            post(move |c| c.done(attempt, r.map_err(|e| Failure::new(&c.lang(), &e))));
        });
    }
}

/// Copia previa (`backupsProvider.create`) y `change_instance_version` del
/// bridge.
pub async fn change_in_core(core: &lcore::Core, c: Change) -> lcore::Result<lcore::instance::Instance> {
    let kind = parse_loader(&c.loader)?;
    if c.mc.trim().is_empty() {
        return Err(lcore::Error::InvalidInput("falta la versión de Minecraft".into()));
    }
    if let Some(label) = c.backup_label {
        let req = lcore::backup::BackupRequest { label: Some(label), pinned: false, automatic: false };
        core.create_backup(&c.id, req).await?;
    }
    let loader_version = if kind == lcore::instance::LoaderKind::Vanilla { None } else { c.loader_version };
    core.update_instance(&c.id, |i| i.game = lcore::instance::GameVersion { mc: c.mc, loader: kind, loader_version }).await
}

impl ChangeVersionActions for crate::fixtures::Recorder {
    fn list_versions(&self, snapshots: bool) {
        self.push(format!("versions {snapshots}"));
    }
    fn loader_versions(&self, loader: &str, mc: &str) {
        self.push(format!("loader-versions {loader} {mc}"));
    }
    fn change(&self, c: &Change, attempt: u64) {
        let lv = c.loader_version.as_deref().unwrap_or("-");
        let backup = c.backup_label.as_deref().unwrap_or("-");
        self.push(format!("change {}|{}|{}|{lv}|{backup} #{attempt}", c.id, c.mc, c.loader));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current(loader: &str, lv: Option<&str>) -> Current {
        Current {
            id: "survival-amigos".into(),
            name: "Survival con amigos".into(),
            mc: "1.21.4".into(),
            loader: loader.into(),
            loader_version: lv.map(String::from),
        }
    }

    #[test]
    fn sin_cambios_al_abrir() {
        let c = current("fabric", Some("0.16.10"));
        let p = c.picker();
        assert!(!changed(&c, &p));
        assert_eq!(c.label("es"), "1.21.4 · Fabric 0.16.10");
        assert_eq!(current("vanilla", None).label("en"), "1.21.4 · Vanilla");
    }

    #[test]
    fn que_cuenta_como_cambio() {
        let c = current("fabric", Some("0.16.10"));
        let mut p = c.picker();
        p.loader_version = Some("0.16.14".into());
        assert!(changed(&c, &p), "otra versión del loader");
        let mut p = c.picker();
        p.select_loader("neoforge");
        assert!(changed(&c, &p));
        let mut p = c.picker();
        p.select_mc("1.21.5");
        assert!(changed(&c, &p));
        // Elegir la misma versión de nuevo vuelve a «la más reciente»: cambia.
        let mut p = c.picker();
        p.select_mc("1.21.4");
        assert!(changed(&c, &p));
        // En vanilla la versión del loader no cuenta.
        let v = current("vanilla", None);
        let mut p = v.picker();
        p.loader_version = Some("x".into());
        assert!(!changed(&v, &p));
    }

    #[test]
    fn loader_que_no_se_puede_elegir_abre_en_vanilla() {
        let c = current("labymod", None);
        let p = c.picker();
        assert_eq!(p.loader, "vanilla");
        assert!(changed(&c, &p), "como Flutter: ya cuenta como cambio");
    }
}
