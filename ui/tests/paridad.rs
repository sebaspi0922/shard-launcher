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
use shard_ui::{InstancesState, MainWindow};
use slint::ComponentHandle;
use slint::platform::{PointerEventButton, WindowEvent};

struct NoActions;

impl Actions for NoActions {
    fn open_instance(&self, _: &str) {}
    fn play(&self, _: &str) {}
    fn new_instance(&self) {}
    fn show_trash(&self) {}
}

/// Una escena: idioma y qué hacer después de cargar los datos de ejemplo.
struct Scene {
    name: &'static str,
    lang: &'static str,
    setup: fn(&Controller, &MainWindow),
}

const SCENES: &[Scene] = &[
    Scene {
        name: "instances_grid_es",
        lang: "es",
        setup: |c, _| {
            c.set_instances(fixtures::sample_instances());
            c.set_phase("create-aeronautica", Phase::Running);
        },
    },
    Scene {
        name: "instances_list_en",
        lang: "en",
        setup: |c, ui| {
            c.set_instances(fixtures::sample_instances());
            ui.global::<InstancesState>().set_grid(false);
            c.model.borrow_mut().grid = false;
        },
    },
    Scene { name: "instances_empty_es", lang: "es", setup: |c, _| c.set_instances(vec![]) },
    Scene {
        name: "instances_search_es",
        lang: "es",
        setup: |c, ui| {
            c.set_instances(fixtures::sample_instances());
            type_into(ui, "search", "sur");
        },
    },
    Scene {
        name: "instances_noresults_es",
        lang: "es",
        setup: |c, ui| {
            c.set_instances(fixtures::sample_instances());
            type_into(ui, "search", "zzz");
        },
    },
];

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
    for name in ["rail", "titlebar", "pill", "txt-title", "txt-count", "search", "view-toggle", "trash", "new", "empty"] {
        if let Some(e) = by_id(ui, name).first() {
            out.insert(name.to_owned(), rect(e));
        }
    }
    for kind in ["card", "row"] {
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

/// Segundo del fondo animado en la referencia de Flutter.
fn atmosphere_s(refs: &Path, scene: &str) -> f64 {
    std::fs::read_to_string(refs.join(format!("{scene}.json")))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("atmosphere_s").and_then(|t| t.as_f64()))
        .unwrap_or(0.0)
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
    ui.set_background_still_at((atmosphere_s(refs, scene.name) * 1000.0).round() as i64);
    let ctl = Controller::new(&ui, scene.lang, Box::new(|| fixtures::NOW), Box::new(NoActions));
    ctl.set_account(Some(fixtures::ACTIVE_ACCOUNT));
    ui.show().unwrap();
    (scene.setup)(&ctl, &ui);
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
