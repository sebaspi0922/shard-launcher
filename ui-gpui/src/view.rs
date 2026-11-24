//! Pantalla «Instancias» (rejilla) con el marco de la app: raíl, barra de
//! título propia y fondo «Grafito». Medidas sacadas de
//! `app/lib/ui/instances/instances_page.dart`, `shell/app_shell.dart`,
//! `shell/title_bar.dart` y `widgets/atmosphere.dart`.

use std::sync::Arc;
use std::time::Instant;

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use smallvec::smallvec;

use crate::art::{self, Rgba};
use crate::data::{self, Card};

// Tokens del tema grafito (`AppColors.graphite`).
const BACKGROUND: u32 = 0xFF212121;
const BACKGROUND_DEEP: u32 = 0xFF171717;
const SURFACE1: u32 = 0x0DFFFFFF;
const SURFACE2: u32 = 0x14FFFFFF;
const BORDER: u32 = 0x1FFFFFFF;
const BORDER_STRONG: u32 = 0x33FFFFFF;
const TEXT: u32 = 0xFFECECEC;
const TEXT_SECONDARY: u32 = 0xFFB4B4B4;
const TEXT_MUTED: u32 = 0xFF8F8F8F;
/// `card(surface1)` = surface1 sobre panelBase (0xEB212121), ya mezclado.
const CARD: u32 = 0xEC2D2D2D;
const CARD_HOVER: u32 = 0xEE333333;

pub const FONT: &str = "Inter";
const ICONS: &str = "ShardIcons";
const ICONS_FILL: &str = "ShardIconsFill";

mod icon {
    pub const HOME: &str = "\u{e9b2}";
    pub const INSTANCES: &str = "\u{f720}";
    pub const CONTENT: &str = "\u{e87b}";
    pub const SERVERS: &str = "\u{e875}";
    pub const SKINS: &str = "\u{f19e}";
    pub const SETTINGS: &str = "\u{e8b8}";
    pub const SEARCH: &str = "\u{ef7a}";
    pub const GRID: &str = "\u{e9b0}";
    pub const LIST: &str = "\u{e8e9}";
    pub const DELETE: &str = "\u{e92e}";
    pub const ADD: &str = "\u{e145}";
    pub const SCHEDULE: &str = "\u{efd6}";
    pub const MINIMIZE: &str = "\u{e15b}";
    pub const MAXIMIZE: &str = "\u{e3c6}";
    pub const CLOSE: &str = "\u{e5cd}";
    pub const PERSON: &str = "\u{f0d3}";
}

fn c(argb: u32) -> Hsla {
    let a = ((argb >> 24) & 0xff) as f32 / 255.0;
    let rgb = argb & 0x00FF_FFFF;
    let mut h: Hsla = rgb_u32(rgb).into();
    h.a = a;
    h
}

fn rgb_u32(rgb: u32) -> Rgba8 {
    Rgba8(rgb)
}

/// `gpui::rgb` devuelve `gpui::Rgba`; envoltorio para no chocar con `art::Rgba`.
struct Rgba8(u32);
impl From<Rgba8> for Hsla {
    fn from(v: Rgba8) -> Self {
        gpui_kit::rgb(v.0).into()
    }
}

fn from_art(col: Rgba) -> Hsla {
    c(col.to_u32())
}

fn icon_el(glyph: &'static str, size: f32, color: u32, filled: bool) -> Div {
    div()
        .font_family(if filled { ICONS_FILL } else { ICONS })
        .font_weight(if filled { FontWeight::MEDIUM } else { FontWeight::NORMAL })
        .text_size(px(size))
        .line_height(px(size))
        .text_color(c(color))
        .child(glyph)
}

/// Imagen BGRA de un paisaje ya rasterizado.
fn landscape_image(seed: &str, w: f32, h: f32, scale: f32) -> Arc<RenderImage> {
    let cv = art::landscape(seed, 12.0, w as f64, h as f64, scale as f64);
    let buf = image::RgbaImage::from_raw(cv.w as u32, cv.h as u32, cv.to_bgra()).expect("tamaño");
    Arc::new(RenderImage::new(smallvec![image::Frame::new(buf)]))
}

pub struct InstancesScreen {
    cards: Vec<Card>,
    error: Option<String>,
    images: Vec<(String, f32, Arc<RenderImage>)>,
    search: Entity<InputState>,
    query: String,
    hovered: Option<usize>,
    grid: bool,
    started: Instant,
    /// Fondo animado (como Flutter por defecto) o quieto (modo rendimiento).
    animate: bool,
    first_frame_logged: bool,
    process_start: Instant,
    _subs: Vec<Subscription>,
}

