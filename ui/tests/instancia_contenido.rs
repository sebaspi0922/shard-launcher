//! Pruebas de interacción de la pestaña Contenido del detalle (fase 3):
//! clics y teclado de verdad; lo que se pide al núcleo se apunta
//! (`fixtures::Recorder`) y el test entrega los resultados al controlador,
//! como haría `CoreContent`. Aserciones sobre lo pedido y sobre la UI.
//!
//! Un solo `#[test]`: el backend de pruebas de Slint se inicia una vez por
//! proceso.

use std::rc::Rc;

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use shard_ui::controller::{Actions, Controller};
use shard_ui::errors::Failure;
use shard_ui::fixtures::{self, Recorder, inst_content as fx};
use shard_ui::inst_content::InstContent;
use shard_ui::instance::InstanceDetail;
use shard_ui::{InstContentState, InstanceTab, LoadState, MainWindow, Route};
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

struct App {
    ui: MainWindow,
    detail: Rc<InstanceDetail>,
    content: Rc<InstContent>,
    rec: Recorder,
}

/// Detalle de «Survival con amigos» en la pestaña Contenido, con lo pedido
/// al abrir ya comprobado y sin entregar.
fn app(lang: &str) -> App {
    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    let ctl = Controller::new(&ui, lang, Box::new(|| fixtures::NOW), Box::new(NoActions));
    ctl.set_instances(fixtures::sample_instances());
    ui.set_animate_background(false);
    let rec = Recorder::default();
    let detail = InstanceDetail::new(&ui, ctl.clone(), Box::new(rec.clone()));
    let content = InstContent::new(&ui, &detail, Box::new(rec.clone()));
    ui.show().unwrap();
    detail.open(ID, Some(InstanceTab::Content));
    detail.loaded(ID, Ok(fixtures::detail::sample_detail(ID).unwrap()));
    settle();
    assert_eq!(
        rec.take(),
        [format!("load-instance {ID}"), format!("content-list {ID}"), format!("content-updates {ID}"), format!("content-icons {ID}")]
    );
    App { ui, detail, content, rec }
}

/// Con todo lo del puente falso entregado.
fn loaded(lang: &str) -> App {
    let a = app(lang);
    fx::deliver(&a.content, ID);
    settle();
    // Pidió las imágenes de los 8 iconos conocidos (el desconocido no tiene).
    let asked = a.rec.take();
    assert_eq!(asked.len(), 8, "{asked:?}");
    assert!(asked.iter().all(|s| s.starts_with("content-icon https://cdn.modrinth.com/data/test/")));
    a
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

fn key(ui: &MainWindow, text: &str) {
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: text.into() });
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: text.into() });
    settle();
}

fn type_text(ui: &MainWindow, text: &str) {
    for ch in text.chars() {
        key(ui, &ch.to_string());
    }
}

fn st(a: &App) -> InstContentState<'_> {
    a.ui.global::<InstContentState>()
}

/// Nombres de las filas visibles (en orden).
fn names(a: &App) -> Vec<String> {
    st(a).get_rows().iter().map(|r| r.name.to_string()).collect()
}

fn toast(a: &App) -> String {
    a.ui.get_toast_message().to_string()
}

fn failure(kind: &str, message: &str) -> Failure {
    Failure { kind: kind.into(), message: message.into(), detail: format!("{kind}: detalle") }
}

