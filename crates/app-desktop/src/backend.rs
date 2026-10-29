//! Provider and network work on a tokio runtime, off the UI thread. The UI sends
//! [`Request`]s and picks up [`Reply`]s each frame; every reply wakes it up.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;

use apple_music::{AppleMusic, AudioStream, LiveStream};
use music_core::model::{
    Album, Artist, ArtistDetail, Collection, Cursor, FavouriteKind, ItemId, Lyrics, Page, Playlist,
    SearchResults, Shelf, Track,
};
use music_core::{Error, Provider};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::app::SearchScope;

pub enum Request {
    /// The library songs and playlists, page by page.
    Library,
    /// The library albums, page by page.
    LibraryAlbums,
    /// The library artists, page by page.
    LibraryArtists,
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
    SongLinks { song: ItemId, radio: bool },
    /// Take a song out of the library, by its library id.
    RemoveFromLibrary(ItemId),
    /// Add songs to playlists.
    AddToPlaylists {
        playlists: Vec<ItemId>,
        tracks: Vec<ItemId>,
    },
    /// Whether something is a favourite.
    Favourite { kind: FavouriteKind, id: ItemId },
    /// A track's audio, ready to hand to the player.
    Fetch { token: player::Token, track: ItemId },
    /// An album and its tracks.
    Album(ItemId),
    /// An artist's top songs and releases.
    Artist(ItemId),
    /// A playlist's tracks, page by page.
    Playlist(ItemId),
    /// The three colours of the cover at this URL.
    Palette(String),
    /// A track's lyrics.
    Lyrics(ItemId),
    /// Songs, albums, artists and playlists matching a query, in the
    /// catalog or in the library.
    Search(SearchScope, String),
}

pub enum Reply {
    /// Connected to Apple Music, in this storefront (country).
    Connected {
        storefront: String,
    },
    /// The browser sign-in ended: the token is saved, or why not.
    SignedIn(Result<(), String>),
    /// A song's links, asked for its artist (`radio: false`) or its station.
    SongLinks {
        radio: bool,
        result: Result<music_core::model::SongLinks, String>,
    },
    /// A change to the library went through (the message to show), or why not.
    Changed(Result<String, String>),
    /// A favourite was set (`set: Some(on)`) or looked up; the state or why not.
    Favourite {
        id: ItemId,
        set: Option<bool>,
        result: Result<Option<bool>, String>,
    },
    NotSignedIn,
    Failed(String),
    Songs {
        tracks: Vec<Track>,
        done: bool,
    },
    Playlists(Vec<Playlist>),
    LibraryAlbums {
        albums: Vec<Album>,
        done: bool,
    },
    LibraryArtists {
        artists: Vec<Artist>,
        done: bool,
    },
    RecentlyAdded {
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
        result: Result<(Album, Vec<Track>), String>,
    },
    Artist {
        id: ItemId,
        result: Result<ArtistDetail, String>,
    },
    PlaylistTracks {
        id: ItemId,
        result: Result<Vec<Track>, String>,
        done: bool,
    },
    Palette {
        url: String,
        palette: Option<crate::palette::Palette>,
    },
    Lyrics {
        id: ItemId,
        result: Result<Option<Lyrics>, String>,
    },
    Search {
        scope: SearchScope,
        query: String,
        result: Result<SearchResults, String>,
    },
}

/// What a fetch hands the player: a track, or a station's stream.
pub enum Playable {
    Track(AudioStream),
    Live(LiveStream),
}

pub struct Backend {
    requests: UnboundedSender<Request>,
    replies: mpsc::Receiver<Reply>,
}

