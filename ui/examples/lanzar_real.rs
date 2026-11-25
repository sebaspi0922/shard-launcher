//! Prueba a mano del flujo de lanzamiento con el núcleo real: abre Inicio
//! con una carpeta de datos de verdad, pulsa «Jugar» con un clic sobre la UI
//! y guarda una captura en cada fase (Preparando, Descargando x/y, Iniciando,
//! En juego). Con el juego abierto lo cierra (como si se cerrara mal) y
//! captura el aviso de cierre inesperado.
//!
//!   cargo run -p shard-ui --example lanzar_real -- DATOS SALIDA
//!
//! DATOS necesita una instancia y una cuenta (`shard account add-offline`,
//! `shard instance create`). El juego abre su ventana en `DISPLAY`. Usa el
//! backend de pruebas de Slint (sin ventana) porque en un weston sin pantalla
//! Xwayland se cae con los clics sintéticos de xdotool.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use i_slint_backend_testing::ElementRoot;
use lcore::{Core, Options};
use shard_ui::backend::{self, Backend, CoreActions};
use shard_ui::controller::Controller;
use shard_ui::{HomeState, LaunchPhase, MainWindow, Notices, Route};
use slint::ComponentHandle;
use slint::platform::PointerEventButton;

fn snapshot(ui: &MainWindow, path: &std::path::Path) {
    let snap = ui.window().take_snapshot().expect("captura");
    let file = std::fs::File::create(path).expect("png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), snap.width(), snap.height());
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(snap.as_bytes()).unwrap();
    println!("captura {}", path.display());
}

fn main() {
    let mut args = std::env::args().skip(1);
    let data = PathBuf::from(args.next().expect("DATOS"));
    let out = PathBuf::from(args.next().expect("SALIDA"));
    std::fs::create_dir_all(&out).unwrap();
    tracing_subscriber::fmt().with_env_filter("info").init();
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions { mock_time: false, threading: true, renderer_name: Some("software".into()) },
    )))
    .unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let core = rt.block_on(Core::open(Options { data_dir: Some(data), ..Default::default() })).expect("núcleo");
    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    let b = Backend::new(core.clone(), rt.handle().clone());
    let now = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let ctl = Controller::new(&ui, "es", Box::new(now), Box::new(CoreActions(b.clone())));
    backend::install(ctl.clone());
    b.load();
    b.pump_events();
    ui.set_route(Route::Home);
    ui.show().unwrap();

    let started = Instant::now();
    let clicked = Cell::new(false);
    let last = RefCell::new(String::new());
    let shots = Cell::new(0);
    let running_since: Cell<Option<Instant>> = Cell::new(None);
    let killed = Cell::new(false);
    let done = Cell::new(false);
    let weak = ui.as_weak();
    let timer = slint::Timer::default();
    let ctl2 = Rc::clone(&ctl);
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(200), move || {
        let Some(ui) = weak.upgrade() else { return };
        let hs = ui.global::<HomeState>();
        if !clicked.get() && hs.get_has_hero() {
            clicked.set(true);
            let btn = ui
                .root_element()
                .query_descendants()
                .match_predicate(|e| e.accessible_id().as_deref() == Some("hero-play-btn"))
                .find_first()
                .expect("botón Jugar");
            println!("clic en «{}»", btn.accessible_label().unwrap_or_default());
            btn.mock_single_click(PointerEventButton::Left);
        }
        let hero = hs.get_hero();
        let phase = format!("{:?} {}/{}", hero.launch.phase, hero.launch.done, hero.launch.total);
        let key = format!("{:?}", hero.launch.phase);
        if *last.borrow() != key {
            println!("{:>6.1} s  {phase}", started.elapsed().as_secs_f32());
            let n = shots.get() + 1;
            shots.set(n);
            snapshot(&ui, &out.join(format!("{n:02}-{key}.png").to_lowercase()));
            *last.borrow_mut() = key;
        }
        if hero.launch.phase == LaunchPhase::Running && running_since.get().is_none() {
            running_since.set(Some(Instant::now()));
        }
        // Con el juego en marcha un rato, se cierra a la fuerza: el núcleo
        // avisa GameExited con código ≠ 0 y sale el aviso de cierre.
        if let Some(t) = running_since.get()
            && t.elapsed() > Duration::from_secs(8)
            && !killed.get()
        {
            killed.set(true);
            println!("cerrando el juego de {:?}", ctl2.model.borrow().all.first().map(|r| r.id.clone()));
            let core = core.clone();
            rt.spawn(async move {
                for g in core.running_games().await {
                    if let Some(pid) = g.pid {
                        let _ = std::process::Command::new("kill").arg(pid.to_string()).status();
                    }
                }
            });
        }
        if killed.get() && ui.global::<Notices>().get_crash_shown() && !done.get() {
            done.set(true);
            snapshot(&ui, &out.join("99-aviso-cierre.png"));
            println!("aviso: {}", ui.global::<Notices>().get_crash_line());
            slint::quit_event_loop().unwrap();
        }
        if started.elapsed() > Duration::from_secs(900) {
            println!("tiempo agotado");
            slint::quit_event_loop().unwrap();
        }
    });
    slint::run_event_loop().unwrap();
    // Sin esperar a las tareas del núcleo que siguen escuchando eventos.
    std::process::exit(0);
}
