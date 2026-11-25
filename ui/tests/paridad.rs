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
use shard_ui::launch::Failure;
use shard_ui::notices::CrashNotice;
use shard_ui::prefs::UiPrefs;
use shard_ui::skins::Skins;
use shard_ui::theme::Style;
use shard_ui::{AccountsState, Backdrop, InstancesState, MainWindow, Route};
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
    content: &'a Content,
}

/// Una escena: idioma y qué hacer después de cargar los datos de ejemplo.
struct Scene {
    name: &'static str,
    lang: &'static str,
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

const SCENES: &[Scene] = &[
    Scene {
        name: "instances_grid_es",
        lang: "es",
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            c.ctl.set_phase("create-aeronautica", Phase::Running);
        },
    },
    Scene {
        name: "instances_list_en",
        lang: "en",
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            c.ui.global::<InstancesState>().set_grid(false);
            c.ctl.model.borrow_mut().grid = false;
        },
    },
    Scene { name: "instances_empty_es", lang: "es", setup: |c| c.ctl.set_instances(vec![]) },
    Scene {
        name: "instances_search_es",
        lang: "es",
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            type_into(c.ui, "search", "sur");
        },
    },
    Scene {
        name: "instances_noresults_es",
        lang: "es",
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            type_into(c.ui, "search", "zzz");
        },
    },
    // Fase 2. Panel y diálogos sobre Instancias (como sus referencias).
    Scene {
        name: "accounts_panel_es",
        lang: "es",
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            by_label(c.ui, "NadiaK").mock_single_click(PointerEventButton::Left);
        },
    },
    Scene {
        name: "ms_login_es",
        lang: "es",
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            c.ui.global::<AccountsState>().invoke_add_microsoft();
            c.acc.login_code(1, fixtures::sample_device_code());
        },
    },
    Scene {
        name: "offline_account_en",
        lang: "en",
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            c.ui.global::<AccountsState>().invoke_add_offline();
        },
    },
    Scene {
        name: "skins_es",
        lang: "es",
        setup: |c| {
            c.ui.set_route(Route::Skins);
            sample_skins(c);
        },
    },
    Scene {
        name: "skins_offline_en",
        lang: "en",
        setup: |c| {
            c.acc.set_accounts(Ok(fixtures::offline_accounts()));
            c.ui.set_route(Route::Skins);
        },
    },
    Scene { name: "home_es", lang: "es", setup: |x| home(x.ctl, x.ui) },
    Scene {
        name: "home_downloading_es",
        lang: "es",
        setup: |x| {
            let (c, ui) = (x.ctl, x.ui);
            home(c, ui);
            c.set_phase("survival-amigos", Phase::Downloading { done: 1834, total: 3920 });
        },
    },
    Scene {
        name: "home_error_es",
        lang: "es",
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
        lang: "en",
        setup: |x| {
            let (c, ui) = (x.ctl, x.ui);
            home(c, ui);
            c.set_account(None);
            c.set_instances(vec![]);
        },
    },
    Scene { name: "home_style_graphite_es", lang: "es", setup: |x| styled(x.ctl, x.ui, Style::Graphite) },
    Scene { name: "home_style_sebaspi_es", lang: "es", setup: |x| styled(x.ctl, x.ui, Style::Sebaspi) },
    Scene { name: "home_style_landscape_es", lang: "es", setup: |x| styled(x.ctl, x.ui, Style::Landscape) },
    Scene { name: "home_style_aurora_es", lang: "es", setup: |x| styled(x.ctl, x.ui, Style::Aurora) },
    Scene {
        name: "web_home_graphite_es",
        lang: "es",
        setup: |x| {
            let (c, ui) = (x.ctl, x.ui);
            ui.global::<Backdrop>().set_hidden(true);
            home(c, ui);
        },
    },
    Scene {
        name: "crash_banner_es",
        lang: "es",
        setup: |x| {
            let (c, ui) = (x.ctl, x.ui);
            home(c, ui);
            c.show_crash(CrashNotice { game_id: 2, instance_id: "create-aeronautica".into(), code: Some(1), crash_report: None });
        },
    },
    Scene {
        name: "update_banner_es",
        lang: "es",
        setup: |x| {
            let (c, ui) = (x.ctl, x.ui);
            home(c, ui);
            c.update_checked(Some(fixtures::sample_update()), false);
        },
    },
    // Fase 4: contenido y servidores.
    Scene { name: "content_search_es", lang: "es", setup: content_ready },
    Scene { name: "content_target_es", lang: "es", setup: content_target },
    Scene {
        name: "content_curseforge_en",
        lang: "en",
        setup: |c| {
            c.content.key_state(false);
            content_ready(c);
            let f = shard_ui::content::Filters { source: "curseforge".into(), ..c.content.filters() };
            c.content.set_filters(f, false);
        },
    },
    Scene {
        name: "project_sheet_es",
        lang: "es",
        setup: |c| {
            content_target(c);
            c.content.open_sheet("sodium");
            c.content.versions_loaded(c.content.versions_seq(), Ok(fixtures::sample_versions("sodium", Some("1.21.4"), Some("fabric"))));
        },
    },
    Scene {
        name: "modpack_installing_es",
        lang: "es",
        setup: |c| {
            c.ctl.set_instances(fixtures::sample_instances());
            content_ready(c);
            c.content.open_modpack(Some("https://cdn.modrinth.com/data/fo/pack.mrpack".into()), "Fabulously Optimized");
            by_id(c.ui, "filled")[0].mock_single_click(PointerEventButton::Left);
            c.content.set_modpack_progress(87, 212);
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
fn boxes(ui: &MainWindow) -> BTreeMap<String, [f32; 4]> {
    let mut out = BTreeMap::new();
    for name in [
        "rail", "titlebar", "pill", "txt-title", "txt-count", "search", "view-toggle", "trash", "new", "empty", "txt-greeting",
        "txt-subtitle", "txt-hero-name", "txt-recent", "hero", "hero-icon", "hero-play-btn", "hero-picker-btn", "hero-failure",
        "no-instances", "session", "running", "servers", "update-banner", "crash-banner",
        // Fase 2.
        "panel", "txt-panel-title", "dialog", "txt-dialog-title", "field", "field-2", "code", "outlined", "filled", "cancel",
        "progress", "chip", "empty-action", "viewer", "change", "source", "reset", "apply", "capes",
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
    for kind in ["account", "action", "stage", "model", "cape"] {
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
    let content = Content::new(&ui, ctl.clone(), Box::new(rec));
    acc.set_accounts(Ok(fixtures::sample_accounts()));
    ui.set_route(Route::Instances);
    ui.show().unwrap();
    (scene.setup)(&Ctx { ui: &ui, ctl: &ctl, acc: &acc, skins: &skins, content: &content });
    // El fondo animado de Flutter va quieto en el modo de la escena.
    ui.set_animate_background(false);
    // Que terminen las transiciones (150–260 ms) antes de capturar.
    i_slint_backend_testing::testing_backend::mock_elapsed_time(1000);
    let snap = ui.window().take_snapshot().expect("captura");
    write_png(&out.join(format!("{}.png", scene.name)), &snap);
    let json = serde_json::json!({ "boxes": boxes(&ui) });
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
