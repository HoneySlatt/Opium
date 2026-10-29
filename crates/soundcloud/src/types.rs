//! The slice of `api-v2`'s JSON this crate reads.

use serde::Deserialize;

/// A page of a list: `next_href` until the end (without the client id).
#[derive(Debug, Deserialize)]
pub struct Collection<T> {
    pub collection: Vec<T>,
    pub next_href: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub full_name: Option<String>,
    pub avatar_url: Option<String>,
    pub permalink_url: Option<String>,
}

/// A track; in a playlist past its first few, only `id` is there.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Track {
    pub id: u64,
    pub title: Option<String>,
    /// Milliseconds.
    pub duration: Option<u64>,
    pub artwork_url: Option<String>,
    pub user: Option<User>,
    pub publisher_metadata: Option<PublisherMetadata>,
    pub media: Option<Media>,
    pub track_authorization: Option<String>,
    /// The uploader lets it be downloaded, as they uploaded it.
    pub downloadable: bool,
    pub has_downloads_left: bool,
    /// `SNIP` when only a preview plays without Go+.
    pub policy: Option<String>,
    pub permalink_url: Option<String>,
    pub release_date: Option<String>,
    pub display_date: Option<String>,
    pub genre: Option<String>,
    pub station_urn: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PublisherMetadata {
    pub artist: Option<String>,
    pub album_title: Option<String>,
    pub explicit: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Media {
    pub transcodings: Vec<Transcoding>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Transcoding {
    pub url: String,
    /// `aac_256k`, `aac_160k`, `aac_96k`, `abr_sq`, `mp3_0_1`…
    pub preset: String,
    /// A preview of a Go+ track.
    pub snipped: bool,
    pub format: Format,
    /// `hq` (Go+), `sq`, `lq`.
    pub quality: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Format {
    /// `hls`, `progressive`, `ctr-encrypted-hls`, `cbc-encrypted-hls`.
    pub protocol: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Playlist {
    pub id: u64,
    pub title: String,
    pub description: Option<String>,
    pub artwork_url: Option<String>,
    pub user: Option<User>,
    pub track_count: Option<u32>,
    pub tracks: Vec<Track>,
    pub is_album: bool,
    pub permalink_url: Option<String>,
    pub release_date: Option<String>,
    pub published_at: Option<String>,
}

/// One of the user's liked tracks.
#[derive(Debug, Deserialize)]
pub struct TrackLike {
    pub track: Option<Track>,
}

/// One of the user's likes, of a track or a playlist.
#[derive(Debug, Deserialize)]
pub struct Like {
    pub playlist: Option<Playlist>,
}

/// Where a transcoding plays from.
#[derive(Debug, Deserialize)]
pub struct StreamUrl {
    pub url: String,
}

/// Where an original download is.
#[derive(Debug, Deserialize)]
pub struct Download {
    #[serde(rename = "redirectUri")]
    pub redirect_uri: String,
}

/// Tokens OAuth issues.
#[derive(Debug, Deserialize)]
pub struct Grant {
    pub access_token: String,
    pub refresh_token: Option<String>,
}
