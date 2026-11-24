//! Pruebas de interacción: clics y teclado de verdad (eventos de ventana)
//! sobre la UI con los datos de ejemplo, y aserciones sobre el estado de
//! Rust y de la UI, no sobre píxeles.
//!
//! Un solo `#[test]` porque el backend de pruebas de Slint se inicia una vez
//! por proceso; cada caso usa una ventana nueva.

use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use lcore::events::{Event, LogStream, ProgressUnit};
use lcore::settings::CloseBehavior;
use shard_ui::controller::{Actions, Controller};
use shard_ui::instances::Phase;
use shard_ui::launch::Failure;
use shard_ui::prefs::UiPrefs;
use shard_ui::theme::Style;
use shard_ui::window::Shell;
use shard_ui::{
    Backdrop, BackdropKind, HomeState, InstancesState, LaunchPhase, LoadState, MainWindow, Notices, Palette, Route, Theme, Tips, UpdatePhase,
    fixtures,
};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, Model};

/// Acciones registradas en lugar de ir al núcleo.
#[derive(Clone, Default)]
struct Log(Rc<RefCell<Vec<String>>>);

impl Actions for Log {
    fn open_instance(&self, id: &str) {
        self.0.borrow_mut().push(format!("open {id}"));
    }
    fn play(&self, id: &str) {
        self.0.borrow_mut().push(format!("play {id}"));
    }
    fn new_instance(&self) {
        self.0.borrow_mut().push("new".into());
    }
    fn show_trash(&self) {
        self.0.borrow_mut().push("trash".into());
    }
    fn join_server(&self, id: &str, address: &str, account: Option<&str>) {
        self.0.borrow_mut().push(format!("join {id} {address} {account:?}"));
    }
    fn accounts(&self) {
        self.0.borrow_mut().push("accounts".into());
    }
    fn diagnose(&self, id: &str) {
        self.0.borrow_mut().push(format!("diagnose {id}"));
    }
    fn show_logs(&self, id: &str) {
        self.0.borrow_mut().push(format!("logs {id}"));
    }
    fn install_update(&self) {
        self.0.borrow_mut().push("install".into());
    }
    fn restart_update(&self, path: &std::path::Path) {
        self.0.borrow_mut().push(format!("restart {}", path.display()));
    }
    fn open_url(&self, url: &str) {
        self.0.borrow_mut().push(format!("url {url}"));
    }
}

struct App {
    ui: MainWindow,
    ctl: Rc<Controller>,
    log: Log,
}

fn app(lang: &str) -> App {
    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    ui.set_animate_background(false);
    let log = Log::default();
    let ctl = Controller::new(&ui, lang, Box::new(|| fixtures::NOW), Box::new(log.clone()));
    ctl.set_instances(fixtures::sample_instances());
    // Estas pruebas son de Instancias (la app arranca en Inicio).
    ui.set_route(Route::Instances);
    ui.show().unwrap();
    settle();
    App { ui, ctl, log }
}

fn settle() {
    i_slint_backend_testing::testing_backend::mock_elapsed_time(500);
}

fn by_id(ui: &MainWindow, id: &str) -> Vec<ElementHandle> {
    let id = id.to_owned();
    ui.root_element()
        .query_descendants()
        .match_predicate(move |e| e.accessible_id().as_deref() == Some(id.as_str()))
        .find_all()
}

fn by_label(ui: &MainWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(ui, label)
        .next()
        .unwrap_or_else(|| panic!("no hay elemento «{label}»"))
}

fn click(e: &ElementHandle) {
    e.mock_single_click(PointerEventButton::Left);
    settle();
}

fn type_text(ui: &MainWindow, text: &str) {
    for ch in text.chars() {
        let t: slint::SharedString = ch.to_string().into();
        ui.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
        ui.window().dispatch_event(WindowEvent::KeyReleased { text: t });
    }
    settle();
}

fn visible_names(ui: &MainWindow) -> Vec<String> {
    let items = ui.global::<InstancesState>().get_items();
    (0..items.row_count()).map(|i| items.row_data(i).unwrap().name.to_string()).collect()
}

