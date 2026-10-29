//! App state, and the glue between the queue, the backend and the player.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use music_core::model::{
    Album, Artist, ArtistDetail, Collection, FavouriteKind, ItemId, Lyrics, Playlist, ProviderKind,
    SearchResults, Shelf, Track,
};
use music_core::{Queue, Repeat};
use player::{Event, Player, Status, Token};

use crate::backend::{Backend, Reply, Request};
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
    Search(String),
    Settings,
}

/// A list that arrives page by page, fetched on first visit.
pub struct Streamed<T> {
    pub items: Vec<T>,
    pub done: bool,
    requested: bool,
}

impl<T> Default for Streamed<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            done: false,
            requested: false,
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
    }
}

/// Something fetched on demand.
pub enum Loaded<T> {
    Loading,
    Ready(T),
    Failed(String),
}

/// Where Search looks: everything on Apple Music, or the user's library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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

/// An album or playlist waiting for its tracks before going into the queue.
enum Waiting {
    Album(ItemId),
    Playlist(ItemId),
}

/// Now Playing's tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NpTab {
    #[default]
    PlayingNext,
    Lyrics,
}

/// What the signal path popover hangs from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalAnchor {
    /// The quality chip in the player bar: the popover opens above it.
    PlayerBar,
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

/// What `SongsView::order` was built from: the song count, sort and filter.
pub type OrderKey = (usize, Option<(SortKey, bool)>, Option<ProviderKind>);

/// How many tracks in a row may fail to start, and be skipped, before the
/// app stops trying.
const MAX_SKIPS: usize = 5;

/// How long typing pauses before the search runs, in seconds.
const SEARCH_PAUSE: f64 = 0.16;

