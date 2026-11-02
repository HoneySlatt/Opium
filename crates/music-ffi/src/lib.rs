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

/// Run `work` on the runtime's threads (several: see `async-compat` in the
/// manifest). A future runs where it is polled,
/// and Kotlin polls where it waits, often Android's main thread: the TLS
/// (whose certificate check may go to the network) and the parsing belong
/// elsewhere.
async fn off<T: Send + 'static>(
    work: impl std::future::Future<Output = Result<T, MusicError>> + Send + 'static,
) -> Result<T, MusicError> {
    tokio::spawn(work).await.map_err(|e| MusicError::Failed {
        reason: e.to_string(),
    })?
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
    /// Music's own playlists, kept with the other devices: see [`Session::own`].
    own: Arc<RwLock<Option<Arc<Connect>>>>,
    /// Where the app keeps its own: see [`Session::keep_in`].
    dir: RwLock<Option<std::path::PathBuf>>,
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

    fn connect(&self) -> Result<Arc<Connect>, MusicError> {
        self.own
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or_else(|| MusicError::Failed {
                reason: "Music's playlists aren't there yet".into(),
            })
    }

    fn own_playlist(&self, id: &ItemId) -> Result<synced::Playlist, MusicError> {
        self.connect()?
            .1
            .playlist(&id.id)
            .ok_or_else(|| MusicError::Failed {
                reason: "This playlist was deleted".into(),
            })
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
            own: Arc::default(),
            dir: RwLock::default(),
        })
    }

    /// Keep in `dir` what the services' sign-ins need kept: when Apple's
    /// newest songs were first seen, for Recently Added.
    pub fn keep_in(&self, dir: String) {
        *self.dir.write().unwrap_or_else(|e| e.into_inner()) = Some(dir.into());
    }

    /// Music's own playlists, as `connect` keeps them, answered with the
    /// services' as `ProviderKind::Music`.
    pub fn own(&self, connect: Arc<Connect>) {
        *self.own.write().unwrap_or_else(|e| e.into_inner()) = Some(connect);
    }

    /// Sign in to Apple Music with the `media-user-token` its sign-in page
    /// left; its storefront, as `fr`.
    pub async fn connect_apple(self: Arc<Self>, user_token: String) -> Result<String, MusicError> {
        off(async move {
            let apple = Arc::new(AppleMusic::connect(&user_token).await?);
            if let Some(dir) = self.dir.read().unwrap_or_else(|e| e.into_inner()).clone() {
                apple.keep_seen_in(
                    dir.join("apple-seen.json"),
                    Some(Arc::new(SeenShared(self.own.clone()))),
                );
            }
            let storefront = apple.storefront().to_owned();
            let warming = apple.clone();
            tokio::spawn(async move { warming.warm().await });
            self.update(|s| s.apple = Some(apple));
            Ok(storefront)
        })
        .await
    }

    /// Where to fetch the cover at `url` from: signed, if it is the
    /// server's (kept and shown, its address carries no sign-in).
    pub fn signed_cover(&self, url: String) -> String {
        self.services()
            .navidrome
            .and_then(|nd| nd.sign(&url))
            .unwrap_or(url)
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
    pub async fn connect_navidrome(
        self: Arc<Self>,
        credentials: String,
    ) -> Result<String, MusicError> {
        off(async move {
            let credentials: navidrome::Credentials =
                serde_json::from_str(&credentials).map_err(|e| MusicError::Failed {
                    reason: e.to_string(),
                })?;
            let server = Arc::new(Navidrome::connect(credentials).await?);
            let user = server.user().to_owned();
            self.update(|s| s.navidrome = Some(server));
            Ok(user)
        })
        .await
    }

    /// Sign in to SoundCloud with the web player's tokens; the user's name.
    /// `keeper` is told the tokens each time they are renewed; `browser`
    /// sends the changes, which SoundCloud turns away from any other client.
    pub async fn connect_soundcloud(
        self: Arc<Self>,
        oauth: String,
        refresh: Option<String>,
        keeper: Arc<dyn TokenKeeper>,
        browser: Arc<dyn SoundCloudBrowser>,
    ) -> Result<String, MusicError> {
        off(async move {
            let on_tokens: soundcloud::OnTokens = Box::new(move |t: &soundcloud::Tokens| {
                keeper.keep(t.oauth.clone(), t.refresh.clone());
            });
            let tokens = soundcloud::Tokens { oauth, refresh };
            let browser: Arc<dyn soundcloud::Browser> = Arc::new(ForeignBrowser(browser));
            let sc =
                Arc::new(soundcloud::SoundCloud::connect(tokens, on_tokens, Some(browser)).await?);
            let user = sc.username().to_owned();
            self.update(|s| s.soundcloud = Some(sc));
            Ok(user)
        })
        .await
    }

    /// Where a SoundCloud track plays from: the uploader's file when
    /// `original` and they allow it, else the best stream.
    pub async fn soundcloud_stream(
        self: Arc<Self>,
        track: ItemId,
        original: bool,
    ) -> Result<Arc<SoundCloudStream>, MusicError> {
        off(async move {
            let sc = self
                .services()
                .soundcloud
                .ok_or_else(|| MusicError::Failed {
                    reason: "SoundCloud isn't connected".into(),
                })?;
            let playable = sc.playable(&track, original).await?;
            Ok(Arc::new(SoundCloudStream { sc, playable }))
        })
        .await
    }

    /// Where a Navidrome song's file streams from, as it is on the server.
    /// Signed with the account's token: never shown or logged.
    pub fn navidrome_stream_url(&self, track: ItemId) -> Result<String, MusicError> {
        let server = self
            .services()
            .navidrome
            .ok_or_else(|| MusicError::Failed {
                reason: "Navidrome isn't connected".into(),
            })?;
        Ok(server.stream_url(&track))
    }

    /// Stop using a service; its sign-in stays where the app keeps it.
    pub fn disconnect(&self, kind: ProviderKind) {
        self.update(|s| match kind {
            ProviderKind::AppleMusic => s.apple = None,
            ProviderKind::SoundCloud => s.soundcloud = None,
            ProviderKind::Navidrome => s.navidrome = None,
            ProviderKind::Music => {}
        });
    }

    /// Every connected service's results, each song and album once; or
    /// `only` that service's, all of them.
    pub async fn search(
        self: Arc<Self>,
        scope: SearchScope,
        query: String,
        only: Option<ProviderKind>,
    ) -> Result<SearchResults, MusicError> {
        off(async move {
            let scope = match scope {
                SearchScope::Catalog => services::Scope::Catalog,
                SearchScope::Library => services::Scope::Library,
            };
            self.services()
                .search(scope, &query, only)
                .await
                .map_err(|reason| MusicError::Failed { reason })
        })
        .await
    }

    /// Every Apple Music library song at once: its pages are asked for many
    /// at a time, where one after another takes half a minute.
    pub async fn apple_library_tracks(self: Arc<Self>) -> Result<Vec<Track>, MusicError> {
        off(async move {
            let mut all = Vec::new();
            self.apple()?
                .each_library_track(|page, _| all.extend(page))
                .await?;
            Ok(all)
        })
        .await
    }

    /// Every Apple Music library song at once, the latest added first.
    pub async fn apple_library_tracks_newest(self: Arc<Self>) -> Result<Vec<Track>, MusicError> {
        off(async move {
            let mut all = Vec::new();
            self.apple()?
                .each_newest_library_track(|page, _| all.extend(page))
                .await?;
            Ok(all)
        })
        .await
    }

    /// Every Apple Music library album at once, as [`Self::apple_library_tracks`].
    pub async fn apple_library_albums(self: Arc<Self>) -> Result<Vec<Album>, MusicError> {
        off(async move {
            let mut all = Vec::new();
            self.apple()?
                .each_library_album(|page, _| all.extend(page))
                .await?;
            Ok(all)
        })
        .await
    }

    /// Every Apple Music library artist at once, as [`Self::apple_library_tracks`].
    pub async fn apple_library_artists(self: Arc<Self>) -> Result<Vec<Artist>, MusicError> {
        off(async move {
            let mut all = Vec::new();
            self.apple()?
                .each_library_artist(|page, _| all.extend(page))
                .await?;
            Ok(all)
        })
        .await
    }

    pub async fn library_tracks(
        self: Arc<Self>,
        kind: ProviderKind,
        page: Option<String>,
    ) -> Result<TrackPage, MusicError> {
        off(async move {
            let page = self
                .provider(kind)?
                .library_tracks(cursor(page).as_ref())
                .await?;
            Ok(TrackPage {
                items: page.items,
                next: next(page.next),
            })
        })
        .await
    }

    /// The saved songs, the latest added first.
    pub async fn library_tracks_newest(
        self: Arc<Self>,
        kind: ProviderKind,
        page: Option<String>,
    ) -> Result<TrackPage, MusicError> {
        off(async move {
            let page = self
                .provider(kind)?
                .library_tracks_newest(cursor(page).as_ref())
                .await?;
            Ok(TrackPage {
                items: page.items,
                next: next(page.next),
            })
        })
        .await
    }

    pub async fn library_albums(
        self: Arc<Self>,
        kind: ProviderKind,
        page: Option<String>,
    ) -> Result<AlbumPage, MusicError> {
        off(async move {
            let page = self
                .provider(kind)?
                .library_albums(cursor(page).as_ref())
                .await?;
            Ok(AlbumPage {
                items: page.items,
                next: next(page.next),
            })
        })
        .await
    }

    pub async fn library_artists(
        self: Arc<Self>,
        kind: ProviderKind,
        page: Option<String>,
    ) -> Result<ArtistPage, MusicError> {
        off(async move {
            let page = self
                .provider(kind)?
                .library_artists(cursor(page).as_ref())
                .await?;
            Ok(ArtistPage {
                items: page.items,
                next: next(page.next),
            })
        })
        .await
    }

    pub async fn library_playlists(
        self: Arc<Self>,
        kind: ProviderKind,
        page: Option<String>,
    ) -> Result<PlaylistPage, MusicError> {
        if kind == ProviderKind::Music {
            return Ok(PlaylistPage {
                items: self
                    .connect()?
                    .1
                    .playlists()
                    .iter()
                    .map(synced::Playlist::model)
                    .collect(),
                next: None,
            });
        }
        off(async move {
            let page = self
                .provider(kind)?
                .library_playlists(cursor(page).as_ref())
                .await?;
            Ok(PlaylistPage {
                items: page.items,
                next: next(page.next),
            })
        })
        .await
    }

    pub async fn recently_added(
        self: Arc<Self>,
        kind: ProviderKind,
        page: Option<String>,
    ) -> Result<CollectionPage, MusicError> {
        off(async move {
            let page = self
                .provider(kind)?
                .recently_added(cursor(page).as_ref())
                .await?;
            Ok(CollectionPage {
                items: page.items,
                next: next(page.next),
            })
        })
        .await
    }

    pub async fn album(self: Arc<Self>, id: ItemId) -> Result<AlbumDetail, MusicError> {
        off(async move {
            let provider = self.provider(id.provider)?;
            let (album, tracks) =
                tokio::try_join!(provider.album(&id), provider.album_tracks(&id))?;
            Ok(AlbumDetail { album, tracks })
        })
        .await
    }

    /// The album's animated cover, without its songs: Now Playing's
    /// background.
    pub async fn album_motion(self: Arc<Self>, id: ItemId) -> Result<Option<String>, MusicError> {
        off(async move { Ok(self.provider(id.provider)?.album(&id).await?.motion) }).await
    }

    pub async fn artist(self: Arc<Self>, id: ItemId) -> Result<ArtistDetail, MusicError> {
        off(async move { Ok(self.provider(id.provider)?.artist(&id).await?) }).await
    }

    pub async fn playlist_tracks(
        self: Arc<Self>,
        id: ItemId,
        page: Option<String>,
    ) -> Result<TrackPage, MusicError> {
        if id.provider == ProviderKind::Music {
            return Ok(TrackPage {
                items: self.own_playlist(&id)?.tracks(),
                next: None,
            });
        }
        off(async move {
            let page = self
                .provider(id.provider)?
                .playlist_tracks(&id, cursor(page).as_ref())
                .await?;
            Ok(TrackPage {
                items: page.items,
                next: next(page.next),
            })
        })
        .await
    }

    /// Add to the favourites, or take out. `id` names the catalog item.
    pub async fn set_favourite(
        self: Arc<Self>,
        kind: FavouriteKind,
        id: ItemId,
        on: bool,
    ) -> Result<(), MusicError> {
        off(async move {
            Ok(self
                .provider(id.provider)?
                .set_favourite(kind, &id, on)
                .await?)
        })
        .await
    }

    /// Whether `id` is a favourite, when the service can say.
    pub async fn is_favourite(
        self: Arc<Self>,
        kind: FavouriteKind,
        id: ItemId,
    ) -> Result<Option<bool>, MusicError> {
        off(async move { Ok(self.provider(id.provider)?.is_favourite(kind, &id).await?) }).await
    }

    /// Ask for less like `id`, or take that back. `id` names the catalog item.
    pub async fn suggest_less(
        self: Arc<Self>,
        kind: FavouriteKind,
        id: ItemId,
        on: bool,
    ) -> Result<(), MusicError> {
        off(async move {
            Ok(self
                .provider(id.provider)?
                .suggest_less(kind, &id, on)
                .await?)
        })
        .await
    }

    /// Whether less like `id` was asked for, when the service can say.
    pub async fn suggests_less(
        self: Arc<Self>,
        kind: FavouriteKind,
        id: ItemId,
    ) -> Result<Option<bool>, MusicError> {
        off(async move { Ok(self.provider(id.provider)?.suggests_less(kind, &id).await?) }).await
    }

    /// The page of `id` on its service's website, to share.
    pub async fn share_link(
        self: Arc<Self>,
        kind: FavouriteKind,
        id: ItemId,
    ) -> Result<Option<String>, MusicError> {
        off(async move { Ok(self.provider(id.provider)?.share_link(kind, &id).await?) }).await
    }

    pub async fn add_to_library(
        self: Arc<Self>,
        kind: FavouriteKind,
        id: ItemId,
    ) -> Result<(), MusicError> {
        off(async move {
            Ok(self
                .provider(id.provider)?
                .add_to_library(kind, &id)
                .await?)
        })
        .await
    }

    /// Take a song out of the library, by its library id.
    pub async fn remove_from_library(
        self: Arc<Self>,
        library_id: ItemId,
    ) -> Result<(), MusicError> {
        off(async move {
            Ok(self
                .provider(library_id.provider)?
                .remove_from_library(&library_id)
                .await?)
        })
        .await
    }

    pub async fn create_playlist(
        self: Arc<Self>,
        kind: ProviderKind,
        name: String,
        description: String,
        tracks: Vec<Track>,
    ) -> Result<Playlist, MusicError> {
        if kind == ProviderKind::Music {
            let own = self.connect()?;
            let (id, change) = own.1.create_playlist(name, description, tracks);
            own.changed(change);
            return Ok(self
                .own_playlist(&ItemId::new(ProviderKind::Music, id))?
                .model());
        }
        let tracks: Vec<ItemId> = tracks.into_iter().map(|t| t.id).collect();
        off(async move {
            Ok(self
                .provider(kind)?
                .create_playlist(&name, &description, &tracks)
                .await?)
        })
        .await
    }

    pub async fn edit_playlist(
        self: Arc<Self>,
        playlist: ItemId,
        name: String,
        description: String,
    ) -> Result<(), MusicError> {
        if playlist.provider == ProviderKind::Music {
            let own = self.connect()?;
            own.changed(own.1.edit_playlist(&playlist.id, name, description));
            return Ok(());
        }
        off(async move {
            Ok(self
                .provider(playlist.provider)?
                .edit_playlist(&playlist, &name, &description)
                .await?)
        })
        .await
    }

    pub async fn delete_playlist(self: Arc<Self>, playlist: ItemId) -> Result<(), MusicError> {
        if playlist.provider == ProviderKind::Music {
            let own = self.connect()?;
            own.changed(own.1.delete_playlist(&playlist.id));
            return Ok(());
        }
        off(async move {
            Ok(self
                .provider(playlist.provider)?
                .delete_playlist(&playlist)
                .await?)
        })
        .await
    }

    /// Take a song out of a playlist, by its row's id there; out of one of
    /// Music's, by its own, every time it is in.
    pub async fn remove_from_playlist(
        self: Arc<Self>,
        playlist: ItemId,
        entry: ItemId,
    ) -> Result<(), MusicError> {
        if playlist.provider == ProviderKind::Music {
            let entries = self.own_playlist(&playlist)?.entries_of(&entry);
            let own = self.connect()?;
            own.changed(own.1.remove_from_playlist(&playlist.id, &entries));
            return Ok(());
        }
        off(async move {
            Ok(self
                .provider(playlist.provider)?
                .remove_from_playlist(&playlist, &entry)
                .await?)
        })
        .await
    }

    pub async fn add_to_playlist(
        self: Arc<Self>,
        playlist: ItemId,
        tracks: Vec<Track>,
    ) -> Result<(), MusicError> {
        if playlist.provider == ProviderKind::Music {
            let own = self.connect()?;
            own.changed(own.1.add_to_playlist(&playlist.id, tracks));
            return Ok(());
        }
        let tracks: Vec<ItemId> = tracks.into_iter().map(|t| t.id).collect();
        off(async move {
            Ok(self
                .provider(playlist.provider)?
                .add_to_playlist(&playlist, &tracks)
                .await?)
        })
        .await
    }

    /// The next `count` songs of a station.
    pub async fn station_tracks(
        self: Arc<Self>,
        station: ItemId,
        count: u32,
    ) -> Result<Vec<Track>, MusicError> {
        off(async move {
            Ok(self
                .provider(station.provider)?
                .station_tracks(&station, count as usize)
                .await?)
        })
        .await
    }

    /// A song's artist and station, to open from it.
    pub async fn song_links(self: Arc<Self>, song: ItemId) -> Result<SongLinks, MusicError> {
        off(async move { Ok(self.provider(song.provider)?.song_links(&song).await?) }).await
    }

    /// A library artist's releases in the library: Apple Music's from its
    /// library, the other services' from their artist pages.
    pub async fn library_artist_albums(
        self: Arc<Self>,
        id: ItemId,
    ) -> Result<Vec<Album>, MusicError> {
        off(async move {
            if id.provider == ProviderKind::AppleMusic {
                return Ok(self.apple()?.library_artist_albums(&id).await?);
            }
            let artist = self.provider(id.provider)?.artist(&id).await?;
            Ok([artist.albums, artist.singles, artist.compilations].concat())
        })
        .await
    }

    /// The account's profile on Apple Music, where it has made one.
    pub async fn apple_profile(self: Arc<Self>) -> Result<Option<AppleProfile>, MusicError> {
        off(async move {
            Ok(self.apple()?.profile().await?.map(|p| AppleProfile {
                name: p.name,
                handle: p.handle,
                picture: p.picture,
            }))
        })
        .await
    }

    /// A catalog album's other editions and its songs' music videos.
    pub async fn album_extras(self: Arc<Self>, id: ItemId) -> Result<AlbumExtras, MusicError> {
        off(async move { Ok(self.apple()?.album_extras(&id).await?) }).await
    }

    /// Apple Music's Home, New and Radio pages, shelf by shelf.
    pub async fn home(self: Arc<Self>) -> Result<Vec<Shelf>, MusicError> {
        off(async move { Ok(self.apple()?.home().await?) }).await
    }

    pub async fn new_music(self: Arc<Self>) -> Result<Vec<Shelf>, MusicError> {
        off(async move { Ok(self.apple()?.new_music().await?) }).await
    }

    /// Apple Music's genres, to browse from Search.
    pub async fn genres(self: Arc<Self>) -> Result<Vec<music_core::model::Genre>, MusicError> {
        off(async move { Ok(self.apple()?.genres().await?) }).await
    }

    /// A genre's charts, as shelves.
    pub async fn genre_charts(self: Arc<Self>, genre: String) -> Result<Vec<Shelf>, MusicError> {
        off(async move { Ok(self.apple()?.genre_charts(&genre).await?) }).await
    }

    pub async fn radio(self: Arc<Self>) -> Result<Vec<Shelf>, MusicError> {
        off(async move { Ok(self.apple()?.radio().await?) }).await
    }

    /// A song's synced lyrics: its service's, else the sites', in `sites`
    /// order (or the sites first, when `sites_first`). Synced lyrics from
    /// either beat plain ones from the other.
    pub async fn lyrics(
        self: Arc<Self>,
        track: Track,
        sites: Vec<LyricsSite>,
        sites_first: bool,
    ) -> Result<Option<Lyrics>, MusicError> {
        off(async move {
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
        })
        .await
    }

    /// An Apple song's encrypted file, for the player's own CDM: with
    /// `spatial`, a song mixed in Dolby Atmos as Spatial Audio.
    pub async fn protected(
        self: Arc<Self>,
        song: ItemId,
        spatial: bool,
    ) -> Result<Option<Arc<ProtectedTrack>>, MusicError> {
        off(async move {
            let apple = self.apple()?;
            Ok(apple
                .protected(&song, spatial)
                .await?
                .map(|protected| ProtectedTrack::new(apple.clone(), protected)))
        })
        .await
    }

    /// A station that streams (on air, or an episode), to tune in to.
    pub async fn live_station(
        self: Arc<Self>,
        station: ItemId,
    ) -> Result<Arc<LiveStation>, MusicError> {
        off(async move {
            let apple = self.apple()?;
            let asset = apple.live_asset(&station).await?;
            let (init, key_uri) = apple.live_parts(&asset.master).await?;
            Ok(Arc::new(LiveStation {
                apple,
                asset,
                init,
                key_uri,
            }))
        })
        .await
    }
}

