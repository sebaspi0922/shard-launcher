//! Lado Slint del arnés de paridad: pinta cada escena con el renderer por
//! software de Slint, sin ventana ni GPU (reproducible en CI), con los datos
//! de `fixtures` (= fake_bridge.dart), 1280 × 800 a escala 1, y exporta la
//! captura y las cajas de los elementos clave con los mismos nombres que
//! `tools/paridad/flutter/parity_ref_test.dart`.
//!
//!   PARIDAD_OUT=dir PARIDAD_SCENES=all|a,b cargo test -p shard-ui --test paridad
//!
//! Sin `PARIDAD_OUT` escribe en `target/paridad/slint`. La comparación
//! (SSIM, cajas, informe) la hace `tools/paridad.sh`.
//!
//! Un solo `#[test]`: el backend de pruebas de Slint se inicia una vez por
//! proceso.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use shard_ui::controller::{Actions, Controller};
use shard_ui::fixtures;
use shard_ui::instances::Phase;
use shard_ui::accounts::Accounts;
use shard_ui::content::Content;
use shard_ui::servers::Servers;
use shard_ui::launch::Failure;
use shard_ui::notices::CrashNotice;
use shard_ui::prefs::UiPrefs;
use shard_ui::doctor::Doctor;
use shard_ui::first_run::FirstRun;
use shard_ui::settings::Settings;
use shard_ui::skins::Skins;
use shard_ui::theme::Style;
use shard_ui::instance::InstanceDetail;
use shard_ui::inst_content::InstContent;
use shard_ui::console::Console;
use shard_ui::inst_settings::InstSettings;
use shard_ui::backups::Backups; // F3 copias
use shard_ui::{change_version, create_instance, trash};
use shard_ui::{AccountsState, Backdrop, InstanceTab, InstancesState, MainWindow, Route};
use slint::ComponentHandle;
use slint::platform::{PointerEventButton, WindowEvent};

struct NoActions;

impl Actions for NoActions {
    fn open_instance(&self, _: &str) {}
    fn play(&self, _: &str) {}
    fn new_instance(&self) {}
    fn show_trash(&self) {}
}

/// Lo que una escena puede tocar.
struct Ctx<'a> {
    ui: &'a MainWindow,
    ctl: &'a Controller,
    acc: &'a Accounts,
    skins: &'a Skins,
    // Fase 3.
    detail: &'a InstanceDetail,
    // F3 contenido
    inst_content: &'a InstContent,
    console: &'a Console,
    inst_settings: &'a InstSettings,
    // F3 copias
    backups: &'a Backups,
    worlds: &'a shard_ui::worlds::Worlds,
    shots: &'a shard_ui::screenshots::Screenshots,
    // Fase 4.
    content: &'a Content,
    servers: &'a Servers,
    settings: &'a std::rc::Rc<Settings>,
    doctor: &'a Doctor,
    first_run: &'a FirstRun,
    /// Desplazamiento de la referencia de Flutter (`scroll_y`).
    scroll: f32,
}

/// Detalle de `id` en la pestaña `tab` con los datos de ejemplo (como
/// `/instances/:id?tab=`), con la instancia completa ya cargada.
fn detail(c: &Ctx, id: &str, tab: InstanceTab) {
    c.ctl.set_instances(fixtures::sample_instances());
    c.detail.open(id, Some(tab));
    c.detail.loaded(id, Ok(fixtures::detail::sample_detail(id).expect("instancia de ejemplo")));
}

/// Una escena: idioma y qué hacer después de cargar los datos de ejemplo.
struct Scene {
    name: &'static str,
    lang: &'static str,
    /// Idioma de los ajustes (`None` = el del sistema).
    language: Option<&'static str>,
    setup: fn(&Ctx),
}

/// Perfil de ejemplo con sus texturas ya descargadas.
fn sample_skins(c: &Ctx) {
    c.skins.set_profile("a1", c.skins.profile_seq(), Ok(fixtures::sample_profile()));
    for url in [fixtures::SKIN_URL, fixtures::CAPE_1_URL, fixtures::CAPE_2_URL] {
        c.skins.texture_loaded(url, Some(fixtures::sample_texture(url)));
    }
}

