//! Pruebas de interacción de los diálogos de instancias (fase 3): «Nueva
//! instancia», «Cambiar versión» y la papelera. Clics y teclado de verdad
//! sobre la UI; lo que se pide al núcleo se apunta (`fixtures::Recorder`) y
//! el test entrega los resultados al controlador, como haría el núcleo. Las
//! aserciones son sobre lo pedido y el estado de la UI, no sobre píxeles.
//!
//! Un solo `#[test]`: el backend de pruebas de Slint se inicia una vez por
//! proceso. El mismo recorrido contra `lcore` real está en
//! `nucleo_dialogos.rs`.

use std::rc::Rc;

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use shard_ui::change_version::ChangeVersion;
use shard_ui::controller::{Actions, Controller};
use shard_ui::create_instance::CreateInstance;
use shard_ui::errors::Failure;
use shard_ui::fixtures::versions::{sample_loader_versions, sample_versions};
use shard_ui::fixtures::{self, Recorder};
use shard_ui::instance::InstanceDetail;
use shard_ui::trash::Trash;
use shard_ui::{ChangeVersionState, CreateInstanceState, InstanceTab, MainWindow, Route, SelectMenu, TrashState};
use slint::platform::{Key, PointerEventButton, WindowEvent};
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
    ctl: Rc<Controller>,
    detail: Rc<InstanceDetail>,
    create: Rc<CreateInstance>,
    change: Rc<ChangeVersion>,
    trash: Rc<Trash>,
    rec: Recorder,
}

fn app(lang: &str) -> App {
    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    ui.set_animate_background(false);
    let ctl = Controller::new(&ui, lang, Box::new(|| fixtures::NOW), Box::new(NoActions));
    ctl.set_instances(fixtures::sample_instances());
    ui.set_animate_background(false);
    let rec = Recorder::default();
    let detail = InstanceDetail::new(&ui, ctl.clone(), Box::new(rec.clone()));
    let create = CreateInstance::new(&ui, ctl.clone(), Box::new(rec.clone()));
    let trash = Trash::new(&ui, ctl.clone(), Box::new(rec.clone()), Box::new(|| fixtures::NOW));
    shard_ui::create_instance::dialog_hooks(&ctl, &create, &trash);
    let change = ChangeVersion::new(&ui, detail.clone(), Box::new(rec.clone()));
    ui.set_route(Route::Instances);
    ui.show().unwrap();
    settle();
    App { ui, ctl, detail, create, change, trash, rec }
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

fn by_label(ui: &MainWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(ui, label).next().unwrap_or_else(|| panic!("no hay «{label}»"))
}

fn click(e: &ElementHandle) {
    e.mock_single_click(PointerEventButton::Left);
    settle();
}

fn key(ui: &MainWindow, t: slint::SharedString) {
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: t });
}

fn type_text(ui: &MainWindow, text: &str) {
    for ch in text.chars() {
        key(ui, ch.to_string().into());
    }
    settle();
}

fn erase(ui: &MainWindow, n: usize) {
    for _ in 0..n {
        key(ui, Key::Backspace.into());
    }
    settle();
}

fn toast(ui: &MainWindow) -> String {
    ui.get_toast_message().to_string()
}

fn failure() -> Failure {
    Failure { kind: "network".into(), message: "Sin conexión".into(), detail: "network: x".into() }
}

fn version_ids(ui: &MainWindow) -> Vec<String> {
    ui.global::<CreateInstanceState>().get_picker().rows.iter().map(|r| r.id.to_string()).collect()
}

