//! Pruebas de interacción de la fase 5 (Ajustes, Doctor, primer arranque):
//! clics y teclado de verdad sobre la UI; lo que se pide fuera de la UI se
//! apunta (`fixtures::Recorder`) y el test entrega los resultados al
//! controlador, como haría el backend. Las aserciones son sobre lo pedido y
//! sobre el estado de la UI, no sobre píxeles.
//!
//! Un solo `#[test]`: el backend de pruebas de Slint se inicia una vez por
//! proceso.

use std::rc::Rc;

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use shard_ui::accounts::Accounts;
use shard_ui::controller::{Actions, Controller};
use shard_ui::curseforge::KeyActions;
use shard_ui::doctor::Doctor;
use shard_ui::errors::Failure;
use shard_ui::first_run::FirstRun;
use shard_ui::fixtures::{self, Recorder};
use shard_ui::settings::Settings;
use shard_ui::theme::Style;
use shard_ui::{AccountsState, DoctorState, FirstRunState, LoadState, MainWindow, Route, SettingsState};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, Model};

/// Apunta `diagnose` para comprobar que Doctor se abre desde Inicio.
struct Act(Recorder);

impl Actions for Act {
    fn open_instance(&self, _: &str) {}
    fn play(&self, _: &str) {}
    fn new_instance(&self) {}
    fn show_trash(&self) {}
    fn diagnose(&self, id: &str) {
        self.0.0.borrow_mut().push(format!("diagnose-from-ui {id}"));
        shard_ui::doctor::with(|d| d.open(id));
    }
}

/// Clave de CurseForge (hasta que `Recorder` implemente `KeyActions`).
struct Keys(Recorder);

impl KeyActions for Keys {
    fn check(&self) {}
    fn set(&self, _: &str) {
        self.0.0.borrow_mut().push("cf-set".into());
    }
    fn clear(&self) {
        self.0.0.borrow_mut().push("cf-clear".into());
    }
}

struct App {
    keys: Rc<shard_ui::curseforge::Keys>,
    ui: MainWindow,
    ctl: Rc<Controller>,
    settings: Rc<Settings>,
    doctor: Rc<Doctor>,
    first_run: Rc<FirstRun>,
    rec: Recorder,
}

fn app(lang: &str) -> App {
    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    let rec = Recorder::default();
    let ctl = Controller::new(&ui, lang, Box::new(|| fixtures::NOW), Box::new(Act(rec.clone())));
    ctl.set_instances(fixtures::sample_instances());
    ui.set_animate_background(false);
    let acc = Accounts::new(&ui, ctl.clone(), Box::new(rec.clone()), Box::new(|| 0));
    acc.set_accounts(Ok(fixtures::sample_accounts()));
    let settings = Settings::new(&ui, ctl.clone(), Box::new(rec.clone()), fixtures::TOTAL_MB, fixtures::DATA_DIR, fixtures::CORE_VERSION);
    let doctor = Doctor::new(&ui, ctl.clone(), Box::new(rec.clone()));
    let first_run = FirstRun::new(&ui, ctl.clone(), Box::new(rec.clone()), Some(lang.to_owned()));
    let keys = shard_ui::curseforge::Keys::new(&ui, ctl.clone(), Box::new(Keys(rec.clone())));
    ui.show().unwrap();
    settle();
    rec.take();
    App { keys, ui, ctl, settings, doctor, first_run, rec }
}

fn settle() {
    i_slint_backend_testing::testing_backend::mock_elapsed_time(600);
}

fn by_id(ui: &MainWindow, id: &str) -> Vec<ElementHandle> {
    let id = id.to_owned();
    ui.root_element()
        .query_descendants()
        .match_predicate(move |e| e.accessible_id().as_deref() == Some(id.as_str()))
        .find_all()
}

fn by_label(ui: &MainWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(ui, label).next().unwrap_or_else(|| panic!("no hay «{label}»"))
}

