//! Geist (SIL Open Font License, `crates/app-desktop/assets/fonts/OFL.txt`) in
//! the four weights the design uses, the egui window's files. GPUI has font
//! weights, so they are one family; the scripts Geist lacks fall back to the
//! system's fonts in GPUI's text system itself, which the egui window had to
//! look for (`crates/app-desktop/src/fonts.rs`).

use std::borrow::Cow;

use gpui::{App, FontWeight};

pub const FAMILY: &str = "Geist";

#[derive(Clone, Copy)]
pub enum Weight {
    Regular,
    Medium,
    SemiBold,
    Bold,
}

impl Weight {
    pub fn gpui(self) -> FontWeight {
        match self {
            Weight::Regular => FontWeight::NORMAL,
            Weight::Medium => FontWeight::MEDIUM,
            Weight::SemiBold => FontWeight::SEMIBOLD,
            Weight::Bold => FontWeight::BOLD,
        }
    }
}

/// How tall egui lays a line of Geist out, in points, at `size`: its
/// ascent less its descent (1005 and −295 of 1000 units, no line gap), so
/// that text centred on a point in egui sits on the same baseline here.
pub fn line_height(size: f32) -> f32 {
    1.3 * size
}

pub fn install(cx: &mut App) {
    let fonts: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(include_bytes!(
            "../../crates/app-desktop/assets/fonts/Geist-Regular.otf"
        )),
        Cow::Borrowed(include_bytes!(
            "../../crates/app-desktop/assets/fonts/Geist-Medium.otf"
        )),
        Cow::Borrowed(include_bytes!(
            "../../crates/app-desktop/assets/fonts/Geist-SemiBold.otf"
        )),
        Cow::Borrowed(include_bytes!(
            "../../crates/app-desktop/assets/fonts/Geist-Bold.otf"
        )),
    ];
    cx.text_system()
        .add_fonts(fonts)
        .expect("Geist's files are valid fonts");
}
