//! Cuentas contra el núcleo real (`lcore` en una carpeta temporal, sin red y
//! con secretos en memoria): clics y teclado de verdad sobre la UI, el
//! trabajo en tokio (`accounts_backend`) y los resultados de vuelta por
//! `invoke_from_event_loop`. Comprueba lo que queda en `accounts.json`.
//!
//! Un solo `#[test]` (el backend de pruebas de Slint se inicia una vez por
//! proceso), con hilos y tiempo real para que corra el bucle de eventos.

use std::sync::Arc;
use std::time::{Duration, Instant};

use i_slint_backend_testing::ElementHandle;
use lcore::{Core, Options};
use shard_ui::accounts::Accounts;
use shard_ui::accounts_backend::{CoreAccounts, CoreSkins};
use shard_ui::backend::Backend;
use shard_ui::controller::{Actions, Controller};
use shard_ui::skins::Skins;
use shard_ui::{AccountsState, LoginPhase, MainWindow, Route, SkinsState};
use slint::{ComponentHandle, Model};
use slint::platform::{PointerEventButton, WindowEvent};

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
    slint::run_event_loop().unwrap();
    assert!(ok.get(), "no llegó: {what}");
}

fn by_label(ui: &MainWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(ui, label).next().unwrap_or_else(|| panic!("no hay «{label}»"))
}

fn click(ui: &MainWindow, label: &str) {
    by_label(ui, label).mock_single_click(PointerEventButton::Left);
}

/// Abre el alta Shard y deja que el diálogo se cree y tome el foco.
fn open_shard(ui: &MainWindow) {
    ui.global::<AccountsState>().invoke_add_shard();
    let w = ui.as_weak();
    wait("diálogo Shard", move || w.unwrap().global::<AccountsState>().get_offline_open());
}

fn type_line(ui: &MainWindow, text: &str) {
    for ch in text.chars().chain(['\n']) {
        let t: slint::SharedString = ch.to_string().into();
        ui.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
        ui.window().dispatch_event(WindowEvent::KeyReleased { text: t });
    }
}

#[test]
fn cuentas_con_el_nucleo_real() {
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
    let backend = Backend { core: core.clone(), rt: rt.handle().clone() };

    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    ui.set_animate_background(false);
    let ctl = Controller::new(&ui, "es", Box::new(|| 0), Box::new(NoActions));
    let clock = Instant::now();
    let acc = Accounts::new(&ui, ctl.clone(), Box::new(CoreAccounts::new(backend.clone(), "es")), Box::new(move || clock.elapsed().as_millis() as u64));
    let _skins = Skins::new(&ui, ctl, acc.clone(), Box::new(CoreSkins { backend: backend.clone(), lang: "es".into() }));
    ui.show().unwrap();

    // Sin cuentas.
    let s = ui.as_weak();
    wait("lista vacía", move || !s.unwrap().global::<AccountsState>().get_loading());
    assert!(!ui.get_account().present);

    // Alta offline desde el panel, con teclado.
    for name in ["Steve_01", "Alex"] {
        ui.global::<AccountsState>().set_panel_open(true);
        click(&ui, "Añadir cuenta offline");
        type_line(&ui, name);
        let (w, n) = (ui.as_weak(), name.to_owned());
        wait("cuenta añadida", move || w.unwrap().get_toast_message() == format!("Cuenta «{n}» añadida"));
    }
    let s = ui.as_weak();
    wait("dos cuentas", move || s.unwrap().global::<AccountsState>().get_items().row_count() == 2);
    let file = rt.block_on(core.accounts()).unwrap();
    let names: Vec<_> = file.accounts.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Steve_01", "Alex"]);
    let active = file.active_account().unwrap().name.clone();
    assert_eq!(ui.get_account().name, active.as_str());

    // Nombre inválido: el núcleo no se toca.
    ui.global::<AccountsState>().set_panel_open(true);
    click(&ui, "Añadir cuenta offline");
    type_line(&ui, "no vale");
    assert!(ui.global::<AccountsState>().get_offline_open());
    click(&ui, "Cancelar");
    assert_eq!(rt.block_on(core.accounts()).unwrap().accounts.len(), 2);

    // Cambiar a la otra desde el panel.
    let other = if active == "Alex" { "Steve_01" } else { "Alex" };
    ui.global::<AccountsState>().set_panel_open(true);
    click(&ui, other);
    let (w, o) = (ui.as_weak(), other.to_owned());
    wait("cuenta activa", move || w.unwrap().get_account().name == o.as_str());
    assert_eq!(rt.block_on(core.accounts()).unwrap().active_account().unwrap().name, other);

    // Skins con una cuenta offline: el estado vacío de offline.
    ui.set_route(Route::Skins);
    assert!(ui.global::<SkinsState>().get_has_account());
    assert!(!ui.global::<SkinsState>().get_microsoft());

    // Login de Microsoft sin red: termina en error, no se queda colgado.
    ui.global::<AccountsState>().invoke_add_microsoft();
    let s = ui.as_weak();
    wait("error del login", move || s.unwrap().global::<AccountsState>().get_login_phase() == LoginPhase::Failed);
    assert!(!ui.global::<AccountsState>().get_login_error().is_empty());
    click(&ui, "Cancelar");
    assert!(!ui.global::<AccountsState>().get_login_open());

    // Shard sin red: error en el diálogo, sin cuenta nueva.
    ui.global::<AccountsState>().set_panel_open(true);
    click(&ui, "Cuenta Shard");
    type_line(&ui, "prueba_ui");
    type_line(&ui, "x");
    let s = ui.as_weak();
    wait("error de Shard", move || !s.unwrap().global::<AccountsState>().get_offline_error().is_empty());
    assert!(ui.global::<AccountsState>().get_offline_open());
    click(&ui, "Cancelar");
    assert_eq!(rt.block_on(core.accounts()).unwrap().accounts.len(), 2);

    // Quitar con confirmación.
    ui.global::<AccountsState>().set_panel_open(true);
    let remove = ElementHandle::find_by_accessible_label(&ui, "Quitar").next().unwrap();
    remove.mock_single_click(PointerEventButton::Left);
    let confirm = ElementHandle::find_by_accessible_label(&ui, "Quitar").last().unwrap();
    confirm.mock_single_click(PointerEventButton::Left);
    let s = ui.as_weak();
    wait("una cuenta", move || s.unwrap().global::<AccountsState>().get_items().row_count() == 1);
    assert_eq!(rt.block_on(core.accounts()).unwrap().accounts.len(), 1);
}

