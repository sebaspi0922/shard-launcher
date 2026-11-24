//! Pruebas de interacción de cuentas y skins (fase 2): clics y teclado de
//! verdad sobre la UI; las acciones del núcleo se apuntan
//! (`fixtures::Recorder`) y el test entrega sus resultados al controlador,
//! como haría `accounts_backend`. Las aserciones son sobre lo que se pide al
//! núcleo y sobre el estado de la UI, no sobre píxeles.
//!
//! Un solo `#[test]`: el backend de pruebas de Slint se inicia una vez por
//! proceso.

use std::rc::Rc;

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use shard_ui::accounts::{Accounts, Login};
use shard_ui::controller::{Actions, Controller};
use shard_ui::errors::Failure;
use shard_ui::fixtures::{self, Recorder};
use shard_ui::skins::{Apply, Skins};
use shard_ui::{AccountsState, LoginPhase, MainWindow, Route, SkinsState};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, Model};

struct NoActions;

impl Actions for NoActions {
    fn open_instance(&self, _: &str) {}
    fn play(&self, _: &str) {}
    fn new_instance(&self) {}
    fn show_trash(&self) {}
}

struct App {
    ui: MainWindow,
    acc: Rc<Accounts>,
    skins: Rc<Skins>,
    rec: Recorder,
    /// Reloj del login (ms), que el test adelanta.
    clock: Rc<std::cell::Cell<u64>>,
}

