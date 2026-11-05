//! Apple resources → the app's unified models.

use std::time::Duration;

use opium_core::model::{
    Album, Artist, Artwork, AudioQuality, Codec, Collection, ItemId, MusicVideo, Playlist,
    ProviderKind, Shelf, Station, StationKind, Track,
};

use crate::types;

fn id(id: impl Into<String>) -> ItemId {
    ItemId::new(ProviderKind::AppleMusic, id)
}

/// Apple sizes art with `{w}x{h}` in the URL.
pub(crate) fn artwork(art: Option<&types::Artwork>) -> Option<Artwork> {
    art.filter(|a| !a.url.is_empty()).map(|a| Artwork {
        // Some (a profile's picture) leave the crop and the format to the
        // asker too: a square, as JPEG.
        url_template: a
            .url
            .replace("{w}", "{size}")
            .replace("{h}", "{size}")
            .replace("{c}", "cc")
            .replace("{f}", "jpg"),
        background: a.bg_color.as_deref().and_then(hex_color),
    })
}

/// `cb6900` → `[0xcb, 0x69, 0x00]`.
/// `rrggbb` as `0xrrggbb`.
fn hex_color(hex: &str) -> Option<u32> {
    u32::from_str_radix(hex.get(..6)?, 16).ok()
}

/// What the web player's `28:ctrp256` flavour delivers, the best Apple serves us.
fn web_quality() -> AudioQuality {
    AudioQuality {
        codec: Codec::Aac,
        sample_rate: Some(44_100),
        bit_depth: None,
        bitrate_kbps: Some(256),
    }
}

/// A catalog song, or a library song: library rows carry a `catalogId` that is the
/// id playback needs. Songs without one are the user's own uploads, played by
/// their library id.
pub fn track(song: types::Song) -> Track {
    let attrs = song.attributes.unwrap_or_default();
    let catalog_id = attrs
        .play_params
        .as_ref()
        .and_then(|p| p.catalog_id.clone())
        .filter(|c| !c.is_empty());
    let in_catalog = catalog_id.is_some() || !song.id.starts_with("i.");
    // Songs read from the library (its songs, playlists, albums) are in it.
    let in_library = song.kind == "library-songs" || song.id.starts_with("i.");
    let library_id = in_library.then(|| id(song.id.clone()));
    let album_id = song
        .relationships
        .as_ref()
        .and_then(|r| r.albums.as_ref())
        .and_then(|a| a.data.first())
        .map(|a| a.id.clone())
        .or_else(|| attrs.url.as_deref().and_then(album_in_url))
        .map(id);

    Track {
        id: id(catalog_id.unwrap_or(song.id)),
        artwork: artwork(attrs.artwork.as_ref()),
        title: attrs.name,
        artists: vec![attrs.artist_name],
        album: attrs.album_name,
        album_id,
        duration: attrs.duration_in_millis.map(Duration::from_millis),
        // A user's upload says 0 when it has no number.
        track_number: attrs.track_number.filter(|&n| n > 0),
        disc_number: attrs.disc_number,
        explicit: attrs.content_rating.as_deref() == Some("explicit"),
        quality: in_catalog.then(web_quality),
        in_library,
        library_id,
    }
}

/// `https://music.apple.com/fr/album/instant-crush/617154241?i=617154362` → `617154241`.
fn album_in_url(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("/album/")?;
    let album = rest.split('/').nth(1)?;
    let album = album.split('?').next()?;
    (!album.is_empty() && album.bytes().all(|b| b.is_ascii_digit())).then(|| album.to_owned())
}

