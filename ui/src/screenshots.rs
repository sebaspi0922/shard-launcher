//! Pestaña Capturas (screenshots_tab.dart): rejilla de miniaturas con la
//! fecha, abrir la carpeta y el visor a pantalla completa (← →, Escape,
//! abrir con el visor del sistema y borrar con confirmación).
//!
//! `Screenshots` corre en el hilo de la UI y copia el estado a
//! `ScreenshotsState`. El núcleo y el disco los toca un
//! [`ScreenshotActions`] ([`CoreScreenshots`] en la app,
//! `fixtures::Recorder` en los tests) que responde con [`post`].

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use lcore::world::ScreenshotInfo;
use slint::{ComponentHandle, Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};

use crate::backend::Backend;
use crate::errors::Failure;
use crate::instance::{DetailEvent, InstanceDetail};
use crate::instances::Phase;
use crate::{InstanceTab, MainWindow, ScreenshotsState, ShotItem};

/// Ancho de las miniaturas (`cacheWidth: 600`).
pub const THUMB_WIDTH: u32 = 600;
/// Ancho máximo de la imagen del visor (0 = la original).
pub const FULL_WIDTH: u32 = 0;

/// Píxeles de una imagen decodificada (cruza de tokio al hilo de la UI).
pub type Pixels = SharedPixelBuffer<Rgba8Pixel>;

/// (año, mes 1–12, día) del día `days` contado desde 1970-01-01
/// (algoritmo `civil_from_days` de H. Hinnant).
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// `formatDateTime` (`DateFormat.yMMMd().add_Hm()`) de segundos Unix ya en
/// hora local: «11 oct 2026 13:30», «6 oct 2026 0:30» / «Oct 11, 2026 13:30».
pub fn format_datetime(lang: &str, local_secs: i64) -> String {
    const ES: [&str; 12] = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic"];
    const EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (y, m, d) = civil_from_days(local_secs.div_euclid(86_400));
    let s = local_secs.rem_euclid(86_400);
    let (hh, mm) = (s / 3600, s % 3600 / 60);
    if lang == "es" {
        format!("{d} {} {y} {hh}:{mm:02}", ES[m as usize - 1])
    } else {
        format!("{} {d}, {y} {hh:02}:{mm:02}", EN[m as usize - 1])
    }
}

/// Índice que queda a la vista tras borrar la captura `index` de una lista
/// que ahora tiene `left` (`None` = ya no queda ninguna: se cierra).
pub fn after_delete(index: usize, left: usize) -> Option<usize> {
    (left > 0).then(|| index.min(left - 1))
}

/// Lo que el controlador pide fuera de la UI.
pub trait ScreenshotActions {
    /// → [`Screenshots::listed`].
    fn list(&self, instance: &str);
    /// Decodifica y reduce a `max_width` (0 = sin reducir) →
    /// [`Screenshots::image_loaded`].
    fn load_image(&self, path: &Path, max_width: u32);
    /// → [`Screenshots::deleted`].
    fn delete(&self, instance: &str, file: &str);
    /// Abre una carpeta o una imagen con el programa del sistema.
    fn open_path(&self, path: &Path);
}

