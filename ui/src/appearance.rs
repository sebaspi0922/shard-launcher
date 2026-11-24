//! Apariencia: estilo, acento y fondo (`buildTheme`, `accentProvider` y
//! `AppBackdrop` de Flutter). Copia los colores a `Palette`, los radios a
//! `Theme` y pinta el fondo que toque (`backdrop`). Corre en el hilo de la UI.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::ComponentHandle;

use crate::backdrop::{self, Sphere};
use crate::prefs::{AccentMode, UiPrefs};
use crate::theme::{Colors, Style, brand_tones};
use crate::{Backdrop, BackdropKind, MainWindow, Palette, Theme};

fn color(argb: u32) -> slint::Color {
    slint::Color::from_argb_u8((argb >> 24) as u8, (argb >> 16) as u8, (argb >> 8) as u8, argb as u8)
}

/// Escala de la UI para un tamaño de ventana (`uiScaleFor`): 1 a 1280 × 800,
/// entre 0,8 y 1,5.
pub fn ui_scale_for(w: f32, h: f32) -> f32 {
    (w / 1280.0).min(h / 800.0).clamp(0.8, 1.5)
}

/// Fondo que toca según las preferencias (lo que decide `AppBackdrop`).
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Atmosphere,
    Sphere,
    Aurora,
    Landscape,
    Photo(std::path::PathBuf),
}

pub fn kind_for(p: &UiPrefs) -> Kind {
    if let Some(path) = p.uses_image().filter(|p| p.is_file()) {
        return Kind::Photo(path.to_owned());
    }
    match p.style {
        Style::Graphite => Kind::Atmosphere,
        Style::Sebaspi => Kind::Sphere,
        Style::Aurora => Kind::Aurora,
        Style::Landscape => Kind::Landscape,
    }
}

/// Acento elegido (`accentOverrideProvider`); `None` = el del estilo.
pub fn accent_override(p: &UiPrefs, image_accent: Option<u32>) -> Option<u32> {
    match p.accent {
        AccentMode::Style => None,
        AccentMode::Preset => Some(p.accent_preset),
        AccentMode::Auto => p.uses_image().and(image_accent),
    }
}

pub struct Appearance {
    ui: slint::Weak<MainWindow>,
    prefs: RefCell<UiPrefs>,
    image_accent: Cell<Option<u32>>,
    colors: Cell<Colors>,
    /// Tamaño lógico y escala física con que se pintó el fondo.
    painted: Cell<(f32, f32, f32)>,
    size: Cell<(f32, f32)>,
    repaint: slint::Timer,
    sphere: RefCell<Sphere>,
    sphere_timer: slint::Timer,
    /// Segundos de giro fijos (escenas); `None` = reloj real.
    sphere_at: Cell<Option<f64>>,
    started: Instant,
    /// Escala de la ventana que puso el sistema y la que aplicamos.
    base_scale: Cell<f32>,
    applied_scale: Cell<f32>,
}

impl Appearance {
    pub fn new(ui: &MainWindow) -> Rc<Self> {
        let scale = ui.window().scale_factor();
        Rc::new(Self {
            ui: ui.as_weak(),
            prefs: RefCell::default(),
            image_accent: Cell::new(None),
            colors: Cell::new(Colors::for_style(Style::Graphite, None)),
            painted: Cell::new((0.0, 0.0, 0.0)),
            size: Cell::new((0.0, 0.0)),
            repaint: slint::Timer::default(),
            sphere: RefCell::new(Sphere::new()),
            sphere_timer: slint::Timer::default(),
            sphere_at: Cell::new(None),
            started: Instant::now(),
            base_scale: Cell::new(scale),
            applied_scale: Cell::new(scale),
        })
    }

    pub fn prefs(&self) -> UiPrefs {
        self.prefs.borrow().clone()
    }

    pub fn colors(&self) -> Colors {
        self.colors.get()
    }

    /// Cambia las preferencias y lo vuelve a aplicar todo.
    pub fn set_prefs(self: &Rc<Self>, p: UiPrefs) {
        *self.prefs.borrow_mut() = p;
        self.apply();
    }

    /// Acento extraído de la imagen de fondo (llega de un hilo de fondo).
    pub fn set_image_accent(self: &Rc<Self>, accent: Option<u32>) {
        self.image_accent.set(accent);
        self.apply();
    }

