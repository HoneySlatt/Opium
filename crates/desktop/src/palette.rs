//! A cover's palette, from the core, in plain RGB: the window turns it into
//! its own colours.

/// Cover colours, best first. Repeats the best when the cover has fewer.
pub type Palette = [[u8; 3]; 3];

/// The palette of an encoded image (JPEG, PNG…).
pub fn from_image(bytes: &[u8]) -> Option<Palette> {
    music_core::palette::from_image(bytes)
}
