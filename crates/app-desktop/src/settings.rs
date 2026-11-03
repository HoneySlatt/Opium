//! What the app remembers between runs, in `~/.config/music/settings.toml`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where the colours come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    /// The theme's own (Stock's: Carbonfox's black with Apple Music's pink).
    Stock,
    /// Whatever Amane's theme switch last wrote to `~/.config/music/theme.toml`.
    Amane,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The source each page was last narrowed to: Albums and Recently
    /// Added, Songs, Search.
    pub grid_source: Option<music_core::model::ProviderKind>,
    pub songs_source: Option<music_core::model::ProviderKind>,
    pub search_source: Option<music_core::model::ProviderKind>,
    /// The service Home, New and Radio come from, when several are
    /// connected; unset: Apple Music, else the first connected.
    pub browse_source: Option<music_core::model::ProviderKind>,
    /// Unset: Amane when its theme file exists, Stock otherwise.
    pub appearance: Option<Appearance>,
    /// The theme's id: a built-in one, or a file's stem in
    /// `~/.config/music/themes/`. Unset or gone, Stock.
    pub theme: Option<String>,
    /// The interface scale, 1.0 for 100 % (which draws at [`SCALE_UNIT`]);
    /// Ctrl +, Ctrl − and Ctrl 0 change it. `None` is 100 %.
    pub ui_scale: Option<f32>,
    /// A song favourited joins the library if it isn't there (on unless
    /// turned off).
    pub favourites_to_library: Option<bool>,
    /// A song added to a playlist joins the library if it isn't there (on
    /// unless turned off), as Apple Music's own option does.
    pub playlist_songs_to_library: Option<bool>,
    /// A song taken out of the library leaves the user's playlists too:
    /// Music's and SoundCloud's, Apple's doing so itself. Off unless set.
    pub removals_leave_playlists: Option<bool>,
    /// The playlists songs were last added to, most recent first, as kept
    /// here before the devices shared them: taken in once, then emptied.
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
    /// The lyrics sites turned on: LRCLIB alone unless set, as the others
    /// are told, from China, of every song looked up.
    pub lyrics_on: Option<Vec<lyrics::Source>>,
    /// The sites' lyrics before the service's own (off unless set).
    pub lyrics_sites_first: Option<bool>,
    /// A song mixed in Dolby Atmos plays as Apple's Spatial Audio, its
    /// binaural rendering (on unless turned off).
    pub spatial_audio: Option<bool>,
    /// The services turned off: still signed in, but not used.
    pub services_off: Vec<music_core::model::ProviderKind>,
    /// The sidebar was folded when the app last closed.
    pub sidebar_collapsed: Option<bool>,
    /// The column beside the page, as last left: closed, or what it showed.
    pub side_closed: Option<bool>,
    pub side: Option<crate::app::Side>,
    /// The relay the devices meet through away from home, as
    /// `https://relay.example.org`; n0's unless set.
    pub relay: Option<String>,
    /// Recently Added merges the services' by when each was added (off
    /// unless set); otherwise Apple Music's, SoundCloud's, then Navidrome's.
    pub recent_by_date: Option<bool>,
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

/// Where the app keeps what it keeps: the settings, the paired devices.
pub fn dir() -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_default();
    config.join("music")
}

fn path() -> PathBuf {
    dir().join("settings.toml")
}

impl Settings {
    /// Missing settings are the defaults. Of a file that doesn't read
    /// whole (a value this build doesn't know, a typo), every setting that
    /// reads is kept, and the file as it was is kept beside, as
    /// `settings.toml.bad`, before anything is written over it.
    pub fn load() -> Self {
        let Ok(text) = std::fs::read_to_string(path()) else {
            return Self::default();
        };
        if let Ok(settings) = toml::from_str(&text) {
            return settings;
        }
        let _ = std::fs::write(path().with_extension("toml.bad"), &text);
        text.parse::<toml::Table>()
            .map(Self::what_reads)
            .unwrap_or_default()
    }

    /// The settings of `table` that read, each on its own.
    fn what_reads(table: toml::Table) -> Self {
        let readable: toml::Table = table
            .into_iter()
            .filter(|(key, value)| {
                let mut alone = toml::Table::new();
                alone.insert(key.clone(), value.clone());
                Self::deserialize(alone).is_ok()
            })
            .collect();
        Self::deserialize(readable).unwrap_or_default()
    }

    pub fn save(&self) {
        // Written aside, then moved over: a crash midway leaves the last.
        if let Ok(s) = toml::to_string(self) {
            let _ = crate::backend::write_private(&path(), s.as_bytes());
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
    fn a_setting_that_doesnt_read_loses_only_itself() {
        let text = "volume = 0.3\nrepeat = \"sometimes\"\nshuffle = true\n";
        assert!(toml::from_str::<Settings>(text).is_err());
        let kept = Settings::what_reads(text.parse().unwrap());
        assert_eq!(kept.volume, Some(0.3));
        assert_eq!(kept.shuffle, Some(true));
        assert_eq!(kept.repeat, None);
    }

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
