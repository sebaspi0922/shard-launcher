//! F3 copias
//! Pestaña Copias del detalle de instancia (backups_tab.dart y los
//! `BackupsController`/`BackupSettingsController` de instance_data.dart):
//! lista de copias con crear, fijar, etiquetar, borrar y restaurar (todo o
//! solo los mundos, con una copia previa opcional), y los ajustes de las
//! copias automáticas, que viven en `Instance.backups`.
//!
//! `Backups` corre en el hilo de la UI y copia el estado a `BackupsState`.
//! El trabajo con el núcleo lo hace un [`BackupActions`] ([`CoreBackups`] en
//! la app, `fixtures::Recorder` en los tests) que entrega los resultados con
//! [`post`].

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use lcore::backup::{BackupEntry, BackupRequest, DEFAULT_INTERVAL_HOURS, DEFAULT_MAX_AUTOMATIC, RestoreMode};
use lcore::instance::BackupSettings;
use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::backend::Backend;
use crate::errors::Failure;
use crate::instance::{DetailEvent, InstanceDetail};
use crate::instances::Phase;
use crate::{BackupItem, BackupsState, InstanceTab, MainWindow, i18n};

/// Frecuencias que se ofrecen (horas): cada 6 h, 12 h, a diario, cada 3
/// días y semanal.
pub const INTERVALS: [u32; 5] = [6, 12, 24, 72, 168];

/// Espera antes de guardar al mover el deslizador de «cuántas se conservan».
pub const KEEP_DEBOUNCE: Duration = Duration::from_millis(450);

/// Ajustes de las copias automáticas con los valores por defecto del núcleo
/// ya aplicados (`BackupSettingsView`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Auto {
    pub on: bool,
    pub interval_hours: u32,
    pub max_count: u32,
}

impl Auto {
    pub fn from_core(s: &BackupSettings) -> Self {
        Self {
            on: s.auto,
            interval_hours: s.interval_hours.unwrap_or(DEFAULT_INTERVAL_HOURS),
            max_count: s.max_count.unwrap_or(DEFAULT_MAX_AUTOMATIC),
        }
    }

    /// Lo que se guarda (al menos 1 hora y 1 copia, como el bridge).
    pub fn to_core(self) -> BackupSettings {
        BackupSettings { auto: self.on, interval_hours: Some(self.interval_hours.max(1)), max_count: Some(self.max_count.max(1)) }
    }
}

/// Frecuencias del selector: las de serie y, al final, la de la instancia si
/// es otra (`{...intervals, s.intervalHours}`).
pub fn interval_options(current: u32) -> Vec<u32> {
    let mut v = INTERVALS.to_vec();
    if !v.contains(&current) {
        v.push(current);
    }
    v
}

/// Título de una copia: su etiqueta o «Copia automática» / «Copia manual»
/// (`backupTitle`).
pub fn title(lang: &str, b: &BackupEntry) -> String {
    match &b.label {
        Some(l) => l.clone(),
        None => i18n::text(lang, if b.automatic { "backupAutomatic" } else { "backupManual" }),
    }
}

