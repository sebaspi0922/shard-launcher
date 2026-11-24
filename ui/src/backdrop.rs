//! Fondos que Slint no sabe pintar y se rasterizan aquí, en CPU
//! (`app/lib/ui/shell/backdrop.dart`, `widgets/ascii_sphere.dart` y
//! `widgets/art.dart` `Backdrop`):
//!
//! - paisaje de bloques o imagen propia, con desenfoque (Slint no tiene
//!   filtro de blur: se desenfoca una vez al cambiar tamaño o ajuste);
//! - los dos halos del estilo «aurora»;
//! - halo y esfera ASCII del estilo «sebaspi» (la esfera, a 15 fps).
//!
//! Todo a escala física. El oscurecido va en Slint (un degradado encima).

use std::path::Path;

use ab_glyph::{Font, FontRef, PxScale, point};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

use crate::art;

/// Semilla del paisaje de fondo (`kBackdropLandscapeSeed`).
pub const LANDSCAPE_SEED: &str = "shard-ocaso";

/// Imagen RGBA sin premultiplicar.
pub struct Pixels {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl Pixels {
    pub fn image(&self) -> Image {
        Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&self.px, self.w as u32, self.h as u32))
    }
}

fn physical(v: f64, scale: f64) -> usize {
    ((v * scale).round() as usize).max(1)
}

/// Tamaños de las tres pasadas de caja que aproximan una gaussiana de
/// `sigma` (como el desenfoque por software de Skia).
fn box_sizes(sigma: f64) -> [usize; 3] {
    let n = 3.0;
    let ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut wl = ideal.floor() as i64;
    if wl % 2 == 0 {
        wl -= 1;
    }
    let wu = wl + 2;
    let m = ((12.0 * sigma * sigma - n * (wl * wl) as f64 - 4.0 * n * wl as f64 - 3.0 * n) / (-4.0 * wl as f64 - 4.0)).round() as i64;
    let mut out = [0; 3];
    for (i, o) in out.iter_mut().enumerate() {
        *o = (if (i as i64) < m { wl } else { wu }).max(1) as usize;
    }
    out
}

/// Una pasada de caja horizontal sobre `src` (canales RGB intercalados),
/// con los bordes extendidos (`TileMode.clamp`).
fn box_pass(src: &[f32], dst: &mut [f32], w: usize, h: usize, radius: usize) {
    let r = radius as isize;
    let norm = 1.0 / (2 * radius + 1) as f32;
    for y in 0..h {
        let row = &src[y * w * 3..(y + 1) * w * 3];
        let out = &mut dst[y * w * 3..(y + 1) * w * 3];
        let at = |x: isize, c: usize| row[(x.clamp(0, w as isize - 1) as usize) * 3 + c];
        let mut acc = [0f32; 3];
        for (c, a) in acc.iter_mut().enumerate() {
            for x in -r..=r {
                *a += at(x, c);
            }
        }
        for x in 0..w as isize {
            for c in 0..3 {
                out[x as usize * 3 + c] = acc[c] * norm;
                acc[c] += at(x + r + 1, c) - at(x - r, c);
            }
        }
    }
}

fn transpose(src: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut out = vec![0f32; src.len()];
    for y in 0..h {
        for x in 0..w {
            for c in 0..3 {
                out[(x * h + y) * 3 + c] = src[(y * w + x) * 3 + c];
            }
        }
    }
    out
}

/// Desenfoque gaussiano (aprox. de tres cajas) de una imagen opaca.
pub fn blur(p: &mut Pixels, sigma: f64) {
    if sigma <= 0.5 {
        return;
    }
    let (w, h) = (p.w, p.h);
    let mut a: Vec<f32> = p.px.as_chunks::<4>().0.iter().flat_map(|c| [c[0] as f32, c[1] as f32, c[2] as f32]).collect();
    let mut b = vec![0f32; a.len()];
    let sizes = box_sizes(sigma);
    for s in sizes {
        box_pass(&a, &mut b, w, h, (s - 1) / 2);
        std::mem::swap(&mut a, &mut b);
    }
    let mut t = transpose(&a, w, h);
    let mut u = vec![0f32; t.len()];
    for s in sizes {
        box_pass(&t, &mut u, h, w, (s - 1) / 2);
        std::mem::swap(&mut t, &mut u);
    }
    let a = transpose(&t, h, w);
    for (i, c) in p.px.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        for k in 0..3 {
            c[k] = a[i * 3 + k].round().clamp(0.0, 255.0) as u8;
        }
        c[3] = 255;
    }
}