impl InstancesScreen {
    pub fn new(
        cards: Result<Vec<Card>, String>,
        animate: bool,
        process_start: Instant,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Buscar por nombre, versión o loader"));
        let sub = cx.subscribe(&search, |this, state, ev: &InputEvent, cx| {
            if let InputEvent::Change = ev {
                this.query = state.read(cx).value().trim().to_lowercase();
                cx.notify();
            }
        });
        let (cards, error) = match cards {
            Ok(c) => (c, None),
            Err(e) => (Vec::new(), Some(e)),
        };
        Self {
            cards,
            error,
            images: Vec::new(),
            search,
            query: String::new(),
            hovered: None,
            grid: true,
            started: Instant::now(),
            animate,
            first_frame_logged: false,
            process_start,
            _subs: vec![sub],
        }
    }

    fn image_for(&mut self, id: &str, w: f32, scale: f32) -> Arc<RenderImage> {
        if let Some((_, _, img)) = self.images.iter().find(|(i, iw, _)| i == id && (*iw - w).abs() < 0.5) {
            return img.clone();
        }
        let img = landscape_image(id, w, 104.0, scale);
        self.images.retain(|(i, _, _)| i != id);
        self.images.push((id.to_owned(), w, img.clone()));
        img
    }

    fn visible(&self) -> Vec<(usize, &Card)> {
        self.cards
            .iter()
            .enumerate()
            .filter(|(_, i)| {
                if self.query.is_empty() {
                    return true;
                }
                let hay = format!("{} {} {}", i.name, i.mc, data::loader_name(&i.loader)).to_lowercase();
                self.query.split_whitespace().all(|w| hay.contains(w))
            })
            .collect()
    }
}

impl Render for InstancesScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.first_frame_logged {
            self.first_frame_logged = true;
            let start = self.process_start;
            window.on_next_frame(move |_, _| {
                let unix_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                println!("SHARD_FIRST_FRAME unix_ms={unix_ms} desde_main_ms={}", start.elapsed().as_millis());
            });
        }
        if self.animate {
            window.request_animation_frame();
        }
        let t = self.started.elapsed().as_secs_f32();
        let animate = self.animate;
        let scale = window.scale_factor();
        let viewport = window.viewport_size();

        div()
            .size_full()
            .relative()
            .bg(c(BACKGROUND))
            .font_family(FONT)
            .text_color(c(TEXT))
            .child(atmosphere(if animate { t } else { 0.0 }))
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .flex_row()
                    .child(rail())
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(title_bar())
                            .child(self.page(viewport, scale, cx)),
                    ),
            )
    }
}

