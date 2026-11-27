//! Ajustes (settings/settings_page.dart, appearance_group.dart,
//! java_group.dart): ajustes del núcleo, apariencia (`ui.json`), Java,
//! autoarranque, clave de CurseForge y actualizaciones.
//!
//! `Settings` corre en el hilo de la UI: copia el estado a `SettingsState` y
//! atiende sus callbacks. El trabajo fuera de la UI lo hace un
//! `SettingsActions` (`settings_backend::CoreSettings` en la app, un
//! registro en los tests), que entrega los resultados con [`post`].

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use lcore::events::Event;
use lcore::java::JavaSource;
use lcore::settings::CloseBehavior;
use slint::{ComponentHandle, Image, ModelRc, SharedPixelBuffer, VecModel};

use crate::controller::Controller;
use crate::errors::Failure;
use crate::prefs::{AccentMode, UiPrefs};
use crate::theme::{Colors, Style};
use crate::{JavaInstallItem, JavaItem, LoadState, MainWindow, Route, SettingsState, StyleCard, backdrop, i18n};

/// Lo que se edita en Ajustes de `settings.json` (`SettingsView`).
#[derive(Clone, Debug, PartialEq)]
pub struct Values {
    /// `None` = el idioma del sistema.
    pub language: Option<String>,
    pub close: CloseBehavior,
    pub multiple: bool,
    pub memory_mb: u32,
    pub jvm_args: Vec<String>,
    pub concurrency: u32,
    pub discord: bool,
    pub check_updates: bool,
    /// Hay cliente de Azure para el login de Microsoft.
    pub msa_set: bool,
}

impl Values {
    pub fn from_core(s: &lcore::settings::Settings) -> Self {
        Self {
            language: s.language.clone(),
            close: s.close_behavior,
            multiple: s.allow_multiple_games,
            memory_mb: s.default_memory_mb,
            jvm_args: s.default_jvm_args.clone(),
            concurrency: s.max_concurrent_downloads.min(u32::MAX as usize) as u32,
            discord: s.discord_rpc,
            check_updates: s.check_updates,
            msa_set: s.effective_msa_client_id().is_some(),
        }
    }

    /// Copia en `s` lo editable, con los mismos límites que `save_settings`
    /// del bridge.
    pub fn apply(&self, s: &mut lcore::settings::Settings) {
        s.language = self.language.clone();
        s.close_behavior = self.close;
        s.allow_multiple_games = self.multiple;
        s.default_memory_mb = self.memory_mb.max(512);
        s.default_jvm_args = self.jvm_args.clone();
        s.max_concurrent_downloads = self.concurrency.clamp(1, 64) as usize;
        s.discord_rpc = self.discord;
        s.check_updates = self.check_updates;
    }
}

/// `splitArgs`: argumentos separados por espacios.
pub fn split_args(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_owned).collect()
}

/// Una Java detectada (`JavaView`).
#[derive(Clone, Debug, PartialEq)]
pub struct Java {
    pub major: u32,
    pub version: String,
    pub vendor: Option<String>,
    pub path: String,
    pub source: JavaSource,
}

impl From<lcore::java::JavaInstall> for Java {
    fn from(j: lcore::java::JavaInstall) -> Self {
        Self { major: j.major, version: j.version, vendor: j.vendor, path: j.path.display().to_string(), source: j.source }
    }
}

/// Versiones mayores que se ofrecen para instalar (las que pide Minecraft).
pub const INSTALLABLE_JAVA: [u32; 4] = [8, 17, 21, 25];

/// `kAccentPresets` (ARGB).
pub const ACCENT_PRESETS: [u32; 8] =
    [0xFF35D69B, 0xFF4FA8FF, 0xFF9B7BFF, 0xFFFF6F91, 0xFFFFA94D, 0xFF3CC8E0, 0xFFE5C04A, 0xFFE879C6];

