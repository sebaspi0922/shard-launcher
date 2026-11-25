//! Pruebas de interacción de Contenido y Servidores (fase 4): clics y
//! teclado de verdad sobre la UI; lo que se pide al núcleo se apunta
//! (`fixtures::Recorder`) y el test entrega los resultados a los
//! controladores, como harían `content_backend` y `servers::CoreServers`.
//!
//! Un solo `#[test]`: el backend de pruebas de Slint se inicia una vez por
//! proceso.

use std::rc::Rc;

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use shard_ui::accounts::Accounts;
use shard_ui::content::{Content, Filters, ModpackDone, Outcome, Page};
use shard_ui::controller::Controller;
use shard_ui::curseforge::Keys;
use shard_ui::errors::Failure;
use shard_ui::fixtures::{self, Recorder};
use shard_ui::servers::Servers;
use shard_ui::{ContentState, CurseforgeKey, HomeState, MainWindow, ModpackPhase, Route, ServersState};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, Model};

struct App {
    ui: MainWindow,
    ctl: Rc<Controller>,
    content: Rc<Content>,
    keys: Rc<Keys>,
    servers: Rc<Servers>,
    rec: Recorder,
}

fn app(lang: &str) -> App {
    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    ui.set_animate_background(false);
    let rec = Recorder::default();
    let ctl = Controller::new(&ui, lang, Box::new(|| fixtures::NOW), Box::new(rec.clone()));
    ctl.set_instances(fixtures::sample_instances());
    ui.set_animate_background(false);
    let acc = Accounts::new(&ui, ctl.clone(), Box::new(rec.clone()), Box::new(|| 0));
    acc.set_accounts(Ok(fixtures::sample_accounts()));
    let keys = Keys::new(&ui, ctl.clone(), Box::new(rec.clone()));
    let content = Content::new(&ui, ctl.clone(), Box::new(rec.clone()));
    let servers = Servers::new(&ui, ctl.clone(), acc, Box::new(rec.clone()));
    {
        let (c, s) = (content.clone(), servers.clone());
        ctl.on_refresh(move |_| {
            c.refresh();
            s.refresh();
        });
        let c = content.clone();
        keys.on_change(move |has| c.key_state(has));
    }
    keys.state(true);
    ui.show().unwrap();
    settle();
    rec.take();
    App { ui, ctl, content, keys, servers, rec }
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

fn toast(ui: &MainWindow) -> String {
    ui.get_toast_message().to_string()
}

fn fail() -> Failure {
    Failure { kind: "network".into(), message: "Sin conexión".into(), detail: "network: x".into() }
}

/// Abre Contenido y contesta la primera búsqueda con los datos de ejemplo.
fn open_content(a: &App) {
    a.ui.set_route(Route::Content);
    a.ui.invoke_route_changed(Route::Content);
    settle();
    answer(a);
}

/// Contesta la búsqueda en curso (FakeBridge).
fn answer(a: &App) {
    let s = a.content.search_state();
    a.content.page(s.generation(), Ok(fixtures::search_page(&s.filters, 0, shard_ui::content::PAGE)));
    settle();
}

fn searches(rec: &Recorder) -> Vec<String> {
    rec.take().into_iter().filter(|x| x.starts_with("search ")).collect()
}

fn busqueda_filtros_y_texto() {
    let a = app("es");
    a.ui.set_route(Route::Content);
    settle();
    let asked = a.rec.take();
    assert!(asked.contains(&"releases".to_owned()), "{asked:?}");
    assert!(asked.contains(&"search modrinth mod «» 0 g1".to_owned()), "{asked:?}");
    answer(&a);
    assert_eq!(by_id(&a.ui, "card").len(), 6);
    assert_eq!(by_id(&a.ui, "txt-results")[0].accessible_label().unwrap(), "6 resultados");

    // Otro tipo: busca ya, y la respuesta vieja no pisa la nueva.
    let old = a.content.search_state().generation();
    click(&by_label(&a.ui, "Shaders")[0]);
    assert_eq!(searches(&a.rec), ["search modrinth shader «» 0 g2"]);
    a.content.page(old, Ok(Page { items: fixtures::sample_projects(), total: 10 }));
    settle();
    assert_eq!(by_id(&a.ui, "card").len(), 0, "respuesta de una búsqueda vieja");
    answer(&a);
    assert_eq!(by_id(&a.ui, "card").len(), 1);

    // Texto: espera a dejar de teclear y busca una sola vez.
    click(&by_label(&a.ui, "Mods")[0]);
    a.rec.take();
    answer(&a);
    click(&by_id(&a.ui, "search")[0]);
    for ch in "sod".chars() {
        let t: slint::SharedString = ch.to_string().into();
        a.ui.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
        a.ui.window().dispatch_event(WindowEvent::KeyReleased { text: t });
        i_slint_backend_testing::testing_backend::mock_elapsed_time(100);
    }
    assert!(searches(&a.rec).is_empty(), "todavía no");
    settle();
    let s = searches(&a.rec);
    assert_eq!(s.len(), 1, "{s:?}");
    assert!(s[0].starts_with("search modrinth mod «sod» 0"), "{s:?}");
    answer(&a);
    assert_eq!(by_id(&a.ui, "card").len(), 1);

    // Orden y error con reintento.
    a.content.set_filters(Filters { text: String::new(), ..a.content.filters() }, false);
    a.rec.take();
    let g = a.content.search_state().generation();
    a.content.page(g, Err(fail()));
    settle();
    assert_eq!(by_id(&a.ui, "error").len(), 1);
    click(&by_id(&a.ui, "retry")[0]);
    assert_eq!(searches(&a.rec).len(), 1);
    answer(&a);
    assert_eq!(by_id(&a.ui, "card").len(), 6);
}

fn mas_resultados_al_bajar() {
    let a = app("es");
    open_content(&a);
    a.rec.take();
    // 30 en total: al acercarse al final pide desde el 20.
    let g = a.content.search_state().generation();
    a.content.set_filters(Filters { text: "x".into(), ..a.content.filters() }, false);
    let g2 = a.content.search_state().generation();
    assert!(g2 > g);
    let mut items = Vec::new();
    for i in 0..20 {
        let mut p = fixtures::sample_projects()[0].clone();
        p.id = format!("p{i}");
        p.icon_url = None;
        items.push(p);
    }
    a.rec.take();
    a.content.page(g2, Ok(Page { items, total: 30 }));
    settle();
    a.ui.global::<ContentState>().invoke_scrolled(100.0);
    assert_eq!(searches(&a.rec), [format!("search modrinth mod «x» 20 g{g2}")]);
    // Una sola petición a la vez.
    a.ui.global::<ContentState>().invoke_scrolled(50.0);
    assert!(searches(&a.rec).is_empty());
}

fn ficha_e_instalacion() {
    let a = app("es");
    open_content(&a);
    a.rec.take();
    // Sin destino, «Instalar» abre la ficha con la primera instancia.
    click(&by_id(&a.ui, "card-play")[0]);
    assert!(a.content.sheet_open());
    let asked = a.rec.take();
    assert!(asked.iter().any(|x| x.starts_with("versions sodium Some(\"1.21.4\") Some(\"fabric\")")), "{asked:?}");
    assert!(asked.contains(&"installed survival-amigos".to_owned()), "{asked:?}");
    a.content.installed_loaded("survival-amigos", fixtures::sample_installed());
    a.content.versions_loaded(a.content.versions_seq(), Ok(fixtures::sample_versions("sodium", Some("1.21.4"), Some("fabric"))));
    settle();
    assert_eq!(by_id(&a.ui, "version").len(), 4);
    assert!(by_id(&a.ui, "txt-installed")[0].accessible_label().unwrap().contains("0.6.13+mc1.21.4"));

    // Una versión concreta: se instala esa; mientras, no se puede otra.
    click(&by_id(&a.ui, "version-play")[1]);
    assert_eq!(a.rec.take(), ["install survival-amigos sodium Some(\"sodium-v1\") None"]);
    click(&by_id(&a.ui, "latest-play")[0]);
    assert!(a.rec.take().is_empty());
    let req = shard_ui::content::InstallRequest {
        instance_id: "survival-amigos".into(),
        source: "modrinth".into(),
        project: "sodium".into(),
        project_title: "Sodium".into(),
        version: Some("sodium-v1".into()),
        world: None,
    };
    a.content.install_done(&req, Ok(Outcome { new_dependencies: 1, skipped: 0 }));
    settle();
    assert_eq!(toast(&a.ui), "Sodium instalado en Survival con amigos con 1 dependencia");
    assert!(!a.content.sheet_open(), "la ficha se cierra al instalar");
    assert_eq!(a.rec.take(), ["installed survival-amigos"]);

    // Otra instancia en la ficha: versiones de esa.
    click(&by_id(&a.ui, "card")[1]);
    a.rec.take();
    a.ui.global::<ContentState>().invoke_sheet_pick_instance("create-aeronautica".into());
    settle();
    let asked = a.rec.take();
    assert!(asked.iter().any(|x| x.starts_with("versions lithium Some(\"1.20.1\") Some(\"forge\")")), "{asked:?}");
    // Error al instalar: aviso en rojo y la ficha sigue abierta.
    a.content.versions_loaded(a.content.versions_seq(), Ok(fixtures::sample_versions("lithium", Some("1.20.1"), Some("forge"))));
    settle();
    click(&by_id(&a.ui, "latest-play")[0]);
    assert_eq!(a.rec.take(), ["install create-aeronautica lithium None None"]);
    let req = shard_ui::content::InstallRequest {
        instance_id: "create-aeronautica".into(),
        source: "modrinth".into(),
        project: "lithium".into(),
        project_title: "Lithium".into(),
        version: None,
        world: None,
    };
    a.content.install_done(&req, Err(fail()));
    settle();
    assert!(a.ui.get_toast_error());
    assert!(a.content.sheet_open());
    click(&by_id(&a.ui, "sheet-close")[0]);
    assert!(!a.content.sheet_open());
}

fn ficha_con_la_instancia_de_inicio_y_respuesta_en_camino() {
    let a = app("es");
    // La ficha empieza en la instancia elegida en Inicio, no en la primera.
    a.ui.global::<HomeState>().invoke_select("vanilla-puro".into());
    open_content(&a);
    click(&by_label(&a.ui, "Paquetes de recursos")[0]);
    answer(&a);
    a.rec.take();
    click(&by_id(&a.ui, "card")[0]);
    assert_eq!(a.ui.global::<ContentState>().get_sheet_instance(), "vanilla-puro");
    let asked = a.rec.take();
    assert!(asked.iter().any(|x| x.starts_with("versions fresh-animations Some(\"1.21.4\") None")), "{asked:?}");
    // Otra instancia con los mismos filtros mientras llega la respuesta: no
    // se pide de nuevo y la respuesta en camino sigue valiendo.
    let seq = a.content.versions_seq();
    a.ui.global::<ContentState>().invoke_sheet_pick_instance("survival-amigos".into());
    settle();
    assert!(!a.rec.take().iter().any(|x| x.starts_with("versions ")));
    a.content.versions_loaded(seq, Ok(fixtures::sample_versions("fresh-animations", Some("1.21.4"), None)));
    settle();
    assert_eq!(by_id(&a.ui, "version").len(), 4);
}

fn destino_desde_una_instancia() {
    let a = app("es");
    a.content.open_for("survival-amigos", Some("mod"), None);
    settle();
    assert_eq!(a.ui.get_route(), Route::Content);
    let f = a.content.filters();
    assert_eq!((f.mc.as_deref(), f.loader.as_deref()), (Some("1.21.4"), Some("fabric")));
    a.content.installed_loaded("survival-amigos", fixtures::sample_installed());
    answer(&a);
    assert_eq!(by_id(&a.ui, "target").len(), 1);
    // Sodium ya está: insignia en lugar de botón.
    let items = a.ui.global::<ContentState>().get_items();
    assert!(items.row_data(0).unwrap().installed);
    assert!(!items.row_data(3).unwrap().installed);
    a.rec.take();
    // Con destino, «Instalar» instala ya la compatible más nueva.
    let plays = by_id(&a.ui, "card-play");
    click(&plays[3]);
    assert_eq!(a.rec.take(), ["install survival-amigos entityculling None None"]);
    assert!(!a.content.sheet_open());
    // «Volver a la instancia» y quitar el destino.
    click(&by_id(&a.ui, "back-to-instance")[0]);
    assert_eq!(a.rec.take(), ["open-instance survival-amigos"]);
    click(&by_id(&a.ui, "clear-target")[0]);
    assert!(a.content.target().is_none());
    assert!(by_id(&a.ui, "target").is_empty());
}

fn curseforge_sin_clave() {
    let a = app("es");
    open_content(&a);
    a.keys.state(false);
    settle();
    a.rec.take();
    click(&by_label(&a.ui, "CurseForge")[0]);
    assert!(searches(&a.rec).is_empty(), "sin clave no se busca");
    assert!(a.ui.global::<ContentState>().get_needs_key());
    click(&by_id(&a.ui, "empty-action")[0]);
    assert!(a.ui.global::<CurseforgeKey>().get_dialog_open());
    type_text(&a.ui, "  abc123  ");
    click(&by_id(&a.ui, "filled").last().unwrap().clone());
    assert_eq!(a.rec.take(), ["set-key (6 caracteres)"]);
    assert!(!a.ui.global::<CurseforgeKey>().get_dialog_open());
    a.keys.saved(Ok(()));
    settle();
    assert_eq!(toast(&a.ui), "Clave guardada");
    // Con clave, CurseForge busca.
    assert_eq!(searches(&a.rec).len(), 1);
    assert!(!a.ui.global::<ContentState>().get_needs_key());
    a.keys.saved(Err(fail()));
    assert!(a.ui.get_toast_error());
}

fn modpacks() {
    let a = app("es");
    open_content(&a);
    a.rec.take();
    click(&by_id(&a.ui, "import")[0]);
    assert!(a.ui.global::<ContentState>().get_modpack_open());
    // Sin archivo ni URL no se puede instalar.
    click(&by_id(&a.ui, "filled").last().unwrap().clone());
    assert!(a.rec.take().is_empty());
    click(&by_id(&a.ui, "modpack-file")[0]);
    assert_eq!(a.rec.take(), ["pick-modpack"]);
    a.content.modpack_picked(Some("/tmp/pack.mrpack".into()));
    settle();
    click(&by_label(&a.ui, "URL")[0]);
    click(&by_id(&a.ui, "modpack-url")[0]);
    type_text(&a.ui, "https://x.net/p.mrpack");
    click(&by_id(&a.ui, "filled").last().unwrap().clone());
    assert_eq!(a.rec.take(), ["modpack https://x.net/p.mrpack None a1"]);
    assert_eq!(a.ui.global::<ContentState>().get_modpack_phase(), ModpackPhase::Installing);
    // Mientras instala no se cierra.
    click(&by_id(&a.ui, "cancel").last().unwrap().clone());
    assert!(a.ui.global::<ContentState>().get_modpack_open());
    a.content.on_event(&lcore::events::Event::Progress {
        task_id: 1,
        stage: "modpack".into(),
        done: 5,
        total: 10,
        unit: lcore::events::ProgressUnit::Files,
    });
    assert_eq!(a.ui.global::<ContentState>().get_modpack_progress(), 0.5);
    // Un resultado de un intento viejo se ignora.
    a.content.modpack_done(0, Err(fail()));
    assert_eq!(a.ui.global::<ContentState>().get_modpack_phase(), ModpackPhase::Installing);
    let inst = fixtures::sample_instances()[0].clone();
    a.content.modpack_done(1, Ok(ModpackDone { instance: inst, files_total: 10, files_downloaded: 10, warnings: vec!["aviso".into()] }));
    settle();
    assert_eq!(a.ui.global::<ContentState>().get_modpack_phase(), ModpackPhase::Done);
    click(&by_id(&a.ui, "filled").last().unwrap().clone());
    assert!(!a.ui.global::<ContentState>().get_modpack_open());
    assert_eq!(a.rec.take(), ["open-instance survival-amigos"]);

    // Error: se ve y se puede reintentar.
    click(&by_id(&a.ui, "import")[0]);
    a.content.modpack_picked(Some("/tmp/pack.mrpack".into()));
    settle();
    click(&by_id(&a.ui, "filled").last().unwrap().clone());
    assert_eq!(a.rec.take(), ["modpack /tmp/pack.mrpack None a2"]);
    a.content.modpack_done(2, Err(fail()));
    settle();
    assert_eq!(by_id(&a.ui, "error").len(), 1);
    assert_eq!(a.ui.global::<ContentState>().get_modpack_phase(), ModpackPhase::Form);
    click(&by_id(&a.ui, "cancel").last().unwrap().clone());
    assert!(!a.ui.global::<ContentState>().get_modpack_open());
}

/// Servidores con lista y estados.
fn open_servers(a: &App) {
    a.ctl.set_servers(fixtures::sample_servers());
    a.ctl.set_statuses(fixtures::sample_statuses_all());
    a.ui.set_route(Route::Servers);
    settle();
    a.rec.take();
}

fn servidores_lista_y_entrar() {
    let a = app("es");
    open_servers(&a);
    assert_eq!(by_id(&a.ui, "server").len(), 4);
    assert_eq!(a.ui.global::<ServersState>().get_subtitle(), "4 servidores · 3 en línea");
    // «Entrar» lanza su instancia con la dirección; el de sin instancia no.
    click(&by_id(&a.ui, "server-play")[0]);
    assert_eq!(a.rec.take(), ["join survival-amigos mc.amigos.example None"]);
    click(&by_id(&a.ui, "server-play")[3]);
    assert!(a.rec.take().is_empty());
    // «Actualizar estado»: olvida los estados y consulta.
    click(&by_id(&a.ui, "refresh")[0]);
    assert_eq!(a.rec.take(), ["reload-servers"]);
    assert!(a.ui.global::<ServersState>().get_pinging());
    let items = a.ui.global::<ServersState>().get_items();
    assert_eq!(items.row_data(0).unwrap().motd, "Consultando…");
    a.ctl.set_statuses(fixtures::sample_statuses_all());
    settle();
    assert!(!a.ui.global::<ServersState>().get_pinging());
}

fn servidores_dialogo() {
    let a = app("es");
    open_servers(&a);
    click(&by_id(&a.ui, "add")[0]);
    assert!(a.servers.dialog_open());
    // Instancia por defecto: la de Inicio.
    assert_eq!(a.ui.global::<ServersState>().get_dialog_instance(), "survival-amigos");
    // Dirección vacía o inválida: error y no se guarda.
    click(&by_id(&a.ui, "filled").last().unwrap().clone());
    assert!(a.rec.take().is_empty());
    assert_eq!(a.ui.global::<ServersState>().get_dialog_error(), "Escribe una dirección válida (host o host:puerto).");
    type_text(&a.ui, "mi server");
    click(&by_id(&a.ui, "filled").last().unwrap().clone());
    assert!(a.rec.take().is_empty());
    // Corregida, Intro guarda.
    a.ui.global::<ServersState>().set_dialog_address("".into());
    click(&by_id(&a.ui, "field")[0]);
    type_text(&a.ui, "play.nuevo.net");
    assert_eq!(a.ui.global::<ServersState>().get_dialog_error(), "");
    a.ui.global::<ServersState>().invoke_dialog_account_changed("a2".into());
    a.ui.window().dispatch_event(WindowEvent::KeyPressed { text: "\n".into() });
    a.ui.window().dispatch_event(WindowEvent::KeyReleased { text: "\n".into() });
    settle();
    assert_eq!(a.rec.take(), ["save-server None «» play.nuevo.net Some(\"survival-amigos\") Some(\"a2\")"]);
    // Error del núcleo: se ve y el diálogo sigue.
    a.servers.saved(Err(fail()));
    settle();
    assert!(a.servers.dialog_open());
    assert_eq!(by_id(&a.ui, "error").len(), 1);
    click(&by_id(&a.ui, "filled").last().unwrap().clone());
    a.rec.take();
    a.servers.saved(Ok(()));
    settle();
    assert!(!a.servers.dialog_open());
    assert_eq!(a.rec.take(), ["reload-servers"]);

    // Editar: campos con lo guardado.
    click(&by_id(&a.ui, "server-more")[1]);
    click(&by_id(&a.ui, "menu-edit")[0]);
    let st = a.ui.global::<ServersState>();
    assert!(st.get_dialog_editing());
    assert_eq!((st.get_dialog_address().as_str(), st.get_dialog_name().as_str()), ("mc.hypixel.net", "Hypixel"));
    assert_eq!(st.get_dialog_instance(), "vanilla-puro");
    st.invoke_dialog_instance_changed("".into());
    click(&by_id(&a.ui, "filled").last().unwrap().clone());
    assert_eq!(a.rec.take(), ["save-server Some(\"s2\") «Hypixel» mc.hypixel.net None None"]);
}

fn servidores_menu_importar_y_orden() {
    let a = app("es");
    open_servers(&a);
    // Copiar avisa; quitar pide confirmación.
    click(&by_id(&a.ui, "server-more")[0]);
    click(&by_id(&a.ui, "menu-copy")[0]);
    assert_eq!(toast(&a.ui), "Copiado");
    click(&by_id(&a.ui, "server-more")[3]);
    click(&by_id(&a.ui, "menu-remove")[0]);
    assert_eq!(by_id(&a.ui, "confirm").len(), 1);
    assert!(a.rec.take().is_empty());
    click(&by_label(&a.ui, "Quitar").last().unwrap().clone());
    assert_eq!(a.rec.take(), ["remove-server s4 «Servidor de pruebas»"]);
    a.servers.removed(Ok(()));
    assert_eq!(a.rec.take(), ["reload-servers"]);

    // Importar de una instancia.
    click(&by_id(&a.ui, "import")[0]);
    assert_eq!(by_id(&a.ui, "import-menu").len(), 1);
    click(&by_label(&a.ui, "Create: Aeronáutica").last().unwrap().clone());
    assert_eq!(a.rec.take(), ["import-servers create-aeronautica"]);
    a.servers.imported("Create: Aeronáutica", Ok(2));
    assert_eq!(toast(&a.ui), "2 servidores importados de Create: Aeronáutica");
    assert_eq!(a.rec.take(), ["reload-servers"]);
    a.servers.imported("Create: Aeronáutica", Ok(0));
    assert_eq!(toast(&a.ui), "Create: Aeronáutica no tiene servidores nuevos");

    // Arrastrar el primero dos puestos abajo: se ve movido y se guarda.
    let card = &by_id(&a.ui, "server")[0];
    let (p, _) = (card.absolute_position(), card.size());
    let start = LogicalPosition::new(p.x + 20.0, p.y + 42.0);
    let end = LogicalPosition::new(start.x, start.y + 2.0 * 96.0 + 10.0);
    let w = a.ui.window();
    w.dispatch_event(WindowEvent::PointerPressed { position: start, button: PointerEventButton::Left });
    w.dispatch_event(WindowEvent::PointerMoved { position: LogicalPosition::new(start.x, start.y + 100.0) });
    w.dispatch_event(WindowEvent::PointerMoved { position: end });
    w.dispatch_event(WindowEvent::PointerReleased { position: end, button: PointerEventButton::Left });
    settle();
    assert_eq!(a.rec.take(), ["move-server s1 2"]);
    let items = a.ui.global::<ServersState>().get_items();
    let order: Vec<String> = (0..items.row_count()).map(|i| items.row_data(i).unwrap().id.to_string()).collect();
    assert_eq!(order, ["s2", "s3", "s1", "s4"]);
    // Si falla al guardar, aviso y se vuelve a leer.
    a.servers.moved(Err(fail()));
    assert!(a.ui.get_toast_error());
    assert_eq!(a.rec.take(), ["reload-servers"]);
}

#[test]
fn contenido_y_servidores() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .unwrap();
    busqueda_filtros_y_texto();
    mas_resultados_al_bajar();
    ficha_e_instalacion();
    ficha_con_la_instancia_de_inicio_y_respuesta_en_camino();
    destino_desde_una_instancia();
    curseforge_sin_clave();
    modpacks();
    servidores_lista_y_entrar();
    servidores_dialogo();
    servidores_menu_importar_y_orden();
}
