//! Cara pixelada generada a partir del nombre (`_FacePainter` de
//! avatar.dart), para cuentas sin skin.

use crate::art::{Canvas, Rgba, stable_hash, tone};

/// 0 piel, 1 sombra, 2 pelo, 3 ojo blanco, 4 pupila, 5 boca.
const ROWS: [&str; 8] = ["22222222", "22222222", "20000002", "00000000", "03400430", "00011000", "00155100", "10000001"];

fn paints(name: &str) -> [Rgba; 6] {
    let hue = (stable_hash(&name.to_lowercase()) % 360) as f64;
    let t = |shift: f64, chroma: f64, tone_: f64| tone(hue + shift, chroma, tone_);
    [t(40.0, 24.0, 72.0), t(40.0, 24.0, 64.0), t(0.0, 40.0, 30.0), t(180.0, 30.0, 92.0), t(200.0, 50.0, 30.0), t(10.0, 40.0, 40.0)]
}

/// La cara pintada como en Flutter: un rectángulo con antialias por celda,
/// medio píxel más grande para tapar la junta (a 5,25 px por celda se ven
/// costuras finas, igual que allí). `size` lógico, `scale` físico.
pub fn painted_face(name: &str, size: f64, scale: f64) -> slint::Image {
    let paints = paints(name);
    let n = ((size * scale).round() as usize).max(1);
    let mut cv = Canvas::new(n, n);
    let cell = size / 8.0 * scale;
    for (y, row) in ROWS.iter().enumerate() {
        for (x, ch) in row.bytes().enumerate() {
            let c = paints[(ch - b'0') as usize];
            let (l, t) = (x as f64 * cell, y as f64 * cell);
            cv.rect(l, t, l + cell + 0.5 * scale, t + cell + 0.5 * scale, |_, _| c);
        }
    }
    let mut px = cv.to_bgra();
    for p in px.as_chunks_mut::<4>().0.iter_mut() {
        p.swap(0, 2);
    }
    slint::Image::from_rgba8(slint::SharedPixelBuffer::clone_from_slice(&px, n as u32, n as u32))
}

/// 8 × 8 píxeles RGBA.
pub fn generated_face(name: &str) -> Vec<u8> {
    let paints = paints(name);
    let mut out = Vec::with_capacity(8 * 8 * 4);
    for row in ROWS {
        for ch in row.bytes() {
            let c = paints[(ch - b'0') as usize];
            let v = |x: f64| x.round().clamp(0.0, 255.0) as u8;
            out.extend_from_slice(&[v(c.r), v(c.g), v(c.b), 255]);
        }
    }
    out
}

pub fn generated_face_image(name: &str) -> slint::Image {
    let px = generated_face(name);
    slint::Image::from_rgba8(slint::SharedPixelBuffer::clone_from_slice(&px, 8, 8))
}

#[cfg(test)]
mod tests {
    #[test]
    fn misma_cara_para_el_mismo_nombre_sin_mayusculas() {
        assert_eq!(super::generated_face("NadiaK"), super::generated_face("nadiak"));
        assert_ne!(super::generated_face("NadiaK"), super::generated_face("Pruebas_01"));
        assert_eq!(super::generated_face("x").len(), 256);
    }
}
