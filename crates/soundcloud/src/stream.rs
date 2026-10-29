//! A track's audio at its best: the uploader's original file when they let it
//! be downloaded and the app wants it, else the best unencrypted AAC stream
//! (256 kbps with Go+, 160 without), its HLS segments fetched side by side
//! and joined into the fragmented MP4 they cut.

use futures_util::StreamExt;
use music_core::model::{AudioQuality, Codec, ItemId};
use music_core::{Error, Result};

use crate::{SoundCloud, types};

/// Segments fetched at once.
const SEGMENTS_AT_ONCE: usize = 6;

/// A track's audio, whole.
pub struct Audio {
    pub bytes: Vec<u8>,
    /// `m4a`, `flac`, `wav`, `aiff`, `mp3`, `ogg`: a hint for the decoder.
    pub extension: &'static str,
    pub quality: AudioQuality,
    /// The uploader's own file, not SoundCloud's transcoding.
    pub original: bool,
}

/// The best unencrypted AAC stream of the full track, if it has one.
pub(crate) fn best_transcoding(track: &types::Track) -> Option<&types::Transcoding> {
    let rank = |t: &types::Transcoding| {
        let tier = match t.quality.as_str() {
            "hq" => 3,
            "sq" => 2,
            _ => 1,
        };
        let kbps: u32 = t
            .preset
            .split('_')
            .nth(1)
            .and_then(|k| k.trim_end_matches('k').parse().ok())
            .unwrap_or(0);
        (tier, kbps)
    };
    track
        .media
        .as_ref()?
        .transcodings
        .iter()
        .filter(|t| {
            t.format.protocol == "hls" && t.format.mime_type.starts_with("audio/mp4") && !t.snipped
        })
        .max_by_key(|t| rank(t))
}

/// Why a track has no stream to play.
fn unplayable(track: &types::Track) -> Error {
    let transcodings = track
        .media
        .as_ref()
        .map(|m| m.transcodings.as_slice())
        .unwrap_or_default();
    let message = if transcodings
        .iter()
        .any(|t| t.format.protocol.contains("encrypted"))
    {
        "this track is encrypted; it plays in a later version"
    } else if transcodings.iter().any(|t| t.snipped) || track.policy.as_deref() == Some("SNIP") {
        "only a preview of this track plays without SoundCloud Go+"
    } else {
        "this track has no stream"
    };
    Error::Playback(format!("SoundCloud: {message}"))
}

/// The format of a downloaded original, from its type, else its first bytes.
fn original_extension(content_type: Option<&str>, bytes: &[u8]) -> &'static str {
    let by_type = content_type.and_then(|t| {
        let t = t.split(';').next()?.trim().to_ascii_lowercase();
        Some(match t.as_str() {
            "audio/wav" | "audio/x-wav" | "audio/wave" | "audio/vnd.wave" => "wav",
            "audio/flac" | "audio/x-flac" => "flac",
            "audio/aiff" | "audio/x-aiff" => "aiff",
            "audio/mpeg" | "audio/mp3" => "mp3",
            "audio/mp4" | "audio/x-m4a" | "audio/m4a" | "audio/aac" => "m4a",
            "audio/ogg" | "application/ogg" => "ogg",
            _ => return None,
        })
    });
    by_type.unwrap_or(match bytes {
        [b'R', b'I', b'F', b'F', ..] => "wav",
        [b'f', b'L', b'a', b'C', ..] => "flac",
        [b'F', b'O', b'R', b'M', ..] => "aiff",
        [b'O', b'g', b'g', b'S', ..] => "ogg",
        [_, _, _, _, b'f', b't', b'y', b'p', ..] => "m4a",
        _ => "mp3",
    })
}

/// The init segment's and the segments' URLs, in order.
fn segments(playlist: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for line in playlist.lines().map(str::trim) {
        if let Some(map) = line.strip_prefix("#EXT-X-MAP:") {
            if let Some(uri) = map.split("URI=\"").nth(1).and_then(|r| r.split('"').next()) {
                urls.insert(0, uri.to_owned());
            }
        } else if !line.is_empty() && !line.starts_with('#') {
            urls.push(line.to_owned());
        }
    }
    urls
}

