//! Apple Music through the web player's private API (`amp-api.music.apple.com`).
//!
//! Neither token comes from an official developer account: the developer token is
//! read from the web player ([`auth`]) and the user token from a browser sign-in
//! ([`signin`]). Request shapes follow Kopuz (<https://github.com/Kopuz-org/kopuz>).

pub mod auth;
mod convert;
mod lyrics;
mod motion;
mod playback;
mod radio;
mod resume;
mod seen;
pub use seen::SeenElsewhere;
pub mod signin;
mod types;

pub use cenc::AudioStream;
pub use playback::{ASSET_HOST, PRIMING, Protected};
pub use radio::{LiveAsset, LiveStream, name_the_key};

use async_trait::async_trait;
use music_core::model::{
    Album, AlbumExtras, Artist, ArtistDetail, Artwork, Collection, Cursor, FavouriteKind, ItemId,
    Lyrics, Page, Pick, Playlist, ProviderKind, SearchResults, Shelf, SongLinks, Track,
};
use music_core::{Error, Provider, Result};
use serde::de::DeserializeOwned;

const API: &str = "https://amp-api.music.apple.com";
const WEB_PLAYER: &str = "https://music.apple.com";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
/// The largest page the library and catalog endpoints accept.
const PAGE: u32 = 100;
/// The longest wait Apple may ask for before a request is tried again.
const MAX_RETRY_AFTER: u64 = 10;
/// An editorial grouping's first tab (the subscriber's) as shelves: the big
/// row first (element kind 316: the stations on air, the week's picks, each
/// with its badge), then every titled shelf of covers (326) or of songs (327)
/// holding anything the app can play or open. Items that only link to
/// another page are left out.
fn editorial_shelves(page: &serde_json::Value) -> Vec<Shelf> {
    let items = |element: &serde_json::Value| -> Vec<serde_json::Value> {
        element["relationships"]["contents"]["data"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    };
    let contents = |element: &serde_json::Value| -> Vec<Collection> {
        items(element)
            .into_iter()
            .filter_map(|item| serde_json::from_value(item).ok())
            .filter_map(convert::collection)
            .collect()
    };
    let children = |element: &serde_json::Value| -> Vec<serde_json::Value> {
        element["relationships"]["children"]["data"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    };
    let name = |element: &serde_json::Value| -> String {
        element["attributes"]["name"]
            .as_str()
            .unwrap_or("")
            .to_owned()
    };
    let tabs = page["data"][0]["relationships"]["tabs"]["data"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let Some(tab) = tabs.first() else {
        return Vec::new();
    };
    let mut shelves = Vec::new();
    for element in children(tab) {
        let kind = element["attributes"]["editorialElementKind"]
            .as_str()
            .unwrap_or("");
        match kind {
            "316" => {
                let mut shelf = Shelf {
                    hero: true,
                    ..Shelf::default()
                };
                for child in children(&element) {
                    let Some(item) = contents(&child).into_iter().next() else {
                        continue;
                    };
                    // The item's own editorial picture and line, as Cider shows them.
                    let attributes = &items(&child)[0]["attributes"];
                    let wide = attributes["editorialArtwork"]["subscriptionHero"]["url"]
                        .as_str()
                        .map(|url| Artwork {
                            // Wide, at the shape Cider shows it; Apple crops to it.
                            url_template: url.replace("{w}x{h}", "1280x720"),
                            background: None,
                        });
                    shelf.items.push(item);
                    shelf.picks.push(Pick {
                        badge: child["attributes"]["designBadge"]
                            .as_str()
                            .unwrap_or("")
                            .to_owned(),
                        blurb: attributes["editorialNotes"]["short"]
                            .as_str()
                            .map(str::to_owned),
                        wide,
                    });
                }
                if !shelf.items.is_empty() {
                    shelves.push(shelf);
                }
            }
            "326" => {
                let (title, items) = (name(&element), contents(&element));
                if !items.is_empty() && !title.is_empty() {
                    shelves.push(Shelf {
                        title,
                        items,
                        ..Shelf::default()
                    });
                }
            }
            "327" => {
                let songs: Vec<Track> = items(&element)
                    .into_iter()
                    .filter_map(|item| serde_json::from_value::<types::Song>(item).ok())
                    .map(convert::track)
                    .collect();
                let title = name(&element);
                if !songs.is_empty() && !title.is_empty() {
                    shelves.push(Shelf {
                        title,
                        songs,
                        ..Shelf::default()
                    });
                }
            }
            _ => {}
        }
    }
    shelves
}

/// The catalog's name for a kind of item.
fn catalog_type(kind: FavouriteKind) -> &'static str {
    match kind {
        FavouriteKind::Song => "songs",
        FavouriteKind::Album => "albums",
        FavouriteKind::Artist => "artists",
        FavouriteKind::Playlist => "playlists",
    }
}

/// Album details in English, as the app's text is: Apple writes most editorial
/// notes in English only, and names genres in the storefront's language otherwise.
const NOTES_LANGUAGE: &str = "en-US";
/// The editorial pages (New, Radio) in English: they take the storefront's own
/// English, British for France, and fall back to its first language for
/// another one.
const EDITORIAL_LANGUAGE: &str = "en-GB";

/// The genre all of the catalogue's are under.
const MUSIC_GENRE: &str = "34";
/// Recently Added answers at most 25 at a time.
const RECENT_PAGE: u32 = 25;
/// How much of Recently Added the phone asks for: it shows 66.
const RECENT_SHOWN: usize = 100;
/// Search caps each result type at 25.
const SEARCH_LIMIT: u32 = 25;

pub struct AppleMusic {
    http: reqwest::Client,
    /// Borrowed from the web player, which swaps it now and then: read again
    /// when Apple turns it down.
    developer_token: std::sync::RwLock<String>,
    /// When it was last read again, so a run of refusals reads it once.
    renewed: tokio::sync::Mutex<Option<std::time::Instant>>,
    user_token: String,
    storefront: String,
    /// When the newest songs were first seen, once kept: see `seen`.
    seen: std::sync::Mutex<Option<seen::Seen>>,
    /// The library's songs' catalog ids checked: still there, or gone, with
    /// the song that took the place of each. See `released_now`.
    current: std::sync::Mutex<std::collections::HashMap<String, Option<Track>>>,
}

/// How long a developer token read again is trusted before it may be read
/// once more.
const RENEW_EVERY: std::time::Duration = std::time::Duration::from_secs(300);

/// The account as others see it on Apple Music.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub name: Option<String>,
    /// Without the `@`.
    pub handle: Option<String>,
    pub picture: Option<music_core::model::Artwork>,
}

impl AppleMusic {
    /// Fetch a developer token and look up the account's storefront (its country),
    /// which every catalog request is scoped to.
    pub async fn connect(user_token: impl Into<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            // A server out of reach, or a download that stalls, fails rather
            // than hang what waits on it.
            .connect_timeout(std::time::Duration::from_secs(10))
            .read_timeout(std::time::Duration::from_secs(30))
            // A song's start needs three of Apple's hosts: their connections
            // are kept a while, not dropped after 90 s.
            .pool_idle_timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| Error::Network(e.to_string()))?;
        let developer_token = auth::developer_token(&http).await?;
        let mut am = Self {
            http,
            developer_token: std::sync::RwLock::new(developer_token),
            renewed: tokio::sync::Mutex::new(None),
            user_token: user_token.into(),
            storefront: String::new(),
            seen: std::sync::Mutex::new(None),
            current: Default::default(),
        };
        let storefronts: types::Response<types::Resource<serde::de::IgnoredAny>> =
            am.get("/v1/me/storefront").await?;
        am.storefront = storefronts
            .data
            .into_iter()
            .next()
            .map(|s| s.id)
            .ok_or_else(|| Error::Protocol("the account has no storefront".into()))?;
        Ok(am)
    }

    /// Keep in `path` when each new library song was first seen, which
    /// dates the albums songs were added to in Recently Added; with
    /// `elsewhere`, the earliest the user's devices saw each.
    pub fn keep_seen_in(
        &self,
        path: std::path::PathBuf,
        elsewhere: Option<std::sync::Arc<dyn SeenElsewhere>>,
    ) {
        *self.seen.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(seen::Seen::open(path, elsewhere));
    }

    /// The account's profile on Apple Music, where it has made one.
    pub async fn profile(&self) -> Result<Option<Profile>> {
        #[derive(serde::Deserialize)]
        struct Attributes {
            name: Option<String>,
            handle: Option<String>,
            artwork: Option<types::Artwork>,
        }
        let profile: Option<types::Response<types::Resource<Attributes>>> =
            self.fetch("/v1/me/social-profile").await?;
        Ok(profile
            .and_then(|r| r.data.into_iter().next())
            .and_then(|p| p.attributes)
            .map(|p| Profile {
                picture: convert::artwork(p.artwork.as_ref()),
                name: p.name.filter(|n| !n.is_empty()),
                handle: p.handle.filter(|h| !h.is_empty()),
            }))
    }

    /// Whether the account's subscription is active, as Apple says it:
    /// what keeps downloaded songs playable. `None` when it doesn't say.
    pub async fn subscribed(&self) -> Result<Option<bool>> {
        let account: Option<serde_json::Value> =
            self.fetch("/v1/me/account?meta=subscription").await?;
        Ok(account.and_then(|a| a["meta"]["subscription"]["active"].as_bool()))
    }

    pub fn storefront(&self) -> &str {
        &self.storefront
    }

    fn developer_token(&self) -> String {
        self.developer_token
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// What every request to Apple carries: both tokens, presented the way the
    /// web player presents them.
    fn authorized(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        self.authorized_with(&self.developer_token(), req)
    }

    fn authorized_with(
        &self,
        token: &str,
        req: reqwest::RequestBuilder,
    ) -> reqwest::RequestBuilder {
        req.bearer_auth(token)
            .header("Origin", WEB_PLAYER)
            .header("Referer", format!("{WEB_PLAYER}/"))
            .header("Cookie", format!("media-user-token={}", self.user_token))
    }

    /// Send `request` with both tokens. Turned down with a 401, the developer
    /// token having expired or been swapped, it is read again from the web
    /// player and the request sent once more: a long session doesn't end up
    /// signed out for it.
    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        what: &str,
    ) -> Result<reqwest::Response> {
        let token = self.developer_token();
        let again = request.try_clone();
        let failed = |e: reqwest::Error| Error::Network(format!("{what}: {e}"));
        let resp = self
            .authorized_with(&token, request)
            .send()
            .await
            .map_err(failed)?;
        if resp.status().as_u16() == 401
            && let Some(again) = again
            && self.renew_developer_token(&token).await?
        {
            return self.authorized(again).send().await.map_err(failed);
        }
        Ok(resp)
    }

    /// Read the developer token again, unless another request just did, in
    /// place of `refused`; whether there is a new one to try. The web player
    /// out of reach is said as such, and tried again on the next refusal,
    /// rather than taken for a sign-out for a while.
    async fn renew_developer_token(&self, refused: &str) -> Result<bool> {
        let mut renewed = self.renewed.lock().await;
        if self.developer_token() != refused {
            return Ok(true);
        }
        if renewed.is_some_and(|at| at.elapsed() < RENEW_EVERY) {
            return Ok(false);
        }
        let token = match auth::developer_token(&self.http).await {
            Ok(token) => token,
            Err(e @ Error::Network(_)) => return Err(e),
            Err(_) => {
                *renewed = Some(std::time::Instant::now());
                return Ok(false);
            }
        };
        *renewed = Some(std::time::Instant::now());
        if token == refused {
            return Ok(false);
        }
        *self
            .developer_token
            .write()
            .unwrap_or_else(|e| e.into_inner()) = token;
        Ok(true)
    }

    /// `Ok(None)` on 404, which Apple also answers for some empty collections.
    async fn fetch<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>> {
        let mut waited = false;
        let resp = loop {
            let resp = self
                .send(
                    self.http
                        .get(format!("{API}{path}"))
                        .header("Media-User-Token", &self.user_token),
                    path,
                )
                .await?;
            // Asked to slow down: once, after the wait it names if short.
            if resp.status().as_u16() != 429 || waited {
                break resp;
            }
            waited = true;
            let wait = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok())
                .unwrap_or(1);
            if wait > MAX_RETRY_AFTER {
                break resp;
            }
            tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
        };
        let status = resp.status();
        match status.as_u16() {
            401 | 403 => return Err(Error::NotSignedIn),
            404 => return Ok(None),
            _ if !status.is_success() => {
                return Err(Error::Protocol(format!("{path}: HTTP {status}")));
            }
            _ => {}
        }
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Network(format!("{path}: {e}")))?;
        serde_json::from_str(&body)
            .map(Some)
            .map_err(|e| Error::Protocol(format!("{path}: {e}")))
    }

    /// Run a search request; the catalog and the library answer alike, but
    /// for their artists, read by `artist`.
    async fn search_at(
        &self,
        path: &str,
        artist: fn(types::Artist) -> Artist,
    ) -> Result<SearchResults> {
        let Some(resp) = self.fetch::<types::SearchResponse>(path).await? else {
            return Ok(SearchResults::default());
        };
        let r = resp.results;
        fn all<A, T>(list: Option<types::Response<A>>, f: fn(A) -> T) -> Vec<T> {
            list.map(|l| l.data.into_iter().map(f).collect())
                .unwrap_or_default()
        }
        Ok(SearchResults {
            tracks: all(r.songs, convert::track),
            albums: all(r.albums, convert::album),
            artists: all(r.artists, artist),
            playlists: all(r.playlists, convert::playlist),
        })
    }

    /// Send a request that changes the account, and say whether Apple took it.
    async fn send_change(&self, req: reqwest::RequestBuilder, path: &str) -> Result<()> {
        let resp = self
            .send(req.header("Media-User-Token", &self.user_token), path)
            .await?;
        match resp.status().as_u16() {
            401 | 403 => Err(Error::NotSignedIn),
            code if (200..300).contains(&code) => Ok(()),
            _ => Err(Error::Protocol(format!("{path}: HTTP {}", resp.status()))),
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.fetch(path)
            .await?
            .ok_or_else(|| Error::Protocol(format!("{path}: not found")))
    }

    /// One page of a list endpoint. `first` is the first page's request; the cursor
    /// is Apple's `next`, completed with the parameters it leaves out.
    async fn page<A, T>(
        &self,
        first: &str,
        cursor: Option<&Cursor>,
        convert: fn(types::Resource<A>) -> T,
    ) -> Result<Page<T>>
    where
        A: DeserializeOwned,
    {
        let path = cursor.map_or(first, |c| c.0.as_str());
        let Some(resp) = self
            .fetch::<types::Response<types::Resource<A>>>(path)
            .await?
        else {
            return Ok(Page {
                items: vec![],
                next: None,
            });
        };
        Ok(Page {
            items: resp.data.into_iter().map(convert).collect(),
            next: resp
                .next
                .map(|next| Cursor(carry_query_forward(&next, first))),
        })
    }
}

