//! Pruebas de interacción de las pestañas Mundos y Capturas del detalle de
//! instancia (fase 3): clics y teclado de verdad sobre la UI; lo que se
//! pide al núcleo se apunta (`fixtures::Recorder`) y el test entrega los
//! resultados al controlador, como haría `CoreWorlds`/`CoreScreenshots`.
//! Las aserciones son sobre lo pedido al núcleo y el estado de la UI.
//!
//! Un solo `#[test]`: el backend de pruebas de Slint se inicia una vez por
//! proceso.

use std::rc::Rc;

use i_slint_backend_testing::{AccessibleRole, ElementHandle, ElementRoot};
use shard_ui::controller::{Actions, Controller};
use shard_ui::errors::Failure;
use shard_ui::fixtures::{self, Recorder};
use shard_ui::instance::InstanceDetail;
use shard_ui::instances::Phase;
use shard_ui::screenshots::Screenshots;
use shard_ui::worlds::Worlds;
use shard_ui::{InstanceTab, MainWindow, ScreenshotsState, WorldsState};
use slint::platform::{Key, PointerEventButton, WindowEvent};
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
    ctl: Rc<Controller>,
    detail: Rc<InstanceDetail>,
    worlds: Rc<Worlds>,
    shots: Rc<Screenshots>,
    rec: Recorder,
}

fn app() -> App {
    let ui = MainWindow::new().unwrap();
    // Alta: el panel del mundo entero a la vista (la búsqueda de
    // elementos no ve lo que el Flickable deja fuera).
    ui.window().set_size(slint::LogicalSize::new(1280.0, 1400.0));
    let ctl = Controller::new(&ui, "es", Box::new(|| fixtures::NOW), Box::new(NoActions));
    ctl.set_instances(fixtures::sample_instances());
    ui.set_animate_background(false);
    let rec = Recorder::default();
    let detail = InstanceDetail::new(&ui, ctl.clone(), Box::new(rec.clone()));
    let worlds = Worlds::new(&ui, &detail, Box::new(rec.clone()), Box::new(|| fixtures::NOW));
    let shots = Screenshots::new(&ui, &detail, Box::new(rec.clone()), Box::new(|_| fixtures::screenshots::UTC_OFFSET));
    ui.show().unwrap();
    App { ui, ctl, detail, worlds, shots, rec }
}

/// Detalle de la instancia en `tab`, con la instancia completa ya cargada.
fn open(a: &App, tab: InstanceTab) {
    a.detail.open(ID, Some(tab));
    a.detail.loaded(ID, Ok(fixtures::detail::sample_detail(ID).unwrap()));
    settle();
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

/// Botón (o segmento) con ese nombre accesible.
fn button(ui: &MainWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(ui, label)
        .filter(|e| e.accessible_role() == Some(AccessibleRole::Button))
        .last()
        .unwrap_or_else(|| panic!("no hay botón «{label}»"))
}

fn click(e: &ElementHandle) {
    e.mock_single_click(PointerEventButton::Left);
    settle();
}

fn key(ui: &MainWindow, text: impl Into<slint::SharedString>) {
    let t = text.into();
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: t });
}

fn type_text(ui: &MainWindow, text: &str) {
    for ch in text.chars() {
        key(ui, ch.to_string());
    }
    settle();
}

fn fail(kind: &str, message: &str) -> Failure {
    Failure { kind: kind.into(), message: message.into(), detail: format!("{kind}: algo") }
}

fn toast(ui: &MainWindow) -> String {
    ui.get_toast_message().to_string()
}

/// Mundos cargados con los datos de ejemplo y los datapacks del primero.
fn worlds_loaded(a: &App) {
    open(a, InstanceTab::Worlds);
    assert_eq!(a.rec.take(), [format!("load-instance {ID}"), format!("worlds {ID}")]);
    let st = a.ui.global::<WorldsState>();
    assert!(st.get_loading());
    a.worlds.listed(ID, Ok(fixtures::worlds::sample_worlds()));
    assert_eq!(a.rec.take(), [format!("datapacks {ID} Mundo de verano")]);
    a.worlds.datapacks_listed(ID, "Mundo de verano", Ok(fixtures::worlds::sample_datapacks()));
    settle();
}

