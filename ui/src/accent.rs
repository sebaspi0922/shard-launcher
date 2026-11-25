//! Acento extraído de una imagen de fondo (`app/lib/ui/theme/accent.dart`):
//! la imagen a 96 px de ancho, cuantizador Celebi de 128 colores y la
//! puntuación de Material. Wu, k-means y el escalado se copian de Dart/Skia
//! porque `material-colors` redondea distinto y elige otro color; de ese
//! crate solo se usan Lab/Hct y `Score`, que dan lo mismo que Dart.

use std::path::Path;

use material_colors::color::{Lab, Rgb};
use material_colors::quantize::{PointProvider, PointProviderLab, QuantizerResult};
use material_colors::score::Score;

use crate::art::DartRandom;

mod wu;

type IndexMap<K, V> = indexmap::IndexMap<K, V, std::hash::BuildHasherDefault<ahash::AHasher>>;

/// Presets propios del acento (`kAccentPresets`).
pub const PRESETS: [u32; 8] = [0xFF35D69B, 0xFF4FA8FF, 0xFF9B7BFF, 0xFFFF6F91, 0xFFFFA94D, 0xFF3CC8E0, 0xFFE5C04A, 0xFFE879C6];

/// `accentFromPixels`: color dominante «bonito» de unos píxeles ARGB. `None`
/// si la imagen no tiene color útil. El alfa importa: Wu solo cuenta los
/// opacos y k-means separa colores iguales con distinto alfa.
pub fn from_pixels(pixels: &[u32]) -> Option<u32> {
    if pixels.is_empty() {
        return None;
    }
    let result = celebi(pixels, 128);
    let best = *Score::score(&result.color_to_count, None, Some(Rgb::from_u32(0)), None).first()?;
    let argb = ((best.red as u32) << 16) | ((best.green as u32) << 8) | best.blue as u32;
    if argb == 0 { None } else { Some(0xFF00_0000 | argb) }
}

/// `QuantizerCelebi` de material_color_utilities (Dart): Wu para los
/// centros iniciales y k-means ponderado. `material-colors` trae su propio
/// k-means con 10 iteraciones y el generador aleatorio de Java; el de Dart
/// hace 5 y usa `Random` de Dart, y eso cambia el color elegido.
fn celebi(pixels: &[u32], max_colors: usize) -> QuantizerResult {
    wsmeans(pixels, max_colors, &wu::quantize(pixels, max_colors))
}

/// `QuantizerWsmeans.quantize` de Dart (5 iteraciones).
fn wsmeans(input: &[u32], max_colors: usize, starting: &[Rgb]) -> QuantizerResult {
    let mut pixel_to_count: IndexMap<u32, u32> = IndexMap::default();
    let mut points: Vec<Lab> = vec![];
    for p in input {
        let c = pixel_to_count.entry(*p).or_insert(0);
        *c += 1;
        if *c == 1 {
            points.push(PointProviderLab::lab_from_int(&Rgb::from_u32(*p)));
        }
    }
    let counts: Vec<u32> = pixel_to_count.values().copied().collect();
    let point_count = points.len();
    let cluster_count = max_colors.min(point_count);
    let mut clusters: Vec<Lab> = starting.iter().map(PointProviderLab::lab_from_int).collect();
    if cluster_count > clusters.len() {
        let mut random = DartRandom::new(0x42688);
        let mut indices: Vec<usize> = vec![];
        for _ in 0..cluster_count - clusters.len() {
            let mut index = random.next_int(point_count as u32) as usize;
            while indices.contains(&index) {
                index = random.next_int(point_count as u32) as usize;
            }
            indices.push(index);
        }
        clusters.extend(indices.into_iter().map(|i| points[i]));
    }
    let mut cluster_indices: Vec<usize> = (0..point_count).map(|i| i % cluster_count.max(1)).collect();
    // Matriz de distancias entre centros que persiste entre iteraciones y
    // se ordena por filas en su sitio, como en Dart: después se compara con
    // la j-ésima menor (no con la del centro j) y la casilla propia arrastra
    // un valor viejo. Copiado tal cual para elegir el mismo color.
    let mut sorted = vec![vec![0f64; cluster_count]; cluster_count];
    let mut sums = vec![0u32; cluster_count];
    for iteration in 0..5 {
        for i in 0..cluster_count {
            for j in i + 1..cluster_count {
                let d = PointProviderLab::distance(&clusters[i], &clusters[j]);
                sorted[j][i] = d;
                sorted[i][j] = d;
            }
            sorted[i].sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        }
        let mut moved = 0;
        for i in 0..point_count {
            let prev = cluster_indices[i];
            let prev_d = PointProviderLab::distance(&points[i], &clusters[prev]);
            let mut min = prev_d;
            let mut best = None;
            for j in 0..cluster_count {
                if sorted[prev][j] >= 4.0 * prev_d {
                    continue;
                }
                let d = PointProviderLab::distance(&points[i], &clusters[j]);
                if d < min {
                    min = d;
                    best = Some(j);
                }
            }
            if let Some(j) = best {
                moved += 1;
                cluster_indices[i] = j;
            }
        }
        if moved == 0 && iteration > 0 {
            break;
        }
        let mut acc = vec![[0f64; 3]; cluster_count];
        sums.iter_mut().for_each(|s| *s = 0);
        for i in 0..point_count {
            let (c, n) = (cluster_indices[i], counts[i]);
            sums[c] += n;
            acc[c][0] += points[i].l * n as f64;
            acc[c][1] += points[i].a * n as f64;
            acc[c][2] += points[i].b * n as f64;
        }
        for i in 0..cluster_count {
            let n = sums[i] as f64;
            clusters[i] = if sums[i] == 0 { Lab::new(0.0, 0.0, 0.0) } else { Lab::new(acc[i][0] / n, acc[i][1] / n, acc[i][2] / n) };
        }
    }
    let mut color_to_count: IndexMap<Rgb, u32> = IndexMap::default();
    for i in 0..cluster_count {
        if sums[i] == 0 {
            continue;
        }
        let rgb = PointProviderLab::lab_to_int(&clusters[i]);
        color_to_count.entry(rgb).or_insert(sums[i]);
    }
    QuantizerResult { color_to_count, input_pixel_to_cluster_pixel: IndexMap::default() }
}

