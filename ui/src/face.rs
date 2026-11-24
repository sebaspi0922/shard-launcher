//! Cara pixelada generada a partir del nombre (`_FacePainter` de
//! avatar.dart), para cuentas sin skin.

use crate::art::{stable_hash, tone};

/// 8 × 8 píxeles RGBA.
pub fn generated_face(name: &str) -> Vec<u8> {
    let hue = (stable_hash(&name.to_lowercase()) % 360) as f64;
    let t = |shift: f64, chroma: f64, tone_: f64| tone(hue + shift, chroma, tone_);
    // 0 piel, 1 sombra, 2 pelo, 3 ojo blanco, 4 pupila, 5 boca.
    let paints = [t(40.0, 24.0, 72.0), t(40.0, 24.0, 64.0), t(0.0, 40.0, 30.0), t(180.0, 30.0, 92.0), t(200.0, 50.0, 30.0), t(10.0, 40.0, 40.0)];
    const ROWS: [&str; 8] = ["22222222", "22222222", "20000002", "00000000", "03400430", "00011000", "00155100", "10000001"];
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