/// Inicio con los datos de ejemplo de los goldens (instancias, cuenta de
/// Microsoft, servidores con su estado).
fn home(c: &Controller, ui: &MainWindow) {
    ui.set_route(Route::Home);
    c.set_account_kind(true);
    c.set_instances(fixtures::sample_instances());
    c.set_servers(fixtures::sample_servers());
    c.set_statuses(fixtures::sample_statuses());
}

/// `home_style_*`: el estilo con el desenfoque y oscurecido del golden.
fn styled(c: &Controller, ui: &MainWindow, style: Style) {
    c.set_prefs(UiPrefs { style, blur: 10.0, dim: 0.5, first_run_done: Some(true), ..Default::default() });
    home(c, ui);
}

/// Contenido con la búsqueda respondida (FakeBridge) y los iconos cargados.
fn content_ready(c: &Ctx) {
    c.ui.set_route(Route::Content);
    c.content.shown();
    answer_search(c);
}

/// Responde la búsqueda en curso con los datos de ejemplo.
fn answer_search(c: &Ctx) {
    let s = c.content.search_state();
    c.content.page(s.generation(), Ok(fixtures::search_page(&s.filters, 0, shard_ui::content::PAGE)));
    for p in fixtures::sample_projects() {
        let url = p.icon_url.unwrap();
        c.content.icon_loaded(&url, fixtures::sample_icon(&url));
    }
}

/// `/content?instance=survival-amigos&kind=mod`: como en Flutter, el destino
/// se configura antes de que lleguen las instancias (filtros sin versión).
fn content_target(c: &Ctx) {
    c.content.open_for("survival-amigos", Some("mod"), None);
    c.ctl.set_instances(fixtures::sample_instances());
    c.content.installed_loaded("survival-amigos", fixtures::sample_installed());
    c.content.shown();
    answer_search(c);
}

/// Servidores con la lista y los estados de `FakeBridge`.
fn servers_ready(c: &Ctx) {
    c.ctl.set_instances(fixtures::sample_instances());
    c.ctl.set_servers(fixtures::sample_servers());
    c.ctl.set_statuses(fixtures::sample_statuses_all());
    c.ui.set_route(Route::Servers);
}

/// Ajustes con los datos de fake_bridge.dart, en el desplazamiento de la
/// referencia.
fn settings(c: &Ctx, language: Option<&str>) {
    c.ui.set_route(Route::Settings);
    c.settings.loaded(c.settings.last_load(), Ok(fixtures::sample_values(language)));
    c.settings.java_listed(Ok(fixtures::sample_javas()));
    c.settings.autostart_loaded(false);
    c.settings.scroll_to(c.scroll);
}

