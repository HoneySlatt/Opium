//! Apple Music through the web player's private API (`amp-api.music.apple.com`).
//!
//! Neither token comes from an official developer account: the developer token is
//! read from the web player ([`auth`]) and the user token from a browser sign-in
//! ([`signin`]).

pub mod auth;
mod convert;
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub mod fuzzing;
mod lyrics;
mod motion;
mod playback;
mod radio;
mod resume;
mod seen;
pub use seen::SeenElsewhere;
#[cfg(test)]
mod mutation_tests;
pub mod signin;
mod station;
#[cfg(test)]
mod test_support;
mod types;

pub use cenc::AudioStream;
pub use playback::{ASSET_HOST, PRIMING, Protected};
pub use radio::{LiveAsset, LiveStream, name_the_key};

use async_trait::async_trait;
use opium_core::model::{
    Album, AlbumExtras, Artist, ArtistDetail, Artwork, Collection, Cursor, FavouriteKind, ItemId,
    Lyrics, Page, Pick, Playlist, ProviderKind, SearchResults, Shelf, SongLinks, Track,
};
use opium_core::{Error, Provider, Result};
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

/// Where Apple's services answer: always Apple's own addresses, as [`Hosts::apple`] gives
/// them, but in tests, which put a local server in their place. Each is a scheme and a
/// host, without a final `/`.
#[derive(Debug, Clone)]
struct Hosts {
    /// The API (`amp-api.music.apple.com`).
    api: String,
    /// The web player, which the developer token is read from.
    web_player: String,
    /// The web player's playback service (`webPlayback`).
    play: String,
    /// The licence server.
    licence: String,
}

impl Hosts {
    fn apple() -> Self {
        Self {
            api: API.into(),
            web_player: WEB_PLAYER.into(),
            play: playback::PLAY_HOST.into(),
            licence: playback::LICENCE_HOST.into(),
        }
    }

    /// Each host on the local server at `port`, plain HTTP, under Apple's names: the test
    /// client resolves them to `127.0.0.1`.
    #[cfg(test)]
    fn local(port: u16) -> Self {
        let local = |url: &str| format!("{}:{port}", url.replacen("https://", "http://", 1));
        Self {
            api: local(API),
            web_player: local(WEB_PLAYER),
            play: local(playback::PLAY_HOST),
            licence: local(playback::LICENCE_HOST),
        }
    }
}

/// The crate's one HTTP client, but for what tests add: its user agent, its timeouts and its
/// redirect policy.
fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        // A server out of reach, or a download that stalls, fails rather
        // than hang what waits on it.
        .connect_timeout(std::time::Duration::from_secs(10))
        .read_timeout(std::time::Duration::from_secs(30))
        // A song's start needs three of Apple's hosts: their connections
        // are kept a while, not dropped after 90 s.
        .pool_idle_timeout(std::time::Duration::from_secs(300))
        // Every request carries the account's token in a header of
        // Apple's own, which nothing strips on the way: a redirect to
        // another host would hand the account over with it, so one is
        // followed only to Apple's own hosts.
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let apple = attempt
                .url()
                .host_str()
                .is_some_and(|host| host == "apple.com" || host.ends_with(".apple.com"));
            if !apple {
                attempt.stop()
            } else if attempt.previous().len() > 5 {
                attempt.error("too many redirects")
            } else {
                attempt.follow()
            }
        }))
}

pub struct AppleMusic {
    http: reqwest::Client,
    hosts: Hosts,
    /// Borrowed from the web player, which swaps it now and then: read again
    /// when Apple turns it down.
    developer_token: std::sync::RwLock<String>,
    /// When it was last read again on a refusal and the web player answered, so a run of
    /// refusals reads it once; a reading before it expires leaves it as it is. Its lock is
    /// held through every reading, of either kind, so that one runs at a time.
    renewed: tokio::sync::Mutex<Option<std::time::Instant>>,
    /// When a reading before it expires was last begun, whatever its outcome: one in
    /// [`RENEW_EVERY`] at most, while the old token is still good.
    read_before_expiry: std::sync::Mutex<Option<std::time::Instant>>,
    /// How far a test has moved the clocks on, as [`now`](Self::now) and
    /// [`renewal_clock`](Self::renewal_clock) read them.
    #[cfg(test)]
    clock_moved_on: std::sync::Mutex<std::time::Duration>,
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

/// How long before the developer token expires it is read again.
const RENEW_BEFORE_EXPIRY: std::time::Duration = std::time::Duration::from_secs(600);

/// Whether `token` expires within [`RENEW_BEFORE_EXPIRY`] of `now`, or has: never for a token
/// whose expiry does not read.
fn expires_soon(token: &str, now: std::time::SystemTime) -> bool {
    auth::expires_at(token).is_some_and(|at| {
        match at.duration_since(now) {
            Ok(left) => left <= RENEW_BEFORE_EXPIRY,
            // Expired already.
            Err(_) => true,
        }
    })
}

/// `id`, when it is an id of Apple's that may go into an address unescaped: a catalog
/// number, or one to three lower-case letters, a dot and one or more ASCII letters, digits
/// or dashes (`i.AbC9-x`, `pl.u-1b2`, `ra.123`).
///
/// # Errors
///
/// [`Error::Protocol`], naming `id`, for anything else: a request that would change the
/// account is refused before it is sent.
fn apple_id(id: &str) -> Result<&str> {
    let catalog = !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit());
    let prefixed = id.split_once('.').is_some_and(|(kind, name)| {
        (1..=3).contains(&kind.len())
            && kind.bytes().all(|b| b.is_ascii_lowercase())
            && !name.is_empty()
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    });
    if catalog || prefixed {
        Ok(id)
    } else {
        Err(Error::Protocol(format!("not an Apple Music id: {id:?}")))
    }
}

/// The account as others see it on Apple Music.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub name: Option<String>,
    /// Without the `@`.
    pub handle: Option<String>,
    pub picture: Option<opium_core::model::Artwork>,
}

impl AppleMusic {
    /// Fetch a developer token and look up the account's storefront (its country),
    /// which every catalog request is scoped to.
    pub async fn connect(user_token: impl Into<String>) -> Result<Self> {
        let http = client_builder()
            .build()
            .map_err(|e| Error::Network(e.to_string()))?;
        Self::connect_through(http, Hosts::apple(), user_token.into()).await
    }