fn mundos_lista_y_seleccion() {
    let a = app();
    worlds_loaded(&a);
    let st = a.ui.global::<WorldsState>();
    assert!(!st.get_loading());
    assert_eq!(by_id(&a.ui, "world").len(), 3);
    assert_eq!(st.get_selected(), 0);
    assert_eq!(by_id(&a.ui, "datapack").len(), 3);
    // Tarjeta: modo · dificultad y hace cuánto · tamaño.
    let w = st.get_items().row_data(2).unwrap();
    assert_eq!((w.mode, w.difficulty, w.hardcore, w.size.as_str()), (0, 3, true, "311 MB"));

    // Elegir otro mundo pide sus datapacks y cambia el panel.
    click(&by_id(&a.ui, "world")[1]);
    assert_eq!(st.get_selected(), 1);
    assert_eq!(a.rec.take(), [format!("datapacks {ID} Creativo-pruebas")]);
    assert!(st.get_datapacks_loading());
    a.worlds.datapacks_listed(ID, "Creativo-pruebas", Ok(vec![]));
    settle();
    assert_eq!(by_id(&a.ui, "datapacks-empty").len(), 1);
    // Una respuesta vieja (de otro mundo) no se mezcla.
    a.worlds.datapacks_listed(ID, "Mundo de verano", Ok(fixtures::worlds::sample_datapacks()));
    settle();
    assert_eq!(by_id(&a.ui, "datapack").len(), 0);

    // Al volver a la pestaña se recarga y se conserva el elegido.
    a.detail.select_tab(InstanceTab::Content);
    a.detail.select_tab(InstanceTab::Worlds);
    assert_eq!(a.rec.take(), [format!("worlds {ID}")]);
    a.worlds.listed(ID, Ok(fixtures::worlds::sample_worlds()));
    assert_eq!(st.get_selected(), 1);
    a.rec.take();

    // Abrir la carpeta del mundo.
    click(&by_id(&a.ui, "world-folder")[0]);
    assert_eq!(a.rec.take(), [format!("open-folder /tmp/shard-test/instances/{ID}/minecraft/saves/Creativo-pruebas")]);
}

fn mundos_modo_dificultad_hardcore_y_trucos() {
    let a = app();
    worlds_loaded(&a);
    let st = a.ui.global::<WorldsState>();
    a.rec.take();

    click(&button(&a.ui, "Creativo"));
    assert_eq!(
        a.rec.take(),
        [format!("update-world {ID} Mundo de verano WorldPatch {{ game_mode: Some(Creative), difficulty: None, hardcore: None, allow_commands: None }}")]
    );
    // El núcleo lo guarda: el mundo nuevo sustituye al anterior.
    let mut w = fixtures::worlds::sample_worlds().remove(0);
    w.game_mode = Some(lcore::world::GameMode::Creative);
    a.worlds.changed(ID, Ok(w));
    settle();
    assert_eq!(st.get_items().row_data(0).unwrap().mode, 1);

    click(&button(&a.ui, "Difícil"));
    assert!(a.rec.take()[0].contains("difficulty: Some(Hard)"));
    // El núcleo falla: aviso y el segmento vuelve a «Normal».
    a.worlds.changed(ID, Err(fail("game_already_running", "El juego ya está abierto")));
    settle();
    assert_eq!(toast(&a.ui), "El juego ya está abierto");
    assert_eq!(button(&a.ui, "Normal").accessible_checked(), Some(true));
    assert_eq!(button(&a.ui, "Difícil").accessible_checked(), Some(false));

    let switches = by_id(&a.ui, "switch");
    click(&switches[0]);
    assert!(a.rec.take()[0].contains("hardcore: Some(true)"));
    click(&switches[1]);
    assert!(a.rec.take()[0].contains("allow_commands: Some(true)"));
    // Sin respuesta buena todavía: al fallar, el interruptor se apaga.
    a.worlds.changed(ID, Err(fail("io", "Error de disco")));
    settle();
    assert_eq!(by_id(&a.ui, "switch")[1].accessible_checked(), Some(false));
}

