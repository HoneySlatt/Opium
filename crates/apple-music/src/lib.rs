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
mod radio;
pub mod signin;
mod stream;
mod types;

pub use radio::LiveStream;
pub use stream::AudioStream;

use async_trait::async_trait;
use music_core::model::{
    Album, Artist, ArtistDetail, Artwork, Collection, Cursor, FavouriteKind, ItemId, Lyrics, Page,
    Pick, Playlist, ProviderKind, SearchResults, Shelf, SongLinks, Track,
};
use music_core::{Error, Provider, Result};
use serde::de::DeserializeOwned;

const API: &str = "https://amp-api.music.apple.com";
const WEB_PLAYER: &str = "https://music.apple.com";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
/// The largest page the library and catalog endpoints accept.
const PAGE: u32 = 100;
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
        // With the tags, which mark Apple's playlist of favourite songs.
        let first = format!(
            "/v1/me/library/playlists?limit={PAGE}&extend%5Blibrary-playlists%5D=tags,inFavorites"
        );
        self.page(&first, cursor, convert::playlist).await
    }

    async fn library_artists(&self, cursor: Option<&Cursor>) -> Result<Page<Artist>> {
        let first = format!("/v1/me/library/artists?limit={PAGE}&include=catalog");
        self.page(&first, cursor, convert::library_artist).await
    }

    async fn recently_added(&self, cursor: Option<&Cursor>) -> Result<Page<Collection>> {
        let first = format!("/v1/me/library/recently-added?limit={RECENT_PAGE}");
        let page = self.page(&first, cursor, convert::collection).await?;
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
                "/v1/me/library/albums/{}/catalog?include=artists&extend=inFavorites&l={NOTES_LANGUAGE}",
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
                "/v1/catalog/{}/albums/{}?include=artists&extend=inFavorites&l={NOTES_LANGUAGE}",
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
            "/v1/catalog/{}/artists/{catalog_id}?views=top-songs,latest-release,full-albums,singles&extend=inFavorites&limit[view.top-songs]=30&limit[view.full-albums]=50&limit[view.singles]=50",
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

    /// As Kopuz makes them: the name, and the songs as the playlist's tracks.
    async fn create_playlist(&self, name: &str, tracks: &[ItemId]) -> Result<Playlist> {
        let path = "/v1/me/library/playlists";
        let data: Vec<serde_json::Value> = tracks
            .iter()
            .map(|t| serde_json::json!({ "id": t.id, "type": "songs" }))
            .collect();
        let body = serde_json::json!({
            "attributes": { "name": name },
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
