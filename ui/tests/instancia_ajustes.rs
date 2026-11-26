//! Pestaña Ajustes de la instancia (fase 3): clics y teclado de verdad.
//! Primero con las acciones apuntadas (`fixtures::Recorder`): qué se pide
//! al núcleo y cómo queda la UI; después contra `lcore` real en una carpeta
//! temporal: el `instance.json` cambia, la lista y la cabecera también.
//!
//! Un solo `#[test]` (el backend de pruebas de Slint se inicia una vez por
//! proceso), con hilos y tiempo real para que corra el bucle de eventos.

use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use lcore::{Core, Options};
use shard_ui::backend::Backend;
use shard_ui::controller::{Actions, Controller};
use shard_ui::errors::Failure;
use shard_ui::fixtures::{self, Recorder};
use shard_ui::inst_settings::{CoreInstSettings, InstSettings};
use shard_ui::instance::{CoreInstance, InstanceDetail};
use shard_ui::{InstSettingsState, InstanceState, InstanceTab, MainWindow, Route};
use slint::ComponentHandle;
use slint::platform::{Key, PointerEventButton, WindowEvent};

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
    let ok = Rc::new(std::cell::Cell::new(false));
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

fn by_id(ui: &MainWindow, id: &str) -> Vec<ElementHandle> {
    let id = id.to_owned();
    ui.root_element().query_descendants().match_predicate(move |e| e.accessible_id().as_deref() == Some(id.as_str())).find_all()
}

fn one(ui: &MainWindow, id: &str) -> ElementHandle {
    by_id(ui, id).into_iter().next().unwrap_or_else(|| panic!("no está «{id}»"))
}

fn click(e: &ElementHandle) {
    e.mock_single_click(PointerEventButton::Left);
}

fn key(ui: &MainWindow, k: Key, n: usize) {
    let t: slint::SharedString = k.into();
    for _ in 0..n {
        ui.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
        ui.window().dispatch_event(WindowEvent::KeyReleased { text: t.clone() });
    }
}

fn type_text(ui: &MainWindow, text: &str) {
    for ch in text.chars() {
        let t: slint::SharedString = ch.to_string().into();
        ui.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
        ui.window().dispatch_event(WindowEvent::KeyReleased { text: t });
    }
}

/// Clic en el campo, al final, borra `erase` caracteres y escribe `text`.
fn edit(ui: &MainWindow, id: &str, erase: usize, text: &str) {
    click(&one(ui, id));
    key(ui, Key::End, 1);
    key(ui, Key::Backspace, erase);
    type_text(ui, text);
}

/// Interruptores «Usar el valor global» (0 = memoria, 1 = JVM).
fn global_switch(ui: &MainWindow, i: usize) -> ElementHandle {
    let group = &by_id(ui, "global-switch")[i];
    let (x, y) = (group.absolute_position().x + group.size().width - 30.0, group.absolute_position().y + 24.0);
    ElementHandle::find_by_accessible_label(ui, "Usar el valor global")
        .find(|e| {
            let p = e.absolute_position();
            (p.x..p.x + e.size().width).contains(&x) && (p.y..p.y + e.size().height).contains(&y) && e.accessible_checked().is_some()
        })
        .expect("interruptor")
}

#[test]
fn ajustes_de_la_instancia() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions { mock_time: false, threading: true, renderer_name: Some("software".into()) },
    )))
    .unwrap();
    // La primera ventana sigue abierta: cerrar la última pide salir del bucle.
    let _first = con_acciones_apuntadas();
    con_el_nucleo_real();
}

