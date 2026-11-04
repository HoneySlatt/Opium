//! Colour maths in OKLab, the space the design mixes in (`color-mix(in oklab, …)`).

use egui::Color32;
pub use opium_core::color::{Oklab, to_linear};

/// A colour in OKLab, from the core's maths.
pub fn oklab(c: Color32) -> Oklab {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    Oklab::from_rgb([r, g, b])
}

pub fn from_oklab(o: Oklab) -> Color32 {
    let [r, g, b] = o.to_rgb();
    Color32::from_rgb(r, g, b)
}

/// `color-mix(in oklab, a p, b)`: `p` of `a`, the rest of `b`. Both opaque.
pub fn mix(a: Color32, b: Color32, p: f32) -> Color32 {
    let (x, y) = (oklab(a), oklab(b));
    from_oklab(Oklab {
        l: x.l * p + y.l * (1.0 - p),
        a: x.a * p + y.a * (1.0 - p),
        b: x.b * p + y.b * (1.0 - p),
    })
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
            let back = from_oklab(oklab(c));
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
        let w = oklab(Color32::WHITE);
        assert!((w.l - 1.0).abs() < 1e-3 && w.a.abs() < 1e-3 && w.b.abs() < 1e-3);
    }

    #[test]
    fn hex_parses_and_rejects() {
        assert_eq!(hex("#7c3aed"), Some(Color32::from_rgb(0x7c, 0x3a, 0xed)));
        assert_eq!(hex("7c3aed"), None);
        assert_eq!(hex("#7c3"), None);
    }
}
