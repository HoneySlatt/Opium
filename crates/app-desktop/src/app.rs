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

    /// A service's page, or with `whole` its whole list, replacing its
    /// items only: those `of_it` says are.
    fn arrived_from(&mut self, items: Vec<T>, done: bool, whole: bool, of_it: impl Fn(&T) -> bool) {
        if whole {
            self.items.retain(|item| !of_it(item));
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

/// What the column beside the page shows, in a theme that has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    NowPlaying,
    Queue,
    History,
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

/// A search's scope, the one service asked (or all), and its query.
type SearchKey = (SearchScope, Option<ProviderKind>, String);

/// A playlist's songs that match a search, and where each is in the playlist.
struct Searched {
    query: String,
    from: Arc<Vec<Track>>,
    tracks: Arc<Vec<Track>>,
    numbers: Arc<Vec<u32>>,
}

pub use music_core::text::fold;

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
    /// It couldn't be: not asked again while the pointer stays on it.
    failed: bool,
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
    /// The quality under the song in the docked player: above it.
    PlayerBar,
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

/// A song's lyrics being looked for: its service and the lyrics sites are
/// asked in turn, until one has them.
struct LyricsHunt {
    track: Track,
    service_asked: bool,
    sites_asked: bool,
    /// The service's failure, shown if no one has them.
    error: Option<String>,
}

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

/// A server's sign-in, as typed in Settings: the password is handed to the
/// backend once, to make the token kept, and emptied here.
#[derive(Debug, Clone, Default)]
pub struct NavidromeForm {
    pub url: String,
    pub user: String,
    pub password: String,
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
/// Searches kept for going back to them, before starting over.
const SEARCHES_KEPT: usize = 64;

/// How long typing pauses before the search runs, in seconds.
const SEARCH_PAUSE: f64 = 0.16;

/// What plays, as shown: on this computer, or on the device played on.
pub struct Shown {
    pub track: Option<Track>,
    /// The queue has a song: the transport has something to act on.
    pub has_track: bool,
    /// The song picked, on its way.
    pub loading: Option<Track>,
    pub playing: bool,
    pub position: Duration,
    pub duration: Option<Duration>,
    pub shuffle: bool,
    pub repeat: Repeat,
    /// `None` where it can't be set from here.
    pub volume: Option<f32>,
    pub muted: bool,
    /// The device played on, when it isn't this computer.
    pub on: Option<connect::Device>,
}

pub struct App {
    pub theme: Theme,
    backend: Backend,
    pub player: Player,
    /// The desktop's media controls.
    mpris: crate::mpris::Mpris,
    /// The user's other devices, on the network.
    pub devices: crate::devices::Devices,
    /// The device chosen to play on, when it isn't this computer: its id.
    pub remote: Option<String>,
    /// Carrying out what another device asked: for this computer's player,
    /// whatever device is chosen.
    obeying: bool,
    /// Which devices played, to follow one that starts while this
    /// computer is quiet.
    far_playing: HashMap<String, bool>,
    /// The volume of the device played on before it was muted from here.
    far_unmuted: Option<f32>,
    pub connection: Connection,
    /// The Apple Music storefront (country) of the account, once connected.
    pub storefront: Option<String>,
    /// The account's picture on Apple Music, once read.
    pub profile_picture: Option<music_core::model::Artwork>,
    /// The browser sign-in is open, waiting for the user.
    pub signing_in: bool,
    pub soundcloud: SoundCloudState,
    /// SoundCloud's browser sign-in is open.
    pub soundcloud_signing_in: bool,
    /// Navidrome's connection, as SoundCloud's.
    pub navidrome: SoundCloudState,
    pub navidrome_form: NavidromeForm,
    /// The server is being signed in to.
    pub navidrome_signing_in: bool,
    /// Its sign-in shows again, though connected.
    pub navidrome_editing: bool,
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
    /// By scope, the one service asked (or all), and query.
    searches: HashMap<SearchKey, Loaded<Arc<SearchResults>>>,
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
    /// The copies the library's lists leave out, by the version of the list
    /// they were found in: albums, artists, recent additions.
    copies: [Option<(u64, Arc<std::collections::HashSet<usize>>)>; 3],
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
    /// Albums made whole from their copies on other services, and how many
    /// copies they were made from.
    whole_albums: HashMap<ItemId, (usize, Arc<Vec<Track>>)>,
    /// An album's copies on other services, as of a version of the
    /// library's albums; an artist's, of its artists.
    twins: HashMap<ItemId, (u64, Vec<ItemId>)>,
    artist_twins: HashMap<ItemId, (u64, Vec<ItemId>)>,
    pub artists: HashMap<ItemId, Loaded<Arc<ArtistDetail>>>,
    /// Catalog albums' other editions and music videos.
    album_extras: HashMap<ItemId, Loaded<Arc<music_core::model::AlbumExtras>>>,
    pub playlist_tracks: HashMap<ItemId, PlaylistTracks>,
    palettes: HashMap<String, PaletteState>,
    lyrics: HashMap<ItemId, Loaded<Option<Arc<Lyrics>>>>,
    /// Where each song's lyrics have been looked for so far.
    lyrics_hunts: HashMap<ItemId, LyricsHunt>,
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
    /// The column beside the page, when it shows.
    pub side: Option<Side>,
    /// The lyrics over the page, in a theme that shows them there.
    pub lyrics_page: bool,
    /// Tracks that were playing before, most recent last.
    pub history: Vec<Track>,
    pub queue: Queue,
    tracks: HashMap<Token, Track>,
    /// The listed track each token plays the server's copy of, when it does.
    stands_for: HashMap<Token, ItemId>,
    /// What the track playing stands for, when it is the server's copy.
    now_stands_for: Option<ItemId>,
    /// The server's songs by title and artist, and how many songs the
    /// library had when they were indexed.
    server_songs: Option<(usize, music_core::dedupe::Copies)>,
    next_token: Token,
    /// Fetching, to play as soon as it arrives.
    pending_play: Option<Token>,
    /// Where a track handed over from another device goes on from, once
    /// it plays.
    start_at: Option<(Token, Duration)>,
    /// The song that played when the app last closed, shown paused, and
    /// where it was: it goes on from there once played.
    resume: Option<(ItemId, Duration)>,
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
    /// When covers out of sight were last let go of.
    covers_forgotten: f64,
    /// When the window was last drawn, in egui's time.
    drawn_at: f64,
    /// Apple Music out of reach (offline at start, say): when to try again,
    /// and how long the wait was.
    apple_retry: Option<(f64, f64)>,
    /// The track handed to the player to play at once, until it starts: an
    /// error before then means it can't play at all.
    playing: Option<Token>,
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
        // A cover on the GPU isn't kept decoded in memory too: a library's
        // worth of them took hundreds of megabytes.
        cc.egui_ctx.options_mut(|o| o.reduce_texture_memory = true);
        let backend = Backend::start(cc.egui_ctx.clone(), settings.services_off.clone());
        backend.send(Request::Library);
        let mut app = Self {
            theme,
            backend,
            player: Player::new(),
            mpris: crate::mpris::Mpris::start(cc.egui_ctx.clone()),
            devices: crate::devices::Devices::start(cc.egui_ctx.clone()),
            remote: None,
            obeying: false,
            far_playing: HashMap::new(),
            far_unmuted: None,
            connection: if settings.services_off.contains(&ProviderKind::AppleMusic) {
                Connection::NotSignedIn
            } else {
                Connection::Connecting
            },
            storefront: None,
            profile_picture: None,
            signing_in: false,
            soundcloud: SoundCloudState::Connecting,
            soundcloud_signing_in: false,
            navidrome: SoundCloudState::Connecting,
            navidrome_form: NavidromeForm {
                url: "0.0.0.0:4533".into(),
                ..NavidromeForm::default()
            },
            navidrome_signing_in: false,
            navidrome_editing: false,
            songs_coming: Default::default(),
            page: Page::Home,
            back: Vec::new(),
            forward: Vec::new(),
            sidebar_collapsed: settings.sidebar_collapsed.unwrap_or(false),
            search: String::new(),
            search_edited: None,
            focus_search: false,
            searches: HashMap::new(),
            search_scope: settings.search_scope.unwrap_or_default(),
            search_source: settings
                .search_source
                .filter(|p| !settings.services_off.contains(p)),
            other_playlists: HashMap::new(),
            songs: Vec::new(),
            songs_done: false,
            // Apple returns the library in no useful order; the Music app sorts by title.
            songs_view: SongsView {
                sort: Some((SortKey::Title, true)),
                source: settings
                    .songs_source
                    .filter(|p| !settings.services_off.contains(p)),
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
            copies: Default::default(),
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
            grid_source: settings
                .grid_source
                .filter(|p| !settings.services_off.contains(p)),
            albums: HashMap::new(),
            whole_albums: HashMap::new(),
            twins: HashMap::new(),
            artist_twins: HashMap::new(),
            artists: HashMap::new(),
            album_extras: HashMap::new(),
            playlist_tracks: HashMap::new(),
            palettes: HashMap::new(),
            lyrics: HashMap::new(),
            lyrics_hunts: HashMap::new(),
            detail_selected: None,
            page_search: String::new(),
            notes: None,
            page_search_open: false,
            searched: None,
            drawer_tab: DrawerTab::default(),
            // As left: open on what it showed, unless it was closed.
            side: match settings.side_closed {
                Some(true) => None,
                _ => Some(settings.side.unwrap_or(Side::NowPlaying)),
            },
            lyrics_page: std::env::var_os("MUSIC_SCREENSHOT_LYRICS").is_some(),
            history: read_history(),
            queue: Queue::new(),
            tracks: HashMap::new(),
            stands_for: HashMap::new(),
            now_stands_for: None,
            server_songs: None,
            next_token: 0,
            pending_play: None,
            start_at: None,
            resume: None,
            queued_next: None,
            next_ready: false,
            warm: None,
            motion_files: HashMap::new(),
            motion_views: HashMap::new(),
            motion_seen: std::collections::HashSet::new(),
            resting: None,
            waiting: None,
            failed_in_row: 0,
            playing: None,
            covers_forgotten: 0.0,
            drawn_at: 0.0,
            apple_retry: None,
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
        app.resume_session();
        // The first page loads what it shows, as any page does on arrival.
        app.arrived();
        app
    }

    /// Whether an album or a playlist is in the library, if known: a library
    /// one is, a catalog one as Apple says, asked the first time it is shown.
    fn collection_in_library(&self, kind: FavouriteKind, id: &ItemId) -> Option<bool> {
        // A SoundCloud playlist is in the library once liked or made; its
        // albums aren't kept there yet.
        if id.provider == ProviderKind::SoundCloud {
            return Some(
                self.added_to_library.contains(id)
                    || (kind != FavouriteKind::Album && self.playlists.iter().any(|p| p.id == *id)),
            );
        }
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
        // A page shows under Now Playing and the lyrics: they make way.
        self.now_playing = false;
        self.lyrics_page = false;
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
            // Their releases on every service the library has them on.
            Page::LibraryArtist(id) => {
                let id = id.clone();
                for twin in self.artist_twins(&id) {
                    if !self.artist_albums.contains_key(&twin) {
                        self.artist_albums.insert(twin.clone(), Loaded::Loading);
                        self.backend.send(Request::LibraryArtistAlbums(twin));
                    }
                }
            }
            Page::Search(query) => {
                let key = (self.search_scope, self.search_source, query.clone());
                if let std::collections::hash_map::Entry::Vacant(e) = self.searches.entry(key) {
                    e.insert(Loaded::Loading);
                    self.backend.send(Request::Search(
                        self.search_scope,
                        self.search_source,
                        query.clone(),
                    ));
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

    /// Whether a service is on: signed in, it is used.
    /// Albums' and Recently Added's source, kept for next time.
    pub fn set_grid_source(&mut self, source: Option<ProviderKind>) {
        self.grid_source = source;
        self.settings.grid_source = source;
        self.settings.save();
    }

    /// Songs' source, kept for next time.
    pub fn set_songs_source(&mut self, source: Option<ProviderKind>) {
        self.songs_view.source = source;
        self.songs_view.selected = None;
        self.settings.songs_source = source;
        self.settings.save();
    }

    pub fn service_on(&self, provider: ProviderKind) -> bool {
        !self.settings.services_off.contains(&provider)
    }

    /// The services signed in and on: those the source filters offer.
    pub fn services_in_use(&self) -> Vec<ProviderKind> {
        let signed_in = |provider| match provider {
            ProviderKind::AppleMusic => !matches!(self.connection, Connection::NotSignedIn),
            ProviderKind::SoundCloud => !matches!(self.soundcloud, SoundCloudState::NotSignedIn),
            ProviderKind::Navidrome => !matches!(self.navidrome, SoundCloudState::NotSignedIn),
        };
        [
            ProviderKind::AppleMusic,
            ProviderKind::SoundCloud,
            ProviderKind::Navidrome,
        ]
        .into_iter()
        .filter(|&p| self.service_on(p) && signed_in(p))
        .collect()
    }

    /// Turn a service on, loading its library again, or off: its songs,
    /// albums, artists and playlists leave the library, and it is asked
    /// nothing more, its sign-in kept.
    pub fn set_service(&mut self, provider: ProviderKind, on: bool) {
        self.settings.services_off.retain(|p| *p != provider);
        if !on {
            self.settings.services_off.push(provider);
            // A page narrowed to it shows them all again.
            let was = Some(provider);
            if self.grid_source == was {
                self.set_grid_source(None);
            }
            if self.songs_view.source == was {
                self.set_songs_source(None);
            }
            if self.search_source == was {
                self.set_search_source(None);
            }
        }
        self.settings.save();
        self.backend.send(Request::SetService { provider, on });
        let of_it = |id: &ItemId| id.provider == provider;
        match provider {
            ProviderKind::AppleMusic => {
                self.connection = if on {
                    Connection::Connecting
                } else {
                    Connection::NotSignedIn
                }
            }
            ProviderKind::SoundCloud => self.soundcloud = SoundCloudState::Connecting,
            ProviderKind::Navidrome => self.navidrome = SoundCloudState::Connecting,
        }
        if on {
            // Read again, the service's albums with the rest, when shown.
            self.library_albums = Streamed::default();
            self.library_artists = Streamed::default();
            self.recently_added = Streamed::default();
            self.arrived();
            return;
        }
        self.songs.retain(|t| !of_it(&t.id));
        self.library_songs.retain(|id| !of_it(id));
        self.songs_coming.remove(&provider);
        self.songs_done = self.songs_coming.is_empty();
        self.songs_view.order_key = None;
        self.songs_view.selected = None;
        self.playlists.retain(|p| !of_it(&p.id));
        self.library_albums
            .arrived_from(Vec::new(), true, true, |a| of_it(&a.id));
        self.library_artists
            .arrived_from(Vec::new(), true, true, |a| of_it(&a.id));
        self.recently_added
            .arrived_from(Vec::new(), true, true, |c| match c {
                Collection::Album(a) => of_it(&a.id),
                Collection::Playlist(p) => of_it(&p.id),
                Collection::Station(s) => of_it(&s.id),
            });
        self.searches.clear();
        self.server_songs = None;
        self.whole_albums.clear();
    }

    /// Sign in to the server with what the form holds; its password is
    /// emptied at once.
    pub fn sign_in_navidrome(&mut self) {
        if self.navidrome_signing_in {
            return;
        }
        let form = &mut self.navidrome_form;
        let password = std::mem::take(&mut form.password);
        self.navidrome_signing_in = true;
        self.backend.send(Request::SignInNavidrome {
            url: form.url.clone(),
            user: form.user.clone(),
            password,
        });
    }

    pub fn soundcloud_originals(&self) -> bool {
        self.settings.soundcloud_originals.unwrap_or(true)
    }

    pub fn set_soundcloud_originals(&mut self, on: bool) {
        self.settings.soundcloud_originals = Some(on);
        self.settings.save();
    }

    pub fn spatial_audio(&self) -> bool {
        self.settings.spatial_audio.unwrap_or(true)
    }

    pub fn set_spatial_audio(&mut self, on: bool) {
        self.settings.spatial_audio = Some(on);
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

    /// Choose the theme by its id; redraws at once.
    pub fn set_theme(&mut self, id: &str, ctx: &egui::Context) {
        self.settings.theme = (id != "stock").then(|| id.to_string());
        self.settings.save();
        let theme = resolve_theme(&self.settings);
        theme.apply(ctx);
        self.theme = theme;
    }

    /// The chosen theme's id.
    pub fn theme_id(&self) -> &str {
        self.settings.theme.as_deref().unwrap_or("stock")
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
        if let Some(id) = self.far_id() {
            self.devices.node.send(
                &id,
                connect::Command::PlayStation {
                    station: station.clone(),
                },
            );
            return;
        }
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
            // A SoundCloud song's like is its favourite and its library both.
            None if id.provider == ProviderKind::SoundCloud => {
                kind == FavouriteKind::Song && self.library_songs.contains(id)
            }
            None => kind == FavouriteKind::Song && self.favourite_songs.contains(id),
        }
    }

    /// Add to the favourites, or take out; shown at once, undone if the
    /// service refuses.
    pub fn toggle_favourite(&mut self, kind: FavouriteKind, id: ItemId) {
        let on = !self.is_favourite(kind, &id);
        self.favourites_known.insert(id.clone(), on);
        // On SoundCloud, a like: into the library, or out of it.
        if id.provider == ProviderKind::SoundCloud {
            if kind == FavouriteKind::Song {
                if on {
                    self.library_songs.insert(id.clone());
                } else {
                    self.library_songs.remove(&id);
                    self.songs.retain(|t| t.id != id);
                }
            }
            self.backend.send(Request::SetFavourite { kind, id, on });
            return;
        }
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

    /// The user's own playlists that `tracks` can go in: those of their
    /// service, as a playlist takes its own service's songs only.
    pub fn editable_playlists(&self, tracks: &[Track]) -> Vec<Playlist> {
        let provider = tracks.first().map(|t| t.id.provider);
        let one_service = tracks.iter().all(|t| Some(t.id.provider) == provider);
        self.playlists
            .iter()
            .filter(|p| p.editable && one_service && Some(p.id.provider) == provider)
            .cloned()
            .collect()
    }

    /// The playlists songs were last added to, most recent first, that
    /// `tracks` can go in.
    pub fn recent_playlists(&self, tracks: &[Track]) -> Vec<Playlist> {
        let fit = self.editable_playlists(tracks);
        self.settings
            .recent_playlists
            .iter()
            .filter_map(|id| fit.iter().find(|p| p.id.id == *id))
            .cloned()
            .collect()
    }

    /// Open the Add to Playlists window for `tracks`, loading the playlists'
    /// songs so it can say which already have them.
    pub fn open_playlist_dialog(&mut self, tracks: Vec<Track>) {
        for playlist in self.editable_playlists(&tracks) {
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
        self.searches
            .get(&(self.search_scope, self.search_source, query.to_owned()))
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

    /// Search one service alone, or all: the one chosen is asked by itself,
    /// for its own copies of what others have too.
    pub fn set_search_source(&mut self, source: Option<ProviderKind>) {
        if source != self.search_source {
            self.search_source = source;
            self.settings.search_source = source;
            self.settings.save();
            self.arrived();
        }
    }

    /// SoundCloud has no search of one's likes: they are matched here, in the
    /// library as read, taken in turns with the other services' songs. A song
    /// another service has too shows once.
    fn with_likes(&self, results: &mut SearchResults, query: &str) {
        let words: Vec<String> = fold(query.trim())
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        let found = |text: String| {
            let text = fold(&text);
            words.iter().all(|w| text.contains(w.as_str()))
        };
        let liked = |id: &ItemId| id.provider == ProviderKind::SoundCloud;
        let songs = self.songs.iter().filter(|t| {
            liked(&t.id)
                && found(format!(
                    "{} {} {}",
                    t.title,
                    t.artists.join(" "),
                    t.album.as_deref().unwrap_or_default()
                ))
        });
        let all = services::in_turns(
            std::mem::take(&mut results.tracks),
            songs.cloned().collect(),
        );
        let worse = music_core::dedupe::songs(&all);
        results.tracks = all
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !worse.contains(i))
            .map(|(_, t)| t)
            .collect();
        results.playlists.extend(
            self.playlists
                .iter()
                .filter(|p| liked(&p.id) && found(p.name.clone()))
                .cloned(),
        );
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
        let key = (self.search_scope, self.search_source, query.clone());
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
    /// The song's lyrics, looked for the first time: from its service, and
    /// from the lyrics sites when it has none (or first, if so set).
    pub fn lyrics(&mut self, track: &Track) -> &Loaded<Option<Arc<Lyrics>>> {
        let id = track.id.clone();
        if !self.lyrics.contains_key(&id) {
            self.lyrics.insert(id.clone(), Loaded::Loading);
            self.lyrics_hunts.insert(
                id.clone(),
                LyricsHunt {
                    track: track.clone(),
                    service_asked: false,
                    // A station's stream has no song to look up.
                    sites_asked: track.id.id.starts_with("ra."),
                    error: None,
                },
            );
            self.hunt_lyrics(&id);
        }
        self.lyrics.entry(id).or_insert(Loaded::Loading)
    }

    /// The lyrics sites turned on, in the order they are asked.
    pub fn lyrics_sources(&self) -> Vec<lyrics::Source> {
        self.lyrics_sites()
            .into_iter()
            .filter_map(|(s, on)| on.then_some(s))
            .collect()
    }

    /// Every lyrics site, in order, and whether it is on.
    pub fn lyrics_sites(&self) -> Vec<(lyrics::Source, bool)> {
        let order = &self.settings.lyrics_order;
        let mut all: Vec<lyrics::Source> = order.clone();
        all.extend(lyrics::Source::ALL.iter().filter(|s| !order.contains(s)));
        all.into_iter()
            .map(|s| (s, !self.settings.lyrics_off.contains(&s)))
            .collect()
    }

    pub fn set_lyrics_site(&mut self, source: lyrics::Source, on: bool) {
        self.settings.lyrics_off.retain(|s| *s != source);
        if !on {
            self.settings.lyrics_off.push(source);
        }
        self.settings.save();
        self.lyrics_found_anew();
    }

    /// Ask `source` before the one before it.
    pub fn raise_lyrics_site(&mut self, source: lyrics::Source) {
        let mut order: Vec<lyrics::Source> =
            self.lyrics_sites().into_iter().map(|(s, _)| s).collect();
        if let Some(at) = order.iter().position(|s| *s == source)
            && at > 0
        {
            order.swap(at, at - 1);
        }
        self.settings.lyrics_order = order;
        self.settings.save();
        self.lyrics_found_anew();
    }

    pub fn lyrics_sites_first(&self) -> bool {
        self.settings.lyrics_sites_first.unwrap_or(false)
    }

    pub fn set_lyrics_sites_first(&mut self, on: bool) {
        self.settings.lyrics_sites_first = Some(on);
        self.settings.save();
        self.lyrics_found_anew();
    }

    /// Lyrics are looked for again, as the sources changed.
    fn lyrics_found_anew(&mut self) {
        self.lyrics.clear();
        self.lyrics_hunts.clear();
    }

    /// Ask the next of the song's service and the lyrics sites; when no one
    /// is left, it has none.
    fn hunt_lyrics(&mut self, id: &ItemId) {
        let sources = self.lyrics_sources();
        let sites_first = self.lyrics_sites_first();
        let Some(hunt) = self.lyrics_hunts.get_mut(id) else {
            return;
        };
        if sources.is_empty() {
            hunt.sites_asked = true;
        }
        let ask_sites = !hunt.sites_asked && (sites_first || hunt.service_asked);
        if ask_sites {
            hunt.sites_asked = true;
            self.backend.send(Request::SiteLyrics {
                track: Box::new(hunt.track.clone()),
                sources,
            });
        } else if !hunt.service_asked {
            hunt.service_asked = true;
            self.backend.send(Request::Lyrics(id.clone()));
        } else {
            let loaded = match hunt.error.take() {
                Some(e) => Loaded::Failed(e),
                None => Loaded::Ready(None),
            };
            self.lyrics.insert(id.clone(), loaded);
            self.lyrics_hunts.remove(id);
        }
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

    /// Spotify's Play on a page: pause or go on when the page is what plays,
    /// else play `tracks` from it, shuffled if shuffle is on.
    pub fn play_page(&mut self, tracks: Vec<Track>, seed: f64) {
        if self.playing_from.as_ref() == Some(&self.page) && self.shown().track.is_some() {
            self.toggle_play();
        } else {
            let shuffle = self.shown().shuffle;
            self.play_shuffled_or_not(tracks, shuffle, seed);
        }
    }

    /// The page shown is what plays, and it is playing.
    pub fn page_playing(&self) -> bool {
        self.playing_from.as_ref() == Some(&self.page) && self.is_playing()
    }

    /// The page the queue was played from, if it was.
    pub fn playing_from(&self) -> Option<&Page> {
        self.playing_from.as_ref()
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
        self.title_of(&self.page)
    }

    /// What a page is called.
    pub fn title_of(&self, page: &Page) -> String {
        match page {
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
        // Apple Music's by their library id; the others' have one id.
        self.library_artists.items.iter().find(|a| {
            a.library_id.as_ref() == Some(id)
                || (a.id.provider != ProviderKind::AppleMusic && a.id == *id)
        })
    }

    /// The artist's page ids on every service the library has them on, by
    /// their name: `id`'s own first.
    pub fn artist_twins(&mut self, id: &ItemId) -> Vec<ItemId> {
        let version = self.library_artists.version;
        if let Some((built, twins)) = self.artist_twins.get(id)
            && *built == version
        {
            return twins.clone();
        }
        let Some(name) = self.library_artist(id).map(|a| fold(a.name.trim())) else {
            return vec![id.clone()];
        };
        let mut twins = vec![id.clone()];
        for a in &self.library_artists.items {
            let page = a.library_id.clone().unwrap_or_else(|| a.id.clone());
            if page != *id && fold(a.name.trim()) == name {
                twins.push(page);
            }
        }
        self.artist_twins
            .insert(id.clone(), (version, twins.clone()));
        twins
    }

    /// The album's copies on the library's other services, by their title
    /// and artist.
    pub fn album_twins(&mut self, id: &ItemId) -> Vec<ItemId> {
        let version = self.library_albums.version;
        if let Some((built, twins)) = self.twins.get(id)
            && *built == version
        {
            return twins.clone();
        }
        let Some(Loaded::Ready((album, _))) = self.albums.get(id) else {
            return Vec::new();
        };
        let key = music_core::dedupe::album_key(album);
        let twins: Vec<ItemId> = self
            .library_albums
            .items
            .iter()
            .filter(|a| a.id.provider != id.provider && music_core::dedupe::album_key(a) == key)
            .map(|a| a.id.clone())
            .collect();
        self.twins.insert(id.clone(), (version, twins.clone()));
        twins
    }

    /// The album's tracks, whole from its copies on every service the
    /// library has it on, asked for the first time; its own until they come.
    pub fn album_tracks(&mut self, id: &ItemId) -> Option<Arc<Vec<Track>>> {
        let own = match self.albums.get(id) {
            Some(Loaded::Ready((_, tracks))) => tracks.clone(),
            _ => return None,
        };
        let mut copies = vec![own.clone()];
        for twin in self.album_twins(id) {
            match self.albums.get(&twin) {
                Some(Loaded::Ready((_, tracks))) => copies.push(tracks.clone()),
                Some(_) => {}
                None => {
                    self.albums.insert(twin.clone(), Loaded::Loading);
                    self.backend.send(Request::Album(twin));
                }
            }
        }
        if copies.len() == 1 {
            return Some(own);
        }
        if let Some((made_of, whole)) = self.whole_albums.get(id)
            && *made_of == copies.len()
        {
            return Some(whole.clone());
        }
        let slices: Vec<&[Track]> = copies.iter().map(|c| c.as_slice()).collect();
        let whole = Arc::new(music_core::dedupe::album_tracks(&slices));
        self.whole_albums
            .insert(id.clone(), (copies.len(), whole.clone()));
        Some(whole)
    }

    /// The copies the Albums (`0`), Artists (`1`) or Recently Added (`2`)
    /// page leaves out: found again when its list changes.
    pub fn copies(&mut self, list: usize) -> Arc<std::collections::HashSet<usize>> {
        let version = match list {
            0 => self.library_albums.version,
            1 => self.library_artists.version,
            _ => self.recently_added.version,
        };
        if let Some((built, copies)) = &self.copies[list]
            && *built == version
        {
            return copies.clone();
        }
        let copies = Arc::new(match list {
            0 => music_core::dedupe::albums(&self.library_albums.items),
            1 => music_core::dedupe::artists(&self.library_artists.items),
            _ => {
                // The albums among them, as albums.
                let (at, albums): (Vec<usize>, Vec<Album>) = self
                    .recently_added
                    .items
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| match c {
                        Collection::Album(a) => Some((i, a.clone())),
                        _ => None,
                    })
                    .unzip();
                music_core::dedupe::albums(&albums)
                    .into_iter()
                    .map(|k| at[k])
                    .collect()
            }
        });
        self.copies[list] = Some((version, copies.clone()));
        copies
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
        self.status.token.is_some()
            && (self.now.as_ref().is_some_and(|n| n.id == track.id)
                || self.now_stands_for.as_ref() == Some(&track.id))
    }

    /// Whether `token` plays `id`, or the server's copy of it.
    fn token_plays(&self, token: Token, id: &ItemId) -> bool {
        self.tracks.get(&token).is_some_and(|t| t.id == *id)
            || self.stands_for.get(&token) == Some(id)
    }

    /// The server's copy of `track`, when it has one: it plays in its place,
    /// being the file itself.
    fn server_copy(&mut self, track: &Track) -> Option<Track> {
        if track.id.provider == ProviderKind::Navidrome
            || !matches!(self.navidrome, SoundCloudState::Connected(_))
        {
            return None;
        }
        if self
            .server_songs
            .as_ref()
            .is_none_or(|(n, _)| *n != self.songs.len())
        {
            let index = music_core::dedupe::Copies::of(
                self.songs
                    .iter()
                    .filter(|t| t.id.provider == ProviderKind::Navidrome),
            );
            self.server_songs = Some((self.songs.len(), index));
        }
        let (_, index) = self.server_songs.as_ref()?;
        index.find(track).cloned()
    }

    /// What `token` is to play for `track`: the server's copy if it has
    /// one, kept as standing for the track listed.
    fn plays_as(&mut self, token: Token, track: Track) -> Track {
        let playing = match self.server_copy(&track) {
            Some(copy) => {
                self.stands_for.insert(token, track.id.clone());
                copy
            }
            None => track,
        };
        self.tracks.insert(token, playing.clone());
        playing
    }

    /// Play `tracks` from the top, or shuffled from a random one.
    pub fn play_shuffled_or_not(&mut self, tracks: Vec<Track>, shuffle: bool, seed: f64) {
        if tracks.is_empty() {
            return;
        }
        if let Some(id) = self.far_id() {
            self.devices
                .node
                .send(&id, connect::Command::SetShuffle { on: shuffle });
        } else if self.queue.shuffle() != shuffle {
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
        if let Some(id) = self.far_id() {
            let start = start as u32;
            self.devices
                .node
                .send(&id, connect::Command::PlayList { tracks, start });
            return;
        }
        self.failed_in_row = 0;
        // Whatever plays now is no longer a station of songs.
        self.station = None;
        self.station_loading = false;
        self.queue.replace(tracks, start);
        self.start_current();
        // A song picked plays, even with the music paused: once what played
        // has stopped, so none of it is heard again.
        self.player.resume();
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
        let track = self.plays_as(token, track);
        self.fetch(token, &track);
        token
    }

    /// Fetch `track` itself, not the server's copy of it.
    fn fetch_as_is(&mut self, track: Track) -> Token {
        let token = self.next_token;
        self.next_token += 1;
        self.fetch(token, &track);
        self.tracks.insert(token, track);
        token
    }

    fn fetch(&self, token: Token, track: &Track) {
        self.backend.send(Request::Fetch {
            token,
            track: track.id.clone(),
            fallback: track.library_id.clone(),
            original: self.soundcloud_originals(),
            spatial: self.spatial_audio(),
        });
    }

    /// Show the track of `token` as Dolby Atmos when `stream` is Spatial
    /// Audio.
    fn says_if_atmos(&mut self, token: Token, stream: &Playable) {
        if let Playable::Track { spatial: true, .. } = stream
            && let Some(track) = self.tracks.get_mut(&token)
        {
            track.quality = Some(music_core::model::AudioQuality {
                codec: music_core::model::Codec::Other("Dolby Atmos".into()),
                sample_rate: Some(48_000),
                bit_depth: None,
                bitrate_kbps: None,
            });
        }
    }

    fn play_now(&mut self, token: Token, stream: Playable) {
        self.playing = Some(token);
        self.player.play(token, Self::source(stream));
        if let Some((_, at)) = self.start_at.take_if(|(t, _)| *t == token) {
            self.player.seek(at);
        }
    }

    /// Show what played when the app last closed, paused where it was; it
    /// is fetched only once played, the services being still on their way.
    fn resume_session(&mut self) {
        let Some(session) = std::fs::read(crate::settings::dir().join("session.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<connect::Handoff>(&b).ok())
        else {
            return;
        };
        let order = session.order.into_iter().map(|i| i as usize).collect();
        self.queue
            .restore(session.tracks, order, session.at as usize, session.shuffle);
        self.queue.set_repeat(session.repeat);
        self.station = session.station;
        self.station_loading = false;
        self.now = self.queue.current().cloned();
        self.resume = self
            .now
            .as_ref()
            .map(|t| (t.id.clone(), Duration::from_millis(session.position_ms)));
    }

    /// Keep what plays for the next time the app opens: the queue and
    /// where it is; written off the interface's thread, but at once when
    /// the app closes.
    fn keep_session(&self, closing: bool) {
        crate::settings::keep_json("session.json", self.handoff(), closing);
    }

    fn now_title(&self, token: Option<Token>) -> String {
        token
            .and_then(|t| self.tracks.get(&t))
            .map(|t| t.title.clone())
            .unwrap_or_default()
    }

    /// A track that couldn't play is skipped (gone from the catalog, not in
    /// this country, a broken file), as Apple Music does; a run of failures
    /// stops, in case nothing plays.
    fn skip_failed(&mut self, title: &str, error: &str, now: f64) {
        // Offline, every song would fail the same: the queue waits on this
        // one, for Play to try again.
        if error.starts_with("network error") {
            self.notify(format!("Couldn't play “{title}”: no connection"), now);
            return;
        }
        self.failed_in_row += 1;
        if self.failed_in_row <= MAX_SKIPS && self.queue.skip_next().is_some() {
            self.notify(format!("Skipped “{title}”: {error}"), now);
            self.start_current();
        } else {
            self.notify(format!("Couldn't play “{title}”: {error}"), now);
        }
    }

    /// The player ran out with nothing lined up behind the track: the next
    /// one follows as soon as it can.
    fn played_out(&mut self) {
        if self.pending_play.is_some() {
            return;
        }
        match self.queued_next {
            // Ready, but too late to join the track: it starts now, and its
            // Started moves the queue on.
            Some(_) if self.next_ready => self.player.skip_to_next(),
            // Still on its way: it plays when it comes.
            Some(token) => {
                self.queued_next = None;
                self.queue.track_finished();
                self.pending_play = Some(token);
            }
            None => {
                if self.queue.track_finished().is_some() {
                    self.start_current();
                }
            }
        }
    }

    fn start_current(&mut self) {
        self.queued_next = None;
        self.next_ready = false;
        let resume = self.resume.take();
        if let Some(track) = self.queue.current().cloned() {
            let resume = resume.filter(|(id, _)| *id == track.id).map(|(_, at)| at);
            // What played stops at once, as in Apple Music, rather than go on
            // over the wait.
            self.player.stop();
            // Fetched already, or on its way, while the pointer rested on it.
            if let Some(warm) = self.warm.take_if(|w| w.track.id == track.id && !w.failed) {
                self.plays_as(warm.token, track);
                if let Some(at) = resume {
                    self.start_at = Some((warm.token, at));
                }
                match warm.ready {
                    Some(stream) => {
                        // Fetched before it was listed to play: Spatial
                        // Audio says so now.
                        self.says_if_atmos(warm.token, &stream);
                        self.pending_play = None;
                        self.play_now(warm.token, stream);
                    }
                    None => self.pending_play = Some(warm.token),
                }
                return;
            }
            let token = self.issue(track);
            self.pending_play = Some(token);
            if let Some(at) = resume {
                self.start_at = Some((token, at));
            }
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
            .is_some_and(|t| self.token_plays(t, &track.id));
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
        let playing = self.server_copy(&track).unwrap_or_else(|| track.clone());
        self.backend.send(Request::Warm {
            token,
            track: playing.id.clone(),
            fallback: playing.library_id.clone(),
            original: self.soundcloud_originals(),
            spatial: self.spatial_audio(),
        });
        self.warm = Some(Warm {
            token,
            track,
            ready: None,
            failed: false,
        });
    }

    /// Have the track after the current one fetched, so it follows without a gap.
    fn prefetch_next(&mut self) {
        // Already lined up (shuffle or repeat changed, and not what follows).
        if let (Some(token), Some(next)) = (self.queued_next, self.queue.peek_finished())
            && self.token_plays(token, &next.id)
        {
            return;
        }
        // The player may have the old one already: it mustn't play.
        if self.queued_next.is_some() {
            self.player.clear_next();
        }
        self.queued_next = self.queue.peek_finished().cloned().map(|t| self.issue(t));
        self.next_ready = false;
    }

    pub fn toggle_play(&mut self) {
        if self.ask_far(connect::Command::Toggle) {
            return;
        }
        if self.status.token.is_none() {
            if self.pending_play.is_none() {
                self.start_current();
            }
        } else if self.status.paused {
            self.player.resume();
        } else {
            self.player.pause();
            self.keep_session(false);
        }
    }

    pub fn next(&mut self) {
        if self.ask_far(connect::Command::Next) {
            return;
        }
        self.failed_in_row = 0;
        // The track lined up to follow is the one Next moves to, unless the
        // current one repeats: play what was fetched for it rather than fetch
        // it again.
        let lined_up = self.queued_next.filter(|&token| {
            self.queue.repeat() != Repeat::One
                && self
                    .queue
                    .peek_finished()
                    .is_some_and(|t| self.token_plays(token, &t.id))
        });
        match lined_up {
            // The player stuck on a read (the network gone): it would wait
            // to switch; the next song is fetched anew instead.
            Some(_) if self.next_ready && self.player.stalled() => {
                if self.queue.skip_next().is_some() {
                    self.start_current();
                }
            }
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
        if self.ask_far(connect::Command::Previous) {
            return;
        }
        self.failed_in_row = 0;
        // The player stuck on a read: the song starts again afresh.
        if self.status.position > Duration::from_secs(3) && self.player.stalled() {
            self.start_current();
        } else if self.status.position > Duration::from_secs(3) {
            self.player.seek(Duration::ZERO);
        } else {
            self.queue.skip_previous();
            self.start_current();
        }
    }

    pub fn seek(&mut self, to: Duration) {
        let position_ms = to.as_millis() as u64;
        if self.ask_far(connect::Command::Seek { position_ms }) {
            return;
        }
        // Not fetched yet: it will go on from there.
        if self.status.token.is_none()
            && let Some((_, at)) = &mut self.resume
        {
            *at = to;
            return;
        }
        self.player.seek(to);
    }

    /// Jump to `to` and play from there.
    pub fn play_from(&mut self, to: Duration) {
        let position_ms = to.as_millis() as u64;
        if self.ask_far(connect::Command::Seek { position_ms }) {
            self.ask_far(connect::Command::Play);
            return;
        }
        self.player.seek(to);
        if self.status.paused {
            self.player.resume();
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        if !volume.is_finite() {
            return;
        }
        // Asked by the device played on itself, it is for here.
        if self.far_id().is_some()
            && self
                .far()
                .is_some_and(|p| p.playing.as_ref().is_some_and(|p| p.volume.is_some()))
        {
            self.far_unmuted = None;
            let volume = volume.clamp(0.0, 1.0);
            self.ask_far(connect::Command::SetVolume { volume });
            return;
        }
        // Next to full is full: the samples then go out untouched.
        self.volume = if volume >= 0.99 { 1.0 } else { volume.max(0.0) };
        self.muted = false;
        self.player.set_volume(self.volume);
        self.volume_changed = Some(std::time::Instant::now());
    }

    pub fn toggle_mute(&mut self) {
        if let Some(volume) = self
            .far()
            .filter(|_| !self.obeying)
            .map(|p| p.playing.as_ref().and_then(|p| p.volume))
        {
            // Muted from here: back to where it was; it can't be set: as it is.
            if let Some(volume) = volume {
                let to = match self.far_unmuted.take() {
                    Some(before) => before,
                    None => {
                        self.far_unmuted = Some(volume);
                        0.0
                    }
                };
                self.ask_far(connect::Command::SetVolume { volume: to });
            }
            return;
        }
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
        if let Some(id) = self.far_id()
            && how != Enqueue::Now
        {
            let command = match how {
                Enqueue::Next => connect::Command::PlayNext { tracks },
                _ => connect::Command::AddToQueue { tracks },
            };
            self.devices.node.send(&id, command);
            return;
        }
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
        // Apple names a playlist's row by its library id; SoundCloud has a
        // track once in a playlist, named by its own.
        let entry = if playlist.provider == ProviderKind::SoundCloud {
            track.id.clone()
        } else {
            let Some(entry) = track.library_id.clone() else {
                return;
            };
            entry
        };
        if let Some(loaded) = self.playlist_tracks.get_mut(playlist) {
            Arc::make_mut(&mut loaded.tracks)
                .retain(|t| t.library_id.as_ref() != Some(&entry) && t.id != entry);
        }
        self.backend.send(Request::RemoveFromPlaylist {
            playlist: playlist.clone(),
            entry,
        });
    }

    /// Queue an album, fetching its tracks first if need be.
    pub fn enqueue_album(&mut self, id: ItemId, how: Enqueue, now: f64) {
        if let Some(tracks) = self.album_tracks(&id) {
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
        if self.ask_far(connect::Command::Jump { k: k as u32 }) {
            return;
        }
        self.failed_in_row = 0;
        if k < self.queue.upcoming_len() {
            self.queue.jump(k);
            self.start_current();
            self.player.resume();
        }
    }

    pub fn remove_upcoming(&mut self, k: usize) {
        if self.ask_far(connect::Command::RemoveUpcoming { k: k as u32 }) {
            return;
        }
        self.queue.remove_upcoming(k);
        if k == 0 {
            self.prefetch_next();
        }
    }

    pub fn clear_upcoming(&mut self) {
        if self.ask_far(connect::Command::ClearUpcoming) {
            return;
        }
        self.queue.clear_upcoming();
        self.prefetch_next();
    }

    pub fn toggle_shuffle(&mut self) {
        let on = !self.shown().shuffle;
        if self.ask_far(connect::Command::SetShuffle { on }) {
            return;
        }
        self.queue.set_shuffle(!self.queue.shuffle());
        self.prefetch_next();
        self.settings.shuffle = Some(self.queue.shuffle());
        self.settings.save();
    }

    pub fn cycle_repeat(&mut self) {
        self.set_repeat(match self.shown().repeat {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::One,
            Repeat::One => Repeat::Off,
        });
    }

    pub fn set_repeat(&mut self, repeat: Repeat) {
        if self.ask_far(connect::Command::SetRepeat { repeat }) {
            return;
        }
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
            Playable::Track { stream, .. } => {
                let len = stream.len();
                let hint = stream.seek_hint();
                let extension = stream.extension();
                // Apple's encoder primes each song with 2112 frames, which
                // its streams don't say.
                player::Source::new(stream, len, Some(extension))
                    .with_seek_hint(hint)
                    .with_priming(apple_music::PRIMING)
            }
            // A station's stream is ADTS, forward only.
            Playable::Live(stream) => player::Source::stream(stream, Some("aac")),
            Playable::Navidrome(audio) => {
                player::Source::new(audio.reader, audio.len, Some(audio.extension))
            }
            Playable::SoundCloud(audio) => {
                let hint = audio.reader.seek_hint();
                let source = player::Source::new(audio.reader, audio.len, Some(audio.extension));
                match hint {
                    // An encrypted track decrypts from where a seek lands.
                    Some(hint) => source.with_seek_hint(hint),
                    None => source,
                }
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
                Reply::ProfilePicture(picture) => self.profile_picture = Some(picture),
                Reply::Connected { storefront } => {
                    // After a sign-in, the library loads now; at start it was
                    // already asked for.
                    let reconnected = self.connection != Connection::Connecting;
                    self.connection = Connection::Connected;
                    self.apple_retry = None;
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
                Reply::SoundCloud(_) if !self.service_on(ProviderKind::SoundCloud) => {}
                Reply::Navidrome(_) if !self.service_on(ProviderKind::Navidrome) => {}
                Reply::SoundCloud(result) => {
                    self.soundcloud = match result {
                        Ok(Some(name)) => SoundCloudState::Connected(name),
                        Ok(None) => SoundCloudState::NotSignedIn,
                        Err(e) => SoundCloudState::Failed(e),
                    };
                }
                Reply::Navidrome(result) => {
                    self.navidrome = match result {
                        Ok(Some(name)) => SoundCloudState::Connected(name),
                        Ok(None) => SoundCloudState::NotSignedIn,
                        Err(e) => SoundCloudState::Failed(e),
                    };
                }
                Reply::SignedInNavidrome(result) => {
                    self.navidrome_signing_in = false;
                    match result {
                        Ok(()) => {
                            self.navidrome_editing = false;
                            self.notify("Signed in to Navidrome", now);
                        }
                        Err(e) => self.notify(format!("Navidrome: {e}"), now),
                    }
                }
                Reply::Starred(ids) => {
                    for id in ids {
                        self.favourites_known.entry(id).or_insert(true);
                    }
                }
                Reply::SignedInSoundCloud(result) => {
                    self.soundcloud_signing_in = false;
                    match result {
                        Ok(()) => self.notify("Signed in to SoundCloud", now),
                        Err(e) => self.notify(format!("Sign-in didn't finish: {e}"), now),
                    }
                }
                Reply::CouldntConnect(e) => {
                    // Said once; tried again, less and less often.
                    if self.apple_retry.is_none() {
                        self.notify(e.clone(), now);
                    }
                    let wait = self
                        .apple_retry
                        .map_or(10.0, |(_, wait)| (wait * 2.0).min(300.0));
                    self.apple_retry = Some((now + wait, wait));
                    self.connection = Connection::Failed(e);
                }
                Reply::Failed(e) => self.notify(e, now),
                // A service turned off since says nothing more.
                Reply::Songs { provider, .. }
                | Reply::Playlists { provider, .. }
                | Reply::LibraryAlbums { provider, .. }
                | Reply::LibraryArtists { provider, .. }
                | Reply::RecentlyAdded { provider, .. }
                    if !self.service_on(provider) => {}
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
                    provider,
                    albums,
                    done,
                    whole,
                } => self
                    .library_albums
                    .arrived_from(albums, done, whole, |a| a.id.provider == provider),
                Reply::LibraryArtists {
                    provider,
                    artists,
                    done,
                    whole,
                } => self
                    .library_artists
                    .arrived_from(artists, done, whole, |a| a.id.provider == provider),
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
                Reply::RecentlyAdded {
                    provider,
                    items,
                    done,
                } => {
                    // Apple Music's come page by page; the server's whole.
                    let whole = provider != ProviderKind::AppleMusic;
                    self.recently_added
                        .arrived_from(items, done, whole, |c| match c {
                            Collection::Album(a) => a.id.provider == provider,
                            Collection::Playlist(p) => p.id.provider == provider,
                            Collection::Station(s) => s.id.provider == provider,
                        });
                }
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
                Reply::Lyrics { id, result } => match result {
                    Ok(Some(lyrics)) => {
                        self.lyrics_hunts.remove(&id);
                        self.lyrics
                            .insert(id, Loaded::Ready(Some(Arc::new(lyrics))));
                    }
                    // The sites may have them, if not asked yet.
                    Ok(None) => self.hunt_lyrics(&id),
                    Err(e) => {
                        if let Some(hunt) = self.lyrics_hunts.get_mut(&id) {
                            hunt.error = Some(e);
                        }
                        self.hunt_lyrics(&id);
                    }
                },
                Reply::SiteLyrics { id, found } => match found {
                    Some((lyrics, _)) => {
                        self.lyrics_hunts.remove(&id);
                        self.lyrics
                            .insert(id, Loaded::Ready(Some(Arc::new(lyrics))));
                    }
                    None => self.hunt_lyrics(&id),
                },
                Reply::Search {
                    scope,
                    source,
                    query,
                    result,
                } => {
                    let loaded = match result {
                        Ok(mut results) => {
                            // Asked before a service was turned off.
                            let on =
                                |id: &ItemId| !self.settings.services_off.contains(&id.provider);
                            results.tracks.retain(|t| on(&t.id));
                            results.albums.retain(|a| on(&a.id));
                            results.artists.retain(|a| on(&a.id));
                            results.playlists.retain(|p| on(&p.id));
                            if scope == SearchScope::Library
                                && source.is_none_or(|s| s == ProviderKind::SoundCloud)
                            {
                                self.with_likes(&mut results, &query);
                            }
                            Loaded::Ready(Arc::new(results))
                        }
                        Err(e) => Loaded::Failed(e),
                    };
                    // Only the last few searches are kept, and those on their
                    // way: the page waiting on one would wait forever.
                    if self.searches.len() >= SEARCHES_KEPT {
                        self.searches.retain(|_, l| matches!(l, Loaded::Loading));
                    }
                    self.searches.insert((scope, source, query), loaded);
                }
                Reply::Fetched { token, result } => {
                    // Spatial Audio says so where the encode shows.
                    if let Ok(stream) = &result {
                        self.says_if_atmos(token, stream);
                    }
                    // An encode below Apple's usual 256 kbps shows as it is,
                    // and is told of when it is to play.
                    if let Ok(Playable::Track {
                        kbps: Some(kbps), ..
                    }) = &result
                        && *kbps < 240
                        && let Some(track) = self.tracks.get_mut(&token)
                    {
                        if let Some(q) = &mut track.quality {
                            q.bitrate_kbps = Some(*kbps);
                        }
                        if self.pending_play == Some(token) || self.queued_next == Some(token) {
                            let title = track.title.clone();
                            self.notify(
                                format!(
                                    "“{title}” plays at {kbps} kbps: Apple has no better of it here"
                                ),
                                now,
                            );
                        }
                    }
                    match result {
                        Ok(stream) if self.pending_play == Some(token) => {
                            self.pending_play = None;
                            self.play_now(token, stream);
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
                            if let Some(warm) = &mut self.warm {
                                warm.failed = true;
                            }
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
                                // The server's copy out of reach: the song as listed.
                                if let Some(listed) = self.stands_for.remove(&token)
                                    && let Some(track) = self.queue.current().cloned()
                                    && track.id == listed
                                {
                                    self.pending_play = Some(self.fetch_as_is(track));
                                } else {
                                    self.skip_failed(&title, &e, now);
                                }
                            } else if self.queued_next == Some(token) {
                                // The next track can't follow: it is tried again,
                                // and skipped if need be, when its turn comes.
                                self.queued_next = None;
                            }
                        }
                    }
                }
            }
        }

        while let Ok(event) = self.player.events().try_recv() {
            match event {
                Event::Started { token, .. } => {
                    if self.playing == Some(token) {
                        self.playing = None;
                    }
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
                        keep_history(self.history.clone());
                    }
                    self.now = started;
                    self.now_stands_for = self.stands_for.get(&token).cloned();
                    self.keep_session(false);
                    let (pending, queued) = (self.pending_play, self.queued_next);
                    let wanted =
                        |t: &Token| *t == token || Some(*t) == pending || Some(*t) == queued;
                    self.tracks.retain(|t, _| wanted(t));
                    self.stands_for.retain(|t, _| wanted(t));
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
                Event::Finished => self.played_out(),
                Event::Error { token, message } => {
                    // A next track the player couldn't open: Next fetches anew.
                    if token.is_some() && token == self.queued_next {
                        self.queued_next = None;
                        self.next_ready = false;
                        // The track before it had already ended, waiting on it:
                        // the queue moves on to it anyway, to fetch it anew.
                        if self.player.status().token.is_none() && self.pending_play.is_none() {
                            self.queue.track_finished();
                            self.start_current();
                        } else {
                            self.notify(message, now);
                        }
                    } else if token.is_some() && token == self.playing {
                        // What was to play now can't: the queue goes on.
                        self.playing = None;
                        let title = self.now_title(token);
                        self.skip_failed(&title, &message, now);
                    } else {
                        self.notify(message, now);
                    }
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
                Action::Play if !self.shown().playing => self.toggle_play(),
                Action::Pause if self.shown().playing => self.toggle_play(),
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

    /// What the desktop's media controls show: another device's music
    /// while it is the one played on, as the app shows it.
    fn media_state(&self) -> crate::mpris::State {
        use crate::mpris::Playback;
        if self.far().is_some() {
            let shown = self.shown();
            return crate::mpris::State {
                length: shown.duration,
                playback: if shown.playing {
                    Playback::Playing
                } else if shown.track.is_some() {
                    Playback::Paused
                } else {
                    Playback::Stopped
                },
                position: shown.position,
                volume: shown.volume.unwrap_or(1.0),
                shuffle: shown.shuffle,
                repeat: shown.repeat,
                track: shown.track,
            };
        }
        self.here_state()
    }

    /// What this computer's player plays.
    fn here_state(&self) -> crate::mpris::State {
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

    /// Do what the other devices asked of this one.
    fn other_devices(&mut self, now: f64) {
        use connect::Event;
        while let Some(event) = self.devices.next_event() {
            match event {
                Event::Changed => {
                    self.devices.read();
                    self.follow();
                }
                Event::AskCode { device } => self.devices.asking = Some((device, String::new())),
                Event::ShowCode { device, code } => self.devices.pairing = Some((device, code)),
                Event::Paired { device, error } => {
                    self.devices.pairing = None;
                    self.devices.asking = None;
                    self.devices.read();
                    match error {
                        None => self.notify(format!("Paired with {}", device.name), now),
                        Some(e) => {
                            self.notify(format!("Not paired with {}: {e}", device.name), now)
                        }
                    }
                }
                Event::Handed { from, handoff } => {
                    self.remote = None;
                    let name = self.device_name(&from);
                    self.take_over(handoff);
                    self.notify(format!("Playing here, from {name}"), now);
                }
                Event::Asked { from, command } => {
                    self.obeying = true;
                    self.obey(from, command, now);
                    self.obeying = false;
                }
            }
        }
    }

    /// Do what another device asked of this computer's player.
    fn obey(&mut self, from: String, command: connect::Command, now: f64) {
        use connect::Command;
        match command {
            Command::Play if !self.is_playing() => self.toggle_play(),
            Command::Pause if self.is_playing() => self.player.pause(),
            Command::Play | Command::Pause => {}
            Command::Toggle => self.toggle_play(),
            Command::Next => self.next(),
            Command::Previous => self.previous(),
            Command::Seek { position_ms } => {
                self.seek(Duration::from_millis(position_ms));
            }
            Command::SetVolume { volume } => self.set_volume(volume),
            Command::SetShuffle { on } => {
                if on != self.queue.shuffle() {
                    self.toggle_shuffle();
                }
            }
            Command::SetRepeat { repeat } => self.set_repeat(repeat),
            Command::Jump { k } => self.jump_to(k as usize),
            Command::RemoveUpcoming { k } => self.remove_upcoming(k as usize),
            Command::ClearUpcoming => self.clear_upcoming(),
            Command::PlayList { tracks, start } => {
                self.play_list(tracks, start as usize);
                // Played from the phone's page, not this one's.
                self.playing_from = None;
            }
            Command::PlayNext { tracks } => self.enqueue(tracks, Enqueue::Next, now),
            Command::AddToQueue { tracks } => self.enqueue(tracks, Enqueue::Last, now),
            Command::PlayStation { station } => self.play_station(&station),
            Command::HandOver => {
                let handoff = self.handoff();
                if self.devices.node.hand_off(&from, handoff) {
                    self.player.pause();
                }
            }
        }
    }

    /// The device played on, when it isn't this computer, as last heard.
    pub fn far(&self) -> Option<&connect::Peer> {
        let id = self.remote.as_ref()?;
        self.devices
            .peers
            .iter()
            .find(|p| &p.device.id == id && p.connected)
    }

    /// The device what is asked goes to, if not this computer's player.
    fn far_id(&self) -> Option<String> {
        if self.obeying {
            return None;
        }
        self.far().map(|p| p.device.id.clone())
    }

    /// Ask the device played on; false if this computer's player is to do it.
    fn ask_far(&self, command: connect::Command) -> bool {
        match self.far_id() {
            Some(id) => {
                self.devices.node.send(&id, command);
                true
            }
            None => false,
        }
    }

    /// Play on the device `id`, or on this computer when `None`: what plays
    /// here moves there, or what plays there comes here, as with Spotify's
    /// devices. A device that plays while this computer doesn't is only
    /// controlled from here.
    pub fn play_on(&mut self, id: Option<String>) {
        let from = self.far().map(|p| p.device.id.clone());
        if id == from {
            return;
        }
        match &id {
            None => {
                if let Some(from) = &from {
                    self.devices.node.send(from, connect::Command::HandOver);
                }
            }
            Some(to) => {
                if from.is_none()
                    && self.is_playing()
                    && self.devices.node.hand_off(to, self.handoff())
                {
                    self.player.pause();
                }
            }
        }
        self.far_unmuted = None;
        self.remote = id;
    }

    /// A device starting to play while this computer is quiet is shown and
    /// controlled here; one that left is no longer.
    fn follow(&mut self) {
        for p in &self.devices.peers {
            let on = p.connected && p.playing.as_ref().is_some_and(|p| p.playing);
            let before = self
                .far_playing
                .insert(p.device.id.clone(), on)
                .unwrap_or(false);
            if on
                && !before
                && self.remote.is_none()
                && !self.is_playing()
                && self.pending_play.is_none()
            {
                self.remote = Some(p.device.id.clone());
            }
        }
        if let Some(id) = &self.remote
            && !self
                .devices
                .peers
                .iter()
                .any(|p| &p.device.id == id && p.connected)
        {
            self.remote = None;
        }
    }

    /// What the player bar and Now Playing show: this computer's player,
    /// or the device played on.
    pub fn shown(&self) -> Shown {
        if let Some(peer) = self.far() {
            let p = peer.playing.clone().unwrap_or_default();
            let mut position = p.position_ms;
            if p.playing {
                position += self.devices.read_at.elapsed().as_millis() as u64;
            }
            if let Some(d) = p.duration_ms {
                position = position.min(d);
            }
            return Shown {
                has_track: p.track.is_some(),
                track: p.track,
                loading: None,
                playing: p.playing,
                position: Duration::from_millis(position),
                duration: p.duration_ms.map(Duration::from_millis),
                shuffle: p.shuffle,
                repeat: p.repeat,
                volume: p.volume,
                muted: p.volume == Some(0.0),
                on: Some(peer.device.clone()),
            };
        }
        Shown {
            track: self.now.clone(),
            has_track: self.queue.current().is_some(),
            loading: self.loading().cloned(),
            playing: self.is_playing(),
            position: self.shown_here_position(),
            duration: self.duration(),
            shuffle: self.queue.shuffle(),
            repeat: self.queue.repeat(),
            volume: Some(self.volume),
            muted: self.muted,
            on: None,
        }
    }

    /// What comes up, here or on the device played on.
    pub fn upcoming_shown(&self) -> Box<dyn Iterator<Item = &Track> + '_> {
        match self.far() {
            Some(peer) => Box::new(peer.upcoming.iter()),
            None => Box::new(self.queue.upcoming()),
        }
    }

    pub fn upcoming_len_shown(&self) -> usize {
        match self.far() {
            Some(peer) => peer.upcoming.len(),
            None => self.queue.upcoming_len(),
        }
    }

    fn device_name(&self, id: &str) -> String {
        self.devices
            .peers
            .iter()
            .find(|p| p.device.id == id)
            .map_or_else(|| "another device".to_owned(), |p| p.device.name.clone())
    }

    /// The queue as it stands, for another device to play on from here.
    fn handoff(&self) -> connect::Handoff {
        let (tracks, order, at) = self.queue.snapshot();
        connect::Handoff {
            tracks,
            order: order.into_iter().map(|i| i as u32).collect(),
            at: at as u32,
            shuffle: self.queue.shuffle(),
            repeat: self.queue.repeat(),
            position_ms: self.shown_here_position().as_millis() as u64,
            playing: self.is_playing(),
            station: self.station.clone(),
        }
    }

    /// Where the song is here: as far as it played, or where the one shown
    /// since the app opened will go on from.
    fn shown_here_position(&self) -> Duration {
        match &self.resume {
            Some((_, at)) if self.status.token.is_none() => *at,
            _ => self.status.position,
        }
    }

    /// Play on from where another device left off.
    fn take_over(&mut self, handoff: connect::Handoff) {
        self.failed_in_row = 0;
        let order = handoff.order.into_iter().map(|i| i as usize).collect();
        self.queue
            .restore(handoff.tracks, order, handoff.at as usize, handoff.shuffle);
        self.queue.set_repeat(handoff.repeat);
        self.playing_from = None;
        self.start_current();
        self.player.resume();
        self.station = handoff.station;
        self.station_loading = false;
        let at = Duration::from_millis(handoff.position_ms);
        match self.pending_play {
            Some(token) => self.start_at = Some((token, at)),
            None => self.seek(at),
        }
    }

    /// Tell the other devices what plays here.
    fn tell_devices(&mut self) {
        let state = self.here_state();
        self.devices.node.publish(connect::Playing {
            live: state.track.is_some() && state.length.is_none(),
            track: state.track,
            playing: state.playback == crate::mpris::Playback::Playing,
            position_ms: state.position.as_millis() as u64,
            duration_ms: state.length.map(|d| d.as_millis() as u64),
            shuffle: state.shuffle,
            repeat: state.repeat,
            volume: Some(state.volume),
        });
        let revision = self.queue.revision();
        if self.devices.upcoming_told != Some(revision) {
            self.devices.upcoming_told = Some(revision);
            self.devices
                .node
                .publish_upcoming(self.queue.upcoming().take(100).cloned().collect());
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
        self.keep_session(true);
        // The sidebars stay as they were left.
        let (collapsed, closed) = (Some(self.sidebar_collapsed), Some(self.side.is_none()));
        let side = self.side.or(self.settings.side);
        if (collapsed, closed, side)
            != (
                self.settings.sidebar_collapsed,
                self.settings.side_closed,
                self.settings.side,
            )
        {
            self.settings.sidebar_collapsed = collapsed;
            self.settings.side_closed = closed;
            self.settings.side = side;
            self.settings.save();
        }
        // A volume still settling is kept as it is.
        if self.volume_changed.is_some() {
            self.settings.volume = Some(self.volume);
            self.settings.muted = Some(self.muted);
            self.settings.save();
        }
    }

    /// What goes on whether the window shows or not: on another workspace
    /// eframe runs only this, and the music, the media keys and the other
    /// devices mustn't wait for the window to come back.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.keep_volume(ctx);
        if self.theme_changed.swap(false, Ordering::Relaxed) {
            let theme = resolve_theme(&self.settings);
            if theme != self.theme {
                theme.apply(ctx);
                self.theme = theme;
            }
        }
        let now = ctx.input(|i| i.time);
        self.pump(now);
        self.media_controls(ctx);
        self.other_devices(now);
        if let Some(edited) = self.search_edited {
            if now - edited >= SEARCH_PAUSE {
                self.submit_search();
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(SEARCH_PAUSE));
            }
        }
        if let Some((at, wait)) = self.apple_retry
            && matches!(self.connection, Connection::Failed(_))
        {
            if now >= at {
                // Its next failure sets the next try.
                self.apple_retry = Some((f64::INFINITY, wait));
                self.backend.send(Request::Reconnect);
            } else {
                ctx.request_repaint_after(Duration::from_secs_f64(at - now));
            }
        }
        // Out of sight a while: the moving covers stop decoding.
        if now - self.drawn_at > 2.0 {
            self.motion_views.clear();
        }
        if self.is_playing()
            || self
                .far()
                .and_then(|p| p.playing.as_ref())
                .is_some_and(|p| p.playing)
        {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        self.mpris.update(self.media_state());
        self.tell_devices();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.zoom(ui.ctx());
        self.drawn_at = ui.input(|i| i.time);
        ui::show(self, ui);
        self.forget_unseen_motion();
        // Every few seconds, the covers long out of sight leave memory.
        let now = ui.input(|i| i.time);
        if now - self.covers_forgotten > 10.0 {
            self.covers_forgotten = now;
            crate::covers::forget_unseen(ui.ctx());
        }
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
            self.lyrics_page = std::env::var_os("MUSIC_SCREENSHOT_LYRICS").is_some();
        }
        // Play a row of the open playlist, or of Songs, once it has loaded.
        if self.screenshot.is_some() {
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
                    .filter_map(|&i| self.songs.get(i).cloned())
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
    }
}

/// The songs played, as kept when the app last closed.
fn read_history() -> Vec<Track> {
    std::fs::read(crate::settings::dir().join("history.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Keep the songs played for the next time the app opens, off the
/// interface's thread.
fn keep_history(history: Vec<Track>) {
    crate::settings::keep_json("history.json", history, false);
}

/// The theme chosen (Stock if it is gone or broken), in its own colours or
/// in Amane's when they are chosen (or, until it is chosen either way, when
/// Amane's file exists). A missing or invalid Amane file leaves the theme's.
fn resolve_theme(settings: &crate::settings::Settings) -> Theme {
    use crate::settings::Appearance;
    let theme = settings
        .theme
        .as_deref()
        .and_then(Theme::named)
        .unwrap_or(Theme::STOCK);
    match settings.appearance {
        Some(Appearance::Stock) => theme,
        Some(Appearance::Amane) | None => match Theme::amane() {
            Some(colors) => theme.recoloured(&colors),
            None => theme,
        },
    }
}

/// Watch Amane's theme file and the theme files the way Kopuz watches its
/// palette: check when they last changed every second and wake the UI when
/// one did.
fn watch_theme(ctx: egui::Context, changed: Arc<AtomicBool>) {
    let amane = Theme::amane_path();
    let themes = Theme::themes_dir();
    let modified = move || {
        let mut times: Vec<_> = std::fs::read_dir(&themes)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| (e.path(), e.metadata().and_then(|m| m.modified()).ok()))
            .collect();
        times.push((
            amane.clone(),
            std::fs::metadata(&amane).and_then(|m| m.modified()).ok(),
        ));
        times.sort();
        times
    };
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
