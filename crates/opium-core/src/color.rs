//! Colour maths in OKLab, the space the designs mix and clamp colours in.

/// An sRGB channel in linear light.
pub fn to_linear(c: u8) -> f32 {
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
    pub fn from_rgb([r, g, b]: [u8; 3]) -> Self {
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

    pub fn to_rgb(self) -> [u8; 3] {
        let l = (self.l + 0.396_337_78 * self.a + 0.215_803_76 * self.b).powi(3);
        let m = (self.l - 0.105_561_346 * self.a - 0.063_854_17 * self.b).powi(3);
        let s = (self.l - 0.089_484_18 * self.a - 1.291_485_5 * self.b).powi(3);
        [
            to_srgb(4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s),
            to_srgb(-1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s),
            to_srgb(-0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s),
        ]
    }
}

impl Oklab {
    /// How colourful it is.
    pub fn chroma(self) -> f32 {
        self.a.hypot(self.b)
    }
}

/// A colour in OKLCH (`hue` in degrees), its chroma lowered until it fits
/// in sRGB.
pub fn oklch(l: f32, c: f32, hue: f32) -> [u8; 3] {
    let (sin, cos) = hue.to_radians().sin_cos();
    let mut chroma = c;
    loop {
        let o = Oklab {
            l,
            a: chroma * cos,
            b: chroma * sin,
        };
        let rgb = o.to_rgb();
        let back = Oklab::from_rgb(rgb);
        let fits = (back.l - l).abs() < 0.01
            && ((back.a - o.a).powi(2) + (back.b - o.b).powi(2)).sqrt() < 0.01;
        if fits || chroma < 0.005 {
            return rgb;
        }
        chroma -= 0.005;
    }
}

/// A colour's chroma and hue (degrees), in OKLCH.
pub fn chroma_hue(rgb: [u8; 3]) -> (f32, f32) {
    let o = Oklab::from_rgb(rgb);
    (o.a.hypot(o.b), o.b.atan2(o.a).to_degrees())
}
