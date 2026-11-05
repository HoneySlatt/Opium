//! Full-track playback: `webPlayback` → media playlist → Widevine licence → CENC.
//!
//! Apple's web player only licenses the `28:ctrp256` flavour to Widevine: AAC-LC,
//! 256 kbps, 44.1 kHz. That is the best quality this path can reach.
//!
//! Spatial Audio comes from a song's enhanced HLS instead: Apple's binaural
//! rendering of its Dolby Atmos mix, for headphones, whose key Widevine is
//! licensed too (the ALAC's it isn't).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use opium_core::model::ItemId;
use opium_core::{Error, Result};
use widevine::{Cdm, Session};

use crate::AppleMusic;
use cenc::AudioStream;

/// The web player's playback service, connected to ahead of a song.
pub(crate) const PLAY: &str = "https://play.music.apple.com";
/// Its path that names a song's assets.
const WEB_PLAYBACK: &str = "/WebObjects/MZPlay.woa/wa/webPlayback";
/// Where the songs' playlists and files are.
pub const ASSET_HOST: &str = "https://aod-ssl.itunes.apple.com/";
/// The licence server, connected to ahead of the licence.
pub(crate) const LICENCE: &str = "https://play.itunes.apple.com";
/// Its path that answers a Widevine challenge.
const LICENSE: &str = "/WebObjects/MZPlay.woa/wa/acquireWebPlaybackLicense";
/// The key system the licence is asked for.
const KEY_SYSTEM: &str = "com.widevine.alpha";
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

/// The catalog song a `songList` entry plays, when it names one: a user's
/// own upload says -1.
fn song_id(song: &serde_json::Value) -> Option<String> {
    match &song["songId"] {
        serde_json::Value::String(id) if !id.is_empty() && id != "-1" => Some(id.clone()),
        serde_json::Value::Number(n) if n.as_i64().is_some_and(|n| n > 0) => Some(n.to_string()),
        _ => None,
    }
}

/// The two kinds of song ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdKind {
    /// A decimal number: a song of the catalog, which anyone can play.
    Catalog,
    /// An item of the user's own library: `i.` (a song), `l.` (an album), `p.` (a
    /// playlist) or `a.` (an artist), then one or more ASCII letters, digits or dashes.
    Library,
}

/// Which kind of id `id` is: `None` for neither.
fn id_kind(id: &str) -> Option<IdKind> {
    if !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) {
        return Some(IdKind::Catalog);
    }
    let (kind, name) = id.split_once('.')?;
    let library = matches!(kind, "i" | "l" | "p" | "a")
        && !name.is_empty()
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    library.then_some(IdKind::Library)
}

/// The body of the `webPlayback` request for the song `id`: `salableAdamId` for a catalog
/// id, `universalLibraryId` for a library one, the id as a string.
///
/// # Errors
///
/// A playback error naming `id` when it is neither kind: nothing is asked.
fn playback_request(id: &str) -> Result<serde_json::Value> {
    let key = match id_kind(id) {
        Some(IdKind::Catalog) => "salableAdamId",
        Some(IdKind::Library) => "universalLibraryId",
        None => return Err(playback_error(format!("not a song id: {id:?}"))),
    };
    Ok(serde_json::json!({ key: id }))
}

/// What a song plays, as its `webPlayback` entry says.
#[derive(Debug, PartialEq)]
enum Source {
    /// A user's own upload: the address of its file, or of an encrypted media playlist of it.
    Upload(String),
    /// A catalog song's encode: the address of its HLS media playlist, and its bit rate as its
    /// flavour names it.
    Encode { playlist: String, kbps: u32 },
}

/// What `entry`, a `songList` entry, plays: its one asset without a flavour when it is an
/// upload, else the asset Widevine can open (a `:ctrp` flavour) with the highest bit rate,
/// the first of them on a tie. An asset needs a non-empty `URL` to be either.
///
/// # Errors
///
/// A playback error when `entry` has no assets, or none Widevine can open (FairPlay's
/// `:cbcp` ones only, say).
fn choose_source(entry: &serde_json::Value) -> Result<Source> {
    let assets = entry["assets"]
        .as_array()
        .filter(|assets| !assets.is_empty())
        .ok_or_else(|| playback_error("webPlayback returned no assets"))?;
    if let [only] = assets.as_slice()
        && only["flavor"].is_null()
        && let Some(url) = asset_url(only)
    {
        return Ok(Source::Upload(url.to_owned()));
    }
    let mut best: Option<Source> = None;
    for asset in assets {
        let (Some(url), Some(kbps)) = (
            asset_url(asset),
            asset["flavor"].as_str().and_then(ctr_kbps),
        ) else {
            continue;
        };
        let better = match &best {
            Some(Source::Encode { kbps: best, .. }) => kbps > *best,
            _ => true,
        };
        if better {
            best = Some(Source::Encode {
                playlist: url.to_owned(),
                kbps,
            });
        }
    }
    best.ok_or_else(|| playback_error("no Widevine asset (only FairPlay's)"))
}

/// An asset's `URL`, when it has one that is not empty.
fn asset_url(asset: &serde_json::Value) -> Option<&str> {
    asset["URL"].as_str().filter(|url| !url.is_empty())
}

/// The bit rate a Widevine flavour names: the decimal number after its `:ctrp`
/// (`28:ctrp256` is 256 kbps). `None` for another flavour, or a rest that is not a number
/// fitting in 32 bits.
fn ctr_kbps(flavor: &str) -> Option<u32> {
    let (_, rest) = flavor.split_once(CTR)?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

/// A catalog song's file and key, from its HLS media playlist `body` at `playlist_url`
/// (RFC 8216): the key and the map in force for the first segment, as `m3u8-rs` gives them
/// in it. The key's `URI` is `<prefix>,<key id in standard base64>`, kept whole for the
/// licence; the map's `URI`, resolved against `playlist_url` (RFC 3986 §5), is the whole
/// fragmented MP4.
///
/// `None` when `body` is not a media playlist, has no segment, or its first segment has no
/// key (`METHOD=NONE`, or no `URI`), no key id of 16 bytes, or no map that resolves.
fn catalog_file(playlist_url: &str, body: &[u8]) -> Option<EncryptedFile> {
    let playlist = m3u8_rs::parse_media_playlist_res(body).ok()?;
    let first = playlist.segments.first()?;
    let key = first.key.as_ref()?;
    if key.method == m3u8_rs::KeyMethod::None {
        return None;
    }
    let uri = key.uri.as_ref()?;
    let (_, encoded) = uri.split_once(',')?;
    let id: [u8; 16] = STANDARD.decode(encoded).ok()?.try_into().ok()?;
    let map = first.map.as_ref()?;
    let url = url::Url::parse(playlist_url).ok()?.join(&map.uri).ok()?;
    Some(EncryptedFile {
        url: url.into(),
        keys: vec![Key {
            uri: uri.clone(),
            id,
        }],
    })
}

/// The licence in `answer`, the licence server's JSON: its `license`, in standard base64.
///
/// # Errors
///
/// A playback error when `errorCode` is a number other than 0 (naming it), or when the
/// licence is missing, empty or does not decode. No error carries the licence.
fn licence_in(answer: &serde_json::Value) -> Result<Vec<u8>> {
    if let Some(code) = answer["errorCode"].as_number()
        && code.as_f64().is_some_and(|code| code != 0.0)
    {
        return Err(playback_error(format!(
            "licence refused (errorCode {code})"
        )));
    }
    answer["license"]
        .as_str()
        .and_then(|licence| STANDARD.decode(licence).ok())
        .filter(|licence| !licence.is_empty())
        .ok_or_else(|| playback_error("licence response without a licence"))
}

/// A `cbcs` sample of Spatial Audio or the live radio, as the CDM decrypts it with `key_id` and
/// `iv`: its runs, and the pattern its file gives.
pub(crate) fn cbcs_sample<'a>(
    sample: &'a cenc::Sample<'_>,
    key_id: &'a [u8; 16],
    iv: &'a [u8; 16],
) -> widevine::Protected<'a> {
    widevine::Protected {
        key_id,
        iv,
        subsamples: sample.subsamples,
        scheme: widevine::Scheme::Cbcs {
            crypt: sample.pattern.crypt,
            skip: sample.pattern.skip,
        },
    }
}

