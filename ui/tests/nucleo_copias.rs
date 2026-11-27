//! F3 copias
//! La pestaña Copias contra el núcleo real (`lcore` en una carpeta
//! temporal): clics y teclado de verdad, el trabajo en tokio
//! (`CoreBackups`, `CoreInstance`) y los resultados de vuelta por
//! `invoke_from_event_loop`. Comprueba el efecto en disco: el zip y
//! `backups.json`, la carpeta del juego tras restaurar y `instance.json`.
//!
//! Un solo `#[test]` (el backend de pruebas de Slint se inicia una vez por
//! proceso), con hilos y tiempo real para que corra el bucle de eventos.

use std::time::{Duration, Instant};

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use lcore::instance::{GameVersion, INSTANCE_SCHEMA, Instance, LoaderKind};
use lcore::{Core, Options};
use shard_ui::backend::Backend;
use shard_ui::backups::{Backups, CoreBackups};
use shard_ui::controller::{Actions, Controller};
use shard_ui::instance::{CoreInstance, InstanceDetail};
use shard_ui::instances::Record;
use shard_ui::{BackupsState, InstanceTab, MainWindow, Route};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, Model};

const ID: &str = "prueba";

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
    slint::run_event_loop().unwrap();
    assert!(ok.get(), "no llegó: {what}");
}

fn by_id(ui: &MainWindow, id: &str) -> Vec<ElementHandle> {
    let id = id.to_owned();
    ui.root_element()
        .query_descendants()
        .match_predicate(move |e| e.accessible_id().as_deref() == Some(id.as_str()))
        .find_all()
}

fn click(e: &ElementHandle) {
    e.mock_single_click(PointerEventButton::Left);
}

fn last_label(ui: &MainWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(ui, label).last().unwrap_or_else(|| panic!("no hay «{label}»"))
}

fn type_line(ui: &MainWindow, text: &str) {
    for ch in text.chars().chain(['\n']) {
        let t: slint::SharedString = ch.to_string().into();
        ui.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
        ui.window().dispatch_event(WindowEvent::KeyReleased { text: t });
    }
}

/// Espera a que la UI tenga `n` copias.
fn wait_items(ui: &MainWindow, n: usize) {
    let w = ui.as_weak();
    wait(&format!("{n} copias"), move || w.unwrap().global::<BackupsState>().get_items().row_count() == n);
}