fn carga_lista_iconos_y_avisos() {
    let a = app("es");
    // Mientras llega la lista: ruedita, sin filas ni avisos.
    assert_eq!(st(&a).get_load(), LoadState::Loading);
    assert_eq!(by_id(&a.ui, "content-loading").len(), 1);
    assert!(by_id(&a.ui, "banner").is_empty());
    // Buscando actualizaciones: el botón de refrescar está ocupado.
    assert!(st(&a).get_updates_loading());
    click(&by_label(&a.ui, "Buscar actualizaciones")[0]);
    assert!(a.rec.take().is_empty(), "ocupado: no repite la consulta");

    fx::deliver(&a.content, ID);
    settle();
    a.rec.take();
    // Todo menos datapacks; contadores por tipo.
    assert_eq!(names(&a).len(), 9);
    assert_eq!(st(&a).get_counts().iter().collect::<Vec<_>>(), [9, 7, 1, 1]);
    assert_eq!(by_id(&a.ui, "filter").len(), 4);
    // Dos avisos: 2 actualizaciones y 1 archivo desconocido.
    let banners = by_id(&a.ui, "banner");
    assert_eq!(banners.len(), 2);
    assert_eq!(banners[0].accessible_label().unwrap(), "2 actualizaciones disponibles");
    assert_eq!(banners[1].accessible_label().unwrap(), "1 archivo de origen desconocido");
    // Iconos del proyecto donde los hay; el desconocido, el de su tipo.
    let rows: Vec<_> = st(&a).get_rows().iter().collect();
    assert!(rows.iter().filter(|r| r.source != "unknown").all(|r| r.has_icon));
    let unknown = rows.iter().find(|r| r.source == "unknown").unwrap();
    assert!(!unknown.has_icon);
    assert_eq!(unknown.name, "mi-mod-casero.jar");
    assert_eq!(unknown.details, "48 KB");
    assert_eq!(rows[0].details, "fabric-api-0.119.2+1.21.4.jar  ·  0.119.2+1.21.4  ·  2,2 MB");
    // Solo Sodium y Fresh Animations tienen versión nueva.
    let with_update: Vec<_> = rows.iter().filter(|r| !r.update.is_empty()).map(|r| (r.name.to_string(), r.update.to_string())).collect();
    assert_eq!(with_update, [("Sodium".into(), "0.6.14+mc1.21.4".into()), ("Fresh Animations".into(), "1.9.4".into())]);

    // Refrescar: actualizaciones e iconos otra vez (la lista no).
    click(&by_label(&a.ui, "Buscar actualizaciones")[0]);
    assert_eq!(a.rec.take(), [format!("content-updates {ID}"), format!("content-icons {ID}")]);
    assert!(st(&a).get_updates_loading());
    // Un error al buscar: sin aviso de actualizaciones (sin toast).
    a.content.updates_loaded(ID, Err(failure("network", "Sin conexión")));
    settle();
    assert!(!st(&a).get_updates_loading());
    assert_eq!(by_id(&a.ui, "banner").len(), 1);
}

fn filtros_texto_y_anadir() {
    let a = loaded("es");
    // Píldora «Mods».
    click(&by_label(&a.ui, "Mods")[0]);
    assert_eq!(st(&a).get_filter(), 1);
    assert_eq!(names(&a).len(), 7);
    // «Añadir» con un filtro busca ese tipo.
    click(&by_label(&a.ui, "Shaders")[0]);
    assert_eq!(names(&a), ["Complementary Shaders - Reimagined"]);
    click(&by_label(&a.ui, "Añadir")[0]);
    assert_eq!(a.rec.take(), [format!("content-browse {ID} shader")]);
    // Texto: en el proyecto o el archivo, sin mayúsculas.
    click(&by_label(&a.ui, "Todo")[0]);
    click(&by_id(&a.ui, "content-search")[0]);
    type_text(&a.ui, "FAB");
    assert_eq!(names(&a), ["Fabric API", "Sodium", "Iris Shaders", "Lithium", "AppleSkin"]);
    type_text(&a.ui, "ric-api");
    assert_eq!(names(&a), ["Fabric API"]);
    // Nada coincide: el texto de «sin resultados», no el estado vacío.
    type_text(&a.ui, "zz");
    assert!(names(&a).is_empty());
    assert_eq!(by_id(&a.ui, "content-no-matches").len(), 1);
    assert!(by_id(&a.ui, "content-empty").is_empty());
    // Con «Todo», «Añadir» busca mods.
    click(&by_label(&a.ui, "Añadir")[0]);
    assert_eq!(a.rec.take(), [format!("content-browse {ID} mod")]);

    // Otra pestaña y vuelta: estado nuevo (sin filtro ni texto) y datos
    // pedidos otra vez, como Flutter.
    a.detail.select_tab(InstanceTab::Worlds);
    a.detail.select_tab(InstanceTab::Content);
    settle();
    assert_eq!(st(&a).get_filter(), 0);
    assert_eq!(a.rec.take(), [format!("content-list {ID}"), format!("content-updates {ID}"), format!("content-icons {ID}")]);
    assert_eq!(st(&a).get_load(), LoadState::Loading);
    // Un resultado de otra instancia no se mezcla.
    a.content.listed("vanilla-puro", Ok(vec![]));
    assert_eq!(st(&a).get_load(), LoadState::Loading);
    fx::deliver(&a.content, ID);
    settle();
    assert_eq!(names(&a).len(), 9);
}