fn library_songs_path() -> String {
    format!("/v1/me/library/songs?limit={PAGE}&relate=albums")
}

/// Apple sorts them so, though it doesn't say when each was added.
fn newest_songs_path() -> String {
    format!("{}&sort=-dateAdded", library_songs_path())
}

/// With the catalog's rating, for the E under explicit covers.
fn library_albums_path() -> String {
    format!("/v1/me/library/albums?limit={PAGE}&include=catalog&fields%5Balbums%5D=contentRating")
}

/// With the tags, which mark Apple's playlist of favourite songs.
fn library_playlists_path() -> String {
    format!("/v1/me/library/playlists?limit={PAGE}&extend%5Blibrary-playlists%5D=tags,inFavorites")
}

fn library_artists_path() -> String {
    format!("/v1/me/library/artists?limit={PAGE}&include=catalog")
}

fn recently_added_path() -> String {
    format!(
        "/v1/me/library/recently-added?limit={RECENT_PAGE}&include=catalog&fields%5Balbums%5D=contentRating"
    )
}

/// How many songs the catalog is asked about at once.
const CHECKED_AT_ONCE: usize = 300;

/// How many pages of a list are asked for at once: Apple answers 32 about as
/// fast as one, where one after another a 5 000-song playlist took a minute.
const PAGES_AT_ONCE: usize = 32;

