//! Provider and network work on a tokio runtime, off the UI thread. The UI sends
//! [`Request`]s and picks up [`Reply`]s each frame; every reply wakes it up.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;

use apple_music::{AppleMusic, AudioStream, LiveStream};
use music_core::model::{
    Album, Artist, ArtistDetail, Collection, Cursor, FavouriteKind, ItemId, Lyrics, Playlist,
    ProviderKind, SearchResults, Shelf, Track,
};
use music_core::{Error, Provider};
use navidrome::Navidrome;
use serde::Serialize;
use serde::de::DeserializeOwned;
use services::Services;
use soundcloud::SoundCloud;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::app::SearchScope;
use crate::cache;

mod downloading;
pub use downloading::{Downloading, KeptSong};

pub enum Request {
    /// The library songs and playlists, page by page.
    Library,
    /// The library playlists again, once one was added.
    Playlists,
    /// Which catalog albums or playlists are wholly in the library.
    InLibrary {
        kind: FavouriteKind,
        ids: Vec<ItemId>,
    },
    /// The library albums, page by page.
    LibraryAlbums,
    /// The library artists, page by page.
    LibraryArtists,
    /// A library artist's albums in the library.
    LibraryArtistAlbums(ItemId),
    /// A catalog album's other editions and music videos.
    AlbumExtras(ItemId),
    /// What was added to the library lately.
    RecentlyAdded,
    /// The home page's shelves.
    Home,
    /// The radio page's shelves.
    Radio,
    /// The new music page's shelves.
    NewMusic,
    /// The next songs of a station of songs.
    StationTracks(ItemId),
    /// Sign in to Apple Music in the browser, then connect with the new token.
    SignIn,
    /// Connect again with the saved token (after a sign-in).
    Reconnect,
    /// Turn a service on, connecting and loading its library, or off,
    /// leaving it signed in but unused.
    SetService {
        provider: ProviderKind,
        on: bool,
    },
    /// Sign in to SoundCloud in the browser, then connect and load its library.
    SignInSoundCloud,
    ReconnectSoundCloud,
    /// Connect to the Navidrome server again, after it was out of reach.
    ReconnectNavidrome,
    /// Sign out of a service: its sign-in, its browser profile and what was
    /// kept of its library go from the disk.
    SignOut(ProviderKind),
    /// Sign in to a Navidrome server; the password makes the token kept,
    /// and goes no further.
    SignInNavidrome {
        url: String,
        user: String,
        password: String,
    },
    /// Add to the favourites, or take out.
    SetFavourite {
        kind: FavouriteKind,
        id: ItemId,
        on: bool,
    },
    /// Add a song, an album or a playlist to the library.
    /// `quiet`: say nothing unless it fails (it goes with another change).
    AddToLibrary {
        kind: FavouriteKind,
        id: ItemId,
        quiet: bool,
    },
    /// Where a song leads, to go there: its artist's page or its station.
    SongLinks {
        song: ItemId,
        radio: bool,
    },
    /// Take a song out of the library, by its library id.
    RemoveFromLibrary(ItemId),
    /// Make a playlist with these songs, in `service`.
    CreatePlaylist {
        service: ProviderKind,
        name: String,
        description: String,
        tracks: Vec<ItemId>,
    },
    /// Make one of Music's own playlists, of songs from any service.
    CreateMusicPlaylist {
        name: String,
        description: String,
        tracks: Vec<Track>,
    },
    /// Add songs to one of Music's own playlists.
    AddToMusicPlaylist {
        playlist: ItemId,
        tracks: Vec<Track>,
    },
    /// Change a playlist's name and description.
    EditPlaylist {
        id: ItemId,
        name: String,
        description: String,
    },
    DeletePlaylist(ItemId),
    /// Take a song out of a playlist, by its row's id there.
    RemoveFromPlaylist {
        playlist: ItemId,
        entry: ItemId,
    },
    /// Add songs to playlists.
    AddToPlaylists {
        playlists: Vec<ItemId>,
        tracks: Vec<ItemId>,
    },
    /// Whether something is a favourite.
    Favourite {
        kind: FavouriteKind,
        id: ItemId,
    },
    /// Ask for less like this, or take that back.
    SuggestLess {
        kind: FavouriteKind,
        id: ItemId,
        on: bool,
    },
    /// Whether less like this was asked for.
    SuggestsLess {
        kind: FavouriteKind,
        id: ItemId,
    },
    /// Its page on the service's website, to copy.
    ShareLink {
        kind: FavouriteKind,
        id: ItemId,
    },
    /// A track's audio, ready to hand to the player.
    Fetch {
        token: player::Token,
        track: ItemId,
        /// The library's copy, played if the catalog's can't be.
        fallback: Option<ItemId>,
        /// A SoundCloud uploader's original file, when they allow it.
        original: bool,
        /// Apple's Spatial Audio, for a song mixed in Dolby Atmos.
        spatial: bool,
    },
    /// An animated cover's video at `size` pixels, kept on disk.
    Motion {
        url: String,
        size: u32,
    },
    /// Fetch a track the pointer rests on, in case it is played: as `Fetch`,
    /// but a newer one calls off the last.
    Warm {
        token: player::Token,
        track: ItemId,
        fallback: Option<ItemId>,
        original: bool,
        spatial: bool,
    },
    /// Download songs to play offline; `original`: a SoundCloud
    /// uploader's own file, where they allow it.
    Download {
        tracks: Vec<Track>,
        original: bool,
    },
    /// Downloads no longer kept, or no longer to come.
    RemoveDownloads(Vec<ItemId>),
    /// Every service's library songs as they were added, the last first.
    NewestSongs,
    /// An album and its tracks.
    Album(ItemId),
    /// An artist's top songs and releases.
    Artist(ItemId),
    /// A playlist's tracks, page by page.
    Playlist(ItemId),
    /// The three colours of the cover at this URL.
    Palette(String),
    /// A track's lyrics, from its service.
    Lyrics(ItemId),
    /// A track's lyrics from the lyrics sites, asked in this order.
    SiteLyrics {
        track: Box<Track>,
        sources: Vec<lyrics::Source>,
    },
    /// Songs, albums, artists and playlists matching a query, in the
    /// catalog or in the library.
    /// A search, of every service or only the one given.
    Search(SearchScope, Option<ProviderKind>, String),
}

pub enum Reply {
    /// Every song kept to play offline.
    Downloads(Vec<KeptSong>),
    /// How a song's download goes.
    Download {
        id: ItemId,
        state: Downloading,
    },
    /// Connected to Apple Music, in this storefront (country).
    Connected {
        storefront: String,
    },
    /// The account's picture on Apple Music, once read.
    ProfilePicture(music_core::model::Artwork),
    /// The browser sign-in ended: the token is saved, or why not.
    SignedIn(Result<(), String>),
    /// SoundCloud is connected as this user, or isn't signed in (`None`),
    /// or failed to connect.
    SoundCloud(Result<Option<String>, String>),
    SignedInSoundCloud(Result<(), String>),
    /// Navidrome is connected as this user, or isn't signed in (`None`), or
    /// failed to connect.
    Navidrome(Result<Option<String>, String>),
    SignedInNavidrome(Result<(), String>),
    /// What the user starred, on a service that says so in a list.
    Starred(Vec<ItemId>),
    /// A song's links, asked for its artist (`radio: false`) or its station.
    SongLinks {
        radio: bool,
        result: Result<music_core::model::SongLinks, String>,
    },
    /// A playlist was made, or why not.
    PlaylistCreated(Result<Playlist, String>),
    PlaylistEdited {
        id: ItemId,
        name: String,
        description: String,
        result: Result<(), String>,
    },
    RemovedFromPlaylist {
        playlist: ItemId,
        result: Result<(), String>,
    },
    PlaylistDeleted {
        id: ItemId,
        result: Result<(), String>,
    },
    /// A change to the library went through (the message to show), or why not.
    Changed(Result<String, String>),
    /// A song left the library (the message to show), or why not: then the
    /// library, changed at once, is loaded again as it is.
    RemovedFromLibrary(Result<String, String>),
    /// Whether catalog albums or playlists are in the library; those Apple
    /// didn't answer for are left out.
    InLibrary(Vec<(ItemId, bool)>),
    /// Where an animated cover's video is kept, or why it couldn't be.
    Motion {
        url: String,
        size: u32,
        result: Result<PathBuf, String>,
    },
    /// Something was added to the library, or why not.
    AddedToLibrary {
        kind: FavouriteKind,
        id: ItemId,
        quiet: bool,
        result: Result<(), String>,
    },
    /// A favourite was set (`set: Some(on)`) or looked up; the state or why not.
    Favourite {
        id: ItemId,
        set: Option<bool>,
        result: Result<Option<bool>, String>,
    },
    /// Suggest Less was set (`set: Some(on)`) or looked up; as `Favourite`.
    SuggestsLess {
        id: ItemId,
        set: Option<bool>,
        result: Result<Option<bool>, String>,
    },
    /// The link to copy, none if there is none, or why not.
    ShareLink(Result<Option<String>, String>),
    /// A service's library songs, the last added first, all of them.
    NewestSongs {
        provider: ProviderKind,
        ids: Vec<ItemId>,
    },
    NotSignedIn,
    Failed(String),
    /// Apple Music couldn't be reached.
    CouldntConnect(String),
    /// A page of a service's songs, or with `whole` all of them, to replace
    /// those of it shown: the list kept from last time, or the fresh one
    /// that follows it.
    Songs {
        provider: ProviderKind,
        tracks: Vec<Track>,
        done: bool,
        whole: bool,
    },
    /// A service's playlists, all of them.
    Playlists {
        provider: ProviderKind,
        playlists: Vec<Playlist>,
    },
    /// A service's albums or artists, as its songs come.
    LibraryAlbums {
        provider: ProviderKind,
        albums: Vec<Album>,
        done: bool,
        whole: bool,
    },
    LibraryArtists {
        provider: ProviderKind,
        artists: Vec<Artist>,
        done: bool,
        whole: bool,
    },
    LibraryArtistAlbums {
        id: ItemId,
        result: Result<Vec<Album>, String>,
    },
    AlbumExtras {
        id: ItemId,
        result: Result<Arc<music_core::model::AlbumExtras>, String>,
    },
    RecentlyAdded {
        provider: ProviderKind,
        items: Vec<Collection>,
        done: bool,
    },
    Home(Result<Vec<Shelf>, String>),
    Fetched {
        token: player::Token,
        result: Result<Playable, String>,
    },
    StationTracks {
        id: ItemId,
        result: Result<Vec<Track>, String>,
    },
    Radio(Result<Vec<Shelf>, String>),
    NewMusic(Result<Vec<Shelf>, String>),
    Album {
        id: ItemId,
        result: Result<Box<(Album, Vec<Track>)>, String>,
    },
    Artist {
        id: ItemId,
        result: Result<Arc<ArtistDetail>, String>,
    },
    PlaylistTracks {
        id: ItemId,
        result: Result<Vec<Track>, String>,
        done: bool,
        whole: bool,
    },
    Palette {
        url: String,
        palette: Option<crate::palette::Palette>,
    },
    Lyrics {
        id: ItemId,
        result: Result<Option<Lyrics>, String>,
    },
    /// What the lyrics sites had for a track, and which one.
    SiteLyrics {
        id: ItemId,
        found: Option<(Lyrics, lyrics::Source)>,
    },
    Search {
        scope: SearchScope,
        source: Option<ProviderKind>,
        query: String,
        result: Result<SearchResults, String>,
    },
}

