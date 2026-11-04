//! The covers as the window draws them: egui's loader in front of the HTTP
//! one, reading the kept file else fetching it and keeping it, and the
//! covers let go of once unseen. Where they are kept is the library's.

use std::collections::HashMap;
use std::sync::Arc;
use std::task::Poll;

use egui::load::{Bytes, BytesLoadResult, BytesLoader, BytesPoll, LoadError};
use egui::mutex::Mutex;

pub use desktop::covers::*;

/// When each cover was last drawn, to let go of those not seen for a while.
static SEEN: std::sync::LazyLock<std::sync::Mutex<HashMap<String, f64>>> =
    std::sync::LazyLock::new(Default::default);
/// How long a cover may go unseen before its image and texture are dropped.
const UNSEEN_FOR: f64 = 60.0;
/// The most covers kept in memory: a few screens of them.
const KEPT_AT_MOST: usize = 400;

/// The cover at `url` is drawn now.
pub fn seen(ctx: &egui::Context, url: &str) {
    let now = ctx.input(|i| i.time);
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    match seen.get_mut(url) {
        Some(at) => *at = now,
        None => {
            seen.insert(url.to_owned(), now);
        }
    }
}

/// Drop the covers not drawn for a while from memory (the disk keeps
/// them), so that scrolling through a large library doesn't fill it.
pub fn forget_unseen(ctx: &egui::Context) {
    let now = ctx.input(|i| i.time);
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    // Past the most kept, the longest unseen go too, however recent.
    let oldest_kept = if seen.len() > KEPT_AT_MOST {
        let mut times: Vec<f64> = seen.values().copied().collect();
        times.sort_by(|a, b| b.total_cmp(a));
        times[KEPT_AT_MOST - 1]
    } else {
        f64::NEG_INFINITY
    };
    seen.retain(|url, at| {
        let keep = now - *at < UNSEEN_FOR && *at >= oldest_kept;
        if !keep {
            ctx.forget_image(url);
        }
        keep
    });
}

type Entry = Poll<Result<Arc<[u8]>, String>>;

#[derive(Default)]
pub struct CoverLoader {
    loaded: Arc<Mutex<HashMap<String, Entry>>>,
}

impl CoverLoader {
    pub const ID: &'static str = egui::generate_loader_id!(CoverLoader);
}

impl BytesLoader for CoverLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri: &str) -> BytesLoadResult {
        // A cover kept here (a playlist's chosen one): read as it is.
        if let Some(path) = uri.strip_prefix("file://") {
            return match std::fs::read(path) {
                Ok(bytes) => Ok(BytesPoll::Ready {
                    size: None,
                    mime: sniff(&bytes),
                    bytes: Bytes::Shared(bytes.into()),
                }),
                Err(e) => Err(LoadError::Loading(format!("{uri}: {e}"))),
            };
        }
        // A server's covers (often plain http at home) are signed here as
        // they are fetched: egui's own loader would ask without the sign-in.
        let server = uri.contains(SERVER_COVER);
        if !uri.starts_with("https://") && !(server && uri.starts_with("http://")) {
            return Err(LoadError::NotSupported);
        }
        // The server not connected yet (its covers shown from the library
        // kept): asked again once it is, not failed for good meanwhile.
        if server && !has_signer() {
            return Ok(BytesPoll::Pending { size: None });
        }
        let mut loaded = self.loaded.lock();
        if let Some(entry) = loaded.get(uri) {
            return match entry {
                Poll::Ready(Ok(bytes)) => Ok(BytesPoll::Ready {
                    size: None,
                    mime: sniff(bytes),
                    bytes: Bytes::Shared(bytes.clone()),
                }),
                Poll::Ready(Err(e)) => Err(LoadError::Loading(e.clone())),
                Poll::Pending => Ok(BytesPoll::Pending { size: None }),
            };
        }
        let uri = uri.to_owned();
        loaded.insert(uri.clone(), Poll::Pending);
        drop(loaded);

        let ctx = ctx.clone();
        let loaded = self.loaded.clone();
        // Done with the lock off: egui's context may not be locked under it.
        let finish = move |uri: String, result: Result<Arc<[u8]>, String>| {
            let still_wanted = match loaded.lock().get_mut(&uri) {
                Some(entry) => {
                    *entry = Poll::Ready(result);
                    true
                }
                None => false,
            };
            if still_wanted {
                ctx.request_repaint();
            }
        };
        let wanted = self.loaded.clone();
        fetch_later(Box::new(move || {
            // Scrolled past and let go of before its turn: not fetched.
            if !wanted.lock().contains_key(&uri) {
                return;
            }
            if let Some(bytes) = read(&uri) {
                return finish(uri, Ok(bytes.into()));
            }
            let from = signed(&uri);
            let result = match ehttp::fetch_blocking(&ehttp::Request::get(&from)) {
                Ok(r) if r.ok => {
                    write(&uri, &r.bytes);
                    Ok(r.bytes.into())
                }
                Ok(r) => Err(format!("{uri}: {} {}", r.status, r.status_text)),
                // Said without the sign-in it was fetched with.
                Err(e) => Err(format!("{uri}: {}", e.replace(&from, &uri))),
            };
            finish(uri, result);
        }));
        Ok(BytesPoll::Pending { size: None })
    }

    fn forget(&self, uri: &str) {
        self.loaded.lock().remove(uri);
    }

    fn forget_all(&self) {
        self.loaded.lock().clear();
    }

    fn byte_size(&self) -> usize {
        self.loaded
            .lock()
            .values()
            .map(|entry| match entry {
                Poll::Ready(Ok(bytes)) => bytes.len(),
                _ => 0,
            })
            .sum()
    }

    fn has_pending(&self) -> bool {
        self.loaded.lock().values().any(|e| e.is_pending())
    }
}