pub fn album(album: types::Album) -> Album {
    let attrs = album.attributes.unwrap_or_default();
    let artist = album
        .relationships
        .as_ref()
        .and_then(|r| r.artists.as_ref())
        .and_then(|a| a.data.first());
    // "Music" is Apple's catch-all genre, listed beside the real one.
    let genre = attrs
        .genre_names
        .iter()
        .find(|g| !matches!(g.as_str(), "Music" | "Musique"))
        .cloned();
    let description = attrs
        .editorial_notes
        .as_ref()
        // The longer notes, as Cider shows them, cut to three lines by the app.
        .and_then(|n| {
            n.standard
                .as_deref()
                .map(strip_tags)
                .or_else(|| n.short.clone())
        })
        .filter(|d| !d.trim().is_empty());
    // A library album says so only through its catalog counterpart.
    let catalog = album
        .relationships
        .as_ref()
        .and_then(|r| r.catalog.as_ref())
        .and_then(|c| c.data.first());
    let catalog_id = catalog.map(|c| id(c.id.clone()));
    let catalog_rating = catalog
        .and_then(|c| c.attributes.as_ref())
        .and_then(|a| a.content_rating.as_deref());
    let explicit =
        attrs.content_rating.as_deref() == Some("explicit") || catalog_rating == Some("explicit");
    let motion = attrs.editorial_video.as_ref().and_then(|v| {
        v.motion_detail_square
            .as_ref()
            .or(v.motion_square_video1x1.as_ref())
            .map(|m| m.video.clone())
            .filter(|url| !url.is_empty())
    });
    Album {
        explicit,
        motion,
        copyright: attrs.copyright.clone(),
        mastered: attrs.is_mastered_for_itunes == Some(true),
        added: attrs.date_added.clone(),
        id: id(album.id),
        artist_id: artist.map(|a| id(a.id.clone())),
        artist_artwork: artwork(
            artist
                .and_then(|a| a.attributes.as_ref())
                .and_then(|a| a.artwork.as_ref()),
        ),
        released: attrs.release_date.clone(),
        catalog_id,
        in_favourites: attrs.in_favorites,
        genre,
        description,
        artwork: artwork(attrs.artwork.as_ref()),
        title: attrs.name,
        artists: vec![attrs.artist_name],
        year: attrs
            .release_date
            .as_deref()
            .and_then(|d| d.get(..4))
            .and_then(|y| y.parse().ok()),
        track_count: attrs.track_count,
    }
}

/// Apple's longer notes carry a little HTML (`<i>`, `<b>`, `<br />`).
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Apple tags the playlist it keeps of the user's favourite songs
/// `favorited` (as Kopuz found); it is named in the storefront's language
/// ("Morceaux préférés"), so the app names it in its own.
pub fn playlist(playlist: types::Playlist) -> Playlist {
    let attrs = playlist.attributes.unwrap_or_default();
    let favourites = attrs
        .tags
        .as_ref()
        .is_some_and(|tags| tags.iter().any(|t| t == "favorited"));
    Playlist {
        id: id(playlist.id),
        artwork: artwork(attrs.artwork.as_ref()),
        name: if favourites {
            "Favourite Songs".into()
        } else {
            attrs.name
        },
        favourites,
        description: attrs.description.and_then(|d| d.standard),
        curator: attrs.curator_name,
        track_count: None,
        catalog_id: attrs.play_params.and_then(|p| p.global_id).map(id),
        in_favourites: attrs.in_favorites,
        editable: attrs.can_edit == Some(true) && !favourites,
        added: attrs.date_added,
    }
}

/// A library artist fetched with `include=catalog`: the catalog artist's id and
/// photo, so the artist page needs no extra lookup.
pub fn library_artist(artist: types::Artist) -> Artist {
    let catalog = artist
        .relationships
        .as_ref()
        .and_then(|r| r.catalog.as_ref())
        .and_then(|c| c.data.first());
    let artwork = artwork(
        catalog
            .and_then(|c| c.attributes.as_ref())
            .and_then(|a| a.artwork.as_ref()),
    );
    let library_id = Some(id(artist.id.clone()));
    let id = id(catalog.map_or_else(|| artist.id.clone(), |c| c.id.clone()));
    Artist {
        id,
        artwork,
        name: artist.attributes.unwrap_or_default().name,
        in_favourites: None,
        library_id,
    }
}