impl AppleMusic {
    fn playlist_tracks_path(&self, playlist: &ItemId) -> String {
        if playlist.id.starts_with("p.") {
            format!(
                // `relate` gives library songs their album's id, but not a
                // playlist's: those need the albums themselves.
                "/v1/me/library/playlists/{}/tracks?limit={PAGE}&include=albums",
                playlist.id
            )
        } else {
            format!(
                "/v1/catalog/{}/playlists/{}/tracks?limit={PAGE}&include=albums",
                self.storefront, playlist.id
            )
        }
    }

    /// Every library song: `each` gets them in order, and whether that was
    /// the last page; all at once, each as released now (see
    /// `released_now`).
    pub async fn each_library_track(&self, each: impl FnMut(Vec<Track>, bool)) -> Result<()> {
        self.each_released_now(&library_songs_path(), each).await
    }

    /// The songs of `first` and the pages after, as released now, given all
    /// at once.
    async fn each_released_now(
        &self,
        first: &str,
        mut each: impl FnMut(Vec<Track>, bool),
    ) -> Result<()> {
        let mut all = Vec::new();
        self.each_page(first, convert::track, usize::MAX, |page, _| {
            all.extend(page)
        })
        .await?;
        self.released_now(&mut all).await;
        each(all, true);
        Ok(())
    }

    /// Songs saved long ago may name a release replaced since, gone from
    /// the catalog: Apple Music plays the one in its place, with its rating,
    /// its quality and its lyrics. So do these, keeping their place in the
    /// library (its id, its album there). Checked once each, a few hundred
    /// at a time; what couldn't be is left as it is, and tried again later.
    pub async fn released_now(&self, tracks: &mut [Track]) {
        let unknown: Vec<String> = {
            let known = self.current.lock().unwrap_or_else(|e| e.into_inner());
            let mut asked = std::collections::HashSet::new();
            tracks
                .iter()
                .filter(|t| t.library_id.is_some() && t.id.id.bytes().all(|b| b.is_ascii_digit()))
                .map(|t| t.id.id.clone())
                .filter(|id| !known.contains_key(id) && asked.insert(id.clone()))
                .collect()
        };
        if !unknown.is_empty() {
            let mut asking = Vec::new();
            for ids in unknown.chunks(CHECKED_AT_ONCE) {
                asking.push(self.check_released(ids));
            }
            let checked = futures_util::future::join_all(asking).await;
            self.current
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .extend(checked.into_iter().flatten());
        }
        let known = self.current.lock().unwrap_or_else(|e| e.into_inner());
        for t in tracks.iter_mut() {
            if let Some(Some(now)) = known.get(&t.id.id) {
                *t = Track {
                    library_id: t.library_id.take(),
                    in_library: t.in_library,
                    album_id: t.album_id.take().or_else(|| now.album_id.clone()),
                    ..now.clone()
                };
            }
        }
    }

    /// Which of `ids` are in the catalog still, and the songs in place of
    /// the others; none of those that couldn't be told.
    async fn check_released(&self, ids: &[String]) -> Vec<(String, Option<Track>)> {
        let path = format!(
            "/v1/catalog/{}/songs?ids={}&fields%5Bsongs%5D=name",
            self.storefront,
            ids.join(",")
        );
        let Ok(there) = self
            .fetch::<types::Response<types::Resource<serde::de::IgnoredAny>>>(&path)
            .await
        else {
            return Vec::new();
        };
        let there: std::collections::HashSet<String> = there
            .map(|r| r.data.into_iter().map(|s| s.id).collect())
            .unwrap_or_default();
        let mut checked = Vec::with_capacity(ids.len());
        for id in ids {
            if there.contains(id) {
                checked.push((id.clone(), None));
                continue;
            }
            let path = format!(
                "/v1/catalog/{}/songs?filter%5Bequivalents%5D={id}&include=albums",
                self.storefront
            );
            if let Ok(now) = self.fetch::<types::Response<types::Song>>(&path).await {
                let now = now
                    .and_then(|r| r.data.into_iter().find(|s| s.id != *id))
                    .map(convert::track);
                checked.push((id.clone(), now));
            }
        }
        checked
    }

    /// Every library song, the latest added first, as
    /// [`each_library_track`](Self::each_library_track).
    pub async fn each_newest_library_track(
        &self,
        each: impl FnMut(Vec<Track>, bool),
    ) -> Result<()> {
        self.each_released_now(&newest_songs_path(), each).await
    }

    /// Every song of a playlist, as [`each_library_track`](Self::each_library_track).
    pub async fn each_playlist_track(
        &self,
        playlist: &ItemId,
        each: impl FnMut(Vec<Track>, bool),
    ) -> Result<()> {
        let first = self.playlist_tracks_path(playlist);
        self.each_released_now(&first, each).await
    }

    pub async fn each_library_album(&self, each: impl FnMut(Vec<Album>, bool)) -> Result<()> {
        self.each_page(&library_albums_path(), convert::album, usize::MAX, each)
            .await
    }

    pub async fn each_library_artist(&self, each: impl FnMut(Vec<Artist>, bool)) -> Result<()> {
        self.each_page(
            &library_artists_path(),
            convert::library_artist,
            usize::MAX,
            each,
        )
        .await
    }

