//! Servidores y Contenido contra el núcleo real (`lcore` en una carpeta
//! temporal, sin red): clics y teclado de verdad sobre la UI, el trabajo en
//! tokio (`servers::CoreServers`, `content_backend`) y los resultados de
//! vuelta por `invoke_from_event_loop`. Comprueba lo que queda en
//! `servers.json`.
//!
//! Un solo `#[test]` (el backend de pruebas de Slint se inicia una vez por
//! proceso), con hilos y tiempo real para que corra el bucle de eventos.

use std::sync::Arc;
use std::time::{Duration, Instant};

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use lcore::{Core, Options};
use shard_ui::accounts::Accounts;
use shard_ui::accounts_backend::CoreAccounts;
use shard_ui::backend::{self, Backend};
use shard_ui::content::Content;
use shard_ui::content_backend::CoreContent;
use shard_ui::controller::{Actions, Controller};
use shard_ui::servers::{CoreServers, Servers};
use shard_ui::{ContentState, MainWindow, Route, ServersState};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, Model};

struct NoActions;

impl Actions for NoActions {
    fn open_instance(&self, _: &str) {}
    fn play(&self, _: &str) {}
    fn new_instance(&self) {}
    fn show_trash(&self) {}
}

/// Corre el bucle de eventos hasta que `cond` se cumpla (10 s como mucho).
fn wait(what: &str, cond: impl Fn() -> bool + 'static) {
    let start = Instant::now();
    let timer = slint::Timer::default();
    let ok = std::rc::Rc::new(std::cell::Cell::new(false));
    let ok2 = ok.clone();
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(10), move || {
        if cond() {
            ok2.set(true);
            let _ = slint::quit_event_loop();
        } else if start.elapsed() > Duration::from_secs(10) {
            let _ = slint::quit_event_loop();
        }
    });
    // Una salida pendiente de un tick anterior puede cortar el bucle antes
    // de tiempo: se vuelve a entrar mientras quede plazo.
    while !ok.get() && start.elapsed() <= Duration::from_secs(10) {
        slint::run_event_loop().unwrap();
    }
    assert!(ok.get(), "no llegó: {what}");
}

fn by_id(ui: &MainWindow, id: &str) -> Vec<ElementHandle> {
    let id = id.to_owned();
    ui.root_element()
        .query_descendants()
        .match_predicate(move |e| e.accessible_id().as_deref() == Some(id.as_str()))
        .find_all()
}

fn click_id(ui: &MainWindow, id: &str, i: usize) {
    let list = by_id(ui, id);
    let e = list.get(i).unwrap_or_else(|| panic!("no hay «{id}» #{i}"));
    e.mock_single_click(PointerEventButton::Left);
}

fn type_line(ui: &MainWindow, text: &str) {
    for ch in text.chars().chain(['\n']) {
        let t: slint::SharedString = ch.to_string().into();
        ui.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
        ui.window().dispatch_event(WindowEvent::KeyReleased { text: t });
    }
}

fn names(ui: &MainWindow) -> Vec<String> {
    let items = ui.global::<ServersState>().get_items();
    (0..items.row_count()).map(|i| items.row_data(i).unwrap().name.to_string()).collect()
}

/// Abre el diálogo de añadir, escribe la dirección y pulsa Intro.
fn add(ui: &MainWindow, address: &str) {
    click_id(ui, "add", 0);
    let w = ui.as_weak();
    wait("diálogo", move || w.unwrap().global::<ServersState>().get_dialog_open());
    // Que el campo exista y tome el foco antes de teclear.
    let w = ui.as_weak();
    wait("campo", move || by_id(&w.unwrap(), "field").first().is_some_and(|f| f.accessible_value().is_some()));
    type_line(ui, address);
    let (w, a) = (ui.as_weak(), address.to_owned());
    wait("dirección escrita", move || {
        let ui = w.unwrap();
        let st = ui.global::<ServersState>();
        !st.get_dialog_open() || st.get_dialog_address() == a.as_str()
    });
    let w = ui.as_weak();
    wait("guardado", move || !w.unwrap().global::<ServersState>().get_dialog_open());
}