pub struct App {
    pub theme: Theme,
    backend: Backend,
    pub player: Player,
    pub connection: Connection,
    /// The Apple Music storefront (country) of the account, once connected.
    pub storefront: Option<String>,
    /// The browser sign-in is open, waiting for the user.
    pub signing_in: bool,
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
    pub library_artists: Streamed<Artist>,
    pub recently_added: Streamed<Collection>,
    /// The songs in Apple's playlist of favourites.
    favourite_songs: std::collections::HashSet<ItemId>,
    /// What is known to be a favourite or not: set by the user here, or read
    /// from the service. It wins over `favourite_songs`.
    favourites_known: HashMap<ItemId, bool>,
    /// The home shelves, once asked for.
    pub home: Option<Loaded<Arc<Vec<Shelf>>>>,
    /// The source filter on the cover grids.
    pub grid_source: Option<ProviderKind>,
    pub albums: HashMap<ItemId, Loaded<AlbumPage>>,
    pub artists: HashMap<ItemId, Loaded<Arc<ArtistDetail>>>,
    pub playlist_tracks: HashMap<ItemId, PlaylistTracks>,
    palettes: HashMap<String, PaletteState>,
    lyrics: HashMap<ItemId, Loaded<Option<Arc<Lyrics>>>>,
    /// The selected row on an album or playlist page.
    pub detail_selected: Option<usize>,
    pub drawer_open: bool,
    pub signal_open: Option<SignalAnchor>,
    pub now_playing: bool,
    pub np_tab: NpTab,
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
    /// An album or playlist to queue once its tracks are in.
    waiting: Option<(Waiting, Enqueue)>,
    /// Tracks that failed to start one after the other, skipped.
    failed_in_row: usize,
    /// The track that is audible, or was last.
    pub now: Option<Track>,
    pub status: Status,
    pub volume: f32,
    pub muted: bool,
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
            connection: Connection::Connecting,
            storefront: None,
            signing_in: false,
            page: Page::Home,
            back: Vec::new(),
            forward: Vec::new(),
            sidebar_collapsed: false,
            search: String::new(),
            search_edited: None,
            focus_search: false,
            searches: HashMap::new(),
            search_scope: SearchScope::default(),
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
            library_artists: Streamed::default(),
            recently_added: Streamed::default(),
            favourite_songs: Default::default(),
            favourites_known: HashMap::new(),
            home: None,
            grid_source: None,
            albums: HashMap::new(),
            artists: HashMap::new(),
            playlist_tracks: HashMap::new(),
            palettes: HashMap::new(),
            lyrics: HashMap::new(),
            detail_selected: None,
            drawer_tab: DrawerTab::default(),
            history: Vec::new(),
            queue: Queue::new(),
            tracks: HashMap::new(),
            next_token: 0,
            pending_play: None,
            queued_next: None,
            waiting: None,
            failed_in_row: 0,
            now: None,
            status: Status::default(),
            volume: 1.0,
            muted: false,
            toast: None,
            drawer_open: std::env::var_os("MUSIC_SCREENSHOT_DRAWER").is_some(),
            signal_open: std::env::var_os("MUSIC_SCREENSHOT_SIGNAL")
                .map(|_| SignalAnchor::PlayerBar),
            now_playing: std::env::var_os("MUSIC_SCREENSHOT_NP").is_some(),
            np_tab: match std::env::var("MUSIC_SCREENSHOT_NP").as_deref() {
                Ok("lyrics") => NpTab::Lyrics,
                _ => NpTab::default(),
            },
            screenshot: crate::screenshot::Screenshot::from_env(),
            settings,
            theme_changed,
            zoom: None,
        };
        // The first page loads what it shows, as any page does on arrival.
        app.arrived();
        app
    }

    // ---- Navigation ----

    pub fn go(&mut self, page: Page) {
        if page != self.page {
            self.back.push(std::mem::replace(&mut self.page, page));
            self.forward.clear();
            self.arrived();
        }
    }

    /// Fetch what the page shows, unless it already has been.
    fn arrived(&mut self) {
        self.detail_selected = None;
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

    /// Load the library again, as at start: after signing in anew.
    fn reload_library(&mut self) {
        self.songs.clear();
        self.songs_done = false;
        self.playlists.clear();
        self.library_albums = Streamed::default();
        self.library_artists = Streamed::default();
        self.recently_added = Streamed::default();
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

    /// The interface scale in use, once the first frame has set it.
    pub fn scale(&self) -> Option<f32> {
        self.zoom
    }

    /// Choose the interface scale, or go back to the one that suits the screen.
    pub fn set_scale(&mut self, scale: Option<f32>) {
        self.settings.ui_scale = scale.map(|s| ((s * 10.0).round() / 10.0).clamp(0.75, 3.0));
        self.settings.save();
        // The next frame applies it.
        self.zoom = None;
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
        self.backend.send(Request::SetFavourite { kind, id, on });
    }

    /// The id favourites know an album by: its catalog one.
    pub fn album_favourite_id(album: &Album) -> ItemId {
        album.catalog_id.clone().unwrap_or_else(|| album.id.clone())
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
            Page::Search(_) => "Search".into(),
            Page::Settings => "Settings".into(),
        }
    }

    pub fn notify(&mut self, message: impl Into<String>, now: f64) {
        self.toast = Some((message.into(), now));
    }

    // ---- Playback ----

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
        self.queue.replace(tracks, start);
        self.start_current();
    }

    fn issue(&mut self, track: Track) -> Token {
        let token = self.next_token;
        self.next_token += 1;
        self.backend.send(Request::Fetch {
            token,
            track: track.id.clone(),
        });
        self.tracks.insert(token, track);
        token
    }

    fn start_current(&mut self) {
        self.queued_next = None;
        if let Some(track) = self.queue.current().cloned() {
            self.pending_play = Some(self.issue(track));
        }
    }

    /// Have the track after the current one fetched, so it follows without a gap.
    fn prefetch_next(&mut self) {
        self.queued_next = self.queue.peek_finished().cloned().map(|t| self.issue(t));
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
        if self.queue.skip_next().is_some() {
            self.start_current();
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
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        self.player
            .set_volume(if self.muted { 0.0 } else { self.volume });
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

    /// Queue an album, fetching its tracks first if need be.
    pub fn enqueue_album(&mut self, id: ItemId, how: Enqueue, now: f64) {
        if let Some(Loaded::Ready((_, tracks))) = self.albums.get(&id) {
            let tracks = tracks.to_vec();
            self.enqueue(tracks, how, now);
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
            self.enqueue(tracks, how, now);
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
            && let Some((_, how)) = self.waiting.take()
        {
            self.enqueue(tracks, how, now);
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
    }

    pub fn cycle_repeat(&mut self) {
        self.queue.set_repeat(match self.queue.repeat() {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::One,
            Repeat::One => Repeat::Off,
        });
        self.prefetch_next();
    }

    /// The track's duration from its metadata, else from the decoder.
    pub fn duration(&self) -> Option<Duration> {
        self.now
            .as_ref()
            .and_then(|t| t.duration)
            .or_else(|| self.status.source.as_ref().and_then(|s| s.duration))
    }

    fn source(stream: apple_music::AudioStream) -> player::Source {
        let len = stream.len();
        let hint = stream.seek_hint();
        player::Source::new(stream, len, Some("m4a")).with_seek_hint(hint)
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
                Reply::Failed(e) => {
                    if self.connection != Connection::Connected {
                        self.connection = Connection::Failed(e.clone());
                    }
                    self.notify(e, now);
                }
                Reply::Songs { tracks, done } => {
                    self.songs.extend(tracks);
                    self.songs_done = done;
                }
                Reply::Playlists(playlists) => {
                    self.playlists = playlists;
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
                Reply::LibraryAlbums { albums, done } => self.library_albums.extend(albums, done),
                Reply::LibraryArtists { artists, done } => {
                    self.library_artists.extend(artists, done)
                }
                Reply::Home(result) => {
                    self.home = Some(match result {
                        Ok(shelves) => Loaded::Ready(Arc::new(shelves)),
                        Err(e) => Loaded::Failed(e),
                    });
                }
                Reply::RecentlyAdded { items, done } => self.recently_added.extend(items, done),
                Reply::Album { id, result } => {
                    let loaded = match result {
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
                            Loaded::Ready(Arc::new(artist))
                        }
                        Err(e) => Loaded::Failed(e),
                    };
                    self.artists.insert(id, loaded);
                }
                Reply::PlaylistTracks { id, result, done } => {
                    // The favourites fill in page by page: Apple's playlist of
                    // them can run to thousands of songs.
                    if self.favourites().as_ref() == Some(&id)
                        && let Ok(tracks) = &result
                    {
                        self.favourite_songs
                            .extend(tracks.iter().map(|t| t.id.clone()));
                    }
                    let entry = self.playlist_tracks.entry(id).or_default();
                    match result {
                        Ok(tracks) => Arc::make_mut(&mut entry.tracks).extend(tracks),
                        Err(e) => entry.error = Some(e),
                    }
                    entry.done = done;
                }
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
                }
                Event::Finished => {}
                Event::Error { message, .. } => self.notify(message, now),
            }
        }
        self.status = self.player.status();
        if self.waiting.is_some() {
            self.queue_waiting(now);
        }
    }
}