/// What a fetch hands the player: a track, a station's stream, or a whole
/// file in memory.
pub enum Playable {
    /// An Apple Music track, the bit rate of its encode when known, and
    /// whether it is Spatial Audio.
    Track {
        stream: AudioStream,
        kbps: Option<u32>,
        spatial: bool,
    },
    /// A SoundCloud track, in memory or still downloading.
    SoundCloud(soundcloud::Audio),
    /// A Navidrome file, as it is on the server, still downloading.
    Navidrome(navidrome::Audio),
    Live(LiveStream),
    /// A song downloaded, played from the disk.
    Stored(Box<downloads::store::Open>),
}

pub struct Backend {
    requests: UnboundedSender<Request>,
    replies: mpsc::Receiver<Reply>,
    own: Own,
}

/// What Music keeps of its own, its playlists, answered at once rather
/// than by a service, and shared with the other devices as it changes.
pub struct Own {
    pub kept: synced::Kept,
    pub share: connect::Sharer,
    replies: Option<Replies>,
}

impl Own {
    pub fn new(kept: synced::Kept, share: connect::Sharer) -> Self {
        Self {
            kept,
            share,
            replies: None,
        }
    }

    fn reply(&self, reply: Reply) {
        if let Some(replies) = &self.replies {
            replies.send(reply);
        }
    }

    /// The playlists, and the songs of `loaded`, as they are now.
    fn tell(&self, loaded: &[ItemId]) {
        let playlists = self.kept.playlists();
        self.reply(Reply::Playlists {
            provider: ProviderKind::Music,
            playlists: playlists.iter().map(synced::Playlist::model).collect(),
        });
        for id in loaded {
            self.reply(Reply::PlaylistTracks {
                id: id.clone(),
                result: match playlists.iter().find(|p| p.id == id.id) {
                    Some(p) => Ok(p.tracks()),
                    None => Err("This playlist was deleted".to_owned()),
                },
                done: true,
                whole: true,
            });
        }
    }

    fn share(&self, change: synced::Shared) {
        self.share.share(change);
    }

    /// Answer `request` if it is about Music's own; hand it back if it is a
    /// service's.
    fn answer(&self, request: Request) -> Option<Request> {
        let music = |id: &ItemId| id.provider == ProviderKind::Music;
        match request {
            Request::Library | Request::Playlists => {
                self.tell(&[]);
                Some(request)
            }
            Request::Playlist(id) if music(&id) => {
                self.tell(&[id]);
                None
            }
            Request::CreateMusicPlaylist {
                name,
                description,
                tracks,
            } => {
                let (id, change) = self.kept.create_playlist(name, description, tracks);
                self.share(change);
                if let Some(p) = self.kept.playlist(&id) {
                    self.reply(Reply::PlaylistCreated(Ok(p.model())));
                }
                self.tell(&[]);
                None
            }
            Request::AddToMusicPlaylist { playlist, tracks } => {
                self.share(self.kept.add_to_playlist(&playlist.id, tracks));
                self.tell(&[playlist]);
                None
            }
            Request::EditPlaylist {
                id,
                name,
                description,
            } if music(&id) => {
                self.share(
                    self.kept
                        .edit_playlist(&id.id, name.clone(), description.clone()),
                );
                self.reply(Reply::PlaylistEdited {
                    id,
                    name,
                    description,
                    result: Ok(()),
                });
                None
            }
            Request::DeletePlaylist(id) if music(&id) => {
                self.share(self.kept.delete_playlist(&id.id));
                self.reply(Reply::PlaylistDeleted { id, result: Ok(()) });
                self.tell(&[]);
                None
            }
            // Every entry of the song, as a service's playlist does.
            Request::RemoveFromPlaylist { playlist, entry } if music(&playlist) => {
                let entries = self
                    .kept
                    .playlist(&playlist.id)
                    .map(|p| p.entries_of(&entry))
                    .unwrap_or_default();
                self.share(self.kept.remove_from_playlist(&playlist.id, &entries));
                self.reply(Reply::RemovedFromPlaylist {
                    playlist: playlist.clone(),
                    result: Ok(()),
                });
                self.tell(&[playlist]);
                None
            }
            Request::AddToPlaylists { playlists, tracks } => {
                let playlists: Vec<ItemId> = playlists.into_iter().filter(|p| !music(p)).collect();
                (!playlists.is_empty()).then_some(Request::AddToPlaylists { playlists, tracks })
            }
            // Nothing else is Music's to answer, nor a service's.
            request if target(&request) == Some(ProviderKind::Music) => None,
            request => Some(request),
        }
    }
}

/// Where the Apple Music sign-in keeps its token (see the `am` example).
pub fn apple_music_dir() -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_default();
    config.join("music/apple-music")
}

/// Where the SoundCloud sign-in keeps its tokens (see the `sc` example).
fn soundcloud_dir() -> PathBuf {
    apple_music_dir().with_file_name("soundcloud")
}

/// A directory only the user can open, made if missing.
fn private_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
}

/// Write a secret readable by the user only: aside, then moved over, so
/// that a crash midway leaves the last one whole.
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    if let Some(dir) = path.parent() {
        private_dir(dir)?;
    }
    let partial = path.with_extension("part");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&partial)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    std::fs::rename(&partial, path)
}

/// Keep SoundCloud's tokens, readable by the user only.
fn keep_soundcloud_tokens(tokens: &soundcloud::Tokens) {
    let json = serde_json::json!({ "oauth": tokens.oauth, "refresh": tokens.refresh });
    let _ = write_private(
        &soundcloud_dir().join("tokens.json"),
        json.to_string().as_bytes(),
    );
}

fn kept_soundcloud_tokens() -> Option<soundcloud::Tokens> {
    let text = std::fs::read_to_string(soundcloud_dir().join("tokens.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    Some(soundcloud::Tokens {
        oauth: json["oauth"].as_str()?.to_owned(),
        refresh: json["refresh"].as_str().map(str::to_owned),
    })
}

/// Where the Navidrome sign-in keeps the server, the user and the token.
fn navidrome_file() -> PathBuf {
    apple_music_dir()
        .with_file_name("navidrome")
        .join("credentials.json")
}

/// Keep the server's sign-in, readable by the user only.
fn keep_navidrome(credentials: &navidrome::Credentials) -> Result<(), String> {
    let json = serde_json::to_string(credentials).map_err(|e| e.to_string())?;
    write_private(&navidrome_file(), json.as_bytes()).map_err(|e| e.to_string())
}

/// What a sign-in left on the disk, gone: the tokens, the browser profile
/// (its cookies), and the library kept from the account.
fn forget_sign_in(provider: ProviderKind) {
    let gone = |path: PathBuf| {
        let _ = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
    };
    match provider {
        ProviderKind::AppleMusic => {
            gone(apple_music_dir().join("media-user-token"));
            gone(apple_music_dir().join("browser-profile"));
        }
        ProviderKind::SoundCloud => {
            gone(soundcloud_dir().join("tokens.json"));
            gone(soundcloud_dir().join("datadome"));
            gone(soundcloud_dir().join("browser-profile"));
        }
        ProviderKind::Navidrome => gone(navidrome_file()),
        ProviderKind::Music => {}
    }
    cache::clear();
}

/// Connect to the Navidrome server kept, telling the UI how it went.
async fn connect_navidrome(replies: &Replies) -> Option<Arc<Navidrome>> {
    let kept = std::fs::read_to_string(navidrome_file())
        .ok()
        .and_then(|text| serde_json::from_str::<navidrome::Credentials>(&text).ok());
    let Some(credentials) = kept else {
        replies.send(Reply::Navidrome(Ok(None)));
        return None;
    };
    match Navidrome::connect(credentials).await {
        Ok(nd) => {
            replies.send(Reply::Navidrome(Ok(Some(nd.user().to_owned()))));
            Some(Arc::new(nd))
        }
        Err(Error::NotSignedIn) => {
            replies.send(Reply::Navidrome(Ok(None)));
            None
        }
        Err(e) => {
            replies.send(Reply::Navidrome(Err(e.to_string())));
            None
        }
    }
}

/// Connect to SoundCloud with the kept tokens, telling the UI how it went.
async fn connect_soundcloud(replies: &Replies) -> Option<Arc<SoundCloud>> {
    let Some(tokens) = kept_soundcloud_tokens() else {
        replies.send(Reply::SoundCloud(Ok(None)));
        return None;
    };
    let notify = replies.clone();
    let browser = crate::soundcloud_browser::ChromeLike::new(
        sign_in_browser(),
        soundcloud_dir().join("browser-profile"),
        soundcloud_dir().join("datadome"),
        Box::new(move |message| notify.send(Reply::Changed(Ok(message)))),
    )
    .ok()
    .map(|b| Arc::new(b) as Arc<dyn soundcloud::Browser>);
    match SoundCloud::connect(tokens, Box::new(keep_soundcloud_tokens), browser).await {
        Ok(sc) => {
            replies.send(Reply::SoundCloud(Ok(Some(sc.username().to_owned()))));
            Some(Arc::new(sc))
        }
        Err(Error::NotSignedIn) => {
            replies.send(Reply::SoundCloud(Ok(None)));
            None
        }
        Err(e) => {
            replies.send(Reply::SoundCloud(Err(e.to_string())));
            None
        }
    }
}