/// Told SoundCloud's tokens each time they are renewed, to keep them.
#[uniffi::export(with_foreign)]
pub trait TokenKeeper: Send + Sync {
    fn keep(&self, oauth: String, refresh: Option<String>);
}

/// A browser sending SoundCloud's changes (likes, follows, playlists), as
/// its bot guard takes only a browser's; when the guard asks for a check,
/// it has the user pass it and sends again.
#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait SoundCloudBrowser: Send + Sync {
    /// `method` on `url` with `headers` (the user's token among them) and a
    /// JSON `body`; what came back, or status 0 and why not.
    async fn send(
        &self,
        method: String,
        url: String,
        headers: std::collections::HashMap<String, String>,
        body: Option<String>,
    ) -> SoundCloudAnswer;
}

#[derive(uniffi::Record)]
pub struct SoundCloudAnswer {
    pub status: u16,
    pub body: String,
}

struct ForeignBrowser(Arc<dyn SoundCloudBrowser>);

#[async_trait::async_trait]
impl soundcloud::Browser for ForeignBrowser {
    async fn send(&self, change: soundcloud::Change) -> music_core::Result<soundcloud::Answer> {
        let answer = self
            .0
            .send(
                change.method,
                change.url,
                change.headers.into_iter().collect(),
                change.body,
            )
            .await;
        if answer.status == 0 {
            return Err(music_core::Error::Network(format!(
                "SoundCloud: {}",
                answer.body
            )));
        }
        // A page's script can't read the guard's header: its body says it.
        Ok(soundcloud::Answer {
            status: answer.status,
            guard_header: false,
            body: answer.body,
        })
    }
}