/// Where the Apple Music sign-in keeps its token (see the `am` example).
pub fn apple_music_dir() -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_default();
    config.join("music/apple-music")
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
    pub fn start(ctx: egui::Context) -> Self {
        let (requests, request_rx) = tokio::sync::mpsc::unbounded_channel();
        let again = requests.clone();
        let (tx, replies) = mpsc::channel();
        let replies_out = Replies { tx, ctx };
        std::thread::Builder::new()
            .name("backend".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("start the tokio runtime");
                runtime.block_on(run(request_rx, again, replies_out));
            })
            .expect("spawn the backend thread");
        Self { requests, replies }
    }

    pub fn send(&self, request: Request) {
        let _ = self.requests.send(request);
    }

    pub fn try_recv(&self) -> Option<Reply> {
        self.replies.try_recv().ok()
    }
}

/// The browser the sign-in opens: `MUSIC_BROWSER`, else Brave Origin.
fn sign_in_browser() -> std::ffi::OsString {
    std::env::var_os("MUSIC_BROWSER").unwrap_or_else(|| "brave-origin".into())
}

/// Connect with the saved token, telling the UI how it went.
async fn connect(replies: &Replies) -> Option<Arc<AppleMusic>> {
    let Ok(token) = std::fs::read_to_string(apple_music_dir().join("media-user-token")) else {
        replies.send(Reply::NotSignedIn);
        return None;
    };
    match AppleMusic::connect(token.trim()).await {
        Ok(am) => {
            replies.send(Reply::Connected {
                storefront: am.storefront().to_owned(),
            });
            Some(Arc::new(am))
        }
        Err(Error::NotSignedIn) => {
            replies.send(Reply::NotSignedIn);
            None
        }
        Err(e) => {
            replies.send(Reply::Failed(e.to_string()));
            None
        }
    }
}

/// Open the browser on Apple's sign-in page, wait for the user, and save the
/// token readable by the user only, where the `am` example keeps it too.
async fn sign_in() -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let dir = apple_music_dir();
    let profile = dir.join("browser-profile");
    std::fs::create_dir_all(&profile).map_err(|e| e.to_string())?;
    let browser = sign_in_browser();
    let token = apple_music::signin::sign_in(
        browser.as_ref(),
        &profile,
        std::time::Duration::from_secs(600),
    )
    .await
    .map_err(|e| e.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(dir.join("media-user-token"))
        .map_err(|e| e.to_string())?;
    file.write_all(token.as_bytes()).map_err(|e| e.to_string())
}

