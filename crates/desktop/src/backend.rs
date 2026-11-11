//! Provider and network work on a tokio runtime, off the UI thread. The UI sends
//! [`Request`]s and picks up [`Reply`]s each frame; every reply wakes it up.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;

use apple_music::{AppleMusic, AudioStream, LiveStream};
use navidrome::Navidrome;
use opium_core::model::{
    Album, Artist, ArtistDetail, Collection, Cursor, FavouriteKind, ItemId, Lyrics, Playlist,
    ProviderKind, SearchResults, Shelf, Track,
};
use opium_core::{Error, Provider};
use serde::Serialize;
use serde::de::DeserializeOwned;
use services::Services;
use soundcloud::SoundCloud;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::SearchScope;
use crate::cache;

mod downloading;
#[cfg(test)]
mod tests;
pub use downloading::{Downloading, KeptSong};

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
    /// The home page's shelves, as this service shows them.
    Home(ProviderKind),
    /// The radio page's shelves.
    Radio(ProviderKind),
    /// The new music page's shelves.
    NewMusic(ProviderKind),
    /// The next songs of a station of songs.
    StationTracks(ItemId),
    /// Sign in to Apple Music in the browser, then connect with the new token.
    SignIn,
    /// Connect again with the saved token (after a sign-in).
    Reconnect,
    /// Turn a service on, connecting and loading its library, or off,
    /// leaving it signed in but unused.
    SetService {
        provider: ProviderKind,
        on: bool,
    },
    /// Sign in to SoundCloud in the browser, then connect and load its library.
    SignInSoundCloud,
    ReconnectSoundCloud,
    /// Connect to the Navidrome server again, after it was out of reach.
    ReconnectNavidrome,
    /// Sign out of a service: its sign-in, its browser profile and what was
    /// kept of its library go from the disk.
    SignOut(ProviderKind),
    /// Sign in to a Navidrome server; the password makes the token kept,
    /// and goes no further.
    SignInNavidrome {
        url: String,
        user: String,
        password: String,
    },
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
    /// Make a playlist with these songs, in `service`.
    CreatePlaylist {
        service: ProviderKind,
        name: String,
        description: String,
        tracks: Vec<ItemId>,
    },
    /// Make one of Opium's own playlists, of songs from any service.
    CreateOpiumPlaylist {
        name: String,
        description: String,
        tracks: Vec<Track>,
    },
    /// Add songs to one of Opium's own playlists.
    AddToOpiumPlaylist {
        playlist: ItemId,
        tracks: Vec<Track>,
    },
    /// Change a playlist's name and description.
    EditPlaylist {
        id: ItemId,
        name: String,
        description: String,
    },
    DeletePlaylist(ItemId),
    /// A playlist's cover `jpeg` (square, see `opium_core::cover`), or the
    /// one it had: Opium's own and a Navidrome server's take one.
    SetPlaylistCover {
        id: ItemId,
        jpeg: Option<Vec<u8>>,
    },
    /// The password for a session on the Navidrome server's own API (a
    /// cover change wants one): never kept, the session is.
    NavidromeSession(String),
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
    /// Ask for less like this, or take that back.
    SuggestLess {
        kind: FavouriteKind,
        id: ItemId,
        on: bool,
    },
    /// Whether less like this was asked for.
    SuggestsLess {
        kind: FavouriteKind,
        id: ItemId,
    },
    /// Its page on the service's website, to copy.
    ShareLink {
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
        /// Apple's Spatial Audio, for a song mixed in Dolby Atmos.
        spatial: bool,
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
        spatial: bool,
    },
    /// Download songs to play offline; `original`: a SoundCloud
    /// uploader's own file, where they allow it.
    Download {
        tracks: Vec<Track>,
        original: bool,
    },
    /// Downloads no longer kept, or no longer to come.
    RemoveDownloads(Vec<ItemId>),
    /// The folders whose music files make the library's own; none, no files.
    SetLocalFolders(Vec<PathBuf>),
    /// Read the folders again: something in them changed.
    RescanLocal,
    /// Files opened from elsewhere (the file manager, the command line, a
    /// drop on the window), a folder for its music files: to play now.
    OpenFiles(Vec<PathBuf>),
    /// Write `edit` into the tags of the music files `paths` name.
    WriteTags {
        paths: Vec<PathBuf>,
        edit: local::TagEdit,
    },
    /// Every service's library songs as they were added, the last first.
    NewestSongs,
    /// An album and its tracks.
    Album(ItemId),
    /// An artist's top songs and releases.
    Artist(ItemId),
    /// A playlist's tracks, page by page.
    Playlist(ItemId),
    /// The three colours of the cover at this URL.
    Palette(String),
    /// A track's lyrics, from its service.
    Lyrics(ItemId),
    /// A track's lyrics from the lyrics sites, asked in this order.
    SiteLyrics {
        track: Box<Track>,
        sources: Vec<lyrics::Source>,
    },
    /// Songs, albums, artists and playlists matching a query, in the
    /// catalog or in the library.
    /// A search, of every service or only the one given.
    Search(SearchScope, Option<ProviderKind>, String),
}

/// Why a cover wasn't changed.
#[derive(Debug, Clone)]
pub enum CoverRefused {
    /// Navidrome wants the password again (see `Request::NavidromeSession`);
    /// the cover, to give again once it has it.
    Password(Option<Vec<u8>>),
    Failed(String),
}

pub enum Reply {
    /// Every song kept to play offline.
    Downloads(Vec<KeptSong>),
    /// How a song's download goes.
    Download {
        id: ItemId,
        state: Downloading,
    },
    /// Connected to Apple Music, in this storefront (country).
    Connected {
        storefront: String,
    },
    /// The account's picture on Apple Music, once read.
    ProfilePicture(opium_core::model::Artwork),
    /// The browser sign-in ended: the token is saved, or why not.
    SignedIn(Result<(), String>),
    /// SoundCloud is connected as this user, or isn't signed in (`None`),
    /// or failed to connect.
    SoundCloud(Result<Option<String>, String>),
    SignedInSoundCloud(Result<(), String>),
    /// Navidrome is connected as this user, or isn't signed in (`None`), or
    /// failed to connect.
    Navidrome(Result<Option<String>, String>),
    SignedInNavidrome(Result<(), String>),
    /// What the user starred, on a service that says so in a list.
    Starred(Vec<ItemId>),
    /// A song's links, asked for its artist (`radio: false`) or its station.
    SongLinks {
        radio: bool,
        result: Result<opium_core::model::SongLinks, String>,
    },
    /// A playlist was made, or why not.
    PlaylistCreated(Result<Playlist, String>),
    CoverChanged {
        id: ItemId,
        result: Result<(), CoverRefused>,
    },
    /// Whether Navidrome opened a session for the password given.
    NavidromeSession(Result<bool, String>),
    /// The files: how many songs, and whether the folders are being read;
    /// `None` without folders.
    Files(Option<(usize, bool)>),
    /// The files changed: their albums, artists and recent additions to read again.
    FilesChanged,
    /// Files opened from elsewhere, as songs, to play.
    Opened(Vec<Track>),
    /// The tags were written, or why some couldn't be.
    TagsWritten(Result<(), String>),
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
    /// A song left the library (the message to show), or why not: then the
    /// library, changed at once, is loaded again as it is.
    RemovedFromLibrary(Result<String, String>),
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
    /// Suggest Less was set (`set: Some(on)`) or looked up; as `Favourite`.
    SuggestsLess {
        id: ItemId,
        set: Option<bool>,
        result: Result<Option<bool>, String>,
    },
    /// The link to copy, none if there is none, or why not.
    ShareLink(Result<Option<String>, String>),
    /// A service's library songs, the last added first, all of them.
    NewestSongs {
        provider: ProviderKind,
        ids: Vec<ItemId>,
    },
    NotSignedIn,
    Failed(String),
    /// Apple Music couldn't be reached.
    CouldntConnect(String),
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
    /// A service's albums or artists, as its songs come.
    LibraryAlbums {
        provider: ProviderKind,
        albums: Vec<Album>,
        done: bool,
        whole: bool,
    },
    LibraryArtists {
        provider: ProviderKind,
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
        result: Result<Arc<opium_core::model::AlbumExtras>, String>,
    },
    RecentlyAdded {
        provider: ProviderKind,
        items: Vec<Collection>,
        done: bool,
    },
    /// Home's, Radio's or New's shelves, from `source`.
    Home {
        source: ProviderKind,
        result: Result<Vec<Shelf>, String>,
    },
    Fetched {
        token: player::Token,
        result: Result<Playable, String>,
    },
    StationTracks {
        id: ItemId,
        result: Result<Vec<Track>, String>,
    },
    Radio {
        source: ProviderKind,
        result: Result<Vec<Shelf>, String>,
    },
    NewMusic {
        source: ProviderKind,
        result: Result<Vec<Shelf>, String>,
    },
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
    /// What the lyrics sites had for a track, and which one.
    SiteLyrics {
        id: ItemId,
        found: Option<(Lyrics, lyrics::Source)>,
    },
    Search {
        scope: SearchScope,
        source: Option<ProviderKind>,
        query: String,
        result: Result<SearchResults, String>,
    },
}

/// What a fetch hands the player: a track, a station's stream, or a whole
/// file in memory.
pub enum Playable {
    /// An Apple Music track, the bit rate of its encode when known, and
    /// whether it is Spatial Audio.
    Track {
        stream: AudioStream,
        kbps: Option<u32>,
        spatial: bool,
    },
    /// A SoundCloud track, in memory or still downloading.
    SoundCloud(soundcloud::Audio),
    /// A Navidrome file, as it is on the server, still downloading.
    Navidrome(navidrome::Audio),
    /// A music file of this computer.
    Local(local::Audio),
    Live(LiveStream),
    /// A song downloaded, played from the disk.
    Stored(Box<downloads::store::Open>),
}

pub struct Backend {
    requests: UnboundedSender<Request>,
    replies: mpsc::Receiver<Reply>,
    own: Own,
}

/// What Opium keeps of its own, its playlists, answered at once rather
/// than by a service, and shared with the other devices as it changes.
pub struct Own {
    pub kept: synced::Kept,
    pub share: connect::Sharer,
    replies: Option<Replies>,
}

impl Own {
    pub fn new(kept: synced::Kept, share: connect::Sharer) -> Self {
        Self {
            kept,
            share,
            replies: None,
        }
    }

    fn reply(&self, reply: Reply) {
        if let Some(replies) = &self.replies {
            replies.send(reply);
        }
    }

    /// The playlists, and the songs of `loaded`, as they are now.
    fn tell(&self, loaded: &[ItemId]) {
        let playlists = self.kept.playlists();
        self.reply(Reply::Playlists {
            provider: ProviderKind::Opium,
            playlists: playlists.iter().map(synced::Playlist::model).collect(),
        });
        for id in loaded {
            self.reply(Reply::PlaylistTracks {
                id: id.clone(),
                result: match playlists.iter().find(|p| p.id == id.id) {
                    Some(p) => Ok(p.tracks()),
                    None => Err("This playlist was deleted".to_owned()),
                },
                done: true,
                whole: true,
            });
        }
    }

    fn share(&self, change: synced::Shared) {
        self.share.share(change);
    }