/// A SoundCloud track as Android's player plays it: an HLS playlist or the
/// uploader's file, and an encrypted stream's licences.
#[derive(uniffi::Object)]
pub struct SoundCloudStream {
    sc: Arc<soundcloud::SoundCloud>,
    playable: soundcloud::Playable,
}

#[uniffi::export(async_runtime = "tokio")]
impl SoundCloudStream {
    pub fn url(&self) -> String {
        self.playable.url.clone()
    }

    /// An HLS playlist; else the uploader's file, whole.
    pub fn hls(&self) -> bool {
        self.playable.hls
    }

    pub fn encrypted(&self) -> bool {
        self.playable.license_token.is_some()
    }

    pub fn original(&self) -> bool {
        self.playable.original
    }

    pub fn quality(&self) -> Option<music_core::model::AudioQuality> {
        self.playable.quality.clone()
    }

    /// A Widevine licence for `challenge`.
    pub async fn license(self: Arc<Self>, challenge: Vec<u8>) -> Result<Vec<u8>, MusicError> {
        off(async move {
            let token = self
                .playable
                .license_token
                .clone()
                .ok_or_else(|| MusicError::Failed {
                    reason: "the stream isn't encrypted".into(),
                })?;
            Ok(self.sc.widevine_license(&token, challenge).await?)
        })
        .await
    }
}