fn buscar_filtra_y_borrar_restaura() {
    let a = app("es");
    click(&by_id(&a.ui, "search")[0]);
    type_text(&a.ui, "sur");
    assert_eq!(a.ctl.model.borrow().query, "sur");
    assert_eq!(visible_names(&a.ui), ["Survival con amigos"]);
    assert_eq!(by_id(&a.ui, "card").len(), 1);
    // El contador sigue contando todas.
    assert_eq!(a.ui.global::<InstancesState>().get_total(), 5);

    // Acentos y varias palabras (nombre + loader).
    click(&by_label(&a.ui, "Limpiar"));
    type_text(&a.ui, "técnico neoforge");
    assert_eq!(visible_names(&a.ui), ["Skyblock técnico"]);

    click(&by_label(&a.ui, "Limpiar"));
    assert_eq!(a.ctl.model.borrow().query, "");
    assert_eq!(visible_names(&a.ui).len(), 5);

    type_text(&a.ui, "zzz");
    assert!(visible_names(&a.ui).is_empty());
    assert_eq!(by_id(&a.ui, "no-results").len(), 1);
}

fn cambiar_a_lista_y_jugar() {
    let a = app("es");
    assert_eq!(by_id(&a.ui, "card").len(), 5);
    click(&by_label(&a.ui, "Lista"));
    assert!(!a.ctl.model.borrow().grid);
    assert!(!a.ui.global::<InstancesState>().get_grid());
    assert_eq!(by_id(&a.ui, "card").len(), 0);
    assert_eq!(by_id(&a.ui, "row").len(), 5);

    // Primera fila = la jugada más reciente.
    click(&by_id(&a.ui, "row-play")[0]);
    assert_eq!(*a.log.0.borrow(), ["play survival-amigos"]);

    // Abrir la fila.
    click(&by_id(&a.ui, "row")[1]);
    assert_eq!(a.log.0.borrow().last().unwrap(), "open vanilla-puro");

    click(&by_label(&a.ui, "Cuadrícula"));
    assert!(a.ctl.model.borrow().grid);
    assert_eq!(by_id(&a.ui, "card").len(), 5);
}

fn botones_de_la_cabecera() {
    let a = app("es");
    click(&by_id(&a.ui, "new")[0]);
    click(&by_id(&a.ui, "trash")[0]);
    assert_eq!(*a.log.0.borrow(), ["new", "trash"]);
}

fn partida_en_marcha_en_tarjeta_y_titulo() {
    let a = app("es");
    a.ctl.set_phase("create-aeronautica", Phase::Running);
    settle();
    let act = a.ui.get_activity();
    assert!(act.present);
    assert_eq!(act.name, "Create: Aeronáutica");
    assert_eq!(by_id(&a.ui, "pill").len(), 1);
    // La tarjeta en marcha muestra «En juego» y no deja relanzar.
    // Orden «recientes»: Survival, Vanilla, Create…
    let play = &by_id(&a.ui, "card-play")[2];
    assert_eq!(play.accessible_label().as_deref(), Some("En juego"));
    assert_eq!(play.accessible_enabled(), Some(false));
    a.ctl.set_phase("create-aeronautica", Phase::Idle);
    settle();
    assert!(!a.ui.get_activity().present);
    assert!(by_id(&a.ui, "pill").is_empty());
}

fn estados_de_carga_error_y_vacio() {
    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    let ctl = Controller::new(&ui, "en", Box::new(|| fixtures::NOW), Box::new(Log::default()));
    ui.set_route(Route::Instances);
    ui.show().unwrap();
    settle();
    assert_eq!(ui.global::<InstancesState>().get_state(), LoadState::Loading);
    assert_eq!(by_id(&ui, "loading").len(), 1);

    ctl.set_failed("Disk error".into(), "io: denied".into());
    settle();
    assert_eq!(ui.global::<InstancesState>().get_state(), LoadState::Failed);
    assert_eq!(by_id(&ui, "error")[0].accessible_label().as_deref(), Some("Disk error"));

    ctl.set_instances(vec![]);
    settle();
    assert_eq!(by_id(&ui, "empty").len(), 1);
    // En inglés.
    assert_eq!(by_id(&ui, "txt-count")[0].accessible_label().as_deref(), Some("No instances"));
}