impl InstancesScreen {
    fn page(&mut self, viewport: Size<Pixels>, scale: f32, cx: &mut Context<Self>) -> impl IntoElement {
        // Ancho útil: ventana − raíl − padding (32 + 32).
        let avail = f32::from(viewport.width) - 76.0 - 64.0;
        let cols = (avail / (300.0 + 16.0)).ceil().max(1.0);
        let card_w = ((avail - (cols - 1.0) * 16.0) / cols).floor();
        let visible: Vec<(usize, Card)> = self.visible().into_iter().map(|(i, c)| (i, c.clone())).collect();
        let total = self.cards.len();
        let now = data::now_secs();
        let mut grid = div().id("grid").flex().flex_row().flex_wrap().gap(px(16.)).pb(px(32.));
        for (idx, card) in visible {
            let img = self.image_for(&card.id, card_w, scale);
            let hovered = self.hovered == Some(idx);
            grid = grid.child(instance_card(idx, &card, img, card_w, hovered, now, cx));
        }
        let body = if let Some(e) = &self.error {
            div().child(format!("Error: {e}")).into_any_element()
        } else {
            grid.into_any_element()
        };

        div()
            .flex_1()
            .flex()
            .flex_col()
            .px(px(32.))
            .pt(px(4.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_end()
                    .h(px(55.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(26.))
                                    .line_height(px(31.2))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Instancias"),
                            )
                            .child(
                                div()
                                    .mt(px(4.))
                                    .text_size(px(13.5))
                                    .line_height(px(20.25))
                                    .text_color(c(TEXT_SECONDARY))
                                    .child(if total == 1 { "1 instancia".to_string() } else { format!("{total} instancias") }),
                            ),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .w(px(300.))
                            .h(px(48.))
                            .flex()
                            .items_center()
                            .px(px(14.))
                            .gap(px(12.))
                            .bg(c(SURFACE1))
                            .border_1()
                            .border_color(c(BORDER))
                            .rounded(px(8.))
                            .text_size(px(13.5))
                            .child(icon_el(icon::SEARCH, 18., TEXT_MUTED, false))
                            .child(Input::new(&self.search).appearance(false).flex_1()),
                    )
                    .child(div().w(px(12.)))
                    .child(segmented(self.grid, cx))
                    .child(div().w(px(12.)))
                    .child(
                        div()
                            .id("trash")
                            .mb(px(2.))
                            .size(px(40.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_1()
                            .border_color(c(BORDER_STRONG))
                            .rounded(px(8.))
                            .hover(|s| s.bg(c(SURFACE2)))
                            .child(icon_el(icon::DELETE, 20., TEXT_SECONDARY, false)),
                    )
                    .child(div().w(px(12.)))
                    .child(
                        div()
                            .id("new")
                            .h(px(44.))
                            .px(px(16.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .bg(c(0xFFFFFFFF))
                            .hover(|s| s.bg(c(0xFFE6E6E6)))
                            .rounded(px(8.))
                            .text_color(c(0xFF0D0D0D))
                            .text_size(px(13.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(icon_el(icon::ADD, 20., 0xFF0D0D0D, false))
                            .child("Nueva instancia"),
                    ),
            )
            .child(div().h(px(24.)))
            .child(div().id("scroll").flex_1().overflow_y_scroll().child(body))
    }
}

fn segmented(grid: bool, cx: &mut Context<InstancesScreen>) -> impl IntoElement {
    let seg = |id: &'static str, glyph: &'static str, sel: bool, value: bool| {
        div()
            .id(id)
            .w(px(56.))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .when(sel, |d| d.bg(c(0x29FFFFFF)))
            .child(icon_el(glyph, 18., if sel { TEXT } else { TEXT_SECONDARY }, false))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.grid = value;
                cx.notify();
            }))
    };
    div()
        .h(px(32.))
        .flex()
        .flex_row()
        .border_1()
        .border_color(c(BORDER_STRONG))
        .rounded(px(8.))
        .overflow_hidden()
        .child(seg("seg-grid", icon::GRID, grid, true))
        .child(div().w(px(1.)).h_full().bg(c(BORDER_STRONG)))
        .child(seg("seg-list", icon::LIST, !grid, false))
}

fn instance_card(
    idx: usize,
    card: &Card,
    img: Arc<RenderImage>,
    w: f32,
    hovered: bool,
    now: i64,
    cx: &mut Context<InstancesScreen>,
) -> impl IntoElement {
    let (from, to) = art::icon_colors(&card.id);
    div()
        .id(("card", idx))
        .w(px(w))
        .h(px(232.))
        .relative()
        .rounded(px(14.))
        .overflow_hidden()
        .bg(c(if hovered { CARD_HOVER } else { CARD }))
        .border_1()
        .border_color(c(if hovered { BORDER_STRONG } else { BORDER }))
        .on_hover(cx.listener(move |this, h: &bool, _, cx| {
            this.hovered = if *h { Some(idx) } else if this.hovered == Some(idx) { None } else { this.hovered };
            cx.notify();
        }))
        .child(img_el(img, w))
        .child(
            div()
                .absolute()
                .top(px(80.))
                .left(px(16.))
                .size(px(48.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(12.48))
                .border_1()
                .border_color(c(0x29FFFFFF))
                .bg(linear_gradient(135., linear_color_stop(from_art(from), 0.), linear_color_stop(from_art(to), 1.)))
                .shadow(vec![BoxShadow {
                    color: c(0x59000000),
                    offset: point(px(0.), px(3.84)),
                    blur_radius: px(14.4),
                    spread_radius: px(0.),
                    inset: false,
                }])
                .text_size(px(17.28))
                .line_height(px(17.28))
                .font_weight(FontWeight::BOLD)
                .text_color(c(0xF2FFFFFF))
                .child(art::initials(&card.name)),
        )
        .child(
            div()
                .absolute()
                .top(px(136.))
                .left(px(16.))
                .right(px(16.))
                .bottom(px(16.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(15.))
                        .line_height(px(20.25))
                        .font_weight(FontWeight::SEMIBOLD)
                        .truncate()
                        .child(card.name.clone()),
                )
                .child(
                    div()
                        .mt(px(3.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(div().size(px(7.)).rounded(px(2.)).bg(c(0xFF00_0000 | data::loader_color(&card.loader))))
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(17.4))
                                .text_color(c(TEXT_SECONDARY))
                                .child(format!("{} · {}", card.mc, data::loader_name(&card.loader))),
                        ),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .text_size(px(12.))
                        .line_height(px(17.4))
                        .text_color(c(TEXT_MUTED))
                        .child(icon_el(icon::SCHEDULE, 14., TEXT_MUTED, false))
                        .child(div().ml(px(4.)).child(data::playtime(card.playtime_seconds)))
                        .child(div().flex_1())
                        .child(data::relative(card.last_played, now)),
                ),
        )
}

fn img_el(img: Arc<RenderImage>, w: f32) -> impl IntoElement {
    gpui_kit::img(ImageSource::Render(img)).w(px(w)).h(px(104.))
}

fn rail() -> impl IntoElement {
    let item = |glyph: &'static str, sel: bool| {
        div()
            .w(px(76.))
            .h(px(48.))
            .my(px(3.))
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .when(sel, |d| {
                d.child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(12.))
                        .bottom(px(12.))
                        .w(px(3.))
                        .bg(c(0xFFFFFFFF))
                        .rounded_r(px(3.)),
                )
            })
            .child(
                div()
                    .w(px(48.))
                    .h(px(44.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(12.))
                    // `accentContainer.withValues(alpha: 0.75)`: blanco al 75 %.
                    .when(sel, |d| d.bg(c(0xBFFFFFFF)))
                    .when(!sel, |d| d.hover(|s| s.bg(c(SURFACE2))))
                    .child(icon_el(glyph, 24., if sel { 0xFFFFFFFF } else { TEXT_SECONDARY }, sel)),
            )
    };
    div()
        .w(px(76.))
        .h_full()
        .flex()
        .flex_col()
        .items_center()
        .bg(c(0x8C171717))
        .border_r_1()
        .border_color(c(BORDER))
        .child(
            div()
                .w_full()
                .h(px(72.))
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
                .child(brand_mark(30.)),
        )
        .child(div().h(px(8.)))
        .child(item(icon::HOME, false))
        .child(item(icon::INSTANCES, true))
        .child(item(icon::CONTENT, false))
        .child(item(icon::SERVERS, false))
        .child(item(icon::SKINS, false))
        .child(div().flex_1())
        .child(item(icon::SETTINGS, false))
        .child(div().h(px(8.)))
        .child(div().w(px(36.)).h(px(1.)).bg(c(BORDER)))
        .child(div().h(px(12.)))
        .child(
            // Sin cuenta: avatar vacío con el «+» del acento (como Flutter con
            // la carpeta sembrada, que no tiene cuentas).
            div()
                .p(px(6.))
                .child(
                    div()
                        .size(px(40.))
                        .relative()
                        .rounded(px(9.6))
                        .bg(c(SURFACE2))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon_el(icon::PERSON, 22., TEXT_MUTED, false))
                        .child(
                            div()
                                .absolute()
                                .right(px(-3.))
                                .bottom(px(-3.))
                                .size(px(16.))
                                .rounded_full()
                                .bg(c(0xFFFFFFFF))
                                .border_2()
                                .border_color(c(BACKGROUND_DEEP))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(icon_el(icon::ADD, 11., 0xFF0D0D0D, false)),
                        ),
                ),
        )
        .child(div().h(px(16.)))
}

/// Marca (`BrandMarkPainter`): cuatro triángulos en tonos del acento blanco.
fn brand_mark(size: f32) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let k = size / 256.0 * 1.14;
            let o = bounds.center();
            let p = |x: f32, y: f32| point(o.x + px((x - 128.0) * k), o.y + px((y - 128.0) * k));
            let tris: [([(f32, f32); 3], f64); 4] = [
                ([(206., 30.), (98., 30.), (158., 122.)], 88.),
                ([(98., 30.), (50., 122.), (158., 122.)], 72.),
                ([(50., 226.), (98., 134.), (158., 226.)], 56.),
                ([(158., 226.), (206., 134.), (98., 134.)], 42.),
            ];
            for (pts, t) in tris {
                let mut b = PathBuilder::fill();
                b.move_to(p(pts[0].0, pts[0].1));
                b.line_to(p(pts[1].0, pts[1].1));
                b.line_to(p(pts[2].0, pts[2].1));
                b.close();
                if let Ok(path) = b.build() {
                    window.paint_path(path, from_art(art::tone(0.0, 0.0, t)));
                }
            }
        },
    )
    .size(px(size))
}

