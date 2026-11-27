//! Mundos y capturas contra el núcleo real (`lcore` en una carpeta
//! temporal, sin red): clics y teclado de verdad sobre la UI, el trabajo en
//! tokio (`CoreWorlds`, `CoreScreenshots`) y los resultados de vuelta por
//! `invoke_from_event_loop`. Comprueba lo que queda en disco (`level.dat`,
//! `datapacks/`, `screenshots/`).
//!
//! Un solo `#[test]` (el backend de pruebas de Slint se inicia una vez por
//! proceso), con hilos y tiempo real para que corra el bucle de eventos.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use i_slint_backend_testing::{AccessibleRole, ElementHandle, ElementRoot};
use lcore::world::{Difficulty, GameMode};
use lcore::{Core, Options};
use shard_ui::backend::Backend;
use shard_ui::controller::{Actions, Controller};
use shard_ui::instance::{CoreInstance, InstanceDetail};
use shard_ui::screenshots::{CoreScreenshots, Screenshots};
use shard_ui::worlds::{CoreWorlds, Worlds};
use shard_ui::{InstanceTab, MainWindow, ScreenshotsState, WorldsState, fixtures};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, Model};

struct NoActions;

impl Actions for NoActions {
    fn open_instance(&self, _: &str) {}
    fn play(&self, _: &str) {}
    fn new_instance(&self) {}
    fn show_trash(&self) {}
}

const ID: &str = "survival-amigos";

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
    slint::run_event_loop().unwrap();
    assert!(ok.get(), "no llegó: {what}");
}