fn mundos_renombrar_y_borrar() {
    let a = app();
    worlds_loaded(&a);
    let st = a.ui.global::<WorldsState>();
    a.rec.take();

    // Renombrar: el nombre viene escrito y seleccionado; se reemplaza al
    // escribir. Vacío no se puede guardar.
    click(&by_id(&a.ui, "rename")[0]);
    assert!(st.get_rename_open());
    for _ in 0..1 {
        key(&a.ui, Key::Backspace);
    }
    settle();
    click(&button(&a.ui, "Guardar"));
    assert!(st.get_rename_open(), "vacío no se guarda");
    type_text(&a.ui, "  Verano 2  ");
    key(&a.ui, "\n");
    settle();
    assert!(!st.get_rename_open());
    assert_eq!(a.rec.take(), [format!("rename-world {ID} Mundo de verano Verano 2")]);
    let mut w = fixtures::worlds::sample_worlds().remove(0);
    w.name = "Verano 2".into();
    a.worlds.changed(ID, Ok(w));
    settle();
    assert_eq!(st.get_items().row_data(0).unwrap().name, "Verano 2");

    // «Cancelar» no renombra.
    click(&by_id(&a.ui, "rename")[0]);
    click(&button(&a.ui, "Cancelar"));
    assert!(!st.get_rename_open());
    assert!(a.rec.take().is_empty());

    // Borrar con confirmación: aviso y queda el siguiente elegido.
    click(&by_id(&a.ui, "delete-world")[0]);
    assert_eq!(by_id(&a.ui, "confirm-delete-world").len(), 1);
    click(&button(&a.ui, "Cancelar"));
    assert!(a.rec.take().is_empty());
    click(&by_id(&a.ui, "delete-world")[0]);
    click(&button(&a.ui, "Borrar para siempre"));
    assert_eq!(a.rec.take(), [format!("delete-world {ID} Mundo de verano Verano 2")]);
    a.worlds.deleted(ID, "Mundo de verano", "Verano 2", Ok(()));
    settle();
    assert_eq!(toast(&a.ui), "Mundo «Verano 2» borrado");
    assert_eq!(st.get_items().row_count(), 2);
    assert_eq!(st.get_items().row_data(0).unwrap().folder, "Creativo-pruebas");
    assert_eq!(a.rec.take(), [format!("datapacks {ID} Creativo-pruebas")]);

    // Un error al borrar no quita nada.
    click(&by_id(&a.ui, "delete-world")[0]);
    click(&button(&a.ui, "Borrar para siempre"));
    a.rec.take();
    a.worlds.deleted(ID, "Creativo-pruebas", "Creativo de pruebas", Err(fail("io", "Error de disco")));
    settle();
    assert_eq!(toast(&a.ui), "Error de disco");
    assert_eq!(st.get_items().row_count(), 2);
}

fn mundos_datapacks() {
    let a = app();
    worlds_loaded(&a);
    a.rec.take();
    let rows = by_id(&a.ui, "datapack");
    assert_eq!(rows.len(), 3);

    // Activar el desactivado: se pide y la lista se vuelve a leer.
    let switch = |name: &str| {
        ElementHandle::find_by_accessible_label(&a.ui, name).find(|e| e.accessible_role() == Some(AccessibleRole::Switch)).unwrap()
    };
    let sw = switch("sin-phantoms.zip");
    assert_eq!(sw.accessible_checked(), Some(false));
    click(&sw);
    assert_eq!(a.rec.take(), [format!("set-datapack {ID} Mundo de verano sin-phantoms.zip true")]);
    a.worlds.datapack_done(ID, "Mundo de verano", Err(fail("io", "Error de disco")));
    assert_eq!(toast(&a.ui), "Error de disco");
    assert_eq!(a.rec.take(), [format!("datapacks {ID} Mundo de verano")]);
    a.worlds.datapacks_listed(ID, "Mundo de verano", Ok(fixtures::worlds::sample_datapacks()));
    settle();
    assert_eq!(switch("sin-phantoms.zip").accessible_checked(), Some(false), "vuelve a como está en disco");

    // Quitar con confirmación.
    let remove = ElementHandle::find_by_accessible_label(&a.ui, "Quitar").next().unwrap();
    click(&remove);
    assert_eq!(a.ui.global::<WorldsState>().get_remove_pack(), "Terralith_1.21_v2.5.8.zip");
    click(&button(&a.ui, "Quitar"));
    assert_eq!(a.rec.take(), [format!("remove-datapack {ID} Mundo de verano Terralith_1.21_v2.5.8.zip")]);
    assert_eq!(a.ui.global::<WorldsState>().get_remove_pack(), "");

    // Añadir abre el selector de archivos (en el núcleo).
    click(&by_id(&a.ui, "add-datapacks")[0]);
    assert_eq!(a.rec.take(), [format!("add-datapacks {ID} Mundo de verano")]);
}

