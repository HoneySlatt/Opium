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
    /// `songs`, `library-albums`…; read where one list mixes kinds.
    #[serde(rename = "type", default)]
    pub kind: String,
    pub attributes: Option<A>,
    /// Present when the request asked for them with `include=`.
    pub relationships: Option<Relationships>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Relationships {
    pub albums: Option<Related>,
    pub artists: Option<Related>,
    /// A library item's catalog counterpart.
    pub catalog: Option<Related>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Related {
    pub data: Vec<RelatedRef>,
}

#[derive(Debug, Deserialize)]
pub struct RelatedRef {
    pub id: String,
    /// Present when the request asked for the related items with `include=`.
    pub attributes: Option<RelatedAttributes>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RelatedAttributes {
    pub artwork: Option<Artwork>,
}

#[derive(Debug, Deserialize)]
pub struct Artwork {
    pub url: String,
    /// `cb6900`.
    #[serde(rename = "bgColor")]
    pub bg_color: Option<String>,
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
    /// The song's page on music.apple.com, which names its album.
    pub url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AlbumAttributes {
    pub name: String,
    pub artist_name: String,
    pub artwork: Option<Artwork>,
    pub release_date: Option<String>,
    pub track_count: Option<u32>,
    pub genre_names: Vec<String>,
    pub editorial_notes: Option<EditorialNotes>,
    /// With `extend=inFavorites`.
    pub in_favorites: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct EditorialNotes {
    pub short: Option<String>,
    pub standard: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Description {
    pub standard: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistAttributes {
    pub name: String,
    pub description: Option<Description>,
    pub artwork: Option<Artwork>,
    pub curator_name: Option<String>,
    /// With `extend[library-playlists]=tags`: `favorited` marks the playlist
    /// Apple keeps of the user's favourite songs, and nothing else.
    pub tags: Option<Vec<String>>,
    /// With `extend=inFavorites`.
    pub in_favorites: Option<bool>,
    pub play_params: Option<PlaylistPlayParams>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PlaylistPlayParams {
    /// A library playlist's catalog id (`pl.u-…`); Apple's own playlists have none.
    pub global_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ArtistAttributes {
    pub name: String,
    pub artwork: Option<Artwork>,
    /// With `extend=inFavorites`.
    pub in_favorites: Option<bool>,
}

pub type Song = Resource<SongAttributes>;
pub type Album = Resource<AlbumAttributes>;
pub type Playlist = Resource<PlaylistAttributes>;
pub type Artist = Resource<ArtistAttributes>;

/// An artist fetched with `views=`: lists Apple's web player shows on the page.
#[derive(Debug, Deserialize)]
pub struct ArtistWithViews {
    pub id: String,
    pub attributes: Option<ArtistAttributes>,
    #[serde(default)]
    pub views: ArtistViews,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ArtistViews {
    #[serde(rename = "top-songs")]
    pub top_songs: Option<Response<Song>>,
    #[serde(rename = "full-albums")]
    pub full_albums: Option<Response<Album>>,
    #[serde(rename = "latest-release")]
    pub latest_release: Option<Response<Album>>,
    pub singles: Option<Response<Album>>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub results: SearchResults,
}

#[derive(Debug, Deserialize)]
pub struct SearchResults {
    #[serde(alias = "library-songs")]
    pub songs: Option<Response<Song>>,
    #[serde(alias = "library-albums")]
    pub albums: Option<Response<Album>>,
    #[serde(alias = "library-artists")]
    pub artists: Option<Response<Artist>>,
    #[serde(alias = "library-playlists")]
    pub playlists: Option<Response<Playlist>>,
}

#[derive(Debug, Deserialize)]
pub struct LyricsAttributes {
    pub ttml: Option<String>,
}

/// One shelf of `/v1/me/recommendations`.
#[derive(Debug, Deserialize)]
pub struct Recommendation {
    pub attributes: Option<RecommendationAttributes>,
    pub relationships: Option<RecommendationContents>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RecommendationAttributes {
    pub title: Option<DisplayTitle>,
    pub display: Option<Display>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DisplayTitle {
    pub string_for_display: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Display {
    pub kind: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RecommendationContents {
    pub contents: Option<Response<Resource<serde_json::Value>>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StationAttributes {
    pub name: String,
    pub artwork: Option<Artwork>,
    pub is_live: bool,
    pub media_kind: Option<String>,
    pub play_params: Option<StationPlayParams>,
    pub editorial_notes: Option<StationNotes>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct StationPlayParams {
    /// `stream` for what airs or aired, `tracks` for a station of songs.
    pub format: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct StationNotes {
    pub short: Option<String>,
    pub tagline: Option<String>,
}

pub type LyricsResponse = Response<Resource<LyricsAttributes>>;