fn title_bar() -> impl IntoElement {
    let button = |id: &'static str, glyph: &'static str, size: f32, danger: bool| {
        div()
            .id(id)
            .w(px(40.))
            .h(px(30.))
            .mx(px(2.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(8.))
            .hover(move |s| s.bg(c(if danger { 0xFFE5484D } else { SURFACE2 })))
            .child(icon_el(glyph, size, TEXT_SECONDARY, false))
    };
    div()
        .h(px(44.))
        .flex()
        .flex_row()
        .items_center()
        .child(
            div()
                .flex_1()
                .h_full()
                .on_mouse_down(MouseButton::Left, |ev: &MouseDownEvent, window, _| {
                    if ev.click_count == 2 {
                        window.zoom_window();
                    } else {
                        window.start_window_move();
                    }
                }),
        )
        .child(button("min", icon::MINIMIZE, 18., false).on_click(|_, window, _| window.minimize_window()))
        .child(button("max", icon::MAXIMIZE, 16., false).on_click(|_, window, _| window.zoom_window()))
        .child(
            // Cerrar = a la bandeja (comportamiento «tray» de Flutter).
            button("close", icon::CLOSE, 18., true).on_click(|_, window, cx| {
                window.remove_window();
                crate::window_closed(cx);
            }),
        )
        .child(div().w(px(4.)))
}

/// Fondo «Grafito» (`Atmosphere`): círculos finos y puntos que van y vuelven
/// en 18 s.
fn atmosphere(t: f32) -> impl IntoElement {
    // (x, fx, y, fy, sx, lx, sy, ly, size, color, radius(-1 = círculo), outline)
    #[rustfmt::skip]
    const SHAPES: [(f32, f32, f32, f32, f32, f32, f32, f32, f32, u32, f32, bool); 12] = [
        (-70., 0., -40., 0., 0.4, 0., 0., 1., 340., 0x66AEBCFF, -1., true),
        (70., 0., 0., 0.38, -0.3, 0., 0., 1., 220., 0x55C4B6FF, -1., true),
        (-210., 1., -250., 1., 1., 0., 0., -1., 260., 0x508FD4FF, -1., true),
        (0., 0.55, 18., 0., 0., -0.4, 0.2, 0., 92., 0x38FFFFFF, -1., true),
        (36., 0., -120., 1., 0., 0.2, 0.3, 0., 14., 0xFF9EB0FF, 3., false),
        (0., 0.22, 64., 0., 1., 0., 0., -0.3, 8., 0xFFD2C6FF, -1., false),
        (-120., 1., 0., 0.22, -0.5, 0., 0., 1., 10., 0xFF8FD4FF, -1., false),
        (0., 0.4, -70., 1., 0., 0.5, -0.2, 0., 6., 0x88FFFFFF, 1., false),
        (180., 0., 0., 0.62, -0.6, 0., 0., 0., 18., 0xFFAEBCFF, -1., true),
        (0., 0.72, 120., 0., 0.5, 0., 0., 0.4, 150., 0x30FFFFFF, -1., true),
        (0., 0.12, 0., 0.18, 0., -0.3, 0.4, 0., 7., 0xFFC4B6FF, -1., false),
        (-64., 1., 0., 0.72, 0.2, 0., 0., -0.3, 11., 0xAA8FD4FF, 2., false),
    ];
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let phase = (t / 18.0).fract() * 2.0;
            let swing = if phase < 1.0 { phase } else { 2.0 - phase };
            let sway = -28.0 + 62.0 * swing;
            let lift = 30.0 - 72.0 * swing;
            let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
            for (x, fx, y, fy, sx, lx, sy, ly, size, color, radius, outline) in SHAPES {
                let left = x + fx * w + sx * sway + lx * lift;
                let top = y + fy * h + sy * sway + ly * lift;
                let b = Bounds::new(bounds.origin + point(px(left), px(top)), gpui_kit::size(px(size), px(size)));
                let r = if radius < 0.0 { size / 2.0 } else { radius };
                if outline {
                    window.paint_quad(quad(b, px(r), transparent_black(), px(1.), c(color), BorderStyle::Solid));
                } else {
                    window.paint_quad(fill(b, c(color)).corner_radii(px(r)));
                }
            }
        },
    )
    .absolute()
    .inset_0()
}
