//! From the Subsonic API's JSON to the app's model.

use std::time::Duration;

use music_core::model::{
    Album, Artist, Artwork, AudioQuality, Codec, ItemId, LyricLine, Lyrics, Playlist, ProviderKind,
    Track,
};

use crate::types;

pub(crate) fn id(id: &str) -> ItemId {
    ItemId::new(ProviderKind::Navidrome, id)
}

/// Where the server's pictures are: their URLs carry the sign-in, as every
/// request does.
pub(crate) struct Urls {
    pub base: String,
    pub auth: String,
}

impl Urls {
    /// A cover by its id, at the size asked.
    pub(crate) fn cover(&self, cover: Option<&str>) -> Option<Artwork> {
        let cover = cover.filter(|c| !c.is_empty())?;
        Some(Artwork {
            url_template: format!(
                "{}/rest/getCoverArt.view?id={cover}&size={{size}}&{}",
                self.base, self.auth
            ),
            background: None,
        })
    }
}

/// What the file is, as the server read it: its format, bits and rate.
fn quality(song: &types::Song) -> Option<AudioQuality> {
    let suffix = song.suffix.as_deref()?.to_ascii_lowercase();
    let lossless_mp4 = suffix == "m4a"
        && song
            .content_type
            .as_deref()
            .is_some_and(|t| t.contains("alac"));
    let codec = match suffix.as_str() {
        "flac" => Codec::Flac,
        "wav" | "aif" | "aiff" => Codec::Wav,
        "mp3" => Codec::Mp3,
        "opus" => Codec::Opus,
        "ogg" | "oga" => Codec::Vorbis,
        "alac" => Codec::Alac,
        "m4a" if lossless_mp4 => Codec::Alac,
        // An m4a that says nothing more is AAC, as most are.
        "m4a" | "aac" | "mp4" => Codec::Aac,
        other => Codec::Other(other.to_uppercase()),
    };
    Some(AudioQuality {
        codec,
        sample_rate: song.sampling_rate.filter(|&r| r > 0),
        bit_depth: song.bit_depth.filter(|&b| b > 0),
        bitrate_kbps: song.bit_rate.filter(|&b| b > 0),
    })
}

/// OpenSubsonic's list of artists, else the one name.
fn artists(list: &[types::Named], one: Option<&str>) -> Vec<String> {
    if !list.is_empty() {
        return list.iter().map(|a| a.name.clone()).collect();
    }
    one.filter(|a| !a.is_empty())
        .map(str::to_owned)
        .into_iter()
        .collect()
}

pub(crate) fn track(urls: &Urls, song: types::Song) -> Track {
    Track {
        id: id(&song.id),
        artists: artists(&song.artists, song.artist.as_deref()),
        album: song.album.clone().filter(|a| !a.is_empty()),
        album_id: song.album_id.as_deref().map(id),
        duration: song.duration.map(Duration::from_secs),
        track_number: song.track.filter(|&n| n > 0),
        disc_number: song.disc_number.filter(|&n| n > 0),
        artwork: urls.cover(song.cover_art.as_deref()),
        explicit: song.explicit_status.as_deref() == Some("explicit"),
        quality: quality(&song),
        // All of the server is the library.
        in_library: true,
        library_id: None,
        title: song.title,
    }
}

/// `YYYY-MM-DD` from OpenSubsonic's release date, else the year alone.
fn released(album: &types::Album) -> Option<String> {
    let date = album.release_date.as_ref()?;
    let year = date.year.filter(|&y| y > 0)?;
    Some(match (date.month, date.day) {
        (Some(m), Some(d)) if m > 0 && d > 0 => format!("{year:04}-{m:02}-{d:02}"),
        _ => format!("{year:04}-01-01"),
    })
}

