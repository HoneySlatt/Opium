//! What SoundCloud shows beside the library, as its website does: the
//! selections of its home (the mixes made for the user, SoundCloud's own
//! playlists), what the people followed posted, the charts, the stations of
//! the songs played lately, and the genres it keeps a trending list of.

use music_core::Result;
use music_core::model::{
    Collection, Genre, ItemId, Playlist, ProviderKind, Shelf, Station, StationKind, Track,
};
use serde::Deserialize;

use crate::{STATION, SoundCloud, convert, types};

/// SoundCloud's own playlists (the mixes, the trending lists) are named so.
pub(crate) const SYSTEM: &str = "soundcloud:system-playlists:";
/// The trending list of a genre: `…:trending-by-genre:<genre>`.
const TRENDING: &str = "soundcloud:system-playlists:trending-by-genre:";

#[derive(Debug, Deserialize)]
struct Selection {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    items: Option<types::Collection<Item>>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum Item {
    #[serde(rename = "playlist")]
    Playlist(types::Playlist),
    #[serde(rename = "system-playlist")]
    System(SystemPlaylist),
    #[serde(other)]
    Other,
}

/// One of SoundCloud's own playlists: a mix made for the user, a trending list.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct SystemPlaylist {
    pub urn: String,
    pub title: Option<String>,
    pub short_title: Option<String>,
    pub description: Option<String>,
    pub short_description: Option<String>,
    pub artwork_url: Option<String>,
    pub calculated_artwork_url: Option<String>,
    pub tracks: Vec<types::Track>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Played {
    track: Option<types::Track>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Posted {
    #[serde(rename = "type")]
    kind: String,
    track: Option<types::Track>,
    playlist: Option<types::Playlist>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Charted {
    track: Option<types::Track>,
}

/// A system playlist as the apps show any other: SoundCloud's, not the user's.
pub(crate) fn system_playlist(p: &SystemPlaylist) -> Playlist {
    let artwork = p
        .artwork_url
        .as_deref()
        .or(p.calculated_artwork_url.as_deref());
    Playlist {
        id: ItemId::new(ProviderKind::SoundCloud, p.urn.clone()),
        name: p
            .title
            .clone()
            .or_else(|| p.short_title.clone())
            .unwrap_or_default(),
        description: p
            .short_description
            .clone()
            .or_else(|| p.description.clone())
            .filter(|d| !d.trim().is_empty()),
        curator: Some("SoundCloud".into()),
        favourites: false,
        artwork: convert::artwork(artwork)
            .or_else(|| convert::artwork(p.tracks.first().and_then(|t| t.artwork_url.as_deref()))),
        track_count: (!p.tracks.is_empty()).then_some(p.tracks.len() as u32),
        catalog_id: None,
        in_favourites: None,
        editable: false,
        added: None,
    }
}

fn songs(title: &str, songs: Vec<Track>) -> Option<Shelf> {
    (!songs.is_empty()).then(|| Shelf {
        title: title.into(),
        hero: false,
        items: Vec::new(),
        picks: Vec::new(),
        songs,
    })
}

fn covers(title: &str, hero: bool, items: Vec<Collection>) -> Option<Shelf> {
    (!items.is_empty()).then(|| Shelf {
        title: title.into(),
        hero,
        items,
        picks: Vec::new(),
        songs: Vec::new(),
    })
}

impl SoundCloud {
    /// The website's home: its selections, the mixes made for the user first.
    async fn selections(&self) -> Result<Vec<(String, Vec<Collection>)>> {
        let page: types::Collection<Selection> = self
            .get_with("/mixed-selections", &[("limit", "10")])
            .await?;
        Ok(page
            .collection
            .into_iter()
            .map(|s| {
                let items = s
                    .items
                    .map(|i| i.collection)
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|item| match item {
                        Item::Playlist(p) => {
                            Some(Collection::Playlist(convert::playlist(p, Some(self.me.id))))
                        }
                        Item::System(p) => Some(Collection::Playlist(system_playlist(&p))),
                        Item::Other => None,
                    })
                    .collect();
                (s.title.unwrap_or_default(), items)
            })
            .filter(|(title, items): &(String, Vec<Collection>)| {
                !title.is_empty() && !items.is_empty()
            })
            .collect())
    }

    /// The songs played lately, the last first, each once.
    async fn played(&self) -> Result<Vec<types::Track>> {
        let page: types::Collection<Played> = self
            .get_with("/me/play-history/tracks", &[("limit", "50")])
            .await?;
        let mut seen = std::collections::HashSet::new();
        Ok(page
            .collection
            .into_iter()
            .filter_map(|p| p.track)
            .filter(|t| t.title.is_some() && seen.insert(t.id))
            .collect())
    }

    async fn chart(&self, kind: &str) -> Result<Vec<Track>> {
        let page: types::Collection<Charted> = self
            .get_with(
                "/charts",
                &[
                    ("kind", kind),
                    ("genre", "soundcloud:genres:all-music"),
                    ("limit", "50"),
                ],
            )
            .await?;
        Ok(page
            .collection
            .into_iter()
            .filter_map(|c| c.track)
            .filter(|t| t.title.is_some())
            .map(convert::track)
            .collect())
    }

    pub(crate) async fn discover_home(&self) -> Result<Vec<Shelf>> {
        let (selections, played) = tokio::join!(self.selections(), self.played());
        let mut shelves: Vec<Shelf> = played
            .map(|p| {
                songs(
                    "Recently Played",
                    p.into_iter().take(20).map(convert::track).collect(),
                )
            })
            .ok()
            .flatten()
            .into_iter()
            .collect();
        // The first selection (the mixes made for the user) shown large;
        // SoundCloud's own Recently Played left out, the songs above being so.
        let ours: Vec<String> = shelves.iter().map(|s| s.title.to_lowercase()).collect();
        let selections = selections?
            .into_iter()
            .filter(|(title, _)| !ours.contains(&title.to_lowercase()));
        for (i, (title, items)) in selections.enumerate() {
            shelves.extend(covers(&title, i == 0, items));
        }
        Ok(shelves)
    }

    pub(crate) async fn discover_new(&self) -> Result<Vec<Shelf>> {
        let (stream, hot, top) = tokio::join!(
            self.get_with::<types::Collection<Posted>>("/stream", &[("limit", "50")]),
            self.chart("trending"),
            self.chart("top"),
        );
        let mut shelves = Vec::new();
        if let Ok(stream) = stream {
            let mut tracks = Vec::new();
            let mut releases = Vec::new();
            for post in stream.collection {
                // Their own posts, not what they reposted.
                match post.kind.as_str() {
                    "track" => {
                        tracks.extend(post.track.filter(|t| t.title.is_some()).map(convert::track))
                    }
                    "playlist" => releases.extend(post.playlist.map(|p| {
                        if p.is_album {
                            Collection::Album(convert::album(p))
                        } else {
                            Collection::Playlist(convert::playlist(p, Some(self.me.id)))
                        }
                    })),
                    _ => {}
                }
            }
            shelves.extend(songs("New From Who You Follow", tracks));
            shelves.extend(covers("New Releases From Who You Follow", false, releases));
        }
        // The charts aren't everywhere: those missing are left out.
        shelves.extend(songs("New & Hot", hot.unwrap_or_default()));
        shelves.extend(songs("Top 50", top.unwrap_or_default()));
        Ok(shelves)
    }

    /// A station of each song played lately: its songs, then related ones.
    pub(crate) async fn discover_radio(&self) -> Result<Vec<Shelf>> {
        let stations: Vec<Collection> = self
            .played()
            .await?
            .into_iter()
            .take(24)
            .map(|t| {
                let track = convert::track(t.clone());
                Collection::Station(Station {
                    id: ItemId::new(ProviderKind::SoundCloud, format!("{STATION}{}", t.id)),
                    name: track.title.clone(),
                    description: Some(format!("Songs like it · {}", track.artists.join(", "))),
                    artwork: track.artwork,
                    kind: StationKind::Tracks,
                })
            })
            .collect();
        Ok(covers("Stations From Your Listening", true, stations)
            .into_iter()
            .collect())
    }

    /// The genres SoundCloud keeps a trending list of.
    pub(crate) async fn discover_genres(&self) -> Result<Vec<Genre>> {
        let page: types::Collection<Selection> = self
            .get_with("/mixed-selections", &[("limit", "10")])
            .await?;
        let mut genres: Vec<Genre> = page
            .collection
            .into_iter()
            .flat_map(|s| s.items.map(|i| i.collection).unwrap_or_default())
            .filter_map(|item| match item {
                Item::System(p) => {
                    let genre = p.urn.strip_prefix(TRENDING)?.to_owned();
                    let name = p
                        .short_title
                        .clone()
                        .or(p.title.clone())
                        .unwrap_or_else(|| genre.clone());
                    Some(Genre { id: genre, name })
                }
                _ => None,
            })
            .collect();
        genres.dedup_by(|a, b| a.id == b.id);
        Ok(genres)
    }

    /// A genre's trending songs.
    pub(crate) async fn discover_genre(&self, genre: &str) -> Result<Vec<Shelf>> {
        let list = self.system(&format!("{TRENDING}{genre}")).await?;
        Ok(songs(
            "Trending",
            list.tracks.into_iter().map(convert::track).collect(),
        )
        .into_iter()
        .collect())
    }

    /// One of SoundCloud's own playlists, its songs whole.
    pub(crate) async fn system(&self, urn: &str) -> Result<SystemPlaylist> {
        let mut list: SystemPlaylist = self.get(&format!("/system-playlists/{urn}")).await?;
        self.whole_tracks(&mut list.tracks).await?;
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_home_selection_reads_both_kinds_of_playlist_and_skips_the_rest() {
        let json = r#"{"collection":[{"title":"Made for you","kind":"selection","items":{"collection":[
            {"kind":"system-playlist","urn":"soundcloud:system-playlists:weekly:1","title":"Weekly Wave","calculated_artwork_url":"https://i1.sndcdn.com/a-large.jpg","tracks":[{"id":5,"kind":"track"}]},
            {"kind":"playlist","id":7,"title":"Emerging","tracks":[]},
            {"kind":"user","id":3}
        ],"next_href":null}}],"next_href":null}"#;
        let page: types::Collection<Selection> = serde_json::from_str(json).unwrap();
        let items = page.collection.into_iter().next().unwrap().items.unwrap().collection;
        assert_eq!(items.len(), 3);
        let Item::System(weekly) = &items[0] else { panic!("a system playlist") };
        let shown = system_playlist(weekly);
        assert_eq!(shown.id.id, "soundcloud:system-playlists:weekly:1");
        assert_eq!(shown.name, "Weekly Wave");
        assert!(shown.artwork.unwrap().url_template.contains("t500x500"));
        assert!(matches!(items[1], Item::Playlist(_)));
        assert!(matches!(items[2], Item::Other));
    }
}
