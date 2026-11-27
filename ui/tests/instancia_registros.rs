//! Pestaña Registros (fase 3): clics y teclado de verdad sobre la consola y
//! aserciones sobre el estado de la UI (`ConsoleState`, filas visibles,
//! aviso). Incluye la medida de rendimiento del PLAN: 5000 líneas cargadas,
//! desplazar y llegar líneas nuevas sin pasar del tiempo de un fotograma.
//!
//!   cargo test -p shard-ui --test instancia_registros -- --nocapture
//!
//! Un solo `#[test]`: el backend de pruebas de Slint se inicia una vez por
//! proceso.

use std::rc::Rc;
use std::time::{Duration, Instant};

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use shard_ui::console::{CAPACITY, Console, RawLine};
use shard_ui::controller::{Actions, Controller};
use shard_ui::fixtures::{self, Recorder};
use shard_ui::instance::InstanceDetail;
use shard_ui::{ConsoleState, InstanceState, InstanceTab, LogGroup, MainWindow, Route};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, Model};

struct NoActions;

impl Actions for NoActions {
    fn open_instance(&self, _: &str) {}
    fn play(&self, _: &str) {}
    fn new_instance(&self) {}
    fn show_trash(&self) {}
}

struct App {
    ui: MainWindow,
    detail: Rc<InstanceDetail>,
    console: Rc<Console>,
}

fn app() -> App {
    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    let ctl = Controller::new(&ui, "es", Box::new(|| fixtures::NOW), Box::new(NoActions));
    ui.set_animate_background(false);
    ctl.set_instances(fixtures::sample_instances());
    let rec = Recorder::default();
    let detail = InstanceDetail::new(&ui, ctl.clone(), Box::new(rec.clone()));
    let console = Console::new(&ui, &detail);
    ui.set_route(Route::Instances);
    ui.show().unwrap();
    settle();
    App { ui, detail, console }
}

fn settle() {
    i_slint_backend_testing::testing_backend::mock_elapsed_time(300);
}

fn by_id(ui: &MainWindow, id: &str) -> Vec<ElementHandle> {
    let id = id.to_owned();
    ui.root_element().query_descendants().match_predicate(move |e| e.accessible_id().as_deref() == Some(id.as_str())).find_all()
}

fn by_label(ui: &MainWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(ui, label).next().unwrap_or_else(|| panic!("no está «{label}»"))
}

fn has_label(ui: &MainWindow, label: &str) -> bool {
    ElementHandle::find_by_accessible_label(ui, label).next().is_some()
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

fn backspace(ui: &MainWindow, n: usize) {
    let t: slint::SharedString = slint::platform::Key::Backspace.into();
    for _ in 0..n {
        ui.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
        ui.window().dispatch_event(WindowEvent::KeyReleased { text: t.clone() });
    }
    settle();
}

fn rows(ui: &MainWindow) -> Vec<(i32, String, LogGroup)> {
    let m = ui.global::<ConsoleState>().get_rows();
    (0..m.row_count()).filter_map(|i| m.row_data(i)).map(|r| (r.number, r.text.to_string(), r.level)).collect()
}

fn counts(ui: &MainWindow) -> Vec<i32> {
    ui.global::<ConsoleState>().get_counts().iter().collect()
}

/// Textos de las líneas que la lista tiene creadas (las visibles).
fn shown_lines(ui: &MainWindow) -> Vec<String> {
    // La lista crea sus filas al pintar.
    ui.window().take_snapshot().expect("captura");
    ui.root_element()
        .query_descendants()
        .match_predicate(|e| e.accessible_id().as_deref() == Some("line"))
        .find_all()
        .into_iter()
        .filter_map(|e| e.accessible_label().map(|s| s.to_string()))
        .collect()
}

fn scroll(ui: &MainWindow, dy: f32) {
    let p = LogicalPosition::new(600.0, 560.0);
    ui.window().dispatch_event(WindowEvent::PointerMoved { position: p });
    ui.window().dispatch_event(WindowEvent::PointerScrolled { position: p, delta_x: 0.0, delta_y: dy });
    settle();
}

fn line(i: usize) -> RawLine {
    let level = ["INFO", "INFO", "WARN", "DEBUG", "ERROR"][i % 5];
    RawLine::new(format!("[18:{:02}:{:02}] [Render thread/{level}]: línea {i} con algo de texto para llenar la fila", i / 60 % 60, i % 60), false)
}

/// Mediana del tiempo de pintar un fotograma completo (renderer por
/// software, 1280 × 800) tras `before`.
fn frame_ms(ui: &MainWindow, mut before: impl FnMut(usize)) -> f64 {
    let mut times: Vec<f64> = (0..15)
        .map(|i| {
            before(i);
            let t = Instant::now();
            let snap = ui.window().take_snapshot().expect("captura");
            std::hint::black_box(snap);
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    times.sort_by(f64::total_cmp);
    times[times.len() / 2]
}

#[test]
fn registros() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions { mock_time: true, threading: false, renderer_name: Some("software".into()) },
    )))
    .expect("plataforma");
    let a = app();
    vacio_y_partida(&a);
    filtro_y_niveles(&a);
    seguir_copiar_y_limpiar(&a);
    otra_pestana_y_partida_nueva(&a);
    rendimiento(&a);
}

