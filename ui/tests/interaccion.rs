//! Pruebas de interacción: clics y teclado de verdad (eventos de ventana)
//! sobre la UI con los datos de ejemplo, y aserciones sobre el estado de
//! Rust y de la UI, no sobre píxeles.
//!
//! Un solo `#[test]` porque el backend de pruebas de Slint se inicia una vez
//! por proceso; cada caso usa una ventana nueva.

use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use lcore::settings::CloseBehavior;
use shard_ui::controller::{Actions, Controller};
use shard_ui::instances::Phase;
use shard_ui::window::Shell;
use shard_ui::{InstancesState, LoadState, MainWindow, Route, fixtures};
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
}