/// Paisaje de fondo del estilo «landscape» a pantalla completa.
pub fn landscape(w: f64, h: f64, scale: f64, sigma: f64) -> Pixels {
    let cv = art::landscape(LANDSCAPE_SEED, 26.0, w, h, scale);
    let mut px = cv.to_bgra();
    for p in px.as_chunks_mut::<4>().0.iter_mut() {
        p.swap(0, 2);
        // Bajo el paisaje hay un fondo opaco: la viñeta ya va mezclada.
        p[3] = 255;
    }
    let mut p = Pixels { w: cv.w, h: cv.h, px };
    blur(&mut p, sigma * scale);
    p
}

/// Imagen propia recortada para cubrir la ventana (`BoxFit.cover`) y
/// desenfocada.
pub fn photo(path: &Path, w: f64, h: f64, scale: f64, sigma: f64) -> Result<Pixels, image::ImageError> {
    let img = image::ImageReader::open(path)?.with_guessed_format()?.decode()?.to_rgba8();
    let (pw, ph) = (physical(w, scale) as u32, physical(h, scale) as u32);
    let (iw, ih) = (img.width() as f64, img.height() as f64);
    let k = (pw as f64 / iw).max(ph as f64 / ih);
    let (cw, ch) = ((pw as f64 / k).round().max(1.0) as u32, (ph as f64 / k).round().max(1.0) as u32);
    let (cx, cy) = ((img.width().saturating_sub(cw)) / 2, (img.height().saturating_sub(ch)) / 2);
    let crop = image::imageops::crop_imm(&img, cx, cy, cw.min(img.width()), ch.min(img.height())).to_image();
    let out = image::imageops::resize(&crop, pw, ph, image::imageops::FilterType::Triangle);
    let mut p = Pixels { w: pw as usize, h: ph as usize, px: out.into_raw() };
    // Lo transparente se ve sobre el fondo profundo (aquí, negro).
    for c in p.px.as_chunks_mut::<4>().0.iter_mut() {
        let a = c[3] as u32;
        for v in &mut c[..3] {
            *v = (*v as u32 * a / 255) as u8;
        }
        c[3] = 255;
    }
    blur(&mut p, sigma * scale);
    Ok(p)
}

/// Color de 0–255 + alfa 0–1 de un ARGB.
fn rgba(argb: u32) -> [f64; 4] {
    [((argb >> 16) & 0xff) as f64, ((argb >> 8) & 0xff) as f64, (argb & 0xff) as f64, ((argb >> 24) & 0xff) as f64 / 255.0]
}

/// Mezcla `src` (alfa `a`) sobre `dst` opaco.
fn over(dst: &mut [f64; 3], src: [f64; 3], a: f64) {
    for k in 0..3 {
        dst[k] = src[k] * a + dst[k] * (1.0 - a);
    }
}

/// Degradado radial de Flutter (`RadialGradient`): centro en `Alignment`,
/// radio como fracción del lado más corto; de `color` (alfa `a0`) a negro
/// transparente, interpolando sin premultiplicar como Skia.
struct Radial {
    cx: f64,
    cy: f64,
    r: f64,
    color: [f64; 3],
    a0: f64,
    to_black: bool,
}

impl Radial {
    fn sample(&self, x: f64, y: f64) -> ([f64; 3], f64) {
        let t = (((x - self.cx).powi(2) + (y - self.cy).powi(2)).sqrt() / self.r).min(1.0);
        let c = if self.to_black { self.color.map(|v| v * (1.0 - t)) } else { self.color };
        (c, self.a0 * (1.0 - t))
    }
}

fn paint(w: usize, h: usize, base: u32, layers: &[Radial]) -> Pixels {
    let b = rgba(base);
    let mut px = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
            let mut c = [b[0], b[1], b[2]];
            for l in layers {
                let (s, a) = l.sample(fx, fy);
                over(&mut c, s, a);
            }
            px.extend_from_slice(&[c[0].round() as u8, c[1].round() as u8, c[2].round() as u8, 255]);
        }
    }
    Pixels { w, h, px }
}