/// A station's stream, played by Android's player: its HLS playlist, and
/// the licences for its Widevine keys.
#[derive(uniffi::Object)]
pub struct LiveStation {
    apple: Arc<AppleMusic>,
    asset: apple_music::LiveAsset,
    /// Its init segment, and its key's line, as it started.
    init: Option<String>,
    key_uri: Option<String>,
}

#[uniffi::export(async_runtime = "tokio")]
impl LiveStation {
    pub fn master(&self) -> String {
        self.asset.master.clone()
    }

    /// Its init segment's address, which `fix_init` reads.
    pub fn init_uri(&self) -> Option<String> {
        self.init.clone()
    }

    /// The init segment, with the key id an episode leaves out put in.
    pub fn fix_init(&self, init: Vec<u8>) -> Vec<u8> {
        let mut init = init;
        if let Some(uri) = &self.key_uri {
            apple_music::name_the_key(&mut init, uri);
        }
        init
    }

    /// Whether its segments are encrypted (Apple's own stations are).
    pub fn encrypted(&self) -> bool {
        self.asset.key_server.is_some()
    }

    /// A licence for `challenge`, for the key the stream uses now: the key
    /// server names keys by the playlist's key line, which a challenge
    /// doesn't carry.
    pub async fn license(self: Arc<Self>, challenge: Vec<u8>) -> Result<Vec<u8>, MusicError> {
        off(async move {
            let key_server =
                self.asset
                    .key_server
                    .as_deref()
                    .ok_or_else(|| MusicError::Failed {
                        reason: "the station isn't encrypted".into(),
                    })?;
            let key_uri = self
                .apple
                .live_key_uri(&self.asset.master)
                .await?
                .ok_or_else(|| MusicError::Failed {
                    reason: "the station's playlist names no key".into(),
                })?;
            Ok(self
                .apple
                .live_license(key_server, &key_uri, &challenge)
                .await?)
        })
        .await
    }
}