pub(crate) fn album(urls: &Urls, album: types::Album) -> Album {
    Album {
        id: id(&album.id),
        title: album.name.clone(),
        artists: artists(&album.artists, album.artist.as_deref()),
        artist_id: album
            .artist_id
            .as_deref()
            .or(album.artists.first().map(|a| a.id.as_str()))
            .map(id),
        artist_artwork: None,
        released: released(&album),
        genre: album.genre.clone().filter(|g| !g.is_empty()),
        description: None,
        catalog_id: None,
        in_favourites: Some(album.starred.is_some()),
        year: album.year.filter(|&y| y > 0),
        artwork: urls.cover(album.cover_art.as_deref()),
        track_count: album.song_count,
        explicit: album.explicit_status.as_deref() == Some("explicit"),
        added: album.created.clone(),
        motion: None,
        copyright: None,
        mastered: false,
    }
}

pub(crate) fn artist(urls: &Urls, artist: types::Artist) -> Artist {
    let artwork = urls.cover(artist.cover_art.as_deref()).or_else(|| {
        artist
            .artist_image_url
            .clone()
            .filter(|u| !u.is_empty())
            .map(|url| Artwork {
                url_template: url,
                background: None,
            })
    });
    Artist {
        id: id(&artist.id),
        name: artist.name,
        artwork,
        in_favourites: Some(artist.starred.is_some()),
        library_id: None,
    }
}

pub(crate) fn playlist(urls: &Urls, playlist: types::Playlist, user: &str) -> Playlist {
    Playlist {
        id: id(&playlist.id),
        editable: playlist.owner.as_deref() == Some(user),
        name: playlist.name,
        description: playlist.comment.filter(|c| !c.trim().is_empty()),
        curator: playlist.owner,
        favourites: false,
        artwork: urls.cover(playlist.cover_art.as_deref()),
        track_count: playlist.song_count,
        catalog_id: None,
        in_favourites: None,
    }
}

/// The synced lyrics when there are, else the plain ones.
pub(crate) fn lyrics(list: types::LyricsList) -> Option<Lyrics> {
    let mut all = list.structured_lyrics;
    let at = all.iter().position(|l| l.synced).unwrap_or(0);
    if at >= all.len() {
        return None;
    }
    let chosen = all.swap_remove(at);
    let starts: Vec<Option<Duration>> = chosen
        .line
        .iter()
        .map(|l| l.start.map(Duration::from_millis))
        .collect();
    let lines: Vec<LyricLine> = chosen
        .line
        .into_iter()
        .enumerate()
        .map(|(i, l)| LyricLine {
            start: starts[i].unwrap_or_default(),
            end: starts.get(i + 1).copied().flatten(),
            text: l.value.trim().to_owned(),
        })
        .collect();
    (!lines.is_empty()).then_some(Lyrics {
        lines,
        synced: chosen.synced,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urls() -> Urls {
        Urls {
            base: "http://server".into(),
            auth: "u=me&t=x&s=y".into(),
        }
    }

    #[test]
    fn a_flac_says_its_bits_and_rate() {
        let song: types::Song = serde_json::from_value(serde_json::json!({
            "id": "s1", "title": "One", "suffix": "flac", "bitDepth": 24,
            "samplingRate": 96000, "bitRate": 2400, "duration": 200,
            "artists": [{ "id": "a1", "name": "First" }, { "id": "a2", "name": "Second" }],
            "coverArt": "al-1"
        }))
        .unwrap();
        let t = track(&urls(), song);
        let q = t.quality.unwrap();
        assert_eq!(
            (q.codec, q.bit_depth, q.sample_rate),
            (Codec::Flac, Some(24), Some(96000))
        );
        assert_eq!(t.artists, ["First", "Second"]);
        assert_eq!(
            t.artwork.unwrap().url(300),
            "http://server/rest/getCoverArt.view?id=al-1&size=300&u=me&t=x&s=y"
        );
    }

    #[test]
    fn synced_lyrics_win_and_each_line_ends_where_the_next_starts() {
        let list: types::LyricsList = serde_json::from_value(serde_json::json!({
            "structuredLyrics": [
                { "synced": false, "line": [{ "value": "plain" }] },
                { "synced": true, "line": [
                    { "start": 1000, "value": "first" }, { "start": 4500, "value": "second" }
                ]}
            ]
        }))
        .unwrap();
        let l = lyrics(list).unwrap();
        assert!(l.synced);
        assert_eq!(l.lines[0].end, Some(Duration::from_millis(4500)));
        assert_eq!(l.lines[1].text, "second");
    }
}
