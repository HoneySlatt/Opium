//! Navidrome, or any server speaking the Subsonic API (with OpenSubsonic's
//! additions): the user's own files, played as they are. Requests are signed
//! with Subsonic's salted token, `md5(password + salt)`: the password itself
//! is used once, to make it, and kept nowhere.

mod convert;
mod resume;
mod types;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use async_trait::async_trait;
use md5::{Digest, Md5};
use music_core::growing::{Growing, growing};
use music_core::model::{
    Album, Artist, ArtistDetail, Collection, Cursor, FavouriteKind, Genre, ItemId, Lyrics, Page,
    Playlist, ProviderKind, SearchResults, Shelf, SongLinks, Station, StationKind, Track,
};
use music_core::{Error, Provider, Result};
use serde::{Deserialize, Serialize};

use convert::Urls;

/// The Subsonic API version asked for, and the client's name.
const VERSION: &str = "1.16.1";
const CLIENT: &str = "music";
/// Items a page of a list asks for.
const PAGE: usize = 500;
/// Songs named in one request's URL.
const SONGS_PER_CALL: usize = 100;
/// A song's station, as the app names it: `similar:<song id>`.
const STATION: &str = "similar:";
/// The station of the whole library, at random: `similar:everything`.
const EVERYTHING: &str = "everything";

/// A shelf of `albums` titled `title`; none if there are none.
fn shelf(title: &str, albums: Vec<Album>) -> Option<Shelf> {
    (!albums.is_empty()).then(|| Shelf {
        title: title.into(),
        hero: false,
        items: albums.into_iter().map(Collection::Album).collect(),
        picks: Vec::new(),
        songs: Vec::new(),
    })
}

/// This year, by the clock.
fn current_year() -> i64 {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400) as i64;
    // Days to a civil year (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    yoe + era * 400 + i64::from(mp >= 10)
}

#[cfg(test)]
mod year {
    #[test]
    fn this_year_is_read_off_the_clock() {
        // Run in 2026 or later.
        assert!(super::current_year() >= 2026);
    }
}

/// How the app signs in to a server: its address, the user, and a salted
/// token in place of the password.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credentials {
    /// `http://0.0.0.0:4533`, without the trailing slash.
    pub url: String,
    pub user: String,
    pub salt: String,
    pub token: String,
    /// A session on the server's own API, which some changes need (a
    /// playlist's cover): given for the password, renewed as it is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

/// The token and salt sign in as the password does: never printed.
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("url", &self.url)
            .field("user", &self.user)
            .finish_non_exhaustive()
    }
}

impl Credentials {
    /// Make the token from `password` and a fresh salt; the password goes no
    /// further.
    pub fn new(url: &str, user: &str, password: &str) -> Result<Self> {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).map_err(|e| Error::Protocol(format!("random salt: {e}")))?;
        let salt = hex(&bytes);
        let token = hex(&Md5::digest(format!("{password}{salt}").as_bytes()));
        let url = url.trim().trim_end_matches('/');
        // An address alone is reached over HTTP, as a server at home is.
        let url = if url.contains("://") {
            url.to_owned()
        } else {
            format!("http://{url}")
        };
        Ok(Self {
            url,
            user: user.trim().to_owned(),
            salt,
            token,
            session: None,
        })
    }

    /// The sign-in every request carries.
    fn query(&self) -> String {
        format!(
            "u={}&t={}&s={}&v={VERSION}&c={CLIENT}",
            encode(&self.user),
            self.token,
            self.salt
        )
    }
}

/// `url`, a cover of the server at `base`, signed with `auth`.
fn signed_cover(base: &str, auth: &str, url: &str) -> Option<String> {
    let rest = url.strip_prefix(base)?.strip_prefix(convert::COVER)?;
    let kept: Vec<&str> = rest
        .split('&')
        .filter(|p| {
            !["u=", "t=", "s=", "v=", "c="]
                .iter()
                .any(|k| p.starts_with(k))
        })
        .collect();
    Some(format!("{base}{}{}&{auth}", convert::COVER, kept.join("&")))
}

/// Kept JSON (the history, a session, a library list) with every cover of
/// a server in it unsigned: kept before covers were kept so, they carried
/// the sign-in.
pub fn scrub_kept(json: &str) -> String {
    let mut out = String::with_capacity(json.len());
    let mut rest = json;
    while let Some(at) = rest.find(convert::COVER) {
        let start = at + convert::COVER.len();
        out.push_str(&rest[..start]);
        let end = rest[start..].find('"').map_or(rest.len(), |e| start + e);
        let kept: Vec<&str> = rest[start..end]
            .split('&')
            .filter(|p| {
                !["u=", "t=", "s=", "v=", "c="]
                    .iter()
                    .any(|k| p.starts_with(k))
            })
            .collect();
        out.push_str(&kept.join("&"));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A query value, percent-encoded.
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// A track's file, as it comes from the server: read while it downloads.
pub struct Audio {
    pub reader: Reader,
    pub len: u64,
    /// `flac`, `wav`, `mp3`…: a hint for the decoder.
    pub extension: &'static str,
}

/// Where a track is read from.
pub enum Reader {
    Whole(std::io::Cursor<Vec<u8>>),
    Growing(Growing),
}

impl std::io::Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Whole(r) => r.read(buf),
            Self::Growing(r) => r.read(buf),
        }
    }
}

