//! App state, and the glue between the queue, the backend and the player.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use music_core::model::{
    Album, Artist, ArtistDetail, Collection, FavouriteKind, ItemId, Lyrics, Playlist, ProviderKind,
    SearchResults, Shelf, Station, StationKind, Track,
};
use music_core::{Queue, Repeat};
use player::{Event, Player, Status, Token};

use crate::backend::{Backend, Playable, Reply, Request};
use crate::palette::Palette;
use crate::theme::Theme;
use crate::ui;

#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    Home,
    New,
    Radio,
    RecentlyAdded,
    Songs,
    Albums,
    Artists,
    Favourites,
    Playlist(ItemId),
    Album(ItemId),
    Artist(ItemId),
    /// An artist as the library has them: their albums and singles there.
    LibraryArtist(ItemId),
    Search(String),
    Settings,
    /// One shelf of Home, New or Radio in full, named by its title.
    Shelf(Shelves, String),
}

/// A page of shelves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shelves {
    Home,
    New,
    Radio,
}

/// A list that arrives page by page, fetched on first visit.
pub struct Streamed<T> {
    pub items: Vec<T>,
    pub done: bool,
    requested: bool,
    /// Moves on with every change, for what is built from the items.
    pub version: u64,
}

impl<T> Default for Streamed<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            done: false,
            requested: false,
            version: 0,
        }
    }
}

impl<T> Streamed<T> {
    /// Whether to ask for it now: true once.
    fn first_ask(&mut self) -> bool {
        !std::mem::replace(&mut self.requested, true)
    }

    fn extend(&mut self, items: Vec<T>, done: bool) {
        self.items.extend(items);
        self.done = done;
        // From one count for every list, so a list emptied and read again
        // never meets an old version.
        static VERSIONS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        self.version = VERSIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// A page, or with `whole` the whole list, replacing what is there.
    fn arrived(&mut self, items: Vec<T>, done: bool, whole: bool) {
        if whole {
            self.items.clear();
        }
        self.extend(items, done);
    }
}

/// Something fetched on demand.
pub enum Loaded<T> {
    Loading,
    Ready(T),
    Failed(String),
}

/// Where Search looks: everything on Apple Music, or the user's library.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum SearchScope {
    #[default]
    AppleMusic,
    Library,
}

/// The queue drawer's tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawerTab {
    #[default]
    PlayingNext,
    History,
    Lyrics,
}

/// Where tracks go in the queue.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Enqueue {
    /// Replace the queue and play them now.
    Now,
    /// Right after the current track.
    Next,
    /// At the end.
    Last,
}

/// What the albums' order was built from: the list's version and the sort.
type AlbumsOrderKey = (u64, crate::settings::AlbumSort, bool);

/// A playlist's songs that match a search, and where each is in the playlist.
struct Searched {
    query: String,
    from: Arc<Vec<Track>>,
    tracks: Arc<Vec<Track>>,
    numbers: Arc<Vec<u32>>,
}

/// Lower case, with the accents French and its neighbours use taken off, so
/// that `cafe` finds `Café`.
pub fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            c => c,
        })
        .collect()
}

/// An animated cover playing: its video and size, the decoder, and the
/// texture its latest frame went into.
struct MotionView {
    key: (String, u32),
    motion: motion::Motion,
    texture: Option<egui::TextureHandle>,
}

/// The sizes moving covers are fetched at, as Apple makes them: the smallest
/// at least as wide as asked is taken.
const MOTION_SIZES: [u32; 5] = [360, 486, 768, 960, 1080];

/// A song fetched ahead because the pointer rests on it.
struct Warm {
    token: Token,
    track: Track,
    /// Once fetched.
    ready: Option<Playable>,
}

/// How long the pointer rests on a song before it is fetched ahead: longer
/// than passing over it, shorter than reaching for a double click.
const WARM_AFTER: f64 = 0.3;

/// What a playlist's menu asked for.
#[derive(Clone, Copy)]
pub enum PlaylistChoice {
    Enqueue(Enqueue),
    Edit,
    Delete,
}

/// The New Playlist window: its name and description so far, and the songs
/// it starts with.
pub struct NewPlaylist {
    pub name: String,
    pub description: String,
    pub tracks: Vec<Track>,
    /// The playlist being edited, when the window edits one instead.
    pub editing: Option<ItemId>,
}

/// The Add to Playlists window: the songs, the playlists ticked, the search.
pub struct PlaylistDialog {
    pub tracks: Vec<Track>,
    pub chosen: Vec<ItemId>,
    pub filter: String,
}

/// What a track's right-click menu asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrackChoice {
    PlayNext,
    AddToQueue,
    /// Add to the favourites, or take out.
    Favourite,
    AddToLibrary,
    AddToPlaylist,
    /// Play the song's station.
    StartRadio,
    GoToArtist,
    GoToAlbum,
    /// Take it out of the playlist shown; the page does it, knowing which.
    RemoveFromPlaylist,
    RemoveFromLibrary,
}

/// An album or playlist waiting for its tracks before going into the queue.
enum Waiting {
    Album(ItemId),
    Playlist(ItemId),
}

/// What the signal path popover hangs from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalAnchor {
    /// The waveform button in the top bar: the popover opens below it.
    TopBar,
}

impl SignalAnchor {
    /// Where the anchor's last drawn rect is kept.
    pub fn id(self) -> egui::Id {
        egui::Id::unique(("signal-anchor", self as u8))
    }
}

/// How many played tracks the History tab keeps.
const HISTORY: usize = 100;

/// An album and its tracks.
pub type AlbumPage = (Album, Arc<Vec<Track>>);

/// A playlist's tracks, filled page by page.
#[derive(Default)]
pub struct PlaylistTracks {
    pub tracks: Arc<Vec<Track>>,
    pub done: bool,
    pub error: Option<String>,
}

