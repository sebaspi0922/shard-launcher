//! `QuantizerWu` de material_color_utilities 0.13.0 (Dart), copiado línea a
//! línea. El de `material-colors` trunca la media de cada caja (`as u8`)
//! donde Dart redondea, y eso da otros centros iniciales para k-means.

use material_colors::color::Rgb;

const INDEX_BITS: u32 = 5;
const MAX_INDEX: i64 = 32;
const SIDE: i64 = 33;
const TOTAL: usize = 35937;

#[derive(Clone, Copy, Default)]
struct Cube {
    r0: i64,
    r1: i64,
    g0: i64,
    g1: i64,
    b0: i64,
    b1: i64,
    vol: i64,
}

#[derive(Clone, Copy)]
enum Dir {
    R,
    G,
    B,
}

fn index(r: i64, g: i64, b: i64) -> usize {
    ((r << (INDEX_BITS * 2)) + (r << (INDEX_BITS + 1)) + (g << INDEX_BITS) + r + g + b) as usize
}

/// Suma de un momento sobre la caja (inclusión-exclusión en el cubo acumulado).
fn volume<T: Copy + std::ops::Add<Output = T> + std::ops::Sub<Output = T>>(c: &Cube, m: &[T]) -> T {
    m[index(c.r1, c.g1, c.b1)] - m[index(c.r1, c.g1, c.b0)] - m[index(c.r1, c.g0, c.b1)] + m[index(c.r1, c.g0, c.b0)]
        - m[index(c.r0, c.g1, c.b1)]
        + m[index(c.r0, c.g1, c.b0)]
        + m[index(c.r0, c.g0, c.b1)]
        - m[index(c.r0, c.g0, c.b0)]
}

fn bottom(c: &Cube, d: Dir, m: &[i64]) -> i64 {
    match d {
        Dir::R => -m[index(c.r0, c.g1, c.b1)] + m[index(c.r0, c.g1, c.b0)] + m[index(c.r0, c.g0, c.b1)] - m[index(c.r0, c.g0, c.b0)],
        Dir::G => -m[index(c.r1, c.g0, c.b1)] + m[index(c.r1, c.g0, c.b0)] + m[index(c.r0, c.g0, c.b1)] - m[index(c.r0, c.g0, c.b0)],
        Dir::B => -m[index(c.r1, c.g1, c.b0)] + m[index(c.r1, c.g0, c.b0)] + m[index(c.r0, c.g1, c.b0)] - m[index(c.r0, c.g0, c.b0)],
    }
}

fn top(c: &Cube, d: Dir, p: i64, m: &[i64]) -> i64 {
    match d {
        Dir::R => m[index(p, c.g1, c.b1)] - m[index(p, c.g1, c.b0)] - m[index(p, c.g0, c.b1)] + m[index(p, c.g0, c.b0)],
        Dir::G => m[index(c.r1, p, c.b1)] - m[index(c.r1, p, c.b0)] - m[index(c.r0, p, c.b1)] + m[index(c.r0, p, c.b0)],
        Dir::B => m[index(c.r1, c.g1, p)] - m[index(c.r1, c.g0, p)] - m[index(c.r0, c.g1, p)] + m[index(c.r0, c.g0, p)],
    }
}

struct Wu {
    w: Vec<i64>,
    mr: Vec<i64>,
    mg: Vec<i64>,
    mb: Vec<i64>,
    m2: Vec<f64>,
}

impl Wu {
    fn variance(&self, c: &Cube) -> f64 {
        let (dr, dg, db) = (volume(c, &self.mr), volume(c, &self.mg), volume(c, &self.mb));
        let xx = volume(c, &self.m2);
        xx - (dr * dr + dg * dg + db * db) as f64 / volume(c, &self.w) as f64
    }

    /// Mejor corte en una dirección: (posición o -1, máximo).
    fn maximize(&self, c: &Cube, d: Dir, first: i64, last: i64, whole: [i64; 4]) -> (i64, f64) {
        let [br, bg, bb, bw] = [bottom(c, d, &self.mr), bottom(c, d, &self.mg), bottom(c, d, &self.mb), bottom(c, d, &self.w)];
        let (mut max, mut cut) = (0.0, -1);
        for i in first..last {
            let (hr, hg, hb, hw) = (br + top(c, d, i, &self.mr), bg + top(c, d, i, &self.mg), bb + top(c, d, i, &self.mb), bw + top(c, d, i, &self.w));
            if hw == 0 {
                continue;
            }
            let mut temp = (hr * hr + hg * hg + hb * hb) as f64 / hw as f64;
            let (hr, hg, hb, hw) = (whole[0] - hr, whole[1] - hg, whole[2] - hb, whole[3] - hw);
            if hw == 0 {
                continue;
            }
            temp += (hr * hr + hg * hg + hb * hb) as f64 / hw as f64;
            if temp > max {
                max = temp;
                cut = i;
            }
        }
        (cut, max)
    }

