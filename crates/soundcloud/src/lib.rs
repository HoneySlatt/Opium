//! SoundCloud, through the web player's own API (`api-v2`): the official one
//! takes no new apps. Requests carry the web player's client id, read from
//! soundcloud.com (see [`client_id`]), and the user's token from the
//! `oauth_token` cookie of a browser sign-in (see [`signin`]). That token now
//! lasts a day or so: it is renewed the way the web does, with the
//! `oauth_refresh_token` cookie, and the new pair handed back to be kept.

mod client_id;
mod convert;
mod resume;
pub mod signin;
mod stream;
mod types;

use std::sync::{Mutex, RwLock};

use async_trait::async_trait;
use music_core::model::{
    Album, Artist, ArtistDetail, Collection, Cursor, FavouriteKind, ItemId, Page, Playlist,
    ProviderKind, SearchResults, SongLinks, Track,
};
use music_core::{Error, Provider, Result};
use serde::de::DeserializeOwned;

pub use stream::{Audio, Playable, Reader};

const API: &str = "https://api-v2.soundcloud.com";
pub(crate) const WEB: &str = "https://soundcloud.com";
/// Where tokens are issued and renewed.
const SECURE: &str = "https://secure.soundcloud.com";
/// As the web player says it is: some of the API turns other clients away.
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
/// Items a page of a list asks for.
const PAGE: u32 = 200;
/// The most of an artist's tracks their page lists.
const UPLOADS_MAX: usize = 1000;
/// Tracks `tracks?ids=` takes at once.
const IDS_AT_ONCE: usize = 50;
/// Tracks a playlist holds, as the web player allows.
const PLAYLIST_MAX: usize = 500;

/// The client id is read again at most this often.
const RESCRAPE_EVERY: std::time::Duration = std::time::Duration::from_secs(600);

/// The longest wait SoundCloud may ask for before a request is tried again.
const MAX_RETRY_AFTER: u64 = 10;

fn slow_down() -> Error {
    Error::Protocol("SoundCloud asks to slow down: try again in a few minutes".into())
}

/// SoundCloud's bot guard (DataDome) turned a change away, a like or a
/// playlist: with a [`Browser`], after the check it asked for, if any.
fn turned_away() -> Error {
    Error::Protocol(
        "SoundCloud's bot guard turned this away: try again, passing its check if asked".into(),
    )
}

/// Whether an answer is the bot guard's refusal: its header says so, or,
/// where that header can't be read, its body names the check.
fn guarded(status: u16, guard_header: bool, body: &str) -> bool {
    status == 403 && (guard_header || body.contains("captcha-delivery.com"))
}

/// A change to send to SoundCloud's API: a like, a follow, a playlist.
pub struct Change {
    pub method: String,
    /// With its query, the client id's.
    pub url: String,
    /// The user's token among them: never printed.
    pub headers: Vec<(String, String)>,
    /// JSON, when there is one.
    pub body: Option<String>,
}

/// What SoundCloud answered a [`Change`].
pub struct Answer {
    pub status: u16,
    /// The bot guard's header (`x-dd-b`) was there.
    pub guard_header: bool,
    pub body: String,
}

/// Sends changes as a browser does: SoundCloud's bot guard turns away the
/// likes, follows and playlist changes of any other client. When the guard
/// asks for a check, the browser has the user pass it, and sends again.
#[async_trait]
pub trait Browser: Send + Sync {
    async fn send(&self, change: Change) -> Result<Answer>;
}

/// The user's tokens, as the browser keeps them.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Tokens {
    pub oauth: String,
    pub refresh: Option<String>,
}

/// Never printed.
impl std::fmt::Debug for Tokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tokens").finish_non_exhaustive()
    }
}

/// Told the tokens each time they are renewed, to keep them.
pub type OnTokens = Box<dyn Fn(&Tokens) + Send + Sync>;

