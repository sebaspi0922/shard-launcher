//! Arte procedural de `app/lib/ui/widgets/art.dart`, portado 1:1: mismo hash,
//! mismo generador aleatorio que Dart y mismos colores HCT, para que los
//! paisajes salgan iguales que en Flutter. Se rasteriza en CPU a una imagen
//! BGRA (GPUI no tiene degradados radiales).

use material_colors::{color::Rgb, hct::Hct};

/// FNV-1a sobre unidades UTF-16, como `stableHash` en Dart.
pub fn stable_hash(s: &str) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for unit in s.encode_utf16() {
        h ^= unit as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

pub fn hue_for(id: &str) -> f64 {
    (stable_hash(id) % 360) as f64
}

/// `dart:math` `Random(seed)` de la VM (multiply-with-carry de 64 bits).
pub struct DartRandom(u64);

impl DartRandom {
    pub fn new(seed: i64) -> Self {
        let mut n = seed as u64;
        n = (!n).wrapping_add(n << 21);
        n ^= n >> 24;
        n = n.wrapping_mul(265);
        n ^= n >> 14;
        n = n.wrapping_mul(21);
        n ^= n >> 28;
        n = n.wrapping_add(n << 31);
        if n == 0 {
            n = 0x5a17;
        }
        let mut r = Self(n);
        for _ in 0..4 {
            r.next_state();
        }
        r
    }

    fn next_state(&mut self) {
        const A: u64 = 0xffffda61;
        let lo = self.0 & 0xFFFF_FFFF;
        let hi = self.0 >> 32;
        self.0 = A.wrapping_mul(lo).wrapping_add(hi);
    }

    pub fn next_int(&mut self, max: u32) -> u32 {
        let max = max as u64;
        if max.isolate_lowest_one() == max {
            self.next_state();
            return (self.0 & 0xFFFF_FFFF & (max - 1)) as u32;
        }
        loop {
            self.next_state();
            let rnd32 = self.0 & 0xFFFF_FFFF;
            let result = rnd32 % max;
            if rnd32 - result + max <= 1 << 32 {
                return result as u32;
            }
        }
    }

    pub fn next_double(&mut self) -> f64 {
        let a = self.next_int(1 << 26) as f64;
        let b = self.next_int(1 << 27) as f64;
        (a * (1u64 << 27) as f64 + b) / (1u64 << 53) as f64
    }
}

/// Color lineal simple (0–255 por canal, alfa 0–1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Rgba {
    pub const fn hex(argb: u32) -> Self {
        Self {
            r: ((argb >> 16) & 0xff) as f64,
            g: ((argb >> 8) & 0xff) as f64,
            b: (argb & 0xff) as f64,
            a: ((argb >> 24) & 0xff) as f64 / 255.0,
        }
    }
    pub fn alpha(self, a: f64) -> Self {
        Self { a, ..self }
    }
    pub fn lerp(self, o: Self, t: f64) -> Self {
        Self {
            r: self.r + (o.r - self.r) * t,
            g: self.g + (o.g - self.g) * t,
            b: self.b + (o.b - self.b) * t,
            a: self.a + (o.a - self.a) * t,
        }
    }
    pub fn to_u32(self) -> u32 {
        let c = |v: f64| v.round().clamp(0.0, 255.0) as u32;
        (c(self.a * 255.0) << 24) | (c(self.r) << 16) | (c(self.g) << 8) | c(self.b)
    }
}

pub fn tone(hue: f64, chroma: f64, tone: f64) -> Rgba {
    let rgb: Rgb = Hct::from(hue.rem_euclid(360.0), chroma, tone).into();
    Rgba {
        r: rgb.red as f64,
        g: rgb.green as f64,
        b: rgb.blue as f64,
        a: 1.0,
    }
}

struct Biome {
    hue: f64,
    chroma: f64,
    tone: f64,
    top_hue: f64,
    top_chroma: f64,
    top_tone: f64,
}

