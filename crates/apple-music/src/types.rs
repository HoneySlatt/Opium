//! The slice of Apple's `amp-api` JSON this crate reads.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Response<T> {
    pub data: Vec<T>,
    pub next: Option<String>,
    /// A list's length, on its pages.
    #[serde(default)]
    pub meta: Option<Meta>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Meta {
    pub total: Option<usize>,
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
    /// A song's station.
    pub station: Option<Related>,
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
    /// A library album's catalog rating: the library's own has none.
    #[serde(rename = "contentRating")]
    pub content_rating: Option<String>,
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
    /// `explicit` or `clean`, when rated.
    pub content_rating: Option<String>,
    /// When a library album was added, as Apple writes it (RFC 3339).
    pub date_added: Option<String>,
    /// With `extend=editorialVideo`: the animated cover, when there is one.
    pub editorial_video: Option<EditorialVideo>,
    /// `℗ 2026 R.A.T MUSIC`.
    pub copyright: Option<String>,
    /// Mastered for Apple: an Apple Digital Master.
    pub is_mastered_for_itunes: Option<bool>,
}

/// An album's animated covers, as HLS master playlists.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EditorialVideo {
    pub motion_detail_square: Option<Video>,
    pub motion_square_video1x1: Option<Video>,
    /// An artist's, across the top of their page.
    pub motion_artist_fullscreen16x9: Option<Video>,
    pub motion_artist_wide16x9: Option<Video>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Video {
    pub video: String,
    /// Its first picture, to show while it comes.
    #[serde(rename = "previewFrame")]
    pub preview_frame: Option<Artwork>,
}

/// The pictures Apple's editors chose, at the sizes they made.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EditorialArtwork {
    /// 2:1, across the top of an artist's page.
    pub centered_fullscreen_background: Option<Artwork>,
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
    /// Library playlists: whether the user may change it (theirs, not one
    /// they follow or Apple's Favourite Songs).
    pub can_edit: Option<bool>,
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
    /// With `extend=editorialVideo`.
    pub editorial_video: Option<EditorialVideo>,
    /// With `extend=editorialArtwork`.
    pub editorial_artwork: Option<EditorialArtwork>,
}

pub type Song = Resource<SongAttributes>;
pub type Album = Resource<AlbumAttributes>;

/// An album or playlist asked for with `fields=inLibrary`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InLibrary {
    pub in_library: bool,
}
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
    #[serde(rename = "compilation-albums")]
    pub compilation_albums: Option<Response<Album>>,
    #[serde(rename = "appears-on-albums")]
    pub appears_on_albums: Option<Response<Album>>,
    #[serde(rename = "similar-artists")]
    pub similar_artists: Option<Response<Artist>>,
    /// Essential Albums.
    #[serde(rename = "featured-albums")]
    pub featured_albums: Option<Response<Album>>,
    #[serde(rename = "music-videos")]
    pub music_videos: Option<Response<MusicVideo>>,
    #[serde(rename = "featured-playlists")]
    pub featured_playlists: Option<Response<Playlist>>,
}

/// An album fetched with `views=`: the lists under its songs.
#[derive(Debug, Deserialize)]
pub struct AlbumWithViews {
    #[serde(default)]
    pub views: AlbumViews,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AlbumViews {
    #[serde(rename = "other-versions")]
    pub other_versions: Option<Response<Album>>,
    #[serde(rename = "related-videos")]
    pub related_videos: Option<Response<MusicVideo>>,
}

pub type MusicVideo = Resource<MusicVideoAttributes>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicVideoAttributes {
    pub name: String,
    #[serde(default)]
    pub artist_name: String,
    pub artwork: Option<Artwork>,
    pub content_rating: Option<String>,
    pub url: Option<String>,
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