pub struct SoundCloud {
    http: reqwest::Client,
    client_id: RwLock<String>,
    tokens: Mutex<Tokens>,
    on_tokens: OnTokens,
    /// Held while the tokens are renewed, and the client id read again: a
    /// refresh token serves once, so requests refused together renew once.
    renewing: tokio::sync::Mutex<()>,
    /// Held while a playlist's tracks are read and written whole: two
    /// changes at once would each write the list without the other's.
    editing: tokio::sync::Mutex<()>,
    /// When the client id was last read again.
    rescraping: tokio::sync::Mutex<Option<std::time::Instant>>,
    /// The signed-in user.
    me: types::User,
    /// What each station has played, so that it goes on without repeating.
    stations: Mutex<std::collections::HashMap<String, Station>>,
    /// Sends the changes, when there is one.
    browser: Option<std::sync::Arc<dyn Browser>>,
}

/// A station's songs so far: their ids, and the last, which the next ones
/// are related to.
#[derive(Default)]
struct Station {
    seen: std::collections::HashSet<u64>,
    last: Option<u64>,
}

impl SoundCloud {
    /// Sign in with `tokens`, renewing them first when they have expired;
    /// `on_tokens` is told every new pair. Without a token, only what anyone
    /// can hear is there; without a `browser`, changes are mostly turned away.
    pub async fn connect(
        tokens: Tokens,
        on_tokens: OnTokens,
        browser: Option<std::sync::Arc<dyn Browser>>,
    ) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            // A server out of reach, or a download that stalls, fails rather
            // than hang what waits on it.
            .connect_timeout(std::time::Duration::from_secs(10))
            .read_timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Network(e.to_string()))?;
        let client_id = client_id::scrape(&http).await?;
        let mut sc = Self {
            http,
            client_id: RwLock::new(client_id),
            tokens: Mutex::new(tokens),
            on_tokens,
            renewing: tokio::sync::Mutex::new(()),
            rescraping: tokio::sync::Mutex::new(None),
            editing: tokio::sync::Mutex::new(()),
            me: types::User::default(),
            stations: Mutex::default(),
            browser,
        };
        if !sc.token().is_empty() {
            sc.me = sc.get("/me").await?;
        }
        Ok(sc)
    }

    /// The signed-in user's name.
    pub fn username(&self) -> &str {
        &self.me.username
    }

    fn client_id(&self) -> String {
        self.client_id
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn token(&self) -> String {
        self.tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .oauth
            .clone()
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.get_with(path, &[]).await
    }

    /// `path` on the API, or a URL it handed back (`next_href`, a
    /// transcoding's), with `query` and the client id.
    async fn get_with<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T> {
        let body = self.call(reqwest::Method::GET, path, query, None).await?;
        serde_json::from_str(&body).map_err(|e| Error::Protocol(format!("{path}: {e}")))
    }

    /// A change on the API, with `body` as JSON when there is one; what it
    /// answered.
    async fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<String> {
        self.call(method, path, &[], body).await
    }

    /// A request to the API as the web player makes it. A refused token is
    /// renewed, a refused client id read again, once each.
    async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<&serde_json::Value>,
    ) -> Result<String> {
        let url = if path.starts_with("https://") {
            path.to_owned()
        } else {
            format!("{API}{path}")
        };
        // The token goes to SoundCloud's API only, not wherever a URL it
        // handed back points.
        let ours = reqwest::Url::parse(&url).is_ok_and(|u| {
            u.scheme() == "https"
                && u.host_str()
                    .is_some_and(|h| h == "soundcloud.com" || h.ends_with(".soundcloud.com"))
        });
        let browser = self
            .browser
            .as_ref()
            .filter(|_| ours && method != reqwest::Method::GET);
        let (mut renewed, mut rescraped, mut waited) = (false, false, false);
        loop {
            let client_id = self.client_id();
            let token = if ours { self.token() } else { String::new() };
            let (status, guard_header, retry_after, text) = match browser {
                Some(browser) => {
                    let mut full = reqwest::Url::parse(&url)
                        .map_err(|e| Error::Protocol(format!("{url}: {e}")))?;
                    full.query_pairs_mut()
                        .append_pair("client_id", &client_id)
                        .extend_pairs(query);
                    let mut headers = vec![(
                        "Accept".to_owned(),
                        "application/json, text/javascript, */*; q=0.01".to_owned(),
                    )];
                    if !token.is_empty() {
                        headers.push(("Authorization".to_owned(), format!("OAuth {token}")));
                    }
                    if body.is_some() {
                        headers.push(("Content-Type".to_owned(), "application/json".to_owned()));
                    }
                    let answer = browser
                        .send(Change {
                            method: method.to_string(),
                            url: full.into(),
                            headers,
                            body: body.map(|b| b.to_string()),
                        })
                        .await?;
                    (answer.status, answer.guard_header, None, answer.body)
                }
                None => {
                    let mut request = self
                        .http
                        .request(method.clone(), &url)
                        .query(&[("client_id", client_id.as_str())])
                        .query(query);
                    if !token.is_empty() {
                        request = request.header("Authorization", format!("OAuth {token}"));
                    }
                    if let Some(body) = body {
                        request = request.json(body);
                    }
                    let resp = request
                        .header("Origin", WEB)
                        .header("Referer", format!("{WEB}/"))
                        .send()
                        .await
                        .map_err(|e| Error::Network(format!("{url}: {}", e.without_url())))?;
                    let status = resp.status().as_u16();
                    let guard_header = resp.headers().contains_key("x-dd-b");
                    let retry_after = resp
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.trim().parse::<u64>().ok());
                    let text = if (200..300).contains(&status) {
                        resp.text()
                            .await
                            .map_err(|e| Error::Network(format!("{url}: {}", e.without_url())))?
                    } else {
                        String::new()
                    };
                    (status, guard_header, retry_after, text)
                }
            };
            match status {
                401 if !renewed && !token.is_empty() => {
                    renewed = true;
                    self.renew(&token).await?;
                }
                // Its guard says so; another client id wouldn't change it.
                403 if guarded(status, guard_header, &text) => return Err(turned_away()),
                // Refused still: the web player's client id may have changed.
                401 | 403 if !rescraped && self.rescrape(&client_id).await? => rescraped = true,
                401 => return Err(Error::NotSignedIn),
                // Asked to slow down: once, after the wait it names if short.
                429 if !waited => {
                    waited = true;
                    let wait = retry_after.unwrap_or(2);
                    if wait > MAX_RETRY_AFTER {
                        return Err(slow_down());
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                }
                429 => return Err(slow_down()),
                s if !(200..300).contains(&s) => {
                    return Err(Error::Protocol(format!("{url}: HTTP {s}")));
                }
                _ => return Ok(text),
            }
        }
    }

    /// Read the web player's client id again, as `refused` was: unless it
    /// was read again meanwhile, for another request refused, or lately
    /// (a refusal for another reason, a private track, isn't worth reading
    /// soundcloud.com each time). Whether it is worth asking again.
    async fn rescrape(&self, refused: &str) -> Result<bool> {
        let mut last = self.rescraping.lock().await;
        if self.client_id() != refused {
            return Ok(true);
        }
        if last.is_some_and(|at| at.elapsed() < RESCRAPE_EVERY) {
            return Ok(false);
        }
        *last = Some(std::time::Instant::now());
        let id = client_id::scrape(&self.http).await?;
        let changed = id != refused;
        *self.client_id.write().unwrap_or_else(|e| e.into_inner()) = id;
        Ok(changed)
    }

    /// Renew the token with the refresh token, as the web player's server
    /// does: OAuth's `refresh_token` grant, for the app the token was issued
    /// to (its `client_id` claim). A refresh token serves once; both come
    /// back new. `refused` is the token refused: one renewed meanwhile, for
    /// another request, is taken as it is.
    async fn renew(&self, refused: &str) -> Result<()> {
        let _renewing = self.renewing.lock().await;
        if self.token() != refused {
            return Ok(());
        }
        let tokens = self
            .tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let Some(refresh) = &tokens.refresh else {
            return Err(Error::NotSignedIn);
        };
        let client = jwt_client_id(&tokens.oauth).unwrap_or_else(|| self.client_id());
        let resp = self
            .http
            .post(format!("{SECURE}/oauth/token"))
            .form(&[
                ("client_id", client.as_str()),
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh.as_str()),
            ])
            .header("Origin", WEB)
            .header("Referer", format!("{WEB}/"))
            .send()
            .await
            .map_err(|e| Error::Network(format!("SoundCloud sign-in: {}", e.without_url())))?;
        // Only a refresh token turned down signs out: SoundCloud busy, or its
        // bot wall, is said as such and tried again later.
        match resp.status().as_u16() {
            400 | 401 => return Err(Error::NotSignedIn),
            403 if resp.headers().contains_key("x-dd-b") => return Err(turned_away()),
            _ if !resp.status().is_success() => {
                return Err(Error::Protocol(format!(
                    "SoundCloud sign-in: HTTP {}",
                    resp.status()
                )));
            }
            _ => {}
        }
        let grant: types::Grant = resp
            .json()
            .await
            .map_err(|e| Error::Protocol(format!("SoundCloud sign-in: {}", e.without_url())))?;
        let renewed = Tokens {
            oauth: grant.access_token,
            refresh: grant.refresh_token.or(tokens.refresh),
        };
        (self.on_tokens)(&renewed);
        *self.tokens.lock().unwrap_or_else(|e| e.into_inner()) = renewed;
        Ok(())
    }

    /// A public file, a signed URL: as it is, without the API's parameters.
    async fn public_bytes(&self, url: &str) -> Result<(Vec<u8>, Option<String>)> {
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| Error::Network(format!("SoundCloud stream: {}", e.without_url())))?;
        if !resp.status().is_success() {
            return Err(Error::Protocol(format!(
                "SoundCloud stream: HTTP {}",
                resp.status()
            )));
        }
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|t| t.to_str().ok())
            .map(str::to_owned);
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| Error::Network(format!("SoundCloud stream: {}", e.without_url())))?;
        Ok((bytes.to_vec(), content_type))
    }

    async fn public_text(&self, url: &str) -> Result<String> {
        let (bytes, _) = self.public_bytes(url).await?;
        String::from_utf8(bytes).map_err(|e| Error::Protocol(format!("SoundCloud stream: {e}")))
    }

    /// A page of `first`, or the one `cursor` names.
    async fn page<T: DeserializeOwned>(
        &self,
        first: &str,
        cursor: Option<&Cursor>,
    ) -> Result<types::Collection<T>> {
        match cursor {
            Some(next) => self.get(&next.0).await,
            None => {
                let limit = PAGE.to_string();
                self.get_with(first, &[("limit", &limit), ("linked_partitioning", "1")])
                    .await
            }
        }
    }

    /// A playlist's track ids, in its order.
    async fn playlist_ids(&self, playlist: &ItemId) -> Result<Vec<u64>> {
        let found: types::Playlist = self.get(&format!("/playlists/{}", playlist.id)).await?;
        Ok(found.tracks.iter().map(|t| t.id).collect())
    }

    /// Make `ids` a playlist's tracks, in that order: SoundCloud takes the
    /// whole list each time, 500 at most.
    async fn set_playlist_tracks(&self, playlist: &ItemId, ids: &[u64]) -> Result<()> {
        if ids.len() > PLAYLIST_MAX {
            return Err(Error::Protocol(format!(
                "a SoundCloud playlist holds {PLAYLIST_MAX} tracks at most"
            )));
        }
        let body = serde_json::json!({ "playlist": { "tracks": ids } });
        self.send(
            reqwest::Method::PUT,
            &format!("/playlists/{}", playlist.id),
            Some(&body),
        )
        .await
        .map(drop)
    }

    /// Like or unlike: a track, or a playlist or album.
    async fn like(&self, kind: FavouriteKind, id: &ItemId, on: bool) -> Result<()> {
        let path = match kind {
            FavouriteKind::Song => format!("/users/{}/track_likes/{}", self.me.id, id.id),
            // A single is its track.
            FavouriteKind::Album if id.id.starts_with(convert::SINGLE) => format!(
                "/users/{}/track_likes/{}",
                self.me.id,
                &id.id[convert::SINGLE.len()..]
            ),
            FavouriteKind::Album | FavouriteKind::Playlist => {
                format!("/users/{}/playlist_likes/{}", self.me.id, id.id)
            }
            // An artist is followed.
            FavouriteKind::Artist => {
                let method = if on {
                    reqwest::Method::POST
                } else {
                    reqwest::Method::DELETE
                };
                return self
                    .send(method, &format!("/me/followings/{}", id.id), None)
                    .await
                    .map(drop);
            }
        };
        let method = if on {
            reqwest::Method::PUT
        } else {
            reqwest::Method::DELETE
        };
        self.send(method, &path, None).await.map(drop)
    }

    /// An artist's own tracks as singles, newest first, up to
    /// [`UPLOADS_MAX`].
    async fn uploads(&self, artist: &ItemId) -> Result<Vec<Album>> {
        let mut page: types::Collection<types::Track> = self
            .page(&format!("/users/{}/tracks", artist.id), None)
            .await?;
        let mut tracks = std::mem::take(&mut page.collection);
        while let Some(next) = page.next_href.take()
            && tracks.len() < UPLOADS_MAX
        {
            page = self.get(&next).await?;
            tracks.append(&mut page.collection);
        }
        tracks.truncate(UPLOADS_MAX);
        let mut singles: Vec<Album> = tracks.into_iter().map(convert::single).collect();
        singles.sort_by(|a, b| b.released.cmp(&a.released));
        Ok(singles)
    }

    /// Every track of a playlist, in its order: it lists only the first few
    /// whole, the others by id, fetched 50 at a time.
    async fn whole_playlist(&self, id: &ItemId) -> Result<types::Playlist> {
        let mut playlist: types::Playlist = self
            .get_with(
                &format!("/playlists/{}", id.id),
                &[("representation", "full")],
            )
            .await?;
        let missing: Vec<u64> = playlist
            .tracks
            .iter()
            .filter(|t| t.title.is_none())
            .map(|t| t.id)
            .collect();
        let mut found = std::collections::HashMap::new();
        for chunk in missing.chunks(IDS_AT_ONCE) {
            let ids = chunk
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let tracks: Vec<types::Track> = self.get_with("/tracks", &[("ids", &ids)]).await?;
            found.extend(tracks.into_iter().map(|t| (t.id, t)));
        }
        for track in &mut playlist.tracks {
            if track.title.is_none()
                && let Some(whole) = found.remove(&track.id)
            {
                *track = whole;
            }
        }
        // Those SoundCloud no longer has stay out.
        playlist.tracks.retain(|t| t.title.is_some());
        Ok(playlist)
    }
}

