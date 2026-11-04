//! What egui drew as meshes of coloured corners and GPUI's own gradients
//! don't reproduce: GPUI blends a gradient's two stops in OKLab or in a
//! space of its own, egui's meshes straight between the bytes of their
//! corners. Painted here once, as an image the size it is drawn.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use gpui::{RenderImage, Rgba};

use crate::color::{bytes, lerp, mix};

/// The two colours' bytes and the size in pixels.
type Painted = ([u8; 4], [u8; 4], u32);

thread_local! {
    static PAINTED: RefCell<HashMap<Painted, Arc<RenderImage>>> = RefCell::default();
}

/// A square `pixels` wide of `from` at its top left to `to` at its bottom
/// right, as egui's `ui::gradient` meshes it: a fan from its middle, the two
/// mixed in OKLab, to its corners, whose four triangles blend each byte
/// straight across. That is `from`, the middle and `to` along the
/// diagonal, straight between them; the rim's corners its own business
/// (the image is rounded where it is drawn).
pub fn diagonal(from: Rgba, to: Rgba, pixels: u32) -> Arc<RenderImage> {
    let key = (bytes(from), bytes(to), pixels);
    if let Some(image) = PAINTED.with_borrow(|p| p.get(&key).cloned()) {
        return image;
    }
    let n = pixels.max(1);
    let middle = mix(to, from, 0.5);
    let half = n as f32 / 2.0;
    let mut buffer = image::RgbaImage::new(n, n);
    for (x, y, pixel) in buffer.enumerate_pixels_mut() {
        let (dx, dy) = (x as f32 + 0.5 - half, y as f32 + 0.5 - half);
        // From −1 at the top left corner to 1 at the bottom right.
        let s = ((dx + dy) / (2.0 * half)).clamp(-1.0, 1.0);
        let c = if s < 0.0 {
            lerp(middle, from, -s)
        } else {
            lerp(middle, to, s)
        };
        let [r, g, b, a] = bytes(c);
        // GPUI's images are BGRA.
        *pixel = image::Rgba([b, g, r, a]);
    }
    let image = Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]));
    PAINTED.with_borrow_mut(|p| p.insert(key, image.clone()));
    image
}