    /// The other editions of the catalog album `album`, and the music videos
    /// of its songs, as Apple's web player lists them under the songs.
    pub async fn album_extras(&self, album: &ItemId) -> Result<AlbumExtras> {
        let path = format!(
            "/v1/catalog/{}/albums/{}?views=other-versions,related-videos&limit[view.related-videos]=20",
            self.storefront, album.id
        );
        let Some(resp) = self
            .fetch::<types::Response<types::AlbumWithViews>>(&path)
            .await?
        else {
            return Ok(AlbumExtras::default());
        };
        let views = resp
            .data
            .into_iter()
            .next()
            .map(|a| a.views)
            .unwrap_or_default();
        Ok(AlbumExtras {
            other_versions: views
                .other_versions
                .map(|l| l.data.into_iter().map(convert::album).collect())
                .unwrap_or_default(),
            videos: views
                .related_videos
                .map(|l| {
                    l.data
                        .into_iter()
                        .filter_map(convert::music_video)
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    /// A library artist's albums in the library, with the catalog's rating
    /// for the E under explicit ones.
    pub async fn library_artist_albums(&self, artist: &ItemId) -> Result<Vec<Album>> {
        let first = format!(
            "/v1/me/library/artists/{}/albums?limit={PAGE}&include=catalog&fields%5Balbums%5D=contentRating",
            artist.id
        );
        let mut albums = Vec::new();
        self.each_page(&first, convert::album, usize::MAX, |page, _| {
            albums.extend(page)
        })
        .await?;
        Ok(albums)
    }

    pub async fn each_library_playlist(&self, each: impl FnMut(Vec<Playlist>, bool)) -> Result<()> {
        self.each_page(
            &library_playlists_path(),
            convert::playlist,
            usize::MAX,
            each,
        )
        .await
    }

    /// What was added last, as far back as `limit` items.
    /// Recently Added as Apple Music's app shows it: an album a song was just
    /// added to comes up, though the album joined the library long before,
    /// where Apple's list keeps it where it first came.
    pub async fn recently_added_by_activity(&self, limit: usize) -> Result<Vec<Collection>> {
        let mut listed = Vec::new();
        let (got, newest) = tokio::join!(
            self.each_recently_added(limit, |items, _| listed.extend(items)),
            self.library_tracks_newest(None),
        );
        got?;
        // Without the newest songs, the list as Apple keeps it.
        let newest = newest.map(|p| p.items).unwrap_or_default();
        let ids: Vec<String> = newest.iter().map(|t| t.id.id.clone()).collect();
        let times = match self.seen.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            Some(seen) => seen.when(&ids),
            None => vec![None; ids.len()],
        };
        let newest: Vec<(Track, Option<String>)> = newest.into_iter().zip(times).collect();
        Ok(by_activity(listed, &newest))
    }

    pub async fn each_recently_added(
        &self,
        limit: usize,
        mut each: impl FnMut(Vec<Collection>, bool),
    ) -> Result<()> {
        self.each_page(
            &recently_added_path(),
            convert::collection,
            limit,
            |items, last| each(items.into_iter().flatten().collect(), last),
        )
        .await
    }

    /// Every page of the list `first` begins, up to `limit` items, handed to
    /// `each` in order with whether it is the last. The first page says how
    /// long the list is; Apple's lists page by offset, so the rest are then
    /// asked for many at once. Without a length, they come one by one.
    async fn each_page<A, T>(
        &self,
        first: &str,
        convert: fn(types::Resource<A>) -> T,
        limit: usize,
        mut each: impl FnMut(Vec<T>, bool),
    ) -> Result<()>
    where
        A: DeserializeOwned,
    {
        use futures_util::StreamExt;

        let Some(resp) = self
            .fetch::<types::Response<types::Resource<A>>>(first)
            .await?
        else {
            each(Vec::new(), true);
            return Ok(());
        };
        // How far the next page starts, as Apple says: the page's length,
        // items left out for being unreadable included.
        let got = resp
            .next
            .as_deref()
            .and_then(offset_in)
            .unwrap_or(resp.data.len());
        let total = resp.meta.as_ref().and_then(|m| m.total);
        let items: Vec<T> = resp.data.into_iter().map(convert).collect();
        let next = resp.next.filter(|_| got > 0 && got < limit);
        let Some(next) = next else {
            each(items, true);
            return Ok(());
        };
        let Some(total) = total else {
            each(items, false);
            return self
                .each_page_in_turn(first, next, convert, got, limit, each)
                .await;
        };
        let total = total.min(limit);
        let offsets: Vec<usize> = (got..total).step_by(got).collect();
        each(items, offsets.is_empty());
        let count = offsets.len();
        let mut pages = futures_util::stream::iter(offsets)
            .map(|offset| {
                let path = at_offset(&next, offset, first);
                async move { self.fetch_page::<A>(&path).await }
            })
            .buffered(PAGES_AT_ONCE)
            .enumerate();
        while let Some((i, page)) = pages.next().await {
            let items = page?.into_iter().map(convert).collect();
            each(items, i + 1 == count);
        }
        Ok(())
    }

    /// One page, tried again once: with many asked at once, one may fail
    /// where asking again works.
    async fn fetch_page<A: DeserializeOwned>(&self, path: &str) -> Result<Vec<types::Resource<A>>> {
        let resp = match self
            .fetch::<types::Response<types::Resource<A>>>(path)
            .await
        {
            Err(Error::NotSignedIn) => return Err(Error::NotSignedIn),
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                self.fetch(path).await?
            }
            Ok(resp) => resp,
        };
        Ok(resp.map(|r| r.data).unwrap_or_default())
    }

    /// The rest of a list after its first `got` items, one page after another.
    async fn each_page_in_turn<A, T>(
        &self,
        first: &str,
        next: String,
        convert: fn(types::Resource<A>) -> T,
        mut got: usize,
        limit: usize,
        mut each: impl FnMut(Vec<T>, bool),
    ) -> Result<()>
    where
        A: DeserializeOwned,
    {
        let mut next = Some(next);
        while let Some(path) = next.take() {
            let path = carry_query_forward(&path, first);
            let Some(resp) = self
                .fetch::<types::Response<types::Resource<A>>>(&path)
                .await?
            else {
                // Gone from under it (Apple says 404 for some empty pages): done.
                each(Vec::new(), true);
                break;
            };
            got = resp
                .next
                .as_deref()
                .and_then(offset_in)
                .unwrap_or(got + resp.data.len());
            next = resp.next.filter(|_| got < limit);
            each(resp.data.into_iter().map(convert).collect(), next.is_none());
        }
        Ok(())
    }
}

/// The page of `next`'s list starting at `offset`, with `first`'s parameters.
/// Where the page `next` starts.
fn offset_in(next: &str) -> Option<usize> {
    let base = url::Url::parse(API).ok()?;
    base.join(next)
        .ok()?
        .query_pairs()
        .find(|(k, _)| k == "offset")
        .and_then(|(_, v)| v.parse().ok())
}

fn at_offset(next: &str, offset: usize, first: &str) -> String {
    let base = url::Url::parse(API).expect("API base");
    let Ok(mut url) = base.join(next) else {
        return carry_query_forward(next, first);
    };
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| k != "offset")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    url.query_pairs_mut()
        .clear()
        .extend_pairs(kept)
        .append_pair("offset", &offset.to_string());
    let path = format!("{}?{}", url.path(), url.query().unwrap_or_default());
    carry_query_forward(&path, first)
}

/// Apple's `next` keeps the path and the offset but drops `limit` and `include`,
/// so later pages would shrink to the default size. Re-add whatever the first
/// request had that `next` doesn't.
fn carry_query_forward(next: &str, first: &str) -> String {
    let base = url::Url::parse(API).expect("API base");
    let (Ok(mut merged), Ok(first)) = (base.join(next), base.join(first)) else {
        return next.to_owned();
    };
    let present: Vec<String> = merged.query_pairs().map(|(k, _)| k.into_owned()).collect();
    let missing: Vec<(String, String)> = first
        .query_pairs()
        .filter(|(k, _)| !present.iter().any(|p| p == k))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    merged.query_pairs_mut().extend_pairs(missing);
    match merged.query() {
        Some(q) => format!("{}?{q}", merged.path()),
        None => merged.path().to_owned(),
    }
}

#[async_trait]
impl Provider for AppleMusic {
    fn kind(&self) -> ProviderKind {
        ProviderKind::AppleMusic
    }

    async fn search(&self, query: &str) -> Result<SearchResults> {
        let term: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
        let path = format!(
            "/v1/catalog/{}/search?term={term}&types=songs,albums,artists,playlists&limit={SEARCH_LIMIT}",
            self.storefront
        );
        self.search_at(&path, convert::artist).await
    }

    async fn search_library(&self, query: &str) -> Result<SearchResults> {
        let term: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
        // The songs' albums too: without, a song found has none to open,
        // nor its moving cover to show. The artists' catalog entries, for
        // their photos, which the library's own lack.
        let path = format!(
            "/v1/me/library/search?term={term}&types=library-songs,library-albums,library-artists,library-playlists&limit={SEARCH_LIMIT}&include%5Blibrary-songs%5D=albums&include%5Blibrary-artists%5D=catalog"
        );
        let mut found = self.search_at(&path, convert::library_artist).await?;
        self.released_now(&mut found.tracks).await;
        Ok(found)
    }

    async fn library_tracks(&self, cursor: Option<&Cursor>) -> Result<Page<Track>> {
        let mut page = self
            .page(&library_songs_path(), cursor, convert::track)
            .await?;
        self.released_now(&mut page.items).await;
        Ok(page)
    }

    async fn library_tracks_newest(&self, cursor: Option<&Cursor>) -> Result<Page<Track>> {
        let mut page = self
            .page(&newest_songs_path(), cursor, convert::track)
            .await?;
        self.released_now(&mut page.items).await;
        Ok(page)
    }

    async fn library_albums(&self, cursor: Option<&Cursor>) -> Result<Page<Album>> {
        self.page(&library_albums_path(), cursor, convert::album)
            .await
    }

    async fn library_playlists(&self, cursor: Option<&Cursor>) -> Result<Page<Playlist>> {
        self.page(&library_playlists_path(), cursor, convert::playlist)
            .await
    }

    async fn library_artists(&self, cursor: Option<&Cursor>) -> Result<Page<Artist>> {
        self.page(&library_artists_path(), cursor, convert::library_artist)
            .await
    }

    /// The whole of it at once, as the app shows it (see
    /// [`AppleMusic::recently_added_by_activity`]).
    async fn recently_added(&self, cursor: Option<&Cursor>) -> Result<Page<Collection>> {
        let items = match cursor {
            None => self.recently_added_by_activity(RECENT_SHOWN).await?,
            Some(_) => Vec::new(),
        };
        Ok(Page { items, next: None })
    }

    async fn album(&self, album: &ItemId) -> Result<Album> {
        // A library album lacks the genre, the notes and the artist; its catalog
        // counterpart has them, so the header reads that one when there is one.
        let path = if album.id.starts_with("l.") {
            let catalog = format!(
                "/v1/me/library/albums/{}/catalog?include=artists&extend=inFavorites,editorialVideo&l={NOTES_LANGUAGE}",
                album.id
            );
            if let Some(resp) = self
                .fetch::<types::Response<types::Album>>(&catalog)
                .await?
                && let Some(found) = resp.data.into_iter().next()
            {
                let catalog = convert::album(found);
                return Ok(Album {
                    id: album.clone(),
                    catalog_id: Some(catalog.id.clone()),
                    ..catalog
                });
            }
            format!("/v1/me/library/albums/{}", album.id)
        } else {
            // With the artists, for the header's photo of them.
            format!(
                "/v1/catalog/{}/albums/{}?include=artists&extend=inFavorites,editorialVideo&l={NOTES_LANGUAGE}",
                self.storefront, album.id
            )
        };
        let resp: types::Response<types::Album> = self.get(&path).await?;
        resp.data
            .into_iter()
            .next()
            .map(convert::album)
            .ok_or_else(|| Error::Protocol(format!("{path}: no album")))
    }

    async fn album_tracks(&self, album: &ItemId) -> Result<Vec<Track>> {
        let first = if album.id.starts_with("l.") {
            format!("/v1/me/library/albums/{}/tracks?limit={PAGE}", album.id)
        } else {
            format!(
                "/v1/catalog/{}/albums/{}/tracks?limit={PAGE}",
                self.storefront, album.id
            )
        };
        let mut tracks = Vec::new();
        let mut cursor = None;
        loop {
            let page = self.page(&first, cursor.as_ref(), convert::track).await?;
            tracks.extend(page.items.into_iter().map(|mut t| {
                t.album_id.get_or_insert_with(|| album.clone());
                t
            }));
            match page.next {
                Some(next) => cursor = Some(next),
                None => {
                    self.released_now(&mut tracks).await;
                    return Ok(tracks);
                }
            }
        }
    }

    async fn playlist_tracks(
        &self,
        playlist: &ItemId,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Track>> {
        let mut page = self
            .page(&self.playlist_tracks_path(playlist), cursor, convert::track)
            .await?;
        self.released_now(&mut page.items).await;
        Ok(page)
    }

    /// The artist's top songs, albums, and singles and EPs, as the web player
    /// shows them. A library artist (`r.` id) is looked up in the catalog first.
    async fn artist(&self, artist: &ItemId) -> Result<ArtistDetail> {
        let catalog_id = if artist.id.starts_with("r.") {
            let path = format!("/v1/me/library/artists/{}/catalog", artist.id);
            let resp: types::Response<types::Artist> = self.get(&path).await?;
            resp.data
                .into_iter()
                .next()
                .map(|a| a.id)
                .ok_or_else(|| Error::Protocol(format!("{path}: not in the catalog")))?
        } else {
            artist.id.clone()
        };
        let path = format!(
            "/v1/catalog/{}/artists/{catalog_id}?views=top-songs,latest-release,featured-albums,full-albums,singles,music-videos,featured-playlists,compilation-albums,appears-on-albums,similar-artists&extend=inFavorites,editorialVideo,editorialArtwork&limit[view.top-songs]=30&limit[view.full-albums]=50&limit[view.singles]=50&limit[view.music-videos]=20&limit[view.appears-on-albums]=30&limit[view.similar-artists]=20",
            self.storefront
        );
        let resp: types::Response<types::ArtistWithViews> = self.get(&path).await?;
        let found = resp
            .data
            .into_iter()
            .next()
            .ok_or_else(|| Error::Protocol(format!("{path}: no artist")))?;
        fn all<A, T>(list: Option<types::Response<A>>, f: fn(A) -> T) -> Vec<T> {
            list.map(|l| l.data.into_iter().map(f).collect())
                .unwrap_or_default()
        }
        let views = found.views;
        let (motion, banner) = found
            .attributes
            .as_ref()
            .map(convert::artist_banner)
            .unwrap_or_default();
        Ok(ArtistDetail {
            artist: convert::artist(types::Artist {
                id: found.id,
                kind: String::new(),
                attributes: found.attributes,
                relationships: None,
            }),
            top_songs: all(views.top_songs, convert::track),
            latest: views
                .latest_release
                .and_then(|l| l.data.into_iter().next())
                .map(convert::album),
            albums: all(views.full_albums, convert::album),
            singles: all(views.singles, convert::album),
            compilations: all(views.compilation_albums, convert::album),
            appears_on: all(views.appears_on_albums, convert::album),
            similar: all(views.similar_artists, convert::artist),
            essentials: all(views.featured_albums, convert::album),
            videos: views
                .music_videos
                .map(|l| {
                    l.data
                        .into_iter()
                        .filter_map(convert::music_video)
                        .collect()
                })
                .unwrap_or_default(),
            playlists: all(views.featured_playlists, convert::playlist),
            motion,
            banner,
        })
    }

    /// Apple's favourites, as its own client sets them: `POST` adds, `DELETE`
    /// removes, the item named in the query (Kopuz found the form).
    async fn set_favourite(&self, kind: FavouriteKind, id: &ItemId, on: bool) -> Result<()> {
        let path = format!("/v1/me/favorites?ids%5B{}%5D={}", catalog_type(kind), id.id);
        let req = if on {
            self.http
                .post(format!("{API}{path}"))
                .header("Content-Length", "0")
        } else {
            self.http.delete(format!("{API}{path}"))
        };
        self.send_change(req, &path).await
    }

    /// As Apple's client adds: the item in the query, no body (Kopuz's form).
    async fn add_to_library(&self, kind: FavouriteKind, id: &ItemId) -> Result<()> {
        let path = format!(
            "/v1/me/library?ids%5B{}%5D={}&representation=ids",
            catalog_type(kind),
            id.id
        );
        let req = self
            .http
            .post(format!("{API}{path}"))
            .header("Content-Length", "0");
        self.send_change(req, &path).await
    }

    /// Apple's `inLibrary`: an album is in only once all its songs are, which
    /// is when the web player stops offering to add it.
    async fn in_library(&self, kind: FavouriteKind, ids: &[ItemId]) -> Result<Vec<(ItemId, bool)>> {
        let mut known = Vec::new();
        for chunk in ids.chunks(25) {
            let list: Vec<&str> = chunk.iter().map(|i| i.id.as_str()).collect();
            let path = format!(
                "/v1/catalog/{}/{}?ids={}&fields=inLibrary",
                self.storefront,
                catalog_type(kind),
                list.join(",")
            );
            let Some(resp) = self
                .fetch::<types::Response<types::Resource<types::InLibrary>>>(&path)
                .await?
            else {
                continue;
            };
            known.extend(resp.data.into_iter().map(|r| {
                let on = r.attributes.is_some_and(|a| a.in_library);
                (ItemId::new(ProviderKind::AppleMusic, r.id), on)
            }));
        }
        Ok(known)
    }

    /// Both from one request for the catalog song (Kopuz asks for the station
    /// the same way).
    async fn song_links(&self, song: &ItemId) -> Result<SongLinks> {
        let path = format!(
            "/v1/catalog/{}/songs/{}?include=artists,station",
            self.storefront, song.id
        );
        let resp: Option<types::Response<types::Song>> = self.fetch(&path).await?;
        let relationships = resp
            .and_then(|r| r.data.into_iter().next())
            .and_then(|s| s.relationships);
        let first = |related: Option<&types::Related>| {
            related
                .and_then(|r| r.data.first())
                .map(|r| ItemId::new(ProviderKind::AppleMusic, r.id.clone()))
        };
        Ok(SongLinks {
            artist: first(relationships.as_ref().and_then(|r| r.artists.as_ref())),
            station: first(relationships.as_ref().and_then(|r| r.station.as_ref())),
        })
    }

    /// A library album's id starts with `l.`, a song's with `i.`.
    async fn remove_from_library(&self, library_id: &ItemId) -> Result<()> {
        let kind = if library_id.id.starts_with("l.") {
            "albums"
        } else {
            "songs"
        };
        let path = format!("/v1/me/library/{kind}/{}", library_id.id);
        let req = self.http.delete(format!("{API}{path}"));
        self.send_change(req, &path).await
    }

    /// As the web player edits them. Apple takes no cover from it.
    async fn edit_playlist(&self, playlist: &ItemId, name: &str, description: &str) -> Result<()> {
        let path = format!("/v1/me/library/playlists/{}", playlist.id);
        let body = serde_json::json!({
            "attributes": { "name": name, "description": description },
        });
        let req = self.http.patch(format!("{API}{path}")).json(&body);
        self.send_change(req, &path).await
    }

    /// As the web player does it: every row of that song goes.
    async fn remove_from_playlist(&self, playlist: &ItemId, entry: &ItemId) -> Result<()> {
        let path = format!(
            "/v1/me/library/playlists/{}/tracks?ids%5Blibrary-songs%5D={}&mode=all",
            playlist.id, entry.id
        );
        let req = self.http.delete(format!("{API}{path}"));
        self.send_change(req, &path).await
    }

    /// As the web player deletes them.
    async fn delete_playlist(&self, playlist: &ItemId) -> Result<()> {
        let path = format!("/v1/me/library/playlists/{}", playlist.id);
        let req = self.http.delete(format!("{API}{path}"));
        self.send_change(req, &path).await
    }

    /// As Kopuz makes them: the name, and the songs as the playlist's tracks.
    async fn create_playlist(
        &self,
        name: &str,
        description: &str,
        tracks: &[ItemId],
    ) -> Result<Playlist> {
        let path = "/v1/me/library/playlists";
        let data: Vec<serde_json::Value> = tracks
            .iter()
            .map(|t| serde_json::json!({ "id": t.id, "type": "songs" }))
            .collect();
        let mut attributes = serde_json::json!({ "name": name });
        if !description.is_empty() {
            attributes["description"] = description.into();
        }
        let body = serde_json::json!({
            "attributes": attributes,
            "relationships": { "tracks": { "data": data } },
        });
        let resp = self
            .send(
                self.http
                    .post(format!("{API}{path}"))
                    .header("Media-User-Token", &self.user_token)
                    .json(&body),
                path,
            )
            .await?;
        let status = resp.status();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(Error::NotSignedIn);
        }
        if !status.is_success() {
            return Err(Error::Protocol(format!("{path}: HTTP {status}")));
        }
        let created: types::Response<types::Playlist> = resp
            .json()
            .await
            .map_err(|e| Error::Protocol(format!("{path}: {e}")))?;
        let playlist = created
            .data
            .into_iter()
            .next()
            .ok_or_else(|| Error::Protocol(format!("{path}: no playlist made")))?;
        // The user's own, whatever the answer says.
        Ok(Playlist {
            editable: true,
            ..convert::playlist(playlist)
        })
    }

    /// Songs go by catalog id with their type, or Apple accepts them and
    /// drops them silently (as Kopuz found).
    async fn add_to_playlist(&self, playlist: &ItemId, tracks: &[ItemId]) -> Result<()> {
        let path = format!(
            "/v1/me/library/playlists/{}/tracks?representation=resources",
            playlist.id
        );
        let data: Vec<serde_json::Value> = tracks
            .iter()
            .map(|t| serde_json::json!({ "id": t.id, "type": "songs" }))
            .collect();
        let req = self
            .http
            .post(format!("{API}{path}"))
            .json(&serde_json::json!({ "data": data }));
        self.send_change(req, &path).await
    }

    async fn is_favourite(&self, kind: FavouriteKind, id: &ItemId) -> Result<Option<bool>> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Favourite {
            in_favorites: Option<bool>,
        }
        let path = format!(
            "/v1/catalog/{}/{}/{}?extend=inFavorites",
            self.storefront,
            catalog_type(kind),
            id.id
        );
        let resp: Option<types::Response<types::Resource<Favourite>>> = self.fetch(&path).await?;
        Ok(resp
            .and_then(|r| r.data.into_iter().next())
            .and_then(|r| r.attributes)
            .and_then(|a| a.in_favorites))
    }

    /// Apple's Suggest Less: the item rated -1, as its clients dislike.
    async fn suggest_less(&self, kind: FavouriteKind, id: &ItemId, on: bool) -> Result<()> {
        let path = format!("/v1/me/ratings/{}/{}", catalog_type(kind), id.id);
        let req = if on {
            self.http
                .put(format!("{API}{path}"))
                .json(&serde_json::json!({
                    "type": "rating",
                    "attributes": { "value": -1 },
                }))
        } else {
            self.http.delete(format!("{API}{path}"))
        };
        self.send_change(req, &path).await
    }

    /// Unrated, Apple says not found.
    async fn suggests_less(&self, kind: FavouriteKind, id: &ItemId) -> Result<Option<bool>> {
        #[derive(serde::Deserialize)]
        struct Rating {
            value: i8,
        }
        let path = format!("/v1/me/ratings/{}/{}", catalog_type(kind), id.id);
        let resp: Option<types::Response<types::Resource<Rating>>> = self.fetch(&path).await?;
        Ok(Some(
            resp.and_then(|r| r.data.into_iter().next())
                .and_then(|r| r.attributes)
                .is_some_and(|a| a.value < 0),
        ))
    }

    async fn share_link(&self, kind: FavouriteKind, id: &ItemId) -> Result<Option<String>> {
        let kind = match kind {
            FavouriteKind::Song => "song",
            FavouriteKind::Album => "album",
            FavouriteKind::Artist => "artist",
            FavouriteKind::Playlist => "playlist",
        };
        Ok(Some(format!(
            "https://music.apple.com/{}/{kind}/{}",
            self.storefront, id.id
        )))
    }

    /// Apple's radio page: the stations on air in a row of their own, then
    /// its shelves of shows, episodes and stations, as the web player shows
    /// them to a subscriber.
    async fn radio(&self) -> Result<Vec<Shelf>> {
        let path = format!(
            "/v1/editorial/{}/groupings?platform=web&name=radio&l={EDITORIAL_LANGUAGE}",
            self.storefront
        );
        let Some(page) = self.fetch::<serde_json::Value>(&path).await? else {
            return Ok(Vec::new());
        };
        Ok(editorial_shelves(&page))
    }

    /// Apple's new music page: the week's picks, the songs of the moment,
    /// new albums and updated playlists, as the web player shows them.
    async fn new_music(&self) -> Result<Vec<Shelf>> {
        let path = format!(
            "/v1/editorial/{}/groupings?platform=web&name=music&extend=editorialArtwork&l={EDITORIAL_LANGUAGE}",
            self.storefront
        );
        let Some(page) = self.fetch::<serde_json::Value>(&path).await? else {
            return Ok(Vec::new());
        };
        Ok(editorial_shelves(&page))
    }

    /// The catalogue's genres under Music, as Apple names them here.
    async fn genres(&self) -> Result<Vec<music_core::model::Genre>> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Attributes {
            name: String,
            parent_id: Option<String>,
        }
        let path = format!(
            "/v1/catalog/{}/genres?limit=100&l={EDITORIAL_LANGUAGE}",
            self.storefront
        );
        let Some(resp) = self
            .fetch::<types::Response<types::Resource<Attributes>>>(&path)
            .await?
        else {
            return Ok(Vec::new());
        };
        // Music itself (34) is the root: the genres are its children.
        Ok(resp
            .data
            .into_iter()
            .filter_map(|r| {
                let a = r.attributes?;
                (a.parent_id.as_deref() == Some(MUSIC_GENRE)).then_some(music_core::model::Genre {
                    id: r.id,
                    name: a.name,
                })
            })
            .collect())
    }

