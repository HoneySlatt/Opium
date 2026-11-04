//! Cover art kept on disk in `~/.cache/music/covers/`, so the covers seen once
//! show at once afterwards: where each is kept, the signing of a server's on
//! the way, the queue they are fetched in, and the trimming of the folder.
//! The window's own loader, which draws them, is the app's.

use std::path::PathBuf;
use std::sync::Arc;

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

/// Signs a server's cover as it is fetched: kept and shown, its address
/// carries no sign-in (see `navidrome::Navidrome::sign`).
pub type Signer = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;
static SIGNER: std::sync::RwLock<Option<Signer>> = std::sync::RwLock::new(None);

/// What a server's cover address holds (see `navidrome::Navidrome::sign`).
pub const SERVER_COVER: &str = "/rest/getCoverArt.view?";

/// Covers are signed by `signer` from now on, or by none.
pub fn set_signer(signer: Option<Signer>) {
    *SIGNER.write().unwrap_or_else(|e| e.into_inner()) = signer;
}

/// Whether covers are signed now: a server's asked for before it has
/// connected waits for it.
pub fn has_signer() -> bool {
    SIGNER.read().unwrap_or_else(|e| e.into_inner()).is_some()
}

/// Where to fetch the cover at `url` from: signed, if it is a server's.
pub fn signed(url: &str) -> String {
    SIGNER
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|sign| sign(url))
        .unwrap_or_else(|| url.to_owned())
}

/// Covers fetched at once: scrolling fast through a library asks for
/// hundreds, which would each get threads of their own.
const FETCHERS: usize = 6;

pub type Job = Box<dyn FnOnce() + Send>;

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

/// Fetch a cover in turn, the last asked first.
pub fn fetch_later(job: Job) {
    let (jobs, ready) = &*JOBS;
    jobs.lock().unwrap_or_else(|e| e.into_inner()).push(job);
    ready.notify_one();
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
pub fn sniff(bytes: &[u8]) -> Option<String> {
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
