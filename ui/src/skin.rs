//! Texturas de skin y capa en ARGB (port de `app/lib/core/skins.dart`), la
//! cara del avatar (`_SkinFacePainter` de avatar.dart) y la skin y capas de
//! muestra de los tests de Flutter (`test/support/sample_skin.dart`), con los
//! mismos píxeles para que las escenas de paridad coincidan.

/// Textura en ARGB (skin 64×64 / 64×32 o capa 64×32; las HD se muestrean).
#[derive(Clone, Debug, PartialEq)]
pub struct SkinTexture {
    pub width: u32,
    pub height: u32,
    pub argb: Vec<u32>,
}

impl SkinTexture {
    /// Decodifica un PNG (paleta, gris y 16 bits incluidos) a ARGB.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
        dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = dec.read_info().ok()?;
        let mut buf = vec![0; reader.output_buffer_size()?];
        let info = reader.next_frame(&mut buf).ok()?;
        let (w, h) = (info.width, info.height);
        let px = &buf[..info.buffer_size()];
        let channels = match info.color_type {
            png::ColorType::Grayscale => 1,
            png::ColorType::GrayscaleAlpha => 2,
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            png::ColorType::Indexed => return None,
        };
        let argb = px
            .chunks_exact(channels)
            .map(|p| {
                let (r, g, b, a) = match channels {
                    1 => (p[0], p[0], p[0], 255),
                    2 => (p[0], p[0], p[0], p[1]),
                    3 => (p[0], p[1], p[2], 255),
                    _ => (p[0], p[1], p[2], p[3]),
                };
                (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32
            })
            .collect::<Vec<_>>();
        (argb.len() == (w * h) as usize && w > 0).then_some(Self { width: w, height: h, argb })
    }

    /// Factor respecto a la plantilla de 64 de ancho.
    pub fn scale(&self) -> u32 {
        (self.width / 64).clamp(1, 16)
    }

    /// Plantilla antigua (64×32): sin capas en cuerpo y extremidades, y las
    /// extremidades izquierdas copian las derechas.
    pub fn legacy(&self) -> bool {
        self.height * 2 == self.width
    }

    /// Píxel en coordenadas de la plantilla de 64 (0 = transparente).
    pub fn at(&self, u: i32, v: i32) -> u32 {
        let s = self.scale() as i64;
        let (x, y) = (u as i64 * s, v as i64 * s);
        if x < 0 || y < 0 || x >= self.width as i64 || y >= self.height as i64 {
            return 0;
        }
        self.argb[(y * self.width as i64 + x) as usize]
    }

    /// Medidas válidas de una skin para subirla.
    pub fn valid_skin_size(&self) -> bool {
        self.width == 64 && (self.height == 64 || self.height == 32)
    }
}

/// Las texturas de Mojang llegan por `http://`; la caché solo admite `https://`.
pub fn secure_texture_url(url: &str) -> String {
    match url.strip_prefix("http://") {
        Some(rest) => format!("https://{rest}"),
        None => url.to_owned(),
    }
}

/// Cara 8 × 8 en RGBA: la base (8,8) y encima el sombrero (40,8).
pub fn face_rgba(skin: &SkinTexture) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 * 8 * 4);
    for y in 0..8 {
        for x in 0..8 {
            let px = over(skin.at(40 + x, 8 + y), skin.at(8 + x, 8 + y));
            out.extend_from_slice(&[(px >> 16) as u8, (px >> 8) as u8, px as u8, (px >> 24) as u8]);
        }
    }
    out
}

/// `fg` sobre `bg` (ARGB sin premultiplicar).
fn over(fg: u32, bg: u32) -> u32 {
    let fa = (fg >> 24) as f64 / 255.0;
    let ba = (bg >> 24) as f64 / 255.0;
    let oa = fa + ba * (1.0 - fa);
    if oa == 0.0 {
        return 0;
    }
    let ch = |s: u32| {
        let f = ((fg >> s) & 0xFF) as f64;
        let b = ((bg >> s) & 0xFF) as f64;
        (((f * fa + b * ba * (1.0 - fa)) / oa).round() as u32) << s
    };
    ((oa * 255.0).round() as u32) << 24 | ch(16) | ch(8) | ch(0)
}

// --- Muestras (sample_skin.dart) ----------------------------------------------

fn hash(x: i64, y: i64) -> i64 {
    let h = (x * 374761393 + y * 668265263) & 0x7fffffff;
    ((h ^ (h >> 13)) * 1274126177) & 0x7fffffff
}

