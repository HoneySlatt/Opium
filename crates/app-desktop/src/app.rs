//! App state, and the glue between the queue, the backend and the player.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use music_core::model::{Album, ItemId, Playlist, ProviderKind, Track};
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
    Settings,
}

/// Something fetched on demand.
pub enum Loaded<T> {
    Loading,
    Ready(T),
    Failed(String),
}

/// The queue drawer's tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DrawerTab {
    #[default]
    PlayingNext,
    History,
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

pub struct App {
    pub theme: Theme,
    backend: Backend,
    pub player: Player,
    pub connection: Connection,
    pub page: Page,
    back: Vec<Page>,
    forward: Vec<Page>,
    pub sidebar_collapsed: bool,
    pub search: String,
    pub songs: Vec<Track>,
    pub songs_done: bool,
    pub songs_view: SongsView,
    pub playlists: Vec<Playlist>,
    pub albums: HashMap<ItemId, Loaded<AlbumPage>>,
    pub playlist_tracks: HashMap<ItemId, PlaylistTracks>,
    palettes: HashMap<String, PaletteState>,
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
        Self {
            theme,
            backend,
            player: Player::new(),
            connection: Connection::Connecting,
            page: Page::Songs,
            back: Vec::new(),
            forward: Vec::new(),
            sidebar_collapsed: false,
            search: String::new(),
            songs: Vec::new(),
            songs_done: false,
            // Apple returns the library in no useful order; the Music app sorts by title.
            songs_view: SongsView {
                sort: Some((SortKey::Title, true)),
                ..SongsView::default()
            },
            playlists: Vec::new(),
            albums: HashMap::new(),
            playlist_tracks: HashMap::new(),
            palettes: HashMap::new(),
            detail_selected: None,
            drawer_tab: DrawerTab::default(),
            history: Vec::new(),
            queue: Queue::new(),
            tracks: HashMap::new(),
            next_token: 0,
            pending_play: None,
            queued_next: None,
            now: None,
            status: Status::default(),
            volume: 1.0,
            muted: false,
            toast: None,
            drawer_open: std::env::var_os("MUSIC_SCREENSHOT_DRAWER").is_some(),
            signal_open: std::env::var_os("MUSIC_SCREENSHOT_SIGNAL")
                .map(|_| SignalAnchor::PlayerBar),
            now_playing: std::env::var_os("MUSIC_SCREENSHOT_NP").is_some(),
            np_tab: NpTab::default(),
            screenshot: crate::screenshot::Screenshot::from_env(),
            settings,
            theme_changed,
            zoom: None,
        }
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
        match &self.page {
            Page::Album(id) if !self.albums.contains_key(id) => {
                self.albums.insert(id.clone(), Loaded::Loading);
                self.backend.send(Request::Album(id.clone()));
            }
            Page::Playlist(id) if !self.playlist_tracks.contains_key(id) => {
                self.playlist_tracks
                    .insert(id.clone(), PlaylistTracks::default());
                self.backend.send(Request::Playlist(id.clone()));
            }
            _ => {}
        }
    }

    pub fn playlist(&self, id: &ItemId) -> Option<&Playlist> {
        self.playlists.iter().find(|p| &p.id == id)
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
                Reply::Connected => self.connection = Connection::Connected,
                Reply::NotSignedIn => self.connection = Connection::NotSignedIn,
                Reply::Failed(e) => {
                    if self.connection == Connection::Connecting {
                        self.connection = Connection::Failed(e.clone());
                    }
                    self.notify(e, now);
                }
                Reply::Songs { tracks, done } => {
                    self.songs.extend(tracks);
                    self.songs_done = done;
                }
                Reply::Playlists(playlists) => self.playlists = playlists,
                Reply::Album { id, result } => {
                    let loaded = match result {
                        Ok((album, tracks)) => Loaded::Ready((album, Arc::new(tracks))),
                        Err(e) => Loaded::Failed(e),
                    };
                    self.albums.insert(id, loaded);
                }
                Reply::PlaylistTracks { id, result, done } => {
                    let entry = self.playlist_tracks.entry(id).or_default();
                    match result {
                        Ok(tracks) => Arc::make_mut(&mut entry.tracks).extend(tracks),
                        Err(e) => entry.error = Some(e),
                    }
                    entry.done = done;
                }
                Reply::Palette { url, palette } => {
                    let state = palette.map_or(PaletteState::Failed, PaletteState::Ready);
                    self.palettes.insert(url, state);
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
                        if self.pending_play == Some(token) {
                            self.pending_play = None;
                        }
                        let title = self
                            .tracks
                            .get(&token)
                            .map(|t| t.title.clone())
                            .unwrap_or_default();
                        self.notify(format!("Couldn't play “{title}”: {e}"), now);
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
                }
                Event::Finished => {}
                Event::Error { message, .. } => self.notify(message, now),
            }
        }
        self.status = self.player.status();
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
