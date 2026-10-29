use std::fmt;
use std::time::Duration;

/// A music service the app can pull from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    AppleMusic,
    SoundCloud,
    Navidrome,
}

/// An item within one provider. The id is opaque: only its provider knows what it means.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemId {
    pub provider: ProviderKind,
    pub id: String,
}

impl ItemId {
    pub fn new(provider: ProviderKind, id: impl Into<String>) -> Self {
        Self {
            provider,
            id: id.into(),
        }
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}:{}", self.provider, self.id)
    }
}

/// Cover art at any size: a URL with `{size}` where the edge length in pixels goes.
///
/// Each service sizes art differently (Apple's `{w}x{h}`, SoundCloud's `-t500x500`,
/// Subsonic's `size=`); providers translate theirs into this one placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artwork {
    pub url_template: String,
}

impl Artwork {
    pub fn url(&self, size: u32) -> String {
        self.url_template.replace("{size}", &size.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Codec {
    Aac,
    Alac,
    Flac,
    Mp3,
    Opus,
    Vorbis,
    Wav,
    Other(String),
}

impl Codec {
    pub fn is_lossless(&self) -> bool {
        matches!(self, Codec::Alac | Codec::Flac | Codec::Wav)
    }
}

/// What a source actually delivers, as far as the provider knows before decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioQuality {
    pub codec: Codec,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub bitrate_kbps: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub id: ItemId,
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub album_id: Option<ItemId>,
    pub duration: Option<Duration>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub artwork: Option<Artwork>,
    pub explicit: bool,
    pub quality: Option<AudioQuality>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Album {
    pub id: ItemId,
    pub title: String,
    pub artists: Vec<String>,
    /// The main artist, when the service says which.
    pub artist_id: Option<ItemId>,
    pub year: Option<u16>,
    pub artwork: Option<Artwork>,
    pub track_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Artist {
    pub id: ItemId,
    pub name: String,
    pub artwork: Option<Artwork>,
}

/// Something added to the library: an album or a playlist.
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryItem {
    Album(Album),
    Playlist(Playlist),
}

/// An artist's page: their best-known songs and their releases.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtistDetail {
    pub artist: Artist,
    pub top_songs: Vec<Track>,
    pub albums: Vec<Album>,
    pub singles: Vec<Album>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Playlist {
    pub id: ItemId,
    pub name: String,
    pub description: Option<String>,
    pub artwork: Option<Artwork>,
    pub track_count: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchResults {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub playlists: Vec<Playlist>,
}

/// Where the next page starts. Opaque because services disagree: Apple and Subsonic
/// page by offset, SoundCloud hands back a `next_href`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor(pub String);

#[derive(Debug, Clone, PartialEq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next: Option<Cursor>,
}

/// One line of lyrics. `text` is empty for an instrumental gap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricLine {
    pub start: std::time::Duration,
    pub end: Option<std::time::Duration>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    /// Whether the lines carry times. Unsynced lyrics are shown as plain text.
    pub synced: bool,
}

impl Lyrics {
    /// The line playing at `position`: the last one started.
    pub fn current(&self, position: std::time::Duration) -> Option<usize> {
        if !self.synced {
            return None;
        }
        self.lines.iter().rposition(|l| l.start <= position)
    }
}
