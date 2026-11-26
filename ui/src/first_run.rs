//! Asistente de primer arranque (first_run/first_run_page.dart): idioma,
//! importar de Roxy y de `.minecraft` (simulando antes), cuenta y resumen.
//!
//! `FirstRun` corre en el hilo de la UI y rellena `FirstRunState`; el trabajo
//! con el núcleo lo hace un `FirstRunActions` (`CoreFirstRun` en la app, un
//! registro en los tests), que entrega los resultados con [`post`].

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use lcore::events::Event;
use slint::{ComponentHandle, ModelRc, VecModel};

use crate::controller::Controller;
use crate::errors::Failure;
use crate::prefs::UiPrefs;
use crate::settings::format_bytes;
use crate::{FirstRunState, LoadState, MainWindow, Route, RoxyRow, art, i18n};

/// ¿Hay que mostrar el asistente? Explícitamente pendiente, o sin marcar y
/// sin instancias ni cuentas (`needsFirstRun`).
pub fn needs_first_run(prefs: &UiPrefs, has_instances: bool, has_accounts: bool) -> bool {
    match prefs.first_run_done {
        Some(done) => !done,
        None => !has_instances && !has_accounts,
    }
}

/// Orígenes encontrados (`ImportSourcesView`).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Sources {
    pub roxy: Option<String>,
    pub minecraft: Option<String>,
}

/// Instancia del plan de Roxy (`PlannedInstanceView`).
#[derive(Clone, Debug, PartialEq)]
pub struct Planned {
    pub original_id: String,
    pub name: String,
    pub mc: String,
    pub loader: String,
    pub bytes: u64,
    pub backups: u32,
    pub skipped: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct RoxyPlan {
    pub instances: Vec<Planned>,
    pub shared_bytes: u64,
    pub warnings: Vec<String>,
}

/// Sección del plan de `.minecraft` (`SectionView`).
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub section: String,
    pub present: bool,
    pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MinecraftPlan {
    pub name: String,
    pub mc: String,
    pub loader: String,
    pub bytes: u64,
    pub sections: Vec<Section>,
    pub warnings: Vec<String>,
}

impl From<lcore::import::ImportPlan> for RoxyPlan {
    fn from(p: lcore::import::ImportPlan) -> Self {
        Self {
            instances: p
                .instances
                .into_iter()
                .map(|i| Planned {
                    original_id: i.original_id,
                    name: i.name,
                    mc: i.mc,
                    loader: i.loader,
                    bytes: i.bytes,
                    backups: i.backups as u32,
                    skipped: i.skipped_reason,
                })
                .collect(),
            shared_bytes: p.shared.libraries_bytes + p.shared.assets_bytes,
            warnings: p.warnings,
        }
    }
}

impl From<lcore::import::DotMinecraftPlan> for MinecraftPlan {
    fn from(p: lcore::import::DotMinecraftPlan) -> Self {
        Self {
            name: p.name,
            mc: p.game.mc,
            loader: p.game.loader.as_str().into(),
            bytes: p.bytes,
            sections: p
                .sections
                .into_iter()
                .map(|s| Section { section: s.section.as_str().into(), present: s.present, bytes: s.bytes })
                .collect(),
            warnings: p.warnings,
        }
    }
}

/// Nombre visible de una sección de `.minecraft` (`_section`).
fn section_key(s: &str) -> Option<&'static str> {
    Some(match s {
        "saves" => "sectionSaves",
        "mods" => "sectionMods",
        "config" => "sectionConfig",
        "resourcepacks" => "sectionResourcepacks",
        "shaderpacks" => "sectionShaderpacks",
        "screenshots" => "sectionScreenshots",
        "options" => "sectionOptions",
        "servers" => "sectionServers",
        _ => return None,
    })
}

