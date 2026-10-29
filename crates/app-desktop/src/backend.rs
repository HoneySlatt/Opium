//! Provider and network work on a tokio runtime, off the UI thread. The UI sends
//! [`Request`]s and picks up [`Reply`]s each frame; every reply wakes it up.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;

use apple_music::{AppleMusic, AudioStream, LiveStream};
use music_core::model::{
    Album, Artist, ArtistDetail, Collection, Cursor, FavouriteKind, ItemId, Lyrics, Playlist,
    ProviderKind, SearchResults, Shelf, Track,
};
use music_core::{Error, Provider};
use serde::Serialize;
use serde::de::DeserializeOwned;
use soundcloud::SoundCloud;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::app::SearchScope;
use crate::cache;

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
    /// Sign in to SoundCloud in the browser, then connect and load its library.
    SignInSoundCloud,
    ReconnectSoundCloud,
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
    /// Make a playlist with these songs.
    CreatePlaylist {
        name: String,
        description: String,
        tracks: Vec<ItemId>,
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
    /// A track's audio, ready to hand to the player.
    Fetch {
        token: player::Token,
        track: ItemId,
        /// The library's copy, played if the catalog's can't be.
        fallback: Option<ItemId>,
        /// A SoundCloud uploader's original file, when they allow it.
        original: bool,
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
    },
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
    /// SoundCloud is connected as this user, or isn't signed in (`None`),
    /// or failed to connect.
    SoundCloud(Result<Option<String>, String>),
    SignedInSoundCloud(Result<(), String>),
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
    NotSignedIn,
    Failed(String),
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
    LibraryAlbums {
        albums: Vec<Album>,
        done: bool,
        whole: bool,
    },
    LibraryArtists {
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
    Search {
        scope: SearchScope,
        query: String,
        result: Result<SearchResults, String>,
    },
}

/// What a fetch hands the player: a track, a station's stream, or a whole
/// file in memory.
pub enum Playable {
    Track(AudioStream),
    /// A SoundCloud track, in memory or still downloading.
    SoundCloud(soundcloud::Audio),
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

/// Where the SoundCloud sign-in keeps its tokens (see the `sc` example).
fn soundcloud_dir() -> PathBuf {
    apple_music_dir().with_file_name("soundcloud")
}

/// Keep SoundCloud's tokens, readable by the user only.
fn keep_soundcloud_tokens(tokens: &soundcloud::Tokens) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let dir = soundcloud_dir();
    let _ = std::fs::create_dir_all(&dir);
    let json = serde_json::json!({ "oauth": tokens.oauth, "refresh": tokens.refresh });
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(dir.join("tokens.json"))
    {
        let _ = file.write_all(json.to_string().as_bytes());
    }
}

fn kept_soundcloud_tokens() -> Option<soundcloud::Tokens> {
    let text = std::fs::read_to_string(soundcloud_dir().join("tokens.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    Some(soundcloud::Tokens {
        oauth: json["oauth"].as_str()?.to_owned(),
        refresh: json["refresh"].as_str().map(str::to_owned),
    })
}

/// Connect to SoundCloud with the kept tokens, telling the UI how it went.
async fn connect_soundcloud(replies: &Replies) -> Option<Arc<SoundCloud>> {
    let Some(tokens) = kept_soundcloud_tokens() else {
        replies.send(Reply::SoundCloud(Ok(None)));
        return None;
    };
    match SoundCloud::connect(tokens, Box::new(keep_soundcloud_tokens)).await {
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
    std::fs::create_dir_all(&profile).map_err(|e| e.to_string())?;
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
    let mut sc = connect_soundcloud(&replies).await;
    let cdm = widevine::locate()
        .ok_or_else(|| "no Widevine module found: install Brave, Chrome or Firefox".to_owned())
        .and_then(|path| widevine::Cdm::open(&path).map_err(|e| e.to_string()));

    let http = reqwest::Client::new();
    let mut warming: Option<tokio::task::JoinHandle<()>> = None;
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
                        // The library kept may be another account's.
                        cache::clear();
                        let _ = again.send(Request::Reconnect);
                    }
                });
                continue;
            }
            Request::Reconnect => {
                apple = connect(&replies).await;
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
                sc = connect_soundcloud(&replies).await;
                if let Some(sc) = sc.clone() {
                    tokio::spawn(load_soundcloud_library(sc, replies));
                }
                continue;
            }
            // Both services at once.
            Request::Library => {
                if let Some(am) = apple.clone() {
                    tokio::spawn(load_library(am, replies.clone()));
                }
                if let Some(sc) = sc.clone() {
                    tokio::spawn(load_soundcloud_library(sc, replies));
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
                    tokio::spawn(async move {
                        if let Ok(playlists) = soundcloud_playlists(&sc).await {
                            replies.send(Reply::Playlists {
                                provider: ProviderKind::SoundCloud,
                                playlists,
                            });
                        }
                    });
                }
                continue;
            }
            Request::Search(scope, query) => {
                tokio::spawn(search(apple.clone(), sc.clone(), scope, query, replies));
                continue;
            }
            _ => {}
        }
        // What is SoundCloud's goes there.
        if target(&request) == Some(ProviderKind::SoundCloud) {
            match sc.clone() {
                Some(sc) => soundcloud_request(sc, request, replies, &mut warming),
                None => {
                    if let Request::Fetch { token, .. } | Request::Warm { token, .. } = request {
                        replies.send(Reply::Fetched {
                            token,
                            result: Err("SoundCloud isn't connected".into()),
                        });
                    }
                }
            }
            continue;
        }
        // Without a connection there is nothing to ask; a track to play
        // still hears why.
        let Some(am) = apple.clone() else {
            if let Request::Fetch { token, .. } | Request::Warm { token, .. } = request {
                replies.send(Reply::Fetched {
                    token,
                    result: Err("Apple Music isn't connected".into()),
                });
            }
            continue;
        };
        match request {
            Request::SignIn
            | Request::Reconnect
            | Request::SignInSoundCloud
            | Request::ReconnectSoundCloud
            | Request::Library
            | Request::Playlists
            | Request::Search(..) => {}
            Request::Album(id) => {
                tokio::spawn(async move {
                    let result = tokio::try_join!(am.album(&id), am.album_tracks(&id))
                        .map(Box::new)
                        .map_err(|e| e.to_string());
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
            Request::Palette(url) => {
                let http = http.clone();
                tokio::spawn(async move {
                    // The cover is often kept already, the page showing it.
                    let bytes = match crate::covers::read(&url) {
                        Some(bytes) => Some(bytes),
                        None => match http.get(&url).send().await {
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
            }
            Request::Lyrics(id) => {
                tokio::spawn(async move {
                    let result = am.lyrics(&id).await.map_err(|e| e.to_string());
                    replies.send(Reply::Lyrics { id, result });
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
            Request::CreatePlaylist {
                name,
                description,
                tracks,
            } => {
                tokio::spawn(async move {
                    let result = am
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
                    let result = am
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
                    let result = am
                        .remove_from_playlist(&playlist, &entry)
                        .await
                        .map_err(|e| format!("Couldn't take it out of the playlist: {e}"));
                    replies.send(Reply::RemovedFromPlaylist { playlist, result });
                });
            }
            Request::DeletePlaylist(id) => {
                tokio::spawn(async move {
                    let result = am
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
                tokio::spawn(async move {
                    let show = |albums, done, whole| {
                        replies.send(Reply::LibraryAlbums {
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
                tokio::spawn(async move {
                    let show = |artists, done, whole| {
                        replies.send(Reply::LibraryArtists {
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
                tokio::spawn(async move {
                    let result = am
                        .each_recently_added(RECENT_LIMIT, |items, done| {
                            replies.send(Reply::RecentlyAdded { items, done })
                        })
                        .await;
                    failed(&replies, result, || Reply::RecentlyAdded {
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
                ..
            } => {
                let cdm = cdm.clone();
                tokio::spawn(async move {
                    let result = playable(&am, cdm, &track, fallback.as_ref()).await;
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
                ..
            } => {
                if let Some(last) = warming.take() {
                    last.abort();
                }
                let cdm = cdm.clone();
                warming = Some(tokio::spawn(async move {
                    let result = playable(&am, cdm, &track, fallback.as_ref()).await;
                    replies.send(Reply::Fetched { token, result });
                }));
            }
        }
    }
}

/// How many songs a station of songs is asked for at a time.
const STATION_BATCH: usize = 20;

/// How far back Recently Added goes.
const RECENT_LIMIT: usize = 200;

/// A track or a station, ready to play: stations (`ra.`) stream, everything
/// else is a track. A catalog song Apple no longer offers here is played from
/// the library's copy, when there is one, as Apple Music does.
async fn playable(
    am: &AppleMusic,
    cdm: Result<widevine::Cdm, String>,
    track: &ItemId,
    fallback: Option<&ItemId>,
) -> Result<Playable, String> {
    let first = playable_as(am, cdm.clone(), track).await;
    match (first, fallback) {
        (Err(_), Some(copy)) if copy != track => playable_as(am, cdm, copy).await,
        (result, _) => result,
    }
}

async fn playable_as(
    am: &AppleMusic,
    cdm: Result<widevine::Cdm, String>,
    track: &ItemId,
) -> Result<Playable, String> {
    match cdm {
        Ok(cdm) if track.id.starts_with("ra.") => am
            .live_station(&cdm, track)
            .await
            .map(Playable::Live)
            .map_err(|e| e.to_string()),
        Ok(cdm) => am
            .fetch_track(&cdm, track)
            .await
            .map(Playable::Track)
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
        | Request::EditPlaylist { id, .. }
        | Request::RemoveFromPlaylist { playlist: id, .. } => id,
        Request::InLibrary { ids, .. } => ids.first()?,
        Request::AddToPlaylists { playlists, .. } => playlists.first()?,
        Request::CreatePlaylist { tracks, .. } => tracks.first()?,
        _ => return None,
    };
    Some(id.provider)
}

/// A request about a SoundCloud item.
fn soundcloud_request(
    sc: Arc<SoundCloud>,
    request: Request,
    replies: Replies,
    warming: &mut Option<tokio::task::JoinHandle<()>>,
) {
    match request {
        Request::Album(id) => {
            tokio::spawn(async move {
                let result = tokio::try_join!(sc.album(&id), sc.album_tracks(&id))
                    .map(Box::new)
                    .map_err(|e| e.to_string());
                replies.send(Reply::Album { id, result });
            });
        }
        Request::Artist(id) => {
            tokio::spawn(async move {
                let result = sc
                    .artist(&id)
                    .await
                    .map(Arc::new)
                    .map_err(|e| e.to_string());
                replies.send(Reply::Artist { id, result });
            });
        }
        Request::Playlist(id) => {
            tokio::spawn(async move {
                let result = sc
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
        Request::Lyrics(id) => replies.send(Reply::Lyrics {
            id,
            result: Ok(None),
        }),
        Request::SongLinks { song, radio } => {
            tokio::spawn(async move {
                let result = sc.song_links(&song).await.map_err(|e| e.to_string());
                replies.send(Reply::SongLinks { radio, result });
            });
        }
        Request::Favourite { id, .. } => replies.send(Reply::Favourite {
            id,
            set: None,
            result: Ok(None),
        }),
        Request::Fetch {
            token,
            track,
            original,
            ..
        } => {
            tokio::spawn(async move {
                let result = soundcloud_playable(&sc, &track, original).await;
                replies.send(Reply::Fetched { token, result });
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
            *warming = Some(tokio::spawn(async move {
                let result = soundcloud_playable(&sc, &track, original).await;
                replies.send(Reply::Fetched { token, result });
            }));
        }
        // Covers that are SoundCloud's aren't asked about.
        Request::InLibrary { .. } => {}
        // SoundCloud has no other editions or videos to show.
        Request::AlbumExtras(id) => replies.send(Reply::AlbumExtras {
            id,
            result: Ok(Arc::default()),
        }),
        _ => replies.send(Reply::Changed(Err(
            "That comes to SoundCloud in a later version".into(),
        ))),
    }
}

async fn soundcloud_playable(
    sc: &SoundCloud,
    track: &ItemId,
    original: bool,
) -> Result<Playable, String> {
    sc.audio(track, original)
        .await
        .map(Playable::SoundCloud)
        .map_err(|e| e.to_string())
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

/// SoundCloud's side of the library: the playlists, then the liked tracks,
/// kept as Apple Music's are.
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

/// A search of both services: the catalog's in both, the library's in Apple
/// Music's. One that fails leaves the other's results.
async fn search(
    apple: Option<Arc<AppleMusic>>,
    sc: Option<Arc<SoundCloud>>,
    scope: SearchScope,
    query: String,
    replies: Replies,
) {
    let apple_side = async {
        let am = apple?;
        Some(match scope {
            SearchScope::AppleMusic => am.search(&query).await,
            SearchScope::Library => am.search_library(&query).await,
        })
    };
    let sc_side = async {
        match (sc, scope) {
            (Some(sc), SearchScope::AppleMusic) => Some(sc.search(&query).await),
            _ => None,
        }
    };
    let (a, b) = tokio::join!(apple_side, sc_side);
    let mut sides = Vec::new();
    let mut error = None;
    for side in [a, b].into_iter().flatten() {
        match side {
            Ok(r) => sides.push(r),
            Err(e) => error = Some(e.to_string()),
        }
    }
    let any = !sides.is_empty();
    // Taken in turns, Apple Music's first: the first rows show both. What
    // both have is Apple Music's, at its better quality.
    let mut merged = SearchResults::default();
    let mut sides = sides.into_iter();
    if let Some(first) = sides.next() {
        merged = first;
        for mut other in sides {
            let key = |title: &str, artists: &[String]| {
                (
                    crate::app::fold(title),
                    crate::app::fold(artists.first().map_or("", String::as_str)),
                )
            };
            let songs: std::collections::HashSet<_> = merged
                .tracks
                .iter()
                .map(|t| key(&t.title, &t.artists))
                .collect();
            other
                .tracks
                .retain(|t| !songs.contains(&key(&t.title, &t.artists)));
            let albums: std::collections::HashSet<_> = merged
                .albums
                .iter()
                .map(|a| key(&a.title, &a.artists))
                .collect();
            other
                .albums
                .retain(|a| !albums.contains(&key(&a.title, &a.artists)));
            merged.tracks = in_turns(merged.tracks, other.tracks);
            merged.albums = in_turns(merged.albums, other.albums);
            merged.artists = in_turns(merged.artists, other.artists);
            merged.playlists = in_turns(merged.playlists, other.playlists);
        }
    }
    let result = match (any, error) {
        (false, Some(e)) => Err(e),
        (false, None) => Err("No service is connected".into()),
        _ => Ok(merged),
    };
    replies.send(Reply::Search {
        scope,
        query,
        result,
    });
}

/// `a`'s and `b`'s items in turns, starting with `a`'s; the longer one's
/// rest after.
fn in_turns<T>(a: Vec<T>, b: Vec<T>) -> Vec<T> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    let (mut a, mut b) = (a.into_iter(), b.into_iter());
    loop {
        match (a.next(), b.next()) {
            (None, None) => return out,
            (x, y) => out.extend(x.into_iter().chain(y)),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::in_turns;

    #[test]
    fn results_come_in_turns_then_the_rest() {
        assert_eq!(in_turns(vec![1, 3, 5, 7], vec![2, 4]), [1, 2, 3, 4, 5, 7]);
        assert_eq!(in_turns(Vec::new(), vec![2, 4]), [2, 4]);
    }
}