    /// Answer `request` if it is about Opium's own; hand it back if it is a
    /// service's.
    fn answer(&self, request: Request) -> Option<Request> {
        let opium = |id: &ItemId| id.provider == ProviderKind::Opium;
        match request {
            Request::Library | Request::Playlists => {
                self.tell(&[]);
                Some(request)
            }
            Request::Playlist(id) if opium(&id) => {
                self.tell(&[id]);
                None
            }
            Request::CreateOpiumPlaylist {
                name,
                description,
                tracks,
            } => {
                let (id, change) = self.kept.create_playlist(name, description, tracks);
                self.share(change);
                if let Some(p) = self.kept.playlist(&id) {
                    self.reply(Reply::PlaylistCreated(Ok(p.model())));
                }
                self.tell(&[]);
                None
            }
            Request::AddToOpiumPlaylist { playlist, tracks } => {
                self.share(self.kept.add_to_playlist(&playlist.id, tracks));
                self.tell(&[playlist]);
                None
            }
            Request::EditPlaylist {
                id,
                name,
                description,
            } if opium(&id) => {
                self.share(
                    self.kept
                        .edit_playlist(&id.id, name.clone(), description.clone()),
                );
                self.reply(Reply::PlaylistEdited {
                    id,
                    name,
                    description,
                    result: Ok(()),
                });
                None
            }
            Request::SetPlaylistCover { id, jpeg } if opium(&id) => {
                self.share(self.kept.cover_playlist(&id.id, jpeg.as_deref()));
                self.reply(Reply::CoverChanged {
                    id: id.clone(),
                    result: Ok(()),
                });
                self.tell(&[id]);
                None
            }
            Request::DeletePlaylist(id) if opium(&id) => {
                self.share(self.kept.delete_playlist(&id.id));
                self.reply(Reply::PlaylistDeleted { id, result: Ok(()) });
                self.tell(&[]);
                None
            }
            // Every entry of the song, as a service's playlist does.
            Request::RemoveFromPlaylist { playlist, entry } if opium(&playlist) => {
                let entries = self
                    .kept
                    .playlist(&playlist.id)
                    .map(|p| p.entries_of(&entry))
                    .unwrap_or_default();
                self.share(self.kept.remove_from_playlist(&playlist.id, &entries));
                self.reply(Reply::RemovedFromPlaylist {
                    playlist: playlist.clone(),
                    result: Ok(()),
                });
                self.tell(&[playlist]);
                None
            }
            Request::AddToPlaylists { playlists, tracks } => {
                let playlists: Vec<ItemId> = playlists.into_iter().filter(|p| !opium(p)).collect();
                (!playlists.is_empty()).then_some(Request::AddToPlaylists { playlists, tracks })
            }
            // Nothing else is Opium's to answer, nor a service's.
            request if target(&request) == Some(ProviderKind::Opium) => None,
            request => Some(request),
        }
    }
}

/// Where the Apple Music sign-in keeps its token (see the `am` example):
/// `apple-music` in the config folder.
///
/// # Panics
///
/// Without a config folder, as [`crate::settings::dir`].
pub fn apple_music_dir() -> PathBuf {
    crate::settings::dir().join("apple-music")
}

/// Where the SoundCloud sign-in keeps its tokens (see the `sc` example).
fn soundcloud_dir() -> PathBuf {
    apple_music_dir().with_file_name("soundcloud")
}

/// A directory only the user can open, made if missing.
fn private_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
}

/// Write a secret readable by the user only, in a folder only the user can
/// open, made if missing: whole or not at all
/// ([`write_whole`](opium_core::files::write_whole)), so that a crash midway
/// leaves the last one whole.
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        private_dir(dir)?;
    }
    opium_core::files::write_whole(path, bytes, opium_core::files::Mode::Private)
}

/// The folders whose files are written whole, each beside its partial
/// file: the config folder with its `apple-music/`, `soundcloud/`,
/// `navidrome/` and the playlists' covers; in the cache folder, the library
/// as last read, the covers, the animated covers, and the files' index and
/// covers. Not the downloads, which sweep their own, nor a folder the user
/// chose, nor the sign-in browsers' profiles.
///
/// # Panics
///
/// Without a config folder, as [`crate::settings::dir`].
fn written_whole() -> Vec<PathBuf> {
    let config = crate::settings::dir();
    let local = local_dir();
    let mut folders = vec![
        config.join(synced::COVERS_FOLDER),
        apple_music_dir(),
        soundcloud_dir(),
        navidrome_dir(),
        config,
    ];
    folders.extend(local.as_ref().map(|local| local.join(local::COVERS_FOLDER)));
    folders.extend(
        [
            crate::cache::dir(),
            crate::covers::dir(),
            crate::covers::motion_dir(),
            local,
        ]
        .into_iter()
        .flatten(),
    );
    folders
}

/// Remove the partial files a crash left in the folders written whole
/// ([`opium_core::files::sweep_partials`]), saying on standard error where
/// any went. Call it at start, right after [`crate::instance::take`], before
/// anything is written: `_held`, the profile's lock, proves that no other
/// Opium writes in its config folder.
///
/// The lock guards the config folder only. A second Opium given another
/// config folder but the same cache folder (only `XDG_CONFIG_HOME` set) may
/// have a cache write under way there, which the sweep then makes fail: a
/// cache, read again.
///
/// # Panics
///
/// Without a config folder, as [`crate::settings::dir`]: [`crate::instance::take`]
/// gave none without one.
pub fn sweep_partials(_held: &crate::instance::Instance) {
    for folder in written_whole() {
        let removed = opium_core::files::sweep_partials(&folder);
        if removed > 0 {
            eprintln!(
                "opium: removed {removed} partial files a crash left in {}",
                folder.display()
            );
        }
    }
}

/// Keep SoundCloud's tokens, readable by the user only.
fn keep_soundcloud_tokens(tokens: &soundcloud::Tokens) {
    let json = serde_json::json!({ "oauth": tokens.oauth, "refresh": tokens.refresh });
    let _ = write_private(
        &soundcloud_dir().join("tokens.json"),
        json.to_string().as_bytes(),
    );
}

/// What keeps the tokens SoundCloud renews for the sign-in there is now, in
/// a connect or any request after it: none once that sign-in was signed out
/// of, so that a renewal still running never writes back the tokens the
/// sign-out deleted.
fn keep_soundcloud_tokens_while_signed_in(sign_outs: &SignOuts) -> soundcloud::OnTokens {
    let (sign_outs, since) = (sign_outs.clone(), sign_outs.count(ProviderKind::SoundCloud));
    Box::new(move |tokens| {
        sign_outs.unless_signed_out_since(ProviderKind::SoundCloud, since, || {
            keep_soundcloud_tokens(tokens)
        });
    })
}

/// What keeps, at `path`, the bot guard's cookie that SoundCloud renews for
/// the sign-in there is now, in the changes sent through its browser: none
/// once that sign-in was signed out of, so that a change still under way
/// never writes back the cookie the sign-out deleted, as
/// [`keep_soundcloud_tokens_while_signed_in`] does for the tokens.
fn keep_soundcloud_cookie_while_signed_in(
    sign_outs: &SignOuts,
    path: PathBuf,
) -> crate::soundcloud_browser::KeepCookie {
    let (sign_outs, since) = (sign_outs.clone(), sign_outs.count(ProviderKind::SoundCloud));
    Box::new(move |cookie| {
        sign_outs.unless_signed_out_since(ProviderKind::SoundCloud, since, || {
            let _ = write_private(&path, cookie.as_bytes());
        });
    })
}

fn kept_soundcloud_tokens() -> Option<soundcloud::Tokens> {
    let text = std::fs::read_to_string(soundcloud_dir().join("tokens.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    Some(soundcloud::Tokens {
        oauth: json["oauth"].as_str()?.to_owned(),
        refresh: json["refresh"].as_str().map(str::to_owned),
    })
}

/// Where the files' index and covers are kept: a cache, read again from the
/// files if it goes. `None` without a cache folder.
fn local_dir() -> Option<PathBuf> {
    Some(crate::folders::cache()?.join("local"))
}

/// Why the files can't be opened without [`local_dir`].
const NO_CACHE_FOLDER: &str = concat!(
    "no cache folder to keep their index and covers in: ",
    "neither XDG_CACHE_HOME nor HOME names an absolute folder"
);

/// The files of `folders`, as indexed last, then read again and watched.
fn open_local(
    folders: &[PathBuf],
    again: &UnboundedSender<Request>,
    replies: &Replies,
) -> (Option<Arc<local::Local>>, Option<local::Watching>) {
    if folders.is_empty() {
        replies.send(Reply::Files(None));
        return (None, None);
    }
    let Some(dir) = local_dir() else {
        eprintln!("files: not opened, {NO_CACHE_FOLDER}");
        replies.send(Reply::Files(None));
        return (None, None);
    };
    let files = Arc::new(local::Local::open(folders.to_vec(), &dir));
    show_local(&files, true, replies);
    let changed = again.clone();
    let watching = files.watch(move || {
        let _ = changed.send(Request::RescanLocal);
    });
    let _ = again.send(Request::RescanLocal);
    (Some(files), watching)
}

/// The files' songs, and how many, `scanning` or not.
fn show_local(files: &local::Local, scanning: bool, replies: &Replies) {
    replies.send(Reply::Songs {
        provider: ProviderKind::Local,
        tracks: files.tracks(),
        done: true,
        whole: true,
    });
    replies.send(Reply::Files(Some((files.count(), scanning))));
}

/// The files' albums, artists or recent additions.
async fn local_lists(files: Arc<local::Local>, request: Request, replies: Replies) {
    let provider = ProviderKind::Local;
    let reply = match request {
        Request::LibraryAlbums => {
            files
                .library_albums(None)
                .await
                .map(|page| Reply::LibraryAlbums {
                    provider,
                    albums: page.items,
                    done: true,
                    whole: true,
                })
        }
        Request::LibraryArtists => {
            files
                .library_artists(None)
                .await
                .map(|page| Reply::LibraryArtists {
                    provider,
                    artists: page.items,
                    done: true,
                    whole: true,
                })
        }
        _ => files
            .recently_added(None)
            .await
            .map(|page| Reply::RecentlyAdded {
                provider,
                items: page.items,
                done: true,
            }),
    };
    if let Ok(reply) = reply {
        replies.send(reply);
    }
}

/// A request about a music file: played from the disk, whether or not it
/// is in the folders; the rest asked of the files' library.
fn local_request(
    files: Option<Arc<local::Local>>,
    request: Request,
    replies: Replies,
    warming: &mut Option<tokio::task::JoinHandle<()>>,
) {
    let kept = files.clone();
    let fetch = |track: ItemId, _original: bool| async move {
        let audio = match &kept {
            Some(files) => files.audio(&track),
            None => local::file_audio(&track),
        };
        audio.map(Playable::Local).map_err(|e| e.to_string())
    };
    let Some(request) = fetch_with(request, replies.clone(), warming, fetch) else {
        return;
    };
    let Some(files) = files else {
        // A file opened from elsewhere has its lyrics all the same.
        if let Request::Lyrics(id) = request {
            tokio::task::spawn_blocking(move || {
                let result = Ok(local::lyrics_of(Path::new(&id.id), ""));
                replies.send(Reply::Lyrics { id, result });
            });
            return;
        }
        return unanswered(request, "Files", &replies);
    };
    let Some(request) = shared_request(files.clone(), request, replies.clone()) else {
        return;
    };
    if item_request(files, request, replies.clone()).is_some() {
        replies.send(Reply::Changed(Err("That isn't something files do".into())));
    }
}

/// Where the Navidrome sign-in is kept: `navidrome` in the config folder.
fn navidrome_dir() -> PathBuf {
    apple_music_dir().with_file_name("navidrome")
}

/// Where the Navidrome sign-in keeps the server, the user and the token.
fn navidrome_file() -> PathBuf {
    navidrome_dir().join("credentials.json")
}

/// Keep the server's sign-in, readable by the user only.
fn keep_navidrome(credentials: &navidrome::Credentials) -> Result<(), String> {
    let json = serde_json::to_string(credentials).map_err(|e| e.to_string())?;
    write_private(&navidrome_file(), json.as_bytes()).map_err(|e| e.to_string())
}

/// What keeps the session a connect started now renews: nothing once the
/// sign-in was signed out of, so that a connect still running never writes
/// back the sign-in the sign-out deleted.
fn keep_navidrome_while_signed_in(
    sign_outs: &SignOuts,
) -> impl FnOnce(&navidrome::Credentials) + Send + 'static {
    let (sign_outs, since) = (sign_outs.clone(), sign_outs.count(ProviderKind::Navidrome));
    move |credentials| {
        sign_outs.unless_signed_out_since(ProviderKind::Navidrome, since, || {
            let _ = keep_navidrome(credentials);
        });
    }
}

/// What a sign-in left on the disk, gone: the tokens, the browser profile
/// (its cookies), and the library kept from the account.
fn forget_sign_in(provider: ProviderKind) {
    let gone = |path: PathBuf| {
        let _ = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
    };
    match provider {
        ProviderKind::AppleMusic => {
            gone(apple_music_dir().join("media-user-token"));
            gone(apple_music_dir().join("browser-profile"));
        }
        ProviderKind::SoundCloud => {
            gone(soundcloud_dir().join("tokens.json"));
            gone(soundcloud_dir().join("datadome"));
            gone(soundcloud_dir().join("browser-profile"));
        }
        ProviderKind::Navidrome => gone(navidrome_file()),
        ProviderKind::Opium => {}
        // The files have no sign-in.
        ProviderKind::Local => return,
    }
    cache::clear();
}