fn navegar_por_el_rail() {
    let a = app("en");
    click(&by_label(&a.ui, "Home"));
    assert_eq!(a.ui.get_route(), Route::Home);
    assert!(by_id(&a.ui, "search").is_empty());
    click(&by_label(&a.ui, "Instances"));
    assert_eq!(a.ui.get_route(), Route::Instances);
    assert_eq!(by_id(&a.ui, "card").len(), 5);
}

fn cerrar_pregunta_y_recuerda() {
    // SAFETY: un solo hilo de test.
    unsafe { std::env::set_var("SHARD_TRAY", "1") };
    let a = app("es");
    let shell = Shell::new(&a.ui, CloseBehavior::Ask, None);
    click(&by_label(&a.ui, "Cerrar"));
    assert!(a.ui.get_close_dialog_open());
    click(&by_label(&a.ui, "Cancelar"));
    assert!(!a.ui.get_close_dialog_open());
    assert!(a.ui.window().is_visible());

    // «Recordar» + «Dejar en segundo plano»: se oculta y la próxima vez ya
    // no pregunta.
    click(&by_label(&a.ui, "Cerrar"));
    click(&by_label(&a.ui, "Recordar mi elección"));
    click(&by_label(&a.ui, "Dejar en segundo plano"));
    assert!(!a.ui.get_close_dialog_open());
    assert!(!a.ui.window().is_visible());
    a.ui.show().unwrap();
    settle();
    click(&by_label(&a.ui, "Cerrar"));
    assert!(!a.ui.get_close_dialog_open());
    assert!(!a.ui.window().is_visible());
    drop(shell);
}

fn sin_bandeja_minimiza_en_vez_de_ocultar() {
    // SAFETY: un solo hilo de test.
    unsafe { std::env::set_var("SHARD_TRAY", "0") };
    let a = app("es");
    let shell = Shell::new(&a.ui, CloseBehavior::Tray, None);
    click(&by_label(&a.ui, "Cerrar"));
    assert!(a.ui.window().is_visible(), "sin bandeja la ventana no puede quedar oculta");
    assert!(a.ui.window().is_minimized());
    drop(shell);
}

/// Inicio con los datos de ejemplo.
fn home_app(lang: &str) -> App {
    let a = app(lang);
    a.ctl.set_account(Some(fixtures::ACTIVE_ACCOUNT));
    a.ctl.set_servers(fixtures::sample_servers());
    a.ctl.set_statuses(fixtures::sample_statuses());
    click(&by_label(&a.ui, if lang == "es" { "Inicio" } else { "Home" }));
    a
}

fn hero_id(ui: &MainWindow) -> String {
    ui.global::<HomeState>().get_hero().id.to_string()
}

fn hero_phase(ui: &MainWindow) -> LaunchPhase {
    ui.global::<HomeState>().get_hero().launch.phase
}

fn inicio_y_cambiar_instancia() {
    let a = home_app("es");
    assert_eq!(a.ui.get_route(), Route::Home);
    assert_eq!(by_id(&a.ui, "txt-greeting")[0].accessible_label().as_deref(), Some("Hola, NadiaK"));
    // La tarjeta grande es la última jugada; las recientes, las tres siguientes.
    assert_eq!(hero_id(&a.ui), "survival-amigos");
    let recent: Vec<String> = by_id(&a.ui, "recent").iter().map(|e| e.accessible_label().unwrap().to_string()).collect();
    assert_eq!(recent, ["Vanilla puro", "Create: Aeronáutica", "Skyblock técnico"]);
    click(&by_id(&a.ui, "hero-picker-btn")[0]);
    // Menú con todas, en orden «recientes».
    let items = by_id(&a.ui, "picker-item");
    assert_eq!(items.len(), 5);
    click(&items[3]);
    assert_eq!(hero_id(&a.ui), "skyblock-tecnico");
    let recent: Vec<String> = by_id(&a.ui, "recent").iter().map(|e| e.accessible_label().unwrap().to_string()).collect();
    assert_eq!(recent, ["Survival con amigos", "Vanilla puro", "Create: Aeronáutica"]);
    // «Ver todas» lleva a Instancias.
    click(&by_label(&a.ui, "Ver todas"));
    assert_eq!(a.ui.get_route(), Route::Instances);
}

