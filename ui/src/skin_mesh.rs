//! Modelo 3D del jugador (port 1:1 de `app/lib/ui/skins/skin_mesh.dart`) y su
//! pintado (`_SkinPainter` y `_CapePainter` de skin_viewer.dart).
//!
//! Cada píxel de la textura es un cuadrilátero; se proyectan en ortográfica,
//! se ordenan por profundidad (pintor) y se rasterizan en CPU sin
//! antialias, como `drawVertices` de Skia: un píxel se pinta si su centro
//! cae dentro (filas y bordes redondeados al centro, vértices en 1/64 px).
//! Slint no pinta triángulos con textura: el resultado es una imagen RGBA.

use crate::skin::SkinTexture;

/// Cámara: giro horizontal (`yaw`) y vertical (`pitch`), en radianes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub yaw: f64,
    pub pitch: f64,
}

/// Ángulo inicial del visor: tres cuartos, algo desde arriba.
pub const DEFAULT_CAMERA: Camera = Camera { yaw: -0.5, pitch: 0.16 };

/// Giro por píxel arrastrado (`onPanUpdate` del visor).
pub const DRAG_YAW: f64 = 0.012;
pub const DRAG_PITCH: f64 = 0.008;

/// Límite del giro vertical al arrastrar.
pub fn clamp_pitch(p: f64) -> f64 {
    p.clamp(-0.6, 0.8)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Face {
    Top,
    Bottom,
    Right,
    Front,
    Left,
    Back,
}

const FACES: [Face; 6] = [Face::Top, Face::Bottom, Face::Right, Face::Front, Face::Left, Face::Back];

/// Luz fija por cara (da volumen sin depender del ángulo).
fn shade(f: Face) -> f64 {
    match f {
        Face::Top => 1.0,
        Face::Bottom => 0.55,
        Face::Front | Face::Back => 0.9,
        Face::Right | Face::Left => 0.74,
    }
}

#[derive(Clone, Copy)]
struct V3 {
    x: f64,
    y: f64,
    z: f64,
}

const fn v3(x: f64, y: f64, z: f64) -> V3 {
    V3 { x, y, z }
}

/// Giro de una pieza alrededor de un pivote (brazos abiertos, capa caída).
#[derive(Clone, Copy)]
struct Pose {
    pivot: V3,
    roll: f64,
    pitch: f64,
}

impl Pose {
    fn apply(&self, p: V3) -> V3 {
        let (mut x, mut y, mut z) = (p.x - self.pivot.x, p.y - self.pivot.y, p.z - self.pivot.z);
        if self.roll != 0.0 {
            let (c, s) = (self.roll.cos(), self.roll.sin());
            (x, y) = (x * c - y * s, x * s + y * c);
        }
        if self.pitch != 0.0 {
            let (c, s) = (self.pitch.cos(), self.pitch.sin());
            (y, z) = (y * c - z * s, y * s + z * c);
        }
        v3(x + self.pivot.x, y + self.pivot.y, z + self.pivot.z)
    }
}

#[derive(Clone, Copy)]
struct Box3 {
    x: f64,
    y: f64,
    z: f64,
    w: i32,
    h: i32,
    d: i32,
    u: i32,
    v: i32,
    inflate: f64,
    mirror: bool,
    opaque: bool,
    swap_front_back: bool,
    pose: Option<Pose>,
    cape: bool,
}

const BASE: Box3 = Box3 {
    x: 0.0,
    y: 0.0,
    z: 0.0,
    w: 0,
    h: 0,
    d: 0,
    u: 0,
    v: 0,
    inflate: 0.0,
    mirror: false,
    opaque: true,
    swap_front_back: false,
    pose: None,
    cape: false,
};

/// Piezas del jugador (y de la capa, si hay).
fn boxes(slim: bool, legacy: bool, cape: bool) -> Vec<Box3> {
    let aw = if slim { 3 } else { 4 };
    let awf = aw as f64;
    let right_arm = Some(Pose { pivot: v3(-4.5, 22.5, 0.0), roll: -0.07, pitch: 0.0 });
    let left_arm = Some(Pose { pivot: v3(4.5, 22.5, 0.0), roll: 0.07, pitch: 0.0 });
    let outer = Box3 { inflate: 0.25, opaque: false, ..BASE };
    let mut out = vec![
        // Cabeza y sombrero.
        Box3 { x: -4.0, y: 24.0, z: -4.0, w: 8, h: 8, d: 8, ..BASE },
        Box3 { x: -4.0, y: 24.0, z: -4.0, w: 8, h: 8, d: 8, u: 32, inflate: 0.5, opaque: false, ..BASE },
        // Cuerpo.
        Box3 { x: -4.0, y: 12.0, z: -2.0, w: 8, h: 12, d: 4, u: 16, v: 16, ..BASE },
        // Brazos.
        Box3 { x: -4.0 - awf, y: 12.0, z: -2.0, w: aw, h: 12, d: 4, u: 40, v: 16, pose: right_arm, ..BASE },
    ];
    let left = Box3 { x: 4.0, y: 12.0, z: -2.0, w: aw, h: 12, d: 4, pose: left_arm, ..BASE };
    out.push(if legacy { Box3 { u: 40, v: 16, mirror: true, ..left } } else { Box3 { u: 32, v: 48, ..left } });
    // Piernas.
    let leg = Box3 { y: 0.0, z: -2.0, w: 4, h: 12, d: 4, ..BASE };
    out.push(Box3 { x: -4.0, v: 16, ..leg });
    out.push(if legacy { Box3 { x: 0.0, v: 16, mirror: true, ..leg } } else { Box3 { x: 0.0, u: 16, v: 48, ..leg } });
    // Capas exteriores (solo en la plantilla de 64×64).
    if !legacy {
        out.extend([
            Box3 { x: -4.0, y: 12.0, z: -2.0, w: 8, h: 12, d: 4, u: 16, v: 32, ..outer },
            Box3 { x: -4.0 - awf, y: 12.0, z: -2.0, w: aw, h: 12, d: 4, u: 40, v: 32, pose: right_arm, ..outer },
            Box3 { x: 4.0, y: 12.0, z: -2.0, w: aw, h: 12, d: 4, u: 48, v: 48, pose: left_arm, ..outer },
            Box3 { x: -4.0, y: 0.0, z: -2.0, w: 4, h: 12, d: 4, u: 0, v: 32, ..outer },
            Box3 { x: 0.0, y: 0.0, z: -2.0, w: 4, h: 12, d: 4, u: 0, v: 48, ..outer },
        ]);
    }
    if cape {
        out.push(Box3 {
            x: -5.0,
            y: 8.0,
            z: -3.2,
            w: 10,
            h: 16,
            d: 1,
            swap_front_back: true,
            cape: true,
            pose: Some(Pose { pivot: v3(0.0, 24.0, -2.2), roll: 0.0, pitch: 0.14 }),
            ..BASE
        });
    }
    out
}

/// Rectángulo de la UV de cada cara: (u, v, ancho, alto).
fn uv(b: &Box3, f: Face) -> (i32, i32, i32, i32) {
    let (u, v, w, h, d) = (b.u, b.v, b.w, b.h, b.d);
    let mut face = f;
    if b.mirror {
        face = match f {
            Face::Right => Face::Left,
            Face::Left => Face::Right,
            o => o,
        };
    }
    if b.swap_front_back {
        face = match face {
            Face::Front => Face::Back,
            Face::Back => Face::Front,
            o => o,
        };
    }
    match face {
        Face::Top => (u + d, v, w, d),
        Face::Bottom => (u + d + w, v, w, d),
        Face::Right => (u, v + d, d, h),
        Face::Front => (u + d, v + d, w, h),
        Face::Left => (u + d + w, v + d, d, h),
        Face::Back => (u + 2 * d + w, v + d, w, h),
    }
}

/// Esquinas de la cara vista desde fuera: arriba-izquierda, arriba-derecha y
/// abajo-izquierda (el frente mira a +z; la derecha del jugador es −x).
fn corners(b: &Box3, f: Face) -> (V3, V3, V3) {
    let i = b.inflate;
    let (x0, y0, z0) = (b.x - i, b.y - i, b.z - i);
    let (x1, y1, z1) = (b.x + b.w as f64 + i, b.y + b.h as f64 + i, b.z + b.d as f64 + i);
    match f {
        Face::Front => (v3(x0, y1, z1), v3(x1, y1, z1), v3(x0, y0, z1)),
        Face::Back => (v3(x1, y1, z0), v3(x0, y1, z0), v3(x1, y0, z0)),
        Face::Right => (v3(x0, y1, z0), v3(x0, y1, z1), v3(x0, y0, z0)),
        Face::Left => (v3(x1, y1, z1), v3(x1, y1, z0), v3(x1, y0, z1)),
        Face::Top => (v3(x0, y1, z0), v3(x1, y1, z0), v3(x0, y1, z1)),
        Face::Bottom => (v3(x0, y0, z1), v3(x1, y0, z1), v3(x0, y0, z0)),
    }
}

/// Un píxel de la textura en pantalla: esquina, vectores de sus lados,
/// profundidad (mayor = más cerca) y color ARGB ya iluminado.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub x: f64,
    pub y: f64,
    pub ux: f64,
    pub uy: f64,
    pub vx: f64,
    pub vy: f64,
    pub depth: f64,
    pub color: u32,
}