impl App {
    /// The interface scale: the saved one, or one that suits the screen, changed
    /// with Ctrl +, Ctrl − and Ctrl 0.
    fn zoom(&mut self, ctx: &egui::Context) {
        let (bigger, smaller, reset) = ctx.input_mut(|i| {
            let ctrl = egui::Modifiers::COMMAND;
            (
                i.consume_key(ctrl, egui::Key::Plus) || i.consume_key(ctrl, egui::Key::Equals),
                i.consume_key(ctrl, egui::Key::Minus),
                i.consume_key(ctrl, egui::Key::Num0),
            )
        });
        let current = self.zoom.unwrap_or_else(|| {
            self.settings
                .ui_scale
                .unwrap_or_else(|| crate::settings::default_scale(ctx))
        });
        let wanted = if reset {
            self.settings.ui_scale = None;
            self.settings.save();
            crate::settings::default_scale(ctx)
        } else if bigger || smaller {
            let step = if bigger { 0.1 } else { -0.1 };
            let scale = (((current + step) * 10.0).round() / 10.0).clamp(0.75, 3.0);
            self.settings.ui_scale = Some(scale);
            self.settings.save();
            scale
        } else {
            current
        };
        if self.zoom != Some(wanted) {
            self.zoom = Some(wanted);
            ctx.set_zoom_factor(wanted);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.zoom(ui.ctx());
        if self.theme_changed.swap(false, Ordering::Relaxed) {
            let theme = resolve_theme(&self.settings);
            if theme != self.theme {
                theme.apply(ui.ctx());
                self.theme = theme;
            }
        }
        let now = ui.input(|i| i.time);
        self.pump(now);
        if let Some(edited) = self.search_edited {
            if now - edited >= SEARCH_PAUSE {
                self.submit_search();
            } else {
                ui.ctx()
                    .request_repaint_after(Duration::from_secs_f64(SEARCH_PAUSE));
            }
        }
        ui::show(self, ui);
        if let Some(page) = self
            .screenshot
            .as_mut()
            .and_then(|s| s.page(&self.playlists))
        {
            self.go(page);
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
        if let Some(screenshot) = &mut self.screenshot {
            screenshot.update(ui.ctx());
        }
        if self.is_playing() {
            ui.ctx().request_repaint_after(Duration::from_millis(250));
        }
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