fn jugar_con_progreso_hasta_en_juego() {
    let a = home_app("es");
    click(&by_id(&a.ui, "hero-play-btn")[0]);
    assert_eq!(*a.log.0.borrow(), ["play survival-amigos"]);
    assert_eq!(hero_phase(&a.ui), LaunchPhase::Preparing);
    // Segundo clic: ya se está lanzando, no se pide otra vez.
    click(&by_id(&a.ui, "hero-play-btn")[0]);
    assert_eq!(a.log.0.borrow().len(), 1);
    // Descargas del núcleo (Progress / TaskFinished).
    a.ctl.on_event(&Event::Progress { task_id: 7, stage: "assets".into(), done: 1834, total: 3920, unit: ProgressUnit::Files });
    settle();
    let hero = a.ui.global::<HomeState>().get_hero();
    assert_eq!((hero.launch.phase, hero.launch.done, hero.launch.total), (LaunchPhase::Downloading, 1834, 3920));
    assert_eq!(by_id(&a.ui, "hero-play-btn")[0].accessible_label().as_deref(), Some("Descargando 1834/3920"));
    assert_eq!(by_id(&a.ui, "pill").len(), 1);
    a.ctl.on_event(&Event::TaskFinished { task_id: 7, ok: true, message: None });
    assert_eq!(hero_phase(&a.ui), LaunchPhase::Preparing);
    // Arranca el proceso y el juego dice que ya abrió su ventana.
    a.ctl.launch_done("survival-amigos", Ok(3));
    assert_eq!(hero_phase(&a.ui), LaunchPhase::Starting);
    a.ctl.on_event(&Event::GameLog { game_id: 3, stream: LogStream::Stdout, line: "[Sound Library Loader/INFO]: Sound engine started".into() });
    settle();
    assert_eq!(hero_phase(&a.ui), LaunchPhase::Running);
    let running = a.ui.global::<HomeState>().get_running();
    assert_eq!(running.row_count(), 1);
    assert_eq!(running.row_data(0).unwrap().id, "survival-amigos");
    // Cierre normal: vuelve a «Jugar» y no hay aviso.
    a.ctl.on_event(&Event::GameExited { game_id: 3, instance_id: "survival-amigos".into(), code: Some(0), seconds_played: 60 });
    settle();
    assert_eq!(hero_phase(&a.ui), LaunchPhase::Idle);
    assert!(!a.ui.global::<Notices>().get_crash_shown());
}

fn iniciando_pasa_a_en_juego_a_los_20_s() {
    let a = home_app("es");
    a.ctl.play("vanilla-puro");
    a.ctl.launch_done("vanilla-puro", Ok(4));
    let phase = |a: &App| a.ctl.model.borrow().phase("vanilla-puro");
    assert_eq!(phase(&a), Phase::Starting);
    shard_ui::backend::install(a.ctl.clone());
    i_slint_backend_testing::testing_backend::mock_elapsed_time(19_000);
    slint::platform::update_timers_and_animations();
    assert_eq!(phase(&a), Phase::Starting);
    i_slint_backend_testing::testing_backend::mock_elapsed_time(1_500);
    slint::platform::update_timers_and_animations();
    assert_eq!(phase(&a), Phase::Running);
}

fn fallo_al_lanzar_y_cerrar_el_aviso() {
    let a = home_app("es");
    click(&by_id(&a.ui, "hero-play-btn")[0]);
    a.ctl.launch_done(
        "survival-amigos",
        Err(Failure::Launch { kind: "java_required".into(), message: "hace falta Java 21".into() }),
    );
    settle();
    let f = a.ui.global::<HomeState>().get_hero_failure();
    assert_eq!(f.title, "No se pudo iniciar");
    assert_eq!(f.detail, "Falta la versión de Java que necesita esta instancia.");
    assert_eq!(f.tech, "java_required: hace falta Java 21");
    assert_eq!(by_id(&a.ui, "hero-failure").len(), 1);
    assert_eq!(by_id(&a.ui, "hero-play-btn")[0].accessible_label().as_deref(), Some("Reintentar"));
    click(&inside(&by_id(&a.ui, "hero-failure")[0], "Cerrar"));
    assert!(by_id(&a.ui, "hero-failure").is_empty());
    assert_eq!(hero_phase(&a.ui), LaunchPhase::Idle);
}

