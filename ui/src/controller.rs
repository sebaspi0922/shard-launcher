//! Une el estado de Rust con la UI Slint. Corre en el hilo de la UI: el
//! trabajo con el núcleo lo hace `backend` en tokio y entrega los resultados
//! aquí con `slint::invoke_from_event_loop`.
//!
//! Regla: Rust es la fuente de verdad. Slint solo guarda estado de vista
//! (hover, foco); lo demás se copia desde los modelos en `refresh`.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use lcore::events::Event;
use slint::{ComponentHandle, Image, Model as _, ModelRc, SharedPixelBuffer, VecModel};

use crate::appearance::Appearance;
use crate::home::{self, Server, Status};
use crate::instances::{self, Load, Phase, Record};
use crate::launch::{self, Failure};
use crate::notices::{CrashNotice, Crashes, Update, UpdateStatus, UpdateView};
use crate::prefs::UiPrefs;
use crate::{
    AccountChip, Activity, HomeState, InstanceItem, InstancesState, LaunchFailure, LaunchPhase, LaunchState, LoadState, MainWindow,
    Notices, Route, ServerItem, UpdatePhase, art, errors, face, i18n,
};

/// Lo que la UI pide hacer fuera de ella (núcleo, diálogos de otras fases).
/// La app real usa el núcleo; los tests, un registro. Los métodos con cuerpo
/// por defecto son de fases posteriores o de la fase 1 (compatibles).
pub trait Actions {
    fn open_instance(&self, id: &str);
    /// Lanzar (el controlador ya marcó la instancia como «Preparando»).
    fn play(&self, id: &str);
    fn new_instance(&self);
    fn show_trash(&self);
    /// Lanzar entrando directo a un servidor, con su cuenta si tiene.
    fn join_server(&self, instance_id: &str, _address: &str, _account: Option<&str>) {
        self.play(instance_id)
    }
    /// Panel de cuentas (fase 2).
    fn accounts(&self) {}
    fn add_microsoft_account(&self) {}
    fn add_offline_account(&self) {}
    /// Añadir servidor (fase 4).
    fn add_server(&self) {}
    /// Doctor (fase 5).
    fn diagnose(&self, _instance_id: &str) {}
    /// Registros de la instancia (fase 3: detalle, pestaña Registros).
    fn show_logs(&self, instance_id: &str) {
        self.open_instance(instance_id)
    }
    /// Instalar la versión nueva (AppImage).
    fn install_update(&self) {}
    /// Abrir el AppImage nuevo y salir.
    fn restart_update(&self, _path: &std::path::Path) {}
    fn open_url(&self, _url: &str) {}
    /// Volver a consultar el estado de los servidores (al entrar en Inicio).
    fn refresh_servers(&self) {}
}

/// Quien quiere enterarse de cada cambio del modelo (bandeja, etc.).
type Listener = Box<dyn Fn(&instances::Model)>;

pub struct Controller {
    ui: slint::Weak<MainWindow>,
    lang: RefCell<String>,
    pub model: RefCell<instances::Model>,
    /// Hora actual en segundos Unix (fija en las escenas de paridad).
    now: Box<dyn Fn() -> i64>,
    /// Paisajes ya pintados por (semilla, bloque, ancho, alto físicos).
    art: RefCell<HashMap<(String, u32, u32, u32), Image>>,
    items: Rc<VecModel<InstanceItem>>,
    home_recent: Rc<VecModel<InstanceItem>>,
    home_picker: Rc<VecModel<InstanceItem>>,
    home_running: Rc<VecModel<InstanceItem>>,
    home_servers: Rc<VecModel<ServerItem>>,
    actions: Box<dyn Actions>,
    toast_timer: slint::Timer,
    listeners: RefCell<Vec<Listener>>,
    /// Instancia elegida en Inicio (`homeSelectionProvider`).
    selected: RefCell<Option<String>>,
    /// Servidores guardados (`None` = aún sin leer) y su estado por dirección.
    servers: RefCell<Option<Vec<Server>>>,
    statuses: RefCell<HashMap<String, Status>>,
    favicons: RefCell<HashMap<String, Image>>,
    hero_size: Cell<(f32, f32)>,
    /// Temporizadores de «Iniciando» por instancia.
    starting: RefCell<HashMap<String, slint::Timer>>,
    pub update: RefCell<Update>,
    /// Progreso de la instalación de la versión nueva (etapa `update`).
    update_progress: Cell<f32>,
    pub crashes: RefCell<Crashes>,
    pub appearance: Rc<Appearance>,
}