/// Connect to the Navidrome server kept, `keep` keeping the session it
/// renews.
async fn connect_navidrome(keep: impl FnOnce(&navidrome::Credentials)) -> Ended {
    let kept = std::fs::read_to_string(navidrome_file())
        .ok()
        .and_then(|text| serde_json::from_str::<navidrome::Credentials>(&text).ok());
    let Some(credentials) = kept else {
        return Ended {
            connection: Connection::Navidrome(None),
            told: Some(Reply::Navidrome(Ok(None))),
        };
    };
    let (server, told) = match Navidrome::connect(credentials).await {
        Ok(nd) => {
            // Its session renewed as it connected: kept so.
            keep(&nd.credentials());
            let told = Reply::Navidrome(Ok(Some(nd.user().to_owned())));
            (Some(Arc::new(nd)), told)
        }
        Err(Error::NotSignedIn) => (None, Reply::Navidrome(Ok(None))),
        Err(e) => (None, Reply::Navidrome(Err(e.to_string()))),
    };
    Ended {
        connection: Connection::Navidrome(server),
        told: Some(told),
    }
}

/// Connect to SoundCloud with the kept tokens, `keep` keeping those it
/// renews and `keep_cookie` its bot guard's cookie.
async fn connect_soundcloud(
    replies: &Replies,
    keep: soundcloud::OnTokens,
    keep_cookie: crate::soundcloud_browser::KeepCookie,
) -> Ended {
    let unconnected = |told| Ended {
        connection: Connection::SoundCloud(None),
        told: Some(told),
    };
    let Some(tokens) = kept_soundcloud_tokens() else {
        return unconnected(Reply::SoundCloud(Ok(None)));
    };
    // Its changes go through a browser: without one it can't connect, and
    // says why rather than stay connecting.
    let Some(browser) = sign_in_browser() else {
        return unconnected(Reply::SoundCloud(Err(NO_BROWSER.to_owned())));
    };
    let notify = replies.clone();
    let browser = crate::soundcloud_browser::ChromeLike::new(
        browser,
        soundcloud_dir().join("browser-profile"),
        soundcloud_dir().join("datadome"),
        keep_cookie,
        Box::new(move |message| notify.send(Reply::Changed(Ok(message)))),
    )
    .ok()
    .map(|b| Arc::new(b) as Arc<dyn soundcloud::Browser>);
    match SoundCloud::connect(tokens, keep, browser).await {
        Ok(sc) => {
            let told = Reply::SoundCloud(Ok(Some(sc.username().to_owned())));
            Ended {
                connection: Connection::SoundCloud(Some(Arc::new(sc))),
                told: Some(told),
            }
        }
        Err(Error::NotSignedIn) => unconnected(Reply::SoundCloud(Ok(None))),
        Err(e) => unconnected(Reply::SoundCloud(Err(e.to_string()))),
    }
}

/// Open the browser on SoundCloud's sign-in page, wait for the user to sign
/// in and close it, and keep the tokens.
async fn sign_in_soundcloud() -> Result<(), String> {
    let profile = soundcloud_dir().join("browser-profile");
    private_dir(&profile).map_err(|e| e.to_string())?;
    let browser = sign_in_browser().ok_or(NO_BROWSER)?;
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
    wake: crate::Wake,
}

impl Replies {
    fn send(&self, reply: Reply) {
        let _ = self.tx.send(reply);
        (self.wake)();
    }
}

impl Backend {
    /// Start, connecting to every service but those `off`.
    pub fn start(wake: crate::Wake, off: Vec<ProviderKind>, mut own: Own) -> Self {
        // What the app keeps, sign-ins and browser profiles among it, for the
        // user's eyes only, folders made before included.
        for dir in [
            crate::settings::dir(),
            apple_music_dir(),
            apple_music_dir().join("browser-profile"),
            soundcloud_dir(),
            soundcloud_dir().join("browser-profile"),
        ] {
            if dir.exists() {
                let _ = private_dir(&dir);
            }
        }
        let (requests, request_rx) = tokio::sync::mpsc::unbounded_channel();
        let again = requests.clone();
        let seen: Arc<dyn apple_music::SeenElsewhere> =
            Arc::new(SeenShared(own.kept.clone(), own.share.clone()));
        let lends = own.share.clone();
        let (tx, replies) = mpsc::channel();
        own.replies = Some(Replies {
            tx: tx.clone(),
            wake: wake.clone(),
        });
        let replies_out = Replies { tx, wake };
        std::thread::Builder::new()
            .name("backend".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("start the tokio runtime");
                runtime.block_on(run(request_rx, again, replies_out, off, seen, lends));
            })
            .expect("spawn the backend thread");
        Self {
            requests,
            replies,
            own,
        }
    }

    pub fn send(&self, request: Request) {
        if let Some(request) = self.own.answer(request) {
            let _ = self.requests.send(request);
        }
    }

    /// Opium's own playlists changed on another device: what shows of
    /// them, and the songs of those `loaded`, again.
    pub fn own_changed(&self, loaded: &[ItemId]) {
        self.own.tell(loaded);
    }

    pub fn try_recv(&self) -> Option<Reply> {
        self.replies.try_recv().ok()
    }
}

/// What a sign-in needs, said as the user can act on it.
pub const NO_BROWSER: &str = "Signing in needs one of Chromium, Chrome, Brave, Vivaldi or Edge installed: Opium opens the service's own page in it. None was found on this computer.";

/// The browser the sign-in opens, to call it by in the window: none when
/// there is none to open.
pub fn browser_name() -> Option<String> {
    static FOUND: std::sync::LazyLock<Option<String>> = std::sync::LazyLock::new(|| {
        sign_in_browser().map(|browser| {
            std::path::Path::new(&browser)
                .file_name()
                .unwrap_or(browser.as_os_str())
                .to_string_lossy()
                .into_owned()
        })
    });
    FOUND.clone()
}