/// A track's station, as the app names it: `track-stations:<id>`.
const STATION: &str = "track-stations:";

/// The app a JWT token was issued to: its `client_id` claim.
fn jwt_client_id(token: &str) -> Option<String> {
    use base64::Engine;
    let payload = token.split('.').nth(1)?;
    let json = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&json).ok()?;
    claims["client_id"].as_str().map(str::to_owned)
}

/// Where a list of playlists goes on: the user's own first, then the ones
/// they liked.
const OWN: &str = "own:";
const LIKED: &str = "liked:";

#[async_trait]
impl Provider for SoundCloud {
    fn kind(&self) -> ProviderKind {
        ProviderKind::SoundCloud
    }

    async fn search(&self, query: &str) -> Result<SearchResults> {
        let q = [("q", query), ("limit", "20")];
        // Each kind as it came: one that failed leaves the others shown,
        // unless they all did.
        let (tracks, users, playlists, albums) = tokio::join!(
            self.get_with::<types::Collection<types::Track>>("/search/tracks", &q),
            self.get_with::<types::Collection<types::User>>("/search/users", &q),
            self.get_with::<types::Collection<types::Playlist>>(
                "/search/playlists_without_albums",
                &q
            ),
            self.get_with::<types::Collection<types::Playlist>>("/search/albums", &q),
        );
        if tracks.is_err() && users.is_err() && playlists.is_err() && albums.is_err() {
            return tracks.map(|_| SearchResults::default());
        }
        fn got<T>(page: Result<types::Collection<T>>) -> Vec<T> {
            page.map(|p| p.collection).unwrap_or_default()
        }
        let (tracks, users, playlists, albums) =
            (got(tracks), got(users), got(playlists), got(albums));
        let me = Some(self.me.id);
        Ok(SearchResults {
            tracks: tracks.into_iter().map(convert::track).collect(),
            albums: albums.into_iter().map(convert::album).collect(),
            artists: users.into_iter().map(convert::user).collect(),
            playlists: playlists
                .into_iter()
                .map(|p| convert::playlist(p, me))
                .collect(),
        })
    }

