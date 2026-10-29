//! SoundCloud, through the web player's own API (`api-v2`): the official one
//! takes no new apps. Requests carry the web player's client id, read from
//! soundcloud.com (see [`client_id`]), and the user's token from the
//! `oauth_token` cookie of a browser sign-in (see [`signin`]). That token now
//! lasts a day or so: it is renewed the way the web does, with the
//! `oauth_refresh_token` cookie, and the new pair handed back to be kept.

mod client_id;
mod convert;
pub mod signin;
mod stream;
mod types;

use std::sync::{Mutex, RwLock};

use async_trait::async_trait;
use music_core::model::{
    Album, Artist, ArtistDetail, Collection, Cursor, ItemId, Page, Playlist, ProviderKind,
    SearchResults, SongLinks, Track,
};
use music_core::{Error, Provider, Result};
use serde::de::DeserializeOwned;

pub use stream::Audio;

const API: &str = "https://api-v2.soundcloud.com";
const WEB: &str = "https://soundcloud.com";
/// Where tokens are issued and renewed.
const SECURE: &str = "https://secure.soundcloud.com";
/// As the web player says it is: some of the API turns other clients away.
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
/// Items a page of a list asks for.
const PAGE: u32 = 200;
/// Tracks `tracks?ids=` takes at once.
const IDS_AT_ONCE: usize = 50;

/// The user's tokens, as the browser keeps them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tokens {
    pub oauth: String,
    pub refresh: Option<String>,
}

/// Told the tokens each time they are renewed, to keep them.
pub type OnTokens = Box<dyn Fn(&Tokens) + Send + Sync>;

pub struct SoundCloud {
    http: reqwest::Client,
    client_id: RwLock<String>,
    tokens: Mutex<Tokens>,
    on_tokens: OnTokens,
    /// The signed-in user.
    me: types::User,
}

impl SoundCloud {
    /// Sign in with `tokens`, renewing them first when they have expired;
    /// `on_tokens` is told every new pair. Without a token, only what anyone
    /// can hear is there.
    pub async fn connect(tokens: Tokens, on_tokens: OnTokens) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| Error::Network(e.to_string()))?;
        let client_id = client_id::scrape(&http).await?;
        let mut sc = Self {
            http,
            client_id: RwLock::new(client_id),
            tokens: Mutex::new(tokens),
            on_tokens,
            me: types::User::default(),
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
    /// transcoding's), with `query` and the client id. A refused token is
    /// renewed, a refused client id read again, once each.
    async fn get_with<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T> {
        let url = if path.starts_with("https://") {
            path.to_owned()
        } else {
            format!("{API}{path}")
        };
        let (mut renewed, mut rescraped) = (false, false);
        loop {
            let mut request = self
                .http
                .get(&url)
                .query(&[("client_id", self.client_id().as_str())])
                .query(query);
            let token = self.token();
            if !token.is_empty() {
                request = request.header("Authorization", format!("OAuth {token}"));
            }
            let resp = request
                .header("Origin", WEB)
                .header("Referer", format!("{WEB}/"))
                .send()
                .await
                .map_err(|e| Error::Network(format!("{url}: {e}")))?;
            match resp.status().as_u16() {
                401 if !renewed => {
                    renewed = true;
                    self.renew().await?;
                }
                401 => return Err(Error::NotSignedIn),
                403 if !rescraped => {
                    rescraped = true;
                    let id = client_id::scrape(&self.http).await?;
                    *self.client_id.write().unwrap_or_else(|e| e.into_inner()) = id;
                }
                429 => {
                    return Err(Error::Protocol(
                        "SoundCloud asks to slow down: try again in a few minutes".into(),
                    ));
                }
                s if !(200..300).contains(&s) => {
                    return Err(Error::Protocol(format!("{url}: HTTP {s}")));
                }
                _ => {
                    let body = resp
                        .text()
                        .await
                        .map_err(|e| Error::Network(format!("{url}: {e}")))?;
                    return serde_json::from_str(&body)
                        .map_err(|e| Error::Protocol(format!("{url}: {e}")));
                }
            }
        }
    }

    /// Renew the token with the refresh token, as the web player's server
    /// does: OAuth's `refresh_token` grant, for the app the token was issued
    /// to (its `client_id` claim). A refresh token serves once; both come
    /// back new.
    async fn renew(&self) -> Result<()> {
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
            .map_err(|e| Error::Network(format!("SoundCloud sign-in: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::NotSignedIn);
        }
        let grant: types::Grant = resp
            .json()
            .await
            .map_err(|e| Error::Protocol(format!("SoundCloud sign-in: {e}")))?;
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
            .map_err(|e| Error::Network(format!("SoundCloud stream: {e}")))?;
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
            .map_err(|e| Error::Network(format!("SoundCloud stream: {e}")))?;
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
        let (tracks, users, playlists, albums) = tokio::try_join!(
            self.get_with::<types::Collection<types::Track>>("/search/tracks", &q),
            self.get_with::<types::Collection<types::User>>("/search/users", &q),
            self.get_with::<types::Collection<types::Playlist>>(
                "/search/playlists_without_albums",
                &q
            ),
            self.get_with::<types::Collection<types::Playlist>>("/search/albums", &q),
        )?;
        let me = Some(self.me.id);
        Ok(SearchResults {
            tracks: tracks.collection.into_iter().map(convert::track).collect(),
            albums: albums.collection.into_iter().map(convert::album).collect(),
            artists: users.collection.into_iter().map(convert::user).collect(),
            playlists: playlists
                .collection
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
                .map(|t| Track {
                    in_library: true,
                    ..convert::track(t)
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

    async fn recently_added(&self, _cursor: Option<&Cursor>) -> Result<Page<Collection>> {
        Ok(Page {
            items: Vec::new(),
            next: None,
        })
    }

    /// SoundCloud's albums are playlists marked so.
    async fn album(&self, album: &ItemId) -> Result<Album> {
        let playlist: types::Playlist = self.get(&format!("/playlists/{}", album.id)).await?;
        Ok(convert::album(playlist))
    }

    async fn album_tracks(&self, album: &ItemId) -> Result<Vec<Track>> {
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
        let (top, albums) = tokio::try_join!(
            self.get_with::<types::Collection<types::Track>>(&top_path, &limit),
            self.get_with::<types::Collection<types::Playlist>>(&albums_path, &limit),
        )?;
        Ok(ArtistDetail {
            artist: convert::user(user),
            top_songs: top.collection.into_iter().map(convert::track).collect(),
            latest: None,
            albums: albums.collection.into_iter().map(convert::album).collect(),
            singles: Vec::new(),
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
            station: None,
        })
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
