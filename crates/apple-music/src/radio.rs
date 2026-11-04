//! Apple's live radio (Apple Music 1, Hits, Country…), as the web player plays
//! it: `/v1/play/assets` names an HLS stream of CMAF segments encrypted `cbcs`,
//! and a Widevine licence comes from the radio key server. The segments are
//! decrypted as they air and handed on as an endless ADTS stream, which needs
//! no seeking to decode.

use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use opium_core::model::ItemId;
use opium_core::{Error, Result};
use widevine::{Cdm, Session};

use crate::AppleMusic;

/// The Widevine system id, as HLS names its key format.
const WIDEVINE: &str = "urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed";
/// How far behind the live edge to start, in segments: enough to buffer.
const LEAD_SEGMENTS: usize = 2;
/// How long a station may stay out of reach before playback is told.
const OUT_OF_REACH: Duration = Duration::from_secs(30);

/// What the radio's live playlist says about the stream now.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct MediaPlaylist {
    pub target_duration: f64,
    pub media_sequence: u64,
    /// The Widevine key line's URI (a `data:` URI holding the PSSH) and IV.
    pub key: Option<(String, Option<Vec<u8>>)>,
    /// The init segment.
    pub map: Option<String>,
    pub segments: Vec<String>,
    /// The stream is over (an episode, not what airs).
    pub ended: bool,
}

/// An `ATTR=value` list from an HLS tag, quoted values unquoted.
pub(crate) fn attribute<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = line.split_once(':')?.1;
    while !rest.is_empty() {
        let (key, after) = rest.split_once('=')?;
        let (value, next) = if let Some(quoted) = after.strip_prefix('"') {
            let end = quoted.find('"')?;
            (&quoted[..end], quoted[end + 1..].trim_start_matches(','))
        } else {
            match after.split_once(',') {
                Some((v, n)) => (v, n),
                None => (after, ""),
            }
        };
        if key.trim() == name {
            return Some(value);
        }
        rest = next;
    }
    None
}

fn hex(s: &str) -> Option<Vec<u8>> {
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))?;
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// The variant of a master playlist to play: the best AAC-LC one, else the
/// best of any (the BBC offers only HE-AAC abroad).
pub(crate) fn best_variant(master: &str) -> Option<String> {
    let mut best: Option<((bool, u64), String)> = None;
    let mut lines = master.lines();
    while let Some(line) = lines.next() {
        if !line.starts_with("#EXT-X-STREAM-INF:") {
            continue;
        }
        let lc = attribute(line, "CODECS").is_some_and(|c| c.contains("mp4a.40.2"));
        let bandwidth = attribute(line, "BANDWIDTH")
            .and_then(|b| b.parse().ok())
            .unwrap_or(0);
        if let Some(uri) = lines.next()
            && best
                .as_ref()
                .is_none_or(|(rank, _)| (lc, bandwidth) > *rank)
        {
            best = Some(((lc, bandwidth), uri.trim().to_owned()));
        }
    }
    best.map(|(_, uri)| uri)
}

pub(crate) fn media_playlist(text: &str) -> MediaPlaylist {
    let mut playlist = MediaPlaylist::default();
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
            playlist.target_duration = v.parse().unwrap_or(10.0);
        } else if let Some(v) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
            playlist.media_sequence = v.parse().unwrap_or(0);
        } else if line.starts_with("#EXT-X-KEY:") {
            if attribute(line, "KEYFORMAT") == Some(WIDEVINE)
                && let Some(uri) = attribute(line, "URI")
            {
                let iv = attribute(line, "IV").and_then(hex);
                playlist.key = Some((uri.to_owned(), iv));
            }
        } else if line == "#EXT-X-ENDLIST" {
            playlist.ended = true;
        } else if line.starts_with("#EXT-X-MAP:") {
            playlist.map = attribute(line, "URI").map(str::to_owned);
        } else if !line.is_empty() && !line.starts_with('#') {
            playlist.segments.push(line.to_owned());
        }
    }
    playlist
}