    async fn search_library(&self, _query: &str) -> Result<SearchResults> {
        Ok(SearchResults::default())
    }

    /// The user's liked tracks, newest first.
    async fn library_tracks(&self, cursor: Option<&Cursor>) -> Result<Page<Track>> {
        let page: types::Collection<types::TrackLike> = self
            .page(&format!("/users/{}/track_likes", self.me.id), cursor)
            .await?;
        Ok(Page {
            items: page
                .collection
                .into_iter()
                .filter_map(|like| like.track)
                .map(|t| {
                    let track = convert::track(t);
                    // Remove from Library unlikes it, by its own id.
                    Track {
                        in_library: true,
                        library_id: Some(track.id.clone()),
                        ..track
                    }
                })
                .collect(),
            next: page.next_href.map(Cursor),
        })
    }

    async fn library_albums(&self, _cursor: Option<&Cursor>) -> Result<Page<Album>> {
        Ok(Page {
            items: Vec::new(),
            next: None,
        })
    }

    /// The user's own playlists, then those they liked.
    async fn library_playlists(&self, cursor: Option<&Cursor>) -> Result<Page<Playlist>> {
        let me = Some(self.me.id);
        // What follows the prefix: the page to go on from, none at first.
        let after = |prefix: &str| {
            cursor
                .and_then(|c| c.0.strip_prefix(prefix))
                .filter(|n| !n.is_empty())
                .map(|n| Cursor(n.to_owned()))
        };
        if cursor.is_some_and(|c| c.0.starts_with(LIKED)) {
            let page: types::Collection<types::Like> = self
                .page(
                    &format!("/users/{}/likes", self.me.id),
                    after(LIKED).as_ref(),
                )
                .await?;
            return Ok(Page {
                items: page
                    .collection
                    .into_iter()
                    .filter_map(|like| like.playlist)
                    .map(|p| convert::playlist(p, me))
                    .collect(),
                next: page.next_href.map(|n| Cursor(format!("{LIKED}{n}"))),
            });
        }
        let page: types::Collection<types::Playlist> = self
            .page(
                &format!("/users/{}/playlists", self.me.id),
                after(OWN).as_ref(),
            )
            .await?;
        Ok(Page {
            items: page
                .collection
                .into_iter()
                .map(|p| convert::playlist(p, me))
                .collect(),
            next: Some(Cursor(match page.next_href {
                Some(n) => format!("{OWN}{n}"),
                None => LIKED.to_owned(),
            })),
        })
    }

