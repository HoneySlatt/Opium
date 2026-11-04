//! The window's state, as the egui window's `app.rs` keeps it, the part
//! drawn so far: the library's songs, playlists, albums, artists and recent
//! additions as the backend sends them, the Songs page's order and sort,
//! the albums, playlists and library artists opened and their covers'
//! colours, Home's, New's and Radio's shelves; the pages gone to, back and
//! forth; the player, the queue and the last session, the media controls;
//! and the glue to the backend. Its names and shapes are egui's `App`'s,
//! for the screens still to come to slot in.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use desktop::backend::{Backend, Downloading, Own, Playable, Reply, Request};
use desktop::settings::AlbumSort;
use gpui::{Context, FocusHandle, IntoElement, Render, ScrollHandle, Window};
use music_core::model::{
    Album, Artist, ArtistDetail, Collection, FavouriteKind, ItemId, Playlist, ProviderKind, Shelf,
    Station, StationKind, Track,
};
use music_core::{Queue, Repeat};
use player::{Event, Player, Status, Token};

pub use desktop::Side;
pub use music_core::text::fold;

use crate::palette::Palette;
use crate::theme::Theme;
use crate::ui::tracks::{Badge, PageDownload};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
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

/// A page of shelves. (Home, New and Radio, whose See All open one of
/// them in full, come with their pages.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum Shelves {
    Home,
    New,
    Radio,
}

/// A list that arrives page by page, fetched on first visit.
pub struct Streamed<T> {
    pub items: Vec<T>,
    /// Every service that lists it has answered all of it, not the first.
    pub done: bool,
    requested: bool,
    /// The services that have.
    answered: HashSet<ProviderKind>,
    /// Moves on with every change, for what is built from the items.
    pub version: u64,
}

impl<T> Default for Streamed<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            done: false,
            requested: false,
            answered: Default::default(),
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

    /// `provider`'s page, or with `whole` its whole list, replacing its
    /// items only: those `of_it` says are. Done once every one of
    /// `listing` has answered.
    fn arrived_from(
        &mut self,
        provider: ProviderKind,
        (items, done, whole): (Vec<T>, bool, bool),
        listing: &[ProviderKind],
        of_it: impl Fn(&T) -> bool,
    ) {
        if whole {
            self.items.retain(|item| !of_it(item));
        }
        if done {
            self.answered.insert(provider);
        } else {
            self.answered.remove(&provider);
        }
        self.extend(items, listing.iter().all(|p| self.answered.contains(p)));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Connection {
    Connecting,
    Connected,
    NotSignedIn,
    Failed(String),
}

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

/// What Your Library's chips narrow its list to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryFilter {
    #[default]
    Playlists,
    Albums,
    Artists,
}

/// How the Songs page is sorted and filtered.
#[derive(Default)]
pub struct SongsView {
    /// The column and whether it is ascending.
    pub sort: Option<(SortKey, bool)>,
    pub source: Option<ProviderKind>,
    /// Only the songs downloaded, as Spotify's Downloaded and Apple Music's
    /// Downloaded Music.
    pub downloaded: bool,
    /// A position in `order`.
    pub selected: Option<usize>,
    /// Indices into `Music::songs`, sorted and filtered; rebuilt when `order_key` changes.
    pub order: Vec<usize>,
    pub order_key: Option<OrderKey>,
    /// What each search and filter of the songs starts from, made once for
    /// as many songs: the copies across services, each song's words folded.
    pub prepared: Option<PreparedSongs>,
}

#[derive(Default)]
pub struct PreparedSongs {
    pub count: usize,
    /// Made of the downloads, not the library.
    pub downloaded: bool,
    pub copies: HashSet<usize>,
    pub folded: Vec<String>,
}

/// What `SongsView::order` was built from: the song count, sort, source
/// filter and search, and the newest order's version when it is followed.
pub type OrderKey = (
    usize,
    Option<(SortKey, bool)>,
    Option<ProviderKind>,
    String,
    Option<u64>,
    bool,
);

/// Something fetched on demand.
pub enum Loaded<T> {
    Loading,
    Ready(T),
    Failed(String),
}

/// An album and its tracks, as its page shows them.
pub type AlbumPage = (Album, Arc<Vec<Track>>);

/// What the albums' order was built from: the list's version and the sort.
type AlbumsOrderKey = (u64, AlbumSort, bool);

/// An album or a playlist whose songs are on their way, to play once they
/// are in: a card's Play. (Queuing them next or last comes with the menus.)
enum Waiting {
    Album(ItemId),
    Playlist(ItemId),
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

/// A station's songs are topped up when fewer than this are left to come.
const STATION_REFILL: usize = 5;

/// Songs skipped one after the other, failing, before the queue stops.
const MAX_SKIPS: usize = 5;

/// How often the window looks at the player while it plays, as egui's
/// repaint four times a second: the position moves on, an end is seen.
const TICK: Duration = Duration::from_millis(250);

/// A bar of the dock held down and dragged: where it lies in the window,
/// in points.
#[derive(Debug, Clone, Copy)]
pub enum Drag {
    /// The position, its preview following the pointer until let go.
    Seek {
        left: f32,
        width: f32,
        total: Duration,
    },
    /// The volume, set as it goes.
    Volume { left: f32, width: f32 },
}

/// What plays, as shown: on this computer (another device's comes later).
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
    pub on: Option<()>,
}

