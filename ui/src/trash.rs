//! Papelera de instancias (trash_dialog.dart): recuperar o borrar para
//! siempre (con confirmación), vaciarla y cuántos días le quedan a cada una.
//! El núcleo borra solo lo que lleva más de 30 días.
//!
//! [`Trash`] corre en el hilo de la UI y copia el estado a `TrashState`; el
//! trabajo con el núcleo lo hace un [`TrashActions`] que entrega con [`post`].

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};

use crate::accounts::fill;
use crate::backend::Backend;
use crate::controller::Controller;
use crate::errors::Failure;
use crate::{MainWindow, TrashItem, TrashState, art, i18n};

/// Días que una instancia pasa en la papelera (`kTrashRetentionDays`).
pub const RETENTION_DAYS: i64 = 30;

/// Una instancia en la papelera (`TrashedView`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trashed {
    pub trash_id: String,
    pub id: String,
    pub name: String,
    pub mc: String,
    pub loader: String,
    pub playtime_seconds: u64,
    /// Segundos Unix.
    pub deleted_at: i64,
}

impl From<lcore::instance::TrashedInstance> for Trashed {
    fn from(t: lcore::instance::TrashedInstance) -> Self {
        Self {
            trash_id: t.trash_id,
            id: t.instance.id,
            name: t.instance.name,
            mc: t.instance.game.mc,
            loader: t.instance.game.loader.as_str().into(),
            playtime_seconds: t.instance.playtime_seconds,
            deleted_at: t.deleted_at,
        }
    }
}

/// Días que le quedan antes de borrarse sola: 30 menos los días enteros
/// desde que se borró, entre 0 y 30.
pub fn days_left(deleted_at: i64, now: i64) -> i64 {
    (RETENTION_DAYS - (now - deleted_at) / 86_400).clamp(0, RETENTION_DAYS)
}

/// Qué se está haciendo (`_busy` de Flutter: un id o `'*'`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Busy {
    One(String),
    All,
}

/// Lo que el diálogo pide fuera de la UI.
pub trait TrashActions {
    /// → [`Trash::listed`].
    fn list(&self);
    /// Recupera, recarga la lista de instancias y → [`Trash::restored`].
    fn restore(&self, trash_id: &str);
    /// Borra una a una (para en el primer error) → [`Trash::purged`].
    fn purge(&self, trash_ids: &[String]);
}

