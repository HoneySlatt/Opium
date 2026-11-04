//! A cover file at the size it is drawn. A service's cover comes at the size
//! its address asks for, but a folder's own `cover.jpg` is whatever size the
//! user has, and egui's image loader decodes it whole and keeps it whole: a
//! 3000 px cover cost a full-resolution decode and some 36 MB of texture for
//! a 36 px row, with up to four hundred of them in memory at once. So a file
//! is addressed with the size it is wanted at, and read here at that size.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Once};
use std::task::Poll;

use egui::ColorImage;
use egui::load::{ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint};

/// What addresses a cover file at a size: `cover://256//home/…/cover.jpg`.
const COVER: &str = "cover://";

/// The sizes a cover is read at, in pixels: a row's thumbnail, a card, a
/// page's header, Now Playing. A handful of sizes keeps the same cover shown
/// twice over, or a window resized, from reading it again for every width.
/// Drawn larger than the last of them, it is read as it is.
const SIZES: [u32; 5] = [64, 128, 256, 512, 1024];

/// How to address `url` for a `size` point square: a file of this device by
/// the pixels it is drawn at, anything else as it is.
pub fn sized(ctx: &egui::Context, url: &str, size: egui::Vec2) -> String {
    let Some(path) = url.strip_prefix("file://") else {
        return url.to_owned();
    };
    install(ctx);
    let pixels = size.max_elem() * ctx.pixels_per_point();
    let at = SIZES
        .iter()
        .copied()
        .find(|size| *size as f32 >= pixels)
        .unwrap_or(u32::MAX);
    format!("{COVER}{at}/{path}")
}

/// The loader is put in front of egui's own the first time a cover file is
/// drawn: the app's own start-up is another's to change.
fn install(ctx: &egui::Context) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| ctx.add_image_loader(Arc::new(Thumbs::default())));
}

type Kept = Poll<Result<Arc<ColorImage>, String>>;

#[derive(Default)]
struct Thumbs {
    kept: Arc<Mutex<HashMap<String, Kept>>>,
}

impl Thumbs {
    const ID: &'static str = egui::generate_loader_id!(Thumbs);
}

impl ImageLoader for Thumbs {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri: &str, _: SizeHint) -> ImageLoadResult {
        let Some((at, path)) = uri
            .strip_prefix(COVER)
            .and_then(|rest| rest.split_once('/'))
            .and_then(|(at, path)| at.parse::<u32>().ok().map(|at| (at, path)))
        else {
            return Err(LoadError::NotSupported);
        };
        let mut kept = self.kept.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = kept.get(uri) {
            return match entry {
                Poll::Ready(Ok(image)) => Ok(ImagePoll::Ready {
                    image: image.clone(),
                }),
                Poll::Ready(Err(e)) => Err(LoadError::Loading(e.clone())),
                Poll::Pending => Ok(ImagePoll::Pending { size: None }),
            };
        }
        kept.insert(uri.to_owned(), Poll::Pending);
        drop(kept);
        // Read and shrunk off the painting thread, as egui's own loader
        // decodes off it; the file is a folder's, so there is no fetching it.
        let (path, uri) = (format!("/{path}"), uri.to_owned());
        let ctx = ctx.clone();
        let into = self.kept.clone();
        std::thread::Builder::new()
            .name("cover-file".into())
            .spawn(move || {
                let read = shrink(&path, at);
                // Let go of meanwhile: not kept, and nothing to repaint for.
                let wanted = match into
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get_mut(&uri)
                {
                    Some(entry) => {
                        *entry = Poll::Ready(read);
                        true
                    }
                    None => false,
                };
                if wanted {
                    ctx.request_repaint();
                }
            })
            .expect("spawn the cover reader");
        Ok(ImagePoll::Pending { size: None })
    }

    fn forget(&self, uri: &str) {
        self.kept.lock().unwrap_or_else(|e| e.into_inner()).remove(uri);
    }

    fn forget_all(&self) {
        self.kept.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    fn byte_size(&self) -> usize {
        self.kept
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .map(|kept| match kept {
                Poll::Ready(Ok(image)) => image.pixels.len() * size_of::<egui::Color32>(),
                _ => 0,
            })
            .sum()
    }

    fn has_pending(&self) -> bool {
        self.kept
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .any(|kept| kept.is_pending())
    }
}

/// The cover at `path`, no larger than `at` pixels either way, its shape as
/// it was.
fn shrink(path: &str, at: u32) -> Result<Arc<ColorImage>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let read = image::load_from_memory(&bytes).map_err(|e| format!("{path}: {e}"))?;
    let shown = if read.width() > at || read.height() > at {
        read.resize(at, at, image::imageops::FilterType::Triangle)
    } else {
        read
    };
    let rgba = shown.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Ok(Arc::new(ColorImage::from_rgba_unmultiplied(
        size,
        rgba.as_raw(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cover_file_is_read_no_larger_than_it_is_drawn() {
        let dir = std::env::temp_dir().join(format!("opium-covers-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cover.png");
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(600, 600, image::Rgb([200, 40, 90]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        std::fs::write(&path, png.into_inner()).unwrap();

        let ctx = egui::Context::default();
        let url = format!("file://{}", path.display());
        let uri = sized(&ctx, &url, egui::vec2(36.0, 36.0));
        assert_eq!(uri, format!("cover://64/{}", path.display()));
        // A service's address carries its size already.
        let remote = "https://art/120x120bb.jpg";
        assert_eq!(sized(&ctx, remote, egui::vec2(36.0, 36.0)), remote);

        let image = loop {
            match ctx.try_load_image(&uri, SizeHint::Scale(1.0.into())) {
                Ok(ImagePoll::Ready { image }) => break image,
                Ok(ImagePoll::Pending { .. }) => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => panic!("{e}"),
            }
        };
        assert_eq!(image.size, [64, 64]);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