/// Login real contra el servidor Shard de la comunidad (red): solo a mano,
/// con la contraseña de `prueba_ui` en un archivo (nunca en el repo ni en logs):
///
///   SHARD_PRUEBA_PASSWORD_FILE=/ruta/0600 cargo test -p shard-ui --test nucleo_cuentas -- --ignored
#[test]
#[ignore = "red y contraseña de prueba_ui"]
fn login_shard_real() {
    let Some(file) = std::env::var_os("SHARD_PRUEBA_PASSWORD_FILE") else {
        panic!("falta SHARD_PRUEBA_PASSWORD_FILE");
    };
    let password = std::fs::read_to_string(file).unwrap().trim_end_matches(['\n', '\r']).to_owned();
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions { mock_time: false, threading: true, renderer_name: Some("software".into()) },
    )))
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let core = rt
        .block_on(Core::open(Options {
            data_dir: Some(dir.path().to_path_buf()),
            secrets: Some(Arc::new(lcore::secrets::MemoryStore::default())),
            ..Default::default()
        }))
        .unwrap();
    let backend = Backend { core: core.clone(), rt: rt.handle().clone() };
    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    let ctl = Controller::new(&ui, "es", Box::new(|| 0), Box::new(NoActions));
    let acc = Accounts::new(&ui, ctl.clone(), Box::new(CoreAccounts::new(backend.clone(), "es")), Box::new(|| 0));
    let _skins = Skins::new(&ui, ctl, acc, Box::new(CoreSkins { backend, lang: "es".into() }));
    ui.show().unwrap();

    // Contraseña mala primero: el error de credenciales.
    open_shard(&ui);
    type_line(&ui, "prueba_ui");
    type_line(&ui, "no-es-la-clave");
    assert!(ui.global::<AccountsState>().get_offline_busy(), "no se envió el formulario");
    let s = ui.as_weak();
    wait("credenciales malas", move || !s.unwrap().global::<AccountsState>().get_offline_error().is_empty());
    assert_eq!(
        ui.global::<AccountsState>().get_offline_error(),
        "Usuario o contraseña incorrectos para la cuenta Shard (Yggdrasil)."
    );
    click(&ui, "Cancelar");

    // La buena.
    open_shard(&ui);
    type_line(&ui, "prueba_ui");
    type_line(&ui, &password);
    let w = ui.as_weak();
    wait("cuenta Shard añadida", move || w.unwrap().get_toast_message() == "Cuenta «prueba_ui» añadida");
    let w = ui.as_weak();
    wait("en el raíl", move || w.unwrap().get_account().name == "prueba_ui");
    let file = rt.block_on(core.accounts()).unwrap();
    let a = file.accounts.iter().find(|a| a.name == "prueba_ui").expect("cuenta guardada");
    assert_eq!(a.kind, lcore::auth::AccountKind::Shard);
    // Y sirve para jugar: el núcleo saca una sesión con ella.
    let session = rt.block_on(core.session(Some(&a.id))).expect("sesión Shard");
    assert_eq!(session.name, "prueba_ui");
    ui.set_route(Route::Skins);
    assert!(ui.global::<SkinsState>().get_shard());
}