fn crear_elige_version_loader_y_nombre() {
    let a = app("es");
    let st = a.ui.global::<CreateInstanceState>();
    // «Nueva instancia» de Instancias abre el diálogo y pide las versiones.
    click(&by_id(&a.ui, "new")[0]);
    assert!(st.get_open());
    assert_eq!(a.rec.take(), ["versions false"]);
    assert!(st.get_picker().loading);
    assert!(!st.get_can_create(), "sin versión no se crea");

    // Llega la lista: se elige la última release y se propone el nombre.
    a.create.versions_loaded(false, Ok(sample_versions(false)));
    settle();
    assert_eq!(st.get_picker().selected, "1.21.5");
    assert_eq!(st.get_name(), "Minecraft 1.21.5");
    assert!(st.get_can_create());
    assert_eq!(by_id(&a.ui, "version-tag").len(), 1, "«Última» solo en 1.21.5");

    // Fabric: pide sus versiones y cambia el nombre.
    click(&by_label(&a.ui, "Fabric"));
    assert_eq!(a.rec.take(), ["loader-versions fabric 1.21.5"]);
    assert_eq!(st.get_name(), "Fabric 1.21.5");
    assert_eq!(st.get_picker().lv_state, 1, "cargando");
    a.create.loader_versions_loaded("fabric", "1.21.5", Ok(sample_loader_versions()));
    settle();
    assert_eq!(st.get_picker().lv_state, 2);
    assert_eq!(st.get_picker().lv_label, "La más reciente");

    // Buscar «1.20» y elegir 1.20.4 con el ratón.
    click(&by_id(&a.ui, "field-2")[0]);
    type_text(&a.ui, "1.20");
    assert_eq!(version_ids(&a.ui), ["1.20.6", "1.20.4", "1.20.1"]);
    click(&by_label(&a.ui, "1.20.4"));
    assert_eq!(st.get_picker().selected, "1.20.4");
    assert_eq!(st.get_name(), "Fabric 1.20.4");
    assert_eq!(a.rec.take(), ["loader-versions fabric 1.20.4"]);
    a.create.loader_versions_loaded("fabric", "1.20.4", Ok(sample_loader_versions()));
    settle();

    // Versión del loader desde su menú.
    click(&by_id(&a.ui, "loader-version")[0]);
    assert!(a.ui.global::<SelectMenu>().get_open());
    click(&by_label(&a.ui, "0.16.12"));
    assert_eq!(st.get_picker().lv_label, "0.16.12");

    // El nombre escrito a mano ya no se propone.
    click(&by_id(&a.ui, "field")[0]);
    erase(&a.ui, 20);
    type_text(&a.ui, "Mi mundo");
    click(&by_label(&a.ui, "Quilt"));
    assert_eq!(st.get_name(), "Mi mundo");
    click(&by_label(&a.ui, "Fabric"));
    assert_eq!(st.get_picker().lv_label, "La más reciente", "otro loader vuelve a «la más reciente»");
    click(&by_id(&a.ui, "loader-version")[0]);
    click(&by_label(&a.ui, "0.16.12"));
    a.rec.take();

    // Crear: falla y se ve el error; el segundo intento sale bien.
    click(&by_label(&a.ui, "Crear"));
    let first = a.rec.take();
    assert_eq!(first.len(), 1);
    assert!(first[0].starts_with("create Mi mundo|1.20.4|fabric|0.16.12 #"), "{first:?}");
    assert!(st.get_busy() && !st.get_can_create());
    let attempt: u64 = first[0].rsplit('#').next().unwrap().parse().unwrap();
    a.create.created(attempt, Err(failure()));
    settle();
    assert!(!st.get_busy());
    assert_eq!(st.get_error(), "Sin conexión");
    click(&by_label(&a.ui, "Crear"));
    assert_eq!(a.rec.take().len(), 1);
    assert_eq!(st.get_error(), "", "el error se borra al reintentar");
    a.create.created(attempt, Ok(("mi-mundo".into(), "Mi mundo".into())));
    settle();

    // Cerrado, aviso y la instancia nueva abierta.
    assert!(!st.get_open());
    assert_eq!(toast(&a.ui), "Instancia «Mi mundo» creada");
    assert_eq!(a.detail.id().as_deref(), Some("mi-mundo"));
    assert_eq!(a.rec.take(), ["load-instance mi-mundo"]);
}