const BIOMES: [Biome; 6] = [
    Biome { hue: 135.0, chroma: 34.0, tone: 34.0, top_hue: 128.0, top_chroma: 52.0, top_tone: 52.0 },
    Biome { hue: 80.0, chroma: 30.0, tone: 60.0, top_hue: 85.0, top_chroma: 34.0, top_tone: 74.0 },
    Biome { hue: 230.0, chroma: 10.0, tone: 70.0, top_hue: 230.0, top_chroma: 6.0, top_tone: 94.0 },
    Biome { hue: 40.0, chroma: 46.0, tone: 42.0, top_hue: 55.0, top_chroma: 40.0, top_tone: 58.0 },
    Biome { hue: 140.0, chroma: 26.0, tone: 32.0, top_hue: 350.0, top_chroma: 40.0, top_tone: 78.0 },
    Biome { hue: 150.0, chroma: 28.0, tone: 22.0, top_hue: 145.0, top_chroma: 40.0, top_tone: 36.0 },
];

/// Lienzo RGBA en CPU con mezcla "source over" y cobertura de borde.
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    px: Vec<[f64; 4]>,
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Self {
        Self { w, h, px: vec![[0.0; 4]; w * h] }
    }

    fn blend(&mut self, x: usize, y: usize, c: Rgba, cover: f64) {
        let a = c.a * cover;
        if a <= 0.0 {
            return;
        }
        let p = &mut self.px[y * self.w + x];
        let out_a = a + p[3] * (1.0 - a);
        if out_a <= 0.0 {
            return;
        }
        for (i, v) in [c.r, c.g, c.b].into_iter().enumerate() {
            p[i] = (v * a + p[i] * p[3] * (1.0 - a)) / out_a;
        }
        p[3] = out_a;
    }

    /// Rectángulo con cobertura fraccional en los bordes (antialias).
    pub fn rect(&mut self, l: f64, t: f64, r: f64, b: f64, color: impl Fn(f64, f64) -> Rgba) {
        let (x0, x1) = (l.max(0.0).floor() as usize, (r.min(self.w as f64)).ceil() as usize);
        let (y0, y1) = (t.max(0.0).floor() as usize, (b.min(self.h as f64)).ceil() as usize);
        for y in y0..y1 {
            let cy = (b.min(y as f64 + 1.0) - t.max(y as f64)).clamp(0.0, 1.0);
            for x in x0..x1 {
                let cx = (r.min(x as f64 + 1.0) - l.max(x as f64)).clamp(0.0, 1.0);
                let c = color(x as f64 + 0.5, y as f64 + 0.5);
                self.blend(x, y, c, cx * cy);
            }
        }
    }

    /// Píxeles en BGRA premultiplicado no; GPUI espera BGRA.
    pub fn to_bgra(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.w * self.h * 4);
        for p in &self.px {
            let c = |v: f64| v.round().clamp(0.0, 255.0) as u8;
            out.extend_from_slice(&[c(p[2]), c(p[1]), c(p[0]), c(p[3] * 255.0)]);
        }
        out
    }
}