enum PaletteState {
    Pending,
    Ready(Palette),
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Connection {
    Connecting,
    Connected,
    NotSignedIn,
    Failed(String),
}

/// SoundCloud's connection: as Apple Music's, and the user's name once in.
#[derive(Debug, Clone, PartialEq)]
pub enum SoundCloudState {
    Connecting,
    Connected(String),
    NotSignedIn,
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Title,
    Artist,
    Album,
    Duration,
}

/// How the Songs page is sorted and filtered.
#[derive(Default)]
pub struct SongsView {
    /// The column and whether it is ascending.
    pub sort: Option<(SortKey, bool)>,
    pub source: Option<ProviderKind>,
    /// A position in `order`.
    pub selected: Option<usize>,
    /// Indices into `App::songs`, sorted and filtered; rebuilt when `order_key` changes.
    pub order: Vec<usize>,
    pub order_key: Option<OrderKey>,
}

/// What `SongsView::order` was built from: the song count, sort, source
/// filter and search.
pub type OrderKey = (usize, Option<(SortKey, bool)>, Option<ProviderKind>, String);

/// A station of songs asks for more when fewer than this many are left.
const STATION_REFILL: usize = 5;

/// How many tracks in a row may fail to start, and be skipped, before the
/// app stops trying.
const MAX_SKIPS: usize = 5;

/// How long typing pauses before the search runs, in seconds.
const SEARCH_PAUSE: f64 = 0.16;

pub struct App {
    pub theme: Theme,
    backend: Backend,
    pub player: Player,
    /// The desktop's media controls.
    mpris: crate::mpris::Mpris,
    pub connection: Connection,
    /// The Apple Music storefront (country) of the account, once connected.
    pub storefront: Option<String>,
    /// The browser sign-in is open, waiting for the user.
    pub signing_in: bool,
    pub soundcloud: SoundCloudState,
    /// SoundCloud's browser sign-in is open.
    pub soundcloud_signing_in: bool,
    /// The services whose songs are still coming.
    songs_coming: std::collections::HashSet<ProviderKind>,
    pub page: Page,
    back: Vec<Page>,
    forward: Vec<Page>,
    pub sidebar_collapsed: bool,
    pub search: String,
    /// When the search field last changed, to search once typing pauses.
    pub search_edited: Option<f64>,
    /// Put the cursor in the search field on the next frame.
    pub focus_search: bool,
    searches: HashMap<(SearchScope, String), Loaded<Arc<SearchResults>>>,
    pub search_scope: SearchScope,
    pub search_source: Option<ProviderKind>,
    /// Playlists opened from outside the library (search results).
    other_playlists: HashMap<ItemId, Playlist>,
    pub songs: Vec<Track>,
    pub songs_done: bool,
    pub songs_view: SongsView,
    pub playlists: Vec<Playlist>,
    pub library_albums: Streamed<Album>,
    /// The library albums in the order the Albums page sorts them, and what
    /// it was sorted from.
    albums_order: Option<(AlbumsOrderKey, Arc<Vec<usize>>)>,
    /// Albums and playlists added to the library since its lists were read.
    added_to_library: std::collections::HashSet<ItemId>,
    /// Whether catalog albums and playlists are wholly in the library, as
    /// Apple said.
    in_library: HashMap<ItemId, bool>,
    /// Those to ask Apple about next frame, and those asked already.
    in_library_wanted: std::cell::RefCell<Vec<(FavouriteKind, ItemId)>>,
    in_library_asked: std::collections::HashSet<ItemId>,
    pub library_artists: Streamed<Artist>,
    /// Library artists' albums in the library, by artist.
    pub artist_albums: HashMap<ItemId, Loaded<Arc<Vec<Album>>>>,
    pub recently_added: Streamed<Collection>,
    /// The songs in the library, by catalog id.
    library_songs: std::collections::HashSet<ItemId>,
    /// The song whose station is being looked up, to play it.
    radio_from: Option<Track>,
    /// The New Playlist window, when open.
    pub new_playlist: Option<NewPlaylist>,
    /// The playlist the Delete window asks about, when open.
    pub deleting: Option<Playlist>,
    /// A playlist being made from the sidebar, to open once it exists.
    open_created: bool,
    /// The Add to Playlists window, when open.
    pub playlist_dialog: Option<PlaylistDialog>,
    /// The songs in Apple's playlist of favourites.
    favourite_songs: std::collections::HashSet<ItemId>,
    /// What is known to be a favourite or not: set by the user here, or read
    /// from the service. It wins over `favourite_songs`.
    favourites_known: HashMap<ItemId, bool>,
    /// The new music page's shelves, once asked for.
    pub new_music: Option<Loaded<Arc<Vec<Shelf>>>>,
    /// The radio page's shelves, once asked for.
    pub radio: Option<Loaded<Arc<Vec<Shelf>>>>,
    /// The station of songs playing, which the queue keeps topping up.
    station: Option<ItemId>,
    /// More of its songs are on their way.
    station_loading: bool,
    /// The home shelves, once asked for.
    pub home: Option<Loaded<Arc<Vec<Shelf>>>>,
    /// The source filter on the cover grids.
    pub grid_source: Option<ProviderKind>,
    pub albums: HashMap<ItemId, Loaded<AlbumPage>>,
    pub artists: HashMap<ItemId, Loaded<Arc<ArtistDetail>>>,
    /// Catalog albums' other editions and music videos.
    album_extras: HashMap<ItemId, Loaded<Arc<music_core::model::AlbumExtras>>>,
    pub playlist_tracks: HashMap<ItemId, PlaylistTracks>,
    palettes: HashMap<String, PaletteState>,
    lyrics: HashMap<ItemId, Loaded<Option<Arc<Lyrics>>>>,
    /// The selected row on an album or playlist page.
    pub detail_selected: Option<usize>,
    /// An album's notes shown whole, and its title.
    pub notes: Option<(String, String)>,
    /// What the page shown (a playlist, Songs, Albums, Artists) is searched
    /// for; emptied on leaving it.
    pub page_search: String,
    /// Its search field is open, not just its magnifier.
    pub page_search_open: bool,
    /// The playlist's songs matching it, kept until either changes.
    searched: Option<Searched>,
    pub drawer_open: bool,
    pub signal_open: Option<SignalAnchor>,
    pub now_playing: bool,
    pub drawer_tab: DrawerTab,
    /// Tracks that were playing before, most recent last.
    pub history: Vec<Track>,
    pub queue: Queue,
    tracks: HashMap<Token, Track>,
    next_token: Token,
    /// Fetching, to play as soon as it arrives.
    pending_play: Option<Token>,
    /// Fetching or fetched, to follow the current track.
    queued_next: Option<Token>,
    /// The player has `queued_next` as its next track, fetched.
    next_ready: bool,
    /// The song the pointer rests on, fetched in case it is played.
    warm: Option<Warm>,
    /// Animated covers' videos, by master playlist and size: on their way,
    /// kept on disk, or not to be had.
    motion_files: HashMap<(String, u32), Loaded<std::path::PathBuf>>,
    /// The animated covers playing, by where they show.
    motion_views: HashMap<&'static str, MotionView>,
    /// Where animated covers showed this frame: the others stop.
    motion_seen: std::collections::HashSet<&'static str>,
    /// The song the pointer is on, and since when.
    resting: Option<(ItemId, f64)>,
    /// An album or playlist to queue once its tracks are in.
    waiting: Option<(Waiting, Enqueue)>,
    /// Tracks that failed to start one after the other, skipped.
    failed_in_row: usize,
    /// The page the queue was played from, to go back to.
    playing_from: Option<Page>,
    /// The track that is audible, or was last.
    pub now: Option<Track>,
    pub status: Status,
    pub volume: f32,
    pub muted: bool,
    /// When the volume last changed, until it is kept in the settings.
    volume_changed: Option<std::time::Instant>,
    pub toast: Option<(String, f64)>,
    screenshot: Option<crate::screenshot::Screenshot>,
    settings: crate::settings::Settings,
    /// Raised by the watcher when Amane rewrites the theme file.
    theme_changed: Arc<AtomicBool>,
    /// The zoom set last, `None` until the first frame knows the screen.
    zoom: Option<f32>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::fonts::install(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        // Covers come from disk when kept; tried before the HTTP loader.
        cc.egui_ctx
            .add_bytes_loader(Arc::new(crate::covers::CoverLoader::default()));
        std::thread::spawn(crate::covers::trim);
        let settings = crate::settings::Settings::load();
        let theme = resolve_theme(&settings);
        theme.apply(&cc.egui_ctx);
        let theme_changed = Arc::new(AtomicBool::new(false));
        watch_theme(cc.egui_ctx.clone(), theme_changed.clone());
        // Ctrl +, − and 0 are ours: 0 goes back to the scale that suits the screen.
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
        let backend = Backend::start(cc.egui_ctx.clone());
        backend.send(Request::Library);
        let mut app = Self {
            theme,
            backend,
            player: Player::new(),
            mpris: crate::mpris::Mpris::start(cc.egui_ctx.clone()),
            connection: Connection::Connecting,
            storefront: None,
            signing_in: false,
            soundcloud: SoundCloudState::Connecting,
            soundcloud_signing_in: false,
            songs_coming: Default::default(),
            page: Page::Home,
            back: Vec::new(),
            forward: Vec::new(),
            sidebar_collapsed: false,
            search: String::new(),
            search_edited: None,
            focus_search: false,
            searches: HashMap::new(),
            search_scope: settings.search_scope.unwrap_or_default(),
            search_source: None,
            other_playlists: HashMap::new(),
            songs: Vec::new(),
            songs_done: false,
            // Apple returns the library in no useful order; the Music app sorts by title.
            songs_view: SongsView {
                sort: Some((SortKey::Title, true)),
                ..SongsView::default()
            },
            playlists: Vec::new(),
            library_albums: Streamed::default(),
            albums_order: None,
            added_to_library: std::collections::HashSet::new(),
            in_library: HashMap::new(),
            in_library_wanted: Default::default(),
            in_library_asked: std::collections::HashSet::new(),
            library_artists: Streamed::default(),
            artist_albums: HashMap::new(),
            recently_added: Streamed::default(),
            favourite_songs: Default::default(),
            library_songs: Default::default(),
            radio_from: None,
            new_playlist: None,
            deleting: None,
            open_created: false,
            playlist_dialog: None,
            favourites_known: HashMap::new(),
            new_music: None,
            radio: None,
            station: None,
            station_loading: false,
            home: None,
            grid_source: None,
            albums: HashMap::new(),
            artists: HashMap::new(),
            album_extras: HashMap::new(),
            playlist_tracks: HashMap::new(),
            palettes: HashMap::new(),
            lyrics: HashMap::new(),
            detail_selected: None,
            page_search: String::new(),
            notes: None,
            page_search_open: false,
            searched: None,
            drawer_tab: DrawerTab::default(),
            history: Vec::new(),
            queue: Queue::new(),
            tracks: HashMap::new(),
            next_token: 0,
            pending_play: None,
            queued_next: None,
            next_ready: false,
            warm: None,
            motion_files: HashMap::new(),
            motion_views: HashMap::new(),
            motion_seen: std::collections::HashSet::new(),
            resting: None,
            waiting: None,
            failed_in_row: 0,
            playing_from: None,
            now: None,
            status: Status::default(),
            volume: settings.volume.unwrap_or(1.0).clamp(0.0, 1.0),
            muted: settings.muted.unwrap_or(false),
            volume_changed: None,
            toast: None,
            drawer_open: std::env::var_os("MUSIC_SCREENSHOT_DRAWER").is_some(),
            signal_open: std::env::var_os("MUSIC_SCREENSHOT_SIGNAL").map(|_| SignalAnchor::TopBar),
            now_playing: std::env::var_os("MUSIC_SCREENSHOT_NP").is_some(),
            screenshot: crate::screenshot::Screenshot::from_env(),
            settings,
            theme_changed,
            zoom: None,
        };
        // The volume, shuffle and repeat as they were left.
        app.player
            .set_volume(if app.muted { 0.0 } else { app.volume });
        app.queue.set_shuffle(app.settings.shuffle.unwrap_or(false));
        if let Some(repeat) = app.settings.repeat {
            app.queue.set_repeat(repeat.into());
        }
        // The first page loads what it shows, as any page does on arrival.
        app.arrived();
        app
    }

    /// Whether an album or a playlist is in the library, if known: a library
    /// one is, a catalog one as Apple says, asked the first time it is shown.
    fn collection_in_library(&self, kind: FavouriteKind, id: &ItemId) -> Option<bool> {
        let library = match kind {
            FavouriteKind::Album => id.id.starts_with("l."),
            _ => id.id.starts_with("p."),
        };
        if library || self.added_to_library.contains(id) {
            return Some(true);
        }
        let known = self.in_library.get(id).copied();
        if known.is_none() && !self.in_library_asked.contains(id) {
            self.in_library_wanted.borrow_mut().push((kind, id.clone()));
        }
        known
    }

    /// Whether Add to Library is offered for a cover: an album or a playlist
    /// Apple says isn't wholly in the library.
    pub fn can_add_to_library(&self, item: &Collection) -> bool {
        match item {
            Collection::Album(a) => self.can_add_album(&a.id),
            Collection::Playlist(p) => {
                self.collection_in_library(FavouriteKind::Playlist, &p.id) == Some(false)
            }
            Collection::Station(_) => false,
        }
    }

    /// Whether Add to Library is offered for an album.
    pub fn can_add_album(&self, id: &ItemId) -> bool {
        self.collection_in_library(FavouriteKind::Album, id) == Some(false)
    }

    /// Ask Apple, in one request a kind, about the covers shown last frame.
    fn ask_in_library(&mut self) {
        let wanted = std::mem::take(&mut *self.in_library_wanted.borrow_mut());
        for kind in [FavouriteKind::Album, FavouriteKind::Playlist] {
            let ids: Vec<ItemId> = wanted
                .iter()
                .filter(|(k, _)| *k == kind)
                .map(|(_, id)| id.clone())
                // Apple answers for its own only.
                .filter(|id| id.provider == ProviderKind::AppleMusic)
                .filter(|id| self.in_library_asked.insert(id.clone()))
                .collect();
            if !ids.is_empty() {
                self.backend.send(Request::InLibrary { kind, ids });
            }
        }
    }

    /// Add a whole album or playlist to the library.
    pub fn add_collection_to_library(&mut self, item: &Collection, now: f64) {
        let (kind, id, name) = match item {
            Collection::Album(a) => (FavouriteKind::Album, &a.id, &a.title),
            Collection::Playlist(p) => (FavouriteKind::Playlist, &p.id, &p.name),
            Collection::Station(_) => return,
        };
        self.added_to_library.insert(id.clone());
        self.backend.send(Request::AddToLibrary {
            kind,
            id: id.clone(),
            quiet: false,
        });
        self.notify(format!("Adding “{name}” to the library…"), now);
    }

    // ---- Navigation ----

    pub fn go(&mut self, page: Page) {
        // A page shows under Now Playing: it makes way.
        self.now_playing = false;
        if page != self.page {
            self.back.push(std::mem::replace(&mut self.page, page));
            self.forward.clear();
            self.arrived();
        }
    }

    /// Fetch what the page shows, unless it already has been.
    fn arrived(&mut self) {
        self.detail_selected = None;
        self.page_search.clear();
        self.page_search_open = false;
        // Favourite Songs is Apple's playlist of them, once the library says which.
        if self.page == Page::Favourites
            && let Some(id) = self.favourites()
        {
            self.page = Page::Playlist(id);
        }
        match &self.page {
            Page::Album(id) if !self.albums.contains_key(id) => {
                self.albums.insert(id.clone(), Loaded::Loading);
                self.backend.send(Request::Album(id.clone()));
            }
            Page::Home if self.home.is_none() => self.refresh_home(),
            Page::Shelf(which, _) if self.shelves(*which).is_none() => self.refresh(*which),
            Page::Radio if self.radio.is_none() => self.refresh_radio(),
            Page::New if self.new_music.is_none() => self.refresh_new_music(),
            Page::Albums if self.library_albums.first_ask() => {
                self.backend.send(Request::LibraryAlbums);
            }
            Page::Artists if self.library_artists.first_ask() => {
                self.backend.send(Request::LibraryArtists);
            }
            Page::RecentlyAdded if self.recently_added.first_ask() => {
                self.backend.send(Request::RecentlyAdded);
            }
            Page::Artist(id) if !self.artists.contains_key(id) => {
                self.artists.insert(id.clone(), Loaded::Loading);
                self.backend.send(Request::Artist(id.clone()));
            }
            Page::LibraryArtist(id) if !self.artist_albums.contains_key(id) => {
                self.artist_albums.insert(id.clone(), Loaded::Loading);
                self.backend.send(Request::LibraryArtistAlbums(id.clone()));
            }
            Page::Search(query) => {
                let key = (self.search_scope, query.clone());
                if let std::collections::hash_map::Entry::Vacant(e) = self.searches.entry(key) {
                    e.insert(Loaded::Loading);
                    self.backend
                        .send(Request::Search(self.search_scope, query.clone()));
                }
            }
            // A catalog playlist says whether it is a favourite when asked.
            Page::Playlist(id)
                if id.id.starts_with("pl.") && !self.favourites_known.contains_key(id) =>
            {
                if !self.playlist_tracks.contains_key(id) {
                    self.playlist_tracks
                        .insert(id.clone(), PlaylistTracks::default());
                    self.backend.send(Request::Playlist(id.clone()));
                }
                self.backend.send(Request::Favourite {
                    kind: FavouriteKind::Playlist,
                    id: id.clone(),
                });
            }
            Page::Playlist(id) if !self.playlist_tracks.contains_key(id) => {
                self.playlist_tracks
                    .insert(id.clone(), PlaylistTracks::default());
                self.backend.send(Request::Playlist(id.clone()));
            }
            _ => {}
        }
    }