/// Píxeles de pantalla por píxel del modelo. El modelo mide ~33 de alto y,
/// girado, ~20 de ancho: se deja margen.
pub fn skin_scale(w: f64, h: f64) -> f64 {
    (h / 40.0).min(w / 24.0)
}

/// Aplica la luz de la cara a un color ARGB.
fn lit(argb: u32, k: f64) -> u32 {
    let ch = |s: u32| ((((argb >> s) & 0xFF) as f64 * k).round() as u32) << s;
    (argb & 0xFF00_0000) | ch(16) | ch(8) | ch(0)
}

/// Cuadriláteros visibles del jugador dentro de `w × h`, del más lejano al
/// más cercano.
pub fn build_mesh(skin: &SkinTexture, cape: Option<&SkinTexture>, slim: bool, cam: Camera, w: f64, h: f64) -> Vec<Quad> {
    let scale = skin_scale(w, h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let (cyaw, syaw) = (cam.yaw.cos(), cam.yaw.sin());
    let (cp, sp) = (cam.pitch.cos(), cam.pitch.sin());
    let project = |p: V3| {
        let y = p.y - 16.0;
        let x1 = p.x * cyaw + p.z * syaw;
        let z1 = -p.x * syaw + p.z * cyaw;
        let y2 = y * cp - z1 * sp;
        let z2 = y * sp + z1 * cp;
        (cx + x1 * scale, cy - y2 * scale, z2)
    };
    let mut quads = Vec::new();
    for b in boxes(slim, skin.legacy(), cape.is_some()) {
        let tex = if b.cape { cape.expect("capa") } else { skin };
        for f in FACES {
            let (mut tl, mut tr, mut bl) = corners(&b, f);
            if let Some(pose) = b.pose {
                (tl, tr, bl) = (pose.apply(tl), pose.apply(tr), pose.apply(bl));
            }
            let (ax, ay, az) = project(tl);
            let (bx, by, bz) = project(tr);
            let (qx, qy, qz) = project(bl);
            // Cara de espaldas a la cámara: no se dibuja.
            if (bx - ax) * (qy - ay) - (by - ay) * (qx - ax) <= 0.0 {
                continue;
            }
            let (fu, fv, fw, fh) = uv(&b, f);
            let (fwf, fhf) = (fw as f64, fh as f64);
            let (ux, uy, uz) = ((bx - ax) / fwf, (by - ay) / fwf, (bz - az) / fwf);
            let (vx, vy, vz) = ((qx - ax) / fhf, (qy - ay) / fhf, (qz - az) / fhf);
            let k = shade(f);
            for j in 0..fh {
                for i in 0..fw {
                    let ti = if b.mirror { fw - 1 - i } else { i };
                    let mut argb = tex.at(fu + ti, fv + j);
                    if b.opaque {
                        argb |= 0xFF00_0000;
                    } else if argb >> 24 == 0 {
                        continue;
                    }
                    let (fi, fj) = (i as f64, j as f64);
                    quads.push(Quad {
                        x: ax + ux * fi + vx * fj,
                        y: ay + uy * fi + vy * fj,
                        ux,
                        uy,
                        vx,
                        vy,
                        depth: az + uz * (fi + 0.5) + vz * (fj + 0.5),
                        color: lit(argb, k),
                    });
                }
            }
        }
    }
    // El orden de los empates (caras paralelas a la pantalla) decide qué
    // cuadrilátero gana en las costuras: se ordena como `List.sort` de Dart.
    dart_sort(&mut quads, |a, b| dart_compare(a.depth, b.depth));
    quads
}

/// `double.compareTo` de Dart (−0,0 va antes que 0,0).
fn dart_compare(a: f64, b: f64) -> i32 {
    if a < b {
        -1
    } else if a > b {
        1
    } else if a == b {
        if a == 0.0 { (b.is_sign_negative() as i32) - (a.is_sign_negative() as i32) } else { 0 }
    } else {
        // NaN: no aparece en la malla; mismo criterio que Dart (NaN al final).
        (a.is_nan() as i32) - (b.is_nan() as i32)
    }
}

/// `Sort.sort` de `dart:_internal` (inserción hasta 32 y quicksort de dos
/// pivotes), portado tal cual: no es estable y el orden de los empates
/// cambia lo que se ve en las costuras.
fn dart_sort<T: Copy>(a: &mut [T], cmp: impl Fn(&T, &T) -> i32 + Copy) {
    if !a.is_empty() {
        do_sort(a, 0, a.len() as isize - 1, cmp);
    }
}

fn do_sort<T: Copy>(a: &mut [T], left: isize, right: isize, cmp: impl Fn(&T, &T) -> i32 + Copy) {
    if right - left <= 32 {
        insertion_sort(a, left, right, cmp);
    } else {
        dual_pivot_quicksort(a, left, right, cmp);
    }
}

fn insertion_sort<T: Copy>(a: &mut [T], left: isize, right: isize, cmp: impl Fn(&T, &T) -> i32) {
    for i in left + 1..=right {
        let el = a[i as usize];
        let mut j = i;
        while j > left && cmp(&a[(j - 1) as usize], &el) > 0 {
            a[j as usize] = a[(j - 1) as usize];
            j -= 1;
        }
        a[j as usize] = el;
    }
}

fn dual_pivot_quicksort<T: Copy>(a: &mut [T], left: isize, right: isize, cmp: impl Fn(&T, &T) -> i32 + Copy) {
    let at = |i: isize| i as usize;
    let sixth = (right - left + 1) / 6;
    let index1 = left + sixth;
    let index5 = right - sixth;
    let index3 = (left + right) / 2;
    let index2 = index3 - sixth;
    let index4 = index3 + sixth;
    let (mut el1, mut el2, mut el3, mut el4, mut el5) = (a[at(index1)], a[at(index2)], a[at(index3)], a[at(index4)], a[at(index5)]);
    let swap = |x: &mut T, y: &mut T| {
        if cmp(x, y) > 0 {
            std::mem::swap(x, y);
        }
    };
    swap(&mut el1, &mut el2);
    swap(&mut el4, &mut el5);
    swap(&mut el1, &mut el3);
    swap(&mut el2, &mut el3);
    swap(&mut el1, &mut el4);
    swap(&mut el3, &mut el4);
    swap(&mut el2, &mut el5);
    swap(&mut el2, &mut el3);
    swap(&mut el4, &mut el5);
    let (pivot1, pivot2) = (el2, el4);
    a[at(index1)] = el1;
    a[at(index3)] = el3;
    a[at(index5)] = el5;
    a[at(index2)] = a[at(left)];
    a[at(index4)] = a[at(right)];
    let mut less = left + 1;
    let mut great = right - 1;
    let pivots_are_equal = cmp(&pivot1, &pivot2) == 0;
    if pivots_are_equal {
        let pivot = pivot1;
        let mut k = less;
        while k <= great {
            let ak = a[at(k)];
            let comp = cmp(&ak, &pivot);
            if comp < 0 {
                if k != less {
                    a[at(k)] = a[at(less)];
                    a[at(less)] = ak;
                }
                less += 1;
            } else if comp > 0 {
                loop {
                    let comp = cmp(&a[at(great)], &pivot);
                    if comp > 0 {
                        great -= 1;
                        continue;
                    } else if comp < 0 {
                        a[at(k)] = a[at(less)];
                        a[at(less)] = a[at(great)];
                        less += 1;
                        a[at(great)] = ak;
                        great -= 1;
                    } else {
                        a[at(k)] = a[at(great)];
                        a[at(great)] = ak;
                        great -= 1;
                    }
                    break;
                }
            }
            k += 1;
        }
    } else {
        let mut k = less;
        while k <= great {
            let ak = a[at(k)];
            if cmp(&ak, &pivot1) < 0 {
                if k != less {
                    a[at(k)] = a[at(less)];
                    a[at(less)] = ak;
                }
                less += 1;
            } else if cmp(&ak, &pivot2) > 0 {
                loop {
                    if cmp(&a[at(great)], &pivot2) > 0 {
                        great -= 1;
                        if great < k {
                            break;
                        }
                        continue;
                    }
                    if cmp(&a[at(great)], &pivot1) < 0 {
                        a[at(k)] = a[at(less)];
                        a[at(less)] = a[at(great)];
                        less += 1;
                        a[at(great)] = ak;
                        great -= 1;
                    } else {
                        a[at(k)] = a[at(great)];
                        a[at(great)] = ak;
                        great -= 1;
                    }
                    break;
                }
            }
            k += 1;
        }
    }
    a[at(left)] = a[at(less - 1)];
    a[at(less - 1)] = pivot1;
    a[at(right)] = a[at(great + 1)];
    a[at(great + 1)] = pivot2;
    do_sort(a, left, less - 2, cmp);
    do_sort(a, great + 2, right, cmp);
    if pivots_are_equal {
        return;
    }
    if less < index1 && great > index5 {
        while cmp(&a[at(less)], &pivot1) == 0 {
            less += 1;
        }
        while cmp(&a[at(great)], &pivot2) == 0 {
            great -= 1;
        }
        let mut k = less;
        while k <= great {
            let ak = a[at(k)];
            if cmp(&ak, &pivot1) == 0 {
                if k != less {
                    a[at(k)] = a[at(less)];
                    a[at(less)] = ak;
                }
                less += 1;
            } else if cmp(&ak, &pivot2) == 0 {
                loop {
                    if cmp(&a[at(great)], &pivot2) == 0 {
                        great -= 1;
                        if great < k {
                            break;
                        }
                        continue;
                    }
                    if cmp(&a[at(great)], &pivot1) < 0 {
                        a[at(k)] = a[at(less)];
                        a[at(less)] = a[at(great)];
                        less += 1;
                        a[at(great)] = ak;
                        great -= 1;
                    } else {
                        a[at(k)] = a[at(great)];
                        a[at(great)] = ak;
                        great -= 1;
                    }
                    break;
                }
            }
            k += 1;
        }
    }
    do_sort(a, less, great, cmp);
}

/// Imagen RGBA sin premultiplicar (para `slint::Image::from_rgba8`).
#[derive(Clone, Debug, PartialEq)]
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Pixels {
    fn new(w: f64, h: f64, s: f64) -> Self {
        let (width, height) = ((w * s).round().max(0.0) as u32, (h * s).round().max(0.0) as u32);
        Self { width, height, rgba: vec![0; (width * height * 4) as usize] }
    }

    /// `argb` sobre el píxel (x, y).
    fn blend(&mut self, x: i64, y: i64, argb: u32) {
        if x < 0 || y < 0 || x >= self.width as i64 || y >= self.height as i64 {
            return;
        }
        let i = ((y * self.width as i64 + x) * 4) as usize;
        let fa = (argb >> 24) as f64 / 255.0;
        if fa >= 1.0 {
            self.rgba[i..i + 4].copy_from_slice(&[(argb >> 16) as u8, (argb >> 8) as u8, argb as u8, 255]);
            return;
        }
        let ba = self.rgba[i + 3] as f64 / 255.0;
        let oa = fa + ba * (1.0 - fa);
        if oa == 0.0 {
            return;
        }
        let fg = [(argb >> 16) & 0xFF, (argb >> 8) & 0xFF, argb & 0xFF];
        for (c, f) in fg.into_iter().enumerate() {
            let b = self.rgba[i + c] as f64;
            self.rgba[i + c] = ((f as f64 * fa + b * ba * (1.0 - fa)) / oa).round() as u8;
        }
        self.rgba[i + 3] = (oa * 255.0).round() as u8;
    }

    /// Triángulo sin antialias, como el rasterizador de Skia: vértices en
    /// 1/64 px, filas cuyo centro cae dentro y en cada fila los píxeles de
    /// `round(x_izq)` a `round(x_der)` (sin incluir este).
    fn triangle(&mut self, p: [(f64, f64); 3], argb: u32) {
        let q = |v: f64| (v * 64.0).round() / 64.0;
        let p = p.map(|(x, y)| (q(x), q(y)));
        let round = |v: f64| (v + 0.5).floor() as i64;
        let top = round(p.iter().map(|v| v.1).fold(f64::INFINITY, f64::min));
        let bot = round(p.iter().map(|v| v.1).fold(f64::NEG_INFINITY, f64::max));
        for row in top.max(0)..bot.min(self.height as i64) {
            let yc = row as f64 + 0.5;
            let mut xs = [0.0; 2];
            let mut n = 0;
            for e in 0..3 {
                let (a, b) = if p[e].1 <= p[(e + 1) % 3].1 { (p[e], p[(e + 1) % 3]) } else { (p[(e + 1) % 3], p[e]) };
                if round(a.1) <= row && row < round(b.1) && n < 2 {
                    xs[n] = a.0 + (b.0 - a.0) * (yc - a.1) / (b.1 - a.1);
                    n += 1;
                }
            }
            if n < 2 {
                continue;
            }
            let (l, r) = (round(xs[0].min(xs[1])), round(xs[0].max(xs[1])));
            for x in l.max(0)..r.min(self.width as i64) {
                self.blend(x, row, argb);
            }
        }
    }
}

/// Pinta el jugador en `w × h` lógicos a `scale_factor` (fondo transparente),
/// con la sombra elíptica bajo los pies si `shadow`.
#[allow(clippy::too_many_arguments)]
pub fn render(skin: &SkinTexture, cape: Option<&SkinTexture>, slim: bool, cam: Camera, w: f64, h: f64, scale_factor: f64, shadow: bool) -> Pixels {
    let s = scale_factor;
    let mut out = Pixels::new(w, h, s);
    if w <= 0.0 || h <= 0.0 {
        return out;
    }
    if shadow {
        // `RadialGradient` negro 0,45 → 0 sobre el rect del óvalo: radio =
        // la mitad del lado corto, así que es un círculo dentro del óvalo.
        let k = skin_scale(w, h);
        let (fx, fy) = (w / 2.0 * s, (h / 2.0 + 16.0 * k) * s);
        let r = (15.0 * k).min(3.6 * k) / 2.0 * s;
        let (x0, x1) = ((fx - r).floor() as i64, (fx + r).ceil() as i64);
        let (y0, y1) = ((fy - r).floor() as i64, (fy + r).ceil() as i64);
        for y in y0..y1 {
            for x in x0..x1 {
                let d = ((x as f64 + 0.5 - fx).powi(2) + (y as f64 + 0.5 - fy).powi(2)).sqrt() / r;
                if d < 1.0 {
                    let a = (0.45 * (1.0 - d) * 255.0).round() as u32;
                    out.blend(x, y, a << 24);
                }
            }
        }
    }
    // Un poco de solape (0,04 px lógicos) para que no se vean costuras.
    const E: f64 = 0.04;
    for q in build_mesh(skin, cape, slim, cam, w, h) {
        let (x0, y0) = (q.x - E, q.y - E);
        let (ux, uy, vx, vy) = (q.ux * (1.0 + E), q.uy * (1.0 + E), q.vx * (1.0 + E), q.vy * (1.0 + E));
        let a = (x0 * s, y0 * s);
        let b = ((x0 + ux) * s, (y0 + uy) * s);
        let c = ((x0 + vx) * s, (y0 + vy) * s);
        let d = ((x0 + ux + vx) * s, (y0 + uy + vy) * s);
        out.triangle([a, b, c], q.color);
        out.triangle([b, d, c], q.color);
    }
    out
}

/// Miniatura del frente de una capa (10×16 en 1,1), pixelada, en `w × h`.
pub fn cape_preview(cape: &SkinTexture, w: f64, h: f64, scale_factor: f64) -> Pixels {
    let s = scale_factor;
    let mut out = Pixels::new(w, h, s);
    let px = w / 10.0;
    let round = |v: f64| (v * s + 0.5).floor() as i64;
    for y in 0..16 {
        for x in 0..10 {
            let argb = cape.at(1 + x, 1 + y);
            if argb >> 24 == 0 {
                continue;
            }
            let (l, t) = (x as f64 * px, y as f64 * px);
            for py in round(t)..round(t + px + 0.5) {
                for pxl in round(l)..round(l + px + 0.5) {
                    out.blend(pxl, py, argb | 0xFF00_0000);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::{SAMPLE_CAPE_1, sample_cape, sample_skin};

    const FRONT: Camera = Camera { yaw: 0.0, pitch: 0.0 };

    #[test]
    fn de_frente_solo_se_ven_las_caras_frontales() {
        let skin = sample_skin();
        let front = build_mesh(&skin, None, true, FRONT, 400.0, 600.0).len();
        let turned = build_mesh(&skin, None, true, Camera { yaw: -0.6, pitch: 0.2 }, 400.0, 600.0).len();
        // Cara 8×8 + cuerpo 8×12 + brazos 2×(3×12) + piernas 2×(4×12), más
        // lo opaco de las capas exteriores.
        assert!(front >= 64 + 96 + 72 + 96, "{front}");
        assert!(turned > front);
        let cape = sample_cape(SAMPLE_CAPE_1.0, SAMPLE_CAPE_1.1);
        let back = build_mesh(&skin, Some(&cape), true, Camera { yaw: std::f64::consts::PI, pitch: 0.0 }, 400.0, 600.0).len();
        assert!(back > front);
    }

    #[test]
    fn ordenados_del_fondo_al_frente() {
        let m = build_mesh(&sample_skin(), None, false, DEFAULT_CAMERA, 444.0, 555.0);
        assert!(m.windows(2).all(|w| w[0].depth <= w[1].depth));
    }

    #[test]
    fn el_render_cabe_y_escala() {
        let skin = sample_skin();
        let one = render(&skin, None, true, DEFAULT_CAMERA, 100.0, 150.0, 1.0, true);
        assert_eq!((one.width, one.height), (100, 150));
        let two = render(&skin, None, true, DEFAULT_CAMERA, 100.0, 150.0, 2.0, true);
        assert_eq!((two.width, two.height), (200, 300));
        let opaque = |p: &Pixels| p.rgba.chunks(4).filter(|c| c[3] == 255).count();
        // A doble escala, unas cuatro veces los píxeles del modelo.
        let ratio = opaque(&two) as f64 / opaque(&one) as f64;
        assert!((3.6..4.4).contains(&ratio), "{ratio}");
        // Las esquinas quedan transparentes.
        assert_eq!(one.rgba[3], 0);
    }

    #[test]
    fn el_arrastre_no_pasa_de_los_limites() {
        assert_eq!(clamp_pitch(2.0), 0.8);
        assert_eq!(clamp_pitch(-2.0), -0.6);
        assert_eq!(clamp_pitch(0.1), 0.1);
    }

    #[test]
    fn miniatura_de_capa() {
        let cape = sample_cape(SAMPLE_CAPE_1.0, SAMPLE_CAPE_1.1);
        let p = cape_preview(&cape, 50.0, 80.0, 1.0);
        assert_eq!((p.width, p.height), (50, 80));
        assert!(p.rgba.chunks(4).all(|c| c[3] == 255));
    }

    /// Referencias de Flutter: `tools/paridad/flutter/skin_mesh_ref_test.dart`.
    #[test]
    fn igual_que_flutter() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
        let refs: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("skin_mesh_ref.json")).unwrap()).unwrap();
        let skin = sample_skin();
        let legacy = SkinTexture { width: 64, height: 32, argb: skin.argb[..64 * 32].to_vec() };
        let cape = sample_cape(SAMPLE_CAPE_1.0, SAMPLE_CAPE_1.1);
        for case in refs["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let f = |k: &str| case[k].as_f64().unwrap();
            let tex = if case["legacy"].as_bool().unwrap() { &legacy } else { &skin };
            let cape = case["cape"].as_bool().unwrap().then_some(&cape);
            let slim = case["slim"].as_bool().unwrap();
            let cam = Camera { yaw: f("yaw"), pitch: f("pitch") };
            let (w, h) = (f("w"), f("h"));
            let quads = build_mesh(tex, cape, slim, cam, w, h).len();
            assert_eq!(quads as u64, case["quads"].as_u64().unwrap(), "{name}: cuadriláteros");
            let got = render(tex, cape, slim, cam, w, h, 1.0, case["shadow"].as_bool().unwrap());
            let want = crate::skin::SkinTexture::decode(&std::fs::read(dir.join(format!("skin_{name}.png"))).unwrap()).unwrap();
            assert_eq!((got.width, got.height), (want.width, want.height), "{name}: tamaño");
            let differ = got
                .rgba
                .chunks(4)
                .zip(&want.argb)
                .filter(|(g, w)| {
                    let w = [(**w >> 16) as u8, (**w >> 8) as u8, **w as u8, (**w >> 24) as u8];
                    // Sin alfa el color no importa.
                    if g[3] == 0 && w[3] == 0 {
                        return false;
                    }
                    g.iter().zip(w).any(|(a, b)| a.abs_diff(b) > 6)
                })
                .count();
            let share = differ as f64 / (got.width * got.height) as f64;
            println!("{name}: {quads} cuadriláteros, {differ} píxeles distintos ({:.3} %)", share * 100.0);
            assert!(share <= 0.001, "{name}: {:.3} % de píxeles distintos", share * 100.0);
        }
    }
}