/// Máximos de los deslizadores de apariencia (el deslizador va de 0 a 1).
pub const BLUR_MAX: f64 = 40.0;
pub const DIM_MAX: f64 = 0.85;

/// Esperas antes de guardar (deslizadores y texto): ajustes del núcleo y
/// `ui.json`, como Flutter.
pub const SAVE_DEBOUNCE: Duration = Duration::from_millis(500);
pub const PREFS_DEBOUNCE: Duration = Duration::from_millis(400);

/// `formatBytes` de format.dart.
pub fn format_bytes(lang: &str, bytes: u64) -> String {
    let b = bytes as f64;
    let one = |v: f64| {
        let t = (v * 10.0).round() / 10.0;
        let s = if t.fract() == 0.0 { format!("{}", t as u64) } else { format!("{t:.1}") };
        s.replace('.', i18n::decimal(lang))
    };
    if b < 1024.0 {
        format!("{bytes} B")
    } else if b < 1024.0 * 1024.0 {
        format!("{} KB", (b / 1024.0).round() as u64)
    } else if b < 1024.0 * 1024.0 * 1024.0 {
        format!("{} MB", one(b / (1024.0 * 1024.0)))
    } else {
        format!("{} GB", one(b / (1024.0 * 1024.0 * 1024.0)))
    }
}

/// Lo que Ajustes pide fuera de la UI.
pub trait SettingsActions {
    /// Ajustes del núcleo → [`Settings::loaded`] con el mismo `req`.
    fn load(&self, req: u64);
    /// Guarda → [`Settings::saved`] con el mismo `seq`.
    fn save(&self, values: &Values, seq: u64);
    /// → [`Settings::java_listed`].
    fn list_java(&self);
    /// → [`Settings::java_installed`].
    fn install_java(&self, major: u32);
    /// ¿Hay autoarranque en este sistema?
    fn autostart_supported(&self) -> bool {
        true
    }
    /// → [`Settings::autostart_loaded`].
    fn load_autostart(&self);
    /// → [`Settings::autostart_set`].
    fn set_autostart(&self, on: bool);
    /// Guarda `ui.json`.
    fn save_prefs(&self, prefs: &UiPrefs);
    /// Elige una imagen y la copia a la carpeta de datos →
    /// [`Settings::background_imported`].
    fn pick_background(&self);
    /// Acento de la imagen de fondo → `Appearance::set_image_accent`.
    fn image_accent(&self, path: &Path);
    /// `true` si quedó en el portapapeles.
    fn copy_text(&self, text: &str) -> bool;
    /// Busca una versión nueva (manual).
    fn check_update(&self);
}

thread_local! {
    static SETTINGS: RefCell<Option<Rc<Settings>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador de Ajustes.
pub fn post(f: impl FnOnce(&Rc<Settings>) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || with(f));
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// `f` con el controlador, ya en el hilo de la UI.
pub fn with(f: impl FnOnce(&Rc<Settings>)) {
    SETTINGS.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            f(s);
        }
    })
}

/// Aviso de cambio de `close_behavior` (la ventana lo aplica).
type CloseListener = Box<dyn Fn(CloseBehavior)>;

pub struct Settings {
    ui: slint::Weak<MainWindow>,
    shell: Rc<Controller>,
    actions: Box<dyn SettingsActions>,
    values: RefCell<Option<Values>>,
    /// Último guardado pedido (los resultados viejos no pisan).
    seq: Cell<u64>,
    /// Último guardado terminado: mientras no llegue a `seq`, una lectura
    /// del núcleo puede ser vieja y no pisa lo editado.
    saved_seq: Cell<u64>,
    /// Cambios hechos aquí, la última lectura pedida y cuántos cambios había
    /// entonces: solo vale la respuesta a esa lectura, y si no hubo cambios.
    rev: Cell<u64>,
    load_req: Cell<u64>,
    load_rev: Cell<u64>,
    save_timer: slint::Timer,
    prefs_timer: slint::Timer,
    java: RefCell<Option<Result<Vec<Java>, Failure>>>,
    installing: Cell<Option<u32>>,
    /// Progreso de la instalación de Java (hecho, total) en bytes.
    java_progress: Cell<(u64, u64)>,
    styles: RefCell<Vec<StyleCard>>,
    image_thumb: RefCell<Option<(PathBuf, Image)>>,
    javas: Rc<VecModel<JavaItem>>,
    installs: Rc<VecModel<JavaInstallItem>>,
    close_listeners: RefCell<Vec<CloseListener>>,
}