    /// Sign in to Apple Music in the browser; the app reconnects when done.
    pub fn sign_in(&mut self) {
        if !self.signing_in {
            self.signing_in = true;
            self.backend.send(Request::SignIn);
        }
    }

    pub fn sign_in_soundcloud(&mut self) {
        if !self.soundcloud_signing_in {
            self.soundcloud_signing_in = true;
            self.backend.send(Request::SignInSoundCloud);
        }
    }

    pub fn soundcloud_originals(&self) -> bool {
        self.settings.soundcloud_originals.unwrap_or(true)
    }

    pub fn set_soundcloud_originals(&mut self, on: bool) {
        self.settings.soundcloud_originals = Some(on);
        self.settings.save();
    }

    /// Load the library again, as at start: after signing in anew.
    fn reload_library(&mut self) {
        self.songs.clear();
        self.songs_done = false;
        self.library_songs.clear();
        self.playlists.clear();
        self.library_albums = Streamed::default();
        self.library_artists = Streamed::default();
        self.recently_added = Streamed::default();
        self.added_to_library.clear();
        self.in_library.clear();
        self.in_library_asked.clear();
        self.home = None;
        self.searches.clear();
        self.backend.send(Request::Library);
        self.arrived();
    }

    /// Where the colours come from; recolours at once.
    pub fn set_appearance(&mut self, appearance: crate::settings::Appearance, ctx: &egui::Context) {
        self.settings.appearance = Some(appearance);
        self.settings.save();
        let theme = resolve_theme(&self.settings);
        theme.apply(ctx);
        self.theme = theme;
    }

    pub fn appearance(&self) -> crate::settings::Appearance {
        use crate::settings::Appearance;
        match self.settings.appearance {
            Some(a) => a,
            None if Theme::amane().is_some() => Appearance::Amane,
            None => Appearance::Stock,
        }
    }

    /// The interface scale, 1.0 for 100 %.
    pub fn scale(&self) -> f32 {
        self.settings.ui_scale.unwrap_or(1.0)
    }

    /// Choose the interface scale, or go back to 100 %.
    pub fn set_scale(&mut self, scale: Option<f32>) {
        self.settings.ui_scale = scale.map(|s| ((s * 10.0).round() / 10.0).clamp(0.7, 2.5));
        self.settings.save();
        // The next frame applies it.
        self.zoom = None;
    }

    /// Ask for the new music page again.
    pub fn refresh_new_music(&mut self) {
        self.new_music = Some(Loaded::Loading);
        self.backend.send(Request::NewMusic);
    }

    /// Ask for the radio page again.
    pub fn refresh_radio(&mut self) {
        self.radio = Some(Loaded::Loading);
        self.backend.send(Request::Radio);
    }

    /// Play a station: what airs (or an episode) as one endless track, a
    /// station of songs as a queue that tops itself up.
    pub fn play_station(&mut self, station: &Station) {
        match station.kind {
            StationKind::Live | StationKind::Episode => {
                let track = Track {
                    id: station.id.clone(),
                    title: station.name.clone(),
                    artists: vec![match station.kind {
                        StationKind::Live => "Live radio".to_owned(),
                        _ => "Radio".to_owned(),
                    }],
                    album: station.description.clone(),
                    album_id: None,
                    duration: None,
                    track_number: None,
                    disc_number: None,
                    artwork: station.artwork.clone(),
                    explicit: false,
                    quality: None,
                    in_library: false,
                    library_id: None,
                };
                self.play_list(vec![track], 0);
            }
            StationKind::Tracks => {
                self.play_list(Vec::new(), 0);
                self.station = Some(station.id.clone());
                self.station_loading = true;
                self.backend
                    .send(Request::StationTracks(station.id.clone()));
            }
        }
    }

    /// Home's, New's or Radio's shelves, as far as they have come.
    pub fn shelves(&self, which: Shelves) -> Option<&Loaded<Arc<Vec<Shelf>>>> {
        match which {
            Shelves::Home => self.home.as_ref(),
            Shelves::New => self.new_music.as_ref(),
            Shelves::Radio => self.radio.as_ref(),
        }
    }

    /// Ask for Home's, New's or Radio's shelves again.
    pub fn refresh(&mut self, which: Shelves) {
        match which {
            Shelves::Home => self.refresh_home(),
            Shelves::New => self.refresh_new_music(),
            Shelves::Radio => self.refresh_radio(),
        }
    }

    /// Ask for the home shelves again.
    pub fn refresh_home(&mut self) {
        self.home = Some(Loaded::Loading);
        self.backend.send(Request::Home);
    }

    /// Whether `id` is among the favourites, as far as the app knows.
    pub fn is_favourite(&self, kind: FavouriteKind, id: &ItemId) -> bool {
        match self.favourites_known.get(id) {
            Some(&on) => on,
            None => kind == FavouriteKind::Song && self.favourite_songs.contains(id),
        }
    }

    /// Add to the favourites, or take out; shown at once, undone if the
    /// service refuses.
    pub fn toggle_favourite(&mut self, kind: FavouriteKind, id: ItemId) {
        let on = !self.is_favourite(kind, &id);
        self.favourites_known.insert(id.clone(), on);
        if kind == FavouriteKind::Song {
            // Apple's playlist changes with it: load it again when next opened.
            if let Some(favourites) = self.favourites() {
                self.playlist_tracks.remove(&favourites);
                if self.page == Page::Playlist(favourites) {
                    self.arrived();
                }
            }
        }
        // A song favourited joins the library, unless the user said not to.
        if on
            && kind == FavouriteKind::Song
            && self.favourites_to_library()
            && !self.in_library(&id)
        {
            self.library_songs.insert(id.clone());
            self.backend.send(Request::AddToLibrary {
                kind,
                id: id.clone(),
                quiet: true,
            });
        }
        self.backend.send(Request::SetFavourite { kind, id, on });
    }

    /// Whether a song is in the library, as far as the app has loaded it.
    pub fn in_library(&self, id: &ItemId) -> bool {
        self.library_songs.contains(id)
    }

    /// Whether a track is in the library: read from it, or found in it.
    pub fn track_in_library(&self, track: &Track) -> bool {
        track.in_library || self.in_library(&track.id)
    }

    pub fn favourites_to_library(&self) -> bool {
        self.settings.favourites_to_library.unwrap_or(true)
    }

    pub fn set_favourites_to_library(&mut self, on: bool) {
        self.settings.favourites_to_library = Some(on);
        self.settings.save();
    }

    pub fn playlist_songs_to_library(&self) -> bool {
        self.settings.playlist_songs_to_library.unwrap_or(true)
    }

    pub fn set_playlist_songs_to_library(&mut self, on: bool) {
        self.settings.playlist_songs_to_library = Some(on);
        self.settings.save();
    }

    pub fn add_to_library(&mut self, track: &Track, now: f64) {
        self.library_songs.insert(track.id.clone());
        self.backend.send(Request::AddToLibrary {
            kind: FavouriteKind::Song,
            id: track.id.clone(),
            quiet: false,
        });
        self.notify(format!("Adding “{}” to the library…", track.title), now);
    }

    /// The user's own playlists, which songs can be added to.
    pub fn editable_playlists(&self) -> Vec<Playlist> {
        // SoundCloud's take songs in a later version.
        self.playlists
            .iter()
            .filter(|p| p.editable && p.id.provider == ProviderKind::AppleMusic)
            .cloned()
            .collect()
    }

    /// The playlists songs were last added to, most recent first.
    pub fn recent_playlists(&self) -> Vec<Playlist> {
        self.settings
            .recent_playlists
            .iter()
            .filter_map(|id| self.playlists.iter().find(|p| p.id.id == *id && p.editable))
            .cloned()
            .collect()
    }

    /// Open the Add to Playlists window for `tracks`, loading the playlists'
    /// songs so it can say which already have them.
    pub fn open_playlist_dialog(&mut self, tracks: Vec<Track>) {
        for playlist in self.editable_playlists() {
            if !self.playlist_tracks.contains_key(&playlist.id) {
                self.playlist_tracks
                    .insert(playlist.id.clone(), PlaylistTracks::default());
                self.backend.send(Request::Playlist(playlist.id.clone()));
            }
        }
        self.playlist_dialog = Some(PlaylistDialog {
            tracks,
            chosen: Vec::new(),
            filter: String::new(),
        });
    }

    /// Whether `playlist` has every one of `tracks`: `None` while its songs
    /// are still loading.
    pub fn playlist_has(&self, playlist: &ItemId, tracks: &[Track]) -> Option<bool> {
        let entry = self.playlist_tracks.get(playlist)?;
        let has = tracks
            .iter()
            .all(|t| entry.tracks.iter().any(|p| p.id == t.id));
        (has || entry.done).then_some(has)
    }

    /// Open the New Playlist window, the playlist to start with `tracks`.
    pub fn open_new_playlist(&mut self, tracks: Vec<Track>) {
        self.playlist_dialog = None;
        self.new_playlist = Some(NewPlaylist {
            name: String::new(),
            description: String::new(),
            tracks,
            editing: None,
        });
    }

    /// What a playlist's menu in the sidebar asked for.
    pub fn playlist_choice(&mut self, id: &ItemId, choice: PlaylistChoice, now: f64) {
        let Some(playlist) = self.playlist(id).cloned() else {
            return;
        };
        match choice {
            PlaylistChoice::Enqueue(how) => self.enqueue_playlist(&playlist, how, now),
            PlaylistChoice::Edit => {
                self.playlist_dialog = None;
                self.new_playlist = Some(NewPlaylist {
                    name: playlist.name,
                    description: playlist.description.unwrap_or_default(),
                    tracks: Vec::new(),
                    editing: Some(playlist.id),
                });
            }
            PlaylistChoice::Delete => self.deleting = Some(playlist),
        }
    }

    /// Delete the playlist the Delete window asked about, and close it.
    pub fn delete_playlist(&mut self) {
        if let Some(playlist) = self.deleting.take() {
            self.backend.send(Request::DeletePlaylist(playlist.id));
        }
    }