fn phase_state(p: Phase) -> LaunchState {
    let (phase, done, total) = match p {
        Phase::Idle => (LaunchPhase::Idle, 0, 0),
        Phase::Preparing => (LaunchPhase::Preparing, 0, 0),
        Phase::Downloading { done, total } => (LaunchPhase::Downloading, done, total),
        Phase::Starting => (LaunchPhase::Starting, 0, 0),
        Phase::Running => (LaunchPhase::Running, 0, 0),
        Phase::Error => (LaunchPhase::Error, 0, 0),
    };
    let progress = if total > 0 { (done as f32 / total as f32).min(1.0) } else { -1.0 };
    LaunchState { phase, done: done.min(i32::MAX as u64) as i32, total: total.min(i32::MAX as u64) as i32, progress }
}

fn color(c: art::Rgba) -> slint::Color {
    slint::Color::from_argb_u8((c.a * 255.0).round() as u8, c.r.round() as u8, c.g.round() as u8, c.b.round() as u8)
}

/// Copia `new` en `model` tocando solo las filas que cambian (así Slint no
/// recrea las tarjetas y no se pierde el hover).
fn sync<T: Clone + PartialEq + 'static>(model: &VecModel<T>, new: Vec<T>) {
    if model.row_count() != new.len() {
        model.set_vec(new);
        return;
    }
    for (i, row) in new.into_iter().enumerate() {
        if model.row_data(i).as_ref() != Some(&row) {
            model.set_row_data(i, row);
        }
    }
}

/// `{nombre}` → valor en un texto del ARB.
fn fill(text: String, args: &[(&str, &str)]) -> String {
    args.iter().fold(text, |t, (k, v)| t.replace(&format!("{{{k}}}"), v))
}

/// Conecta un callback de Slint con el controlador (referencia débil).
macro_rules! on {
    ($ctl:expr, $st:expr, $set:ident, | $c:ident $(, $arg:ident)* | $body:expr) => {{
        let weak = Rc::downgrade(&$ctl);
        $st.$set(move |$($arg),*| {
            if let Some($c) = weak.upgrade() {
                $body;
            }
        });
    }};
}