/// The positions of `songs` to leave out of a list of every service's: the
/// copies of a song another service has a better copy of.
#[uniffi::export]
pub fn songs_left_out(songs: Vec<Track>) -> Vec<u32> {
    sorted(music_core::dedupe::songs(&songs))
}

/// Every service's Recently Added in one list, by when each item was added.
#[uniffi::export]
pub fn recent_in_time(items: Vec<Collection>) -> Vec<Collection> {
    music_core::recent::in_time_order(items)
}

#[uniffi::export]
pub fn albums_left_out(albums: Vec<Album>) -> Vec<u32> {
    sorted(music_core::dedupe::albums(&albums))
}

#[uniffi::export]
pub fn artists_left_out(artists: Vec<Artist>) -> Vec<u32> {
    sorted(music_core::dedupe::artists(&artists))
}

/// The library's lists as last read, kept in the app's files: shown at once
/// at the next launch while the services are asked again. Written aside,
/// then moved over the old one.
fn keep<T: serde::Serialize>(path: &str, items: &[T]) {
    let Ok(bytes) = serde_json::to_vec(items) else {
        return;
    };
    let partial = format!("{path}.part");
    let _ = std::fs::write(&partial, bytes).and_then(|()| std::fs::rename(&partial, path));
}

/// What was kept at `path`: nothing if nothing readable was.
fn kept<T: serde::de::DeserializeOwned>(path: &str) -> Vec<T> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&navidrome::scrub_kept(&text)).ok())
        .unwrap_or_default()
}