/// Etiqueta escrita por el usuario: sin espacios a los lados; vacía = sin
/// etiqueta.
pub fn clean_label(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

/// Número con un decimal como mucho (`NumberFormat('0.#')`).
fn one_decimal(lang: &str, v: f64) -> String {
    let tenths = (v * 10.0).round() as u64;
    let (whole, frac) = (tenths / 10, tenths % 10);
    if frac == 0 {
        whole.to_string()
    } else {
        format!("{whole}{}{frac}", i18n::decimal(lang))
    }
}

/// `formatBytes`: «870 MB», «1,2 GB», «512 KB», «12 B».
pub fn format_bytes(lang: &str, bytes: u64) -> String {
    const K: f64 = 1024.0;
    let b = bytes as f64;
    if b < K {
        format!("{bytes} B")
    } else if b < K * K {
        format!("{} KB", (b / K).round())
    } else if b < K * K * K {
        format!("{} MB", one_decimal(lang, b / (K * K)))
    } else {
        format!("{} GB", one_decimal(lang, b / (K * K * K)))
    }
}

/// (año, mes 1–12, día) de un número de días desde 1970-01-01 (algoritmo
/// `civil_from_days` de H. Hinnant).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// `formatDateTime` (`DateFormat.yMMMd().add_Hm()`): «11 oct 2026 18:30» o
/// «Oct 11, 2026 18:30», en la hora local que da `utc_offset` (segundos).
pub fn format_date_time(lang: &str, unix: i64, utc_offset: i32) -> String {
    const ES: [&str; 12] = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic"];
    const EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let local = unix + i64::from(utc_offset);
    let (y, m, d) = civil_from_days(local.div_euclid(86_400));
    let secs = local.rem_euclid(86_400);
    let (hh, mm) = (secs / 3600, secs % 3600 / 60);
    let i = (m - 1) as usize;
    match lang {
        "en" => format!("{} {d}, {y} {hh:02}:{mm:02}", EN[i]),
        _ => format!("{d} {} {y} {hh:02}:{mm:02}", ES[i]),
    }
}

/// Desfase de la hora local con UTC en el instante `unix` (segundos).
pub fn local_offset(unix: i64) -> i32 {
    use chrono::{Local, Offset, TimeZone};
    Local.timestamp_opt(unix, 0).single().map(|d| d.offset().fix().local_minus_utc()).unwrap_or(0)
}

/// Paso de una restauración en curso.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Copia del estado actual antes de restaurar.
    Safety,
    Restore,
}

/// Lo que la pestaña pide fuera de la UI. Cada resultado vuelve con el id de
/// la instancia.
pub trait BackupActions {
    /// → [`Backups::listed`].
    fn list(&self, instance: &str);
    /// Copia manual → [`Backups::created`]; progreso con [`Backups::progress`].
    fn create(&self, instance: &str, label: Option<String>, pinned: bool);
    /// → [`Backups::updated`].
    fn set_pinned(&self, instance: &str, id: &str, pinned: bool);
    /// `None` quita la etiqueta → [`Backups::updated`].
    fn set_label(&self, instance: &str, id: &str, label: Option<String>);
    /// → [`Backups::deleted`].
    fn delete(&self, instance: &str, id: &str);
    /// Con `safety` (etiqueta), antes una copia del estado actual →
    /// [`Backups::restore_step`] y [`Backups::restored`].
    fn restore(&self, instance: &str, id: &str, worlds_only: bool, safety: Option<String>);
    /// → [`Backups::settings_saved`].
    fn save_settings(&self, instance: &str, settings: BackupSettings);
}

