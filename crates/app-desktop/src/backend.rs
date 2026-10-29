//! Provider and network work on a tokio runtime, off the UI thread. The UI sends
//! [`Request`]s and picks up [`Reply`]s each frame; every reply wakes it up.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;

use apple_music::{AppleMusic, AudioStream};
use music_core::model::{Album, Cursor, ItemId, Lyrics, Playlist, SearchResults, Track};
use music_core::{Error, Provider};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::app::SearchScope;

pub enum Request {
    /// The library songs and playlists, page by page.
    Library,
    /// A track's audio, ready to hand to the player.
    Fetch { token: player::Token, track: ItemId },
    /// An album and its tracks.
    Album(ItemId),
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
    Connected,
    NotSignedIn,
    Failed(String),
    Songs {
        tracks: Vec<Track>,
        done: bool,
    },
    Playlists(Vec<Playlist>),
    Fetched {
        token: player::Token,
        result: Result<AudioStream, String>,
    },
    Album {
        id: ItemId,
        result: Result<(Album, Vec<Track>), String>,
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
        let (tx, replies) = mpsc::channel();
        let replies_out = Replies { tx, ctx };
        std::thread::Builder::new()
            .name("backend".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("start the tokio runtime");
                runtime.block_on(run(request_rx, replies_out));
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

async fn run(mut requests: UnboundedReceiver<Request>, replies: Replies) {
    let Ok(token) = std::fs::read_to_string(apple_music_dir().join("media-user-token")) else {
        replies.send(Reply::NotSignedIn);
        return;
    };
    let am = match AppleMusic::connect(token.trim()).await {
        Ok(am) => Arc::new(am),
        Err(Error::NotSignedIn) => {
            replies.send(Reply::NotSignedIn);
            return;
        }
        Err(e) => {
            replies.send(Reply::Failed(e.to_string()));
            return;
        }
    };
    replies.send(Reply::Connected);
    let cdm = widevine::locate()
        .ok_or_else(|| "no Widevine module found: install Brave, Chrome or Firefox".to_owned())
        .and_then(|path| widevine::Cdm::open(&path).map_err(|e| e.to_string()));

    let http = reqwest::Client::new();
    while let Some(request) = requests.recv().await {
        let (am, replies) = (am.clone(), replies.clone());
        match request {
            Request::Album(id) => {
                tokio::spawn(async move {
                    let result = tokio::try_join!(am.album(&id), am.album_tracks(&id))
                        .map_err(|e| e.to_string());
                    replies.send(Reply::Album { id, result });
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
            Request::Library => {
                tokio::spawn(load_library(am, replies));
            }
            Request::Fetch { token, track } => {
                let cdm = cdm.clone();
                tokio::spawn(async move {
                    let result = match cdm {
                        Ok(cdm) => am
                            .fetch_track(&cdm, &track)
                            .await
                            .map_err(|e| e.to_string()),
                        Err(e) => Err(e),
                    };
                    replies.send(Reply::Fetched { token, result });
                });
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