/// `accentFromImageBytes`: la imagen a 96 px de ancho (alto proporcional),
/// sin los píxeles casi transparentes. Como en Flutter, los colores salen
/// premultiplicados por el alfa (`toByteData(rawRgba)`) y el escalado se hace
/// sobre píxeles premultiplicados.
pub fn from_image(img: &image::DynamicImage) -> Option<u32> {
    let w = 96u32;
    let h = ((img.height() as f64 * w as f64 / img.width().max(1) as f64).round() as u32).max(1);
    let mut premul = img.to_rgba8();
    for p in premul.pixels_mut() {
        let a = p[3] as u32;
        for k in 0..3 {
            p[k] = ((p[k] as u32 * a + 127) / 255) as u8;
        }
    }
    let pixels: Vec<u32> = resize_skia(&premul, w, h).into_iter().filter(|p| p >> 24 >= 128).collect();
    from_pixels(&pixels)
}

/// Escalado de `instantiateImageCodec(targetWidth:)`: el `bilerp_clamp_8888`
/// de Skia sin mipmaps, en f32 y con el mismo orden de operaciones (sin
/// FMA), y redondeo al par (`cvtps`). Así los bytes salen idénticos a los de
/// Flutter; con cualquier otro filtro cambian unos pocos píxeles y, con
/// ellos, el acento.
fn resize_skia(src: &image::RgbaImage, w: u32, h: u32) -> Vec<u32> {
    let (sw, sh) = (src.width() as f32, src.height() as f32);
    let fract = |v: f32| v - v.floor();
    let mut out = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let cx = (x as f32 + 0.5) * (sw / w as f32);
            let cy = (y as f32 + 0.5) * (sh / h as f32);
            let (fx, fy) = (fract(cx + 0.5), fract(cy + 0.5));
            let mut acc = [0f32; 4];
            for py in [-0.5f32, 0.5] {
                for px in [-0.5f32, 0.5] {
                    let p = src.get_pixel((cx + px).clamp(0.0, sw - 1.0) as u32, (cy + py).clamp(0.0, sh - 1.0) as u32);
                    let area = if px > 0.0 { fx } else { 1.0 - fx } * if py > 0.0 { fy } else { 1.0 - fy };
                    for k in 0..4 {
                        acc[k] += p[k] as f32 * (1.0 / 255.0) * area;
                    }
                }
            }
            let [r, g, b, a] = acc.map(|v| (v.clamp(0.0, 1.0) * 255.0).round_ties_even() as u8);
            out.push(u32::from_be_bytes([a, r, g, b]));
        }
    }
    out
}

pub fn from_path(path: &Path) -> Option<u32> {
    let img = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.decode().ok()?;
    from_image(&img)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    /// Lo que da Flutter (`tools/paridad/flutter/accent_ref_test.dart`).
    const FLUTTER: [(&str, u32); 6] = [
        ("test/fixtures/icons/complementary-reimagined.png", 0xFFBB9199),
        ("test/fixtures/icons/fabulously-optimized.png", 0xFFDBCEB3),
        ("test/fixtures/icons/terralith.png", 0xFF8FB6E2),
        ("test/fixtures/icons/journeymap.png", 0xFF3FD91E),
        ("test/goldens/home_style_landscape_es.png", 0xFF7F5115),
        ("test/goldens/home_style_aurora_es.png", 0xFF27CE94),
    ];

    #[test]
    fn mismo_acento_que_flutter() {
        let app = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../app");
        let mut wrong = vec![];
        for (path, want) in FLUTTER {
            let got = super::from_path(&app.join(path));
            if got != Some(want) {
                wrong.push(format!("{path}: {got:08X?} ≠ {want:08X}"));
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    #[test]
    fn sin_pixeles_no_hay_acento() {
        assert_eq!(super::from_pixels(&[]), None);
    }
}