/// `uri` relative to the playlist at `base`.
pub(crate) fn resolve(base: &str, uri: &str) -> String {
    if uri.starts_with("http://") || uri.starts_with("https://") {
        return uri.to_owned();
    }
    // `/path`: from the host's root.
    if uri.starts_with('/')
        && let Some(scheme_end) = base.find("://")
    {
        let host_end = base[scheme_end + 3..]
            .find('/')
            .map_or(base.len(), |i| scheme_end + 3 + i);
        return format!("{}{}", &base[..host_end], uri);
    }
    match base.rfind('/') {
        Some(i) => format!("{}/{}", &base[..i], uri),
        None => uri.to_owned(),
    }
}

/// A 7-byte ADTS header for one AAC frame of `payload` bytes, from the stream's
/// `AudioSpecificConfig`.
fn adts_header(asc: &[u8], payload: usize) -> [u8; 7] {
    let object = asc.first().map_or(2, |b| b >> 3);
    let frequency = asc
        .get(..2)
        .map_or(4, |b| ((b[0] & 0x07) << 1) | (b[1] >> 7));
    let channels = asc.get(1).map_or(2, |b| (b >> 3) & 0x0f);
    let profile = object.saturating_sub(1) & 0x03;
    let len = payload + 7;
    [
        0xff,
        0xf1,
        (profile << 6) | (frequency << 2) | (channels >> 2),
        ((channels & 0x03) << 6) | ((len >> 11) as u8 & 0x03),
        (len >> 3) as u8,
        (((len & 0x07) as u8) << 5) | 0x1f,
        0xfc,
    ]
}

/// The live stream as bytes: ADTS frames as the segments air. Dropping it
/// stops the fetching.
pub struct LiveStream {
    /// In a mutex only so the stream is `Sync`, as decoders' sources must be.
    frames: std::sync::Mutex<mpsc::Receiver<std::result::Result<Vec<u8>, String>>>,
    buffer: Vec<u8>,
    pos: usize,
    stop: Arc<AtomicBool>,
}

