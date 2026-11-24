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
use shard_ui::skins::Skins;
use shard_ui::controller::Controller;
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
    let backend = core.map(|core| Backend { core, rt: rt.handle().clone() });
    let ui_prefs = backend.as_ref().map(|b| prefs::load(b.core.paths().root())).unwrap_or_default();
    ui.set_animate_background(!ui_prefs.performance);
    ui.global::<shard_ui::Theme>().set_motion(!ui_prefs.performance);

    let actions: Box<dyn shard_ui::controller::Actions> = match &backend {
        Some(b) => Box::new(CoreActions(b.clone())),
        None => Box::new(NoCore),
    };
    let now = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let ctl = Controller::new(&ui, lang, Box::new(now), actions);
    backend::install(ctl.clone());
    // Cuentas y skins (sin núcleo no hay cuentas: listas vacías).
    let (acc_actions, skin_actions): (Box<dyn shard_ui::accounts::AccountActions>, Box<dyn shard_ui::skins::SkinActions>) = match &backend {
        Some(b) => (Box::new(CoreAccounts::new(b.clone(), lang)), Box::new(CoreSkins { backend: b.clone(), lang: lang.to_owned() })),
        None => (Box::new(NoCore), Box::new(NoCore)),
    };
    let clock = Instant::now();
    let accounts = Accounts::new(&ui, ctl.clone(), acc_actions, Box::new(move || clock.elapsed().as_millis() as u64));
    let _skins = Skins::new(&ui, ctl.clone(), accounts.clone(), skin_actions);
    let shell = Shell::new(&ui, settings.close_behavior, backend.clone());
    {
        let shell = shell.clone();
        ctl.on_refresh(move |m| shell.set_tray_last(m.all.first().map(|r| r.name.as_str()).unwrap_or("")));
    }
    match &backend {
        Some(b) => {
            b.load();
            b.pump_events();
        }
        None => ctl.set_failed(i18n::text(lang, "errGeneric"), "lcore: no se pudo abrir la carpeta de datos".into()),
    }

    if std::env::var_os("SHARD_TIMING").is_some() {
        stamp_first_frame(&ui, started);
    }
    if args.background {
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
    fn add_offline(&self, _: &str) {}
    fn add_shard(&self, _: &str, _: &str) {}
    fn begin_login(&self, _: u64) {}
    fn cancel_login(&self) {}
    fn open_url(&self, _: &str) {}
    fn load_face(&self, _: &str) {}
}

impl shard_ui::skins::SkinActions for NoCore {
    fn load_profile(&self, _: &str) {}
    fn load_texture(&self, _: &str) {}
    fn pick_file(&self, _: &str) {}
    fn load_local(&self, _: &str) {}
    fn apply(&self, _: &str, _: &shard_ui::skins::Apply) {}
    fn reset(&self, _: &str) {}
    fn cape(&self, _: &str, _: Option<&str>) {}
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
