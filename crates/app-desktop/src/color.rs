//! Colour maths in OKLab, the space the design mixes in (`color-mix(in oklab, …)`).

use egui::Color32;

fn to_linear(c: u8) -> f32 {
    let c = f32::from(c) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn to_srgb(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let c = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0).round() as u8
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

impl Oklab {
    pub fn from_color(c: Color32) -> Self {
        let [r, g, b, _] = c.to_srgba_unmultiplied();
        let (r, g, b) = (to_linear(r), to_linear(g), to_linear(b));
        let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
        let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
        let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
        Self {
            l: 0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
            a: 1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
            b: 0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
        }
    }

    pub fn to_color(self) -> Color32 {
        let l = (self.l + 0.396_337_78 * self.a + 0.215_803_76 * self.b).powi(3);
        let m = (self.l - 0.105_561_346 * self.a - 0.063_854_17 * self.b).powi(3);
        let s = (self.l - 0.089_484_18 * self.a - 1.291_485_5 * self.b).powi(3);
        Color32::from_rgb(
            to_srgb(4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s),
            to_srgb(-1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s),
            to_srgb(-0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s),
        )
    }
}

/// `color-mix(in oklab, a p, b)`: `p` of `a`, the rest of `b`. Both opaque.
pub fn mix(a: Color32, b: Color32, p: f32) -> Color32 {
    let (x, y) = (Oklab::from_color(a), Oklab::from_color(b));
    Oklab {
        l: x.l * p + y.l * (1.0 - p),
        a: x.a * p + y.a * (1.0 - p),
        b: x.b * p + y.b * (1.0 - p),
    }
    .to_color()
}

/// `color-mix(in oklab, c p, transparent)`: `c` at `p` opacity.
pub fn alpha(c: Color32, p: f32) -> Color32 {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(r, g, b, (p.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// WCAG relative luminance.
pub fn luminance(c: Color32) -> f32 {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b)
}

/// `#rrggbb`.
pub fn hex(s: &str) -> Option<Color32> {
    let s = s.trim().strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oklab_round_trips() {
        for c in [
            Color32::from_rgb(0x7c, 0x3a, 0xed),
            Color32::from_rgb(0x0b, 0x0a, 0x0f),
            Color32::WHITE,
        ] {
            let back = Oklab::from_color(c).to_color();
            let [r, g, b, _] = back.to_array();
            let [r0, g0, b0, _] = c.to_array();
            assert!(
                r.abs_diff(r0) <= 1 && g.abs_diff(g0) <= 1 && b.abs_diff(b0) <= 1,
                "{c:?} → {back:?}"
            );
        }
    }

    #[test]
    fn white_is_oklab_one() {
        let w = Oklab::from_color(Color32::WHITE);
        assert!((w.l - 1.0).abs() < 1e-3 && w.a.abs() < 1e-3 && w.b.abs() < 1e-3);
    }

    #[test]
    fn hex_parses_and_rejects() {
        assert_eq!(hex("#7c3aed"), Some(Color32::from_rgb(0x7c, 0x3a, 0xed)));
        assert_eq!(hex("7c3aed"), None);
        assert_eq!(hex("#7c3"), None);
    }
}