    async fn library_artists(&self, _cursor: Option<&Cursor>) -> Result<Page<Artist>> {
        Ok(Page {
            items: Vec::new(),
            next: None,
        })
    }

    /// What the user liked last, newest first: a track as a single, an
    /// album as one (each with when it was liked), a playlist.
    async fn recently_added(&self, cursor: Option<&Cursor>) -> Result<Page<Collection>> {
        if cursor.is_some() {
            return Ok(Page {
                items: Vec::new(),
                next: None,
            });
        }
        let page: types::Collection<types::Like> = self
            .page(&format!("/users/{}/likes", self.me.id), None)
            .await?;
        let me = Some(self.me.id);
        let items = page
            .collection
            .into_iter()
            .filter_map(|like| {
                let added = like.created_at;
                if let Some(track) = like.track {
                    return Some(Collection::Album(Album {
                        added,
                        ..convert::single(track)
                    }));
                }
                let playlist = like.playlist?;
                Some(if playlist.is_album {
                    Collection::Album(Album {
                        added,
                        ..convert::album(playlist)
                    })
                } else {
                    Collection::Playlist(Playlist {
                        added,
                        ..convert::playlist(playlist, me)
                    })
                })
            })
            .collect();
        Ok(Page { items, next: None })
    }

    /// SoundCloud's albums are playlists marked so.
    async fn album(&self, album: &ItemId) -> Result<Album> {
        if let Some(single) = album.id.strip_prefix(convert::SINGLE) {
            let track: types::Track = self.get(&format!("/tracks/{single}")).await?;
            return Ok(convert::single(track));
        }
        let playlist: types::Playlist = self.get(&format!("/playlists/{}", album.id)).await?;
        Ok(convert::album(playlist))
    }

