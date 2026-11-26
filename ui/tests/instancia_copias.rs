//! F3 copias
//! Pruebas de interacción de la pestaña Copias (backups_tab.dart): clics y
//! teclado de verdad sobre la UI; lo que se pide al núcleo se apunta
//! (`fixtures::Recorder`) y el test entrega los resultados al controlador,
//! como haría `CoreBackups`. Aserciones sobre lo pedido y sobre el estado de
//! la UI, no sobre píxeles.
//!
//! Un solo `#[test]`: el backend de pruebas de Slint se inicia una vez por
//! proceso.

use std::rc::Rc;

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use lcore::backup::BackupEntry;
use lcore::instance::BackupSettings;
use shard_ui::backups::{Backups, Step};
use shard_ui::controller::{Actions, Controller};
use shard_ui::errors::Failure;
use shard_ui::fixtures::{self, Recorder};
use shard_ui::instance::InstanceDetail;
use shard_ui::instances::Phase;
use shard_ui::{BackupsState, InstanceTab, MainWindow, Route};
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, Model};

const ID: &str = "survival-amigos";

struct NoActions;

impl Actions for NoActions {
    fn open_instance(&self, _: &str) {}
    fn play(&self, _: &str) {}
    fn new_instance(&self) {}
    fn show_trash(&self) {}
}

struct App {
    ui: MainWindow,
    ctl: Rc<Controller>,
    detail: Rc<InstanceDetail>,
    b: Rc<Backups>,
    rec: Recorder,
}

impl App {
    fn st(&self) -> BackupsState<'_> {
        self.ui.global::<BackupsState>()
    }

    fn item(&self, i: usize) -> shard_ui::BackupItem {
        self.st().get_items().row_data(i).expect("copia")
    }
}