fn argb(c: u32) -> slint::Color {
    slint::Color::from_argb_encoded(c)
}

impl Settings {
    /// Crea el controlador y lo registra para [`post`]. `total_mb` = RAM del
    /// equipo; `data_dir` = carpeta de datos; `core_version` = versión que se
    /// muestra en Acerca de.
    pub fn new(
        ui: &MainWindow,
        shell: Rc<Controller>,
        actions: Box<dyn SettingsActions>,
        total_mb: u64,
        data_dir: &str,
        core_version: &str,
    ) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            shell,
            actions,
            values: RefCell::default(),
            seq: Cell::new(0),
            saved_seq: Cell::new(0),
            rev: Cell::new(0),
            load_req: Cell::new(0),
            load_rev: Cell::new(0),
            save_timer: slint::Timer::default(),
            prefs_timer: slint::Timer::default(),
            java: RefCell::default(),
            installing: Cell::new(None),
            java_progress: Cell::new((0, 0)),
            styles: RefCell::default(),
            image_thumb: RefCell::default(),
            javas: Rc::new(VecModel::default()),
            installs: Rc::new(VecModel::default()),
            close_listeners: RefCell::default(),
        });
        SETTINGS.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<SettingsState>();
        st.set_javas(ModelRc::from(me.javas.clone()));
        st.set_installs(ModelRc::from(me.installs.clone()));
        st.set_presets(ModelRc::new(VecModel::from(ACCENT_PRESETS.iter().map(|c| argb(*c)).collect::<Vec<_>>())));
        st.set_total_mb(total_mb.min(i32::MAX as u64) as i32);
        st.set_data_dir(data_dir.into());
        st.set_core_version(core_version.into());
        st.set_autostart_supported(me.actions.autostart_supported());

        macro_rules! on {
            ($set:ident, | $s:ident $(, $arg:ident)* | $body:expr) => {{
                let weak = Rc::downgrade(&me);
                st.$set(move |$($arg),*| {
                    if let Some($s) = weak.upgrade() {
                        $body;
                    }
                });
            }};
        }
        on!(on_language_changed, |s, i| s.edit(false, |v| {
            v.language = match i {
                1 => Some("es".into()),
                2 => Some("en".into()),
                _ => None,
            }
        }));
        on!(on_close_changed, |s, i| s.edit(false, |v| {
            v.close = match i {
                1 => CloseBehavior::Tray,
                2 => CloseBehavior::Quit,
                _ => CloseBehavior::Ask,
            }
        }));
        on!(on_multiple_changed, |s, b| s.edit(false, |v| v.multiple = b));
        on!(on_memory_changed, |s, mb| s.edit(true, |v| v.memory_mb = mb.max(0) as u32));
        on!(on_jvm_changed, |s, t| s.edit(true, |v| v.jvm_args = split_args(&t)));
        on!(on_concurrency_changed, |s, n| s.edit(true, |v| v.concurrency = n.clamp(1, 32) as u32));
        on!(on_discord_changed, |s, b| s.edit(false, |v| v.discord = b));
        on!(on_auto_updates_changed, |s, b| s.edit(false, |v| v.check_updates = b));
        on!(on_autostart_changed, |s, b| {
            s.set_state(|st| st.set_autostart(b));
            s.actions.set_autostart(b);
        });

        on!(on_style_changed, |s, i| s.prefs(false, |p| p.style = Style::ALL[(i.clamp(0, 3)) as usize]));
        on!(on_background_style, |s| s.prefs(false, |p| p.background_image = false));
        on!(on_background_image, |s| s.prefs(false, |p| p.background_image = true));
        on!(on_pick_image, |s| s.actions.pick_background());
        on!(on_accent_style, |s| s.prefs(false, |p| p.accent = AccentMode::Style));
        on!(on_accent_auto, |s| s.prefs(false, |p| p.accent = AccentMode::Auto));
        on!(on_accent_preset, |s, i| s.prefs(false, |p| {
            p.accent = AccentMode::Preset;
            p.accent_preset = ACCENT_PRESETS[(i.clamp(0, 7)) as usize];
        }));
        on!(on_blur_changed, |s, x| s.prefs(true, |p| p.blur = (x as f64).clamp(0.0, 1.0) * BLUR_MAX));
        on!(on_dim_changed, |s, x| s.prefs(true, |p| p.dim = (x as f64).clamp(0.0, 1.0) * DIM_MAX));
        on!(on_performance_changed, |s, b| s.prefs(false, |p| p.performance = b));

        on!(on_java_rescan, |s| s.rescan_java());
        on!(on_java_install, |s, major| s.install_java(major.max(0) as u32));
        on!(on_check_updates, |s| s.actions.check_update());
        on!(on_copy_data_dir, |s| {
            let dir = s.ui.upgrade().map(|u| u.global::<SettingsState>().get_data_dir().to_string()).unwrap_or_default();
            if s.actions.copy_text(&dir) {
                s.toast(&i18n::text(&s.shell.lang(), "copied"), false);
            }
        });
        me.refresh_prefs();
        me.refresh_java();
        me
    }

    /// Número de la última lectura pedida (escenas y tests).
    pub fn last_load(&self) -> u64 {
        self.load_req.get()
    }

    /// Avisa de cada cambio de «al cerrar la ventana».
    pub fn on_close_changed(&self, f: impl Fn(CloseBehavior) + 'static) {
        self.close_listeners.borrow_mut().push(Box::new(f));
    }

    /// Al entrar en Ajustes: relee todo (como los providers de Flutter, que
    /// se crean al abrir la página).
    pub fn open(&self) {
        self.set_state(|st| st.set_scroll(0.0));
        if self.values.borrow().is_none() {
            self.set_state(|st| st.set_state(LoadState::Loading));
        }
        let req = self.load_req.get() + 1;
        self.load_req.set(req);
        self.load_rev.set(self.rev.get());
        self.actions.load(req);
        self.actions.load_autostart();
        self.rescan_java();
        self.refresh_prefs();
    }

    fn set_state(&self, f: impl FnOnce(&SettingsState)) {
        if let Some(ui) = self.ui.upgrade() {
            f(&ui.global::<SettingsState>());
        }
    }

    fn toast(&self, message: &str, error: bool) {
        self.shell.toast(message, error);
    }

    /// Ajustes del núcleo leídos.
    pub fn loaded(&self, req: u64, r: Result<Values, Failure>) {
        // La respuesta a una lectura anterior nunca vale (llegará la nueva).
        if req != self.load_req.get() {
            return;
        }
        let pending = self.save_timer.running() || self.saved_seq.get() < self.seq.get();
        let stale = self.load_rev.get() != self.rev.get();
        if (pending || stale) && self.values.borrow().is_some() {
            return;
        }
        match r {
            Ok(v) => {
                let jvm = v.jvm_args.join(" ");
                *self.values.borrow_mut() = Some(v);
                self.set_state(|st| st.set_jvm_args(jvm.into()));
                self.refresh_values();
            }
            Err(f) => {
                if self.values.borrow().is_none() {
                    self.set_state(|st| {
                        st.set_error_message(f.message.as_str().into());
                        st.set_error_detail(f.detail.as_str().into());
                        st.set_state(LoadState::Failed);
                    });
                }
            }
        }
    }

    /// Cambia un valor y lo guarda (al momento o tras `SAVE_DEBOUNCE`).
    fn edit(self: &Rc<Self>, debounce: bool, f: impl FnOnce(&mut Values)) {
        let next = {
            let mut cur = self.values.borrow_mut();
            let Some(v) = cur.as_mut() else { return };
            let before = v.clone();
            f(v);
            if *v == before {
                return;
            }
            v.clone()
        };
        self.rev.set(self.rev.get() + 1);
        // El idioma se ve al momento.
        let lang = i18n::resolve(next.language.as_deref(), i18n::system_locale().as_deref());
        if lang != self.shell.lang() {
            self.shell.set_language(lang);
            self.refresh_prefs();
        }
        for l in self.close_listeners.borrow().iter() {
            l(next.close);
        }
        self.refresh_values();
        if debounce {
            let weak = Rc::downgrade(self);
            self.save_timer.start(slint::TimerMode::SingleShot, SAVE_DEBOUNCE, move || {
                if let Some(s) = weak.upgrade() {
                    s.flush();
                }
            });
        } else {
            self.save_timer.stop();
            self.flush();
        }
    }

    /// Guarda ya lo pendiente.
    pub fn flush(&self) {
        let Some(v) = self.values.borrow().clone() else { return };
        self.save_timer.stop();
        let seq = self.seq.get() + 1;
        self.seq.set(seq);
        self.actions.save(&v, seq);
    }

    /// Resultado de guardar.
    pub fn saved(&self, seq: u64, r: Result<Values, Failure>) {
        self.saved_seq.set(self.saved_seq.get().max(seq));
        if let Err(f) = r {
            tracing::warn!(error = %f.detail, "no se pudieron guardar los ajustes");
            if seq == self.seq.get() {
                self.toast(&i18n::text(&self.shell.lang(), "settingsSaveFailed"), true);
            }
        }
    }

    fn refresh_values(&self) {
        let Some(v) = self.values.borrow().clone() else { return };
        self.set_state(|st| {
            st.set_language(match v.language.as_deref() {
                Some("es") => 1,
                Some("en") => 2,
                _ => 0,
            });
            st.set_close_behavior(match v.close {
                CloseBehavior::Ask => 0,
                CloseBehavior::Tray => 1,
                CloseBehavior::Quit => 2,
            });
            st.set_multiple_games(v.multiple);
            st.set_memory_mb(v.memory_mb.min(i32::MAX as u32) as i32);
            st.set_concurrency(v.concurrency.min(i32::MAX as u32) as i32);
            st.set_discord(v.discord);
            st.set_auto_updates(v.check_updates);
            st.set_msa_configured(v.msa_set);
            st.set_state(LoadState::Ready);
        });
    }

    // --- Apariencia -----------------------------------------------------

    /// Cambia `ui.json`, lo aplica a la ventana y lo guarda.
    fn prefs(self: &Rc<Self>, debounce: bool, f: impl FnOnce(&mut UiPrefs)) {
        let mut p = self.shell.appearance.prefs();
        let before = p.clone();
        f(&mut p);
        if p == before {
            return;
        }
        let needs_accent = p.accent == AccentMode::Auto
            && (before.accent != AccentMode::Auto || before.uses_image() != p.uses_image())
            && p.uses_image().is_some();
        self.shell.set_prefs(p.clone());
        if needs_accent && let Some(path) = p.uses_image() {
            self.actions.image_accent(path);
        }
        self.refresh_prefs();
        if debounce {
            let weak = Rc::downgrade(self);
            self.prefs_timer.start(slint::TimerMode::SingleShot, PREFS_DEBOUNCE, move || {
                if let Some(s) = weak.upgrade() {
                    s.actions.save_prefs(&s.shell.appearance.prefs());
                }
            });
        } else {
            self.prefs_timer.stop();
            self.actions.save_prefs(&p);
        }
    }

    /// Imagen elegida y copiada a la carpeta de datos (`None` = cancelada).
    pub fn background_imported(self: &Rc<Self>, r: Result<Option<PathBuf>, Failure>) {
        match r {
            Ok(Some(path)) => self.prefs(false, |p| {
                p.background_image = true;
                p.background_path = Some(path);
            }),
            Ok(None) => {}
            Err(f) => self.toast(&f.message, true),
        }
    }

    fn refresh_prefs(&self) {
        let p = self.shell.appearance.prefs();
        if self.styles.borrow().is_empty() {
            let scale = self.ui.upgrade().map(|u| u.window().scale_factor()).unwrap_or(1.0) as f64;
            let lang = self.shell.lang();
            *self.styles.borrow_mut() = Style::ALL.iter().map(|s| style_card(*s, &lang, scale)).collect();
        }
        let has_image = p.background_path.as_deref().is_some_and(Path::exists);
        let image = p.background_image && has_image;
        let thumb = if has_image { p.background_path.as_deref().and_then(|path| self.thumb(path)) } else { None };
        // El nombre en el idioma actual (cambia en vivo).
        let lang = self.shell.lang();
        let styles: Vec<StyleCard> = self
            .styles
            .borrow()
            .iter()
            .zip(Style::ALL)
            .map(|(c, s)| StyleCard { name: i18n::text(&lang, style_key(s)).into(), ..c.clone() })
            .collect();
        self.set_state(|st| {
            st.set_styles(ModelRc::new(VecModel::from(styles)));
            st.set_style(Style::ALL.iter().position(|s| *s == p.style).unwrap_or(0) as i32);
            st.set_has_image(has_image);
            st.set_uses_image(image);
            st.set_image_thumb(thumb.unwrap_or_default());
            st.set_accent_mode(match p.accent {
                AccentMode::Style => 0,
                AccentMode::Auto => 1,
                AccentMode::Preset => 2,
            });
            st.set_preset(ACCENT_PRESETS.iter().position(|c| *c == p.accent_preset).map(|i| i as i32).unwrap_or(-1));
            st.set_blur((p.blur / BLUR_MAX) as f32);
            st.set_dim((p.dim / DIM_MAX) as f32);
            st.set_tunable(image || p.style == Style::Landscape);
            st.set_performance(p.performance);
        });
    }

    /// Miniatura de la imagen de fondo (360 de ancho, como `cacheWidth`).
    fn thumb(&self, path: &Path) -> Option<Image> {
        if let Some((p, img)) = self.image_thumb.borrow().as_ref()
            && p == path
        {
            return Some(img.clone());
        }
        let img = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.decode().ok()?;
        let small = img.thumbnail(360, 360).to_rgba8();
        let out = Image::from_rgba8(SharedPixelBuffer::clone_from_slice(small.as_raw(), small.width(), small.height()));
        *self.image_thumb.borrow_mut() = Some((path.to_owned(), out.clone()));
        Some(out)
    }

    // --- Java -----------------------------------------------------------

    fn rescan_java(&self) {
        *self.java.borrow_mut() = None;
        self.refresh_java();
        self.actions.list_java();
    }

    pub fn java_listed(&self, r: Result<Vec<Java>, Failure>) {
        *self.java.borrow_mut() = Some(r);
        self.refresh_java();
    }

    fn install_java(&self, major: u32) {
        if self.installing.get().is_some() {
            return;
        }
        self.installing.set(Some(major));
        self.java_progress.set((0, 0));
        self.refresh_java();
        self.actions.install_java(major);
    }

    pub fn java_installed(&self, major: u32, r: Result<Java, Failure>) {
        self.installing.set(None);
        let lang = self.shell.lang();
        match r {
            Ok(j) => {
                let text = i18n::text(&lang, "javaInstalled").replace("{major}", &j.major.to_string()).replace("{version}", &j.version);
                self.toast(&text, false);
                self.rescan_java();
            }
            Err(f) => {
                tracing::warn!(major, error = %f.detail, "no se pudo instalar Java");
                self.toast(&f.message, true);
                self.refresh_java();
            }
        }
    }

    /// Eventos del núcleo: progreso de la instalación de Java.
    pub fn on_event(&self, e: &Event) {
        if let Event::Progress { stage, done, total, .. } = e
            && stage == "java"
            && self.installing.get().is_some()
        {
            self.java_progress.set((*done, *total));
            self.refresh_java();
        }
    }

    fn refresh_java(&self) {
        let lang = self.shell.lang();
        let java = self.java.borrow();
        let list: Vec<Java> = match java.as_ref() {
            Some(Ok(l)) => l.clone(),
            _ => vec![],
        };
        let rows: Vec<JavaItem> = list
            .iter()
            .map(|j| JavaItem {
                major: j.major as i32,
                title: match &j.vendor {
                    Some(v) => format!("{} · {v}", j.version),
                    None => j.version.clone(),
                }
                .into(),
                path: j.path.as_str().into(),
                source: match j.source {
                    JavaSource::Managed => 0,
                    JavaSource::Override => 1,
                    JavaSource::System => 2,
                },
            })
            .collect();
        let installing = self.installing.get();
        let (done, total) = self.java_progress.get();
        let installs: Vec<JavaInstallItem> = INSTALLABLE_JAVA
            .iter()
            .map(|m| JavaInstallItem {
                major: *m as i32,
                installed: list.iter().any(|j| j.major == *m && j.source == JavaSource::Managed),
                installing: installing == Some(*m),
                fraction: if installing == Some(*m) && total > 0 { (done as f32 / total as f32).min(1.0) } else { -1.0 },
            })
            .collect();
        let description = match installing {
            None => i18n::text(&lang, "javaInstallDesc"),
            Some(m) if total == 0 => i18n::text(&lang, "javaInstalling").replace("{major}", &m.to_string()),
            Some(m) => i18n::text(&lang, "javaInstallingProgress")
                .replace("{major}", &m.to_string())
                .replace("{done}", &format_bytes(&lang, done))
                .replace("{total}", &format_bytes(&lang, total)),
        };
        crate::controller::sync(&self.javas, rows);
        crate::controller::sync(&self.installs, installs);
        let state = match java.as_ref() {
            None => LoadState::Loading,
            Some(Ok(_)) => LoadState::Ready,
            Some(Err(_)) => LoadState::Failed,
        };
        let err = match java.as_ref() {
            Some(Err(f)) => Some(f.clone()),
            _ => None,
        };
        self.set_state(|st| {
            st.set_java_state(state);
            if let Some(f) = err {
                st.set_java_error(f.message.into());
                st.set_java_error_detail(f.detail.into());
            }
            st.set_java_installing(installing.is_some());
            st.set_install_description(description.into());
        });
    }

    // --- Autoarranque ------------------------------------

    pub fn autostart_loaded(&self, on: bool) {
        self.set_state(|st| st.set_autostart(on));
    }

    pub fn autostart_set(&self, r: Result<bool, Failure>) {
        match r {
            Ok(on) => self.autostart_loaded(on),
            Err(f) => {
                self.toast(&f.message, true);
                self.actions.load_autostart();
            }
        }
    }

    /// Escenas y tests: ¿qué se ve? (la página fijada en un desplazamiento).
    pub fn scroll_to(&self, y: f32) {
        self.set_state(|st| st.set_scroll(-y));
    }

    /// La ruta cambió: al entrar en Ajustes se relee; al salir se guarda lo
    /// pendiente (`dispose` de la página).
    pub fn route_changed(&self, r: Route) {
        if r == Route::Settings {
            self.open();
        } else if self.save_timer.running() {
            self.flush();
        }
    }
}