/// Color con algo de ruido para que parezca tela.
fn noisy(argb: u32, x: i64, y: i64) -> u32 {
    let amount = 0.07;
    let k = 1.0 + ((hash(x, y) % 1000) as f64 / 1000.0 - 0.5) * 2.0 * amount;
    let ch = |s: u32| ((((argb >> s) & 0xFF) as f64 * k).round().clamp(0.0, 255.0) as u32) << s;
    (argb & 0xFF00_0000) | ch(16) | ch(8) | ch(0)
}

struct Canvas {
    w: i64,
    h: i64,
    px: Vec<u32>,
}

impl Canvas {
    fn new(w: i64, h: i64) -> Self {
        Self { w, h, px: vec![0; (w * h) as usize] }
    }

    fn set(&mut self, x: i64, y: i64, argb: u32, noise: bool) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        self.px[(y * self.w + x) as usize] = if noise { noisy(argb, x, y) } else { argb };
    }

    fn fill(&mut self, x: i64, y: i64, fw: i64, fh: i64, argb: u32) {
        self.fill_n(x, y, fw, fh, argb, true);
    }

    fn fill_n(&mut self, x: i64, y: i64, fw: i64, fh: i64, argb: u32, noise: bool) {
        for j in 0..fh {
            for i in 0..fw {
                self.set(x + i, y + j, argb, noise);
            }
        }
    }

    /// Toda la UV de una caja (las seis caras).
    fn box_(&mut self, u: i64, v: i64, bw: i64, bh: i64, d: i64, argb: u32) {
        self.fill(u + d, v, bw * 2, d, argb);
        self.fill(u, v + d, 2 * d + 2 * bw, bh, argb);
    }

    /// Franja horizontal en las cuatro caras laterales de una caja.
    #[allow(clippy::too_many_arguments)]
    fn band(&mut self, u: i64, v: i64, bw: i64, d: i64, from_row: i64, rows: i64, argb: u32) {
        self.fill(u, v + d + from_row, 2 * d + 2 * bw, rows, argb);
    }

    fn texture(self) -> SkinTexture {
        SkinTexture { width: self.w as u32, height: self.h as u32, argb: self.px }
    }
}

/// Skin 64×64 de brazos delgados: gorro rojo, sudadera turquesa y vaqueros.
pub fn sample_skin() -> SkinTexture {
    const SKIN: u32 = 0xFFE0AC86;
    const SKIN_SHADE: u32 = 0xFFC98F6B;
    const HAIR: u32 = 0xFF3B2618;
    const HOODIE: u32 = 0xFF2E8C83;
    const HOODIE_DARK: u32 = 0xFF226860;
    const BEANIE: u32 = 0xFFD9573F;
    const PANTS: u32 = 0xFF2D3550;
    const SHOE: u32 = 0xFF2A2A2E;
    const SOLE: u32 = 0xFFE8E4DA;
    let mut c = Canvas::new(64, 64);
    // Cabeza.
    c.box_(0, 0, 8, 8, 8, SKIN);
    c.fill(8, 0, 8, 8, HAIR);
    c.band(0, 0, 8, 8, 0, 2, HAIR);
    c.fill(24, 8, 8, 6, HAIR);
    c.fill(0, 8, 2, 5, HAIR);
    c.fill(22, 8, 2, 5, HAIR);
    // Cara (frente en 8,8).
    for x in [1, 5] {
        c.set(8 + x, 12, 0xFFFFFFFF, false);
        c.set(9 + x, 12, 0xFF1F3A5F, false);
    }
    c.fill(10, 11, 1, 1, HAIR);
    c.fill(13, 11, 1, 1, HAIR);
    c.fill_n(11, 14, 2, 1, 0xFF9C4A3C, false);
    c.fill(10, 13, 4, 1, SKIN_SHADE);
    // Gorro (capa del sombrero en 32,0).
    c.fill(40, 0, 8, 8, BEANIE);
    c.band(32, 0, 8, 8, 0, 3, BEANIE);
    c.band(32, 0, 8, 8, 2, 1, 0xFFB8402D);
    c.set(43, 0, 0xFFF2E6D8, false);
    c.set(44, 0, 0xFFF2E6D8, false);
    // Cuerpo.
    c.box_(16, 16, 8, 12, 4, HOODIE);
    c.fill(23, 20, 2, 12, HOODIE_DARK);
    c.fill(21, 28, 6, 3, HOODIE_DARK);
    c.fill(22, 20, 4, 1, 0xFFF2E6D8);
    // Brazos (delgados: 3 de ancho).
    c.box_(40, 16, 3, 12, 4, HOODIE);
    c.band(40, 16, 3, 4, 9, 3, SKIN);
    c.fill(47, 16, 3, 4, SKIN);
    c.box_(32, 48, 3, 12, 4, HOODIE);
    c.band(32, 48, 3, 4, 9, 3, SKIN);
    c.fill(39, 48, 3, 4, SKIN);
    // Piernas.
    c.box_(0, 16, 4, 12, 4, PANTS);
    c.band(0, 16, 4, 4, 9, 3, SHOE);
    c.band(0, 16, 4, 4, 11, 1, SOLE);
    c.box_(16, 48, 4, 12, 4, PANTS);
    c.band(16, 48, 4, 4, 9, 3, SHOE);
    c.band(16, 48, 4, 4, 11, 1, SOLE);
    // Capucha caída sobre la espalda (capa exterior del cuerpo).
    c.fill(32, 36, 8, 3, HOODIE_DARK);
    c.texture()
}