thread_local! {
    static TRASH: RefCell<Option<Rc<Trash>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador de la papelera.
pub fn post(f: impl FnOnce(&Trash) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || with(f));
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// `f` con el controlador de la papelera, ya en el hilo de la UI.
pub fn with(f: impl FnOnce(&Trash)) {
    TRASH.with(|c| {
        if let Some(x) = c.borrow().as_ref() {
            f(x);
        }
    })
}

/// Lista: cargando, lista o error.
#[derive(Clone, Debug, PartialEq)]
enum List {
    Loading,
    Ready(Vec<Trashed>),
    Failed(Failure),
}

struct State {
    open: bool,
    list: List,
    busy: Option<Busy>,
    /// Pendientes de confirmar «Borrar para siempre».
    confirm: Option<Vec<String>>,
    /// Cuántas veces se abrió el diálogo, y en cuál empezó la operación en
    /// curso: un resultado tardío no cierra ni navega desde otra apertura.
    session: u64,
    busy_session: u64,
}

pub struct Trash {
    ui: slint::Weak<MainWindow>,
    shell: Rc<Controller>,
    actions: Box<dyn TrashActions>,
    now: Box<dyn Fn() -> i64>,
    state: RefCell<State>,
}

impl Trash {
    /// Crea el controlador y lo registra para [`post`]. `now`: segundos Unix.
    pub fn new(ui: &MainWindow, shell: Rc<Controller>, actions: Box<dyn TrashActions>, now: Box<dyn Fn() -> i64>) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            shell,
            actions,
            now,
            state: RefCell::new(State { open: false, list: List::Loading, busy: None, confirm: None, session: 0, busy_session: 0 }),
        });
        TRASH.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<TrashState>();
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
        on!(on_close, |t| t.close());
        on!(on_restore, |t, id| t.restore(&id));
        on!(on_ask_purge, |t, id| t.ask_purge(Some(id.as_str())));
        on!(on_ask_empty, |t| t.ask_purge(None));
        on!(on_confirm, |t| t.confirm());
        on!(on_cancel_confirm, |t| {
            t.state.borrow_mut().confirm = None;
            t.refresh();
        });
        me
    }

    fn lang(&self) -> String {
        self.shell.lang()
    }

    pub fn is_open(&self) -> bool {
        self.state.borrow().open
    }

    /// Abre la papelera y pide la lista.
    /// La operación en curso (si la hay) sigue: no se puede empezar otra.
    pub fn open(&self) {
        {
            let mut s = self.state.borrow_mut();
            s.open = true;
            s.list = List::Loading;
            s.confirm = None;
            s.session += 1;
        }
        self.actions.list();
        self.refresh();
    }

    pub fn close(&self) {
        let mut s = self.state.borrow_mut();
        s.open = false;
        s.confirm = None;
        drop(s);
        self.refresh();
    }

    pub fn listed(&self, r: Result<Vec<Trashed>, Failure>) {
        self.state.borrow_mut().list = r.map_or_else(List::Failed, List::Ready);
        self.refresh();
    }

    fn items(&self) -> Vec<Trashed> {
        match &self.state.borrow().list {
            List::Ready(l) => l.clone(),
            _ => Vec::new(),
        }
    }

    fn restore(&self, trash_id: &str) {
        if self.state.borrow().busy.is_some() {
            return;
        }
        {
            let mut s = self.state.borrow_mut();
            s.busy = Some(Busy::One(trash_id.to_owned()));
            s.busy_session = s.session;
        }
        self.actions.restore(trash_id);
        self.refresh();
    }

    /// Resultado de [`TrashActions::restore`]: `(id, nombre)` recuperada.
    pub fn restored(&self, r: Result<(String, String), Failure>) {
        let same_session = {
            let mut s = self.state.borrow_mut();
            s.busy = None;
            s.open && s.busy_session == s.session
        };
        match r {
            Ok((id, name)) => {
                self.shell.toast(&fill(&i18n::text(&self.lang(), "trashRestored"), &name), false);
                if same_session {
                    self.close();
                    self.shell.open_instance(&id, None);
                }
            }
            Err(f) => {
                self.shell.toast(&f.message, true);
                self.refresh();
            }
        }
    }

    /// «Borrar para siempre» de una fila (`Some`) o «Vaciar papelera»: pide
    /// confirmación.
    fn ask_purge(&self, trash_id: Option<&str>) {
        let ids: Vec<String> = match trash_id {
            Some(id) => vec![id.to_owned()],
            None => self.items().into_iter().map(|t| t.trash_id).collect(),
        };
        if ids.is_empty() || self.state.borrow().busy.is_some() {
            return;
        }
        self.state.borrow_mut().confirm = Some(ids);
        self.refresh();
    }

    fn confirm(&self) {
        let Some(ids) = self.state.borrow_mut().confirm.take() else { return };
        {
            let mut s = self.state.borrow_mut();
            s.busy = Some(if ids.len() == 1 { Busy::One(ids[0].clone()) } else { Busy::All });
            s.busy_session = s.session;
        }
        self.actions.purge(&ids);
        self.refresh();
    }

    /// Resultado de [`TrashActions::purge`]: vuelve a leer la lista.
    pub fn purged(&self, r: Result<(), Failure>) {
        self.state.borrow_mut().busy = None;
        if let Err(f) = r {
            self.shell.toast(&f.message, true);
        }
        if self.is_open() {
            self.actions.list();
        }
        self.refresh();
    }

    fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<TrashState>();
        let s = self.state.borrow();
        st.set_open(s.open);
        let now = (self.now)();
        let (state, rows) = match &s.list {
            List::Loading => (0, Vec::new()),
            List::Failed(f) => {
                st.set_error(f.message.as_str().into());
                st.set_error_detail(f.detail.as_str().into());
                (3, Vec::new())
            }
            List::Ready(l) if l.is_empty() => (1, Vec::new()),
            List::Ready(l) => (2, l.iter().map(|t| item(t, now, s.busy.as_ref())).collect()),
        };
        st.set_state(state);
        st.set_items(ModelRc::new(VecModel::from(rows)));
        st.set_busy(s.busy.is_some());
        match &s.confirm {
            Some(ids) => {
                let single = (ids.len() == 1).then(|| self.items().into_iter().find(|t| t.trash_id == ids[0]).map(|t| t.name)).flatten();
                // Una sola fila: «¿Borrar «x» para siempre?»; vaciar: el plural.
                st.set_confirm_name(single.unwrap_or_default().into());
                st.set_confirm_count(ids.len() as i32);
                st.set_confirm_open(true);
            }
            None => st.set_confirm_open(false),
        }
    }
}