    /// A genre's charts: its top songs, then its top albums and playlists.
    async fn genre_charts(&self, genre: &str) -> Result<Vec<Shelf>> {
        #[derive(serde::Deserialize)]
        #[serde(bound(deserialize = "T: serde::de::DeserializeOwned"))]
        struct Chart<T> {
            name: String,
            #[serde(deserialize_with = "types::readable")]
            data: Vec<T>,
        }
        #[derive(serde::Deserialize, Default)]
        #[serde(default)]
        struct Charts {
            songs: Vec<Chart<types::Song>>,
            albums: Vec<Chart<types::Album>>,
            playlists: Vec<Chart<types::Playlist>>,
        }
        #[derive(serde::Deserialize)]
        struct Resp {
            results: Charts,
        }
        let genre: String = url::form_urlencoded::byte_serialize(genre.as_bytes()).collect();
        let path = format!(
            "/v1/catalog/{}/charts?types=songs,albums,playlists&genre={genre}&limit=30&l={EDITORIAL_LANGUAGE}",
            self.storefront
        );
        let Some(resp) = self.fetch::<Resp>(&path).await? else {
            return Ok(Vec::new());
        };
        let r = resp.results;
        let mut shelves = Vec::new();
        for chart in r.songs {
            shelves.push(Shelf {
                title: chart.name,
                songs: chart.data.into_iter().map(convert::track).collect(),
                ..Shelf::default()
            });
        }
        for chart in r.albums {
            shelves.push(Shelf {
                title: chart.name,
                items: chart
                    .data
                    .into_iter()
                    .map(|a| Collection::Album(convert::album(a)))
                    .collect(),
                ..Shelf::default()
            });
        }
        for chart in r.playlists {
            shelves.push(Shelf {
                title: chart.name,
                items: chart
                    .data
                    .into_iter()
                    .map(|p| Collection::Playlist(convert::playlist(p)))
                    .collect(),
                ..Shelf::default()
            });
        }
        Ok(shelves
            .into_iter()
            .filter(|s| !s.songs.is_empty() || !s.items.is_empty())
            .collect())
    }