/// Botón principal del paso del asistente.
fn primary(ui: &MainWindow) -> ElementHandle {
    by_id(ui, "filled").into_iter().next().expect("botón principal")
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
}

/// Entra en Ajustes como lo haría el raíl y entrega los datos de ejemplo.
fn open_settings(a: &App) {
    a.ui.set_route(Route::Settings);
    a.ui.invoke_route_changed(Route::Settings);
    settle();
    let asked = a.rec.take();
    for want in ["settings", "autostart?", "java"] {
        assert!(asked.iter().any(|x| x == want), "al entrar se pide {want}: {asked:?}");
    }
    a.settings.loaded(a.settings.last_load(), Ok(fixtures::sample_values(Some("es"))));
    a.settings.java_listed(Ok(fixtures::sample_javas()));
    a.settings.autostart_loaded(false);
    settle();
}

fn scroll(a: &App, y: f32) {
    a.settings.scroll_to(y);
    settle();
}

/// El último guardado de ajustes apuntado.
fn last_save(rec: &Recorder) -> String {
    rec.take().into_iter().rfind(|x| x.starts_with("save ")).unwrap_or_default()
}

fn ajustes(a: &App) {
    // Reabrir antes de que llegue la primera lectura: su respuesta no vale.
    a.settings.open();
    let first = a.settings.last_load();
    a.settings.open();
    a.rec.take();
    a.settings.loaded(first, Ok(fixtures::sample_values(Some("es"))));
    assert_eq!(a.ui.global::<SettingsState>().get_state(), LoadState::Loading);
    open_settings(a);
    let st = a.ui.global::<SettingsState>();
    assert_eq!(st.get_state(), LoadState::Ready);
    assert_eq!(st.get_language(), 1);

    // Idioma: clic en «English» → se guarda y la UI cambia al momento.
    click(&by_label(&a.ui, "English"));
    assert!(last_save(&a.rec).contains("language: Some(\"en\")"));
    assert_eq!(a.ctl.lang(), "en");
    // Los nombres de los estilos siguen al idioma.
    assert_eq!(st.get_styles().row_data(2).unwrap().name, "Landscape");
    click(&by_label(&a.ui, "Español"));
    assert!(last_save(&a.rec).contains("language: Some(\"es\")"));
    assert_eq!(a.ctl.lang(), "es");

    // Al cerrar: «Segundo plano».
    click(&by_label(&a.ui, "Segundo plano"));
    assert!(last_save(&a.rec).contains("close: Tray"));
    assert_eq!(st.get_close_behavior(), 1);
    // Con el guardado sin confirmar, una lectura vieja no pisa lo editado.
    a.settings.loaded(a.settings.last_load(), Ok(fixtures::sample_values(Some("es"))));
    assert_eq!(st.get_close_behavior(), 1, "lectura vieja ignorada");
    // Aunque el guardado ya se haya confirmado: la lectura se pidió antes.
    a.settings.saved(u64::MAX, Ok(fixtures::sample_values(Some("es"))));
    a.settings.loaded(a.settings.last_load(), Ok(fixtures::sample_values(Some("es"))));
    assert_eq!(st.get_close_behavior(), 1, "lectura pedida antes del cambio");
    // Reabrir pide otra: la respuesta a la anterior ya no vale.
    let old = a.settings.last_load();
    a.settings.open();
    a.rec.take();
    let mut stale = fixtures::sample_values(Some("es"));
    stale.close = lcore::settings::CloseBehavior::Ask;
    a.settings.loaded(old, Ok(stale.clone()));
    assert_eq!(st.get_close_behavior(), 1, "respuesta a una lectura anterior");
    stale.close = lcore::settings::CloseBehavior::Tray;
    a.settings.loaded(a.settings.last_load(), Ok(stale));
    assert_eq!(st.get_close_behavior(), 1);

    // Interruptores: varias partidas y autoarranque.
    let switches = by_id(&a.ui, "switch");
    click(&switches[0]);
    assert!(last_save(&a.rec).contains("multiple: true"));
    click(&switches[1]);
    assert_eq!(a.rec.take(), ["autostart true"]);
    assert!(st.get_autostart());

    // Apariencia: estilo «Paisaje» y acento preset (se guarda ui.json).
    scroll(a, 300.0);
    click(&by_label(&a.ui, "Paisaje"));
    let saved = a.rec.take();
    assert!(saved.iter().any(|x| x.starts_with("prefs ") && x.contains("\"style\":\"landscape\"")), "{saved:?}");
    assert_eq!(a.ctl.appearance.prefs().style, Style::Landscape);
    assert!(st.get_tunable(), "con paisaje se puede desenfocar");
    let swatches = by_id(&a.ui, "swatch");
    assert_eq!(swatches.len(), 8);
    click(&swatches[1]);
    let saved = a.rec.take();
    assert!(saved.iter().any(|x| x.contains("\"accent\":\"preset\"") && x.contains(&format!("{}", 0xFF4FA8FFu32))), "{saved:?}");
    assert_eq!(st.get_preset(), 1);
    // Volver al estilo por defecto para el resto.
    click(&by_label(&a.ui, "Grafito"));
    a.rec.take();

    // Desenfoque con el teclado: el cambio se ve ya y se guarda tras 400 ms.
    let sliders = by_id(&a.ui, "slider");
    assert!(!sliders.is_empty());
    // «Elegir imagen…» pide el selector.
    click(&by_label(&a.ui, "Elegir imagen…"));
    assert_eq!(a.rec.take(), ["pick-background"]);

    // Juego: memoria con el teclado y argumentos de JVM (con espera).
    scroll(a, 900.0);
    // El deslizador (no el título de la fila, que tiene el mismo nombre).
    let mem = ElementHandle::find_by_accessible_label(&a.ui, "Memoria por defecto")
        .find(|e| e.accessible_id().as_deref() == Some("slider"))
        .expect("deslizador de memoria");
    mem.invoke_accessible_increment_action();
    // Sin esperar no se guarda nada; a los 500 ms, una vez.
    assert!(a.rec.take().iter().all(|x| !x.starts_with("save ")));
    settle();
    let s = last_save(&a.rec);
    assert!(s.contains("memory_mb:"), "{s}");
    assert_ne!(st.get_memory_mb(), 4096, "el valor cambió");
    let field = &by_id(&a.ui, "field")[0];
    field.mock_single_click(PointerEventButton::Left);
    type_text(&a.ui, " -Xss2M");
    settle();
    let s = last_save(&a.rec);
    assert!(s.contains("jvm_args: [\"-XX:+UseG1GC\", \"-Xss2M\"]"), "{s}");

    // Java: instalar 17 → progreso → instalada (aviso y nueva lista).
    scroll(a, 1300.0);
    let installs = by_id(&a.ui, "install");
    assert_eq!(installs.len(), 4);
    click(&installs[1]);
    assert_eq!(a.rec.take(), ["install-java 17"]);
    assert!(st.get_java_installing());
    a.settings.on_event(&lcore::events::Event::Progress {
        task_id: 1,
        stage: "java".into(),
        done: 10 * 1024 * 1024,
        total: 40 * 1024 * 1024,
        unit: lcore::events::ProgressUnit::Bytes,
    });
    assert_eq!(st.get_install_description(), "Instalando Java 17: 10 MB de 40 MB");
    let mut javas = fixtures::sample_javas();
    javas[1].source = lcore::java::JavaSource::Managed;
    a.settings.java_installed(17, Ok(javas[1].clone()));
    assert!(!st.get_java_installing());
    assert_eq!(a.rec.take(), ["java"], "se vuelve a buscar");
    assert_eq!(a.ui.get_toast_message(), "Java 17 instalada (17.0.13)");
    // Un fallo deja un aviso de error.
    a.settings.java_installed(25, Err(Failure { kind: "network".into(), message: "Sin conexión".into(), detail: "x".into() }));
    assert!(a.ui.get_toast_error());

    // Integraciones (diálogo de la clave de F4): quitar pide confirmación.
    a.keys.state(true);
    scroll(a, 1731.0);
    click(&by_label(&a.ui, "Quitar"));
    assert!(a.rec.take().is_empty(), "sin confirmar no se borra");
    let quitar: Vec<_> = ElementHandle::find_by_accessible_label(&a.ui, "Quitar").collect();
    click(quitar.last().unwrap());
    assert_eq!(a.rec.take(), ["cf-clear"]);
    click(&by_label(&a.ui, "Cambiar clave"));
    assert!(a.ui.global::<shard_ui::CurseforgeKey>().get_dialog_open());
    a.ui.global::<shard_ui::CurseforgeKey>().set_dialog_open(false);
    settle();

    // Acerca de: copiar la carpeta de datos y buscar versión nueva.
    click(&by_label(&a.ui, "Copiar"));
    assert_eq!(a.rec.take(), [format!("copy {}", fixtures::DATA_DIR)]);
    assert_eq!(a.ui.get_toast_message(), "Copiado");
    click(&by_label(&a.ui, "Buscar"));
    assert_eq!(a.rec.take(), ["check-update"]);
    a.ctl.update_checking();
    assert!(st.get_update_checking());
    assert_eq!(st.get_update_description(), "Buscando…");
    a.ctl.update_checked(None, true);
    assert_eq!(st.get_update_description(), "Tienes la última versión (0.1.0).");

    // Un guardado fallido avisa.
    a.settings.saved(99, Err(Failure { kind: "io".into(), message: "x".into(), detail: "x".into() }));
    a.settings.flush();
    let seq_err = Failure { kind: "io".into(), message: "x".into(), detail: "disco".into() };
    a.rec.take();
    // El último `seq` es el del `flush`: su error sí avisa.
    a.settings.saved(u64::MAX, Err(seq_err.clone()));
    assert!(a.ui.get_toast_message() != "No se pudieron guardar los ajustes", "un resultado viejo no avisa");

    // Al salir de Ajustes se guarda lo pendiente.
    a.ui.set_route(Route::Home);
    a.ui.invoke_route_changed(Route::Home);
    settle();
}