impl std::io::Seek for Reader {
    fn seek(&mut self, to: std::io::SeekFrom) -> std::io::Result<u64> {
        match self {
            Self::Whole(r) => r.seek(to),
            Self::Growing(r) => r.seek(to),
        }
    }
}

/// The file's format, from its type, else its first bytes.
fn extension(content_type: Option<&str>, bytes: &[u8]) -> &'static str {
    let by_type = content_type.and_then(|t| {
        let t = t.split(';').next()?.trim().to_ascii_lowercase();
        Some(match t.as_str() {
            "audio/flac" | "audio/x-flac" => "flac",
            "audio/wav" | "audio/x-wav" | "audio/wave" => "wav",
            "audio/aiff" | "audio/x-aiff" => "aiff",
            "audio/mpeg" | "audio/mp3" => "mp3",
            "audio/mp4" | "audio/x-m4a" | "audio/m4a" | "audio/aac" => "m4a",
            "audio/ogg" | "audio/opus" | "application/ogg" => "ogg",
            _ => return None,
        })
    });
    by_type.unwrap_or(match bytes {
        [b'f', b'L', b'a', b'C', ..] => "flac",
        [b'R', b'I', b'F', b'F', ..] => "wav",
        [b'F', b'O', b'R', b'M', ..] => "aiff",
        [b'O', b'g', b'g', b'S', ..] => "ogg",
        [_, _, _, _, b'f', b't', b'y', b'p', ..] => "m4a",
        _ => "mp3",
    })
}

pub struct Navidrome {
    http: reqwest::Client,
    credentials: Credentials,
    urls: Urls,
    /// What each station has played, so that it goes on without repeating.
    stations: Mutex<HashMap<String, HashSet<String>>>,
    /// The session on the server's own API, as last renewed.
    session: Mutex<Option<String>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The header the server's own API takes its session in, and renews it in.
const SESSION_HEADER: &str = "x-nd-authorization";

impl Navidrome {
    /// Sign in to the server; a wrong password or user is `NotSignedIn`.
    pub async fn connect(credentials: Credentials) -> Result<Self> {
        let http = reqwest::Client::builder()
            // A server out of reach, or a download that stalls, fails rather
            // than hang what waits on it.
            .connect_timeout(std::time::Duration::from_secs(10))
            .read_timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Network(e.to_string()))?;
        let urls = Urls {
            base: credentials.url.clone(),
            auth: credentials.query(),
        };
        let session = Mutex::new(credentials.session.clone());
        let nd = Self {
            http,
            credentials,
            urls,
            stations: Mutex::default(),
            session,
        };
        // The one address the user types: what went wrong with it is said
        // in a way they can act on, rather than as the request's own words.
        nd.call("ping", &[]).await.map_err(|e| match e {
            Error::NotSignedIn => Error::NotSignedIn,
            Error::Network(_) => Error::Network(format!(
                "Nothing answered at {}: check the address, and that the server is on",
                nd.credentials.url
            )),
            Error::Protocol(said) if said.contains("HTTP 404") => Error::Protocol(format!(
                "{} answered, but not as a Navidrome server: give its address alone, without /rest",
                nd.credentials.url
            )),
            Error::Protocol(_) => Error::Protocol(format!(
                "{} answered, but not as a Navidrome server: check the address",
                nd.credentials.url
            )),
            other => other,
        })?;
        // Used, the session is renewed: kept alive while the app is.
        if lock(&nd.session).is_some() {
            let _ = nd
                .native(reqwest::Method::GET, "/api/keepalive/keepalive", None)
                .await;
        }
        Ok(nd)
    }

    /// The sign-in as it stands now, its session renewed: what to keep.
    pub fn credentials(&self) -> Credentials {
        Credentials {
            session: lock(&self.session).clone(),
            ..self.credentials.clone()
        }
    }