/// Fondo «aurora» (`Backdrop` de art.dart): fondo profundo y dos halos del
/// acento, el segundo con el tono girado 50°.
pub fn aurora(glow: u32, deep: u32, w: f64, h: f64, scale: f64) -> Pixels {
    let (pw, ph) = (physical(w, scale), physical(h, scale));
    let (fw, fh) = (pw as f64, ph as f64);
    let short = fw.min(fh);
    let at = |ax: f64, ay: f64| (fw / 2.0 * (1.0 + ax), fh / 2.0 * (1.0 + ay));
    let g = rgba(glow);
    let g2 = rgba(crate::theme::rotate_hue(glow, 50.0));
    let (c1x, c1y) = at(-0.85, -1.1);
    let (c2x, c2y) = at(1.15, 1.2);
    paint(
        pw,
        ph,
        deep,
        &[
            Radial { cx: c1x, cy: c1y, r: 1.25 * short, color: [g[0], g[1], g[2]], a0: 0.20, to_black: true },
            Radial { cx: c2x, cy: c2y, r: 1.1 * short, color: [g2[0], g2[1], g2[2]], a0: 0.10, to_black: true },
        ],
    )
}

/// Radio y centro de la esfera (lógicos).
pub fn sphere_geometry(w: f64, h: f64) -> (f64, f64, f64) {
    ((h * 0.40).min(w * 0.30), w * 0.70, h * 0.53)
}

/// Fondo del estilo «sebaspi» sin la esfera: negro y halo del acento.
pub fn sebaspi_base(accent: u32, w: f64, h: f64, scale: f64) -> Pixels {
    let (pw, ph) = (physical(w, scale), physical(h, scale));
    let (r, cx, cy) = sphere_geometry(w, h);
    let a = rgba(accent);
    paint(
        pw,
        ph,
        0xFF000000,
        &[Radial { cx: cx * scale, cy: cy * scale, r: r * 1.6 * scale, color: [a[0], a[1], a[2]], a0: 0.10, to_black: false }],
    )
}

/// Rampa de densidad, de vacío a lleno.
const RAMP: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
/// Segundos por vuelta.
pub const TURN_S: f64 = 90.0;
/// Fotogramas por segundo del giro.
pub const SPHERE_FPS: u32 = 15;
const FONT_PX: f32 = 12.0;
const LINE_HEIGHT: f32 = 1.15;

static MONO: &[u8] = include_bytes!("../fonts/JetBrainsMono-Regular.ttf");

/// Esfera ASCII que gira (`_SpherePainter`): una capa transparente del
/// tamaño de la esfera con los glifos del acento. Como Flutter, los glifos
/// se pintan una vez en un atlas y cada celda lo muestrea con filtro
/// bilineal (`drawAtlas` con `FilterQuality.medium`).
pub struct Sphere {
    font: FontRef<'static>,
    /// Celda lógica (ancho, alto) y línea base dentro de la celda.
    cell: (f64, f64),
    baseline: f32,
    /// Atlas de cobertura (escala física, ancho, alto, cobertura).
    atlas: Option<(f64, usize, usize, Vec<f32>)>,
}

/// Una capa ya pintada: imagen y su esquina en píxeles lógicos.
pub struct SphereFrame {
    pub pixels: Pixels,
    pub x: f64,
    pub y: f64,
}

impl Default for Sphere {
    fn default() -> Self {
        Self::new()
    }
}

impl Sphere {
    pub fn new() -> Self {
        let font = FontRef::try_from_slice(MONO).expect("JetBrains Mono");
        let upm = font.units_per_em().unwrap_or(1000.0);
        let (asc, desc) = (font.ascent_unscaled(), -font.descent_unscaled());
        let block = FONT_PX * LINE_HEIGHT;
        // Alto de línea fijo: Flutter reparte ascendente y descendente en
        // proporción a las métricas de la fuente y redondea el alto del
        // párrafo (13,8 → 14) poniendo lo que sobra encima de la línea base
        // (medido con TextPainter: alto 14, línea base 10,8636).
        let rounded = block.round();
        let baseline = asc / (asc + desc) * block + (rounded - block);
        let adv = RAMP.iter().map(|c| font.h_advance_unscaled(font.glyph_id(*c))).fold(0f32, f32::max) / upm * FONT_PX;
        Self { font, cell: (adv as f64, rounded as f64), baseline, atlas: None }
    }

    pub fn cell(&self) -> (f64, f64) {
        self.cell
    }

