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

/// A blob of colour in a [`wash`]: where its middle is and how far it
/// reaches, as fractions of the rect; how strong it is there, and where it
/// has faded out, as a fraction of its reach.
#[derive(Clone, Copy)]
pub struct Blob {
    pub center: (f32, f32),
    pub radii: (f32, f32),
    pub color: Rgba,
    pub alpha: f32,
    pub fade: f32,
}

/// What a wash is painted from: the base's and the blobs' bytes with
/// their numbers, whether it fades to the base, and its size in pixels.
type Washed = ([u8; 4], Vec<([u8; 4], [u32; 6])>, bool, u32, u32);

thread_local! {
    static WASHED: RefCell<HashMap<Washed, Arc<RenderImage>>> = RefCell::default();
}

/// Blobs over `base`, `w` by `h` pixels, as egui's `ui::wash` meshes them:
/// a grid of 24 by 16 cells, each vertex the blobs composited over `base`
/// (and, with `fade_to_base`, `base` again over the lower part), each
/// triangle blending its corners' bytes straight across. The rim's its
/// own business, as for [`diagonal`].
pub fn wash(base: Rgba, blobs: &[Blob], fade_to_base: bool, w: u32, h: u32) -> Arc<RenderImage> {
    let key: Washed = (
        bytes(base),
        blobs
            .iter()
            .map(|b| {
                let f = |v: f32| v.to_bits();
                (
                    bytes(b.color),
                    [
                        f(b.center.0),
                        f(b.center.1),
                        f(b.radii.0),
                        f(b.radii.1),
                        f(b.alpha),
                        f(b.fade),
                    ],
                )
            })
            .collect(),
        fade_to_base,
        w,
        h,
    );
    if let Some(image) = WASHED.with_borrow(|p| p.get(&key).cloned()) {
        return image;
    }
    const COLS: usize = 24;
    const ROWS: usize = 16;
    let veil = |fy: f32| {
        if !fade_to_base || fy < 0.30 {
            0.0
        } else if fy < 0.62 {
            0.45 * (fy - 0.30) / (0.62 - 0.30)
        } else {
            0.45 + 0.55 * (fy - 0.62) / 0.38
        }
    };
    // egui's premultiplied bytes: `alpha` then `over`.
    let premultiplied = |c: Rgba, a: f32| -> [f32; 4] {
        let [r, g, b, _] = bytes(c);
        let a = (a.clamp(0.0, 1.0) * 255.0).round();
        let m = |v: u8| (f32::from(v) * a / 255.0).round();
        [m(r), m(g), m(b), a]
    };
    let over = |top: [f32; 4], bottom: [f32; 4]| -> [f32; 4] {
        let a = top[3] / 255.0;
        let m = |t: f32, b: f32| (t + b * (1.0 - a)).round().min(255.0);
        [
            m(top[0], bottom[0]),
            m(top[1], bottom[1]),
            m(top[2], bottom[2]),
            m(top[3], bottom[3]),
        ]
    };
    let mut vertices = [[0.0f32; 4]; (COLS + 1) * (ROWS + 1)];
    for row in 0..=ROWS {
        for col in 0..=COLS {
            let f = (col as f32 / COLS as f32, row as f32 / ROWS as f32);
            let mut color = premultiplied(base, 1.0);
            for blob in blobs {
                let d = ((f.0 - blob.center.0) / blob.radii.0)
                    .hypot((f.1 - blob.center.1) / blob.radii.1);
                let t = (1.0 - d / blob.fade).clamp(0.0, 1.0);
                color = over(premultiplied(blob.color, blob.alpha * t), color);
            }
            let a = veil(f.1);
            if a > 0.0 {
                color = over(premultiplied(base, a), color);
            }
            vertices[row * (COLS + 1) + col] = color;
        }
    }
    let (w, h) = (w.max(1), h.max(1));
    let mut buffer = image::RgbaImage::new(w, h);
    for (x, y, pixel) in buffer.enumerate_pixels_mut() {
        let gx = (x as f32 + 0.5) / w as f32 * COLS as f32;
        let gy = (y as f32 + 0.5) / h as f32 * ROWS as f32;
        let (col, row) = ((gx as usize).min(COLS - 1), (gy as usize).min(ROWS - 1));
        let (u, v) = (gx - col as f32, gy - row as f32);
        let at = |c: usize, r: usize| vertices[r * (COLS + 1) + c];
        // The cell's two triangles, split from its top right to its bottom
        // left: (top left, top right, bottom left), (top right, bottom
        // right, bottom left).
        let (a, b, c, wa, wb, wc) = if u + v <= 1.0 {
            (
                at(col, row),
                at(col + 1, row),
                at(col, row + 1),
                1.0 - u - v,
                u,
                v,
            )
        } else {
            (
                at(col + 1, row),
                at(col + 1, row + 1),
                at(col, row + 1),
                1.0 - v,
                u + v - 1.0,
                1.0 - u,
            )
        };
        let mix = |i: usize| {
            (a[i] * wa + b[i] * wb + c[i] * wc)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        // Opaque over an opaque base; GPUI's images are BGRA.
        *pixel = image::Rgba([mix(2), mix(1), mix(0), mix(3)]);
    }
    let image = Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]));
    WASHED.with_borrow_mut(|p| {
        // A window resized paints it anew at each width: the last few kept.
        if p.len() > 8 {
            p.clear();
        }
        p.insert(key, image.clone());
    });
    image
}