fn item(t: &Trashed, now: i64, busy: Option<&Busy>) -> TrashItem {
    let (from, to) = art::icon_colors(&t.id);
    let color = |c: art::Rgba| slint::Color::from_argb_u8((c.a * 255.0).round() as u8, c.r.round() as u8, c.g.round() as u8, c.b.round() as u8);
    TrashItem {
        trash_id: t.trash_id.as_str().into(),
        name: t.name.as_str().into(),
        mc: t.mc.as_str().into(),
        loader: t.loader.as_str().into(),
        playtime: t.playtime_seconds.min(i32::MAX as u64) as i32,
        ago: (now - t.deleted_at).clamp(0, i32::MAX as i64) as i32,
        days_left: days_left(t.deleted_at, now) as i32,
        initials: art::initials(&t.name).into(),
        icon_from: color(from),
        icon_to: color(to),
        busy: matches!(busy, Some(Busy::All)) || busy == Some(&Busy::One(t.trash_id.clone())),
    }
}

/// [`TrashActions`] contra el núcleo.
pub struct CoreTrash(pub Backend);

impl TrashActions for CoreTrash {
    fn list(&self) {
        let core = self.0.core.clone();
        self.0.rt.spawn(async move {
            let r = core.list_trash().await.map(|l| l.into_iter().map(Trashed::from).collect());
            post(move |t| t.listed(r.map_err(|e| Failure::new(&t.lang(), &e))));
        });
    }

    fn restore(&self, trash_id: &str) {
        let core = self.0.core.clone();
        let trash_id = trash_id.to_owned();
        self.0.rt.spawn(async move {
            let r = core.restore_instance(&trash_id).await;
            if r.is_ok() {
                crate::create_instance::reload_instances(&core).await;
            }
            let r = r.map(|i| (i.id, i.name));
            post(move |t| t.restored(r.map_err(|e| Failure::new(&t.lang(), &e))));
        });
    }

    fn purge(&self, trash_ids: &[String]) {
        let core = self.0.core.clone();
        let ids = trash_ids.to_vec();
        self.0.rt.spawn(async move {
            let mut r = Ok(());
            for id in &ids {
                r = core.purge_instance(id).await;
                if r.is_err() {
                    break;
                }
            }
            post(move |t| t.purged(r.map_err(|e| Failure::new(&t.lang(), &e))));
        });
    }
}

impl TrashActions for crate::fixtures::Recorder {
    fn list(&self) {
        self.push("trash".into());
    }
    fn restore(&self, trash_id: &str) {
        self.push(format!("restore {trash_id}"));
    }
    fn purge(&self, trash_ids: &[String]) {
        self.push(format!("purge {}", trash_ids.join(",")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::NOW;

    const D: i64 = 86_400;

    #[test]
    fn dias_que_quedan() {
        assert_eq!(days_left(NOW, NOW), 30);
        assert_eq!(days_left(NOW - 2 * D, NOW), 28);
        assert_eq!(days_left(NOW - 19 * D, NOW), 11);
        // Días enteros: a las 23 h sigue contando el día entero.
        assert_eq!(days_left(NOW - 2 * D - 23 * 3600, NOW), 28);
        assert_eq!(days_left(NOW - 30 * D, NOW), 0);
        assert_eq!(days_left(NOW - 45 * D, NOW), 0, "nunca negativo");
        assert_eq!(days_left(NOW + D, NOW), 30, "reloj atrasado: no más de 30");
    }

    #[test]
    fn fila_ocupada_segun_lo_que_se_hace() {
        let t = &crate::fixtures::trash::sample_trash()[0];
        assert!(!item(t, NOW, None).busy);
        assert!(item(t, NOW, Some(&Busy::All)).busy);
        assert!(item(t, NOW, Some(&Busy::One(t.trash_id.clone()))).busy);
        assert!(!item(t, NOW, Some(&Busy::One("otra".into()))).busy);
        let i = item(t, NOW, None);
        assert_eq!((i.days_left, i.ago, i.initials.as_str()), (28, 2 * D as i32, "PV"));
    }
}
