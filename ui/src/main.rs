//! Shard (UI Slint).
//!
//!   shard-ui [--data-dir DIR] [--background]
//!   shard-ui --seed DIR        escribe las instancias de ejemplo y sale
//!
//! Renderer: software por defecto (el que dio 30 MB en el PC de Sebas);
//! `SLINT_BACKEND=winit-femtovg` usa OpenGL. `SHARD_TIMING=1` imprime
//! `SHARD_FIRST_FRAME` para `tools/medir.sh`.

use std::path::PathBuf;
use std::time::Instant;

use lcore::{Core, Options};
use shard_ui::accounts::Accounts;
use shard_ui::accounts_backend::{CoreAccounts, CoreSkins};
use shard_ui::backend::{self, Backend, CoreActions};
use shard_ui::settings::Settings;
use shard_ui::settings_backend::CoreSettings;
use shard_ui::skins::Skins;
use shard_ui::content::Content;
use shard_ui::content_backend::CoreContent;
use shard_ui::controller::Controller;
use shard_ui::servers::{CoreServers, Servers};
use shard_ui::curseforge::{CoreKeys, Keys, NoKeys};
use shard_ui::window::Shell;
use shard_ui::{MainWindow, i18n, prefs};
use slint::ComponentHandle;

struct Args {
    data_dir: Option<PathBuf>,
    seed: Option<PathBuf>,
    background: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut out = Args { data_dir: std::env::var_os("SHARD_DATA_DIR").map(PathBuf::from), seed: None, background: false };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--data-dir" => out.data_dir = Some(it.next().ok_or("--data-dir necesita una carpeta")?.into()),
            "--seed" => out.seed = Some(it.next().ok_or("--seed necesita una carpeta")?.into()),
            "--background" => out.background = true,
            o => return Err(format!("argumento desconocido: {o}")),
        }
    }
    Ok(out)
}