thread_local! {
    static SHOTS: RefCell<Option<Rc<Screenshots>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador de Capturas.
pub fn post(f: impl FnOnce(&Screenshots) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || {
        SHOTS.with(|c| {
            if let Some(s) = c.borrow().as_ref() {
                f(s);
            }
        })
    });
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

pub struct Screenshots {
    ui: slint::Weak<MainWindow>,
    detail: Rc<InstanceDetail>,
    actions: Box<dyn ScreenshotActions>,
    /// Desfase de la hora local en segundos para un instante Unix (fijo en
    /// las escenas).
    utc_offset: Box<dyn Fn(i64) -> i64>,
    instance: RefCell<Option<String>>,
    list: RefCell<Vec<ScreenshotInfo>>,
    items: Rc<VecModel<ShotItem>>,
    /// Miniaturas ya decodificadas y las pedidas.
    thumbs: RefCell<HashMap<PathBuf, Image>>,
    pending: RefCell<HashSet<(PathBuf, u32)>>,
    /// Tamaño de una celda (paisajes de reserva).
    cell: Cell<(f32, f32)>,
    /// Captura del visor y su imagen completa, si ya llegó.
    viewing: RefCell<Option<PathBuf>>,
    running: Cell<bool>,
}

macro_rules! on {
    ($me:expr, $st:expr, $set:ident, | $s:ident $(, $arg:ident)* | $body:expr) => {{
        let weak = Rc::downgrade(&$me);
        $st.$set(move |$($arg),*| {
            if let Some($s) = weak.upgrade() {
                $body;
            }
        });
    }};
}

impl Screenshots {
    pub fn new(
        ui: &MainWindow,
        detail: &Rc<InstanceDetail>,
        actions: Box<dyn ScreenshotActions>,
        utc_offset: Box<dyn Fn(i64) -> i64>,
    ) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            detail: detail.clone(),
            actions,
            utc_offset,
            instance: RefCell::default(),
            list: RefCell::default(),
            items: Rc::new(VecModel::default()),
            thumbs: RefCell::default(),
            pending: RefCell::default(),
            cell: Cell::new((0.0, 0.0)),
            viewing: RefCell::default(),
            running: Cell::new(false),
        });
        SHOTS.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<ScreenshotsState>();
        st.set_items(ModelRc::from(me.items.clone()));
        on!(me, st, on_cell_size, |s, w, h| {
            if s.cell.replace((w, h)) != (w, h) {
                s.refresh();
            }
        });
        on!(me, st, on_view, |s, i| s.view(i));
        on!(me, st, on_open_folder, |s| {
            if let Some(d) = s.detail.detail() {
                s.actions.open_path(&d.game_dir.join("screenshots"));
            }
        });
        on!(me, st, on_open_file, |s| {
            if let Some(p) = s.viewing.borrow().clone() {
                s.actions.open_path(&p);
            }
        });
        on!(me, st, on_delete, |s| s.delete());
        let weak = Rc::downgrade(&me);
        detail.on(move |e| {
            if let Some(s) = weak.upgrade() {
                s.on_detail(e);
            }
        });
        // Al cerrarse el juego puede haber capturas nuevas.
        let weak = Rc::downgrade(&me);
        detail.controller().on_refresh(move |m| {
            if let Some(s) = weak.upgrade() {
                let Some(id) = s.instance.borrow().clone() else { return };
                let running = m.phase(&id) == Phase::Running;
                if s.running.replace(running) && !running && s.detail.tab() == InstanceTab::Screenshots {
                    s.actions.list(&id);
                }
            }
        });
        me
    }

    pub fn lang(&self) -> String {
        self.detail.lang()
    }

    fn on_detail(&self, e: &DetailEvent) {
        match *e {
            DetailEvent::Shown { id, tab } => {
                if self.instance.borrow().as_deref() != Some(id) {
                    self.reset(Some(id));
                }
                if tab == InstanceTab::Screenshots {
                    self.load();
                }
            }
            DetailEvent::Closed => self.reset(None),
            _ => {}
        }
    }

    fn reset(&self, id: Option<&str>) {
        *self.instance.borrow_mut() = id.map(str::to_owned);
        self.list.borrow_mut().clear();
        self.items.set_vec(Vec::new());
        self.thumbs.borrow_mut().clear();
        *self.viewing.borrow_mut() = None;
        self.running.set(id.is_some_and(|id| self.detail.controller().model.borrow().phase(id) == Phase::Running));
        if let Some(ui) = self.ui.upgrade() {
            let st = ui.global::<ScreenshotsState>();
            st.set_loading(false);
            st.set_error("".into());
            st.set_error_detail("".into());
            st.set_viewer_open(false);
            st.set_confirm_delete(false);
        }
    }

    pub fn load(&self) {
        let Some(id) = self.instance.borrow().clone() else { return };
        if let Some(ui) = self.ui.upgrade() {
            ui.global::<ScreenshotsState>().set_loading(true);
        }
        self.actions.list(&id);
    }

    /// Resultado de [`ScreenshotActions::list`].
    pub fn listed(&self, instance: &str, result: Result<Vec<ScreenshotInfo>, Failure>) {
        if self.instance.borrow().as_deref() != Some(instance) {
            return;
        }
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<ScreenshotsState>();
        st.set_loading(false);
        match result {
            Ok(list) => {
                st.set_error("".into());
                st.set_error_detail("".into());
                *self.list.borrow_mut() = list;
                self.refresh();
            }
            Err(f) if self.list.borrow().is_empty() => {
                st.set_error(f.message.into());
                st.set_error_detail(f.detail.into());
            }
            Err(f) => self.detail.controller().toast(&f.message, true),
        }
    }

    /// Copia la lista a la UI y pide las miniaturas que falten.
    fn refresh(&self) {
        let lang = self.lang();
        let (w, h) = self.cell.get();
        let ctl = self.detail.controller();
        let list = self.list.borrow();
        let mut items = Vec::with_capacity(list.len());
        for s in list.iter() {
            let image = match self.thumbs.borrow().get(&s.path) {
                Some(img) => img.clone(),
                // BlockLandscape(seed: archivo, block: 12) mientras tanto.
                None if w >= 1.0 && h >= 1.0 => ctl.landscape(&s.file, 12.0, w, h),
                None => Image::default(),
            };
            let caption = match s.modified {
                Some(t) => format_datetime(&lang, t + (self.utc_offset)(t)),
                None => s.file.clone(),
            };
            items.push(ShotItem { file: s.file.as_str().into(), caption: caption.into(), image });
        }
        let missing: Vec<PathBuf> = list.iter().filter(|s| !self.thumbs.borrow().contains_key(&s.path)).map(|s| s.path.clone()).collect();
        drop(list);
        if self.items.row_count() == items.len() {
            for (i, it) in items.into_iter().enumerate() {
                if self.items.row_data(i).as_ref() != Some(&it) {
                    self.items.set_row_data(i, it);
                }
            }
        } else {
            self.items.set_vec(items);
        }
        for p in missing {
            self.request(&p, THUMB_WIDTH);
        }
    }

    fn request(&self, path: &Path, max_width: u32) {
        if self.pending.borrow_mut().insert((path.to_owned(), max_width)) {
            self.actions.load_image(path, max_width);
        }
    }

    /// Resultado de [`ScreenshotActions::load_image`] (`None` = no se pudo:
    /// se queda el paisaje).
    pub fn image_loaded(&self, path: &Path, max_width: u32, pixels: Option<Pixels>) {
        self.pending.borrow_mut().remove(&(path.to_owned(), max_width));
        let Some(px) = pixels else { return };
        let img = Image::from_rgba8(px);
        if max_width == THUMB_WIDTH {
            if !self.list.borrow().iter().any(|s| s.path == path) {
                return;
            }
            self.thumbs.borrow_mut().insert(path.to_owned(), img);
            self.refresh();
        } else if self.viewing.borrow().as_deref() == Some(path)
            && let Some(ui) = self.ui.upgrade()
        {
            ui.global::<ScreenshotsState>().set_viewer_image(img);
        }
    }

    /// Abre el visor en la captura `i` (o pasa a ella).
    pub fn view(&self, i: i32) {
        let Some(ui) = self.ui.upgrade() else { return };
        let Some(path) = usize::try_from(i).ok().and_then(|i| self.list.borrow().get(i).map(|s| s.path.clone())) else { return };
        let st = ui.global::<ScreenshotsState>();
        st.set_viewer_index(i);
        st.set_viewer_open(true);
        if self.viewing.borrow().as_deref() != Some(path.as_path()) {
            st.set_viewer_image(Image::default());
            *self.viewing.borrow_mut() = Some(path.clone());
            self.request(&path, FULL_WIDTH);
        }
    }

    fn delete(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let Some(id) = self.instance.borrow().clone() else { return };
        let i = ui.global::<ScreenshotsState>().get_viewer_index();
        let file = usize::try_from(i).ok().and_then(|i| self.list.borrow().get(i).map(|s| s.file.clone()));
        if let Some(file) = file {
            self.actions.delete(&id, &file);
        }
    }

    /// Resultado de [`ScreenshotActions::delete`]: fuera de la lista y el
    /// visor pasa a la siguiente (o se cierra si no quedan).
    pub fn deleted(&self, instance: &str, file: &str, result: Result<(), Failure>) {
        if self.instance.borrow().as_deref() != Some(instance) {
            return;
        }
        if let Err(f) = result {
            self.detail.controller().toast(&f.message, true);
            return;
        }
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<ScreenshotsState>();
        let Some(index) = self.list.borrow().iter().position(|s| s.file == file) else { return };
        let removed = self.list.borrow_mut().remove(index);
        self.thumbs.borrow_mut().remove(&removed.path);
        let left = self.list.borrow().len();
        self.refresh();
        if self.viewing.borrow().as_deref() == Some(removed.path.as_path()) {
            *self.viewing.borrow_mut() = None;
            match after_delete(index, left) {
                Some(next) => self.view(next as i32),
                None => st.set_viewer_open(false),
            }
        }
    }
}