fn button(ui: &MainWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(ui, label)
        .filter(|e| e.accessible_role() == Some(AccessibleRole::Button))
        .last()
        .unwrap_or_else(|| panic!("no hay botón «{label}»"))
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

fn type_line(ui: &MainWindow, text: &str) {
    for ch in text.chars().chain(['\n']) {
        let t: slint::SharedString = ch.to_string().into();
        ui.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
        ui.window().dispatch_event(WindowEvent::KeyReleased { text: t });
    }
}

// --- level.dat mínimo (NBT sin comprimir, que el núcleo también lee) -------

fn name(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u16).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn tag(out: &mut Vec<u8>, kind: u8, key: &str) {
    out.push(kind);
    name(out, key);
}

/// `{ Data: { LevelName, GameType, Difficulty, hardcore, allowCommands,
/// LastPlayed, Version: { Name } } }`.
fn level_dat(level_name: &str) -> Vec<u8> {
    let mut b = Vec::new();
    tag(&mut b, 10, "");
    tag(&mut b, 10, "Data");
    tag(&mut b, 8, "LevelName");
    name(&mut b, level_name);
    tag(&mut b, 3, "GameType");
    b.extend_from_slice(&0i32.to_be_bytes());
    tag(&mut b, 1, "Difficulty");
    b.push(2);
    tag(&mut b, 1, "hardcore");
    b.push(0);
    tag(&mut b, 1, "allowCommands");
    b.push(0);
    tag(&mut b, 4, "LastPlayed");
    b.extend_from_slice(&1_700_000_000_000i64.to_be_bytes());
    tag(&mut b, 10, "Version");
    tag(&mut b, 8, "Name");
    name(&mut b, "1.21.4");
    b.push(0);
    b.push(0);
    b.push(0);
    b
}

fn png(path: &Path, w: u32, h: u32) {
    image::RgbaImage::from_pixel(w, h, image::Rgba([40, 90, 160, 255])).save(path).unwrap();
}

#[test]
fn mundos_y_capturas_con_el_nucleo_real() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions { mock_time: false, threading: true, renderer_name: Some("software".into()) },
    )))
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let core = rt
        .block_on(Core::open(Options {
            data_dir: Some(dir.path().join("datos")),
            offline: true,
            secrets: Some(Arc::new(lcore::secrets::MemoryStore::default())),
            ..Default::default()
        }))
        .unwrap();
    rt.block_on(fixtures::seed(&core)).unwrap();
    let game = core.instance_game_dir(ID).unwrap();
    let world = game.join("saves/Mundo A");
    std::fs::create_dir_all(world.join("datapacks")).unwrap();
    std::fs::write(world.join("level.dat"), level_dat("Mundo A")).unwrap();
    std::fs::write(world.join("datapacks/a.zip"), b"zip").unwrap();
    std::fs::create_dir_all(game.join("screenshots")).unwrap();
    png(&game.join("screenshots/uno.png"), 1280, 720);
    let backend = Backend::new(core.clone(), rt.handle().clone());

    let ui = MainWindow::new().unwrap();
    // Alta: el panel entero a la vista.
    ui.window().set_size(slint::LogicalSize::new(1280.0, 1400.0));
    ui.set_animate_background(false);
    let now = || std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let ctl = Controller::new(&ui, "es", Box::new(now), Box::new(NoActions));
    ctl.set_instances(fixtures::sample_instances());
    let detail = InstanceDetail::new(&ui, ctl.clone(), Box::new(CoreInstance(backend.clone())));
    let _worlds = Worlds::new(&ui, &detail, Box::new(CoreWorlds(backend.clone())), Box::new(now));
    let _shots = Screenshots::new(&ui, &detail, Box::new(CoreScreenshots(backend.clone())), Box::new(|_| 0));
    ui.show().unwrap();
    let list = |core: &Core| rt.block_on(core.list_worlds(ID)).unwrap();

    // Mundos: la lista y los datapacks del primero.
    detail.open(ID, Some(InstanceTab::Worlds));
    let s = ui.as_weak();
    wait("mundos", move || s.unwrap().global::<WorldsState>().get_items().row_count() == 1);
    let s = ui.as_weak();
    wait("datapacks", move || s.unwrap().global::<WorldsState>().get_datapacks().row_count() == 1);
    let w = ui.global::<WorldsState>().get_items().row_data(0).unwrap();
    assert_eq!((w.name.as_str(), w.mode, w.difficulty, w.version.as_str()), ("Mundo A", 0, 2, "1.21.4"));

    // Modo creativo y trucos: quedan en level.dat.
    click(&button(&ui, "Creativo"));
    let s = ui.as_weak();
    wait("modo", move || s.unwrap().global::<WorldsState>().get_items().row_data(0).unwrap().mode == 1);
    click(&button(&ui, "Fácil"));
    let s = ui.as_weak();
    wait("dificultad", move || s.unwrap().global::<WorldsState>().get_items().row_data(0).unwrap().difficulty == 1);
    click(&by_id(&ui, "switch")[1]);
    let s = ui.as_weak();
    wait("trucos", move || s.unwrap().global::<WorldsState>().get_items().row_data(0).unwrap().cheats);
    let w = &list(&core)[0];
    assert_eq!((w.game_mode, w.difficulty, w.cheats), (Some(GameMode::Creative), Some(Difficulty::Easy), Some(true)));

    // Renombrar con el teclado (la carpeta no cambia).
    click(&by_id(&ui, "rename")[0]);
    let s = ui.as_weak();
    wait("diálogo", move || s.unwrap().global::<WorldsState>().get_rename_open());
    type_line(&ui, "Mundo nuevo");
    let s = ui.as_weak();
    wait("nombre", move || s.unwrap().global::<WorldsState>().get_items().row_data(0).unwrap().name == "Mundo nuevo");
    assert_eq!((list(&core)[0].name.as_str(), list(&core)[0].folder.as_str()), ("Mundo nuevo", "Mundo A"));

    // Desactivar y quitar el datapack.
    let sw = ElementHandle::find_by_accessible_label(&ui, "a.zip").find(|e| e.accessible_role() == Some(AccessibleRole::Switch)).unwrap();
    click(&sw);
    let s = ui.as_weak();
    wait("desactivado", move || !s.unwrap().global::<WorldsState>().get_datapacks().row_data(0).is_some_and(|p| p.enabled));
    assert!(world.join("datapacks/.disabled/a.zip").exists());
    click(&ElementHandle::find_by_accessible_label(&ui, "Quitar").next().unwrap());
    let s = ui.as_weak();
    wait("confirmación", move || s.unwrap().global::<WorldsState>().get_remove_pack() == "a.zip");
    click(&button(&ui, "Quitar"));
    let s = ui.as_weak();
    wait("sin datapacks", move || s.unwrap().global::<WorldsState>().get_datapacks().row_count() == 0);
    assert!(rt.block_on(core.list_datapacks(ID, "Mundo A")).unwrap().is_empty());

    // Añadir: lo que hace «Añadir datapacks» tras el selector de archivos.
    let zips = [dir.path().join("b.zip"), dir.path().join("c.txt")];
    std::fs::write(&zips[0], b"zip").unwrap();
    std::fs::write(&zips[1], b"no").unwrap();
    let r = rt.block_on(shard_ui::worlds::add_all(&core, ID, "Mundo A", &zips));
    assert!(r.is_err(), "c.txt no es un datapack");
    assert!(world.join("datapacks/b.zip").exists(), "b.zip se copia igual");

    // Borrar el mundo para siempre.
    click(&by_id(&ui, "delete-world")[0]);
    click(&button(&ui, "Borrar para siempre"));
    let s = ui.as_weak();
    wait("sin mundos", move || s.unwrap().global::<WorldsState>().get_items().row_count() == 0);
    assert_eq!(ui.get_toast_message(), "Mundo «Mundo nuevo» borrado");
    assert!(!world.exists());

    // Capturas: miniatura real (600 de ancho) y visor con la imagen entera.
    detail.select_tab(InstanceTab::Screenshots);
    let s = ui.as_weak();
    wait("miniatura", move || {
        s.unwrap().global::<ScreenshotsState>().get_items().row_data(0).is_some_and(|i| i.image.size().width == 600)
    });
    click(&by_id(&ui, "shot")[0]);
    let s = ui.as_weak();
    wait("imagen completa", move || s.unwrap().global::<ScreenshotsState>().get_viewer_image().size().width == 1280);
    click(&button(&ui, "Eliminar"));
    let s = ui.as_weak();
    wait("confirmación", move || s.unwrap().global::<ScreenshotsState>().get_confirm_delete());
    click(&button(&ui, "Eliminar"));
    let s = ui.as_weak();
    wait("sin capturas", move || {
        let ui = s.unwrap();
        let st = ui.global::<ScreenshotsState>();
        st.get_items().row_count() == 0 && !st.get_viewer_open()
    });
    assert!(!game.join("screenshots/uno.png").exists());
}
