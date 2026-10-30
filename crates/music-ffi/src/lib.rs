//! The Rust core as the Android app sees it, through UniFFI: Kotlin calls
//! these as suspending functions and objects.

use std::sync::{Arc, RwLock};

use apple_music::{AppleMusic, Protected};
use music_core::Provider;
use music_core::model::{
    Album, Artist, ArtistDetail, Collection, Cursor, ItemId, Lyrics, Playlist, ProviderKind,
    SearchResults, Shelf, Track,
};
use navidrome::Navidrome;
use services::Services;

uniffi::setup_scaffolding!();

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MusicError {
    #[error("not signed in")]
    NotSignedIn,
    // Not `message`: Kotlin's exceptions have one of their own.
    #[error("{reason}")]
    Failed { reason: String },
}

impl From<music_core::Error> for MusicError {
    fn from(e: music_core::Error) -> Self {
        match e {
            music_core::Error::NotSignedIn => Self::NotSignedIn,
            e => Self::Failed {
                reason: e.to_string(),
            },
        }
    }
}

/// A page of a list, and where the next one starts (`None` at the end).
#[derive(uniffi::Record)]
pub struct TrackPage {
    pub items: Vec<Track>,
    pub next: Option<String>,
}

#[derive(uniffi::Record)]
pub struct AlbumPage {
    pub items: Vec<Album>,
    pub next: Option<String>,
}

#[derive(uniffi::Record)]
pub struct ArtistPage {
    pub items: Vec<Artist>,
    pub next: Option<String>,
}

#[derive(uniffi::Record)]
pub struct PlaylistPage {
    pub items: Vec<Playlist>,
    pub next: Option<String>,
}

#[derive(uniffi::Record)]
pub struct CollectionPage {
    pub items: Vec<Collection>,
    pub next: Option<String>,
}

/// An album and its songs.
#[derive(uniffi::Record)]
pub struct AlbumDetail {
    pub album: Album,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum SearchScope {
    /// Apple Music's and SoundCloud's catalogues, and the server.
    Catalog,
    /// The library: Apple Music's, and the server.
    Library,
}

/// The lyrics sites, in the order they are asked.
#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum LyricsSite {
    Lrclib,
    NetEase,
    KuGou,
}

impl From<LyricsSite> for lyrics::Source {
    fn from(site: LyricsSite) -> Self {
        match site {
            LyricsSite::Lrclib => lyrics::Source::Lrclib,
            LyricsSite::NetEase => lyrics::Source::NetEase,
            LyricsSite::KuGou => lyrics::Source::KuGou,
        }
    }
}

fn cursor(next: Option<String>) -> Option<Cursor> {
    next.map(Cursor)
}

fn next(page_next: Option<Cursor>) -> Option<String> {
    page_next.map(|c| c.0)
}

/// The services the app is signed in to, as one: what Kotlin asks of the
/// core goes through here.
#[derive(uniffi::Object)]
pub struct Session {
    services: RwLock<Services>,
    finder: lyrics::Finder,
}

impl Session {
    fn services(&self) -> Services {
        self.services
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn update(&self, change: impl FnOnce(&mut Services)) {
        change(&mut self.services.write().unwrap_or_else(|e| e.into_inner()));
    }

    fn provider(&self, kind: ProviderKind) -> Result<Arc<dyn Provider>, MusicError> {
        self.services()
            .provider(kind)
            .ok_or_else(|| MusicError::Failed {
                reason: format!("{kind:?} isn't connected"),
            })
    }

    async fn service_lyrics(&self, track: &Track) -> Option<Lyrics> {
        let provider = self.services().of(&track.id)?;
        provider.lyrics(&track.id).await.ok().flatten()
    }

    async fn site_lyrics(&self, track: &Track, sites: &[LyricsSite]) -> Option<Lyrics> {
        let query = lyrics::Query {
            title: track.title.clone(),
            artist: services::first_artist(&track.artists),
            album: track.album.clone(),
            duration: track.duration,
        };
        let sources: Vec<lyrics::Source> = sites.iter().map(|&s| s.into()).collect();
        self.finder.find(&query, &sources).await.map(|(l, _)| l)
    }

    fn apple(&self) -> Result<Arc<AppleMusic>, MusicError> {
        self.services().apple.ok_or_else(|| MusicError::Failed {
            reason: "Apple Music isn't connected".into(),
        })
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl Session {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            services: RwLock::default(),
            finder: lyrics::Finder::new(),
        })
    }

    /// Sign in to Apple Music with the `media-user-token` its sign-in page
    /// left; its storefront, as `fr`.
    pub async fn connect_apple(&self, user_token: String) -> Result<String, MusicError> {
        let apple = Arc::new(AppleMusic::connect(&user_token).await?);
        let storefront = apple.storefront().to_owned();
        self.update(|s| s.apple = Some(apple));
        Ok(storefront)
    }