impl Controller {
    pub fn new(ui: &MainWindow, lang: &str, now: Box<dyn Fn() -> i64>, actions: Box<dyn Actions>) -> Rc<Self> {
        i18n::select(lang);
        let ctl = Rc::new(Self {
            ui: ui.as_weak(),
            lang: RefCell::new(lang.to_owned()),
            model: RefCell::new(instances::Model { grid: true, ..Default::default() }),
            now,
            art: RefCell::default(),
            items: Rc::new(VecModel::default()),
            home_recent: Rc::new(VecModel::default()),
            home_picker: Rc::new(VecModel::default()),
            home_running: Rc::new(VecModel::default()),
            home_servers: Rc::new(VecModel::default()),
            actions,
            toast_timer: slint::Timer::default(),
            listeners: RefCell::default(),
            selected: RefCell::default(),
            servers: RefCell::default(),
            statuses: RefCell::default(),
            favicons: RefCell::default(),
            hero_size: Cell::new((0.0, 0.0)),
            starting: RefCell::default(),
            update: RefCell::default(),
            update_progress: Cell::new(-1.0),
            crashes: RefCell::default(),
            appearance: Appearance::new(ui),
        });
        let st = ui.global::<InstancesState>();
        st.set_items(ModelRc::from(ctl.items.clone()));
        on!(ctl, st, on_search, |c, q| {
            c.model.borrow_mut().query = q.to_string();
            c.refresh();
        });
        on!(ctl, st, on_grid_changed, |c, g| c.model.borrow_mut().grid = g);
        on!(ctl, st, on_open, |c, id| c.actions.open_instance(&id));
        on!(ctl, st, on_play, |c, id| c.play(&id));
        on!(ctl, st, on_new_instance, |c| c.actions.new_instance());
        on!(ctl, st, on_show_trash, |c| c.actions.show_trash());
        let weak = Rc::downgrade(&ctl);
        st.on_art(move |id, width| weak.upgrade().map(|c| c.landscape(&id, 12.0, width, 104.0)).unwrap_or_default());

        let hs = ui.global::<HomeState>();
        hs.set_recent(ModelRc::from(ctl.home_recent.clone()));
        hs.set_picker(ModelRc::from(ctl.home_picker.clone()));
        hs.set_running(ModelRc::from(ctl.home_running.clone()));
        hs.set_servers(ModelRc::from(ctl.home_servers.clone()));
        on!(ctl, hs, on_select, |c, id| {
            *c.selected.borrow_mut() = Some(id.to_string());
            c.refresh();
        });
        on!(ctl, hs, on_play, |c, id| c.play(&id));
        on!(ctl, hs, on_dismiss_failure, |c, id| {
            c.model.borrow_mut().launches.dismiss(&id);
            c.refresh();
        });
        on!(ctl, hs, on_open, |c, id| c.actions.open_instance(&id));
        on!(ctl, hs, on_see_all_instances, |c| c.navigate(Route::Instances));
        on!(ctl, hs, on_see_all_servers, |c| c.navigate(Route::Servers));
        on!(ctl, hs, on_join, |c, id| c.join(&id));
        on!(ctl, hs, on_new_instance, |c| c.actions.new_instance());
        on!(ctl, hs, on_change_account, |c| c.actions.accounts());
        on!(ctl, hs, on_add_microsoft, |c| c.actions.add_microsoft_account());
        on!(ctl, hs, on_add_offline, |c| c.actions.add_offline_account());
        on!(ctl, hs, on_add_server, |c| c.actions.add_server());
        on!(ctl, hs, on_diagnose, |c, id| c.actions.diagnose(&id));
        on!(ctl, hs, on_show_logs, |c, id| c.actions.show_logs(&id));
        on!(ctl, hs, on_hero_size, |c, w, h| {
            c.hero_size.set((w, h));
            c.refresh_hero_art();
        });

        let nt = ui.global::<Notices>();
        on!(ctl, nt, on_update_dismiss, |c| {
            c.update.borrow_mut().dismissed = true;
            c.refresh_notices();
        });
        on!(ctl, nt, on_update_install, |c| {
            let ok = c.update.borrow_mut().begin_install();
            if ok {
                c.update_progress.set(-1.0);
                c.refresh_notices();
                c.actions.install_update();
            }
        });
        on!(ctl, nt, on_update_restart, |c| {
            let path = c.update.borrow().path.clone();
            if let Some(p) = path {
                c.actions.restart_update(&p);
            }
        });
        on!(ctl, nt, on_update_download, |c| {
            let url = c.update.borrow().info.as_ref().map(|i| i.download_url.clone());
            if let Some(u) = url {
                c.actions.open_url(&u);
            }
        });
        on!(ctl, nt, on_update_notes, |c| {
            let url = c.update.borrow().info.as_ref().and_then(|i| i.notes_url.clone());
            if let Some(u) = url {
                c.actions.open_url(&u);
            }
        });
        on!(ctl, nt, on_crash_dismiss, |c| {
            c.crashes.borrow_mut().dismiss();
            c.refresh_notices();
        });
        on!(ctl, nt, on_crash_diagnose, |c| {
            if let Some(id) = c.take_crash() {
                c.actions.diagnose(&id);
            }
        });
        on!(ctl, nt, on_crash_logs, |c| {
            if let Some(id) = c.take_crash() {
                c.actions.show_logs(&id);
            }
        });

        on!(ctl, ui, on_account_clicked, |c| c.actions.accounts());
        on!(ctl, ui, on_activity_clicked, |c, id| c.actions.open_instance(&id));
        on!(ctl, ui, on_route_changed, |c, r| {
            if r == Route::Home {
                c.actions.refresh_servers();
            }
        });
        on!(ctl, ui, on_resized, |c, w, h| c.appearance.resized(w, h));
        ctl.appearance.set_prefs(UiPrefs::default());
        ctl.refresh();
        ctl.refresh_notices();
        ctl
    }

    /// Llama a `f` cada vez que cambia el modelo (bandeja, etc.).
    pub fn on_refresh(&self, f: impl Fn(&instances::Model) + 'static) {
        self.listeners.borrow_mut().push(Box::new(f));
    }

    fn navigate(&self, r: Route) {
        if let Some(ui) = self.ui.upgrade() {
            ui.set_route(r);
            ui.invoke_route_changed(r);
        }
    }