const SCENES: &[Scene] = &[
    Scene {
        name: "instances_grid_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            c.ctl.set_phase("create-aeronautica", Phase::Running);
        },
    },
    Scene {
        name: "instances_list_en",
        lang: "en", language: Some("en"),
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            c.ui.global::<InstancesState>().set_grid(false);
            c.ctl.model.borrow_mut().grid = false;
        },
    },
    Scene { name: "instances_empty_es", lang: "es", language: Some("es"), setup: |c| c.ctl.set_instances(vec![]) },
    Scene {
        name: "instances_search_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            type_into(c.ui, "search", "sur");
        },
    },
    Scene {
        name: "instances_noresults_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            type_into(c.ui, "search", "zzz");
        },
    },
    // Fase 2. Panel y diálogos sobre Instancias (como sus referencias).
    Scene {
        name: "accounts_panel_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            by_label(c.ui, "NadiaK").mock_single_click(PointerEventButton::Left);
        },
    },
    Scene {
        name: "ms_login_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            c.ui.global::<AccountsState>().invoke_add_microsoft();
            c.acc.login_code(1, fixtures::sample_device_code());
        },
    },
    Scene {
        name: "offline_account_en",
        lang: "en", language: Some("en"),
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            c.ui.global::<AccountsState>().invoke_add_offline();
        },
    },
    Scene {
        name: "skins_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            c.ui.set_route(Route::Skins);
            sample_skins(c);
        },
    },
    Scene {
        name: "skins_offline_en",
        lang: "en", language: Some("en"),
        setup: |c| {
            c.acc.set_accounts(Ok(fixtures::offline_accounts()));
            c.ui.set_route(Route::Skins);
        },
    },
    Scene { name: "home_es", lang: "es", language: Some("es"), setup: |x| home(x.ctl, x.ui) },
    Scene {
        name: "home_downloading_es",
        lang: "es", language: Some("es"),
        setup: |x| {
            let (c, ui) = (x.ctl, x.ui);
            home(c, ui);
            c.set_phase("survival-amigos", Phase::Downloading { done: 1834, total: 3920 });
        },
    },
    Scene {
        name: "home_error_es",
        lang: "es", language: Some("es"),
        setup: |x| {
            let (c, ui) = (x.ctl, x.ui);
            home(c, ui);
            c.set_failure(
                "survival-amigos",
                Failure::Launch { kind: "java_required".into(), message: "hace falta Java 21: instálala con `lcore java install 21`".into() },
            );
        },
    },
    Scene {
        name: "home_empty_en",
        lang: "en", language: Some("en"),
        setup: |x| {
            let (c, ui) = (x.ctl, x.ui);
            home(c, ui);
            c.set_account(None);
            c.set_instances(vec![]);
        },
    },
    Scene { name: "home_style_graphite_es", lang: "es", language: Some("es"), setup: |x| styled(x.ctl, x.ui, Style::Graphite) },
    Scene { name: "home_style_sebaspi_es", lang: "es", language: Some("es"), setup: |x| styled(x.ctl, x.ui, Style::Sebaspi) },
    Scene { name: "home_style_landscape_es", lang: "es", language: Some("es"), setup: |x| styled(x.ctl, x.ui, Style::Landscape) },
    Scene { name: "home_style_aurora_es", lang: "es", language: Some("es"), setup: |x| styled(x.ctl, x.ui, Style::Aurora) },
    Scene {
        name: "web_home_graphite_es",
        lang: "es", language: Some("es"),
        setup: |x| {
            let (c, ui) = (x.ctl, x.ui);
            ui.global::<Backdrop>().set_hidden(true);
            home(c, ui);
        },
    },
    Scene {
        name: "crash_banner_es",
        lang: "es", language: Some("es"),
        setup: |x| {
            let (c, ui) = (x.ctl, x.ui);
            home(c, ui);
            c.show_crash(CrashNotice { game_id: 2, instance_id: "create-aeronautica".into(), code: Some(1), crash_report: None });
        },
    },
    Scene {
        name: "update_banner_es",
        lang: "es", language: Some("es"),
        setup: |x| {
            let (c, ui) = (x.ctl, x.ui);
            home(c, ui);
            c.update_checked(Some(fixtures::sample_update()), false);
        },
    },
    // Fase 3: detalle de instancia. Cada pestaña o diálogo añade sus escenas
    // debajo de su marca (así las ramas no chocan).
    // F3 menú
    Scene {
        name: "instance_menu_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            detail(c, "survival-amigos", InstanceTab::Worlds);
            c.worlds.listed("survival-amigos", Ok(fixtures::worlds::sample_worlds()));
            c.worlds.datapacks_listed("survival-amigos", "Mundo de verano", Ok(fixtures::worlds::sample_datapacks()));
            by_label(c.ui, "Más opciones").mock_single_click(PointerEventButton::Left);
        },
    },
    // F3 contenido
    Scene {
        name: "instance_content_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            detail(c, "survival-amigos", InstanceTab::Content);
            fixtures::inst_content::deliver(c.inst_content, "survival-amigos");
        },
    },
    // F3 mundos
    Scene {
        name: "instance_worlds_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            detail(c, "survival-amigos", InstanceTab::Worlds);
            c.worlds.listed("survival-amigos", Ok(fixtures::worlds::sample_worlds()));
            c.worlds.datapacks_listed("survival-amigos", "Mundo de verano", Ok(fixtures::worlds::sample_datapacks()));
        },
    },
    // F3 capturas
    Scene {
        name: "instance_screenshots_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            detail(c, "survival-amigos", InstanceTab::Screenshots);
            c.shots.listed("survival-amigos", Ok(fixtures::screenshots::sample_screenshots()));
        },
    },
    // F3 copias
    Scene {
        name: "instance_backups_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            detail(c, "survival-amigos", InstanceTab::Backups);
            c.backups.listed("survival-amigos", Ok(fixtures::backups::sample_backups()));
        },
    },
    Scene {
        name: "backup_restore_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            detail(c, "survival-amigos", InstanceTab::Backups);
            c.backups.listed("survival-amigos", Ok(fixtures::backups::sample_backups()));
            by_id(c.ui, "restore")[1].mock_single_click(PointerEventButton::Left);
        },
    },
    // F3 registros
    Scene {
        name: "instance_logs_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            detail(c, "survival-amigos", InstanceTab::Logs);
            c.ctl.set_phase("survival-amigos", Phase::Running);
            c.console.started(1, "survival-amigos");
            c.console.lines(1, fixtures::logs::sample_log());
        },
    },
    // F3 ajustes
    Scene {
        name: "instance_settings_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            detail(c, "survival-amigos", InstanceTab::Settings);
            c.inst_settings.globals_loaded(fixtures::logs::sample_globals());
        },
    },
    // F3 crear instancia
    Scene {
        name: "create_instance_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            by_id(c.ui, "new")[0].mock_single_click(PointerEventButton::Left);
            create_instance::with(|d| d.versions_loaded(false, Ok(fixtures::versions::sample_versions(false))));
            by_label(c.ui, "Fabric").mock_single_click(PointerEventButton::Left);
            create_instance::with(|d| d.loader_versions_loaded("fabric", "1.21.5", Ok(fixtures::versions::sample_loader_versions())));
        },
    },
    // F3 cambiar versión
    Scene {
        name: "change_version_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            detail(c, "survival-amigos", InstanceTab::Settings);
            by_label(c.ui, "Más opciones").mock_single_click(PointerEventButton::Left);
            by_label(c.ui, "Cambiar versión").mock_single_click(PointerEventButton::Left);
            change_version::with(|d| d.versions_loaded(false, Ok(fixtures::versions::sample_versions(false))));
            by_label(c.ui, "NeoForge").mock_single_click(PointerEventButton::Left);
            change_version::with(|d| d.loader_versions_loaded("neoforge", "1.21.4", Ok(fixtures::versions::sample_loader_versions())));
        },
    },
    // F3 papelera
    Scene {
        name: "trash_en",
        lang: "en", language: Some("en"),
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            by_id(c.ui, "trash")[0].mock_single_click(PointerEventButton::Left);
            trash::with(|t| t.listed(Ok(fixtures::trash::sample_trash())));
        },
    },
    // Fase 4: contenido y servidores.
    Scene { name: "content_search_es", lang: "es", language: Some("es"), setup: content_ready },
    Scene { name: "content_target_es", lang: "es", language: Some("es"), setup: content_target },
    Scene {
        name: "content_curseforge_en",
        lang: "en", language: Some("en"),
        setup: |c| {
            c.content.key_state(false);
            content_ready(c);
            let f = shard_ui::content::Filters { source: "curseforge".into(), ..c.content.filters() };
            c.content.set_filters(f, false);
        },
    },
    Scene {
        name: "project_sheet_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            content_target(c);
            c.content.open_sheet("sodium");
            c.content.versions_loaded(c.content.versions_seq(), Ok(fixtures::sample_versions("sodium", Some("1.21.4"), Some("fabric"))));
        },
    },
    Scene {
        name: "modpack_installing_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            content_ready(c);
            c.content.open_modpack(Some("https://cdn.modrinth.com/data/fo/pack.mrpack".into()), "Fabulously Optimized");
            by_id(c.ui, "filled")[0].mock_single_click(PointerEventButton::Left);
            c.content.set_modpack_progress(87, 212);
        },
    },
    Scene { name: "servers_es", lang: "es", language: Some("es"), setup: servers_ready },
    Scene {
        name: "server_dialog_en",
        lang: "en", language: Some("en"),
        setup: |c| {
            servers_ready(c);
            c.servers.open_dialog(None);
        },
    },
    // Fase 5.
    // Primer arranque. Bienvenida: idioma del sistema (en el test, inglés).
    Scene {
        name: "first_run_welcome_es",
        lang: "en", language: None,
        setup: |c| {
            c.acc.set_accounts(Ok(vec![]));
            c.ctl.set_account(None);
            c.ctl.set_instances(vec![]);
            c.first_run.show(0);
        },
    },
    Scene {
        name: "first_run_import_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            c.acc.set_accounts(Ok(vec![]));
            c.ctl.set_account(None);
            c.ctl.set_instances(vec![]);
            c.first_run.show(1);
            c.first_run.sources(Ok(fixtures::sample_sources()));
            c.first_run.roxy_plan(Ok(fixtures::sample_roxy_plan()));
            // Marcar .minecraft (el golden pulsa su icono).
            let st = c.ui.global::<shard_ui::FirstRunState>();
            st.set_minecraft_selected(true);
            st.invoke_minecraft_toggled(true);
            c.first_run.minecraft_plan(Ok(fixtures::sample_minecraft_plan()));
        },
    },
    Scene {
        name: "first_run_account_en",
        lang: "en", language: Some("en"),
        setup: |c| {
            c.ctl.set_instances(vec![]);
            c.first_run.show(2);
        },
    },
    Scene {
        name: "first_run_done_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            c.first_run.show(3);
        },
    },
    Scene {
        name: "doctor_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            c.doctor.open("survival-amigos");
            c.doctor.report(1, Ok(fixtures::sample_report()));
        },
    },
    Scene { name: "settings_es", lang: "es", language: Some("es"), setup: |c| settings(c, Some("es")) },
    Scene { name: "settings_appearance_es", lang: "es", language: Some("es"), setup: |c| settings(c, Some("es")) },
    Scene { name: "settings_java_es", lang: "es", language: Some("es"), setup: |c| settings(c, Some("es")) },
    Scene {
        name: "settings_integrations_es",
        lang: "es", language: Some("es"),
        setup: |c| {
            c.ui.global::<shard_ui::CurseforgeKey>().set_has_key(true);
            settings(c, Some("es"));
        },
    },
    Scene {
        name: "settings_update_en",
        lang: "en", language: Some("en"),
        setup: |c| {
            c.ctl.update_checked(Some(fixtures::flatpak_update()), false);
            c.ctl.update.borrow_mut().dismissed = true;
            c.ctl.refresh_notices();
            settings(c, Some("en"));
        },
    },
];