fn open_logs(a: &App) {
    a.detail.open("survival-amigos", Some(InstanceTab::Logs));
    a.detail.loaded("survival-amigos", Ok(fixtures::detail::sample_detail("survival-amigos").unwrap()));
    settle();
}

fn vacio_y_partida(a: &App) {
    open_logs(a);
    let st = a.ui.global::<ConsoleState>();
    assert_eq!(st.get_total(), 0);
    assert!(has_label(&a.ui, "Aún no hay registros"));
    // Las líneas de otra instancia no llegan a esta consola.
    a.console.started(7, "create-aeronautica");
    a.console.lines(7, vec![RawLine::new("[main/INFO]: otra", false)]);
    settle();
    assert_eq!(st.get_total(), 0);
    a.console.started(1, "survival-amigos");
    a.console.lines(1, fixtures::logs::sample_log());
    settle();
    assert_eq!(st.get_total(), 21);
    assert!(!has_label(&a.ui, "Aún no hay registros"));
    assert_eq!(counts(&a.ui), [4, 2, 14, 1]);
    assert_eq!(by_id(&a.ui, "txt-lines")[0].accessible_label().unwrap(), "21 líneas");
    let r = rows(&a.ui);
    assert_eq!(r.len(), 21);
    assert_eq!(r[5], (6, fixtures::logs::SAMPLE_LOG[5].0.to_owned(), LogGroup::Warn));
    assert_eq!(r[12].2, LogGroup::Error, "el stack trace hereda el error");
    assert_eq!(r[8].2, LogGroup::Debug);
    assert_eq!(shown_lines(&a.ui).len(), 21, "caben todas");
    // Un lote más se añade al final.
    a.console.lines(1, vec![RawLine::new("[Server thread/INFO]: Saving chunks", false)]);
    settle();
    assert_eq!(rows(&a.ui).last().unwrap().0, 22);
    assert_eq!(by_id(&a.ui, "txt-lines")[0].accessible_label().unwrap(), "22 líneas");
}

fn filtro_y_niveles(a: &App) {
    let st = a.ui.global::<ConsoleState>();
    click(&by_id(&a.ui, "filter")[0]);
    type_text(&a.ui, "CAPE");
    assert_eq!(rows(&a.ui).iter().map(|r| r.0).collect::<Vec<_>>(), [1, 2], "numeradas en lo que se ve");
    assert_eq!(st.get_total(), 22, "«N líneas» cuenta todas");
    // Ocultar los errores con su chip: no queda ninguna.
    click(&by_label(&a.ui, "Errores"));
    assert_eq!(st.get_hidden().row_data(0), Some(true));
    assert!(rows(&a.ui).is_empty());
    assert_eq!(by_id(&a.ui, "no-matches").len(), 1);
    assert_eq!(by_label(&a.ui, "Errores").accessible_checked(), Some(false));
    backspace(&a.ui, 4);
    assert_eq!(rows(&a.ui).len(), 22 - 4);
    click(&by_label(&a.ui, "Info"));
    click(&by_label(&a.ui, "Avisos"));
    assert_eq!(rows(&a.ui).iter().map(|r| r.2).collect::<Vec<_>>(), [LogGroup::Debug]);
    for chip in ["Errores", "Info", "Avisos"] {
        click(&by_label(&a.ui, chip));
    }
    assert_eq!(rows(&a.ui).len(), 22);
    assert_eq!(counts(&a.ui), [4, 2, 15, 1], "la cuenta no depende del filtro");
}