    async fn album_tracks(&self, album: &ItemId) -> Result<Vec<Track>> {
        if let Some(single) = album.id.strip_prefix(convert::SINGLE) {
            let track: types::Track = self.get(&format!("/tracks/{single}")).await?;
            return Ok(vec![convert::track(track)]);
        }
        let playlist = self.whole_playlist(album).await?;
        Ok(playlist.tracks.into_iter().map(convert::track).collect())
    }

    async fn artist(&self, artist: &ItemId) -> Result<ArtistDetail> {
        let user: types::User = self.get(&format!("/users/{}", artist.id)).await?;
        let limit = [("limit", "20")];
        let (top_path, albums_path) = (
            format!("/users/{}/toptracks", artist.id),
            format!("/users/{}/albums", artist.id),
        );
        let (top, albums, singles) = tokio::join!(
            self.get_with::<types::Collection<types::Track>>(&top_path, &limit),
            self.get_with::<types::Collection<types::Playlist>>(&albums_path, &limit),
            self.uploads(artist),
        );
        let (top, albums) = (top?, albums?);
        Ok(ArtistDetail {
            artist: convert::user(user),
            top_songs: top.collection.into_iter().map(convert::track).collect(),
            latest: None,
            albums: albums.collection.into_iter().map(convert::album).collect(),
            // Without them the page still shows what it has.
            singles: singles.unwrap_or_default(),
            compilations: Vec::new(),
            appears_on: Vec::new(),
            similar: Vec::new(),
            essentials: Vec::new(),
            videos: Vec::new(),
            playlists: Vec::new(),
            motion: None,
            banner: None,
        })
    }