fn doctor(a: &App) {
    // Desde «Diagnosticar» (Actions::diagnose) se abre con el nombre.
    a.ui.global::<shard_ui::HomeState>().invoke_diagnose("survival-amigos".into());
    settle();
    let st = a.ui.global::<DoctorState>();
    assert!(st.get_open());
    assert_eq!(st.get_instance_name(), "Survival con amigos");
    assert_eq!(st.get_state(), LoadState::Loading);
    let asked = a.rec.take();
    assert_eq!(asked, ["diagnose-from-ui survival-amigos", "diagnose survival-amigos 1"]);
    // Un informe viejo no pisa al actual.
    a.doctor.report(0, Ok(Default::default()));
    assert_eq!(st.get_state(), LoadState::Loading);
    a.doctor.report(1, Ok(fixtures::sample_report()));
    settle();
    assert_eq!(st.get_state(), LoadState::Ready);
    assert_eq!((st.get_critical(), st.get_warnings(), st.get_notes()), (1, 1, 1));
    assert_eq!(st.get_diagnoses().row_count(), 3);
    assert!(by_id(&a.ui, "diag").len() >= 2, "las que caben sin desplazar");

    // Abrir el crash report.
    click(&by_label(&a.ui, "Abrir crash report"));
    let asked = a.rec.take();
    assert!(asked[0].starts_with("open-path /home/user/.local/share/shard/instances/survival-amigos"), "{asked:?}");

    // Aplicar el primer arreglo: los demás esperan, luego aviso y nuevo análisis.
    let apply = by_id(&a.ui, "apply-fix");
    assert_eq!(apply.len(), 2, "el tercero no tiene arreglo");
    click(&apply[0]);
    assert_eq!(a.rec.take(), ["fix survival-amigos mod_dependency"]);
    assert!(st.get_applying());
    assert!(st.get_diagnoses().row_data(0).unwrap().applying);
    click(&apply[1]);
    assert!(a.rec.take().is_empty(), "un arreglo a la vez");
    a.doctor.fixed("survival-amigos", Ok(String::new()));
    assert_eq!(a.ui.get_toast_message(), "Arreglo aplicado");
    assert_eq!(a.rec.take(), ["diagnose survival-amigos 2"]);
    // Sin problemas: el estado de sano.
    a.doctor.report(2, Ok(Default::default()));
    settle();
    assert_eq!(st.get_diagnoses().row_count(), 0);
    assert!(ElementHandle::find_by_accessible_label(&a.ui, "No vemos ningún problema").next().is_some());

    // Volver a analizar y error.
    click(&by_label(&a.ui, "Volver a analizar"));
    assert_eq!(a.rec.take(), ["diagnose survival-amigos 3"]);
    a.doctor.report(3, Err(Failure { kind: "instance_not_found".into(), message: "No existe".into(), detail: "d".into() }));
    assert_eq!(st.get_state(), LoadState::Failed);
    assert_eq!(st.get_error_message(), "No existe");

    // Un arreglo de A termina con B abierta: B queda libre y no se vuelve
    // a analizar nada.
    a.doctor.report(3, Ok(fixtures::sample_report()));
    settle();
    click(&by_id(&a.ui, "apply-fix")[0]);
    assert_eq!(a.rec.take(), ["fix survival-amigos mod_dependency"]);
    a.doctor.open("create-aeronautica");
    assert_eq!(a.rec.take(), ["diagnose create-aeronautica 4"]);
    a.doctor.report(4, Err(Failure { kind: "io".into(), message: "Falló".into(), detail: "d".into() }));
    assert!(st.get_applying(), "el de A sigue en marcha");
    a.doctor.fixed("survival-amigos", Ok(String::new()));
    assert!(!st.get_applying());
    assert_eq!(st.get_state(), LoadState::Failed, "el error de B se queda");
    // Igual si el arreglo de A falla.
    click(&by_label(&a.ui, "Volver a analizar"));
    a.rec.take();
    a.doctor.report(5, Ok(fixtures::sample_report()));
    settle();
    a.doctor.open("survival-amigos");
    a.doctor.report(6, Ok(fixtures::sample_report()));
    settle();
    click(&by_id(&a.ui, "apply-fix")[0]);
    a.doctor.open("create-aeronautica");
    a.doctor.report(7, Err(Failure { kind: "io".into(), message: "Falló".into(), detail: "d".into() }));
    a.rec.take();
    a.doctor.fixed("survival-amigos", Err(Failure { kind: "io".into(), message: "No".into(), detail: "d".into() }));
    assert!(!st.get_applying());
    assert_eq!(st.get_state(), LoadState::Failed, "el error de B se queda");
    assert!(a.rec.take().is_empty(), "no se analiza B por el arreglo de A");

    // Cerrar.
    click(&by_id(&a.ui, "filled").into_iter().find(|e| e.accessible_label().as_deref() == Some("Cerrar")).unwrap());
    assert!(!st.get_open());
}