fn con_acciones_apuntadas() -> MainWindow {
    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    let ctl = Controller::new(&ui, "es", Box::new(|| fixtures::NOW), Box::new(NoActions));
    ui.set_animate_background(false);
    ctl.set_instances(fixtures::sample_instances());
    let rec = Recorder::default();
    let detail = InstanceDetail::new(&ui, ctl.clone(), Box::new(rec.clone()));
    let settings = InstSettings::new(&ui, &detail, Box::new(rec.clone()));
    ui.set_route(Route::Instances);
    ui.show().unwrap();
    let st = ui.global::<InstSettingsState>();

    detail.open("survival-amigos", Some(InstanceTab::Settings));
    assert_eq!(rec.take(), ["load-instance survival-amigos", "settings globals"]);
    assert!(!st.get_ready(), "sin la instancia completa no hay formulario");
    detail.loaded("survival-amigos", Ok(fixtures::detail::sample_detail("survival-amigos").unwrap()));
    settings.globals_loaded(fixtures::logs::sample_globals());
    assert!(st.get_ready());
    assert_eq!(st.get_name(), "Survival con amigos");
    assert_eq!((st.get_custom_memory(), st.get_memory_mb()), (true, 6144));
    assert_eq!(st.get_jvm(), "-XX:+UseG1GC -XX:MaxGCPauseMillis=50");
    assert_eq!((st.get_width().as_str(), st.get_height().as_str()), ("1600", "900"));
    assert!(!st.get_dirty());
    assert!(by_id(&ui, "unsaved").is_empty());

    // Cambiar el nombre saca la barra; «Descartar» la quita.
    edit(&ui, "name-field", 0, " II");
    assert_eq!(st.get_name(), "Survival con amigos II");
    assert!(st.get_dirty());
    click(&one(&ui, "discard"));
    assert_eq!(st.get_name(), "Survival con amigos");
    assert!(!st.get_dirty());

    // Ancho: solo dígitos, como mucho 5.
    edit(&ui, "width-field", 4, "1x28a0");
    assert_eq!(st.get_width(), "1280");
    edit(&ui, "height-field", 3, "1234567");
    assert_eq!(st.get_height(), "12345");
    edit(&ui, "height-field", 5, "900");
    // Memoria global: el texto «Global: 4 GB» en lugar del deslizador.
    click(&global_switch(&ui, 0));
    assert!(!st.get_custom_memory());
    assert_eq!(one(&ui, "memory-global").accessible_label().unwrap(), "Global: 4 GB");
    // JVM global y vuelta a propia con el campo vacío: se rellena con la global.
    click(&global_switch(&ui, 1));
    assert!(!st.get_custom_jvm());
    st.set_jvm("".into());
    click(&global_switch(&ui, 1));
    assert_eq!(st.get_jvm(), "-XX:+UseG1GC");
    // Pantalla completa.
    click(&one(&ui, "fullscreen"));
    assert!(st.get_fullscreen());
    assert!(st.get_dirty());

    // Guardar: lo que se pide al núcleo.
    click(&one(&ui, "save"));
    assert_eq!(
        rec.take(),
        ["settings save survival-amigos name=Survival con amigos memory=None jvm=Some([\"-XX:+UseG1GC\"]) window=Some(1280)xSome(900) fullscreen=true"]
    );
    assert!(st.get_saving());
    click(&one(&ui, "save"));
    assert!(rec.take().is_empty(), "no se guarda dos veces a la vez");

    // Error: nota con el motivo, el formulario sigue igual.
    let err = Failure { kind: "io".into(), message: "No se pudo guardar".into(), detail: "io: disco lleno".into() };
    settings.saved("survival-amigos", Err(err));
    assert!(!st.get_saving());
    assert_eq!(st.get_error(), "No se pudo guardar");
    assert_eq!(one(&ui, "error").accessible_label().unwrap(), "No se pudo guardar");
    assert!(st.get_dirty());

    // Guardado: el formulario pasa a ser lo guardado, aviso y el detalle se relee.
    click(&one(&ui, "save"));
    rec.take();
    let mut saved = fixtures::detail::sample_detail("survival-amigos").unwrap().instance;
    settings.form().unwrap().values().apply(&mut saved);
    settings.saved("survival-amigos", Ok(saved));
    assert_eq!(ui.get_toast_message(), "Cambios guardados");
    assert_eq!(rec.take(), ["load-instance survival-amigos"]);
    assert!(!st.get_dirty());
    assert_eq!(st.get_error(), "");

    // Nombre vacío: no se guarda.
    edit(&ui, "name-field", 40, "   ");
    assert!(st.get_dirty());
    assert!(!st.get_can_save());
    click(&one(&ui, "save"));
    assert!(rec.take().is_empty());

    // Al volver a la pestaña, lo que no se guardó se pierde (widget nuevo).
    ui.global::<InstanceState>().invoke_select_tab(InstanceTab::Logs);
    ui.global::<InstanceState>().invoke_select_tab(InstanceTab::Settings);
    assert_eq!(st.get_name(), "Survival con amigos");
    assert!(!st.get_dirty());
    ui
}

fn con_el_nucleo_real() {
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
    let backend = Backend::new(core.clone(), rt.handle().clone());

    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    let ctl = Controller::new(&ui, "es", Box::new(|| fixtures::NOW), Box::new(NoActions));
    ui.set_animate_background(false);
    shard_ui::backend::install(ctl.clone());
    let detail = InstanceDetail::new(&ui, ctl.clone(), Box::new(CoreInstance(backend.clone())));
    let _settings = InstSettings::new(&ui, &detail, Box::new(CoreInstSettings(backend.clone())));
    ui.set_route(Route::Instances);
    ui.show().unwrap();
    backend.load();
    let c = ctl.clone();
    wait("instancias", move || c.instance_item("survival-amigos").is_some());

    detail.open("survival-amigos", Some(InstanceTab::Settings));
    let w = ui.as_weak();
    wait("formulario", move || w.unwrap().global::<InstSettingsState>().get_ready());
    let st = ui.global::<InstSettingsState>();
    // La instancia sembrada usa los valores globales (4 GB por defecto).
    assert_eq!(st.get_name(), "Survival con amigos");
    assert!(!st.get_custom_memory());
    let w = ui.as_weak();
    wait("RAM del equipo", move || w.unwrap().global::<InstSettingsState>().get_total_mb() > 0);

    edit(&ui, "name-field", 0, " II");
    click(&global_switch(&ui, 0));
    assert!(st.get_custom_memory());
    edit(&ui, "width-field", 0, "1280");
    edit(&ui, "height-field", 0, "720");
    click(&one(&ui, "save"));
    let w = ui.as_weak();
    wait("guardado", move || w.unwrap().get_toast_message() == "Cambios guardados");
    assert!(!st.get_dirty());

    // En disco.
    let i = rt.block_on(core.get_instance("survival-amigos")).unwrap();
    assert_eq!(i.name, "Survival con amigos II");
    assert_eq!(i.java.memory_mb, Some(4096));
    assert_eq!(i.java.jvm_args, None);
    assert_eq!((i.window.width, i.window.height, i.window.fullscreen), (Some(1280), Some(720), false));
    let json = std::fs::read_to_string(dir.path().join("instances/survival-amigos/instance.json")).unwrap();
    assert!(json.contains("\"name\": \"Survival con amigos II\""), "{json}");
    // La lista y la cabecera se recargan.
    let w = ui.as_weak();
    wait("cabecera", move || w.unwrap().global::<InstanceState>().get_item().name == "Survival con amigos II");
    ui.hide().unwrap();
}
