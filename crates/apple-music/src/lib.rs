//! Apple Music through the web player's private API (`amp-api.music.apple.com`).
//!
//! Neither token comes from an official developer account: the developer token is
//! read from the web player ([`auth`]) and the user token from a browser sign-in
//! ([`signin`]). Request shapes follow Kopuz (<https://github.com/Kopuz-org/kopuz>).

pub mod auth;
mod cenc;
mod convert;
mod lyrics;
mod playback;
pub mod signin;
mod stream;
mod types;

pub use stream::AudioStream;

use async_trait::async_trait;
use music_core::model::{
    Album, ArtistDetail, Cursor, ItemId, Lyrics, Page, Playlist, ProviderKind, SearchResults, Track,
};
use music_core::{Error, Provider, Result};
use serde::de::DeserializeOwned;

const API: &str = "https://amp-api.music.apple.com";
const WEB_PLAYER: &str = "https://music.apple.com";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
/// The largest page the library and catalog endpoints accept.
const PAGE: u32 = 100;
/// Search caps each result type at 25.
const SEARCH_LIMIT: u32 = 25;

pub struct AppleMusic {
    http: reqwest::Client,
    developer_token: String,
    user_token: String,
    storefront: String,
}

impl AppleMusic {
    /// Fetch a developer token and look up the account's storefront (its country),
    /// which every catalog request is scoped to.
    pub async fn connect(user_token: impl Into<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| Error::Network(e.to_string()))?;
        let developer_token = auth::developer_token(&http).await?;
        let mut am = Self {
            http,
            developer_token,
            user_token: user_token.into(),
            storefront: String::new(),
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

    pub fn storefront(&self) -> &str {
        &self.storefront
    }

    /// What every request to Apple carries: both tokens, presented the way the
    /// web player presents them.
    fn authorized(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.bearer_auth(&self.developer_token)
            .header("Origin", WEB_PLAYER)
            .header("Referer", format!("{WEB_PLAYER}/"))
            .header("Cookie", format!("media-user-token={}", self.user_token))
    }

    /// `Ok(None)` on 404, which Apple also answers for some empty collections.
    async fn fetch<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>> {
        let resp = self
            .authorized(self.http.get(format!("{API}{path}")))
            .header("Media-User-Token", &self.user_token)
            .send()
            .await
            .map_err(|e| Error::Network(format!("{path}: {e}")))?;
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

    /// Run a search request; the catalog and the library answer alike.
    async fn search_at(&self, path: &str) -> Result<SearchResults> {
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
            artists: all(r.artists, convert::artist),
            playlists: all(r.playlists, convert::playlist),
        })
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
        self.search_at(&path).await
    }

    async fn search_library(&self, query: &str) -> Result<SearchResults> {
        let term: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
        let path = format!(
            "/v1/me/library/search?term={term}&types=library-songs,library-albums,library-artists,library-playlists&limit={SEARCH_LIMIT}"
        );
        self.search_at(&path).await
    }

    async fn library_tracks(&self, cursor: Option<&Cursor>) -> Result<Page<Track>> {
        let first = format!("/v1/me/library/songs?limit={PAGE}&include=albums");
        self.page(&first, cursor, convert::track).await
    }

    async fn library_albums(&self, cursor: Option<&Cursor>) -> Result<Page<Album>> {
        let first = format!("/v1/me/library/albums?limit={PAGE}");
        self.page(&first, cursor, convert::album).await
    }

    async fn library_playlists(&self, cursor: Option<&Cursor>) -> Result<Page<Playlist>> {
        let first = format!("/v1/me/library/playlists?limit={PAGE}");
        self.page(&first, cursor, convert::playlist).await
    }

    async fn album(&self, album: &ItemId) -> Result<Album> {
        let path = if album.id.starts_with("l.") {
            format!("/v1/me/library/albums/{}", album.id)
        } else {
            format!("/v1/catalog/{}/albums/{}", self.storefront, album.id)
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
                None => return Ok(tracks),
            }
        }
    }

    async fn playlist_tracks(
        &self,
        playlist: &ItemId,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Track>> {
        let first = if playlist.id.starts_with("p.") {
            format!(
                "/v1/me/library/playlists/{}/tracks?limit={PAGE}&include=albums",
                playlist.id
            )
        } else {
            format!(
                "/v1/catalog/{}/playlists/{}/tracks?limit={PAGE}&include=albums",
                self.storefront, playlist.id
            )
        };
        self.page(&first, cursor, convert::track).await
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
            "/v1/catalog/{}/artists/{catalog_id}?views=top-songs,full-albums,singles",
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
        Ok(ArtistDetail {
            artist: convert::artist(types::Artist {
                id: found.id,
                attributes: found.attributes,
                relationships: None,
            }),
            top_songs: all(views.top_songs, convert::track),
            albums: all(views.full_albums, convert::album),
            singles: all(views.singles, convert::album),
        })
    }

    /// Line-timed lyrics, else the word-timed ones read line by line. Uploads
    /// (`i.` ids) are not in the catalog and have none.
    async fn lyrics(&self, track: &ItemId) -> Result<Option<Lyrics>> {
        if track.id.starts_with("i.") {
            return Ok(None);
        }
        for kind in ["lyrics", "syllable-lyrics"] {
            let path = format!("/v1/catalog/{}/songs/{}/{kind}", self.storefront, track.id);
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn the_next_page_wins_where_both_set_a_parameter() {
        let next = carry_query_forward(
            "/v1/me/library/albums?offset=200&limit=50",
            "/v1/me/library/albums?limit=100",
        );
        assert_eq!(next, "/v1/me/library/albums?offset=200&limit=50");
    }
}