#[test]
fn servidores_con_el_nucleo_real() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions { mock_time: false, threading: true, renderer_name: Some("software".into()) },
    )))
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let core = rt
        .block_on(Core::open(Options {
            data_dir: Some(dir.path().to_path_buf()),
            offline: true,
            secrets: Some(Arc::new(lcore::secrets::MemoryStore::default())),
            ..Default::default()
        }))
        .unwrap();
    let backend = Backend::new(core.clone(), rt.handle().clone());

    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    ui.set_animate_background(false);
    let ctl = Controller::new(&ui, "es", Box::new(|| 0), Box::new(NoActions));
    backend::install(ctl.clone());
    let acc = Accounts::new(&ui, ctl.clone(), Box::new(CoreAccounts::new(backend.clone(), "es")), Box::new(|| 0));
    let servers = Servers::new(&ui, ctl.clone(), acc, Box::new(CoreServers { backend: backend.clone(), lang: "es".into() }));
    let content = Content::new(&ui, ctl.clone(), Box::new(CoreContent { backend: backend.clone(), lang: "es".into() }));
    {
        let (s, c) = (servers.clone(), content.clone());
        ctl.on_refresh(move |_| {
            s.refresh();
            c.refresh();
        });
    }
    backend.load();
    ui.set_route(Route::Servers);
    ui.show().unwrap();

    // Sin servidores: estado vacío.
    let w = ui.as_weak();
    wait("lista vacía", move || w.unwrap().global::<ServersState>().get_loaded());
    assert_eq!(by_id(&ui, "empty").len(), 1);

    // Dos altas con teclado; el nombre sale de la dirección.
    add(&ui, "127.0.0.1:1");
    add(&ui, "localhost:2");
    let w = ui.as_weak();
    wait("dos servidores", move || w.unwrap().global::<ServersState>().get_items().row_count() == 2);
    let saved = rt.block_on(core.list_servers()).unwrap();
    assert_eq!(saved.iter().map(|s| s.address.as_str()).collect::<Vec<_>>(), ["127.0.0.1:1", "localhost:2"]);
    // El ping sin servidor al otro lado acaba en «No responde».
    let w = ui.as_weak();
    wait("estado", move || w.unwrap().global::<ServersState>().get_items().row_data(0).is_some_and(|c| c.known && !c.online));

    // Reordenar guarda el orden nuevo.
    let second = saved[1].id.clone();
    let first = names(&ui)[0].clone();
    ui.global::<ServersState>().invoke_move(second.into(), 0);
    // Se ve movido ya, antes de que el núcleo conteste.
    assert_eq!(names(&ui)[1], first);
    let c = core.clone();
    let h = rt.handle().clone();
    wait("orden guardado", move || h.block_on(c.list_servers()).unwrap()[0].address == "localhost:2");

    // Editar el nombre.
    click_id(&ui, "server-more", 0);
    let w = ui.as_weak();
    wait("menú", move || !by_id(&w.unwrap(), "menu-edit").is_empty());
    click_id(&ui, "menu-edit", 0);
    let w = ui.as_weak();
    wait("diálogo de editar", move || w.unwrap().global::<ServersState>().get_dialog_editing());
    ui.global::<ServersState>().set_dialog_name("Pruebas".into());
    click_id(&ui, "filled", by_id(&ui, "filled").len() - 1);
    let w = ui.as_weak();
    wait("editado", move || names(&w.unwrap())[0] == "Pruebas" && !w.unwrap().global::<ServersState>().get_dialog_open());
    assert_eq!(rt.block_on(core.list_servers()).unwrap()[0].name, "Pruebas");

    // Quitar con confirmación.
    click_id(&ui, "server-more", 0);
    let w = ui.as_weak();
    wait("menú", move || !by_id(&w.unwrap(), "menu-remove").is_empty());
    click_id(&ui, "menu-remove", 0);
    let w = ui.as_weak();
    wait("confirmación", move || !by_id(&w.unwrap(), "confirm").is_empty());
    let quitar = ElementHandle::find_by_accessible_label(&ui, "Quitar").last().unwrap();
    quitar.mock_single_click(PointerEventButton::Left);
    let w = ui.as_weak();
    wait("quitado", move || w.unwrap().global::<ServersState>().get_items().row_count() == 1);
    assert_eq!(rt.block_on(core.list_servers()).unwrap().len(), 1);

    // Contenido: la primera búsqueda contesta (resultados o, sin red, el
    // error con su mensaje).
    ui.set_route(Route::Content);
    let w = ui.as_weak();
    // Sin pintar, la página no se crea hasta que algo recorre el árbol.
    assert_eq!(by_id(&ui, "search").len(), 1);
    wait("búsqueda", move || {
        let ui = w.unwrap();
        let st = ui.global::<ContentState>();
        st.get_started() && !st.get_loading()
    });
    let st = ui.global::<ContentState>();
    assert!(st.get_items().row_count() > 0 || !st.get_error_message().is_empty());
    drop(content);
}