/// An entry of Recently Added; stations and anything else are skipped.
pub fn collection(item: types::Resource<serde_json::Value>) -> Option<Collection> {
    fn typed<A: serde::de::DeserializeOwned>(
        item: types::Resource<serde_json::Value>,
    ) -> types::Resource<A> {
        types::Resource {
            id: item.id,
            kind: item.kind,
            attributes: item.attributes.and_then(|v| serde_json::from_value(v).ok()),
            relationships: item.relationships,
        }
    }
    match item.kind.as_str() {
        "albums" | "library-albums" => Some(Collection::Album(album(typed(item)))),
        "playlists" | "library-playlists" => Some(Collection::Playlist(playlist(typed(item)))),
        "stations" => station(typed(item)).map(Collection::Station),
        _ => None,
    }
}

/// A station the app can play: on air, an episode, or songs. Video stations
/// are left out.
pub fn station(station: types::Resource<types::StationAttributes>) -> Option<Station> {
    let attrs = station.attributes?;
    if attrs.media_kind.as_deref().is_some_and(|k| k != "audio") {
        return None;
    }
    let stream = attrs.play_params.as_ref().and_then(|p| p.format.as_deref()) == Some("stream");
    let kind = match (stream, attrs.is_live) {
        (true, true) => StationKind::Live,
        (true, false) => StationKind::Episode,
        (false, _) => StationKind::Tracks,
    };
    Some(Station {
        id: id(station.id),
        name: attrs.name,
        description: attrs
            .editorial_notes
            .and_then(|n| n.short.or(n.tagline))
            .filter(|d| !d.is_empty()),
        artwork: artwork(attrs.artwork.as_ref()),
        kind,
    })
}

/// A home shelf; anything it holds the app can't show is left out.
pub fn shelf(recommendation: types::Recommendation) -> Option<Shelf> {
    let attrs = recommendation.attributes.unwrap_or_default();
    let items: Vec<Collection> = recommendation
        .relationships
        .and_then(|r| r.contents)
        .map(|c| c.data.into_iter().filter_map(collection).collect())
        .unwrap_or_default();
    let title = attrs
        .title
        .map(|t| t.string_for_display)
        .unwrap_or_default();
    (!items.is_empty() && !title.is_empty()).then(|| Shelf {
        title,
        hero: attrs
            .display
            .is_some_and(|d| d.kind == "MusicNotesHeroShelf"),
        items,
        ..Shelf::default()
    })
}

pub fn artist(artist: types::Artist) -> Artist {
    let attrs = artist.attributes.unwrap_or_default();
    Artist {
        library_id: None,
        id: id(artist.id),
        artwork: artwork(attrs.artwork.as_ref()),
        name: attrs.name,
        in_favourites: attrs.in_favorites,
    }
}

/// An artist's video across the top of their page, and a wide picture for
/// it: the video's first frame, else the editors' banner.
pub fn artist_banner(attrs: &types::ArtistAttributes) -> (Option<String>, Option<String>) {
    let video = attrs.editorial_video.as_ref().and_then(|v| {
        v.motion_artist_fullscreen16x9
            .as_ref()
            .or(v.motion_artist_wide16x9.as_ref())
            .filter(|m| !m.video.is_empty())
    });
    let sized = |a: &types::Artwork, size: &str| {
        (!a.url.is_empty()).then(|| a.url.replace("{w}x{h}", size))
    };
    let banner = video
        .and_then(|v| v.preview_frame.as_ref())
        .and_then(|a| sized(a, "2400x1350"))
        .or_else(|| {
            attrs
                .editorial_artwork
                .as_ref()
                .and_then(|e| e.centered_fullscreen_background.as_ref())
                .and_then(|a| sized(a, "2400x1200"))
        });
    (video.map(|v| v.video.clone()), banner)
}