thread_local! {
    static BACKUPS: RefCell<Option<Rc<Backups>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador de la pestaña.
pub fn post(f: impl FnOnce(&Backups) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || {
        BACKUPS.with(|c| {
            if let Some(b) = c.borrow().as_ref() {
                f(b);
            }
        })
    });
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

type Offset = Box<dyn Fn(i64) -> i32>;

pub struct Backups {
    ui: slint::Weak<MainWindow>,
    detail: Rc<InstanceDetail>,
    actions: Box<dyn BackupActions>,
    /// Instancia de la lista que se ve.
    instance: RefCell<Option<String>>,
    list: RefCell<Vec<BackupEntry>>,
    loading: Cell<bool>,
    error: RefCell<Option<Failure>>,
    /// Ajustes a la vista (con el cambio sin guardar, si lo hay) y los
    /// últimos guardados.
    shown: Cell<Option<Auto>>,
    saved: Cell<Option<Auto>>,
    /// Cambio esperando a guardarse (instancia, ajustes).
    pending: RefCell<Option<(String, Auto)>>,
    /// Guardados de ajustes pedidos y sin respuesta: solo la última respuesta
    /// se pinta, para que una vieja no deshaga lo que se acaba de elegir.
    saving_settings: Cell<u32>,
    save_timer: slint::Timer,
    create_busy: Cell<bool>,
    /// Copia que se está restaurando y paso.
    restoring: RefCell<Option<BackupEntry>>,
    step: Cell<Option<Step>>,
    utc_offset: RefCell<Offset>,
    items: Rc<VecModel<BackupItem>>,
}

macro_rules! on {
    ($me:expr, $st:expr, $set:ident, | $b:ident $(, $arg:ident)* | $body:expr) => {{
        let weak = Rc::downgrade(&$me);
        $st.$set(move |$($arg),*| {
            if let Some($b) = weak.upgrade() {
                $body;
            }
        });
    }};
}

impl Backups {
    pub fn new(ui: &MainWindow, detail: Rc<InstanceDetail>, actions: Box<dyn BackupActions>) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            detail: detail.clone(),
            actions,
            instance: RefCell::default(),
            list: RefCell::default(),
            loading: Cell::new(false),
            error: RefCell::default(),
            shown: Cell::new(None),
            saved: Cell::new(None),
            pending: RefCell::default(),
            saving_settings: Cell::new(0),
            save_timer: slint::Timer::default(),
            create_busy: Cell::new(false),
            restoring: RefCell::default(),
            step: Cell::new(None),
            utc_offset: RefCell::new(Box::new(local_offset)),
            items: Rc::new(VecModel::default()),
        });
        BACKUPS.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<BackupsState>();
        st.set_items(ModelRc::from(me.items.clone()));
        on!(me, st, on_open_create, |b| b.open_create());
        on!(me, st, on_close_create, |b| b.close_create());
        on!(me, st, on_create, |b, label, pinned| b.create(&label, pinned));
        on!(me, st, on_set_pinned, |b, id, pinned| b.set_pinned(&id, pinned));
        on!(me, st, on_open_restore, |b, item| b.open_restore(&item.id));
        on!(me, st, on_close_restore, |b| b.close_restore());
        on!(me, st, on_restore, |b, id, worlds, safety| b.restore(&id, worlds, safety));
        on!(me, st, on_rename, |b, id, label| b.rename(&id, &label));
        on!(me, st, on_delete, |b, id| b.delete(&id));
        on!(me, st, on_set_auto, |b, v| b.change(|a| a.on = v, false));
        on!(me, st, on_set_interval, |b, h| b.change(|a| a.interval_hours = h.max(1) as u32, false));
        on!(me, st, on_set_keep, |b, n| b.change(|a| a.max_count = n.clamp(1, 20) as u32, true));
        let weak = Rc::downgrade(&me);
        detail.on(move |e| {
            if let Some(b) = weak.upgrade() {
                b.on_detail(e);
            }
        });
        let weak = Rc::downgrade(&me);
        detail.controller().on_refresh(move |_| {
            if let Some(b) = weak.upgrade() {
                b.refresh_lock();
            }
        });
        me
    }

    /// Hora local de las fechas (los tests y las escenas la fijan).
    pub fn set_utc_offset(&self, f: impl Fn(i64) -> i32 + 'static) {
        *self.utc_offset.borrow_mut() = Box::new(f);
        self.refresh();
    }

    /// Copias a la vista.
    pub fn list(&self) -> Vec<BackupEntry> {
        self.list.borrow().clone()
    }

    /// Ajustes a la vista.
    pub fn auto(&self) -> Option<Auto> {
        self.shown.get()
    }

    fn lang(&self) -> String {
        self.detail.lang()
    }

    fn current(&self) -> Option<String> {
        self.instance.borrow().clone()
    }

    fn is_current(&self, instance: &str) -> bool {
        self.instance.borrow().as_deref() == Some(instance)
    }

    fn on_detail(&self, e: &DetailEvent) {
        match *e {
            DetailEvent::Shown { id, tab: InstanceTab::Backups } => self.show(id),
            DetailEvent::Loaded(d) if self.is_current(&d.instance.id) => self.settings_loaded(&d.instance.backups),
            DetailEvent::Closed => {
                if let Some(ui) = self.ui.upgrade() {
                    ui.global::<BackupsState>().set_menu_open(false);
                }
            }
            _ => {}
        }
    }

    /// La pestaña se muestra: la lista se vuelve a leer (como el provider
    /// `autoDispose` de Flutter al volver a la pestaña).
    fn show(&self, id: &str) {
        if !self.is_current(id) {
            *self.instance.borrow_mut() = Some(id.to_owned());
            self.list.borrow_mut().clear();
            *self.error.borrow_mut() = None;
            self.shown.set(None);
            self.saved.set(None);
            if let Some(d) = self.detail.detail().filter(|d| d.instance.id == id) {
                self.settings_loaded(&d.instance.backups);
            }
        }
        self.loading.set(true);
        self.refresh();
        self.actions.list(id);
    }

    /// Ajustes de la instancia (al cargarla o tras guardarlos). Un cambio
    /// pendiente de guardar manda sobre lo leído.
    fn settings_loaded(&self, s: &BackupSettings) {
        let a = Auto::from_core(s);
        self.saved.set(Some(a));
        if self.pending.borrow().is_none() {
            self.shown.set(Some(a));
        }
        self.refresh();
    }

    // --- Resultados del núcleo ---------------------------------------------

    /// Resultado de [`BackupActions::list`].
    pub fn listed(&self, instance: &str, result: Result<Vec<BackupEntry>, Failure>) {
        if !self.is_current(instance) {
            return;
        }
        self.loading.set(false);
        match result {
            Ok(list) => {
                *self.list.borrow_mut() = list;
                *self.error.borrow_mut() = None;
            }
            Err(f) => {
                // Como `AsyncValue.guard`: el error se ve si no hay lista.
                tracing::warn!(instance, error = %f.detail, "no se pudieron leer las copias");
                self.list.borrow_mut().clear();
                *self.error.borrow_mut() = Some(f);
            }
        }
        self.refresh();
    }

    fn reload(&self) {
        if let Some(id) = self.current() {
            self.actions.list(&id);
        }
    }

    /// Resultado de [`BackupActions::create`].
    pub fn created(&self, instance: &str, result: Result<BackupEntry, Failure>) {
        self.create_busy.set(false);
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<BackupsState>();
        st.set_create_busy(false);
        match result {
            Ok(_) => {
                st.set_create_open(false);
                self.detail.controller().toast(&i18n::text(&self.lang(), "backupCreated"), false);
                if self.is_current(instance) {
                    self.reload();
                }
            }
            Err(f) => {
                st.set_create_error(f.message.into());
                st.set_create_error_detail(f.detail.into());
            }
        }
    }

    /// Resultado de fijar o etiquetar: la copia cambiada.
    pub fn updated(&self, instance: &str, result: Result<BackupEntry, Failure>) {
        match result {
            Ok(b) => {
                if self.is_current(instance) {
                    if let Some(x) = self.list.borrow_mut().iter_mut().find(|x| x.id == b.id) {
                        *x = b;
                    }
                    self.refresh();
                }
            }
            Err(f) => self.detail.controller().toast(&f.message, true),
        }
    }

    /// Resultado de [`BackupActions::delete`].
    pub fn deleted(&self, instance: &str, result: Result<(), Failure>) {
        match result {
            Ok(()) if self.is_current(instance) => self.reload(),
            Ok(()) => {}
            Err(f) => self.detail.controller().toast(&f.message, true),
        }
    }

    /// La copia previa terminó: empieza la restauración.
    pub fn restore_step(&self, instance: &str, step: Step) {
        if !self.is_current(instance) || self.step.get().is_none() {
            return;
        }
        self.step.set(Some(step));
        self.refresh_dialogs();
    }

    /// Resultado de [`BackupActions::restore`].
    pub fn restored(&self, instance: &str, result: Result<(), Failure>) {
        self.step.set(None);
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<BackupsState>();
        match result {
            Ok(()) => {
                st.set_restore_open(false);
                let name = self.restoring.borrow_mut().take().map(|b| title(&self.lang(), &b)).unwrap_or_default();
                let msg = i18n::text(&self.lang(), "backupRestored").replace("{name}", &name);
                self.detail.controller().toast(&msg, false);
                // Lo restaurado cambia mundos, contenido y quizá la instancia.
                if self.is_current(instance) {
                    self.detail.reload();
                }
            }
            Err(f) => {
                st.set_restore_error(f.message.into());
                st.set_restore_error_detail(f.detail.into());
            }
        }
        self.refresh_dialogs();
        // La copia previa ya está en la lista aunque la restauración fallara.
        if self.is_current(instance) {
            self.reload();
        }
    }

    /// Resultado de [`BackupActions::save_settings`].
    pub fn settings_saved(&self, instance: &str, result: Result<BackupSettings, Failure>) {
        let left = self.saving_settings.get().saturating_sub(1);
        self.saving_settings.set(left);
        // Llegan en el orden pedido (`CoreBackups` los escribe uno tras
        // otro): mientras quede otro en camino, este ya no es lo último.
        if !self.is_current(instance) || (left > 0 && result.is_ok()) {
            return;
        }
        match result {
            Ok(s) => {
                self.settings_loaded(&s);
                // `Instance.backups` cambió: que el detalle lo sepa.
                self.detail.reload();
            }
            Err(f) => {
                if self.pending.borrow().is_none() {
                    self.shown.set(self.saved.get());
                }
                self.refresh();
                self.detail.controller().toast(&f.message, true);
            }
        }
    }

    /// Avance de la tarea en curso (etapa `backup` o `restore` del núcleo).
    pub fn progress(&self, stage: &str, done: u64, total: u64, bytes: bool) {
        let expected = match self.step.get() {
            Some(Step::Restore) => "restore",
            Some(Step::Safety) => "backup",
            None if self.create_busy.get() => "backup",
            None => return,
        };
        if stage != expected {
            return;
        }
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<BackupsState>();
        if total == 0 {
            st.set_progress(-1.0);
            st.set_progress_text("".into());
            return;
        }
        st.set_progress((done as f32 / total as f32).clamp(0.0, 1.0));
        let lang = self.lang();
        let text =
            if bytes { format!("{} / {}", format_bytes(&lang, done), format_bytes(&lang, total)) } else { format!("{done} / {total}") };
        st.set_progress_text(text.into());
    }

    // --- Acciones de la UI ---------------------------------------------------

    fn reset_progress(st: &BackupsState) {
        st.set_progress(-1.0);
        st.set_progress_text("".into());
    }

    fn open_create(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<BackupsState>();
        st.set_create_error("".into());
        st.set_create_error_detail("".into());
        st.set_create_busy(false);
        st.set_create_open(true);
    }

    fn close_create(&self) {
        if self.create_busy.get() {
            return;
        }
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<BackupsState>().set_create_open(false);
        }
    }

    fn create(&self, label: &str, pinned: bool) {
        let (Some(id), Some(ui)) = (self.current(), self.ui.upgrade()) else { return };
        if self.create_busy.get() {
            return;
        }
        self.create_busy.set(true);
        let st = ui.global::<BackupsState>();
        st.set_create_busy(true);
        st.set_create_error("".into());
        st.set_create_error_detail("".into());
        Self::reset_progress(&st);
        self.actions.create(&id, clean_label(label), pinned);
    }

    fn set_pinned(&self, backup: &str, pinned: bool) {
        if let Some(id) = self.current() {
            self.actions.set_pinned(&id, backup, pinned);
        }
    }

    fn rename(&self, backup: &str, label: &str) {
        if let Some(id) = self.current() {
            self.actions.set_label(&id, backup, clean_label(label));
        }
    }

    fn delete(&self, backup: &str) {
        if let Some(id) = self.current() {
            self.actions.delete(&id, backup);
        }
    }

    fn open_restore(&self, backup: &str) {
        let Some(b) = self.list.borrow().iter().find(|b| b.id == backup).cloned() else { return };
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<BackupsState>();
        st.set_restore_item(self.item(&b));
        st.set_restore_worlds(false);
        st.set_restore_safety(true);
        st.set_restore_error("".into());
        st.set_restore_error_detail("".into());
        *self.restoring.borrow_mut() = Some(b);
        self.step.set(None);
        self.refresh_dialogs();
        st.set_restore_open(true);
    }

    fn close_restore(&self) {
        if self.step.get().is_some() {
            return;
        }
        *self.restoring.borrow_mut() = None;
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<BackupsState>().set_restore_open(false);
        }
    }

    fn restore(&self, backup: &str, worlds_only: bool, safety: bool) {
        let (Some(id), Some(ui)) = (self.current(), self.ui.upgrade()) else { return };
        if self.step.get().is_some() {
            return;
        }
        self.step.set(Some(if safety { Step::Safety } else { Step::Restore }));
        let st = ui.global::<BackupsState>();
        st.set_restore_error("".into());
        st.set_restore_error_detail("".into());
        self.refresh_dialogs();
        let label = safety.then(|| i18n::text(&self.lang(), "backupSafetyLabel"));
        self.actions.restore(&id, backup, worlds_only, label);
    }

    /// Cambio en los ajustes: se ve enseguida y se guarda ya o, con
    /// `debounce`, tras [`KEEP_DEBOUNCE`] sin más cambios.
    fn change(self: &Rc<Self>, f: impl FnOnce(&mut Auto), debounce: bool) {
        let (Some(id), Some(mut a)) = (self.current(), self.shown.get()) else { return };
        f(&mut a);
        self.shown.set(Some(a));
        *self.pending.borrow_mut() = Some((id, a));
        self.refresh();
        if debounce {
            let weak = Rc::downgrade(self);
            self.save_timer.start(slint::TimerMode::SingleShot, KEEP_DEBOUNCE, move || {
                if let Some(b) = weak.upgrade() {
                    b.flush();
                }
            });
        } else {
            self.save_timer.stop();
            self.flush();
        }
    }

    /// Guarda el cambio pendiente, si lo hay.
    pub fn flush(&self) {
        let Some((id, a)) = self.pending.borrow_mut().take() else { return };
        self.saving_settings.set(self.saving_settings.get() + 1);
        self.actions.save_settings(&id, a.to_core());
    }

    // --- Copia a la UI -------------------------------------------------------

    fn item(&self, b: &BackupEntry) -> BackupItem {
        let lang = self.lang();
        let now = self.detail.controller().now();
        let offset = (self.utc_offset.borrow())(b.created_at);
        BackupItem {
            id: b.id.clone().into(),
            title: title(&lang, b).into(),
            label: b.label.clone().unwrap_or_default().into(),
            automatic: b.automatic,
            pinned: b.pinned,
            date: format_date_time(&lang, b.created_at, offset).into(),
            ago: (now - b.created_at).clamp(0, i32::MAX as i64) as i32,
            size: format_bytes(&lang, b.size_bytes).into(),
        }
    }

    fn refresh_lock(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let running = self.current().is_some_and(|id| self.detail.controller().model.borrow().phase(&id) == Phase::Running);
        ui.global::<BackupsState>().set_locked(running);
    }

    fn refresh_dialogs(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<BackupsState>();
        st.set_restore_step(match self.step.get() {
            None => 0,
            Some(Step::Safety) => 1,
            Some(Step::Restore) => 2,
        });
        Self::reset_progress(&st);
    }

    fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<BackupsState>();
        let list = self.list.borrow();
        let items: Vec<BackupItem> = list.iter().map(|b| self.item(b)).collect();
        // Fila a fila: sin recrear las que no cambian (hover, menús).
        for (i, it) in items.iter().enumerate() {
            if i < self.items.row_count() {
                if self.items.row_data(i).as_ref() != Some(it) {
                    self.items.set_row_data(i, it.clone());
                }
            } else {
                self.items.push(it.clone());
            }
        }
        while self.items.row_count() > items.len() {
            self.items.remove(self.items.row_count() - 1);
        }
        let total: u64 = list.iter().map(|b| b.size_bytes).sum();
        st.set_total(format_bytes(&self.lang(), total).into());
        st.set_loading(self.loading.get() && list.is_empty());
        let (message, detail) = self.error.borrow().as_ref().map(|f| (f.message.clone(), f.detail.clone())).unwrap_or_default();
        st.set_error(message.into());
        st.set_error_detail(detail.into());
        match self.shown.get() {
            Some(a) => {
                st.set_settings_ready(true);
                st.set_auto(a.on);
                let opts: Vec<i32> = interval_options(a.interval_hours).into_iter().map(|h| h as i32).collect();
                st.set_intervals(ModelRc::from(Rc::new(VecModel::from(opts))));
                st.set_interval(a.interval_hours as i32);
                st.set_keep(a.max_count as i32);
            }
            None => st.set_settings_ready(false),
        }
        drop(list);
        self.refresh_lock();
    }
}

