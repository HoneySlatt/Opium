//! The library as it was last read, kept in `~/.cache/opium/library/`: shown at
//! once when the app starts or a playlist opens, while Apple is asked again.

use std::path::PathBuf;

use opium_core::files::{Mode, write_whole};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// The library's folder: `library` in the cache folder.
pub(crate) fn dir() -> Option<PathBuf> {
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

/// Keep `value` under `name`, replacing the old one whole or not at all, as
/// a cache ([`write_whole`], [`Mode::Cache`]), even with two writers of one
/// entry at once (an album opened twice over). A write that fails keeps
/// nothing, and says nothing: it is a cache.
pub fn write<T: Serialize>(name: &str, value: &T) {
    let (Some(dir), Some(path)) = (dir(), file(name)) else {
        return;
    };
    let Ok(bytes) = serde_json::to_vec(value) else {
        return;
    };
    let _ = std::fs::create_dir_all(&dir).and_then(|()| write_whole(&path, &bytes, Mode::Cache));
}

/// Forget everything kept: another account may sign in.
pub fn clear() {
    if let Some(dir) = dir() {
        let _ = std::fs::remove_dir_all(dir);
    }
}
