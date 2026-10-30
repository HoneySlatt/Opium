//! What the app remembers between runs, in `~/.config/music/settings.toml`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where the colours come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    /// Near-black with Pluto's violet.
    Stock,
    /// Whatever Amane's theme switch last wrote to `~/.config/music/theme.toml`.
    Amane,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Unset: Amane when its theme file exists, Stock otherwise.
    pub appearance: Option<Appearance>,
    /// The interface scale, 1.0 for 100 % (which draws at [`SCALE_UNIT`]);
    /// Ctrl +, Ctrl − and Ctrl 0 change it. `None` is 100 %.
    pub ui_scale: Option<f32>,
    /// A song favourited joins the library if it isn't there (on unless
    /// turned off).
    pub favourites_to_library: Option<bool>,
    /// A song added to a playlist joins the library if it isn't there (on
    /// unless turned off), as Apple Music's own option does.
    pub playlist_songs_to_library: Option<bool>,
    /// The playlists songs were last added to, most recent first.
    pub recent_playlists: Vec<String>,
    /// The volume, 0 to 1, as left last time; full unless set.
    pub volume: Option<f32>,
    /// Muted when last left.
    pub muted: Option<bool>,
    /// Shuffle was on when last left.
    pub shuffle: Option<bool>,
    /// Repeat as last left; off unless set.
    pub repeat: Option<Repeat>,
    /// Where Search looked last: Apple Music unless set.
    pub search_scope: Option<crate::app::SearchScope>,
    /// Covers move where Apple made them move (on unless turned off).
    pub animated_artwork: Option<bool>,
    /// How sharp the moving covers are; Automatic unless set.
    pub motion_quality: Option<MotionQuality>,
    /// How the Albums page is sorted; by name, A to Z, unless set.
    pub albums_sort: Option<AlbumSort>,
    pub albums_ascending: Option<bool>,
    /// A SoundCloud track plays from the uploader's original file when they
    /// let it be downloaded (on unless turned off).
    pub soundcloud_originals: Option<bool>,
    /// The lyrics sites, in the order they are asked; all of them unless set.
    pub lyrics_order: Vec<lyrics::Source>,
    /// The lyrics sites turned off.
    pub lyrics_off: Vec<lyrics::Source>,
    /// The sites' lyrics before the service's own (off unless set).
    pub lyrics_sites_first: Option<bool>,
    /// The services turned off: still signed in, but not used.
    pub services_off: Vec<music_core::model::ProviderKind>,
}

/// Repeat, as kept: the queue's own, by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
    Off,
    All,
    One,
}

impl From<Repeat> for music_core::Repeat {
    fn from(repeat: Repeat) -> Self {
        match repeat {
            Repeat::Off => Self::Off,
            Repeat::All => Self::All,
            Repeat::One => Self::One,
        }
    }
}

impl From<music_core::Repeat> for Repeat {
    fn from(repeat: music_core::Repeat) -> Self {
        match repeat {
            music_core::Repeat::Off => Self::Off,
            music_core::Repeat::All => Self::All,
            music_core::Repeat::One => Self::One,
        }
    }
}

/// How sharp moving covers are fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MotionQuality {
    /// As many pixels as the cover takes on screen.
    Automatic,
    Low,
    High,
}

/// What the Albums page sorts by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlbumSort {
    Added,
    Name,
    Artist,
    Released,
}

fn path() -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_default();
    config.join("music/settings.toml")
}

impl Settings {
    /// Missing or unreadable settings are the defaults.
    pub fn load() -> Self {
        std::fs::read_to_string(path())
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Written aside, then moved over: a crash midway leaves the last.
        if let Ok(s) = toml::to_string(self) {
            let partial = path.with_extension("toml.part");
            let _ = std::fs::write(&partial, s).and_then(|()| std::fs::rename(&partial, &path));
        }
    }
}

/// What 100 % draws at: the design's sizes, 10 % larger (the user's call).
pub const SCALE_UNIT: f32 = 1.1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::SearchScope;

    #[test]
    fn what_is_kept_reads_back_the_same() {
        let kept = Settings {
            search_scope: Some(SearchScope::Library),
            shuffle: Some(true),
            repeat: Some(Repeat::One),
            volume: Some(0.3),
            ..Settings::default()
        };
        let text = toml::to_string(&kept).unwrap();
        assert!(text.contains("search_scope = \"library\""), "{text}");
        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), kept);
    }
}