    /// A station's next songs, about `count` of them. Apple hands out two a
    /// call and its calls share a cursor, so they go four at a time and
    /// repeats are dropped (Kopuz measured the trade-off).
    async fn station_tracks(&self, station: &ItemId, count: usize) -> Result<Vec<Track>> {
        let path = format!("/v1/me/stations/next-tracks/{}", station.id);
        let call = || async {
            let resp = self
                .send(
                    self.http
                        .post(format!("{API}{path}"))
                        .header("Media-User-Token", &self.user_token)
                        .header("Content-Length", "0"),
                    &path,
                )
                .await?;
            if !resp.status().is_success() {
                return Err(Error::Protocol(format!("{path}: HTTP {}", resp.status())));
            }
            resp.json::<types::Response<types::Song>>()
                .await
                .map_err(|e| Error::Protocol(format!("{path}: {e}")))
        };
        let mut tracks: Vec<Track> = Vec::new();
        let mut barren = 0;
        // Why no song came, if none does: signed out, or out of reach.
        let mut failed = None;
        for _ in 0..8 {
            if tracks.len() >= count {
                break;
            }
            let before = tracks.len();
            let (a, b, c, d) = tokio::join!(call(), call(), call(), call());
            let mut answered = false;
            for page in [a, b, c, d] {
                let page = match page {
                    Ok(page) => page,
                    Err(e) => {
                        failed.get_or_insert(e);
                        continue;
                    }
                };
                answered = true;
                for song in page.data {
                    let track = convert::track(song);
                    if !tracks.iter().any(|t| t.id == track.id) {
                        tracks.push(track);
                    }
                }
            }
            if !answered {
                break;
            }
            if tracks.len() == before {
                barren += 1;
                if barren == 2 {
                    break;
                }
            }
        }
        if tracks.is_empty() {
            return Err(
                failed.unwrap_or_else(|| Error::Playback("the station offered no songs".into()))
            );
        }
        tracks.truncate(count);
        Ok(tracks)
    }

