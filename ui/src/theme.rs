//! Estilos de serie y colores del tema (`app/lib/ui/theme/{styles,tokens}.dart`):
//! paleta de cada estilo, acento propio o elegido y tonos HCT derivados.
//! `controller` copia el resultado a los globals `Palette` y `Theme`.

use material_colors::{hct::Hct, palette::TonalPalette};

/// `AppStyle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Style {
    /// Gris neutro y plano, acento blanco y anillos que derivan.
    #[default]
    Graphite,
    /// Negro con dorado y una esfera ASCII que gira.
    Sebaspi,
    /// Paisaje de bloques al atardecer.
    Landscape,
    /// Halos suaves del acento.
    Aurora,
}

impl Style {
    pub const ALL: [Style; 4] = [Style::Graphite, Style::Sebaspi, Style::Landscape, Style::Aurora];

    /// Nombre en `ui.json` (`AppStyle.name`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Graphite => "graphite",
            Self::Sebaspi => "sebaspi",
            Self::Landscape => "landscape",
            Self::Aurora => "aurora",
        }
    }

    /// `AppStyle.parse`: desconocido = grafito.
    pub fn parse(name: &str) -> Self {
        match name {
            "sebaspi" => Self::Sebaspi,
            "landscape" => Self::Landscape,
            "aurora" => Self::Aurora,
            _ => Self::Graphite,
        }
    }

    /// Acento propio del estilo (ARGB).
    pub fn accent(self) -> u32 {
        match self {
            Self::Graphite => 0xFFFFFFFF,
            Self::Sebaspi => 0xFFF1C40F,
            Self::Landscape => 0xFFFFA94D,
            Self::Aurora => 0xFF35D69B,
        }
    }

    /// Radios (control, tarjeta, panel).
    pub fn radii(self) -> (f32, f32, f32) {
        match self {
            Self::Graphite => (8.0, 8.0, 10.0),
            _ => (8.0, 12.0, 16.0),
        }
    }
}

/// `AccentSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentSet {
    pub accent: u32,
    pub hover: u32,
    pub on_accent: u32,
    pub container: u32,
    pub on_container: u32,
    pub glow: u32,
}

fn argb(rgb: material_colors::color::Rgb) -> u32 {
    0xFF00_0000 | ((rgb.red as u32) << 16) | ((rgb.green as u32) << 8) | rgb.blue as u32
}

fn hct(argb_: u32) -> Hct {
    Hct::new(material_colors::color::Rgb::from_u32(argb_ & 0x00FF_FFFF))
}

impl AccentSet {
    /// `AccentSet.fromSeed`: tonos de la semilla con croma 40–90.
    pub fn from_seed(seed: u32) -> Self {
        let h = hct(seed);
        let p = TonalPalette::of(h.get_hue(), h.get_chroma().clamp(40.0, 90.0));
        let soft = TonalPalette::of(h.get_hue(), 24.0);
        Self {
            accent: argb(p.tone(74)),
            hover: argb(p.tone(82)),
            on_accent: argb(p.tone(12)),
            container: argb(soft.tone(26)),
            on_container: argb(p.tone(90)),
            glow: argb(p.tone(60)),
        }
    }
}

/// `AppColors` (ARGB).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colors {
    pub background: u32,
    pub background_deep: u32,
    pub surface1: u32,
    pub surface2: u32,
    pub surface3: u32,
    pub border: u32,
    pub border_strong: u32,
    pub text: u32,
    pub text_secondary: u32,
    pub text_muted: u32,
    pub accent: AccentSet,
    pub success: u32,
    pub warning: u32,
    pub danger: u32,
    pub info: u32,
    /// Base casi opaca bajo tarjetas; 0 = superficies de cristal.
    pub panel_base: u32,
}

impl Colors {
    /// `AppColors.fromSeed`.
    pub fn from_seed(seed: u32) -> Self {
        let h = hct(seed);
        let tint = TonalPalette::of(h.get_hue(), 10.0);
        let t = |tone| argb(tint.tone(tone));
        Self {
            background: t(6),
            background_deep: t(4),
            surface1: 0x0FFFFFFF,
            surface2: 0x17FFFFFF,
            surface3: 0x21FFFFFF,
            border: 0x14FFFFFF,
            border_strong: 0x26FFFFFF,
            text: t(96),
            text_secondary: t(78),
            text_muted: t(60),
            accent: AccentSet::from_seed(seed),
            success: 0xFF4ADE80,
            warning: 0xFFFBBF24,
            danger: 0xFFF87171,
            info: 0xFF7DD3FC,
            panel_base: 0,
        }
    }

