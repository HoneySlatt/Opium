use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// A music service the app can pull from; the music files of this device;
/// or Music itself, for what it keeps of its own (its playlists of songs
/// from any service).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum ProviderKind {
    AppleMusic,
    SoundCloud,
    Navidrome,
    Music,
    /// The files in the folders chosen on this device.
    Local,
}

/// An item within one provider. The id is opaque: only its provider knows what it means.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Artwork {
    pub url_template: String,
    /// The colour behind the art, when the service gives it (Apple does).
    pub background: Option<u32>,
}

impl Artwork {
    pub fn url(&self, size: u32) -> String {
        self.url_template.replace("{size}", &size.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct AudioQuality {
    pub codec: Codec,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub bitrate_kbps: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
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
    /// It came from the user's library, so it is there.
    pub in_library: bool,
    /// Its id in the library, which removing it from there names.
    pub library_id: Option<ItemId>,
}

/// Where a song leads: its artist's page and its station.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct SongLinks {
    pub artist: Option<ItemId>,
    pub station: Option<ItemId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Album {
    pub id: ItemId,
    pub title: String,
    pub artists: Vec<String>,
    /// The main artist, when the service says which.
    pub artist_id: Option<ItemId>,
    /// The main artist's photo, when it came along.
    pub artist_artwork: Option<Artwork>,
    /// The release date, `YYYY-MM-DD`.
    pub released: Option<String>,
    pub genre: Option<String>,
    /// The service's words about the album.
    pub description: Option<String>,
    /// A library album's catalog counterpart, which favourites name.
    pub catalog_id: Option<ItemId>,
    /// Whether the user has it in their favourites, when the service said.
    pub in_favourites: Option<bool>,
    pub year: Option<u16>,
    pub artwork: Option<Artwork>,
    pub track_count: Option<u32>,
    /// Rated explicit.
    pub explicit: bool,
    /// When it joined the library, for a library album: RFC 3339, so the
    /// text sorts in time order.
    pub added: Option<String>,
    /// Its animated cover, square: where the service lists the video.
    pub motion: Option<String>,
    /// `℗ 2026 R.A.T MUSIC`.
    pub copyright: Option<String>,
    /// Mastered for the service, as Apple's Digital Masters are.
    #[serde(default)]
    pub mastered: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Artist {
    pub id: ItemId,
    pub name: String,
    pub artwork: Option<Artwork>,
    /// Whether the user has them in their favourites, when the service said.
    pub in_favourites: Option<bool>,
    /// The artist's id in the library, for one read from it: `id` is then
    /// the catalog's, when the library knows it.
    pub library_id: Option<ItemId>,
}

/// What can be a favourite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum FavouriteKind {
    Song,
    Album,
    Artist,
    Playlist,
}

/// An album or a playlist: what Recently Added and the home shelves list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Collection {
    Album(Album),
    Playlist(Playlist),
    Station(Station),
}

/// A radio station.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Station {
    pub id: ItemId,
    pub name: String,
    /// What it plays, in a line.
    pub description: Option<String>,
    pub artwork: Option<Artwork>,
    pub kind: StationKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum StationKind {
    /// On air now (Apple Music 1, Hits…).
    Live,
    /// A show that aired, streamed from the start.
    Episode,
    /// Songs one after the other, picked by the service (an artist's or the
    /// user's station).
    Tracks,
}

/// An artist's page: their best-known songs and their releases.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ArtistDetail {
    pub artist: Artist,
    pub top_songs: Vec<Track>,
    /// The newest release, when it is recent.
    pub latest: Option<Album>,
    pub albums: Vec<Album>,
    pub singles: Vec<Album>,
    pub compilations: Vec<Album>,
    /// Others' releases they are on.
    pub appears_on: Vec<Album>,
    pub similar: Vec<Artist>,
    /// Essential Albums, as Apple's editors pick them.
    pub essentials: Vec<Album>,
    pub videos: Vec<MusicVideo>,
    /// Playlists of their songs.
    pub playlists: Vec<Playlist>,
    /// A video across the top of their page: where the service lists it.
    pub motion: Option<String>,
    /// A wide picture for the top of their page, or the video's first frame.
    pub banner: Option<String>,
}

/// A music video, as an album's page lists them.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct MusicVideo {
    pub id: ItemId,
    pub title: String,
    pub artists: Vec<String>,
    /// A still of it, 16:9.
    pub thumbnail: Option<String>,
    pub explicit: bool,
    /// Its page on the service's website, where it plays.
    pub url: Option<String>,
}

/// What an album's page shows besides its songs: its other editions, and
/// the music videos of its songs.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct AlbumExtras {
    pub other_versions: Vec<Album>,
    pub videos: Vec<MusicVideo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Playlist {
    pub id: ItemId,
    pub name: String,
    pub description: Option<String>,
    /// Who made it: `Apple Music Hip-Hop`, a user.
    pub curator: Option<String>,
    /// The service's own list of the user's favourite songs.
    pub favourites: bool,
    pub artwork: Option<Artwork>,
    pub track_count: Option<u32>,
    /// A library playlist's catalog id, which favourites name.
    pub catalog_id: Option<ItemId>,
    /// Whether the user has it in their favourites, when the service said.
    pub in_favourites: Option<bool>,
    /// The user's own: songs can be added to it.
    pub editable: bool,
    /// When it joined the library (made, liked, added), as the service
    /// writes it, for Recently Added.
    #[serde(default)]
    pub added: Option<String>,
}

/// An editors' pick: what it is, in a word, and how they show it.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Pick {
    /// `NEW ALBUM`, `UPDATED PLAYLIST`.
    pub badge: String,
    /// A line about it, shown over the picture.
    pub blurb: Option<String>,
    /// A wide picture made for it, when there is one.
    pub wide: Option<Artwork>,
}

/// A row of the home, radio or new music page.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Shelf {
    pub title: String,
    /// Shown large: the playlists made for the user, the stations on air,
    /// the week's picks.
    pub hero: bool,
    pub items: Vec<Collection>,
    /// How the editors present each item, for a hero shelf; empty or as long
    /// as `items`.
    pub picks: Vec<Pick>,
    /// A shelf of songs instead of covers.
    pub songs: Vec<Track>,
}

/// A genre of the catalogue, whose charts it has.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Genre {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
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

#[cfg(feature = "ffi")]
uniffi::custom_newtype!(Cursor, String);

#[derive(Debug, Clone, PartialEq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next: Option<Cursor>,
}

/// One line of lyrics. `text` is empty for an instrumental gap.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct LyricLine {
    pub start: std::time::Duration,
    pub end: Option<std::time::Duration>,
    pub text: String,
    /// When the lyrics are timed word by word: the line's words (or
    /// syllables) in order, their text making up `text`; empty otherwise.
    pub words: Vec<LyricWord>,
}

/// A word or syllable sung from `start` to `end`. Its text keeps the space
/// after it, if any, so that the words of a line join into its text.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct LyricWord {
    pub start: std::time::Duration,
    pub end: std::time::Duration,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
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