fn main() {
    let started = Instant::now();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_env("SHARD_LOG").unwrap_or_else(|_| "warn".into()))
        .init();
    let args = parse_args().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().expect("tokio");

    if let Some(dir) = args.seed {
        let r = rt.block_on(async {
            let core = Core::open(Options { data_dir: Some(dir.clone()), offline: true, ..Default::default() }).await?;
            shard_ui::fixtures::seed(&core).await
        });
        match r {
            Ok(()) => println!("sembrado en {}", dir.display()),
            Err(e) => {
                eprintln!("no se pudo sembrar: {e}");
                std::process::exit(1)
            }
        }
        return;
    }

    if std::env::var_os("SLINT_BACKEND").is_none()
        && let Err(e) = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("software".into()).select()
    {
        eprintln!("no se pudo iniciar la ventana: {e}");
        std::process::exit(1)
    }

    // Abrir el núcleo es leer unos JSON locales: se espera aquí para pintar
    // el primer fotograma ya con idioma y ajustes.
    let core = rt.block_on(Core::open(Options { data_dir: args.data_dir, ..Default::default() }));
    let ui = MainWindow::new().expect("ventana");
    let (core, settings) = match core {
        Ok(core) => {
            if let Err(e) = rt.block_on(core.reconcile_sessions()) {
                tracing::warn!(error = %e, "no se pudieron revisar las partidas");
            }
            let s = rt.block_on(core.settings());
            (Some(core), s)
        }
        Err(e) => {
            tracing::error!(error = %e, "no se pudo abrir el núcleo");
            (None, Default::default())
        }
    };
    let lang = i18n::resolve(settings.language.as_deref(), i18n::system_locale().as_deref());
    let backend = core.map(|core| Backend::new(core, rt.handle().clone()));
    let ui_prefs = backend.as_ref().map(|b| prefs::load(b.core.paths().root())).unwrap_or_default();

    let actions: Box<dyn shard_ui::controller::Actions> = match &backend {
        Some(b) => Box::new(CoreActions(b.clone())),
        None => Box::new(NoCore),
    };
    let now = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let ctl = Controller::new(&ui, lang, Box::new(now), actions);
    ctl.set_prefs(ui_prefs.clone());
    backend::install(ctl.clone());
    // Cuentas y skins (sin núcleo no hay cuentas: listas vacías).
    let (acc_actions, skin_actions): (Box<dyn shard_ui::accounts::AccountActions>, Box<dyn shard_ui::skins::SkinActions>) = match &backend {
        Some(b) => (Box::new(CoreAccounts::new(b.clone(), lang)), Box::new(CoreSkins { backend: b.clone(), lang: lang.to_owned() })),
        None => (Box::new(NoCore), Box::new(NoCore)),
    };
    let clock = Instant::now();
    let accounts = Accounts::new(&ui, ctl.clone(), acc_actions, Box::new(move || clock.elapsed().as_millis() as u64));
    let _skins = Skins::new(&ui, ctl.clone(), accounts.clone(), skin_actions);
    // Fase 3: detalle de instancia, pestañas y diálogos de instancias.
    let detail_actions: Box<dyn shard_ui::instance::InstanceActions> = match &backend {
        Some(b) => Box::new(shard_ui::instance::CoreInstance(b.clone())),
        None => Box::new(NoCore),
    };
    let detail = shard_ui::instance::InstanceDetail::new(&ui, ctl.clone(), detail_actions);
    // F3 contenido
    let content_actions: Box<dyn shard_ui::inst_content::ContentActions> = match &backend {
        Some(b) => Box::new(shard_ui::inst_content::CoreContent(b.clone())),
        None => Box::new(NoCore),
    };
    let _content = shard_ui::inst_content::InstContent::new(&ui, &detail, content_actions);
    // F3 registros y ajustes: la consola recibe las líneas de `pump_events`.
    let _console = shard_ui::console::Console::new(&ui, &detail);
    let inst_settings_actions: Box<dyn shard_ui::inst_settings::InstSettingsActions> = match &backend {
        Some(b) => Box::new(shard_ui::inst_settings::CoreInstSettings(b.clone())),
        None => Box::new(NoCore),
    };
    let _inst_settings = shard_ui::inst_settings::InstSettings::new(&ui, &detail, inst_settings_actions);
    // F3 copias
    let backup_actions: Box<dyn shard_ui::backups::BackupActions> = match &backend {
        Some(b) => Box::new(shard_ui::backups::CoreBackups::new(b.clone())),
        None => Box::new(NoCore),
    };
    let _backups = shard_ui::backups::Backups::new(&ui, detail.clone(), backup_actions);
    // F3 mundos y capturas
    let world_actions: Box<dyn shard_ui::worlds::WorldActions> = match &backend {
        Some(b) => Box::new(shard_ui::worlds::CoreWorlds(b.clone())),
        None => Box::new(NoCore),
    };
    let _worlds = shard_ui::worlds::Worlds::new(&ui, &detail, world_actions, Box::new(now));
    let shot_actions: Box<dyn shard_ui::screenshots::ScreenshotActions> = match &backend {
        Some(b) => Box::new(shard_ui::screenshots::CoreScreenshots(b.clone())),
        None => Box::new(NoCore),
    };
    let utc_offset = |t: i64| {
        chrono::DateTime::from_timestamp(t, 0).map_or(0, |d| i64::from(d.with_timezone(&chrono::Local).offset().local_minus_utc()))
    };
    let _screenshots = shard_ui::screenshots::Screenshots::new(&ui, &detail, shot_actions, Box::new(utc_offset));
    // F3 crear instancia, cambiar versión y papelera.
    let (create_actions, change_actions, trash_actions): (
        Box<dyn shard_ui::create_instance::CreateActions>,
        Box<dyn shard_ui::change_version::ChangeVersionActions>,
        Box<dyn shard_ui::trash::TrashActions>,
    ) = match &backend {
        Some(b) => (
            Box::new(shard_ui::create_instance::CoreCreate(b.clone())),
            Box::new(shard_ui::change_version::CoreChangeVersion(b.clone())),
            Box::new(shard_ui::trash::CoreTrash(b.clone())),
        ),
        None => (Box::new(NoCore), Box::new(NoCore), Box::new(NoCore)),
    };
    let create = shard_ui::create_instance::CreateInstance::new(&ui, ctl.clone(), create_actions);
    let trash = shard_ui::trash::Trash::new(&ui, ctl.clone(), trash_actions, Box::new(now));
    shard_ui::create_instance::dialog_hooks(&ctl, &create, &trash);
    let _change = shard_ui::change_version::ChangeVersion::new(&ui, detail.clone(), change_actions);
    // (fin de la fase 3)
    // Clave de CurseForge (Contenido y Ajustes).
    let key_actions: Box<dyn shard_ui::curseforge::KeyActions> = match &backend {
        Some(b) => Box::new(CoreKeys { backend: b.clone(), lang: lang.to_owned() }),
        None => Box::new(NoKeys),
    };
    let keys = Keys::new(&ui, ctl.clone(), key_actions);
    // Contenido (Modrinth y CurseForge).
    let content_actions: Box<dyn shard_ui::content::ContentActions> = match &backend {
        Some(b) => Box::new(CoreContent { backend: b.clone(), lang: lang.to_owned() }),
        None => Box::new(NoCore),
    };
    let content = Content::new(&ui, ctl.clone(), content_actions);
    {
        let weak = std::rc::Rc::downgrade(&content);
        keys.on_change(move |has| {
            if let Some(c) = weak.upgrade() {
                c.key_state(has);
            }
        });
        if let Some(has) = keys.has_key() {
            content.key_state(has);
        }
        let weak = std::rc::Rc::downgrade(&content);
        ctl.on_refresh(move |_| {
            if let Some(c) = weak.upgrade() {
                c.refresh();
            }
        });
    }
    // Servidores (la lista y los estados los lleva el controlador).
    let server_actions: Box<dyn shard_ui::servers::ServerActions> = match &backend {
        Some(b) => Box::new(CoreServers { backend: b.clone(), lang: lang.to_owned() }),
        None => Box::new(NoCore),
    };
    let servers = Servers::new(&ui, ctl.clone(), accounts.clone(), server_actions);
    {
        let weak = std::rc::Rc::downgrade(&servers);
        ctl.on_refresh(move |_| {
            if let Some(s) = weak.upgrade() {
                s.refresh();
            }
        });
    }
    let shell = Shell::new(&ui, settings.close_behavior, backend.clone());
    // Ajustes (sin núcleo, la página muestra el error de carga).
    let total_mb = {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        sys.total_memory() / (1024 * 1024)
    };
    let data_dir = backend.as_ref().map(|b| b.core.paths().root().display().to_string()).unwrap_or_default();
    let settings_actions: Box<dyn shard_ui::settings::SettingsActions> = match &backend {
        Some(b) => Box::new(CoreSettings::new(b.clone(), lang)),
        None => Box::new(NoCore),
    };
    let _doctor = backend.as_ref().map(|b| {
        shard_ui::doctor::Doctor::new(&ui, ctl.clone(), Box::new(shard_ui::doctor::CoreDoctor { backend: b.clone(), lang: lang.to_owned() }))
    });
    let settings_ctl = Settings::new(&ui, ctl.clone(), settings_actions, total_mb, &data_dir, env!("CARGO_PKG_VERSION"));
    {
        let shell = shell.clone();
        settings_ctl.on_close_changed(move |c| shell.set_close_behavior(c));
    }
    {
        let shell = shell.clone();
        ctl.on_refresh(move |m| shell.set_tray_last(m.all.first().map(|r| r.name.as_str()).unwrap_or("")));
    }
    match &backend {
        Some(b) => {
            b.load();
            b.pump_events();
            b.schedule_update_checks();
            if ui_prefs.accent == prefs::AccentMode::Auto
                && let Some(path) = ui_prefs.uses_image()
            {
                b.image_accent(path.to_owned());
            }
        }
        None => ctl.set_failed(i18n::text(lang, "errGeneric"), "lcore: no se pudo abrir la carpeta de datos".into()),
    }

    // Primer arranque: se decide antes de pintar para no mostrar Inicio un
    // instante (main.dart).
    let first_run = match &backend {
        Some(b) if ui_prefs.first_run_done != Some(true) => {
            let has_instances = rt.block_on(b.core.list_instances()).is_ok_and(|l| !l.is_empty());
            let has_accounts = rt.block_on(b.core.accounts()).is_ok_and(|a| !a.accounts.is_empty());
            shard_ui::first_run::needs_first_run(&ui_prefs, has_instances, has_accounts)
        }
        _ => false,
    };
    let _first_run = backend.as_ref().map(|b| {
        let fr = shard_ui::first_run::FirstRun::new(
            &ui,
            ctl.clone(),
            Box::new(shard_ui::first_run::CoreFirstRun { backend: b.clone(), lang: lang.to_owned() }),
            settings.language.clone(),
        );
        if first_run {
            fr.show(0);
        }
        fr
    });

    if std::env::var_os("SHARD_TIMING").is_some() {
        stamp_first_frame(&ui, started);
    }
    // Con el asistente pendiente no se arranca oculta.
    // Oculta solo con bandeja (como Flutter): si no, no habría forma de volver.
    if args.background && !first_run && shard_ui::window::tray_host_available() {
        shell.to_background();
    } else {
        ui.show().expect("mostrar la ventana");
    }
    slint::run_event_loop_until_quit().expect("bucle de eventos");
    drop(shell);
}