    /// Make the playlist the New Playlist window names, or save the one it
    /// edits, and close it. Made empty (from the sidebar), it opens once it
    /// exists.
    pub fn create_playlist(&mut self) {
        let Some(new) = self.new_playlist.take() else {
            return;
        };
        let name = new.name.trim().to_owned();
        let description = new.description.trim().to_owned();
        if name.is_empty() {
            return;
        }
        if let Some(id) = new.editing {
            let changed = self.playlist(&id).is_some_and(|p| {
                p.name != name || p.description.as_deref().unwrap_or_default() != description
            });
            if changed {
                self.backend.send(Request::EditPlaylist {
                    id,
                    name,
                    description,
                });
            }
            return;
        }
        self.open_created = new.tracks.is_empty();
        if self.playlist_songs_to_library() {
            for track in &new.tracks {
                if !self.track_in_library(track) {
                    self.library_songs.insert(track.id.clone());
                    self.backend.send(Request::AddToLibrary {
                        kind: FavouriteKind::Song,
                        id: track.id.clone(),
                        quiet: true,
                    });
                }
            }
        }
        self.backend.send(Request::CreatePlaylist {
            name,
            description,
            tracks: new.tracks.into_iter().map(|t| t.id).collect(),
        });
    }

    /// Add the window's songs to the playlists ticked in it, and close it.
    pub fn add_to_chosen_playlists(&mut self) {
        let Some(dialog) = self.playlist_dialog.take() else {
            return;
        };
        if dialog.chosen.is_empty() {
            return;
        }
        for id in &dialog.chosen {
            // What the playlist pages show, at once.
            if let Some(entry) = self.playlist_tracks.get_mut(id) {
                Arc::make_mut(&mut entry.tracks).extend(dialog.tracks.iter().cloned());
            }
            self.settings.recent_playlists.retain(|r| *r != id.id);
            self.settings.recent_playlists.insert(0, id.id.clone());
        }
        self.settings.recent_playlists.truncate(3);
        self.settings.save();
        // Songs added to a playlist join the library, unless the user said not to.
        if self.playlist_songs_to_library() {
            for track in &dialog.tracks {
                if !self.track_in_library(track) {
                    self.library_songs.insert(track.id.clone());
                    self.backend.send(Request::AddToLibrary {
                        kind: FavouriteKind::Song,
                        id: track.id.clone(),
                        quiet: true,
                    });
                }
            }
        }
        self.backend.send(Request::AddToPlaylists {
            playlists: dialog.chosen,
            tracks: dialog.tracks.into_iter().map(|t| t.id).collect(),
        });
    }

    /// The id favourites know an album by: its catalog one.
    pub fn album_favourite_id(album: &Album) -> ItemId {
        album.catalog_id.clone().unwrap_or_else(|| album.id.clone())
    }

    /// The id favourites know a playlist by: a catalog one (`pl.…`). The
    /// user's own playlists have one too; Apple's Favourite Songs has none.
    pub fn playlist_favourite_id(playlist: &Playlist) -> Option<ItemId> {
        playlist.catalog_id.clone().or_else(|| {
            playlist
                .id
                .id
                .starts_with("pl.")
                .then(|| playlist.id.clone())
        })
    }

    /// The library's playlist of favourite songs, when there is one.
    pub fn favourites(&self) -> Option<ItemId> {
        self.playlists
            .iter()
            .find(|p| p.favourites)
            .map(|p| p.id.clone())
    }

    pub fn playlist(&self, id: &ItemId) -> Option<&Playlist> {
        self.playlists
            .iter()
            .find(|p| &p.id == id)
            .or_else(|| self.other_playlists.get(id))
    }

    /// Open a playlist that is not in the library.
    pub fn open_playlist(&mut self, playlist: &Playlist) {
        self.other_playlists
            .entry(playlist.id.clone())
            .or_insert_with(|| playlist.clone());
        self.go(Page::Playlist(playlist.id.clone()));
    }

    /// The results for `query` where Search looks now.
    pub fn search_results(&self, query: &str) -> Option<&Loaded<Arc<SearchResults>>> {
        self.searches.get(&(self.search_scope, query.to_owned()))
    }

    /// Look in the catalog or the library, searching there if not done yet.
    pub fn set_search_scope(&mut self, scope: SearchScope) {
        if scope != self.search_scope {
            self.search_scope = scope;
            // Search looks there again next time too.
            self.settings.search_scope = Some(scope);
            self.settings.save();
            self.arrived();
        }
    }

    /// Search for what the field holds: a new page, or the search page
    /// updated in place while typing.
    pub fn submit_search(&mut self) {
        self.search_edited = None;
        let query = self.search.trim().to_owned();
        if query.is_empty() {
            return;
        }
        // A search that failed is tried again.
        let key = (self.search_scope, query.clone());
        if matches!(self.searches.get(&key), Some(Loaded::Failed(_))) {
            self.searches.remove(&key);
        }
        if matches!(self.page, Page::Search(_)) {
            self.page = Page::Search(query);
            self.arrived();
        } else {
            self.go(Page::Search(query));
        }
    }

    /// The cover colours for `url`, fetched on first ask.
    pub fn palette(&mut self, url: &str) -> Option<Palette> {
        match self.palettes.get(url) {
            Some(PaletteState::Ready(palette)) => Some(*palette),
            Some(_) => None,
            None => {
                self.palettes.insert(url.to_owned(), PaletteState::Pending);
                self.backend.send(Request::Palette(url.to_owned()));
                None
            }
        }
    }

    /// A track's lyrics, fetched on first ask.
    pub fn lyrics(&mut self, id: &ItemId) -> &Loaded<Option<Arc<Lyrics>>> {
        if !self.lyrics.contains_key(id) {
            self.backend.send(Request::Lyrics(id.clone()));
        }
        self.lyrics.entry(id.clone()).or_insert(Loaded::Loading)
    }

    /// Whether the drawer is open on the lyrics, or on the queue.
    pub fn drawer_shows(&self, lyrics: bool) -> bool {
        self.drawer_open && (self.drawer_tab == DrawerTab::Lyrics) == lyrics
    }