    /// Apple's home: the shelves the web player shows, titled in English as
    /// the app is.
    async fn home(&self) -> Result<Vec<Shelf>> {
        let path = format!("/v1/me/recommendations?limit=20&l={NOTES_LANGUAGE}");
        let Some(resp) = self
            .fetch::<types::Response<types::Recommendation>>(&path)
            .await?
        else {
            return Ok(Vec::new());
        };
        Ok(resp.data.into_iter().filter_map(convert::shelf).collect())
    }

    /// Word-timed lyrics, else line-timed ones. Uploads (`i.` ids) are not
    /// in the catalog and have none. A song saved long ago may name a
    /// release since replaced, without lyrics: the one in its place has
    /// them, as Apple Music shows.
    async fn lyrics(&self, track: &ItemId) -> Result<Option<Lyrics>> {
        if track.id.starts_with("i.") {
            return Ok(None);
        }
        if let Some(lyrics) = self.lyrics_of(&track.id).await? {
            return Ok(Some(lyrics));
        }
        let path = format!(
            "/v1/catalog/{}/songs?filter%5Bequivalents%5D={}",
            self.storefront, track.id
        );
        let Some(resp) = self
            .fetch::<types::Response<types::Resource<serde::de::IgnoredAny>>>(&path)
            .await?
        else {
            return Ok(None);
        };
        match resp.data.into_iter().find(|r| r.id != track.id) {
            Some(now) => self.lyrics_of(&now.id).await,
            None => Ok(None),
        }
    }
}