/// Sin núcleo (carpeta de datos ilegible): la UI muestra el error y los
/// botones no hacen nada.
struct NoCore;

impl shard_ui::controller::Actions for NoCore {
    fn open_instance(&self, _: &str) {}
    fn play(&self, _: &str) {}
    fn new_instance(&self) {}
    fn show_trash(&self) {}
}

impl shard_ui::accounts::AccountActions for NoCore {
    fn load(&self) {
        shard_ui::accounts::post(|a| a.set_accounts(Ok(Vec::new())));
    }
    fn set_active(&self, _: &str) {}
    fn remove(&self, _: &str, _: &str) {}
    fn add_offline(&self, _: &str, _: u64) {}
    fn begin_login(&self, _: u64) {}
    fn cancel_login(&self) {}
    fn open_url(&self, _: &str) {}
    fn load_face(&self, _: &str) {}
}

impl shard_ui::instance::InstanceActions for NoCore {
    fn load(&self, _: &str) {}
    fn open_folder(&self, _: &std::path::Path) {}
    fn delete(&self, _: &str, _: &str) {}
}

// F3 contenido
impl shard_ui::inst_content::ContentActions for NoCore {
    fn list(&self, _: &str) {}
    fn icons(&self, _: &str) {}
    fn icon_image(&self, _: &str) {}
    fn check_updates(&self, _: &str) {}
    fn set_enabled(&self, _: &str, _: &str, _: bool) {}
    fn remove(&self, _: &str, _: &str, _: &str) {}
    fn update(&self, _: &str, _: &str, _: &str) {}
    fn update_all(&self, _: &str, _: &[String]) {}
    fn identify(&self, _: &str) {}
    fn browse(&self, _: &str, _: &str) {}
}