/// The account as others see it on Apple Music: its name, its handle
/// (without the `@`) and its picture.
#[derive(uniffi::Record)]
pub struct AppleProfile {
    pub name: Option<String>,
    pub handle: Option<String>,
    pub picture: Option<music_core::model::Artwork>,
}

/// What was opened or played from a search, as Recently Searched lists it.
#[derive(Clone, serde::Serialize, serde::Deserialize, uniffi::Enum)]
pub enum Searched {
    AlbumOpened { album: Album },
    ArtistOpened { artist: Artist },
    SongPlayed { track: Track },
    PlaylistOpened { playlist: Playlist },
}

/// A search's find, and whether it was found in the library.
#[derive(Clone, serde::Serialize, serde::Deserialize, uniffi::Record)]
pub struct RecentlySearched {
    pub item: Searched,
    pub library: bool,
}

#[uniffi::export]
pub fn keep_searched(path: String, items: Vec<RecentlySearched>) {
    keep(&path, &items);
}

#[uniffi::export]
pub fn kept_searched(path: String) -> Vec<RecentlySearched> {
    kept(&path)
}

/// Which of `others` are copies of `album` on other services, by their
/// title and artist: their indices.
#[uniffi::export]
pub fn album_twins(album: Album, others: Vec<Album>) -> Vec<u32> {
    let key = music_core::dedupe::album_key(&album);
    others
        .iter()
        .enumerate()
        .filter(|(_, a)| {
            a.id.provider != album.id.provider && music_core::dedupe::album_key(a) == key
        })
        .map(|(i, _)| i as u32)
        .collect()
}

/// An album whole from its copies: each song from the best copy that has
/// it (the server's first), the others filling in what it lacks.
#[uniffi::export]
pub fn whole_album(copies: Vec<Vec<Track>>) -> Vec<Track> {
    let slices: Vec<&[Track]> = copies.iter().map(Vec::as_slice).collect();
    music_core::dedupe::album_tracks(&slices)
}

/// What played when the app last closed, as kept before the devices shared
/// it, if anything did.
fn kept_session(path: String) -> Option<connect::Handoff> {
    serde_json::from_str(&navidrome::scrub_kept(&std::fs::read_to_string(path).ok()?)).ok()
}

#[uniffi::export]
pub fn keep_tracks(path: String, items: Vec<Track>) {
    keep(&path, &items);
}

#[uniffi::export]
pub fn kept_tracks(path: String) -> Vec<Track> {
    kept(&path)
}

#[uniffi::export]
pub fn keep_albums(path: String, items: Vec<Album>) {
    keep(&path, &items);
}

#[uniffi::export]
pub fn kept_albums(path: String) -> Vec<Album> {
    kept(&path)
}

#[uniffi::export]
pub fn keep_artists(path: String, items: Vec<Artist>) {
    keep(&path, &items);
}

#[uniffi::export]
pub fn kept_artists(path: String) -> Vec<Artist> {
    kept(&path)
}

#[uniffi::export]
pub fn keep_playlists(path: String, items: Vec<Playlist>) {
    keep(&path, &items);
}

