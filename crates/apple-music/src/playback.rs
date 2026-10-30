//! Full-track playback: `webPlayback` → media playlist → Widevine licence → CENC.
//!
//! Endpoints and request shapes follow Kopuz (<https://github.com/Kopuz-org/kopuz>,
//! revision a58c385, `daemon/server/src/applemusic/stream.rs`), EUPL-1.2.
//!
//! Apple's web player only licenses the `28:ctrp256` flavour to Widevine: AAC-LC,
//! 256 kbps, 44.1 kHz. That is the best quality this path can reach.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use music_core::model::ItemId;
use music_core::{Error, Result};
use widevine::{Cdm, Session};

use crate::AppleMusic;
use cenc::AudioStream;

const WEB_PLAYBACK: &str = "https://play.music.apple.com/WebObjects/MZPlay.woa/wa/webPlayback";
/// The web player's host, to connect to ahead of a song.
const WEB_PLAYBACK_HOST: &str = "https://play.music.apple.com/";
/// Where the songs' playlists and files are.
pub const ASSET_HOST: &str = "https://aod-ssl.itunes.apple.com/";
/// The licence server's host, to connect to ahead of the licence.
const LICENSE_HOST: &str = "https://play.itunes.apple.com/";
const LICENSE: &str =
    "https://play.itunes.apple.com/WebObjects/MZPlay.woa/wa/acquireWebPlaybackLicense";
/// The catalog flavours Widevine can open are `ctrp` ones (the `cbcp` use
/// FairPlay): `28:ctrp256` as a rule, `32:ctrp64` for some songs only kept
/// in a user's library.
const CTR: &str = ":ctrp";
/// The frames Apple's AAC encoder puts before the music, which its files
/// don't say.
pub const PRIMING: u32 = 2112;
/// An AAC-LC frame's length, in samples per channel.
const AAC_FRAMES: u32 = 1024;
/// Each sample of that flavour is one AAC-LC frame: 1024 frames at 44.1 kHz.
const AAC_FRAME: std::time::Duration =
    std::time::Duration::from_nanos(1024 * 1_000_000_000 / 44_100);

fn playback_error(msg: impl Into<String>) -> Error {
    Error::Playback(msg.into())
}

