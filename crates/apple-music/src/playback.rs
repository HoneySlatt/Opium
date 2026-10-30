//! Full-track playback: `webPlayback` → media playlist → Widevine licence → CENC.
//!
//! Endpoints and request shapes follow Kopuz (<https://github.com/Kopuz-org/kopuz>,
//! revision a58c385, `daemon/server/src/applemusic/stream.rs`), EUPL-1.2.
//!
//! Apple's web player only licenses the `28:ctrp256` flavour to Widevine: AAC-LC,
//! 256 kbps, 44.1 kHz. That is the best quality this path can reach.
//!
//! Spatial Audio comes from a song's enhanced HLS instead: Apple's binaural
//! rendering of its Dolby Atmos mix, for headphones, whose key Widevine is
//! licensed too (the ALAC's it isn't).

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
/// The enhanced HLS variant of Spatial Audio: the Dolby Atmos mix rendered
/// binaurally, AAC-LC 256 kbps at 48 kHz.
const BINAURAL: &str = "audio-stereo-256-binaural";
/// The Widevine system id, as HLS names its key format.
const WIDEVINE: &str = "urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed";
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

/// A sample's `(clear, encrypted)` byte runs, as the CDM takes them.
fn subsamples(sample: &cenc::EncryptedSample) -> Vec<(u32, u32)> {
    sample
        .subs
        .iter()
        .map(|&(c, e)| (u32::from(c), e))
        .collect()
}

/// The fragmented MP4 `resp` brings, readable from its first fragment on
/// while the rest downloads, `decrypt` working through its samples, each
/// `frame` long.
async fn as_it_downloads(
    mut resp: reqwest::Response,
    frame: std::time::Duration,
    decrypt: cenc::Decrypt,
) -> Result<AudioStream> {
    let len = resp
        .content_length()
        .ok_or_else(|| playback_error("asset download: no length"))?;
    let (stream, feed) = AudioStream::downloading(len, frame, decrypt);
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
    Ok(stream)
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
    /// Its keys, in the order its sample entries use them: one for the
    /// legacy flavours, two for the enhanced ones (the first segment's,
    /// then the song's).
    keys: Vec<Key>,
}

/// A key as the licence request names it, and its id.
#[derive(Debug, Clone, PartialEq)]
struct Key {
    /// The KEY URI: `<prefix>,<base64 key id>` for the legacy flavours, a
    /// `data:` URI holding the `pssh` for the enhanced ones.
    uri: String,
    id: Vec<u8>,
}

/// `None` when `body` isn't an encrypted media playlist.
fn read_media_playlist(playlist_url: &str, body: &[u8]) -> Option<EncryptedFile> {
    let playlist = m3u8_rs::parse_media_playlist_res(body).ok()?;
    let first = playlist.segments.first()?;
    let uri = first.key.as_ref()?.uri.as_deref()?;
    let id = STANDARD.decode(uri.split_once(',')?.1).ok()?;
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
        keys: vec![Key {
            uri: uri.to_owned(),
            id,
        }],
    })
}

/// An enhanced HLS media playlist's file and Widevine keys, in the order
/// they come.
fn read_enhanced_playlist(playlist_url: &str, body: &str) -> Option<EncryptedFile> {
    let mut keys: Vec<Key> = Vec::new();
    let mut map = None;
    for line in body.lines() {
        if line.starts_with("#EXT-X-KEY")
            && crate::radio::attribute(line, "KEYFORMAT") == Some(WIDEVINE)
            && let Some(uri) = crate::radio::attribute(line, "URI")
            && let Some(id) = crate::radio::pssh_key_id(uri)
            && !keys.iter().any(|k| k.uri == uri)
        {
            keys.push(Key {
                uri: uri.to_owned(),
                id: id.to_vec(),
            });
        } else if line.starts_with("#EXT-X-MAP") && map.is_none() {
            map = crate::radio::attribute(line, "URI");
        }
    }
    (!keys.is_empty()).then_some(EncryptedFile {
        url: crate::radio::resolve(playlist_url, map?),
        keys,
    })
}