/// Miniatura de un estilo (StylePreview): fondo del estilo quieto, a escala
/// de la tarjeta (1280 × 800 → 192 × 120 lógicos).
pub fn style_card(style: Style, lang: &str, scale: f64) -> StyleCard {
    const K: f64 = 0.15;
    let c = Colors::for_style(style, None);
    let s = K * scale;
    let art = match style {
        Style::Graphite => None,
        Style::Sebaspi => {
            let mut base = backdrop::sebaspi_base(c.accent.accent, 1280.0, 800.0, s);
            let f = backdrop::Sphere::new().frame(0.0, 1280.0, 800.0, s, c.accent.accent);
            over(&mut base, &f.pixels, (f.x * s).round() as i64, (f.y * s).round() as i64);
            Some(base.image())
        }
        Style::Landscape => Some(backdrop::landscape(1280.0, 800.0, s, 0.0).image()),
        Style::Aurora => Some(backdrop::aurora(c.accent.glow, c.background_deep, 1280.0, 800.0, s).image()),
    };
    StyleCard {
        name: i18n::text(lang, style_key(style)).into(),
        atmosphere: art.is_none(),
        art: art.unwrap_or_default(),
        background: argb(c.background),
        accent: argb(c.accent.accent),
        radius: style.radii().0,
    }
}