impl SoundCloud {
    /// `track`'s audio, the original file first when `original` and the
    /// uploader allows it.
    pub async fn audio(&self, track: &ItemId, original: bool) -> Result<Audio> {
        let found: types::Track = self.get(&format!("/tracks/{}", track.id)).await?;
        if original && found.downloadable && found.has_downloads_left {
            match self.original(&found).await {
                Ok(audio) => return Ok(audio),
                // Not for this account or client: the stream will do.
                Err(Error::NotSignedIn | Error::Protocol(_)) => {}
                Err(e) => return Err(e),
            }
        }
        let best = best_transcoding(&found).ok_or_else(|| unplayable(&found))?;
        let mut query = Vec::new();
        if let Some(auth) = &found.track_authorization {
            query.push(("track_authorization", auth.as_str()));
        }
        let at: types::StreamUrl = self.get_with(&best.url, &query).await?;
        let playlist = self.public_text(&at.url).await?;
        let urls = segments(&playlist);
        if urls.is_empty() {
            return Err(Error::Protocol("SoundCloud: an empty stream".into()));
        }
        // The segments' URLs are signed: they go as they are.
        let parts: Vec<Result<Vec<u8>>> = futures_util::stream::iter(urls)
            .map(|url| async move { self.public_bytes(&url).await.map(|(b, _)| b) })
            .buffered(SEGMENTS_AT_ONCE)
            .collect()
            .await;
        let mut bytes = Vec::new();
        for part in parts {
            bytes.extend(part?);
        }
        Ok(Audio {
            bytes,
            extension: "m4a",
            quality: AudioQuality {
                codec: Codec::Aac,
                sample_rate: Some(44_100),
                bit_depth: None,
                bitrate_kbps: Some(if best.quality == "hq" { 256 } else { 160 }),
            },
            original: false,
        })
    }

    /// The file the uploader put up, as it is.
    async fn original(&self, track: &types::Track) -> Result<Audio> {
        let at: types::Download = self.get(&format!("/tracks/{}/download", track.id)).await?;
        let (bytes, content_type) = self.public_bytes(&at.redirect_uri).await?;
        let extension = original_extension(content_type.as_deref(), &bytes);
        let codec = match extension {
            "wav" => Codec::Wav,
            "flac" => Codec::Flac,
            "mp3" => Codec::Mp3,
            "ogg" => Codec::Vorbis,
            other => Codec::Other(other.to_uppercase()),
        };
        Ok(Audio {
            bytes,
            extension,
            quality: AudioQuality {
                codec,
                sample_rate: None,
                bit_depth: None,
                bitrate_kbps: None,
            },
            original: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transcoding(
        preset: &str,
        quality: &str,
        protocol: &str,
        snipped: bool,
    ) -> serde_json::Value {
        serde_json::json!({
            "url": format!("https://api-v2.soundcloud.com/{preset}/{protocol}"),
            "preset": preset, "quality": quality, "snipped": snipped,
            "format": { "protocol": protocol, "mime_type": "audio/mp4; codecs=\"mp4a.40.2\"" }
        })
    }

    fn track(transcodings: Vec<serde_json::Value>) -> types::Track {
        serde_json::from_value(serde_json::json!({
            "id": 1, "media": { "transcodings": transcodings }
        }))
        .unwrap()
    }

    #[test]
    fn the_best_unencrypted_full_stream_is_chosen() {
        let t = track(vec![
            transcoding("aac_96k", "lq", "hls", false),
            transcoding("aac_160k", "sq", "hls", false),
            transcoding("aac_256k", "hq", "ctr-encrypted-hls", false),
            transcoding("aac_256k", "hq", "hls", false),
        ]);
        let best = best_transcoding(&t).unwrap();
        assert_eq!(
            (best.preset.as_str(), best.format.protocol.as_str()),
            ("aac_256k", "hls")
        );
        let free = track(vec![
            transcoding("aac_96k", "lq", "hls", false),
            transcoding("aac_160k", "sq", "hls", false),
        ]);
        assert_eq!(best_transcoding(&free).unwrap().preset, "aac_160k");
    }

    #[test]
    fn encrypted_and_preview_tracks_say_why_they_dont_play() {
        let drm = track(vec![transcoding(
            "aac_160k",
            "sq",
            "ctr-encrypted-hls",
            false,
        )]);
        assert!(best_transcoding(&drm).is_none());
        assert!(unplayable(&drm).to_string().contains("encrypted"));
        let preview = track(vec![transcoding("mp3_0_1", "sq", "hls", true)]);
        assert!(unplayable(&preview).to_string().contains("Go+"));
    }

    #[test]
    fn the_init_segment_comes_first() {
        let playlist = "#EXTM3U\n#EXT-X-MAP:URI=\"https://x/init.mp4?s=1\"\n#EXTINF:10.0,\nhttps://x/data000.m4s?s=1\n#EXTINF:9.9,\nhttps://x/data001.m4s?s=1\n#EXT-X-ENDLIST\n";
        assert_eq!(
            segments(playlist),
            [
                "https://x/init.mp4?s=1",
                "https://x/data000.m4s?s=1",
                "https://x/data001.m4s?s=1"
            ]
        );
    }

    #[test]
    fn originals_are_told_by_type_then_by_their_first_bytes() {
        assert_eq!(original_extension(Some("audio/x-wav"), b""), "wav");
        assert_eq!(
            original_extension(Some("application/octet-stream"), b"fLaC\0"),
            "flac"
        );
        assert_eq!(original_extension(None, b"FORM\0\0\0\0AIFF"), "aiff");
        assert_eq!(original_extension(None, b"\0\0\0\x20ftypM4A "), "m4a");
    }
}
