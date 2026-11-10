//! The library as it was last read, kept in `~/.cache/opium/library/`: shown at
//! once when the app starts or a playlist opens, while Apple is asked again.

use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;

fn dir() -> Option<PathBuf> {
    Some(crate::folders::cache()?.join("library"))
}

/// `playlist p.AbC` → `playlist-p.AbC.json`: ids are letters, digits and dots.
fn file(name: &str) -> Option<PathBuf> {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    Some(dir()?.join(format!("{safe}.json")))
}

/// What was kept under `name`, if anything readable was.
pub fn read<T: DeserializeOwned>(name: &str) -> Option<T> {
    let text = std::fs::read_to_string(file(name)?).ok()?;
    serde_json::from_str(&navidrome::scrub_kept(&text)).ok()
}

/// Keep `value` under `name`, replacing the old one whole: written aside, then
/// moved over it.
pub fn write<T: Serialize>(name: &str, value: &T) {
    let (Some(dir), Some(path)) = (dir(), file(name)) else {
        return;
    };
    let Ok(bytes) = serde_json::to_vec(value) else {
        return;
    };
    // A name of its own per write: two writers of one entry (an album
    // opened twice over) would otherwise each put part of theirs in the
    // same file, and what was renamed into place wouldn't read.
    static WRITES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let partial = path.with_extension(format!(
        "json.part.{}",
        WRITES.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(&partial, bytes))
        .and_then(|()| std::fs::rename(&partial, &path));
}

/// Forget everything kept: another account may sign in.
pub fn clear() {
    if let Some(dir) = dir() {
        let _ = std::fs::remove_dir_all(dir);
    }
}