/// Open the browser on SoundCloud's sign-in page, wait for the user to sign
/// in and close it, and keep the tokens.
async fn sign_in_soundcloud() -> Result<(), String> {
    let profile = soundcloud_dir().join("browser-profile");
    private_dir(&profile).map_err(|e| e.to_string())?;
    let browser = sign_in_browser();
    let tokens = soundcloud::signin::sign_in(
        browser.as_ref(),
        &profile,
        std::time::Duration::from_secs(600),
    )
    .await
    .map_err(|e| e.to_string())?;
    keep_soundcloud_tokens(&tokens);
    Ok(())
}

#[derive(Clone)]
struct Replies {
    tx: mpsc::Sender<Reply>,
    ctx: egui::Context,
}

impl Replies {
    fn send(&self, reply: Reply) {
        let _ = self.tx.send(reply);
        self.ctx.request_repaint();
    }
}

impl Backend {
    /// Start, connecting to every service but those `off`.
    pub fn start(ctx: egui::Context, off: Vec<ProviderKind>, mut own: Own) -> Self {
        // What the app keeps, sign-ins and browser profiles among it, for the
        // user's eyes only, folders made before included.
        for dir in [
            crate::settings::dir(),
            apple_music_dir(),
            apple_music_dir().join("browser-profile"),
            soundcloud_dir(),
            soundcloud_dir().join("browser-profile"),
        ] {
            if dir.exists() {
                let _ = private_dir(&dir);
            }
        }
        let (requests, request_rx) = tokio::sync::mpsc::unbounded_channel();
        let again = requests.clone();
        let seen: Arc<dyn apple_music::SeenElsewhere> =
            Arc::new(SeenShared(own.kept.clone(), own.share.clone()));
        let (tx, replies) = mpsc::channel();
        own.replies = Some(Replies {
            tx: tx.clone(),
            ctx: ctx.clone(),
        });
        let replies_out = Replies { tx, ctx };
        std::thread::Builder::new()
            .name("backend".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("start the tokio runtime");
                runtime.block_on(run(request_rx, again, replies_out, off, seen));
            })
            .expect("spawn the backend thread");
        Self {
            requests,
            replies,
            own,
        }
    }

    pub fn send(&self, request: Request) {
        if let Some(request) = self.own.answer(request) {
            let _ = self.requests.send(request);
        }
    }

    /// Music's own playlists changed on another device: what shows of
    /// them, and the songs of those `loaded`, again.
    pub fn own_changed(&self, loaded: &[ItemId]) {
        self.own.tell(loaded);
    }

    pub fn try_recv(&self) -> Option<Reply> {
        self.replies.try_recv().ok()
    }
}