/// The browser the sign-in opens: `OPIUM_BROWSER`, else the first
/// Chromium-family browser on the `PATH`; none when none is installed.
fn sign_in_browser() -> Option<std::ffi::OsString> {
    const BROWSERS: [&str; 9] = [
        "brave-origin",
        "brave",
        "brave-browser",
        "chromium",
        "chromium-browser",
        "google-chrome-stable",
        "google-chrome",
        "vivaldi",
        "microsoft-edge",
    ];
    if let Some(browser) = std::env::var_os("OPIUM_BROWSER") {
        return Some(browser);
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    BROWSERS
        .iter()
        .find(|b| std::env::split_paths(&path).any(|dir| dir.join(b).is_file()))
        .map(|b| (*b).into())
}

/// Apple's newest songs as first seen, kept in common with the other devices.
struct SeenShared(synced::Kept, connect::Sharer);

impl apple_music::SeenElsewhere for SeenShared {
    fn known(&self, ids: &[String]) -> std::collections::HashMap<String, u64> {
        self.0.seen(ids)
    }

    fn seen_here(&self, times: Vec<(String, u64)>) {
        self.1.share(self.0.first_seen(times));
    }
}

/// Connect to Apple Music with the saved token.
async fn connect(seen: Arc<dyn apple_music::SeenElsewhere>) -> Ended {
    let Ok(token) = std::fs::read_to_string(apple_music_dir().join("media-user-token")) else {
        return Ended {
            connection: Connection::Apple(None),
            told: Some(Reply::NotSignedIn),
        };
    };
    let (am, told) = match AppleMusic::connect(token.trim()).await {
        Ok(am) => {
            am.keep_seen_in(crate::settings::dir().join("apple-seen.json"), Some(seen));
            let told = Reply::Connected {
                storefront: am.storefront().to_owned(),
            };
            (Some(Arc::new(am)), told)
        }
        Err(Error::NotSignedIn) => (None, Reply::NotSignedIn),
        Err(e) => (None, Reply::CouldntConnect(e.to_string())),
    };
    Ended {
        connection: Connection::Apple(am),
        told: Some(told),
    }
}

/// The account's picture, once Apple says it.
fn show_profile_picture(am: Arc<AppleMusic>, replies: Replies) {
    tokio::spawn(async move {
        if let Ok(Some(profile)) = am.profile().await
            && let Some(picture) = profile.picture
        {
            replies.send(Reply::ProfilePicture(picture));
        }
    });
}

/// Open the browser on Apple's sign-in page, wait for the user, and save the
/// token readable by the user only, where the `am` example keeps it too.
async fn sign_in() -> Result<(), String> {
    let dir = apple_music_dir();
    let profile = dir.join("browser-profile");
    private_dir(&profile).map_err(|e| e.to_string())?;
    let browser = sign_in_browser().ok_or(NO_BROWSER)?;
    let token = apple_music::signin::sign_in(
        browser.as_ref(),
        &profile,
        std::time::Duration::from_secs(600),
    )
    .await
    .map_err(|e| e.to_string())?;
    write_private(&dir.join("media-user-token"), token.as_bytes()).map_err(|e| e.to_string())
}

/// The server's covers signed by `nd` as they are fetched, or by none.
fn sign_covers_with(nd: &Option<Arc<Navidrome>>) {
    crate::covers::set_signer(nd.clone().map(|nd| {
        Arc::new(move |url: &str| nd.sign(url)) as Arc<dyn Fn(&str) -> Option<String> + Send + Sync>
    }));
}

/// A service's connection as its connect ended: none inside when it
/// couldn't be made.
enum Connection {
    Apple(Option<Arc<AppleMusic>>),
    SoundCloud(Option<Arc<SoundCloud>>),
    Navidrome(Option<Arc<Navidrome>>),
}

/// How a connect ended: the connection, and what the window is told of it,
/// said only if the loop keeps it.
struct Ended {
    connection: Connection,
    told: Option<Reply>,
}

/// A connect of `provider` ended, while requests went on.
struct Arrived {
    provider: ProviderKind,
    /// How it ended; none if it panicked, which changes nothing.
    ended: Option<Ended>,
    /// Its library then loads (once the library was asked for).
    load: bool,
    /// The service's sign-outs counted as it started: one since makes it
    /// stale, the connection of a sign-in gone.
    sign_outs: u64,
}

/// A request as the loop takes it. `only`: the one service whose part of a
/// list several answer is asked, that list having been asked while it
/// connected; none for a request as it was asked.
struct Asked {
    request: Request,
    only: Option<ProviderKind>,
}

/// Each service's sign-outs so far, shared with what keeps the tokens a
/// sign-in renews: under the same lock, tokens renewed for a sign-in signed
/// out of since are never written back once the sign-out has deleted them.
#[derive(Clone, Default)]
struct SignOuts(Arc<std::sync::Mutex<HashMap<ProviderKind, u64>>>);

impl SignOuts {
    fn count(&self, provider: ProviderKind) -> u64 {
        self.counts().get(&provider).copied().unwrap_or(0)
    }

    /// One more sign-out of `provider`: counted before its sign-in is
    /// deleted from the disk.
    fn sign_out(&self, provider: ProviderKind) {
        *self.counts().entry(provider).or_default() += 1;
    }

    /// `keep` run, under the lock, unless `provider` was signed out of since
    /// its sign-outs were `since`: what it returned, or none.
    fn unless_signed_out_since<T>(
        &self,
        provider: ProviderKind,
        since: u64,
        keep: impl FnOnce() -> T,
    ) -> Option<T> {
        let counts = self.counts();
        (counts.get(&provider).copied().unwrap_or(0) == since).then(keep)
    }

    fn counts(&self) -> std::sync::MutexGuard<'_, HashMap<ProviderKind, u64>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// The connects under way, and what was asked of a service while one of
/// them was: held, then handed back to the loop once none is under way, so
/// that it is answered as the service then stands, connected or not. The
/// one place where requests wait on a connect.
struct Connects {
    arrived: UnboundedSender<Arrived>,
    sign_outs: SignOuts,
    /// The connects of each service under way, each by the service's
    /// sign-outs when it started: one started before a sign-out holds
    /// nothing, the loop dropping what it brings.
    under_way: HashMap<ProviderKind, Vec<u64>>,
    /// What waits on each service.
    held: HashMap<ProviderKind, Vec<Asked>>,
    /// What is handed back, in the order it was held.
    ready: VecDeque<Asked>,
}

impl Connects {
    fn new(arrived: UnboundedSender<Arrived>, sign_outs: SignOuts) -> Self {
        Self {
            arrived,
            sign_outs,
            under_way: HashMap::new(),
            held: HashMap::new(),
            ready: VecDeque::new(),
        }
    }

    /// Connect to `provider` through `connect`, off the loop: how it ended
    /// comes back through `arrived`, its library loading then if `load` and
    /// the library was asked for.
    fn start(
        &mut self,
        provider: ProviderKind,
        load: bool,
        connect: impl std::future::Future<Output = Ended> + Send + 'static,
    ) {
        let (arrived, sign_outs) = (self.arrived.clone(), self.sign_outs.count(provider));
        self.under_way.entry(provider).or_default().push(sign_outs);
        tokio::spawn(async move {
            // One that panicked comes back all the same, so that nothing
            // waits on it for ever.
            let ended = tokio::spawn(connect).await.ok();
            let _ = arrived.send(Arrived {
                provider,
                ended,
                load,
                sign_outs,
            });
        });
    }

    /// Whether a connect of `provider` started since its last sign-out is
    /// under way.
    fn under_way(&self, provider: ProviderKind) -> bool {
        let now = self.sign_outs.count(provider);
        self.under_way
            .get(&provider)
            .is_some_and(|started| started.contains(&now))
    }

    /// `request` waits on `provider`'s connect; with `only`, as that one
    /// service's part of a list (see [`Asked`]).
    fn hold(&mut self, provider: ProviderKind, request: Request, only: Option<ProviderKind>) {
        let asked = Asked { request, only };
        self.held.entry(provider).or_default().push(asked);
    }

    /// A connect of `provider`, started at its sign-outs `sign_outs`, came
    /// back: what waits on it is handed back once none started since the
    /// last sign-out is under way.
    fn ended(&mut self, provider: ProviderKind, sign_outs: u64) {
        if let Some(started) = self.under_way.get_mut(&provider)
            && let Some(at) = started.iter().position(|s| *s == sign_outs)
        {
            started.swap_remove(at);
        }
        if !self.under_way(provider) {
            self.release(provider);
        }
    }

    /// The songs of `provider` waiting for its connect to be downloaded,
    /// called off: signed out of.
    fn forget_downloads(&mut self, provider: ProviderKind) {
        if let Some(held) = self.held.get_mut(&provider) {
            held.retain(|asked| !matches!(asked.request, Request::Download { .. }));
        }
    }

    /// `ids` no longer to be downloaded, among the songs waiting for a
    /// connect.
    fn remove_downloads(&mut self, ids: &[ItemId]) {
        for held in self.held.values_mut() {
            held.retain_mut(|asked| match &mut asked.request {
                Request::Download { tracks, .. } => {
                    tracks.retain(|t| !ids.contains(&t.id));
                    !tracks.is_empty()
                }
                _ => true,
            });
        }
    }

    /// What waits on `provider` handed back now, to be answered as it
    /// stands: turned off, or signed out of.
    fn release(&mut self, provider: ProviderKind) {
        self.ready
            .extend(self.held.remove(&provider).unwrap_or_default());
    }

    /// The next request handed back.
    fn next(&mut self) -> Option<Asked> {
        self.ready.pop_front()
    }
}

/// The one service a request is for, which it waits on while that service
/// connects: an item's, a page's, a narrowed search's, an animated cover's
/// (Apple Music's). None for what the files answer, for a list several
/// services answer ([`list_copy`]), and for a song warmed ahead, answered at
/// once and fetched again if it is played.
fn waits_on(request: &Request) -> Option<ProviderKind> {
    match request {
        Request::Warm { .. } => None,
        Request::Home(source) | Request::Radio(source) | Request::NewMusic(source) => Some(*source),
        Request::Search(_, source, _) => *source,
        Request::Motion { .. } => Some(ProviderKind::AppleMusic),
        // Answered at once: a password held for a connect could reach the
        // server another sign-in connects to.
        Request::NavidromeSession(_) => None,
        Request::SetPlaylistCover { id, .. } => Some(id.provider),
        request => target(request),
    }
}

/// A list several services answer, copied to ask again of one that was
/// still connecting: none for any other request. A search of every service
/// is asked whole again, its answer being one.
fn list_copy(request: &Request) -> Option<Request> {
    Some(match request {
        Request::LibraryAlbums => Request::LibraryAlbums,
        Request::LibraryArtists => Request::LibraryArtists,
        Request::RecentlyAdded => Request::RecentlyAdded,
        Request::NewestSongs => Request::NewestSongs,
        Request::Playlists => Request::Playlists,
        Request::Search(scope, None, query) => Request::Search(*scope, None, query.clone()),
        _ => return None,
    })
}

async fn run(
    mut requests: UnboundedReceiver<Request>,
    again: UnboundedSender<Request>,
    replies: Replies,
    off: Vec<ProviderKind>,
    seen: Arc<dyn apple_music::SeenElsewhere>,
    lends: connect::Sharer,
) {
    let on = |p| !off.contains(&p);
    // Each service's sign-outs: a connect started before one is stale, and
    // so is an answer on Apple's subscription asked before one.
    let sign_outs = SignOuts::default();
    // Every connect comes back here once it ends, those of the start as
    // those made later (one turned on, or tried again), so that none holds up
    // the loop: on a stalled network a service takes tens of seconds to give
    // up, and the files, the downloads and the other services would wait on
    // it. What is asked of a service meanwhile waits for it (`Connects`).
    let (arrived_tx, mut arrived) = tokio::sync::mpsc::unbounded_channel::<Arrived>();
    let mut connects = Connects::new(arrived_tx, sign_outs.clone());
    // Those of the start load their library as they come, if it was asked
    // for meanwhile.
    if on(ProviderKind::AppleMusic) {
        connects.start(ProviderKind::AppleMusic, true, connect(seen.clone()));
    }
    if on(ProviderKind::SoundCloud) {
        let keep = keep_soundcloud_tokens_while_signed_in(&sign_outs);
        let keep_cookie =
            keep_soundcloud_cookie_while_signed_in(&sign_outs, soundcloud_dir().join("datadome"));
        let replies = replies.clone();
        connects.start(ProviderKind::SoundCloud, true, async move {
            connect_soundcloud(&replies, keep, keep_cookie).await
        });
    }
    if on(ProviderKind::Navidrome) {
        connects.start(
            ProviderKind::Navidrome,
            true,
            connect_navidrome(keep_navidrome_while_signed_in(&sign_outs)),
        );
    }
    let mut apple: Option<Arc<AppleMusic>> = None;
    let mut sc: Option<Arc<SoundCloud>> = None;
    let mut nd = None;
    // Asked for once: a service connected afterwards loads its library.
    let mut library_wanted = false;
    // Recently Added was asked for: SoundCloud answers once connected.
    let mut recent_wanted = false;
    // Turned off meanwhile: a connection made before is dropped as it comes.
    let mut turned_off: std::collections::HashSet<ProviderKind> = off.iter().copied().collect();
    // Signed out of meanwhile, until signed in again.
    let mut signed_out = std::collections::HashSet::new();
    // Apple's answers on the subscription, each with the sign-outs of Apple
    // counted when it was asked (`confirm_subscription`).
    let (answers_tx, mut answers) = tokio::sync::mpsc::unbounded_channel::<(bool, u64)>();
    // Out of reach, their libraries shown as kept: once, not at each try.
    let mut kept_shown = std::collections::HashSet::new();
    // The music files of the folders chosen, and their watching (the app
    // says which folders as it starts).
    let mut local: Option<Arc<local::Local>> = None;
    let mut local_folders: Vec<PathBuf> = Vec::new();
    let mut _watching: Option<local::Watching> = None;
    let cdm = widevine::find_cdm()
        .ok_or_else(|| "no Widevine module found: install Brave, Chrome or Firefox".to_owned())
        .and_then(|path| widevine::Cdm::load(&path).map_err(|e| e.to_string()));

    let http = reqwest::Client::new();
    let finder = Arc::new(lyrics::Finder::new());
    let mut loading = Loading::default();
    let mut warming: Option<tokio::task::JoinHandle<()>> = None;
    let downloads = downloading::Downloads::open(replies.clone());
    // The paired phone borrows what it can't download itself.
    lends.lend(downloads.lender());
    loop {
        // What the next download fetches with.
        downloads.services(downloading::Services {
            apple: apple.clone(),
            soundcloud: sc.clone(),
            navidrome: nd.clone(),
            cdm: cdm.clone().ok(),
        });
        // What waited on a connect first, then what comes.
        let Asked { request, only } = match connects.next() {
            Some(asked) => asked,
            None => tokio::select! {
                Some(came) = arrived.recv() => {
                    let replies = replies.clone();
                    let (provider, load) = (came.provider, came.load);
                    // What waited on it is asked again once the loop has taken it in.
                    connects.ended(provider, came.sign_outs);
                    // Turned off or signed out of meanwhile, or started before a
                    // sign-out: dropped, and nothing said of it.
                    let kept = !turned_off.contains(&provider)
                        && !signed_out.contains(&provider)
                        && came.sign_outs == sign_outs.count(provider);
                    let Some(Ended { connection, told }) = came.ended.filter(|_| kept) else {
                        continue;
                    };
                    if let Some(told) = told {
                        replies.send(told);
                    }
                    match connection {
                        Connection::Apple(am) => {
                            apple = am;
                            if let Some(am) = apple.clone() {
                                kept_shown.remove(&provider);
                                let asked_after = sign_outs.count(provider);
                                confirm_subscription(am.clone(), answers_tx.clone(), asked_after);
                                show_profile_picture(am, replies.clone());
                            }
                            if let Some(am) = apple.clone().filter(|_| load && library_wanted) {
                                loading.start(ProviderKind::AppleMusic, load_library(am, replies));
                            }
                        }
                        Connection::SoundCloud(c) => {
                            sc = c;
                            // Signed in still: its songs play on.
                            if sc.is_some() {
                                kept_shown.remove(&provider);
                                downloads.subscription(ProviderKind::SoundCloud, true);
                            }
                            if let Some(sc) = sc.clone().filter(|_| load && recent_wanted) {
                                tokio::spawn(soundcloud_recent(sc, replies.clone()));
                            }
                            if let Some(sc) = sc.clone().filter(|_| load && library_wanted) {
                                loading.start(
                                    ProviderKind::SoundCloud,
                                    load_soundcloud_library(sc, replies),
                                );
                            }
                        }
                        Connection::Navidrome(server) => {
                            nd = server;
                            sign_covers_with(&nd);
                            if nd.is_some() {
                                kept_shown.remove(&provider);
                            } else if library_wanted
                                && signed_in(provider)
                                && kept_shown.insert(provider)
                            {
                                show_kept(provider, replies.clone());
                            }
                            if let Some(nd) = nd.clone().filter(|_| load && library_wanted) {
                                loading.start(
                                    ProviderKind::Navidrome,
                                    load_navidrome_library(nd, replies),
                                );
                            }
                        }
                    }
                    continue;
                }
                Some((active, asked_after)) = answers.recv() => {
                    // Asked before a sign-out, the answer is about the account
                    // that left: its songs mustn't play 30 days more.
                    if asked_after == sign_outs.count(ProviderKind::AppleMusic) {
                        downloads.subscription(ProviderKind::AppleMusic, active);
                    }
                    continue;
                }
                request = requests.recv() => match request {
                    Some(request) => Asked { request, only: None },
                    None => break,
                },
            },
        };
        let replies = replies.clone();
        // A song downloaded plays from the disk, whatever is connected.
        if let Request::Fetch {
            token,
            track,
            fallback,
            ..
        }
        | Request::Warm {
            token,
            track,
            fallback,
            ..
        } = &request
            && let Some(playable) = std::iter::once(track)
                .chain(fallback)
                .find_map(|id| downloads.playable(id))
        {
            if matches!(request, Request::Warm { .. })
                && let Some(last) = warming.take()
            {
                last.abort();
            }
            replies.send(Reply::Fetched {
                token: *token,
                result: Ok(playable),
            });
            continue;
        }
        // The services still connecting: a connect under way whose
        // connection the loop will take in (neither turned off nor signed out
        // of since). What is for one of them alone waits for it, and so does
        // its part of a list several answer, the others answering at once.
        let connecting: Vec<ProviderKind> = [
            ProviderKind::AppleMusic,
            ProviderKind::SoundCloud,
            ProviderKind::Navidrome,
        ]
        .into_iter()
        .filter(|p| connects.under_way(*p) && !turned_off.contains(p) && !signed_out.contains(p))
        .collect();
        if only.is_none() {
            if let Some(provider) = waits_on(&request).filter(|p| connecting.contains(p)) {
                connects.hold(provider, request, None);
                continue;
            }
            for &provider in &connecting {
                if let Some(part) = list_copy(&request) {
                    connects.hold(provider, part, Some(provider));
                }
            }
        }
        // Whether a list several answer asks `provider`'s part now: only the
        // one it waited on, when handed back; else those not connecting.
        let asks = |provider: ProviderKind| match only {
            Some(waited_on) => provider == waited_on,
            None => !connecting.contains(&provider),
        };
        match request {
            Request::Download { tracks, original } => {
                // A song of a service still connecting waits for it.
                let (later, now): (Vec<Track>, Vec<Track>) = tracks
                    .into_iter()
                    .partition(|t| connecting.contains(&t.id.provider));
                for &provider in &connecting {
                    let tracks: Vec<Track> = later
                        .iter()
                        .filter(|t| t.id.provider == provider)
                        .cloned()
                        .collect();
                    if !tracks.is_empty() {
                        let request = Request::Download { tracks, original };
                        connects.hold(provider, request, None);
                    }
                }
                if !now.is_empty() {
                    downloads.add(now, original);
                }
                continue;
            }
            Request::RemoveDownloads(ids) => {
                connects.remove_downloads(&ids);
                downloads.remove(ids);
                continue;
            }
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
                signed_out.remove(&ProviderKind::AppleMusic);
                connects.start(ProviderKind::AppleMusic, false, connect(seen.clone()));
                continue;
            }
            Request::SetLocalFolders(folders) => {
                local_folders = folders;
                (local, _watching) = if turned_off.contains(&ProviderKind::Local) {
                    (None, None)
                } else {
                    open_local(&local_folders, &again, &replies)
                };
                continue;
            }
            Request::RescanLocal => {
                if let Some(files) = local.clone() {
                    tokio::spawn(async move {
                        let scanned = files.clone();
                        let Ok(changed) =
                            tokio::task::spawn_blocking(move || scanned.rescan()).await
                        else {
                            return;
                        };
                        if changed {
                            show_local(&files, false, &replies);
                            replies.send(Reply::FilesChanged);
                        } else {
                            replies.send(Reply::Files(Some((files.count(), false))));
                        }
                    });
                }
                continue;
            }
            Request::OpenFiles(paths) => {
                let Some(dir) = local_dir() else {
                    eprintln!("files: not opened, {NO_CACHE_FOLDER}");
                    replies.send(Reply::Opened(Vec::new()));
                    continue;
                };
                let covers = dir.join("covers");
                tokio::spawn(async move {
                    let opened = tokio::task::spawn_blocking(move || {
                        local::files_in(&paths)
                            .iter()
                            .filter_map(|path| local::file_track(path, &covers))
                            .collect::<Vec<Track>>()
                    })
                    .await
                    .unwrap_or_default();
                    replies.send(Reply::Opened(opened));
                });
                continue;
            }
            Request::WriteTags { paths, edit } => {
                let files = local.clone();
                tokio::spawn(async move {
                    let written = tokio::task::spawn_blocking(move || {
                        let failed: Vec<String> = paths
                            .iter()
                            .filter_map(|path| {
                                let name = path.file_name()?.to_string_lossy().into_owned();
                                local::write_tags(path, &edit)
                                    .err()
                                    .map(|e| format!("{name}: {e}"))
                            })
                            .collect();
                        // Read again at once, not when the watching settles.
                        let changed = files.as_ref().is_some_and(|f| f.rescan());
                        (failed, changed, files)
                    })
                    .await;
                    let Ok((failed, changed, files)) = written else {
                        return replies.send(Reply::TagsWritten(Err(
                            "the tags couldn't be written".into(),
                        )));
                    };
                    if let (true, Some(files)) = (changed, files) {
                        show_local(&files, false, &replies);
                        replies.send(Reply::FilesChanged);
                    }
                    replies.send(Reply::TagsWritten(match failed.is_empty() {
                        true => Ok(()),
                        false => Err(failed.join("; ")),
                    }));
                });
                continue;
            }
            Request::SetService { provider, on } => {
                if on {
                    turned_off.remove(&provider);
                } else {
                    turned_off.insert(provider);
                    // What waited on its connect is answered as it is now: off.
                    connects.release(provider);
                }
                match (provider, on) {
                    (ProviderKind::AppleMusic, false) => {
                        apple = None;
                        loading.stop(provider);
                    }
                    (ProviderKind::SoundCloud, false) => {
                        sc = None;
                        loading.stop(provider);
                    }
                    (ProviderKind::Navidrome, false) => {
                        nd = None;
                        sign_covers_with(&nd);
                        loading.stop(provider);
                    }
                    (ProviderKind::Local, false) => {
                        (local, _watching) = (None, None);
                    }
                    (ProviderKind::Local, true) => {
                        (local, _watching) = open_local(&local_folders, &again, &replies);
                    }
                    (ProviderKind::Opium, _) => {}
                    // Turned on: its library loads once it is connected.
                    (ProviderKind::AppleMusic, true) => {
                        connects.start(provider, true, connect(seen.clone()));
                    }
                    (ProviderKind::SoundCloud, true) => {
                        let keep = keep_soundcloud_tokens_while_signed_in(&sign_outs);
                        let keep_cookie = keep_soundcloud_cookie_while_signed_in(
                            &sign_outs,
                            soundcloud_dir().join("datadome"),
                        );
                        connects.start(provider, true, async move {
                            connect_soundcloud(&replies, keep, keep_cookie).await
                        });
                    }
                    (ProviderKind::Navidrome, true) => {
                        connects.start(
                            provider,
                            true,
                            connect_navidrome(keep_navidrome_while_signed_in(&sign_outs)),
                        );
                    }
                }
                continue;
            }
            Request::SignOut(provider) => {
                // A connection on its way is dropped as it comes, with what it
                // would have said; tokens it renews are no longer kept. Counted
                // before the sign-in is deleted below.
                signed_out.insert(provider);
                sign_outs.sign_out(provider);
                // What waited on its connect is answered as it is now: signed
                // out; its songs waiting to be downloaded are called off.
                connects.forget_downloads(provider);
                connects.release(provider);
                downloads.forget(provider);
                loading.stop(provider);
                match provider {
                    ProviderKind::AppleMusic => apple = None,
                    ProviderKind::SoundCloud => sc = None,
                    ProviderKind::Navidrome => {
                        nd = None;
                        sign_covers_with(&nd);
                    }
                    ProviderKind::Local | ProviderKind::Opium => {}
                }
                tokio::task::spawn_blocking(move || forget_sign_in(provider));
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
                signed_out.remove(&ProviderKind::SoundCloud);
                let keep = keep_soundcloud_tokens_while_signed_in(&sign_outs);
                let keep_cookie = keep_soundcloud_cookie_while_signed_in(
                    &sign_outs,
                    soundcloud_dir().join("datadome"),
                );
                connects.start(ProviderKind::SoundCloud, true, async move {
                    connect_soundcloud(&replies, keep, keep_cookie).await
                });
                continue;
            }
            Request::ReconnectNavidrome => {
                connects.start(
                    ProviderKind::Navidrome,
                    true,
                    connect_navidrome(keep_navidrome_while_signed_in(&sign_outs)),
                );
                continue;
            }
            Request::SignInNavidrome {
                url,
                user,
                password,
            } => {
                signed_out.remove(&ProviderKind::Navidrome);
                let result = navidrome::Credentials::new(&url, &user, &password);
                let credentials = match result {
                    Ok(c) => c,
                    Err(e) => {
                        replies.send(Reply::SignedInNavidrome(Err(e.to_string())));
                        continue;
                    }
                };
                // Off the loop: reaching a server that isn't there takes
                // tens of seconds, and nothing else — no page, no search, no
                // song starting — would be answered meanwhile. Signed out of
                // meanwhile, the sign-in is neither kept nor told.
                let (guard, since) = (sign_outs.clone(), sign_outs.count(ProviderKind::Navidrome));
                connects.start(ProviderKind::Navidrome, true, async move {
                    let connected = Navidrome::connect(credentials).await;
                    // A session on its own API too, for a cover changed;
                    // the password goes no further.
                    if let Ok(server) = &connected {
                        let _ = server.open_session(&password).await;
                    }
                    drop(password);
                    let (server, told) = match connected {
                        Ok(server) => {
                            let kept = guard.unless_signed_out_since(
                                ProviderKind::Navidrome,
                                since,
                                || keep_navidrome(&server.credentials()),
                            );
                            if let Some(kept) = kept {
                                replies.send(Reply::SignedInNavidrome(kept));
                            }
                            let told = Reply::Navidrome(Ok(Some(server.user().to_owned())));
                            (Some(Arc::new(server)), Some(told))
                        }
                        Err(Error::NotSignedIn) => {
                            replies.send(Reply::SignedInNavidrome(Err(
                                "the server didn't take this user and password".into(),
                            )));
                            (None, None)
                        }
                        Err(e) => {
                            replies.send(Reply::SignedInNavidrome(Err(e.to_string())));
                            (None, None)
                        }
                    };
                    Ended {
                        connection: Connection::Navidrome(server),
                        told,
                    }
                });
                continue;
            }
            // Every service at once.
            Request::Library => {
                library_wanted = true;
                if let Some(files) = local.clone() {
                    show_local(&files, false, &replies);
                }
                if let Some(am) = apple.clone() {
                    loading.start(ProviderKind::AppleMusic, load_library(am, replies.clone()));
                }
                if let Some(sc) = sc.clone() {
                    loading.start(
                        ProviderKind::SoundCloud,
                        load_soundcloud_library(sc, replies.clone()),
                    );
                }
                if let Some(nd) = nd.clone() {
                    loading.start(
                        ProviderKind::Navidrome,
                        load_navidrome_library(nd, replies.clone()),
                    );
                }
                // Those signed in but out of reach, or still connecting: as
                // they were kept, their own coming once connected.
                for (provider, connected) in [
                    (ProviderKind::AppleMusic, apple.is_some()),
                    (ProviderKind::SoundCloud, sc.is_some()),
                    (ProviderKind::Navidrome, nd.is_some()),
                ] {
                    if !connected
                        && !turned_off.contains(&provider)
                        && !signed_out.contains(&provider)
                        && signed_in(provider)
                        && kept_shown.insert(provider)
                    {
                        show_kept(provider, replies.clone());
                    }
                }
                continue;
            }
            Request::NewestSongs => {
                // As kept last time, at once; then as the services have it now.
                for (provider, on) in [
                    (ProviderKind::AppleMusic, apple.is_some()),
                    (ProviderKind::SoundCloud, sc.is_some()),
                    (ProviderKind::Navidrome, nd.is_some()),
                    (ProviderKind::Local, local.is_some()),
                ] {
                    if on
                        && asks(provider)
                        && let Some(ids) = cache::read::<Vec<ItemId>>(&newest_kept(provider))
                    {
                        replies.send(Reply::NewestSongs { provider, ids });
                    }
                }
                if asks(ProviderKind::AppleMusic)
                    && let Some(am) = apple.clone()
                {
                    let replies = replies.clone();
                    tokio::spawn(async move {
                        let mut ids = Vec::new();
                        let read = am
                            .each_newest_library_track(|tracks, _| {
                                ids.extend(tracks.into_iter().map(|t| t.id))
                            })
                            .await;
                        if read.is_ok() {
                            newest_came(ProviderKind::AppleMusic, ids, &replies);
                        }
                    });
                }
                let others: [(ProviderKind, Option<Arc<dyn Provider>>); 3] = [
                    (
                        ProviderKind::SoundCloud,
                        sc.clone().map(|p| p as Arc<dyn Provider>),
                    ),
                    (
                        ProviderKind::Navidrome,
                        nd.clone().map(|p| p as Arc<dyn Provider>),
                    ),
                    (
                        ProviderKind::Local,
                        local.clone().map(|p| p as Arc<dyn Provider>),
                    ),
                ];
                for (provider, p) in others {
                    let (Some(p), true, replies) = (p, asks(provider), replies.clone()) else {
                        continue;
                    };
                    tokio::spawn(async move {
                        let mut ids = Vec::new();
                        let mut cursor: Option<Cursor> = None;
                        loop {
                            let Ok(page) = p.library_tracks_newest(cursor.as_ref()).await else {
                                return;
                            };
                            ids.extend(page.items.into_iter().map(|t| t.id));
                            match page.next {
                                Some(next) => cursor = Some(next),
                                None => break,
                            }
                        }
                        newest_came(provider, ids, &replies);
                    });
                }
                continue;
            }
            // Each service's part: the files' and the connected services' own
            // lists. Out of reach, Apple Music's as kept; another, which has
            // none kept, says it has none when the list waited on it as it
            // connected (`only`), so that the list isn't left waiting.
            Request::LibraryAlbums | Request::LibraryArtists | Request::RecentlyAdded => {
                let list = || match &request {
                    Request::LibraryAlbums => Request::LibraryAlbums,
                    Request::LibraryArtists => Request::LibraryArtists,
                    _ => Request::RecentlyAdded,
                };
                // Apple signed in and on, but out of reach: its lists as kept.
                let apple_kept = !turned_off.contains(&ProviderKind::AppleMusic)
                    && !signed_out.contains(&ProviderKind::AppleMusic)
                    && signed_in(ProviderKind::AppleMusic);
                let out_of_reach = |provider| out_of_reach_list(provider, &request, apple_kept);
                if asks(ProviderKind::Local)
                    && let Some(files) = local.clone()
                {
                    tokio::spawn(local_lists(files, list(), replies.clone()));
                }
                if asks(ProviderKind::Navidrome) {
                    match nd.clone() {
                        Some(nd) => {
                            tokio::spawn(navidrome_lists(nd, list(), replies.clone()));
                        }
                        None if only.is_some() => {
                            replies.send(out_of_reach(ProviderKind::Navidrome));
                        }
                        None => {}
                    }
                }
                // SoundCloud's likes, beside the other services' Recently Added.
                if matches!(request, Request::RecentlyAdded) && asks(ProviderKind::SoundCloud) {
                    recent_wanted = true;
                    match sc.clone() {
                        Some(sc) => {
                            tokio::spawn(soundcloud_recent(sc, replies.clone()));
                        }
                        None if only.is_some() => {
                            replies.send(out_of_reach(ProviderKind::SoundCloud));
                        }
                        None => {}
                    }
                }
                if asks(ProviderKind::AppleMusic) {
                    match apple.clone() {
                        Some(am) => {
                            tokio::spawn(apple_lists(am, list(), replies.clone()));
                        }
                        None => replies.send(out_of_reach(ProviderKind::AppleMusic)),
                    }
                }
                continue;
            }
            Request::Playlists => {
                if asks(ProviderKind::AppleMusic)
                    && let Some(am) = apple.clone()
                {
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
                if asks(ProviderKind::SoundCloud)
                    && let Some(sc) = sc.clone()
                {
                    let replies = replies.clone();
                    tokio::spawn(async move {
                        if let Ok(playlists) = soundcloud_playlists(&sc).await {
                            replies.send(Reply::Playlists {
                                provider: ProviderKind::SoundCloud,
                                playlists,
                            });
                        }
                    });
                }
                if asks(ProviderKind::Navidrome)
                    && let Some(nd) = nd.clone()
                {
                    tokio::spawn(async move {
                        if let Ok(page) = nd.library_playlists(None).await {
                            replies.send(Reply::Playlists {
                                provider: ProviderKind::Navidrome,
                                playlists: page.items,
                            });
                        }
                    });
                }
                continue;
            }
            // No service needed.
            Request::SiteLyrics { track, sources } => {
                let finder = finder.clone();
                tokio::spawn(async move {
                    let query = lyrics::Query {
                        artist: services::first_artist(&track.artists),
                        album: track.album.clone(),
                        duration: track.duration,
                        title: track.title.clone(),
                    };
                    let found = finder.find(&query, &sources).await;
                    replies.send(Reply::SiteLyrics {
                        id: track.id,
                        found,
                    });
                });
                continue;
            }
            // A cover's colours: any service's, or none.
            Request::Palette(url) => {
                let http = http.clone();
                tokio::spawn(async move {
                    // A cover of the user's own (a playlist's, a file's) is
                    // read where it is; another is often kept already, the
                    // page showing it.
                    let bytes = if let Some(path) = crate::file_path(&url) {
                        std::fs::read(path).ok()
                    } else if let Some(bytes) = crate::covers::read(&url) {
                        Some(bytes)
                    } else {
                        match http.get(crate::covers::signed(&url)).send().await {
                            Ok(resp) => resp.bytes().await.ok().map(|b| b.to_vec()),
                            Err(_) => None,
                        }
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
                continue;
            }
            // Home, Radio and New: the service chosen for them, whichever.
            Request::Home(source) | Request::Radio(source) | Request::NewMusic(source) => {
                let provider: Option<Arc<dyn Provider>> = match source {
                    ProviderKind::AppleMusic => apple.clone().map(|a| a as Arc<dyn Provider>),
                    ProviderKind::SoundCloud => sc.clone().map(|s| s as Arc<dyn Provider>),
                    ProviderKind::Navidrome => nd.clone().map(|n| n as Arc<dyn Provider>),
                    // The files have no picks.
                    ProviderKind::Local | ProviderKind::Opium => None,
                };
                let Some(provider) = provider else {
                    unanswered(request, "the service chosen", &replies);
                    continue;
                };
                tokio::spawn(async move {
                    let reply = match request {
                        Request::Home(_) => Reply::Home {
                            source,
                            result: provider.home().await.map_err(|e| e.to_string()),
                        },
                        Request::Radio(_) => Reply::Radio {
                            source,
                            result: provider.radio().await.map_err(|e| e.to_string()),
                        },
                        _ => Reply::NewMusic {
                            source,
                            result: provider.new_music().await.map_err(|e| e.to_string()),
                        },
                    };
                    replies.send(reply);
                });
                continue;
            }
            Request::SetPlaylistCover { id, jpeg } => {
                let refused = |reason: &str| CoverRefused::Failed(reason.to_owned());
                match (id.provider, nd.clone()) {
                    (ProviderKind::Navidrome, Some(server)) => {
                        let (guard, since) =
                            (sign_outs.clone(), sign_outs.count(ProviderKind::Navidrome));
                        tokio::spawn(async move {
                            let again = jpeg.clone();
                            let result = match server.set_playlist_cover(&id.id, jpeg).await {
                                Ok(true) => {
                                    guard.unless_signed_out_since(
                                        ProviderKind::Navidrome,
                                        since,
                                        || keep_navidrome(&server.credentials()),
                                    );
                                    Ok(())
                                }
                                Ok(false) => Err(CoverRefused::Password(again)),
                                Err(e) => Err(CoverRefused::Failed(e.to_string())),
                            };
                            replies.send(Reply::CoverChanged { id, result });
                        });
                    }
                    (ProviderKind::Navidrome, None) => replies.send(Reply::CoverChanged {
                        id,
                        result: Err(refused("Navidrome isn't connected")),
                    }),
                    _ => replies.send(Reply::CoverChanged {
                        id,
                        result: Err(refused("This service's playlists keep their own covers")),
                    }),
                }
                continue;
            }
            Request::NavidromeSession(password) => {
                let Some(server) = nd.clone() else {
                    replies.send(Reply::NavidromeSession(Err(
                        "Navidrome isn't connected".into()
                    )));
                    continue;
                };
                let (guard, since) = (sign_outs.clone(), sign_outs.count(ProviderKind::Navidrome));
                tokio::spawn(async move {
                    let result = server
                        .open_session(&password)
                        .await
                        .map_err(|e| e.to_string());
                    drop(password);
                    if result == Ok(true) {
                        guard.unless_signed_out_since(ProviderKind::Navidrome, since, || {
                            keep_navidrome(&server.credentials())
                        });
                    }
                    replies.send(Reply::NavidromeSession(result));
                });
                continue;
            }
            Request::Search(scope, source, query) => {
                // Those still connecting are searched once they come, the
                // whole search asked again then.
                let ready = |provider| !connecting.contains(&provider);
                let services = Services {
                    apple: apple.clone().filter(|_| ready(ProviderKind::AppleMusic)),
                    soundcloud: sc.clone().filter(|_| ready(ProviderKind::SoundCloud)),
                    navidrome: nd.clone().filter(|_| ready(ProviderKind::Navidrome)),
                    local: local.clone(),
                };
                tokio::spawn(search(services, scope, source, query, replies));
                continue;
            }
            _ => {}
        }
        // What is the server's goes there.
        // A file plays from the disk, whether or not its folder is chosen.
        if target(&request) == Some(ProviderKind::Local) {
            local_request(local.clone(), request, replies, &mut warming);
            continue;
        }
        if target(&request) == Some(ProviderKind::Navidrome) {
            match nd.clone() {
                Some(nd) => navidrome_request(nd, request, replies, &mut warming),
                None => unanswered(request, "Navidrome", &replies),
            }
            continue;
        }
        // What is SoundCloud's goes there.
        if target(&request) == Some(ProviderKind::SoundCloud) {
            match sc.clone() {
                Some(sc) => {
                    soundcloud_request(sc, request, replies, &mut warming, cdm.clone().ok());
                }
                None => unanswered(request, "SoundCloud", &replies),
            }
            continue;
        }
        // Without a connection there is nothing to ask; what waits for an
        // answer still hears why.
        let Some(am) = apple.clone() else {
            unanswered(request, "Apple Music", &replies);
            continue;
        };
        let Some(request) = shared_request(am.clone(), request, replies.clone()) else {
            continue;
        };
        match request {
            // Any service's, done above.
            Request::SetFavourite { .. }
            | Request::AddToLibrary { .. }
            | Request::SongLinks { .. }
            | Request::RemoveFromLibrary(_)
            | Request::CreatePlaylist { .. }
            | Request::CreateOpiumPlaylist { .. }
            | Request::AddToOpiumPlaylist { .. }
            | Request::EditPlaylist { .. }
            | Request::RemoveFromPlaylist { .. }
            | Request::DeletePlaylist(_)
            | Request::SetPlaylistCover { .. }
            | Request::NavidromeSession(_)
            | Request::Home(_)
            | Request::Radio(_)
            | Request::NewMusic(_)
            | Request::AddToPlaylists { .. }
            | Request::Favourite { .. }
            | Request::SuggestLess { .. }
            | Request::SuggestsLess { .. }
            | Request::ShareLink { .. }
            | Request::StationTracks(_) => {}
            Request::SignIn
            | Request::Reconnect
            | Request::SetLocalFolders(_)
            | Request::RescanLocal
            | Request::OpenFiles(_)
            | Request::WriteTags { .. }
            | Request::SignOut(_)
            | Request::SignInSoundCloud
            | Request::ReconnectSoundCloud
            | Request::ReconnectNavidrome
            | Request::SignInNavidrome { .. }
            | Request::SetService { .. }
            | Request::SiteLyrics { .. }
            | Request::Library
            | Request::NewestSongs
            | Request::LibraryAlbums
            | Request::LibraryArtists
            | Request::RecentlyAdded
            | Request::Playlists
            | Request::Palette(_)
            | Request::Download { .. }
            | Request::RemoveDownloads(_)
            | Request::Search(..) => {}
            Request::Album(id) => {
                tokio::spawn(async move {
                    let result = tokio::try_join!(am.album(&id), am.album_tracks(&id))
                        .map(Box::new)
                        .map_err(|e| e.to_string());
                    keep_album(&id, &result);
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
            Request::Lyrics(id) => {
                tokio::spawn(async move {
                    let result = am.lyrics(&id).await.map_err(|e| e.to_string());
                    replies.send(Reply::Lyrics { id, result });
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
                spatial,
                ..
            } => {
                let cdm = cdm.clone();
                tokio::spawn(async move {
                    let result = playable(&am, cdm, &track, fallback.as_ref(), spatial).await;
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
                spatial,
                ..
            } => {
                if let Some(last) = warming.take() {
                    last.abort();
                }
                let cdm = cdm.clone();
                warming = Some(tokio::spawn(async move {
                    let result = playable(&am, cdm, &track, fallback.as_ref(), spatial).await;
                    replies.send(Reply::Fetched { token, result });
                }));
            }
        }
    }
}

/// Whether `provider`'s sign-in is kept on the disk: out of reach, its
/// library shows as it was kept.
fn signed_in(provider: ProviderKind) -> bool {
    match provider {
        ProviderKind::AppleMusic => apple_music_dir().join("media-user-token").exists(),
        ProviderKind::SoundCloud => soundcloud_dir().join("tokens.json").exists(),
        ProviderKind::Navidrome => navidrome_file().exists(),
        ProviderKind::Local | ProviderKind::Opium => false,
    }
}

/// A service out of reach: its songs and playlists as they were kept, so
/// that the library shows offline (its downloads play).
fn show_kept(provider: ProviderKind, replies: Replies) {
    let (songs, playlists) = match provider {
        ProviderKind::AppleMusic => ("songs", "playlists"),
        ProviderKind::SoundCloud => ("soundcloud-songs", "soundcloud-playlists"),
        ProviderKind::Navidrome => ("navidrome-songs", "navidrome-playlists"),
        ProviderKind::Local | ProviderKind::Opium => return,
    };
    tokio::task::spawn_blocking(move || {
        if let Some(playlists) = cache::read::<Vec<Playlist>>(playlists) {
            replies.send(Reply::Playlists {
                provider,
                playlists,
            });
        }
        if let Some(tracks) = cache::read::<Vec<Track>>(songs) {
            replies.send(Reply::Songs {
                provider,
                tracks,
                done: true,
                whole: true,
            });
        }
    });
}

/// The list kept under `name`, if `kept` says to show it; else none.
fn kept_list<T: DeserializeOwned>(kept: bool, name: &str) -> Vec<T> {
    kept.then(|| cache::read(name))
        .flatten()
        .unwrap_or_default()
}

/// `provider`'s part of the albums, artists or recent additions `request`
/// asks for, while it is out of reach, done: Apple Music's albums and
/// artists as kept when `apple_kept` (signed in and on), in place of those
/// it showed; another's, none more.
fn out_of_reach_list(provider: ProviderKind, request: &Request, apple_kept: bool) -> Reply {
    let apple = provider == ProviderKind::AppleMusic;
    match request {
        Request::LibraryAlbums => Reply::LibraryAlbums {
            provider,
            albums: kept_list(apple && apple_kept, "albums"),
            done: true,
            whole: apple,
        },
        Request::LibraryArtists => Reply::LibraryArtists {
            provider,
            artists: kept_list(apple && apple_kept, "artists"),
            done: true,
            whole: apple,
        },
        _ => Reply::RecentlyAdded {
            provider,
            items: Vec::new(),
            done: true,
        },
    }
}

/// Where an album's page is kept, to open offline.
fn album_kept(id: &ItemId) -> String {
    format!("album {:?} {}", id.provider, id.id)
}

/// Keep an album's page as it was read.
fn keep_album(id: &ItemId, result: &Result<Box<(Album, Vec<Track>)>, String>) {
    if let Ok(page) = result {
        let (name, page) = (album_kept(id), page.clone());
        tokio::task::spawn_blocking(move || cache::write(&name, &*page));
    }
}

/// Ask Apple whether the subscription is active, which keeps its
/// downloaded songs playing; nothing changes when it doesn't say. The answer
/// goes back to the backend's loop with `sign_outs`, the sign-outs of Apple
/// counted when it was asked, so that the loop drops one asked before a
/// sign-out: it would keep the songs playing 30 days for an account gone.
fn confirm_subscription(
    am: Arc<AppleMusic>,
    answers: UnboundedSender<(bool, u64)>,
    sign_outs: u64,
) {
    tokio::spawn(async move {
        if let Ok(Some(active)) = am.subscribed().await {
            let _ = answers.send((active, sign_outs));
        }
    });
}

/// How many songs a station of songs is asked for at a time.
const STATION_BATCH: usize = 20;

/// How far back Recently Added goes.
const RECENT_LIMIT: usize = 200;

/// A track or a station, ready to play: stations (`ra.`) stream, everything
/// else is a track, as Spatial Audio with `spatial` when mixed in Dolby
/// Atmos. A catalog song Apple no longer offers here is played from the
/// library's copy, when there is one, as Apple Music does.
async fn playable(
    am: &AppleMusic,
    cdm: Result<widevine::Cdm, String>,
    track: &ItemId,
    fallback: Option<&ItemId>,
    spatial: bool,
) -> Result<Playable, String> {
    let first = playable_as(am, cdm.clone(), track, spatial).await;
    match (first, fallback) {
        (Err(_), Some(copy)) if copy != track => playable_as(am, cdm, copy, spatial).await,
        (result, _) => result,
    }
}

async fn playable_as(
    am: &AppleMusic,
    cdm: Result<widevine::Cdm, String>,
    track: &ItemId,
    spatial: bool,
) -> Result<Playable, String> {
    match cdm {
        Ok(cdm) if track.id.starts_with("ra.") => am
            .live_station(&cdm, track)
            .await
            .map(Playable::Live)
            .map_err(|e| e.to_string()),
        Ok(cdm) => am
            .fetch_track(&cdm, track, spatial)
            .await
            .map(|(stream, kbps, spatial)| Playable::Track {
                stream,
                kbps,
                spatial,
            })
            .map_err(|e| e.to_string()),
        Err(e) => Err(e),
    }
}

/// A list that failed part way: it ends with `done`, and the failure shows
/// as a toast.
fn failed(replies: &Replies, result: opium_core::Result<()>, done: impl FnOnce() -> Reply) {
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

/// Apple Music's albums or artists, as kept then page by page, or its
/// recent additions.
async fn apple_lists(am: Arc<AppleMusic>, request: Request, replies: Replies) {
    let provider = ProviderKind::AppleMusic;
    match request {
        Request::LibraryAlbums => {
            let show = |albums, done, whole| {
                replies.send(Reply::LibraryAlbums {
                    provider,
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
        }
        Request::LibraryArtists => {
            let show = |artists, done, whole| {
                replies.send(Reply::LibraryArtists {
                    provider,
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
        }
        _ => {
            // Whole, in the order of what was added to last.
            let result = am
                .recently_added_by_activity(RECENT_LIMIT)
                .await
                .map(|items| {
                    replies.send(Reply::RecentlyAdded {
                        provider,
                        items,
                        done: true,
                    })
                });
            failed(&replies, result, || Reply::RecentlyAdded {
                provider,
                items: Vec::new(),
                done: true,
            });
        }
    }
}

/// What any service does alike: favourites, the library, playlists, a
/// song's links and stations. Hands back the requests it isn't for.
fn shared_request(p: Arc<dyn Provider>, request: Request, replies: Replies) -> Option<Request> {
    match request {
        Request::SetFavourite { kind, id, on } => {
            tokio::spawn(async move {
                let result = p
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
                let result = p
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
                let result = p.song_links(&song).await.map_err(|e| e.to_string());
                replies.send(Reply::SongLinks { radio, result });
            });
        }
        Request::RemoveFromLibrary(id) => {
            tokio::spawn(async move {
                let result = p
                    .remove_from_library(&id)
                    .await
                    .map(|()| "Removed from the library".to_owned())
                    .map_err(|e| format!("Couldn't remove it from the library: {e}"));
                replies.send(Reply::RemovedFromLibrary(result));
            });
        }
        Request::CreatePlaylist {
            name,
            description,
            tracks,
            ..
        } => {
            tokio::spawn(async move {
                let result = p
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
                let result = p
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
                let result = p
                    .remove_from_playlist(&playlist, &entry)
                    .await
                    .map_err(|e| format!("Couldn't take it out of the playlist: {e}"));
                replies.send(Reply::RemovedFromPlaylist { playlist, result });
            });
        }
        Request::DeletePlaylist(id) => {
            tokio::spawn(async move {
                let result = p
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
                    if let Err(e) = p.add_to_playlist(playlist, &tracks).await {
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
                let result = p.is_favourite(kind, &id).await.map_err(|e| e.to_string());
                replies.send(Reply::Favourite {
                    id,
                    set: None,
                    result,
                });
            });
        }
        Request::SuggestLess { kind, id, on } => {
            tokio::spawn(async move {
                let result = p
                    .suggest_less(kind, &id, on)
                    .await
                    .map(|()| Some(on))
                    .map_err(|e| e.to_string());
                replies.send(Reply::SuggestsLess {
                    id,
                    set: Some(on),
                    result,
                });
            });
        }
        Request::SuggestsLess { kind, id } => {
            tokio::spawn(async move {
                let result = p.suggests_less(kind, &id).await.map_err(|e| e.to_string());
                replies.send(Reply::SuggestsLess {
                    id,
                    set: None,
                    result,
                });
            });
        }
        Request::ShareLink { kind, id } => {
            tokio::spawn(async move {
                let result = p.share_link(kind, &id).await.map_err(|e| e.to_string());
                replies.send(Reply::ShareLink(result));
            });
        }
        Request::StationTracks(id) => {
            tokio::spawn(async move {
                let result = p
                    .station_tracks(&id, STATION_BATCH)
                    .await
                    .map_err(|e| e.to_string());
                replies.send(Reply::StationTracks { id, result });
            });
        }
        other => return Some(other),
    }
    None
}

/// Each service's library as it loads: one load at a time, the newer one
/// (after a sign-in, or turned back on) calling off the last, so that two
/// never add the same songs.
#[derive(Default)]
struct Loading(std::collections::HashMap<ProviderKind, tokio::task::AbortHandle>);

impl Loading {
    fn start(
        &mut self,
        provider: ProviderKind,
        load: impl std::future::Future<Output = ()> + Send + 'static,
    ) {
        self.stop(provider);
        self.0.insert(provider, tokio::spawn(load).abort_handle());
    }

    fn stop(&mut self, provider: ProviderKind) {
        if let Some(load) = self.0.remove(&provider) {
            load.abort();
        }
    }
}

/// Tell what waits on `request` that `service` isn't there to answer it, so
/// that it shows why rather than load for ever.
fn unanswered(request: Request, service: &str, replies: &Replies) {
    let e = format!("{service} isn't connected");
    let reply = match request {
        Request::Fetch { token, .. } | Request::Warm { token, .. } => Reply::Fetched {
            token,
            result: Err(e),
        },
        // An album or a playlist opened before shows as it was.
        Request::Album(id) => {
            let result = cache::read::<(Album, Vec<Track>)>(&album_kept(&id))
                .map(Box::new)
                .ok_or(e);
            Reply::Album { id, result }
        }
        Request::Artist(id) => Reply::Artist { id, result: Err(e) },
        Request::Playlist(id) => Reply::PlaylistTracks {
            result: cache::read::<Vec<Track>>(&format!("playlist {}", id.id)).ok_or(e),
            id,
            done: true,
            whole: true,
        },
        Request::Lyrics(id) => Reply::Lyrics { id, result: Err(e) },
        Request::AlbumExtras(id) => Reply::AlbumExtras { id, result: Err(e) },
        Request::LibraryArtistAlbums(id) => Reply::LibraryArtistAlbums { id, result: Err(e) },
        Request::StationTracks(id) => Reply::StationTracks { id, result: Err(e) },
        Request::Home(source) => Reply::Home {
            source,
            result: Err(e),
        },
        Request::Radio(source) => Reply::Radio {
            source,
            result: Err(e),
        },
        Request::NewMusic(source) => Reply::NewMusic {
            source,
            result: Err(e),
        },
        // A change shown at once is undone, and the user told why.
        Request::SetFavourite { id, on, .. } => Reply::Favourite {
            id,
            set: Some(on),
            result: Err(e),
        },
        Request::AddToLibrary { kind, id, quiet } => Reply::AddedToLibrary {
            kind,
            id,
            quiet,
            result: Err(e),
        },
        Request::RemoveFromLibrary(_) => {
            Reply::RemovedFromLibrary(Err(format!("Couldn't remove it from the library: {e}")))
        }
        Request::SuggestLess { id, on, .. } => Reply::SuggestsLess {
            id,
            set: Some(on),
            result: Err(e),
        },
        Request::ShareLink { .. } => Reply::ShareLink(Err(e)),
        Request::AddToPlaylists { .. } => {
            Reply::Changed(Err(format!("Couldn't add to the playlist: {e}")))
        }
        Request::CreatePlaylist { .. } => Reply::PlaylistCreated(Err(e)),
        Request::EditPlaylist {
            id,
            name,
            description,
        } => Reply::PlaylistEdited {
            id,
            name,
            description,
            result: Err(e),
        },
        Request::DeletePlaylist(id) => Reply::PlaylistDeleted { id, result: Err(e) },
        Request::RemoveFromPlaylist { playlist, .. } => Reply::RemovedFromPlaylist {
            playlist,
            result: Err(e),
        },
        Request::SongLinks { radio, .. } => Reply::SongLinks {
            radio,
            result: Err(e),
        },
        _ => return,
    };
    replies.send(reply);
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
        | Request::SuggestLess { id, .. }
        | Request::SuggestsLess { id, .. }
        | Request::ShareLink { id, .. }
        | Request::EditPlaylist { id, .. }
        | Request::RemoveFromPlaylist { playlist: id, .. } => id,
        Request::InLibrary { ids, .. } => ids.first()?,
        Request::AddToPlaylists { playlists, .. } => playlists.first()?,
        Request::CreatePlaylist { service, .. } => return Some(*service),
        _ => return None,
    };
    Some(id.provider)
}

/// What a service other than Apple Music shows alike: its albums, artists,
/// playlists and lyrics. Hands back the requests it isn't for.
fn item_request(p: Arc<dyn Provider>, request: Request, replies: Replies) -> Option<Request> {
    match request {
        Request::Album(id) => {
            tokio::spawn(async move {
                let result = tokio::try_join!(p.album(&id), p.album_tracks(&id))
                    .map(Box::new)
                    .map_err(|e| e.to_string());
                keep_album(&id, &result);
                replies.send(Reply::Album { id, result });
            });
        }
        Request::Artist(id) => {
            tokio::spawn(async move {
                let result = p.artist(&id).await.map(Arc::new).map_err(|e| e.to_string());
                replies.send(Reply::Artist { id, result });
            });
        }
        // An artist of the library: all their releases there.
        Request::LibraryArtistAlbums(id) => {
            tokio::spawn(async move {
                let result = p
                    .artist(&id)
                    .await
                    .map(|a| [a.albums, a.singles, a.compilations].concat())
                    .map_err(|e| e.to_string());
                replies.send(Reply::LibraryArtistAlbums { id, result });
            });
        }
        // These services' playlists come whole.
        Request::Playlist(id) => {
            tokio::spawn(async move {
                let result = p
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
        Request::Lyrics(id) => {
            tokio::spawn(async move {
                let result = p.lyrics(&id).await.map_err(|e| e.to_string());
                replies.send(Reply::Lyrics { id, result });
            });
        }
        // Their covers aren't asked about, and they have no other editions
        // or videos to show.
        Request::InLibrary { .. } => {}
        Request::AlbumExtras(id) => replies.send(Reply::AlbumExtras {
            id,
            result: Ok(Arc::default()),
        }),
        other => return Some(other),
    }
    None
}

/// Fetch a track with `fetch`; as a warm-up, calling off the last one.
fn fetch_with<F>(
    request: Request,
    replies: Replies,
    warming: &mut Option<tokio::task::JoinHandle<()>>,
    fetch: impl FnOnce(ItemId, bool) -> F,
) -> Option<Request>
where
    F: std::future::Future<Output = Result<Playable, String>> + Send + 'static,
{
    match request {
        Request::Fetch {
            token,
            track,
            original,
            ..
        } => {
            let job = fetch(track, original);
            tokio::spawn(async move {
                replies.send(Reply::Fetched {
                    token,
                    result: job.await,
                });
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
            let job = fetch(track, original);
            *warming = Some(tokio::spawn(async move {
                replies.send(Reply::Fetched {
                    token,
                    result: job.await,
                });
            }));
        }
        other => return Some(other),
    }
    None
}

/// A request about a SoundCloud item.
fn soundcloud_request(
    sc: Arc<SoundCloud>,
    request: Request,
    replies: Replies,
    warming: &mut Option<tokio::task::JoinHandle<()>>,
    cdm: Option<widevine::Cdm>,
) {
    let Some(request) = shared_request(sc.clone(), request, replies.clone()) else {
        return;
    };
    let Some(request) = item_request(sc.clone(), request, replies.clone()) else {
        return;
    };
    let fetch = |track: ItemId, original: bool| async move {
        soundcloud_playable(&sc, &track, original, cdm.as_ref()).await
    };
    if fetch_with(request, replies.clone(), warming, fetch).is_some() {
        replies.send(Reply::Changed(Err(
            "That isn't something SoundCloud does".into()
        )));
    }
}

/// A request about a Navidrome item.
fn navidrome_request(
    nd: Arc<Navidrome>,
    request: Request,
    replies: Replies,
    warming: &mut Option<tokio::task::JoinHandle<()>>,
) {
    let Some(request) = shared_request(nd.clone(), request, replies.clone()) else {
        return;
    };
    let Some(request) = item_request(nd.clone(), request, replies.clone()) else {
        return;
    };
    let fetch = |track: ItemId, _original: bool| async move {
        nd.audio(&track)
            .await
            .map(Playable::Navidrome)
            .map_err(|e| e.to_string())
    };
    if fetch_with(request, replies.clone(), warming, fetch).is_some() {
        replies.send(Reply::Changed(Err(
            "That isn't something Navidrome does".into()
        )));
    }
}

/// A SoundCloud track's audio; `cdm` decrypts a major label's.
async fn soundcloud_playable(
    sc: &SoundCloud,
    track: &ItemId,
    original: bool,
    cdm: Option<&widevine::Cdm>,
) -> Result<Playable, String> {
    sc.audio(track, original, cdm)
        .await
        .map(Playable::SoundCloud)
        .map_err(|e| e.to_string())
}

/// What the user liked last on SoundCloud, for Recently Added.
async fn soundcloud_recent(sc: Arc<SoundCloud>, replies: Replies) {
    let provider = ProviderKind::SoundCloud;
    let items = match sc.recently_added(None).await {
        Ok(page) => page.items,
        Err(e) => {
            replies.send(Reply::Failed(e.to_string()));
            Vec::new()
        }
    };
    replies.send(Reply::RecentlyAdded {
        provider,
        items,
        done: true,
    });
}

/// Every SoundCloud playlist of the user's: their own, then those they liked.
async fn soundcloud_playlists(sc: &SoundCloud) -> opium_core::Result<Vec<Playlist>> {
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

/// The server's side of the library: its playlists, what the user starred,
/// then every song, kept as Apple Music's are.
async fn load_navidrome_library(nd: Arc<Navidrome>, replies: Replies) {
    let provider = ProviderKind::Navidrome;
    if let Some(playlists) = cache::read::<Vec<Playlist>>("navidrome-playlists") {
        replies.send(Reply::Playlists {
            provider,
            playlists,
        });
    }
    match nd.library_playlists(None).await {
        Ok(page) => {
            let kept = page.items.clone();
            tokio::task::spawn_blocking(move || cache::write("navidrome-playlists", &kept));
            replies.send(Reply::Playlists {
                provider,
                playlists: page.items,
            });
        }
        Err(e) => replies.send(Reply::Failed(e.to_string())),
    }
    if let Ok(starred) = nd.starred().await {
        replies.send(Reply::Starred(starred));
    }
    let show = |tracks, done, whole| {
        replies.send(Reply::Songs {
            provider,
            tracks,
            done,
            whole,
        })
    };
    let mut list = Kept::start("navidrome-songs", &show);
    let mut cursor: Option<Cursor> = None;
    let result = loop {
        match nd.library_tracks(cursor.as_ref()).await {
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

/// The server's albums, artists or recent additions, page by page.
async fn navidrome_lists(nd: Arc<Navidrome>, request: Request, replies: Replies) {
    let provider = ProviderKind::Navidrome;
    match request {
        Request::LibraryAlbums => {
            let mut cursor: Option<Cursor> = None;
            let mut first = true;
            loop {
                match nd.library_albums(cursor.as_ref()).await {
                    Ok(page) => {
                        let done = page.next.is_none();
                        replies.send(Reply::LibraryAlbums {
                            provider,
                            albums: page.items,
                            done,
                            whole: first,
                        });
                        first = false;
                        match page.next {
                            Some(next) => cursor = Some(next),
                            None => return,
                        }
                    }
                    Err(e) => return replies.send(Reply::Failed(e.to_string())),
                }
            }
        }
        Request::LibraryArtists => match nd.library_artists(None).await {
            Ok(page) => replies.send(Reply::LibraryArtists {
                provider,
                artists: page.items,
                done: true,
                whole: true,
            }),
            Err(e) => replies.send(Reply::Failed(e.to_string())),
        },
        Request::RecentlyAdded => match nd.recently_added(None).await {
            Ok(page) => replies.send(Reply::RecentlyAdded {
                provider,
                items: page.items,
                done: true,
            }),
            Err(e) => replies.send(Reply::Failed(e.to_string())),
        },
        _ => {}
    }
}

/// SoundCloud's side of the library: the playlists, then the liked tracks,
/// kept as Apple Music's are.
/// Where a service's songs, the last added first, are kept between launches.
fn newest_kept(provider: ProviderKind) -> String {
    let name = match provider {
        ProviderKind::AppleMusic => "apple",
        ProviderKind::SoundCloud => "soundcloud",
        ProviderKind::Local => "local",
        _ => "navidrome",
    };
    format!("{name}-newest")
}

/// A service's songs, the last added first, as it has them now: kept, and shown.
fn newest_came(provider: ProviderKind, ids: Vec<ItemId>, replies: &Replies) {
    let kept = ids.clone();
    tokio::task::spawn_blocking(move || cache::write(&newest_kept(provider), &kept));
    replies.send(Reply::NewestSongs { provider, ids });
}

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

/// A search of every service (see `services::Services::search`).
async fn search(
    services: Services,
    scope: SearchScope,
    source: Option<ProviderKind>,
    query: String,
    replies: Replies,
) {
    let within = match scope {
        SearchScope::AppleMusic => services::Scope::Catalog,
        SearchScope::Library => services::Scope::Library,
    };
    let result = services.search(within, &query, source).await;
    replies.send(Reply::Search {
        scope,
        source,
        query,
        result,
    });
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
    /// What was kept had something in it.
    had: bool,
    all: Vec<T>,
}

impl<T: Clone + Serialize + DeserializeOwned + Send + 'static> Kept<T> {
    /// Show what was kept under `name` with `show(items, done, whole)`.
    fn start(name: &str, show: &impl Fn(Vec<T>, bool, bool)) -> Self {
        let kept = cache::read::<Vec<T>>(name);
        let shown = kept.is_some();
        let had = kept.as_ref().is_some_and(|items| !items.is_empty());
        if let Some(items) = kept {
            show(items, true, true);
        }
        Self {
            name: Some(name.to_owned()),
            shown,
            had,
            all: Vec::new(),
        }
    }

    /// A list not kept: it shows page by page.
    fn none() -> Self {
        Self {
            name: None,
            shown: false,
            had: false,
            all: Vec::new(),
        }
    }

    fn page(&mut self, items: Vec<T>, done: bool, show: &impl Fn(Vec<T>, bool, bool)) {
        if self.shown {
            self.all.extend(items);
            if done {
                // A service that answers with nothing at all, where it had
                // a library last time, has far more likely failed in a way
                // that reads as empty — a 404, a shape it no longer sends —
                // than been emptied. What was kept stays shown, and stays
                // on disk, rather than the library going for good.
                if self.all.is_empty() && self.had {
                    if let Some(name) = self.name.take() {
                        eprintln!("{name}: came back empty, so what was kept stays");
                    }
                    return;
                }
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
            // Nothing is kept as a whole library: read again next time.
            if !all.is_empty() {
                tokio::task::spawn_blocking(move || cache::write(&name, &all));
            }
        }
    }

    /// A failure part way: the kept list stays shown, or the pages so far
    /// end there. Either way it shows as a toast.
    fn failed(
        &self,
        replies: &Replies,
        result: opium_core::Result<()>,
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