    fn cut(&self, one: &mut Cube, two: &mut Cube) -> bool {
        let whole = [volume(one, &self.mr), volume(one, &self.mg), volume(one, &self.mb), volume(one, &self.w)];
        let (cr, max_r) = self.maximize(one, Dir::R, one.r0 + 1, one.r1, whole);
        let (cg, max_g) = self.maximize(one, Dir::G, one.g0 + 1, one.g1, whole);
        let (cb, max_b) = self.maximize(one, Dir::B, one.b0 + 1, one.b1, whole);
        two.r1 = one.r1;
        two.g1 = one.g1;
        two.b1 = one.b1;
        if max_r >= max_g && max_r >= max_b {
            if cr < 0 {
                return false;
            }
            one.r1 = cr;
            (two.r0, two.g0, two.b0) = (one.r1, one.g0, one.b0);
        } else if max_g >= max_r && max_g >= max_b {
            one.g1 = cg;
            (two.r0, two.g0, two.b0) = (one.r0, one.g1, one.b0);
        } else {
            one.b1 = cb;
            (two.r0, two.g0, two.b0) = (one.r0, one.g0, one.b1);
        }
        one.vol = (one.r1 - one.r0) * (one.g1 - one.g0) * (one.b1 - one.b0);
        two.vol = (two.r1 - two.r0) * (two.g1 - two.g0) * (two.b1 - two.b0);
        true
    }
}

/// Colores de `QuantizerWu.quantize` (píxeles ARGB) en el orden de Dart,
/// sin repetidos.
pub fn quantize(pixels: &[u32], max_colors: usize) -> Vec<Rgb> {
    // `QuantizerMap`: cuenta por color en orden de primera aparición y, como
    // en Dart, sin los píxeles semitransparentes.
    let mut counts: super::IndexMap<Rgb, i64> = super::IndexMap::default();
    for &p in pixels.iter().filter(|&&p| p >> 24 == 255) {
        *counts.entry(Rgb::from_u32(p)).or_insert(0) += 1;
    }
    let mut wu = Wu { w: vec![0; TOTAL], mr: vec![0; TOTAL], mg: vec![0; TOTAL], mb: vec![0; TOTAL], m2: vec![0.0; TOTAL] };
    for (p, &n) in &counts {
        let (r, g, b) = (p.red as i64, p.green as i64, p.blue as i64);
        let i = index((r >> 3) + 1, (g >> 3) + 1, (b >> 3) + 1);
        wu.w[i] += n;
        wu.mr[i] += r * n;
        wu.mg[i] += g * n;
        wu.mb[i] += b * n;
        wu.m2[i] += (n * (r * r + g * g + b * b)) as f64;
    }
    // computeMoments: acumulado 3D.
    for r in 1..SIDE {
        let (mut area, mut area_r, mut area_g, mut area_b) = ([0i64; 33], [0i64; 33], [0i64; 33], [0i64; 33]);
        let mut area2 = [0f64; 33];
        for g in 1..SIDE {
            let (mut line, mut line_r, mut line_g, mut line_b, mut line2) = (0i64, 0i64, 0i64, 0i64, 0f64);
            for b in 1..SIDE {
                let (i, prev, k) = (index(r, g, b), index(r - 1, g, b), b as usize);
                line += wu.w[i];
                line_r += wu.mr[i];
                line_g += wu.mg[i];
                line_b += wu.mb[i];
                line2 += wu.m2[i];
                area[k] += line;
                area_r[k] += line_r;
                area_g[k] += line_g;
                area_b[k] += line_b;
                area2[k] += line2;
                wu.w[i] = wu.w[prev] + area[k];
                wu.mr[i] = wu.mr[prev] + area_r[k];
                wu.mg[i] = wu.mg[prev] + area_g[k];
                wu.mb[i] = wu.mb[prev] + area_b[k];
                wu.m2[i] = wu.m2[prev] + area2[k];
            }
        }
    }
    // createBoxes
    let mut cubes = vec![Cube::default(); max_colors];
    cubes[0] = Cube { r1: MAX_INDEX, g1: MAX_INDEX, b1: MAX_INDEX, ..Cube::default() };
    let mut variance = vec![0f64; max_colors];
    let (mut next, mut generated) = (0usize, max_colors);
    let mut i = 1;
    while i < max_colors {
        let (mut one, mut two) = (cubes[next], cubes[i]);
        if wu.cut(&mut one, &mut two) {
            cubes[next] = one;
            cubes[i] = two;
            variance[next] = if one.vol > 1 { wu.variance(&one) } else { 0.0 };
            variance[i] = if two.vol > 1 { wu.variance(&two) } else { 0.0 };
        } else {
            cubes[next] = one;
            cubes[i] = two;
            variance[next] = 0.0;
            i -= 1;
        }
        next = 0;
        let mut temp = variance[0];
        for (j, &v) in variance.iter().enumerate().take(i + 1).skip(1) {
            if v > temp {
                temp = v;
                next = j;
            }
        }
        if temp <= 0.0 {
            generated = i + 1;
            break;
        }
        i += 1;
    }
    // createResult + `Map.fromEntries` (los repetidos se juntan).
    let mut colors: Vec<Rgb> = vec![];
    for c in &cubes[..generated] {
        let weight = volume(c, &wu.w);
        if weight > 0 {
            let avg = |m: &[i64]| (volume(c, m) as f64 / weight as f64).round() as u8;
            let rgb = Rgb::new(avg(&wu.mr), avg(&wu.mg), avg(&wu.mb));
            if !colors.contains(&rgb) {
                colors.push(rgb);
            }
        }
    }
    colors
}