/// The `pssh` box a key's licence is asked with: the enhanced keys' own,
/// else one made for the key id.
fn key_pssh(key: &Key) -> Vec<u8> {
    key.uri
        .rsplit_once("base64,")
        .filter(|_| key.uri.starts_with("data:text/plain"))
        .and_then(|(_, pssh)| STANDARD.decode(pssh).ok())
        .unwrap_or_else(|| widevine::build_pssh(&key.id))
}

/// The variant `group` of an enhanced HLS master playlist: its address.
fn variant(master_url: &str, master: &str, group: &str) -> Option<String> {
    let mut lines = master.lines();
    while let Some(line) = lines.next() {
        if line.starts_with("#EXT-X-STREAM-INF")
            && crate::radio::attribute(line, "AUDIO") == Some(group)
        {
            let uri = lines.find(|l| !l.starts_with('#') && !l.trim().is_empty())?;
            return Some(crate::radio::resolve(master_url, uri.trim()));
        }
    }
    None
}

/// An enhanced media playlist as ExoPlayer plays it: its Widevine keys
/// only (Apple's also name FairPlay's and PlayReady's), its init segment
/// `init.mp4` beside it (see [`Protected::init`]), every other address
/// absolute.
fn enhanced_playlist(body: &str, playlist_url: &str) -> String {
    body.lines()
        .filter(|line| {
            !line.starts_with("#EXT-X-KEY")
                || crate::radio::attribute(line, "KEYFORMAT") == Some(WIDEVINE)
        })
        .map(|line| {
            if line.starts_with("#EXT-X-MAP") {
                "#EXT-X-MAP:URI=\"init.mp4\"".to_owned()
            } else if !line.starts_with('#') && !line.trim().is_empty() {
                crate::radio::resolve(playlist_url, line.trim())
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Where a media playlist's init segment is in its file: `(offset, length)`.
fn map_range(body: &str) -> Option<(u64, u64)> {
    let line = body.lines().find(|l| l.starts_with("#EXT-X-MAP"))?;
    let (length, offset) = crate::radio::attribute(line, "BYTERANGE")?.split_once('@')?;
    Some((offset.parse().ok()?, length.parse().ok()?))
}

/// Write `ids` into the zero key ids of `init`'s `tenc` boxes, in their
/// order: the sample entries', one key each.
fn name_keys(init: &mut [u8], ids: &[&[u8]]) {
    let mut from = 0;
    let mut entry = 0;
    while let Some(at) = init
        .get(from..)
        .and_then(|rest| rest.windows(4).position(|w| w == b"tenc"))
    {
        let at = from + at;
        // The key id sits 8 bytes into the box's body, after its type.
        if let (Some(slot), Some(id)) = (init.get_mut(at + 12..at + 28), ids.get(entry))
            && slot.iter().all(|&b| b == 0)
            && id.len() == 16
        {
            slot.copy_from_slice(id);
        }
        entry += 1;
        from = at + 4;
    }
}

/// The sample rate an AAC `AudioSpecificConfig` says.
fn aac_rate(asc: &[u8]) -> Option<u32> {
    const RATES: [u32; 13] = [
        96_000, 88_200, 64_000, 48_000, 44_100, 32_000, 24_000, 22_050, 16_000, 12_000, 11_025,
        8_000, 7_350,
    ];
    let index = (u16::from_be_bytes([*asc.first()?, *asc.get(1)?]) >> 7) & 0x0f;
    RATES.get(usize::from(index)).copied()
}

/// A catalog track's encrypted file, and what its licence is asked with:
/// for a player that decrypts by itself (Android's MediaDrm), which sends
/// its own challenge through [`AppleMusic::license`].
#[derive(Debug, Clone)]
pub struct Protected {
    /// The whole fragmented MP4.
    pub url: String,
    /// The key the song starts with.
    pub key_id: Vec<u8>,
    /// A Widevine `pssh` box for that key, which Apple's file lacks.
    pub pssh: Vec<u8>,
    /// Spatial Audio: the binaural rendering of the song's Dolby Atmos mix.
    pub spatial: bool,
    /// Spatial Audio's init segment, which the playlist names `init.mp4`:
    /// Apple's leaves its key ids zero (the playlist naming the keys), and
    /// players decrypt with the init segment's, so they are written in.
    pub init: Option<Vec<u8>>,
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
    /// upload, which has no licence to ask for. With `spatial`, a song with
    /// a Dolby Atmos mix comes as its binaural rendering.
    pub async fn protected(&self, track: &ItemId, spatial: bool) -> Result<Option<Protected>> {
        let adam_id = track.id.as_str();
        // The licence comes next, from another host: its connection opens
        // meanwhile.
        let http = self.http.clone();
        tokio::spawn(async move {
            let _ = http.head(LICENSE_HOST).send().await;
        });
        let (song, binaural) = self.song_and_binaural(adam_id, spatial).await?;
        // A library song Apple matched to the catalog is licensed by the
        // catalog's id, said here, though asked for as the library's.
        let licensed_as = song_id(&song).unwrap_or_else(|| adam_id.to_owned());
        if let Some((url, body)) = binaural
            && let Some(file) = read_enhanced_playlist(&url, &body)
            && let Some(range) = map_range(&body)
            && let Ok(mut init) = self.range(&file.url, range).await
        {
            let ids: Vec<&[u8]> = file.keys.iter().map(|k| k.id.as_slice()).collect();
            name_keys(&mut init, &ids);
            let first = &file.keys[0];
            return Ok(Some(Protected {
                url: file.url.clone(),
                key_id: first.id.clone(),
                pssh: key_pssh(first),
                spatial: true,
                init: Some(init),
                playlist: enhanced_playlist(&body, &url),
                kbps: 256,
                licensed_as,
                is_library: false,
                file,
            }));
        }
        let Asset::Catalog { url, kbps } = select_asset(&song)? else {
            return Ok(None);
        };
        let playlist = self.download(&url).await?;
        let file = read_media_playlist(&url, &playlist)
            .ok_or_else(|| playback_error("catalog asset without a Widevine key"))?;
        let key_id = file.keys[0].id.clone();
        let pssh = widevine::build_pssh(&key_id);
        Ok(Some(Protected {
            url: file.url.clone(),
            playlist: widevine_playlist(&String::from_utf8_lossy(&playlist), &url, &pssh),
            pssh,
            key_id,
            spatial: false,
            init: None,
            kbps,
            licensed_as,
            is_library: is_library_id(adam_id),
            file,
        }))
    }

    /// The web player's entry for `adam_id`, and with `spatial` the
    /// binaural playlist of its Dolby Atmos mix when it has one, looked for
    /// side by side when the catalog id is known already. Spatial Audio
    /// that can't be found plays in stereo.
    async fn song_and_binaural(
        &self,
        adam_id: &str,
        spatial: bool,
    ) -> Result<(serde_json::Value, Option<(String, String)>)> {
        if !spatial {
            return Ok((self.web_playback(adam_id).await?, None));
        }
        if !is_library_id(adam_id) {
            let (song, binaural) = tokio::join!(self.web_playback(adam_id), self.binaural(adam_id));
            return Ok((song?, binaural.ok().flatten()));
        }
        let song = self.web_playback(adam_id).await?;
        let binaural = match song_id(&song) {
            Some(catalog) => self.binaural(&catalog).await.ok().flatten(),
            None => None,
        };
        Ok((song, binaural))
    }

    /// The binaural media playlist of the catalog song `id`'s Dolby Atmos
    /// mix, its address and text: `None` for a song without one.
    async fn binaural(&self, id: &str) -> Result<Option<(String, String)>> {
        let path = format!(
            "/v1/catalog/{}/songs/{id}?extend=extendedAssetUrls",
            self.storefront()
        );
        let Some(found) = self.fetch::<serde_json::Value>(&path).await? else {
            return Ok(None);
        };
        let attributes = &found["data"][0]["attributes"];
        let atmos = attributes["audioTraits"]
            .as_array()
            .is_some_and(|traits| traits.iter().any(|t| t == "atmos"));
        let Some(master_url) = attributes["extendedAssetUrls"]["enhancedHls"]
            .as_str()
            .filter(|_| atmos)
        else {
            return Ok(None);
        };
        let master = String::from_utf8_lossy(&self.download(master_url).await?).into_owned();
        let Some(url) = variant(master_url, &master, BINAURAL) else {
            return Ok(None);
        };
        let body = String::from_utf8_lossy(&self.download(&url).await?).into_owned();
        Ok(Some((url, body)))
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

    /// The licence for `protected`, in answer to a CDM's `challenge`: for
    /// the key the challenge names, which it carries as it is (Spatial
    /// Audio's file has two).
    pub async fn license(&self, protected: &Protected, challenge: &[u8]) -> Result<Vec<u8>> {
        let keys = &protected.file.keys;
        let key = keys
            .iter()
            .find(|k| challenge.windows(k.id.len()).any(|w| w == k.id))
            .unwrap_or(&keys[0]);
        self.request_license(
            challenge,
            &protected.licensed_as,
            protected.is_library,
            &key.uri,
        )
        .await
    }

    /// Download `track` and license it. The result reads as a plain MP4 any AAC
    /// decoder accepts, decrypting in the background as it goes; with the
    /// audio's bit rate, about 256 kbps but for a few songs only kept in a
    /// library, and whether it is Spatial Audio. With `spatial`, a song with
    /// a Dolby Atmos mix comes as its binaural rendering, and in stereo
    /// should that fail.
    pub async fn fetch_track(
        &self,
        cdm: &Cdm,
        track: &ItemId,
        spatial: bool,
    ) -> Result<(AudioStream, Option<u32>, bool)> {
        let adam_id = track.id.as_str();
        let (song, binaural) = self.song_and_binaural(adam_id, spatial).await?;
        // A library song Apple matched to the catalog is licensed by the
        // catalog's id, said here, though asked for as the library's.
        let licensed_as = song_id(&song).unwrap_or_else(|| adam_id.to_owned());
        if let Some((url, body)) = binaural
            && let Some(file) = read_enhanced_playlist(&url, &body)
            && let Ok(stream) = self.fetch_enhanced(cdm, &licensed_as, &file).await
        {
            return Ok((stream, None, true));
        }
        let asset = select_asset(&song)?;
        let (Asset::Catalog { url, .. } | Asset::Upload(url)) = &asset;

        let fetched = self.download(url).await?;

        let Some(file) = read_media_playlist(url, &fetched) else {
            return match asset {
                // An upload that isn't a playlist is the user's file itself.
                Asset::Upload(_) => Ok((AudioStream::plain(fetched), None, false)),
                Asset::Catalog { .. } => {
                    Err(playback_error("catalog asset without a Widevine key"))
                }
            };
        };

        let key = file.keys[0].clone();
        let (session, challenge) = cdm
            .start_session(&widevine::build_pssh(&key.id))
            .map_err(|e| playback_error(e.to_string()))?;
        let (resp, ()) = tokio::try_join!(
            self.start_download(&file.url),
            self.load_license(
                &session,
                &challenge,
                &licensed_as,
                is_library_id(adam_id),
                &key.uri,
            ),
        )?;
        let decrypt = Box::new(move |ciphertext: &[u8], sample: &cenc::EncryptedSample| {
            session
                .decrypt(ciphertext, &key.id, &sample.iv, &subsamples(sample))
                .map_err(|e| e.to_string())
        });
        let stream = as_it_downloads(resp, AAC_FRAME, decrypt).await?;
        let kbps = stream.with_samples(audio_kbps);
        Ok((stream, kbps, false))
    }

    /// An enhanced file (Spatial Audio's), licensed and downloading: its
    /// sample entries each name their key and constant IV, which its start
    /// says, and its frames are 48 kHz ones.
    async fn fetch_enhanced(
        &self,
        cdm: &Cdm,
        adam_id: &str,
        file: &EncryptedFile,
    ) -> Result<AudioStream> {
        let license = |key: &Key| {
            let key = key.clone();
            async move {
                let (session, challenge) = cdm
                    .start_session(&key_pssh(&key))
                    .map_err(|e| playback_error(e.to_string()))?;
                self.load_license(&session, &challenge, adam_id, false, &key.uri)
                    .await?;
                Ok::<_, Error>((session, key.id))
            }
        };
        // The movie box is in the first few kilobytes.
        let head = self.range(&file.url, (0, 16 * 1024));
        let (keys, head, resp) = tokio::try_join!(
            futures_util::future::try_join_all(file.keys.iter().map(license)),
            head,
            self.start_download(&file.url),
        )?;
        let tencs = cenc::tencs(&head);
        if tencs.is_empty() {
            return Err(playback_error("enhanced file without its keys' IVs"));
        }
        let rate = cenc::audio_specific_config(&head)
            .and_then(|asc| aac_rate(&asc))
            .unwrap_or(48_000);
        let frame = std::time::Duration::from_nanos(
            u64::from(AAC_FRAMES) * 1_000_000_000 / u64::from(rate),
        );
        let decrypt = Box::new(move |ciphertext: &[u8], sample: &cenc::EncryptedSample| {
            let entry = sample.description as usize;
            let tenc = &tencs[entry.min(tencs.len() - 1)];
            let iv: &[u8] = match &tenc.constant_iv {
                Some(iv) if sample.iv == [0; 16] => iv,
                _ => &sample.iv,
            };
            // Each sample entry's key, in the playlist's order; another of
            // them should that not be it.
            let order = keys
                .iter()
                .skip(entry)
                .chain(keys.iter().take(entry.min(keys.len())));
            let mut last = String::from("no key");
            for (session, key_id) in order {
                match session.decrypt_cbcs(
                    ciphertext,
                    key_id,
                    iv,
                    tenc.pattern,
                    &subsamples(sample),
                ) {
                    Ok(clear) => return Ok(clear),
                    Err(e) => last = e.to_string(),
                }
            }
            Err(last)
        });
        as_it_downloads(resp, frame, decrypt).await
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
        key_uri: &str,
    ) -> Result<()> {
        let license = self
            .request_license(challenge, adam_id, is_library, key_uri)
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
        key_uri: &str,
    ) -> Result<Vec<u8>> {
        let envelope = serde_json::json!({
            "challenge": STANDARD.encode(challenge),
            "key-system": "com.widevine.alpha",
            "uri": key_uri,
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

    /// `(offset, length)` of the file at `url`.
    async fn range(&self, url: &str, (offset, length): (u64, u64)) -> Result<Vec<u8>> {
        let failed =
            |e: reqwest::Error| Error::Network(format!("asset download: {}", e.without_url()));
        let resp = self
            .asset_request(url)
            .header("Range", format!("bytes={offset}-{}", offset + length - 1))
            .send()
            .await
            .map_err(failed)?;
        resp.bytes().await.map(|b| b.to_vec()).map_err(failed)
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
            description: 0,
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
        assert_eq!(file.keys.len(), 1);
        assert_eq!(file.keys[0].uri, "data:;base64,AAAAAAAAAAAAAAAAAAAAAA==");
        assert_eq!(file.keys[0].id, [0; 16]);
    }

    /// Spatial Audio's playlist, as Apple writes it (its PlayReady lines cut
    /// short): the first segment under a key of its own.
    const ENHANCED: &str = "#EXTM3U\n\
        #EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"skd://itunes.apple.com/P000000000/s1/e1\",KEYFORMAT=\"com.apple.streamingkeydelivery\",KEYFORMATVERSIONS=\"1\"\n\
        #EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"data:text/plain;charset=UTF-16;base64,vgEAAA==\",KEYFORMAT=\"com.microsoft.playready\",KEYFORMATVERSIONS=\"1\"\n\
        #EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"data:text/plain;base64,AAAAOHBzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAABgSEAAAAAAAAAAAczEvZTEgICBI88aJmwY=\",KEYFORMAT=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\",KEYFORMATVERSIONS=\"1\"\n\
        #EXT-X-MAP:URI=\"P1_bm_m.mp4\",BYTERANGE=\"1124@0\"\n\
        #EXTINF:14.99733,\n\
        #EXT-X-BYTERANGE:475343@1124\n\
        P1_bm_m.mp4\n\
        #EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"skd://itunes.apple.com/p1/c24\",KEYFORMAT=\"com.apple.streamingkeydelivery\",KEYFORMATVERSIONS=\"1\"\n\
        #EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"data:text/plain;base64,AAAAOHBzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAABgSEAAAAABK1kd0YzI0ICAgICBI88aJmwY=\",KEYFORMAT=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\",KEYFORMATVERSIONS=\"1\"\n\
        #EXTINF:14.99733,\n\
        #EXT-X-BYTERANGE:481472@476467\n\
        P1_bm_m.mp4\n\
        #EXT-X-ENDLIST\n";

    #[test]
    fn an_enhanced_playlist_names_its_widevine_keys_in_order() {
        let file =
            read_enhanced_playlist("https://aod.itunes.apple.com/x/P1_bm.m3u8", ENHANCED).unwrap();
        assert_eq!(file.url, "https://aod.itunes.apple.com/x/P1_bm_m.mp4");
        assert_eq!(file.keys.len(), 2);
        assert_eq!(&file.keys[0].id[8..13], b"s1/e1");
        assert!(file.keys[1].uri.ends_with("YzI0ICAgICBI88aJmwY="));
        assert_eq!(key_pssh(&file.keys[1])[4..8], *b"pssh");
    }

    #[test]
    fn exoplayer_gets_the_widevine_keys_only_and_whole_addresses() {
        let out = enhanced_playlist(ENHANCED, "https://aod.itunes.apple.com/x/P1_bm.m3u8");
        let keys: Vec<&str> = out
            .lines()
            .filter(|l| l.starts_with("#EXT-X-KEY"))
            .collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.iter().all(|k| k.contains("edef8ba9")));
        assert!(out.contains("#EXT-X-MAP:URI=\"init.mp4\"\n"));
        assert_eq!(map_range(ENHANCED), Some((0, 1124)));
        assert!(
            out.lines()
                .any(|l| l == "https://aod.itunes.apple.com/x/P1_bm_m.mp4")
        );
    }

    #[test]
    fn the_binaural_variant_is_found_in_the_master() {
        let master = "#EXTM3U\n\
            #EXT-X-STREAM-INF:BANDWIDTH=264242,CODECS=\"mp4a.40.2\",AUDIO=\"audio-stereo-256-binaural\"\n\
            P1_bm.m3u8\n\
            #EXT-X-STREAM-INF:BANDWIDTH=262619,CODECS=\"mp4a.40.2\",AUDIO=\"audio-stereo-256\"\n\
            P1_m.m3u8\n";
        let base = "https://aod.itunes.apple.com/x/P1.m3u8?a=1";
        assert_eq!(
            variant(base, master, BINAURAL).as_deref(),
            Some("https://aod.itunes.apple.com/x/P1_bm.m3u8")
        );
        assert_eq!(
            variant(base, master, "audio-stereo-256").as_deref(),
            Some("https://aod.itunes.apple.com/x/P1_m.m3u8")
        );
    }

    #[test]
    fn each_sample_entry_gets_its_key_id() {
        let tenc = |kid: u8| {
            let mut body = vec![1, 0, 0, 0, 0, 0, 1, 0];
            body.extend_from_slice(&[kid; 16]);
            let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
            out.extend_from_slice(b"tenc");
            out.extend_from_slice(&body);
            out
        };
        let mut init = [tenc(0), tenc(0), tenc(7)].concat();
        name_keys(&mut init, &[&[1; 16], &[2; 16], &[3; 16]]);
        let ids: Vec<u8> = cenc::tencs(&init).iter().map(|t| t.key_id[0]).collect();
        // A key id already there stays.
        assert_eq!(ids, [1, 2, 7]);
    }

    #[test]
    fn the_sample_rate_is_read_from_the_audio_config() {
        // AAC-LC: 48 kHz, then 44.1 kHz.
        assert_eq!(aac_rate(&[0x11, 0x90]), Some(48_000));
        assert_eq!(aac_rate(&[0x12, 0x10]), Some(44_100));
    }

    #[test]
    fn a_plain_file_is_not_a_playlist() {
        assert_eq!(
            read_media_playlist("https://x/a.m4a", b"\0\0\0\x20ftypM4A "),
            None
        );
    }
}