/// [`BackupActions`] contra el núcleo.
pub struct CoreBackups {
    backend: Backend,
    /// Ajustes de copias por guardar: un solo trabajador los escribe en el
    /// orden pedido (en tareas sueltas, una vieja podía escribir la última).
    settings: tokio::sync::mpsc::UnboundedSender<(String, BackupSettings)>,
}

impl CoreBackups {
    pub fn new(backend: Backend) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(String, BackupSettings)>();
        let core = backend.core.clone();
        backend.rt.spawn(async move {
            while let Some((id, settings)) = rx.recv().await {
                let r = core.update_backup_settings(&id, settings).await;
                post(move |b| b.settings_saved(&id, r.map_err(|e| fail(b, &e))));
            }
        });
        Self { backend, settings: tx }
    }

    /// Reenvía a la pestaña el progreso de las etapas `backup` y `restore`
    /// hasta que se aborte la tarea devuelta.
    fn watch_progress(&self) -> tokio::task::AbortHandle {
        use lcore::events::{Event, ProgressUnit};
        use tokio::sync::broadcast::error::RecvError;
        let mut rx = self.backend.core.subscribe();
        self.backend
            .rt
            .spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(Event::Progress { stage, done, total, unit, .. }) if stage == "backup" || stage == "restore" => {
                            post(move |b| b.progress(&stage, done, total, unit == ProgressUnit::Bytes));
                        }
                        Ok(_) | Err(RecvError::Lagged(_)) => {}
                        Err(RecvError::Closed) => break,
                    }
                }
            })
            .abort_handle()
    }
}