/// Library ids (`i.`/`l.`/`a.`/`p.` and an alphanumeric tail) as opposed to a
/// numeric catalog id. Only the user's uploads still carry one by playback time.
fn is_library_id(id: &str) -> bool {
    let mut chars = id.chars();
    if !matches!(chars.next(), Some('a' | 'i' | 'l' | 'p')) || chars.next() != Some('.') {
        return false;
    }
    let tail = &id[2..];
    !tail.is_empty() && tail.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// The catalog song a `songList` entry plays, when it names one: a user's
/// own upload says -1.
fn song_id(song: &serde_json::Value) -> Option<String> {
    match &song["songId"] {
        serde_json::Value::String(id) if !id.is_empty() && id != "-1" => Some(id.clone()),
        serde_json::Value::Number(n) if n.as_i64().is_some_and(|n| n > 0) => Some(n.to_string()),
        _ => None,
    }
}

fn web_playback_body(id: &str) -> serde_json::Value {
    if is_library_id(id) {
        serde_json::json!({ "universalLibraryId": id })
    } else {
        serde_json::json!({ "salableAdamId": id })
    }
}

/// The asset to play out of a `songList` entry.
#[derive(Debug, PartialEq)]
enum Asset {
    /// A catalog encode: a media playlist whose KEY carries the key id, and
    /// its bit rate.
    Catalog { url: String, kbps: u32 },
    /// The user's upload: a single flavourless asset, possibly not encrypted.
    Upload(String),
}

fn select_asset(song: &serde_json::Value) -> Result<Asset> {
    let assets = song["assets"]
        .as_array()
        .ok_or_else(|| playback_error("webPlayback returned no assets"))?;

    if let [only] = assets.as_slice()
        && only["flavor"].as_str().is_none()
        && let Some(url) = only["URL"].as_str().filter(|u| !u.is_empty())
    {
        return Ok(Asset::Upload(url.to_owned()));
    }

    // The best Widevine flavour there is.
    assets
        .iter()
        .filter_map(|a| {
            let kbps = a["flavor"].as_str()?.split_once(CTR)?.1.parse().ok()?;
            let url = a["URL"].as_str().filter(|u| !u.is_empty())?;
            Some(Asset::Catalog {
                url: url.to_owned(),
                kbps,
            })
        })
        .max_by_key(|a| match a {
            Asset::Catalog { kbps, .. } => *kbps,
            Asset::Upload(_) => 0,
        })
        .ok_or_else(|| playback_error("no Widevine asset (only FairPlay's)"))
}

/// The audio's bit rate, from its samples: each one AAC frame.
fn audio_kbps(samples: &[cenc::EncryptedSample]) -> Option<u32> {
    let seconds = samples.len() as f64 * AAC_FRAME.as_secs_f64();
    let bytes: usize = samples.iter().map(|s| s.len).sum();
    (seconds > 0.0).then(|| (bytes as f64 * 8.0 / seconds / 1000.0).round() as u32)
}

/// An encrypted track as its media playlist describes it.
#[derive(Debug, Clone, PartialEq)]
struct EncryptedFile {
    /// The whole fMP4: Apple maps every segment onto one file.
    url: String,
    /// The KEY URI is `<prefix>,<base64 key id>`; the licence request wants both.
    uri_prefix: String,
    key_id_base64: String,
}

/// `None` when `body` isn't an encrypted media playlist.
fn read_media_playlist(playlist_url: &str, body: &[u8]) -> Option<EncryptedFile> {
    let playlist = m3u8_rs::parse_media_playlist_res(body).ok()?;
    let first = playlist.segments.first()?;
    let (uri_prefix, key_id_base64) = first.key.as_ref()?.uri.as_deref()?.split_once(',')?;
    let map = first.map.as_ref()?.uri.as_str();
    let url = if map.starts_with("http") {
        map.to_owned()
    } else {
        let base = playlist_url
            .rsplit_once('/')
            .map_or(playlist_url, |(base, _)| base);
        format!("{base}/{map}")
    };
    Some(EncryptedFile {
        url,
        uri_prefix: uri_prefix.to_owned(),
        key_id_base64: key_id_base64.to_owned(),
    })
}

/// A catalog track's encrypted file, and what its licence is asked with:
/// for a player that decrypts by itself (Android's MediaDrm), which sends
/// its own challenge through [`AppleMusic::license`].
#[derive(Debug, Clone)]
pub struct Protected {
    /// The whole fragmented MP4.
    pub url: String,
    pub key_id: Vec<u8>,
    /// A Widevine `pssh` box for the key, which Apple's file lacks.
    pub pssh: Vec<u8>,
    /// The track's media playlist as an HLS player reads Widevine CENC:
    /// its key named the standard way, its addresses absolute.
    pub playlist: String,
    /// The encode's bit rate, as its flavour names it.
    pub kbps: u32,
    licensed_as: String,
    is_library: bool,
    file: EncryptedFile,
}

/// Apple's media playlist as players read Widevine CENC: its key (method
/// `ISO-23001-7`, a bare key id) as `SAMPLE-AES-CTR` with the `pssh`, and
/// every address made absolute from `playlist_url`.
fn widevine_playlist(body: &str, playlist_url: &str, pssh: &[u8]) -> String {
    let base = playlist_url
        .rsplit_once('/')
        .map_or(playlist_url, |(base, _)| base);
    let absolute = |uri: &str| {
        if uri.starts_with("http") {
            uri.to_owned()
        } else {
            format!("{base}/{uri}")
        }
    };
    body.lines()
        .map(|line| {
            if line.starts_with("#EXT-X-KEY") {
                format!(
                    "#EXT-X-KEY:METHOD=SAMPLE-AES-CTR,URI=\"data:text/plain;base64,{}\",\
                     KEYFORMAT=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\",KEYFORMATVERSIONS=\"1\"",
                    STANDARD.encode(pssh)
                )
            } else if let Some(rest) = line.strip_prefix("#EXT-X-MAP:URI=\"")
                && let Some((uri, after)) = rest.split_once('"')
            {
                format!("#EXT-X-MAP:URI=\"{}\"{after}", absolute(uri))
            } else if !line.starts_with('#') && !line.trim().is_empty() {
                absolute(line.trim())
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl AppleMusic {
    /// Where `track`'s encrypted file is, and its key: `None` for a user's
    /// upload, which has no licence to ask for.
    pub async fn protected(&self, track: &ItemId) -> Result<Option<Protected>> {
        let adam_id = track.id.as_str();
        // The licence comes next, from another host: its connection opens
        // meanwhile.
        let http = self.http.clone();
        tokio::spawn(async move {
            let _ = http.head(LICENSE_HOST).send().await;
        });
        let song = self.web_playback(adam_id).await?;
        // A library song Apple matched to the catalog is licensed by the
        // catalog's id, said here, though asked for as the library's.
        let licensed_as = song_id(&song).unwrap_or_else(|| adam_id.to_owned());
        let Asset::Catalog { url, kbps } = select_asset(&song)? else {
            return Ok(None);
        };
        let playlist = self.download(&url).await?;
        let file = read_media_playlist(&url, &playlist)
            .ok_or_else(|| playback_error("catalog asset without a Widevine key"))?;
        let key_id = STANDARD
            .decode(&file.key_id_base64)
            .map_err(|e| playback_error(format!("key id: {e}")))?;
        let pssh = widevine::build_pssh(&key_id);
        Ok(Some(Protected {
            url: file.url.clone(),
            playlist: widevine_playlist(&String::from_utf8_lossy(&playlist), &url, &pssh),
            pssh,
            key_id,
            kbps,
            licensed_as,
            is_library: is_library_id(adam_id),
            file,
        }))
    }

    /// The frames after the music in `protected`'s last AAC frame, to leave
    /// out with the [`PRIMING`] before it, so that songs follow without a
    /// gap: its last fragment's last frame is that much shorter than 1024.
    /// Only the fragment's start is read.
    pub async fn padding(&self, protected: &Protected) -> u32 {
        self.padding_of(&protected.url, protected.playlist.as_bytes())
            .await
            .unwrap_or(0)
    }

    async fn padding_of(&self, url: &str, playlist: &[u8]) -> Option<u32> {
        let playlist = m3u8_rs::parse_media_playlist_res(playlist).ok()?;
        let mut end = 0u64;
        let mut last = None;
        for segment in &playlist.segments {
            let range = segment.byte_range.as_ref()?;
            let start = range.offset.unwrap_or(end);
            end = start + range.length;
            last = Some(start);
        }
        let from = last?;
        // Its `moof` is a few kilobytes at most.
        let to = (from + 16 * 1024).min(end) - 1;
        let resp = self
            .asset_request(url)
            .header("Range", format!("bytes={from}-{to}"))
            .send()
            .await
            .ok()?;
        let fragment = resp.bytes().await.ok()?;
        cenc::last_sample_duration(&fragment)
            .filter(|&d| d > 0 && d < AAC_FRAMES)
            .map(|d| AAC_FRAMES - d)
    }

    /// What `protected`'s encode really is, when its flavour is less than
    /// the usual 256 kbps (whose name says it right), as a flavour's name
    /// may say less. The whole song's average, without downloading it: its
    /// fragments' sizes, less what their boxes take per sample as the first
    /// fragment shows.
    pub async fn lesser_kbps(&self, protected: &Protected) -> Option<u32> {
        if protected.kbps >= 256 {
            return None;
        }
        let playlist = m3u8_rs::parse_media_playlist_res(protected.playlist.as_bytes()).ok()?;
        let first = playlist.segments.first()?.byte_range.as_ref()?;
        // The movie box, then the first fragment.
        let end = first.offset? + first.length;
        let resp = self
            .asset_request(&protected.url)
            .header("Range", format!("bytes=0-{}", end - 1))
            .send()
            .await
            .ok()?;
        let head = resp.bytes().await.ok()?;
        let samples = cenc::index_fmp4(&head).ok()?.samples;
        let audio: usize = samples.iter().map(|s| s.len).sum();
        let boxes = first.length.checked_sub(audio as u64)? as f64 / samples.len() as f64;
        let bytes: u64 = playlist
            .segments
            .iter()
            .filter_map(|s| s.byte_range.as_ref())
            .map(|r| r.length)
            .sum();
        let seconds: f64 = playlist
            .segments
            .iter()
            .map(|s| f64::from(s.duration))
            .sum();
        let frames = seconds / AAC_FRAME.as_secs_f64();
        let audio = bytes as f64 - boxes * frames;
        (seconds > 0.0 && audio > 0.0).then(|| (audio * 8.0 / seconds / 1000.0).round() as u32)
    }

    /// Open the connections a song's start goes through (the web player,
    /// the files, the licences), so that the first song starts as fast as
    /// the next ones.
    pub async fn warm(&self) {
        let _ = tokio::join!(
            self.http.head(WEB_PLAYBACK_HOST).send(),
            self.http.head(ASSET_HOST).send(),
            self.http.head(LICENSE_HOST).send(),
        );
    }

    /// The licence for `protected`, in answer to a CDM's `challenge`.
    pub async fn license(&self, protected: &Protected, challenge: &[u8]) -> Result<Vec<u8>> {
        self.request_license(
            challenge,
            &protected.licensed_as,
            protected.is_library,
            &protected.file,
        )
        .await
    }

    /// Download `track` and license it. The result reads as a plain MP4 any AAC
    /// decoder accepts, decrypting in the background as it goes; with the
    /// audio's bit rate, about 256 kbps but for a few songs only kept in a
    /// library.
    pub async fn fetch_track(
        &self,
        cdm: &Cdm,
        track: &ItemId,
    ) -> Result<(AudioStream, Option<u32>)> {
        let adam_id = track.id.as_str();
        let song = self.web_playback(adam_id).await?;
        // A library song Apple matched to the catalog is licensed by the
        // catalog's id, said here, though asked for as the library's.
        let licensed_as = song_id(&song).unwrap_or_else(|| adam_id.to_owned());
        let asset = select_asset(&song)?;
        let (Asset::Catalog { url, .. } | Asset::Upload(url)) = &asset;

        let fetched = self.download(url).await?;

        let Some(file) = read_media_playlist(url, &fetched) else {
            return match asset {
                // An upload that isn't a playlist is the user's file itself.
                Asset::Upload(_) => Ok((AudioStream::plain(fetched), None)),
                Asset::Catalog { .. } => {
                    Err(playback_error("catalog asset without a Widevine key"))
                }
            };
        };

        let key_id = STANDARD
            .decode(&file.key_id_base64)
            .map_err(|e| playback_error(format!("key id: {e}")))?;
        let (session, challenge) = cdm
            .start_session(&widevine::build_pssh(&key_id))
            .map_err(|e| playback_error(e.to_string()))?;
        let (mut resp, ()) = tokio::try_join!(
            self.start_download(&file.url),
            self.load_license(
                &session,
                &challenge,
                &licensed_as,
                is_library_id(adam_id),
                &file
            ),
        )?;
        let len = resp
            .content_length()
            .ok_or_else(|| playback_error("asset download: no length"))?;
        let decrypt = Box::new(move |ciphertext: &[u8], sample: &cenc::EncryptedSample| {
            let subsamples: Vec<(u32, u32)> = sample
                .subs
                .iter()
                .map(|&(c, e)| (u32::from(c), e))
                .collect();
            session
                .decrypt(ciphertext, &key_id, &sample.iv, &subsamples)
                .map_err(|e| e.to_string())
        });
        // The track plays as it downloads: from its first fragment, whose
        // encode says the bit rate (a flavour's name may say less).
        let (stream, feed) = AudioStream::downloading(len, AAC_FRAME, decrypt);
        let failed = |e: reqwest::Error| format!("asset download: {}", e.without_url());
        while feed.samples() == 0 {
            match resp.chunk().await {
                Ok(Some(bytes)) => {
                    feed.push(&bytes);
                }
                Ok(None) => {
                    feed.end(None);
                    return Err(playback_error("no encrypted samples found in the track"));
                }
                Err(e) => return Err(playback_error(failed(e))),
            }
        }
        let kbps = stream.with_samples(audio_kbps);
        tokio::spawn(async move {
            loop {
                match resp.chunk().await {
                    Ok(Some(bytes)) => {
                        // Nobody reads it any more.
                        if !feed.push(&bytes) {
                            return;
                        }
                    }
                    Ok(None) => return feed.end(None),
                    Err(e) => return feed.end(Some(failed(e))),
                }
            }
        });
        Ok((stream, kbps))
    }

    async fn web_playback(&self, adam_id: &str) -> Result<serde_json::Value> {
        let resp = self
            .send(
                self.http
                    .post(WEB_PLAYBACK)
                    .header("x-apple-music-user-token", &self.user_token)
                    .json(&web_playback_body(adam_id)),
                "webPlayback",
            )
            .await?;
        let json = json_body(resp, "webPlayback").await?;
        json["songList"]
            .as_array()
            .and_then(|list| list.first())
            .cloned()
            .ok_or_else(|| playback_error("webPlayback returned no song"))
    }

    /// Exchange the CDM's challenge for a licence and load it into the session.
    /// The content key never leaves the CDM.
    async fn load_license(
        &self,
        session: &Session,
        challenge: &[u8],
        adam_id: &str,
        is_library: bool,
        file: &EncryptedFile,
    ) -> Result<()> {
        let license = self
            .request_license(challenge, adam_id, is_library, file)
            .await?;
        session
            .update(&license)
            .map_err(|e| playback_error(e.to_string()))
    }

    /// The licence server's answer to a CDM's challenge.
    async fn request_license(
        &self,
        challenge: &[u8],
        adam_id: &str,
        is_library: bool,
        file: &EncryptedFile,
    ) -> Result<Vec<u8>> {
        let envelope = serde_json::json!({
            "challenge": STANDARD.encode(challenge),
            "key-system": "com.widevine.alpha",
            "uri": format!("{},{}", file.uri_prefix, file.key_id_base64),
            "adamId": adam_id,
            "isLibrary": is_library,
            "user-initiated": true,
        });
        let resp = self
            .send(
                self.http
                    .post(LICENSE)
                    .header("x-apple-music-user-token", &self.user_token)
                    .json(&envelope),
                "licence",
            )
            .await?;
        let json = json_body(resp, "licence").await?;
        if let Some(code) = json["errorCode"].as_i64().filter(|&c| c != 0) {
            return Err(playback_error(format!(
                "licence refused (errorCode {code})"
            )));
        }
        let license = json["license"]
            .as_str()
            .and_then(|l| STANDARD.decode(l).ok())
            .ok_or_else(|| playback_error("licence response without a licence"))?;
        Ok(license)
    }

    pub(crate) async fn download(&self, url: &str) -> Result<Vec<u8>> {
        self.start_download(url)
            .await?
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| Error::Network(format!("asset download: {}", e.without_url())))
    }

    /// A download under way, its headers in.
    async fn start_download(&self, url: &str) -> Result<reqwest::Response> {
        let resp = self
            .asset_request(url)
            .send()
            .await
            .map_err(|e| Error::Network(format!("asset download: {}", e.without_url())))?;
        if !resp.status().is_success() {
            return Err(playback_error(format!(
                "asset download: HTTP {}",
                resp.status()
            )));
        }
        Ok(resp)
    }

    fn asset_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut request = self.http.get(url);
        // The user's token goes to Apple only: a radio station's segments
        // may be a broadcaster's.
        let apple = reqwest::Url::parse(url).is_ok_and(|u| {
            u.scheme() == "https"
                && u.host_str()
                    .is_some_and(|h| h == "apple.com" || h.ends_with(".apple.com"))
        });
        if apple {
            request = request
                .header("x-apple-music-user-token", &self.user_token)
                .header("Cookie", format!("media-user-token={}", self.user_token));
        }
        request
    }
}

async fn json_body(resp: reqwest::Response, what: &str) -> Result<serde_json::Value> {
    match resp.status().as_u16() {
        401 | 403 => return Err(Error::NotSignedIn),
        _ if !resp.status().is_success() => {
            return Err(playback_error(format!("{what}: HTTP {}", resp.status())));
        }
        _ => {}
    }
    resp.json()
        .await
        .map_err(|e| Error::Protocol(format!("{what}: {e}")))
}

#[cfg(test)]
mod widevine_playlist_tests {
    use super::*;

    #[test]
    fn the_key_is_named_the_standard_way_and_addresses_are_absolute() {
        let body = "#EXTM3U\n#EXT-X-KEY:METHOD=ISO-23001-7,URI=\"data:;base64,AAAA\",KEYFORMAT=\"x\"\n\
                    #EXT-X-MAP:URI=\"t.mp4\",BYTERANGE=\"10@0\"\n#EXTINF:10.0,\n#EXT-X-BYTERANGE:5@10\nt.mp4\n";
        let out = widevine_playlist(body, "https://a.apple.com/x/p.m3u8", &[1, 2, 3]);
        let lines: Vec<&str> = out.lines().collect();
        assert!(
            lines[1].starts_with(
                "#EXT-X-KEY:METHOD=SAMPLE-AES-CTR,URI=\"data:text/plain;base64,AQID\""
            )
        );
        assert_eq!(
            lines[2],
            "#EXT-X-MAP:URI=\"https://a.apple.com/x/t.mp4\",BYTERANGE=\"10@0\""
        );
        assert_eq!(lines[5], "https://a.apple.com/x/t.mp4");
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_matched_library_song_names_its_catalog_song_and_an_upload_none() {
        assert_eq!(
            super::song_id(&serde_json::json!({ "songId": "1744445314" })).as_deref(),
            Some("1744445314")
        );
        assert_eq!(super::song_id(&serde_json::json!({ "songId": -1 })), None);
        assert_eq!(super::song_id(&serde_json::json!({})), None);
    }

    use super::*;

    #[test]
    fn library_ids_are_told_apart_from_catalog_ids() {
        assert!(is_library_id("i.ABC123"));
        assert!(!is_library_id("1463409605"));
        assert!(!is_library_id("i."));
        assert_eq!(
            web_playback_body("1463409605")["salableAdamId"],
            "1463409605"
        );
        assert_eq!(web_playback_body("i.ABC")["universalLibraryId"], "i.ABC");
    }

    #[test]
    fn a_catalog_song_plays_its_best_ctr_flavour() {
        let song = serde_json::json!({ "assets": [
            { "flavor": "34:cbcp64", "URL": "https://example.com/cbcp.m3u8" },
            { "flavor": "32:ctrp64", "URL": "https://example.com/ctr64.m3u8" },
            { "flavor": "28:ctrp256", "URL": "https://example.com/ctr.m3u8" },
        ]});
        assert_eq!(
            select_asset(&song).unwrap(),
            Asset::Catalog {
                url: "https://example.com/ctr.m3u8".into(),
                kbps: 256
            }
        );
        // A song only kept in a library may have no better than 64 kbps.
        let kept = serde_json::json!({ "assets": [
            { "flavor": "38:ibhp64", "URL": "https://example.com/he.m3u8" },
            { "flavor": "34:cbcp64", "URL": "https://example.com/cbcp.m3u8" },
            { "flavor": "32:ctrp64", "URL": "https://example.com/ctr64.m3u8" },
        ]});
        assert_eq!(
            select_asset(&kept).unwrap(),
            Asset::Catalog {
                url: "https://example.com/ctr64.m3u8".into(),
                kbps: 64
            }
        );
    }

    #[test]
    fn the_bit_rate_is_read_from_the_samples() {
        let sample = |len| cenc::EncryptedSample {
            fragment: 0,
            start: 0,
            len,
            iv: [0; 16],
            subs: Vec::new(),
        };
        // 744 bytes a frame of 1024 at 44.1 kHz: 256 kbps.
        let samples: Vec<_> = (0..430).map(|_| sample(744)).collect();
        assert_eq!(audio_kbps(&samples), Some(256));
        assert_eq!(audio_kbps(&[]), None);
    }

    #[test]
    fn a_lone_flavourless_asset_is_an_upload() {
        let song = serde_json::json!({ "assets": [{ "URL": "https://example.com/mine.m4a" }] });
        assert_eq!(
            select_asset(&song).unwrap(),
            Asset::Upload("https://example.com/mine.m4a".into())
        );
    }

    #[test]
    fn the_media_playlist_gives_the_file_and_the_key() {
        let body = b"#EXTM3U\n\
            #EXT-X-VERSION:7\n\
            #EXT-X-TARGETDURATION:10\n\
            #EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"data:;base64,AAAAAAAAAAAAAAAAAAAAAA==\",KEYFORMAT=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\",KEYFORMATVERSIONS=\"1\"\n\
            #EXT-X-MAP:URI=\"P1_track.mp4\",BYTERANGE=\"1000@0\"\n\
            #EXTINF:10.0,\n\
            #EXT-X-BYTERANGE:5000@1000\n\
            P1_track.mp4\n\
            #EXT-X-ENDLIST\n";
        let file = read_media_playlist("https://aod.itunes.apple.com/x/y/P1.m3u8", body).unwrap();
        assert_eq!(file.url, "https://aod.itunes.apple.com/x/y/P1_track.mp4");
        assert_eq!(file.uri_prefix, "data:;base64");
        assert_eq!(file.key_id_base64, "AAAAAAAAAAAAAAAAAAAAAA==");
    }

    #[test]
    fn a_plain_file_is_not_a_playlist() {
        assert_eq!(
            read_media_playlist("https://x/a.m4a", b"\0\0\0\x20ftypM4A "),
            None
        );
    }
}