    /// Cierra el aviso de cierre inesperado y devuelve su instancia.
    fn take_crash(&self) -> Option<String> {
        let id = self.crashes.borrow().notice.as_ref().map(|n| n.instance_id.clone());
        self.crashes.borrow_mut().dismiss();
        self.refresh_notices();
        id
    }

    /// Lanza una instancia (botón «Jugar» o menú de la bandeja).
    pub fn play(&self, id: &str) {
        let begun = self.model.borrow_mut().launches.begin(id);
        if begun {
            self.refresh();
            self.actions.play(id);
        }
    }

    /// «Entrar» de un servidor rápido.
    fn join(&self, server_id: &str) {
        let server = self.servers.borrow().as_ref().and_then(|l| l.iter().find(|s| s.id == server_id).cloned());
        let Some(s) = server else { return };
        let Some(id) = s.instance_id.clone() else { return };
        if self.model.borrow().by_id(&id).is_none() {
            return;
        }
        let begun = self.model.borrow_mut().launches.begin(&id);
        if begun {
            self.refresh();
            self.actions.join_server(&id, &s.address, s.account_id.as_deref());
        }
    }

    /// Resultado de `Core::launch`.
    pub fn launch_done(&self, id: &str, result: Result<u64, Failure>) {
        let timers = match result {
            Ok(game) => self.model.borrow_mut().launches.launched(id, game),
            Err(f) => {
                self.model.borrow_mut().launches.failed(id, f);
                vec![]
            }
        };
        self.timers(timers);
        self.refresh();
    }

    fn timers(&self, list: Vec<launch::Timer>) {
        for t in list {
            match t {
                launch::Timer::Start(id) => {
                    let timer = slint::Timer::default();
                    let key = id.clone();
                    let weak = self.ui.clone();
                    timer.start(slint::TimerMode::SingleShot, launch::STARTING_TIMEOUT, move || {
                        if weak.upgrade().is_none() {
                            return;
                        }
                        crate::backend::with_controller(|c| c.starting_timeout(&key));
                    });
                    self.starting.borrow_mut().insert(id, timer);
                }
                launch::Timer::Cancel(id) => {
                    self.starting.borrow_mut().remove(&id);
                }
            }
        }
    }

    /// Venció «Iniciando»: se da el juego por abierto.
    pub fn starting_timeout(&self, id: &str) {
        let more = self.model.borrow_mut().launches.running(id);
        self.timers(more);
        self.refresh();
    }

    /// Un evento del núcleo (ya en el hilo de la UI).
    pub fn on_event(&self, e: &Event) {
        let timers = self.model.borrow_mut().launches.on_event(e);
        self.timers(timers);
        let crash_changed = self.crashes.borrow_mut().on_event(e);
        if let Event::Progress { stage, done, total, .. } = e
            && stage == "update"
            && *total > 0
        {
            self.update_progress.set((*done as f32 / *total as f32).min(1.0));
            self.refresh_notices();
        }
        if crash_changed {
            self.refresh_notices();
        }
        self.refresh();
    }

    pub fn lang(&self) -> String {
        self.lang.borrow().clone()
    }

    pub fn set_language(&self, lang: &str) {
        *self.lang.borrow_mut() = lang.to_owned();
        i18n::select(lang);
        self.refresh();
        self.refresh_notices();
    }

    pub fn set_instances(&self, list: Vec<Record>) {
        self.model.borrow_mut().set_all(list);
        self.refresh();
        // El nombre de la instancia del aviso puede haber llegado ahora.
        self.refresh_notices();
    }

    pub fn set_failed(&self, message: String, detail: String) {
        self.model.borrow_mut().load = Load::Failed { message, detail };
        self.refresh();
    }

    /// Fija la fase de una instancia (tests y escenas; `debugSet` en Flutter).
    pub fn set_phase(&self, id: &str, phase: Phase) {
        self.model.borrow_mut().launches.force(id, phase, None);
        self.refresh();
    }

    /// Como `set_phase`, con el motivo de un fallo.
    pub fn set_failure(&self, id: &str, failure: Failure) {
        self.model.borrow_mut().launches.force(id, Phase::Error, Some(failure));
        self.refresh();
    }

    /// Partida abierta al arrancar la app.
    pub fn restore_game(&self, id: &str, game_id: u64) {
        self.model.borrow_mut().launches.restore(id, game_id);
        self.refresh();
    }