/// Lo que el asistente pide fuera de la UI.
pub trait FirstRunActions {
    /// → [`FirstRun::sources`].
    fn detect(&self);
    /// → [`FirstRun::roxy_plan`].
    fn plan_roxy(&self);
    /// → [`FirstRun::minecraft_plan`].
    fn plan_minecraft(&self);
    /// Importa lo elegido, en orden → [`FirstRun::importing`] por origen y
    /// [`FirstRun::imported`] al final.
    fn import(&self, roxy: bool, minecraft: bool);
    /// Guarda el idioma (`None` = el del sistema).
    fn save_language(&self, language: Option<String>);
    /// Guarda `ui.json` (marca de asistente hecho).
    fn save_prefs(&self, prefs: &UiPrefs);
    /// Vuelve a leer las instancias (tras importar).
    fn reload(&self);
}

thread_local! {
    static FIRST_RUN: RefCell<Option<Rc<FirstRun>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el asistente.
pub fn post(f: impl FnOnce(&FirstRun) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || with(f));
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

pub fn with(f: impl FnOnce(&FirstRun)) {
    FIRST_RUN.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            f(s);
        }
    })
}

pub struct FirstRun {
    ui: slint::Weak<MainWindow>,
    shell: Rc<Controller>,
    actions: Box<dyn FirstRunActions>,
    language: RefCell<Option<String>>,
    sources: RefCell<Option<Sources>>,
    roxy: RefCell<Option<Result<RoxyPlan, Failure>>>,
    minecraft: RefCell<Option<Result<MinecraftPlan, Failure>>>,
    /// 0 eligiendo, 1 importando, 2 hecho.
    phase: Cell<i32>,
    /// Origen que se importa ahora.
    current: RefCell<Option<&'static str>>,
    started: Cell<bool>,
    rows: Rc<VecModel<RoxyRow>>,
}