fn mundos_con_el_juego_abierto_y_estados() {
    let a = app();
    worlds_loaded(&a);
    let st = a.ui.global::<WorldsState>();
    a.rec.take();
    a.ctl.set_phase(ID, Phase::Running);
    settle();
    assert!(st.get_running());
    assert_eq!(by_id(&a.ui, "running-note").len(), 1);
    // Nada se puede editar.
    click(&button(&a.ui, "Creativo"));
    click(&by_id(&a.ui, "rename")[0]);
    click(&by_id(&a.ui, "switch")[0]);
    click(&by_id(&a.ui, "delete-world")[0]);
    click(&by_id(&a.ui, "add-datapacks")[0]);
    assert!(a.rec.take().is_empty());
    assert!(!st.get_rename_open() && !st.get_delete_open());
    // Al cerrar el juego la lista se vuelve a leer.
    a.ctl.set_phase(ID, Phase::Idle);
    assert!(!st.get_running());
    assert_eq!(a.rec.take(), [format!("worlds {ID}")]);

    // Sin mundos: estado vacío; con error y sin datos: el error.
    a.worlds.listed(ID, Ok(vec![]));
    settle();
    assert_eq!(by_id(&a.ui, "empty").len(), 1);
    a.detail.select_tab(InstanceTab::Worlds);
    a.worlds.listed(ID, Err(fail("io", "Error de disco")));
    settle();
    assert_eq!(by_id(&a.ui, "worlds-error").len(), 1);

    // Otra instancia: no se mezclan los resultados de la anterior.
    a.detail.open("vanilla-puro", Some(InstanceTab::Worlds));
    a.worlds.listed(ID, Ok(fixtures::worlds::sample_worlds()));
    assert_eq!(st.get_items().row_count(), 0);
}