fn aviso_de_cierre_inesperado() {
    let a = home_app("es");
    a.ctl.on_event(&Event::GameStarted { game_id: 9, instance_id: "create-aeronautica".into(), pid: None });
    a.ctl.on_event(&Event::GameExited { game_id: 9, instance_id: "create-aeronautica".into(), code: Some(1), seconds_played: 5 });
    settle();
    let n = a.ui.global::<Notices>();
    assert!(n.get_crash_shown());
    assert_eq!(n.get_crash_line(), "Create: Aeronáutica · código 1");
    click(&by_id(&a.ui, "crash-diagnose")[0]);
    assert!(!n.get_crash_shown());
    assert_eq!(a.log.0.borrow().last().unwrap(), "diagnose create-aeronautica");
    // Otra partida que se cierra mal: «Ver registros».
    a.ctl.on_event(&Event::GameCrashed { game_id: 10, instance_id: "vanilla-puro".into(), crash_report: None });
    settle();
    assert!(n.get_crash_shown());
    click(&by_id(&a.ui, "crash-logs")[0]);
    assert_eq!(a.log.0.borrow().last().unwrap(), "logs vanilla-puro");
}

fn aviso_de_version_nueva() {
    let a = home_app("es");
    a.ctl.update_checked(Some(fixtures::sample_update()), false);
    settle();
    let n = a.ui.global::<Notices>();
    assert!(n.get_update_shown());
    assert_eq!(n.get_update_status(), "Actualizador integrado y arreglos de la bandeja.");
    click(&by_id(&a.ui, "update-install")[0]);
    assert_eq!(a.log.0.borrow().last().unwrap(), "install");
    assert_eq!(n.get_update_phase(), UpdatePhase::Installing);
    a.ctl.update_installed(Ok("/tmp/Shard.AppImage".into()));
    settle();
    assert_eq!(n.get_update_phase(), UpdatePhase::Ready);
    click(&by_label(&a.ui, "Reiniciar"));
    assert_eq!(a.log.0.borrow().last().unwrap(), "restart /tmp/Shard.AppImage");
    click(&by_label(&a.ui, "Novedades"));
    assert_eq!(a.log.0.borrow().last().unwrap(), "url https://shardlauncher.com/novedades");
    // Cerrado no vuelve con la misma versión en una consulta automática.
    click(&inside(&by_id(&a.ui, "update-banner")[0], "Cerrar"));
    assert!(!n.get_update_shown());
    a.ctl.update_checked(Some(fixtures::sample_update()), false);
    assert!(!n.get_update_shown());
}

/// Elemento con esa etiqueta dentro de otro.
fn inside(parent: &ElementHandle, label: &str) -> ElementHandle {
    let want = label.to_owned();
    parent
        .query_descendants()
        .match_predicate(move |e| e.accessible_label().as_deref() == Some(want.as_str()))
        .find_first()
        .unwrap_or_else(|| panic!("no hay «{label}» dentro"))
}

fn entrar_a_un_servidor() {
    let a = home_app("en");
    let servers = a.ui.global::<HomeState>().get_servers();
    assert_eq!(servers.row_count(), 3);
    assert_eq!(servers.row_data(1).unwrap().status, "31.8K / 200K · 118 ms");
    assert_eq!(servers.row_data(0).unwrap().tooltip, "Join with Survival con amigos");
    click(&by_id(&a.ui, "server-join")[0]);
    assert_eq!(*a.log.0.borrow(), ["join survival-amigos mc.amigos.example None"]);
    // Lanzándose: el botón queda ocupado.
    assert!(a.ui.global::<HomeState>().get_servers().row_data(0).unwrap().busy);
    // Sin instancias no se puede entrar.
    a.ctl.set_instances(vec![]);
    settle();
    assert!(!a.ui.global::<HomeState>().get_servers().row_data(0).unwrap().can_join);
}

fn sin_cuenta_ni_instancias() {
    let a = home_app("en");
    a.ctl.set_account(None);
    a.ctl.set_instances(vec![]);
    settle();
    assert_eq!(by_id(&a.ui, "txt-greeting")[0].accessible_label().as_deref(), Some("Welcome"));
    assert_eq!(by_id(&a.ui, "no-instances").len(), 1);
    click(&by_label(&a.ui, "New instance"));
    assert_eq!(a.log.0.borrow().last().unwrap(), "new");
}