/// The window and what it stands on.
pub struct Music {
    pub theme: Theme,
    backend: Backend,
    pub player: Player,
    /// The desktop's media controls.
    mpris: desktop::mpris::Mpris,
    /// The user's other devices, on the network.
    devices: desktop::devices::Devices,
    pub kept: synced::Kept,
    /// The last stop on another device shown here, so that it is taken up
    /// once.
    followed: Option<synced::Stamp>,
    settings: desktop::settings::Settings,
    pub connection: Connection,
    /// The account's picture on Apple Music, once read.
    pub profile_picture: Option<music_core::model::Artwork>,
    pub soundcloud: SoundCloudState,
    pub navidrome: SoundCloudState,
    /// The music files of the folders chosen: none, being read, or how many.
    pub local: SoundCloudState,
    /// The services whose songs are still coming.
    songs_coming: HashSet<ProviderKind>,
    pub page: Page,
    back: Vec<Page>,
    forward: Vec<Page>,
    pub sidebar_collapsed: bool,
    /// Files opened from elsewhere, one call each, played together once
    /// they stop coming: when.
    opening: (Vec<PathBuf>, Instant),
    pub songs: Vec<Track>,
    pub songs_done: bool,
    pub songs_view: SongsView,
    pub playlists: Vec<Playlist>,
    pub library_albums: Streamed<Album>,
    /// The library albums in the order the Albums page sorts them, and what
    /// it was sorted from.
    albums_order: Option<(AlbumsOrderKey, Arc<Vec<usize>>)>,
    /// Whether catalog albums and playlists are wholly in the library, as
    /// Apple said.
    in_library: HashMap<ItemId, bool>,
    /// Those to ask Apple about once drawn, and those asked already.
    in_library_wanted: RefCell<Vec<(FavouriteKind, ItemId)>>,
    in_library_asked: HashSet<ItemId>,
    pub library_artists: Streamed<Artist>,
    /// The copies the library's lists leave out, by the version of the list
    /// they were found in: albums, artists, recent additions.
    copies: [Option<(u64, Arc<HashSet<usize>>)>; 3],
    /// Library artists' albums in the library, by artist.
    pub artist_albums: HashMap<ItemId, Loaded<Arc<Vec<Album>>>>,
    pub recently_added: Streamed<Collection>,
    /// Playlists opened from outside the library (a shelf, Recently Added).
    other_playlists: HashMap<ItemId, Playlist>,
    /// The new music page's shelves, once asked for.
    pub new_music: Option<Loaded<Arc<Vec<Shelf>>>>,
    /// The radio page's shelves, once asked for.
    pub radio: Option<Loaded<Arc<Vec<Shelf>>>>,
    /// The home shelves, once asked for.
    pub home: Option<Loaded<Arc<Vec<Shelf>>>>,
    /// The service Home, New and Radio were last asked of.
    browse_from: Option<ProviderKind>,
    /// The source filter on the cover grids.
    pub grid_source: Option<ProviderKind>,
    /// Albums made whole from their copies on other services, and how many
    /// copies they were made from.
    whole_albums: HashMap<ItemId, (usize, Arc<Vec<Track>>)>,
    /// An album's copies on other services, as of a version of the
    /// library's albums; an artist's, of its artists.
    twins: HashMap<ItemId, (u64, Vec<ItemId>)>,
    artist_twins: HashMap<ItemId, (u64, Vec<ItemId>)>,
    /// Catalog artists' pages, as far as they have come: an album's More by
    /// reads them. (The page itself comes later.)
    pub artists: HashMap<ItemId, Loaded<Arc<ArtistDetail>>>,
    /// Catalog albums' other editions and music videos.
    album_extras: HashMap<ItemId, Loaded<Arc<music_core::model::AlbumExtras>>>,
    /// An album or a playlist to play once its songs are in.
    waiting: Option<Waiting>,
    /// The songs in the library, by catalog id.
    library_songs: HashSet<ItemId>,
    /// The songs in Apple's playlist of favourites.
    favourite_songs: HashSet<ItemId>,
    /// What is known to be a favourite or not, read from the service. It
    /// wins over `favourite_songs`.
    favourites_known: HashMap<ItemId, bool>,
    pub albums: HashMap<ItemId, Loaded<AlbumPage>>,
    pub playlist_tracks: HashMap<ItemId, PlaylistTracks>,
    palettes: HashMap<String, PaletteState>,
    /// The selected row on an album or playlist page.
    pub detail_selected: Option<usize>,
    /// Songs downloaded or downloading, by their id: how each stands.
    downloads: HashMap<ItemId, Downloading>,
    /// The songs kept to play offline, the last downloaded first.
    pub downloaded: Arc<Vec<Track>>,
    /// Where each library song comes when the last added are first: its
    /// service's place among them, and its own within it.
    pub newest: HashMap<ItemId, (u8, u32)>,
    /// Moves on with every list of them that comes.
    pub newest_version: u64,
    newest_asked: bool,
    /// What the page shown is searched for.
    pub page_search: String,
    /// The column beside the page, when it shows.
    pub side: Option<Side>,
    /// The lyrics over the page, in a theme that shows them there.
    pub lyrics_page: bool,
    pub now_playing: bool,
    pub queue: Queue,
    tracks: HashMap<Token, Track>,
    /// The listed track each token plays the server's copy of, when it does.
    stands_for: HashMap<Token, ItemId>,
    /// What the track playing stands for, when it is the server's copy.
    now_stands_for: Option<ItemId>,
    /// The server's songs by title and artist, and how many songs the
    /// library had when they were indexed.
    server_songs: Option<(usize, music_core::dedupe::Copies)>,
    /// Every library song by what copies share, as it was at so many songs:
    /// another device's file plays from one here.
    library_copies: Option<(usize, music_core::dedupe::Copies)>,
    next_token: Token,
    /// Fetching, to play as soon as it arrives.
    pending_play: Option<Token>,
    /// Where a track goes on from once it plays: the last session's song.
    start_at: Option<(Token, Duration)>,
    /// The song that played when the app last closed, shown paused, and
    /// where it was: it goes on from there once played.
    resume: Option<(ItemId, Duration)>,
    /// Fetching or fetched, to follow the current track.
    queued_next: Option<Token>,
    /// The player has `queued_next` as its next track, fetched.
    next_ready: bool,
    /// Paused while the song was still loading: it starts paused.
    hold_paused: bool,
    /// Tracks that failed to start one after the other, skipped.
    failed_in_row: usize,
    /// The track handed to the player to play at once, until it starts: an
    /// error before then means it can't play at all.
    playing: Option<Token>,
    /// The station of songs playing, which the queue keeps topping up.
    station: Option<ItemId>,
    /// More of its songs are on their way.
    station_loading: bool,
    /// The page the queue was played from.
    playing_from: Option<Page>,
    /// The track that is audible, or was last.
    pub now: Option<Track>,
    pub status: Status,
    pub volume: f32,
    pub muted: bool,
    /// When the volume last changed, until it is kept in the settings.
    volume_changed: Option<Instant>,
    /// What the toast says, and since when: drawn with the screens to come.
    pub toast: Option<(String, Instant)>,
    // What only this window keeps, of egui's `ui::data`.
    pub library_filter: LibraryFilter,
    /// The pointer is over Your Library: its fold button shows.
    pub library_hovered: bool,
    /// Each chip's list keeps its own place, as egui's scroll areas by id.
    pub library_scroll: [ScrollHandle; 3],
    pub songs_scroll: ScrollHandle,
    /// Each album and playlist keeps its own place, as egui's scroll areas
    /// salted by the page.
    page_scroll: HashMap<Page, ScrollHandle>,
    /// The cover grids whose source filter shows, its title clicked.
    pub sources_shown: HashSet<Page>,
    /// Where each shelf is scrolled sideways, by its name.
    pub shelf_scroll: HashMap<String, crate::ui::grid::ShelfScroll>,
    // What GPUI asks for besides.
    /// Takes the keys: the window's own, as egui's input is.
    pub focus: FocusHandle,
    /// A look at the player is on its way, while it plays.
    ticking: bool,
    /// A bar of the dock being dragged.
    pub dragging: Option<Drag>,
    /// Where a drag along the position would go, shown while it lasts.
    pub seek_preview: Option<Duration>,
    /// What closing keeps is kept: once.
    exited: bool,
}