impl shard_ui::inst_settings::InstSettingsActions for NoCore {
    fn load_globals(&self) {}
    fn save(&self, _: &str, _: &shard_ui::inst_settings::Values) {}
}

// F3 copias
impl shard_ui::backups::BackupActions for NoCore {
    fn list(&self, instance: &str) {
        let id = instance.to_owned();
        shard_ui::backups::post(move |b| b.listed(&id, Ok(Vec::new())));
    }
    fn create(&self, _: &str, _: Option<String>, _: bool) {}
    fn set_pinned(&self, _: &str, _: &str, _: bool) {}
    fn set_label(&self, _: &str, _: &str, _: Option<String>) {}
    fn delete(&self, _: &str, _: &str) {}
    fn restore(&self, _: &str, _: &str, _: bool, _: Option<String>) {}
    fn save_settings(&self, _: &str, _: lcore::instance::BackupSettings) {}
}

impl shard_ui::worlds::WorldActions for NoCore {
    fn list(&self, _: &str) {}
    fn rename(&self, _: &str, _: &str, _: &str) {}
    fn update(&self, _: &str, _: &str, _: lcore::world::WorldPatch) {}
    fn delete(&self, _: &str, _: &str, _: &str) {}
    fn list_datapacks(&self, _: &str, _: &str) {}
    fn set_datapack(&self, _: &str, _: &str, _: &str, _: bool) {}
    fn remove_datapack(&self, _: &str, _: &str, _: &str) {}
    fn add_datapacks(&self, _: &str, _: &str) {}
    fn open_folder(&self, _: &std::path::Path) {}
}

impl shard_ui::screenshots::ScreenshotActions for NoCore {
    fn list(&self, _: &str) {}
    fn load_image(&self, _: &std::path::Path, _: u32) {}
    fn delete(&self, _: &str, _: &str) {}
    fn open_path(&self, _: &std::path::Path) {}
}

// F3 crear instancia, cambiar versión y papelera: sin núcleo no hay
// versiones ni papelera (listas vacías).
impl shard_ui::create_instance::CreateActions for NoCore {
    fn list_versions(&self, snapshots: bool) {
        shard_ui::create_instance::post(move |c| c.versions_loaded(snapshots, Ok(Vec::new())));
    }
    fn loader_versions(&self, _: &str, _: &str) {}
    fn create(&self, _: &shard_ui::create_instance::NewInstance, _: u64) {}
}