fn tooltip_del_rail_tras_la_espera() {
    let a = home_app("es");
    let item = by_label(&a.ui, "Contenido");
    let p = item.absolute_position();
    let s = item.size();
    let center = slint::LogicalPosition::new(p.x + s.width / 2.0, p.y + s.height / 2.0);
    a.ui.window().dispatch_event(WindowEvent::PointerMoved { position: center });
    i_slint_backend_testing::testing_backend::mock_elapsed_time(100);
    assert!(!a.ui.global::<Tips>().get_shown(), "Material espera 350 ms");
    i_slint_backend_testing::testing_backend::mock_elapsed_time(400);
    slint::platform::update_timers_and_animations();
    assert!(a.ui.global::<Tips>().get_shown());
    assert_eq!(a.ui.global::<Tips>().get_text(), "Contenido");
    a.ui.window().dispatch_event(WindowEvent::PointerMoved { position: slint::LogicalPosition::new(640.0, 400.0) });
    settle();
    assert!(!a.ui.global::<Tips>().get_shown());
}

fn estilos_y_escala_de_ui() {
    let a = home_app("es");
    a.ctl.set_prefs(UiPrefs { style: Style::Aurora, ..Default::default() });
    settle();
    let pal = a.ui.global::<Palette>();
    // Acento de «aurora»: tonos del jade (AccentSet.fromSeed).
    assert_ne!(pal.get_accent(), slint::Color::from_rgb_u8(255, 255, 255));
    assert_eq!(a.ui.global::<Backdrop>().get_kind(), BackdropKind::Image);
    assert_eq!(a.ui.global::<Theme>().get_radius_card(), 12.0);
    a.ctl.set_prefs(UiPrefs { style: Style::Graphite, ..Default::default() });
    assert_eq!(a.ui.global::<Backdrop>().get_kind(), BackdropKind::Atmosphere);
    assert_eq!(pal.get_accent(), slint::Color::from_rgb_u8(255, 255, 255));
    // Ventana al mínimo (960 × 600): la UI escala a 0,8 (1200 × 750 lógicos).
    resize_physical(&a.ui, 960.0, 600.0);
    assert_eq!(a.ui.global::<Theme>().get_ui_scale(), 0.8);
    assert!((a.ui.window().scale_factor() - 0.8).abs() < 1e-4);
    // El raíl mide el alto lógico de la ventana.
    assert_eq!(by_id(&a.ui, "rail")[0].size().height, 750.0);
    // Más grande que el diseño: hasta 1,5.
    resize_physical(&a.ui, 1920.0, 1200.0);
    assert_eq!(a.ui.global::<Theme>().get_ui_scale(), 1.5);
    resize_physical(&a.ui, 1280.0, 800.0);
    assert_eq!(a.ui.global::<Theme>().get_ui_scale(), 1.0);
    assert!((a.ui.window().scale_factor() - 1.0).abs() < 1e-4);
}

/// Cambio de tamaño como lo manda winit: tamaño físico convertido con la
/// escala actual de la ventana (el backend de pruebas usaría escala 1).
fn resize_physical(ui: &MainWindow, w: f32, h: f32) {
    let sf = ui.window().scale_factor();
    ui.window().dispatch_event(WindowEvent::Resized { size: slint::LogicalSize::new(w / sf, h / sf) });
    settle();
}

#[test]
fn interacciones() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    buscar_filtra_y_borrar_restaura();
    cambiar_a_lista_y_jugar();
    botones_de_la_cabecera();
    partida_en_marcha_en_tarjeta_y_titulo();
    estados_de_carga_error_y_vacio();
    navegar_por_el_rail();
    cerrar_pregunta_y_recuerda();
    sin_bandeja_minimiza_en_vez_de_ocultar();
    inicio_y_cambiar_instancia();
    jugar_con_progreso_hasta_en_juego();
    iniciando_pasa_a_en_juego_a_los_20_s();
    fallo_al_lanzar_y_cerrar_el_aviso();
    aviso_de_cierre_inesperado();
    aviso_de_version_nueva();
    entrar_a_un_servidor();
    sin_cuenta_ni_instancias();
    tooltip_del_rail_tras_la_espera();
    estilos_y_escala_de_ui();
}
