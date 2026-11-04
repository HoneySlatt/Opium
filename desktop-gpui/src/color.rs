//! Colour maths in OKLab, the space the design mixes in (`color-mix(in oklab, …)`),
//! as the egui window's `color.rs`, on GPUI's colours. A colour is kept as
//! its sRGB bytes would be: the maths round to them as egui's `Color32` does.

use gpui::Rgba;
pub use music_core::color::{Oklab, to_linear};

/// The colour's sRGB bytes, unmultiplied.
pub fn bytes(c: Rgba) -> [u8; 4] {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [b(c.r), b(c.g), b(c.b), b(c.a)]
}

pub fn from_bytes([r, g, b, a]: [u8; 4]) -> Rgba {
    Rgba {
        r: f32::from(r) / 255.0,
        g: f32::from(g) / 255.0,
        b: f32::from(b) / 255.0,
        a: f32::from(a) / 255.0,
    }
}

/// `0xrrggbb`, opaque.
pub const fn rgb(v: u32) -> Rgba {
    Rgba {
        r: ((v >> 16) & 0xff) as f32 / 255.0,
        g: ((v >> 8) & 0xff) as f32 / 255.0,
        b: (v & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

pub const BLACK: Rgba = rgb(0x000000);
pub const WHITE: Rgba = rgb(0xffffff);
pub const TRANSPARENT: Rgba = Rgba {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.0,
};

/// egui's `Color32::from_white_alpha`, `from_black_alpha`: those bytes.
pub fn white_alpha(a: u8) -> Rgba {
    from_bytes([255, 255, 255, a])
}

pub fn black_alpha(a: u8) -> Rgba {
    from_bytes([0, 0, 0, a])
}

/// A colour in OKLab, from the core's maths.
pub fn oklab(c: Rgba) -> Oklab {
    let [r, g, b, _] = bytes(c);
    Oklab::from_rgb([r, g, b])
}

pub fn from_oklab(o: Oklab) -> Rgba {
    let [r, g, b] = o.to_rgb();
    from_bytes([r, g, b, 255])
}

/// `color-mix(in oklab, a p, b)`: `p` of `a`, the rest of `b`. Both opaque.
pub fn mix(a: Rgba, b: Rgba, p: f32) -> Rgba {
    let (x, y) = (oklab(a), oklab(b));
    from_oklab(Oklab {
        l: x.l * p + y.l * (1.0 - p),
        a: x.a * p + y.a * (1.0 - p),
        b: x.b * p + y.b * (1.0 - p),
    })
}

/// `color-mix(in oklab, c p, transparent)`: `c` at `p` opacity.
pub fn alpha(c: Rgba, p: f32) -> Rgba {
    let [r, g, b, _] = bytes(c);
    from_bytes([r, g, b, (p.clamp(0.0, 1.0) * 255.0).round() as u8])
}

/// WCAG relative luminance.
pub fn luminance(c: Rgba) -> f32 {
    let [r, g, b, _] = bytes(c);
    0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b)
}

/// `#rrggbb`.
pub fn hex(s: &str) -> Option<Rgba> {
    let s = s.trim().strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(rgb(v))
}

/// The colour at `t` of the way from `a` to `b`, each byte straight between
/// them: how egui's meshes blend the colours of their corners.
pub fn lerp(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let (x, y) = (bytes(a), bytes(b));
    let l = |i: usize| (f32::from(x[i]) + (f32::from(y[i]) - f32::from(x[i])) * t).round() as u8;
    from_bytes([l(0), l(1), l(2), l(3)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oklab_round_trips() {
        for c in [rgb(0x7c3aed), rgb(0x0b0a0f), WHITE] {
            let back = bytes(from_oklab(oklab(c)));
            let c = bytes(c);
            assert!(
                (0..3).all(|i| back[i].abs_diff(c[i]) <= 1),
                "{c:?} → {back:?}"
            );
        }
    }

    #[test]
    fn hex_parses_and_rejects() {
        assert_eq!(hex("#7c3aed").map(bytes), Some([0x7c, 0x3a, 0xed, 255]));
        assert_eq!(hex("7c3aed"), None);
        assert_eq!(hex("#7c3"), None);
    }
}
