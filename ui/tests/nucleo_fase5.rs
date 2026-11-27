//! Fase 5 contra el núcleo real (`lcore` en una carpeta temporal, sin red):
//! clics de verdad sobre Ajustes, Doctor y el primer arranque, el trabajo en
//! tokio y los resultados de vuelta por `invoke_from_event_loop`. Comprueba
//! lo que queda en `settings.json`, `ui.json` y las instancias.
//!
//! Un solo `#[test]` (el backend de pruebas de Slint se inicia una vez por
//! proceso), con hilos y tiempo real para que corra el bucle de eventos.

use std::sync::Arc;
use std::time::{Duration, Instant};

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use lcore::{Core, Options};
use shard_ui::backend::Backend;
use shard_ui::controller::{Actions, Controller};
use shard_ui::doctor::{CoreDoctor, Doctor};
use shard_ui::first_run::{CoreFirstRun, FirstRun};
use shard_ui::settings::Settings;
use shard_ui::settings_backend::CoreSettings;
use shard_ui::{DoctorState, FirstRunState, LoadState, MainWindow, Route, SettingsState};
use slint::ComponentHandle;
use slint::platform::{PointerEventButton, WindowEvent};

struct NoActions;

impl Actions for NoActions {
    fn open_instance(&self, _: &str) {}
    fn play(&self, _: &str) {}
    fn new_instance(&self) {}
    fn show_trash(&self) {}
}

/// Corre el bucle de eventos hasta que `cond` se cumpla (20 s como mucho).
fn wait(what: &str, cond: impl Fn() -> bool + 'static) {
    let deadline = Instant::now() + Duration::from_secs(20);
    let timer = slint::Timer::default();
    let result = std::rc::Rc::new(std::cell::Cell::new(None));
    let result_from_tick = result.clone();
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(10), move || {
        if result_from_tick.get().is_some() {
            return;
        }
        let ready = cond();
        if ready || Instant::now() >= deadline {
            result_from_tick.set(Some(ready));
            let _ = slint::quit_event_loop();
        }
    });
    // El backend de pruebas puede consumir una salida de una espera anterior.
    // Solo termina esta espera cuando su propio temporizador da un resultado.
    while result.get().is_none() && Instant::now() < deadline {
        slint::run_event_loop().unwrap();
    }
    assert_eq!(result.get(), Some(true), "no llegó: {what}");
}

/// Deja correr el bucle `ms` (temporizadores de guardado).
fn pause(ms: u64) {
    let start = Instant::now();
    wait("pausa", move || start.elapsed() >= Duration::from_millis(ms));
}