/// Colores (fondo, emblema) de las dos capas de muestra.
pub const SAMPLE_CAPE_1: (u32, u32) = (0xFF4B2D7F, 0xFFE5C04A);
pub const SAMPLE_CAPE_2: (u32, u32) = (0xFF1F5E46, 0xFFE8E4DA);

/// Capa 64×32 con un rombo.
pub fn sample_cape(color: u32, emblem: u32) -> SkinTexture {
    let mut c = Canvas::new(64, 32);
    c.box_(0, 0, 10, 16, 1, color);
    for y in 0..16 {
        for x in 0..10 {
            let dx = (x as f64 - 4.5).abs();
            let dy = (y as f64 - 7.5).abs();
            if dx / 3.5 + dy / 5.5 <= 1.0 {
                c.set(1 + x, 1 + y, emblem, true);
            }
            if y == 0 || y == 15 {
                c.set(1 + x, 1 + y, emblem, true);
            }
        }
    }
    c.texture()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(w: u32, h: u32, color: png::ColorType, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(color);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(data).unwrap();
        out
    }

    #[test]
    fn textura_y_plantilla() {
        let skin = sample_skin();
        assert!(!skin.legacy());
        assert_eq!(skin.scale(), 1);
        assert_eq!(skin.at(8, 8) >> 24, 0xFF);
        let legacy = SkinTexture { width: 64, height: 32, argb: skin.argb[..64 * 32].to_vec() };
        assert!(legacy.legacy());
        assert_eq!(legacy.at(-1, 0), 0);
        assert_eq!(legacy.at(64, 0), 0);
        assert!(legacy.valid_skin_size());
        let cape = sample_cape(SAMPLE_CAPE_1.0, SAMPLE_CAPE_1.1);
        assert!(!SkinTexture { width: 32, height: 32, argb: cape.argb[..32 * 32].to_vec() }.valid_skin_size());
    }

    #[test]
    fn url_de_textura_segura() {
        assert_eq!(secure_texture_url("http://textures.minecraft.net/texture/abc"), "https://textures.minecraft.net/texture/abc");
        assert_eq!(secure_texture_url("https://x.y/z.png"), "https://x.y/z.png");
    }

    #[test]
    fn decodifica_rgba_y_gris() {
        let rgba = encode(2, 1, png::ColorType::Rgba, &[255, 0, 0, 255, 0, 0, 255, 128]);
        let t = SkinTexture::decode(&rgba).unwrap();
        assert_eq!((t.width, t.height), (2, 1));
        assert_eq!(t.argb, vec![0xFFFF0000, 0x800000FF]);
        let gray = encode(1, 1, png::ColorType::Grayscale, &[7]);
        assert_eq!(SkinTexture::decode(&gray).unwrap().argb, vec![0xFF070707]);
        assert!(SkinTexture::decode(b"no es png").is_none());
    }

    #[test]
    fn cara_con_sombrero_encima() {
        let mut skin = sample_skin();
        let face = face_rgba(&skin);
        assert_eq!(face.len(), 256);
        // Abajo el sombrero de la muestra es transparente: se ve la base.
        let base = skin.at(8, 15);
        assert_eq!(&face[224..228], &[(base >> 16) as u8, (base >> 8) as u8, base as u8, 255]);
        // Arriba lo tapa el gorro (sombrero opaco en 40,8).
        let hat = skin.at(40, 8);
        assert_eq!(&face[..4], &[(hat >> 16) as u8, (hat >> 8) as u8, hat as u8, 255]);
        // Medio transparente: mezcla.
        skin.argb[15 * 64 + 40] = 0x80FFFFFF;
        assert!(face_rgba(&skin)[224] > face[224]);
    }
}