#[test]
fn copias_con_el_nucleo_real() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions { mock_time: false, threading: true, renderer_name: Some("software".into()) },
    )))
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let core = rt.block_on(Core::open(Options { data_dir: Some(dir.path().to_path_buf()), offline: true, ..Default::default() })).unwrap();
    let instance = Instance {
        schema: INSTANCE_SCHEMA,
        id: ID.into(),
        name: "Prueba".into(),
        game: GameVersion { mc: "1.21.4".into(), loader: LoaderKind::Vanilla, loader_version: None },
        created_at: 0,
        last_played: None,
        playtime_seconds: 0,
        java: Default::default(),
        window: Default::default(),
        backups: Default::default(),
        dev_console: false,
        imported_from: None,
    };
    rt.block_on(core.save_instance(&instance)).unwrap();
    let game = core.instance_game_dir(ID).unwrap();
    let level = game.join("saves/Mundo/level.dat");
    let options = game.join("options.txt");
    std::fs::create_dir_all(level.parent().unwrap()).unwrap();
    std::fs::write(&level, "mundo v1").unwrap();
    std::fs::write(&options, "opciones v1").unwrap();
    let backend = Backend::new(core.clone(), rt.handle().clone());

    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    ui.set_animate_background(false);
    let now = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let ctl = Controller::new(&ui, "es", Box::new(now), Box::new(NoActions));
    ctl.set_instances(vec![Record {
        id: ID.into(),
        name: "Prueba".into(),
        mc: "1.21.4".into(),
        loader: "vanilla".into(),
        created_at: 0,
        last_played: None,
        playtime_seconds: 0,
    }]);
    let detail = InstanceDetail::new(&ui, ctl.clone(), Box::new(CoreInstance(backend.clone())));
    let _b = Backups::new(&ui, detail.clone(), Box::new(CoreBackups::new(backend.clone())));
    ui.set_route(Route::Instances);
    ui.show().unwrap();
    detail.open(ID, Some(InstanceTab::Backups));
    let w = ui.as_weak();
    wait("lista y ajustes", move || {
        let ui = w.unwrap();
        let st = ui.global::<BackupsState>();
        !st.get_loading() && st.get_settings_ready()
    });
    assert_eq!(by_id(&ui, "empty").len(), 1);
    assert!(!ui.global::<BackupsState>().get_auto());

    // Crear con etiqueta (Intro en el campo).
    click(&by_id(&ui, "create-play")[0]);
    let w = ui.as_weak();
    wait("diálogo de crear", move || w.unwrap().global::<BackupsState>().get_create_open());
    type_line(&ui, "Base");
    let w = ui.as_weak();
    wait("copia creada", move || w.unwrap().get_toast_message() == "Copia creada");
    wait_items(&ui, 1);
    let list = rt.block_on(core.list_backups(ID)).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].label.as_deref(), Some("Base"));
    assert!(!list[0].automatic && !list[0].pinned);
    assert!(dir.path().join("backups").join(ID).join(format!("{}.zip", list[0].id)).is_file());
    let base = list[0].id.clone();

    // Fijar.
    click(&by_id(&ui, "pin")[0]);
    let w = ui.as_weak();
    wait("fijada", move || w.unwrap().global::<BackupsState>().get_items().row_data(0).is_some_and(|i| i.pinned));
    assert!(rt.block_on(core.list_backups(ID)).unwrap()[0].pinned);

    // Cambia el juego y se restauran solo los mundos, con copia previa.
    std::fs::write(&level, "mundo v2").unwrap();
    std::fs::write(&options, "opciones v2").unwrap();
    click(&by_id(&ui, "restore")[0]);
    let w = ui.as_weak();
    wait("diálogo de restaurar", move || w.unwrap().global::<BackupsState>().get_restore_open());
    click(&by_id(&ui, "mode")[1]);
    click(&by_id(&ui, "filled-play")[0]);
    let w = ui.as_weak();
    wait("restaurada", move || w.unwrap().get_toast_message() == "Copia «Base» restaurada");
    assert!(!ui.global::<BackupsState>().get_restore_open());
    assert_eq!(std::fs::read_to_string(&level).unwrap(), "mundo v1");
    assert_eq!(std::fs::read_to_string(&options).unwrap(), "opciones v2");
    wait_items(&ui, 2);
    let list = rt.block_on(core.list_backups(ID)).unwrap();
    assert_eq!(list.iter().filter(|b| b.label.as_deref() == Some("Antes de restaurar")).count(), 1);

    // Restaurar todo sin copia previa: también vuelve options.txt.
    click(&by_id(&ui, "restore")[1]);
    let w = ui.as_weak();
    wait("diálogo de restaurar", move || w.unwrap().global::<BackupsState>().get_restore_open());
    assert_eq!(ui.global::<BackupsState>().get_restore_item().id, base.as_str());
    click(&by_id(&ui, "safety")[0]);
    click(&by_id(&ui, "filled-play")[0]);
    let w = ui.as_weak();
    wait("restaurada todo", move || !w.unwrap().global::<BackupsState>().get_restore_open());
    assert_eq!(std::fs::read_to_string(&options).unwrap(), "opciones v1");
    assert_eq!(rt.block_on(core.list_backups(ID)).unwrap().len(), 2);

    // Copias automáticas: encender y elegir semanal queda en instance.json.
    click(&by_id(&ui, "auto-switch")[0]);
    let (c, r) = (core.clone(), rt.handle().clone());
    wait("automáticas encendidas", move || r.block_on(c.get_instance(ID)).unwrap().backups.auto);
    click(&last_label(&ui, "Semanal"));
    let (c, r) = (core.clone(), rt.handle().clone());
    wait("semanal", move || r.block_on(c.get_instance(ID)).unwrap().backups.interval_hours == Some(168));

    // Cambiar etiqueta y eliminar desde el menú ⋮.
    click(&by_id(&ui, "dots")[1]);
    click(&last_label(&ui, "Cambiar etiqueta"));
    let w = ui.as_weak();
    wait("diálogo de etiqueta", move || w.unwrap().global::<BackupsState>().get_rename_open());
    type_line(&ui, "Inicial");
    let (c, r, b) = (core.clone(), rt.handle().clone(), base.clone());
    wait("etiqueta nueva", move || {
        r.block_on(c.list_backups(ID)).unwrap().iter().any(|x| x.id == b && x.label.as_deref() == Some("Inicial"))
    });
    click(&by_id(&ui, "dots")[1]);
    click(&last_label(&ui, "Eliminar"));
    let w = ui.as_weak();
    wait("confirmación", move || w.unwrap().global::<BackupsState>().get_delete_open());
    click(&last_label(&ui, "Eliminar"));
    wait_items(&ui, 1);
    let list = rt.block_on(core.list_backups(ID)).unwrap();
    assert!(list.iter().all(|x| x.id != base));
    assert!(!dir.path().join("backups").join(ID).join(format!("{base}.zip")).exists());
}