fn by_label(ui: &MainWindow, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(ui, label).next().unwrap_or_else(|| panic!("no hay «{label}»"))
}

/// Elementos con `accessible-id` (en orden del árbol).
pub fn by_id(ui: &MainWindow, id: &str) -> Vec<ElementHandle> {
    let id = id.to_owned();
    ui.root_element()
        .query_descendants()
        .match_predicate(move |e| e.accessible_id().as_deref() == Some(id.as_str()))
        .find_all()
}

/// Clic en el elemento y teclear `text` (como un usuario).
fn type_into(ui: &MainWindow, id: &str, text: &str) {
    let field = by_id(ui, id).into_iter().next().expect("campo");
    field.mock_single_click(PointerEventButton::Left);
    for ch in text.chars() {
        let t: slint::SharedString = ch.to_string().into();
        ui.window().dispatch_event(WindowEvent::KeyPressed { text: t.clone() });
        ui.window().dispatch_event(WindowEvent::KeyReleased { text: t });
    }
}

fn rect(e: &ElementHandle) -> [f32; 4] {
    let p = e.absolute_position();
    let s = e.size();
    [p.x, p.y, s.width, s.height]
}

/// Cajas con los nombres de la referencia de Flutter.
fn boxes(ui: &MainWindow, wanted: &[String]) -> BTreeMap<String, [f32; 4]> {
    let mut out = BTreeMap::new();
    // Fase 3 en adelante: cualquier caja de la referencia se busca por su
    // `accessible-id` exacto o, si acaba en `-<n>`, como el n-ésimo elemento
    // con el id sin el número (`world-2` = tercer `world`).
    for name in wanted {
        // `*-btn`: se compara sin el ancho (texto); el id no lo lleva.
        let name = name.as_str();
        let (key, name) = (name.to_owned(), name.strip_suffix("-btn").unwrap_or(name));
        let exact = by_id(ui, name);
        let found = match exact.first() {
            Some(e) => Some(e.clone()),
            None => name.rsplit_once('-').and_then(|(kind, n)| {
                let n: usize = n.parse().ok()?;
                by_id(ui, kind).get(n).cloned()
            }),
        };
        if let Some(e) = found {
            let r = rect(&e);
            if r[3] > 1.0 {
                out.insert(key, r);
            }
        }
    }
    for name in [
        "rail", "titlebar", "pill", "txt-title", "txt-count", "search", "view-toggle", "trash", "new", "empty", "txt-greeting",
        "txt-subtitle", "txt-hero-name", "txt-recent", "hero", "hero-icon", "hero-play-btn", "hero-picker-btn", "hero-failure",
        "no-instances", "session", "running", "servers", "update-banner", "crash-banner",
        // Fase 2.
        "panel", "txt-panel-title", "dialog", "txt-dialog-title", "field", "field-2", "code", "outlined", "filled", "cancel",
        "progress", "chip", "empty-action", "excerpt", "steps", "wizard", "viewer", "change", "source", "reset", "apply", "capes",
        // Fase 4.
        "txt-subtitle", "import", "target", "txt-results", "sheet", "sheet-icon", "txt-sheet-title", "sheet-select",
        "txt-installed", "txt-versions", "latest-play", "add",
    ] {
        if let Some(e) = by_id(ui, name).first() {
            let r = rect(e);
            // Los avisos ocultos miden 0 de alto (Flutter no los cuenta).
            if r[3] > 1.0 {
                out.insert(name.to_owned(), r);
            }
        }
    }
    for (kind, name) in [("filter", "filter"), ("select", "select"), ("kind-pill", "kind")] {
        for (i, e) in by_id(ui, kind).iter().enumerate() {
            out.insert(format!("{name}-{i}"), rect(e));
        }
    }
    for kind in ["account", "action", "stage", "model", "cape", "group", "panel", "seg", "switch", "slider", "option", "swatch", "java", "install", "diag", "step", "choice", "feature", "source", "summary-row", "text-btn"] {
        for (i, e) in by_id(ui, kind).iter().enumerate() {
            out.insert(format!("{kind}-{i}"), rect(e));
        }
    }
    for kind in ["card", "row", "recent", "version", "server"] {
        let items = by_id(ui, kind);
        let icons = by_id(ui, &format!("{kind}-icon"));
        let plays = by_id(ui, &format!("{kind}-play"));
        for (i, e) in items.iter().enumerate() {
            out.insert(format!("{kind}-{i}"), rect(e));
            if let Some(icon) = icons.get(i) {
                out.insert(format!("{kind}-{i}-icon"), rect(icon));
            }
            if let Some(play) = plays.get(i) {
                out.insert(format!("{kind}-{i}-play"), rect(play));
            }
        }
    }
    out
}

