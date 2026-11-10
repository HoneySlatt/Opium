//! SoundCloud, through the web player's own API (`api-v2`): the official one
//! takes no new apps. Requests carry the web player's client id, read from
//! soundcloud.com (see `client_id`), and the user's token from the
//! `oauth_token` cookie of a browser sign-in (see [`signin`]). That token now
//! lasts a day or so: it is renewed the way the web does, with the
//! `oauth_refresh_token` cookie, and the new pair handed back to be kept.

mod client_id;
mod convert;
mod discover;
mod resume;
pub mod signin;
mod stream;
mod types;

use std::sync::{Mutex, RwLock};

use async_trait::async_trait;
use opium_core::model::{
    Album, Artist, ArtistDetail, Collection, Cursor, FavouriteKind, Genre, ItemId, Page, Playlist,
    ProviderKind, SearchResults, Shelf, SongLinks, Track,
};
use opium_core::{Error, Provider, Result};
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
        let playlist = numeric_id(&playlist.id)?;
        if ids.len() > PLAYLIST_MAX {
            return Err(Error::Protocol(format!(
                "a SoundCloud playlist holds {PLAYLIST_MAX} tracks at most"
            )));
        }
        let body = serde_json::json!({ "playlist": { "tracks": ids } });
        self.send(
            reqwest::Method::PUT,
            &format!("/playlists/{playlist}"),
            Some(&body),
        )
        .await
        .map(drop)
    }

    /// Like or unlike: a track, or a playlist or album.
    ///
    /// # Errors
    ///
    /// [`Error::Protocol`] for an id that is not a number (a single's, after its prefix),
    /// before anything is sent.
    async fn like(&self, kind: FavouriteKind, id: &ItemId, on: bool) -> Result<()> {
        let path = match kind {
            FavouriteKind::Song => {
                format!("/users/{}/track_likes/{}", self.me.id, numeric_id(&id.id)?)
            }
            // A single is its track.
            FavouriteKind::Album if id.id.starts_with(convert::SINGLE) => format!(
                "/users/{}/track_likes/{}",
                self.me.id,
                numeric_id(&id.id[convert::SINGLE.len()..]).map_err(|_| not_an_id(&id.id))?
            ),
            FavouriteKind::Album | FavouriteKind::Playlist => {
                format!(
                    "/users/{}/playlist_likes/{}",
                    self.me.id,
                    numeric_id(&id.id)?
                )
            }
            // An artist is followed.
            FavouriteKind::Artist => {
                let artist = numeric_id(&id.id)?;
                let method = if on {
                    reqwest::Method::POST
                } else {
                    reqwest::Method::DELETE
                };
                return self
                    .send(method, &format!("/me/followings/{artist}"), None)
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
        let first = self
            .page(&format!("/users/{}/tracks", artist.id), None)
            .await?;
        let tracks = uploads_from(first, |next| async move { self.get(&next).await }).await?;
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
        self.whole_tracks(&mut playlist.tracks).await?;
        Ok(playlist)
    }

    /// `tracks` as SoundCloud has them whole: a list gives only the first
    /// few so, the others by their ids; those it no longer has go.
    async fn whole_tracks(&self, tracks: &mut Vec<types::Track>) -> Result<()> {
        let missing: Vec<u64> = tracks
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
        for track in tracks.iter_mut() {
            if track.title.is_none()
                && let Some(whole) = found.remove(&track.id)
            {
                *track = whole;
            }
        }
        // Those SoundCloud no longer has stay out.
        tracks.retain(|t| t.title.is_some());
        Ok(())
    }
}

/// A track's station, as the app names it: `track-stations:<id>`.
const STATION: &str = "track-stations:";

/// `id`, when it is one of SoundCloud's that may go into the path of a change: a number
/// (users, tracks and playlists have no other), in ASCII digits and nothing else.
///
/// # Errors
///
/// [`Error::Protocol`], naming `id`, for anything else (`..`, `/`, `?`, `#`, `%2e`, nothing):
/// a request that would change the account is refused before it is sent, so that an id from a
/// paired device cannot turn one change into another.
fn numeric_id(id: &str) -> Result<&str> {
    if !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) && id.parse::<u64>().is_ok() {
        Ok(id)
    } else {
        Err(not_an_id(id))
    }
}