    /// Escenas: esfera quieta en `t` segundos.
    pub fn set_sphere_time(self: &Rc<Self>, t: Option<f64>) {
        self.sphere_at.set(t);
        self.paint_sphere();
    }

    fn apply(self: &Rc<Self>) {
        let Some(ui) = self.ui.upgrade() else { return };
        let p = self.prefs.borrow().clone();
        let c = Colors::for_style(p.style, accent_override(&p, self.image_accent.get()));
        self.colors.set(c);
        let pal = ui.global::<Palette>();
        pal.set_background(color(c.background));
        pal.set_background_deep(color(c.background_deep));
        pal.set_surface1(color(c.surface1));
        pal.set_surface2(color(c.surface2));
        pal.set_surface3(color(c.surface3));
        pal.set_border(color(c.border));
        pal.set_border_strong(color(c.border_strong));
        pal.set_text(color(c.text));
        pal.set_text_secondary(color(c.text_secondary));
        pal.set_text_muted(color(c.text_muted));
        pal.set_accent(color(c.accent.accent));
        pal.set_accent_hover(color(c.accent.hover));
        pal.set_on_accent(color(c.accent.on_accent));
        pal.set_accent_container(color(c.accent.container));
        pal.set_on_accent_container(color(c.accent.on_container));
        pal.set_glow(color(c.accent.glow));
        pal.set_success(color(c.success));
        pal.set_warning(color(c.warning));
        pal.set_danger(color(c.danger));
        pal.set_info(color(c.info));
        pal.set_panel_base(color(c.panel_base));
        let brand: Vec<slint::Color> = brand_tones(c.accent.accent).into_iter().map(color).collect();
        pal.set_brand(slint::ModelRc::new(slint::VecModel::from(brand)));
        let (control, card, panel) = p.style.radii();
        let th = ui.global::<Theme>();
        th.set_radius_control(control);
        th.set_radius_card(card);
        th.set_radius_panel(panel);
        th.set_motion(!p.performance);
        ui.set_animate_background(!p.performance);
        // Fondo: hay que repintarlo.
        self.painted.set((0.0, 0.0, 0.0));
        self.paint_backdrop();
    }

    /// La ventana cambió de tamaño: escala de UI y, sin prisa, el fondo.
    pub fn resized(self: &Rc<Self>, w: f32, h: f32) {
        self.size.set((w, h));
        self.rescale(w, h);
        if self.painted.get().2 == 0.0 {
            self.paint_backdrop();
            return;
        }
        // Mientras se arrastra el borde se estira la imagen vieja.
        let weak = Rc::downgrade(self);
        self.repaint.start(slint::TimerMode::SingleShot, Duration::from_millis(120), move || {
            if let Some(a) = weak.upgrade() {
                a.paint_backdrop();
            }
        });
    }