fn by_label(ui: &MainWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(ui, label).next().unwrap_or_else(|| panic!("no hay «{label}»"))
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

#[test]
fn fase5_con_el_nucleo_real() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions { mock_time: false, threading: true, renderer_name: Some("software".into()) },
    )))
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("datos");
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let core = rt
        .block_on(Core::open(Options {
            data_dir: Some(data.clone()),
            offline: true,
            secrets: Some(Arc::new(lcore::secrets::MemoryStore::default())),
            ..Default::default()
        }))
        .unwrap();
    rt.block_on(shard_ui::fixtures::seed(&core)).unwrap();
    let backend = Backend::new(core.clone(), rt.handle().clone());

    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    let ctl = Controller::new(&ui, "es", Box::new(|| 0), Box::new(NoActions));
    ui.set_animate_background(false);
    shard_ui::backend::install(ctl.clone());
    let settings = Settings::new(
        &ui,
        ctl.clone(),
        Box::new(CoreSettings::new(backend.clone(), "es")),
        16384,
        &data.display().to_string(),
        "0.1.0",
    );
    let doctor = Doctor::new(&ui, ctl.clone(), Box::new(CoreDoctor { backend: backend.clone(), lang: "es".into() }));
    let first_run = FirstRun::new(&ui, ctl.clone(), Box::new(CoreFirstRun { backend: backend.clone(), lang: "es".into() }), None);
    ui.show().unwrap();
    backend.load();
    let w = ui.as_weak();
    wait("instancias", move || w.unwrap().global::<shard_ui::InstancesState>().get_total() == 5);

    // --- Ajustes: lo que se toca llega a settings.json y ui.json. ----------
    ui.set_route(Route::Settings);
    ui.invoke_route_changed(Route::Settings);
    let w = ui.as_weak();
    wait("ajustes", move || w.unwrap().global::<SettingsState>().get_state() == LoadState::Ready);
    let w = ui.as_weak();
    wait("java", move || w.unwrap().global::<SettingsState>().get_java_state() != LoadState::Loading);
    click(&by_label(&ui, "Salir"));
    click(&by_id(&ui, "switch")[0]);
    click(&by_label(&ui, "English"));
    let c = core.clone();
    let h = rt.handle().clone();
    wait("settings.json", move || {
        let s = h.block_on(c.settings());
        s.language.as_deref() == Some("en") && s.close_behavior == lcore::settings::CloseBehavior::Quit && s.allow_multiple_games
    });
    assert_eq!(ctl.lang(), "en");
    // Argumentos de JVM: se guardan tras la espera.
    settings.scroll_to(900.0);
    pause(100);
    click(&by_id(&ui, "field")[0]);
    for ch in " -Xss4M".chars() {
        let t: slint::SharedString = ch.to_string().into();
        ui.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
        ui.window().dispatch_event(WindowEvent::KeyReleased { text: t });
    }
    let c = core.clone();
    let h = rt.handle().clone();
    wait("jvm", move || h.block_on(c.settings()).default_jvm_args.iter().any(|a| a == "-Xss4M"));
    // Estilo: ui.json.
    settings.scroll_to(300.0);
    pause(100);
    click(&by_label(&ui, "Aurora"));
    let ui_json = data.join("ui.json");
    let path = ui_json.clone();
    wait("ui.json", move || std::fs::read_to_string(&path).is_ok_and(|t| t.contains("\"aurora\"")));
    assert_eq!(shard_ui::prefs::load(&data).style, shard_ui::theme::Style::Aurora);
    // Volver a español para los textos de abajo.
    settings.scroll_to(0.0);
    pause(100);
    click(&by_label(&ui, "Español"));
    let c = core.clone();
    let h = rt.handle().clone();
    wait("es", move || h.block_on(c.settings()).language.as_deref() == Some("es"));

    // --- Doctor sobre una instancia que nunca se jugó. ----------------------
    // Una salida encolada no debe cancelar la espera del informe.
    slint::quit_event_loop().unwrap();
    doctor.open("survival-amigos");
    let w = ui.as_weak();
    wait("informe", move || w.unwrap().global::<DoctorState>().get_state() != LoadState::Loading);
    let st = ui.global::<DoctorState>();
    assert_eq!(st.get_state(), LoadState::Ready, "{}", st.get_error_detail());
    assert_eq!(st.get_instance_name(), "Survival con amigos");
    click(&by_label(&ui, "Cerrar"));
    assert!(!st.get_open());

    // --- Primer arranque: importar un .minecraft de verdad (Linux/macOS:
    // la carpeta sale de $HOME). ---------------------------------------------
    #[cfg(unix)]
    {
        let home = dir.path().join("home");
        let mc = shard_minecraft(&home);
        // SAFETY: un solo hilo de test toca el entorno, antes de pedir nada.
        unsafe { std::env::set_var("HOME", &home) };
        assert_eq!(lcore::import::minecraft::default_source_root(), mc);
        first_run.show(1);
        let w = ui.as_weak();
        wait("orígenes", move || w.unwrap().global::<FirstRunState>().get_sources_state() == LoadState::Ready);
        let fr = ui.global::<FirstRunState>();
        assert_eq!(fr.get_minecraft_path(), mc.display().to_string());
        assert_eq!(fr.get_roxy_path(), "", "no hay Roxy");
        let check = ElementHandle::find_by_accessible_label(&ui, "Launcher oficial (.minecraft)")
            .find(|e| e.accessible_checked().is_some())
            .expect("casilla");
        click(&check);
        let w = ui.as_weak();
        wait("plan", move || w.unwrap().global::<FirstRunState>().get_minecraft_plan_state() != LoadState::Loading);
        assert_eq!(fr.get_minecraft_plan_state(), LoadState::Ready, "{}", fr.get_minecraft_plan_error());
        click(&by_id(&ui, "filled")[0]);
        let w = ui.as_weak();
        wait("importado", move || w.unwrap().global::<FirstRunState>().get_import_phase() == 2);
        assert_eq!(fr.get_import_error(), "", "{}", fr.get_import_error_detail());
        assert_eq!(fr.get_import_count(), 1);
        let w = ui.as_weak();
        wait("6 instancias", move || w.unwrap().global::<shard_ui::InstancesState>().get_total() == 6);
        // Saltar al final: ui.json queda con el asistente hecho.
        click(&by_label(&ui, "Saltar asistente"));
        let path = ui_json.clone();
        wait("first_run_done", move || std::fs::read_to_string(&path).is_ok_and(|t| t.contains("\"first_run_done\": true")));
        assert!(!ui.get_first_run());
    }
    drop(first_run);
}

/// Un `.minecraft` mínimo: una versión vanilla instalada y un mundo.
#[cfg(unix)]
fn shard_minecraft(home: &std::path::Path) -> std::path::PathBuf {
    let mc = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/minecraft")
    } else {
        home.join(".minecraft")
    };
    let v = mc.join("versions/1.21.4");
    std::fs::create_dir_all(&v).unwrap();
    std::fs::write(v.join("1.21.4.json"), r#"{"id":"1.21.4","type":"release"}"#).unwrap();
    let w = mc.join("saves/Mundo de prueba");
    std::fs::create_dir_all(&w).unwrap();
    std::fs::write(w.join("level.dat"), b"x").unwrap();
    std::fs::write(mc.join("options.txt"), b"lang:es_es\n").unwrap();
    mc
}