    /// `AppColors.graphite`.
    pub fn graphite(accent: Option<AccentSet>) -> Self {
        Self {
            background: 0xFF212121,
            background_deep: 0xFF171717,
            surface1: 0x0DFFFFFF,
            surface2: 0x14FFFFFF,
            surface3: 0x1CFFFFFF,
            border: 0x1FFFFFFF,
            border_strong: 0x33FFFFFF,
            text: 0xFFECECEC,
            text_secondary: 0xFFB4B4B4,
            text_muted: 0xFF8F8F8F,
            accent: accent.unwrap_or(AccentSet {
                accent: 0xFFFFFFFF,
                hover: 0xFFE6E6E6,
                on_accent: 0xFF0D0D0D,
                container: 0x29FFFFFF,
                on_container: 0xFFFFFFFF,
                glow: 0xFFAEBCFF,
            }),
            success: 0xFF10A37F,
            warning: 0xFFE0B04A,
            danger: 0xFFF87171,
            info: 0xFF8FD4FF,
            panel_base: 0xEB212121,
        }
    }

    /// `AppColors.gold`.
    pub fn gold(accent: Option<AccentSet>) -> Self {
        Self {
            background: 0xFF000000,
            background_deep: 0xFF000000,
            surface1: 0x0DFFFFFF,
            surface2: 0x14FFFFFF,
            surface3: 0x1FFFFFFF,
            border: 0x24FFFFFF,
            border_strong: 0x38FFFFFF,
            text: 0xFFE5E5E5,
            text_secondary: 0xFFA8A8A8,
            text_muted: 0xFF767676,
            accent: accent.unwrap_or(AccentSet {
                accent: 0xFFF1C40F,
                hover: 0xFFFAC814,
                on_accent: 0xFF121212,
                container: 0x2EF1C40F,
                on_container: 0xFFF8E187,
                glow: 0xFFF1C40F,
            }),
            success: 0xFF21C45D,
            warning: 0xFFF39C12,
            danger: 0xFFE74C3C,
            info: 0xFF7DD3FC,
            panel_base: 0xF0000000,
        }
    }

    /// `StyleSpec.palette`: colores del estilo con el acento elegido
    /// (`None` = el del estilo).
    pub fn for_style(style: Style, accent: Option<u32>) -> Self {
        match style {
            Style::Graphite => Self::graphite(accent.map(AccentSet::from_seed)),
            Style::Sebaspi => Self::gold(accent.map(AccentSet::from_seed)),
            Style::Landscape | Style::Aurora => Self::from_seed(accent.unwrap_or(style.accent())),
        }
    }
}

/// Tonos de la marca (`BrandMarkPainter`): HCT del acento a 88/72/56/42.
pub fn brand_tones(accent: u32) -> [u32; 4] {
    let h = hct(accent);
    [88.0, 72.0, 56.0, 42.0].map(|t| argb(Hct::from(h.get_hue(), h.get_chroma(), t).into()))
}

/// Color con el tono desplazado (`HSLColor.withHue(hue + delta)`), para el
/// segundo halo del fondo «aurora».
pub fn rotate_hue(argb_: u32, delta: f64) -> u32 {
    let (r, g, b) = (((argb_ >> 16) & 0xff) as f64 / 255.0, ((argb_ >> 8) & 0xff) as f64 / 255.0, (argb_ & 0xff) as f64 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    let s = if d == 0.0 { 0.0 } else { d / (1.0 - (2.0 * l - 1.0).abs()) };
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let h = (h.rem_euclid(360.0) + delta).rem_euclid(360.0);
    // HSLColor.toColor
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let q = |v: f64| (((v + m) * 255.0).round().clamp(0.0, 255.0)) as u32;
    (argb_ & 0xFF00_0000) | (q(r1) << 16) | (q(g1) << 8) | q(b1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estilos_por_nombre() {
        assert_eq!(Style::parse("aurora"), Style::Aurora);
        assert_eq!(Style::parse("raro"), Style::Graphite);
    }

    #[test]
    fn grafito_sin_acento_es_el_de_los_goldens() {
        let c = Colors::for_style(Style::Graphite, None);
        assert_eq!(c.accent.accent, 0xFFFFFFFF);
        assert_eq!(c.background, 0xFF212121);
    }

    #[test]
    fn marca_gris_con_acento_blanco() {
        // `Hct.from(hue, chroma, tono)` del blanco en Dart.
        assert_eq!(brand_tones(0xFFFFFFFF), [0xFFDCDDDD, 0xFFB0B1B1, 0xFF858687, 0xFF626364]);
    }

    #[test]
    fn girar_tono() {
        assert_eq!(rotate_hue(0xFFFF0000, 120.0), 0xFF00FF00);
        assert_eq!(rotate_hue(0x80FFFFFF, 50.0), 0x80FFFFFF);
    }
}
