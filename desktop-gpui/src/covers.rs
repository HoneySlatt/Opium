//! The covers as the window draws them, as the egui window's `covers.rs` and
//! `ui/thumbs.rs` load them: the kept file, else fetched (a server's signed
//! on the way) in the fetchers' turn and kept, through GPUI's asset cache;
//! a cover file of this device read at the size it is drawn; and the
//! covers let go of once unseen. Where they are kept is the library's.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{App, Asset, ImageCacheError, RenderImage, SharedString, Window};

use desktop::covers::{SERVER_COVER, fetch_later, has_signer, read, signed, write};

/// How long a cover may go unseen before its image is dropped.
const UNSEEN_FOR: Duration = Duration::from_secs(60);
/// The most covers kept in memory: a few screens of them.
const KEPT_AT_MOST: usize = 400;
/// The sizes a cover file is read at, in pixels, as `ui/thumbs.rs`'s.
const SIZES: [u32; 5] = [64, 128, 256, 512, 1024];

/// A cover as it is drawn: its address, and for a file of this device the
/// pixels it is read at; a service's comes at the size its address asks.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Cover {
    url: SharedString,
    at: Option<u32>,
}

impl Cover {
    /// The cover at `url` drawn `pixels` wide, if it is one this window can
    /// load: a file, or a service's over https (a server's over http too).
    pub fn drawn(url: &str, pixels: f32) -> Option<Cover> {
        if url.starts_with("file://") {
            let at = SIZES
                .iter()
                .copied()
                .find(|size| *size as f32 >= pixels)
                .unwrap_or(u32::MAX);
            return Some(Cover {
                url: url.to_owned().into(),
                at: Some(at),
            });
        }
        let server = url.contains(SERVER_COVER);
        if !url.starts_with("https://") && !(server && url.starts_with("http://")) {
            return None;
        }
        // The server not connected yet (its covers shown from the library
        // kept): asked once it is, the fill meanwhile.
        if server && !has_signer() {
            return None;
        }
        Some(Cover {
            url: url.to_owned().into(),
            at: None,
        })
    }
}

pub enum CoverLoader {}

impl Asset for CoverLoader {
    type Source = Cover;
    type Output = Result<Arc<RenderImage>, ImageCacheError>;

    #[allow(clippy::manual_async_fn)] // GPUI's trait asks for a future that is `Send`.
    fn load(cover: Cover, _: &mut App) -> impl Future<Output = Self::Output> + Send + 'static {
        async move {
            let failed = |e: String| ImageCacheError::Io(Arc::new(std::io::Error::other(e)));
            let url = cover.url.to_string();
            if let Some(at) = cover.at {
                // A folder's own file: no fetching it, read off this thread.
                let path = url.strip_prefix("file://").unwrap_or(&url).to_owned();
                let bytes = std::fs::read(&path).map_err(|e| failed(format!("{path}: {e}")))?;
                return decode(&bytes, Some(at)).map_err(failed);
            }
            let (done, result) = async_channel::bounded(1);
            fetch_later(Box::new(move || {
                // Let go of before its turn (scrolled past): not fetched.
                if done.is_closed() {
                    return;
                }
                let bytes = match read(&url) {
                    Some(bytes) => Ok(bytes),
                    None => {
                        let from = signed(&url);
                        match ehttp::fetch_blocking(&ehttp::Request::get(&from)) {
                            Ok(r) if r.ok => {
                                write(&url, &r.bytes);
                                Ok(r.bytes)
                            }
                            Ok(r) => Err(format!("{url}: {} {}", r.status, r.status_text)),
                            // Said without the sign-in it was fetched with.
                            Err(e) => Err(format!("{url}: {}", e.replace(&from, &url))),
                        }
                    }
                };
                let _ = done.try_send(bytes.and_then(|b| decode(&b, None)));
            }));
            result
                .recv()
                .await
                .map_err(|e| failed(e.to_string()))?
                .map_err(failed)
        }
    }
}

/// The image, no larger than `at` pixels either way when given, in GPUI's
/// BGRA.
fn decode(bytes: &[u8], at: Option<u32>) -> Result<Arc<RenderImage>, String> {
    let read = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let shown = match at {
        Some(at) if read.width() > at || read.height() > at => {
            read.resize(at, at, image::imageops::FilterType::Triangle)
        }
        _ => read,
    };
    let mut rgba = shown.to_rgba8();
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    Ok(Arc::new(RenderImage::new(vec![image::Frame::new(rgba)])))
}

thread_local! {
    /// When each cover was last drawn.
    static SEEN: RefCell<HashMap<Cover, Instant>> = RefCell::default();
    static FORGOTTEN: RefCell<Option<Instant>> = const { RefCell::new(None) };
}

/// The cover is drawn now.
pub fn seen(cover: &Cover) {
    SEEN.with_borrow_mut(|seen| {
        seen.insert(cover.clone(), Instant::now());
    });
}

/// Every few seconds, drop the covers not drawn for a while from memory and
/// from the GPU (the disk keeps them), so that scrolling through a large
/// library doesn't fill it.
pub fn forget_unseen(window: &mut Window, cx: &mut App) {
    let now = Instant::now();
    let due = FORGOTTEN.with_borrow_mut(|at| {
        let due = at.is_none_or(|at| now - at > Duration::from_secs(10));
        if due {
            *at = Some(now);
        }
        due
    });
    if !due {
        return;
    }
    let gone: Vec<Cover> = SEEN.with_borrow_mut(|seen| {
        // Past the most kept, the longest unseen go too, however recent.
        let oldest_kept = if seen.len() > KEPT_AT_MOST {
            let mut times: Vec<Instant> = seen.values().copied().collect();
            times.sort_by(|a, b| b.cmp(a));
            Some(times[KEPT_AT_MOST - 1])
        } else {
            None
        };
        let gone: Vec<Cover> = seen
            .iter()
            .filter(|(_, at)| now - **at >= UNSEEN_FOR || oldest_kept.is_some_and(|o| **at < o))
            .map(|(cover, _)| cover.clone())
            .collect();
        for cover in &gone {
            seen.remove(cover);
        }
        gone
    });
    for cover in gone {
        if let Some(Ok(image)) = cx.fetch_asset::<CoverLoader>(&cover) {
            let _ = window.drop_image(image);
        }
        cx.remove_asset::<CoverLoader>(&cover);
    }
}
