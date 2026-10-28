//! Apple resources → the app's unified models.

use std::time::Duration;

use music_core::model::{
    Album, Artist, Artwork, AudioQuality, Codec, ItemId, Playlist, ProviderKind, Track,
};

use crate::types;

fn id(id: impl Into<String>) -> ItemId {
    ItemId::new(ProviderKind::AppleMusic, id)
}

/// Apple sizes art with `{w}x{h}` in the URL.
fn artwork(art: Option<&types::Artwork>) -> Option<Artwork> {
    art.filter(|a| !a.url.is_empty()).map(|a| Artwork {
        url_template: a.url.replace("{w}", "{size}").replace("{h}", "{size}"),
    })
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

    Track {
        id: id(catalog_id.unwrap_or(song.id)),
        artwork: artwork(attrs.artwork.as_ref()),
        title: attrs.name,
        artists: vec![attrs.artist_name],
        album: attrs.album_name,
        album_id: None,
        duration: attrs.duration_in_millis.map(Duration::from_millis),
        track_number: attrs.track_number,
        disc_number: attrs.disc_number,
        explicit: attrs.content_rating.as_deref() == Some("explicit"),
        quality: in_catalog.then(web_quality),
    }
}

pub fn album(album: types::Album) -> Album {
    let attrs = album.attributes.unwrap_or_default();
    Album {
        id: id(album.id),
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

pub fn playlist(playlist: types::Playlist) -> Playlist {
    let attrs = playlist.attributes.unwrap_or_default();
    Playlist {
        id: id(playlist.id),
        artwork: artwork(attrs.artwork.as_ref()),
        name: attrs.name,
        description: attrs.description.and_then(|d| d.standard),
        track_count: None,
    }
}

pub fn artist(artist: types::Artist) -> Artist {
    let attrs = artist.attributes.unwrap_or_default();
    Artist {
        id: id(artist.id),
        artwork: artwork(attrs.artwork.as_ref()),
        name: attrs.name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(json: serde_json::Value) -> types::Song {
        serde_json::from_value(json).expect("song")
    }

    #[test]
    fn a_library_song_plays_by_its_catalog_id() {
        let t = track(song(serde_json::json!({
            "id": "i.abc",
            "attributes": {
                "name": "Kool-Aid",
                "artistName": "Bring Me The Horizon",
                "durationInMillis": 208_000,
                "contentRating": "explicit",
                "artwork": { "url": "https://is1.mzstatic.com/x/{w}x{h}bb.jpg" },
                "playParams": { "id": "i.abc", "kind": "song", "catalogId": "1811922756" }
            }
        })));
        assert_eq!(t.id.id, "1811922756");
        assert_eq!(t.duration, Some(Duration::from_secs(208)));
        assert!(t.explicit);
        assert_eq!(
            t.artwork.unwrap().url(600),
            "https://is1.mzstatic.com/x/600x600bb.jpg"
        );
        assert_eq!(t.quality.unwrap().bitrate_kbps, Some(256));
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
    fn a_catalog_song_keeps_its_own_id() {
        let t = track(song(serde_json::json!({ "id": "1811922756", "attributes": { "name": "x" } })));
        assert_eq!(t.id.id, "1811922756");
        assert!(t.quality.is_some());
    }
}