    /// [`connect`](Self::connect), through `http` to `hosts`.
    async fn connect_through(
        http: reqwest::Client,
        hosts: Hosts,
        user_token: String,
    ) -> Result<Self> {
        let developer_token = auth::developer_token_at(&http, &hosts.web_player).await?;
        let mut am = Self {
            http,
            hosts,
            developer_token: std::sync::RwLock::new(developer_token),
            renewed: tokio::sync::Mutex::new(None),
            read_before_expiry: std::sync::Mutex::new(None),
            #[cfg(test)]
            clock_moved_on: std::sync::Mutex::new(std::time::Duration::ZERO),
            user_token,
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

    /// The address of `path` on the API's host.
    fn api(&self, path: &str) -> String {
        format!("{}{path}", self.hosts.api)
    }

    /// Send `request` with both tokens. Turned down with a 401, the developer
    /// token having expired or been swapped, it is read again from the web
    /// player and the request sent once more: a long session doesn't end up
    /// signed out for it. A token about to expire is read again before, once
    /// in [`RENEW_EVERY`] at most.
    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        what: &str,
    ) -> Result<reqwest::Response> {
        let mut token = self.developer_token();
        // The old token goes on being sent when the web player can't be read.
        if expires_soon(&token, self.now())
            && self.may_read_before_expiry()
            && self.read_before_expiry(&token).await
        {
            token = self.developer_token();
        }
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

    /// Whether a request may read the developer token again before it expires: not when one
    /// began within [`RENEW_EVERY`], whatever came of it. Saying yes counts as beginning one,
    /// so that requests sent meanwhile go on with the old token, which is still good, rather
    /// than each read the web player again or wait for it.
    fn may_read_before_expiry(&self) -> bool {
        let mut last = self
            .read_before_expiry
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if last.is_some_and(|at| at.elapsed() < RENEW_EVERY) {
            return false;
        }
        *last = Some(std::time::Instant::now());
        true
    }

    /// Read the developer token again before `expiring`, the token in use, expires; whether
    /// there is a new one to send. Never counts towards [`renew_developer_token`]'s
    /// five minutes: Apple may serve the old token until its `exp`, and the first refusal
    /// after it must still read the new one at once.
    ///
    /// [`renew_developer_token`]: Self::renew_developer_token
    async fn read_before_expiry(&self, expiring: &str) -> bool {
        // One reading at a time, of either kind.
        let _reading = self.renewed.lock().await;
        if self.developer_token() != expiring {
            return true;
        }
        matches!(self.read_token_in_place_of(expiring).await, Ok(true))
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
        if renewed.is_some_and(|at| self.renewal_clock().duration_since(at) < RENEW_EVERY) {
            return Ok(false);
        }
        match self.read_token_in_place_of(refused).await {
            Err(e @ Error::Network(_)) => Err(e),
            // The page or the script answered, though not with a token: wait before asking
            // again rather than read the page at every refusal.
            Err(_) => {
                *renewed = Some(self.renewal_clock());
                Ok(false)
            }
            Ok(new) => {
                *renewed = Some(self.renewal_clock());
                Ok(new)
            }
        }
    }

    /// Read the developer token from the web player, and put it in place of `old` when it is
    /// another; whether it is. The caller holds `renewed`'s lock, so that one reading runs at a
    /// time.
    ///
    /// # Errors
    ///
    /// As [`auth::developer_token`]'s; the token in use is kept.
    async fn read_token_in_place_of(&self, old: &str) -> Result<bool> {
        let token = auth::developer_token_at(&self.http, &self.hosts.web_player).await?;
        if token == old {
            return Ok(false);
        }
        *self
            .developer_token
            .write()
            .unwrap_or_else(|e| e.into_inner()) = token;
        Ok(true)
    }

    /// The monotonic clock `renewed` is dated and aged by.
    #[cfg(not(test))]
    fn renewal_clock(&self) -> std::time::Instant {
        std::time::Instant::now()
    }

    /// The monotonic clock `renewed` is dated and aged by: in tests, moved on by
    /// [`move_clock_on`](Self::move_clock_on), so that a test ages a reading by five minutes
    /// without waiting, and without needing a machine that has been up that long.
    #[cfg(test)]
    fn renewal_clock(&self) -> std::time::Instant {
        std::time::Instant::now()
            + *self
                .clock_moved_on
                .lock()
                .unwrap_or_else(|e| e.into_inner())
    }

    /// The time the developer token's `exp` is compared with: the wall clock.
    #[cfg(not(test))]
    fn now(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
    }

    /// The time the developer token's `exp` is compared with: in tests, the wall clock moved
    /// on by [`move_clock_on`](Self::move_clock_on), so that a test crosses `exp` without
    /// waiting for it.
    #[cfg(test)]
    fn now(&self) -> std::time::SystemTime {
        std::time::SystemTime::now()
            + *self
                .clock_moved_on
                .lock()
                .unwrap_or_else(|e| e.into_inner())
    }

    /// Move the clocks that [`now`](Self::now) and [`renewal_clock`](Self::renewal_clock) read
    /// on by `by`, for this account only.
    #[cfg(test)]
    fn move_clock_on(&self, by: std::time::Duration) {
        *self
            .clock_moved_on
            .lock()
            .unwrap_or_else(|e| e.into_inner()) += by;
    }

    /// Forget when the developer token was last read again, as if five minutes had passed.
    #[cfg(test)]
    fn as_if_the_renewals_were_long_ago(&self) {
        *self.renewed.try_lock().expect("no renewal under way") = None;
        *self
            .read_before_expiry
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// `Ok(None)` on 404, which Apple also answers for some empty collections.
    async fn fetch<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>> {
        let mut waited = false;
        let resp = loop {
            let resp = self
                .send(
                    self.http
                        .get(self.api(path))
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
        // No cursor back to the page just asked for, or to an offset not past it: whoever
        // follows cursors would ask for it for ever.
        let starts = offset_in(path).unwrap_or(0);
        let next = resp
            .next
            .filter(|next| offset_moves_on(next, starts))
            .map(|next| next_page_path(&next, first))
            .filter(|next| next != path);
        Ok(Page {
            items: resp.data.into_iter().map(convert).collect(),
            next: next.map(Cursor),
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
    ///
    /// Fails with a protocol error, after handing over the page that showed it, when a page's
    /// `next` goes back, stays where it was asked or names an address already asked: the list is
    /// then not whole, so `each` is never told of a last page.
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

    /// Every album of the library: `each` gets them page by page, in order, and whether that was the last page.
    ///
    /// Fails with a protocol error, after handing over the page that showed it, when a page's
    /// `next` goes back, stays where it was asked or names an address already asked: the list is
    /// then not whole, so `each` is never told of a last page.
    pub async fn each_library_album(&self, each: impl FnMut(Vec<Album>, bool)) -> Result<()> {
        self.each_page(&library_albums_path(), convert::album, usize::MAX, each)
            .await
    }

    /// Every artist of the library: `each` gets them page by page, in order, and whether that was the last page.
    ///
    /// Fails with a protocol error, after handing over the page that showed it, when a page's
    /// `next` goes back, stays where it was asked or names an address already asked: the list is
    /// then not whole, so `each` is never told of a last page.
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

    /// Every playlist of the library: `each` gets them page by page, in order, and whether that was the last page.
    ///
    /// Fails with a protocol error, after handing over the page that showed it, when a page's
    /// `next` goes back, stays where it was asked or names an address already asked: the list is
    /// then not whole, so `each` is never told of a last page.
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

    /// The rest of a list after its first `got` items, one page after another. `got` is where
    /// the page `next` names starts.
    ///
    /// Fails, after handing over the page that showed it, when a page's `next` does not start
    /// past the page itself or names an address already asked: Apple would be asked for the
    /// same page for ever. An error and not an end, because the list is not whole and `each`
    /// must not be told it is.
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
        let mut asked = std::collections::HashSet::new();
        while let Some(path) = next.take() {
            let path = next_page_path(&path, first);
            asked.insert(path.clone());
            let Some(resp) = self
                .fetch::<types::Response<types::Resource<A>>>(&path)
                .await?
            else {
                // Gone from under it (Apple says 404 for some empty pages): done.
                each(Vec::new(), true);
                break;
            };
            let starts = got;
            got = resp
                .next
                .as_deref()
                .and_then(offset_in)
                .unwrap_or(got + resp.data.len());
            let stuck = resp.next.as_deref().and_then(|n| {
                let after = next_page_path(n, first);
                (!offset_moves_on(n, starts) || asked.contains(&after)).then_some(after)
            });
            next = resp.next.filter(|_| got < limit && stuck.is_none());
            let last = next.is_none() && stuck.is_none();
            each(resp.data.into_iter().map(convert).collect(), last);
            if let Some(after) = stuck {
                return Err(Error::Protocol(format!(
                    "{after}: the list's next page does not move on"
                )));
            }
        }
        Ok(())
    }
}

/// Where the page `next` starts.
fn offset_in(next: &str) -> Option<usize> {
    let base = url::Url::parse(API).ok()?;
    base.join(next)
        .ok()?
        .query_pairs()
        .find(|(k, _)| k == "offset")
        .and_then(|(_, v)| v.parse().ok())
}

/// Whether `next` starts past `starts`. A `next` with no offset, a cursor of another kind, is
/// not judged here: only an address asked twice shows that it goes round.
fn offset_moves_on(next: &str, starts: usize) -> bool {
    offset_in(next).is_none_or(|offset| offset > starts)
}

/// The address of the page after one: Apple's `next` for it, completed with each query
/// parameter of `first`, the first page's request, that `next` drops (Apple's keeps the path
/// and the `offset` but not `limit`, `include` or `l`), so that later pages keep the first's
/// size and inclusions.
///
/// Both are resolved against the API's address (RFC 3986 §5), so either may be a path or a
/// whole address. The result is a path and a query on the API's host, whatever host a whole
/// `next` names: `next`'s path as resolved, its query as Apple wrote it, then the parameters
/// added, in `first`'s order, written as `application/x-www-form-urlencoded`. Names are
/// compared once percent-decoded; where both have one, `next`'s value stays. With no
/// parameter at all, the path alone. A `next` or a `first` that does not resolve to a path
/// gives `next` as it is, with a `/` before it when it has none, so that the page is still
/// asked of the API's host. Never panics.
fn next_page_path(next: &str, first: &str) -> String {
    let resolved = url::Url::parse(API)
        .ok()
        .and_then(|api| Some((api.join(next).ok()?, api.join(first).ok()?)));
    let Some((next_url, first_url)) = resolved.filter(|(next, _)| !next.cannot_be_a_base()) else {
        return on_api_host(next.to_owned());
    };
    let query = query_as_written(next);
    let named: std::collections::HashSet<String> = url::form_urlencoded::parse(query.as_bytes())
        .map(|(name, _)| name.into_owned())
        .collect();
    let mut added = url::form_urlencoded::Serializer::new(String::new());
    for (name, value) in first_url.query_pairs() {
        if !named.contains(name.as_ref()) {
            added.append_pair(&name, &value);
        }
    }
    let added = added.finish();
    let mut params = query.to_owned();
    if !added.is_empty() {
        if !params.is_empty() && !params.ends_with('&') {
            params.push('&');
        }
        params.push_str(&added);
    }
    let path = on_api_host(next_url.path().to_owned());
    if url::form_urlencoded::parse(params.as_bytes())
        .next()
        .is_none()
    {
        path
    } else {
        format!("{path}?{params}")
    }
}

/// `path` with a `/` before it when it has none: put after the API's address, it stays on
/// the API's host.
fn on_api_host(path: String) -> String {
    if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    }
}

/// The query of `address` as it is written: after its first `?`, before any `#`.
fn query_as_written(address: &str) -> &str {
    let before_fragment = address.split('#').next().unwrap_or("");
    before_fragment
        .split_once('?')
        .map_or("", |(_, query)| query)
}

/// The page of `next`'s list starting at `offset`, with `first`'s parameters.
fn at_offset(next: &str, offset: usize, first: &str) -> String {
    let base = url::Url::parse(API).expect("API base");
    let Ok(mut url) = base.join(next) else {
        return next_page_path(next, first);
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
    next_page_path(&path, first)
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
        // A page that names a cursor already asked would be asked for for ever.
        let mut asked = std::collections::HashSet::new();
        loop {
            let page = self.page(&first, cursor.as_ref(), convert::track).await?;
            tracks.extend(page.items.into_iter().map(|mut t| {
                t.album_id.get_or_insert_with(|| album.clone());
                t
            }));
            match page.next {
                Some(next) if !asked.insert(next.0.clone()) => {
                    return Err(Error::Protocol(format!(
                        "{}: the album's next page does not move on",
                        next.0
                    )));
                }
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
        let path = format!(
            "/v1/me/favorites?ids%5B{}%5D={}",
            catalog_type(kind),
            apple_id(&id.id)?
        );
        let req = if on {
            self.http
                .post(self.api(&path))
                .header("Content-Length", "0")
        } else {
            self.http.delete(self.api(&path))
        };
        self.send_change(req, &path).await
    }

    /// As Apple's client adds: the item in the query, no body (Kopuz's form).
    async fn add_to_library(&self, kind: FavouriteKind, id: &ItemId) -> Result<()> {
        let path = format!(
            "/v1/me/library?ids%5B{}%5D={}&representation=ids",
            catalog_type(kind),
            apple_id(&id.id)?
        );
        let req = self
            .http
            .post(self.api(&path))
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
        let path = format!("/v1/me/library/{kind}/{}", apple_id(&library_id.id)?);
        let req = self.http.delete(self.api(&path));
        self.send_change(req, &path).await
    }

    /// As the web player edits them. Apple takes no cover from it.
    async fn edit_playlist(&self, playlist: &ItemId, name: &str, description: &str) -> Result<()> {
        let path = format!("/v1/me/library/playlists/{}", apple_id(&playlist.id)?);
        let body = serde_json::json!({
            "attributes": { "name": name, "description": description },
        });
        let req = self.http.patch(self.api(&path)).json(&body);
        self.send_change(req, &path).await
    }

    /// As the web player does it: every row of that song goes.
    async fn remove_from_playlist(&self, playlist: &ItemId, entry: &ItemId) -> Result<()> {
        let path = format!(
            "/v1/me/library/playlists/{}/tracks?ids%5Blibrary-songs%5D={}&mode=all",
            apple_id(&playlist.id)?,
            apple_id(&entry.id)?
        );
        let req = self.http.delete(self.api(&path));
        self.send_change(req, &path).await
    }

    /// As the web player deletes them.
    async fn delete_playlist(&self, playlist: &ItemId) -> Result<()> {
        let path = format!("/v1/me/library/playlists/{}", apple_id(&playlist.id)?);
        let req = self.http.delete(self.api(&path));
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
        let data = tracks
            .iter()
            .map(|t| Ok(serde_json::json!({ "id": apple_id(&t.id)?, "type": "songs" })))
            .collect::<Result<Vec<_>>>()?;
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
                    .post(self.api(path))
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
            apple_id(&playlist.id)?
        );
        let data = tracks
            .iter()
            .map(|t| Ok(serde_json::json!({ "id": apple_id(&t.id)?, "type": "songs" })))
            .collect::<Result<Vec<_>>>()?;
        let req = self
            .http
            .post(self.api(&path))
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
        let path = format!(
            "/v1/me/ratings/{}/{}",
            catalog_type(kind),
            apple_id(&id.id)?
        );
        let req = if on {
            self.http.put(self.api(&path)).json(&serde_json::json!({
                "type": "rating",
                "attributes": { "value": -1 },
            }))
        } else {
            self.http.delete(self.api(&path))
        };
        self.send_change(req, &path).await
    }

    /// Unrated, Apple says not found.
    async fn suggests_less(&self, kind: FavouriteKind, id: &ItemId) -> Result<Option<bool>> {
        #[derive(serde::Deserialize)]
        struct Rating {
            value: i8,
        }
        let path = format!(
            "/v1/me/ratings/{}/{}",
            catalog_type(kind),
            apple_id(&id.id)?
        );
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
    async fn genres(&self) -> Result<Vec<opium_core::model::Genre>> {
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
                (a.parent_id.as_deref() == Some(MUSIC_GENRE)).then_some(opium_core::model::Genre {
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

    /// About `count` of the station's next songs, each once: calls of a few songs each run
    /// side by side, within the bounds `station` sets, so that the first songs can start
    /// while the rest arrive. Fails only when no song came.
    async fn station_tracks(&self, station: &ItemId, count: usize) -> Result<Vec<Track>> {
        use futures_util::StreamExt;

        if count == 0 {
            return Ok(Vec::new());
        }
        let path = format!(
            "/v1/me/stations/next-tracks/{}",
            station::station_id(&station.id)?
        );
        let mut gathering = station::Gathering::new(count);
        let mut calls = futures_util::stream::FuturesUnordered::new();
        loop {
            let more = gathering.calls_to_send();
            for _ in 0..more {
                calls.push(self.next_station_songs(&path));
            }
            gathering.sent(more);
            if gathering.is_over() {
                break;
            }
            let Some(answer) = calls.next().await else {
                break;
            };
            gathering.answered(answer);
        }
        gathering.into_songs()
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
    /// The catalog song `id`'s lyrics: word-timed ones, else line-timed ones. An endpoint
    /// that answers 404, no TTML, or TTML that does not read gives none, and the next is
    /// asked; the first TTML of an answer is the one read.
    ///
    /// # Errors
    ///
    /// Any other failure of either request ends the search with it.
    async fn lyrics_of(&self, id: &str) -> Result<Option<Lyrics>> {
        for endpoint in ["syllable-lyrics", "lyrics"] {
            let path = format!("/v1/catalog/{}/songs/{id}/{endpoint}", self.storefront);
            let Some(answer) = self.fetch::<types::LyricsResponse>(&path).await? else {
                continue;
            };
            let ttml = answer
                .data
                .iter()
                .find_map(|entry| entry.attributes.as_ref()?.ttml.as_deref());
            if let Some(lyrics) = ttml.and_then(lyrics::parse_ttml) {
                return Ok(Some(lyrics));
            }
        }
        Ok(None)
    }

    /// One call for a station's next songs, at `path`.
    ///
    /// # Errors
    ///
    /// [`Error::NotSignedIn`] on a 401 or 403 once the developer token was renewed; another
    /// failure status, or an answer that does not read, names `path`.
    async fn next_station_songs(&self, path: &str) -> Result<Vec<Track>> {
        let request = self
            .http
            .post(self.api(path))
            .header("Media-User-Token", &self.user_token)
            .header("Content-Length", "0");
        let resp = self.send(request, path).await?;
        let status = resp.status();
        match status.as_u16() {
            401 | 403 => return Err(Error::NotSignedIn),
            _ if !status.is_success() => {
                return Err(Error::Protocol(format!("{path}: HTTP {status}")));
            }
            _ => {}
        }
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Network(format!("{path}: {e}")))?;
        station::songs_in(path, &body)
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
}

#[cfg(test)]
mod reading_tests {
    use super::*;

    #[test]
    fn ids_that_may_go_into_an_address() {
        for id in [
            "1234567890",
            "0",
            "i.AbC9-x",
            "pl.u-1b2",
            "ra.123",
            "abc.D",
            "p.-",
        ] {
            assert_eq!(apple_id(id).unwrap(), id);
        }
        for id in [
            "", ".", "i.", ".a", "abcd.x", "Pl.x", "i.a/b", "i.a?b", "i.a b", "i.a.b", "i.é",
            "12a", "1/2", "../x", "pl.u_1", "-1",
        ] {
            let error = apple_id(id).unwrap_err();
            assert!(
                matches!(&error, Error::Protocol(m) if m.contains("not an Apple Music id")),
                "{id}: {error}"
            );
        }
    }

    #[test]
    fn a_later_page_keeps_the_first_pages_parameters() {
        let first =
            "/v1/me/library/albums?limit=100&include=catalog&fields%5Balbums%5D=contentRating";
        assert_eq!(
            next_page_path("/v1/me/library/albums?offset=100", first),
            "/v1/me/library/albums?offset=100&limit=100&include=catalog&fields%5Balbums%5D=contentRating"
        );
    }

    #[test]
    fn nexts_own_parameters_and_their_spelling_stay() {
        let first = "/v1/x?limit=100&ids%5Bsongs%5D=1&l=en-US&q=a+b";
        // `ids[songs]` written either way is the same name; `next`'s value wins.
        assert_eq!(
            next_page_path("/v1/x?ids[songs]=2&offset=5&l=fr", first),
            "/v1/x?ids[songs]=2&offset=5&l=fr&limit=100&q=a+b"
        );
        assert_eq!(
            next_page_path("/v1/x?limit=7&offset=5", "/v1/x?limit=100&a=%C3%A9%20x"),
            "/v1/x?limit=7&offset=5&a=%C3%A9+x"
        );
    }

    #[test]
    fn a_whole_next_stays_on_the_apis_host() {
        assert_eq!(
            next_page_path("https://elsewhere.example/v1/x?offset=2", "/v1/x?limit=10"),
            "/v1/x?offset=2&limit=10"
        );
        assert_eq!(next_page_path("//elsewhere.example/v1/x", "/v1/x"), "/v1/x");
        assert_eq!(
            next_page_path("relative?offset=1", "/v1/a/b?l=x"),
            "/relative?offset=1&l=x"
        );
        assert_eq!(next_page_path("/v1/a/../b/./c", "/v1"), "/v1/b/c");
    }

    #[test]
    fn no_parameter_is_the_path_alone() {
        assert_eq!(next_page_path("/v1/x", "/v1/x"), "/v1/x");
        assert_eq!(next_page_path("/v1/x?", "/v1/x?"), "/v1/x");
        assert_eq!(next_page_path("/v1/x?#f", "/v1/x"), "/v1/x");
        assert_eq!(next_page_path("/v1/x?&", "/v1/y?a=1"), "/v1/x?&a=1");
    }

    #[test]
    fn what_does_not_resolve_is_next_as_it_is_on_the_apis_host() {
        assert_eq!(
            next_page_path("http://[::1", "/v1/x?limit=1"),
            "/http://[::1"
        );
        assert_eq!(
            next_page_path("/v1/x?offset=1", "http://[::1"),
            "/v1/x?offset=1"
        );
        // A scheme of its own and no path to keep: still asked of the API.
        assert_eq!(
            next_page_path("evil.example:99", "/v1/x"),
            "/evil.example:99"
        );
        assert_eq!(next_page_path("", "/v1/x?limit=1"), "/?limit=1");
    }

    #[test]
    fn at_offset_builds_on_it() {
        assert_eq!(
            at_offset(
                "/v1/me/library/songs?offset=100",
                300,
                "/v1/me/library/songs?limit=100&relate=albums"
            ),
            "/v1/me/library/songs?offset=300&limit=100&relate=albums"
        );
    }

    #[test]
    fn a_token_expiring_soon_or_gone_is_renewed_before_use() {
        use crate::test_support::{fake_token, seconds_from_now};
        let now = std::time::SystemTime::now();
        assert!(expires_soon(&fake_token(seconds_from_now(60)), now));
        assert!(expires_soon(&fake_token(1), now));
        assert!(!expires_soon(&fake_token(seconds_from_now(3600)), now));
        assert!(!expires_soon("not.a.token", now));
    }
}

#[cfg(test)]
mod http_tests {
    use super::*;
    use crate::test_support::{
        Reply, Request, TestServer, USER_TOKEN, apple_music, apple_server, fake_token, json_reply,
        lasting_token, reply, seconds_from_now, test_client, web_player_and_storefront,
    };
    use serde_json::json;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const PROFILE: &str = "/v1/me/social-profile";

    fn profile() -> Reply {
        json_reply(&json!({ "data": [{ "id": "me", "attributes": { "name": "Made Up" } }] }))
    }

    /// A server whose web player gives `tokens` in turn, the last for ever after, and whose
    /// API answers with `api`.
    fn rotating(
        tokens: Vec<String>,
        api: impl Fn(&Request) -> Reply + Send + Sync + 'static,
    ) -> TestServer {
        let reads = AtomicUsize::new(0);
        TestServer::start(move |request| {
            if request.path == crate::test_support::SCRIPT {
                let n = reads.fetch_add(1, Ordering::SeqCst).min(tokens.len() - 1);
                return reply(200, &format!("t=\"{}\"", tokens[n]));
            }
            web_player_and_storefront(request, &tokens[0]).unwrap_or_else(|| api(request))
        })
    }

    #[tokio::test]
    async fn a_token_about_to_expire_is_read_again_before_a_request() {
        let expiring = fake_token(seconds_from_now(60));
        let fresh = lasting_token();
        let server = rotating(vec![expiring.clone(), fresh.clone()], |_| profile());
        let am = apple_music(&server).await;
        assert_eq!(
            am.profile().await.unwrap().unwrap().name.as_deref(),
            Some("Made Up")
        );
        assert_eq!(
            am.profile().await.unwrap().unwrap().name.as_deref(),
            Some("Made Up")
        );
        let bearers: Vec<String> = server
            .requests_to(PROFILE)
            .iter()
            .filter_map(|r| r.bearer().map(str::to_owned))
            .collect();
        assert_eq!(bearers, [fresh.clone(), fresh]);
        // Read twice in all: at the connection, then before its first request (the
        // storefront's), and not again.
        assert_eq!(server.requests_to(crate::test_support::SCRIPT).len(), 2);
    }

    /// A server whose web player serves `token` at the first reading, then answers each later
    /// page with `later`, and whose API answers with `api`.
    fn web_player_then(
        token: String,
        later: impl Fn() -> Reply + Send + Sync + 'static,
        api: impl Fn(&Request) -> Reply + Send + Sync + 'static,
    ) -> TestServer {
        let pages = AtomicUsize::new(0);
        TestServer::start(move |request| {
            if request.host_name() == "music.apple.com"
                && request.path == "/"
                && pages.fetch_add(1, Ordering::SeqCst) > 0
            {
                return later();
            }
            web_player_and_storefront(request, &token).unwrap_or_else(|| api(request))
        })
    }

    /// How many times `server`'s web player page was read.
    fn page_readings(server: &TestServer) -> usize {
        server
            .requests()
            .iter()
            .filter(|r| r.host_name() == "music.apple.com" && r.path == "/")
            .count()
    }

    /// Requests sent one after another, then side by side, while the token is about to
    /// expire and `server`'s web player cannot renew it: each is sent with the old token, and
    /// the web player is read once, at the first. Once five minutes have passed, it is read
    /// once more.
    async fn read_once_in_five_minutes(server: &TestServer) {
        let am = apple_music(server).await;
        // At the connection, then before the storefront's request.
        assert_eq!(page_readings(server), 2);
        for _ in 0..4 {
            am.profile().await.unwrap();
        }
        let side_by_side: Vec<_> = (0..6).map(|_| am.profile()).collect();
        for answer in futures_util::future::join_all(side_by_side).await {
            answer.unwrap();
        }
        assert_eq!(page_readings(server), 2);
        assert_eq!(server.requests_to(PROFILE).len(), 10);
        am.as_if_the_renewals_were_long_ago();
        am.profile().await.unwrap();
        am.profile().await.unwrap();
        assert_eq!(page_readings(server), 3);
    }

    #[tokio::test]
    async fn a_web_player_failing_by_status_is_read_once_in_five_minutes_before_expiry() {
        let server = web_player_then(
            fake_token(seconds_from_now(120)),
            || reply(503, ""),
            |_| profile(),
        );
        read_once_in_five_minutes(&server).await;
    }

    #[tokio::test]
    async fn a_web_player_out_of_reach_is_read_once_in_five_minutes_before_expiry() {
        let server = web_player_then(fake_token(seconds_from_now(120)), Reply::hang_up, |_| {
            profile()
        });
        read_once_in_five_minutes(&server).await;
    }

    #[tokio::test]
    async fn a_web_player_serving_the_same_token_is_read_once_in_five_minutes_before_expiry() {
        let expiring = fake_token(seconds_from_now(120));
        let server = rotating(vec![expiring], |_| profile());
        read_once_in_five_minutes(&server).await;
    }

    /// Across the token's `exp`: Apple's web player serves the expiring token until then, and
    /// a new one after; its API takes the old token until then, and refuses it after. The
    /// reading before `exp`, which got the old token, must not hold back the reading the
    /// first refusal after `exp` asks for. The account's clock and the server cross `exp`
    /// together, when the test says, so that nothing waits for the wall clock.
    #[tokio::test]
    async fn a_refusal_after_expiry_reads_the_token_at_once_whatever_was_read_before() {
        use std::sync::atomic::AtomicBool;
        use std::time::Duration;
        // Five minutes ahead: within the ten minutes, so read again before it.
        let to_exp = Duration::from_secs(300);
        let expiring = fake_token(seconds_from_now(to_exp.as_secs()));
        let fresh = lasting_token();
        let past_exp = Arc::new(AtomicBool::new(false));
        let (served, refused, new, expired) = (
            expiring.clone(),
            expiring.clone(),
            fresh.clone(),
            past_exp.clone(),
        );
        let server = TestServer::start(move |request| {
            let expired = expired.load(Ordering::SeqCst);
            let token = if expired { &new } else { &served };
            web_player_and_storefront(request, token).unwrap_or_else(|| {
                if expired && request.bearer() == Some(refused.as_str()) {
                    reply(401, "")
                } else {
                    profile()
                }
            })
        });
        let am = apple_music(&server).await;
        am.profile().await.unwrap();
        // At the connection, then before the storefront's request: the old token both times.
        assert_eq!(page_readings(&server), 2);
        am.move_clock_on(to_exp + Duration::from_secs(1));
        past_exp.store(true, Ordering::SeqCst);
        am.profile().await.unwrap();
        assert_eq!(page_readings(&server), 3);
        let bearers: Vec<Option<String>> = server
            .requests_to(PROFILE)
            .iter()
            .map(|r| r.bearer().map(str::to_owned))
            .collect();
        assert_eq!(
            bearers,
            [Some(expiring.clone()), Some(expiring), Some(fresh)]
        );
    }

    #[tokio::test]
    async fn a_token_whose_expiry_does_not_read_is_kept_until_refused() {
        let header = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            r#"{"alg":"ES256"}"#,
        );
        let payload = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            r#"{"iss":"MADEUP"}"#,
        );
        let no_expiry = format!("{header}.{payload}.c2ln");
        let server = rotating(vec![no_expiry.clone(), lasting_token()], |_| profile());
        let am = apple_music(&server).await;
        am.profile().await.unwrap();
        assert_eq!(
            server.requests_to(PROFILE)[0].bearer(),
            Some(no_expiry.as_str())
        );
        assert_eq!(server.requests_to(crate::test_support::SCRIPT).len(), 1);
    }

    #[tokio::test]
    async fn a_refused_token_is_read_again_and_the_request_sent_once_more() {
        let (old, new) = (
            lasting_token(),
            fake_token(seconds_from_now(80 * 24 * 3600)),
        );
        let accepted = new.clone();
        let server = rotating(vec![old.clone(), new.clone()], move |request| {
            if request.bearer() == Some(accepted.as_str()) {
                profile()
            } else {
                reply(401, "")
            }
        });
        let am = apple_music(&server).await;
        am.profile().await.unwrap();
        let bearers: Vec<Option<String>> = server
            .requests_to(PROFILE)
            .iter()
            .map(|r| r.bearer().map(str::to_owned))
            .collect();
        assert_eq!(bearers, [Some(old), Some(new)]);
    }

    #[tokio::test]
    async fn a_request_refused_again_is_sent_no_third_time() {
        let server = rotating(
            vec![lasting_token(), fake_token(seconds_from_now(9_000_000))],
            |_| reply(401, ""),
        );
        let am = apple_music(&server).await;
        assert!(matches!(am.profile().await, Err(Error::NotSignedIn)));
        assert_eq!(server.requests_to(PROFILE).len(), 2);
    }

    /// Refusals one after another, from `server`'s API, which turns every request down, while
    /// its web player answers without a new token: the first refusal reads the web player,
    /// the others within five minutes of it do not, and one after five minutes reads it again.
    async fn a_refusal_reads_once_in_five_minutes(server: &TestServer) {
        use std::time::Duration;
        let refused = |answer: Result<Option<Profile>>| {
            assert!(matches!(answer, Err(Error::NotSignedIn)));
        };
        let am = apple_music(server).await;
        // At the connection only: the token lasts.
        assert_eq!(page_readings(server), 1);
        for _ in 0..4 {
            refused(am.profile().await);
        }
        assert_eq!(page_readings(server), 2);
        // A second short of five minutes after the reading, then a second past them.
        am.move_clock_on(RENEW_EVERY - Duration::from_secs(1));
        refused(am.profile().await);
        assert_eq!(page_readings(server), 2);
        am.move_clock_on(Duration::from_secs(2));
        refused(am.profile().await);
        refused(am.profile().await);
        assert_eq!(page_readings(server), 3);
        // Each refused request sent once: the web player gave nothing new to send it with.
        assert_eq!(server.requests_to(PROFILE).len(), 7);
    }

    #[tokio::test]
    async fn refusals_read_a_web_player_serving_the_same_token_once_in_five_minutes() {
        let server = rotating(vec![lasting_token()], |_| reply(401, ""));
        a_refusal_reads_once_in_five_minutes(&server).await;
    }

    #[tokio::test]
    async fn refusals_read_a_web_player_failing_by_status_once_in_five_minutes() {
        let server = web_player_then(lasting_token(), || reply(503, ""), |_| reply(401, ""));
        a_refusal_reads_once_in_five_minutes(&server).await;
    }

    /// A reading before `exp` and a 401's renewal, asked side by side: the second waits for
    /// the first, so that the web player is never read twice at once. Its page is held long
    /// enough for the second to show, were it not waiting.
    #[tokio::test]
    async fn a_reading_before_expiry_and_a_refusals_reading_run_one_at_a_time() {
        use std::time::Duration;
        let expiring = fake_token(seconds_from_now(120));
        let reading = Arc::new(AtomicUsize::new(0));
        let most_at_once = Arc::new(AtomicUsize::new(0));
        let (in_flight, peak) = (reading.clone(), most_at_once.clone());
        let server = TestServer::start(move |request| {
            if request.host_name() == "music.apple.com" {
                let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                if request.path == "/" {
                    std::thread::sleep(Duration::from_millis(250));
                }
                in_flight.fetch_sub(1, Ordering::SeqCst);
            }
            web_player_and_storefront(request, &expiring).unwrap_or_else(|| {
                if request.path.starts_with("/v1/me/account") {
                    reply(401, "")
                } else {
                    profile()
                }
            })
        });
        let am = apple_music(&server).await;
        // At the connection, then before the storefront's request.
        assert_eq!(page_readings(&server), 2);
        am.as_if_the_renewals_were_long_ago();
        // `join` polls the profile first: it begins the reading before `exp`, which the
        // subscription's request then finds begun, so it is sent with the old token at once,
        // refused, and its renewal asks for a reading while the first is under way.
        let (profile, subscribed) = futures_util::future::join(am.profile(), am.subscribed()).await;
        profile.unwrap();
        assert!(matches!(subscribed, Err(Error::NotSignedIn)));
        // One reading of each kind, both made.
        assert_eq!(page_readings(&server), 4);
        assert_eq!(most_at_once.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_short_wait_asked_for_is_waited_and_the_request_sent_again() {
        let calls = AtomicUsize::new(0);
        let server = apple_server(move |_| {
            if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                reply(429, "").header("Retry-After", "0")
            } else {
                profile()
            }
        });
        let am = apple_music(&server).await;
        assert!(am.profile().await.unwrap().is_some());
        assert_eq!(server.requests_to(PROFILE).len(), 2);
    }

    #[tokio::test]
    async fn a_long_wait_asked_for_is_not() {
        let server = apple_server(|_| reply(429, "").header("Retry-After", "11"));
        let am = apple_music(&server).await;
        let error = am.profile().await.unwrap_err();
        assert!(error.to_string().contains("429"), "{error}");
        assert_eq!(server.requests_to(PROFILE).len(), 1);
    }

    #[tokio::test]
    async fn a_redirect_off_apples_hosts_is_not_followed() {
        let server = apple_server(|request| {
            if request.host_name() == "elsewhere.example" {
                return profile();
            }
            let port = request.host.rsplit(':').next().unwrap_or("").to_owned();
            reply(302, "").header(
                "Location",
                &format!("http://elsewhere.example:{port}{PROFILE}"),
            )
        });
        let am = apple_music(&server).await;
        let error = am.profile().await.unwrap_err();
        assert!(error.to_string().contains("302"), "{error}");
        assert!(
            server
                .requests()
                .iter()
                .all(|r| r.host_name() != "elsewhere.example")
        );
    }

    fn upload(id: &str) -> serde_json::Value {
        json!({ "id": id, "type": "library-songs", "attributes": { "name": "Made-up upload" } })
    }

    #[tokio::test]
    async fn later_pages_are_asked_with_the_first_pages_parameters() {
        let server = apple_server(|request| {
            let next = if request.path.contains("offset=100") {
                json!(null)
            } else {
                json!("/v1/me/library/songs?offset=100")
            };
            json_reply(&json!({ "data": [upload("i.A")], "next": next }))
        });
        let am = apple_music(&server).await;
        let first = am.library_tracks(None).await.unwrap();
        let cursor = first.next.expect("a next page");
        assert_eq!(
            cursor.0,
            "/v1/me/library/songs?offset=100&limit=100&relate=albums"
        );
        let second = am.library_tracks(Some(&cursor)).await.unwrap();
        assert!(second.next.is_none());
        let paths: Vec<String> = server
            .requests_to("/v1/me/library/songs")
            .into_iter()
            .map(|r| r.path)
            .collect();
        assert_eq!(
            paths,
            [
                "/v1/me/library/songs?limit=100&relate=albums",
                "/v1/me/library/songs?offset=100&limit=100&relate=albums"
            ]
        );
    }

    #[tokio::test]
    async fn pages_asked_at_once_keep_them_too() {
        let server = apple_server(|_| {
            json_reply(&json!({
                "data": [{ "id": "l.x", "type": "library-albums", "attributes": { "name": "Made-up album" } }],
                "next": "/v1/me/library/albums?offset=1",
                "meta": { "total": 3 },
            }))
        });
        let am = apple_music(&server).await;
        let mut albums = 0;
        am.each_library_album(|page, _| albums += page.len())
            .await
            .unwrap();
        assert_eq!(albums, 3);
        let mut later: Vec<String> = server
            .requests_to("/v1/me/library/albums")
            .into_iter()
            .skip(1)
            .map(|r| r.path)
            .collect();
        later.sort();
        let rest = "&limit=100&include=catalog&fields%5Balbums%5D=contentRating";
        assert_eq!(
            later,
            [
                format!("/v1/me/library/albums?offset=1{rest}"),
                format!("/v1/me/library/albums?offset=2{rest}")
            ]
        );
    }

    // Stage 1 audit W2: a `next` that never moves on must not be asked for for ever.
    /// A list whose `next` never moves on: asked a few times at most, not for ever.
    #[tokio::test]
    async fn a_next_page_that_never_moves_on_is_not_asked_for_ever() {
        let server = apple_server(|_| {
            json_reply(&json!({
                "data": [{ "id": "l.x", "type": "library-albums", "attributes": { "name": "Made-up album" } }],
                "next": "/v1/me/library/albums?offset=1",
            }))
        });
        let am = apple_music(&server).await;
        let ended = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            am.each_library_album(|_, _| {}),
        )
        .await;
        let asked = server.requests_to("/v1/me/library/albums").len();
        assert!(ended.is_ok(), "still asking after {asked} requests");
        assert!(asked <= 3, "{asked} requests for a list of one page");
    }

    /// A list whose `next` goes back to an earlier offset ends, as an error that keeps the
    /// pages already handed over.
    #[tokio::test]
    async fn a_next_page_that_goes_back_ends_the_list_with_an_error() {
        let server = apple_server(|request| {
            let next = if request.path.contains("offset=5") {
                "/v1/me/library/albums?offset=2"
            } else {
                "/v1/me/library/albums?offset=5"
            };
            json_reply(&json!({
                "data": [{ "id": "l.x", "type": "library-albums", "attributes": { "name": "Made-up album" } }],
                "next": next,
            }))
        });
        let am = apple_music(&server).await;
        let mut pages = 0;
        let ended = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            am.each_library_album(|_, last| {
                assert!(!last, "a list cut short is never the whole");
                pages += 1;
            }),
        )
        .await;
        let asked = server.requests_to("/v1/me/library/albums").len();
        let ended = ended.unwrap_or_else(|_| panic!("still asking after {asked} requests"));
        let error = ended.unwrap_err();
        assert!(error.to_string().contains("offset=2"), "{error}");
        assert_eq!(pages, asked, "every page read was handed over");
        assert!(asked <= 3, "{asked} requests");
    }

    /// A `next` with no offset that names an address already asked ends the list too.
    #[tokio::test]
    async fn a_next_address_already_asked_ends_the_list_with_an_error() {
        let server = apple_server(|_| {
            json_reply(&json!({
                "data": [{ "id": "l.x", "type": "library-albums", "attributes": { "name": "Made-up album" } }],
                "next": "/v1/me/library/albums?cursor=same",
            }))
        });
        let am = apple_music(&server).await;
        let ended = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            am.each_library_album(|_, _| {}),
        )
        .await;
        let asked = server.requests_to("/v1/me/library/albums").len();
        let ended = ended.unwrap_or_else(|_| panic!("still asking after {asked} requests"));
        assert!(ended.is_err());
        assert!(asked <= 3, "{asked} requests");
    }

    /// A list paged by a cursor that is not an offset goes on past a page that comes empty
    /// (every item of it unreadable), to the address that page names.
    #[tokio::test]
    async fn a_cursor_list_goes_on_past_a_page_of_unreadable_items() {
        let server = apple_server(|request| {
            let album = json!({ "id": "l.x", "type": "library-albums", "attributes": { "name": "Made-up album" } });
            let (data, next) = if request.path.contains("cursor=b") {
                (
                    json!([{ "id": "l.bad", "type": "library-albums", "attributes": 7 }]),
                    Some("/v1/me/library/albums?cursor=c"),
                )
            } else if request.path.contains("cursor=c") {
                (json!([album]), None)
            } else {
                (json!([album]), Some("/v1/me/library/albums?cursor=b"))
            };
            json_reply(&json!({ "data": data, "next": next }))
        });
        let am = apple_music(&server).await;
        let (mut albums, mut whole) = (0, false);
        let ended = am
            .each_library_album(|page, last| {
                albums += page.len();
                whole |= last;
            })
            .await;
        assert!(ended.is_ok(), "{ended:?}");
        assert_eq!((albums, whole), (2, true));
    }

    /// A `next` that keeps the offset it was asked at, whatever else in it changes with each
    /// answer, does not move on: only the offset can say so, as no address repeats.
    #[tokio::test]
    async fn a_next_page_at_the_same_offset_under_new_addresses_ends_the_list() {
        let answers = AtomicUsize::new(0);
        let server = apple_server(move |_| {
            let n = answers.fetch_add(1, Ordering::SeqCst);
            json_reply(&json!({
                "data": [{ "id": "l.x", "type": "library-albums", "attributes": { "name": "Made-up album" } }],
                "next": format!("/v1/me/library/albums?offset=1&session={n}"),
            }))
        });
        let am = apple_music(&server).await;
        let ended = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            am.each_library_album(|_, _| {}),
        )
        .await;
        let asked = server.requests_to("/v1/me/library/albums").len();
        let ended = ended.unwrap_or_else(|_| panic!("still asking after {asked} requests"));
        assert!(ended.is_err());
        assert!(asked <= 3, "{asked} requests");
    }

    /// A function that hands back a cursor hands back none when it is the address it was just
    /// asked for, or an offset it was not moved on to.
    #[tokio::test]
    async fn a_cursor_that_does_not_move_on_is_not_handed_back() {
        let server = apple_server(|request| {
            let next = if request.path.contains("offset=5") {
                "/v1/me/library/albums?offset=2"
            } else if request.path.contains("offset=7") {
                "/v1/me/library/albums?offset=7"
            } else {
                "/v1/me/library/albums?offset=5"
            };
            json_reply(&json!({
                "data": [{ "id": "l.x", "type": "library-albums", "attributes": { "name": "Made-up album" } }],
                "next": next,
            }))
        });
        let am = apple_music(&server).await;
        let first = am.library_albums(None).await.unwrap();
        let five = first.next.expect("from the first page on to offset 5");
        // Back to 2 from 5.
        assert!(am.library_albums(Some(&five)).await.unwrap().next.is_none());
        // 7 asked, 7 named.
        let seven = Cursor("/v1/me/library/albums?offset=7&limit=100".into());
        assert!(
            am.library_albums(Some(&seven))
                .await
                .unwrap()
                .next
                .is_none()
        );
    }

    /// An album's songs end at a page whose next page is the one just asked for: the songs
    /// read are given.
    #[tokio::test]
    async fn an_albums_songs_end_at_a_next_page_that_does_not_move_on() {
        let server = apple_server(|_| {
            json_reply(&json!({
                "data": [upload("i.A")],
                "next": "/v1/me/library/albums/l.1/tracks?offset=100",
            }))
        });
        let am = apple_music(&server).await;
        let album = ItemId::new(ProviderKind::AppleMusic, "l.1");
        let ended =
            tokio::time::timeout(std::time::Duration::from_secs(5), am.album_tracks(&album)).await;
        let asked = server.requests_to("/v1/me/library/albums/l.1/tracks").len();
        let ended = ended.unwrap_or_else(|_| panic!("still asking after {asked} requests"));
        assert_eq!(ended.unwrap().len(), 2);
        assert!(asked <= 3, "{asked} requests");
    }

    /// An album's songs, read by cursors that go round (a, b, a), fail rather than go on.
    #[tokio::test]
    async fn an_albums_songs_stop_at_a_cursor_that_comes_back() {
        let server = apple_server(|request| {
            let next = if request.path.contains("cursor=a") {
                "/v1/me/library/albums/l.1/tracks?cursor=b"
            } else {
                "/v1/me/library/albums/l.1/tracks?cursor=a"
            };
            json_reply(&json!({ "data": [upload("i.A")], "next": next }))
        });
        let am = apple_music(&server).await;
        let album = ItemId::new(ProviderKind::AppleMusic, "l.1");
        let ended =
            tokio::time::timeout(std::time::Duration::from_secs(5), am.album_tracks(&album)).await;
        let asked = server.requests_to("/v1/me/library/albums/l.1/tracks").len();
        let ended = ended.unwrap_or_else(|_| panic!("still asking after {asked} requests"));
        assert!(ended.is_err());
        assert!(asked <= 4, "{asked} requests");
    }

    fn station() -> ItemId {
        ItemId::new(ProviderKind::AppleMusic, "ra.1234")
    }

    const NEXT_TRACKS: &str = "/v1/me/stations/next-tracks/ra.1234";

    fn songs(ids: &[String]) -> Reply {
        let data: Vec<serde_json::Value> = ids
            .iter()
            .map(|id| json!({ "id": id, "type": "songs", "attributes": { "name": format!("Made-up {id}") } }))
            .collect();
        json_reply(&json!({ "data": data }))
    }

    /// A station whose `n`th call (from 0) answers `answer(n)`, each held a little so that
    /// calls overlap.
    fn station_server(answer: impl Fn(usize) -> Reply + Send + Sync + 'static) -> TestServer {
        let calls = Arc::new(AtomicUsize::new(0));
        apple_server(move |request| {
            if request.path != NEXT_TRACKS {
                return reply(404, "");
            }
            answer(calls.fetch_add(1, Ordering::SeqCst)).held(std::time::Duration::from_millis(30))
        })
    }

    #[tokio::test]
    async fn a_station_is_asked_side_by_side_within_its_bounds() {
        // Each call brings two songs, one of them the first call's again. The first is held
        // until a second comes (five seconds at most): the calls are sent side by side.
        let arrived = Arc::new(AtomicUsize::new(0));
        let server = station_server(move |n| {
            arrived.fetch_add(1, Ordering::SeqCst);
            let held = std::time::Instant::now();
            while n == 0
                && arrived.load(Ordering::SeqCst) < 2
                && held.elapsed() < std::time::Duration::from_secs(5)
            {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            songs(&["0".into(), format!("{}", n + 1)])
        });
        let am = apple_music(&server).await;
        let tracks = am.station_tracks(&station(), 20).await.unwrap();
        assert_eq!(tracks.len(), 20);
        let ids: std::collections::HashSet<&str> =
            tracks.iter().map(|t| t.id.id.as_str()).collect();
        assert_eq!(ids.len(), 20, "each song once");
        let calls = server.requests_to(NEXT_TRACKS);
        assert!(calls.len() <= station::MOST_CALLS, "{} calls", calls.len());
        let peak = server.peak_in_flight();
        assert!(
            (2..=station::MOST_IN_FLIGHT).contains(&peak),
            "{peak} at once"
        );
        for call in &calls {
            assert_eq!(call.method, "POST");
            assert_eq!(call.header("content-length"), Some("0"));
            assert_eq!(call.header("media-user-token"), Some(USER_TOKEN));
            assert!(call.body.is_empty());
        }
    }

    #[tokio::test]
    async fn a_station_gives_no_more_than_asked_nor_than_its_budget_of_calls() {
        let server =
            station_server(|n| songs(&[format!("a{n}"), format!("b{n}"), format!("c{n}")]));
        let am = apple_music(&server).await;
        assert_eq!(am.station_tracks(&station(), 25).await.unwrap().len(), 25);

        let server = station_server(|n| songs(&[format!("a{n}"), format!("b{n}")]));
        let am = apple_music(&server).await;
        let tracks = am.station_tracks(&station(), 1000).await.unwrap();
        assert_eq!(server.requests_to(NEXT_TRACKS).len(), station::MOST_CALLS);
        assert_eq!(tracks.len(), 2 * station::MOST_CALLS);
        assert!(server.peak_in_flight() <= station::MOST_IN_FLIGHT);
    }

    #[tokio::test]
    async fn a_station_repeating_itself_ends_before_its_budget() {
        let server = station_server(|_| songs(&["7".into(), "8".into()]));
        let am = apple_music(&server).await;
        let tracks = am.station_tracks(&station(), 20).await.unwrap();
        let ids: Vec<&str> = tracks.iter().map(|t| t.id.id.as_str()).collect();
        assert_eq!(ids, ["7", "8"]);
        assert!(server.requests_to(NEXT_TRACKS).len() < station::MOST_CALLS);
    }

    #[tokio::test]
    async fn a_station_offering_nothing_fails_without_spending_its_budget() {
        let server = station_server(|_| songs(&[]));
        let am = apple_music(&server).await;
        let error = am.station_tracks(&station(), 10).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "unexpected response: the station offered no songs"
        );
        assert!(server.requests_to(NEXT_TRACKS).len() < station::MOST_CALLS);
    }

    #[tokio::test]
    async fn a_station_failing_says_how() {
        for (status, body, expected) in [
            (403, "", "not signed in".to_owned()),
            (500, "", format!("{NEXT_TRACKS}: HTTP 500")),
            (
                200,
                "{\"data\":",
                format!("unexpected response: {NEXT_TRACKS}: "),
            ),
        ] {
            let server = station_server(move |_| reply(status, body));
            let am = apple_music(&server).await;
            let error = am.station_tracks(&station(), 10).await.unwrap_err();
            assert!(error.to_string().contains(&expected), "{status}: {error}");
            assert!(server.requests_to(NEXT_TRACKS).len() < station::MOST_CALLS);
        }
        // A failure after songs came is no failure.
        let server = station_server(|n| {
            if n == 0 {
                songs(&["1".into()])
            } else {
                reply(500, "")
            }
        });
        let am = apple_music(&server).await;
        assert_eq!(am.station_tracks(&station(), 10).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_playlists_station_is_asked_by_its_id() {
        const PLAYLISTS_STATION: &str = "/v1/me/stations/next-tracks/ra.cp-pl.u-AbC9";
        let server = apple_server(|request| {
            if request.path == PLAYLISTS_STATION {
                songs(&["1".into(), "2".into()])
            } else {
                reply(404, "")
            }
        });
        let am = apple_music(&server).await;
        let station = ItemId::new(ProviderKind::AppleMusic, "ra.cp-pl.u-AbC9");
        assert_eq!(am.station_tracks(&station, 2).await.unwrap().len(), 2);
        assert_eq!(server.requests_to(PLAYLISTS_STATION).len(), 1);
    }

    #[tokio::test]
    async fn no_song_asked_or_a_station_id_that_is_not_one_segment_sends_nothing() {
        let server = station_server(|_| songs(&["1".into()]));
        let am = apple_music(&server).await;
        assert!(am.station_tracks(&station(), 0).await.unwrap().is_empty());
        for id in ["ra.1/../../me/library", "ra 1", "", "ra.1?x=y", ".", ".."] {
            let station = ItemId::new(ProviderKind::AppleMusic, id);
            let error = am.station_tracks(&station, 10).await.unwrap_err();
            assert!(
                error.to_string().contains("not a station id"),
                "{id}: {error}"
            );
        }
        assert!(
            server
                .requests()
                .iter()
                .all(|r| !r.path.contains("stations"))
        );
    }

    const TTML: &str = r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><div><p begin="1.0" end="2.5">A made-up line</p></div></body></tt>"#;

    fn lyrics_reply(ttml: Option<&str>) -> Reply {
        json_reply(
            &json!({ "data": [{ "id": "1", "type": "lyrics", "attributes": { "ttml": ttml } }] }),
        )
    }

    /// The lyrics of `1234` with its two endpoints answering `word_timed` and `line_timed`,
    /// and which were asked.
    async fn lyrics_with(
        word_timed: Reply,
        line_timed: Reply,
    ) -> (Result<Option<Lyrics>>, Vec<String>) {
        let answers = std::sync::Mutex::new((Some(word_timed), Some(line_timed)));
        let server = apple_server(move |request| {
            let mut answers = answers.lock().unwrap();
            let answer = match request.path.as_str() {
                "/v1/catalog/zz/songs/1234/syllable-lyrics" => answers.0.take(),
                "/v1/catalog/zz/songs/1234/lyrics" => answers.1.take(),
                _ => None,
            };
            answer.unwrap_or_else(|| reply(404, ""))
        });
        let am = apple_music(&server).await;
        let lyrics = am.lyrics_of("1234").await;
        let asked = server
            .requests()
            .into_iter()
            .map(|r| r.path)
            .filter(|p| p.contains("lyrics"))
            .collect();
        (lyrics, asked)
    }

    const WORD: &str = "/v1/catalog/zz/songs/1234/syllable-lyrics";
    const LINE: &str = "/v1/catalog/zz/songs/1234/lyrics";

    #[tokio::test]
    async fn word_timed_lyrics_come_first() {
        let (lyrics, asked) = lyrics_with(lyrics_reply(Some(TTML)), lyrics_reply(Some(TTML))).await;
        assert_eq!(
            lyrics.unwrap().unwrap().lines.last().unwrap().text,
            "A made-up line"
        );
        assert_eq!(asked, [WORD]);
    }

    #[tokio::test]
    async fn line_timed_ones_are_asked_only_when_the_first_gives_none() {
        let none = [
            reply(404, ""),
            json_reply(&json!({ "data": [] })),
            lyrics_reply(None),
            lyrics_reply(Some("<tt><body></body></tt>")),
        ];
        for word_timed in none {
            let (lyrics, asked) = lyrics_with(word_timed, lyrics_reply(Some(TTML))).await;
            assert!(lyrics.unwrap().is_some());
            assert_eq!(asked, [WORD, LINE]);
        }
        let (lyrics, asked) = lyrics_with(reply(404, ""), reply(404, "")).await;
        assert!(lyrics.unwrap().is_none());
        assert_eq!(asked, [WORD, LINE]);
    }

    #[tokio::test]
    async fn any_other_failure_ends_the_search() {
        let (lyrics, asked) = lyrics_with(reply(500, ""), lyrics_reply(Some(TTML))).await;
        assert!(lyrics.unwrap_err().to_string().contains("HTTP 500"));
        assert_eq!(asked, [WORD]);
        let (lyrics, asked) = lyrics_with(reply(200, "not json"), lyrics_reply(Some(TTML))).await;
        assert!(matches!(lyrics, Err(Error::Protocol(_))));
        assert_eq!(asked, [WORD]);
    }

    #[tokio::test]
    async fn an_upload_has_no_lyrics_and_asks_nothing() {
        let server = apple_server(|_| lyrics_reply(Some(TTML)));
        let am = apple_music(&server).await;
        let upload = ItemId::new(ProviderKind::AppleMusic, "i.Mine");
        assert!(am.lyrics(&upload).await.unwrap().is_none());
        assert!(server.requests().iter().all(|r| !r.path.contains("lyrics")));
    }

    #[tokio::test]
    async fn the_client_reaches_only_the_local_server() {
        // A sanity check of the seam itself: Apple's names resolve to the local server.
        let server = TestServer::start(|request| reply(200, &request.host));
        let body = test_client(server.port())
            .get(format!("{}/x", server.hosts().api))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert_eq!(body, format!("amp-api.music.apple.com:{}", server.port()));
    }
}
