use async_trait::async_trait;

use crate::model::{
    Album, Artist, ArtistDetail, Collection, Cursor, FavouriteKind, ItemId, Lyrics, Page, Playlist,
    ProviderKind, SearchResults, Shelf, Track,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not signed in")]
    NotSignedIn,
    #[error("network error: {0}")]
    Network(String),
    #[error("unexpected response: {0}")]
    Protocol(String),
    #[error("playback failed: {0}")]
    Playback(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// One connected music service. Browsing only: playback is resolved separately,
/// since each platform plays streams its own way.
#[async_trait]
pub trait Provider: Send + Sync {
    fn kind(&self) -> ProviderKind;

    async fn search(&self, query: &str) -> Result<SearchResults>;

    /// The same search, in the user's library only.
    async fn search_library(&self, query: &str) -> Result<SearchResults>;

    /// The user's saved songs (Apple library songs, SoundCloud likes, Navidrome starred).
    async fn library_tracks(&self, cursor: Option<&Cursor>) -> Result<Page<Track>>;

    async fn library_albums(&self, cursor: Option<&Cursor>) -> Result<Page<Album>>;

    async fn library_playlists(&self, cursor: Option<&Cursor>) -> Result<Page<Playlist>>;

    async fn library_artists(&self, cursor: Option<&Cursor>) -> Result<Page<Artist>>;

    /// Albums and playlists, most recently added first.
    async fn recently_added(&self, cursor: Option<&Cursor>) -> Result<Page<Collection>>;

    async fn album(&self, album: &ItemId) -> Result<Album>;

    async fn album_tracks(&self, album: &ItemId) -> Result<Vec<Track>>;

    async fn artist(&self, artist: &ItemId) -> Result<ArtistDetail>;

    /// Add `id` to the user's favourites, or take it out. `id` names the
    /// service's catalog item.
    async fn set_favourite(&self, _kind: FavouriteKind, _id: &ItemId, _on: bool) -> Result<()> {
        Err(Error::Protocol("this service has no favourites".into()))
    }

    /// Add a catalog song, album or playlist to the user's library.
    async fn add_to_library(&self, _kind: FavouriteKind, _id: &ItemId) -> Result<()> {
        Err(Error::Protocol(
            "this service has no library to add to".into(),
        ))
    }

    /// Add songs to one of the user's playlists, at its end.
    async fn add_to_playlist(&self, _playlist: &ItemId, _tracks: &[ItemId]) -> Result<()> {
        Err(Error::Protocol(
            "this service's playlists can't be added to".into(),
        ))
    }

    /// Whether `id` is among the user's favourites, when the service can say.
    async fn is_favourite(&self, _kind: FavouriteKind, _id: &ItemId) -> Result<Option<bool>> {
        Ok(None)
    }

    /// The radio page: stations on air, then shelves of stations and shows.
    async fn radio(&self) -> Result<Vec<Shelf>> {
        Ok(Vec::new())
    }

    /// New music, shelf by shelf, as the service's editors show it.
    async fn new_music(&self) -> Result<Vec<Shelf>> {
        Ok(Vec::new())
    }

    /// The next songs of a station that plays songs, about `count` of them.
    async fn station_tracks(&self, _station: &ItemId, _count: usize) -> Result<Vec<Track>> {
        Ok(Vec::new())
    }

    /// What the service recommends, shelf by shelf. Not every service has this.
    async fn home(&self) -> Result<Vec<Shelf>> {
        Ok(Vec::new())
    }

    async fn playlist_tracks(
        &self,
        playlist: &ItemId,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Track>>;

    /// Synced lyrics when the service has them. Services without lyrics keep
    /// this default.
    async fn lyrics(&self, _track: &ItemId) -> Result<Option<Lyrics>> {
        Ok(None)
    }
}
