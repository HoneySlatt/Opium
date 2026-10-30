//! The Rust core as the Android app sees it, through UniFFI: Kotlin calls
//! these as suspending functions and objects.

use std::sync::{Arc, RwLock};

use apple_music::{AppleMusic, Protected};
use music_core::Provider;
use music_core::model::{
    Album, AlbumExtras, Artist, ArtistDetail, Collection, Cursor, FavouriteKind, ItemId, Lyrics,
    Playlist, ProviderKind, SearchResults, Shelf, SongLinks, Track,
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
        let warming = apple.clone();
        tokio::spawn(async move { warming.warm().await });
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

    /// The saved songs, the latest added first.
    pub async fn library_tracks_newest(
        &self,
        kind: ProviderKind,
        page: Option<String>,
    ) -> Result<TrackPage, MusicError> {
        let page = self
            .provider(kind)?
            .library_tracks_newest(cursor(page).as_ref())
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

    /// The album's animated cover, without its songs: Now Playing's
    /// background.
    pub async fn album_motion(&self, id: ItemId) -> Result<Option<String>, MusicError> {
        Ok(self.provider(id.provider)?.album(&id).await?.motion)
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

    /// Add to the favourites, or take out. `id` names the catalog item.
    pub async fn set_favourite(
        &self,
        kind: FavouriteKind,
        id: ItemId,
        on: bool,
    ) -> Result<(), MusicError> {
        Ok(self
            .provider(id.provider)?
            .set_favourite(kind, &id, on)
            .await?)
    }

    /// Whether `id` is a favourite, when the service can say.
    pub async fn is_favourite(
        &self,
        kind: FavouriteKind,
        id: ItemId,
    ) -> Result<Option<bool>, MusicError> {
        Ok(self.provider(id.provider)?.is_favourite(kind, &id).await?)
    }

    pub async fn add_to_library(&self, kind: FavouriteKind, id: ItemId) -> Result<(), MusicError> {
        Ok(self
            .provider(id.provider)?
            .add_to_library(kind, &id)
            .await?)
    }

    /// Take a song out of the library, by its library id.
    pub async fn remove_from_library(&self, library_id: ItemId) -> Result<(), MusicError> {
        Ok(self
            .provider(library_id.provider)?
            .remove_from_library(&library_id)
            .await?)
    }

    pub async fn create_playlist(
        &self,
        kind: ProviderKind,
        name: String,
        description: String,
        tracks: Vec<ItemId>,
    ) -> Result<Playlist, MusicError> {
        Ok(self
            .provider(kind)?
            .create_playlist(&name, &description, &tracks)
            .await?)
    }

    pub async fn edit_playlist(
        &self,
        playlist: ItemId,
        name: String,
        description: String,
    ) -> Result<(), MusicError> {
        Ok(self
            .provider(playlist.provider)?
            .edit_playlist(&playlist, &name, &description)
            .await?)
    }

    pub async fn delete_playlist(&self, playlist: ItemId) -> Result<(), MusicError> {
        Ok(self
            .provider(playlist.provider)?
            .delete_playlist(&playlist)
            .await?)
    }

    /// Take a song out of a playlist, by its row's id there.
    pub async fn remove_from_playlist(
        &self,
        playlist: ItemId,
        entry: ItemId,
    ) -> Result<(), MusicError> {
        Ok(self
            .provider(playlist.provider)?
            .remove_from_playlist(&playlist, &entry)
            .await?)
    }

    pub async fn add_to_playlist(
        &self,
        playlist: ItemId,
        tracks: Vec<ItemId>,
    ) -> Result<(), MusicError> {
        Ok(self
            .provider(playlist.provider)?
            .add_to_playlist(&playlist, &tracks)
            .await?)
    }

    /// The next `count` songs of a station.
    pub async fn station_tracks(
        &self,
        station: ItemId,
        count: u32,
    ) -> Result<Vec<Track>, MusicError> {
        Ok(self
            .provider(station.provider)?
            .station_tracks(&station, count as usize)
            .await?)
    }

    /// A song's artist and station, to open from it.
    pub async fn song_links(&self, song: ItemId) -> Result<SongLinks, MusicError> {
        Ok(self.provider(song.provider)?.song_links(&song).await?)
    }

    /// A library artist's releases in the library: Apple Music's from its
    /// library, the other services' from their artist pages.
    pub async fn library_artist_albums(&self, id: ItemId) -> Result<Vec<Album>, MusicError> {
        if id.provider == ProviderKind::AppleMusic {
            return Ok(self.apple()?.library_artist_albums(&id).await?);
        }
        let artist = self.provider(id.provider)?.artist(&id).await?;
        Ok([artist.albums, artist.singles, artist.compilations].concat())
    }

    /// A catalog album's other editions and its songs' music videos.
    pub async fn album_extras(&self, id: ItemId) -> Result<AlbumExtras, MusicError> {
        Ok(self.apple()?.album_extras(&id).await?)
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
            .map(|protected| ProtectedTrack::new(apple.clone(), protected)))
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

/// What plays after the current song ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum QueueRepeat {
    Off,
    All,
    One,
}

/// The play queue, the desktop's own: tracks in their order, a play order
/// over them for shuffle, repeat.
#[derive(uniffi::Object, Default)]
pub struct PlayQueue(std::sync::Mutex<music_core::Queue>);

impl PlayQueue {
    fn with<T>(&self, f: impl FnOnce(&mut music_core::Queue) -> T) -> T {
        f(&mut self.0.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

#[uniffi::export]
impl PlayQueue {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::default()
    }

    /// Play `tracks`, from `start`.
    pub fn replace(&self, tracks: Vec<Track>, start: u32) {
        self.with(|q| q.replace(tracks, start as usize));
    }

    pub fn current(&self) -> Option<Track> {
        self.with(|q| q.current().cloned())
    }

    /// What follows the current song, in play order.
    pub fn upcoming(&self) -> Vec<Track> {
        self.with(|q| q.upcoming().cloned().collect())
    }

    /// Next, as the user asks: wraps around unless repeat is off.
    pub fn skip_next(&self) -> Option<Track> {
        self.with(|q| q.skip_next().cloned())
    }

    pub fn skip_previous(&self) -> Option<Track> {
        self.with(|q| q.skip_previous().cloned())
    }

    /// The current song played to its end: what plays now.
    pub fn track_finished(&self) -> Option<Track> {
        self.with(|q| q.track_finished().cloned())
    }

    /// What `track_finished` would move to, without moving.
    pub fn peek_finished(&self) -> Option<Track> {
        self.with(|q| q.peek_finished().cloned())
    }

    pub fn add_next(&self, track: Track) {
        self.with(|q| q.add_next(track));
    }

    pub fn add_last(&self, track: Track) {
        self.with(|q| q.add_last(track));
    }

    pub fn remove_upcoming(&self, k: u32) {
        self.with(|q| q.remove_upcoming(k as usize));
    }

    pub fn clear_upcoming(&self) {
        self.with(|q| q.clear_upcoming());
    }

    /// Make the `k`-th upcoming song current.
    pub fn jump(&self, k: u32) -> Option<Track> {
        self.with(|q| q.jump(k as usize).cloned())
    }

    pub fn shuffle(&self) -> bool {
        self.with(|q| q.shuffle())
    }

    pub fn set_shuffle(&self, on: bool) {
        self.with(|q| q.set_shuffle(on));
    }

    pub fn repeat(&self) -> QueueRepeat {
        match self.with(|q| q.repeat()) {
            music_core::Repeat::Off => QueueRepeat::Off,
            music_core::Repeat::All => QueueRepeat::All,
            music_core::Repeat::One => QueueRepeat::One,
        }
    }

    pub fn set_repeat(&self, repeat: QueueRepeat) {
        self.with(|q| {
            q.set_repeat(match repeat {
                QueueRepeat::Off => music_core::Repeat::Off,
                QueueRepeat::All => music_core::Repeat::All,
                QueueRepeat::One => music_core::Repeat::One,
            })
        });
    }
}

/// Where Apple Music's song files are, for the player to connect to ahead.
#[uniffi::export]
pub fn apple_asset_host() -> String {
    apple_music::ASSET_HOST.to_owned()
}

/// A cover's three colours, best first, as `0xrrggbb`, from its encoded
/// image.
#[uniffi::export]
pub fn cover_palette(image: Vec<u8>) -> Option<Vec<u32>> {
    let rgb = |[r, g, b]: [u8; 3]| u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
    music_core::palette::from_image(&image).map(|p| p.iter().copied().map(rgb).collect())
}

/// The three colours of Now Playing's wash, from a cover: its palette,
/// pulled into the theme's band so a dark cover still gives colour and a
/// vivid one never glares. As `0xrrggbb`.
#[uniffi::export]
pub fn wash_colours(image: Vec<u8>, light: bool) -> Option<Vec<u32>> {
    use music_core::palette::{fit, from_image};
    let rgb = |[r, g, b]: [u8; 3]| u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
    from_image(&image).map(|p| p.iter().map(|&c| rgb(fit(c, light))).collect())
}

/// The colour of a page tinted by a cover from top to bottom: the most
/// colourful of the cover's three, dim enough on dark (bright enough on
/// light) for text to keep its contrast. As `0xrrggbb`.
#[uniffi::export]
pub fn page_colour(image: Vec<u8>, light: bool) -> Option<u32> {
    use music_core::palette::{Band, fit_in, from_image, most_colourful};
    let palette = from_image(&image)?;
    let band = if light {
        Band::LIGHT_PAGE
    } else {
        Band::DARK_PAGE
    };
    let [r, g, b] = fit_in(most_colourful(&palette), band);
    Some(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b))
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
        Ok(self
            .0
            .protected(&id)
            .await?
            .map(|protected| ProtectedTrack::new(self.0.clone(), protected)))
    }
}

/// An encrypted song, for the player's own CDM to license.
#[derive(uniffi::Object)]
pub struct ProtectedTrack {
    apple: Arc<AppleMusic>,
    protected: Protected,
    padding: std::sync::Mutex<Option<PaddingTask>>,
}

enum PaddingTask {
    Running(tokio::task::JoinHandle<u32>),
    Done(u32),
}

impl ProtectedTrack {
    fn new(apple: Arc<AppleMusic>, protected: Protected) -> Arc<Self> {
        let task = {
            let (apple, protected) = (apple.clone(), protected.clone());
            tokio::spawn(async move { apple.padding(&protected).await })
        };
        Arc::new(Self {
            apple,
            protected,
            padding: std::sync::Mutex::new(Some(PaddingTask::Running(task))),
        })
    }
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

    /// The frames after the music in the last frame, to leave out: read
    /// in the background from the start, as the song needn't wait for it.
    pub async fn padding(&self) -> u32 {
        let task = self
            .padding
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        let found = match task {
            Some(PaddingTask::Running(task)) => task.await.unwrap_or(0),
            Some(PaddingTask::Done(n)) => n,
            None => 0,
        };
        *self.padding.lock().unwrap_or_else(|e| e.into_inner()) = Some(PaddingTask::Done(found));
        found
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
