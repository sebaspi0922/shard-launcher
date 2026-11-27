//! Diálogos de instancias contra el núcleo real (`lcore` en una carpeta
//! temporal, sin red): crear una instancia vanilla, cambiarle la versión con
//! copia previa, mandarla a la papelera, recuperarla y borrarla para
//! siempre. Clics de verdad sobre la UI, el trabajo en tokio (`CoreCreate`,
//! `CoreChangeVersion`, `CoreTrash`) y los resultados de vuelta por
//! `invoke_from_event_loop`. Comprueba lo que queda en disco.
//!
//! Sin red no hay manifiesto de versiones: se comprueba que el diálogo lo
//! dice y el test entrega una lista (como si hubiera llegado) para seguir.
//!
//! Un solo `#[test]` (el backend de pruebas de Slint se inicia una vez por
//! proceso), con hilos y tiempo real para que corra el bucle de eventos.

use std::sync::Arc;
use std::time::{Duration, Instant};

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use lcore::{Core, Options};
use shard_ui::backend::{self, Backend};
use shard_ui::change_version::{ChangeVersion, CoreChangeVersion};
use shard_ui::controller::{Actions, Controller};
use shard_ui::create_instance::{CoreCreate, CreateInstance, Version};
use shard_ui::instance::{CoreInstance, InstanceDetail};
use shard_ui::trash::{CoreTrash, Trash};
use shard_ui::{ChangeVersionState, CreateInstanceState, MainWindow, Route, TrashState};
use slint::platform::PointerEventButton;
use slint::{ComponentHandle, Model};

struct NoActions;

impl Actions for NoActions {
    fn open_instance(&self, _: &str) {}
    fn play(&self, _: &str) {}
    fn new_instance(&self) {}
    fn show_trash(&self) {}
}