impl shard_ui::change_version::ChangeVersionActions for NoCore {
    fn list_versions(&self, snapshots: bool) {
        shard_ui::change_version::post(move |c| c.versions_loaded(snapshots, Ok(Vec::new())));
    }
    fn loader_versions(&self, _: &str, _: &str) {}
    fn change(&self, _: &shard_ui::change_version::Change, _: u64) {}
}

impl shard_ui::trash::TrashActions for NoCore {
    fn list(&self) {
        shard_ui::trash::post(|t| t.listed(Ok(Vec::new())));
    }
    fn restore(&self, _: &str) {}
    fn purge(&self, _: &[String]) {}
}

impl shard_ui::settings::SettingsActions for NoCore {
    fn load(&self, req: u64) {
        shard_ui::settings::post(move |s| {
            let lang = "es";
            s.loaded(req, Err(shard_ui::errors::Failure {
                kind: "io".into(),
                message: shard_ui::i18n::text(lang, "errGeneric"),
                detail: "lcore: no se pudo abrir la carpeta de datos".into(),
            }))
        });
    }
    fn save(&self, _: &shard_ui::settings::Values, _: u64) {}
    fn list_java(&self) {}
    fn install_java(&self, _: u32) {}
    fn load_autostart(&self) {}
    fn set_autostart(&self, _: bool) {}
    fn save_prefs(&self, _: &prefs::UiPrefs) {}
    fn pick_background(&self) {}
    fn image_accent(&self, _: &std::path::Path) {}
    fn copy_text(&self, _: &str) -> bool {
        false
    }
    fn check_update(&self) {}
}

impl shard_ui::skins::SkinActions for NoCore {
    fn load_profile(&self, _: &str, _: u64) {}
    fn load_texture(&self, _: &str) {}
    fn pick_file(&self, _: &str) {}
    fn load_local(&self, _: &str) {}
    fn apply(&self, _: &str, _: &shard_ui::skins::Apply) {}
    fn reset(&self, _: &str) {}
    fn cape(&self, _: &str, _: Option<&str>) {}
}

impl shard_ui::content::ContentActions for NoCore {
    fn search(&self, _: &shard_ui::content::Filters, _: u32, generation: u64) {
        let page = shard_ui::content::Page { items: Vec::new(), total: 0 };
        shard_ui::content::post(move |c| c.page(generation, Ok(page)));
    }
    fn versions(&self, _: &shard_ui::content::VersionsQuery, seq: u64) {
        shard_ui::content::post(move |c| c.versions_loaded(seq, Ok(Vec::new())));
    }
    fn install(&self, _: &shard_ui::content::InstallRequest) {}
    fn load_installed(&self, _: &str) {}
    fn load_worlds(&self, _: &str) {}
    fn load_releases(&self) {}
    fn load_icon(&self, _: &str) {}
    fn pick_modpack(&self, _: &str) {}
    fn install_modpack(&self, _: &str, _: Option<&str>, _: u64) {}
}

impl shard_ui::servers::ServerActions for NoCore {
    fn save(&self, _: &shard_ui::servers::Draft) {}
    fn remove(&self, _: &str, _: &str) {}
    fn move_to(&self, _: &str, _: usize) {}
    fn import(&self, _: &str, _: &str) {}
    fn reload(&self) {}
}

/// `SHARD_FIRST_FRAME unix_ms=…` tras pintar el primer fotograma (el
/// renderer por software no tiene notificador: se marca en la primera vuelta
/// del bucle, cuando ese fotograma ya se pidió).
fn stamp_first_frame(ui: &MainWindow, started: Instant) {
    let print = move |note: &str| {
        let unix_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        println!("SHARD_FIRST_FRAME unix_ms={unix_ms} desde_main_ms={}{note}", started.elapsed().as_millis());
    };
    let mut done = false;
    let r = ui.window().set_rendering_notifier(move |state, _| {
        if !done && matches!(state, slint::RenderingState::AfterRendering) {
            done = true;
            print("");
        }
    });
    if r.is_err() {
        slint::Timer::single_shot(std::time::Duration::ZERO, move || print(" (aprox, software)"));
    }
}