fn activar_desactivar_con_vuelta_atras() {
    let a = loaded("es");
    // AppleSkin está desactivado (etiqueta «Desactivado»).
    click(&by_id(&a.ui, "content-search")[0]);
    type_text(&a.ui, "apple");
    let row = st(&a).get_rows().row_data(0).unwrap();
    assert!(!row.enabled);
    assert!(!by_label(&a.ui, "Desactivado").is_empty());
    let sw = &by_id(&a.ui, "content-switch")[0];
    assert_eq!(sw.accessible_checked(), Some(false));
    click(sw);
    let key = "mod/appleskin-fabric-mc1.21.4-3.0.6.jar";
    assert_eq!(a.rec.take(), [format!("content-enable {ID} {key} true")]);
    // Al momento, sin esperar al núcleo.
    assert!(st(&a).get_rows().row_data(0).unwrap().enabled);
    assert!(by_label(&a.ui, "Desactivado").is_empty());
    // El núcleo falla: vuelve atrás (también el interruptor) y avisa.
    a.content.enabled_set(ID, key, true, Err(failure("io", "No se pudo leer o escribir un archivo.")));
    settle();
    assert!(!st(&a).get_rows().row_data(0).unwrap().enabled);
    assert_eq!(by_id(&a.ui, "content-switch")[0].accessible_checked(), Some(false));
    assert_eq!(toast(&a), "No se pudo leer o escribir un archivo.");
    assert!(a.ui.get_toast_error());
    // Con éxito no hay aviso y queda como se pidió.
    click(&by_id(&a.ui, "content-switch")[0]);
    a.rec.take();
    a.ui.set_toast_message("".into());
    a.content.enabled_set(ID, key, true, Ok(()));
    settle();
    assert!(st(&a).get_rows().row_data(0).unwrap().enabled);
    assert_eq!(toast(&a), "");
}

fn quitar_con_confirmacion() {
    let a = loaded("es");
    // Cuarta fila: Lithium.
    click(&by_id(&a.ui, "content-remove")[3]);
    assert_eq!(st(&a).get_remove_name(), "Lithium");
    assert!(!by_label(&a.ui, "¿Quitar «Lithium»?").is_empty());
    // Escape cierra sin quitar.
    key(&a.ui, "\u{1b}");
    assert!(by_id(&a.ui, "confirm-remove").is_empty());
    // «Cancelar» tampoco quita.
    click(&by_id(&a.ui, "content-remove")[3]);
    click(&by_label(&a.ui, "Cancelar")[0]);
    assert!(by_id(&a.ui, "confirm-remove").is_empty());
    assert!(a.rec.take().is_empty());
    // «Quitar» en el diálogo (el último con esa etiqueta: los de las filas
    // van antes).
    click(&by_id(&a.ui, "content-remove")[3]);
    let dialog_btn = by_label(&a.ui, "Quitar").into_iter().last().unwrap();
    click(&dialog_btn);
    assert!(by_id(&a.ui, "confirm-remove").is_empty());
    assert_eq!(a.rec.take(), [format!("content-remove {ID} mod/lithium-fabric-0.14.7+mc1.21.4.jar Lithium")]);
    a.content.removed(ID, "Lithium", Ok(()));
    settle();
    assert_eq!(toast(&a), "«Lithium» quitado");
    assert_eq!(a.rec.take(), [format!("content-list {ID}")]);
    // Error: aviso en rojo, sin recargar.
    a.content.removed(ID, "Lithium", Err(failure("io", "No se pudo leer o escribir un archivo.")));
    settle();
    assert!(a.ui.get_toast_error());
    assert!(a.rec.take().is_empty());
}