    /// Open the drawer on the lyrics or on the queue, or close it when it
    /// already shows them.
    pub fn toggle_drawer(&mut self, lyrics: bool) {
        if self.drawer_shows(lyrics) {
            self.drawer_open = false;
            return;
        }
        self.drawer_open = true;
        if lyrics {
            self.drawer_tab = DrawerTab::Lyrics;
        } else if self.drawer_tab == DrawerTab::Lyrics {
            self.drawer_tab = DrawerTab::PlayingNext;
        }
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    pub fn go_back(&mut self) {
        if let Some(page) = self.back.pop() {
            self.forward.push(std::mem::replace(&mut self.page, page));
            self.arrived();
        }
    }

    pub fn go_forward(&mut self) {
        if let Some(page) = self.forward.pop() {
            self.back.push(std::mem::replace(&mut self.page, page));
            self.arrived();
        }
    }

    pub fn page_title(&self) -> String {
        match &self.page {
            Page::Home => "Home".into(),
            Page::New => "New".into(),
            Page::Radio => "Radio".into(),
            Page::RecentlyAdded => "Recently Added".into(),
            Page::Songs => "Songs".into(),
            Page::Albums => "Albums".into(),
            Page::Artists => "Artists".into(),
            Page::Favourites => "Favourite Songs".into(),
            Page::Playlist(id) => self
                .playlist(id)
                .map(|p| p.name.clone())
                .unwrap_or_default(),
            Page::Album(id) => match self.albums.get(id) {
                Some(Loaded::Ready((album, _))) => album.title.clone(),
                _ => String::new(),
            },
            Page::Artist(id) => match self.artists.get(id) {
                Some(Loaded::Ready(artist)) => artist.artist.name.clone(),
                _ => String::new(),
            },
            Page::LibraryArtist(id) => self
                .library_artist(id)
                .map(|a| a.name.clone())
                .unwrap_or_default(),
            Page::Search(_) => "Search".into(),
            Page::Settings => "Settings".into(),
            Page::Shelf(_, title) => title.clone(),
        }
    }

    pub fn notify(&mut self, message: impl Into<String>, now: f64) {
        self.toast = Some((message.into(), now));
    }

    // ---- Playback ----

    /// `tracks` that match `query` in their title, artists or album, every
    /// word of it, with their numbers in `tracks`; or all of them.
    pub fn search_in(
        &mut self,
        tracks: Arc<Vec<Track>>,
        query: &str,
    ) -> (Arc<Vec<Track>>, Option<Arc<Vec<u32>>>) {
        let query = fold(query.trim());
        if query.is_empty() {
            return (tracks, None);
        }
        if let Some(s) = &self.searched
            && s.query == query
            && Arc::ptr_eq(&s.from, &tracks)
        {
            return (s.tracks.clone(), Some(s.numbers.clone()));
        }
        let words: Vec<&str> = query.split_whitespace().collect();
        let (mut found, mut numbers) = (Vec::new(), Vec::new());
        for (i, track) in tracks.iter().enumerate() {
            let text = fold(&format!(
                "{} {} {}",
                track.title,
                track.artists.join(" "),
                track.album.as_deref().unwrap_or_default()
            ));
            if words.iter().all(|w| text.contains(w)) {
                found.push(track.clone());
                numbers.push(i as u32 + 1);
            }
        }
        let (found, numbers) = (Arc::new(found), Arc::new(numbers));
        self.searched = Some(Searched {
            query,
            from: tracks,
            tracks: found.clone(),
            numbers: numbers.clone(),
        });
        (found, Some(numbers))
    }

    pub fn animated_artwork(&self) -> bool {
        self.settings.animated_artwork.unwrap_or(true)
    }

    pub fn set_animated_artwork(&mut self, on: bool) {
        self.settings.animated_artwork = Some(on);
        self.settings.save();
        if !on {
            self.motion_views.clear();
        }
    }

    pub fn motion_quality(&self) -> crate::settings::MotionQuality {
        self.settings
            .motion_quality
            .unwrap_or(crate::settings::MotionQuality::Automatic)
    }

    pub fn set_motion_quality(&mut self, quality: crate::settings::MotionQuality) {
        self.settings.motion_quality = Some(quality);
        self.settings.save();
    }

    /// A library artist by their library id, once the library's artists are
    /// read.
    pub fn library_artist(&self, id: &ItemId) -> Option<&Artist> {
        self.library_artists
            .items
            .iter()
            .find(|a| a.library_id.as_ref() == Some(id))
    }

    /// The library's songs from `albums`, album by album in that order, each
    /// in its discs' and tracks' order.
    pub fn songs_of(&self, albums: &[Album]) -> Vec<Track> {
        let place: HashMap<&ItemId, usize> =
            albums.iter().enumerate().map(|(i, a)| (&a.id, i)).collect();
        let mut songs: Vec<(usize, &Track)> = self
            .songs
            .iter()
            .filter_map(|t| Some((*place.get(t.album_id.as_ref()?)?, t)))
            .collect();
        songs.sort_by_key(|(album, t)| (*album, t.disc_number, t.track_number));
        songs.into_iter().map(|(_, t)| t.clone()).collect()
    }

    /// An artist's page as far as it has come; asks for it the first time.
    pub fn artist_detail(&mut self, id: &ItemId) -> Option<Arc<ArtistDetail>> {
        match self.artists.get(id) {
            Some(Loaded::Ready(artist)) => Some(artist.clone()),
            Some(_) => None,
            None => {
                self.artists.insert(id.clone(), Loaded::Loading);
                self.backend.send(Request::Artist(id.clone()));
                None
            }
        }
    }

    /// The catalog album `id`'s other editions and music videos, asked for
    /// the first time.
    pub fn album_extras(&mut self, id: &ItemId) -> Option<Arc<music_core::model::AlbumExtras>> {
        match self.album_extras.get(id) {
            Some(Loaded::Ready(extras)) => Some(extras.clone()),
            Some(_) => None,
            None => {
                self.album_extras.insert(id.clone(), Loaded::Loading);
                self.backend.send(Request::AlbumExtras(id.clone()));
                None
            }
        }
    }

    /// The animated cover of `track`'s album, once its album says; asks for
    /// the album the first time.
    pub fn album_motion(&mut self, track: &Track) -> Option<String> {
        let id = track.album_id.clone()?;
        match self.albums.get(&id) {
            Some(Loaded::Ready((album, _))) => album.motion.clone(),
            Some(_) => None,
            None => {
                self.albums.insert(id.clone(), Loaded::Loading);
                self.backend.send(Request::Album(id));
                None
            }
        }
    }

    /// The latest frame of the animated cover at `master`, shown in `slot`
    /// over `pixels` on screen; fetches and plays it as needed. `None` until
    /// it plays, or if it can't: the still cover shows then.
    pub fn motion_frame(
        &mut self,
        slot: &'static str,
        master: &str,
        pixels: f32,
        ctx: &egui::Context,
    ) -> Option<egui::load::SizedTexture> {
        if !self.animated_artwork() {
            return None;
        }
        self.motion_seen.insert(slot);
        let wanted = match self.motion_quality() {
            crate::settings::MotionQuality::Low => 486,
            crate::settings::MotionQuality::High => 1080,
            crate::settings::MotionQuality::Automatic => pixels.ceil() as u32,
        };
        let size = MOTION_SIZES
            .into_iter()
            .find(|&s| s >= wanted)
            .unwrap_or(1080);
        let key = (master.to_owned(), size);
        if self.motion_views.get(slot).is_none_or(|v| v.key != key) {
            self.motion_views.remove(slot);
            match self.motion_files.get(&key) {
                None => {
                    self.motion_files.insert(key.clone(), Loaded::Loading);
                    self.backend.send(Request::Motion {
                        url: key.0.clone(),
                        size,
                    });
                    return None;
                }
                Some(Loaded::Ready(path)) => {
                    let wake = ctx.clone();
                    let motion =
                        motion::Motion::play(path.clone(), size, move || wake.request_repaint());
                    self.motion_views.insert(
                        slot,
                        MotionView {
                            key,
                            motion,
                            texture: None,
                        },
                    );
                }
                Some(_) => return None,
            }
        }
        let view = self.motion_views.get_mut(slot)?;
        if view.motion.failed() {
            return None;
        }
        // Back for the next frame when it is due, and only then: a repaint
        // the decoder asked for each frame would come twice.
        ctx.request_repaint_after(view.motion.frame_interval());
        if let Some(frame) = view.motion.take_frame() {
            // Opaque, so premultiplied as it is: the pixels go to the texture
            // without a sum or a copy.
            let image = egui::ColorImage::new(
                [frame.width as usize, frame.height as usize],
                bytemuck::cast_vec(frame.pixels),
            );
            match &mut view.texture {
                Some(texture) => texture.set(image, egui::TextureOptions::LINEAR),
                None => {
                    view.texture = Some(ctx.load_texture(
                        format!("motion-{slot}"),
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
            }
        }
        view.texture
            .as_ref()
            .map(egui::load::SizedTexture::from_handle)
    }

    /// Stop the animated covers that didn't show this frame.
    fn forget_unseen_motion(&mut self) {
        let seen = std::mem::take(&mut self.motion_seen);
        self.motion_views.retain(|slot, _| seen.contains(slot));
    }

    /// How the Albums page is sorted, and whether ascending.
    pub fn albums_sort(&self) -> (crate::settings::AlbumSort, bool) {
        (
            self.settings
                .albums_sort
                .unwrap_or(crate::settings::AlbumSort::Name),
            self.settings.albums_ascending.unwrap_or(true),
        )
    }

    pub fn set_albums_sort(&mut self, by: crate::settings::AlbumSort, ascending: bool) {
        self.settings.albums_sort = Some(by);
        self.settings.albums_ascending = Some(ascending);
        self.settings.save();
    }

    /// The library albums' indices, sorted as the Albums page shows them:
    /// names without case or accents, dates as Apple writes them; those
    /// without the date sorted by go last either way.
    pub fn albums_in_order(&mut self) -> Arc<Vec<usize>> {
        let (by, ascending) = self.albums_sort();
        let key = (self.library_albums.version, by, ascending);
        if let Some((built, order)) = &self.albums_order
            && *built == key
        {
            return order.clone();
        }
        let albums = &self.library_albums.items;
        let name = |i: usize| fold(&albums[i].title);
        let mut order: Vec<usize> = (0..albums.len()).collect();
        let direct = |o: std::cmp::Ordering| if ascending { o } else { o.reverse() };
        match by {
            crate::settings::AlbumSort::Name => {
                let names: Vec<String> = (0..albums.len()).map(name).collect();
                order.sort_by(|&a, &b| direct(names[a].cmp(&names[b])));
            }
            crate::settings::AlbumSort::Artist => {
                let keys: Vec<(String, String)> = (0..albums.len())
                    .map(|i| (fold(&albums[i].artists.join(", ")), name(i)))
                    .collect();
                order.sort_by(|&a, &b| direct(keys[a].cmp(&keys[b])));
            }
            crate::settings::AlbumSort::Added | crate::settings::AlbumSort::Released => {
                let date = |i: usize| match by {
                    crate::settings::AlbumSort::Added => albums[i].added.clone(),
                    _ => albums[i]
                        .released
                        .clone()
                        .or_else(|| albums[i].year.map(|y| y.to_string())),
                };
                let dates: Vec<Option<String>> = (0..albums.len()).map(date).collect();
                order.sort_by(|&a, &b| match (&dates[a], &dates[b]) {
                    (Some(x), Some(y)) => direct(x.cmp(y)),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                });
            }
        }
        let order = Arc::new(order);
        self.albums_order = Some((key, order.clone()));
        order
    }

    /// The track being fetched to play now, until it starts.
    pub fn loading(&self) -> Option<&Track> {
        self.pending_play.and_then(|token| self.tracks.get(&token))
    }

    pub fn is_playing(&self) -> bool {
        self.status.token.is_some() && !self.status.paused
    }

    /// Whether `track` is the one the player has.
    pub fn is_current(&self, track: &Track) -> bool {
        self.status.token.is_some() && self.now.as_ref().is_some_and(|n| n.id == track.id)
    }

    /// Play `tracks` from the top, or shuffled from a random one.
    pub fn play_shuffled_or_not(&mut self, tracks: Vec<Track>, shuffle: bool, seed: f64) {
        if tracks.is_empty() {
            return;
        }
        if self.queue.shuffle() != shuffle {
            self.queue.set_shuffle(shuffle);
        }
        let start = if shuffle {
            (seed * 1000.0) as usize % tracks.len()
        } else {
            0
        };
        self.play_list(tracks, start);
    }

    /// Replace the queue with `tracks` and play from `start`.
    pub fn play_list(&mut self, tracks: Vec<Track>, start: usize) {
        // Whatever plays now is no longer a station of songs.
        self.station = None;
        self.queue.replace(tracks, start);
        self.start_current();
        // Played from a list of songs; elsewhere the song's album will do.
        self.playing_from = match &self.page {
            Page::Playlist(_)
            | Page::Album(_)
            | Page::Artist(_)
            | Page::LibraryArtist(_)
            | Page::Songs
            | Page::Favourites => Some(self.page.clone()),
            _ => None,
        };
    }

    /// Open the page the queue was played from, else the album of the song
    /// playing.
    pub fn go_to_playing(&mut self) {
        let Some(track) = &self.now else {
            return;
        };
        let page = self
            .playing_from
            .clone()
            .or_else(|| track.album_id.clone().map(Page::Album));
        if let Some(page) = page {
            self.now_playing = false;
            if page != self.page {
                self.go(page);
            }
        }
    }

    fn issue(&mut self, track: Track) -> Token {
        let token = self.next_token;
        self.next_token += 1;
        self.backend.send(Request::Fetch {
            token,
            track: track.id.clone(),
            fallback: track.library_id.clone(),
            original: self.soundcloud_originals(),
        });
        self.tracks.insert(token, track);
        token
    }

    fn start_current(&mut self) {
        self.queued_next = None;
        self.next_ready = false;
        if let Some(track) = self.queue.current().cloned() {
            // What played stops at once, as in Apple Music, rather than go on
            // over the wait.
            self.player.stop();
            // Fetched already, or on its way, while the pointer rested on it.
            if let Some(warm) = self.warm.take_if(|w| w.track.id == track.id) {
                self.tracks.insert(warm.token, track);
                match warm.ready {
                    Some(stream) => {
                        self.pending_play = None;
                        self.player.play(warm.token, Self::source(stream));
                    }
                    None => self.pending_play = Some(warm.token),
                }
                return;
            }
            self.pending_play = Some(self.issue(track));
        }
    }

    /// Fetch the song the pointer has rested on a moment, in case it is
    /// played: one at a time, the last called off.
    /// A SoundCloud track that would play from its uploader's file: fetching
    /// it is a download SoundCloud counts, so it waits to be played.
    fn downloads_original(&self, track: &Track) -> bool {
        track.id.provider == ProviderKind::SoundCloud
            && self.soundcloud_originals()
            && track
                .quality
                .as_ref()
                .is_some_and(|q| q.codec == music_core::model::Codec::Other("Original".into()))
    }

    fn rest_on(&mut self, track: Option<Track>, now: f64, ctx: &egui::Context) {
        let Some(track) = track else {
            self.resting = None;
            return;
        };
        let since = match &self.resting {
            Some((id, since)) if *id == track.id => *since,
            _ => {
                self.resting = Some((track.id.clone(), now));
                now
            }
        };
        if now - since < WARM_AFTER {
            ctx.request_repaint_after(Duration::from_secs_f64(WARM_AFTER - (now - since)));
            return;
        }
        let lined_up = self
            .queued_next
            .and_then(|t| self.tracks.get(&t))
            .is_some_and(|t| t.id == track.id);
        let fetched = self.warm.as_ref().is_some_and(|w| w.track.id == track.id);
        // Nothing to do for what plays, is on its way or is ready; and none
        // while a track the user picked is still coming.
        if self.pending_play.is_some()
            || self.is_current(&track)
            || lined_up
            || fetched
            || track.id.id.starts_with("ra.")
            || self.downloads_original(&track)
        {
            return;
        }
        let token = self.next_token;
        self.next_token += 1;
        self.backend.send(Request::Warm {
            token,
            track: track.id.clone(),
            fallback: track.library_id.clone(),
            original: self.soundcloud_originals(),
        });
        self.warm = Some(Warm {
            token,
            track,
            ready: None,
        });
    }

    /// Have the track after the current one fetched, so it follows without a gap.
    fn prefetch_next(&mut self) {
        self.queued_next = self.queue.peek_finished().cloned().map(|t| self.issue(t));
        self.next_ready = false;
    }

    pub fn toggle_play(&mut self) {
        if self.status.token.is_none() {
            if self.pending_play.is_none() {
                self.start_current();
            }
        } else if self.status.paused {
            self.player.resume();
        } else {
            self.player.pause();
        }
    }

    pub fn next(&mut self) {
        // The track lined up to follow is the one Next moves to, unless the
        // current one repeats: play what was fetched for it rather than fetch
        // it again.
        let lined_up = self.queued_next.filter(|token| {
            self.queue.repeat() != Repeat::One
                && self.queue.peek_finished().map(|t| &t.id)
                    == self.tracks.get(token).map(|t| &t.id)
        });
        match lined_up {
            // It starts at once; its Started moves the queue on.
            Some(_) if self.next_ready => self.player.skip_to_next(),
            // Still on its way: it plays when it comes.
            Some(token) => {
                self.queue.skip_next();
                self.queued_next = None;
                self.player.stop();
                self.pending_play = Some(token);
            }
            None => {
                if self.queue.skip_next().is_some() {
                    self.start_current();
                }
            }
        }
    }

    /// Back to the start of the track, or to the previous one when already there.
    pub fn previous(&mut self) {
        if self.status.position > Duration::from_secs(3) {
            self.player.seek(Duration::ZERO);
        } else {
            self.queue.skip_previous();
            self.start_current();
        }
    }

    pub fn seek(&mut self, to: Duration) {
        self.player.seek(to);
    }

    /// Jump to `to` and play from there.
    pub fn play_from(&mut self, to: Duration) {
        self.player.seek(to);
        if self.status.paused {
            self.player.resume();
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.muted = false;
        self.player.set_volume(self.volume);
        self.volume_changed = Some(std::time::Instant::now());
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        self.player
            .set_volume(if self.muted { 0.0 } else { self.volume });
        self.volume_changed = Some(std::time::Instant::now());
    }

    /// Keep the volume for next time, once it has stopped moving: a drag of
    /// the slider changes it every frame.
    fn keep_volume(&mut self, ctx: &egui::Context) {
        const SETTLED: Duration = Duration::from_millis(500);
        let Some(changed) = self.volume_changed else {
            return;
        };
        let since = changed.elapsed();
        if since < SETTLED {
            ctx.request_repaint_after(SETTLED - since);
            return;
        }
        self.volume_changed = None;
        self.settings.volume = Some(self.volume);
        self.settings.muted = Some(self.muted);
        self.settings.save();
    }

    /// Put `tracks` in the queue: instead of it, after the current track, or
    /// at the end. With nothing in the queue, they play now.
    pub fn enqueue(&mut self, tracks: Vec<Track>, how: Enqueue, now: f64) {
        let Some(first) = tracks.first().map(|t| t.title.clone()) else {
            return;
        };
        let count = tracks.len();
        if how == Enqueue::Now || self.queue.current().is_none() {
            self.play_list(tracks, 0);
            return;
        }
        let was_last = self.queue.upcoming_len() == 0;
        match how {
            Enqueue::Next => {
                // Each goes in right after the current track, so the last first.
                for track in tracks.into_iter().rev() {
                    self.queue.add_next(track);
                }
            }
            _ => {
                for track in tracks {
                    self.queue.add_last(track);
                }
            }
        }
        if how == Enqueue::Next || was_last {
            self.prefetch_next();
        }
        let message = match (how, count) {
            (Enqueue::Next, 1) => format!("“{first}” plays next"),
            (Enqueue::Next, n) => format!("{n} songs play next"),
            (_, 1) => format!("“{first}” added to the queue"),
            (_, n) => format!("{n} songs added to the queue"),
        };
        self.notify(message, now);
    }

    /// Do what a track's menu asked.
    pub fn track_choice(&mut self, track: Track, choice: TrackChoice, now: f64) {
        match choice {
            TrackChoice::PlayNext => self.enqueue(vec![track], Enqueue::Next, now),
            TrackChoice::AddToQueue => self.enqueue(vec![track], Enqueue::Last, now),
            TrackChoice::Favourite => self.toggle_favourite(FavouriteKind::Song, track.id),
            TrackChoice::AddToLibrary => self.add_to_library(&track, now),
            TrackChoice::AddToPlaylist => self.open_playlist_dialog(vec![track]),
            TrackChoice::GoToAlbum => {
                if let Some(album) = track.album_id {
                    self.go(Page::Album(album));
                }
            }
            TrackChoice::RemoveFromPlaylist => {}
            TrackChoice::GoToArtist => self.backend.send(Request::SongLinks {
                song: track.id,
                radio: false,
            }),
            TrackChoice::StartRadio => {
                self.backend.send(Request::SongLinks {
                    song: track.id.clone(),
                    radio: true,
                });
                self.radio_from = Some(track);
            }
            TrackChoice::RemoveFromLibrary => {
                if let Some(library_id) = track.library_id {
                    self.library_songs.remove(&track.id);
                    // The Songs list rebuilds its order when its count changes.
                    self.songs.retain(|t| t.id != track.id);
                    // Out of the library is out of its playlists and albums too:
                    // the pages already loaded drop it at once.
                    let gone = |t: &Track| t.library_id.as_ref() == Some(&library_id);
                    for entry in self.playlist_tracks.values_mut() {
                        if entry.tracks.iter().any(gone) {
                            Arc::make_mut(&mut entry.tracks).retain(|t| !gone(t));
                        }
                    }
                    for album in self.albums.values_mut() {
                        if let Loaded::Ready((_, tracks)) = album
                            && tracks.iter().any(gone)
                        {
                            Arc::make_mut(tracks).retain(|t| !gone(t));
                        }
                    }
                    self.favourite_songs.remove(&track.id);
                    self.backend.send(Request::RemoveFromLibrary(library_id));
                }
            }
        }
    }

    /// Take `track` out of one of the user's playlists: off its page at once,
    /// every row of it, as Apple does.
    pub fn remove_from_playlist(&mut self, playlist: &ItemId, track: &Track) {
        let Some(entry) = track.library_id.clone() else {
            return;
        };
        if let Some(loaded) = self.playlist_tracks.get_mut(playlist) {
            Arc::make_mut(&mut loaded.tracks).retain(|t| t.library_id.as_ref() != Some(&entry));
        }
        self.backend.send(Request::RemoveFromPlaylist {
            playlist: playlist.clone(),
            entry,
        });
    }

    /// Queue an album, fetching its tracks first if need be.
    pub fn enqueue_album(&mut self, id: ItemId, how: Enqueue, now: f64) {
        if let Some(Loaded::Ready((_, tracks))) = self.albums.get(&id) {
            let tracks = tracks.to_vec();
            self.enqueue_from(tracks, how, now, Page::Album(id));
            return;
        }
        if !matches!(self.albums.get(&id), Some(Loaded::Loading)) {
            self.albums.insert(id.clone(), Loaded::Loading);
            self.backend.send(Request::Album(id.clone()));
        }
        self.waiting = Some((Waiting::Album(id), how));
    }

    /// Queue a playlist, fetching its tracks first if need be.
    pub fn enqueue_playlist(&mut self, playlist: &Playlist, how: Enqueue, now: f64) {
        self.other_playlists
            .entry(playlist.id.clone())
            .or_insert_with(|| playlist.clone());
        let id = playlist.id.clone();
        if let Some(entry) = self.playlist_tracks.get(&id)
            && entry.done
        {
            let tracks = entry.tracks.to_vec();
            self.enqueue_from(tracks, how, now, Page::Playlist(id));
            return;
        }
        if !self.playlist_tracks.contains_key(&id) {
            self.playlist_tracks
                .insert(id.clone(), PlaylistTracks::default());
            self.backend.send(Request::Playlist(id.clone()));
        }
        self.waiting = Some((Waiting::Playlist(id), how));
    }

    /// Queue the album or playlist that was waiting, once its tracks are in.
    fn queue_waiting(&mut self, now: f64) {
        let tracks = match &self.waiting {
            Some((Waiting::Album(id), _)) => match self.albums.get(id) {
                Some(Loaded::Ready((_, tracks))) => Some(tracks.to_vec()),
                Some(Loaded::Failed(_)) => Some(Vec::new()),
                _ => None,
            },
            Some((Waiting::Playlist(id), _)) => self
                .playlist_tracks
                .get(id)
                .filter(|p| p.done)
                .map(|p| p.tracks.to_vec()),
            None => None,
        };
        if let Some(tracks) = tracks
            && let Some((waiting, how)) = self.waiting.take()
        {
            let from = match waiting {
                Waiting::Album(id) => Page::Album(id),
                Waiting::Playlist(id) => Page::Playlist(id),
            };
            self.enqueue_from(tracks, how, now, from);
        }
    }

    /// [`enqueue`](Self::enqueue) an album's or a playlist's songs: when they
    /// play now, they play from its page.
    fn enqueue_from(&mut self, tracks: Vec<Track>, how: Enqueue, now: f64, from: Page) {
        let plays = !tracks.is_empty() && (how == Enqueue::Now || self.queue.current().is_none());
        self.enqueue(tracks, how, now);
        if plays {
            self.playing_from = Some(from);
        }
    }

    /// Play the `k`-th upcoming track now.
    pub fn jump_to(&mut self, k: usize) {
        if k < self.queue.upcoming_len() {
            self.queue.jump(k);
            self.start_current();
        }
    }

    pub fn remove_upcoming(&mut self, k: usize) {
        self.queue.remove_upcoming(k);
        if k == 0 {
            self.prefetch_next();
        }
    }

    pub fn clear_upcoming(&mut self) {
        self.queue.clear_upcoming();
        self.prefetch_next();
    }

    pub fn toggle_shuffle(&mut self) {
        self.queue.set_shuffle(!self.queue.shuffle());
        self.prefetch_next();
        self.settings.shuffle = Some(self.queue.shuffle());
        self.settings.save();
    }

    pub fn cycle_repeat(&mut self) {
        self.set_repeat(match self.queue.repeat() {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::One,
            Repeat::One => Repeat::Off,
        });
    }

    pub fn set_repeat(&mut self, repeat: Repeat) {
        self.queue.set_repeat(repeat);
        self.prefetch_next();
        self.settings.repeat = Some(self.queue.repeat().into());
        self.settings.save();
    }

    /// The track's duration from its metadata, else from the decoder.
    pub fn duration(&self) -> Option<Duration> {
        self.now
            .as_ref()
            .and_then(|t| t.duration)
            .or_else(|| self.status.source.as_ref().and_then(|s| s.duration))
    }

    fn source(playable: Playable) -> player::Source {
        match playable {
            Playable::Track(stream) => {
                let len = stream.len();
                let hint = stream.seek_hint();
                let extension = stream.extension();
                player::Source::new(stream, len, Some(extension)).with_seek_hint(hint)
            }
            // A station's stream is ADTS, forward only.
            Playable::Live(stream) => player::Source::stream(stream, Some("aac")),
            Playable::SoundCloud(audio) => {
                player::Source::new(audio.reader, audio.len, Some(audio.extension))
            }
        }
    }

    /// Apple Music's playlists came: which are favourites, and which is the
    /// playlist of favourite songs.
    fn apple_playlists_arrived(&mut self) {
        for playlist in &self.playlists {
            if playlist.id.provider != ProviderKind::AppleMusic {
                continue;
            }
            if let (Some(id), Some(on)) = (
                Self::playlist_favourite_id(playlist),
                playlist.in_favourites,
            ) {
                self.favourites_known.entry(id).or_insert(on);
            }
        }
        // Which songs are favourites: those in Apple's playlist of them.
        if let Some(id) = self.favourites()
            && !self.playlist_tracks.contains_key(&id)
        {
            self.playlist_tracks
                .insert(id.clone(), PlaylistTracks::default());
            self.backend.send(Request::Playlist(id));
        }
        // Favourite Songs was opened before the library said which
        // playlist it is.
        if self.page == Page::Favourites
            && let Some(id) = self.favourites()
        {
            self.page = Page::Playlist(id);
            self.arrived();
        }
    }

    // ---- Each frame ----

    fn pump(&mut self, now: f64) {
        while let Some(reply) = self.backend.try_recv() {
            match reply {
                Reply::Connected { storefront } => {
                    // After a sign-in, the library loads now; at start it was
                    // already asked for.
                    let reconnected = self.connection != Connection::Connecting;
                    self.connection = Connection::Connected;
                    self.storefront = Some(storefront);
                    if reconnected {
                        self.reload_library();
                    }
                }
                Reply::SignedIn(result) => {
                    self.signing_in = false;
                    match result {
                        Ok(()) => self.notify("Signed in to Apple Music", now),
                        Err(e) => self.notify(format!("Sign-in didn't finish: {e}"), now),
                    }
                }
                Reply::NotSignedIn => self.connection = Connection::NotSignedIn,
                Reply::SoundCloud(result) => {
                    self.soundcloud = match result {
                        Ok(Some(name)) => SoundCloudState::Connected(name),
                        Ok(None) => SoundCloudState::NotSignedIn,
                        Err(e) => SoundCloudState::Failed(e),
                    };
                }
                Reply::SignedInSoundCloud(result) => {
                    self.soundcloud_signing_in = false;
                    match result {
                        Ok(()) => self.notify("Signed in to SoundCloud", now),
                        Err(e) => self.notify(format!("Sign-in didn't finish: {e}"), now),
                    }
                }
                Reply::Failed(e) => {
                    if self.connection != Connection::Connected {
                        self.connection = Connection::Failed(e.clone());
                    }
                    self.notify(e, now);
                }
                Reply::Songs {
                    provider,
                    tracks,
                    done,
                    whole,
                } => {
                    // A service's songs kept from last time, or the fresh
                    // list after them, in place of its others: the Songs page
                    // sorts them all anew.
                    if whole {
                        self.songs.retain(|t| t.id.provider != provider);
                        self.library_songs.retain(|id| id.provider != provider);
                        self.songs_view.order_key = None;
                        self.songs_view.selected = None;
                    }
                    self.library_songs
                        .extend(tracks.iter().map(|t| t.id.clone()));
                    self.songs.extend(tracks);
                    if done {
                        self.songs_coming.remove(&provider);
                    } else {
                        self.songs_coming.insert(provider);
                    }
                    self.songs_done = self.songs_coming.is_empty();
                }
                Reply::Playlists {
                    provider,
                    playlists,
                } => {
                    // A service's in place of its others, Apple Music's first.
                    self.playlists.retain(|p| p.id.provider != provider);
                    let at = match provider {
                        ProviderKind::AppleMusic => 0,
                        _ => self.playlists.len(),
                    };
                    self.playlists.splice(at..at, playlists);
                    if provider == ProviderKind::AppleMusic {
                        self.apple_playlists_arrived();
                    }
                }
                Reply::LibraryAlbums {
                    albums,
                    done,
                    whole,
                } => self.library_albums.arrived(albums, done, whole),
                Reply::LibraryArtists {
                    artists,
                    done,
                    whole,
                } => self.library_artists.arrived(artists, done, whole),
                Reply::AlbumExtras { id, result } => {
                    let loaded = match result {
                        Ok(extras) => Loaded::Ready(extras),
                        Err(e) => Loaded::Failed(e),
                    };
                    self.album_extras.insert(id, loaded);
                }
                Reply::LibraryArtistAlbums { id, result } => {
                    let loaded = match result {
                        Ok(albums) => Loaded::Ready(Arc::new(albums)),
                        Err(e) => Loaded::Failed(e),
                    };
                    self.artist_albums.insert(id, loaded);
                }
                Reply::NewMusic(result) => {
                    self.new_music = Some(match result {
                        Ok(shelves) => Loaded::Ready(Arc::new(shelves)),
                        Err(e) => Loaded::Failed(e),
                    });
                }
                Reply::Radio(result) => {
                    self.radio = Some(match result {
                        Ok(shelves) => Loaded::Ready(Arc::new(shelves)),
                        Err(e) => Loaded::Failed(e),
                    });
                }
                Reply::StationTracks { id, result } => {
                    if self.station.as_ref() == Some(&id) {
                        self.station_loading = false;
                        match result {
                            // The first songs start it; later ones go on the end.
                            Ok(tracks) if self.queue.current().is_none() => {
                                self.queue.replace(tracks, 0);
                                self.start_current();
                            }
                            Ok(tracks) => {
                                let was_last = self.queue.upcoming_len() == 0;
                                for track in tracks {
                                    self.queue.add_last(track);
                                }
                                if was_last {
                                    self.prefetch_next();
                                }
                            }
                            Err(e) => self.notify(format!("The station stopped: {e}"), now),
                        }
                    }
                }
                Reply::Home(result) => {
                    self.home = Some(match result {
                        Ok(shelves) => Loaded::Ready(Arc::new(shelves)),
                        Err(e) => Loaded::Failed(e),
                    });
                }
                Reply::RecentlyAdded { items, done } => self.recently_added.extend(items, done),
                Reply::Album { id, result } => {
                    let loaded = match result.map(|b| *b) {
                        Ok((album, tracks)) => {
                            if let Some(on) = album.in_favourites {
                                self.favourites_known
                                    .entry(Self::album_favourite_id(&album))
                                    .or_insert(on);
                            }
                            Loaded::Ready((album, Arc::new(tracks)))
                        }
                        Err(e) => Loaded::Failed(e),
                    };
                    self.albums.insert(id, loaded);
                }
                Reply::Artist { id, result } => {
                    let loaded = match result {
                        Ok(artist) => {
                            if let Some(on) = artist.artist.in_favourites {
                                self.favourites_known
                                    .entry(artist.artist.id.clone())
                                    .or_insert(on);
                            }
                            Loaded::Ready(artist)
                        }
                        Err(e) => Loaded::Failed(e),
                    };
                    self.artists.insert(id, loaded);
                }
                Reply::PlaylistTracks {
                    id,
                    result,
                    done,
                    whole,
                } => {
                    // The favourites fill in page by page: Apple's playlist of
                    // them can run to thousands of songs.
                    if self.favourites().as_ref() == Some(&id)
                        && let Ok(tracks) = &result
                    {
                        self.favourite_songs
                            .extend(tracks.iter().map(|t| t.id.clone()));
                    }
                    // Songs read from the library's playlists are in the library.
                    if let Ok(tracks) = &result {
                        self.library_songs
                            .extend(tracks.iter().filter(|t| t.in_library).map(|t| t.id.clone()));
                    }
                    let entry = self.playlist_tracks.entry(id).or_default();
                    match result {
                        // As kept from last time, or fresh after it.
                        Ok(tracks) if whole => {
                            entry.tracks = Arc::new(tracks);
                            entry.error = None;
                        }
                        Ok(tracks) => Arc::make_mut(&mut entry.tracks).extend(tracks),
                        Err(e) => entry.error = Some(e),
                    }
                    entry.done = done;
                }
                Reply::SongLinks { radio, result } => match (radio, result) {
                    (false, Ok(links)) => match links.artist {
                        Some(artist) => self.go(Page::Artist(artist)),
                        None => self.notify("This song has no artist page", now),
                    },
                    (true, Ok(links)) => match (links.station, self.radio_from.take()) {
                        (Some(id), Some(track)) => {
                            self.play_station(&Station {
                                id,
                                name: format!("{} Radio", track.title),
                                description: None,
                                artwork: track.artwork,
                                kind: StationKind::Tracks,
                            });
                        }
                        _ => self.notify("This song has no station", now),
                    },
                    (_, Err(e)) => self.notify(e, now),
                },
                Reply::PlaylistCreated(result) => match result {
                    Ok(playlist) => {
                        self.notify(format!("Made “{}”", playlist.name), now);
                        self.settings
                            .recent_playlists
                            .retain(|r| *r != playlist.id.id);
                        self.settings
                            .recent_playlists
                            .insert(0, playlist.id.id.clone());
                        self.settings.recent_playlists.truncate(3);
                        self.settings.save();
                        let id = playlist.id.clone();
                        self.playlists.push(playlist);
                        if std::mem::take(&mut self.open_created) {
                            self.go(Page::Playlist(id));
                        }
                    }
                    Err(e) => self.notify(e, now),
                },
                Reply::PlaylistEdited {
                    id,
                    name,
                    description,
                    result,
                } => match result {
                    Ok(()) => {
                        if let Some(p) = self.playlists.iter_mut().find(|p| p.id == id) {
                            p.name.clone_from(&name);
                            p.description = Some(description).filter(|d| !d.is_empty());
                        }
                        self.notify(format!("Saved “{name}”"), now);
                    }
                    Err(e) => self.notify(e, now),
                },
                Reply::RemovedFromPlaylist { playlist, result } => match result {
                    Ok(()) => self.notify("Removed from the playlist", now),
                    Err(e) => {
                        // The page dropped it already: read the playlist again.
                        self.playlist_tracks.remove(&playlist);
                        if self.page == Page::Playlist(playlist) {
                            self.arrived();
                        }
                        self.notify(e, now);
                    }
                },
                Reply::PlaylistDeleted { id, result } => match result {
                    Ok(()) => {
                        let name = self.playlist(&id).map(|p| p.name.clone());
                        self.playlists.retain(|p| p.id != id);
                        self.playlist_tracks.remove(&id);
                        self.settings.recent_playlists.retain(|r| *r != id.id);
                        self.settings.save();
                        if self.page == Page::Playlist(id) {
                            self.go(Page::Home);
                        }
                        let message = match name {
                            Some(name) => format!("Deleted “{name}”"),
                            None => "Deleted the playlist".to_owned(),
                        };
                        self.notify(message, now);
                    }
                    Err(e) => self.notify(e, now),
                },
                Reply::InLibrary(known) => self.in_library.extend(known),
                Reply::Motion { url, size, result } => {
                    let loaded = match result {
                        Ok(path) => Loaded::Ready(path),
                        Err(e) => Loaded::Failed(e),
                    };
                    self.motion_files.insert((url, size), loaded);
                }
                Reply::AddedToLibrary {
                    kind,
                    id,
                    quiet,
                    result,
                } => match result {
                    Ok(()) => {
                        match kind {
                            // The sidebar lists it, and the Albums page and
                            // Recently Added are read again to show it.
                            FavouriteKind::Playlist => self.backend.send(Request::Playlists),
                            FavouriteKind::Album => {
                                self.library_albums = Streamed::default();
                                self.recently_added = Streamed::default();
                                if matches!(self.page, Page::Albums | Page::RecentlyAdded) {
                                    self.arrived();
                                }
                            }
                            FavouriteKind::Song | FavouriteKind::Artist => {}
                        }
                        if !quiet {
                            self.notify("Added to the library", now);
                        }
                    }
                    Err(e) => {
                        self.added_to_library.remove(&id);
                        if kind == FavouriteKind::Song {
                            self.library_songs.remove(&id);
                        }
                        self.notify(e, now);
                    }
                },
                Reply::Changed(result) => match result {
                    Ok(message) | Err(message) => self.notify(message, now),
                },
                Reply::Favourite { id, set, result } => match (set, result) {
                    (Some(on), Ok(_)) => {
                        let message = if on {
                            "Added to Favourites"
                        } else {
                            "Removed from Favourites"
                        };
                        self.notify(message, now);
                    }
                    (Some(on), Err(e)) => {
                        self.favourites_known.insert(id, !on);
                        self.notify(format!("Couldn't change the favourites: {e}"), now);
                    }
                    (None, Ok(Some(on))) => {
                        self.favourites_known.entry(id).or_insert(on);
                    }
                    (None, _) => {}
                },
                Reply::Palette { url, palette } => {
                    let state = palette.map_or(PaletteState::Failed, PaletteState::Ready);
                    self.palettes.insert(url, state);
                }
                Reply::Lyrics { id, result } => {
                    let loaded = match result {
                        Ok(lyrics) => Loaded::Ready(lyrics.map(Arc::new)),
                        Err(e) => Loaded::Failed(e),
                    };
                    self.lyrics.insert(id, loaded);
                }
                Reply::Search {
                    scope,
                    query,
                    result,
                } => {
                    let loaded = match result {
                        Ok(results) => Loaded::Ready(Arc::new(results)),
                        Err(e) => Loaded::Failed(e),
                    };
                    self.searches.insert((scope, query), loaded);
                }
                Reply::Fetched { token, result } => match result {
                    Ok(stream) if self.pending_play == Some(token) => {
                        self.pending_play = None;
                        self.player.play(token, Self::source(stream));
                    }
                    Ok(stream) if self.queued_next == Some(token) => {
                        self.player.set_next(token, Self::source(stream));
                        self.next_ready = true;
                    }
                    Ok(stream) if self.warm.as_ref().is_some_and(|w| w.token == token) => {
                        if let Some(warm) = &mut self.warm {
                            warm.ready = Some(stream);
                        }
                    }
                    // Superseded while it was fetching.
                    Ok(_) => {}
                    // Fetched ahead and not played: nothing to say.
                    Err(_) if self.warm.as_ref().is_some_and(|w| w.token == token) => {
                        self.warm = None;
                    }
                    Err(e) => {
                        let title = self
                            .tracks
                            .get(&token)
                            .map(|t| t.title.clone())
                            .unwrap_or_default();
                        // A track Apple has no stream for (gone from the catalog,
                        // or not in this country) is skipped, as Apple Music
                        // does; a run of failures stops, in case nothing plays.
                        if self.pending_play == Some(token) {
                            self.pending_play = None;
                            self.failed_in_row += 1;
                            if self.failed_in_row <= MAX_SKIPS && self.queue.skip_next().is_some() {
                                self.notify(format!("Skipped “{title}”: {e}"), now);
                                self.start_current();
                            } else {
                                self.notify(format!("Couldn't play “{title}”: {e}"), now);
                            }
                        } else if self.queued_next == Some(token) {
                            // The next track can't follow: take it out and line up
                            // the one after.
                            self.queued_next = None;
                            self.queue.remove_upcoming(0);
                            self.prefetch_next();
                            self.notify(format!("Left out “{title}”: {e}"), now);
                        }
                    }
                },
            }
        }

        while let Ok(event) = self.player.events().try_recv() {
            match event {
                Event::Started { token, .. } => {
                    if self.queued_next == Some(token) {
                        self.queue.track_finished();
                        self.queued_next = None;
                    }
                    let started = self.tracks.get(&token).cloned();
                    if let Some(previous) = self.now.take()
                        && started.as_ref().is_none_or(|s| s.id != previous.id)
                    {
                        self.history.push(previous);
                        if self.history.len() > HISTORY {
                            self.history.remove(0);
                        }
                    }
                    self.now = started;
                    let (pending, queued) = (self.pending_play, self.queued_next);
                    self.tracks
                        .retain(|t, _| *t == token || Some(*t) == pending || Some(*t) == queued);
                    self.prefetch_next();
                    self.failed_in_row = 0;
                    // A station of songs is topped up before it runs dry.
                    if let Some(station) = &self.station
                        && !self.station_loading
                        && self.queue.upcoming_len() < STATION_REFILL
                    {
                        self.station_loading = true;
                        self.backend.send(Request::StationTracks(station.clone()));
                    }
                }
                Event::Finished => {}
                Event::Error { token, message } => {
                    // A next track the player couldn't open: Next fetches anew.
                    if token.is_some() && token == self.queued_next {
                        self.queued_next = None;
                        self.next_ready = false;
                    }
                    self.notify(message, now);
                }
            }
        }
        self.status = self.player.status();
        if self.waiting.is_some() {
            self.queue_waiting(now);
        }
    }
}

impl App {
    /// Carry out what the desktop's media controls asked for.
    fn media_controls(&mut self, ctx: &egui::Context) {
        use crate::mpris::Action;
        let mut acted = false;
        while let Some(action) = self.mpris.next_action() {
            acted = true;
            match action {
                Action::PlayPause => self.toggle_play(),
                Action::Play if !self.is_playing() => self.toggle_play(),
                Action::Pause if self.is_playing() => self.player.pause(),
                Action::Play | Action::Pause => {}
                // Paused at the start: Play goes on from there.
                Action::Stop => {
                    self.player.pause();
                    self.player.seek(Duration::ZERO);
                }
                Action::Next => self.next(),
                Action::Previous => self.previous(),
                Action::SeekTo(to) => self.seek(to),
                Action::Volume(volume) => self.set_volume(volume),
                Action::Shuffle(on) => {
                    if on != self.queue.shuffle() {
                        self.toggle_shuffle();
                    }
                }
                Action::Repeat(repeat) => self.set_repeat(repeat),
                Action::Raise => ctx.send_viewport_cmd(egui::ViewportCommand::Focus),
                Action::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
        // The player takes it in a moment: look again then, to show it.
        if acted {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    fn media_state(&self) -> crate::mpris::State {
        use crate::mpris::Playback;
        crate::mpris::State {
            track: self.now.clone(),
            length: self.duration(),
            playback: if self.is_playing() || self.pending_play.is_some() {
                Playback::Playing
            } else if self.status.token.is_some() {
                Playback::Paused
            } else {
                Playback::Stopped
            },
            position: self.status.position,
            volume: if self.muted { 0.0 } else { self.volume },
            shuffle: self.queue.shuffle(),
            repeat: self.queue.repeat(),
        }
    }

    /// The interface scale, changed with Ctrl +, Ctrl − and Ctrl 0.
    fn zoom(&mut self, ctx: &egui::Context) {
        let (bigger, smaller, reset) = ctx.input_mut(|i| {
            let ctrl = egui::Modifiers::COMMAND;
            (
                i.consume_key(ctrl, egui::Key::Plus) || i.consume_key(ctrl, egui::Key::Equals),
                i.consume_key(ctrl, egui::Key::Minus),
                i.consume_key(ctrl, egui::Key::Num0),
            )
        });
        if reset {
            self.set_scale(None);
        } else if bigger || smaller {
            let step = if bigger { 0.1 } else { -0.1 };
            self.set_scale(Some(self.scale() + step));
        }
        let wanted = self.scale() * crate::settings::SCALE_UNIT;
        if self.zoom != Some(wanted) {
            self.zoom = Some(wanted);
            ctx.set_zoom_factor(wanted);
        }
    }
}

impl eframe::App for App {
    fn on_exit(&mut self) {
        // A volume still settling is kept as it is.
        if self.volume_changed.is_some() {
            self.settings.volume = Some(self.volume);
            self.settings.muted = Some(self.muted);
            self.settings.save();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.zoom(ui.ctx());
        self.keep_volume(ui.ctx());
        if self.theme_changed.swap(false, Ordering::Relaxed) {
            let theme = resolve_theme(&self.settings);
            if theme != self.theme {
                theme.apply(ui.ctx());
                self.theme = theme;
            }
        }
        let now = ui.input(|i| i.time);
        self.pump(now);
        self.media_controls(ui.ctx());
        if let Some(edited) = self.search_edited {
            if now - edited >= SEARCH_PAUSE {
                self.submit_search();
            } else {
                ui.ctx()
                    .request_repaint_after(Duration::from_secs_f64(SEARCH_PAUSE));
            }
        }
        ui::show(self, ui);
        self.forget_unseen_motion();
        let under = ui::song_under_pointer(ui.ctx());
        self.rest_on(under, now, ui.ctx());
        // The covers just drawn that Apple hasn't said about yet.
        self.ask_in_library();
        if let Some(page) = self
            .screenshot
            .as_mut()
            .and_then(|s| s.page(&self.playlists))
        {
            self.go(page);
            // Going to a page closes Now Playing: the screenshot may want it.
            self.now_playing = std::env::var_os("MUSIC_SCREENSHOT_NP").is_some();
        }
        // Play a row of the open playlist, or of Songs, once it has loaded.
        let shown: Vec<Track> = match &self.page {
            Page::Playlist(id) => self
                .playlist_tracks
                .get(id)
                .filter(|p| p.done)
                .map(|p| p.tracks.to_vec())
                .unwrap_or_default(),
            _ => self
                .songs_view
                .order
                .iter()
                .map(|&i| self.songs[i].clone())
                .collect(),
        };
        if let Some(row) = self
            .screenshot
            .as_mut()
            .and_then(|s| s.play_row(!shown.is_empty()))
            .filter(|&row| row < shown.len())
        {
            self.play_list(shown, row);
        }
        let connected = self.connection == Connection::Connected;
        if let Some(id) = self.screenshot.as_mut().and_then(|s| s.station(connected)) {
            let live =
                id.starts_with("ra.") && !id.starts_with("ra.u-") && !id.starts_with("ra.q-");
            self.play_station(&Station {
                id: ItemId::new(ProviderKind::AppleMusic, id),
                name: "Station".into(),
                description: None,
                artwork: None,
                kind: if live {
                    StationKind::Live
                } else {
                    StationKind::Tracks
                },
            });
        }
        if let Some(screenshot) = &mut self.screenshot {
            screenshot.update(ui.ctx());
        }
        if self.is_playing() {
            ui.ctx().request_repaint_after(Duration::from_millis(250));
        }
        self.mpris.update(self.media_state());
    }
}

/// Stock, or Amane's theme when it is chosen (or, until it is chosen either
/// way, when its file exists). A missing or invalid file falls back to Stock.
fn resolve_theme(settings: &crate::settings::Settings) -> Theme {
    use crate::settings::Appearance;
    match settings.appearance {
        Some(Appearance::Stock) => Theme::STOCK,
        Some(Appearance::Amane) | None => Theme::amane().unwrap_or(Theme::STOCK),
    }
}

/// Watch Amane's theme file the way Kopuz watches its palette: check its
/// modification time every second and wake the UI when it changes.
fn watch_theme(ctx: egui::Context, changed: Arc<AtomicBool>) {
    let path = Theme::amane_path();
    let modified = move || std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    std::thread::Builder::new()
        .name("theme-watch".into())
        .spawn(move || {
            let mut last = modified();
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let now = modified();
                if now != last {
                    last = now;
                    changed.store(true, Ordering::Relaxed);
                    ctx.request_repaint();
                }
            }
        })
        .expect("spawn the theme watcher");
}