/// Error del núcleo traducido en el hilo de la UI.
fn fail(b: &Backups, e: &lcore::Error) -> Failure {
    Failure::new(&b.lang(), e)
}

impl BackupActions for CoreBackups {
    fn list(&self, instance: &str) {
        let (core, id) = (self.backend.core.clone(), instance.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.list_backups(&id).await;
            post(move |b| b.listed(&id, r.map_err(|e| fail(b, &e))));
        });
    }

    fn create(&self, instance: &str, label: Option<String>, pinned: bool) {
        let (core, id) = (self.backend.core.clone(), instance.to_owned());
        let watch = self.watch_progress();
        self.backend.rt.spawn(async move {
            let r = core.create_backup(&id, BackupRequest { label, pinned, automatic: false }).await;
            watch.abort();
            post(move |b| b.created(&id, r.map_err(|e| fail(b, &e))));
        });
    }

    fn set_pinned(&self, instance: &str, backup: &str, pinned: bool) {
        let (core, id, backup) = (self.backend.core.clone(), instance.to_owned(), backup.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.set_backup_pinned(&id, &backup, pinned).await;
            post(move |b| b.updated(&id, r.map_err(|e| fail(b, &e))));
        });
    }

    fn set_label(&self, instance: &str, backup: &str, label: Option<String>) {
        let (core, id, backup) = (self.backend.core.clone(), instance.to_owned(), backup.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.set_backup_label(&id, &backup, label).await;
            post(move |b| b.updated(&id, r.map_err(|e| fail(b, &e))));
        });
    }

    fn delete(&self, instance: &str, backup: &str) {
        let (core, id, backup) = (self.backend.core.clone(), instance.to_owned(), backup.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.delete_backup(&id, &backup).await;
            post(move |b| b.deleted(&id, r.map_err(|e| fail(b, &e))));
        });
    }

    fn restore(&self, instance: &str, backup: &str, worlds_only: bool, safety: Option<String>) {
        let (core, id, backup) = (self.backend.core.clone(), instance.to_owned(), backup.to_owned());
        let watch = self.watch_progress();
        let mode = if worlds_only { RestoreMode::WorldsOnly } else { RestoreMode::Full };
        self.backend.rt.spawn(async move {
            let r = async {
                if let Some(label) = safety {
                    core.create_backup(&id, BackupRequest { label: Some(label), pinned: false, automatic: false }).await?;
                    let id = id.clone();
                    post(move |b| b.restore_step(&id, Step::Restore));
                }
                core.restore_backup(&id, &backup, mode).await.map(|_| ())
            }
            .await;
            watch.abort();
            post(move |b| b.restored(&id, r.map_err(|e| fail(b, &e))));
        });
    }

    fn save_settings(&self, instance: &str, settings: BackupSettings) {
        if self.settings.send((instance.to_owned(), settings)).is_err() {
            tracing::warn!(instance, "el guardado de ajustes de copias ya no corre");
        }
    }
}