fn seguir_copiar_y_limpiar(a: &App) {
    let st = a.ui.global::<ConsoleState>();
    // Copiar: las líneas visibles y el aviso con cuántas son.
    click(&by_id(&a.ui, "filter")[0]);
    type_text(&a.ui, "load texture");
    assert_eq!(a.console.copy_text(), (fixtures::logs::SAMPLE_LOG[10].0.to_owned(), 1));
    click(&by_label(&a.ui, "Copiar"));
    assert_eq!(a.ui.get_toast_message(), "1 línea copiada");
    backspace(&a.ui, 12);
    click(&by_label(&a.ui, "Copiar"));
    assert_eq!(a.ui.get_toast_message(), "22 líneas copiadas");
    // Seguir al final: el botón lo apaga y lo enciende.
    let follow = by_label(&a.ui, "Seguir al final");
    assert!(st.get_follow());
    click(&follow);
    assert!(!st.get_follow());
    click(&follow);
    assert!(st.get_follow());
    // Limpiar: vacía la vista, pero la partida sigue escribiendo.
    click(&by_label(&a.ui, "Limpiar"));
    assert_eq!(st.get_total(), 0);
    assert!(has_label(&a.ui, "Aún no hay registros"));
    assert_eq!(by_label(&a.ui, "Copiar").accessible_enabled(), Some(false));
    assert_eq!(by_label(&a.ui, "Limpiar").accessible_enabled(), Some(false));
    a.console.lines(1, vec![RawLine::new("[Render thread/INFO]: después", false)]);
    settle();
    assert_eq!(rows(&a.ui), [(1, "[Render thread/INFO]: después".to_owned(), LogGroup::Info)]);
}

fn otra_pestana_y_partida_nueva(a: &App) {
    let st = a.ui.global::<ConsoleState>();
    click(&by_id(&a.ui, "filter")[0]);
    type_text(&a.ui, "zzz");
    click(&by_label(&a.ui, "Errores"));
    // Al volver a la pestaña, filtro y niveles empiezan de cero; las
    // líneas siguen ahí.
    a.ui.global::<InstanceState>().invoke_select_tab(InstanceTab::Settings);
    settle();
    a.ui.global::<InstanceState>().invoke_select_tab(InstanceTab::Logs);
    settle();
    assert_eq!(st.get_query(), "");
    assert_eq!(st.get_hidden().row_data(0), Some(false));
    assert_eq!(rows(&a.ui).len(), 1);
    // Partida nueva de la instancia: consola vacía.
    a.console.started(2, "survival-amigos");
    settle();
    assert_eq!(st.get_total(), 0);
    a.console.lines(1, vec![RawLine::new("de la partida vieja", false)]);
    settle();
    assert_eq!(st.get_total(), 0, "la partida 1 ya no es de nadie que se vea");
    assert_eq!(a.console.buffer("create-aeronautica").unwrap().len(), 1);
}

/// PLAN §4 (fase 3): consola con 5000 líneas sin bajar de 60 fps. Se mide el
/// fotograma completo del renderer por software (1280 × 800) con la consola
/// llena: quieta, desplazándose y recibiendo lotes de 200 líneas (que
/// descartan las viejas). El límite de 60 fps se verifica con `--release`.
fn rendimiento(a: &App) {
    let st = a.ui.global::<ConsoleState>();
    a.ui.global::<InstanceState>().invoke_select_tab(InstanceTab::Logs);
    a.console.started(3, "survival-amigos");
    settle();
    let empty = frame_ms(&a.ui, |_| a.ui.window().request_redraw());
    let t = Instant::now();
    a.console.lines(3, (0..CAPACITY).map(line).collect());
    let load = t.elapsed();
    settle();
    assert_eq!(st.get_total(), CAPACITY as i32);
    let created = shown_lines(&a.ui).len();
    assert!(created < 60, "la lista solo crea las filas visibles ({created})");
    let last = format!("línea {} ", CAPACITY - 1);
    assert!(shown_lines(&a.ui).iter().any(|l| l.contains(&last)), "sigue al final");

    let still = frame_ms(&a.ui, |_| {
        a.ui.window().request_redraw();
    });
    let scrolling = frame_ms(&a.ui, |i| scroll(&a.ui, if i % 2 == 0 { 120.0 } else { -60.0 }));
    assert!(!st.get_follow(), "al subir deja de seguir");
    let mut n = CAPACITY;
    let incoming = frame_ms(&a.ui, |_| {
        a.console.lines(3, (n..n + 200).map(line).collect());
        n += 200;
        settle();
    });
    assert_eq!(st.get_total(), CAPACITY as i32, "acotada a {CAPACITY}");
    let release = !cfg!(debug_assertions);
    println!(
        "consola {CAPACITY} líneas ({}): cargar {:.1} ms; fotograma con la consola vacía {empty:.2} ms, quieta {still:.2} ms, desplazando {scrolling:.2} ms, llegando 200 líneas {incoming:.2} ms",
        if release { "release" } else { "debug" },
        load.as_secs_f64() * 1000.0
    );
    // En debug se comprueban la virtualización, el seguimiento y el límite
    // del búfer. Los tiempos absolutos dependen del runner y del renderer
    // sin optimizar, incluso para una consola vacía.
    if release {
        let budget = 1000.0 / 60.0;
        for (what, ms) in [("quieta", still), ("desplazando", scrolling), ("llegando líneas", incoming)] {
            assert!(ms < budget, "{what}: {ms:.2} ms por fotograma (umbral {budget:.1} ms)");
        }
        assert!(load < Duration::from_secs(2));
    }
}