fn not_an_id(id: &str) -> Error {
    Error::Protocol(format!("not a SoundCloud id: {id:?}"))
}

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
        let next = onward(&page, cursor.map(|c| c.0.as_str())).map(Cursor);
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
            next,
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
            let asked = after(LIKED);
            let page: types::Collection<types::Like> = self
                .page(&format!("/users/{}/likes", self.me.id), asked.as_ref())
                .await?;
            let next = onward(&page, asked.as_ref().map(|c| c.0.as_str()));
            return Ok(Page {
                items: page
                    .collection
                    .into_iter()
                    .filter_map(|like| like.playlist)
                    .map(|p| convert::playlist(p, me))
                    .collect(),
                next: next.map(|n| Cursor(format!("{LIKED}{n}"))),
            });
        }
        let asked = after(OWN);
        let page: types::Collection<types::Playlist> = self
            .page(&format!("/users/{}/playlists", self.me.id), asked.as_ref())
            .await?;
        let next = onward(&page, asked.as_ref().map(|c| c.0.as_str()));
        Ok(Page {
            items: page
                .collection
                .into_iter()
                .map(|p| convert::playlist(p, me))
                .collect(),
            next: Some(Cursor(match next {
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
        let playlist = numeric_id(&playlist.id)?;
        let body = serde_json::json!({ "playlist": { "title": name, "description": description } });
        self.send(
            reqwest::Method::PUT,
            &format!("/playlists/{playlist}"),
            Some(&body),
        )
        .await
        .map(drop)
    }

    /// The user's own goes for good; one they only liked is unliked.
    async fn delete_playlist(&self, playlist: &ItemId) -> Result<()> {
        let id = numeric_id(&playlist.id)?;
        let found: types::Playlist = self.get(&format!("/playlists/{id}")).await?;
        if found.user.is_some_and(|u| u.id == self.me.id) {
            self.send(reqwest::Method::DELETE, &format!("/playlists/{id}"), None)
                .await
                .map(drop)
        } else {
            self.like(FavouriteKind::Playlist, playlist, false).await
        }
    }

    /// `entry` is the track itself: a playlist has a track once.
    async fn remove_from_playlist(&self, playlist: &ItemId, entry: &ItemId) -> Result<()> {
        numeric_id(&playlist.id)?;
        let _editing = self.editing.lock().await;
        let mut ids = self.playlist_ids(playlist).await?;
        ids.retain(|id| id.to_string() != entry.id);
        self.set_playlist_tracks(playlist, &ids).await
    }

    /// At the end, those not in it already.
    async fn add_to_playlist(&self, playlist: &ItemId, tracks: &[ItemId]) -> Result<()> {
        numeric_id(&playlist.id)?;
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
    async fn home(&self) -> Result<Vec<Shelf>> {
        self.discover_home().await
    }

    async fn new_music(&self) -> Result<Vec<Shelf>> {
        self.discover_new().await
    }

    async fn radio(&self) -> Result<Vec<Shelf>> {
        self.discover_radio().await
    }

    async fn genres(&self) -> Result<Vec<Genre>> {
        self.discover_genres().await
    }

    async fn genre_charts(&self, genre: &str) -> Result<Vec<Shelf>> {
        self.discover_genre(genre).await
    }

    async fn playlist_tracks(
        &self,
        playlist: &ItemId,
        _cursor: Option<&Cursor>,
    ) -> Result<Page<Track>> {
        // One of SoundCloud's own (a mix, a trending list).
        if playlist.id.starts_with(discover::SYSTEM) {
            let list = self.system(&playlist.id).await?;
            return Ok(Page {
                items: list.tracks.into_iter().map(convert::track).collect(),
                next: None,
            });
        }
        let playlist = self.whole_playlist(playlist).await?;
        Ok(Page {
            items: playlist.tracks.into_iter().map(convert::track).collect(),
            next: None,
        })
    }
}

/// The address to go on from after `page`, which was asked for at `asked` (none for a list's
/// first page): none when it names the address just asked, or when the page came empty. Whoever
/// follows the cursors would else ask for the same page for ever.
fn onward<T>(page: &types::Collection<T>, asked: Option<&str>) -> Option<String> {
    page.next_href
        .clone()
        .filter(|next| !page.collection.is_empty() && Some(next.as_str()) != asked)
}

/// The tracks of `page` and of the pages after it, each fetched by `more` from the address
/// the one before names, up to [`UPLOADS_MAX`]. The list ends, as read so far and without an
/// error, at a page with no track or at an address already asked: the pages read are good,
/// and the cap already makes this list a part of the artist's uploads.
async fn uploads_from<F, Fut>(
    mut page: types::Collection<types::Track>,
    mut more: F,
) -> Result<Vec<types::Track>>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Result<types::Collection<types::Track>>>,
{
    let mut tracks = std::mem::take(&mut page.collection);
    let mut asked = std::collections::HashSet::new();
    while let Some(next) = page.next_href.take()
        && tracks.len() < UPLOADS_MAX
        && asked.insert(next.clone())
    {
        page = more(next).await?;
        if page.collection.is_empty() {
            break;
        }
        tracks.append(&mut page.collection);
    }
    tracks.truncate(UPLOADS_MAX);
    Ok(tracks)
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

// Stage 1 audit W2: an artist's uploads are not asked for for ever.
#[cfg(test)]
mod uploads_paging {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// More requests than any honest list needs: past it, the fake gives up, so that a loop
    /// without end fails the test instead of hanging it.
    const GIVE_UP: usize = 50;

    fn page(ids: &[u64], next_href: Option<&str>) -> types::Collection<types::Track> {
        types::Collection {
            collection: ids
                .iter()
                .map(|&id| types::Track {
                    id,
                    ..Default::default()
                })
                .collect(),
            next_href: next_href.map(str::to_owned),
        }
    }

    /// The uploads read from `first` when each later page is `answer(n, address)`, `n` counting
    /// the requests from 1; with how many were made.
    async fn read(
        first: types::Collection<types::Track>,
        answer: impl Fn(usize, &str) -> types::Collection<types::Track>,
    ) -> (Result<Vec<types::Track>>, usize) {
        let asked = AtomicUsize::new(0);
        let tracks = uploads_from(first, |address| {
            let n = asked.fetch_add(1, Ordering::SeqCst) + 1;
            let reply = if n > GIVE_UP {
                Err(Error::Protocol(format!(
                    "still asking after {GIVE_UP} requests"
                )))
            } else {
                Ok(answer(n, &address))
            };
            async move { reply }
        })
        .await;
        (tracks, asked.into_inner())
    }

    #[tokio::test]
    async fn pages_are_followed_to_the_last() {
        let (tracks, asked) = read(page(&[1], Some("p1")), |n, address| {
            assert_eq!(address, format!("p{n}"));
            match n {
                1 => page(&[2], Some("p2")),
                _ => page(&[3], None),
            }
        })
        .await;
        let ids: Vec<u64> = tracks.unwrap().iter().map(|t| t.id).collect();
        assert_eq!(ids, [1, 2, 3]);
        assert_eq!(asked, 2);
    }

    #[tokio::test]
    async fn an_empty_page_ends_the_list_whatever_it_names_next() {
        let (tracks, asked) = read(page(&[1], Some("p1")), |n, _| {
            page(&[], Some(&format!("p{}", n + 1)))
        })
        .await;
        assert_eq!(tracks.unwrap().len(), 1);
        assert_eq!(asked, 1);
    }

    #[tokio::test]
    async fn a_next_address_already_asked_ends_the_list() {
        let (tracks, asked) = read(page(&[1], Some("same")), |n, _| {
            page(&[n as u64 + 1], Some("same"))
        })
        .await;
        let ids: Vec<u64> = tracks.unwrap().iter().map(|t| t.id).collect();
        assert_eq!(ids, [1, 2]);
        assert_eq!(asked, 1);
    }

    #[tokio::test]
    async fn an_address_that_comes_back_later_ends_the_list() {
        let (tracks, asked) = read(page(&[1], Some("a")), |n, _| {
            page(&[n as u64 + 1], Some(if n % 2 == 1 { "b" } else { "a" }))
        })
        .await;
        assert_eq!(tracks.unwrap().len(), 3);
        assert_eq!(asked, 2);
    }

    #[tokio::test]
    async fn no_more_than_the_most_are_kept() {
        let ids: Vec<u64> = (0..UPLOADS_MAX as u64).collect();
        let (tracks, asked) = read(page(&ids, Some("p1")), |n, _| {
            page(&[9_999], Some(&format!("p{}", n + 1)))
        })
        .await;
        assert_eq!(tracks.unwrap().len(), UPLOADS_MAX);
        assert_eq!(asked, 0);
    }
}

// Stage 1 audit W2: a cursor handed back never leads to the page just asked for.
#[cfg(test)]
mod cursors {
    use super::*;

    fn page(ids: &[u64], next_href: Option<&str>) -> types::Collection<u64> {
        types::Collection {
            collection: ids.to_vec(),
            next_href: next_href.map(str::to_owned),
        }
    }

    #[test]
    fn a_next_address_is_kept() {
        assert_eq!(onward(&page(&[1], Some("b")), None).as_deref(), Some("b"));
        assert_eq!(
            onward(&page(&[1], Some("b")), Some("a")).as_deref(),
            Some("b")
        );
        assert_eq!(onward(&page(&[1], None), Some("a")), None);
    }

    #[test]
    fn the_address_just_asked_is_not_handed_back() {
        assert_eq!(onward(&page(&[1], Some("a")), Some("a")), None);
    }

    #[test]
    fn an_empty_page_hands_back_nothing() {
        assert_eq!(onward(&page(&[], Some("b")), Some("a")), None);
        assert_eq!(onward(&page(&[], Some("b")), None), None);
    }
}

// Stage 1 audit S3: a change refuses an id that is not one of SoundCloud's.
#[cfg(test)]
mod write_paths {
    use super::*;
    use opium_core::model::ProviderKind;
    use std::sync::Arc;

    /// Keeps each change it is handed, as `METHOD url`, and answers it as SoundCloud answers a
    /// like: 200, an empty object.
    #[derive(Default)]
    struct Recording(Mutex<Vec<String>>);

    #[async_trait]
    impl Browser for Recording {
        async fn send(&self, change: Change) -> Result<Answer> {
            self.0
                .lock()
                .unwrap()
                .push(format!("{} {}", change.method, change.url));
            Ok(Answer {
                status: 200,
                guard_header: false,
                body: "{}".into(),
            })
        }
    }

    /// Signed in as user 42, its changes sent through `browser`; no request leaves the test.
    fn signed_in(browser: Arc<Recording>) -> SoundCloud {
        SoundCloud {
            http: reqwest::Client::new(),
            client_id: RwLock::new("a".repeat(32)),
            tokens: Mutex::new(Tokens {
                oauth: "a-token".into(),
                refresh: None,
            }),
            on_tokens: Box::new(|_: &Tokens| {}),
            renewing: tokio::sync::Mutex::new(()),
            editing: tokio::sync::Mutex::new(()),
            rescraping: tokio::sync::Mutex::new(None),
            me: types::User {
                id: 42,
                ..Default::default()
            },
            stations: Mutex::default(),
            browser: Some(browser),
        }
    }

    /// The path a recorded change went to, its `..` resolved as the request resolves them.
    fn path_of(change: &str) -> String {
        let url = change.split_once(' ').unwrap().1;
        reqwest::Url::parse(url).unwrap().path().to_owned()
    }

    #[tokio::test]
    async fn unliking_a_track_changes_that_track_like_and_nothing_else() {
        let browser = Arc::new(Recording::default());
        let soundcloud = signed_in(browser.clone());
        let id = ItemId::new(ProviderKind::SoundCloud, "1/../../../../playlists/555");
        let unliked = soundcloud.like(FavouriteKind::Song, &id, false).await;
        let sent = browser.0.lock().unwrap().clone();
        for change in &sent {
            assert!(
                path_of(change).starts_with("/users/42/track_likes/"),
                "an unlike went elsewhere: {change}"
            );
        }
        // Refusing the id before anything is sent passes too.
        assert!(unliked.is_err() || !sent.is_empty());
    }

    #[tokio::test]
    async fn editing_a_playlist_changes_that_playlist_and_nothing_else() {
        let browser = Arc::new(Recording::default());
        let soundcloud = signed_in(browser.clone());
        let id = ItemId::new(ProviderKind::SoundCloud, "1/../../me/followings/7");
        let edited = soundcloud.edit_playlist(&id, "name", "").await;
        let sent = browser.0.lock().unwrap().clone();
        for change in &sent {
            assert!(
                path_of(change).starts_with("/playlists/"),
                "an edit went elsewhere: {change}"
            );
        }
        assert!(edited.is_err() || !sent.is_empty());
    }

    /// Ids that are not numbers, each of which would change what a change reaches or what it
    /// sends: dot segments, separators, a query, a fragment, escapes, nothing, a sign, a
    /// fraction, a number too large for an id, digits that are not ASCII.
    const MALFORMED: [&str; 14] = [
        "..",
        "1/../2",
        "1/2",
        "1?x=2",
        "1#2",
        "%2e%2e",
        "%2e",
        "",
        " 1",
        "+1",
        "-1",
        "1.5",
        "99999999999999999999999",
        "\u{ff11}\u{ff12}",
    ];

    /// Signed in as in [`signed_in`], its reads sent to a proxy that is not there: a read that
    /// slipped through fails as a network error, and reaches nothing.
    fn offline(browser: Arc<Recording>) -> SoundCloud {
        let mut soundcloud = signed_in(browser);
        soundcloud.http = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all("http://127.0.0.1:9").unwrap())
            .build()
            .unwrap();
        soundcloud
    }

    /// `result` is a refusal that names `id`, and `browser` was sent nothing.
    fn assert_refused<T: std::fmt::Debug>(result: Result<T>, id: &str, browser: &Recording) {
        match result {
            Err(Error::Protocol(said)) => {
                assert!(
                    said.contains(&format!("{id:?}")),
                    "the refusal omits {id:?}: {said}"
                )
            }
            other => panic!("{id:?} was not refused: {other:?}"),
        }
        let sent = browser.0.lock().unwrap().clone();
        assert!(sent.is_empty(), "{id:?} sent {sent:?}");
    }

    #[tokio::test]
    async fn a_like_or_a_follow_refuses_an_id_that_is_not_a_number() {
        for bad in MALFORMED.into_iter().chain(["single:1/../2", "single:"]) {
            for kind in [
                FavouriteKind::Song,
                FavouriteKind::Album,
                FavouriteKind::Playlist,
                FavouriteKind::Artist,
            ] {
                for on in [true, false] {
                    let browser = Arc::new(Recording::default());
                    let soundcloud = offline(browser.clone());
                    let id = ItemId::new(ProviderKind::SoundCloud, bad);
                    assert_refused(soundcloud.like(kind, &id, on).await, bad, &browser);
                }
            }
        }
    }

    #[tokio::test]
    async fn a_single_prefix_is_followed_only_for_an_album() {
        for kind in [
            FavouriteKind::Song,
            FavouriteKind::Playlist,
            FavouriteKind::Artist,
        ] {
            let browser = Arc::new(Recording::default());
            let soundcloud = offline(browser.clone());
            let id = ItemId::new(ProviderKind::SoundCloud, "single:7");
            assert_refused(soundcloud.like(kind, &id, true).await, "single:7", &browser);
        }
    }

    #[tokio::test]
    async fn the_provider_methods_that_like_refuse_a_malformed_id() {
        for bad in MALFORMED {
            let id = ItemId::new(ProviderKind::SoundCloud, bad);
            let browser = Arc::new(Recording::default());
            let soundcloud = offline(browser.clone());
            let set = soundcloud
                .set_favourite(FavouriteKind::Song, &id, true)
                .await;
            assert_refused(set, bad, &browser);
            let added = soundcloud.add_to_library(FavouriteKind::Album, &id).await;
            assert_refused(added, bad, &browser);
            let removed = soundcloud.remove_from_library(&id).await;
            assert_refused(removed, bad, &browser);
        }
    }

    #[tokio::test]
    async fn a_playlist_change_refuses_a_malformed_playlist_id() {
        let track = ItemId::new(ProviderKind::SoundCloud, "5");
        for bad in MALFORMED {
            let playlist = ItemId::new(ProviderKind::SoundCloud, bad);
            let browser = Arc::new(Recording::default());
            let soundcloud = offline(browser.clone());
            let edited = soundcloud.edit_playlist(&playlist, "name", "").await;
            assert_refused(edited, bad, &browser);
            let deleted = soundcloud.delete_playlist(&playlist).await;
            assert_refused(deleted, bad, &browser);
            let added = soundcloud
                .add_to_playlist(&playlist, std::slice::from_ref(&track))
                .await;
            assert_refused(added, bad, &browser);
            let removed = soundcloud.remove_from_playlist(&playlist, &track).await;
            assert_refused(removed, bad, &browser);
            let whole = soundcloud.set_playlist_tracks(&playlist, &[5]).await;
            assert_refused(whole, bad, &browser);
        }
    }

    #[tokio::test]
    async fn a_playlist_from_the_discovery_pages_is_not_changed() {
        // Its id is no number: the shelves read it, nothing writes it.
        let browser = Arc::new(Recording::default());
        let soundcloud = offline(browser.clone());
        let id = ItemId::new(ProviderKind::SoundCloud, "soundcloud:system-playlists:x");
        let edited = soundcloud.edit_playlist(&id, "name", "").await;
        assert_refused(edited, &id.id, &browser);
    }

    #[tokio::test]
    async fn numbers_go_to_their_own_paths() {
        let cases = [
            (
                FavouriteKind::Song,
                "7",
                true,
                "PUT",
                "/users/42/track_likes/7",
            ),
            (
                FavouriteKind::Song,
                "7",
                false,
                "DELETE",
                "/users/42/track_likes/7",
            ),
            (
                FavouriteKind::Album,
                "single:7",
                true,
                "PUT",
                "/users/42/track_likes/7",
            ),
            (
                FavouriteKind::Album,
                "9",
                true,
                "PUT",
                "/users/42/playlist_likes/9",
            ),
            (
                FavouriteKind::Playlist,
                "9",
                false,
                "DELETE",
                "/users/42/playlist_likes/9",
            ),
            (FavouriteKind::Artist, "5", true, "POST", "/me/followings/5"),
            (
                FavouriteKind::Artist,
                "5",
                false,
                "DELETE",
                "/me/followings/5",
            ),
        ];
        for (kind, id, on, method, path) in cases {
            let browser = Arc::new(Recording::default());
            let soundcloud = offline(browser.clone());
            let id = ItemId::new(ProviderKind::SoundCloud, id);
            soundcloud.like(kind, &id, on).await.unwrap();
            let sent = browser.0.lock().unwrap().clone();
            assert_eq!(sent.len(), 1, "{sent:?}");
            assert_eq!(sent[0].split_once(' ').unwrap().0, method);
            assert_eq!(path_of(&sent[0]), path);
        }
        let browser = Arc::new(Recording::default());
        let soundcloud = offline(browser.clone());
        let id = ItemId::new(ProviderKind::SoundCloud, "9");
        soundcloud.edit_playlist(&id, "name", "").await.unwrap();
        soundcloud.set_playlist_tracks(&id, &[1, 2]).await.unwrap();
        let sent = browser.0.lock().unwrap().clone();
        assert_eq!(sent.len(), 2, "{sent:?}");
        assert!(
            sent.iter()
                .all(|c| c.starts_with("PUT ") && path_of(c) == "/playlists/9")
        );
    }
}
