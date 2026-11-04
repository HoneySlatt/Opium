//! The desktop app's own workings, apart from how it draws them: the
//! services and the player behind the window (`backend`), the settings, the
//! lists and covers kept on disk, the media controls, and the other
//! devices. Whatever draws the window wakes it through a [`Wake`], so that
//! none of this depends on the toolkit doing the drawing.

pub mod backend;
pub mod cache;
pub mod covers;
pub mod devices;
pub mod exit;
pub mod mpris;
pub mod palette;
pub mod settings;
pub mod soundcloud_browser;

use std::sync::Arc;

/// Asks the window for a new frame: something it shows has changed.
pub type Wake = Arc<dyn Fn() + Send + Sync>;

/// Where Search looks: everything on Apple Music, or the user's library.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum SearchScope {
    #[default]
    AppleMusic,
    Library,
}

/// What the column beside the page shows, in a theme that has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    NowPlaying,
    Queue,
    History,
    Lyrics,
}

/// The file a `file://` address names, its escapes undone.
pub fn file_path(uri: &str) -> Option<std::path::PathBuf> {
    let escaped = uri.strip_prefix("file://")?.as_bytes();
    let mut path = Vec::with_capacity(escaped.len());
    let mut i = 0;
    while i < escaped.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match (escaped[i], escaped.get(i + 1).copied().and_then(hex), escaped.get(i + 2).copied().and_then(hex)) {
            (b'%', Some(high), Some(low)) => {
                path.push((high * 16 + low) as u8);
                i += 3;
            }
            (b, _, _) => {
                path.push(b);
                i += 1;
            }
        }
    }
    use std::os::unix::ffi::OsStringExt;
    Some(std::ffi::OsString::from_vec(path).into())
}