fn app(lang: &str) -> App {
    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    ui.set_animate_background(false);
    let ctl = Controller::new(&ui, lang, Box::new(|| fixtures::NOW), Box::new(NoActions));
    ctl.set_instances(fixtures::sample_instances());
    let rec = Recorder::default();
    let clock = Rc::new(std::cell::Cell::new(0));
    let c = clock.clone();
    let acc = Accounts::new(&ui, ctl.clone(), Box::new(rec.clone()), Box::new(move || c.get()));
    let skins = Skins::new(&ui, ctl, acc.clone(), Box::new(rec.clone()));
    assert_eq!(rec.take(), ["load"]);
    acc.set_accounts(Ok(fixtures::sample_accounts()));
    // La cuenta activa es de Microsoft: Skins pide su perfil.
    assert_eq!(rec.take(), ["profile a1"]);
    ui.show().unwrap();
    settle();
    App { ui, acc, skins, rec, clock }
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

fn by_label(ui: &MainWindow, label: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_accessible_label(ui, label).collect()
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

/// Píxeles del visor principal (sin el resto de la pantalla).
fn stage_pixels(ui: &MainWindow) -> Vec<u8> {
    let snap = ui.window().take_snapshot().unwrap();
    let e = &by_id(ui, "stage")[0];
    let (p, sz) = (e.absolute_position(), e.size());
    let w = snap.width() as usize;
    let mut out = Vec::new();
    // Sin la etiqueta del modelo (arriba) ni la ayuda (abajo).
    for y in (p.y + 60.0) as usize..(p.y + sz.height - 40.0) as usize {
        let row = &snap.as_bytes()[(y * w + p.x as usize) * 4..(y * w + (p.x + sz.width) as usize) * 4];
        out.extend_from_slice(row);
    }
    out
}

fn toast(ui: &MainWindow) -> String {
    ui.get_toast_message().to_string()
}

fn panel_cambia_de_cuenta_y_quita_con_confirmacion() {
    let a = app("es");
    // El avatar del raíl abre el panel con las dos cuentas.
    click(&by_label(&a.ui, "NadiaK")[0]);
    assert!(a.ui.global::<AccountsState>().get_panel_open());
    assert_eq!(by_id(&a.ui, "account").len(), 2);

    // Clic en la activa: nada; en la otra: se activa.
    click(&by_id(&a.ui, "account")[0]);
    assert!(a.rec.take().is_empty());
    click(&by_id(&a.ui, "account")[1]);
    assert_eq!(a.rec.take(), ["activate a2"]);
    a.acc.activated(Ok(()));
    assert_eq!(a.rec.take(), ["load"]);
    let mut list = fixtures::sample_accounts();
    list[0].active = false;
    list[1].active = true;
    a.acc.set_accounts(Ok(list));
    settle();
    assert_eq!(a.ui.get_account().name, "Pruebas_01");

    // Quitar pide confirmación; «Cancelar» no quita nada.
    click(&by_id(&a.ui, "account-remove")[0]);
    assert_eq!(a.ui.global::<AccountsState>().get_remove_name(), "NadiaK");
    assert_eq!(by_id(&a.ui, "confirm-remove").len(), 1);
    click(&by_label(&a.ui, "Cancelar")[0]);
    assert!(by_id(&a.ui, "confirm-remove").is_empty());
    assert!(a.rec.take().is_empty());
    click(&by_id(&a.ui, "account-remove")[0]);
    click(&by_label(&a.ui, "Quitar").last().unwrap().clone());
    assert_eq!(a.rec.take(), ["remove a1 NadiaK"]);
    a.acc.removed("NadiaK", Ok(()));
    assert_eq!(toast(&a.ui), "Cuenta «NadiaK» quitada");
    assert_eq!(a.rec.take(), ["load"]);

    // Un error al cambiar de cuenta avisa.
    a.acc.activated(Err(Failure { kind: "io".into(), message: "x".into(), detail: "y".into() }));
    assert!(a.ui.get_toast_error());
}

fn alta_offline_valida_el_nombre() {
    let a = app("en");
    click(&by_label(&a.ui, "NadiaK")[0]);
    click(&by_label(&a.ui, "Add offline account")[0]);
    assert!(!a.ui.global::<AccountsState>().get_panel_open());
    assert!(a.ui.global::<AccountsState>().get_offline_open());
    // El campo tiene el foco: se escribe directamente.
    type_text(&a.ui, "ab");
    click(&by_label(&a.ui, "Add")[0]);
    assert!(a.rec.take().is_empty(), "nombre corto: no se pide nada");
    assert!(!by_label(&a.ui, "Use 3–16 characters: letters, digits or underscore.").is_empty());
    type_text(&a.ui, "c_9");
    // Intro también envía.
    a.ui.window().dispatch_event(WindowEvent::KeyPressed { text: "\n".into() });
    a.ui.window().dispatch_event(WindowEvent::KeyReleased { text: "\n".into() });
    settle();
    assert_eq!(a.rec.take(), ["offline abc_9"]);
    assert!(a.ui.global::<AccountsState>().get_offline_busy());

    // Error del núcleo: se queda abierto y lo muestra.
    a.acc.offline_added(Err(Failure { kind: "io".into(), message: "Disk error".into(), detail: "io: denied".into() }));
    settle();
    assert!(a.ui.global::<AccountsState>().get_offline_open());
    assert!(!by_label(&a.ui, "Disk error").is_empty());

    // Un alta que termina con el diálogo ya cerrado (y otro abierto sin
    // enviar) no lo toca: solo refresca la lista.
    click(&by_label(&a.ui, "Add")[0]);
    a.rec.take();
    click(&by_label(&a.ui, "Cancel")[0]);
    a.ui.global::<AccountsState>().invoke_add_offline();
    settle();
    a.acc.offline_added(Ok("abc_9".into()));
    assert!(a.ui.global::<AccountsState>().get_offline_open());
    assert_eq!(a.rec.take(), ["load"]);
    a.ui.global::<AccountsState>().set_offline_open(false);
    a.ui.global::<AccountsState>().invoke_add_offline();
    settle();
    type_text(&a.ui, "abc_9");
    click(&by_label(&a.ui, "Add")[0]);
    assert_eq!(a.rec.take(), ["offline abc_9"]);
    a.acc.offline_added(Ok("abc_9".into()));
    settle();
    assert!(!a.ui.global::<AccountsState>().get_offline_open());
    assert_eq!(toast(&a.ui), "Account “abc_9” added");
    assert_eq!(a.rec.take(), ["load"]);
}

fn alta_shard_con_usuario_y_contrasena() {
    let a = app("es");
    let st = a.ui.global::<AccountsState>();
    click(&by_label(&a.ui, "NadiaK")[0]);
    click(&by_label(&a.ui, "Cuenta Shard")[0]);
    assert!(st.get_offline_open() && st.get_credentials_shard());
    // Sin contraseña no se puede enviar.
    type_text(&a.ui, "prueba_ui");
    assert_eq!(by_id(&a.ui, "filled")[0].accessible_enabled(), Some(false));
    // Intro en el usuario pasa a la contraseña.
    type_text(&a.ui, "\n");
    assert!(a.rec.take().is_empty());
    type_text(&a.ui, "s3creta!");
    assert_eq!(by_id(&a.ui, "filled")[0].accessible_enabled(), Some(true));
    type_text(&a.ui, "\n");
    assert_eq!(a.rec.take(), ["shard prueba_ui (8 caracteres)"]);
    // Mientras inicia sesión no se puede cancelar ni reenviar.
    assert!(st.get_offline_busy());
    assert_eq!(by_id(&a.ui, "cancel")[0].accessible_enabled(), Some(false));

    // Credenciales malas: el texto de errors.dart y el diálogo sigue abierto.
    let bad = lcore::Error::from(lcore::auth::AuthError::InvalidCredentials);
    a.acc.offline_added(Err(Failure::new("es", &bad)));
    settle();
    assert!(st.get_offline_open() && !st.get_offline_busy());
    assert!(!by_label(&a.ui, "Usuario o contraseña incorrectos para la cuenta Shard (Yggdrasil).").is_empty());

    click(&by_id(&a.ui, "filled")[0]);
    assert_eq!(a.rec.take(), ["shard prueba_ui (8 caracteres)"]);
    a.acc.offline_added(Ok("prueba_ui".into()));
    settle();
    assert!(!st.get_offline_open());
    assert_eq!(toast(&a.ui), "Cuenta «prueba_ui» añadida");
    assert_eq!(a.rec.take(), ["load"], "refresca la lista al terminar");

    // En la lista sale como «Cuenta Shard» y Skins explica dónde cambiarla.
    let mut list = fixtures::sample_accounts();
    list[0].active = false;
    list.push(shard_ui::accounts::Account { id: "shard1".into(), name: "prueba_ui".into(), microsoft: false, shard: true, skin_url: None, active: true });
    a.acc.set_accounts(Ok(list));
    a.ui.set_route(Route::Skins);
    settle();
    assert!(a.rec.take().is_empty(), "sin perfil de Microsoft que pedir");
    assert!(a.ui.global::<SkinsState>().get_shard());
    assert!(!by_label(&a.ui, "Gestiona las skins y capas desde el sitio donde registraste tu cuenta Shard.").is_empty());
    click(&by_id(&a.ui, "chip")[0]);
    let items = st.get_items();
    assert!(items.row_data(2).unwrap().shard);
}

fn login_de_microsoft_con_codigo_y_caducidad() {
    let a = app("es");
    let st = a.ui.global::<AccountsState>();
    click(&by_label(&a.ui, "NadiaK")[0]);
    click(&by_label(&a.ui, "Añadir cuenta de Microsoft")[0]);
    assert_eq!(a.rec.take(), ["login 1"]);
    assert!(st.get_login_open());
    assert_eq!(st.get_login_phase(), LoginPhase::Requesting);

    a.acc.login_code(1, fixtures::sample_device_code());
    settle();
    assert_eq!(st.get_login_phase(), LoginPhase::Code);
    assert_eq!(st.get_user_code(), "K7QF-M2XP");
    assert_eq!(st.get_verification_label(), "www.microsoft.com/link");
    assert_eq!(st.get_expires_label(), "15:00");

    click(&by_label(&a.ui, "Abrir navegador")[0]);
    assert_eq!(a.rec.take(), ["open https://www.microsoft.com/link"]);
    click(&by_label(&a.ui, "Copiar código")[0]);
    assert_eq!(toast(&a.ui), "Copiado");

    // Pasa el tiempo: la cuenta atrás baja y al llegar a 0 caduca.
    a.clock.set(61_000);
    i_slint_backend_testing::testing_backend::mock_elapsed_time(1000);
    assert_eq!(st.get_expires_label(), "13:59");
    a.clock.set(900_000);
    i_slint_backend_testing::testing_backend::mock_elapsed_time(1000);
    assert_eq!(st.get_login_phase(), LoginPhase::Expired);
    assert!(a.rec.take().is_empty(), "caducar no aborta: el núcleo puede estar terminando");

    // «Pedir otro código» empieza otro intento; un resultado del viejo no cuenta.
    click(&by_label(&a.ui, "Pedir otro código")[0]);
    assert_eq!(a.rec.take(), ["login 2"]);
    a.acc.login_done(1, "Viejo");
    assert!(st.get_login_open());
    a.acc.login_done(2, "NadiaK");
    settle();
    assert!(!st.get_login_open());
    assert_eq!(toast(&a.ui), "Sesión iniciada como NadiaK");
    assert_eq!(a.rec.take(), ["load"]);

    // Cancelar cierra y aborta.
    st.invoke_add_microsoft();
    settle();
    a.rec.take();
    click(&by_label(&a.ui, "Cancelar")[0]);
    assert!(!st.get_login_open());
    assert_eq!(a.rec.take(), ["cancel-login"]);
    assert_eq!(a.acc.login_state(), Login::Closed);

    // Error del núcleo: nota de error y «Reintentar».
    st.invoke_add_microsoft();
    settle();
    let attempt: u64 = a.rec.take()[0].trim_start_matches("login ").parse().unwrap();
    a.acc.login_failed(attempt, Failure { kind: "network".into(), message: "Sin red".into(), detail: "network: x".into() });
    settle();
    assert_eq!(st.get_login_phase(), LoginPhase::Failed);
    assert!(!by_label(&a.ui, "Sin red").is_empty());
    assert!(!by_label(&a.ui, "Reintentar").is_empty());
}

fn skins_cambiar_modelo_capa_y_restablecer() {
    let a = app("es");
    a.ui.set_route(Route::Skins);
    settle();
    let st = a.ui.global::<SkinsState>();
    a.skins.set_profile("a1", a.skins.profile_seq(), Ok(fixtures::sample_profile()));
    let mut asked = a.rec.take();
    asked.sort();
    assert_eq!(asked, [format!("texture {}", fixtures::CAPE_1_URL), format!("texture {}", fixtures::CAPE_2_URL), format!("texture {}", fixtures::SKIN_URL)]);
    for url in [fixtures::SKIN_URL, fixtures::CAPE_1_URL, fixtures::CAPE_2_URL] {
        a.skins.texture_loaded(url, Some(fixtures::sample_texture(url)));
    }
    settle();
    assert!(st.get_has_texture());
    assert!(st.get_slim());
    assert!(!st.get_can_apply());
    assert_eq!(by_id(&a.ui, "stage").len(), 3);

    // Cambiar a «Clásica» repinta el visor con brazos de 4 y deja aplicar.
    let before = stage_pixels(&a.ui);
    click(&by_label(&a.ui, "Clásica")[0]);
    assert_ne!(before, stage_pixels(&a.ui), "el visor no cambió de modelo");
    click(&by_label(&a.ui, "Delgada")[0]);
    // Cambiar a «Clásica» deja aplicar: misma textura, otros brazos.
    click(&by_label(&a.ui, "Clásica")[0]);
    assert!(!st.get_slim() && st.get_can_apply());
    click(&by_id(&a.ui, "apply")[0]);
    assert_eq!(a.rec.take(), [format!("apply a1 {:?}", Apply::Model { url: fixtures::SKIN_URL.into(), slim: false })]);
    assert!(st.get_busy());
    a.skins.changed("a1", false, Ok(fixtures::sample_profile()));
    settle();
    assert_eq!(toast(&a.ui), "Skin actualizada");
    assert!(st.get_slim(), "vuelve al modelo de la skin actual");
    assert_eq!(a.rec.take(), ["load"], "refresca la cara de la cuenta");

    // URL: el campo habilita «Aplicar».
    click(&by_label(&a.ui, "URL")[0]);
    assert!(st.get_from_url());
    click(&by_id(&a.ui, "url")[0]);
    type_text(&a.ui, "https://x.y/s.png");
    assert!(st.get_can_apply());
    click(&by_id(&a.ui, "apply")[0]);
    assert_eq!(a.rec.take(), [format!("apply a1 {:?}", Apply::Url { url: "https://x.y/s.png".into(), slim: true })]);
    a.skins.changed("a1", false, Err(Failure { kind: "network".into(), message: "Sin red".into(), detail: String::new() }));
    settle();
    assert_eq!(toast(&a.ui), "Sin red");
    assert!(a.ui.get_toast_error());

    // Capas: «Aniversario» y luego «Sin capa».
    let capes = st.get_capes();
    let labels: Vec<String> = (0..capes.row_count()).map(|i| capes.row_data(i).unwrap().label.to_string()).collect();
    assert_eq!(labels, ["Sin capa", "Pionera", "Aniversario"]);
    click(&by_label(&a.ui, "Pionera")[0]);
    assert!(a.rec.take().is_empty(), "la puesta no se vuelve a pedir");
    click(&by_label(&a.ui, "Aniversario")[0]);
    assert_eq!(a.rec.take(), ["cape a1 c2"]);
    click(&by_label(&a.ui, "Sin capa")[0]);
    assert!(a.rec.take().is_empty(), "una a la vez");
    let mut p = fixtures::sample_profile();
    p.capes[0].active = false;
    p.capes[1].active = true;
    a.skins.cape_done("a1", Ok(p));
    settle();
    click(&by_label(&a.ui, "Sin capa")[0]);
    assert_eq!(a.rec.take(), ["cape a1 -"]);
    a.skins.cape_done("a1", Ok(fixtures::sample_profile()));

    // Restablecer pide confirmación.
    click(&by_id(&a.ui, "reset")[0]);
    assert_eq!(by_id(&a.ui, "confirm-reset").len(), 1);
    click(&by_label(&a.ui, "Restablecer").last().unwrap().clone());
    assert_eq!(a.rec.take(), ["reset a1"]);
    a.skins.changed("a1", true, Ok(fixtures::sample_profile()));
    settle();
    assert_eq!(toast(&a.ui), "Skin restablecida");
    assert_eq!(a.rec.take(), ["load"]);

    // Arrastrar gira el modelo; doble clic lo devuelve al ángulo inicial.
    let before = a.ui.window().take_snapshot().unwrap();
    let st0 = &by_id(&a.ui, "stage")[0];
    let (p, sz) = (st0.absolute_position(), st0.size());
    let at = |dx: f32| slint::LogicalPosition::new(p.x + sz.width / 2.0 + dx, p.y + sz.height / 2.0);
    let w = a.ui.window();
    w.dispatch_event(WindowEvent::PointerPressed { position: at(0.0), button: PointerEventButton::Left });
    for dx in [10.0, 30.0, 60.0] {
        w.dispatch_event(WindowEvent::PointerMoved { position: at(dx) });
    }
    w.dispatch_event(WindowEvent::PointerReleased { position: at(60.0), button: PointerEventButton::Left });
    settle();
    let turned = w.take_snapshot().unwrap();
    assert_ne!(before.as_bytes(), turned.as_bytes(), "arrastrar no giró el modelo");
    for _ in 0..2 {
        w.dispatch_event(WindowEvent::PointerPressed { position: at(0.0), button: PointerEventButton::Left });
        w.dispatch_event(WindowEvent::PointerReleased { position: at(0.0), button: PointerEventButton::Left });
    }
    settle();
    assert_eq!(before.as_bytes(), w.take_snapshot().unwrap().as_bytes(), "doble clic no lo centró");

    // PNG elegido: se lee y, si no es una skin, avisa y no deja aplicar.
    click(&by_label(&a.ui, "Archivo PNG")[0]);
    click(&by_id(&a.ui, "pick")[0]);
    assert_eq!(a.rec.take(), ["pick"]);
    a.skins.file_picked(Some("/tmp/mala.png".into()));
    assert_eq!(a.rec.take(), ["local /tmp/mala.png"]);
    a.skins.local_loaded("/tmp/mala.png", None);
    settle();
    assert!(st.get_preview_error() && !st.get_can_apply());
    assert_eq!(st.get_file_name(), "mala.png");
    a.skins.file_picked(Some("/tmp/buena.png".into()));
    a.rec.take();
    a.skins.local_loaded("/tmp/buena.png", Some(shard_ui::skin::sample_skin()));
    settle();
    assert!(st.get_preview() && st.get_can_apply());
    click(&by_id(&a.ui, "apply")[0]);
    assert_eq!(a.rec.take(), [format!("apply a1 {:?}", Apply::Upload { path: "/tmp/buena.png".into(), slim: true })]);
}

fn skins_resultados_viejos_y_reintentos() {
    let a = app("es");
    a.ui.set_route(Route::Skins);
    let st = a.ui.global::<SkinsState>();
    // Una textura que no se pudo bajar se vuelve a pedir con el perfil.
    a.skins.set_profile("a1", a.skins.profile_seq(), Ok(fixtures::sample_profile()));
    a.rec.take();
    a.skins.texture_loaded(fixtures::SKIN_URL, None);
    a.skins.set_profile("a1", a.skins.profile_seq(), Ok(fixtures::sample_profile()));
    assert_eq!(a.rec.take(), [format!("texture {}", fixtures::SKIN_URL)]);
    a.skins.texture_loaded(fixtures::SKIN_URL, Some(fixtures::sample_texture(fixtures::SKIN_URL)));
    settle();

    // Aplicar en a1 y cambiar de cuenta antes de que conteste.
    click(&by_label(&a.ui, "Clásica")[0]);
    click(&by_id(&a.ui, "apply")[0]);
    a.rec.take();
    let other = shard_ui::accounts::Account { id: "a3".into(), name: "Otra".into(), microsoft: true, shard: false, skin_url: None, active: true };
    let mut list = fixtures::sample_accounts();
    list[0].active = false;
    list.push(other);
    a.acc.set_accounts(Ok(list));
    assert_eq!(a.rec.take(), ["profile a3"]);
    assert!(!st.get_busy());
    a.ui.set_toast_message("".into());
    a.skins.changed("a1", false, Ok(fixtures::sample_profile()));
    a.skins.cape_done("a1", Ok(fixtures::sample_profile()));
    settle();
    // Lo de a1 no pisa a a3 (sigue cargando su perfil) ni avisa.
    assert_eq!(st.get_state(), shard_ui::LoadState::Loading);
    assert_eq!(toast(&a.ui), "");
    assert!(a.rec.take().is_empty());

    // A → B → A con una capa en camino: la lectura de A pedida al volver
    // llega después del resultado de la capa y no lo pisa.
    a.acc.set_accounts(Ok(fixtures::sample_accounts()));
    assert_eq!(a.rec.take(), ["profile a1"]);
    let stale = a.skins.profile_seq();
    let mut newer = fixtures::sample_profile();
    newer.capes[0].active = false;
    newer.capes[1].active = true;
    a.skins.cape_done("a1", Ok(newer));
    a.skins.set_profile("a1", stale, Ok(fixtures::sample_profile()));
    settle();
    let capes = st.get_capes();
    assert!(capes.row_data(2).unwrap().selected, "manda el resultado de la capa, no la lectura vieja");
}

fn skins_sin_cuenta_y_offline() {
    let a = app("en");
    a.ui.set_route(Route::Skins);
    a.acc.set_accounts(Ok(vec![]));
    settle();
    let st = a.ui.global::<SkinsState>();
    assert!(!st.get_has_account());
    click(&by_label(&a.ui, "Add account")[0]);
    assert!(a.ui.global::<AccountsState>().get_panel_open());
    a.ui.global::<AccountsState>().set_panel_open(false);

    a.acc.set_accounts(Ok(fixtures::offline_accounts()));
    settle();
    assert!(st.get_has_account() && !st.get_microsoft());
    a.rec.take();
    click(&by_id(&a.ui, "empty-action")[0]);
    assert!(a.ui.global::<AccountsState>().get_login_open());
    assert_eq!(a.rec.take(), ["login 1"]);
}

#[test]
fn cuentas_y_skins() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    panel_cambia_de_cuenta_y_quita_con_confirmacion();
    alta_offline_valida_el_nombre();
    alta_shard_con_usuario_y_contrasena();
    login_de_microsoft_con_codigo_y_caducidad();
    skins_cambiar_modelo_capa_y_restablecer();
    skins_resultados_viejos_y_reintentos();
    skins_sin_cuenta_y_offline();
}