/// The fragmented MP4 `resp` brings, readable from its first fragment on
/// while the rest downloads, `decrypt` working through its samples, each
/// `frame` long. A download that says no length is read whole first.
async fn as_it_downloads(
    mut resp: reqwest::Response,
    again: reqwest::RequestBuilder,
    frame: std::time::Duration,
    decrypt: cenc::Decrypt,
) -> Result<AudioStream> {
    let Some(len) = resp.content_length() else {
        return read_whole(resp, again, frame, decrypt).await;
    };
    let (stream, feed) = AudioStream::downloading(len, frame, decrypt);
    let failed = |e: reqwest::Error| format!("asset download: {}", e.without_url());
    let mut received = 0u64;
    while feed.samples() == 0 {
        match resp.chunk().await {
            Ok(Some(bytes)) => {
                received += bytes.len() as u64;
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
        // Nobody reading it any more, `push` says so.
        let ended = crate::resume::carry_on(
            resp,
            received,
            |from| crate::resume::rest_of(&again, from),
            |bytes| feed.push(bytes),
        )
        .await;
        feed.end(ended.err().map(|e| format!("asset download: {e}")));
    });
    Ok(stream)
}

/// The most of a download without a length that is read: hours of a song's audio.
const LONGEST_WHOLE_DOWNLOAD: usize = 512 * 1024 * 1024;

/// The fragmented MP4 `resp` brings, read to its end (carried on from where it broke off,
/// asking `again`), then indexed, made to read as a plain MP4 and decrypted in the
/// background by `decrypt`, its samples each `frame` long.
///
/// # Errors
///
/// A playback error when the download breaks off for good or passes
/// [`LONGEST_WHOLE_DOWNLOAD`], when the file does not index, or when it has no sample.
async fn read_whole(
    resp: reqwest::Response,
    again: reqwest::RequestBuilder,
    frame: std::time::Duration,
    decrypt: cenc::Decrypt,
) -> Result<AudioStream> {
    let mut data = Vec::new();
    let mut too_long = false;
    let ended = crate::resume::carry_on(
        resp,
        0,
        |from| crate::resume::rest_of(&again, from),
        |bytes| {
            too_long = data.len() + bytes.len() > LONGEST_WHOLE_DOWNLOAD;
            if !too_long {
                data.extend_from_slice(bytes);
            }
            !too_long
        },
    )
    .await;
    ended.map_err(|e| playback_error(format!("asset download: {e}")))?;
    if too_long {
        return Err(playback_error(format!(
            "asset download: longer than {} MiB",
            LONGEST_WHOLE_DOWNLOAD / (1024 * 1024)
        )));
    }
    let (movie, samples) =
        cenc::index(&data).map_err(|e| playback_error(format!("asset download: {e}")))?;
    if samples.is_empty() {
        return Err(playback_error("no encrypted samples found in the track"));
    }
    cenc::make_clear(&mut data, &movie, &samples);
    Ok(AudioStream::decrypting(data, samples, frame, decrypt))
}

/// The audio's bit rate, from its `samples` and their `bytes`: each sample one AAC frame.
fn audio_kbps(samples: usize, bytes: usize) -> Option<u32> {
    let seconds = samples as f64 * AAC_FRAME.as_secs_f64();
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
    id: [u8; 16],
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
                id,
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
        .unwrap_or_else(|| widevine::pssh_for_key(&key.id))
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
        let licence_host = format!("{}/", self.hosts.licence);
        tokio::spawn(async move {
            let _ = http.head(licence_host).send().await;
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
            // The sample entries' key ids are zero; the playlist names them, in order.
            let ids: Vec<[u8; 16]> = file.keys.iter().map(|k| k.id).collect();
            cenc::write_key_ids(&mut init, &ids);
            let first = &file.keys[0];
            return Ok(Some(Protected {
                url: file.url.clone(),
                key_id: first.id.to_vec(),
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
        let Source::Encode {
            playlist: url,
            kbps,
        } = choose_source(&song)?
        else {
            return Ok(None);
        };
        let playlist = self.download(&url).await?;
        let file = catalog_file(&url, &playlist)
            .ok_or_else(|| playback_error("catalog asset without a Widevine key"))?;
        let key_id = file.keys[0].id;
        let pssh = widevine::pssh_for_key(&key_id);
        Ok(Some(Protected {
            url: file.url.clone(),
            playlist: widevine_playlist(&String::from_utf8_lossy(&playlist), &url, &pssh),
            pssh,
            key_id: key_id.to_vec(),
            spatial: false,
            init: None,
            kbps,
            licensed_as,
            is_library: id_kind(adam_id) == Some(IdKind::Library),
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
        if id_kind(adam_id) != Some(IdKind::Library) {
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
            end = start.checked_add(range.length)?;
            last = Some(start);
        }
        let from = last?;
        // Its `moof` is a few kilobytes at most.
        let to = from.saturating_add(16 * 1024).min(end).checked_sub(1)?;
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
        let last = first.offset?.checked_add(first.length)?.checked_sub(1)?;
        let resp = self
            .asset_request(&protected.url)
            .header("Range", format!("bytes=0-{last}"))
            .send()
            .await
            .ok()?;
        let head = resp.bytes().await.ok()?;
        let (_, samples) = cenc::index(&head).ok()?;
        let audio: usize = samples.iter().map(|s| s.size).sum();
        let boxes = first.length.checked_sub(audio as u64)? as f64 / samples.len() as f64;
        let bytes = playlist
            .segments
            .iter()
            .filter_map(|s| s.byte_range.as_ref())
            .try_fold(0u64, |bytes, r| bytes.checked_add(r.length))?;
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
            self.http.head(format!("{}/", self.hosts.play)).send(),
            self.http.head(ASSET_HOST).send(),
            self.http.head(format!("{}/", self.hosts.licence)).send(),
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
        self.ask_licence(
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
            && let Ok(stream) = self
                .fetch_enhanced(cdm, &licensed_as, &file, map_range(&body))
                .await
        {
            return Ok((stream, None, true));
        }
        let source = choose_source(&song)?;
        let (Source::Encode { playlist: url, .. } | Source::Upload(url)) = &source;

        let fetched = self.download(url).await?;

        let Some(file) = catalog_file(url, &fetched) else {
            return match source {
                // An upload that isn't a playlist is the user's file itself.
                Source::Upload(_) => Ok((AudioStream::plain(fetched), None, false)),
                Source::Encode { .. } => {
                    Err(playback_error("catalog asset without a Widevine key"))
                }
            };
        };

        let key = file.keys[0].clone();
        let (session, challenge) = cdm
            .open_session(&widevine::pssh_for_key(&key.id))
            .map_err(|e| playback_error(e.to_string()))?;
        let (resp, ()) = tokio::try_join!(
            self.start_download(&file.url),
            self.take_licence(
                &session,
                &challenge,
                &licensed_as,
                id_kind(adam_id) == Some(IdKind::Library),
                &key.uri,
                &key.id,
            ),
        )?;
        let decrypt = Box::new(move |bytes: &mut [u8], sample: &cenc::Sample<'_>| {
            let protected = widevine::Protected {
                key_id: &key.id,
                iv: &sample.iv,
                subsamples: sample.subsamples,
                scheme: widevine::Scheme::Cenc,
            };
            session
                .decrypt(bytes, &protected)
                .map_err(|e| e.to_string())
        });
        let again = self.asset_request(&file.url);
        let stream = as_it_downloads(resp, again, AAC_FRAME, decrypt).await?;
        let kbps = stream
            .with_samples(|samples| audio_kbps(samples.len(), samples.iter().map(|s| s.size).sum()))
            .flatten();
        Ok((stream, kbps, false))
    }

    /// An enhanced file (Spatial Audio's), licensed and downloading: its
    /// sample entries each name their key and constant IV, which its init
    /// segment says (at `init`, `(offset, length)`, as its playlist gives it),
    /// and its frames are 48 kHz ones.
    async fn fetch_enhanced(
        &self,
        cdm: &Cdm,
        adam_id: &str,
        file: &EncryptedFile,
        init: Option<(u64, u64)>,
    ) -> Result<AudioStream> {
        let license = |key: &Key| {
            let key = key.clone();
            async move {
                let (session, challenge) = cdm
                    .open_session(&key_pssh(&key))
                    .map_err(|e| playback_error(e.to_string()))?;
                self.take_licence(&session, &challenge, adam_id, false, &key.uri, &key.id)
                    .await?;
                Ok::<_, Error>((session, key.id))
            }
        };
        // Without the playlist's range, the movie box is in the first few kilobytes.
        let head = self.range(&file.url, init.unwrap_or((0, 16 * 1024)));
        let (keys, head, resp) = tokio::try_join!(
            futures_util::future::try_join_all(file.keys.iter().map(license)),
            head,
            self.start_download(&file.url),
        )?;
        let movie = cenc::Movie::read(&head)
            .map_err(|e| playback_error(format!("enhanced file: {e}")))?
            .ok_or_else(|| {
                playback_error("enhanced file: its init segment has no whole movie box")
            })?;
        let entries = movie.tracks.first().map_or(&[][..], |t| &t.entries);
        if !entries.iter().any(|e| e.protection.is_some()) {
            return Err(playback_error("enhanced file without its keys' IVs"));
        }
        let rate = entries
            .first()
            .and_then(|e| e.audio_config.as_deref())
            .and_then(aac_rate)
            .unwrap_or(48_000);
        let frame = std::time::Duration::from_nanos(
            u64::from(AAC_FRAMES) * 1_000_000_000 / u64::from(rate),
        );
        let decrypt = Box::new(move |bytes: &mut [u8], sample: &cenc::Sample<'_>| {
            let entry = sample.entry as usize;
            // Each sample entry's key, in the playlist's order; another of
            // them should that not be it.
            let order = keys
                .iter()
                .skip(entry)
                .chain(keys.iter().take(entry.min(keys.len())));
            let mut last = String::from("no key");
            for (session, key_id) in order {
                match session.decrypt(bytes, &cbcs_sample(sample, key_id, &sample.iv)) {
                    Ok(()) => return Ok(()),
                    Err(e) => last = e.to_string(),
                }
            }
            Err(last)
        });
        let again = self.asset_request(&file.url);
        as_it_downloads(resp, again, frame, decrypt).await
    }

    /// The licence for `challenge`, a CDM's licence request, for the key at `key_uri`: of
    /// the song licensed as `adam_id`, a library id when `is_library`. Its bytes, as the
    /// licence server gives them, for a CDM to take.
    ///
    /// # Errors
    ///
    /// [`Error::NotSignedIn`] on a 401 or 403 once the developer token was renewed; a
    /// playback error naming another failure status, a refusal (its `errorCode`) or an
    /// answer without a licence; [`Error::Protocol`] for an answer that is not JSON.
    async fn ask_licence(
        &self,
        challenge: &[u8],
        adam_id: &str,
        is_library: bool,
        key_uri: &str,
    ) -> Result<Vec<u8>> {
        let body = serde_json::json!({
            "challenge": STANDARD.encode(challenge),
            "key-system": KEY_SYSTEM,
            "uri": key_uri,
            "adamId": adam_id,
            "isLibrary": is_library,
            "user-initiated": true,
        });
        let resp = self
            .send(
                self.http
                    .post(format!("{}{LICENSE}", self.hosts.licence))
                    .header("x-apple-music-user-token", &self.user_token)
                    .json(&body),
                "licence",
            )
            .await?;
        licence_in(&json_body(resp, "licence").await?)
    }

    /// Ask for the licence as [`ask_licence`](Self::ask_licence) does, and give it to
    /// `session`, which returns once `key_id`, the key it unlocks, is usable. The content key
    /// stays inside the CDM.
    ///
    /// # Errors
    ///
    /// Those of [`ask_licence`](Self::ask_licence); a playback error carrying `widevine`'s
    /// message when the session fails to take the licence.
    async fn take_licence(
        &self,
        session: &Session,
        challenge: &[u8],
        adam_id: &str,
        is_library: bool,
        key_uri: &str,
        key_id: &[u8; 16],
    ) -> Result<()> {
        let licence = self
            .ask_licence(challenge, adam_id, is_library, key_uri)
            .await?;
        session
            .give_licence(&licence, key_id)
            .map_err(|e| playback_error(e.to_string()))
    }

    async fn web_playback(&self, adam_id: &str) -> Result<serde_json::Value> {
        let resp = self
            .send(
                self.http
                    .post(format!("{}{WEB_PLAYBACK}", self.hosts.play))
                    .header("x-apple-music-user-token", &self.user_token)
                    .json(&playback_request(adam_id)?),
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

    /// `(offset, length)` of the file at `url`.
    async fn range(&self, url: &str, (offset, length): (u64, u64)) -> Result<Vec<u8>> {
        let failed =
            |e: reqwest::Error| Error::Network(format!("asset download: {}", e.without_url()));
        // A range of no byte, or past the largest offset, is no part of a file.
        let last = offset
            .checked_add(length)
            .and_then(|end| end.checked_sub(1))
            .filter(|_| length > 0)
            .ok_or_else(|| {
                Error::Protocol("asset download: an empty or impossible range".into())
            })?;
        let resp = self
            .asset_request(url)
            .header("Range", format!("bytes={offset}-{last}"))
            .send()
            .await
            .map_err(failed)?;
        // The part asked for, and only it: an error page, or the whole file,
        // isn't what was asked.
        if resp.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(Error::Protocol(format!(
                "asset download: HTTP {}",
                resp.status()
            )));
        }
        let bytes = resp.bytes().await.map_err(failed)?;
        if bytes.len() as u64 != length {
            return Err(Error::Protocol("asset download: cut short".into()));
        }
        Ok(bytes.to_vec())
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
    fn the_bit_rate_is_read_from_the_samples() {
        // 744 bytes a frame of 1024 at 44.1 kHz: 256 kbps.
        assert_eq!(audio_kbps(430, 430 * 744), Some(256));
        assert_eq!(audio_kbps(0, 0), None);
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
    fn the_sample_rate_is_read_from_the_audio_config() {
        // AAC-LC: 48 kHz, then 44.1 kHz.
        assert_eq!(aac_rate(&[0x11, 0x90]), Some(48_000));
        assert_eq!(aac_rate(&[0x12, 0x10]), Some(44_100));
    }
}

#[cfg(test)]
mod reading_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ids_are_of_one_kind_or_neither() {
        assert_eq!(id_kind("1234567890"), Some(IdKind::Catalog));
        assert_eq!(id_kind("0"), Some(IdKind::Catalog));
        for library in ["i.AbC9-x", "l.a", "p.Z-0", "a.-"] {
            assert_eq!(id_kind(library), Some(IdKind::Library), "{library}");
        }
        for neither in [
            "", "i.", ".x", "x.abc", "ii.abc", "I.abc", "i.a_b", "i.a.b", "i.é", "12a", "-1", "+1",
            " 1", "1 ", "pl.u-abc", "ra.123",
        ] {
            assert_eq!(id_kind(neither), None, "{neither}");
        }
    }

    #[test]
    fn the_request_names_the_id_by_its_kind() {
        assert_eq!(
            playback_request("1234").unwrap(),
            json!({ "salableAdamId": "1234" })
        );
        assert_eq!(
            playback_request("i.Up-1").unwrap(),
            json!({ "universalLibraryId": "i.Up-1" })
        );
        let error = playback_request("ra.99").unwrap_err();
        assert!(
            matches!(&error, Error::Playback(m) if m.contains("ra.99")),
            "{error}"
        );
    }

    #[test]
    fn the_best_widevine_encode_is_chosen() {
        let entry = json!({ "assets": [
            { "flavor": "30:cbcp256", "URL": "https://a/fairplay.m3u8" },
            { "flavor": "32:ctrp64", "URL": "https://a/64.m3u8" },
            { "flavor": "28:ctrp256", "URL": "https://a/first256.m3u8" },
            { "flavor": "29:ctrp256", "URL": "https://a/second256.m3u8" },
            { "flavor": "28:ctrp320", "URL": "" },
            { "flavor": "28:ctrp9999999999", "URL": "https://a/too-big.m3u8" },
            { "flavor": "28:ctrp+512", "URL": "https://a/signed.m3u8" },
            { "flavor": 28, "URL": "https://a/number.m3u8" },
        ]});
        assert_eq!(
            choose_source(&entry).unwrap(),
            Source::Encode {
                playlist: "https://a/first256.m3u8".into(),
                kbps: 256
            }
        );
    }

    #[test]
    fn an_upload_is_its_one_asset_without_a_flavour() {
        let upload = json!({ "assets": [{ "URL": "https://a/upload.m4a" }] });
        assert_eq!(
            choose_source(&upload).unwrap(),
            Source::Upload("https://a/upload.m4a".into())
        );
        // Two such assets, or one without an address, are no upload.
        for not_upload in [
            json!({ "assets": [{ "URL": "https://a/1" }, { "URL": "https://a/2" }] }),
            json!({ "assets": [{ "URL": "" }] }),
            json!({ "assets": [{ "URL": 7 }] }),
        ] {
            let error = choose_source(&not_upload).unwrap_err().to_string();
            assert!(error.contains("no Widevine asset"), "{error}");
        }
    }

    #[test]
    fn no_assets_and_fairplay_only_are_errors() {
        for none in [
            json!({}),
            json!({ "assets": [] }),
            json!({ "assets": {} }),
            json!(null),
        ] {
            let error = choose_source(&none).unwrap_err().to_string();
            assert!(
                error.contains("webPlayback returned no assets"),
                "{none}: {error}"
            );
        }
        let fairplay = json!({ "assets": [{ "flavor": "30:cbcp256", "URL": "https://a/x" }] });
        let error = choose_source(&fairplay).unwrap_err().to_string();
        assert!(
            error.contains("no Widevine asset (only FairPlay's)"),
            "{error}"
        );
    }

    #[test]
    fn a_flavours_bit_rate_is_the_number_after_ctrp() {
        assert_eq!(ctr_kbps("28:ctrp256"), Some(256));
        assert_eq!(ctr_kbps("32:ctrp64"), Some(64));
        assert_eq!(ctr_kbps("x:ctrp4294967295"), Some(u32::MAX));
        for none in [
            "28:ctrp",
            "28:ctrp4294967296",
            "28:cbcp256",
            "28:ctrp25 6",
            "",
            ":ctrp-1",
        ] {
            assert_eq!(ctr_kbps(none), None, "{none}");
        }
    }

    /// A catalog song's media playlist as Apple writes it, its key's URI `uri`.
    fn media_playlist(uri: &str) -> String {
        format!(
            "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXT-X-VERSION:7\n\
             #EXT-X-KEY:METHOD=ISO-23001-7,URI=\"{uri}\",KEYFORMAT=\"com.apple.streamingkeydelivery\"\n\
             #EXT-X-MAP:URI=\"../files/song.mp4\",BYTERANGE=\"700@0\"\n\
             #EXTINF:9.9,\n#EXT-X-BYTERANGE:5000@700\n../files/song.mp4\n\
             #EXTINF:9.9,\n#EXT-X-BYTERANGE:5000@5700\n../files/song.mp4\n#EXT-X-ENDLIST\n"
        )
    }

    fn key_uri() -> String {
        format!(
            "skd://made-up.example/key,{}",
            STANDARD.encode(*b"0123456789abcdef")
        )
    }

    #[test]
    fn the_playlist_gives_the_file_and_its_key() {
        let file = catalog_file(
            "https://aod.example/a/b/playlist.m3u8?sig=1",
            media_playlist(&key_uri()).as_bytes(),
        )
        .unwrap();
        assert_eq!(file.url, "https://aod.example/a/files/song.mp4");
        assert_eq!(
            file.keys,
            [Key {
                uri: key_uri(),
                id: *b"0123456789abcdef"
            }]
        );
    }

    #[test]
    fn the_map_resolves_as_rfc_3986_says() {
        let at = |map: &str| {
            let body =
                media_playlist(&key_uri()).replace("../files/song.mp4\",", &format!("{map}\","));
            catalog_file("https://aod.example/a/b/p.m3u8", body.as_bytes()).map(|f| f.url)
        };
        assert_eq!(
            at("https://other.example/x.mp4").as_deref(),
            Some("https://other.example/x.mp4")
        );
        assert_eq!(
            at("x.mp4").as_deref(),
            Some("https://aod.example/a/b/x.mp4")
        );
        assert_eq!(
            at("/root.mp4").as_deref(),
            Some("https://aod.example/root.mp4")
        );
        assert_eq!(
            at("../../../up.mp4").as_deref(),
            Some("https://aod.example/up.mp4")
        );
        assert_eq!(
            at("//cdn.example/x.mp4").as_deref(),
            Some("https://cdn.example/x.mp4")
        );
    }

    #[test]
    fn the_key_in_force_for_the_first_segment_is_the_last_before_it() {
        let earlier = format!("skd://earlier,{}", STANDARD.encode([7u8; 16]));
        let body = media_playlist(&key_uri()).replace(
            "#EXT-X-KEY",
            &format!("#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"{earlier}\"\n#EXT-X-KEY"),
        );
        let file = catalog_file("https://a.example/p.m3u8", body.as_bytes()).unwrap();
        assert_eq!(file.keys[0].uri, key_uri());
    }

    #[test]
    fn a_playlist_without_a_usable_key_or_map_gives_no_file() {
        let url = "https://a.example/p.m3u8";
        let short = format!("skd://k,{}", STANDARD.encode([1u8; 15]));
        let unpadded = format!(
            "skd://k,{}",
            STANDARD.encode([1u8; 16]).trim_end_matches('=')
        );
        for body in [
            String::new(),
            "not a playlist".into(),
            "#EXTM3U\n#EXT-X-TARGETDURATION:10\n".into(),
            media_playlist(&key_uri()).replace("METHOD=ISO-23001-7", "METHOD=NONE"),
            media_playlist("no-comma-here"),
            media_playlist(&short),
            media_playlist(&unpadded),
            media_playlist("skd://k,not base64!"),
            media_playlist(&key_uri()).replace(
                "#EXT-X-MAP:URI=\"../files/song.mp4\",BYTERANGE=\"700@0\"\n",
                "",
            ),
            media_playlist(&key_uri()).replace(&format!("URI=\"{}\",", key_uri()), ""),
        ] {
            assert_eq!(catalog_file(url, body.as_bytes()), None, "{body}");
        }
        assert_eq!(
            catalog_file("not an address", media_playlist(&key_uri()).as_bytes()),
            None
        );
    }

    #[test]
    fn the_licence_is_read_or_refused() {
        let licence = STANDARD.encode(b"made-up licence");
        assert_eq!(
            licence_in(&json!({ "license": licence, "errorCode": 0 })).unwrap(),
            b"made-up licence"
        );
        let refused = licence_in(&json!({ "errorCode": 1005, "license": licence })).unwrap_err();
        assert_eq!(
            refused.to_string(),
            "playback failed: licence refused (errorCode 1005)"
        );
        let negative = licence_in(&json!({ "errorCode": -1 })).unwrap_err();
        assert!(negative.to_string().contains("errorCode -1"), "{negative}");
        for without in [
            json!({}),
            json!({ "errorCode": "1" }),
            json!({ "license": "" }),
            json!({ "license": "not base64!" }),
            json!({ "license": 12 }),
            json!([]),
        ] {
            let error = licence_in(&without).unwrap_err().to_string();
            assert!(
                error.contains("licence response without a licence"),
                "{without}: {error}"
            );
        }
    }
}

#[cfg(test)]
mod http_tests {
    use super::*;
    use crate::test_support::{
        Reply, Request, USER_TOKEN, apple_music, apple_server, encrypted_track, json_reply, reply,
        xor_decrypt,
    };
    use opium_core::model::ProviderKind;
    use serde_json::json;
    use std::io::Read;

    fn port_of(request: &Request) -> &str {
        request.host.rsplit(':').next().unwrap_or("")
    }

    fn key_uri() -> String {
        format!(
            "skd://made-up.example/k,{}",
            STANDARD.encode(*b"fedcba9876543210")
        )
    }

    /// Apple's playback hosts for one catalog song `1234` and one matched library song
    /// `i.Lib-1`, its licences answered with `licence`.
    fn playback_server(
        playlist: String,
        licence: impl Fn() -> Reply + Send + Sync + 'static,
    ) -> crate::test_support::TestServer {
        apple_server(
            move |request| match (request.host_name(), request.path.as_str()) {
                ("play.music.apple.com", WEB_PLAYBACK) => {
                    let asked = request.json();
                    let song_id = if asked["universalLibraryId"] == "i.Lib-1" {
                        json!(555)
                    } else {
                        json!(-1)
                    };
                    let asset = |flavor: &str, name: &str| {
                        json!({
                            "flavor": flavor,
                            "URL": format!("http://aod-ssl.itunes.apple.com:{}/x/{name}", port_of(request)),
                        })
                    };
                    json_reply(&json!({ "songList": [{
                        "songId": song_id,
                        "assets": [
                            asset("30:cbcp256", "fairplay.m3u8"),
                            asset("28:ctrp256", "song.m3u8"),
                            asset("32:ctrp64", "low.m3u8"),
                        ],
                    }]}))
                }
                ("aod-ssl.itunes.apple.com", "/x/song.m3u8") => reply(200, &playlist),
                ("play.itunes.apple.com", LICENSE) => licence(),
                _ => reply(404, ""),
            },
        )
    }

    fn song_playlist(uri: &str) -> String {
        format!(
            "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXT-X-KEY:METHOD=ISO-23001-7,URI=\"{uri}\"\n\
             #EXT-X-MAP:URI=\"song.mp4\",BYTERANGE=\"700@0\"\n#EXTINF:9.9,\n\
             #EXT-X-BYTERANGE:5000@700\nsong.mp4\n#EXT-X-ENDLIST\n"
        )
    }

    #[tokio::test]
    async fn a_song_is_asked_its_encode_chosen_and_its_playlist_read() {
        let server = playback_server(song_playlist(&key_uri()), || reply(500, ""));
        let am = apple_music(&server).await;
        let track = ItemId::new(ProviderKind::AppleMusic, "1234");
        let protected = am.protected(&track, false).await.unwrap().unwrap();
        assert_eq!(
            protected.url,
            format!(
                "http://aod-ssl.itunes.apple.com:{}/x/song.mp4",
                server.port()
            )
        );
        assert_eq!(protected.key_id, b"fedcba9876543210");
        assert_eq!(protected.kbps, 256);
        assert_eq!(protected.licensed_as, "1234");
        assert!(!protected.is_library);

        let asked = &server.requests_to(WEB_PLAYBACK)[0];
        assert_eq!(asked.method, "POST");
        assert_eq!(asked.json(), json!({ "salableAdamId": "1234" }));
        assert!(asked.bearer().is_some());
        assert_eq!(asked.header("x-apple-music-user-token"), Some(USER_TOKEN));
        assert_eq!(asked.header("origin"), Some("https://music.apple.com"));
        assert_eq!(asked.header("referer"), Some("https://music.apple.com/"));
        assert_eq!(
            asked.header("cookie"),
            Some(format!("media-user-token={USER_TOKEN}").as_str())
        );
    }

    #[tokio::test]
    async fn a_licence_is_asked_with_the_key_and_refused() {
        let server = playback_server(song_playlist(&key_uri()), || {
            json_reply(&json!({ "errorCode": 3, "license": STANDARD.encode(b"never used") }))
        });
        let am = apple_music(&server).await;
        let track = ItemId::new(ProviderKind::AppleMusic, "i.Lib-1");
        let protected = am.protected(&track, false).await.unwrap().unwrap();
        let error = am
            .license(&protected, b"made-up challenge")
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "playback failed: licence refused (errorCode 3)"
        );

        let asked = &server.requests_to(LICENSE)[0];
        assert_eq!(asked.host_name(), "play.itunes.apple.com");
        assert_eq!(
            asked.json(),
            json!({
                "challenge": STANDARD.encode(b"made-up challenge"),
                "key-system": "com.widevine.alpha",
                "uri": key_uri(),
                "adamId": "555",
                "isLibrary": true,
                "user-initiated": true,
            })
        );
        assert!(asked.bearer().is_some());
        assert_eq!(asked.header("x-apple-music-user-token"), Some(USER_TOKEN));
        assert_eq!(
            asked.header("cookie"),
            Some(format!("media-user-token={USER_TOKEN}").as_str())
        );
    }

    #[tokio::test]
    async fn a_licence_granted_is_its_bytes() {
        let server = playback_server(song_playlist(&key_uri()), || {
            json_reply(&json!({ "license": STANDARD.encode(b"made-up licence") }))
        });
        let am = apple_music(&server).await;
        let licence = am
            .ask_licence(b"c", "1234", false, &key_uri())
            .await
            .unwrap();
        assert_eq!(licence, b"made-up licence");
    }

    #[tokio::test]
    async fn a_licence_refused_by_status_says_so() {
        for (status, expected) in [(403, "not signed in"), (500, "licence: HTTP 500")] {
            let server = playback_server(song_playlist(&key_uri()), move || reply(status, "{}"));
            let am = apple_music(&server).await;
            let error = am
                .ask_licence(b"c", "1234", false, &key_uri())
                .await
                .unwrap_err();
            assert!(error.to_string().contains(expected), "{status}: {error}");
        }
        let server = playback_server(song_playlist(&key_uri()), || reply(200, "not json"));
        let am = apple_music(&server).await;
        let error = am
            .ask_licence(b"c", "1234", false, &key_uri())
            .await
            .unwrap_err();
        assert!(matches!(error, Error::Protocol(_)), "{error}");
    }

    #[tokio::test]
    async fn a_playlist_without_a_key_is_said_so() {
        let server = playback_server(
            "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXT-X-MAP:URI=\"song.mp4\"\n#EXTINF:9.9,\nsong.mp4\n".into(),
            || reply(500, ""),
        );
        let am = apple_music(&server).await;
        let track = ItemId::new(ProviderKind::AppleMusic, "1234");
        let error = am.protected(&track, false).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "playback failed: catalog asset without a Widevine key"
        );
    }

    #[tokio::test]
    async fn an_id_of_neither_kind_is_refused_before_any_request() {
        let server = playback_server(String::new(), || reply(500, ""));
        let am = apple_music(&server).await;
        let before = server.requests().len();
        let track = ItemId::new(ProviderKind::AppleMusic, "../../etc");
        let error = am.protected(&track, false).await.unwrap_err();
        assert!(error.to_string().contains("not a song id"), "{error}");
        // The licence host's connection, opened ahead, is all that may have gone out.
        assert!(
            server.requests()[before..]
                .iter()
                .all(|r| r.method == "HEAD")
        );
    }

    /// The whole of `stream`, read off the runtime, which its download goes on in.
    async fn read_all(stream: AudioStream) -> Vec<u8> {
        tokio::task::spawn_blocking(move || {
            let mut stream = stream;
            let mut out = Vec::new();
            stream.read_to_end(&mut out).expect("the stream reads");
            out
        })
        .await
        .expect("the reading thread")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_download_with_or_without_a_length_reads_decrypted() {
        let (file, clear) = encrypted_track(3, 4, 300);
        let served = file.clone();
        let server = apple_server(move |request| {
            let whole = reply(200, "").bytes(served.clone());
            match request.path.as_str() {
                "/chunked.mp4" => whole.chunked(),
                _ => whole,
            }
        });
        let am = apple_music(&server).await;
        for path in ["/chunked.mp4", "/sized.mp4"] {
            let url = format!("http://aod-ssl.itunes.apple.com:{}{path}", server.port());
            let resp = am.start_download(&url).await.unwrap();
            assert_eq!(resp.content_length().is_some(), path == "/sized.mp4");
            let stream = as_it_downloads(resp, am.asset_request(&url), AAC_FRAME, xor_decrypt())
                .await
                .unwrap();
            assert_eq!(read_all(stream).await, clear, "{path}");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_download_without_a_length_that_is_no_track_fails() {
        let (file, _) = encrypted_track(1, 2, 50);
        let movie_only = file[..file.len() - 200].to_vec();
        let server = apple_server(move |request| match request.path.as_str() {
            "/junk.mp4" => reply(200, "not an mp4 at all").chunked(),
            _ => reply(200, "").bytes(movie_only.clone()).chunked(),
        });
        let am = apple_music(&server).await;
        for path in ["/junk.mp4", "/cut.mp4"] {
            let url = format!("http://aod-ssl.itunes.apple.com:{}{path}", server.port());
            let resp = am.start_download(&url).await.unwrap();
            let error = as_it_downloads(resp, am.asset_request(&url), AAC_FRAME, xor_decrypt())
                .await
                .err()
                .expect("no track");
            assert!(matches!(error, Error::Playback(_)), "{path}: {error}");
        }
    }

    #[tokio::test]
    async fn empty_or_impossible_ranges_are_errors_not_panics() {
        let server = apple_server(|_| reply(206, "x"));
        let am = apple_music(&server).await;
        let url = format!("http://aod-ssl.itunes.apple.com:{}/f.mp4", server.port());
        for range in [(0, 0), (5, 0), (u64::MAX, 2)] {
            assert!(am.range(&url, range).await.is_err(), "{range:?}");
        }
        assert_eq!(am.range(&url, (3, 1)).await.unwrap(), b"x");
    }

    #[tokio::test]
    async fn byte_ranges_of_nothing_or_past_the_end_measure_nothing() {
        let server = apple_server(|_| reply(206, ""));
        let am = apple_music(&server).await;
        let url = format!("http://aod-ssl.itunes.apple.com:{}/f.mp4", server.port());
        for byte_range in [
            "0@0",
            "18446744073709551615@18446744073709551615",
            "5@18446744073709551615",
        ] {
            let playlist = format!(
                "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXT-X-MAP:URI=\"f.mp4\",BYTERANGE=\"{byte_range}\"\n\
                 #EXTINF:10,\n#EXT-X-BYTERANGE:{byte_range}\nf.mp4\n#EXTINF:10,\n#EXT-X-BYTERANGE:{byte_range}\nf.mp4\n"
            );
            assert_eq!(
                am.padding_of(&url, playlist.as_bytes()).await,
                None,
                "{byte_range}"
            );
            let protected = Protected {
                url: url.clone(),
                key_id: KEY_ID_BYTES.to_vec(),
                pssh: Vec::new(),
                spatial: false,
                init: None,
                playlist,
                kbps: 64,
                licensed_as: "1".into(),
                is_library: false,
                file: EncryptedFile {
                    url: url.clone(),
                    keys: vec![Key {
                        uri: "k,AAAA".into(),
                        id: KEY_ID_BYTES,
                    }],
                },
            };
            assert_eq!(am.lesser_kbps(&protected).await, None, "{byte_range}");
        }
    }

    const KEY_ID_BYTES: [u8; 16] = [3; 16];
}

/// The readers of this file, and Opium's own beside them, fed damaged inputs: see
/// `crate::mutation_tests`.
#[cfg(test)]
mod mutation_tests {
    use super::*;
    use crate::mutation_tests::{ROUNDS, damaged_json, damaged_texts, is_digits};
    use crate::test_support::{Rng, mutate_bytes};
    use serde_json::{Value, json};

    #[test]
    fn ids() {
        let seeds = [
            "1234567890",
            "i.AbC9-x",
            "pl.u-1b2",
            "ra.123",
            "l.x",
            "a.Z-9",
            "",
        ]
        .map(String::from);
        let mut kinds = [0; 3];
        for id in damaged_texts(0x1D5, &seeds) {
            kinds[match id_kind(&id) {
                Some(IdKind::Catalog) => 0,
                Some(IdKind::Library) => 1,
                None => 2,
            }] += 1;
            match id_kind(&id) {
                Some(IdKind::Catalog) => assert!(is_digits(&id), "{id:?}"),
                Some(IdKind::Library) => {
                    let (kind, name) = id.split_once('.').expect("a dot");
                    assert!(["i", "l", "p", "a"].contains(&kind), "{id:?}");
                    assert!(
                        !name.is_empty()
                            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    );
                }
                None => {}
            }
            match playback_request(&id) {
                Ok(body) => {
                    let object = body.as_object().expect("an object");
                    assert_eq!(object.len(), 1);
                    let key = if id_kind(&id) == Some(IdKind::Catalog) {
                        "salableAdamId"
                    } else {
                        "universalLibraryId"
                    };
                    assert_eq!(object[key], id.as_str());
                }
                Err(_) => assert_eq!(id_kind(&id), None),
            }
        }
        assert!(kinds.iter().all(|&n| n > 0), "each kind met: {kinds:?}");
    }

    /// A `songList` entry of a catalog song, and of an upload.
    fn entries() -> Vec<Value> {
        vec![
            json!({ "songId": 900000001, "assets": [
                { "flavor": "30:cbcp256", "URL": "https://aod.example/a/f.m3u8" },
                { "flavor": "28:ctrp256", "URL": "https://aod.example/a/w.m3u8" },
                { "flavor": "32:ctrp64", "URL": "https://aod.example/a/l.m3u8" },
            ]}),
            json!({ "songId": "-1", "assets": [{ "URL": "https://aod.example/u/upload.m4a" }] }),
            json!({ "songId": "900000002", "assets": [] }),
        ]
    }

    #[test]
    fn web_playback_entries() {
        let kbps_of = |asset: &Value| {
            let (_, rest) = asset["flavor"].as_str()?.split_once(":ctrp")?;
            if !is_digits(rest) {
                return None;
            }
            rest.parse::<u32>()
                .ok()
                .filter(|_| asset["URL"].as_str().is_some_and(|u| !u.is_empty()))
        };
        let mut outcomes = [0; 3];
        for entry in damaged_json(0xE27, &entries()) {
            if let Some(id) = song_id(&entry) {
                assert!(!id.is_empty() && id != "-1", "{id:?}");
            }
            let assets = entry["assets"].as_array();
            let source = choose_source(&entry);
            outcomes[match &source {
                Ok(Source::Upload(_)) => 0,
                Ok(Source::Encode { .. }) => 1,
                Err(_) => 2,
            }] += 1;
            match source {
                Ok(Source::Upload(url)) => {
                    let assets = assets.expect("assets");
                    assert_eq!(assets.len(), 1);
                    assert!(assets[0]["flavor"].is_null());
                    assert_eq!(assets[0]["URL"], url.as_str());
                    assert!(!url.is_empty());
                }
                Ok(Source::Encode { playlist, kbps }) => {
                    let assets = assets.expect("assets");
                    // The bit rate is read from the chosen asset's flavour, and none is higher.
                    let first_best = assets
                        .iter()
                        .position(|a| kbps_of(a) == Some(kbps))
                        .expect("its asset");
                    assert_eq!(assets[first_best]["URL"], playlist.as_str());
                    assert!(assets.iter().filter_map(kbps_of).all(|other| other <= kbps));
                }
                Err(_) => {
                    assert!(assets.is_none_or(|assets| assets.iter().all(|a| kbps_of(a).is_none())))
                }
            }
        }
        assert!(
            outcomes.iter().all(|&n| n > 0),
            "each outcome met: {outcomes:?}"
        );
    }

    /// A catalog song's media playlist.
    fn media_playlists() -> Vec<String> {
        let key = STANDARD.encode(*b"0123456789abcdef");
        vec![
            format!(
                "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXT-X-VERSION:7\n\
                 #EXT-X-KEY:METHOD=ISO-23001-7,URI=\"skd://made-up/k,{key}\",KEYFORMAT=\"com.apple.streamingkeydelivery\"\n\
                 #EXT-X-MAP:URI=\"song.mp4\",BYTERANGE=\"700@0\"\n\
                 #EXTINF:9.9,\n#EXT-X-BYTERANGE:5000@700\nsong.mp4\n\
                 #EXTINF:9.9,\n#EXT-X-BYTERANGE:5000@5700\nsong.mp4\n#EXT-X-ENDLIST\n"
            ),
            format!(
                "#EXTM3U\n#EXT-X-MAP:URI=\"https://cdn.example/x/../y.mp4\"\n#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"data:;base64,{key}\"\n#EXTINF:1,\ny.mp4\n"
            ),
        ]
    }

    #[test]
    fn media_playlists_of_catalog_songs() {
        let mut read = 0;
        let bodies = damaged_texts(0x3E8, &media_playlists());
        for body in &bodies {
            let Some(file) = catalog_file("https://aod.example/a/b/p.m3u8?x=1", body.as_bytes())
            else {
                continue;
            };
            read += 1;
            assert_eq!(file.keys.len(), 1);
            let key = &file.keys[0];
            let (_, encoded) = key.uri.split_once(',').expect("a comma");
            assert_eq!(STANDARD.decode(encoded).expect("base64"), key.id);
            assert!(body.contains(&key.uri));
            assert!(url::Url::parse(&file.url).is_ok(), "{}", file.url);
        }
        assert!(
            read > 0 && read < bodies.len(),
            "{read} of {} read",
            bodies.len()
        );
    }

    /// An enhanced (Spatial Audio) media playlist, its master, as Apple writes them, cut short.
    fn enhanced() -> (String, String) {
        let pssh = STANDARD.encode(
            [
                &[0, 0, 0, 0x34][..],
                b"pssh",
                &[0; 4],
                &widevine::SYSTEM_ID,
                &[0, 0, 0, 0x14, 0x12, 0x10],
                b"madeupkeyid12345",
            ]
            .concat(),
        );
        let media = format!(
            "#EXTM3U\n#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"skd://made-up/1\",KEYFORMAT=\"com.apple.streamingkeydelivery\"\n\
             #EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"data:text/plain;base64,{pssh}\",KEYFORMAT=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\"\n\
             #EXT-X-MAP:URI=\"e.mp4\",BYTERANGE=\"1000@0\"\n#EXTINF:15,\n#EXT-X-BYTERANGE:4000@1000\ne.mp4\n#EXT-X-ENDLIST\n"
        );
        let master = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1,CODECS=\"mp4a.40.2\",AUDIO=\"audio-stereo-256-binaural\"\n\
                      e.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2,AUDIO=\"audio-stereo-256\"\ns.m3u8\n"
            .to_owned();
        (media, master)
    }

    #[test]
    fn opiums_own_playlist_readers() {
        let (media, master) = enhanced();
        let seeds = [media, master, media_playlists()[0].clone()];
        let url = "https://aod.example/a/p.m3u8";
        let mut read = 0;
        for body in damaged_texts(0xE4A, &seeds) {
            if let Some(file) = read_enhanced_playlist(url, &body) {
                read += 1;
                assert!(!file.keys.is_empty());
                let mut uris: Vec<&str> = file.keys.iter().map(|k| k.uri.as_str()).collect();
                uris.dedup();
                assert_eq!(uris.len(), file.keys.len(), "each key once");
            }
            let exo = enhanced_playlist(&body, url);
            assert!(
                exo.lines()
                    .filter(|l| l.starts_with("#EXT-X-KEY"))
                    .all(|l| l.contains("edef8ba9-79d6-4ace-a3c8-27dcd51d21ed"))
            );
            let _ = widevine_playlist(&body, url, b"pssh");
            if map_range(&body).is_some() {
                assert!(body.contains("#EXT-X-MAP"));
            }
            if variant(url, &body, "audio-stereo-256-binaural").is_some() {
                assert!(body.contains("#EXT-X-STREAM-INF"));
            }
        }
        assert!(read > 0, "some enhanced playlists still read");
    }

    #[test]
    fn audio_configurations_and_song_ids() {
        let mut rng = Rng::new(0xAAC);
        for _ in 0..ROUNDS {
            let asc = mutate_bytes(&mut rng, &[0x12, 0x10]);
            if let Some(rate) = aac_rate(&asc) {
                assert!((7_350..=96_000).contains(&rate));
            }
        }
        let seeds = [
            json!({ "songId": "900000003" }),
            json!({ "songId": 900000004 }),
            json!({ "songId": -1 }),
        ];
        for entry in damaged_json(0x50D, &seeds) {
            if let Some(id) = song_id(&entry) {
                assert!(!id.is_empty() && id != "-1");
            }
        }
    }

    #[test]
    fn licence_answers() {
        let licence = STANDARD.encode(b"a made-up licence of some length");
        let seeds = [
            json!({ "license": licence, "errorCode": 0 }),
            json!({ "errorCode": 1005 }),
            json!({ "license": licence }),
        ];
        let mut granted = 0;
        for answer in damaged_json(0x11C, &seeds) {
            match licence_in(&answer) {
                Ok(bytes) => {
                    granted += 1;
                    assert!(!bytes.is_empty());
                    assert_eq!(answer["license"], STANDARD.encode(&bytes).as_str());
                    assert!(answer["errorCode"].as_f64().is_none_or(|code| code == 0.0));
                }
                Err(e) => {
                    if let Some(given) = answer["license"].as_str().filter(|l| l.len() >= 8) {
                        assert!(!e.to_string().contains(given), "no licence in an error");
                    }
                }
            }
        }
        assert!(granted > 0 && granted < ROUNDS, "{granted} granted");
    }
}