impl BackupActions for crate::fixtures::Recorder {
    fn list(&self, instance: &str) {
        self.push(format!("backups {instance}"));
    }
    fn create(&self, instance: &str, label: Option<String>, pinned: bool) {
        self.push(format!("backup-create {instance} {label:?} pinned={pinned}"));
    }
    fn set_pinned(&self, instance: &str, id: &str, pinned: bool) {
        self.push(format!("backup-pin {instance} {id} {pinned}"));
    }
    fn set_label(&self, instance: &str, id: &str, label: Option<String>) {
        self.push(format!("backup-label {instance} {id} {label:?}"));
    }
    fn delete(&self, instance: &str, id: &str) {
        self.push(format!("backup-delete {instance} {id}"));
    }
    fn restore(&self, instance: &str, id: &str, worlds_only: bool, safety: Option<String>) {
        let mode = if worlds_only { "worlds" } else { "full" };
        self.push(format!("backup-restore {instance} {id} {mode} safety={safety:?}"));
    }
    fn save_settings(&self, instance: &str, s: BackupSettings) {
        self.push(format!("backup-settings {instance} auto={} h={:?} max={:?}", s.auto, s.interval_hours, s.max_count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(label: Option<&str>, automatic: bool) -> BackupEntry {
        BackupEntry { id: "b".into(), created_at: 0, size_bytes: 0, pinned: false, label: label.map(Into::into), automatic }
    }

    #[test]
    fn titulo_etiqueta_o_tipo() {
        assert_eq!(title("es", &entry(Some("Base"), true)), "Base");
        assert_eq!(title("es", &entry(None, true)), "Copia automática");
        assert_eq!(title("es", &entry(None, false)), "Copia manual");
        assert_eq!(title("en", &entry(None, true)), "Automatic backup");
    }

    #[test]
    fn tamanos_como_format_bytes() {
        const MB: u64 = 1024 * 1024;
        assert_eq!(format_bytes("es", 12), "12 B");
        assert_eq!(format_bytes("es", 1536), "2 KB");
        assert_eq!(format_bytes("es", 870 * MB), "870 MB");
        assert_eq!(format_bytes("es", 1240 * MB), "1,2 GB");
        assert_eq!(format_bytes("en", 1240 * MB), "1.2 GB");
        assert_eq!(format_bytes("es", 4450 * MB), "4,3 GB");
        assert_eq!(format_bytes("es", 1024 * MB), "1 GB");
    }

    #[test]
    fn fechas_como_date_format() {
        // 2026-10-11 15:30 UTC.
        let t = 1_791_732_600;
        assert_eq!(format_date_time("es", t, 0), "11 oct 2026 15:30");
        assert_eq!(format_date_time("en", t, 0), "Oct 11, 2026 15:30");
        assert_eq!(format_date_time("es", t, -5 * 3600), "11 oct 2026 10:30");
        assert_eq!(format_date_time("es", t - 21 * 86_400, 0), "20 sept 2026 15:30");
        assert_eq!(format_date_time("es", 0, 0), "1 ene 1970 00:00");
        assert_eq!(format_date_time("es", 951_782_400, 0), "29 feb 2000 00:00");
    }

    #[test]
    fn frecuencias_con_la_de_la_instancia_al_final() {
        assert_eq!(interval_options(24), vec![6, 12, 24, 72, 168]);
        assert_eq!(interval_options(48), vec![6, 12, 24, 72, 168, 48]);
    }

    #[test]
    fn ajustes_con_los_valores_por_defecto() {
        let a = Auto::from_core(&BackupSettings::default());
        assert_eq!(a, Auto { on: false, interval_hours: DEFAULT_INTERVAL_HOURS, max_count: DEFAULT_MAX_AUTOMATIC });
        let s = Auto { on: true, interval_hours: 0, max_count: 0 }.to_core();
        assert_eq!((s.interval_hours, s.max_count), (Some(1), Some(1)));
    }

    #[test]
    fn etiqueta_vacia_es_ninguna() {
        assert_eq!(clean_label("  "), None);
        assert_eq!(clean_label(" Antes de Create "), Some("Antes de Create".into()));
    }
}
