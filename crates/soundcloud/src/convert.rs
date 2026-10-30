//! From SoundCloud's JSON to the app's model.

use std::time::Duration;

use music_core::model::{
    Album, Artist, Artwork, AudioQuality, Codec, ItemId, Playlist, ProviderKind, Track,
};

use crate::types;

/// The id of a track listed as a single: its own page holds it alone.
pub(crate) const SINGLE: &str = "single:";

pub(crate) fn id(id: u64) -> ItemId {
    ItemId::new(ProviderKind::SoundCloud, id.to_string())
}

/// SoundCloud serves a few fixed sizes (`-large` is 100 px): the largest
/// square one, 500 px, will do everywhere.
fn artwork(url: Option<&str>) -> Option<Artwork> {
    let url = url.filter(|u| !u.is_empty())?;
    Some(Artwork {
        url_template: url.replace("-large.", "-t500x500."),
        background: None,
    })
}

/// The best the track offers, as [`crate::stream`] picks it: its original
/// file when the uploader lets it be downloaded (of a format known only once
/// it comes), else AAC at 256 (Go+) or 160 kbps, encrypted or not.
fn quality(track: &types::Track) -> Option<AudioQuality> {
    if track.downloadable && track.has_downloads_left {
        return Some(AudioQuality {
            codec: Codec::Other("Original".into()),
            sample_rate: None,
            bit_depth: None,
            bitrate_kbps: None,
        });
    }
    let best =
        crate::stream::best_transcoding(track).or_else(|| crate::stream::best_encrypted(track))?;
    Some(AudioQuality {
        codec: Codec::Aac,
        sample_rate: Some(44_100),
        bit_depth: None,
        bitrate_kbps: Some(if best.quality == "hq" { 256 } else { 160 }),
    })
}

pub(crate) fn track(track: types::Track) -> Track {
    let meta = track.publisher_metadata.clone().unwrap_or_default();
    let uploader = track.user.as_ref().map(|u| u.username.clone());
    let artist = meta
        .artist
        .clone()
        .filter(|a| !a.trim().is_empty())
        .or(uploader);
    Track {
        id: id(track.id),
        title: track.title.clone().unwrap_or_default(),
        artists: artist.into_iter().collect(),
        album: meta.album_title.clone().filter(|a| !a.trim().is_empty()),
        album_id: None,
        duration: track.duration.map(Duration::from_millis),
        track_number: None,
        disc_number: None,
        artwork: artwork(
            track
                .artwork_url
                .as_deref()
                .or(track.user.as_ref().and_then(|u| u.avatar_url.as_deref())),
        ),
        explicit: meta.explicit.unwrap_or(false),
        quality: quality(&track),
        in_library: false,
        library_id: None,
    }
}

pub(crate) fn playlist(playlist: types::Playlist, mine: Option<u64>) -> Playlist {
    let owner = playlist.user.as_ref();
    let editable = mine.is_some_and(|me| owner.is_some_and(|u| u.id == me));
    Playlist {
        id: id(playlist.id),
        name: playlist.title,
        description: playlist.description.filter(|d| !d.trim().is_empty()),
        curator: owner.map(|u| u.username.clone()),
        favourites: false,
        // Without its own, a playlist shows its first track's.
        artwork: artwork(
            playlist
                .artwork_url
                .as_deref()
                .or_else(|| playlist.tracks.first()?.artwork_url.as_deref()),
        ),
        track_count: playlist.track_count,
        catalog_id: None,
        in_favourites: None,
        editable,
    }
}

/// An album, which SoundCloud keeps as a playlist marked so.
pub(crate) fn album(album: types::Playlist) -> Album {
    let released = album
        .release_date
        .clone()
        .or(album.published_at.clone())
        .and_then(|d| d.get(..10).map(str::to_owned));
    let owner = album.user.as_ref();
    Album {
        id: id(album.id),
        title: album.title.clone(),
        artists: owner.map(|u| u.username.clone()).into_iter().collect(),
        artist_id: owner.map(|u| id(u.id)),
        artist_artwork: artwork(owner.and_then(|u| u.avatar_url.as_deref())),
        year: released.as_deref().and_then(|d| d.get(..4)?.parse().ok()),
        released,
        genre: None,
        description: album.description.clone().filter(|d| !d.trim().is_empty()),
        catalog_id: None,
        in_favourites: None,
        artwork: artwork(
            album
                .artwork_url
                .as_deref()
                .or_else(|| album.tracks.first()?.artwork_url.as_deref()),
        ),
        track_count: album.track_count,
        explicit: false,
        added: None,
        motion: None,
        copyright: None,
        mastered: false,
    }
}

/// A track on its own, as an artist's page lists their uploads among
/// singles, dated as SoundCloud shows it.
pub(crate) fn single(single: types::Track) -> Album {
    let released = single
        .display_date
        .clone()
        .or(single.release_date.clone())
        .and_then(|d| d.get(..10).map(str::to_owned));
    let owner = single.user.clone();
    let genre = single.genre.clone().filter(|g| !g.trim().is_empty());
    let id = ItemId::new(ProviderKind::SoundCloud, format!("{SINGLE}{}", single.id));
    let single = track(single);
    Album {
        id,
        title: single.title,
        artists: single.artists,
        artist_id: owner.as_ref().map(|u| self::id(u.id)),
        artist_artwork: artwork(owner.as_ref().and_then(|u| u.avatar_url.as_deref())),
        year: released.as_deref().and_then(|d| d.get(..4)?.parse().ok()),
        released,
        genre,
        description: None,
        catalog_id: None,
        in_favourites: None,
        artwork: single.artwork,
        track_count: Some(1),
        explicit: single.explicit,
        added: None,
        motion: None,
        copyright: None,
        mastered: false,
    }
}

pub(crate) fn user(user: types::User) -> Artist {
    Artist {
        id: id(user.id),
        name: user.username,
        artwork: artwork(user.avatar_url.as_deref()),
        in_favourites: None,
        library_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artwork_is_asked_for_at_500_pixels() {
        let art = artwork(Some(
            "https://i1.sndcdn.com/artworks-000067273316-smsiqx-large.jpg",
        ));
        assert_eq!(
            art.map(|a| a.url(600)).as_deref(),
            Some("https://i1.sndcdn.com/artworks-000067273316-smsiqx-t500x500.jpg")
        );
    }

    #[test]
    fn the_credited_artist_wins_over_the_uploader() {
        let t: types::Track = serde_json::from_value(serde_json::json!({
            "id": 293, "title": "Flickermood", "duration": 213886,
            "user": { "id": 183, "username": "forss" },
            "publisher_metadata": { "artist": "Forss" }
        }))
        .unwrap();
        let t = track(t);
        assert_eq!(t.artists, ["Forss"]);
        assert_eq!(t.duration, Some(Duration::from_millis(213_886)));
    }
}