/// A music video, its still at 16:9.
pub fn music_video(video: types::MusicVideo) -> Option<MusicVideo> {
    let attrs = video.attributes?;
    Some(MusicVideo {
        id: id(video.id),
        title: attrs.name,
        artists: vec![attrs.artist_name],
        thumbnail: attrs
            .artwork
            .filter(|a| !a.url.is_empty())
            .map(|a| a.url.replace("{w}x{h}", "800x450")),
        explicit: attrs.content_rating.as_deref() == Some("explicit"),
        url: attrs.url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_colours_read_from_hex() {
        assert_eq!(hex_color("cb6900"), Some(0xcb6900));
        assert_eq!(hex_color("zz"), None);
    }

    #[test]
    fn notes_lose_their_tags() {
        assert_eq!(strip_tags("A <i>great</i><br />record"), "A great record");
    }

    #[test]
    fn a_song_page_names_its_album() {
        let url = "https://music.apple.com/fr/album/instant-crush/617154241?i=617154362";
        assert_eq!(album_in_url(url).as_deref(), Some("617154241"));
        assert_eq!(album_in_url("https://music.apple.com/fr/song/x/1"), None);
    }

    fn song(json: serde_json::Value) -> types::Song {
        serde_json::from_value(json).expect("song")
    }

    #[test]
    fn an_upload_keeps_its_library_id_and_claims_no_quality() {
        let t = track(song(serde_json::json!({
            "id": "i.upload",
            "attributes": { "name": "Demo", "playParams": { "id": "i.upload", "kind": "song" } }
        })));
        assert_eq!(t.id.id, "i.upload");
        assert!(t.quality.is_none());
    }

    #[test]
    fn an_included_album_becomes_the_album_id() {
        let t = track(song(serde_json::json!({
            "id": "i.abc",
            "attributes": { "name": "x" },
            "relationships": { "albums": { "data": [{ "id": "l.album", "type": "library-albums" }] } }
        })));
        assert_eq!(t.album_id.map(|a| a.id), Some("l.album".to_owned()));
    }
}

/// Conversions of songs, albums and the rest made up for the tests: no title, artist, id or
/// address here belongs to a real release.
#[cfg(test)]
mod made_up_tests {
    use super::*;
    use serde_json::json;

    fn read<T: serde::de::DeserializeOwned>(json: serde_json::Value) -> T {
        serde_json::from_value(json).expect("the made-up resource reads")
    }

    #[test]
    fn a_catalog_song_keeps_its_details_and_the_web_players_quality() {
        let t = track(read(json!({
            "id": "900000001",
            "type": "songs",
            "attributes": {
                "name": "Paper Lanterns Over Nowhere",
                "artistName": "The Imaginary Quartet",
                "albumName": "Songs Nobody Recorded",
                "durationInMillis": 187_250,
                "trackNumber": 3,
                "discNumber": 2,
                "contentRating": "explicit",
                "artwork": { "url": "https://art.example/{w}x{h}bb.{f}", "bgColor": "1a2b3c" },
                "url": "https://music.apple.com/zz/album/songs-nobody-recorded/900000000?i=900000001",
            },
        })));
        assert_eq!(t.id, ItemId::new(ProviderKind::AppleMusic, "900000001"));
        assert_eq!(t.title, "Paper Lanterns Over Nowhere");
        assert_eq!(t.artists, ["The Imaginary Quartet"]);
        assert_eq!(t.album.as_deref(), Some("Songs Nobody Recorded"));
        assert_eq!(t.album_id.map(|a| a.id).as_deref(), Some("900000000"));
        assert_eq!(t.duration, Some(Duration::from_millis(187_250)));
        assert_eq!((t.track_number, t.disc_number), (Some(3), Some(2)));
        assert!(t.explicit);
        assert!(!t.in_library && t.library_id.is_none());
        assert_eq!(t.quality, Some(web_quality()));
        let art = t.artwork.unwrap();
        assert_eq!(art.url_template, "https://art.example/{size}x{size}bb.jpg");
        assert_eq!(art.background, Some(0x1a2b3c));
    }

    #[test]
    fn a_library_song_plays_by_its_catalog_id_and_keeps_its_place() {
        let t = track(read(json!({
            "id": "i.MadeUp42",
            "type": "library-songs",
            "attributes": {
                "name": "A Tune for the Tests",
                "artistName": "Placeholder Band",
                "trackNumber": 0,
                "playParams": { "id": "i.MadeUp42", "kind": "song", "catalogId": "900000777" },
            },
            "relationships": { "albums": { "data": [{ "id": "l.Album7", "type": "library-albums" }] } },
        })));
        assert_eq!(t.id.id, "900000777");
        assert_eq!(t.library_id.map(|l| l.id).as_deref(), Some("i.MadeUp42"));
        assert!(t.in_library);
        assert_eq!(t.album_id.map(|a| a.id).as_deref(), Some("l.Album7"));
        // An upload's 0 is no track number.
        assert_eq!(t.track_number, None);
        assert!(t.quality.is_some());
        assert!(!t.explicit);
    }

    #[test]
    fn a_song_with_nothing_but_its_id_still_converts() {
        let t = track(read(json!({ "id": "900000002" })));
        assert_eq!(t.title, "");
        assert_eq!(t.artists, [""]);
        assert!(t.artwork.is_none() && t.duration.is_none() && t.album_id.is_none());
        let empty_catalog = track(read(json!({
            "id": "i.Mine",
            "attributes": { "playParams": { "catalogId": "" } },
        })));
        assert_eq!(empty_catalog.id.id, "i.Mine");
        assert!(empty_catalog.quality.is_none());
    }

    #[test]
    fn an_album_takes_its_real_genre_its_notes_and_its_catalog_rating() {
        let a = album(read(json!({
            "id": "l.MadeUpAlbum",
            "type": "library-albums",
            "attributes": {
                "name": "Echoes of a Fictional Summer",
                "artistName": "Invented Ensemble",
                "releaseDate": "2019-07-14",
                "trackCount": 11,
                "genreNames": ["Music", "Ambient"],
                "editorialNotes": { "standard": "A <b>calm</b><br />record.", "short": "Calm." },
                "copyright": "℗ 2019 Nobody Records",
                "isMasteredForItunes": true,
                "dateAdded": "2026-01-02T03:04:05Z",
                "editorialVideo": { "motionSquareVideo1x1": { "video": "https://video.example/m.m3u8" } },
                "artwork": { "url": "https://art.example/{w}x{h}.jpg" },
            },
            "relationships": {
                "catalog": { "data": [{ "id": "900000100", "attributes": { "contentRating": "explicit" } }] },
                "artists": { "data": [{ "id": "900000200", "attributes": { "artwork": { "url": "https://art.example/a/{w}x{h}.jpg" } } }] },
            },
        })));
        assert_eq!(a.title, "Echoes of a Fictional Summer");
        assert_eq!(a.genre.as_deref(), Some("Ambient"));
        assert_eq!(a.description.as_deref(), Some("A calm record."));
        assert_eq!(a.year, Some(2019));
        assert_eq!(a.track_count, Some(11));
        assert!(a.explicit && a.mastered);
        assert_eq!(a.catalog_id.map(|c| c.id).as_deref(), Some("900000100"));
        assert_eq!(a.artist_id.map(|c| c.id).as_deref(), Some("900000200"));
        assert!(a.artist_artwork.is_some());
        assert_eq!(a.motion.as_deref(), Some("https://video.example/m.m3u8"));
        assert_eq!(a.added.as_deref(), Some("2026-01-02T03:04:05Z"));
    }

    #[test]
    fn an_album_without_details_has_none() {
        let a = album(read(json!({
            "id": "900000101",
            "attributes": { "genreNames": ["Musique"], "releaseDate": "20", "editorialNotes": { "short": "  " } },
        })));
        assert_eq!(a.genre, None);
        assert_eq!(a.year, None);
        assert_eq!(a.description, None);
        assert!(!a.explicit && !a.mastered && a.motion.is_none());
    }

    #[test]
    fn the_favourite_songs_playlist_is_named_in_the_apps_words_and_kept_apart() {
        let favourites = playlist(read(json!({
            "id": "p.Favs",
            "attributes": { "name": "Morceaux inventés", "tags": ["favorited"], "canEdit": true },
        })));
        assert!(favourites.favourites);
        assert_eq!(favourites.name, "Favourite Songs");
        assert!(!favourites.editable);
        let own = playlist(read(json!({
            "id": "p.Mine",
            "attributes": {
                "name": "Road Trip to Nowhere",
                "description": { "standard": "Made up for a test." },
                "canEdit": true,
                "playParams": { "globalId": "pl.u-MadeUp" },
                "curatorName": "A. Tester",
            },
        })));
        assert!(!own.favourites && own.editable);
        assert_eq!(own.name, "Road Trip to Nowhere");
        assert_eq!(own.description.as_deref(), Some("Made up for a test."));
        assert_eq!(own.catalog_id.map(|c| c.id).as_deref(), Some("pl.u-MadeUp"));
        assert_eq!(own.curator.as_deref(), Some("A. Tester"));
    }

    #[test]
    fn a_library_artist_is_its_catalog_artist_with_its_photo() {
        let a = library_artist(read(json!({
            "id": "r.Lib1",
            "attributes": { "name": "Fictional Singer" },
            "relationships": { "catalog": { "data": [{ "id": "900000300", "attributes": { "artwork": { "url": "https://art.example/s/{w}x{h}.jpg" } } }] } },
        })));
        assert_eq!(a.id.id, "900000300");
        assert_eq!(a.library_id.map(|l| l.id).as_deref(), Some("r.Lib1"));
        assert!(a.artwork.is_some());
        let alone = library_artist(read(json!({ "id": "r.Lib2" })));
        assert_eq!(alone.id.id, "r.Lib2");
        assert!(alone.artwork.is_none());
        let catalog = artist(read(
            json!({ "id": "900000301", "attributes": { "name": "Nobody", "inFavorites": true } }),
        ));
        assert_eq!(
            (catalog.name.as_str(), catalog.in_favourites),
            ("Nobody", Some(true))
        );
        assert!(catalog.library_id.is_none());
    }

    #[test]
    fn recently_added_keeps_albums_playlists_and_audio_stations() {
        let kinds = |json: serde_json::Value| collection(read(json));
        assert!(matches!(
            kinds(json!({ "id": "l.a", "type": "library-albums", "attributes": { "name": "x" } })),
            Some(Collection::Album(_))
        ));
        assert!(matches!(
            kinds(json!({ "id": "p.a", "type": "library-playlists" })),
            Some(Collection::Playlist(_))
        ));
        assert!(matches!(
            kinds(
                json!({ "id": "ra.1", "type": "stations", "attributes": { "name": "Made-up FM" } })
            ),
            Some(Collection::Station(_))
        ));
        assert!(kinds(json!({ "id": "1", "type": "songs" })).is_none());
        // Attributes that don't read are left out, the item kept.
        let Some(Collection::Album(a)) =
            kinds(json!({ "id": "l.b", "type": "albums", "attributes": { "name": 5 } }))
        else {
            panic!("an album");
        };
        assert_eq!(a.title, "");
    }

    #[test]
    fn stations_say_what_they_play() {
        let kind_of = |attributes: serde_json::Value| {
            station(read(json!({ "id": "ra.2", "attributes": attributes }))).map(|s| s.kind)
        };
        assert_eq!(
            kind_of(json!({ "isLive": true, "playParams": { "format": "stream" } })),
            Some(StationKind::Live)
        );
        assert_eq!(
            kind_of(json!({ "playParams": { "format": "stream" } })),
            Some(StationKind::Episode)
        );
        assert_eq!(
            kind_of(json!({ "playParams": { "format": "tracks" } })),
            Some(StationKind::Tracks)
        );
        assert_eq!(kind_of(json!({ "mediaKind": "video" })), None);
        assert_eq!(station(read(json!({ "id": "ra.3" }))), None);
        let s = station(read(json!({ "id": "ra.4", "attributes": {
            "name": "Imaginary Radio", "editorialNotes": { "short": "", "tagline": "Never on air" } } }))).unwrap();
        assert_eq!(
            s.description, None,
            "an empty short line is none, the tagline unused"
        );
        let s = station(read(json!({ "id": "ra.5", "attributes": { "editorialNotes": { "tagline": "Never on air" } } }))).unwrap();
        assert_eq!(s.description.as_deref(), Some("Never on air"));
    }

    #[test]
    fn a_home_shelf_needs_a_title_and_something_to_show() {
        let shelf_of = |json: serde_json::Value| shelf(read(json));
        let full = shelf_of(json!({
            "attributes": { "title": { "stringForDisplay": "Made For Tests" }, "display": { "kind": "MusicNotesHeroShelf" } },
            "relationships": { "contents": { "data": [{ "id": "p.x", "type": "playlists" }, { "id": "1", "type": "songs" }] } },
        }))
        .unwrap();
        assert_eq!(full.title, "Made For Tests");
        assert!(full.hero);
        assert_eq!(full.items.len(), 1);
        assert!(
            shelf_of(json!({ "attributes": { "title": { "stringForDisplay": "Empty" } } }))
                .is_none()
        );
        assert!(shelf_of(json!({ "relationships": { "contents": { "data": [{ "id": "p.x", "type": "playlists" }] } } })).is_none());
    }

    #[test]
    fn an_artists_banner_is_its_videos_first_frame_else_the_editors_picture() {
        let attributes: types::ArtistAttributes = read(json!({
            "editorialVideo": { "motionArtistWide16x9": {
                "video": "https://video.example/wide.m3u8",
                "previewFrame": { "url": "https://art.example/f/{w}x{h}.jpg" } } },
            "editorialArtwork": { "centeredFullscreenBackground": { "url": "https://art.example/b/{w}x{h}.jpg" } },
        }));
        assert_eq!(
            artist_banner(&attributes),
            (
                Some("https://video.example/wide.m3u8".into()),
                Some("https://art.example/f/2400x1350.jpg".into())
            )
        );
        let pictured: types::ArtistAttributes = read(json!({
            "editorialVideo": { "motionArtistWide16x9": { "video": "" } },
            "editorialArtwork": { "centeredFullscreenBackground": { "url": "https://art.example/b/{w}x{h}.jpg" } },
        }));
        assert_eq!(
            artist_banner(&pictured),
            (None, Some("https://art.example/b/2400x1200.jpg".into()))
        );
        assert_eq!(
            artist_banner(&types::ArtistAttributes::default()),
            (None, None)
        );
    }

    #[test]
    fn a_music_video_has_its_still_at_16_by_9() {
        let v = music_video(read(json!({ "id": "900000400", "attributes": {
            "name": "A Video Never Shot", "artistName": "Phantom Act", "contentRating": "explicit",
            "artwork": { "url": "https://art.example/v/{w}x{h}.jpg" }, "url": "https://music.apple.com/zz/music-video/x/900000400" } })))
        .unwrap();
        assert_eq!(v.title, "A Video Never Shot");
        assert_eq!(
            v.thumbnail.as_deref(),
            Some("https://art.example/v/800x450.jpg")
        );
        assert!(v.explicit);
        assert!(music_video(read(json!({ "id": "900000401" }))).is_none());
    }

    #[test]
    fn artwork_without_an_address_is_none() {
        let empty: types::Artwork = read(json!({ "url": "" }));
        assert!(artwork(Some(&empty)).is_none());
        assert!(artwork(None).is_none());
        let crop: types::Artwork =
            read(json!({ "url": "https://art.example/{w}x{h}{c}.{f}", "bgColor": "zz0000" }));
        let art = artwork(Some(&crop)).unwrap();
        assert_eq!(art.url_template, "https://art.example/{size}x{size}cc.jpg");
        assert_eq!(art.background, None);
    }
}