    /// A session on the server's own API, for `password`: whether it was
    /// given (a wrong password isn't).
    pub async fn open_session(&self, password: &str) -> Result<bool> {
        let resp = self
            .http
            .post(format!("{}/auth/login", self.credentials.url))
            .json(&serde_json::json!({ "username": self.credentials.user, "password": password }))
            .send()
            .await
            .map_err(|e| Error::Network(format!("Navidrome sign-in: {}", e.without_url())))?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Ok(false);
        }
        if !resp.status().is_success() {
            return Err(Error::Protocol(format!(
                "Navidrome sign-in: HTTP {}",
                resp.status()
            )));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::Protocol(format!("Navidrome sign-in: {}", e.without_url())))?;
        let token = body["token"]
            .as_str()
            .ok_or_else(|| Error::Protocol("Navidrome sign-in: no session".into()))?;
        *lock(&self.session) = Some(token.to_owned());
        Ok(true)
    }

    /// Playlist `id`'s cover `image` (JPEG, PNG…), or the one the server
    /// makes again. False: the server wants the password again (see
    /// [`Navidrome::open_session`]). Navidrome 0.61 and later.
    pub async fn set_playlist_cover(&self, id: &str, image: Option<Vec<u8>>) -> Result<bool> {
        let path = format!("/api/playlist/{}/image", encode(id));
        let (method, form) = match image {
            Some(bytes) => {
                let part = reqwest::multipart::Part::bytes(bytes)
                    .file_name("cover.jpg")
                    .mime_str("image/jpeg")
                    .map_err(|e| Error::Protocol(e.to_string()))?;
                (
                    reqwest::Method::POST,
                    Some(reqwest::multipart::Form::new().part("image", part)),
                )
            }
            None => (reqwest::Method::DELETE, None),
        };
        match self.native(method, &path, form).await? {
            None => Ok(false),
            Some(status) if status.is_success() => Ok(true),
            Some(status)
                if status == reqwest::StatusCode::NOT_FOUND
                    || status == reqwest::StatusCode::METHOD_NOT_ALLOWED =>
            {
                Err(Error::Protocol(
                    "this server can't change covers: Navidrome 0.61 or later can".into(),
                ))
            }
            Some(status) if status == reqwest::StatusCode::FORBIDDEN => Err(Error::Protocol(
                "this server lets only its admins change covers".into(),
            )),
            Some(status) => Err(Error::Protocol(format!("Navidrome cover: HTTP {status}"))),
        }
    }

    /// A request to the server's own API with the session, renewed from
    /// the answer: its status, or none if there is no session or it ended.
    async fn native(
        &self,
        method: reqwest::Method,
        path: &str,
        form: Option<reqwest::multipart::Form>,
    ) -> Result<Option<reqwest::StatusCode>> {
        let Some(session) = lock(&self.session).clone() else {
            return Ok(None);
        };
        let mut request = self
            .http
            .request(method, format!("{}{path}", self.credentials.url))
            .header(SESSION_HEADER, format!("Bearer {session}"));
        if let Some(form) = form {
            request = request.multipart(form);
        }
        let resp = request
            .send()
            .await
            .map_err(|e| Error::Network(format!("Navidrome: {}", e.without_url())))?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            *lock(&self.session) = None;
            return Ok(None);
        }
        if let Some(renewed) = resp
            .headers()
            .get(SESSION_HEADER)
            .and_then(|h| h.to_str().ok())
        {
            let renewed = renewed
                .trim_start_matches("Bearer ")
                .trim_start_matches("bearer ");
            if !renewed.is_empty() {
                *lock(&self.session) = Some(renewed.to_owned());
            }
        }
        Ok(Some(resp.status()))
    }

    pub fn user(&self) -> &str {
        &self.credentials.user
    }

    /// A cover's address of this server, signed to be fetched now; `None`
    /// for any other address. One kept from before covers were kept
    /// unsigned is signed anew, its old sign-in left out.
    pub fn sign(&self, url: &str) -> Option<String> {
        signed_cover(&self.credentials.url, &self.urls.auth, url)
    }

    /// `view` with `query`, signed; its body, or the server's refusal.
    async fn call(&self, view: &str, query: &[(&str, String)]) -> Result<types::Body> {
        let url = format!(
            "{}/rest/{view}.view?{}&f=json",
            self.credentials.url, self.urls.auth
        );
        let resp = self
            .http
            .get(&url)
            .query(query)
            .send()
            .await
            .map_err(|e| Error::Network(format!("Navidrome {view}: {}", e.without_url())))?;
        if !resp.status().is_success() {
            return Err(Error::Protocol(format!(
                "Navidrome {view}: HTTP {}",
                resp.status()
            )));
        }
        let envelope: types::Envelope<types::Body> = resp
            .json()
            .await
            .map_err(|e| Error::Protocol(format!("Navidrome {view}: {}", e.without_url())))?;
        let response = envelope.response;
        if response.status != "ok" {
            let error = response.error.unwrap_or(types::ApiError {
                code: 0,
                message: "refused".into(),
            });
            // A wrong user or password, or a server that takes no tokens.
            return Err(match error.code {
                40 | 41 => Error::NotSignedIn,
                _ => Error::Protocol(format!("Navidrome {view}: {}", error.message)),
            });
        }
        Ok(response.body)
    }

    /// Where a song's file streams from, as it is on the server: signed
    /// with the token, so never shown.
    pub fn stream_url(&self, track: &ItemId) -> String {
        format!(
            "{}/rest/stream.view?id={}&format=raw&{}",
            self.credentials.url,
            encode(&track.id),
            self.urls.auth
        )
    }

    /// A song's file as it is on the server, read as it downloads.
    pub async fn audio(&self, track: &ItemId) -> Result<Audio> {
        let url = self.stream_url(track);
        let failed =
            |e: reqwest::Error| Error::Network(format!("Navidrome stream: {}", e.without_url()));
        let mut resp = self.http.get(&url).send().await.map_err(failed)?;
        if !resp.status().is_success() {
            return Err(Error::Playback(format!(
                "Navidrome stream: HTTP {}",
                resp.status()
            )));
        }
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|t| t.to_str().ok())
            .map(str::to_owned);
        let len = resp.content_length();
        let first = resp.chunk().await.map_err(failed)?.unwrap_or_default();
        let extension = extension(content_type.as_deref(), &first);
        let Some(len) = len else {
            // Of no told length: whole, then.
            let mut bytes = first.to_vec();
            while let Some(chunk) = resp.chunk().await.map_err(failed)? {
                bytes.extend_from_slice(&chunk);
            }
            return Ok(Audio {
                len: bytes.len() as u64,
                reader: Reader::Whole(std::io::Cursor::new(bytes)),
                extension,
            });
        };
        let (reader, feed) = growing(len);
        feed.push(&first);
        let again = self.http.get(&url);
        let received = first.len() as u64;
        tokio::spawn(async move {
            // The reader gone, so is the need: `push` says so.
            let ended = resume::carry_on(
                resp,
                received,
                |from| resume::rest_of(&again, from),
                |bytes| feed.push(bytes),
            )
            .await;
            feed.end(ended.err());
        });
        Ok(Audio {
            reader: Reader::Growing(reader),
            len,
            extension,
        })
    }

    /// The songs, albums and artists the user starred.
    pub async fn starred(&self) -> Result<Vec<ItemId>> {
        let body = self.call("getStarred2", &[]).await?;
        let starred = body.starred2.unwrap_or_default();
        Ok(starred
            .song
            .iter()
            .map(|s| convert::id(&s.id))
            .chain(starred.album.iter().map(|a| convert::id(&a.id)))
            .chain(starred.artist.iter().map(|a| convert::id(&a.id)))
            .collect())
    }

    fn track(&self, song: types::Song) -> Track {
        convert::track(&self.urls, song)
    }

    fn album_of(&self, album: types::Album) -> Album {
        convert::album(&self.urls, album)
    }

    /// `size` albums of the server's list of `kind` (recent, frequent…).
    async fn albums(&self, kind: &str, size: usize) -> Result<Vec<Album>> {
        let body = self
            .call(
                "getAlbumList2",
                &[("type", kind.into()), ("size", size.to_string())],
            )
            .await?;
        Ok(body
            .album_list2
            .unwrap_or_default()
            .album
            .into_iter()
            .map(|a| self.album_of(a))
            .collect())
    }

    /// The offset a cursor names, or the start.
    fn offset(cursor: Option<&Cursor>) -> usize {
        cursor.and_then(|c| c.0.parse().ok()).unwrap_or(0)
    }

    /// The cursor after `got` items at `offset`, unless the list ended: a
    /// server may hand back pages shorter than asked, so only an empty one
    /// ends it.
    fn next(offset: usize, got: usize) -> Option<Cursor> {
        (got > 0).then(|| Cursor((offset + got).to_string()))
    }

    async fn playlist_of(&self, playlist: &ItemId) -> Result<types::Playlist> {
        self.call("getPlaylist", &[("id", playlist.id.clone())])
            .await?
            .playlist
            .ok_or_else(|| Error::Protocol("Navidrome: no such playlist".into()))
    }
}