    /// `UiScale`: toda la UI escala con la ventana respecto a 1280 × 800.
    /// Se hace con el factor de escala de la ventana (sistema × UI).
    fn rescale(&self, w: f32, h: f32) {
        let Some(ui) = self.ui.upgrade() else { return };
        let current = ui.window().scale_factor();
        if (current - self.applied_scale.get()).abs() > 1e-4 {
            // Lo cambió el sistema (otra pantalla, otro DPI).
            self.base_scale.set(current);
        }
        let base = self.base_scale.get();
        // Tamaño a la escala del sistema.
        let (sw, sh) = (w * current / base, h * current / base);
        let u = ui_scale_for(sw, sh);
        let target = base * u;
        ui.global::<Theme>().set_ui_scale(u);
        self.applied_scale.set(target);
        if (target - current).abs() > 1e-4 {
            ui.window().dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged { scale_factor: target });
            // Slint conserva el tamaño lógico al cambiar la escala: el mismo
            // tamaño físico, medido con la escala nueva (lo que haría winit).
            let k = current / target;
            ui.window().dispatch_event(slint::platform::WindowEvent::Resized { size: slint::LogicalSize::new(w * k, h * k) });
        }
    }

    fn paint_backdrop(self: &Rc<Self>) {
        let Some(ui) = self.ui.upgrade() else { return };
        let (w, h) = self.size.get();
        if w < 1.0 || h < 1.0 {
            return;
        }
        let scale = ui.window().scale_factor();
        if self.painted.get() == (w, h, scale) {
            return;
        }
        self.painted.set((w, h, scale));
        let p = self.prefs.borrow().clone();
        let c = self.colors.get();
        let bd = ui.global::<Backdrop>();
        let (wf, hf, sf) = (w as f64, h as f64, scale as f64);
        let blur = if p.performance { 0.0 } else { p.blur };
        self.sphere_timer.stop();
        match kind_for(&p) {
            Kind::Atmosphere => {
                bd.set_kind(BackdropKind::Atmosphere);
                bd.set_base(slint::Image::default());
            }
            Kind::Aurora => {
                bd.set_kind(BackdropKind::Image);
                bd.set_dim(-1.0);
                bd.set_base(backdrop::aurora(c.accent.glow, c.background_deep, wf, hf, sf).image());
            }
            Kind::Landscape => {
                bd.set_kind(BackdropKind::Image);
                bd.set_dim(p.dim as f32);
                bd.set_base(backdrop::landscape(wf, hf, sf, blur).image());
            }
            Kind::Photo(path) => {
                bd.set_kind(BackdropKind::Image);
                bd.set_dim(p.dim as f32);
                match backdrop::photo(&path, wf, hf, sf, blur) {
                    Ok(px) => bd.set_base(px.image()),
                    Err(e) => {
                        // Como `errorBuilder`: el fondo de halos.
                        tracing::warn!(path = %path.display(), error = %e, "no se pudo leer la imagen de fondo");
                        bd.set_base(backdrop::aurora(c.accent.glow, c.background_deep, wf, hf, sf).image());
                    }
                }
            }
            Kind::Sphere => {
                bd.set_kind(BackdropKind::Sphere);
                bd.set_dim(-1.0);
                bd.set_base(backdrop::sebaspi_base(c.accent.accent, wf, hf, sf).image());
                self.paint_sphere();
                if !p.performance && self.sphere_at.get().is_none() {
                    let weak = Rc::downgrade(self);
                    self.sphere_timer.start(
                        slint::TimerMode::Repeated,
                        Duration::from_millis(1000 / backdrop::SPHERE_FPS as u64),
                        move || {
                            if let Some(a) = weak.upgrade() {
                                a.paint_sphere();
                            }
                        },
                    );
                }
            }
        }
    }

    fn paint_sphere(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        if kind_for(&self.prefs.borrow()) != Kind::Sphere {
            return;
        }
        let (w, h) = self.size.get();
        if w < 1.0 {
            return;
        }
        let t = self.sphere_at.get().unwrap_or_else(|| self.started.elapsed().as_secs_f64());
        let scale = ui.window().scale_factor() as f64;
        let f = self.sphere.borrow_mut().frame(t, w as f64, h as f64, scale, self.colors.get().accent.accent);
        let bd = ui.global::<Backdrop>();
        bd.set_sphere(f.pixels.image());
        bd.set_sphere_x(f.x as f32);
        bd.set_sphere_y(f.y as f32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escala_de_ui_como_flutter() {
        assert_eq!(ui_scale_for(1280.0, 800.0), 1.0);
        assert_eq!(ui_scale_for(960.0, 600.0), 0.8);
        assert_eq!(ui_scale_for(1600.0, 800.0), 1.0);
        assert_eq!(ui_scale_for(1920.0, 1200.0), 1.5);
        assert_eq!(ui_scale_for(4000.0, 3000.0), 1.5);
    }

    #[test]
    fn fondo_segun_estilo_e_imagen() {
        let mut p = UiPrefs::default();
        assert_eq!(kind_for(&p), Kind::Atmosphere);
        p.style = Style::Sebaspi;
        assert_eq!(kind_for(&p), Kind::Sphere);
        // Imagen elegida pero que ya no existe: el del estilo.
        p.background_image = true;
        p.background_path = Some("/no/existe.png".into());
        assert_eq!(kind_for(&p), Kind::Sphere);
        let img = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("img/shard-512.png");
        p.background_path = Some(img.clone());
        assert_eq!(kind_for(&p), Kind::Photo(img));
    }

    #[test]
    fn acento_del_estilo_preset_o_imagen() {
        let mut p = UiPrefs::default();
        assert_eq!(accent_override(&p, Some(1)), None);
        p.accent = AccentMode::Preset;
        assert_eq!(accent_override(&p, Some(1)), Some(0xFF35D69B));
        p.accent = AccentMode::Auto;
        assert_eq!(accent_override(&p, Some(1)), None, "sin imagen elegida no hay acento automático");
        p.background_image = true;
        p.background_path = Some("/x.png".into());
        assert_eq!(accent_override(&p, Some(7)), Some(7));
    }
}