fn primer_arranque(a: &App) {
    a.first_run.show(0);
    settle();
    assert!(a.ui.get_first_run());
    assert_eq!(a.rec.take(), ["detect"]);
    let st = a.ui.global::<FirstRunState>();

    // Idioma: «English» se guarda y cambia la UI; «Del sistema» = None.
    click(&by_label(&a.ui, "English"));
    assert_eq!(a.rec.take(), ["language Some(\"en\")"]);
    assert_eq!(a.ctl.lang(), "en");
    assert_eq!(st.get_language(), 2);
    click(&by_label(&a.ui, "Español"));
    assert_eq!(a.rec.take(), ["language Some(\"es\")"]);

    // Empezar → importar.
    click(&primary(&a.ui));
    assert_eq!(st.get_step(), 1);
    a.first_run.sources(Ok(fixtures::sample_sources()));
    assert_eq!(a.rec.take(), ["plan-roxy"], "Roxy viene marcado: se calcula su plan");
    a.first_run.roxy_plan(Ok(fixtures::sample_roxy_plan()));
    settle();
    assert_eq!(st.get_roxy_count(), 3, "la omitida no cuenta");
    // Marcar .minecraft calcula su plan.
    let mc = ElementHandle::find_by_accessible_label(&a.ui, "Launcher oficial (.minecraft)")
        .find(|e| e.accessible_checked().is_some())
        .expect("casilla de .minecraft");
    click(&mc);
    assert!(st.get_minecraft_selected());
    assert_eq!(a.rec.take(), ["plan-minecraft"]);
    a.first_run.minecraft_plan(Ok(fixtures::sample_minecraft_plan()));
    settle();
    assert_eq!(st.get_minecraft_tags().row_count(), 5, "solo las secciones presentes");

    // Importar: progreso por origen y resultado.
    click(&primary(&a.ui));
    assert_eq!(a.rec.take(), ["import roxy=true minecraft=true"]);
    assert_eq!(st.get_import_phase(), 1);
    a.first_run.importing("roxy");
    assert_eq!(st.get_roxy_status(), 1);
    a.first_run.on_event(&lcore::events::Event::Progress {
        task_id: 1,
        stage: "import".into(),
        done: 120,
        total: 4000,
        unit: lcore::events::ProgressUnit::Files,
    });
    assert_eq!((st.get_import_done(), st.get_import_total()), (120, 4000));
    a.first_run.importing("minecraft");
    assert_eq!((st.get_roxy_status(), st.get_minecraft_status()), (2, 1));
    a.first_run.imported(3, None);
    settle();
    assert_eq!(st.get_import_phase(), 2);
    assert_eq!(st.get_import_count(), 3);
    assert_eq!(a.rec.take(), ["reload"]);
    assert!(ElementHandle::find_by_accessible_label(&a.ui, "3 instancias importadas").next().is_some());

    // Siguiente → cuenta: las tarjetas abren los diálogos de cuentas.
    click(&primary(&a.ui));
    assert_eq!(st.get_step(), 2);
    assert_eq!(by_id(&a.ui, "choice").len(), 2, "Microsoft y offline (sin cuenta Shard)");
    click(&by_label(&a.ui, "Añadir cuenta offline"));
    assert!(a.ui.global::<AccountsState>().get_offline_open());
    a.ui.global::<AccountsState>().set_offline_open(false);
    settle();
    assert_eq!(by_id(&a.ui, "account").len(), 2, "las cuentas añadidas");

    // Siguiente → listo, con el resumen, y «Ir a Shard».
    click(&primary(&a.ui));
    assert_eq!(st.get_step(), 3);
    assert_eq!(a.ui.global::<shard_ui::InstancesState>().get_total(), 5);
    click(&primary(&a.ui));
    assert!(!a.ui.get_first_run());
    assert_eq!(a.ui.get_route(), Route::Home);
    assert_eq!(a.ctl.appearance.prefs().first_run_done, Some(true));
    let saved = a.rec.take();
    assert!(saved.iter().any(|x| x.starts_with("prefs ") && x.contains("\"first_run_done\":true")), "{saved:?}");

    // Saltar desde cualquier paso hace lo mismo.
    a.first_run.show(1);
    settle();
    click(&by_label(&a.ui, "Saltar asistente"));
    assert!(!a.ui.get_first_run());
}

#[test]
fn fase5() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions { mock_time: true, threading: false, renderer_name: Some("software".into()) },
    )))
    .expect("plataforma");
    let a = app("es");
    ajustes(&a);
    doctor(&a);
    primer_arranque(&a);
}
