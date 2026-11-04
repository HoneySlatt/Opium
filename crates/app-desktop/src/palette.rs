//! The cover palette, from the core, in egui's colours.

use egui::Color32;

/// Cover colours, best first. Repeats the best when the cover has fewer.
pub type Palette = [Color32; 3];

fn color([r, g, b]: [u8; 3]) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// A palette as the shared workings give it, in egui's colours.
pub fn colors(palette: desktop::palette::Palette) -> Palette {
    palette.map(color)
}

/// Pull a colour into the theme's band (see `music_core::palette::fit`).
pub fn fit(c: Color32, light: bool) -> Color32 {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    color(music_core::palette::fit([r, g, b], light))
}

/// Pull a colour into the band of a page header's wash: on a dark theme,
/// brighter and more colourful than [`fit`] allows, for a wash that shows.
pub fn fit_header(c: Color32, light: bool) -> Color32 {
    if light {
        return fit(c, true);
    }
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    let band = music_core::palette::Band {
        lightness: (0.42, 0.66),
        chroma: 0.24,
    };
    color(music_core::palette::fit_in([r, g, b], band))
}
