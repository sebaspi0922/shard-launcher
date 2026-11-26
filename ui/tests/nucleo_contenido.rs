//! Pestaña Contenido contra el núcleo real (`lcore` en una carpeta temporal,
//! sin red): clics de verdad sobre la UI, el trabajo en tokio
//! (`CoreContent`) y los resultados de vuelta por `invoke_from_event_loop`.
//! Comprueba lo que queda en disco (`mods/`).
//!
//! Un solo `#[test]` (el backend de pruebas de Slint se inicia una vez por
//! proceso), con hilos y tiempo real para que corra el bucle de eventos.

use std::sync::Arc;
use std::time::{Duration, Instant};

use i_slint_backend_testing::ElementHandle;
use lcore::{Core, Options};
use shard_ui::backend::Backend;
use shard_ui::controller::{Actions, Controller};
use shard_ui::fixtures;
use shard_ui::inst_content::{CoreContent, InstContent};
use shard_ui::instance::{CoreInstance, InstanceDetail};
use shard_ui::{InstContentState, InstanceTab, LoadState, MainWindow};
use slint::platform::PointerEventButton;
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
    let done = std::rc::Rc::new(std::cell::Cell::new(false));
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(10), move || {
        // Un solo `quit` por espera: uno de más haría salir a la siguiente.
        if done.get() {
            return;
        }
        if cond() {
            done.set(true);
            ok2.set(true);
            let _ = slint::quit_event_loop();
        } else if start.elapsed() > Duration::from_secs(10) {
            done.set(true);
            let _ = slint::quit_event_loop();
        }
    });
    slint::run_event_loop().unwrap();
    assert!(ok.get(), "no llegó: {what}");
}

fn by_id(ui: &MainWindow, id: &str) -> Vec<ElementHandle> {
    let id = id.to_owned();
    ElementHandle::query_descendants(&i_slint_backend_testing::ElementRoot::root_element(ui))
        .match_predicate(move |e| e.accessible_id().as_deref() == Some(id.as_str()))
        .find_all()
}

fn names(ui: &MainWindow) -> Vec<String> {
    ui.global::<InstContentState>().get_rows().iter().map(|r| r.name.to_string()).collect()
}

#[test]
fn contenido_con_el_nucleo_real() {
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
    rt.block_on(fixtures::seed(&core)).unwrap();
    let id = "survival-amigos";
    let game = core.instance_game_dir(id).unwrap();
    let mods = game.join("mods");
    std::fs::create_dir_all(&mods).unwrap();
    std::fs::write(mods.join("alfa.jar"), b"alfa").unwrap();
    std::fs::write(mods.join("beta.jar.disabled"), b"beta").unwrap();
    std::fs::create_dir_all(game.join("resourcepacks")).unwrap();
    std::fs::write(game.join("resourcepacks/pack.zip"), b"pack").unwrap();
    let backend = Backend::new(core.clone(), rt.handle().clone());

    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    let ctl = Controller::new(&ui, "es", Box::new(|| fixtures::NOW), Box::new(NoActions));
    ctl.set_instances(fixtures::sample_instances());
    ui.set_animate_background(false);
    let detail = InstanceDetail::new(&ui, ctl.clone(), Box::new(CoreInstance(backend.clone())));
    let _content = InstContent::new(&ui, &detail, Box::new(CoreContent(backend.clone())));
    ui.show().unwrap();
    detail.open(id, Some(InstanceTab::Content));

    // Lista leída de disco; sin red no hay versiones nuevas ni iconos.
    let w = ui.as_weak();
    wait("lista", move || w.unwrap().global::<InstContentState>().get_load() == LoadState::Ready);
    let w = ui.as_weak();
    wait("actualizaciones", move || !w.unwrap().global::<InstContentState>().get_updates_loading());
    assert_eq!(names(&ui), ["alfa.jar", "beta.jar", "pack.zip"]);
    let st = ui.global::<InstContentState>();
    assert_eq!(st.get_counts().iter().collect::<Vec<_>>(), [3, 2, 1, 0]);
    assert_eq!(st.get_unknown_count(), 3);
    assert!(!st.get_rows().row_data(1).unwrap().enabled);

    // Desactivar alfa.jar: en disco pasa a alfa.jar.disabled.
    by_id(&ui, "content-switch")[0].mock_single_click(PointerEventButton::Left);
    let m = mods.clone();
    wait("alfa desactivado en disco", move || m.join("alfa.jar.disabled").exists() && !m.join("alfa.jar").exists());
    assert!(!ui.global::<InstContentState>().get_rows().row_data(0).unwrap().enabled);

    // Quitar beta con la confirmación: el archivo desaparece y la lista se
    // vuelve a leer.
    by_id(&ui, "content-remove")[1].mock_single_click(PointerEventButton::Left);
    let w = ui.as_weak();
    wait("diálogo", move || !by_id(&w.unwrap(), "confirm-remove").is_empty());
    let accept = ElementHandle::find_by_accessible_label(&ui, "Quitar").last().unwrap();
    accept.mock_single_click(PointerEventButton::Left);
    let w = ui.as_weak();
    wait("beta quitado", move || names(&w.unwrap()) == ["alfa.jar", "pack.zip"]);
    assert!(!mods.join("beta.jar.disabled").exists());
    assert_eq!(ui.get_toast_message(), "«beta.jar» quitado");

    // Identificar sin red: error del núcleo con su texto, nada cambia.
    by_id(&ui, "identify-play")[0].mock_single_click(PointerEventButton::Left);
    let w = ui.as_weak();
    wait("aviso de identificar", move || {
        let ui = w.unwrap();
        ui.get_toast_error() && !ui.global::<InstContentState>().get_identifying()
    });
    assert_eq!(ui.get_toast_message(), shard_ui::i18n::text("es", "errOffline"));
    assert!(mods.join("alfa.jar.disabled").exists());
    drop(rt);
}