impl AppleMusic {
    /// Catalog song `id`'s lyrics, word-timed if it has them.
    async fn lyrics_of(&self, id: &str) -> Result<Option<Lyrics>> {
        for kind in ["syllable-lyrics", "lyrics"] {
            let path = format!("/v1/catalog/{}/songs/{id}/{kind}", self.storefront);
            let Some(resp) = self.fetch::<types::LyricsResponse>(&path).await? else {
                continue;
            };
            let ttml = resp.data.into_iter().find_map(|r| r.attributes?.ttml);
            if let Some(lyrics) = ttml.as_deref().and_then(lyrics::parse_ttml) {
                return Ok(Some(lyrics));
            }
        }
        Ok(None)
    }
}

/// `listed` (Recently Added, by when each joined the library) in the order of
/// what was added last, as `newest` (the songs, the latest added first) says:
/// an album comes up to where its latest song stands, what is between keeps
/// its order. Each album's `added` becomes when it was last added to, as near
/// as is known, so that a merge by date keeps the order.
fn by_activity(listed: Vec<Collection>, newest: &[(Track, Option<String>)]) -> Vec<Collection> {
    use std::collections::{HashMap, HashSet};
    // The albums of the newest songs, each once, with the song saying so,
    // and when it was added.
    let mut seen = HashSet::new();
    let touched: Vec<(&ItemId, &Track, &Option<String>)> = newest
        .iter()
        .filter_map(|(t, added)| Some((t.album_id.as_ref()?, t, added)))
        .filter(|(id, _, _)| seen.insert(*id))
        .collect();
    let rank: HashMap<&ItemId, usize> = touched
        .iter()
        .enumerate()
        .map(|(i, (id, _, _))| (*id, i))
        .collect();
    let albums: HashMap<ItemId, Album> = listed
        .iter()
        .filter_map(|c| match c {
            Collection::Album(a) => Some((a.id.clone(), a.clone())),
            _ => None,
        })
        .collect();
    let mut out = Vec::with_capacity(listed.len());
    let mut next = 0;
    for item in listed {
        match &item {
            // It comes after the albums added to since; it may have come
            // already, as one of them.
            Collection::Album(a) if let Some(&at) = rank.get(&a.id) => {
                while next <= at {
                    let (id, song, song_added) = touched[next];
                    next += 1;
                    let mut album = albums.get(id).cloned().unwrap_or_else(|| Album {
                        id: id.clone(),
                        title: song.album.clone().unwrap_or_default(),
                        artists: song.artists.clone(),
                        artist_id: None,
                        artist_artwork: None,
                        released: None,
                        genre: None,
                        description: None,
                        catalog_id: None,
                        in_favourites: None,
                        year: None,
                        artwork: song.artwork.clone(),
                        track_count: None,
                        explicit: song.explicit,
                        added: None,
                        motion: None,
                        copyright: None,
                        mastered: false,
                    });
                    // Added to when the song was: among the other services'
                    // too, it is that recent.
                    if *song_added > album.added {
                        album.added.clone_from(song_added);
                    }
                    out.push(Collection::Album(album));
                }
            }
            _ => out.push(item),
        }
    }
    // Last added to no earlier than what follows it.
    let mut floor: Option<String> = None;
    for item in out.iter_mut().rev() {
        if let Collection::Album(a) = item {
            if a.added < floor {
                a.added.clone_from(&floor);
            }
            floor.clone_from(&a.added);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_album_a_song_was_just_added_to_comes_up() {
        let album = |id: &str, added: &str| {
            Collection::Album(Album {
                id: ItemId::new(ProviderKind::AppleMusic, id),
                title: id.to_owned(),
                artists: Vec::new(),
                artist_id: None,
                artist_artwork: None,
                released: None,
                genre: None,
                description: None,
                catalog_id: None,
                in_favourites: None,
                year: None,
                artwork: None,
                track_count: None,
                explicit: false,
                added: Some(added.to_owned()),
                motion: None,
                copyright: None,
                mastered: false,
            })
        };
        let song = |album: &str| Track {
            id: ItemId::new(ProviderKind::AppleMusic, "i.song"),
            title: "song".into(),
            artists: Vec::new(),
            album: Some(album.to_owned()),
            album_id: Some(ItemId::new(ProviderKind::AppleMusic, album)),
            duration: None,
            track_number: None,
            disc_number: None,
            artwork: None,
            explicit: false,
            quality: None,
            in_library: true,
            library_id: None,
        };
        let playlist_item = |name: &str| {
            Collection::Playlist(Playlist {
                id: ItemId::new(ProviderKind::AppleMusic, name),
                name: name.to_owned(),
                description: None,
                curator: None,
                favourites: false,
                artwork: None,
                track_count: None,
                catalog_id: None,
                in_favourites: None,
                editable: true,
                added: None,
            })
        };
        // `old` joined long ago and had a song added last; `p` is a playlist.
        let listed = vec![
            album("a", "2026-10-28"),
            album("b", "2026-10-25"),
            playlist_item("p"),
            album("c", "2026-10-20"),
            album("old", "2026-01-01"),
        ];
        let newest = [
            (song("old"), Some("2026-10-30".to_owned())),
            (song("a"), Some("2026-10-28".to_owned())),
            (song("b"), Some("2026-10-25".to_owned())),
            (song("b"), Some("2026-10-24".to_owned())),
            (song("c"), Some("2026-10-20".to_owned())),
        ];
        let titles: Vec<(String, Option<String>)> = by_activity(listed, &newest)
            .into_iter()
            .map(|c| match c {
                Collection::Album(a) => (a.title, a.added),
                Collection::Playlist(p) => (p.name, None),
                Collection::Station(s) => (s.name, None),
            })
            .collect();
        let date = |d: &str| Some(d.to_owned());
        assert_eq!(
            titles,
            [
                ("old".into(), date("2026-10-30")),
                ("a".into(), date("2026-10-28")),
                ("b".into(), date("2026-10-25")),
                ("p".into(), None),
                ("c".into(), date("2026-10-20")),
            ]
        );
    }

    #[test]
    fn the_next_page_keeps_the_first_requests_parameters() {
        let next = carry_query_forward(
            "/v1/me/library/songs?offset=100",
            "/v1/me/library/songs?limit=100&include=albums",
        );
        assert_eq!(
            next,
            "/v1/me/library/songs?offset=100&limit=100&include=albums"
        );
    }

    #[test]
    fn a_page_at_an_offset_keeps_the_first_requests_parameters() {
        assert_eq!(
            at_offset(
                "/v1/me/library/songs?offset=100",
                700,
                "/v1/me/library/songs?limit=100&include=albums"
            ),
            "/v1/me/library/songs?offset=700&limit=100&include=albums"
        );
    }

    #[test]
    fn the_next_page_wins_where_both_set_a_parameter() {
        let next = carry_query_forward(
            "/v1/me/library/albums?offset=200&limit=50",
            "/v1/me/library/albums?limit=100",
        );
        assert_eq!(next, "/v1/me/library/albums?offset=200&limit=50");
    }
}