    /// Atlas: glifo `i` con origen en (i·ancho de celda, 0), a escala física.
    fn atlas(&mut self, scale: f64) -> &(f64, usize, usize, Vec<f32>) {
        if self.atlas.as_ref().is_none_or(|a| a.0 != scale) {
            let (cw, ch) = self.cell;
            let w = ((cw * RAMP.len() as f64 + 2.0) * scale).ceil() as usize;
            let h = (ch * scale).ceil() as usize;
            let mut cov = vec![0f32; w * h];
            let px = self.font.pt_to_px_scale(FONT_PX * scale as f32 * 72.0 / 96.0).unwrap_or(PxScale::from(FONT_PX));
            for (i, c) in RAMP.iter().enumerate() {
                let origin = point((i as f64 * cw * scale) as f32, self.baseline * scale as f32);
                let g = self.font.glyph_id(*c).with_scale_and_position(px, origin);
                if let Some(og) = self.font.outline_glyph(g) {
                    let b = og.px_bounds();
                    og.draw(|x, y, v| {
                        let (gx, gy) = (b.min.x as i64 + x as i64, b.min.y as i64 + y as i64);
                        if gx >= 0 && gy >= 0 && (gx as usize) < w && (gy as usize) < h {
                            let o = &mut cov[gy as usize * w + gx as usize];
                            *o = (*o + v).min(1.0);
                        }
                    });
                }
            }
            self.atlas = Some((scale, w, h, cov));
        }
        self.atlas.as_ref().expect("atlas")
    }