    /// Aviso breve abajo (SnackBar), 4 s.
    pub fn toast(&self, message: &str, error: bool) {
        let Some(ui) = self.ui.upgrade() else { return };
        ui.set_toast_message(message.into());
        ui.set_toast_error(error);
        ui.set_toast_shown(true);
        let weak = self.ui.clone();
        self.toast_timer.start(slint::TimerMode::SingleShot, std::time::Duration::from_secs(4), move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_toast_shown(false);
            }
        });
    }

    /// Cuenta activa del raíl (`None` = sin cuenta).
    pub fn set_account(&self, name: Option<&str>) {
        let Some(ui) = self.ui.upgrade() else { return };
        // Caras pintadas al tamaño de cada avatar (dentro de su borde de 1 px).
        let scale = ui.window().scale_factor() as f64;
        ui.set_account(match name {
            Some(n) => AccountChip { present: true, name: n.into(), face: face::painted_face(n, 38.0, scale) },
            None => AccountChip::default(),
        });
        ui.global::<HomeState>().set_session_face(name.map(|n| face::painted_face(n, 42.0, scale)).unwrap_or_default());
    }

    /// Tipo de la cuenta activa (tarjeta Sesión de Inicio).
    pub fn set_account_kind(&self, microsoft: bool) {
        if let Some(ui) = self.ui.upgrade() {
            let hs = ui.global::<HomeState>();
            hs.set_account_microsoft(microsoft);
        }
    }

    pub fn set_servers(&self, list: Vec<Server>) {
        *self.servers.borrow_mut() = Some(list);
        self.refresh();
    }

    pub fn set_statuses(&self, list: Vec<(String, Status)>) {
        self.statuses.borrow_mut().extend(list);
        self.refresh();
    }

    /// Direcciones de los servidores rápidos (para el ping).
    pub fn server_addresses(&self) -> Vec<String> {
        self.servers.borrow().iter().flatten().map(|s| s.address.clone()).collect()
    }

    /// Preferencias de apariencia (`ui.json`).
    pub fn set_prefs(&self, p: UiPrefs) {
        self.appearance.set_prefs(p);
    }

    /// `false` si el resultado se descarta (hay una instalación en marcha).
    pub fn update_checked(&self, info: Option<UpdateView>, manual: bool) -> bool {
        let accepted = self.update.borrow_mut().checked(info, manual);
        self.refresh_notices();
        accepted
    }

    pub fn update_check_failed(&self, error: (String, String), manual: bool) {
        self.update.borrow_mut().check_failed(error, manual);
        self.refresh_notices();
    }

    pub fn update_installed(&self, result: Result<std::path::PathBuf, (String, String)>) {
        {
            let mut u = self.update.borrow_mut();
            match result {
                Ok(p) => {
                    u.status = UpdateStatus::Ready;
                    u.path = Some(p);
                }
                Err(e) => {
                    u.status = UpdateStatus::Failed;
                    u.error = Some(e);
                }
            }
        }
        self.refresh_notices();
    }

    /// Aviso de cierre inesperado (escenas; `debugShow`).
    pub fn show_crash(&self, n: CrashNotice) {
        self.crashes.borrow_mut().notice = Some(n);
        self.refresh_notices();
    }

    /// Paisaje de bloques (art.dart) a un tamaño lógico, pintado a la escala
    /// de la ventana y guardado para no repetirlo.
    fn landscape(&self, seed: &str, block: f64, width: f32, height: f32) -> Image {
        let scale = self.ui.upgrade().map(|u| u.window().scale_factor()).unwrap_or(1.0) as f64;
        let (w, h) = ((width as f64).max(1.0), (height as f64).max(1.0));
        let key = (seed.to_owned(), (block * 10.0) as u32, (w * scale).round() as u32, (h * scale).round() as u32);
        if let Some(img) = self.art.borrow().get(&key) {
            return img.clone();
        }
        let cv = art::landscape(seed, block, w, h, scale);
        let mut px = cv.to_bgra();
        for p in px.as_chunks_mut::<4>().0.iter_mut() {
            p.swap(0, 2);
        }
        let img = Image::from_rgba8(SharedPixelBuffer::clone_from_slice(&px, cv.w as u32, cv.h as u32));
        self.art.borrow_mut().insert(key, img.clone());
        img
    }

    fn refresh_hero_art(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let (w, h) = self.hero_size.get();
        let id = ui.global::<HomeState>().get_hero().id;
        if w >= 1.0 && h >= 1.0 && !id.is_empty() {
            ui.global::<HomeState>().set_hero_art(self.landscape(&id, 18.0, w, h));
        }
    }

    fn item(&self, m: &instances::Model, r: &Record) -> InstanceItem {
        let now = (self.now)();
        let (from, to) = art::icon_colors(&r.id);
        InstanceItem {
            id: r.id.as_str().into(),
            name: r.name.as_str().into(),
            mc: r.mc.as_str().into(),
            loader: r.loader.as_str().into(),
            playtime: r.playtime_seconds.min(i32::MAX as u64) as i32,
            last_ago: r.last_played.map(|t| (now - t).clamp(0, i32::MAX as i64) as i32).unwrap_or(-1),
            initials: art::initials(&r.name).into(),
            icon_from: color(from),
            icon_to: color(to),
            launch: phase_state(m.phase(&r.id)),
        }
    }

    fn failure(&self, f: Option<&Failure>) -> LaunchFailure {
        let lang = self.lang();
        let t = |k| i18n::text(&lang, k);
        match f {
            None => LaunchFailure::default(),
            Some(Failure::Launch { kind, message }) => LaunchFailure {
                present: true,
                title: t("launchFailed").into(),
                detail: t(errors::key(kind)).into(),
                tech: if message.is_empty() { String::new() } else { format!("{kind}: {message}") }.into(),
                crashed: false,
            },
            Some(Failure::Crash { code, crash_report }) => {
                let mut lines = vec![];
                if let Some(c) = code {
                    lines.push(fill(t("launchExitCode"), &[("code", &c.to_string())]));
                }
                if let Some(p) = crash_report {
                    lines.push(fill(t("launchCrashReport"), &[("path", p)]));
                }
                LaunchFailure { present: true, title: t("launchCrashed").into(), detail: lines.join("\n").into(), tech: "".into(), crashed: true }
            }
        }
    }

    fn server_item(&self, m: &instances::Model, s: &Server) -> ServerItem {
        let lang = self.lang();
        let statuses = self.statuses.borrow();
        let st = statuses.get(&s.address);
        let instance = s.instance_id.as_deref().and_then(|id| m.by_id(id));
        let ls = instance.map(|r| m.launches.state(&r.id)).unwrap_or_default();
        let running = ls.phase == Phase::Running;
        let tooltip = match instance {
            None => i18n::text(&lang, "serverNoInstance"),
            Some(r) if running => fill(i18n::text(&lang, "serverAlreadyRunning"), &[("instance", &r.name)]),
            Some(r) => fill(i18n::text(&lang, "serverJoinWith"), &[("instance", &r.name)]),
        };
        let favicon = st.and_then(|st| st.favicon_png_base64.as_deref()).and_then(|b64| self.favicon(b64));
        ServerItem {
            id: s.id.as_str().into(),
            name: s.name.as_str().into(),
            status: home::status_line(&lang, st).into(),
            level: home::ping_level(st),
            online: st.is_some_and(|s| s.online),
            has_favicon: favicon.is_some(),
            icon: favicon.unwrap_or_else(|| self.landscape(&s.address, 6.0, 34.0, 34.0)),
            can_join: instance.is_some() && !ls.busy() && !running,
            busy: ls.busy(),
            running,
            tooltip: tooltip.into(),
        }
    }

    /// Favicon PNG en base64 del ping (`faviconProvider`).
    fn favicon(&self, b64: &str) -> Option<Image> {
        if let Some(img) = self.favicons.borrow().get(b64) {
            return Some(img.clone());
        }
        use base64::Engine as _;
        let clean: String = b64.chars().filter(|c| !c.is_whitespace()).collect();
        let bytes = base64::engine::general_purpose::STANDARD.decode(clean).ok()?;
        let rgba = image::load_from_memory(&bytes).ok()?.to_rgba8();
        let img = Image::from_rgba8(SharedPixelBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height()));
        self.favicons.borrow_mut().insert(b64.to_owned(), img.clone());
        Some(img)
    }

    /// Copia el estado de los avisos a la UI.
    pub fn refresh_notices(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let lang = self.lang();
        let t = |k| i18n::text(&lang, k);
        let nt = ui.global::<Notices>();
        let u = self.update.borrow();
        nt.set_update_shown(u.visible());
        if let Some(info) = &u.info {
            nt.set_update_version(info.version.as_str().into());
            nt.set_update_can_install(info.can_install);
            nt.set_update_has_notes(info.notes_url.is_some());
            nt.set_update_phase(match u.status {
                UpdateStatus::Installing => UpdatePhase::Installing,
                UpdateStatus::Ready => UpdatePhase::Ready,
                UpdateStatus::Failed => UpdatePhase::Failed,
                _ => UpdatePhase::Available,
            });
            let status = match (&u.status, &u.error) {
                (UpdateStatus::Installing, _) => t("updateInstalling"),
                (UpdateStatus::Ready, _) => t("updateReady"),
                (UpdateStatus::Failed, Some((msg, _))) => format!("{}: {msg}", t("updateFailed")),
                _ if !info.can_install => t("updateManualHint"),
                _ => info.notes.clone().unwrap_or_else(|| fill(t("updateAvailableBody"), &[("current", &info.current)])),
            };
            nt.set_update_status(status.into());
        }
        nt.set_update_progress(self.update_progress.get());
        let c = self.crashes.borrow();
        nt.set_crash_shown(c.notice.is_some());
        if let Some(n) = &c.notice {
            let m = self.model.borrow();
            let name = m.by_id(&n.instance_id).map(|r| r.name.clone());
            let mut parts = vec![name.clone().unwrap_or_else(|| n.instance_id.clone())];
            if let Some(code) = n.code {
                parts.push(fill(t("crashBannerCode"), &[("code", &code.to_string())]));
            }
            nt.set_crash_line(parts.join(" · ").into());
            nt.set_crash_has_instance(name.is_some());
        }
    }

    /// Copia el modelo de Rust a la UI.
    pub fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let lang = self.lang();
        let m = self.model.borrow();
        let st = ui.global::<InstancesState>();
        let items: Vec<InstanceItem> = m.visible(&lang).into_iter().map(|r| self.item(&m, r)).collect();
        sync(&self.items, items);
        st.set_total(m.all.len() as i32);
        st.set_grid(m.grid);
        let state = match &m.load {
            Load::Loading => LoadState::Loading,
            Load::Ready => LoadState::Ready,
            Load::Failed { message, detail } => {
                st.set_error_message(message.as_str().into());
                st.set_error_detail(detail.as_str().into());
                LoadState::Failed
            }
        };
        st.set_state(state);

        // Inicio.
        let hs = ui.global::<HomeState>();
        hs.set_state(state);
        if let Load::Failed { message, detail } = &m.load {
            hs.set_error_message(message.as_str().into());
            hs.set_error_detail(detail.as_str().into());
        }
        let selected = self.selected.borrow().clone();
        let hero = home::home_instance(&m.all, selected.as_deref());
        let hero_changed = hero.map(|r| r.id.as_str()) != Some(hs.get_hero().id.as_str());
        hs.set_has_hero(hero.is_some());
        if let Some(h) = hero {
            hs.set_hero(self.item(&m, h));
            hs.set_hero_failure(self.failure(m.launches.state(&h.id).failure.as_ref()));
        }
        let recent: Vec<InstanceItem> =
            home::recent_cards(&m.all, hero.map(|r| r.id.as_str())).into_iter().map(|r| self.item(&m, r)).collect();
        sync(&self.home_recent, recent);
        sync(&self.home_picker, m.all.iter().map(|r| self.item(&m, r)).collect());
        let running: Vec<InstanceItem> = m.all.iter().filter(|r| m.phase(&r.id) == Phase::Running).map(|r| self.item(&m, r)).collect();
        sync(&self.home_running, running);
        let servers: Vec<ServerItem> = self.servers.borrow().iter().flatten().take(3).map(|s| self.server_item(&m, s)).collect();
        sync(&self.home_servers, servers);

        for f in self.listeners.borrow().iter() {
            f(&m);
        }
        ui.set_activity(match m.active() {
            Some((r, p)) => {
                let s = phase_state(p);
                Activity {
                    present: true,
                    instance_id: r.id.as_str().into(),
                    name: r.name.as_str().into(),
                    phase: s.phase,
                    done: s.done,
                    total: s.total,
                    progress: s.progress,
                }
            }
            None => Activity::default(),
        });
        drop(m);
        if hero_changed {
            self.refresh_hero_art();
        }
    }
}