    /// What the app keeps to sign in to a Navidrome server again: the
    /// address, the user and a salted token, never the password, as JSON.
    pub fn navidrome_credentials(
        &self,
        url: String,
        user: String,
        password: String,
    ) -> Result<String, MusicError> {
        let credentials = navidrome::Credentials::new(&url, &user, &password)?;
        drop(password);
        serde_json::to_string(&credentials).map_err(|e| MusicError::Failed {
            reason: e.to_string(),
        })
    }

    /// Sign in to the server with what `navidrome_credentials` made; the
    /// user's name.
    pub async fn connect_navidrome(&self, credentials: String) -> Result<String, MusicError> {
        let credentials: navidrome::Credentials =
            serde_json::from_str(&credentials).map_err(|e| MusicError::Failed {
                reason: e.to_string(),
            })?;
        let server = Arc::new(Navidrome::connect(credentials).await?);
        let user = server.user().to_owned();
        self.update(|s| s.navidrome = Some(server));
        Ok(user)
    }

    /// Stop using a service; its sign-in stays where the app keeps it.
    pub fn disconnect(&self, kind: ProviderKind) {
        self.update(|s| match kind {
            ProviderKind::AppleMusic => s.apple = None,
            ProviderKind::SoundCloud => s.soundcloud = None,
            ProviderKind::Navidrome => s.navidrome = None,
        });
    }

    /// Every connected service's results, each song and album once.
    pub async fn search(
        &self,
        scope: SearchScope,
        query: String,
    ) -> Result<SearchResults, MusicError> {
        let scope = match scope {
            SearchScope::Catalog => services::Scope::Catalog,
            SearchScope::Library => services::Scope::Library,
        };
        self.services()
            .search(scope, &query)
            .await
            .map_err(|reason| MusicError::Failed { reason })
    }

    pub async fn library_tracks(
        &self,
        kind: ProviderKind,
        page: Option<String>,
    ) -> Result<TrackPage, MusicError> {
        let page = self
            .provider(kind)?
            .library_tracks(cursor(page).as_ref())
            .await?;
        Ok(TrackPage {
            items: page.items,
            next: next(page.next),
        })
    }

    pub async fn library_albums(
        &self,
        kind: ProviderKind,
        page: Option<String>,
    ) -> Result<AlbumPage, MusicError> {
        let page = self
            .provider(kind)?
            .library_albums(cursor(page).as_ref())
            .await?;
        Ok(AlbumPage {
            items: page.items,
            next: next(page.next),
        })
    }

    pub async fn library_artists(
        &self,
        kind: ProviderKind,
        page: Option<String>,
    ) -> Result<ArtistPage, MusicError> {
        let page = self
            .provider(kind)?
            .library_artists(cursor(page).as_ref())
            .await?;
        Ok(ArtistPage {
            items: page.items,
            next: next(page.next),
        })
    }

    pub async fn library_playlists(
        &self,
        kind: ProviderKind,
        page: Option<String>,
    ) -> Result<PlaylistPage, MusicError> {
        let page = self
            .provider(kind)?
            .library_playlists(cursor(page).as_ref())
            .await?;
        Ok(PlaylistPage {
            items: page.items,
            next: next(page.next),
        })
    }

    pub async fn recently_added(
        &self,
        kind: ProviderKind,
        page: Option<String>,
    ) -> Result<CollectionPage, MusicError> {
        let page = self
            .provider(kind)?
            .recently_added(cursor(page).as_ref())
            .await?;
        Ok(CollectionPage {
            items: page.items,
            next: next(page.next),
        })
    }

    pub async fn album(&self, id: ItemId) -> Result<AlbumDetail, MusicError> {
        let provider = self.provider(id.provider)?;
        let (album, tracks) = tokio::try_join!(provider.album(&id), provider.album_tracks(&id))?;
        Ok(AlbumDetail { album, tracks })
    }

    pub async fn artist(&self, id: ItemId) -> Result<ArtistDetail, MusicError> {
        Ok(self.provider(id.provider)?.artist(&id).await?)
    }

    pub async fn playlist_tracks(
        &self,
        id: ItemId,
        page: Option<String>,
    ) -> Result<TrackPage, MusicError> {
        let page = self
            .provider(id.provider)?
            .playlist_tracks(&id, cursor(page).as_ref())
            .await?;
        Ok(TrackPage {
            items: page.items,
            next: next(page.next),
        })
    }

    /// Apple Music's Home, New and Radio pages, shelf by shelf.
    pub async fn home(&self) -> Result<Vec<Shelf>, MusicError> {
        Ok(self.apple()?.home().await?)
    }

    pub async fn new_music(&self) -> Result<Vec<Shelf>, MusicError> {
        Ok(self.apple()?.new_music().await?)
    }

    pub async fn radio(&self) -> Result<Vec<Shelf>, MusicError> {
        Ok(self.apple()?.radio().await?)
    }