#[uniffi::export]
pub fn kept_playlists(path: String) -> Vec<Playlist> {
    kept(&path)
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

/// A queue's songs in their own order, the play order over them, and where
/// in that the song playing is.
#[derive(uniffi::Record)]
pub struct QueueSnapshot {
    pub tracks: Vec<Track>,
    pub order: Vec<u32>,
    pub at: u32,
}

/// Where the music last stopped, and on which device; `device` and `at` say
/// which stop it is.
#[derive(uniffi::Record)]
pub struct LastSession {
    pub handoff: connect::Handoff,
    pub here: bool,
    pub device: String,
    pub at: u64,
}

/// What Music Connect says, told to the app as it comes; from the node's
/// own threads.
#[uniffi::export(with_foreign)]
pub trait ConnectListener: Send + Sync {
    fn heard(&self, event: connect::Event);
}

/// Apple's newest songs as first seen, kept in common with the other
/// devices once Music Connect is there.
struct SeenShared(Arc<RwLock<Option<Arc<Connect>>>>);

impl SeenShared {
    fn connect(&self) -> Option<Arc<Connect>> {
        self.0.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl apple_music::SeenElsewhere for SeenShared {
    fn known(&self, ids: &[String]) -> std::collections::HashMap<String, u64> {
        self.connect().map(|c| c.1.seen(ids)).unwrap_or_default()
    }

    fn seen_here(&self, times: Vec<(String, u64)>) {
        if let Some(c) = self.connect() {
            c.0.share(c.1.first_seen(times));
        }
    }
}

/// This phone on the network, as Music Connect: see the `connect` crate;
/// and what it keeps in common with the other devices: see `synced`.
#[derive(uniffi::Object)]
pub struct Connect(connect::Node, synced::Kept, Arc<dyn ConnectListener>);

impl Connect {
    /// `change` made here: told the devices, and the app as theirs are.
    fn changed(&self, change: synced::Shared) {
        self.0.share(change);
        self.2.heard(connect::Event::Synced);
    }
}

#[uniffi::export]
impl Connect {
    /// Go on the network as `name`, keeping the pairings and what is kept
    /// in common in `dir`, away from home through `relay` (n0's when
    /// `None`); the songs played and where the music stopped as kept before
    /// in `history` and `session` are taken in.
    #[uniffi::constructor]
    pub fn new(
        name: String,
        dir: String,
        history: String,
        session: String,
        relay: Option<String>,
        listener: Arc<dyn ConnectListener>,
    ) -> Arc<Self> {
        let dir = std::path::Path::new(&dir);
        let kept = synced::Kept::open(dir, || self::kept(&history));
        if kept.session().is_none() {
            let at = std::fs::metadata(&session)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
            if let (Some(old), Some(at)) = (kept_session(session), at) {
                kept.take_old_session(old, at.as_millis() as u64);
            }
        }
        let node = connect::Node::start(
            name,
            connect::DeviceKind::Phone,
            dir,
            kept.clone(),
            relay,
            {
                let listener = listener.clone();
                move |event| listener.heard(event)
            },
        );
        Arc::new(Self(node, kept, listener))
    }

    /// The songs played lately, here and on the other devices, the last
    /// first.
    pub fn history(&self) -> Vec<Track> {
        self.1.history()
    }

    /// `track` played here: in the history, and told the devices.
    pub fn played(&self, track: Track) {
        self.0.share(self.1.played(track));
    }

    /// The history cleared, on every device.
    pub fn clear_history(&self) {
        self.0.share(self.1.clear_history());
    }

    /// Where the music last stopped, here or on another device.
    pub fn session(&self) -> Option<LastSession> {
        let me = self.1.me();
        self.1.session().map(|(handoff, stamp)| LastSession {
            handoff,
            here: stamp.device == me,
            device: stamp.device,
            at: stamp.at,
        })
    }

    /// The music stopped here, or moved on, at `session`: told the devices,
    /// and on disk before this returns, the app being maybe on its way out.
    pub fn left(&self, session: connect::Handoff) {
        self.0.share(self.1.left(session));
        self.1.flush();
    }

    /// The settings alike on every device, by name, as text.
    pub fn settings(&self) -> std::collections::HashMap<String, String> {
        self.1.settings().into_iter().collect()
    }

    /// Setting `name` set to `value` here: told the devices.
    pub fn set_setting(&self, name: String, value: String) {
        self.0.share(self.1.set_setting(&name, value));
    }

    /// Setting `name` as set here before the devices shared it: taken if
    /// none is shared yet.
    pub fn offer_setting(&self, name: String, value: String) {
        self.0.share(self.1.offer_setting(&name, value));
    }

    /// The playlists songs were last added to, on any device, by id, the
    /// last first.
    pub fn used_playlists(&self) -> Vec<String> {
        self.1.used()
    }

    /// Songs added now to the playlists `ids`, the first last of all: told
    /// the devices.
    pub fn use_playlists(&self, ids: Vec<String>) {
        self.0.share(self.1.use_playlists(&ids));
    }

    /// Reachable away from home, and reaching the devices away, or not.
    pub fn set_away(&self, on: bool) {
        self.0.set_away(on);
    }

    pub fn me(&self) -> connect::Device {
        self.0.me()
    }

    pub fn peers(&self) -> Vec<connect::Peer> {
        self.0.peers()
    }

    pub fn publish(&self, playing: connect::Playing) {
        self.0.publish(playing);
    }

    pub fn publish_upcoming(&self, upcoming: Vec<Track>) {
        self.0.publish_upcoming(upcoming);
    }

    pub fn send(&self, to: String, command: connect::Command) -> bool {
        self.0.send(&to, command)
    }

    pub fn hand_off(&self, to: String, handoff: connect::Handoff) -> bool {
        self.0.hand_off(&to, handoff)
    }

    pub fn pair(&self, id: String) {
        self.0.pair(&id);
    }

    pub fn enter_code(&self, code: String) {
        self.0.enter_code(code);
    }

    pub fn cancel_pairing(&self) {
        self.0.cancel_pairing();
    }

    pub fn forget(&self, id: String) {
        self.0.forget(&id);
    }
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

    /// The whole queue, for another device to play on from here.
    pub fn snapshot(&self) -> QueueSnapshot {
        self.with(|q| {
            let (tracks, order, at) = q.snapshot();
            QueueSnapshot {
                tracks,
                order: order.into_iter().map(|i| i as u32).collect(),
                at: at as u32,
            }
        })
    }

    /// Take over a queue another device played.
    pub fn restore(&self, tracks: Vec<Track>, order: Vec<u32>, at: u32, shuffle: bool) {
        let order = order.into_iter().map(|i| i as usize).collect();
        self.with(|q| q.restore(tracks, order, at as usize, shuffle));
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
        off(async move {
            Ok(Arc::new(Self(Arc::new(
                AppleMusic::connect(&user_token).await?,
            ))))
        })
        .await
    }

    pub fn storefront(&self) -> String {
        self.0.storefront().to_owned()
    }

    /// A catalog song's encrypted file and its key; `None` for an upload.
    pub async fn protected(
        self: Arc<Self>,
        song: String,
        spatial: bool,
    ) -> Result<Option<Arc<ProtectedTrack>>, MusicError> {
        off(async move {
            let id = ItemId::new(ProviderKind::AppleMusic, song);
            Ok(self
                .0
                .protected(&id, spatial)
                .await?
                .map(|protected| ProtectedTrack::new(self.0.clone(), protected)))
        })
        .await
    }
}

/// An encrypted song, for the player's own CDM to license.
#[derive(uniffi::Object)]
pub struct ProtectedTrack {
    apple: Arc<AppleMusic>,
    protected: Protected,
    /// The padding once read: kept for every asker, one given up on (a
    /// timeout) included.
    padding: tokio::sync::watch::Receiver<Option<u32>>,
}

impl ProtectedTrack {
    fn new(apple: Arc<AppleMusic>, protected: Protected) -> Arc<Self> {
        let (found, padding) = tokio::sync::watch::channel(None);
        {
            let (apple, protected) = (apple.clone(), protected.clone());
            tokio::spawn(async move {
                let _ = found.send(Some(apple.padding(&protected).await));
            });
        }
        Arc::new(Self {
            apple,
            protected,
            padding,
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

    /// Spatial Audio: Apple's binaural rendering of the song's Dolby Atmos
    /// mix, AAC 256 kbps at 48 kHz.
    pub fn spatial(&self) -> bool {
        self.protected.spatial
    }

    /// The init segment the playlist names `init.mp4`, beside it, with its
    /// key ids written in: Spatial Audio's only.
    pub fn init(&self) -> Option<Vec<u8>> {
        self.protected.init.clone()
    }

    /// The frames the encoder put before the music, to leave out.
    pub fn priming(&self) -> u32 {
        apple_music::PRIMING
    }

    /// The frames after the music in the last frame, to leave out: read
    /// in the background from the start, as the song needn't wait for it.
    pub async fn padding(&self) -> u32 {
        // Its work runs in a task of its own already.
        let mut padding = self.padding.clone();
        match padding.wait_for(Option::is_some).await {
            Ok(found) => found.unwrap_or(0),
            Err(_) => 0,
        }
    }

    /// The encode's real bit rate when it is less than the usual 256 kbps:
    /// `None` for the usual.
    pub async fn lesser_kbps(self: Arc<Self>) -> Option<u32> {
        off(async move { Ok(self.apple.lesser_kbps(&self.protected).await) })
            .await
            .unwrap_or(None)
    }

    /// The licence, in answer to the CDM's challenge.
    pub async fn license(self: Arc<Self>, challenge: Vec<u8>) -> Result<Vec<u8>, MusicError> {
        off(async move { Ok(self.apple.license(&self.protected, &challenge).await?) }).await
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

/// A theme the app comes with.
#[derive(uniffi::Record)]
pub struct BuiltInTheme {
    pub id: String,
    pub file: music_core::theme::ThemeFile,
}

/// The themes that come with the app: Stock, then Spotify. M3E is the
/// desktop's alone until the phone's is designed.
#[uniffi::export]
pub fn built_in_themes() -> Vec<BuiltInTheme> {
    music_core::theme::BUILT_IN
        .iter()
        .filter(|(id, _)| *id != "m3e")
        .filter_map(|(id, _)| {
            Some(BuiltInTheme {
                id: id.to_string(),
                file: music_core::theme::built_in(id)?,
            })
        })
        .collect()
}

/// A theme file imported or sent from the desktop, read; what is wrong with
/// it if it can't be.
#[uniffi::export]
pub fn parse_theme(source: String) -> Result<music_core::theme::ThemeFile, MusicError> {
    music_core::theme::parse(&source).map_err(|reason| MusicError::Failed { reason })
}