/// Un reloj de la referencia de Flutter: segundo del fondo animado
/// (`atmosphere_s`) o del giro de la esfera (`sphere_s`).
fn ref_clock(refs: &Path, scene: &str, key: &str) -> Option<f64> {
    std::fs::read_to_string(refs.join(format!("{scene}.json")))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get(key).and_then(|t| t.as_f64()))
}

/// Nombres de las cajas de la referencia de Flutter de `scene`.
fn ref_box_names(refs: &Path, scene: &str) -> Vec<String> {
    std::fs::read_to_string(refs.join(format!("{scene}.json")))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("boxes").and_then(|b| b.as_object()).map(|o| o.keys().cloned().collect()))
        .unwrap_or_default()
}

fn write_png(path: &Path, buf: &slint::SharedPixelBuffer<slint::Rgba8Pixel>) {
    let file = std::fs::File::create(path).expect("png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), buf.width(), buf.height());
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(buf.as_bytes()).unwrap();
}

fn render(scene: &Scene, refs: &Path, out: &Path) {
    let ui = MainWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    ui.set_animate_background(false);
    ui.set_background_still_at((ref_clock(refs, scene.name, "atmosphere_s").unwrap_or(0.0) * 1000.0).round() as i64);
    let ctl = Controller::new(&ui, scene.lang, Box::new(|| fixtures::NOW), Box::new(NoActions));
    ctl.appearance.set_sphere_time(Some(ref_clock(refs, scene.name, "sphere_s").unwrap_or(0.0)));
    let rec = fixtures::Recorder::default();
    let acc = Accounts::new(&ui, ctl.clone(), Box::new(rec.clone()), Box::new(|| 0));
    let skins = Skins::new(&ui, ctl.clone(), acc.clone(), Box::new(rec.clone()));
    let content = Content::new(&ui, ctl.clone(), Box::new(rec.clone()));
    let servers = Servers::new(&ui, ctl.clone(), acc.clone(), Box::new(rec.clone()));
    {
        let s = servers.clone();
        ctl.on_refresh(move |_| s.refresh());
    }
    let settings = Settings::new(&ui, ctl.clone(), Box::new(rec.clone()), fixtures::TOTAL_MB, fixtures::DATA_DIR, fixtures::CORE_VERSION);
    acc.set_accounts(Ok(fixtures::sample_accounts()));
    // Fase 3: detalle, pestañas y diálogos (cada uno añade aquí su controlador).
    let detail = InstanceDetail::new(&ui, ctl.clone(), Box::new(rec.clone()));
    // F3 contenido
    let inst_content = InstContent::new(&ui, &detail, Box::new(rec.clone()));
    let console = Console::new(&ui, &detail);
    let inst_settings = InstSettings::new(&ui, &detail, Box::new(rec.clone()));
    // F3 copias
    let backups = Backups::new(&ui, detail.clone(), Box::new(rec.clone()));
    backups.set_utc_offset(|_| fixtures::backups::UTC_OFFSET);
    let worlds = shard_ui::worlds::Worlds::new(&ui, &detail, Box::new(rec.clone()), Box::new(|| fixtures::NOW));
    let shots = shard_ui::screenshots::Screenshots::new(&ui, &detail, Box::new(rec.clone()), Box::new(|_| fixtures::screenshots::UTC_OFFSET));
    // F3 crear instancia, cambiar versión y papelera (se alcanzan con `with`).
    let create = create_instance::CreateInstance::new(&ui, ctl.clone(), Box::new(rec.clone()));
    let trash_ctl = trash::Trash::new(&ui, ctl.clone(), Box::new(rec.clone()), Box::new(|| fixtures::NOW));
    create_instance::dialog_hooks(&ctl, &create, &trash_ctl);
    change_version::ChangeVersion::new(&ui, detail.clone(), Box::new(rec.clone()));
    // (fin de la fase 3)
    ui.set_route(Route::Instances);
    ui.show().unwrap();
    let doctor = Doctor::new(&ui, ctl.clone(), Box::new(rec.clone()));
    let first_run = FirstRun::new(&ui, ctl.clone(), Box::new(rec.clone()), scene.language.map(str::to_owned));
    let scroll = ref_clock(refs, scene.name, "scroll_y").unwrap_or(0.0) as f32;
    (scene.setup)(&Ctx { ui: &ui, ctl: &ctl, acc: &acc, skins: &skins, content: &content, servers: &servers, detail: &detail, worlds: &worlds, shots: &shots, backups: &backups, console: &console, inst_settings: &inst_settings, inst_content: &inst_content, settings: &settings, doctor: &doctor, first_run: &first_run, scroll });
    // El fondo animado de Flutter va quieto en el modo de la escena.
    ui.set_animate_background(false);
    // Que terminen las transiciones (150–260 ms) antes de capturar.
    i_slint_backend_testing::testing_backend::mock_elapsed_time(1000);
    let snap = ui.window().take_snapshot().expect("captura");
    write_png(&out.join(format!("{}.png", scene.name)), &snap);
    let json = serde_json::json!({ "boxes": boxes(&ui, &ref_box_names(refs, scene.name)) });
    std::fs::write(out.join(format!("{}.json", scene.name)), serde_json::to_string_pretty(&json).unwrap()).unwrap();
    ui.hide().unwrap();
}

#[test]
fn escenas_de_paridad() {
    i_slint_core_platform_init();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let refs = root.join("tools/paridad/ref");
    let out = std::env::var_os("PARIDAD_OUT").map(PathBuf::from).unwrap_or_else(|| root.join("target/paridad/slint"));
    std::fs::create_dir_all(&out).unwrap();
    let wanted = std::env::var("PARIDAD_SCENES").unwrap_or_else(|_| "all".into());
    let mut done = 0;
    for scene in SCENES {
        if wanted == "all" || wanted.split(',').any(|w| w == scene.name) {
            render(scene, &refs, &out);
            done += 1;
        }
    }
    assert!(done > 0, "ninguna escena coincide con {wanted}");
    println!("{done} escenas en {}", out.display());
}

/// Backend de pruebas con el renderer por software real (no el de métricas
/// falsas), tiempo simulado y sin hilos.
fn i_slint_core_platform_init() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            mock_time: true,
            threading: false,
            renderer_name: Some("software".into()),
        },
    )))
    .expect("plataforma");
}