#[async_trait]
impl Provider for Navidrome {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Navidrome
    }

    async fn search(&self, query: &str) -> Result<SearchResults> {
        let body = self
            .call(
                "search3",
                &[
                    ("query", query.to_owned()),
                    ("songCount", "20".into()),
                    ("albumCount", "20".into()),
                    ("artistCount", "20".into()),
                ],
            )
            .await?;
        let found = body.search_result3.unwrap_or_default();
        Ok(SearchResults {
            tracks: found.song.into_iter().map(|s| self.track(s)).collect(),
            albums: found.album.into_iter().map(|a| self.album_of(a)).collect(),
            artists: found
                .artist
                .into_iter()
                .map(|a| convert::artist(&self.urls, a))
                .collect(),
            playlists: Vec::new(),
        })
    }

    /// The server is the library.
    async fn search_library(&self, query: &str) -> Result<SearchResults> {
        self.search(query).await
    }

    /// Every song on the server, page by page.
    async fn library_tracks(&self, cursor: Option<&Cursor>) -> Result<Page<Track>> {
        let offset = Self::offset(cursor);
        let body = self
            .call(
                "search3",
                &[
                    ("query", String::new()),
                    ("songCount", PAGE.to_string()),
                    ("songOffset", offset.to_string()),
                    ("albumCount", "0".into()),
                    ("artistCount", "0".into()),
                ],
            )
            .await?;
        let songs = body.search_result3.unwrap_or_default().song;
        let next = Self::next(offset, songs.len());
        Ok(Page {
            items: songs.into_iter().map(|s| self.track(s)).collect(),
            next,
        })
    }

    async fn library_albums(&self, cursor: Option<&Cursor>) -> Result<Page<Album>> {
        let offset = Self::offset(cursor);
        let body = self
            .call(
                "getAlbumList2",
                &[
                    ("type", "alphabeticalByName".into()),
                    ("size", PAGE.to_string()),
                    ("offset", offset.to_string()),
                ],
            )
            .await?;
        let albums = body.album_list2.unwrap_or_default().album;
        let next = Self::next(offset, albums.len());
        Ok(Page {
            items: albums.into_iter().map(|a| self.album_of(a)).collect(),
            next,
        })
    }

    async fn library_playlists(&self, _cursor: Option<&Cursor>) -> Result<Page<Playlist>> {
        let body = self.call("getPlaylists", &[]).await?;
        let user = self.credentials.user.clone();
        Ok(Page {
            items: body
                .playlists
                .unwrap_or_default()
                .playlist
                .into_iter()
                .map(|p| convert::playlist(&self.urls, p, &user))
                .collect(),
            next: None,
        })
    }

    async fn library_artists(&self, _cursor: Option<&Cursor>) -> Result<Page<Artist>> {
        let body = self.call("getArtists", &[]).await?;
        Ok(Page {
            items: body
                .artists
                .unwrap_or_default()
                .index
                .into_iter()
                .flat_map(|i| i.artist)
                .map(|a| convert::artist(&self.urls, a))
                .collect(),
            next: None,
        })
    }

    /// The albums newest on the server.
    async fn recently_added(&self, _cursor: Option<&Cursor>) -> Result<Page<Collection>> {
        let body = self
            .call(
                "getAlbumList2",
                &[("type", "newest".into()), ("size", "100".into())],
            )
            .await?;
        Ok(Page {
            items: body
                .album_list2
                .unwrap_or_default()
                .album
                .into_iter()
                .map(|a| Collection::Album(self.album_of(a)))
                .collect(),
            next: None,
        })
    }

    async fn album(&self, album: &ItemId) -> Result<Album> {
        let body = self.call("getAlbum", &[("id", album.id.clone())]).await?;
        let found = body
            .album
            .ok_or_else(|| Error::Protocol("Navidrome: no such album".into()))?;
        Ok(self.album_of(types::Album {
            song: Vec::new(),
            ..found
        }))
    }

    async fn album_tracks(&self, album: &ItemId) -> Result<Vec<Track>> {
        let body = self.call("getAlbum", &[("id", album.id.clone())]).await?;
        let found = body.album.unwrap_or_default();
        Ok(found.song.into_iter().map(|s| self.track(s)).collect())
    }

    /// Their albums, singles and EPs and compilations as the files say, their
    /// top songs, and the artists like them the server knows of.
    async fn artist(&self, artist: &ItemId) -> Result<ArtistDetail> {
        let id = [("id", artist.id.clone())];
        // The info (biography, similar artists) only adds: a server without
        // it, or failing on it, still shows the artist.
        let (found, info) = tokio::join!(
            self.call("getArtist", &id),
            self.call("getArtistInfo2", &id),
        );
        let found = found?;
        let found = found
            .artist
            .ok_or_else(|| Error::Protocol("Navidrome: no such artist".into()))?;
        let top = self
            .call(
                "getTopSongs",
                &[("artist", found.name.clone()), ("count", "20".into())],
            )
            .await
            .map(|b| b.top_songs.unwrap_or_default().song)
            .unwrap_or_default();
        let info = info.ok().and_then(|i| i.artist_info2).unwrap_or_default();
        let (mut albums, mut singles, mut compilations) = (Vec::new(), Vec::new(), Vec::new());
        for album in found.album.clone() {
            let kinds: Vec<String> = album
                .release_types
                .iter()
                .map(|t| t.to_ascii_lowercase())
                .collect();
            let album = self.album_of(album);
            if kinds.iter().any(|k| k == "compilation") {
                compilations.push(album);
            } else if kinds.iter().any(|k| k == "single" || k == "ep") {
                singles.push(album);
            } else {
                albums.push(album);
            }
        }
        let newest = |list: &mut Vec<Album>| {
            list.sort_by(|a, b| b.released.cmp(&a.released).then(b.year.cmp(&a.year)))
        };
        newest(&mut albums);
        newest(&mut singles);
        newest(&mut compilations);
        let latest = albums
            .iter()
            .chain(&singles)
            .max_by(|a, b| a.released.cmp(&b.released).then(a.year.cmp(&b.year)))
            .cloned();
        Ok(ArtistDetail {
            artist: convert::artist(
                &self.urls,
                types::Artist {
                    album: Vec::new(),
                    ..found
                },
            ),
            top_songs: top.into_iter().map(|s| self.track(s)).collect(),
            latest,
            albums,
            singles,
            compilations,
            appears_on: Vec::new(),
            similar: info
                .similar_artist
                .into_iter()
                .map(|a| convert::artist(&self.urls, a))
                .collect(),
            essentials: Vec::new(),
            videos: Vec::new(),
            playlists: Vec::new(),
            motion: None,
            banner: info.large_image_url.filter(|u| !u.is_empty()),
        })
    }

    async fn playlist_tracks(
        &self,
        playlist: &ItemId,
        _cursor: Option<&Cursor>,
    ) -> Result<Page<Track>> {
        let found = self.playlist_of(playlist).await?;
        Ok(Page {
            items: found.entry.into_iter().map(|s| self.track(s)).collect(),
            next: None,
        })
    }

    /// Lyrics the server has, in the file or beside it.
    async fn lyrics(&self, track: &ItemId) -> Result<Option<Lyrics>> {
        match self
            .call("getLyricsBySongId", &[("id", track.id.clone())])
            .await
        {
            Ok(body) => Ok(body.lyrics_list.and_then(convert::lyrics)),
            // A server without OpenSubsonic's lyrics has none to give.
            Err(Error::Protocol(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// The song's artist, and a station of songs like it.
    async fn song_links(&self, song: &ItemId) -> Result<SongLinks> {
        let body = self.call("getSong", &[("id", song.id.clone())]).await?;
        let found = body.song.unwrap_or_default();
        let artist = found
            .artist_id
            .as_deref()
            .or(found.artists.first().map(|a| a.id.as_str()))
            .filter(|a| !a.is_empty())
            .map(convert::id);
        Ok(SongLinks {
            artist,
            station: Some(ItemId::new(
                ProviderKind::Navidrome,
                format!("{STATION}{}", song.id),
            )),
        })
    }

    /// Home: what was played lately, most, starred, and some to find again.
    async fn home(&self) -> Result<Vec<Shelf>> {
        let (recent, frequent, starred, random) = tokio::join!(
            self.albums("recent", 20),
            self.albums("frequent", 20),
            self.call("getStarred2", &[]),
            self.albums("random", 20),
        );
        let starred = starred
            .map(|b| b.starred2.unwrap_or_default().album)
            .unwrap_or_default()
            .into_iter()
            .map(|a| self.album_of(a))
            .collect();
        Ok([
            ("Recently Played", recent.unwrap_or_default()),
            ("Most Played", frequent.unwrap_or_default()),
            ("Favourites", starred),
            ("Rediscover", random.unwrap_or_default()),
        ]
        .into_iter()
        .filter_map(|(title, albums)| shelf(title, albums))
        .collect())
    }

    /// New: what joined the server lately, and this year's releases on it.
    async fn new_music(&self) -> Result<Vec<Shelf>> {
        let year = current_year();
        let by_year = [
            ("type", "byYear".to_owned()),
            ("fromYear", year.to_string()),
            ("toYear", (year - 1).to_string()),
            ("size", "40".to_owned()),
        ];
        let (newest, this_year) = tokio::join!(
            self.albums("newest", 40),
            self.call("getAlbumList2", &by_year),
        );
        let newest = newest?;
        // Those shown as added lately aren't shown again.
        let this_year = this_year
            .map(|b| b.album_list2.unwrap_or_default().album)
            .unwrap_or_default()
            .into_iter()
            .map(|a| self.album_of(a))
            .filter(|a| !newest.iter().any(|n| n.id == a.id))
            .collect();
        Ok([("Recently Added", newest), ("Released Lately", this_year)]
            .into_iter()
            .filter_map(|(title, albums)| shelf(title, albums))
            .collect())
    }

    /// Radio: the whole library shuffled, and a station from each song starred.
    async fn radio(&self) -> Result<Vec<Shelf>> {
        // Shown with an album of the library's, at random.
        let cover = self
            .albums("random", 1)
            .await
            .ok()
            .and_then(|a| a.into_iter().next())
            .and_then(|a| a.artwork);
        let all = Station {
            id: ItemId::new(ProviderKind::Navidrome, format!("{STATION}{EVERYTHING}")),
            name: "Shuffle All".into(),
            description: Some("Your whole library, at random".into()),
            artwork: cover,
            kind: StationKind::Tracks,
        };
        let starred = self
            .call("getStarred2", &[])
            .await
            .map(|b| b.starred2.unwrap_or_default().song)
            .unwrap_or_default();
        let stations: Vec<Collection> = starred
            .into_iter()
            .take(20)
            .map(|song| {
                let track = self.track(song.clone());
                Collection::Station(Station {
                    id: ItemId::new(ProviderKind::Navidrome, format!("{STATION}{}", song.id)),
                    name: track.title.clone(),
                    description: Some(format!("Songs like it · {}", track.artists.join(", "))),
                    artwork: track.artwork,
                    kind: StationKind::Tracks,
                })
            })
            .collect();
        let mut shelves = vec![Shelf {
            title: "Stations".into(),
            hero: true,
            items: vec![Collection::Station(all)],
            picks: Vec::new(),
            songs: Vec::new(),
        }];
        if !stations.is_empty() {
            shelves.push(Shelf {
                title: "From Your Favourites".into(),
                hero: false,
                items: stations,
                picks: Vec::new(),
                songs: Vec::new(),
            });
        }
        Ok(shelves)
    }

    /// The genres the server's songs are tagged with, the largest first.
    async fn genres(&self) -> Result<Vec<Genre>> {
        let body = self.call("getGenres", &[]).await?;
        let mut genres = body.genres.unwrap_or_default().genre;
        genres.retain(|g| !g.value.trim().is_empty() && g.album_count > 0);
        genres.sort_by_key(|g| std::cmp::Reverse(g.album_count));
        Ok(genres
            .into_iter()
            .map(|g| Genre {
                id: g.value.clone(),
                name: g.value,
            })
            .collect())
    }

    /// A genre's albums and songs on the server.
    async fn genre_charts(&self, genre: &str) -> Result<Vec<Shelf>> {
        let by_genre = [
            ("type", "byGenre".to_owned()),
            ("genre", genre.to_owned()),
            ("size", "50".to_owned()),
        ];
        let of_genre = [("genre", genre.to_owned()), ("count", "50".to_owned())];
        let (albums, songs) = tokio::join!(
            self.call("getAlbumList2", &by_genre),
            self.call("getSongsByGenre", &of_genre),
        );
        let albums: Vec<Album> = albums?
            .album_list2
            .unwrap_or_default()
            .album
            .into_iter()
            .map(|a| self.album_of(a))
            .collect();
        let songs: Vec<Track> = songs
            .map(|b| b.songs_by_genre.unwrap_or_default().song)
            .unwrap_or_default()
            .into_iter()
            .map(|s| self.track(s))
            .collect();
        let mut shelves = Vec::new();
        if !songs.is_empty() {
            shelves.push(Shelf {
                title: "Songs".into(),
                hero: false,
                items: Vec::new(),
                picks: Vec::new(),
                songs,
            });
        }
        shelves.extend(shelf("Albums", albums));
        Ok(shelves)
    }

    /// Songs like the station's, as the server's agents know them, else at
    /// random; none twice.
    async fn station_tracks(&self, station: &ItemId, count: usize) -> Result<Vec<Track>> {
        let seed = station
            .id
            .strip_prefix(STATION)
            .ok_or_else(|| Error::Protocol(format!("not a Navidrome station: {}", station.id)))?;
        let count = count.max(20).to_string();
        // The whole library's station: at random only.
        let similar = if seed == EVERYTHING {
            Vec::new()
        } else {
            self.call(
                "getSimilarSongs2",
                &[("id", seed.to_owned()), ("count", count.clone())],
            )
            .await
            .map(|b| b.similar_songs2.unwrap_or_default().song)
            .unwrap_or_default()
        };
        let mut songs = similar;
        {
            let seen = self.stations.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(seen) = seen.get(&station.id) {
                songs.retain(|s| !seen.contains(&s.id));
            }
        }
        if songs.is_empty() {
            songs = self
                .call("getRandomSongs", &[("size", count)])
                .await?
                .random_songs
                .unwrap_or_default()
                .song;
        }
        let mut stations = self.stations.lock().unwrap_or_else(|e| e.into_inner());
        let seen = stations.entry(station.id.clone()).or_default();
        songs.retain(|s| seen.insert(s.id.clone()));
        Ok(songs.into_iter().map(|s| self.track(s)).collect())
    }

    /// A star, on the server.
    async fn set_favourite(&self, kind: FavouriteKind, id: &ItemId, on: bool) -> Result<()> {
        let key = match kind {
            FavouriteKind::Song | FavouriteKind::Playlist => "id",
            FavouriteKind::Album => "albumId",
            FavouriteKind::Artist => "artistId",
        };
        let view = if on { "star" } else { "unstar" };
        self.call(view, &[(key, id.id.clone())]).await.map(drop)
    }

    async fn create_playlist(
        &self,
        name: &str,
        description: &str,
        tracks: &[ItemId],
    ) -> Result<Playlist> {
        // The songs go in the URL: a few at a time, so that it stays short
        // enough for the proxies in front of servers.
        let (first, rest) = tracks.split_at(tracks.len().min(SONGS_PER_CALL));
        let mut query = vec![("name", name.to_owned())];
        query.extend(first.iter().map(|t| ("songId", t.id.clone())));
        let made = self
            .call("createPlaylist", &query)
            .await?
            .playlist
            .ok_or_else(|| Error::Protocol("Navidrome: the playlist wasn't made".into()))?;
        if !rest.is_empty() {
            self.add_to_playlist(&convert::id(&made.id), rest).await?;
        }
        if !description.is_empty() {
            self.call(
                "updatePlaylist",
                &[
                    ("playlistId", made.id.clone()),
                    ("comment", description.to_owned()),
                ],
            )
            .await?;
        }
        let user = self.credentials.user.clone();
        Ok(convert::playlist(
            &self.urls,
            types::Playlist {
                comment: Some(description.to_owned()),
                ..made
            },
            &user,
        ))
    }

    async fn edit_playlist(&self, playlist: &ItemId, name: &str, description: &str) -> Result<()> {
        self.call(
            "updatePlaylist",
            &[
                ("playlistId", playlist.id.clone()),
                ("name", name.to_owned()),
                ("comment", description.to_owned()),
            ],
        )
        .await
        .map(drop)
    }

    async fn delete_playlist(&self, playlist: &ItemId) -> Result<()> {
        self.call("deletePlaylist", &[("id", playlist.id.clone())])
            .await
            .map(drop)
    }

    /// `entry` is the song: its first place in the playlist goes.
    async fn remove_from_playlist(&self, playlist: &ItemId, entry: &ItemId) -> Result<()> {
        let found = self.playlist_of(playlist).await?;
        let Some(index) = found.entry.iter().position(|s| s.id == entry.id) else {
            return Ok(());
        };
        self.call(
            "updatePlaylist",
            &[
                ("playlistId", playlist.id.clone()),
                ("songIndexToRemove", index.to_string()),
            ],
        )
        .await
        .map(drop)
    }

    async fn add_to_playlist(&self, playlist: &ItemId, tracks: &[ItemId]) -> Result<()> {
        for some in tracks.chunks(SONGS_PER_CALL) {
            let mut query = vec![("playlistId", playlist.id.clone())];
            query.extend(some.iter().map(|t| ("songIdToAdd", t.id.clone())));
            self.call("updatePlaylist", &query).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_cover_is_signed_only_as_it_is_fetched() {
        let auth = "u=me&t=x&s=y&v=1.16.1&c=music";
        assert_eq!(
            super::signed_cover(
                "http://nas",
                auth,
                "http://nas/rest/getCoverArt.view?id=al-1&size=300"
            ),
            Some(
                "http://nas/rest/getCoverArt.view?id=al-1&size=300&u=me&t=x&s=y&v=1.16.1&c=music"
                    .into()
            )
        );
        // Kept from before, signed: signed anew, once.
        assert_eq!(
            super::signed_cover(
                "http://nas",
                auth,
                "http://nas/rest/getCoverArt.view?id=al-1&size=300&u=old&t=o&s=o"
            ),
            Some(
                "http://nas/rest/getCoverArt.view?id=al-1&size=300&u=me&t=x&s=y&v=1.16.1&c=music"
                    .into()
            )
        );
        assert_eq!(
            super::signed_cover("http://nas", auth, "https://is1-ssl.mzstatic.com/a.jpg"),
            None
        );
        assert_eq!(
            super::scrub_kept(
                r#"{"a":"http://nas/rest/getCoverArt.view?id=1&size={size}&u=me&t=x&s=y","b":"https://x/y"}"#
            ),
            r#"{"a":"http://nas/rest/getCoverArt.view?id=1&size={size}","b":"https://x/y"}"#
        );
    }

    use super::*;

    #[test]
    fn the_token_is_the_salted_password_hashed() {
        // Subsonic's own example: password "sesame", salt "c19b2d".
        assert_eq!(
            hex(&Md5::digest(b"sesamec19b2d")),
            "26719a1196d2a940705a59634eb18eab"
        );
        let c = Credentials::new("http://localhost:4533/", " me ", "sesame").unwrap();
        assert_eq!(
            (c.url.as_str(), c.user.as_str()),
            ("http://localhost:4533", "me")
        );
        let bare = Credentials::new("0.0.0.0:4533", "me", "x").unwrap();
        assert_eq!(bare.url, "http://0.0.0.0:4533");
        assert_eq!(c.salt.len(), 16);
        assert_eq!(
            c.token,
            hex(&Md5::digest(format!("sesame{}", c.salt).as_bytes()))
        );
    }

    #[test]
    fn a_file_says_its_format() {
        assert_eq!(extension(Some("audio/flac"), b""), "flac");
        assert_eq!(extension(Some("application/octet-stream"), b"fLaC"), "flac");
        assert_eq!(extension(None, b"\0\0\0\x20ftypM4A "), "m4a");
    }
}
