//! Cover art kept on disk in `~/.cache/music/covers/`, so the covers seen once
//! show at once afterwards. An egui loader in front of the HTTP one: it reads
//! the kept file, else downloads it as egui would and keeps it.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::task::Poll;

use egui::load::{Bytes, BytesLoadResult, BytesLoader, BytesPoll, LoadError};
use egui::mutex::Mutex;

/// Past this, the oldest covers go at the next start.
const LIMIT: u64 = 500 * 1024 * 1024;
/// …until the rest fits in this.
const TRIMMED: u64 = 400 * 1024 * 1024;

fn dir() -> Option<PathBuf> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(cache.join("music").join("covers"))
}

/// The file for `url`, named by a hash of it (FNV-1a, which never changes).
fn file(url: &str) -> Option<PathBuf> {
    let hash = url.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    Some(dir()?.join(format!("{hash:016x}")))
}

/// The cover kept for `url`, if it was.
pub fn read(url: &str) -> Option<Vec<u8>> {
    std::fs::read(file(url)?).ok()
}

/// Keep the cover downloaded from `url`.
pub fn write(url: &str, bytes: &[u8]) {
    let (Some(dir), Some(path)) = (dir(), file(url)) else {
        return;
    };
    let partial = path.with_extension("part");
    let _ = std::fs::create_dir_all(dir)
        .and_then(|()| std::fs::write(&partial, bytes))
        .and_then(|()| std::fs::rename(&partial, &path));
}

/// Drop the oldest covers if they take too much room; run in the background.
pub fn trim() {
    let Some(dir) = dir() else { return };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    let mut files: Vec<(std::time::SystemTime, u64, PathBuf)> = entries
        .filter_map(|e| {
            let e = e.ok()?;
            let meta = e.metadata().ok()?;
            Some((meta.modified().ok()?, meta.len(), e.path()))
        })
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    if total <= LIMIT {
        return;
    }
    files.sort();
    for (_, len, path) in files {
        if total <= TRIMMED {
            break;
        }
        if std::fs::remove_file(path).is_ok() {
            total -= len;
        }
    }
}

/// What the bytes are, for the image decoder: the kept files have no name to
/// tell it.
fn sniff(bytes: &[u8]) -> Option<String> {
    let mime = match bytes {
        [0xFF, 0xD8, ..] => "image/jpeg",
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => "image/webp",
        [b'G', b'I', b'F', ..] => "image/gif",
        _ => return None,
    };
    Some(mime.to_owned())
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
        if !uri.starts_with("https://") {
            return Err(LoadError::NotSupported);
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
        std::thread::spawn(move || {
            if let Some(bytes) = read(&uri) {
                return finish(uri, Ok(bytes.into()));
            }
            ehttp::fetch(ehttp::Request::get(uri.clone()), move |response| {
                let result = match response {
                    Ok(r) if r.ok => {
                        write(&uri, &r.bytes);
                        Ok(r.bytes.into())
                    }
                    Ok(r) => Err(format!("{uri}: {} {}", r.status, r.status_text)),
                    Err(e) => Err(format!("{uri}: {e}")),
                };
                finish(uri, result);
            });
        });
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
