//! The window's state, as the egui window's `app.rs` keeps it, the part this
//! slice draws: the library's songs and playlists as the backend sends them,
//! the Songs page's order and sort, the last session's song; and the glue
//! to the backend. Its names and shapes are egui's `App`'s, for the screens
//! still to come to slot in.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use desktop::backend::{Backend, Downloading, Own, Reply, Request};
use gpui::{Context, IntoElement, Render, ScrollHandle, Window};
use music_core::model::{FavouriteKind, ItemId, Playlist, ProviderKind, Track};
use music_core::{Queue, Repeat};

pub use desktop::Side;
pub use music_core::text::fold;

use crate::theme::Theme;
use crate::ui::tracks::Badge;

#[derive(Debug, Clone, PartialEq)]
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
    _devices: desktop::devices::Devices,
    _mpris: desktop::mpris::Mpris,
    pub kept: synced::Kept,
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
    pub songs: Vec<Track>,
    pub songs_done: bool,
    pub songs_view: SongsView,
    pub playlists: Vec<Playlist>,
    /// The songs in the library, by catalog id.
    library_songs: HashSet<ItemId>,
    /// The songs in Apple's playlist of favourites.
    favourite_songs: HashSet<ItemId>,
    /// What is known to be a favourite or not, read from the service. It
    /// wins over `favourite_songs`.
    favourites_known: HashMap<ItemId, bool>,
    /// Apple's playlist of favourites' songs, as they come.
    favourites_tracks: Option<(ItemId, Vec<ItemId>)>,
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
    /// The track that is audible, or was last.
    pub now: Option<Track>,
    /// The song that played when the app last closed, shown paused, and
    /// where it was.
    resume: Option<(ItemId, Duration)>,
    /// The page the queue was played from.
    playing_from: Option<Page>,
    pub volume: f32,
    pub muted: bool,
    // What only this window keeps, of egui's `ui::data`.
    pub library_filter: LibraryFilter,
    /// The pointer is over Your Library: its fold button shows.
    pub library_hovered: bool,
    /// Each chip's list keeps its own place, as egui's scroll areas by id.
    pub library_scroll: [ScrollHandle; 3],
    pub songs_scroll: ScrollHandle,
}

