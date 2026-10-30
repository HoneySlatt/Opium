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
pub mod signin;
mod types;

pub use cenc::AudioStream;
pub use playback::Protected;
pub use radio::LiveStream;

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
/// Recently Added answers at most 25 at a time.
const RECENT_PAGE: u32 = 25;
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
            // A server out of reach, or a download that stalls, fails rather
            // than hang what waits on it.
            .connect_timeout(std::time::Duration::from_secs(10))
            .read_timeout(std::time::Duration::from_secs(30))
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
        let mut waited = false;
        let resp = loop {
            let resp = self
                .authorized(self.http.get(format!("{API}{path}")))
                .header("Media-User-Token", &self.user_token)
                .send()
                .await
                .map_err(|e| Error::Network(format!("{path}: {e}")))?;
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

    /// Send a request that changes the account, and say whether Apple took it.
    async fn send_change(&self, req: reqwest::RequestBuilder, path: &str) -> Result<()> {
        let resp = self
            .authorized(req)
            .header("Media-User-Token", &self.user_token)
            .send()
            .await
            .map_err(|e| Error::Network(format!("{path}: {e}")))?;
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

    /// Every library song, page by page: `each` gets them in order, and
    /// whether that was the last page.
    pub async fn each_library_track(&self, each: impl FnMut(Vec<Track>, bool)) -> Result<()> {
        self.each_page(&library_songs_path(), convert::track, usize::MAX, each)
            .await
    }

    /// Every song of a playlist, as [`each_library_track`](Self::each_library_track).
    pub async fn each_playlist_track(
        &self,
        playlist: &ItemId,
        each: impl FnMut(Vec<Track>, bool),
    ) -> Result<()> {
        let first = self.playlist_tracks_path(playlist);
        self.each_page(&first, convert::track, usize::MAX, each)
            .await
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
        let got = resp.data.len();
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
                break;
            };
            got += resp.data.len();
            next = resp.next.filter(|_| got < limit);
            each(resp.data.into_iter().map(convert).collect(), next.is_none());
        }
        Ok(())
    }
}

/// The page of `next`'s list starting at `offset`, with `first`'s parameters.
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
        self.page(&library_songs_path(), cursor, convert::track)
            .await
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

    async fn recently_added(&self, cursor: Option<&Cursor>) -> Result<Page<Collection>> {
        let page = self
            .page(&recently_added_path(), cursor, convert::collection)
            .await?;
        Ok(Page {
            items: page.items.into_iter().flatten().collect(),
            next: page.next,
        })
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
                None => return Ok(tracks),
            }
        }
    }

    async fn playlist_tracks(
        &self,
        playlist: &ItemId,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Track>> {
        self.page(&self.playlist_tracks_path(playlist), cursor, convert::track)
            .await
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

    async fn remove_from_library(&self, library_id: &ItemId) -> Result<()> {
        let path = format!("/v1/me/library/songs/{}", library_id.id);
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
            .authorized(self.http.post(format!("{API}{path}")))
            .header("Media-User-Token", &self.user_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Network(format!("{path}: {e}")))?;
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

    /// A station's next songs, about `count` of them. Apple hands out two a
    /// call and its calls share a cursor, so they go four at a time and
    /// repeats are dropped (Kopuz measured the trade-off).
    async fn station_tracks(&self, station: &ItemId, count: usize) -> Result<Vec<Track>> {
        let path = format!("/v1/me/stations/next-tracks/{}", station.id);
        let call = || async {
            let resp = self
                .authorized(self.http.post(format!("{API}{path}")))
                .header("Media-User-Token", &self.user_token)
                .header("Content-Length", "0")
                .send()
                .await
                .map_err(|e| Error::Network(format!("{path}: {e}")))?;
            if !resp.status().is_success() {
                return Err(Error::Protocol(format!("{path}: HTTP {}", resp.status())));
            }
            resp.json::<types::Response<types::Song>>()
                .await
                .map_err(|e| Error::Protocol(format!("{path}: {e}")))
        };
        let mut tracks: Vec<Track> = Vec::new();
        let mut barren = 0;
        for _ in 0..8 {
            if tracks.len() >= count {
                break;
            }
            let before = tracks.len();
            let (a, b, c, d) = tokio::join!(call(), call(), call(), call());
            for page in [a, b, c, d].into_iter().flatten() {
                for song in page.data {
                    let track = convert::track(song);
                    if !tracks.iter().any(|t| t.id == track.id) {
                        tracks.push(track);
                    }
                }
            }
            if tracks.len() == before {
                barren += 1;
                if barren == 2 {
                    break;
                }
            }
        }
        if tracks.is_empty() {
            return Err(Error::Playback("the station offered no songs".into()));
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
    /// in the catalog and have none.
    async fn lyrics(&self, track: &ItemId) -> Result<Option<Lyrics>> {
        if track.id.starts_with("i.") {
            return Ok(None);
        }
        for kind in ["syllable-lyrics", "lyrics"] {
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