fn capturas_rejilla_y_visor() {
    let a = app();
    open(&a, InstanceTab::Screenshots);
    assert_eq!(a.rec.take(), [format!("load-instance {ID}"), format!("screenshots {ID}")]);
    let st = a.ui.global::<ScreenshotsState>();
    a.shots.listed(ID, Ok(fixtures::screenshots::sample_screenshots()));
    settle();
    // Se piden las nueve miniaturas (600 de ancho).
    let asked = a.rec.take();
    assert_eq!(asked.len(), 9);
    assert_eq!(asked[0], "image /tmp/shard-test/screenshots/shot-0.png 600");
    assert_eq!(by_id(&a.ui, "shot").len(), 9);
    let first = st.get_items().row_data(0).unwrap();
    assert_eq!(first.caption, "11 oct 2026 13:30");
    assert_eq!(st.get_items().row_data(7).unwrap().caption, "6 oct 2026 0:30");
    // Mientras tanto, el paisaje del tamaño de la celda.
    assert_eq!(first.image.size().width, 274);
    // Llega una miniatura: sustituye al paisaje.
    let px = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(600, 375);
    a.shots.image_loaded(std::path::Path::new("/tmp/shard-test/screenshots/shot-0.png"), 600, Some(px));
    assert_eq!(st.get_items().row_data(0).unwrap().image.size().width, 600);

    // Abrir la carpeta.
    click(&by_id(&a.ui, "shots-folder")[0]);
    assert_eq!(a.rec.take(), [format!("open /tmp/shard-test/instances/{ID}/minecraft/screenshots")]);

    // Visor: clic en la tercera, flechas, límites y Escape.
    click(&by_id(&a.ui, "shot")[2]);
    assert!(st.get_viewer_open());
    assert_eq!(st.get_viewer_index(), 2);
    assert_eq!(a.rec.take(), ["image /tmp/shard-test/screenshots/shot-2.png 0"]);
    key(&a.ui, Key::RightArrow);
    settle();
    assert_eq!(st.get_viewer_index(), 3);
    key(&a.ui, Key::LeftArrow);
    key(&a.ui, Key::LeftArrow);
    key(&a.ui, Key::LeftArrow);
    key(&a.ui, Key::LeftArrow);
    settle();
    assert_eq!(st.get_viewer_index(), 0);
    assert_eq!(button(&a.ui, "Anterior").accessible_enabled(), Some(false));
    click(&button(&a.ui, "Siguiente"));
    assert_eq!(st.get_viewer_index(), 1);
    a.rec.take();
    // La imagen completa de otra captura (llegó tarde) no se pone.
    let px = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(1920, 1080);
    a.shots.image_loaded(std::path::Path::new("/tmp/shard-test/screenshots/shot-3.png"), 0, Some(px.clone()));
    assert_eq!(st.get_viewer_image().size().width, 0);
    a.shots.image_loaded(std::path::Path::new("/tmp/shard-test/screenshots/shot-1.png"), 0, Some(px));
    assert_eq!(st.get_viewer_image().size().width, 1920);
    click(&button(&a.ui, "Abrir con el visor del sistema"));
    assert_eq!(a.rec.take(), ["open /tmp/shard-test/screenshots/shot-1.png"]);
    key(&a.ui, Key::Escape);
    settle();
    assert!(!st.get_viewer_open());

    // Borrar desde el visor: confirmación, y pasa a la siguiente.
    click(&by_id(&a.ui, "shot")[8]);
    a.rec.take();
    click(&button(&a.ui, "Eliminar"));
    assert!(st.get_confirm_delete());
    click(&button(&a.ui, "Cancelar"));
    assert!(a.rec.take().is_empty());
    assert!(st.get_viewer_open());
    click(&button(&a.ui, "Eliminar"));
    click(&button(&a.ui, "Eliminar"));
    assert_eq!(a.rec.take(), [format!("delete-screenshot {ID} 2026-10-03_18.08.21.png")]);
    a.shots.deleted(ID, "2026-10-03_18.08.21.png", Ok(()));
    settle();
    assert_eq!(st.get_items().row_count(), 8);
    assert!(st.get_viewer_open());
    assert_eq!(st.get_viewer_index(), 7, "la última pasa a la anterior");
    // El teclado vuelve al visor tras la confirmación.
    key(&a.ui, Key::LeftArrow);
    settle();
    assert_eq!(st.get_viewer_index(), 6);
    // Un error no quita nada.
    a.shots.deleted(ID, "2026-10-04_18.07.21.png", Err(fail("io", "Error de disco")));
    assert_eq!(toast(&a.ui), "Error de disco");
    assert_eq!(st.get_items().row_count(), 8);

    // Si se borra la única, el visor se cierra.
    a.shots.listed(ID, Ok(fixtures::screenshots::sample_screenshots()[..1].to_vec()));
    settle();
    a.shots.view(0);
    a.shots.deleted(ID, "2026-10-11_18.00.21.png", Ok(()));
    settle();
    assert!(!st.get_viewer_open());
    assert_eq!(by_id(&a.ui, "empty").len(), 1);
}

#[test]
fn mundos_y_capturas() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    mundos_lista_y_seleccion();
    mundos_modo_dificultad_hardcore_y_trucos();
    mundos_renombrar_y_borrar();
    mundos_datapacks();
    mundos_con_el_juego_abierto_y_estados();
    capturas_rejilla_y_visor();
}
