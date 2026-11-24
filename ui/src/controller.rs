//! Une el estado de Rust con la UI Slint. Corre en el hilo de la UI: el
//! trabajo con el núcleo lo hace `backend` en tokio y entrega los resultados
//! aquí con `slint::invoke_from_event_loop`.
//!
//! Regla: Rust es la fuente de verdad. Slint solo guarda estado de vista
//! (hover, foco); lo demás se copia desde `instances::Model` en `refresh`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use slint::{ComponentHandle, Image, Model as _, ModelRc, SharedPixelBuffer, VecModel};

use crate::instances::{self, Load, Phase, Record};
use crate::{AccountChip, Activity, InstanceItem, InstancesState, LaunchPhase, LaunchState, LoadState, MainWindow, art, face, i18n};

/// Lo que la pantalla pide hacer fuera de ella (núcleo, navegación, diálogos
/// de otras fases). La app real usa el núcleo; los tests, un registro.
pub trait Actions {
    fn open_instance(&self, id: &str);
    fn play(&self, id: &str);
    fn new_instance(&self);
    fn show_trash(&self);
}

pub struct Controller {
    ui: slint::Weak<MainWindow>,
    lang: RefCell<String>,
    pub model: RefCell<instances::Model>,
    /// Hora actual en segundos Unix (fija en las escenas de paridad).
    now: Box<dyn Fn() -> i64>,
    /// Paisajes ya pintados por (id, ancho físico).
    art: RefCell<HashMap<(String, u32), Image>>,
    items: Rc<VecModel<InstanceItem>>,
    actions: Box<dyn Actions>,
    toast_timer: slint::Timer,
    listeners: RefCell<Vec<Box<dyn Fn(&instances::Model)>>>,
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
    let progress = if total > 0 { done as f32 / total as f32 } else { -1.0 };
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
            actions,
            toast_timer: slint::Timer::default(),
            listeners: RefCell::default(),
        });
        let st = ui.global::<InstancesState>();
        st.set_items(ModelRc::from(ctl.items.clone()));
        let weak = Rc::downgrade(&ctl);
        st.on_search(move |q| {
            if let Some(c) = weak.upgrade() {
                c.model.borrow_mut().query = q.to_string();
                c.refresh();
            }
        });
        let weak = Rc::downgrade(&ctl);
        st.on_grid_changed(move |g| {
            if let Some(c) = weak.upgrade() {
                c.model.borrow_mut().grid = g;
            }
        });
        let weak = Rc::downgrade(&ctl);
        st.on_open(move |id| {
            if let Some(c) = weak.upgrade() {
                c.actions.open_instance(&id);
            }
        });
        let weak = Rc::downgrade(&ctl);
        st.on_play(move |id| {
            if let Some(c) = weak.upgrade() {
                c.actions.play(&id);
            }
        });
        let weak = Rc::downgrade(&ctl);
        st.on_new_instance(move || {
            if let Some(c) = weak.upgrade() {
                c.actions.new_instance();
            }
        });
        let weak = Rc::downgrade(&ctl);
        st.on_show_trash(move || {
            if let Some(c) = weak.upgrade() {
                c.actions.show_trash();
            }
        });
        let weak = Rc::downgrade(&ctl);
        st.on_art(move |id, width| weak.upgrade().map(|c| c.landscape(&id, width)).unwrap_or_default());
        ctl.refresh();
        ctl
    }

    /// Llama a `f` cada vez que cambia el modelo (bandeja, etc.).
    pub fn on_refresh(&self, f: impl Fn(&instances::Model) + 'static) {
        self.listeners.borrow_mut().push(Box::new(f));
    }

    /// Lanza una instancia (botón «Jugar» o menú de la bandeja).
    pub fn play(&self, id: &str) {
        self.actions.play(id);
    }

    pub fn lang(&self) -> String {
        self.lang.borrow().clone()
    }

    pub fn set_language(&self, lang: &str) {
        *self.lang.borrow_mut() = lang.to_owned();
        i18n::select(lang);
        self.refresh();
    }

    pub fn set_instances(&self, list: Vec<Record>) {
        self.model.borrow_mut().set_all(list);
        self.refresh();
    }

    pub fn set_failed(&self, message: String, detail: String) {
        self.model.borrow_mut().load = Load::Failed { message, detail };
        self.refresh();
    }

    pub fn set_phase(&self, id: &str, phase: Phase) {
        {
            let mut m = self.model.borrow_mut();
            if phase == Phase::Idle {
                m.launches.remove(id);
            } else {
                m.launches.insert(id.to_owned(), phase);
            }
        }
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
        ui.set_account(match name {
            Some(n) => AccountChip { present: true, name: n.into(), face: face::generated_face_image(n) },
            None => AccountChip::default(),
        });
    }

    /// Paisaje de la tarjeta al ancho pedido, pintado a la escala de la
    /// ventana y guardado para no repetirlo.
    fn landscape(&self, id: &str, width: f32) -> Image {
        let scale = self.ui.upgrade().map(|u| u.window().scale_factor()).unwrap_or(1.0) as f64;
        let w = (width as f64).max(1.0);
        let key = (id.to_owned(), (w * scale).round() as u32);
        if let Some(img) = self.art.borrow().get(&key) {
            return img.clone();
        }
        let cv = art::landscape(id, 12.0, w, 104.0, scale);
        let mut px = cv.to_bgra();
        for p in px.chunks_exact_mut(4) {
            p.swap(0, 2);
        }
        let img = Image::from_rgba8(SharedPixelBuffer::clone_from_slice(&px, cv.w as u32, cv.h as u32));
        self.art.borrow_mut().insert(key, img.clone());
        img
    }

    /// Copia el modelo de Rust a la UI.
    pub fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let lang = self.lang();
        let m = self.model.borrow();
        let st = ui.global::<InstancesState>();
        let now = (self.now)();
        let items: Vec<InstanceItem> = m
            .visible(&lang)
            .into_iter()
            .map(|r| {
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
            })
            .collect();
        sync(&self.items, items);
        st.set_total(m.all.len() as i32);
        st.set_grid(m.grid);
        match &m.load {
            Load::Loading => st.set_state(LoadState::Loading),
            Load::Ready => st.set_state(LoadState::Ready),
            Load::Failed { message, detail } => {
                st.set_error_message(message.as_str().into());
                st.set_error_detail(detail.as_str().into());
                st.set_state(LoadState::Failed);
            }
        }
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
    }
}
