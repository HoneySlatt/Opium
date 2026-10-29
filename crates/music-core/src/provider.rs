use async_trait::async_trait;

use crate::model::{
    Album, ArtistDetail, Cursor, ItemId, Lyrics, Page, Playlist, ProviderKind, SearchResults, Track,
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

    async fn album(&self, album: &ItemId) -> Result<Album>;

    async fn album_tracks(&self, album: &ItemId) -> Result<Vec<Track>>;

    async fn artist(&self, artist: &ItemId) -> Result<ArtistDetail>;

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