/// Paisaje de bloques (`_LandscapePainter.paint`). `scale` = píxeles físicos
/// por píxel lógico.
pub fn landscape(seed: &str, block: f64, w: f64, h: f64, scale: f64) -> Canvas {
    let hash = stable_hash(seed);
    let sky_hue = (hash % 360) as f64;
    let biome = &BIOMES[((hash >> 9) as usize) % BIOMES.len()];
    let mut rnd = DartRandom::new(hash as i64);
    let (pw, ph) = ((w * scale).round() as usize, (h * scale).round() as usize);
    let mut cv = Canvas::new(pw, ph);
    let s = scale;

    let sky_top = tone(sky_hue, 36.0, 18.0);
    let sky_bottom = tone(sky_hue + 25.0, 44.0, 46.0);
    cv.rect(0.0, 0.0, pw as f64, ph as f64, |_, y| sky_top.lerp(sky_bottom, y / ph as f64));

    let sun = (w * (0.6 + rnd.next_double() * 0.28), h * (0.2 + rnd.next_double() * 0.12));
    let r = (w.min(h) * 0.045).clamp(5.0, 18.0);
    let halo = tone(sky_hue + 40.0, 40.0, 85.0);
    let (hx, hy, hr) = (sun.0 * s, sun.1 * s, r * 5.0 * s);
    cv.rect(hx - hr, hy - hr, hx + hr, hy + hr, |x, y| {
        let d = ((x - hx).powi(2) + (y - hy).powi(2)).sqrt() / hr;
        if d >= 1.0 { halo.alpha(0.0) } else { halo.alpha(0.32 * (1.0 - d)) }
    });
    let outer = tone(sky_hue + 40.0, 24.0, 90.0).alpha(0.9);
    cv.rect((sun.0 - r) * s, (sun.1 - r) * s, (sun.0 + r) * s, (sun.1 + r) * s, |_, _| outer);
    let inner = tone(sky_hue + 40.0, 10.0, 98.0);
    let ir = r * 0.6;
    cv.rect((sun.0 - ir) * s, (sun.1 - ir) * s, (sun.0 + ir) * s, (sun.1 + ir) * s, |_, _| inner);

    let layers = [(0.56, 0.14, 0.5), (0.70, 0.12, 0.22), (0.84, 0.08, 0.0)];
    let cols = (w / block).ceil() as usize + 1;
    for (i, (base, amp, fog)) in layers.into_iter().enumerate() {
        let shade = 1.0 - (2 - i) as f64 * 0.12;
        let fogged = |c: Rgba| c.lerp(sky_bottom, fog);
        let body = fogged(tone(biome.hue, biome.chroma, biome.tone * shade));
        let top = fogged(tone(biome.top_hue, biome.top_chroma, biome.top_tone * shade));
        let mut y = base + (rnd.next_double() - 0.5) * amp;
        let mut run: i32 = 0;
        for c in 0..cols {
            if run <= 0 {
                y += (rnd.next_int(5) as f64 - 2.0) * (block / h);
                y = y.clamp(base - amp, base + amp);
                run = 1 + rnd.next_int(3) as i32;
            }
            run -= 1;
            let py = (y * h / block).round() * block;
            let x = c as f64 * block;
            cv.rect(x * s, py * s, (x + block + 0.5) * s, h * s, |_, _| body);
            cv.rect(x * s, py * s, (x + block + 0.5) * s, (py + block * 0.34) * s, |_, _| top);
        }
    }

    // Viñeta inferior: transparente hasta el 45 % y negro al 45 % abajo.
    let ph_f = ph as f64;
    cv.rect(0.0, 0.0, pw as f64, ph_f, |_, y| {
        let t = ((y / ph_f - 0.45) / 0.55).clamp(0.0, 1.0);
        Rgba::hex(0xFF000000).alpha(0.45 * t)
    });
    cv
}

/// Colores del degradado del icono de instancia (arriba-izq., abajo-der.).
pub fn icon_colors(id: &str) -> (Rgba, Rgba) {
    let hue = hue_for(id);
    (tone(hue + 25.0, 45.0, 62.0), tone(hue - 15.0, 45.0, 34.0))
}

/// Iniciales como `InstanceIcon._initials`.
pub fn initials(name: &str) -> String {
    let s: String = name
        .split_whitespace()
        .take(2)
        .filter_map(|w| w.chars().next())
        .flat_map(char::to_uppercase)
        .collect();
    if s.is_empty() { "?".into() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_igual_que_dart() {
        // Valores de `stableHash` calculados en Dart (FNV-1a 32 bits).
        assert_eq!(stable_hash(""), 0x811c9dc5);
        assert_eq!(stable_hash("a"), 0xe40c292c);
    }

    #[test]
    fn random_igual_que_dart() {
        // `Random(42)`: nextInt(100) ×3 en la VM de Dart 3.15.
        let mut r = DartRandom::new(42);
        let got = [r.next_int(100), r.next_int(100), r.next_int(100)];
        assert_eq!(got, [87, 58, 4]);
        // Semilla real de una tarjeta: `Random(stableHash('survival-amigos'))`.
        let mut r = DartRandom::new(stable_hash("survival-amigos") as i64);
        assert_eq!(r.next_double(), 0.4945766371811895);
        assert_eq!((r.next_int(5), r.next_int(3)), (1, 0));
    }

    #[test]
    fn iniciales() {
        assert_eq!(initials("Survival con amigos"), "SC");
        assert_eq!(initials("Create: Aeronáutica"), "CA");
        assert_eq!(initials("   "), "?");
    }
}