impl Music {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // The workings wake the window from their own threads: one wake
        // waiting is enough, the window then reads all that came.
        let (woken, wakes) = async_channel::bounded::<()>(1);
        let wake: desktop::Wake = Arc::new(move || {
            let _ = woken.try_send(());
        });
        cx.spawn(async move |this, cx| {
            while wakes.recv().await.is_ok() {
                if this.update(cx, |this, cx| this.pump(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();

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
        let mut app = Self {
            theme,
            backend,
            _devices: devices,
            _mpris: desktop::mpris::Mpris::start(wake),
            kept,
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
            library_songs: HashSet::new(),
            favourite_songs: HashSet::new(),
            favourites_known: HashMap::new(),
            favourites_tracks: None,
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
            now: None,
            resume: None,
            playing_from: None,
            volume: settings.volume.unwrap_or(1.0).clamp(0.0, 1.0),
            muted: settings.muted.unwrap_or(false),
            library_filter: LibraryFilter::default(),
            library_hovered: false,
            library_scroll: Default::default(),
            songs_scroll: ScrollHandle::new(),
            settings,
        };
        // Shuffle and repeat as they were left.
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

    /// The song the last session left, shown paused where it was. Read
    /// only: the session kept before devices shared it is shown, not taken
    /// over into what they share, as the egui window does on its start.
    fn resume_session(&mut self) {
        let session = match self.kept.session() {
            Some((session, _)) => Some(session),
            None => std::fs::read_to_string(desktop::settings::dir().join("session.json"))
                .ok()
                .and_then(|text| serde_json::from_str(&navidrome::scrub_kept(&text)).ok()),
        };
        if let Some(session) = session {
            self.show_session(session);
        }
    }

    fn show_session(&mut self, session: music_core::Handoff) {
        let order = session.order.into_iter().map(|i| i as usize).collect();
        self.queue
            .restore(session.tracks, order, session.at as usize, session.shuffle);
        self.queue.set_repeat(session.repeat);
        self.now = self.queue.current().cloned();
        self.resume = self
            .now
            .as_ref()
            .map(|t| (t.id.clone(), Duration::from_millis(session.position_ms)));
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

    /// The id favourites know a playlist by: a catalog one (`pl.…`). The
    /// user's own playlists have one too; Apple's Favourite Songs has none.
    fn playlist_favourite_id(playlist: &Playlist) -> Option<ItemId> {
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
            && self
                .favourites_tracks
                .as_ref()
                .is_none_or(|(asked, _)| *asked != id)
        {
            self.favourites_tracks = Some((id.clone(), Vec::new()));
            self.backend.send(Request::Playlist(id));
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
            // Another device's file plays from a copy here, if there is one:
            // which copies there are comes with the player.
            ProviderKind::Local => true,
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

    /// Nothing plays in this slice: the player comes later.
    pub fn is_playing(&self) -> bool {
        false
    }

    /// Whether `track` is the one the player has: none, until it plays.
    pub fn is_current(&self, _track: &Track) -> bool {
        false
    }

    pub fn duration(&self) -> Option<Duration> {
        self.now.as_ref().and_then(|t| t.duration)
    }

    /// What plays, as shown: the last session's song, paused where it was.
    pub fn shown(&self) -> Shown {
        Shown {
            track: self.now.clone(),
            has_track: self.queue.current().is_some(),
            loading: None,
            playing: self.is_playing(),
            position: self.resume.as_ref().map(|(_, at)| *at).unwrap_or_default(),
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

    /// What the workings have said since last time.
    fn pump(&mut self, cx: &mut Context<Self>) {
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
                Reply::Connected { .. } => self.connection = Connection::Connected,
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
                }
                Reply::Navidrome(result) => {
                    self.navidrome = match result {
                        Ok(Some(name)) => SoundCloudState::Connected(name),
                        Ok(None) => SoundCloudState::NotSignedIn,
                        Err(e) => SoundCloudState::Failed(e),
                    };
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
                Reply::PlaylistTracks {
                    id,
                    result,
                    done,
                    whole,
                } => {
                    // The favourites fill in page by page: Apple's playlist of
                    // them can run to thousands of songs.
                    let favourites = self.favourites().as_ref() == Some(&id);
                    if let Ok(tracks) = &result {
                        if favourites {
                            self.favourite_songs
                                .extend(tracks.iter().map(|t| t.id.clone()));
                        }
                        // Songs read from the library's playlists are in the library.
                        self.library_songs
                            .extend(tracks.iter().filter(|t| t.in_library).map(|t| t.id.clone()));
                    }
                    if let (true, Some((asked, kept))) = (favourites, &mut self.favourites_tracks)
                        && *asked == id
                    {
                        match &result {
                            Ok(tracks) if whole => {
                                *kept = tracks.iter().map(|t| t.id.clone()).collect();
                            }
                            Ok(tracks) => kept.extend(tracks.iter().map(|t| t.id.clone())),
                            Err(_) => {}
                        }
                        // The favourites whole: those no longer in it lose their mark.
                        if done && result.is_ok() {
                            self.favourite_songs = kept.iter().cloned().collect();
                        }
                    }
                }
                // The rest is for screens still to come.
                _ => {}
            }
        }
        cx.notify();
    }
}

impl Render for Music {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::ui::set_zoom(self.scale() * desktop::settings::SCALE_UNIT);
        crate::covers::forget_unseen(window, cx);
        crate::ui::show(self, window, cx)
    }
}

/// The history as it was kept before devices shared it.
fn read_history() -> Vec<Track> {
    std::fs::read_to_string(desktop::settings::dir().join("history.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&navidrome::scrub_kept(&text)).ok())
        .unwrap_or_default()
}