    /// A track leads to its uploader's profile.
    async fn song_links(&self, song: &ItemId) -> Result<SongLinks> {
        let track: types::Track = self.get(&format!("/tracks/{}", song.id)).await?;
        Ok(SongLinks {
            artist: track.user.map(|u| convert::id(u.id)),
            station: Some(ItemId::new(
                ProviderKind::SoundCloud,
                format!("{STATION}{}", track.id),
            )),
        })
    }

    /// The track's station first, then songs related to the last one played,
    /// none twice.
    async fn station_tracks(&self, station: &ItemId, _count: usize) -> Result<Vec<Track>> {
        let seed = station
            .id
            .strip_prefix(STATION)
            .ok_or_else(|| Error::Protocol(format!("not a SoundCloud station: {}", station.id)))?;
        let last = {
            let stations = self.stations.lock().unwrap_or_else(|e| e.into_inner());
            stations.get(&station.id).and_then(|s| s.last)
        };
        let limit = [("limit", "50")];
        let page: types::Collection<types::Track> = match last {
            None => {
                self.get_with(
                    &format!("/stations/soundcloud:{STATION}{seed}/tracks"),
                    &limit,
                )
                .await?
            }
            Some(last) => {
                self.get_with(&format!("/tracks/{last}/related"), &limit)
                    .await?
            }
        };
        let mut stations = self.stations.lock().unwrap_or_else(|e| e.into_inner());
        let state = stations.entry(station.id.clone()).or_default();
        let further = page.collection.last().map(|t| t.id);
        let fresh: Vec<types::Track> = page
            .collection
            .into_iter()
            .filter(|t| t.title.is_some() && state.seen.insert(t.id))
            .collect();
        // All played already: on from one of them, not from the same again.
        state.last = fresh.last().map(|t| t.id).or(further).or(state.last);
        Ok(fresh.into_iter().map(convert::track).collect())
    }

    async fn set_favourite(&self, kind: FavouriteKind, id: &ItemId, on: bool) -> Result<()> {
        self.like(kind, id, on).await
    }

    /// Into the library is a like, on SoundCloud.
    async fn add_to_library(&self, kind: FavouriteKind, id: &ItemId) -> Result<()> {
        self.like(kind, id, true).await
    }

    async fn remove_from_library(&self, library_id: &ItemId) -> Result<()> {
        self.like(FavouriteKind::Song, library_id, false).await
    }