fn crear_valida_el_nombre_snapshots_y_cancelar() {
    let a = app("es");
    let st = a.ui.global::<CreateInstanceState>();
    // También desde Inicio.
    a.ctl.new_instance();
    settle();
    assert!(st.get_open());
    a.create.versions_loaded(false, Ok(sample_versions(false)));
    settle();
    a.rec.take();

    // Nombre vacío: «Crear» no pide nada y avisa.
    click(&by_id(&a.ui, "field")[0]);
    erase(&a.ui, 30);
    assert_eq!(st.get_name_error(), "", "el aviso sale al pulsar «Crear»");
    click(&by_label(&a.ui, "Crear"));
    assert!(a.rec.take().is_empty());
    assert_eq!(st.get_name_error(), "Ponle un nombre.");
    type_text(&a.ui, "   ");
    click(&by_label(&a.ui, "Crear"));
    assert!(a.rec.take().is_empty(), "solo espacios tampoco");
    type_text(&a.ui, "x");
    assert_eq!(st.get_name_error(), "");

    // Snapshots: otra lista; hasta que llega no hay versión elegida.
    click(&by_id(&a.ui, "snapshots")[0]);
    assert_eq!(a.rec.take(), ["versions true"]);
    assert!(st.get_picker().snapshots);
    assert!(!st.get_can_create());
    a.create.versions_loaded(true, Ok(sample_versions(true)));
    settle();
    assert_eq!(version_ids(&a.ui)[0], "25w14a");
    assert_eq!(st.get_picker().selected, "1.21.5", "la última release, no la snapshot");
    // Volver a releases no vuelve a pedir la lista.
    click(&by_id(&a.ui, "snapshots")[0]);
    assert!(a.rec.take().is_empty());
    assert!(st.get_can_create());

    // Error al leer versiones (diálogo nuevo: la lista se vuelve a pedir):
    // se ve y no hay versión.
    click(&by_label(&a.ui, "Cancelar"));
    a.ctl.new_instance();
    settle();
    assert_eq!(a.rec.take(), ["versions false"]);
    a.create.versions_loaded(false, Err(failure()));
    settle();
    assert_eq!(st.get_picker().error, "Sin conexión");
    assert!(!st.get_can_create());

    // Cancelar cierra; un resultado tardío ya no toca nada.
    click(&by_label(&a.ui, "Cancelar"));
    assert!(!st.get_open());
    a.create.created(1, Ok(("x".into(), "x".into())));
    settle();
    assert_eq!(toast(&a.ui), "");
    assert_eq!(a.detail.id(), None);
}

fn cambiar_version_con_y_sin_copia() {
    let a = app("es");
    let st = a.ui.global::<ChangeVersionState>();
    a.detail.open("survival-amigos", Some(InstanceTab::Settings));
    a.detail.loaded("survival-amigos", Ok(fixtures::detail::sample_detail("survival-amigos").unwrap()));
    settle();
    a.rec.take();

    // Menú «Más opciones» → «Cambiar versión».
    click(&by_label(&a.ui, "Más opciones"));
    click(&by_label(&a.ui, "Cambiar versión"));
    assert!(st.get_open());
    assert_eq!(a.rec.take(), ["versions false", "loader-versions fabric 1.21.4"]);
    assert_eq!(st.get_current(), "1.21.4 · Fabric 0.16.10");
    assert!(st.get_backup(), "copia activada por defecto");
    assert!(!st.get_changed() && !st.get_can_apply(), "sin cambios no se aplica");
    a.change.versions_loaded(false, Ok(sample_versions(false)));
    a.change.loader_versions_loaded("fabric", "1.21.4", Ok(sample_loader_versions()));
    settle();
    assert_eq!(st.get_picker().selected, "1.21.4");
    assert_eq!(st.get_picker().lv_label, "0.16.10");

    // NeoForge: hay cambio y se ve a dónde.
    click(&by_label(&a.ui, "NeoForge"));
    assert!(st.get_changed() && st.get_can_apply());
    assert_eq!(st.get_target(), "1.21.4 · NeoForge");
    assert_eq!(a.rec.take(), ["loader-versions neoforge 1.21.4"]);

    // Con copia previa (etiqueta con la versión de antes).
    click(&by_label(&a.ui, "Cambiar versión"));
    let req = a.rec.take();
    assert_eq!(req.len(), 1);
    assert!(req[0].starts_with("change survival-amigos|1.21.4|neoforge|-|Antes de cambiar desde 1.21.4 · Fabric #"), "{req:?}");
    assert!(st.get_busy() && !st.get_can_apply());
    let attempt: u64 = req[0].rsplit('#').next().unwrap().parse().unwrap();
    a.change.done(attempt, Err(failure()));
    settle();
    assert_eq!(st.get_error(), "Sin conexión");
    assert!(!st.get_busy());

    // Sin copia (clic en «Copia antes»).
    click(&by_label(&a.ui, "Copia antes"));
    assert!(!st.get_backup());
    click(&by_label(&a.ui, "Cambiar versión"));
    let req = a.rec.take();
    assert!(req[0].starts_with("change survival-amigos|1.21.4|neoforge|-|- #"), "{req:?}");
    a.change.done(attempt, Ok(("1.21.4".into(), "neoforge".into())));
    settle();
    assert!(!st.get_open());
    assert_eq!(toast(&a.ui), "Versión cambiada a 1.21.4 · NeoForge");
    assert_eq!(a.rec.take(), ["load-instance survival-amigos"], "el detalle se recarga");

    // Vanilla no guarda versión de loader.
    a.change.open(shard_ui::change_version::Current::from_detail(&a.detail.detail().unwrap()));
    settle();
    a.rec.take();
    click(&by_label(&a.ui, "Vanilla"));
    click(&by_label(&a.ui, "Copia antes"));
    click(&by_label(&a.ui, "Cambiar versión"));
    let req = a.rec.take();
    assert!(req[0].starts_with("change survival-amigos|1.21.4|vanilla|-|- #"), "{req:?}");
    click(&by_label(&a.ui, "Cancelar"));
    assert!(st.get_open(), "mientras cambia, «Cancelar» está desactivado");
}

