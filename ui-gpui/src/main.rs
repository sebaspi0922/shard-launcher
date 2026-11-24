//! Spike de Shard en GPUI (fase 0 de `PLAN.md`): pantalla «Instancias» con
//! `lcore` directo, ventana sin decoraciones de 1280×800 y bandeja con
//! `gpui-tray` (abrir / ocultar / salir).
//!
//! Uso:
//!   shard-gpui --seed DIR            escribe los datos de prueba y sale
//!   shard-gpui [--data-dir DIR] [--background]
//!     --background  arranca solo en la bandeja (como `--background` en Flutter)
//!   `performance: true` en `<datos>/ui.json` deja el fondo quieto (como Flutter).

mod art;
mod data;
mod view;

use std::borrow::Cow;
use std::path::PathBuf;
use std::time::Instant;

use gpui_kit::*;
use gpui_tray::{Icon, Tray};

actions!(shard, [Open, Hide, Quit]);

struct Args {
    data_dir: Option<PathBuf>,
    seed: Option<PathBuf>,
    background: bool,
}

fn parse_args() -> Args {
    let mut a = Args { data_dir: None, seed: None, background: false };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--data-dir" => a.data_dir = it.next().map(PathBuf::from),
            "--seed" => a.seed = it.next().map(PathBuf::from),
            "--background" => a.background = true,
            other => {
                eprintln!("argumento desconocido: {other}");
                std::process::exit(2);
            }
        }
    }
    if a.data_dir.is_none() {
        a.data_dir = std::env::var_os("SHARD_DATA_DIR").map(PathBuf::from);
    }
    a
}

/// Estado global: ventana principal (si está abierta), datos y bandeja.
struct AppState {
    window: Option<AnyWindowHandle>,
    cards: Result<Vec<data::Card>, String>,
    still: bool,
    process_start: Instant,
    tray: Option<Tray>,
}
impl Global for AppState {}

fn main() {
    let process_start = Instant::now();
    let args = parse_args();
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().expect("tokio");

    if let Some(dir) = args.seed {
        rt.block_on(data::seed(dir.clone())).expect("no se pudo sembrar");
        println!("sembrado en {}", dir.display());
        return;
    }

    // Lectura directa de `lcore` (lo que en Flutter pasa por el bridge).
    let dir = match args.data_dir {
        Some(d) => d,
        None => lcore::paths::Paths::default_root().expect("carpeta de datos"),
    };
    let still = data::performance_mode(&dir);
    let cards = rt.block_on(data::load(dir)).map_err(|e| e.to_string());

    gpui_kit::application()
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            cx.text_system()
                .add_fonts(vec![
                    Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice()),
                    Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Medium.ttf").as_slice()),
                    Cow::Borrowed(include_bytes!("../assets/fonts/Inter-SemiBold.ttf").as_slice()),
                    Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Bold.ttf").as_slice()),
                    Cow::Borrowed(include_bytes!("../assets/fonts/ShardIcons.ttf").as_slice()),
                    Cow::Borrowed(include_bytes!("../assets/fonts/ShardIconsFill.ttf").as_slice()),
                ])
                .expect("fuentes");
            cx.set_global(AppState { window: None, cards, still, process_start, tray: None });

            cx.on_action(|_: &Open, cx| open_window(cx))
                .on_action(|_: &Hide, cx| hide_window(cx))
                .on_action(|_: &Quit, cx| {
                    if let Some(tray) = cx.global::<AppState>().tray.clone() {
                        tray.close(cx).ok();
                    }
                    cx.quit();
                });

            match build_tray(cx) {
                Ok(tray) => {
                    cx.global_mut::<AppState>().tray = Some(tray);
                    println!("SHARD_TRAY ok");
                }
                Err(e) => eprintln!("SHARD_TRAY error: {e}"),
            }
            let no_tray = cx.global::<AppState>().tray.is_none();
            if !args.background || no_tray {
                open_window(cx);
            }
        });
}

fn open_window(cx: &mut App) {
    if let Some(handle) = cx.global::<AppState>().window {
        if handle.update(cx, |_, window, _| window.activate_window()).is_ok() {
            return;
        }
    }
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(1280.), px(800.)), cx))),
        titlebar: Some(TitlebarOptions { title: Some("Shard".into()), appears_transparent: true, ..Default::default() }),
        window_decorations: Some(WindowDecorations::Client),
        window_min_size: Some(size(px(960.), px(600.))),
        window_background: WindowBackgroundAppearance::Opaque,
        app_id: Some("shard-gpui".into()),
        ..Default::default()
    };
    let state = cx.global::<AppState>();
    let cards = state.cards.clone();
    let animate = !state.still;
    let start = state.process_start;
    match gpui_kit::open_window(options, cx, |window, cx| {
        cx.new(|cx| view::InstancesScreen::new(cards, animate, start, window, cx))
    }) {
        Ok((handle, _)) => cx.global_mut::<AppState>().window = Some(handle),
        Err(e) => {
            eprintln!("no se pudo abrir la ventana: {e}");
            cx.quit();
        }
    }
}

/// Ocultar = cerrar la ventana; la app sigue viva en la bandeja y la vuelve a
/// abrir con «Abrir» (GPUI no tiene «hide» de ventana en Linux).
fn hide_window(cx: &mut App) {
    if let Some(handle) = cx.global_mut::<AppState>().window.take() {
        if let Err(e) = handle.update(cx, |_, window, _| window.remove_window()) {
            eprintln!("no se pudo ocultar la ventana: {e}");
        }
    }
}

/// Lo llama la propia ventana al cerrarse desde su barra de título: dentro de
/// su evento `handle.update` falla (la ventana está en uso), así que se cierra
/// con su `Window` y aquí solo se olvida el handle.
pub fn window_closed(cx: &mut App) {
    cx.global_mut::<AppState>().window = None;
}

fn build_tray(cx: &mut App) -> gpui_tray::Result<Tray> {
    let png = image::load_from_memory(include_bytes!("../../app/assets/brand/shard-512.png"))
        .map_err(|e| gpui_tray::Error::InvalidIcon(e.to_string()))?
        .resize(32, 32, image::imageops::FilterType::Triangle)
        .to_rgba8();
    let icon = Icon::from_rgba(png.into_raw(), 32, 32).build()?;
    Tray::builder()
        .icon(icon)
        .title("Shard")
        .tooltip("Shard")
        .on_activate(Open)
        .menu(|_| {
            vec![
                MenuItem::action("Abrir Shard", Open),
                MenuItem::action("Ocultar", Hide),
                MenuItem::separator(),
                MenuItem::action("Salir", Quit),
            ]
        })
        .build(cx)
}
