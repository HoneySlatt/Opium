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
    cache_dir("covers")
}

fn cache_dir(name: &str) -> Option<PathBuf> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(cache.join("music").join(name))
}

/// A hash of `url` (FNV-1a, which never changes), to name its file.
fn hash(url: &str) -> u64 {
    url.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Where the cover from `url` is kept.
pub fn file(url: &str) -> Option<PathBuf> {
    Some(dir()?.join(format!("{:016x}", hash(url))))
}

/// Where the animated cover at `url` is kept at `size`, in
/// `~/.cache/music/motion/`.
pub fn motion_file(url: &str, size: u32) -> Option<PathBuf> {
    Some(cache_dir("motion")?.join(format!("{:016x}-{size}.mp4", hash(url))))
}

/// Keep an animated cover; returns where.
pub fn write_motion(url: &str, size: u32, bytes: &[u8]) -> Option<PathBuf> {
    let path = motion_file(url, size)?;
    let partial = path.with_extension("part");
    std::fs::create_dir_all(path.parent()?)
        .and_then(|()| std::fs::write(&partial, bytes))
        .and_then(|()| std::fs::rename(&partial, &path))
        .ok()?;
    Some(path)
}

/// Covers fetched at once: scrolling fast through a library asks for
/// hundreds, which would each get threads of their own.
const FETCHERS: usize = 6;

type Job = Box<dyn FnOnce() + Send>;

/// The covers waiting their turn, the last asked first: those on screen now.
static JOBS: std::sync::LazyLock<(std::sync::Mutex<Vec<Job>>, std::sync::Condvar)> =
    std::sync::LazyLock::new(|| {
        for _ in 0..FETCHERS {
            std::thread::spawn(|| {
                loop {
                    let job = {
                        let (jobs, ready) = &*JOBS;
                        let mut jobs = jobs.lock().unwrap_or_else(|e| e.into_inner());
                        loop {
                            if let Some(job) = jobs.pop() {
                                break job;
                            }
                            jobs = ready.wait(jobs).unwrap_or_else(|e| e.into_inner());
                        }
                    };
                    job();
                }
            });
        }
        Default::default()
    });

fn fetch_later(job: Job) {
    let (jobs, ready) = &*JOBS;
    jobs.lock().unwrap_or_else(|e| e.into_inner()).push(job);
    ready.notify_one();
}

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

/// Drop the oldest covers, and moving covers, if they take too much room;
/// run in the background.
pub fn trim() {
    for name in ["covers", "motion"] {
        if let Some(dir) = cache_dir(name) {
            trim_dir(&dir);
        }
    }
}

fn trim_dir(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
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
        let wanted = self.loaded.clone();
        fetch_later(Box::new(move || {
            // Scrolled past and let go of before its turn: not fetched.
            if !wanted.lock().contains_key(&uri) {
                return;
            }
            if let Some(bytes) = read(&uri) {
                return finish(uri, Ok(bytes.into()));
            }
            let result = match ehttp::fetch_blocking(&ehttp::Request::get(uri.clone())) {
                Ok(r) if r.ok => {
                    write(&uri, &r.bytes);
                    Ok(r.bytes.into())
                }
                Ok(r) => Err(format!("{uri}: {} {}", r.status, r.status_text)),
                Err(e) => Err(format!("{uri}: {e}")),
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
