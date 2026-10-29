//! App state, and the glue between the queue, the backend and the player.

use std::collections::HashMap;
use std::time::Duration;

use music_core::model::{Playlist, ProviderKind, Track};
use music_core::{Queue, Repeat};
use player::{Event, Player, Status, Token};

use crate::backend::{Backend, Reply, Request};
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
    Playlist(usize),
    Settings,
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
    /// The zoom set last, `None` until the first frame knows the screen.
    zoom: Option<f32>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::fonts::install(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let theme = Theme::STOCK;
        theme.apply(&cc.egui_ctx);
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
            screenshot: crate::screenshot::Screenshot::from_env(),
            settings: crate::settings::Settings::load(),
            zoom: None,
        }
    }

    // ---- Navigation ----

    pub fn go(&mut self, page: Page) {
        if page != self.page {
            self.back.push(std::mem::replace(&mut self.page, page));
            self.forward.clear();
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
        }
    }

    pub fn go_forward(&mut self) {
        if let Some(page) = self.forward.pop() {
            self.back.push(std::mem::replace(&mut self.page, page));
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
            Page::Playlist(i) => self
                .playlists
                .get(*i)
                .map(|p| p.name.clone())
                .unwrap_or_default(),
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
                    self.now = self.tracks.get(&token).cloned();
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
        let now = ui.input(|i| i.time);
        self.pump(now);
        ui::show(self, ui);
        let row = self
            .screenshot
            .as_mut()
            .and_then(|s| s.play_row(!self.songs.is_empty()))
            .filter(|&row| row < self.songs_view.order.len());
        if let Some(row) = row {
            let tracks = self
                .songs_view
                .order
                .iter()
                .map(|&i| self.songs[i].clone())
                .collect();
            self.play_list(tracks, row);
        }
        if let Some(screenshot) = &mut self.screenshot {
            screenshot.update(ui.ctx());
        }
        if self.is_playing() {
            ui.ctx().request_repaint_after(Duration::from_millis(250));
        }
    }
}