async fn run(
    mut requests: UnboundedReceiver<Request>,
    again: UnboundedSender<Request>,
    replies: Replies,
) {
    let mut apple = connect(&replies).await;
    let cdm = widevine::locate()
        .ok_or_else(|| "no Widevine module found: install Brave, Chrome or Firefox".to_owned())
        .and_then(|path| widevine::Cdm::open(&path).map_err(|e| e.to_string()));

    let http = reqwest::Client::new();
    while let Some(request) = requests.recv().await {
        let replies = replies.clone();
        match request {
            Request::SignIn => {
                let again = again.clone();
                tokio::spawn(async move {
                    let result = sign_in().await;
                    let ok = result.is_ok();
                    replies.send(Reply::SignedIn(result));
                    if ok {
                        let _ = again.send(Request::Reconnect);
                    }
                });
                continue;
            }
            Request::Reconnect => {
                apple = connect(&replies).await;
                continue;
            }
            _ => {}
        }
        // Without a connection there is nothing to ask; a track to play
        // still hears why.
        let Some(am) = apple.clone() else {
            if let Request::Fetch { token, .. } = request {
                replies.send(Reply::Fetched {
                    token,
                    result: Err("Apple Music isn't connected".into()),
                });
            }
            continue;
        };
        match request {
            Request::SignIn | Request::Reconnect => {}
            Request::Album(id) => {
                tokio::spawn(async move {
                    let result = tokio::try_join!(am.album(&id), am.album_tracks(&id))
                        .map_err(|e| e.to_string());
                    replies.send(Reply::Album { id, result });
                });
            }
            Request::Artist(id) => {
                tokio::spawn(async move {
                    let result = am.artist(&id).await.map_err(|e| e.to_string());
                    replies.send(Reply::Artist { id, result });
                });
            }
            Request::Playlist(id) => {
                tokio::spawn(load_playlist(am, id, replies));
            }
            Request::Palette(url) => {
                let http = http.clone();
                tokio::spawn(async move {
                    let palette = match http.get(&url).send().await {
                        Ok(resp) => match resp.bytes().await {
                            Ok(bytes) => tokio::task::spawn_blocking(move || {
                                crate::palette::from_image(&bytes)
                            })
                            .await
                            .ok()
                            .flatten(),
                            Err(_) => None,
                        },
                        Err(_) => None,
                    };
                    replies.send(Reply::Palette { url, palette });
                });
            }
            Request::Lyrics(id) => {
                tokio::spawn(async move {
                    let result = am.lyrics(&id).await.map_err(|e| e.to_string());
                    replies.send(Reply::Lyrics { id, result });
                });
            }
            Request::Search(scope, query) => {
                tokio::spawn(async move {
                    let result = match scope {
                        SearchScope::AppleMusic => am.search(&query).await,
                        SearchScope::Library => am.search_library(&query).await,
                    };
                    let result = result.map_err(|e| e.to_string());
                    replies.send(Reply::Search {
                        scope,
                        query,
                        result,
                    });
                });
            }
            Request::SetFavourite { kind, id, on } => {
                tokio::spawn(async move {
                    let result = am
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
                    let result = am
                        .add_to_library(kind, &id)
                        .await
                        .map(|()| "Added to the library".to_owned())
                        .map_err(|e| format!("Couldn't add to the library: {e}"));
                    if !(quiet && result.is_ok()) {
                        replies.send(Reply::Changed(result));
                    }
                });
            }
            Request::SongLinks { song, radio } => {
                tokio::spawn(async move {
                    let result = am.song_links(&song).await.map_err(|e| e.to_string());
                    replies.send(Reply::SongLinks { radio, result });
                });
            }
            Request::RemoveFromLibrary(id) => {
                tokio::spawn(async move {
                    let result = am
                        .remove_from_library(&id)
                        .await
                        .map(|()| "Removed from the library".to_owned())
                        .map_err(|e| format!("Couldn't remove it from the library: {e}"));
                    replies.send(Reply::Changed(result));
                });
            }
            Request::AddToPlaylists { playlists, tracks } => {
                tokio::spawn(async move {
                    let mut failed = None;
                    for playlist in &playlists {
                        if let Err(e) = am.add_to_playlist(playlist, &tracks).await {
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
                    let result = am.is_favourite(kind, &id).await.map_err(|e| e.to_string());
                    replies.send(Reply::Favourite {
                        id,
                        set: None,
                        result,
                    });
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
            Request::StationTracks(id) => {
                tokio::spawn(async move {
                    let result = am
                        .station_tracks(&id, STATION_BATCH)
                        .await
                        .map_err(|e| e.to_string());
                    replies.send(Reply::StationTracks { id, result });
                });
            }
            Request::Home => {
                tokio::spawn(async move {
                    replies.send(Reply::Home(am.home().await.map_err(|e| e.to_string())));
                });
            }
            Request::LibraryAlbums => {
                tokio::spawn(stream(
                    move |cursor| {
                        let am = am.clone();
                        async move { am.library_albums(cursor.as_ref()).await }
                    },
                    |albums, done| Reply::LibraryAlbums { albums, done },
                    usize::MAX,
                    replies,
                ));
            }
            Request::LibraryArtists => {
                tokio::spawn(stream(
                    move |cursor| {
                        let am = am.clone();
                        async move { am.library_artists(cursor.as_ref()).await }
                    },
                    |artists, done| Reply::LibraryArtists { artists, done },
                    usize::MAX,
                    replies,
                ));
            }
            Request::RecentlyAdded => {
                tokio::spawn(stream(
                    move |cursor| {
                        let am = am.clone();
                        async move { am.recently_added(cursor.as_ref()).await }
                    },
                    |items, done| Reply::RecentlyAdded { items, done },
                    RECENT_LIMIT,
                    replies,
                ));
            }
            Request::Library => {
                tokio::spawn(load_library(am, replies));
            }
            Request::Fetch { token, track } => {
                let cdm = cdm.clone();
                tokio::spawn(async move {
                    // Stations (`ra.`) stream; everything else is a track.
                    let result = match cdm {
                        Ok(cdm) if track.id.starts_with("ra.") => am
                            .live_station(&cdm, &track)
                            .await
                            .map(Playable::Live)
                            .map_err(|e| e.to_string()),
                        Ok(cdm) => am
                            .fetch_track(&cdm, &track)
                            .await
                            .map(Playable::Track)
                            .map_err(|e| e.to_string()),
                        Err(e) => Err(e),
                    };
                    replies.send(Reply::Fetched { token, result });
                });
            }
        }
    }
}

/// How many songs a station of songs is asked for at a time.
const STATION_BATCH: usize = 20;

/// How far back Recently Added goes.
const RECENT_LIMIT: usize = 200;

/// Send a list page by page as it arrives, stopping after `limit` items. A
/// failure ends the list and shows as a toast.
async fn stream<T, F, Fut>(
    mut fetch: F,
    reply: fn(Vec<T>, bool) -> Reply,
    limit: usize,
    replies: Replies,
) where
    F: FnMut(Option<Cursor>) -> Fut,
    Fut: std::future::Future<Output = music_core::Result<Page<T>>>,
{
    let mut cursor = None;
    let mut sent = 0;
    loop {
        match fetch(cursor.take()).await {
            Ok(page) => {
                sent += page.items.len();
                let done = page.next.is_none() || sent >= limit;
                replies.send(reply(page.items, done));
                if done {
                    return;
                }
                cursor = page.next;
            }
            Err(e) => {
                replies.send(reply(Vec::new(), true));
                replies.send(Reply::Failed(e.to_string()));
                return;
            }
        }
    }
}

async fn load_library(am: Arc<AppleMusic>, replies: Replies) {
    let mut playlists = Vec::new();
    let mut cursor: Option<Cursor> = None;
    loop {
        match am.library_playlists(cursor.as_ref()).await {
            Ok(page) => {
                playlists.extend(page.items);
                match page.next {
                    Some(next) => cursor = Some(next),
                    None => break,
                }
            }
            Err(e) => {
                replies.send(Reply::Failed(e.to_string()));
                break;
            }
        }
    }
    replies.send(Reply::Playlists(playlists));

    let mut cursor: Option<Cursor> = None;
    loop {
        match am.library_tracks(cursor.as_ref()).await {
            Ok(page) => {
                let done = page.next.is_none();
                replies.send(Reply::Songs {
                    tracks: page.items,
                    done,
                });
                match page.next {
                    Some(next) => cursor = Some(next),
                    None => return,
                }
            }
            Err(e) => {
                replies.send(Reply::Failed(e.to_string()));
                return;
            }
        }
    }
}

async fn load_playlist(am: Arc<AppleMusic>, id: ItemId, replies: Replies) {
    let mut cursor: Option<Cursor> = None;
    loop {
        match am.playlist_tracks(&id, cursor.as_ref()).await {
            Ok(page) => {
                let done = page.next.is_none();
                replies.send(Reply::PlaylistTracks {
                    id: id.clone(),
                    result: Ok(page.items),
                    done,
                });
                match page.next {
                    Some(next) => cursor = Some(next),
                    None => return,
                }
            }
            Err(e) => {
                replies.send(Reply::PlaylistTracks {
                    id,
                    result: Err(e.to_string()),
                    done: true,
                });
                return;
            }
        }
    }
}