/// Clave del ARB con el nombre del estilo.
fn style_key(style: Style) -> &'static str {
    match style {
        Style::Graphite => "styleGraphite",
        Style::Sebaspi => "styleSebaspi",
        Style::Landscape => "appearanceLandscape",
        Style::Aurora => "appearanceAurora",
    }
}

/// Pinta `top` (RGBA sin premultiplicar) sobre `base` en (x, y) físicos.
fn over(base: &mut backdrop::Pixels, top: &backdrop::Pixels, x: i64, y: i64) {
    for ty in 0..top.h {
        let by = y + ty as i64;
        if by < 0 || by >= base.h as i64 {
            continue;
        }
        for tx in 0..top.w {
            let bx = x + tx as i64;
            if bx < 0 || bx >= base.w as i64 {
                continue;
            }
            let t = &top.px[(ty * top.w + tx) * 4..][..4];
            let a = t[3] as f32 / 255.0;
            if a == 0.0 {
                continue;
            }
            let i = (by as usize * base.w + bx as usize) * 4;
            for (b, t) in base.px[i..i + 3].iter_mut().zip(t) {
                *b = (*t as f32 * a + *b as f32 * (1.0 - a)).round() as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argumentos_separados_por_espacios() {
        assert_eq!(split_args("  -Xss2M   -XX:+UseG1GC "), vec!["-Xss2M", "-XX:+UseG1GC"]);
        assert!(split_args("   ").is_empty());
    }

    #[test]
    fn bytes_como_format_dart() {
        assert_eq!(format_bytes("es", 512), "512 B");
        assert_eq!(format_bytes("es", 2048), "2 KB");
        assert_eq!(format_bytes("es", 412 * 1024 * 1024), "412 MB");
        assert_eq!(format_bytes("es", (1.4 * 1024.0 * 1024.0 * 1024.0) as u64), "1,4 GB");
        assert_eq!(format_bytes("en", (1.4 * 1024.0 * 1024.0 * 1024.0) as u64), "1.4 GB");
        assert_eq!(format_bytes("es", 3 * 1024 * 1024 * 1024), "3 GB");
    }

    #[test]
    fn valores_con_los_limites_del_bridge() {
        let mut core = lcore::settings::Settings::default();
        let mut v = Values::from_core(&core);
        v.memory_mb = 100;
        v.concurrency = 500;
        v.language = Some("en".into());
        v.apply(&mut core);
        assert_eq!(core.default_memory_mb, 512);
        assert_eq!(core.max_concurrent_downloads, 64);
        assert_eq!(core.language.as_deref(), Some("en"));
        assert_eq!(Values::from_core(&core).language.as_deref(), Some("en"));
    }
}
