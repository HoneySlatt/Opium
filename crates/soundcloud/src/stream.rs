//! A track's audio at its best: the uploader's original file when they let it
//! be downloaded and the app wants it, else the best AAC stream (256 kbps
//! with Go+, 160 without), its HLS segments fetched side by side and joined
//! into the fragmented MP4 they cut. A major label's tracks come only
//! encrypted (CENC): the browser's Widevine CDM is licensed for them by
//! SoundCloud, as in its web player, and decrypts them as they are read.

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use base64::Engine;

use futures_util::StreamExt;
use opium_core::growing::{Growing, growing};
use opium_core::model::{AudioQuality, Codec, ItemId};
use opium_core::{Error, Result};

use crate::{SoundCloud, types};

/// Segments fetched at once.
const SEGMENTS_AT_ONCE: usize = 6;
/// Where an encrypted track's Widevine licence comes from.
const LICENSE: &str = "https://license.media-streaming.soundcloud.cloud/playback/widevine";
/// Widevine's system id, as HLS names a key's format.
const WIDEVINE: &str = "urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed";
/// How long an AAC frame plays, at the 44.1 kHz SoundCloud streams.
const AAC_FRAME: Duration = Duration::from_nanos(1_000_000_000 * 1024 / 44_100);

/// A track's audio: whole, or still coming in.
pub struct Audio {
    pub reader: Reader,
    /// Its length in bytes, all of it.
    pub len: u64,
    /// `m4a`, `flac`, `wav`, `aiff`, `mp3`, `ogg`: a hint for the decoder.
    pub extension: &'static str,
    pub quality: AudioQuality,
    /// The uploader's own file, not SoundCloud's transcoding.
    pub original: bool,
}

/// Where the audio is read from: memory, a download still going on, or an
/// encrypted track decrypting.
pub enum Reader {
    Whole(std::io::Cursor<Vec<u8>>),
    Growing(Growing),
    Decrypting(cenc::AudioStream),
}

impl Reader {
    /// For a decrypting track, what to tell before a seek, so that it
    /// decrypts from there.
    pub fn seek_hint(&self) -> Option<impl FnMut(Duration) + Send + 'static> {
        match self {
            Self::Decrypting(r) => Some(r.seek_hint()),
            _ => None,
        }
    }
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Whole(r) => r.read(buf),
            Self::Growing(r) => r.read(buf),
            Self::Decrypting(r) => r.read(buf),
        }
    }
}

impl Seek for Reader {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        match self {
            Self::Whole(r) => r.seek(to),
            Self::Growing(r) => r.seek(to),
            Self::Decrypting(r) => r.seek(to),
        }
    }
}

/// The best unencrypted AAC stream of the full track, if it has one.
pub(crate) fn best_transcoding(track: &types::Track) -> Option<&types::Transcoding> {
    best_of(track, "hls")
}

/// The best Widevine-encrypted (CENC) AAC stream of the full track.
pub(crate) fn best_encrypted(track: &types::Track) -> Option<&types::Transcoding> {
    best_of(track, "ctr-encrypted-hls")
}

/// The best full AAC stream of `protocol`.
fn best_of<'a>(track: &'a types::Track, protocol: &str) -> Option<&'a types::Transcoding> {
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
            t.format.protocol == protocol
                && t.format.mime_type.starts_with("audio/mp4")
                && !t.snipped
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
        "this track is encrypted, and no Widevine module was found: install Brave, Chrome or Firefox"
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