impl Read for LiveStream {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        while self.pos == self.buffer.len() {
            let next = match self.frames.get_mut() {
                Ok(frames) => frames.blocking_recv(),
                Err(_) => return Ok(0),
            };
            match next {
                Some(Ok(next)) => {
                    self.buffer = next;
                    self.pos = 0;
                }
                Some(Err(e)) => return Err(std::io::Error::other(e)),
                // The station stopped (an episode ended): the stream ends.
                None => return Ok(0),
            }
        }
        let n = out.len().min(self.buffer.len() - self.pos);
        out[..n].copy_from_slice(&self.buffer[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

impl Drop for LiveStream {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// What a running station needs to keep its licence: the key server and the
/// tokens, so a rotated key can be licensed again.
#[derive(Clone)]
struct Licensing {
    http: reqwest::Client,
    developer_token: String,
    user_token: String,
    key_server: String,
}

impl Licensing {
    /// Ask the radio key server for the key behind `key_uri`, as MusicKit does
    /// for a live audio station (`event: beats1`).
    async fn license(&self, cdm: &Cdm, key_uri: &str) -> Result<Session> {
        let pssh = key_uri
            .rsplit_once("base64,")
            .and_then(|(_, b64)| STANDARD.decode(b64).ok())
            .ok_or_else(|| Error::Playback("the station's key carries no PSSH".into()))?;
        let (session, challenge) = cdm
            .start_session(&pssh)
            .map_err(|e| Error::Playback(e.to_string()))?;
        let license = self.request(key_uri, &challenge).await?;
        session
            .update(&license)
            .map_err(|e| Error::Playback(e.to_string()))?;
        Ok(session)
    }

    /// The licence for `challenge`, the key server's answer to it.
    async fn request(&self, key_uri: &str, challenge: &[u8]) -> Result<Vec<u8>> {
        let body = serde_json::json!({
            "challenge": STANDARD.encode(challenge),
            "uri": key_uri,
            "key-system": "com.widevine.alpha",
            "event": "beats1",
        });
        let resp = self
            .http
            .post(&self.key_server)
            .bearer_auth(&self.developer_token)
            .header("Origin", crate::WEB_PLAYER)
            .header("Referer", format!("{}/", crate::WEB_PLAYER))
            .header("X-Apple-Music-User-Token", &self.user_token)
            .header("X-Apple-Renewal", "true")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Network(format!("radio licence: {e}")))?;
        let status = resp.status();
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::Protocol(format!("radio licence ({status}): {e}")))?;
        json["license"]
            .as_str()
            .and_then(|l| STANDARD.decode(l).ok())
            .ok_or_else(|| {
                Error::Playback(format!("the radio licence was refused ({status}): {json}"))
            })
    }
}

/// Where a station streams, for a player that reads HLS and asks for
/// Widevine licences itself (Android's).
#[derive(Debug, Clone)]
pub struct LiveAsset {
    /// The master playlist.
    pub master: String,
    /// Where licences come from, when the stream is encrypted.
    pub key_server: Option<String>,
}

/// The key id in a Widevine PSSH carried by a `data:` key URI: the
/// `WidevineCencHeader`'s field 2, 16 bytes.
pub(crate) fn pssh_key_id(key_uri: &str) -> Option<[u8; 16]> {
    let pssh = STANDARD.decode(key_uri.rsplit_once("base64,")?.1).ok()?;
    // Box header (8), version and flags (4), system id (16), data size (4).
    let data = pssh.get(32..)?;
    let at = data.windows(2).position(|w| w == [0x12, 0x10])?;
    data.get(at + 2..at + 18)?.try_into().ok()
}

/// An episode's init segment leaves its key id zero, the playlist's PSSH
/// naming the key: put it in, for a player that decrypts with the init
/// segment's key id. Says whether it did.
pub fn name_the_key(init: &mut [u8], key_uri: &str) -> bool {
    let Some(key_id) = pssh_key_id(key_uri) else {
        return false;
    };
    if cenc::tenc(init).is_none_or(|t| t.key_id != [0; 16]) {
        return false;
    }
    // The key id sits 8 bytes into the box's body, after its type.
    let Some(at) = init.windows(4).position(|w| w == b"tenc") else {
        return false;
    };
    let Some(slot) = init.get_mut(at + 12..at + 28) else {
        return false;
    };
    slot.copy_from_slice(&key_id);
    true
}

/// How a station's segments become AAC frames.
enum Segments {
    /// CMAF: an init segment, then fragments, encrypted `cbcs` (Apple's own
    /// stations) or in the clear.
    Cmaf {
        init: Vec<u8>,
        asc: Vec<u8>,
        protection: Option<(cenc::Tenc, Session)>,
    },
    /// MPEG-TS carrying ADTS, in the clear (the broadcasters' own streams).
    Ts,
}

impl Segments {
    fn frames(&self, segment: &[u8], playlist_iv: Option<&[u8]>) -> Result<Vec<u8>> {
        match self {
            Segments::Ts => ts_to_adts(segment),
            Segments::Cmaf {
                init,
                asc,
                protection,
            } => {
                let mut data = Vec::with_capacity(init.len() + segment.len());
                data.extend_from_slice(init);
                data.extend_from_slice(segment);
                let layout = cenc::index_fmp4(&data).map_err(Error::Playback)?;
                let mut out = Vec::with_capacity(segment.len() + layout.samples.len() * 7);
                for sample in &layout.samples {
                    let bytes = &data[sample.start..sample.end()];
                    let clear = match protection {
                        Some((tenc, session)) => {
                            let iv: &[u8] = if sample.iv != [0; 16] {
                                &sample.iv
                            } else {
                                tenc.constant_iv
                                    .as_deref()
                                    .or(playlist_iv)
                                    .unwrap_or(&sample.iv)
                            };
                            let subs: Vec<(u32, u32)> = sample
                                .subs
                                .iter()
                                .map(|&(c, e)| (u32::from(c), e))
                                .collect();
                            session
                                .decrypt_cbcs(bytes, &tenc.key_id, iv, tenc.pattern, &subs)
                                .map_err(|e| Error::Playback(e.to_string()))?
                        }
                        None => bytes.to_vec(),
                    };
                    out.extend_from_slice(&adts_header(asc, clear.len()));
                    out.extend_from_slice(&clear);
                }
                Ok(out)
            }
        }
    }
}

/// A PSI table's section (PAT, PMT): after the pointer field, as long as its
/// header says.
fn psi_section(body: &[u8]) -> Option<&[u8]> {
    let skip = 1 + usize::from(*body.first()?);
    let section = body.get(skip..)?;
    let len = (usize::from(section.get(1)? & 0x0f) << 8) | usize::from(*section.get(2)?);
    section.get(..(3 + len).min(section.len()))
}

/// The ADTS audio carried in an MPEG-TS segment: the PES payloads of its AAC
/// stream (stream type 0x0F) put back together.
fn ts_to_adts(segment: &[u8]) -> Result<Vec<u8>> {
    const PACKET: usize = 188;
    let mut pmt_pid = None;
    let mut audio_pid = None;
    let mut out = Vec::with_capacity(segment.len());
    for packet in segment.as_chunks::<PACKET>().0 {
        if packet[0] != 0x47 {
            return Err(Error::Playback(
                "the station's segment isn't MPEG-TS".into(),
            ));
        }
        let start = packet[1] & 0x40 != 0;
        let pid = (u16::from(packet[1] & 0x1f) << 8) | u16::from(packet[2]);
        let adaptation = (packet[3] >> 4) & 0x03;
        let mut payload = 4;
        if adaptation & 0x02 != 0 {
            payload += 1 + usize::from(packet[4]);
        }
        if adaptation & 0x01 == 0 || payload >= PACKET {
            continue;
        }
        let body = &packet[payload..];
        if pid == 0 && start {
            // PAT: the first programme's PMT.
            if let Some(section) = psi_section(body) {
                for entry in section
                    .get(8..section.len().saturating_sub(4))
                    .unwrap_or(&[])
                    .as_chunks::<4>()
                    .0
                {
                    if u16::from_be_bytes([entry[0], entry[1]]) != 0 {
                        pmt_pid = Some((u16::from(entry[2] & 0x1f) << 8) | u16::from(entry[3]));
                        break;
                    }
                }
            }
        } else if Some(pid) == pmt_pid && start && audio_pid.is_none() {
            if let Some(section) = psi_section(body) {
                let info = (usize::from(section.get(10).copied().unwrap_or(0) & 0x0f) << 8)
                    | usize::from(section.get(11).copied().unwrap_or(0));
                let mut pos = 12 + info;
                while pos + 5 <= section.len().saturating_sub(4) {
                    let kind = section[pos];
                    let es_pid =
                        (u16::from(section[pos + 1] & 0x1f) << 8) | u16::from(section[pos + 2]);
                    let es_info =
                        (usize::from(section[pos + 3] & 0x0f) << 8) | usize::from(section[pos + 4]);
                    if kind == 0x0f {
                        audio_pid = Some(es_pid);
                        break;
                    }
                    pos += 5 + es_info;
                }
            }
        } else if Some(pid) == audio_pid {
            if start {
                // A PES header: 9 bytes, then its optional fields.
                if body.len() >= 9 && body[..3] == [0, 0, 1] {
                    let skip = 9 + usize::from(body[8]);
                    out.extend_from_slice(body.get(skip..).unwrap_or(&[]));
                }
            } else {
                out.extend_from_slice(body);
            }
        }
    }
    if audio_pid.is_none() {
        return Err(Error::Playback(
            "the station's segment has no AAC audio".into(),
        ));
    }
    Ok(out)
}

impl AppleMusic {
    async fn text(&self, url: &str) -> Result<String> {
        // Said without the address: a station's is signed, and the message
        // is shown and screenshotted into bug reports.
        let failed = |e: reqwest::Error| Error::Network(format!("station: {}", e.without_url()));
        self.http
            .get(url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(failed)?
            .text()
            .await
            .map_err(failed)
    }

    /// Where `station` streams, and its key server.
    pub async fn live_asset(&self, station: &ItemId) -> Result<LiveAsset> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Station {
            play_params: serde_json::Map<String, serde_json::Value>,
        }
        let path = format!("/v1/catalog/{}/stations/{}", self.storefront, station.id);
        let resp: crate::types::Response<crate::types::Resource<Station>> = self.get(&path).await?;
        let params = resp
            .data
            .into_iter()
            .next()
            .and_then(|s| s.attributes)
            .map(|a| a.play_params)
            .ok_or_else(|| Error::Playback("the station has no play parameters".into()))?;
        // The play parameters go back as the query, as MusicKit sends them.
        let query = {
            let mut query = url::form_urlencoded::Serializer::new(String::new());
            for (key, value) in &params {
                let value = match value {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                query.append_pair(key, &value);
            }
            query.append_pair("keyFormat", "web");
            query.finish()
        };
        let assets: serde_json::Value = self.get(&format!("/v1/play/assets?{query}")).await?;
        let asset = &assets["results"]["assets"][0];
        let master_url = asset["url"]
            .as_str()
            .ok_or_else(|| Error::Playback(format!("the station has no stream: {assets}")))?;
        Ok(LiveAsset {
            master: master_url.to_owned(),
            // The licence asks with the user's token: only of Apple.
            key_server: asset["keyServerUrl"]
                .as_str()
                .filter(|url| {
                    reqwest::Url::parse(url).is_ok_and(|u| {
                        u.scheme() == "https"
                            && u.host_str()
                                .is_some_and(|h| h == "apple.com" || h.ends_with(".apple.com"))
                    })
                })
                .map(str::to_owned),
        })
    }

    /// The Widevine key the stream uses now: its `data:` URI, which names
    /// the key to the key server.
    pub async fn live_key_uri(&self, master: &str) -> Result<Option<String>> {
        Ok(self.live_parts(master).await?.1)
    }

    /// The stream's init segment, resolved, and its key's `data:` URI.
    pub async fn live_parts(&self, master: &str) -> Result<(Option<String>, Option<String>)> {
        let text = self.text(master).await?;
        let variant = match best_variant(&text) {
            Some(v) => resolve(master, &v),
            None => master.to_owned(),
        };
        let playlist = media_playlist(&self.text(&variant).await?);
        Ok((
            playlist.map.map(|map| resolve(&variant, &map)),
            playlist.key.map(|(uri, _)| uri),
        ))
    }

    /// A licence for `challenge`, a Widevine CDM's request for the key at
    /// `key_uri`.
    pub async fn live_license(
        &self,
        key_server: &str,
        key_uri: &str,
        challenge: &[u8],
    ) -> Result<Vec<u8>> {
        Licensing {
            http: self.http.clone(),
            developer_token: self.developer_token(),
            user_token: self.user_token.clone(),
            key_server: key_server.to_owned(),
        }
        .request(key_uri, challenge)
        .await
    }

    /// Tune in to a station that streams: one on air from now on, an episode
    /// from its start, decrypted when it is Apple's own.
    pub async fn live_station(&self, cdm: &Cdm, station: &ItemId) -> Result<LiveStream> {
        let asset = self.live_asset(station).await?;
        let master_url = asset.master.as_str();
        let key_server = asset.key_server.as_deref();

        let master = self.text(master_url).await?;
        // A master playlist names variants; a media playlist is the stream.
        let variant = match best_variant(&master) {
            Some(v) => resolve(master_url, &v),
            None => master_url.to_owned(),
        };
        let playlist = media_playlist(&self.text(&variant).await?);

        let licensing = key_server.map(|key_server| Licensing {
            http: self.http.clone(),
            developer_token: self.developer_token(),
            user_token: self.user_token.clone(),
            key_server: key_server.to_owned(),
        });
        let segments = match playlist.map.as_deref() {
            Some(map) => {
                let init = self.download(&resolve(&variant, map)).await?;
                let asc = cenc::audio_specific_config(&init).ok_or_else(|| {
                    Error::Playback("the station's stream says no AAC config".into())
                })?;
                let protection = match (&playlist.key, &licensing) {
                    (Some((uri, _)), Some(licensing)) => {
                        let session = licensing.license(cdm, uri).await?;
                        let mut tenc = cenc::tenc(&init).ok_or_else(|| {
                            Error::Playback("the station's stream says no key id".into())
                        })?;
                        // Episodes leave the key id zero in the init segment;
                        // the playlist's PSSH names it.
                        if tenc.key_id == [0; 16]
                            && let Some(key_id) = pssh_key_id(uri)
                        {
                            tenc.key_id = key_id;
                        }
                        Some((tenc, session))
                    }
                    (Some(_), None) => {
                        return Err(Error::Playback(
                            "the station's stream is encrypted with no key server".into(),
                        ));
                    }
                    (None, _) => None,
                };
                Segments::Cmaf {
                    init,
                    asc,
                    protection,
                }
            }
            None => Segments::Ts,
        };

        // Waited on without holding up a worker of the runtime while the
        // player is paused, or an episode is ahead of it.
        let (tx, frames) = mpsc::channel(8);
        let stop = Arc::new(AtomicBool::new(false));
        let http = self.http.clone();
        let running = stop.clone();
        let cdm = *cdm;
        tokio::spawn(async move {
            let mut segments = segments;
            let mut playlist = playlist;
            let mut key_uri = playlist.key.as_ref().map(|(uri, _)| uri.clone());
            // An episode plays from its start; what airs, from a little behind
            // the live edge, so playback has a buffer.
            let mut next = if playlist.ended {
                playlist.media_sequence
            } else {
                playlist
                    .media_sequence
                    .saturating_add(playlist.segments.len().saturating_sub(LEAD_SEGMENTS) as u64)
            };
            // When the station last answered: out of reach too long, the
            // player is told rather than left waiting in silence.
            let mut heard = std::time::Instant::now();
            while !running.load(Ordering::Relaxed) {
                if heard.elapsed() > OUT_OF_REACH {
                    let _ = tx
                        .send(Err(
                            "the station is out of reach: check the connection".into()
                        ))
                        .await;
                    return;
                }
                // A rotated key needs its licence.
                if let (Some((uri, _)), Some(licensing)) = (&playlist.key, &licensing)
                    && Some(uri) != key_uri.as_ref()
                    && let Segments::Cmaf {
                        protection: Some((_, session)),
                        ..
                    } = &mut segments
                {
                    match licensing.license(&cdm, uri).await {
                        Ok(s) => {
                            *session = s;
                            key_uri = Some(uri.clone());
                        }
                        Err(e) => {
                            let _ = tx.send(Err(e.to_string())).await;
                            return;
                        }
                    }
                }
                let iv = playlist.key.as_ref().and_then(|(_, iv)| iv.clone());
                for (i, segment) in playlist.segments.iter().enumerate() {
                    let seq = playlist.media_sequence + i as u64;
                    if seq < next {
                        continue;
                    }
                    let url = resolve(&variant, segment);
                    // Tried again once before it is skipped.
                    let fetched = match fetch(&http, &url).await {
                        Ok(bytes) => Ok(bytes),
                        Err(_) => {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            fetch(&http, &url).await
                        }
                    };
                    let Ok(bytes) = fetched else {
                        continue;
                    };
                    let frames = match segments.frames(&bytes, iv.as_deref()) {
                        Ok(frames) => frames,
                        Err(e) => {
                            let _ = tx.send(Err(e.to_string())).await;
                            return;
                        }
                    };
                    // The player went away: stop.
                    if tx.send(Ok(frames)).await.is_err() {
                        return;
                    }
                    // Taken, maybe after a long pause: counted from now.
                    heard = std::time::Instant::now();
                    next = seq + 1;
                }
                // An episode ends; dropping the sender ends the stream.
                if playlist.ended {
                    return;
                }
                tokio::time::sleep(Duration::from_secs_f64(
                    (playlist.target_duration / 2.0).max(1.0),
                ))
                .await;
                if let Ok(bytes) = fetch(&http, &variant).await {
                    playlist = media_playlist(&String::from_utf8_lossy(&bytes));
                    heard = std::time::Instant::now();
                }
            }
        });
        Ok(LiveStream {
            frames: std::sync::Mutex::new(frames),
            buffer: Vec::new(),
            pos: 0,
            stop,
        })
    }
}

async fn fetch(http: &reqwest::Client, url: &str) -> reqwest::Result<Vec<u8>> {
    Ok(http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASTER: &str = "#EXTM3U\n\
#EXT-X-STREAM-INF:CODECS=\"mp4a.40.2\",BANDWIDTH=281600,AUDIO=\"audio_0\"\n\
https://a.example/index_1.m3u8\n\
#EXT-X-STREAM-INF:CODECS=\"mp4a.40.5\",BANDWIDTH=70400,AUDIO=\"audio_1\"\n\
https://a.example/index_2.m3u8\n";

    #[test]
    fn the_key_id_comes_out_of_the_pssh() {
        let uri = "data:text/plain;v=v1;base64,AAAAOHBzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAABgSELLrrJ7y0k0vsCOpUfshHKxI88aJmwY=";
        let kid = pssh_key_id(uri).unwrap();
        assert_eq!(kid[..4], [0xb2, 0xeb, 0xac, 0x9e]);
    }

    #[test]
    fn an_episode_gets_its_key_id() {
        let uri = "data:text/plain;v=v1;base64,AAAAOHBzc2gAAAAA7e+LqXnWSs6jyCfc1R0h7QAAABgSELLrrJ7y0k0vsCOpUfshHKxI88aJmwY=";
        // A tenc box, version 0: is protected, IV size 0, key id zero, then a constant IV.
        let mut tenc = vec![0, 0, 0, 0, 0, 0, 1, 0];
        tenc.extend_from_slice(&[0; 16]);
        tenc.extend_from_slice(&[16; 17]);
        let mut init = ((tenc.len() + 8) as u32).to_be_bytes().to_vec();
        init.extend_from_slice(b"tenc");
        init.extend_from_slice(&tenc);
        assert!(name_the_key(&mut init, uri));
        assert_eq!(
            cenc::tenc(&init).unwrap().key_id[..4],
            [0xb2, 0xeb, 0xac, 0x9e]
        );
        // Named already: left as it is.
        assert!(!name_the_key(&mut init, uri));
    }

    #[test]
    fn the_best_aac_lc_variant_is_chosen() {
        assert_eq!(
            best_variant(MASTER).as_deref(),
            Some("https://a.example/index_1.m3u8")
        );
    }

    #[test]
    fn a_live_playlist_reads_its_key_map_and_segments() {
        let text = "#EXTM3U\n#EXT-X-TARGETDURATION:16\n#EXT-X-MEDIA-SEQUENCE:42\n\
#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"skd://other\",KEYFORMAT=\"com.apple.streamingkeydelivery\"\n\
#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"data:text/plain;base64,AAAA\",IV=0x0102,KEYFORMAT=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\"\n\
#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:16.0,\nseg_42.mp4\n#EXTINF:16.0,\nseg_43.mp4\n";
        let p = media_playlist(text);
        assert_eq!(p.target_duration, 16.0);
        assert_eq!(p.media_sequence, 42);
        assert_eq!(
            p.key,
            Some(("data:text/plain;base64,AAAA".into(), Some(vec![1, 2])))
        );
        assert_eq!(p.map.as_deref(), Some("init.mp4"));
        assert_eq!(p.segments, ["seg_42.mp4", "seg_43.mp4"]);
        assert_eq!(
            resolve("https://a/b/index.m3u8", "seg.mp4"),
            "https://a/b/seg.mp4"
        );
    }

    #[test]
    fn adts_headers_carry_profile_rate_channels_and_length() {
        // AAC-LC, 44.1 kHz (index 4), stereo.
        let h = adts_header(&[0x12, 0x10], 100);
        assert_eq!(&h[..2], &[0xff, 0xf1]);
        assert_eq!(h[2], (1 << 6) | (4 << 2));
        let len =
            (usize::from(h[3] & 0x03) << 11) | (usize::from(h[4]) << 3) | usize::from(h[5] >> 5);
        assert_eq!(len, 107);
        assert_eq!((h[2] & 1) << 2 | h[3] >> 6, 2);
    }
}