impl FirstRun {
    /// `language` = ajuste actual del núcleo (`None` = el del sistema).
    pub fn new(ui: &MainWindow, shell: Rc<Controller>, actions: Box<dyn FirstRunActions>, language: Option<String>) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            shell,
            actions,
            language: RefCell::new(language),
            sources: RefCell::default(),
            roxy: RefCell::default(),
            minecraft: RefCell::default(),
            phase: Cell::new(0),
            current: RefCell::default(),
            started: Cell::new(false),
            rows: Rc::new(VecModel::default()),
        });
        FIRST_RUN.with(|c| *c.borrow_mut() = Some(me.clone()));
        let st = ui.global::<FirstRunState>();
        st.set_roxy_rows(ModelRc::from(me.rows.clone()));
        let weak = Rc::downgrade(&me);
        st.on_language_changed(move |i| {
            if let Some(f) = weak.upgrade() {
                f.set_language(match i {
                    1 => Some("es".into()),
                    2 => Some("en".into()),
                    _ => None,
                });
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_finish(move || {
            if let Some(f) = weak.upgrade() {
                f.finish();
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_run_import(move || {
            if let Some(f) = weak.upgrade() {
                f.run_import();
            }
        });
        // El plan de un origen se calcula al elegirlo.
        let weak = Rc::downgrade(&me);
        st.on_roxy_toggled(move |_| {
            if let Some(f) = weak.upgrade() {
                f.refresh();
            }
        });
        let weak = Rc::downgrade(&me);
        st.on_minecraft_toggled(move |on| {
            if let Some(f) = weak.upgrade() {
                if on && f.minecraft.borrow().is_none() {
                    f.actions.plan_minecraft();
                }
                f.refresh();
            }
        });
        me.refresh();
        me
    }

    /// Muestra el asistente en `step` (0 = bienvenida) y empieza a buscar
    /// qué importar.
    pub fn show(&self, step: i32) {
        let Some(ui) = self.ui.upgrade() else { return };
        ui.global::<FirstRunState>().set_step(step);
        ui.set_first_run(true);
        if !self.started.replace(true) {
            self.actions.detect();
        }
        self.refresh();
    }

    fn set_language(&self, language: Option<String>) {
        if *self.language.borrow() == language {
            return;
        }
        *self.language.borrow_mut() = language.clone();
        let lang = i18n::resolve(language.as_deref(), i18n::system_locale().as_deref());
        if lang != self.shell.lang() {
            self.shell.set_language(lang);
        }
        self.actions.save_language(language);
        self.refresh();
    }

    /// Saltar o «Ir a Shard»: se marca hecho y se va a Inicio.
    fn finish(&self) {
        let mut p = self.shell.appearance.prefs();
        p.first_run_done = Some(true);
        self.shell.set_prefs(p.clone());
        self.actions.save_prefs(&p);
        if let Some(ui) = self.ui.upgrade() {
            ui.set_first_run(false);
            ui.set_route(Route::Home);
            ui.invoke_route_changed(Route::Home);
        }
    }

    pub fn sources(&self, r: Result<Sources, Failure>) {
        match r {
            Ok(s) => {
                let roxy = s.roxy.is_some();
                let minecraft = s.minecraft.is_some();
                *self.sources.borrow_mut() = Some(s);
                if roxy {
                    self.actions.plan_roxy();
                }
                // Si solo está .minecraft, se marca (Flutter empieza con
                // Roxy marcado y .minecraft no).
                let wants_mc = self.ui.upgrade().is_some_and(|u| u.global::<FirstRunState>().get_minecraft_selected());
                if minecraft && wants_mc {
                    self.actions.plan_minecraft();
                }
                self.set_state(|st| st.set_sources_state(LoadState::Ready));
            }
            Err(f) => self.set_state(|st| {
                st.set_sources_error(f.message.as_str().into());
                st.set_sources_error_detail(f.detail.as_str().into());
                st.set_sources_state(LoadState::Failed);
            }),
        }
        self.refresh();
    }

    pub fn roxy_plan(&self, r: Result<RoxyPlan, Failure>) {
        *self.roxy.borrow_mut() = Some(r);
        self.refresh();
    }

    pub fn minecraft_plan(&self, r: Result<MinecraftPlan, Failure>) {
        *self.minecraft.borrow_mut() = Some(r);
        self.refresh();
    }

    fn run_import(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<FirstRunState>();
        let s = self.sources.borrow().clone().unwrap_or_default();
        let roxy = st.get_roxy_selected() && s.roxy.is_some();
        let minecraft = st.get_minecraft_selected() && s.minecraft.is_some();
        if self.phase.get() != 0 || !(roxy || minecraft) {
            return;
        }
        self.phase.set(1);
        st.set_import_done(0);
        st.set_import_total(0);
        st.set_import_error("".into());
        self.refresh();
        self.actions.import(roxy, minecraft);
    }

    /// Empieza a importar `source` (`roxy` o `minecraft`).
    pub fn importing(&self, source: &'static str) {
        *self.current.borrow_mut() = Some(source);
        self.set_state(|st| {
            st.set_import_done(0);
            st.set_import_total(0);
        });
        self.refresh();
    }

    /// Terminó: instancias nuevas y el primer error, si hubo.
    pub fn imported(&self, count: u32, error: Option<Failure>) {
        self.phase.set(2);
        *self.current.borrow_mut() = None;
        self.set_state(|st| {
            st.set_import_count(count as i32);
            match &error {
                Some(f) => {
                    st.set_import_error(f.message.as_str().into());
                    st.set_import_error_detail(f.detail.as_str().into());
                }
                None => st.set_import_error("".into()),
            }
        });
        self.actions.reload();
        self.refresh();
    }

    /// Progreso de la importación (etapa `import`).
    pub fn on_event(&self, e: &Event) {
        if let Event::Progress { stage, done, total, .. } = e
            && stage == "import"
            && self.phase.get() == 1
        {
            let (d, t) = (*done, *total);
            self.set_state(|st| {
                st.set_import_done(d.min(i32::MAX as u64) as i32);
                st.set_import_total(t.min(i32::MAX as u64) as i32);
            });
        }
    }

    fn set_state(&self, f: impl FnOnce(&FirstRunState)) {
        if let Some(ui) = self.ui.upgrade() {
            f(&ui.global::<FirstRunState>());
        }
    }

    /// Copia el estado a la UI.
    pub fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<FirstRunState>();
        let lang = self.shell.lang();
        st.set_language(match self.language.borrow().as_deref() {
            Some("es") => 1,
            Some("en") => 2,
            _ => 0,
        });
        let s = self.sources.borrow().clone().unwrap_or_default();
        st.set_roxy_path(s.roxy.clone().unwrap_or_default().into());
        st.set_minecraft_path(s.minecraft.clone().unwrap_or_default().into());
        let phase = self.phase.get();
        st.set_import_phase(phase);
        let current = *self.current.borrow();
        let status = |source: &str, chosen: bool| -> i32 {
            if phase == 0 || !chosen {
                0
            } else if current == Some(source) {
                1
            } else if phase == 2 || current.is_some() {
                2
            } else {
                0
            }
        };
        st.set_roxy_status(status("roxy", st.get_roxy_selected()));
        st.set_minecraft_status(status("minecraft", st.get_minecraft_selected()));

        match self.roxy.borrow().as_ref() {
            None => st.set_roxy_plan_state(LoadState::Loading),
            Some(Err(f)) => {
                st.set_roxy_plan_error(f.message.as_str().into());
                st.set_roxy_plan_state(LoadState::Failed);
            }
            Some(Ok(p)) => {
                let todo: Vec<&Planned> = p.instances.iter().filter(|i| i.skipped.is_none()).collect();
                st.set_roxy_count(todo.len() as i32);
                st.set_roxy_size(format_bytes(&lang, todo.iter().map(|i| i.bytes).sum()).into());
                st.set_roxy_shared(format_bytes(&lang, p.shared_bytes).into());
                let rows: Vec<RoxyRow> = p
                    .instances
                    .iter()
                    .map(|i| {
                        let (from, to) = art::icon_colors(&i.original_id);
                        RoxyRow {
                            name: i.name.as_str().into(),
                            initials: art::initials(&i.name).into(),
                            icon_from: rgba(from),
                            icon_to: rgba(to),
                            mc: i.mc.as_str().into(),
                            loader: i.loader.as_str().into(),
                            size: format_bytes(&lang, i.bytes).into(),
                            backups: i.backups as i32,
                            skipped: i.skipped.clone().unwrap_or_default().into(),
                        }
                    })
                    .collect();
                crate::controller::sync(&self.rows, rows);
                st.set_roxy_warnings(strings(&p.warnings));
                st.set_roxy_plan_state(LoadState::Ready);
            }
        }
        match self.minecraft.borrow().as_ref() {
            None => st.set_minecraft_plan_state(LoadState::Loading),
            Some(Err(f)) => {
                st.set_minecraft_plan_error(f.message.as_str().into());
                st.set_minecraft_plan_state(LoadState::Failed);
            }
            Some(Ok(p)) => {
                st.set_minecraft_name(p.name.as_str().into());
                st.set_minecraft_mc(p.mc.as_str().into());
                st.set_minecraft_loader(p.loader.as_str().into());
                st.set_minecraft_size(format_bytes(&lang, p.bytes).into());
                let tags: Vec<String> = p
                    .sections
                    .iter()
                    .filter(|s| s.present)
                    .map(|s| {
                        let name = section_key(&s.section).map(|k| i18n::text(&lang, k)).unwrap_or_else(|| s.section.clone());
                        format!("{name} · {}", format_bytes(&lang, s.bytes))
                    })
                    .collect();
                st.set_minecraft_tags(strings(&tags));
                st.set_minecraft_warnings(strings(&p.warnings));
                st.set_minecraft_plan_state(LoadState::Ready);
            }
        }
    }
}

fn strings(list: &[String]) -> ModelRc<slint::SharedString> {
    ModelRc::new(VecModel::from(list.iter().map(|s| s.as_str().into()).collect::<Vec<slint::SharedString>>()))
}

fn rgba(c: art::Rgba) -> slint::Color {
    slint::Color::from_argb_u8((c.a * 255.0).round() as u8, c.r.round() as u8, c.g.round() as u8, c.b.round() as u8)
}

/// El asistente contra el núcleo (sustituye a `api/import.rs`).
pub struct CoreFirstRun {
    pub backend: crate::backend::Backend,
    pub lang: String,
}

impl FirstRunActions for CoreFirstRun {
    fn detect(&self) {
        self.backend.rt.spawn(async move {
            let roxy = lcore::import::roxy::default_source_root();
            let mc = lcore::import::minecraft::default_source_root();
            let roxy_ok = tokio::fs::metadata(roxy.join("game").join("accounts")).await.is_ok_and(|m| m.is_dir());
            let mut mc_ok = false;
            for marker in ["versions", "launcher_profiles.json", "saves"] {
                if tokio::fs::try_exists(mc.join(marker)).await.unwrap_or(false) {
                    mc_ok = true;
                    break;
                }
            }
            let s = Sources {
                roxy: roxy_ok.then(|| roxy.display().to_string()),
                minecraft: mc_ok.then(|| mc.display().to_string()),
            };
            post(move |f| f.sources(Ok(s)));
        });
    }

    fn plan_roxy(&self) {
        let (core, lang) = (self.backend.core.clone(), self.lang.clone());
        self.backend.rt.spawn(async move {
            let opts = lcore::import::RoxyImportOptions { dry_run: true, ..Default::default() };
            let r = core.plan_roxy_import(&opts).await.map(RoxyPlan::from).map_err(|e| Failure::new(&lang, &e));
            post(move |f| f.roxy_plan(r));
        });
    }

    fn plan_minecraft(&self) {
        let (core, lang) = (self.backend.core.clone(), self.lang.clone());
        self.backend.rt.spawn(async move {
            let mut opts = lcore::import::DotMinecraftImport::new(lcore::import::minecraft::default_source_root());
            opts.dry_run = true;
            let r = core.import_dot_minecraft(&opts).await.map(|r| MinecraftPlan::from(r.plan)).map_err(|e| Failure::new(&lang, &e));
            post(move |f| f.minecraft_plan(r));
        });
    }

    fn import(&self, roxy: bool, minecraft: bool) {
        let (core, lang) = (self.backend.core.clone(), self.lang.clone());
        self.backend.rt.spawn(async move {
            let mut count = 0u32;
            let mut error = None;
            if roxy {
                post(|f| f.importing("roxy"));
                let opts = lcore::import::RoxyImportOptions { dry_run: false, ..Default::default() };
                match core.import_roxy(&opts).await {
                    Ok(r) => count += r.imported.len() as u32,
                    Err(e) => error = error.or(Some(Failure::new(&lang, &e))),
                }
            }
            if minecraft {
                post(|f| f.importing("minecraft"));
                let opts = lcore::import::DotMinecraftImport::new(lcore::import::minecraft::default_source_root());
                match core.import_dot_minecraft(&opts).await {
                    Ok(r) => count += u32::from(r.imported.is_some()),
                    Err(e) => error = error.or(Some(Failure::new(&lang, &e))),
                }
            }
            post(move |f| f.imported(count, error));
        });
    }

    fn save_language(&self, language: Option<String>) {
        let core = self.backend.core.clone();
        self.backend.rt.spawn(async move {
            if let Err(e) = core.update_settings(|s| s.language = language).await {
                tracing::warn!(error = %e, "no se pudo guardar el idioma");
            }
        });
    }

    fn save_prefs(&self, prefs: &UiPrefs) {
        // En el hilo de la UI, como Ajustes: sin escrituras cruzadas.
        if let Err(e) = crate::prefs::save(self.backend.core.paths().root(), prefs) {
            tracing::warn!(error = %e, "no se pudo guardar ui.json");
        }
    }

    fn reload(&self) {
        self.backend.load();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asistente_solo_la_primera_vez() {
        let mut p = UiPrefs::default();
        assert!(needs_first_run(&p, false, false));
        assert!(!needs_first_run(&p, true, false), "con instancias, sin marca: no");
        assert!(!needs_first_run(&p, false, true), "con cuentas, sin marca: no");
        p.first_run_done = Some(false);
        assert!(needs_first_run(&p, true, true), "pendiente a mano: sí");
        p.first_run_done = Some(true);
        assert!(!needs_first_run(&p, false, false));
    }
}
