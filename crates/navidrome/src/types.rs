//! The slice of the Subsonic API's JSON (with OpenSubsonic's additions) this
//! crate reads.

use serde::Deserialize;

/// Every answer comes wrapped in `subsonic-response`.
#[derive(Debug, Deserialize)]
pub struct Envelope<T> {
    #[serde(rename = "subsonic-response")]
    pub response: Response<T>,
}

#[derive(Debug, Deserialize)]
pub struct Response<T> {
    pub status: String,
    pub error: Option<ApiError>,
    /// Not an `Option`: serde would make a body it can't read `None`, and a
    /// page with one odd song would pass for an empty one, ending the list.
    #[serde(flatten)]
    pub body: T,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub code: u32,
    #[serde(default)]
    pub message: String,
}

/// A name and id, as OpenSubsonic lists a song's or album's artists.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Named {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Song {
    pub id: String,
    pub title: String,
    pub album: Option<String>,
    pub album_id: Option<String>,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub artists: Vec<Named>,
    pub track: Option<u32>,
    pub disc_number: Option<u32>,
    /// Seconds.
    pub duration: Option<u64>,
    pub cover_art: Option<String>,
    pub suffix: Option<String>,
    pub content_type: Option<String>,
    /// kbps.
    pub bit_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub sampling_rate: Option<u32>,
    pub starred: Option<String>,
    /// `explicit`, `clean` or empty.
    pub explicit_status: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub artists: Vec<Named>,
    pub cover_art: Option<String>,
    pub song_count: Option<u32>,
    pub year: Option<u16>,
    pub genre: Option<String>,
    /// When it joined the server: RFC 3339.
    pub created: Option<String>,
    pub starred: Option<String>,
    pub release_date: Option<Date>,
    /// `Album`, `Single`, `EP`, `Compilation`…
    pub release_types: Vec<String>,
    pub explicit_status: Option<String>,
    /// With `getAlbum`.
    pub song: Vec<Song>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Date {
    pub year: Option<u16>,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub cover_art: Option<String>,
    pub artist_image_url: Option<String>,
    pub starred: Option<String>,
    /// With `getArtist`.
    pub album: Vec<Album>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub comment: Option<String>,
    pub owner: Option<String>,
    pub song_count: Option<u32>,
    pub cover_art: Option<String>,
    pub created: Option<String>,
    /// With `getPlaylist`.
    pub entry: Vec<Song>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Songs {
    pub song: Vec<Song>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SearchResult3 {
    pub song: Vec<Song>,
    pub album: Vec<Album>,
    pub artist: Vec<Artist>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Albums {
    pub album: Vec<Album>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Index {
    pub artist: Vec<Artist>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Indexes {
    pub index: Vec<Index>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Playlists {
    pub playlist: Vec<Playlist>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ArtistInfo {
    pub large_image_url: Option<String>,
    pub similar_artist: Vec<Artist>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Starred {
    pub song: Vec<Song>,
    pub album: Vec<Album>,
    pub artist: Vec<Artist>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LyricsList {
    pub structured_lyrics: Vec<StructuredLyrics>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct StructuredLyrics {
    pub synced: bool,
    pub line: Vec<Line>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Line {
    /// Milliseconds, when synced.
    pub start: Option<u64>,
    pub value: String,
}

/// The bodies, each under its own key.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Body {
    pub search_result3: Option<SearchResult3>,
    pub album_list2: Option<Albums>,
    pub album: Option<Album>,
    pub artists: Option<Indexes>,
    pub artist: Option<Artist>,
    pub artist_info2: Option<ArtistInfo>,
    pub top_songs: Option<Songs>,
    pub playlists: Option<Playlists>,
    pub playlist: Option<Playlist>,
    pub starred2: Option<Starred>,
    pub similar_songs2: Option<Songs>,
    pub random_songs: Option<Songs>,
    pub lyrics_list: Option<LyricsList>,
    pub song: Option<Song>,
}