/// The Widevine `pssh` box an encrypted playlist carries, in its key's data
/// URI.
fn widevine_pssh(playlist: &str) -> Option<Vec<u8>> {
    let line = playlist
        .lines()
        .find(|l| l.starts_with("#EXT-X-KEY:") && l.contains(WIDEVINE))?;
    let uri = line.split("URI=\"").nth(1)?.split('"').next()?;
    let data = uri.rsplit_once("base64,")?.1;
    base64::engine::general_purpose::STANDARD.decode(data).ok()
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

/// A stream's quality: AAC at 44.1 kHz, 256 kbps with Go+, 160 without.
fn stream_quality(best: &types::Transcoding) -> AudioQuality {
    AudioQuality {
        codec: Codec::Aac,
        sample_rate: Some(44_100),
        bit_depth: None,
        bitrate_kbps: Some(if best.quality == "hq" { 256 } else { 160 }),
    }
}

/// Where a track plays from, for a player that reads HLS and files, and
/// asks for Widevine licences, itself (Android's).
#[derive(Debug, Clone)]
pub struct Playable {
    pub url: String,
    /// An HLS playlist; else the uploader's file, whole.
    pub hls: bool,
    /// An encrypted stream's: what its licence is asked with.
    pub license_token: Option<String>,
    /// The stream's; an original's is known once it is read.
    pub quality: Option<AudioQuality>,
    pub original: bool,
}

impl SoundCloud {
    /// Where `track` plays from: the uploader's file when `original` and
    /// they allow it, else the best stream, encrypted when it is only so.
    pub async fn playable(&self, track: &ItemId, original: bool) -> Result<Playable> {
        let found: types::Track = self.get(&format!("/tracks/{}", track.id)).await?;
        if original && found.downloadable && found.has_downloads_left {
            // Not for this account or client: the stream will do.
            if let Ok(at) = self
                .get::<types::Download>(&format!("/tracks/{}/download", found.id))
                .await
            {
                return Ok(Playable {
                    url: at.redirect_uri,
                    hls: false,
                    license_token: None,
                    quality: None,
                    original: true,
                });
            }
        }
        if let Some(best) = best_transcoding(&found) {
            let at: types::StreamUrl = self.resolve(&found, best).await?;
            return Ok(Playable {
                url: at.url,
                hls: true,
                license_token: None,
                quality: Some(stream_quality(best)),
                original: false,
            });
        }
        let Some(best) = best_encrypted(&found) else {
            return Err(unplayable(&found));
        };
        let at: types::StreamUrl = self.resolve(&found, best).await?;
        let token = at.license_auth_token.ok_or_else(|| {
            Error::Playback("SoundCloud: an encrypted stream without a licence".into())
        })?;
        Ok(Playable {
            url: at.url,
            hls: true,
            license_token: Some(token),
            quality: Some(stream_quality(best)),
            original: false,
        })
    }

    /// A Widevine licence for an encrypted stream's `challenge`.
    pub async fn widevine_license(&self, token: &str, challenge: Vec<u8>) -> Result<Vec<u8>> {
        self.license(token, challenge).await
    }

    /// `track`'s audio, the original file first when `original` and the
    /// uploader allows it; an encrypted track needs `cdm`.
    pub async fn audio(
        &self,
        track: &ItemId,
        original: bool,
        cdm: Option<&widevine::Cdm>,
    ) -> Result<Audio> {
        let found: types::Track = self.get(&format!("/tracks/{}", track.id)).await?;
        if original && found.downloadable && found.has_downloads_left {
            match self.original(&found).await {
                Ok(audio) => return Ok(audio),
                // Not for this account or client: the stream will do.
                Err(Error::NotSignedIn | Error::Protocol(_)) => {}
                Err(e) => return Err(e),
            }
        }
        let quality = stream_quality;
        if let Some(best) = best_transcoding(&found) {
            let at: types::StreamUrl = self.resolve(&found, best).await?;
            let playlist = self.public_text(&at.url).await?;
            let bytes = self.joined(&playlist).await?;
            return Ok(Audio {
                len: bytes.len() as u64,
                reader: Reader::Whole(std::io::Cursor::new(bytes)),
                extension: "m4a",
                quality: quality(best),
                original: false,
            });
        }
        let (Some(best), Some(cdm)) = (best_encrypted(&found), cdm) else {
            return Err(unplayable(&found));
        };
        let at: types::StreamUrl = self.resolve(&found, best).await?;
        let token = at.license_auth_token.clone().ok_or_else(|| {
            Error::Playback("SoundCloud: an encrypted stream without a licence".into())
        })?;
        let playlist = self.public_text(&at.url).await?;
        let pssh = widevine_pssh(&playlist)
            .ok_or_else(|| Error::Playback("SoundCloud: no Widevine key in the stream".into()))?;
        let (session, challenge) = cdm
            .open_session(&pssh)
            .map_err(|e| Error::Playback(e.to_string()))?;
        let (mut data, license) =
            tokio::try_join!(self.joined(&playlist), self.license(&token, challenge))?;
        let (movie, samples) = cenc::index(&data)
            .map_err(|e| Error::Playback(format!("SoundCloud: the stream: {e}")))?;
        // The licence is ready once the key the stream's samples name is usable.
        let key_id = samples
            .iter()
            .find(|s| s.protected)
            .map(|s| s.key_id)
            .ok_or_else(|| {
                Error::Playback("SoundCloud: no encrypted samples in the stream".into())
            })?;
        session
            .give_licence(&license, &key_id)
            .map_err(|e| Error::Playback(e.to_string()))?;
        cenc::make_clear(&mut data, &movie, &samples);
        // The content key never leaves the CDM: it decrypts each sample in place, by the key id
        // its sample entry names.
        let decrypt = Box::new(move |bytes: &mut [u8], sample: &cenc::Sample<'_>| {
            let protected = widevine::Protected {
                key_id: &sample.key_id,
                iv: &sample.iv,
                subsamples: sample.subsamples,
                scheme: widevine::Scheme::Cenc,
            };
            session
                .decrypt(bytes, &protected)
                .map_err(|e| e.to_string())
        });
        let len = data.len() as u64;
        let stream = cenc::AudioStream::decrypting(data, samples, AAC_FRAME, decrypt);
        Ok(Audio {
            reader: Reader::Decrypting(stream),
            len,
            extension: "m4a",
            quality: quality(best),
            original: false,
        })
    }

    /// Where a transcoding plays from, for this track.
    async fn resolve(
        &self,
        track: &types::Track,
        transcoding: &types::Transcoding,
    ) -> Result<types::StreamUrl> {
        let mut query = Vec::new();
        if let Some(auth) = &track.track_authorization {
            query.push(("track_authorization", auth.as_str()));
        }
        self.get_with(&transcoding.url, &query).await
    }

    /// A playlist's segments, fetched side by side and joined in order.
    async fn joined(&self, playlist: &str) -> Result<Vec<u8>> {
        let urls = segments(playlist);
        if urls.is_empty() {
            return Err(Error::Protocol("SoundCloud: an empty stream".into()));
        }
        // The segments' URLs are signed: they go as they are.
        let parts: Vec<Result<Vec<u8>>> = futures_util::stream::iter(urls)
            .map(|url| async move {
                // One of some sixty failing is a blip: tried twice more,
                // rather than lose the song for it.
                let mut tries = 0;
                loop {
                    match self.public_bytes(&url).await {
                        Ok((bytes, _)) => break Ok(bytes),
                        Err(_) if tries < 2 => {
                            tries += 1;
                            tokio::time::sleep(std::time::Duration::from_millis(500 * tries)).await;
                        }
                        Err(e) => break Err(e),
                    }
                }
            })
            .buffered(SEGMENTS_AT_ONCE)
            .collect()
            .await;
        let mut bytes = Vec::new();
        for part in parts {
            bytes.extend(part?);
        }
        Ok(bytes)
    }

    /// The licence for the CDM's `challenge`, as the web player asks for it.
    async fn license(&self, token: &str, challenge: Vec<u8>) -> Result<Vec<u8>> {
        let failed =
            |e: reqwest::Error| Error::Network(format!("SoundCloud licence: {}", e.without_url()));
        let resp = self
            .http
            .post(LICENSE)
            .query(&[("license_token", token)])
            .header("Content-Type", "application/octet-stream")
            .header("Origin", crate::WEB)
            .header("Referer", format!("{}/", crate::WEB))
            .body(challenge)
            .send()
            .await
            .map_err(failed)?;
        if !resp.status().is_success() {
            return Err(Error::Playback(format!(
                "SoundCloud licence: HTTP {}",
                resp.status()
            )));
        }
        resp.bytes().await.map(|b| b.to_vec()).map_err(failed)
    }

    /// The file the uploader put up, as it is, read as it downloads: its
    /// first bytes say what it is.
    async fn original(&self, track: &types::Track) -> Result<Audio> {
        let at: types::Download = self.get(&format!("/tracks/{}/download", track.id)).await?;
        let failed =
            |e: reqwest::Error| Error::Network(format!("SoundCloud download: {}", e.without_url()));
        let mut resp = self
            .http
            .get(&at.redirect_uri)
            .send()
            .await
            .map_err(failed)?;
        if !resp.status().is_success() {
            return Err(Error::Protocol(format!(
                "SoundCloud download: HTTP {}",
                resp.status()
            )));
        }
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|t| t.to_str().ok())
            .map(str::to_owned);
        let len = resp.content_length();
        let first = resp.chunk().await.map_err(failed)?.unwrap_or_default();
        let extension = original_extension(content_type.as_deref(), &first);
        let (reader, len) = match len {
            Some(len) => {
                let (growing, feed) = growing(len);
                feed.push(&first);
                let again = self.http.get(&at.redirect_uri);
                let received = first.len() as u64;
                tokio::spawn(async move {
                    // The reader gone, so is the need: `push` says so.
                    let ended = crate::resume::carry_on(
                        resp,
                        received,
                        |from| crate::resume::rest_of(&again, from),
                        |bytes| feed.push(bytes),
                    )
                    .await;
                    feed.end(ended.err());
                });
                (Reader::Growing(growing), len)
            }
            // Of no told length: whole, then.
            None => {
                let mut bytes = first.to_vec();
                while let Some(chunk) = resp.chunk().await.map_err(failed)? {
                    bytes.extend_from_slice(&chunk);
                }
                let len = bytes.len() as u64;
                (Reader::Whole(std::io::Cursor::new(bytes)), len)
            }
        };
        let codec = match extension {
            "wav" => Codec::Wav,
            "flac" => Codec::Flac,
            "mp3" => Codec::Mp3,
            "ogg" => Codec::Vorbis,
            other => Codec::Other(other.to_uppercase()),
        };
        Ok(Audio {
            reader,
            len,
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
    fn the_widevine_key_is_read_from_its_data_uri() {
        let playlist = "#EXTM3U\n#EXT-X-KEY:METHOD=SAMPLE-AES,KEYFORMAT=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\",KEYFORMATVERSIONS=\"1\",URI=\"data:text/plain;base64,cHNzaA==\"\n#EXT-X-KEY:METHOD=SAMPLE-AES-CTR,KEYFORMAT=\"com.microsoft.playready\",URI=\"data:text/plain;charset=UTF-16;base64,AAAA\"\n";
        assert_eq!(widevine_pssh(playlist).as_deref(), Some(&b"pssh"[..]));
        assert_eq!(widevine_pssh("#EXTM3U\n"), None);
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