/// The browser the sign-in opens: `MUSIC_BROWSER`, else the first
/// Chromium-family browser on the `PATH`.
fn sign_in_browser() -> std::ffi::OsString {
    const BROWSERS: [&str; 9] = [
        "brave-origin",
        "brave",
        "brave-browser",
        "chromium",
        "chromium-browser",
        "google-chrome-stable",
        "google-chrome",
        "vivaldi",
        "microsoft-edge",
    ];
    if let Some(browser) = std::env::var_os("MUSIC_BROWSER") {
        return browser;
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    BROWSERS
        .iter()
        .find(|b| std::env::split_paths(&path).any(|dir| dir.join(b).is_file()))
        .unwrap_or(&"chromium")
        .into()
}

/// Connect with the saved token, telling the UI how it went.
/// Apple's newest songs as first seen, kept in common with the other devices.
struct SeenShared(synced::Kept, connect::Sharer);

impl apple_music::SeenElsewhere for SeenShared {
    fn known(&self, ids: &[String]) -> std::collections::HashMap<String, u64> {
        self.0.seen(ids)
    }

    fn seen_here(&self, times: Vec<(String, u64)>) {
        self.1.share(self.0.first_seen(times));
    }
}

async fn connect(
    replies: &Replies,
    seen: Arc<dyn apple_music::SeenElsewhere>,
) -> Option<Arc<AppleMusic>> {
    let Ok(token) = std::fs::read_to_string(apple_music_dir().join("media-user-token")) else {
        replies.send(Reply::NotSignedIn);
        return None;
    };
    match AppleMusic::connect(token.trim()).await {
        Ok(am) => {
            am.keep_seen_in(crate::settings::dir().join("apple-seen.json"), Some(seen));
            replies.send(Reply::Connected {
                storefront: am.storefront().to_owned(),
            });
            let am = Arc::new(am);
            let (picture_am, picture_replies) = (am.clone(), replies.clone());
            tokio::spawn(async move {
                if let Ok(Some(profile)) = picture_am.profile().await
                    && let Some(picture) = profile.picture
                {
                    picture_replies.send(Reply::ProfilePicture(picture));
                }
            });
            Some(am)
        }
        Err(Error::NotSignedIn) => {
            replies.send(Reply::NotSignedIn);
            None
        }
        Err(e) => {
            replies.send(Reply::CouldntConnect(e.to_string()));
            None
        }
    }
}

/// Open the browser on Apple's sign-in page, wait for the user, and save the
/// token readable by the user only, where the `am` example keeps it too.
async fn sign_in() -> Result<(), String> {
    let dir = apple_music_dir();
    let profile = dir.join("browser-profile");
    private_dir(&profile).map_err(|e| e.to_string())?;
    let browser = sign_in_browser();
    let token = apple_music::signin::sign_in(
        browser.as_ref(),
        &profile,
        std::time::Duration::from_secs(600),
    )
    .await
    .map_err(|e| e.to_string())?;
    write_private(&dir.join("media-user-token"), token.as_bytes()).map_err(|e| e.to_string())
}

/// The server's covers signed by `nd` as they are fetched, or by none.
fn sign_covers_with(nd: &Option<Arc<Navidrome>>) {
    crate::covers::set_signer(nd.clone().map(|nd| {
        Arc::new(move |url: &str| nd.sign(url)) as Arc<dyn Fn(&str) -> Option<String> + Send + Sync>
    }));
}

/// A connection made while requests went on.
/// With whether its library then loads (once the library was asked for).
enum Arrived {
    Apple(Option<Arc<AppleMusic>>, bool),
    SoundCloud(Option<Arc<SoundCloud>>, bool),
    Navidrome(Option<Arc<Navidrome>>, bool),
}

async fn run(
    mut requests: UnboundedReceiver<Request>,
    again: UnboundedSender<Request>,
    replies: Replies,
    off: Vec<ProviderKind>,
    seen: Arc<dyn apple_music::SeenElsewhere>,
) {
    let on = |p| !off.contains(&p);
    // Apple Music and SoundCloud at once, before anything is asked of them.
    let (mut apple, mut sc) = tokio::join!(
        async {
            match on(ProviderKind::AppleMusic) {
                true => connect(&replies, seen.clone()).await,
                false => None,
            }
        },
        async {
            match on(ProviderKind::SoundCloud) {
                true => connect_soundcloud(&replies).await,
                false => None,
            }
        },
    );
    // A connection made later (the server, often out of reach away from
    // home; one turned on, or tried again) comes back here, so that it
    // holds up nothing meanwhile.
    let (arrived_tx, mut arrived) = tokio::sync::mpsc::unbounded_channel::<Arrived>();
    let mut nd = None;
    if on(ProviderKind::Navidrome) {
        let (arrived_tx, replies) = (arrived_tx.clone(), replies.clone());
        tokio::spawn(async move {
            let _ = arrived_tx.send(Arrived::Navidrome(connect_navidrome(&replies).await, true));
        });
    }
    // Asked for once: a service connected afterwards loads its library.
    let mut library_wanted = false;
    // Recently Added was asked for: SoundCloud answers once connected.
    let mut recent_wanted = false;
    // Turned off meanwhile: a connection made before is dropped as it comes.
    let mut turned_off: std::collections::HashSet<ProviderKind> = off.iter().copied().collect();
    // Signed out of meanwhile, until signed in again.
    let mut signed_out = std::collections::HashSet::new();
    // Out of reach, their libraries shown as kept: once, not at each try.
    let mut kept_shown = std::collections::HashSet::new();
    let cdm = widevine::locate()
        .ok_or_else(|| "no Widevine module found: install Brave, Chrome or Firefox".to_owned())
        .and_then(|path| widevine::Cdm::open(&path).map_err(|e| e.to_string()));

    let http = reqwest::Client::new();
    let finder = Arc::new(lyrics::Finder::new());
    let mut loading = Loading::default();
    let mut warming: Option<tokio::task::JoinHandle<()>> = None;
    let downloads = downloading::Downloads::open(replies.clone());
    if let Some(am) = apple.clone() {
        confirm_subscription(am, downloads.clone(), replies.clone());
    }
    if sc.is_some() {
        downloads.subscription(ProviderKind::SoundCloud, true, &replies);
    }
    loop {
        // What the next download fetches with.
        downloads.services(downloading::Services {
            apple: apple.clone(),
            soundcloud: sc.clone(),
            navidrome: nd.clone(),
            cdm: cdm.clone().ok(),
        });
        let request = tokio::select! {
            Some(came) = arrived.recv() => {
                let replies = replies.clone();
                let provider = match &came {
                    Arrived::Apple(..) => ProviderKind::AppleMusic,
                    Arrived::SoundCloud(..) => ProviderKind::SoundCloud,
                    Arrived::Navidrome(..) => ProviderKind::Navidrome,
                };
                if turned_off.contains(&provider) || signed_out.contains(&provider) {
                    continue;
                }
                match came {
                    Arrived::Apple(am, load) => {
                        apple = am;
                        if let Some(am) = apple.clone() {
                            kept_shown.remove(&provider);
                            confirm_subscription(am, downloads.clone(), replies.clone());
                        }
                        if let Some(am) = apple.clone().filter(|_| load && library_wanted) {
                            loading.start(ProviderKind::AppleMusic, load_library(am, replies));
                        }
                    }
                    Arrived::SoundCloud(c, load) => {
                        sc = c;
                        // Signed in still: its songs play on.
                        if sc.is_some() {
                            kept_shown.remove(&provider);
                            downloads.subscription(ProviderKind::SoundCloud, true, &replies);
                        }
                        if let Some(sc) = sc.clone().filter(|_| load && recent_wanted) {
                            tokio::spawn(soundcloud_recent(sc, replies.clone()));
                        }
                        if let Some(sc) = sc.clone().filter(|_| load && library_wanted) {
                            loading.start(
                                ProviderKind::SoundCloud,
                                load_soundcloud_library(sc, replies),
                            );
                        }
                    }
                    Arrived::Navidrome(server, load) => {
                        nd = server;
                        sign_covers_with(&nd);
                        if nd.is_some() {
                            kept_shown.remove(&provider);
                        } else if library_wanted && signed_in(provider) && kept_shown.insert(provider) {
                            show_kept(provider, replies.clone());
                        }
                        if let Some(nd) = nd.clone().filter(|_| load && library_wanted) {
                            loading.start(ProviderKind::Navidrome, load_navidrome_library(nd, replies));
                        }
                    }
                }
                continue;
            }
            request = requests.recv() => match request {
                Some(request) => request,
                None => break,
            },
        };
        let replies = replies.clone();
        // Apple signed in and on, but out of reach: its lists as kept.
        let apple_kept = !turned_off.contains(&ProviderKind::AppleMusic)
            && !signed_out.contains(&ProviderKind::AppleMusic)
            && signed_in(ProviderKind::AppleMusic);
        // SoundCloud's likes, beside the other services' Recently Added.
        if matches!(request, Request::RecentlyAdded) {
            recent_wanted = true;
            if let Some(sc) = sc.clone() {
                tokio::spawn(soundcloud_recent(sc, replies.clone()));
            }
        }
        // A song downloaded plays from the disk, whatever is connected.
        if let Request::Fetch {
            token,
            track,
            fallback,
            ..
        }
        | Request::Warm {
            token,
            track,
            fallback,
            ..
        } = &request
            && let Some(playable) = std::iter::once(track)
                .chain(fallback)
                .find_map(|id| downloads.playable(id))
        {
            if matches!(request, Request::Warm { .. })
                && let Some(last) = warming.take()
            {
                last.abort();
            }
            replies.send(Reply::Fetched {
                token: *token,
                result: Ok(playable),
            });
            continue;
        }
        match request {
            Request::Download { tracks, original } => {
                downloads.add(tracks, original, &replies);
                continue;
            }
            Request::RemoveDownloads(ids) => {
                downloads.remove(ids, &replies);
                continue;
            }
            Request::SignIn => {
                let again = again.clone();
                tokio::spawn(async move {
                    let result = sign_in().await;
                    let ok = result.is_ok();
                    replies.send(Reply::SignedIn(result));
                    if ok {
                        // The library kept may be another account's.
                        cache::clear();
                        let _ = again.send(Request::Reconnect);
                    }
                });
                continue;
            }
            Request::Reconnect => {
                signed_out.remove(&ProviderKind::AppleMusic);
                let arrived_tx = arrived_tx.clone();
                let seen = seen.clone();
                tokio::spawn(async move {
                    let _ = arrived_tx.send(Arrived::Apple(connect(&replies, seen).await, false));
                });
                continue;
            }
            Request::SetService { provider, on } => {
                if on {
                    turned_off.remove(&provider);
                } else {
                    turned_off.insert(provider);
                }
                match (provider, on) {
                    (ProviderKind::AppleMusic, false) => {
                        apple = None;
                        loading.stop(provider);
                    }
                    (ProviderKind::SoundCloud, false) => {
                        sc = None;
                        loading.stop(provider);
                    }
                    (ProviderKind::Navidrome, false) => {
                        nd = None;
                        sign_covers_with(&nd);
                        loading.stop(provider);
                    }
                    (ProviderKind::Music, _) => {}
                    // Turned on: its library loads once it is connected.
                    (ProviderKind::AppleMusic, true) => {
                        let arrived_tx = arrived_tx.clone();
                        let seen = seen.clone();
                        tokio::spawn(async move {
                            let _ = arrived_tx
                                .send(Arrived::Apple(connect(&replies, seen).await, true));
                        });
                    }
                    (ProviderKind::SoundCloud, true) => {
                        let arrived_tx = arrived_tx.clone();
                        tokio::spawn(async move {
                            let _ = arrived_tx.send(Arrived::SoundCloud(
                                connect_soundcloud(&replies).await,
                                true,
                            ));
                        });
                    }
                    (ProviderKind::Navidrome, true) => {
                        let arrived_tx = arrived_tx.clone();
                        tokio::spawn(async move {
                            let _ = arrived_tx
                                .send(Arrived::Navidrome(connect_navidrome(&replies).await, true));
                        });
                    }
                }
                continue;
            }
            Request::SignOut(provider) => {
                // A connection on its way is dropped as it comes.
                signed_out.insert(provider);
                downloads.forget(provider, &replies);
                loading.stop(provider);
                match provider {
                    ProviderKind::AppleMusic => apple = None,
                    ProviderKind::SoundCloud => sc = None,
                    ProviderKind::Navidrome => {
                        nd = None;
                        sign_covers_with(&nd);
                    }
                    ProviderKind::Music => {}
                }
                tokio::task::spawn_blocking(move || forget_sign_in(provider));
                continue;
            }
            Request::SignInSoundCloud => {
                let again = again.clone();
                tokio::spawn(async move {
                    let result = sign_in_soundcloud().await;
                    let ok = result.is_ok();
                    replies.send(Reply::SignedInSoundCloud(result));
                    if ok {
                        let _ = again.send(Request::ReconnectSoundCloud);
                    }
                });
                continue;
            }
            Request::ReconnectSoundCloud => {
                signed_out.remove(&ProviderKind::SoundCloud);
                let arrived_tx = arrived_tx.clone();
                tokio::spawn(async move {
                    let _ = arrived_tx.send(Arrived::SoundCloud(
                        connect_soundcloud(&replies).await,
                        true,
                    ));
                });
                continue;
            }
            Request::ReconnectNavidrome => {
                let arrived_tx = arrived_tx.clone();
                tokio::spawn(async move {
                    let _ = arrived_tx
                        .send(Arrived::Navidrome(connect_navidrome(&replies).await, true));
                });
                continue;
            }
            Request::SignInNavidrome {
                url,
                user,
                password,
            } => {
                signed_out.remove(&ProviderKind::Navidrome);
                let result = navidrome::Credentials::new(&url, &user, &password);
                drop(password);
                let credentials = match result {
                    Ok(c) => c,
                    Err(e) => {
                        replies.send(Reply::SignedInNavidrome(Err(e.to_string())));
                        continue;
                    }
                };
                match Navidrome::connect(credentials.clone()).await {
                    Ok(server) => {
                        let kept = keep_navidrome(&credentials);
                        replies.send(Reply::SignedInNavidrome(kept));
                        replies.send(Reply::Navidrome(Ok(Some(server.user().to_owned()))));
                        let server = Arc::new(server);
                        nd = Some(server.clone());
                        sign_covers_with(&nd);
                        loading.start(
                            ProviderKind::Navidrome,
                            load_navidrome_library(server, replies),
                        );
                    }
                    Err(Error::NotSignedIn) => replies.send(Reply::SignedInNavidrome(Err(
                        "the server didn't take this user and password".into(),
                    ))),
                    Err(e) => replies.send(Reply::SignedInNavidrome(Err(e.to_string()))),
                }
                continue;
            }
            // Every service at once.
            Request::Library => {
                library_wanted = true;
                if let Some(am) = apple.clone() {
                    loading.start(ProviderKind::AppleMusic, load_library(am, replies.clone()));
                }
                if let Some(sc) = sc.clone() {
                    loading.start(
                        ProviderKind::SoundCloud,
                        load_soundcloud_library(sc, replies.clone()),
                    );
                }
                if let Some(nd) = nd.clone() {
                    loading.start(
                        ProviderKind::Navidrome,
                        load_navidrome_library(nd, replies.clone()),
                    );
                }
                // Those signed in but out of reach: as they were kept.
                for (provider, connected) in [
                    (ProviderKind::AppleMusic, apple.is_some()),
                    (ProviderKind::SoundCloud, sc.is_some()),
                    (ProviderKind::Navidrome, nd.is_some()),
                ] {
                    if !connected
                        && !turned_off.contains(&provider)
                        && !signed_out.contains(&provider)
                        && signed_in(provider)
                        && kept_shown.insert(provider)
                    {
                        show_kept(provider, replies.clone());
                    }
                }
                continue;
            }
            Request::NewestSongs => {
                // As kept last time, at once; then as the services have it now.
                for (provider, on) in [
                    (ProviderKind::AppleMusic, apple.is_some()),
                    (ProviderKind::SoundCloud, sc.is_some()),
                    (ProviderKind::Navidrome, nd.is_some()),
                ] {
                    if on && let Some(ids) = cache::read::<Vec<ItemId>>(&newest_kept(provider)) {
                        replies.send(Reply::NewestSongs { provider, ids });
                    }
                }
                if let Some(am) = apple.clone() {
                    let replies = replies.clone();
                    tokio::spawn(async move {
                        let mut ids = Vec::new();
                        let read = am
                            .each_newest_library_track(|tracks, _| {
                                ids.extend(tracks.into_iter().map(|t| t.id))
                            })
                            .await;
                        if read.is_ok() {
                            newest_came(ProviderKind::AppleMusic, ids, &replies);
                        }
                    });
                }
                let others: [(ProviderKind, Option<Arc<dyn Provider>>); 2] = [
                    (
                        ProviderKind::SoundCloud,
                        sc.clone().map(|p| p as Arc<dyn Provider>),
                    ),
                    (
                        ProviderKind::Navidrome,
                        nd.clone().map(|p| p as Arc<dyn Provider>),
                    ),
                ];
                for (provider, p) in others {
                    let (Some(p), replies) = (p, replies.clone()) else {
                        continue;
                    };
                    tokio::spawn(async move {
                        let mut ids = Vec::new();
                        let mut cursor: Option<Cursor> = None;
                        loop {
                            let Ok(page) = p.library_tracks_newest(cursor.as_ref()).await else {
                                return;
                            };
                            ids.extend(page.items.into_iter().map(|t| t.id));
                            match page.next {
                                Some(next) => cursor = Some(next),
                                None => break,
                            }
                        }
                        newest_came(provider, ids, &replies);
                    });
                }
                continue;
            }
            // The server's own lists; Apple Music's are its own below.
            Request::LibraryAlbums | Request::LibraryArtists | Request::RecentlyAdded
                if apple.is_none() =>
            {
                match nd.clone() {
                    Some(nd) => {
                        tokio::spawn(navidrome_lists(nd, request, replies));
                    }
                    // No service has such lists: they are empty, not loading;
                    // Apple's out of reach, as they were kept.
                    None => replies.send(match request {
                        Request::LibraryAlbums => Reply::LibraryAlbums {
                            provider: ProviderKind::AppleMusic,
                            albums: kept_list(apple_kept, "albums"),
                            done: true,
                            whole: true,
                        },
                        Request::LibraryArtists => Reply::LibraryArtists {
                            provider: ProviderKind::AppleMusic,
                            artists: kept_list(apple_kept, "artists"),
                            done: true,
                            whole: true,
                        },
                        _ => Reply::RecentlyAdded {
                            provider: ProviderKind::AppleMusic,
                            items: Vec::new(),
                            done: true,
                        },
                    }),
                }
                continue;
            }
            Request::Playlists => {
                if let Some(am) = apple.clone() {
                    let replies = replies.clone();
                    tokio::spawn(async move {
                        // Only a whole list replaces the one shown.
                        if let (playlists, true) = load_playlists(&am, &replies).await {
                            replies.send(Reply::Playlists {
                                provider: ProviderKind::AppleMusic,
                                playlists,
                            });
                        }
                    });
                }
                if let Some(sc) = sc.clone() {
                    let replies = replies.clone();
                    tokio::spawn(async move {
                        if let Ok(playlists) = soundcloud_playlists(&sc).await {
                            replies.send(Reply::Playlists {
                                provider: ProviderKind::SoundCloud,
                                playlists,
                            });
                        }
                    });
                }
                if let Some(nd) = nd.clone() {
                    tokio::spawn(async move {
                        if let Ok(page) = nd.library_playlists(None).await {
                            replies.send(Reply::Playlists {
                                provider: ProviderKind::Navidrome,
                                playlists: page.items,
                            });
                        }
                    });
                }
                continue;
            }
            // No service needed.
            Request::SiteLyrics { track, sources } => {
                let finder = finder.clone();
                tokio::spawn(async move {
                    let query = lyrics::Query {
                        artist: services::first_artist(&track.artists),
                        album: track.album.clone(),
                        duration: track.duration,
                        title: track.title.clone(),
                    };
                    let found = finder.find(&query, &sources).await;
                    replies.send(Reply::SiteLyrics {
                        id: track.id,
                        found,
                    });
                });
                continue;
            }
            // A cover's colours: any service's, or none.
            Request::Palette(url) => {
                let http = http.clone();
                tokio::spawn(async move {
                    // The cover is often kept already, the page showing it.
                    let bytes = match crate::covers::read(&url) {
                        Some(bytes) => Some(bytes),
                        None => match http.get(crate::covers::signed(&url)).send().await {
                            Ok(resp) => resp.bytes().await.ok().map(|b| b.to_vec()),
                            Err(_) => None,
                        },
                    };
                    let palette = match bytes {
                        Some(bytes) => {
                            tokio::task::spawn_blocking(move || crate::palette::from_image(&bytes))
                                .await
                                .ok()
                                .flatten()
                        }
                        None => None,
                    };
                    replies.send(Reply::Palette { url, palette });
                });
                continue;
            }
            Request::Search(scope, source, query) => {
                let services = Services {
                    apple: apple.clone(),
                    soundcloud: sc.clone(),
                    navidrome: nd.clone(),
                };
                tokio::spawn(search(services, scope, source, query, replies));
                continue;
            }
            _ => {}
        }
        // What is the server's goes there.
        if target(&request) == Some(ProviderKind::Navidrome) {
            match nd.clone() {
                Some(nd) => navidrome_request(nd, request, replies, &mut warming),
                None => unanswered(request, "Navidrome", &replies),
            }
            continue;
        }
        // What is SoundCloud's goes there.
        if target(&request) == Some(ProviderKind::SoundCloud) {
            match sc.clone() {
                Some(sc) => {
                    soundcloud_request(sc, request, replies, &mut warming, cdm.clone().ok());
                }
                None => unanswered(request, "SoundCloud", &replies),
            }
            continue;
        }
        // Without a connection there is nothing to ask; what waits for an
        // answer still hears why.
        let Some(am) = apple.clone() else {
            unanswered(request, "Apple Music", &replies);
            continue;
        };
        let Some(request) = shared_request(am.clone(), request, replies.clone()) else {
            continue;
        };
        match request {
            // Any service's, done above.
            Request::SetFavourite { .. }
            | Request::AddToLibrary { .. }
            | Request::SongLinks { .. }
            | Request::RemoveFromLibrary(_)
            | Request::CreatePlaylist { .. }
            | Request::CreateMusicPlaylist { .. }
            | Request::AddToMusicPlaylist { .. }
            | Request::EditPlaylist { .. }
            | Request::RemoveFromPlaylist { .. }
            | Request::DeletePlaylist(_)
            | Request::AddToPlaylists { .. }
            | Request::Favourite { .. }
            | Request::SuggestLess { .. }
            | Request::SuggestsLess { .. }
            | Request::ShareLink { .. }
            | Request::StationTracks(_) => {}
            Request::SignIn
            | Request::Reconnect
            | Request::SignOut(_)
            | Request::SignInSoundCloud
            | Request::ReconnectSoundCloud
            | Request::ReconnectNavidrome
            | Request::SignInNavidrome { .. }
            | Request::SetService { .. }
            | Request::SiteLyrics { .. }
            | Request::Library
            | Request::NewestSongs
            | Request::Playlists
            | Request::Palette(_)
            | Request::Download { .. }
            | Request::RemoveDownloads(_)
            | Request::Search(..) => {}
            Request::Album(id) => {
                tokio::spawn(async move {
                    let result = tokio::try_join!(am.album(&id), am.album_tracks(&id))
                        .map(Box::new)
                        .map_err(|e| e.to_string());
                    keep_album(&id, &result);
                    replies.send(Reply::Album { id, result });
                });
            }
            Request::Artist(id) => {
                tokio::spawn(async move {
                    let result = am
                        .artist(&id)
                        .await
                        .map(Arc::new)
                        .map_err(|e| e.to_string());
                    replies.send(Reply::Artist { id, result });
                });
            }
            Request::Playlist(id) => {
                tokio::spawn(load_playlist(am, id, replies));
            }
            Request::Lyrics(id) => {
                tokio::spawn(async move {
                    let result = am.lyrics(&id).await.map_err(|e| e.to_string());
                    replies.send(Reply::Lyrics { id, result });
                });
            }
            Request::NewMusic => {
                tokio::spawn(async move {
                    replies.send(Reply::NewMusic(
                        am.new_music().await.map_err(|e| e.to_string()),
                    ));
                });
            }
            Request::Radio => {
                tokio::spawn(async move {
                    replies.send(Reply::Radio(am.radio().await.map_err(|e| e.to_string())));
                });
            }
            Request::Home => {
                tokio::spawn(async move {
                    replies.send(Reply::Home(am.home().await.map_err(|e| e.to_string())));
                });
            }
            Request::LibraryAlbums => {
                if let Some(nd) = nd.clone() {
                    tokio::spawn(navidrome_lists(nd, Request::LibraryAlbums, replies.clone()));
                }
                tokio::spawn(async move {
                    let show = |albums, done, whole| {
                        replies.send(Reply::LibraryAlbums {
                            provider: ProviderKind::AppleMusic,
                            albums,
                            done,
                            whole,
                        })
                    };
                    let mut list = Kept::start("albums", &show);
                    let result = am
                        .each_library_album(|albums, done| list.page(albums, done, &show))
                        .await;
                    list.failed(&replies, result, &show);
                });
            }
            Request::AlbumExtras(id) => {
                tokio::spawn(async move {
                    let result = am
                        .album_extras(&id)
                        .await
                        .map(Arc::new)
                        .map_err(|e| e.to_string());
                    replies.send(Reply::AlbumExtras { id, result });
                });
            }
            Request::LibraryArtistAlbums(id) => {
                tokio::spawn(async move {
                    let result = am
                        .library_artist_albums(&id)
                        .await
                        .map_err(|e| e.to_string());
                    replies.send(Reply::LibraryArtistAlbums { id, result });
                });
            }
            Request::LibraryArtists => {
                if let Some(nd) = nd.clone() {
                    tokio::spawn(navidrome_lists(
                        nd,
                        Request::LibraryArtists,
                        replies.clone(),
                    ));
                }
                tokio::spawn(async move {
                    let show = |artists, done, whole| {
                        replies.send(Reply::LibraryArtists {
                            provider: ProviderKind::AppleMusic,
                            artists,
                            done,
                            whole,
                        })
                    };
                    let mut list = Kept::start("artists", &show);
                    let result = am
                        .each_library_artist(|artists, done| list.page(artists, done, &show))
                        .await;
                    list.failed(&replies, result, &show);
                });
            }
            Request::RecentlyAdded => {
                if let Some(nd) = nd.clone() {
                    tokio::spawn(navidrome_lists(nd, Request::RecentlyAdded, replies.clone()));
                }
                tokio::spawn(async move {
                    let provider = ProviderKind::AppleMusic;
                    // Whole, in the order of what was added to last.
                    let result = am
                        .recently_added_by_activity(RECENT_LIMIT)
                        .await
                        .map(|items| {
                            replies.send(Reply::RecentlyAdded {
                                provider,
                                items,
                                done: true,
                            })
                        });
                    failed(&replies, result, || Reply::RecentlyAdded {
                        provider,
                        items: Vec::new(),
                        done: true,
                    });
                });
            }
            Request::InLibrary { kind, ids } => {
                tokio::spawn(async move {
                    // A failure only leaves Add to Library unoffered.
                    if let Ok(known) = am.in_library(kind, &ids).await {
                        replies.send(Reply::InLibrary(known));
                    }
                });
            }
            Request::Fetch {
                token,
                track,
                fallback,
                spatial,
                ..
            } => {
                let cdm = cdm.clone();
                tokio::spawn(async move {
                    let result = playable(&am, cdm, &track, fallback.as_ref(), spatial).await;
                    replies.send(Reply::Fetched { token, result });
                });
            }
            Request::Motion { url, size } => {
                tokio::spawn(async move {
                    let result = match crate::covers::motion_file(&url, size) {
                        Some(path) if path.exists() => Ok(path),
                        _ => match am.motion_video(&url, size).await {
                            Ok(bytes) => tokio::task::spawn_blocking({
                                let url = url.clone();
                                move || crate::covers::write_motion(&url, size, &bytes)
                            })
                            .await
                            .ok()
                            .flatten()
                            .ok_or_else(|| "couldn't keep the animated cover".to_owned()),
                            Err(e) => Err(e.to_string()),
                        },
                    };
                    replies.send(Reply::Motion { url, size, result });
                });
            }
            Request::Warm {
                token,
                track,
                fallback,
                spatial,
                ..
            } => {
                if let Some(last) = warming.take() {
                    last.abort();
                }
                let cdm = cdm.clone();
                warming = Some(tokio::spawn(async move {
                    let result = playable(&am, cdm, &track, fallback.as_ref(), spatial).await;
                    replies.send(Reply::Fetched { token, result });
                }));
            }
        }
    }
}

/// Whether `provider`'s sign-in is kept on the disk: out of reach, its
/// library shows as it was kept.
fn signed_in(provider: ProviderKind) -> bool {
    match provider {
        ProviderKind::AppleMusic => apple_music_dir().join("media-user-token").exists(),
        ProviderKind::SoundCloud => soundcloud_dir().join("tokens.json").exists(),
        ProviderKind::Navidrome => navidrome_file().exists(),
        ProviderKind::Music => false,
    }
}

/// A service out of reach: its songs and playlists as they were kept, so
/// that the library shows offline (its downloads play).
fn show_kept(provider: ProviderKind, replies: Replies) {
    let (songs, playlists) = match provider {
        ProviderKind::AppleMusic => ("songs", "playlists"),
        ProviderKind::SoundCloud => ("soundcloud-songs", "soundcloud-playlists"),
        ProviderKind::Navidrome => ("navidrome-songs", "navidrome-playlists"),
        ProviderKind::Music => return,
    };
    tokio::task::spawn_blocking(move || {
        if let Some(playlists) = cache::read::<Vec<Playlist>>(playlists) {
            replies.send(Reply::Playlists {
                provider,
                playlists,
            });
        }
        if let Some(tracks) = cache::read::<Vec<Track>>(songs) {
            replies.send(Reply::Songs {
                provider,
                tracks,
                done: true,
                whole: true,
            });
        }
    });
}

/// The list kept under `name`, if `kept` says to show it; else none.
fn kept_list<T: DeserializeOwned>(kept: bool, name: &str) -> Vec<T> {
    kept.then(|| cache::read(name))
        .flatten()
        .unwrap_or_default()
}

/// Where an album's page is kept, to open offline.
fn album_kept(id: &ItemId) -> String {
    format!("album {:?} {}", id.provider, id.id)
}

/// Keep an album's page as it was read.
fn keep_album(id: &ItemId, result: &Result<Box<(Album, Vec<Track>)>, String>) {
    if let Ok(page) = result {
        let (name, page) = (album_kept(id), page.clone());
        tokio::task::spawn_blocking(move || cache::write(&name, &*page));
    }
}

/// Ask Apple whether the subscription is active, which keeps its
/// downloaded songs playing; nothing changes when it doesn't say.
fn confirm_subscription(am: Arc<AppleMusic>, downloads: downloading::Downloads, replies: Replies) {
    tokio::spawn(async move {
        if let Ok(Some(active)) = am.subscribed().await {
            downloads.subscription(ProviderKind::AppleMusic, active, &replies);
        }
    });
}

/// How many songs a station of songs is asked for at a time.
const STATION_BATCH: usize = 20;

/// How far back Recently Added goes.
const RECENT_LIMIT: usize = 200;

/// A track or a station, ready to play: stations (`ra.`) stream, everything
/// else is a track, as Spatial Audio with `spatial` when mixed in Dolby
/// Atmos. A catalog song Apple no longer offers here is played from the
/// library's copy, when there is one, as Apple Music does.
async fn playable(
    am: &AppleMusic,
    cdm: Result<widevine::Cdm, String>,
    track: &ItemId,
    fallback: Option<&ItemId>,
    spatial: bool,
) -> Result<Playable, String> {
    let first = playable_as(am, cdm.clone(), track, spatial).await;
    match (first, fallback) {
        (Err(_), Some(copy)) if copy != track => playable_as(am, cdm, copy, spatial).await,
        (result, _) => result,
    }
}

async fn playable_as(
    am: &AppleMusic,
    cdm: Result<widevine::Cdm, String>,
    track: &ItemId,
    spatial: bool,
) -> Result<Playable, String> {
    match cdm {
        Ok(cdm) if track.id.starts_with("ra.") => am
            .live_station(&cdm, track)
            .await
            .map(Playable::Live)
            .map_err(|e| e.to_string()),
        Ok(cdm) => am
            .fetch_track(&cdm, track, spatial)
            .await
            .map(|(stream, kbps, spatial)| Playable::Track {
                stream,
                kbps,
                spatial,
            })
            .map_err(|e| e.to_string()),
        Err(e) => Err(e),
    }
}

/// A list that failed part way: it ends with `done`, and the failure shows
/// as a toast.
fn failed(replies: &Replies, result: music_core::Result<()>, done: impl FnOnce() -> Reply) {
    if let Err(e) = result {
        replies.send(done());
        replies.send(Reply::Failed(e.to_string()));
    }
}

/// Every library playlist, and whether all came: a failed page is shown, and
/// ends the list there.
async fn load_playlists(am: &AppleMusic, replies: &Replies) -> (Vec<Playlist>, bool) {
    let mut playlists = Vec::new();
    match am
        .each_library_playlist(|page, _| playlists.extend(page))
        .await
    {
        Ok(()) => (playlists, true),
        Err(e) => {
            replies.send(Reply::Failed(e.to_string()));
            (playlists, false)
        }
    }
}

async fn load_library(am: Arc<AppleMusic>, replies: Replies) {
    let provider = ProviderKind::AppleMusic;
    if let Some(playlists) = cache::read::<Vec<Playlist>>("playlists") {
        replies.send(Reply::Playlists {
            provider,
            playlists,
        });
    }
    let (playlists, whole) = load_playlists(&am, &replies).await;
    if whole {
        let kept = playlists.clone();
        tokio::task::spawn_blocking(move || cache::write("playlists", &kept));
    }
    replies.send(Reply::Playlists {
        provider,
        playlists,
    });

    let show = |tracks, done, whole| {
        replies.send(Reply::Songs {
            provider,
            tracks,
            done,
            whole,
        })
    };
    let mut list = Kept::start("songs", &show);
    let result = am
        .each_library_track(|tracks, done| list.page(tracks, done, &show))
        .await;
    list.failed(&replies, result, &show);
}

/// What any service does alike: favourites, the library, playlists, a
/// song's links and stations. Hands back the requests it isn't for.
fn shared_request(p: Arc<dyn Provider>, request: Request, replies: Replies) -> Option<Request> {
    match request {
        Request::SetFavourite { kind, id, on } => {
            tokio::spawn(async move {
                let result = p
                    .set_favourite(kind, &id, on)
                    .await
                    .map(|()| Some(on))
                    .map_err(|e| e.to_string());
                replies.send(Reply::Favourite {
                    id,
                    set: Some(on),
                    result,
                });
            });
        }
        Request::AddToLibrary { kind, id, quiet } => {
            tokio::spawn(async move {
                let result = p
                    .add_to_library(kind, &id)
                    .await
                    .map_err(|e| format!("Couldn't add to the library: {e}"));
                replies.send(Reply::AddedToLibrary {
                    kind,
                    id,
                    quiet,
                    result,
                });
            });
        }
        Request::SongLinks { song, radio } => {
            tokio::spawn(async move {
                let result = p.song_links(&song).await.map_err(|e| e.to_string());
                replies.send(Reply::SongLinks { radio, result });
            });
        }
        Request::RemoveFromLibrary(id) => {
            tokio::spawn(async move {
                let result = p
                    .remove_from_library(&id)
                    .await
                    .map(|()| "Removed from the library".to_owned())
                    .map_err(|e| format!("Couldn't remove it from the library: {e}"));
                replies.send(Reply::RemovedFromLibrary(result));
            });
        }
        Request::CreatePlaylist {
            name,
            description,
            tracks,
            ..
        } => {
            tokio::spawn(async move {
                let result = p
                    .create_playlist(&name, &description, &tracks)
                    .await
                    .map_err(|e| format!("Couldn't make the playlist: {e}"));
                replies.send(Reply::PlaylistCreated(result));
            });
        }
        Request::EditPlaylist {
            id,
            name,
            description,
        } => {
            tokio::spawn(async move {
                let result = p
                    .edit_playlist(&id, &name, &description)
                    .await
                    .map_err(|e| format!("Couldn't change the playlist: {e}"));
                replies.send(Reply::PlaylistEdited {
                    id,
                    name,
                    description,
                    result,
                });
            });
        }
        Request::RemoveFromPlaylist { playlist, entry } => {
            tokio::spawn(async move {
                let result = p
                    .remove_from_playlist(&playlist, &entry)
                    .await
                    .map_err(|e| format!("Couldn't take it out of the playlist: {e}"));
                replies.send(Reply::RemovedFromPlaylist { playlist, result });
            });
        }
        Request::DeletePlaylist(id) => {
            tokio::spawn(async move {
                let result = p
                    .delete_playlist(&id)
                    .await
                    .map_err(|e| format!("Couldn't delete the playlist: {e}"));
                replies.send(Reply::PlaylistDeleted { id, result });
            });
        }
        Request::AddToPlaylists { playlists, tracks } => {
            tokio::spawn(async move {
                let mut failed = None;
                for playlist in &playlists {
                    if let Err(e) = p.add_to_playlist(playlist, &tracks).await {
                        failed = Some(e);
                    }
                }
                let result = match failed {
                    None if playlists.len() == 1 => Ok("Added to the playlist".to_owned()),
                    None => Ok(format!("Added to {} playlists", playlists.len())),
                    Some(e) => Err(format!("Couldn't add to every playlist: {e}")),
                };
                replies.send(Reply::Changed(result));
            });
        }
        Request::Favourite { kind, id } => {
            tokio::spawn(async move {
                let result = p.is_favourite(kind, &id).await.map_err(|e| e.to_string());
                replies.send(Reply::Favourite {
                    id,
                    set: None,
                    result,
                });
            });
        }
        Request::SuggestLess { kind, id, on } => {
            tokio::spawn(async move {
                let result = p
                    .suggest_less(kind, &id, on)
                    .await
                    .map(|()| Some(on))
                    .map_err(|e| e.to_string());
                replies.send(Reply::SuggestsLess {
                    id,
                    set: Some(on),
                    result,
                });
            });
        }
        Request::SuggestsLess { kind, id } => {
            tokio::spawn(async move {
                let result = p.suggests_less(kind, &id).await.map_err(|e| e.to_string());
                replies.send(Reply::SuggestsLess {
                    id,
                    set: None,
                    result,
                });
            });
        }
        Request::ShareLink { kind, id } => {
            tokio::spawn(async move {
                let result = p.share_link(kind, &id).await.map_err(|e| e.to_string());
                replies.send(Reply::ShareLink(result));
            });
        }
        Request::StationTracks(id) => {
            tokio::spawn(async move {
                let result = p
                    .station_tracks(&id, STATION_BATCH)
                    .await
                    .map_err(|e| e.to_string());
                replies.send(Reply::StationTracks { id, result });
            });
        }
        other => return Some(other),
    }
    None
}

/// Each service's library as it loads: one load at a time, the newer one
/// (after a sign-in, or turned back on) calling off the last, so that two
/// never add the same songs.
#[derive(Default)]
struct Loading(std::collections::HashMap<ProviderKind, tokio::task::AbortHandle>);

impl Loading {
    fn start(
        &mut self,
        provider: ProviderKind,
        load: impl std::future::Future<Output = ()> + Send + 'static,
    ) {
        self.stop(provider);
        self.0.insert(provider, tokio::spawn(load).abort_handle());
    }

    fn stop(&mut self, provider: ProviderKind) {
        if let Some(load) = self.0.remove(&provider) {
            load.abort();
        }
    }
}

/// Tell what waits on `request` that `service` isn't there to answer it, so
/// that it shows why rather than load for ever.
fn unanswered(request: Request, service: &str, replies: &Replies) {
    let e = format!("{service} isn't connected");
    let reply = match request {
        Request::Fetch { token, .. } | Request::Warm { token, .. } => Reply::Fetched {
            token,
            result: Err(e),
        },
        // An album or a playlist opened before shows as it was.
        Request::Album(id) => {
            let result = cache::read::<(Album, Vec<Track>)>(&album_kept(&id))
                .map(Box::new)
                .ok_or(e);
            Reply::Album { id, result }
        }
        Request::Artist(id) => Reply::Artist { id, result: Err(e) },
        Request::Playlist(id) => Reply::PlaylistTracks {
            result: cache::read::<Vec<Track>>(&format!("playlist {}", id.id)).ok_or(e),
            id,
            done: true,
            whole: true,
        },
        Request::Lyrics(id) => Reply::Lyrics { id, result: Err(e) },
        Request::AlbumExtras(id) => Reply::AlbumExtras { id, result: Err(e) },
        Request::LibraryArtistAlbums(id) => Reply::LibraryArtistAlbums { id, result: Err(e) },
        Request::StationTracks(id) => Reply::StationTracks { id, result: Err(e) },
        Request::Home => Reply::Home(Err(e)),
        Request::Radio => Reply::Radio(Err(e)),
        Request::NewMusic => Reply::NewMusic(Err(e)),
        // A change shown at once is undone, and the user told why.
        Request::SetFavourite { id, on, .. } => Reply::Favourite {
            id,
            set: Some(on),
            result: Err(e),
        },
        Request::AddToLibrary { kind, id, quiet } => Reply::AddedToLibrary {
            kind,
            id,
            quiet,
            result: Err(e),
        },
        Request::RemoveFromLibrary(_) => {
            Reply::RemovedFromLibrary(Err(format!("Couldn't remove it from the library: {e}")))
        }
        Request::SuggestLess { id, on, .. } => Reply::SuggestsLess {
            id,
            set: Some(on),
            result: Err(e),
        },
        Request::ShareLink { .. } => Reply::ShareLink(Err(e)),
        Request::AddToPlaylists { .. } => {
            Reply::Changed(Err(format!("Couldn't add to the playlist: {e}")))
        }
        Request::CreatePlaylist { .. } => Reply::PlaylistCreated(Err(e)),
        Request::EditPlaylist {
            id,
            name,
            description,
        } => Reply::PlaylistEdited {
            id,
            name,
            description,
            result: Err(e),
        },
        Request::DeletePlaylist(id) => Reply::PlaylistDeleted { id, result: Err(e) },
        Request::RemoveFromPlaylist { playlist, .. } => Reply::RemovedFromPlaylist {
            playlist,
            result: Err(e),
        },
        Request::SongLinks { radio, .. } => Reply::SongLinks {
            radio,
            result: Err(e),
        },
        _ => return,
    };
    replies.send(reply);
}

/// Which service a request is for, when it names an item.
fn target(request: &Request) -> Option<ProviderKind> {
    let id = match request {
        Request::Album(id)
        | Request::Artist(id)
        | Request::Playlist(id)
        | Request::Lyrics(id)
        | Request::StationTracks(id)
        | Request::AlbumExtras(id)
        | Request::LibraryArtistAlbums(id)
        | Request::RemoveFromLibrary(id)
        | Request::DeletePlaylist(id) => id,
        Request::SongLinks { song: id, .. }
        | Request::Fetch { track: id, .. }
        | Request::Warm { track: id, .. }
        | Request::SetFavourite { id, .. }
        | Request::AddToLibrary { id, .. }
        | Request::Favourite { id, .. }
        | Request::SuggestLess { id, .. }
        | Request::SuggestsLess { id, .. }
        | Request::ShareLink { id, .. }
        | Request::EditPlaylist { id, .. }
        | Request::RemoveFromPlaylist { playlist: id, .. } => id,
        Request::InLibrary { ids, .. } => ids.first()?,
        Request::AddToPlaylists { playlists, .. } => playlists.first()?,
        Request::CreatePlaylist { service, .. } => return Some(*service),
        _ => return None,
    };
    Some(id.provider)
}

/// What a service other than Apple Music shows alike: its albums, artists,
/// playlists and lyrics. Hands back the requests it isn't for.
fn item_request(p: Arc<dyn Provider>, request: Request, replies: Replies) -> Option<Request> {
    match request {
        Request::Album(id) => {
            tokio::spawn(async move {
                let result = tokio::try_join!(p.album(&id), p.album_tracks(&id))
                    .map(Box::new)
                    .map_err(|e| e.to_string());
                keep_album(&id, &result);
                replies.send(Reply::Album { id, result });
            });
        }
        Request::Artist(id) => {
            tokio::spawn(async move {
                let result = p.artist(&id).await.map(Arc::new).map_err(|e| e.to_string());
                replies.send(Reply::Artist { id, result });
            });
        }
        // An artist of the library: all their releases there.
        Request::LibraryArtistAlbums(id) => {
            tokio::spawn(async move {
                let result = p
                    .artist(&id)
                    .await
                    .map(|a| [a.albums, a.singles, a.compilations].concat())
                    .map_err(|e| e.to_string());
                replies.send(Reply::LibraryArtistAlbums { id, result });
            });
        }
        // These services' playlists come whole.
        Request::Playlist(id) => {
            tokio::spawn(async move {
                let result = p
                    .playlist_tracks(&id, None)
                    .await
                    .map(|page| page.items)
                    .map_err(|e| e.to_string());
                replies.send(Reply::PlaylistTracks {
                    id,
                    result,
                    done: true,
                    whole: true,
                });
            });
        }
        Request::Lyrics(id) => {
            tokio::spawn(async move {
                let result = p.lyrics(&id).await.map_err(|e| e.to_string());
                replies.send(Reply::Lyrics { id, result });
            });
        }
        // Their covers aren't asked about, and they have no other editions
        // or videos to show.
        Request::InLibrary { .. } => {}
        Request::AlbumExtras(id) => replies.send(Reply::AlbumExtras {
            id,
            result: Ok(Arc::default()),
        }),
        other => return Some(other),
    }
    None
}

/// Fetch a track with `fetch`; as a warm-up, calling off the last one.
fn fetch_with<F>(
    request: Request,
    replies: Replies,
    warming: &mut Option<tokio::task::JoinHandle<()>>,
    fetch: impl FnOnce(ItemId, bool) -> F,
) -> Option<Request>
where
    F: std::future::Future<Output = Result<Playable, String>> + Send + 'static,
{
    match request {
        Request::Fetch {
            token,
            track,
            original,
            ..
        } => {
            let job = fetch(track, original);
            tokio::spawn(async move {
                replies.send(Reply::Fetched {
                    token,
                    result: job.await,
                });
            });
        }
        Request::Warm {
            token,
            track,
            original,
            ..
        } => {
            if let Some(last) = warming.take() {
                last.abort();
            }
            let job = fetch(track, original);
            *warming = Some(tokio::spawn(async move {
                replies.send(Reply::Fetched {
                    token,
                    result: job.await,
                });
            }));
        }
        other => return Some(other),
    }
    None
}

/// A request about a SoundCloud item.
fn soundcloud_request(
    sc: Arc<SoundCloud>,
    request: Request,
    replies: Replies,
    warming: &mut Option<tokio::task::JoinHandle<()>>,
    cdm: Option<widevine::Cdm>,
) {
    let Some(request) = shared_request(sc.clone(), request, replies.clone()) else {
        return;
    };
    let Some(request) = item_request(sc.clone(), request, replies.clone()) else {
        return;
    };
    let fetch = |track: ItemId, original: bool| async move {
        soundcloud_playable(&sc, &track, original, cdm.as_ref()).await
    };
    if fetch_with(request, replies.clone(), warming, fetch).is_some() {
        replies.send(Reply::Changed(Err(
            "That isn't something SoundCloud does".into()
        )));
    }
}

/// A request about a Navidrome item.
fn navidrome_request(
    nd: Arc<Navidrome>,
    request: Request,
    replies: Replies,
    warming: &mut Option<tokio::task::JoinHandle<()>>,
) {
    let Some(request) = shared_request(nd.clone(), request, replies.clone()) else {
        return;
    };
    let Some(request) = item_request(nd.clone(), request, replies.clone()) else {
        return;
    };
    let fetch = |track: ItemId, _original: bool| async move {
        nd.audio(&track)
            .await
            .map(Playable::Navidrome)
            .map_err(|e| e.to_string())
    };
    if fetch_with(request, replies.clone(), warming, fetch).is_some() {
        replies.send(Reply::Changed(Err(
            "That isn't something Navidrome does".into()
        )));
    }
}

/// A SoundCloud track's audio; `cdm` decrypts a major label's.
async fn soundcloud_playable(
    sc: &SoundCloud,
    track: &ItemId,
    original: bool,
    cdm: Option<&widevine::Cdm>,
) -> Result<Playable, String> {
    sc.audio(track, original, cdm)
        .await
        .map(Playable::SoundCloud)
        .map_err(|e| e.to_string())
}

/// What the user liked last on SoundCloud, for Recently Added.
async fn soundcloud_recent(sc: Arc<SoundCloud>, replies: Replies) {
    let provider = ProviderKind::SoundCloud;
    let items = match sc.recently_added(None).await {
        Ok(page) => page.items,
        Err(e) => {
            replies.send(Reply::Failed(e.to_string()));
            Vec::new()
        }
    };
    replies.send(Reply::RecentlyAdded {
        provider,
        items,
        done: true,
    });
}

/// Every SoundCloud playlist of the user's: their own, then those they liked.
async fn soundcloud_playlists(sc: &SoundCloud) -> music_core::Result<Vec<Playlist>> {
    let mut playlists = Vec::new();
    let mut cursor: Option<Cursor> = None;
    loop {
        let page = sc.library_playlists(cursor.as_ref()).await?;
        playlists.extend(page.items);
        match page.next {
            Some(next) => cursor = Some(next),
            None => return Ok(playlists),
        }
    }
}

/// The server's side of the library: its playlists, what the user starred,
/// then every song, kept as Apple Music's are.
async fn load_navidrome_library(nd: Arc<Navidrome>, replies: Replies) {
    let provider = ProviderKind::Navidrome;
    if let Some(playlists) = cache::read::<Vec<Playlist>>("navidrome-playlists") {
        replies.send(Reply::Playlists {
            provider,
            playlists,
        });
    }
    match nd.library_playlists(None).await {
        Ok(page) => {
            let kept = page.items.clone();
            tokio::task::spawn_blocking(move || cache::write("navidrome-playlists", &kept));
            replies.send(Reply::Playlists {
                provider,
                playlists: page.items,
            });
        }
        Err(e) => replies.send(Reply::Failed(e.to_string())),
    }
    if let Ok(starred) = nd.starred().await {
        replies.send(Reply::Starred(starred));
    }
    let show = |tracks, done, whole| {
        replies.send(Reply::Songs {
            provider,
            tracks,
            done,
            whole,
        })
    };
    let mut list = Kept::start("navidrome-songs", &show);
    let mut cursor: Option<Cursor> = None;
    let result = loop {
        match nd.library_tracks(cursor.as_ref()).await {
            Ok(page) => {
                let done = page.next.is_none();
                list.page(page.items, done, &show);
                match page.next {
                    Some(next) => cursor = Some(next),
                    None => break Ok(()),
                }
            }
            Err(e) => break Err(e),
        }
    };
    list.failed(&replies, result, &show);
}

/// The server's albums, artists or recent additions, page by page.
async fn navidrome_lists(nd: Arc<Navidrome>, request: Request, replies: Replies) {
    let provider = ProviderKind::Navidrome;
    match request {
        Request::LibraryAlbums => {
            let mut cursor: Option<Cursor> = None;
            let mut first = true;
            loop {
                match nd.library_albums(cursor.as_ref()).await {
                    Ok(page) => {
                        let done = page.next.is_none();
                        replies.send(Reply::LibraryAlbums {
                            provider,
                            albums: page.items,
                            done,
                            whole: first,
                        });
                        first = false;
                        match page.next {
                            Some(next) => cursor = Some(next),
                            None => return,
                        }
                    }
                    Err(e) => return replies.send(Reply::Failed(e.to_string())),
                }
            }
        }
        Request::LibraryArtists => match nd.library_artists(None).await {
            Ok(page) => replies.send(Reply::LibraryArtists {
                provider,
                artists: page.items,
                done: true,
                whole: true,
            }),
            Err(e) => replies.send(Reply::Failed(e.to_string())),
        },
        Request::RecentlyAdded => match nd.recently_added(None).await {
            Ok(page) => replies.send(Reply::RecentlyAdded {
                provider,
                items: page.items,
                done: true,
            }),
            Err(e) => replies.send(Reply::Failed(e.to_string())),
        },
        _ => {}
    }
}

/// SoundCloud's side of the library: the playlists, then the liked tracks,
/// kept as Apple Music's are.
/// Where a service's songs, the last added first, are kept between launches.
fn newest_kept(provider: ProviderKind) -> String {
    let name = match provider {
        ProviderKind::AppleMusic => "apple",
        ProviderKind::SoundCloud => "soundcloud",
        _ => "navidrome",
    };
    format!("{name}-newest")
}

/// A service's songs, the last added first, as it has them now: kept, and shown.
fn newest_came(provider: ProviderKind, ids: Vec<ItemId>, replies: &Replies) {
    let kept = ids.clone();
    tokio::task::spawn_blocking(move || cache::write(&newest_kept(provider), &kept));
    replies.send(Reply::NewestSongs { provider, ids });
}

async fn load_soundcloud_library(sc: Arc<SoundCloud>, replies: Replies) {
    let provider = ProviderKind::SoundCloud;
    if let Some(playlists) = cache::read::<Vec<Playlist>>("soundcloud-playlists") {
        replies.send(Reply::Playlists {
            provider,
            playlists,
        });
    }
    match soundcloud_playlists(&sc).await {
        Ok(playlists) => {
            let kept = playlists.clone();
            tokio::task::spawn_blocking(move || cache::write("soundcloud-playlists", &kept));
            replies.send(Reply::Playlists {
                provider,
                playlists,
            });
        }
        Err(e) => replies.send(Reply::Failed(e.to_string())),
    }

    let show = |tracks, done, whole| {
        replies.send(Reply::Songs {
            provider,
            tracks,
            done,
            whole,
        })
    };
    let mut list = Kept::start("soundcloud-songs", &show);
    let mut cursor: Option<Cursor> = None;
    let result = loop {
        match sc.library_tracks(cursor.as_ref()).await {
            Ok(page) => {
                let done = page.next.is_none();
                list.page(page.items, done, &show);
                match page.next {
                    Some(next) => cursor = Some(next),
                    None => break Ok(()),
                }
            }
            Err(e) => break Err(e),
        }
    };
    list.failed(&replies, result, &show);
}

/// A search of every service (see `services::Services::search`).
async fn search(
    services: Services,
    scope: SearchScope,
    source: Option<ProviderKind>,
    query: String,
    replies: Replies,
) {
    let within = match scope {
        SearchScope::AppleMusic => services::Scope::Catalog,
        SearchScope::Library => services::Scope::Library,
    };
    let result = services.search(within, &query, source).await;
    replies.send(Reply::Search {
        scope,
        source,
        query,
        result,
    });
}

async fn load_playlist(am: Arc<AppleMusic>, id: ItemId, replies: Replies) {
    let show = |tracks, done, whole| {
        replies.send(Reply::PlaylistTracks {
            id: id.clone(),
            result: Ok(tracks),
            done,
            whole,
        })
    };
    // Only the library's own playlists are kept: a catalog one is read once.
    let mut list = if id.id.starts_with("p.") {
        Kept::start(&format!("playlist {}", id.id), &show)
    } else {
        Kept::none()
    };
    let result = am
        .each_playlist_track(&id, |tracks, done| list.page(tracks, done, &show))
        .await;
    if let Err(e) = result {
        if list.shown {
            replies.send(Reply::Failed(e.to_string()));
        } else {
            replies.send(Reply::PlaylistTracks {
                id,
                result: Err(e.to_string()),
                done: true,
                whole: false,
            });
        }
    }
}

/// A list read from Apple, kept on disk once complete. What was kept from
/// last time shows at once, and the fresh list then replaces it whole when
/// read; with nothing kept, the list shows page by page as it comes.
struct Kept<T> {
    /// Where it is kept, if it is.
    name: Option<String>,
    /// The kept list was shown.
    shown: bool,
    all: Vec<T>,
}

impl<T: Clone + Serialize + DeserializeOwned + Send + 'static> Kept<T> {
    /// Show what was kept under `name` with `show(items, done, whole)`.
    fn start(name: &str, show: &impl Fn(Vec<T>, bool, bool)) -> Self {
        let kept = cache::read::<Vec<T>>(name);
        let shown = kept.is_some();
        if let Some(items) = kept {
            show(items, true, true);
        }
        Self {
            name: Some(name.to_owned()),
            shown,
            all: Vec::new(),
        }
    }

    /// A list not kept: it shows page by page.
    fn none() -> Self {
        Self {
            name: None,
            shown: false,
            all: Vec::new(),
        }
    }

    fn page(&mut self, items: Vec<T>, done: bool, show: &impl Fn(Vec<T>, bool, bool)) {
        if self.shown {
            self.all.extend(items);
            if done {
                show(self.all.clone(), true, true);
            }
        } else {
            if self.name.is_some() {
                self.all.extend(items.iter().cloned());
            }
            show(items, done, false);
        }
        if done && let Some(name) = self.name.take() {
            let all = std::mem::take(&mut self.all);
            tokio::task::spawn_blocking(move || cache::write(&name, &all));
        }
    }

    /// A failure part way: the kept list stays shown, or the pages so far
    /// end there. Either way it shows as a toast.
    fn failed(
        &self,
        replies: &Replies,
        result: music_core::Result<()>,
        show: &impl Fn(Vec<T>, bool, bool),
    ) {
        if let Err(e) = result {
            if !self.shown {
                show(Vec::new(), true, false);
            }
            replies.send(Reply::Failed(e.to_string()));
        }
    }
}