/// Corre el bucle de eventos hasta que `cond` se cumpla (20 s como mucho).
fn wait(what: &str, cond: impl Fn() -> bool + 'static) {
    let start = Instant::now();
    let timer = slint::Timer::default();
    let ok = std::rc::Rc::new(std::cell::Cell::new(false));
    let ok2 = ok.clone();
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(10), move || {
        if cond() {
            ok2.set(true);
            let _ = slint::quit_event_loop();
        } else if start.elapsed() > Duration::from_secs(20) {
            let _ = slint::quit_event_loop();
        }
    });
    // Un `quit_event_loop` pendiente de una espera anterior puede cortar
    // esta antes de tiempo: se vuelve a entrar hasta que se cumpla o pasen
    // los 20 s.
    while !ok.get() && start.elapsed() <= Duration::from_secs(20) {
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

fn click_label(ui: &MainWindow, label: &str) {
    ElementHandle::find_by_accessible_label(ui, label)
        .next()
        .unwrap_or_else(|| panic!("no hay «{label}»"))
        .mock_single_click(PointerEventButton::Left);
}

fn click_id(ui: &MainWindow, id: &str, n: usize) {
    by_id(ui, id)[n].mock_single_click(PointerEventButton::Left);
}

#[test]
fn dialogos_con_el_nucleo_real() {
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
    let b = Backend::new(core.clone(), rt.handle().clone());

    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    ui.set_animate_background(false);
    let now = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let ctl = Controller::new(&ui, "es", Box::new(now), Box::new(NoActions));
    backend::install(ctl.clone());
    ctl.set_instances(vec![]);
    let detail = InstanceDetail::new(&ui, ctl.clone(), Box::new(CoreInstance(b.clone())));
    let create = CreateInstance::new(&ui, ctl.clone(), Box::new(CoreCreate(b.clone())));
    let trash = Trash::new(&ui, ctl.clone(), Box::new(CoreTrash(b.clone())), Box::new(now));
    shard_ui::create_instance::dialog_hooks(&ctl, &create, &trash);
    let _change = ChangeVersion::new(&ui, detail.clone(), Box::new(CoreChangeVersion(b.clone())));
    ui.set_route(Route::Instances);
    ui.show().unwrap();

    // --- Crear --------------------------------------------------------------
    // Sin red el manifiesto no llega: el diálogo lo dice y no deja crear.
    ctl.new_instance();
    let w = ui.as_weak();
    wait("error de versiones", move || !w.unwrap().global::<CreateInstanceState>().get_picker().error.is_empty());
    assert!(!ui.global::<CreateInstanceState>().get_can_create());
    // Con una lista (como si hubiera llegado), vanilla 1.21.5.
    create.versions_loaded(false, Ok(vec![Version::new("1.21.5", "release"), Version::new("1.21.4", "release")]));
    let w = ui.as_weak();
    wait("versión elegida", move || w.unwrap().global::<CreateInstanceState>().get_can_create());
    assert_eq!(ui.global::<CreateInstanceState>().get_name(), "Minecraft 1.21.5");
    click_label(&ui, "Crear");
    let w = ui.as_weak();
    wait("instancia creada", move || w.unwrap().get_toast_message() == "Instancia «Minecraft 1.21.5» creada");
    assert!(!ui.global::<CreateInstanceState>().get_open());
    let list = rt.block_on(core.list_instances()).unwrap();
    assert_eq!(list.len(), 1);
    let id = list[0].id.clone();
    assert_eq!((list[0].game.mc.as_str(), list[0].game.loader.as_str()), ("1.21.5", "vanilla"));
    assert!(core.instance_game_dir(&id).unwrap().is_dir(), "carpeta del juego creada");
    assert_eq!(detail.id().as_deref(), Some(id.as_str()), "se abre la nueva");
    assert_eq!(ctl.model.borrow().all.len(), 1, "la lista se recargó");

    // --- Cambiar versión (con copia previa) ---------------------------------
    let d = detail.clone();
    wait("detalle cargado", move || d.detail().is_some());
    click_label(&ui, "Más opciones");
    let w = ui.as_weak();
    wait("menú", move || !by_id(&w.unwrap(), "menu").is_empty());
    click_label(&ui, "Cambiar versión");
    let w = ui.as_weak();
    wait("diálogo de versión", move || w.unwrap().global::<ChangeVersionState>().get_open());
    let w = ui.as_weak();
    wait("error de versiones", move || !w.unwrap().global::<ChangeVersionState>().get_picker().error.is_empty());
    shard_ui::change_version::with(|c| c.versions_loaded(false, Ok(vec![Version::new("1.21.5", "release"), Version::new("1.21.4", "release")])));
    click_label(&ui, "1.21.4");
    let w = ui.as_weak();
    wait("se puede aplicar", move || w.unwrap().global::<ChangeVersionState>().get_can_apply());
    click_label(&ui, "Cambiar versión");
    let w = ui.as_weak();
    wait("versión cambiada", move || w.unwrap().get_toast_message() == "Versión cambiada a 1.21.4 · Vanilla");
    let i = rt.block_on(core.get_instance(&id)).unwrap();
    assert_eq!(i.game.mc, "1.21.4");
    let backups = rt.block_on(core.list_backups(&id)).unwrap();
    assert_eq!(backups.len(), 1, "copia antes de cambiar");
    assert_eq!(backups[0].label.as_deref(), Some("Antes de cambiar desde 1.21.5 · Vanilla"));

    // --- Papelera: recuperar ------------------------------------------------
    rt.block_on(core.delete_instance(&id)).unwrap();
    ctl.show_trash();
    let w = ui.as_weak();
    wait("papelera con una", move || w.unwrap().global::<TrashState>().get_items().row_count() == 1);
    let item = ui.global::<TrashState>().get_items().row_data(0).unwrap();
    assert_eq!((item.name.as_str(), item.days_left), ("Minecraft 1.21.5", 30));
    click_id(&ui, "outlined", 0);
    let w = ui.as_weak();
    wait("recuperada", move || w.unwrap().get_toast_message() == "«Minecraft 1.21.5» se recuperó");
    assert!(!ui.global::<TrashState>().get_open());
    assert_eq!(rt.block_on(core.list_instances()).unwrap().len(), 1);
    assert!(rt.block_on(core.list_trash()).unwrap().is_empty());
    assert_eq!(rt.block_on(core.list_backups(&id)).unwrap().len(), 1, "la copia vuelve con ella");

    // --- Papelera: borrar para siempre --------------------------------------
    rt.block_on(core.delete_instance(&id)).unwrap();
    ctl.show_trash();
    let w = ui.as_weak();
    wait("papelera con una", move || w.unwrap().global::<TrashState>().get_items().row_count() == 1);
    click_id(&ui, "trash-purge", 0);
    let w = ui.as_weak();
    wait("confirmación", move || w.unwrap().global::<TrashState>().get_confirm_open());
    by_id(&ui, "confirm-purge")[0]
        .query_descendants()
        .match_accessible_label("Borrar para siempre")
        .find_first()
        .unwrap()
        .mock_single_click(PointerEventButton::Left);
    let w = ui.as_weak();
    wait("papelera vacía", move || w.unwrap().global::<TrashState>().get_state() == 1);
    assert!(rt.block_on(core.list_trash()).unwrap().is_empty());
    assert_eq!(std::fs::read_dir(core.paths().trash_dir()).unwrap().count(), 0, "nada en trash/");
    assert!(rt.block_on(core.list_instances()).unwrap().is_empty());
}