fn actualizar_uno_y_todos() {
    let a = loaded("es");
    // Versión nueva: Sodium y Fresh Animations (esta, fuera de la vista).
    assert_eq!(st(&a).get_rows().row_data(7).unwrap().update, "1.9.4");
    let btn = &by_label(&a.ui, "Actualizar a 0.6.14+mc1.21.4")[0];
    click(btn);
    let key = "mod/sodium-fabric-0.6.13+mc1.21.4.jar";
    assert_eq!(a.rec.take(), [format!("content-update {ID} {key} Sodium")]);
    assert!(st(&a).get_rows().row_data(1).unwrap().updating);
    // Ocupado: otro clic no repite.
    click(&by_label(&a.ui, "Actualizar a 0.6.14+mc1.21.4")[0]);
    assert!(a.rec.take().is_empty());
    a.content.updated(ID, key, "Sodium", Ok(Some("0.6.14+mc1.21.4".into())));
    settle();
    assert_eq!(toast(&a), "Sodium actualizado a 0.6.14+mc1.21.4");
    assert!(!st(&a).get_rows().row_data(1).unwrap().updating);
    // Recarga lista, actualizaciones e iconos.
    assert_eq!(a.rec.take(), [format!("content-list {ID}"), format!("content-updates {ID}"), format!("content-icons {ID}")]);
    a.content.updates_loaded(ID, Ok(fx::sample_updates()));
    settle();

    // «Actualizar todo»: todas las que hay, de una vez.
    click(&by_id(&a.ui, "update-all-play")[0]);
    assert_eq!(
        a.rec.take(),
        [format!("content-update-all {ID} {key},resourcepack/FreshAnimations_v1.9.3.zip")]
    );
    assert!(st(&a).get_updating_all());
    assert!(st(&a).get_rows().iter().all(|r| r.updating));
    // Mientras, la acción del aviso es una ruedita y no hace nada.
    click(&by_id(&a.ui, "update-all-play")[0]);
    assert!(a.rec.take().is_empty());
    a.content.updated_all(ID, 2, 0);
    settle();
    assert_eq!(toast(&a), "2 archivos actualizados");
    assert!(!a.ui.get_toast_error());
    assert!(!st(&a).get_updating_all());
    a.rec.take();
    a.content.updated_all(ID, 1, 1);
    settle();
    assert_eq!(toast(&a), "1 actualizados, 1 con error");
    assert!(a.ui.get_toast_error());
}

fn identificar_desconocidos() {
    let a = loaded("es");
    click(&by_id(&a.ui, "identify-play")[0]);
    assert_eq!(a.rec.take(), [format!("content-identify {ID}")]);
    assert!(st(&a).get_identifying());
    click(&by_id(&a.ui, "identify-play")[0]);
    assert!(a.rec.take().is_empty());
    a.content.identified(ID, Ok((1, 1)));
    settle();
    assert_eq!(toast(&a), "Reconocidos: 1. Sin reconocer: 1.");
    assert!(!st(&a).get_identifying());
    assert_eq!(a.rec.take(), [format!("content-list {ID}"), format!("content-updates {ID}"), format!("content-icons {ID}")]);
    // Error del núcleo: aviso con su texto.
    click(&by_id(&a.ui, "identify-play")[0]);
    a.content.identified(ID, Err(failure("offline", "Sin conexión")));
    settle();
    assert_eq!(toast(&a), "Sin conexión");
    assert!(a.ui.get_toast_error());
}

fn vacio_error_e_ingles() {
    // Vacío: estado con «Explorar contenido», que busca mods.
    let a = app("es");
    a.content.listed(ID, Ok(vec![]));
    a.content.updates_loaded(ID, Ok(vec![]));
    settle();
    assert_eq!(by_id(&a.ui, "content-empty").len(), 1);
    assert!(by_id(&a.ui, "banner").is_empty());
    click(&by_label(&a.ui, "Explorar contenido")[0]);
    assert_eq!(a.rec.take(), [format!("content-browse {ID} mod")]);
    // «Añadir» de verdad (CoreContent::browse) lleva a la pantalla Contenido.
    a.content.go_browse(ID, "mod");
    assert_eq!(a.ui.get_route(), Route::Content);

    // Error al leer: la nota con su texto y su detalle.
    let a = app("es");
    a.content.listed(ID, Err(failure("instance_not_found", "No se encontró la instancia.")));
    settle();
    assert_eq!(st(&a).get_load(), LoadState::Failed);
    assert!(!by_label(&a.ui, "No se encontró la instancia.").is_empty());
    assert_eq!(by_id(&a.ui, "content-error").len(), 1);
    assert_eq!(st(&a).get_error_detail(), "instance_not_found: detalle");

    // Inglés: textos del ARB en.
    let a = loaded("en");
    assert_eq!(by_id(&a.ui, "banner")[0].accessible_label().unwrap(), "2 updates available");
    assert!(!by_label(&a.ui, "Resource packs").is_empty());
    a.content.updated_all(ID, 1, 0);
    settle();
    assert_eq!(toast(&a), "1 file updated");
}

#[test]
fn pestana_contenido() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    carga_lista_iconos_y_avisos();
    filtros_texto_y_anadir();
    activar_desactivar_con_vuelta_atras();
    quitar_con_confirmacion();
    actualizar_uno_y_todos();
    identificar_desconocidos();
    vacio_error_e_ingles();
}