    /// Fotograma en `t` segundos de giro para una ventana `w`×`h` lógica.
    pub fn frame(&mut self, t: f64, w: f64, h: f64, scale: f64, accent: u32) -> SphereFrame {
        let (r, cx, cy) = sphere_geometry(w, h);
        let (cw, ch) = self.cell;
        let theta = t * 2.0 * std::f64::consts::PI / TURN_S;
        let cols = (2.0 * r / cw).floor() as i32;
        let rows = (2.0 * r / ch).floor() as i32;
        let left = cx - cols as f64 * cw / 2.0;
        let top = cy - rows as f64 * ch / 2.0;
        // Capa con un margen de una celda para los glifos que sobresalen.
        let (ox, oy) = ((left - cw).floor(), (top - ch).floor());
        let pw = physical((cols as f64 + 2.0) * cw + 2.0, scale);
        let ph = physical((rows as f64 + 2.0) * ch + 2.0, scale);
        let mut px = vec![0u8; pw * ph * 4];
        let a = rgba(accent);
        let (lx, ly, lz) = (-0.55, -0.5, 0.67);
        let (_, aw, ah, atlas) = self.atlas(scale).clone();
        // Muestra bilineal del atlas en (u, v) (centros de píxel en k + 0,5).
        let sample = |u: f64, v: f64| -> f32 {
            let (fx, fy) = (u - 0.5, v - 0.5);
            let (x0, y0) = (fx.floor(), fy.floor());
            let (tx, ty) = ((fx - x0) as f32, (fy - y0) as f32);
            let at = |x: f64, y: f64| -> f32 {
                if x < 0.0 || y < 0.0 || x as usize >= aw || y as usize >= ah { 0.0 } else { atlas[y as usize * aw + x as usize] }
            };
            let top = at(x0, y0) * (1.0 - tx) + at(x0 + 1.0, y0) * tx;
            let bot = at(x0, y0 + 1.0) * (1.0 - tx) + at(x0 + 1.0, y0 + 1.0) * tx;
            top * (1.0 - ty) + bot * ty
        };
        for row in 0..rows {
            let y = top + (row as f64 + 0.5) * ch;
            let ny = (y - cy) / r;
            for col in 0..cols {
                let x = left + (col as f64 + 0.5) * cw;
                let nx = (x - cx) / r;
                let d = nx * nx + ny * ny;
                if d >= 1.0 {
                    continue;
                }
                let nz = (1.0 - d).sqrt();
                let lon = nx.atan2(nz) + theta;
                let lat = ny.asin();
                let diffuse = (nx * lx + ny * ly + nz * lz).max(0.0);
                // Manchas que giran con la esfera y franjas de latitud.
                let tex = 0.5 + 0.35 * (lon * 3.0).sin() * (lat * 2.0).cos() + 0.15 * (lon * 7.0 + lat * 5.0).sin();
                let band = if (lat * 16.0).sin().abs() < 0.14 { 0.55 } else { 1.0 };
                let v = ((0.12 + 0.88 * diffuse) * (0.4 + 0.6 * tex) * band).clamp(0.0, 1.0);
                let idx = (v * (RAMP.len() as f64 - 0.01)).floor() as usize;
                if idx == 0 {
                    continue;
                }
                let alpha = 0.16 + 0.42 * v;
                // Celda en píxeles físicos de la capa y su origen en el atlas.
                let (gx, gy) = ((x - cw / 2.0 - ox) * scale, (y - ch / 2.0 - oy) * scale);
                let src = idx as f64 * cw * scale;
                let (x0, y0) = (gx.floor() as i64, gy.floor() as i64);
                let (x1, y1) = ((gx + cw * scale).ceil() as i64, (gy + ch * scale).ceil() as i64);
                for py in y0.max(0)..y1.min(ph as i64) {
                    for pxx in x0.max(0)..x1.min(pw as i64) {
                        let c = sample(src + pxx as f64 + 0.5 - gx, py as f64 + 0.5 - gy);
                        if c <= 0.0 {
                            continue;
                        }
                        let o = (py as usize * pw + pxx as usize) * 4;
                        let al = ((c as f64 * alpha) * 255.0).round() as u8;
                        if al > px[o + 3] {
                            px[o..o + 4].copy_from_slice(&[a[0] as u8, a[1] as u8, a[2] as u8, al]);
                        }
                    }
                }
            }
        }
        SphereFrame { pixels: Pixels { w: pw, h: ph, px }, x: ox, y: oy }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cajas_de_la_gaussiana() {
        // σ = 10: tres cajas de ~19 px (varianza total ≈ σ²).
        let s = box_sizes(10.0);
        let var: f64 = s.iter().map(|&w| ((w * w) as f64 - 1.0) / 12.0).sum();
        assert!((var - 100.0).abs() < 8.0, "{s:?} → {var}");
    }

    #[test]
    fn desenfocar_conserva_un_color_liso_y_reparte_un_borde() {
        let mut p = Pixels { w: 40, h: 10, px: [10u8, 200, 30, 255].repeat(400) };
        blur(&mut p, 4.0);
        assert!(p.px.as_chunks::<4>().0.iter().all(|c| *c == [10, 200, 30, 255]));
        let mut px = Vec::new();
        for _ in 0..10 {
            for x in 0..40 {
                px.extend_from_slice(&if x < 20 { [0, 0, 0, 255] } else { [255, 255, 255, 255] });
            }
        }
        let mut p = Pixels { w: 40, h: 10, px };
        blur(&mut p, 4.0);
        let row: Vec<u8> = (0..40).map(|x| p.px[(5 * 40 + x) * 4]).collect();
        assert_eq!(row[0], 0);
        assert_eq!(row[39], 255);
        assert!((100..156).contains(&row[19]) || (100..156).contains(&row[20]), "{row:?}");
        assert!(row.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn celda_de_jetbrains_mono() {
        let s = Sphere::new();
        let (w, h) = s.cell();
        assert!((w - 7.2).abs() < 0.01, "{w}");
        assert_eq!(h, 14.0);
        assert!((s.baseline - 10.8636).abs() < 0.01, "{}", s.baseline);
    }

    #[test]
    fn la_esfera_gira_y_tiene_glifos() {
        let mut s = Sphere::new();
        let a = s.frame(0.0, 1280.0, 800.0, 1.0, 0xFFF1C40F);
        let b = s.frame(10.0, 1280.0, 800.0, 1.0, 0xFFF1C40F);
        let lit = a.pixels.px.as_chunks::<4>().0.iter().filter(|c| c[3] > 0).count();
        assert!(lit > 10_000, "{lit}");
        assert_ne!(a.pixels.px, b.pixels.px);
        // Centrada en (0,70 w, 0,53 h).
        let (r, cx, cy) = sphere_geometry(1280.0, 800.0);
        assert!((a.x + a.pixels.w as f64 / 2.0 - cx).abs() < 8.0 && (a.y + a.pixels.h as f64 / 2.0 - cy).abs() < 10.0, "{} {}", a.x, a.y);
        assert!(a.pixels.w as f64 >= 2.0 * r);
    }

    #[test]
    fn aurora_ilumina_arriba_a_la_izquierda() {
        let p = aurora(0xFF35D69B, 0xFF0A0E0C, 64.0, 40.0, 1.0);
        let at = |x: usize, y: usize| p.px[(y * p.w + x) * 4 + 1];
        assert!(at(0, 0) > at(32, 39));
    }
}