    /// Its permalink, as SoundCloud gives it.
    async fn share_link(&self, kind: FavouriteKind, id: &ItemId) -> Result<Option<String>> {
        let track = |id: &str| format!("/tracks/{id}");
        Ok(match kind {
            FavouriteKind::Song => {
                self.get::<types::Track>(&track(&id.id))
                    .await?
                    .permalink_url
            }
            // A single is its track.
            FavouriteKind::Album if id.id.starts_with(convert::SINGLE) => {
                let path = track(&id.id[convert::SINGLE.len()..]);
                self.get::<types::Track>(&path).await?.permalink_url
            }
            FavouriteKind::Album | FavouriteKind::Playlist => {
                let path = format!("/playlists/{}", id.id);
                self.get::<types::Playlist>(&path).await?.permalink_url
            }
            FavouriteKind::Artist => {
                let path = format!("/users/{}", id.id);
                self.get::<types::User>(&path).await?.permalink_url
            }
        })
    }

    /// Private, as the user's own playlists are until they share them.
    async fn create_playlist(
        &self,
        name: &str,
        description: &str,
        tracks: &[ItemId],
    ) -> Result<Playlist> {
        let ids: Vec<u64> = tracks.iter().filter_map(|t| t.id.parse().ok()).collect();
        let body = serde_json::json!({ "playlist": {
            "title": name,
            "description": description,
            "sharing": "private",
            "tracks": ids,
        }});
        let made = self
            .send(reqwest::Method::POST, "/playlists", Some(&body))
            .await?;
        let made: types::Playlist =
            serde_json::from_str(&made).map_err(|e| Error::Protocol(format!("playlists: {e}")))?;
        Ok(convert::playlist(made, Some(self.me.id)))
    }

    async fn edit_playlist(&self, playlist: &ItemId, name: &str, description: &str) -> Result<()> {
        let body = serde_json::json!({ "playlist": { "title": name, "description": description } });
        self.send(
            reqwest::Method::PUT,
            &format!("/playlists/{}", playlist.id),
            Some(&body),
        )
        .await
        .map(drop)
    }

    /// The user's own goes for good; one they only liked is unliked.
    async fn delete_playlist(&self, playlist: &ItemId) -> Result<()> {
        let found: types::Playlist = self.get(&format!("/playlists/{}", playlist.id)).await?;
        if found.user.is_some_and(|u| u.id == self.me.id) {
            self.send(
                reqwest::Method::DELETE,
                &format!("/playlists/{}", playlist.id),
                None,
            )
            .await
            .map(drop)
        } else {
            self.like(FavouriteKind::Playlist, playlist, false).await
        }
    }

    /// `entry` is the track itself: a playlist has a track once.
    async fn remove_from_playlist(&self, playlist: &ItemId, entry: &ItemId) -> Result<()> {
        let _editing = self.editing.lock().await;
        let mut ids = self.playlist_ids(playlist).await?;
        ids.retain(|id| id.to_string() != entry.id);
        self.set_playlist_tracks(playlist, &ids).await
    }

    /// At the end, those not in it already.
    async fn add_to_playlist(&self, playlist: &ItemId, tracks: &[ItemId]) -> Result<()> {
        let _editing = self.editing.lock().await;
        let mut ids = self.playlist_ids(playlist).await?;
        for id in tracks.iter().filter_map(|t| t.id.parse::<u64>().ok()) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        self.set_playlist_tracks(playlist, &ids).await
    }

    /// All of it at once: SoundCloud playlists are short (500 at most).
    async fn playlist_tracks(
        &self,
        playlist: &ItemId,
        _cursor: Option<&Cursor>,
    ) -> Result<Page<Track>> {
        let playlist = self.whole_playlist(playlist).await?;
        Ok(Page {
            items: playlist.tracks.into_iter().map(convert::track).collect(),
            next: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_names_the_app_it_was_issued_to() {
        use base64::Engine;
        let claims = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"client_id":"web-app","sub":"1"}"#);
        assert_eq!(
            jwt_client_id(&format!("eyJhbGciOiJIUzI1NiJ9.{claims}.sig")).as_deref(),
            Some("web-app")
        );
        assert_eq!(jwt_client_id("2-12345-old-style"), None);
    }
}