impl Music {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // The workings wake the window from their own threads: one wake
        // waiting is enough, the window then reads all that came.
        let (woken, wakes) = async_channel::bounded::<()>(1);
        let wake: desktop::Wake = Arc::new(move || {
            let _ = woken.try_send(());
        });
        cx.spawn_in(window, async move |this, cx| {
            while wakes.recv().await.is_ok() {
                if this
                    .update_in(cx, |this, window, cx| this.logic(window, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        // Closing keeps where the music stopped, as egui's `on_exit`.
        let this = cx.weak_entity();
        window.on_window_should_close(cx, move |_, cx| {
            this.update(cx, |this, _| this.on_exit()).ok();
            true
        });

        std::thread::spawn(desktop::covers::trim);
        let settings = desktop::settings::Settings::load();
        let theme = crate::theme::resolve_theme(&settings);
        // The history kept before it was shared starts it, the last first.
        let kept = synced::Kept::open(&desktop::settings::dir(), || {
            let mut history = read_history();
            history.reverse();
            history
        });
        let devices =
            desktop::devices::Devices::start(wake.clone(), kept.clone(), settings.relay.clone());
        let own = Own::new(kept.clone(), devices.node.sharer());
        let backend = Backend::start(wake.clone(), settings.services_off.clone(), own);
        backend.send(Request::SetLocalFolders(settings.local_folders.clone()));
        backend.send(Request::Library);
        // Started on files (the file manager's Open With): they play.
        let opened = desktop::mpris::files_from_args();
        if !opened.is_empty() {
            backend.send(Request::OpenFiles(opened));
        }
        let mut app = Self {
            theme,
            backend,
            player: Player::new(),
            mpris: desktop::mpris::Mpris::start(wake),
            devices,
            kept,
            followed: None,
            connection: if settings.services_off.contains(&ProviderKind::AppleMusic) {
                Connection::NotSignedIn
            } else {
                Connection::Connecting
            },
            profile_picture: None,
            soundcloud: SoundCloudState::Connecting,
            navidrome: SoundCloudState::Connecting,
            local: if settings.local_folders.is_empty() {
                SoundCloudState::NotSignedIn
            } else {
                SoundCloudState::Connecting
            },
            songs_coming: HashSet::new(),
            // The slice's one page.
            page: Page::Songs,
            back: Vec::new(),
            forward: Vec::new(),
            sidebar_collapsed: settings.sidebar_collapsed.unwrap_or(false),
            opening: (Vec::new(), Instant::now()),
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
            in_library: HashMap::new(),
            in_library_wanted: RefCell::default(),
            in_library_asked: HashSet::new(),
            library_artists: Streamed::default(),
            copies: Default::default(),
            artist_albums: HashMap::new(),
            recently_added: Streamed::default(),
            other_playlists: HashMap::new(),
            new_music: None,
            radio: None,
            home: None,
            browse_from: None,
            grid_source: settings
                .grid_source
                .filter(|p| !settings.services_off.contains(p)),
            whole_albums: HashMap::new(),
            twins: HashMap::new(),
            artist_twins: HashMap::new(),
            artists: HashMap::new(),
            album_extras: HashMap::new(),
            waiting: None,
            library_songs: HashSet::new(),
            favourite_songs: HashSet::new(),
            favourites_known: HashMap::new(),
            albums: HashMap::new(),
            playlist_tracks: HashMap::new(),
            palettes: HashMap::new(),
            detail_selected: None,
            downloads: HashMap::new(),
            downloaded: Arc::default(),
            newest: HashMap::new(),
            newest_version: 0,
            newest_asked: false,
            page_search: String::new(),
            // As left: open on what it showed, unless it was closed.
            side: match settings.side_closed {
                Some(true) => None,
                _ => Some(settings.side.unwrap_or(Side::NowPlaying)),
            },
            lyrics_page: false,
            now_playing: false,
            queue: Queue::new(),
            tracks: HashMap::new(),
            stands_for: HashMap::new(),
            now_stands_for: None,
            server_songs: None,
            library_copies: None,
            next_token: 0,
            pending_play: None,
            start_at: None,
            resume: None,
            queued_next: None,
            next_ready: false,
            hold_paused: false,
            failed_in_row: 0,
            playing: None,
            station: None,
            station_loading: false,
            playing_from: None,
            now: None,
            status: Status::default(),
            volume: settings.volume.unwrap_or(1.0).clamp(0.0, 1.0),
            muted: settings.muted.unwrap_or(false),
            volume_changed: None,
            toast: None,
            library_filter: LibraryFilter::default(),
            library_hovered: false,
            library_scroll: Default::default(),
            songs_scroll: ScrollHandle::new(),
            page_scroll: HashMap::new(),
            sources_shown: HashSet::new(),
            shelf_scroll: HashMap::new(),
            focus: cx.focus_handle(),
            ticking: false,
            dragging: None,
            seek_preview: None,
            exited: false,
            settings,
        };
        app.focus.focus(window, cx);
        // The volume, shuffle and repeat as they were left.
        app.player
            .set_volume(if app.muted { 0.0 } else { app.volume });
        app.queue.set_shuffle(app.settings.shuffle.unwrap_or(false));
        if let Some(repeat) = app.settings.repeat {
            app.queue.set_repeat(repeat.into());
        }
        app.resume_session();
        app
    }

    /// The scale the interface is drawn at, 1.0 for 100 %.
    pub fn scale(&self) -> f32 {
        self.settings
            .ui_scale
            .filter(|s| s.is_finite())
            .unwrap_or(1.0)
            .clamp(0.7, 2.5)
    }

    /// Show where the music last stopped, here or on another device,
    /// paused; it is fetched only once played, the services being still on
    /// their way. The session kept here before the devices shared it is
    /// taken in first, if none is shared yet.
    fn resume_session(&mut self) {
        if self.kept.session().is_none() {
            let path = desktop::settings::dir().join("session.json");
            let at = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
            let session = std::fs::read_to_string(&path).ok().and_then(|text| {
                serde_json::from_str::<music_core::Handoff>(&navidrome::scrub_kept(&text)).ok()
            });
            if let (Some(session), Some(at)) = (session, at) {
                self.kept.take_old_session(session, at.as_millis() as u64);
            }
        }
        if let Some((session, stamp)) = self.kept.session() {
            self.followed = Some(stamp);
            self.show_session(session);
        }
    }

    /// Another device stopped later than this one did, or moved on: what
    /// shows here follows, paused, unless music plays or loads here.
    fn follow_session(&mut self) {
        let Some((session, stamp)) = self.kept.session() else {
            return;
        };
        if stamp.device == self.kept.me()
            || self.followed.as_ref() == Some(&stamp)
            || self.is_playing()
            || self.pending_play.is_some()
        {
            return;
        }
        self.followed = Some(stamp);
        if self.status.token.is_some() {
            self.player.stop();
        }
        self.queued_next = None;
        self.next_ready = false;
        self.now_stands_for = None;
        self.show_session(session);
    }

    fn show_session(&mut self, session: music_core::Handoff) {
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

    /// Keep what plays for the next time the app opens, here or on another
    /// device: the queue and where it is; written off the interface's
    /// thread, but at once when the app closes.
    fn keep_session(&self, closing: bool) {
        self.devices.node.share(self.kept.left(self.handoff()));
        if closing {
            self.kept.flush();
        }
    }

    /// The queue as it stands, for another device to play on from here.
    fn handoff(&self) -> music_core::Handoff {
        let (tracks, order, at) = self.queue.snapshot();
        music_core::Handoff {
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

    /// As the app closes, as egui's `on_exit`: where the music stopped is
    /// kept at once, and a volume still settling as it is.
    fn on_exit(&mut self) {
        if std::mem::replace(&mut self.exited, true) {
            return;
        }
        self.keep_session(true);
        if self.volume_changed.is_some() {
            self.settings.volume = Some(self.volume);
            self.settings.muted = Some(self.muted);
            self.settings.save();
        }
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
            ProviderKind::Local => !matches!(self.local, SoundCloudState::NotSignedIn),
            ProviderKind::Music => false,
        };
        [
            ProviderKind::AppleMusic,
            ProviderKind::SoundCloud,
            ProviderKind::Navidrome,
            ProviderKind::Local,
        ]
        .into_iter()
        .filter(|&p| self.service_on(p) && signed_in(p))
        .collect()
    }

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

    /// One of the library's playlists, or one opened from outside it.
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

    /// Whether an album or a playlist is in the library, if known: a library
    /// one is, a catalog one as Apple says, asked the first time it is shown.
    fn collection_in_library(&self, kind: FavouriteKind, id: &ItemId) -> Option<bool> {
        // A SoundCloud playlist is in the library once liked or made; its
        // albums aren't kept there yet.
        if id.provider == ProviderKind::SoundCloud {
            return Some(
                kind != FavouriteKind::Album && self.playlists.iter().any(|p| p.id == *id),
            );
        }
        let library = match kind {
            FavouriteKind::Album => id.id.starts_with("l."),
            _ => id.id.starts_with("p."),
        };
        if library {
            return Some(true);
        }
        let known = self.in_library.get(id).copied();
        if known.is_none() && !self.in_library_asked.contains(id) {
            self.in_library_wanted.borrow_mut().push((kind, id.clone()));
        }
        known
    }

    /// Ask Apple, in one request a kind, about the covers drawn last.
    pub fn ask_in_library(&mut self) {
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

    /// Whether Add to Library is offered for a playlist.
    pub fn can_add_playlist(&self, id: &ItemId) -> bool {
        self.collection_in_library(FavouriteKind::Playlist, id) == Some(false)
    }

    /// Whether Add to Library is offered for an album.
    pub fn can_add_album(&self, id: &ItemId) -> bool {
        self.collection_in_library(FavouriteKind::Album, id) == Some(false)
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

    /// The catalog id an album's favourites, Suggest Less, radio and link
    /// go by: none for one the user uploaded.
    pub fn album_catalog_id(album: &Album) -> Option<ItemId> {
        album
            .catalog_id
            .clone()
            .or_else(|| (!album.id.id.starts_with("l.")).then(|| album.id.clone()))
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

    /// Ask for the library's albums, unless they were asked for already:
    /// the library sidebar lists them without their page being opened.
    pub fn want_library_albums(&mut self) {
        if self.library_albums.first_ask() {
            self.backend.send(Request::LibraryAlbums);
        }
    }

    /// The same for the library's artists.
    pub fn want_library_artists(&mut self) {
        if self.library_artists.first_ask() {
            self.backend.send(Request::LibraryArtists);
        }
    }

    /// The services whose albums, artists and recent additions are listed
    /// (Apple Music's and the server's), signed in or on their way to.
    fn listing_services(&self) -> Vec<ProviderKind> {
        let mut listing = Vec::new();
        if self.service_on(ProviderKind::AppleMusic)
            && matches!(
                self.connection,
                Connection::Connecting | Connection::Connected
            )
        {
            listing.push(ProviderKind::AppleMusic);
        }
        if self.service_on(ProviderKind::Navidrome)
            && matches!(
                self.navidrome,
                SoundCloudState::Connecting | SoundCloudState::Connected(_)
            )
        {
            listing.push(ProviderKind::Navidrome);
        }
        if self.service_on(ProviderKind::Local)
            && matches!(
                self.local,
                SoundCloudState::Connecting | SoundCloudState::Connected(_)
            )
        {
            listing.push(ProviderKind::Local);
        }
        listing
    }

    pub fn recent_by_date(&self) -> bool {
        self.settings.recent_by_date.unwrap_or(false)
    }

    /// Recently Added each service's together, in the same order every
    /// time; then, by date if so set, merged by when each was added, each
    /// service's own order kept.
    fn order_recent(&mut self) {
        self.recently_added.items.sort_by_key(|c| {
            match match c {
                Collection::Album(a) => a.id.provider,
                Collection::Playlist(p) => p.id.provider,
                Collection::Station(s) => s.id.provider,
            } {
                ProviderKind::AppleMusic => 0,
                ProviderKind::SoundCloud => 1,
                ProviderKind::Navidrome => 2,
                ProviderKind::Local => 3,
                ProviderKind::Music => 4,
            }
        });
        if self.recent_by_date() {
            let items = std::mem::take(&mut self.recently_added.items);
            self.recently_added.items = music_core::recent::in_time_order(items);
        }
        // What was built from the items, by place, is built again.
        self.recently_added.version += 1;
    }

    /// Albums' and Recently Added's source, kept for next time.
    pub fn set_grid_source(&mut self, source: Option<ProviderKind>) {
        self.grid_source = source;
        self.settings.grid_source = source;
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

    /// The copies the Albums (`0`), Artists (`1`) or Recently Added (`2`)
    /// page leaves out: found again when its list changes.
    pub fn copies(&mut self, list: usize) -> Arc<HashSet<usize>> {
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

    /// How the Albums page is sorted, and whether ascending.
    pub fn albums_sort(&self) -> (AlbumSort, bool) {
        (
            self.settings.albums_sort.unwrap_or(AlbumSort::Name),
            self.settings.albums_ascending.unwrap_or(true),
        )
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
            AlbumSort::Name => {
                let names: Vec<String> = (0..albums.len()).map(name).collect();
                order.sort_by(|&a, &b| direct(names[a].cmp(&names[b])));
            }
            AlbumSort::Artist => {
                let keys: Vec<(String, String)> = (0..albums.len())
                    .map(|i| (fold(&albums[i].artists.join(", ")), name(i)))
                    .collect();
                order.sort_by(|&a, &b| direct(keys[a].cmp(&keys[b])));
            }
            AlbumSort::Added | AlbumSort::Released => {
                let date = |i: usize| match by {
                    AlbumSort::Added => albums[i].added.clone(),
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

    // ---- Home, New and Radio's shelves ----

    /// The services connected now that can fill Home, New and Radio.
    pub fn browse_sources(&self) -> Vec<ProviderKind> {
        let connected = |provider| match provider {
            ProviderKind::AppleMusic => matches!(self.connection, Connection::Connected),
            ProviderKind::SoundCloud => matches!(self.soundcloud, SoundCloudState::Connected(_)),
            ProviderKind::Navidrome => matches!(self.navidrome, SoundCloudState::Connected(_)),
            // The files have no picks.
            ProviderKind::Local | ProviderKind::Music => false,
        };
        [
            ProviderKind::AppleMusic,
            ProviderKind::SoundCloud,
            ProviderKind::Navidrome,
        ]
        .into_iter()
        .filter(|&p| self.service_on(p) && connected(p))
        .collect()
    }

    /// Where Home, New and Radio come from: the one chosen if connected,
    /// else Apple Music, else the first connected.
    pub fn browse_source(&self) -> Option<ProviderKind> {
        let connected = self.browse_sources();
        self.settings
            .browse_source
            .filter(|s| connected.contains(s))
            .or_else(|| {
                connected
                    .iter()
                    .copied()
                    .find(|&p| p == ProviderKind::AppleMusic)
            })
            .or_else(|| connected.first().copied())
    }

    /// The pages' service is another now (chosen, or connected): what they
    /// showed goes, to be asked of it on arriving.
    fn browse_changed(&mut self) {
        let source = self.browse_source();
        if source != self.browse_from {
            self.browse_from = source;
            self.home = None;
            self.new_music = None;
            self.radio = None;
        }
    }

    /// Whether a service that could fill Home, New or Radio is still
    /// connecting, so that the pages wait on it.
    fn browse_connecting(&self) -> bool {
        [&self.soundcloud, &self.navidrome]
            .iter()
            .any(|s| matches!(s, SoundCloudState::Connecting))
            || matches!(self.connection, Connection::Connecting)
    }

    /// On a page of shelves, their service another now (chosen, or just
    /// connected): asked of it. Whether it was. (Home, New and Radio
    /// themselves come with their pages; one of their shelves in full is
    /// here.)
    fn browse_page(&mut self) -> bool {
        let shelves = match &self.page {
            Page::Shelf(which, _) => Some(*which),
            _ => None,
        };
        let Some(which) = shelves else { return false };
        // Asked while every service was still connecting, the page was left
        // waiting on one: once none is connecting any more, it is asked
        // again, so that a first run says to connect a service in Settings
        // rather than turning for ever.
        let waiting_on_nothing = self.browse_from.is_none()
            && self.browse_source().is_none()
            && !self.browse_connecting()
            && !matches!(
                self.shelves(which),
                Some(Loaded::Ready(_)) | Some(Loaded::Failed(_))
            );
        if self.browse_source() == self.browse_from && !waiting_on_nothing {
            return false;
        }
        self.browse_changed();
        self.refresh(which);
        true
    }

    /// Ask `which` of the pages' service, or say none is connected.
    fn ask_browse(&mut self, request: fn(ProviderKind) -> Request) -> Loaded<Arc<Vec<Shelf>>> {
        self.browse_changed();
        match self.browse_from {
            Some(source) => {
                self.backend.send(request(source));
                Loaded::Loading
            }
            // One connecting: asked once it has (see `browse_page`).
            None if self.browse_connecting() => Loaded::Loading,
            None => Loaded::Failed("Connect a service in Settings to see its picks here.".into()),
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
        let request = match which {
            Shelves::Home => Request::Home,
            Shelves::New => Request::NewMusic,
            Shelves::Radio => Request::Radio,
        };
        let shelves = Some(self.ask_browse(request));
        match which {
            Shelves::Home => self.home = shelves,
            Shelves::New => self.new_music = shelves,
            Shelves::Radio => self.radio = shelves,
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

    pub fn badge(&self, id: &ItemId) -> Option<Badge> {
        Some(match self.downloads.get(id)? {
            Downloading::Queued => Badge::Queued,
            Downloading::Coming(p) => Badge::Coming(*p),
            Downloading::Kept(downloads::store::Standing::Expired) => Badge::Expired,
            Downloading::Kept(_) => Badge::Kept,
            Downloading::Failed(_) => Badge::Failed,
            Downloading::Gone => return None,
        })
    }

    /// Whether `track` can't play now: its service is out of reach and it
    /// isn't downloaded (or no longer plays, its subscription unconfirmed).
    pub fn unplayable(&self, track: &Track) -> bool {
        let reached = match track.id.provider {
            ProviderKind::AppleMusic => self.connection == Connection::Connected,
            ProviderKind::SoundCloud => matches!(self.soundcloud, SoundCloudState::Connected(_)),
            ProviderKind::Navidrome => matches!(self.navidrome, SoundCloudState::Connected(_)),
            // Another device's file plays from a copy here, if there is one.
            ProviderKind::Local => {
                !Self::elsewhere(track)
                    || self
                        .library_copies
                        .as_ref()
                        .is_none_or(|(_, copies)| copies.best(track).is_some())
            }
            ProviderKind::Music => true,
        };
        !reached
            && !matches!(
                self.downloads.get(&track.id),
                Some(Downloading::Kept(
                    downloads::store::Standing::Plays | downloads::store::Standing::PlaysUntil(_)
                ))
            )
    }

    /// Every service's songs in the order they were added, asked once.
    pub fn want_newest_songs(&mut self) {
        if !self.newest_asked {
            self.newest_asked = true;
            self.backend.send(Request::NewestSongs);
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

    /// Where a page is scrolled to: its own, kept as long as the window, as
    /// egui keeps its scroll areas'.
    pub fn page_scroll(&mut self, page: &Page) -> ScrollHandle {
        self.page_scroll.entry(page.clone()).or_default().clone()
    }

    /// A page's download button for its `tracks`, if any can be.
    pub fn page_download(&self, tracks: &[Track]) -> Option<PageDownload> {
        let (mut total, mut kept, mut came, mut coming) = (0usize, 0usize, 0.0f32, false);
        for t in tracks.iter().filter(|t| self.downloadable(&t.id)) {
            total += 1;
            match self.downloads.get(&t.id) {
                Some(Downloading::Kept(_)) => {
                    kept += 1;
                    came += 1.0;
                }
                Some(Downloading::Coming(p)) => {
                    coming = true;
                    came += p;
                }
                Some(Downloading::Queued) => coming = true,
                _ => {}
            }
        }
        Some(match () {
            _ if total == 0 => return None,
            _ if coming => PageDownload::Coming(came / total as f32),
            _ if kept == total => PageDownload::Kept,
            _ => PageDownload::Offer,
        })
    }

    /// Whether `id` can be downloaded: a service's song, not a station, nor
    /// a file here already.
    pub fn downloadable(&self, id: &ItemId) -> bool {
        !matches!(id.provider, ProviderKind::Music | ProviderKind::Local)
            && !id.id.starts_with("ra.")
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

    /// The track's duration from its metadata, else from the decoder.
    pub fn duration(&self) -> Option<Duration> {
        self.now
            .as_ref()
            .and_then(|t| t.duration)
            .or_else(|| self.status.source.as_ref().and_then(|s| s.duration))
    }

    /// What the player bar shows: this computer's player (the device
    /// played on, when it isn't, comes with Music Connect).
    pub fn shown(&self) -> Shown {
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

    /// The songs the Songs page lists from: the library's, or those downloaded.
    pub fn listed(&self) -> &[Track] {
        if self.songs_view.downloaded {
            &self.downloaded
        } else {
            &self.songs
        }
    }

    /// A column title clicked: its sort, the next way round.
    pub fn sort_songs(&mut self, key: SortKey) {
        self.songs_view.sort = crate::ui::tracks::cycle(self.songs_view.sort, key);
        self.songs_view.selected = None;
    }

    pub fn rebuild_order(&mut self) {
        let view = &mut self.songs_view;
        let query = fold(self.page_search.trim());
        // Spotify's look and M3E list the last added first, whatever was sorted
        // in another.
        let newest_only =
            self.theme.layout.header == crate::theme::Header::Banner || self.theme.expressive();
        let newest = newest_only.then_some(self.newest_version);
        let sort = if newest_only { None } else { view.sort };
        let downloaded = view.downloaded;
        let songs: &[Track] = if downloaded {
            &self.downloaded
        } else {
            &self.songs
        };
        let key = Some((
            songs.len(),
            sort,
            view.source,
            query.clone(),
            newest,
            downloaded,
        ));
        if view.order_key == key {
            return;
        }
        view.order_key = key;
        // Made again only when songs come or go, not at each key typed.
        if view
            .prepared
            .as_ref()
            .is_none_or(|p| p.count != songs.len() || p.downloaded != downloaded)
        {
            view.prepared = Some(PreparedSongs {
                count: songs.len(),
                downloaded,
                copies: music_core::dedupe::songs(songs),
                folded: songs
                    .iter()
                    .map(|t| {
                        fold(&format!(
                            "{} {} {}",
                            t.title,
                            t.artists.join(" "),
                            t.album.as_deref().unwrap_or_default()
                        ))
                    })
                    .collect(),
            });
        }
        let prepared = view.prepared.as_ref().expect("just made");
        // Every word searched for is in the title, the artists or the album.
        let words: Vec<&str> = query.split_whitespace().collect();
        let found =
            |i: usize| words.is_empty() || words.iter().all(|w| prepared.folded[i].contains(w));
        // Across services, one copy of a song; within one, all of its own.
        let mut order: Vec<usize> = (0..songs.len())
            .filter(|&i| {
                view.source.is_none_or(|s| songs[i].id.provider == s)
                    && (view.source.is_some() || !prepared.copies.contains(&i))
                    && found(i)
            })
            .collect();
        if let Some((by, ascending)) = sort {
            // Each song's key made once, not at each comparison.
            let text = |t: &Track| match by {
                SortKey::Title => t.title.to_lowercase(),
                SortKey::Artist => t.artists.join(", ").to_lowercase(),
                SortKey::Album => t.album.as_deref().unwrap_or("").to_lowercase(),
                SortKey::Duration => String::new(),
            };
            match by {
                SortKey::Duration => order.sort_by_key(|&i| songs[i].duration),
                _ => order.sort_by_cached_key(|&i| text(&songs[i])),
            }
            if !ascending {
                order.reverse();
            }
        } else if newest.is_some() {
            // Those not listed among the newest yet, after.
            order.sort_by_key(|&i| {
                self.newest
                    .get(&songs[i].id)
                    .copied()
                    .unwrap_or((u8::MAX, 0))
            });
        }
        if order.len() != view.order.len() {
            view.selected = None;
        }
        view.order = order;
    }

    pub fn soundcloud_originals(&self) -> bool {
        self.settings.soundcloud_originals.unwrap_or(true)
    }

    pub fn spatial_audio(&self) -> bool {
        self.settings.spatial_audio.unwrap_or(true)
    }

    /// Say `message` for a while (its toast is drawn with the screens to
    /// come; until then it goes to the log).
    pub fn notify(&mut self, message: impl Into<String>) {
        let message = message.into();
        eprintln!("music: {message}");
        self.toast = Some((message, Instant::now()));
    }

    /// Play the music files `paths` name (a folder, those under it), from
    /// the file manager or the command line.
    pub fn open_files(&mut self, paths: Vec<PathBuf>) {
        if !paths.is_empty() {
            self.backend.send(Request::OpenFiles(paths));
        }
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

    /// Fetch what the page shows, unless it already has been. (The pages
    /// still to come ask for theirs as they come.)
    fn arrived(&mut self) {
        self.detail_selected = None;
        self.page_search.clear();
        // Favourite Songs is Apple's playlist of them, once the library says which.
        if self.page == Page::Favourites
            && let Some(id) = self.favourites()
        {
            self.page = Page::Playlist(id);
        }
        if self.browse_page() {
            return;
        }
        match &self.page {
            // Not asked yet, or failed (offline, a service not connected
            // yet): asked again on arriving.
            Page::Album(id)
                if !matches!(
                    self.albums.get(id),
                    Some(Loaded::Loading | Loaded::Ready(_))
                ) =>
            {
                self.albums.insert(id.clone(), Loaded::Loading);
                self.backend.send(Request::Album(id.clone()));
            }
            Page::Shelf(which, _) if self.shelves(*which).is_none() => self.refresh(*which),
            Page::Albums if self.library_albums.first_ask() => {
                self.backend.send(Request::LibraryAlbums);
            }
            Page::Artists if self.library_artists.first_ask() => {
                self.backend.send(Request::LibraryArtists);
            }
            Page::RecentlyAdded if self.recently_added.first_ask() => {
                self.backend.send(Request::RecentlyAdded);
            }
            // Their releases on every service the library has them on.
            Page::LibraryArtist(id) => {
                let id = id.clone();
                for twin in self.artist_twins(&id) {
                    if !matches!(
                        self.artist_albums.get(&twin),
                        Some(Loaded::Loading | Loaded::Ready(_))
                    ) {
                        self.artist_albums.insert(twin.clone(), Loaded::Loading);
                        self.backend.send(Request::LibraryArtistAlbums(twin));
                    }
                }
            }
            // A catalog playlist says whether it is a favourite when asked.
            Page::Playlist(id)
                if id.id.starts_with("pl.") && !self.favourites_known.contains_key(id) =>
            {
                if self
                    .playlist_tracks
                    .get(id)
                    .is_none_or(|p| p.error.is_some())
                {
                    self.playlist_tracks
                        .insert(id.clone(), PlaylistTracks::default());
                    self.backend.send(Request::Playlist(id.clone()));
                }
                self.backend.send(Request::Favourite {
                    kind: FavouriteKind::Playlist,
                    id: id.clone(),
                });
            }
            Page::Playlist(id)
                if self
                    .playlist_tracks
                    .get(id)
                    .is_none_or(|p| p.error.is_some()) =>
            {
                self.playlist_tracks
                    .insert(id.clone(), PlaylistTracks::default());
                self.backend.send(Request::Playlist(id.clone()));
            }
            _ => {}
        }
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

    /// The page's title, as the bar over it says it once scrolled.
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

    // ---- Playback ----

    /// Play the page's songs, or pause or go on with them if they are what
    /// plays: its Play button.
    pub fn play_page(&mut self, tracks: Vec<Track>) {
        if self.playing_from.as_ref() == Some(&self.page) && self.shown().track.is_some() {
            self.toggle_play();
        } else {
            let shuffle = self.shown().shuffle;
            self.play_shuffled_or_not(tracks, shuffle, seed());
        }
    }

    /// Whether `token` plays `id`, or the server's copy of it.
    fn token_plays(&self, token: Token, id: &ItemId) -> bool {
        self.tracks.get(&token).is_some_and(|t| t.id == *id)
            || self.stands_for.get(&token) == Some(id)
    }

    /// The server's copy of `track`, when it has one: it plays in its place,
    /// being the file itself.
    fn server_copy(&mut self, track: &Track) -> Option<Track> {
        // A file here plays as it is.
        if matches!(
            track.id.provider,
            ProviderKind::Navidrome | ProviderKind::Local
        ) || !matches!(self.navidrome, SoundCloudState::Connected(_))
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

    /// Whether `track` is another device's file (a phone's, in a playlist of
    /// Music's): not here to play.
    fn elsewhere(track: &Track) -> bool {
        track.id.provider == ProviderKind::Local && !std::path::Path::new(&track.id.id).is_file()
    }

    /// The copy this computer has of another device's file: its own file,
    /// the server's, a service's.
    fn copy_here(&mut self, track: &Track) -> Option<Track> {
        if !Self::elsewhere(track) {
            return None;
        }
        if self
            .library_copies
            .as_ref()
            .is_none_or(|(n, _)| *n != self.songs.len())
        {
            let index = music_core::dedupe::Copies::of(&self.songs);
            self.library_copies = Some((self.songs.len(), index));
        }
        let (_, index) = self.library_copies.as_ref()?;
        index.best(track).cloned()
    }

    /// What `token` is to play for `track`: the server's copy if it has
    /// one, kept as standing for the track listed.
    fn plays_as(&mut self, token: Token, track: Track) -> Track {
        let copy = self.server_copy(&track).or_else(|| self.copy_here(&track));
        let playing = match copy {
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

    /// Play an album now, as a card's Play: its songs fetched first if need
    /// be.
    pub fn enqueue_album(&mut self, id: ItemId) {
        if let Some(tracks) = self.album_tracks(&id) {
            self.play_from(tracks.to_vec(), Page::Album(id));
            return;
        }
        if !matches!(self.albums.get(&id), Some(Loaded::Loading)) {
            self.albums.insert(id.clone(), Loaded::Loading);
            self.backend.send(Request::Album(id.clone()));
        }
        self.waiting = Some(Waiting::Album(id));
    }

    /// Play a playlist now, as a card's Play: its songs fetched first if
    /// need be.
    pub fn enqueue_playlist(&mut self, playlist: &Playlist) {
        self.other_playlists
            .entry(playlist.id.clone())
            .or_insert_with(|| playlist.clone());
        let id = playlist.id.clone();
        if let Some(entry) = self.playlist_tracks.get(&id)
            && entry.done
        {
            let tracks = entry.tracks.to_vec();
            self.play_from(tracks, Page::Playlist(id));
            return;
        }
        if !self.playlist_tracks.contains_key(&id) {
            self.playlist_tracks
                .insert(id.clone(), PlaylistTracks::default());
            self.backend.send(Request::Playlist(id.clone()));
        }
        self.waiting = Some(Waiting::Playlist(id));
    }

    /// Play the album or playlist that was waiting, once its tracks are in.
    fn queue_waiting(&mut self) {
        let tracks = match &self.waiting {
            Some(Waiting::Album(id)) => match self.albums.get(id) {
                Some(Loaded::Ready((_, tracks))) => Some(tracks.to_vec()),
                Some(Loaded::Failed(_)) => Some(Vec::new()),
                _ => None,
            },
            Some(Waiting::Playlist(id)) => self
                .playlist_tracks
                .get(id)
                .filter(|p| p.done)
                .map(|p| p.tracks.to_vec()),
            None => None,
        };
        if let Some(tracks) = tracks
            && let Some(waiting) = self.waiting.take()
        {
            let from = match waiting {
                Waiting::Album(id) => Page::Album(id),
                Waiting::Playlist(id) => Page::Playlist(id),
            };
            self.play_from(tracks, from);
        }
    }

    /// An album's or a playlist's songs played now, as egui's `enqueue_from`
    /// with `Enqueue::Now`: they play from its page.
    fn play_from(&mut self, tracks: Vec<Track>, from: Page) {
        if tracks.is_empty() {
            return;
        }
        self.play_list(tracks, 0);
        self.playing_from = Some(from);
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
        self.player.play(token, source(stream));
        if let Some((_, at)) = self.start_at.take_if(|(t, _)| *t == token) {
            self.player.seek(at);
        }
        if std::mem::take(&mut self.hold_paused) {
            self.player.pause();
        }
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
    fn skip_failed(&mut self, title: &str, error: &str) {
        // Offline, every song would fail the same: the queue waits on this
        // one, for Play to try again.
        if error.starts_with("network error") {
            self.notify(format!("Couldn't play “{title}”: no connection"));
            return;
        }
        // Nor is there any point skipping on when it is the sound itself
        // that can't be opened: every song would go the same way, and the
        // queue would be run through in silence.
        if let Some(why) = error.strip_prefix("audio output: ") {
            self.notify(format!(
                "Music found no sound card to play through: {why}. It plays through PipeWire, which doesn't seem to be running."
            ));
            return;
        }
        self.failed_in_row += 1;
        if self.failed_in_row <= MAX_SKIPS && self.queue.skip_next().is_some() {
            self.notify(format!("Skipped “{title}”: {error}"));
            self.start_current();
        } else {
            self.notify(format!("Couldn't play “{title}”: {error}"));
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
        // Another song asked for: it plays.
        self.hold_paused = false;
        self.queued_next = None;
        self.next_ready = false;
        let resume = self.resume.take();
        if let Some(track) = self.queue.current().cloned() {
            let resume = resume.filter(|(id, _)| *id == track.id).map(|(_, at)| at);
            // What played stops at once, as in Apple Music, rather than go on
            // over the wait.
            self.player.stop();
            // The song the pointer rested on, fetched ahead, comes later.
            let token = self.issue(track);
            self.pending_play = Some(token);
            if let Some(at) = resume {
                self.start_at = Some((token, at));
            }
        }
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
        if self.status.token.is_none() {
            if self.pending_play.is_none() {
                self.start_current();
            } else {
                // Still loading: it starts as asked last, playing or paused.
                self.hold_paused = !self.hold_paused;
            }
        } else if self.status.paused {
            self.player.resume();
        } else {
            self.player.pause();
            self.keep_session(false);
        }
    }

    pub fn next(&mut self) {
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
        // Not fetched yet: it will go on from there.
        if self.status.token.is_none()
            && let Some((_, at)) = &mut self.resume
        {
            *at = to;
            return;
        }
        self.player.seek(to);
    }

    pub fn set_volume(&mut self, volume: f32) {
        if !volume.is_finite() {
            return;
        }
        // Next to full is full: the samples then go out untouched.
        self.volume = if volume >= 0.99 { 1.0 } else { volume.max(0.0) };
        self.muted = false;
        self.player.set_volume(self.volume);
        self.volume_changed = Some(Instant::now());
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        self.player
            .set_volume(if self.muted { 0.0 } else { self.volume });
        self.volume_changed = Some(Instant::now());
    }

    /// Keep the volume for next time, once it has stopped moving: a drag of
    /// the slider changes it at every move.
    fn keep_volume(&mut self) {
        const SETTLED: Duration = Duration::from_millis(500);
        let Some(changed) = self.volume_changed else {
            return;
        };
        if changed.elapsed() < SETTLED {
            return;
        }
        self.volume_changed = None;
        self.settings.volume = Some(self.volume);
        self.settings.muted = Some(self.muted);
        self.settings.save();
    }

    pub fn toggle_shuffle(&mut self) {
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
        self.queue.set_repeat(repeat);
        self.prefetch_next();
        self.settings.repeat = Some(self.queue.repeat().into());
        self.settings.save();
    }

    /// A bar of the dock held down at `x` (points from the window's left):
    /// as egui's click and drag, the volume follows at once, the position
    /// shows where it would go until let go.
    pub fn start_drag(&mut self, drag: Drag, x: f32, window: &mut Window, cx: &mut Context<Self>) {
        self.dragging = Some(drag);
        self.drag_to(x, window, cx);
    }

    pub fn drag_to(&mut self, x: f32, window: &mut Window, cx: &mut Context<Self>) {
        match self.dragging {
            Some(Drag::Seek { left, width, total }) => {
                self.seek_preview = Some(total.mul_f32(((x - left) / width).clamp(0.0, 1.0)));
                cx.notify();
            }
            Some(Drag::Volume { left, width }) => {
                self.set_volume((x - left) / width);
                self.acted(window, cx);
            }
            None => {}
        }
    }

    /// Let go at `x`: the position goes where it was dragged to.
    pub fn end_drag(&mut self, x: f32, window: &mut Window, cx: &mut Context<Self>) {
        self.drag_to(x, window, cx);
        if let (Some(Drag::Seek { .. }), Some(to)) = (self.dragging, self.seek_preview.take()) {
            self.seek(to);
        }
        self.dragging = None;
        self.acted(window, cx);
    }

    // ---- What goes on, drawn or not ----

    /// What goes on whether the window is drawn or not, as egui's `logic`:
    /// the workings' replies and the player's events taken in, the media
    /// controls obeyed and told what plays; while the music plays, again
    /// in a moment.
    pub fn logic(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.keep_volume();
        self.pump();
        self.media_controls(window, cx);
        self.other_devices();
        self.mpris.update(self.media_state());
        self.keep_ticking(window, cx);
        cx.notify();
    }

    /// Something was asked of the player from the window: shown at once,
    /// and again once the player has taken it in, as egui looks again 50 ms
    /// after a media key.
    pub fn acted(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.logic(window, cx);
        self.look_again(Duration::from_millis(50), window, cx);
    }

    fn look_again(&self, after: Duration, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(after).await;
            this.update_in(cx, |this, window, cx| this.logic(window, cx))
                .ok();
        })
        .detach();
    }

    /// Whether the player is to be looked at again soon: it plays, or a
    /// song is on its way, or something waits on time passing.
    fn wants_tick(&self) -> bool {
        self.is_playing()
            || self.pending_play.is_some()
            || self.playing.is_some()
            || self.volume_changed.is_some()
            || !self.opening.0.is_empty()
    }

    fn keep_ticking(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.ticking || !self.wants_tick() {
            return;
        }
        self.ticking = true;
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(TICK).await;
            this.update_in(cx, |this, window, cx| {
                this.ticking = false;
                this.logic(window, cx);
            })
            .ok();
        })
        .detach();
    }

    /// Carry out what the desktop's media controls asked for.
    fn media_controls(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use desktop::mpris::Action;
        let mut acted = false;
        // Files opened one by one (a file manager's selection): played together.
        let mut opened = Vec::new();
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
                Action::Raise => window.activate_window(),
                Action::Quit => {
                    self.on_exit();
                    window.remove_window();
                }
                Action::Open(path) => opened.push(path),
            }
        }
        // The player takes it in a moment: look again then, to show it.
        if acted {
            self.look_again(Duration::from_millis(50), window, cx);
        }
        let now = Instant::now();
        if !opened.is_empty() {
            self.opening.0.extend(opened);
            self.opening.1 = now + Duration::from_millis(250);
        }
        if !self.opening.0.is_empty() && now >= self.opening.1 {
            let files = std::mem::take(&mut self.opening.0);
            self.open_files(files);
        }
    }

    /// What the other devices said, of what this window takes up so far:
    /// where the music stopped on them, as egui's `other_devices` follows
    /// it once synced. Their pairing, handoffs and remote control come with
    /// Music Connect's screens.
    fn other_devices(&mut self) {
        while let Some(event) = self.devices.next_event() {
            if let connect::Event::Synced = event {
                self.follow_session();
            }
        }
    }

    /// What the desktop's media controls show: this computer's player
    /// (another device's, while it is the one played on, comes with Music
    /// Connect).
    fn media_state(&self) -> desktop::mpris::State {
        self.here_state()
    }

    /// What this computer's player plays.
    fn here_state(&self) -> desktop::mpris::State {
        use desktop::mpris::Playback;
        desktop::mpris::State {
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

    /// What the workings and the player have said since last time.
    fn pump(&mut self) {
        while let Some(reply) = self.backend.try_recv() {
            match reply {
                Reply::Downloads(mut kept) => {
                    self.downloads
                        .retain(|_, state| !matches!(state, Downloading::Kept(_)));
                    kept.sort_by_key(|k| std::cmp::Reverse(k.added));
                    for k in &kept {
                        self.downloads
                            .insert(k.track.id.clone(), Downloading::Kept(k.standing));
                    }
                    self.downloaded = Arc::new(kept.into_iter().map(|k| k.track).collect());
                }
                Reply::Download { id, state } => match state {
                    Downloading::Gone => {
                        self.downloads.remove(&id);
                        if self.downloaded.iter().any(|t| t.id == id) {
                            self.downloaded = Arc::new(
                                self.downloaded
                                    .iter()
                                    .filter(|t| t.id != id)
                                    .cloned()
                                    .collect(),
                            );
                        }
                    }
                    state => {
                        self.downloads.insert(id, state);
                    }
                },
                Reply::ProfilePicture(picture) => self.profile_picture = Some(picture),
                Reply::Connected { .. } => {
                    self.connection = Connection::Connected;
                    self.browse_page();
                }
                Reply::NotSignedIn => self.connection = Connection::NotSignedIn,
                Reply::CouldntConnect(e) => self.connection = Connection::Failed(e),
                Reply::SoundCloud(_) if !self.service_on(ProviderKind::SoundCloud) => {}
                Reply::Navidrome(_) if !self.service_on(ProviderKind::Navidrome) => {}
                Reply::SoundCloud(result) => {
                    self.soundcloud = match result {
                        Ok(Some(name)) => SoundCloudState::Connected(name),
                        Ok(None) => SoundCloudState::NotSignedIn,
                        Err(e) => SoundCloudState::Failed(e),
                    };
                    self.browse_page();
                }
                Reply::Navidrome(result) => {
                    self.navidrome = match result {
                        Ok(Some(name)) => SoundCloudState::Connected(name),
                        Ok(None) => SoundCloudState::NotSignedIn,
                        Err(e) => SoundCloudState::Failed(e),
                    };
                    self.browse_page();
                }
                Reply::Starred(ids) => {
                    for id in ids {
                        self.favourites_known.entry(id).or_insert(true);
                    }
                }
                Reply::Files(state) => {
                    self.local = match state {
                        None => SoundCloudState::NotSignedIn,
                        Some((0, true)) => SoundCloudState::Connecting,
                        Some((songs, _)) => SoundCloudState::Connected(format!(
                            "{songs} song{}",
                            if songs == 1 { "" } else { "s" }
                        )),
                    };
                }
                // A service turned off since says nothing more.
                Reply::Songs { provider, .. } | Reply::Playlists { provider, .. }
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
                Reply::NewestSongs { provider, ids } => {
                    let rank = match provider {
                        ProviderKind::AppleMusic => 0,
                        ProviderKind::SoundCloud => 1,
                        _ => 2,
                    };
                    self.newest.retain(|id, _| id.provider != provider);
                    self.newest.extend(
                        ids.into_iter()
                            .enumerate()
                            .map(|(i, id)| (id, (rank, i as u32))),
                    );
                    self.newest_version += 1;
                }
                Reply::Playlists {
                    provider,
                    playlists,
                } => {
                    // A service's in place of its others: Music's own first,
                    // then Apple Music's.
                    self.playlists.retain(|p| p.id.provider != provider);
                    let at = match provider {
                        ProviderKind::Music => 0,
                        ProviderKind::AppleMusic => self
                            .playlists
                            .iter()
                            .take_while(|p| p.id.provider == ProviderKind::Music)
                            .count(),
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
                } => {
                    let listing = self.listing_services();
                    self.library_albums.arrived_from(
                        provider,
                        (albums, done, whole),
                        &listing,
                        |a| a.id.provider == provider,
                    );
                }
                Reply::LibraryArtists {
                    provider,
                    artists,
                    done,
                    whole,
                } => {
                    let listing = self.listing_services();
                    self.library_artists.arrived_from(
                        provider,
                        (artists, done, whole),
                        &listing,
                        |a| a.id.provider == provider,
                    );
                }
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
                // Asked of another service since: not shown.
                Reply::NewMusic { source, .. }
                | Reply::Radio { source, .. }
                | Reply::Home { source, .. }
                    if Some(source) != self.browse_from => {}
                Reply::NewMusic { result, .. } => {
                    self.new_music = Some(match result {
                        Ok(shelves) => Loaded::Ready(Arc::new(shelves)),
                        Err(e) => Loaded::Failed(e),
                    });
                }
                Reply::Radio { result, .. } => {
                    self.radio = Some(match result {
                        Ok(shelves) => Loaded::Ready(Arc::new(shelves)),
                        Err(e) => Loaded::Failed(e),
                    });
                }
                Reply::Home { result, .. } => {
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
                    // Each service's whole, in place of what it said before.
                    let whole = true;
                    let mut listing = self.listing_services();
                    if self.service_on(ProviderKind::SoundCloud)
                        && matches!(
                            self.soundcloud,
                            SoundCloudState::Connecting | SoundCloudState::Connected(_)
                        )
                    {
                        listing.push(ProviderKind::SoundCloud);
                    }
                    self.recently_added.arrived_from(
                        provider,
                        (items, done, whole),
                        &listing,
                        |c| match c {
                            Collection::Album(a) => a.id.provider == provider,
                            Collection::Playlist(p) => p.id.provider == provider,
                            Collection::Station(s) => s.id.provider == provider,
                        },
                    );
                    self.order_recent();
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
                Reply::InLibrary(known) => self.in_library.extend(known),
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
                    let favourites = self.favourites().as_ref() == Some(&id);
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
                    // The favourites whole: those no longer in it lose their mark.
                    if favourites && done && entry.error.is_none() {
                        self.favourite_songs = entry.tracks.iter().map(|t| t.id.clone()).collect();
                    }
                }
                // Whether a playlist is a favourite, asked on opening it
                // (changing it comes with the favourites' buttons).
                Reply::Favourite {
                    id,
                    set: None,
                    result: Ok(Some(on)),
                } => {
                    self.favourites_known.entry(id).or_insert(on);
                }
                Reply::Palette { url, palette } => {
                    let state = palette.map_or(PaletteState::Failed, |p| {
                        PaletteState::Ready(crate::palette::colors(p))
                    });
                    self.palettes.insert(url, state);
                }
                Reply::Failed(e) => self.notify(e),
                Reply::Opened(tracks) => {
                    if tracks.is_empty() {
                        self.notify("Nothing there plays here");
                    } else {
                        self.play_shuffled_or_not(tracks, false, seed());
                    }
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
                            Err(e) => self.notify(format!("The station stopped: {e}")),
                        }
                    }
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
                            self.notify(format!(
                                "“{title}” plays at {kbps} kbps: Apple has no better of it here"
                            ));
                        }
                    }
                    match result {
                        Ok(stream) if self.pending_play == Some(token) => {
                            self.pending_play = None;
                            self.play_now(token, stream);
                        }
                        Ok(stream) if self.queued_next == Some(token) => {
                            self.player.set_next(token, source(stream));
                            self.next_ready = true;
                        }
                        // Superseded while it was fetching.
                        Ok(_) => {}
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
                                    self.skip_failed(&title, &e);
                                }
                            } else if self.queued_next == Some(token) {
                                // The next track can't follow: it is tried again,
                                // and skipped if need be, when its turn comes.
                                self.queued_next = None;
                            }
                        }
                    }
                }
                // The rest is for screens still to come.
                _ => {}
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
                        self.devices.node.share(self.kept.played(previous));
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
                            self.notify(message);
                        }
                    } else if token.is_some() && token == self.playing {
                        // What was to play now can't: the queue goes on.
                        self.playing = None;
                        let title = self.now_title(token);
                        self.skip_failed(&title, &message);
                    } else {
                        self.notify(message);
                    }
                }
            }
        }
        self.status = self.player.status();
        if self.waiting.is_some() {
            self.queue_waiting();
        }
    }
}

impl Render for Music {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::ui::set_zoom(self.scale() * desktop::settings::SCALE_UNIT);
        crate::covers::forget_unseen(window, cx);
        let shown = crate::ui::show(self, window, cx);
        // The covers just drawn that Apple hasn't said about yet.
        self.ask_in_library();
        shown
    }
}

/// What the player plays of what was fetched.
fn source(playable: Playable) -> player::Source {
    match playable {
        Playable::Local(audio) => {
            player::Source::new(audio.file, audio.len, Some(&audio.extension))
        }
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
        Playable::Stored(song) => {
            let song = *song;
            let source = player::Source::new(song.reader, song.len, Some(&song.extension));
            match song.priming {
                0 => source,
                frames => source.with_priming(frames),
            }
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

/// What picks the song a shuffled list starts from, as egui's clock: the
/// seconds since 1970.
fn seed() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

/// The history as it was kept before devices shared it.
fn read_history() -> Vec<Track> {
    std::fs::read_to_string(desktop::settings::dir().join("history.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&navidrome::scrub_kept(&text)).ok())
        .unwrap_or_default()
}
