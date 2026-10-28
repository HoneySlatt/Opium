//! The slice of Apple's `amp-api` JSON this crate reads.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Response<T> {
    pub data: Vec<T>,
    pub next: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Resource<A> {
    pub id: String,
    pub attributes: Option<A>,
}

#[derive(Debug, Deserialize)]
pub struct Artwork {
    pub url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayParams {
    /// Present on library items that are also in the catalog: the id playback needs.
    pub catalog_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SongAttributes {
    pub name: String,
    pub artist_name: String,
    pub album_name: Option<String>,
    pub duration_in_millis: Option<u64>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub artwork: Option<Artwork>,
    pub content_rating: Option<String>,
    pub play_params: Option<PlayParams>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AlbumAttributes {
    pub name: String,
    pub artist_name: String,
    pub artwork: Option<Artwork>,
    pub release_date: Option<String>,
    pub track_count: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Description {
    pub standard: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct PlaylistAttributes {
    pub name: String,
    pub description: Option<Description>,
    pub artwork: Option<Artwork>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ArtistAttributes {
    pub name: String,
    pub artwork: Option<Artwork>,
}

pub type Song = Resource<SongAttributes>;
pub type Album = Resource<AlbumAttributes>;
pub type Playlist = Resource<PlaylistAttributes>;
pub type Artist = Resource<ArtistAttributes>;

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub results: SearchResults,
}

#[derive(Debug, Deserialize)]
pub struct SearchResults {
    pub songs: Option<Response<Song>>,
    pub albums: Option<Response<Album>>,
    pub artists: Option<Response<Artist>>,
    pub playlists: Option<Response<Playlist>>,
}