    /// A song's synced lyrics: its service's, else the sites', in `sites`
    /// order (or the sites first, when `sites_first`). Synced lyrics from
    /// either beat plain ones from the other.
    pub async fn lyrics(
        &self,
        track: Track,
        sites: Vec<LyricsSite>,
        sites_first: bool,
    ) -> Result<Option<Lyrics>, MusicError> {
        let first = if sites_first {
            self.site_lyrics(&track, &sites).await
        } else {
            self.service_lyrics(&track).await
        };
        if first.as_ref().is_some_and(|l| l.synced) {
            return Ok(first);
        }
        let second = if sites_first {
            self.service_lyrics(&track).await
        } else {
            self.site_lyrics(&track, &sites).await
        };
        Ok(match (first, second) {
            (_, Some(s)) if s.synced => Some(s),
            (f, s) => f.or(s),
        })
    }

    /// An Apple song's encrypted file, for the player's own CDM.
    pub async fn protected(&self, song: ItemId) -> Result<Option<Arc<ProtectedTrack>>, MusicError> {
        let apple = self.apple()?;
        Ok(apple
            .protected(&song)
            .await?
            .map(|protected| Arc::new(ProtectedTrack { apple, protected })))
    }
}

/// The positions of `songs` to leave out of a list of every service's: the
/// copies of a song another service has a better copy of.
#[uniffi::export]
pub fn songs_left_out(songs: Vec<Track>) -> Vec<u32> {
    sorted(music_core::dedupe::songs(&songs))
}

#[uniffi::export]
pub fn albums_left_out(albums: Vec<Album>) -> Vec<u32> {
    sorted(music_core::dedupe::albums(&albums))
}

#[uniffi::export]
pub fn artists_left_out(artists: Vec<Artist>) -> Vec<u32> {
    sorted(music_core::dedupe::artists(&artists))
}

fn sorted(set: std::collections::HashSet<usize>) -> Vec<u32> {
    let mut v: Vec<u32> = set.into_iter().map(|i| i as u32).collect();
    v.sort_unstable();
    v
}

/// A cover's three colours, best first, as `0xrrggbb`, from its encoded
/// image.
#[uniffi::export]
pub fn cover_palette(image: Vec<u8>) -> Option<Vec<u32>> {
    let rgb = |[r, g, b]: [u8; 3]| u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
    music_core::palette::from_image(&image).map(|p| p.iter().copied().map(rgb).collect())
}

/// Apple Music, signed in with the user's `media-user-token`.
#[derive(uniffi::Object)]
pub struct Apple(Arc<AppleMusic>);

#[uniffi::export(async_runtime = "tokio")]
impl Apple {
    #[uniffi::constructor]
    pub async fn connect(user_token: String) -> Result<Arc<Self>, MusicError> {
        Ok(Arc::new(Self(Arc::new(
            AppleMusic::connect(&user_token).await?,
        ))))
    }

    pub fn storefront(&self) -> String {
        self.0.storefront().to_owned()
    }

    /// A catalog song's encrypted file and its key; `None` for an upload.
    pub async fn protected(&self, song: String) -> Result<Option<Arc<ProtectedTrack>>, MusicError> {
        let id = ItemId::new(ProviderKind::AppleMusic, song);
        Ok(self.0.protected(&id).await?.map(|protected| {
            Arc::new(ProtectedTrack {
                apple: self.0.clone(),
                protected,
            })
        }))
    }
}

/// An encrypted song, for the player's own CDM to license.
#[derive(uniffi::Object)]
pub struct ProtectedTrack {
    apple: Arc<AppleMusic>,
    protected: Protected,
}

#[uniffi::export(async_runtime = "tokio")]
impl ProtectedTrack {
    pub fn url(&self) -> String {
        self.protected.url.clone()
    }

    /// The `pssh` box the file lacks, to add to it.
    pub fn pssh(&self) -> Vec<u8> {
        self.protected.pssh.clone()
    }

    /// The media playlist, with the key as players read Widevine CENC.
    pub fn playlist(&self) -> String {
        self.protected.playlist.clone()
    }

    pub fn kbps(&self) -> u32 {
        self.protected.kbps
    }

    /// The frames the encoder put before the music, to leave out.
    pub fn priming(&self) -> u32 {
        apple_music::PRIMING
    }

    /// The frames after the music in the last frame, to leave out.
    pub fn padding(&self) -> u32 {
        self.protected.padding
    }

    /// The licence, in answer to the CDM's challenge.
    pub async fn license(&self, challenge: Vec<u8>) -> Result<Vec<u8>, MusicError> {
        Ok(self.apple.license(&self.protected, &challenge).await?)
    }
}

/// TLS checks certificates against Android's own store, which needs the
/// app's context: called once, before any request, from `Tls.init`.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_honeyslatt_music_core_Tls_init<'caller>(
    mut env: jni::EnvUnowned<'caller>,
    _class: jni::objects::JClass<'caller>,
    context: jni::objects::JObject<'caller>,
) {
    env.with_env(|env| rustls_platform_verifier::android::init_with_env(env, context))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}