fn papelera_recuperar_borrar_y_vaciar() {
    let a = app("en");
    let st = a.ui.global::<TrashState>();
    click(&by_id(&a.ui, "trash")[0]);
    assert!(st.get_open());
    assert_eq!(st.get_state(), 0, "cargando");
    assert_eq!(a.rec.take(), ["trash"]);
    let list = fixtures::trash::sample_trash();
    a.trash.listed(Ok(list.clone()));
    settle();
    assert_eq!(st.get_state(), 2);
    let left: Vec<i32> = st.get_items().iter().map(|i| i.days_left).collect();
    assert_eq!(left, [28, 11]);
    assert!(ElementHandle::find_by_accessible_label(&a.ui, "28 days left").next().is_some());

    // Borrar una: pide confirmación; cancelar no borra.
    click(&by_id(&a.ui, "trash-purge")[0]);
    assert!(st.get_confirm_open());
    assert_eq!(st.get_confirm_name(), "Pruebas viejas");
    click(&by_label(&a.ui, "Cancel"));
    assert!(!st.get_confirm_open());
    assert!(a.rec.take().is_empty());
    click(&by_id(&a.ui, "trash-purge")[0]);
    click(&by_id(&a.ui, "confirm-purge")[0].query_descendants().match_accessible_label("Delete forever").find_first().unwrap());
    assert_eq!(a.rec.take(), [format!("purge {}", list[0].trash_id)]);
    assert!(st.get_busy());
    assert!(st.get_items().row_data(0).unwrap().busy && !st.get_items().row_data(1).unwrap().busy);
    a.trash.purged(Ok(()));
    settle();
    assert_eq!(a.rec.take(), ["trash"], "vuelve a leer la papelera");
    a.trash.listed(Ok(list[1..].to_vec()));
    settle();
    assert_eq!(st.get_items().row_count(), 1);

    // Vaciar: confirmación con el número.
    a.trash.listed(Ok(list.clone()));
    settle();
    click(&by_label(&a.ui, "Empty trash"));
    assert!(st.get_confirm_open());
    assert_eq!((st.get_confirm_name().as_str(), st.get_confirm_count()), ("", 2));
    assert!(ElementHandle::find_by_accessible_label(&a.ui, "Delete 2 instances forever?").next().is_some());
    click(&by_id(&a.ui, "confirm-purge")[0].query_descendants().match_accessible_label("Delete forever").find_first().unwrap());
    assert_eq!(a.rec.take(), [format!("purge {},{}", list[0].trash_id, list[1].trash_id)]);
    a.trash.purged(Err(failure()));
    settle();
    assert_eq!(toast(&a.ui), "Sin conexión");
    a.rec.take();
    a.trash.listed(Ok(vec![]));
    settle();
    assert_eq!(st.get_state(), 1, "vacía");
    assert!(ElementHandle::find_by_accessible_label(&a.ui, "Empty trash").next().is_none(), "sin «Vaciar» si no hay nada");

    // Recuperar: aviso, se cierra y abre la instancia.
    a.trash.listed(Ok(list.clone()));
    settle();
    click(&by_id(&a.ui, "outlined")[1]);
    assert_eq!(a.rec.take(), [format!("restore {}", list[1].trash_id)]);
    assert!(st.get_items().row_data(1).unwrap().busy);
    click(&by_id(&a.ui, "outlined")[0]);
    assert!(a.rec.take().is_empty(), "una cosa a la vez");
    a.trash.restored(Err(failure()));
    settle();
    assert!(st.get_open() && !st.get_busy());
    click(&by_id(&a.ui, "outlined")[1]);
    a.rec.take();
    a.trash.restored(Ok(("hardcore-1".into(), "Hardcore 1".into())));
    settle();
    assert!(!st.get_open());
    assert_eq!(toast(&a.ui), "“Hardcore 1” restored");
    assert_eq!(a.detail.id().as_deref(), Some("hardcore-1"));
    let _ = &a.ctl;
}

#[test]
fn dialogos_de_instancias() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    crear_elige_version_loader_y_nombre();
    crear_valida_el_nombre_snapshots_y_cancelar();
    cambiar_version_con_y_sin_copia();
    papelera_recuperar_borrar_y_vaciar();
}
