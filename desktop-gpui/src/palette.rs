//! The cover palette, from the core, in GPUI's colours, as the egui
//! window's `palette.rs`.

use gpui::Rgba;

use crate::color::{bytes, from_bytes};

/// Cover colours, best first. Repeats the best when the cover has fewer.
pub type Palette = [Rgba; 3];

fn color([r, g, b]: [u8; 3]) -> Rgba {
    from_bytes([r, g, b, 255])
}

/// A palette as the shared workings give it, in GPUI's colours.
pub fn colors(palette: desktop::palette::Palette) -> Palette {
    palette.map(color)
}

/// Pull a colour into the theme's band (see `music_core::palette::fit`).
pub fn fit(c: Rgba, light: bool) -> Rgba {
    let [r, g, b, _] = bytes(c);
    color(music_core::palette::fit([r, g, b], light))
}

/// Pull a colour into the band of a page header's wash: on a dark theme,
/// brighter and more colourful than [`fit`] allows, for a wash that shows.
pub fn fit_header(c: Rgba, light: bool) -> Rgba {
    if light {
        return fit(c, true);
    }
    let [r, g, b, _] = bytes(c);
    let band = music_core::palette::Band {
        lightness: (0.42, 0.66),
        chroma: 0.24,
    };
    color(music_core::palette::fit_in([r, g, b], band))
}