/// [`ScreenshotActions`] contra el núcleo.
pub struct CoreScreenshots(pub Backend);

/// Decodifica `path` y lo reduce a `max_width` de ancho (0 = sin reducir).
pub fn decode(path: &Path, max_width: u32) -> Option<Pixels> {
    let img = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.decode().ok()?;
    let img = if max_width > 0 && img.width() > max_width {
        let h = (u64::from(img.height()) * u64::from(max_width) / u64::from(img.width())).max(1) as u32;
        img.resize_exact(max_width, h, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let rgba = img.into_rgba8();
    Some(SharedPixelBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height()))
}

impl ScreenshotActions for CoreScreenshots {
    fn list(&self, instance: &str) {
        let (core, id) = (self.0.core.clone(), instance.to_owned());
        self.0.rt.spawn(async move {
            let r = core.list_screenshots(&id).await;
            post(move |s| s.listed(&id, r.map_err(|e| Failure::new(&s.lang(), &e))));
        });
    }

    fn load_image(&self, path: &Path, max_width: u32) {
        let path = path.to_owned();
        self.0.rt.spawn_blocking(move || {
            let px = decode(&path, max_width);
            post(move |s| s.image_loaded(&path, max_width, px));
        });
    }

    fn delete(&self, instance: &str, file: &str) {
        let (core, id, file) = (self.0.core.clone(), instance.to_owned(), file.to_owned());
        self.0.rt.spawn(async move {
            let r = core.delete_screenshot(&id, &file).await;
            post(move |s| s.deleted(&id, &file, r.map_err(|e| Failure::new(&s.lang(), &e))));
        });
    }

    fn open_path(&self, path: &Path) {
        crate::instance::open_path(path);
    }
}

impl ScreenshotActions for crate::fixtures::Recorder {
    fn list(&self, instance: &str) {
        self.push(format!("screenshots {instance}"));
    }
    fn load_image(&self, path: &Path, max_width: u32) {
        self.push(format!("image {} {max_width}", path.display()));
    }
    fn delete(&self, instance: &str, file: &str) {
        self.push(format!("delete-screenshot {instance} {file}"));
    }
    fn open_path(&self, path: &Path) {
        self.push(format!("open {}", path.display()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fechas_como_intl() {
        // 2026-10-11 13:30 y 2026-10-06 0:30 (UTC, ya «locales»).
        let t = 1_791_725_400;
        assert_eq!(format_datetime("es", t), "11 oct 2026 13:30");
        assert_eq!(format_datetime("en", t), "Oct 11, 2026 13:30");
        let t = t - 5 * 86_400 - 13 * 3600;
        assert_eq!(format_datetime("es", t), "6 oct 2026 0:30");
        assert_eq!(format_datetime("en", t), "Oct 6, 2026 00:30");
        assert_eq!(format_datetime("es", 0), "1 ene 1970 0:00");
        assert_eq!(format_datetime("es", 951_782_400), "29 feb 2000 0:00");
        assert_eq!(format_datetime("es", 1_725_148_800), "1 sept 2024 0:00");
    }

    #[test]
    fn tras_borrar() {
        assert_eq!(after_delete(0, 3), Some(0));
        assert_eq!(after_delete(3, 3), Some(2));
        assert_eq!(after_delete(0, 0), None);
    }

    #[test]
    fn miniatura_reducida() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.png");
        image::RgbaImage::from_pixel(1200, 600, image::Rgba([10, 20, 30, 255])).save(&path).unwrap();
        let px = decode(&path, 600).unwrap();
        assert_eq!((px.width(), px.height()), (600, 300));
        let px = decode(&path, 0).unwrap();
        assert_eq!((px.width(), px.height()), (1200, 600));
        assert!(decode(&dir.path().join("falta.png"), 600).is_none());
    }
}