/// Detalle de «Survival con amigos» en la pestaña Copias con las copias y
/// los ajustes de ejemplo ya cargados.
fn app(lang: &str) -> App {
    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    ui.set_animate_background(false);
    let ctl = Controller::new(&ui, lang, Box::new(|| fixtures::NOW), Box::new(NoActions));
    ctl.set_instances(fixtures::sample_instances());
    ui.set_animate_background(false);
    let rec = Recorder::default();
    let detail = InstanceDetail::new(&ui, ctl.clone(), Box::new(rec.clone()));
    let b = Backups::new(&ui, detail.clone(), Box::new(rec.clone()));
    b.set_utc_offset(|_| fixtures::backups::UTC_OFFSET);
    ui.set_route(Route::Instances);
    ui.show().unwrap();
    detail.open(ID, Some(InstanceTab::Backups));
    assert_eq!(rec.take(), [format!("load-instance {ID}"), format!("backups {ID}")]);
    detail.loaded(ID, Ok(fixtures::detail::sample_detail(ID).unwrap()));
    b.listed(ID, Ok(fixtures::backups::sample_backups()));
    settle();
    App { ui, ctl, detail, b, rec }
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

fn key(ui: &MainWindow, text: slint::SharedString) {
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    ui.window().dispatch_event(WindowEvent::KeyReleased { text });
}

fn type_text(ui: &MainWindow, text: &str) {
    for ch in text.chars() {
        key(ui, ch.to_string().into());
    }
    settle();
}

fn failure(message: &str) -> Failure {
    Failure { kind: "io".into(), message: message.into(), detail: "io: detalle".into() }
}

fn entry(id: &str, pinned: bool, label: Option<&str>) -> BackupEntry {
    let mut e = fixtures::backups::sample_backups().into_iter().find(|e| e.id == id).unwrap();
    e.pinned = pinned;
    e.label = label.map(Into::into);
    e
}

fn lista_resumen_y_ajustes() {
    let a = app("es");
    let st = a.st();
    assert_eq!(st.get_items().row_count(), 4);
    assert_eq!(st.get_total(), "4,3 GB");
    let first = a.item(0);
    assert_eq!((first.title.as_str(), first.date.as_str(), first.size.as_str()), ("Copia automática", "11 oct 2026 15:30", "1,2 GB"));
    assert_eq!(first.ago, 3 * 3600);
    assert!(first.automatic && !first.pinned);
    let second = a.item(1);
    assert_eq!(second.title, "Antes de la actualización 1.21.4");
    assert!(second.pinned && !second.automatic);
    assert_eq!(a.item(3).date, "20 sept 2026 18:30");
    assert!(!by_label(&a.ui, "Fijada").is_empty());
    // Ajustes de la instancia: a diario, se conservan 5.
    assert!(st.get_settings_ready() && st.get_auto());
    assert_eq!((st.get_interval(), st.get_keep()), (24, 5));
    assert_eq!(by_id(&a.ui, "interval").iter().filter(|e| e.accessible_checked() == Some(true)).count(), 1);

    // Al volver a la pestaña se vuelve a leer la lista (sin vaciarla).
    a.detail.select_tab(InstanceTab::Worlds);
    a.detail.select_tab(InstanceTab::Backups);
    assert_eq!(a.rec.take(), [format!("backups {ID}")]);
    assert_eq!(st.get_items().row_count(), 4);

    // Vacía: estado vacío; con error: la nota de error.
    a.b.listed(ID, Ok(vec![]));
    settle();
    assert_eq!(by_id(&a.ui, "empty").len(), 1);
    assert!(by_id(&a.ui, "list").is_empty());
    a.b.listed(ID, Err(failure("No se pudo leer")));
    settle();
    assert_eq!(st.get_error(), "No se pudo leer");
    assert!(by_id(&a.ui, "empty").is_empty());
    // Un resultado de otra instancia no cuenta.
    a.b.listed("otra", Ok(fixtures::backups::sample_backups()));
    assert_eq!(st.get_items().row_count(), 0);
}

fn fijar_y_desfijar() {
    let a = app("es");
    click(&by_id(&a.ui, "pin")[0]);
    assert_eq!(a.rec.take(), [format!("backup-pin {ID} backup-4 true")]);
    a.b.updated(ID, Ok(entry("backup-4", true, None)));
    assert!(a.item(0).pinned);
    // La fijada ofrece «Dejar de fijar».
    click(&by_label(&a.ui, "Dejar de fijar")[0]);
    assert_eq!(a.rec.take(), [format!("backup-pin {ID} backup-4 false")]);
    // Un error se avisa y la lista no cambia.
    a.b.updated(ID, Err(failure("Sin permiso")));
    assert_eq!(a.ui.get_toast_message(), "Sin permiso");
    assert!(a.ui.get_toast_error());
    assert!(a.item(0).pinned);
}

fn crear_con_etiqueta_fijada_y_progreso() {
    let a = app("es");
    let st = a.st();
    click(&by_id(&a.ui, "create-play")[0]);
    assert!(st.get_create_open());
    // El campo de la etiqueta tiene el foco al abrir.
    type_text(&a.ui, "  Antes de Create ");
    click(&by_id(&a.ui, "pin-on-create")[0]);
    let dialog = &by_id(&a.ui, "create-dialog")[0];
    let create = by_id(&a.ui, "filled").into_iter().find(|e| e.absolute_position().y > dialog.absolute_position().y).unwrap();
    click(&create);
    assert_eq!(a.rec.take(), [format!("backup-create {ID} Some(\"Antes de Create\") pinned=true")]);
    assert!(st.get_create_busy());
    // Mientras se crea no se cierra ni se pide otra.
    click(&by_id(&a.ui, "cancel")[0]);
    key(&a.ui, "\n".into());
    assert!(st.get_create_open());
    assert!(a.rec.take().is_empty());
    // Progreso de la etapa `backup` (la de `restore` no cuenta aquí).
    a.b.progress("restore", 1, 2, false);
    assert!(st.get_progress() < 0.0);
    a.b.progress("backup", 32, 64, false);
    assert_eq!((st.get_progress(), st.get_progress_text().as_str()), (0.5, "32 / 64"));
    assert_eq!(by_id(&a.ui, "progress").len(), 1);

    // Error: se ve en el diálogo, que sigue abierto.
    a.b.created(ID, Err(failure("Disco lleno")));
    settle();
    assert!(st.get_create_open() && !st.get_create_busy());
    assert_eq!(st.get_create_error(), "Disco lleno");

    // Reintento con Intro: hecho, aviso y la lista se vuelve a leer.
    by_id(&a.ui, "field")[0].mock_single_click(PointerEventButton::Left);
    key(&a.ui, "\n".into());
    settle();
    assert_eq!(a.rec.take(), [format!("backup-create {ID} Some(\"Antes de Create\") pinned=true")]);
    a.b.created(ID, Ok(entry("backup-4", true, Some("Antes de Create"))));
    settle();
    assert!(!st.get_create_open());
    assert_eq!(a.ui.get_toast_message(), "Copia creada");
    assert_eq!(a.rec.take(), [format!("backups {ID}")]);

    // Sin etiqueta = `None`; Cancelar cierra sin pedir nada.
    click(&by_id(&a.ui, "create-play")[0]);
    assert!(st.get_create_error().is_empty());
    click(&by_id(&a.ui, "cancel")[0]);
    assert!(!st.get_create_open());
    assert!(a.rec.take().is_empty());
}

fn restaurar_solo_mundos_con_copia_previa() {
    let a = app("es");
    let st = a.st();
    click(&by_id(&a.ui, "restore")[1]);
    assert!(st.get_restore_open());
    assert_eq!(st.get_restore_item().id, "backup-3");
    assert!(st.get_restore_safety() && !st.get_restore_worlds());
    assert_eq!(by_id(&a.ui, "filled-play")[0].accessible_label().unwrap(), "Restaurar todo");
    // Solo los mundos.
    click(&by_id(&a.ui, "mode")[1]);
    assert!(st.get_restore_worlds());
    let go = &by_id(&a.ui, "filled-play")[0];
    assert_eq!(go.accessible_label().unwrap(), "Restaurar mundos");
    click(go);
    assert_eq!(a.rec.take(), [format!("backup-restore {ID} backup-3 worlds safety=Some(\"Antes de restaurar\")")]);
    assert_eq!(st.get_restore_step(), 1);
    // En curso: no se cierra, no se cambia de modo.
    click(&by_id(&a.ui, "cancel")[0]);
    click(&by_id(&a.ui, "mode")[0]);
    assert!(st.get_restore_open() && st.get_restore_worlds());
    a.b.progress("backup", 10, 20, false);
    assert_eq!(st.get_progress(), 0.5);
    a.b.restore_step(ID, Step::Restore);
    assert_eq!(st.get_restore_step(), 2);
    assert!(st.get_progress() < 0.0);
    a.b.progress("restore", 3, 4, false);
    assert_eq!(st.get_progress_text(), "3 / 4");
    a.b.restored(ID, Ok(()));
    settle();
    assert!(!st.get_restore_open());
    assert_eq!(a.ui.get_toast_message(), "Copia «Antes de la actualización 1.21.4» restaurada");
    // El detalle (mundos, contenido) y la lista (la copia previa) se releen.
    assert_eq!(a.rec.take(), [format!("load-instance {ID}"), format!("backups {ID}")]);

    // Todo, sin copia previa; un error se queda en el diálogo.
    click(&by_id(&a.ui, "restore")[0]);
    assert!(!st.get_restore_worlds() && st.get_restore_safety());
    click(&by_id(&a.ui, "safety")[0]);
    assert!(!st.get_restore_safety());
    click(&by_id(&a.ui, "filled-play")[0]);
    assert_eq!(a.rec.take(), [format!("backup-restore {ID} backup-4 full safety=None")]);
    assert_eq!(st.get_restore_step(), 2);
    a.b.restored(ID, Err(failure("El juego está abierto")));
    settle();
    assert!(st.get_restore_open());
    assert_eq!((st.get_restore_step(), st.get_restore_error().as_str()), (0, "El juego está abierto"));
    click(&by_id(&a.ui, "cancel")[0]);
    assert!(!st.get_restore_open());

    // Con el juego abierto no se restaura ni se crea.
    a.ctl.set_phase(ID, Phase::Running);
    settle();
    assert!(st.get_locked());
    assert_eq!(by_id(&a.ui, "restore")[0].accessible_enabled(), Some(false));
    click(&by_id(&a.ui, "restore")[0]);
    click(&by_id(&a.ui, "create-play")[0]);
    assert!(!st.get_restore_open() && !st.get_create_open());
    a.ctl.set_phase(ID, Phase::Idle);
    assert!(!st.get_locked());
}

fn menu_cambiar_etiqueta_y_eliminar() {
    let a = app("es");
    let st = a.st();
    // ⋮ de la copia con etiqueta: el campo trae la etiqueta.
    click(&by_id(&a.ui, "dots")[1]);
    assert!(st.get_menu_open());
    assert_eq!(by_id(&a.ui, "backup-menu").len(), 1);
    click(&by_label(&a.ui, "Cambiar etiqueta").into_iter().last().unwrap());
    assert!(st.get_rename_open() && !st.get_menu_open());
    // El texto empieza seleccionado: lo escrito lo sustituye.
    type_text(&a.ui, "Mundo nuevo\n");
    assert!(!st.get_rename_open());
    assert_eq!(a.rec.take(), [format!("backup-label {ID} backup-3 Some(\"Mundo nuevo\")")]);
    a.b.updated(ID, Ok(entry("backup-3", true, Some("Mundo nuevo"))));
    assert_eq!(a.item(1).title, "Mundo nuevo");

    // Vacía = quitar la etiqueta (la automática sin etiqueta empieza vacía).
    click(&by_id(&a.ui, "dots")[0]);
    click(&by_label(&a.ui, "Cambiar etiqueta").into_iter().last().unwrap());
    let field = &by_id(&a.ui, "field")[0];
    assert_eq!(field.accessible_value().unwrap(), "");
    key(&a.ui, "\n".into());
    settle();
    assert_eq!(a.rec.take(), [format!("backup-label {ID} backup-4 None")]);

    // Escape cierra el menú.
    click(&by_id(&a.ui, "dots")[0]);
    key(&a.ui, Key::Escape.into());
    settle();
    assert!(!st.get_menu_open());

    // Eliminar pide confirmación; cancelar no borra.
    click(&by_id(&a.ui, "dots")[3]);
    click(&by_label(&a.ui, "Eliminar").into_iter().last().unwrap());
    assert!(st.get_delete_open());
    assert!(!by_label(&a.ui, "Se borra «Base terminada» del disco.").is_empty());
    click(&by_label(&a.ui, "Cancelar").into_iter().last().unwrap());
    assert!(!st.get_delete_open());
    assert!(a.rec.take().is_empty());
    click(&by_id(&a.ui, "dots")[3]);
    click(&by_label(&a.ui, "Eliminar").into_iter().last().unwrap());
    click(&by_label(&a.ui, "Eliminar").into_iter().last().unwrap());
    assert_eq!(a.rec.take(), [format!("backup-delete {ID} backup-1")]);
    a.b.deleted(ID, Ok(()));
    assert_eq!(a.rec.take(), [format!("backups {ID}")]);
    a.b.deleted(ID, Err(failure("No se pudo borrar")));
    assert_eq!(a.ui.get_toast_message(), "No se pudo borrar");
}

fn ajustes_automaticos() {
    let a = app("es");
    let st = a.st();
    // El interruptor guarda enseguida y se ve ya apagado.
    click(&by_id(&a.ui, "auto-switch")[0]);
    assert!(!st.get_auto());
    assert_eq!(a.rec.take(), [format!("backup-settings {ID} auto=false h=Some(24) max=Some(5)")]);
    // Apagadas, las opciones no responden.
    click(&by_id(&a.ui, "interval")[0]);
    assert!(a.rec.take().is_empty());
    // Si falla, se avisa y vuelve a lo guardado.
    a.b.settings_saved(ID, Err(failure("Sin permiso")));
    assert!(st.get_auto());
    assert_eq!(a.ui.get_toast_message(), "Sin permiso");

    // Frecuencia: cada 12 h.
    click(&by_label(&a.ui, "Cada 12 h")[0]);
    assert_eq!(st.get_interval(), 12);
    assert_eq!(a.rec.take(), [format!("backup-settings {ID} auto=true h=Some(12) max=Some(5)")]);
    // Guardado: el detalle se relee para tener `Instance.backups` al día.
    a.b.settings_saved(ID, Ok(BackupSettings { auto: true, interval_hours: Some(12), max_count: Some(5) }));
    assert_eq!(a.rec.take(), [format!("load-instance {ID}")]);

    // Cuántas se conservan: con el teclado, guarda una vez tras 450 ms.
    let slider = &by_id(&a.ui, "keep")[0];
    slider.invoke_accessible_increment_action();
    slider.invoke_accessible_increment_action();
    i_slint_backend_testing::testing_backend::mock_elapsed_time(200);
    assert_eq!(st.get_keep(), 7);
    assert!(a.rec.take().is_empty());
    // Llega una lectura vieja del detalle: no pisa el cambio pendiente.
    a.detail.loaded(ID, Ok(fixtures::detail::sample_detail(ID).unwrap()));
    assert_eq!(st.get_keep(), 7);
    i_slint_backend_testing::testing_backend::mock_elapsed_time(300);
    assert_eq!(a.rec.take(), [format!("backup-settings {ID} auto=true h=Some(12) max=Some(7)")]);

    // Una frecuencia propia de la instancia sale al final.
    let mut d = fixtures::detail::sample_detail(ID).unwrap();
    d.instance.backups.interval_hours = Some(48);
    a.detail.loaded(ID, Ok(d));
    settle();
    assert_eq!(by_id(&a.ui, "interval").iter().filter(|e| e.is_valid() && e.accessible_label().is_some_and(|l| l == "Cada 48 h")).count(), 1);
}

fn en_ingles() {
    let a = app("en");
    assert_eq!(a.item(0).title, "Automatic backup");
    assert_eq!(a.item(0).date, "Oct 11, 2026 15:30");
    assert_eq!(a.st().get_total(), "4.3 GB");
    assert!(!by_label(&a.ui, "Pinned").is_empty());
}

#[test]
fn pestana_copias() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions { mock_time: true, threading: false, renderer_name: Some("software".into()) },
    )))
    .unwrap();
    lista_resumen_y_ajustes();
    fijar_y_desfijar();
    crear_con_etiqueta_fijada_y_progreso();
    restaurar_solo_mundos_con_copia_previa();
    menu_cambiar_etiqueta_y_eliminar();
    ajustes_automaticos();
    en_ingles();
}
