//! What the tests share: a local HTTP server standing in for Apple's hosts, the crate's own
//! client pointed at it, made-up developer tokens, and a seeded generator of damaged inputs.
//!
//! The server listens on `127.0.0.1` only. The client resolves every Apple host the crate
//! names, and `elsewhere.example` (the host a redirect off Apple's goes to), to it, and uses
//! no proxy: nothing a test does leaves the machine.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::{AppleMusic, Hosts};

/// The hosts the test client sends to the local server.
const LOCAL_HOSTS: [&str; 6] = [
    "amp-api.music.apple.com",
    "music.apple.com",
    "play.music.apple.com",
    "play.itunes.apple.com",
    "aod-ssl.itunes.apple.com",
    "elsewhere.example",
];

/// The user token the tests' accounts have.
pub(crate) const USER_TOKEN: &str = "TEST-USER-TOKEN";

/// The script the local web player names.
pub(crate) const SCRIPT: &str = "/assets/index~t35t.js";

/// A request the local server got.
#[derive(Debug, Clone)]
pub(crate) struct Request {
    pub method: String,
    /// The path and the query, as sent.
    pub path: String,
    /// The `Host` header: a host and the server's port.
    pub host: String,
    /// Each header, its name in lower case.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    /// The first header named `name` (in lower case).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// The host the request was sent to, without the port.
    pub fn host_name(&self) -> &str {
        self.host.split(':').next().unwrap_or("")
    }

    /// The developer token it carried.
    pub fn bearer(&self) -> Option<&str> {
        self.header("authorization")?.strip_prefix("Bearer ")
    }

    /// Its body, as JSON.
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).expect("a JSON body")
    }
}

/// What the local server answers.
pub(crate) struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    /// Sent in chunks, without a `Content-Length`.
    chunked: bool,
    /// The connection closed without an answer.
    hang_up: bool,
    /// How long the answer is held before it is sent.
    hold: Duration,
}

/// An answer of `status` with `body`.
pub(crate) fn reply(status: u16, body: &str) -> Reply {
    Reply {
        status,
        headers: Vec::new(),
        body: body.as_bytes().to_vec(),
        chunked: false,
        hang_up: false,
        hold: Duration::ZERO,
    }
}

/// A 200 answer of `json`.
pub(crate) fn json_reply(json: &serde_json::Value) -> Reply {
    reply(200, &json.to_string()).header("Content-Type", "application/json")
}

impl Reply {
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    pub fn bytes(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    pub fn chunked(mut self) -> Self {
        self.chunked = true;
        self
    }

    /// No answer at all: the connection closes.
    pub fn hang_up() -> Self {
        Reply {
            hang_up: true,
            ..reply(500, "")
        }
    }

    pub fn held(mut self, hold: Duration) -> Self {
        self.hold = hold;
        self
    }
}

type Handler = dyn Fn(&Request) -> Reply + Send + Sync;

/// A local HTTP/1.1 server answering each request with its handler's reply, on a thread of
/// its own, and recording the requests in the order they came. Stopped when dropped.
pub(crate) struct TestServer {
    port: u16,
    requests: Arc<Mutex<Vec<Request>>>,
    peak: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    accepting: Option<JoinHandle<()>>,
}

impl TestServer {
    pub fn start(handler: impl Fn(&Request) -> Reply + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
        let port = listener.local_addr().expect("its address").port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let peak = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let handler: Arc<Handler> = Arc::new(handler);
        let accepting = {
            let (requests, peak, stop) = (requests.clone(), peak.clone(), stop.clone());
            let active = Arc::new(AtomicUsize::new(0));
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    let (handler, requests, peak, active) = (
                        handler.clone(),
                        requests.clone(),
                        peak.clone(),
                        active.clone(),
                    );
                    std::thread::spawn(move || {
                        serve(stream, &*handler, &requests, &peak, &active);
                    });
                }
            })
        };
        Self {
            port,
            requests,
            peak,
            stop,
            accepting: Some(accepting),
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Every request so far, in the order they came.
    pub fn requests(&self) -> Vec<Request> {
        self.requests
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// The requests so far to `path`, query left out.
    pub fn requests_to(&self, path: &str) -> Vec<Request> {
        self.requests()
            .into_iter()
            .filter(|r| r.path.split('?').next() == Some(path))
            .collect()
    }

    /// The most requests that were being answered at once: from the request read to the reply
    /// about to be written.
    pub fn peak_in_flight(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }

    /// Apple's hosts, each on this server.
    pub fn hosts(&self) -> Hosts {
        Hosts::local(self.port)
    }

    /// The web player, on this server.
    pub fn web_player(&self) -> String {
        self.hosts().web_player
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wakes the accepting thread, which then sees `stop`.
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(accepting) = self.accepting.take() {
            let _ = accepting.join();
        }
    }
}

/// Answer the one request `stream` brings, then close it.
fn serve(
    stream: TcpStream,
    handler: &Handler,
    requests: &Mutex<Vec<Request>>,
    peak: &AtomicUsize,
    active: &AtomicUsize,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let Some(request) = read_request(&stream) else {
        return;
    };
    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
    peak.fetch_max(now, Ordering::SeqCst);
    requests
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(request.clone());
    let reply = handler(&request);
    std::thread::sleep(reply.hold);
    // Answered once its reply is on its way, not once it is written: a client that has read the
    // reply sends its next request at once, and the count must not still hold this one then.
    active.fetch_sub(1, Ordering::SeqCst);
    if !reply.hang_up {
        let _ = write_reply(&stream, &reply);
    }
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

fn read_request(stream: &TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_owned();
    let path = parts.next()?.to_owned();
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':')?;
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
    }
    let length = headers
        .iter()
        .find(|(n, _)| n == "content-length")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    let host = headers
        .iter()
        .find(|(n, _)| n == "host")
        .map(|(_, v)| v.clone())
        .unwrap_or_default();
    Some(Request {
        method,
        path,
        host,
        headers,
        body,
    })
}

fn write_reply(mut stream: &TcpStream, reply: &Reply) -> std::io::Result<()> {
    let mut head = format!("HTTP/1.1 {} Test\r\nConnection: close\r\n", reply.status);
    for (name, value) in &reply.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    if reply.chunked {
        head.push_str("Transfer-Encoding: chunked\r\n\r\n");
        stream.write_all(head.as_bytes())?;
        for chunk in reply.body.chunks(1000) {
            stream.write_all(format!("{:x}\r\n", chunk.len()).as_bytes())?;
            stream.write_all(chunk)?;
            stream.write_all(b"\r\n")?;
        }
        stream.write_all(b"0\r\n\r\n")?;
    } else {
        head.push_str(&format!("Content-Length: {}\r\n\r\n", reply.body.len()));
        stream.write_all(head.as_bytes())?;
        stream.write_all(&reply.body)?;
    }
    stream.flush()
}

/// The crate's own client, its user agent and redirect policy included, sending Apple's
/// hosts to the local server on `port`.
pub(crate) fn test_client(port: u16) -> reqwest::Client {
    let local = SocketAddr::from(([127, 0, 0, 1], port));
    LOCAL_HOSTS
        .iter()
        .fold(crate::client_builder(), |builder, host| {
            builder.resolve(host, local)
        })
        .no_proxy()
        .build()
        .expect("the test client")
}

/// A made-up developer token, signed ES256 as far as its header says, expiring at `exp`
/// (seconds since 1970).
pub(crate) fn fake_token(exp: u64) -> String {
    let header = r#"{"alg":"ES256","kid":"MADEUPKID1","typ":"JWT"}"#;
    let payload = serde_json::json!({
        "iss": "MADEUPTEAM",
        "iat": exp.saturating_sub(6_000_000),
        "exp": exp,
        "root_https_origin": ["example.org"],
    });
    let signature = [0x5a_u8; 64];
    format!(
        "{}.{}.{}",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(payload.to_string()),
        URL_SAFE_NO_PAD.encode(signature)
    )
}

/// A token that expires in 70 days.
pub(crate) fn lasting_token() -> String {
    fake_token(seconds_from_now(70 * 24 * 3600))
}

/// The time `seconds` from now, in seconds since 1970.
pub(crate) fn seconds_from_now(seconds: u64) -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_secs()
        + seconds
}

/// What every account's connection asks the local server: the web player's page and script,
/// holding `token`, and the storefront. `None` for any other request.
pub(crate) fn web_player_and_storefront(request: &Request, token: &str) -> Option<Reply> {
    match (request.host_name(), request.path.as_str()) {
        ("music.apple.com", "/") => Some(reply(
            200,
            &format!("<html><head><script type=\"module\" crossorigin src=\"{SCRIPT}\"></script>"),
        )),
        ("music.apple.com", SCRIPT) => Some(reply(200, &format!("const t=\"{token}\";"))),
        ("amp-api.music.apple.com", "/v1/me/storefront") => Some(json_reply(
            &serde_json::json!({ "data": [{ "id": "zz", "type": "storefronts" }] }),
        )),
        _ => None,
    }
}

/// An account connected through `server`, which must answer as
/// [`web_player_and_storefront`] does.
pub(crate) async fn apple_music(server: &TestServer) -> AppleMusic {
    AppleMusic::connect_through(
        test_client(server.port()),
        server.hosts(),
        USER_TOKEN.into(),
    )
    .await
    .expect("the test account connects")
}

/// A server answering as [`web_player_and_storefront`] with a lasting token, and as
/// `handler` otherwise.
pub(crate) fn apple_server(
    handler: impl Fn(&Request) -> Reply + Send + Sync + 'static,
) -> TestServer {
    let token = lasting_token();
    TestServer::start(move |request| {
        web_player_and_storefront(request, &token).unwrap_or_else(|| handler(request))
    })
}

/// A seeded generator (xorshift64*): the same seed, the same inputs, on every run.
pub(crate) struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number below `n`, which must not be 0.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// Text that readers find hard: huge numbers, empty and very long strings, and the bytes
/// their formats give a meaning to.
const AWKWARD: [&str; 16] = [
    "",
    "0",
    "-1",
    "18446744073709551616",
    "99999999999999999999999999999",
    "4294967296",
    "1e400",
    ",",
    ".",
    "=",
    "\"",
    "\n",
    "#EXT-X-KEY:METHOD=NONE\n",
    "eyJ",
    "<script type=module src=",
    "%5B",
];

/// The longest a repeat makes a damaged input.
const LARGEST_DAMAGED: usize = 256 * 1024;

/// `seed`, damaged once: bytes flipped, cut short, a part repeated, removed or moved, or
/// awkward text put in.
pub(crate) fn mutate_bytes(rng: &mut Rng, seed: &[u8]) -> Vec<u8> {
    let mut out = seed.to_vec();
    let at = |rng: &mut Rng, len: usize| rng.below(len + 1);
    match rng.below(8) {
        0 if !out.is_empty() => {
            for _ in 0..=rng.below(4) {
                let i = rng.below(out.len());
                out[i] ^= 1 << rng.below(8);
            }
        }
        1 => out.truncate(at(rng, seed.len())),
        2 => {
            let start = at(rng, out.len());
            let end = start + rng.below(out.len() - start + 1);
            let part = out[start..end].to_vec();
            // Up to 64 times, within a quarter of a megabyte: long enough, and the tests stay quick.
            for _ in 0..=rng.below(64) {
                if out.len() + part.len() > LARGEST_DAMAGED {
                    break;
                }
                out.splice(end..end, part.iter().copied());
            }
        }
        3 => {
            let start = at(rng, out.len());
            let end = start + rng.below(out.len() - start + 1);
            out.drain(start..end);
        }
        4 => {
            let text = rng.pick(&AWKWARD).as_bytes().to_vec();
            let i = at(rng, out.len());
            out.splice(i..i, text);
        }
        5 => {
            let long = vec![*rng.pick(b"a9=,.-_\"/"); 1 + rng.below(30_000)];
            let i = at(rng, out.len());
            out.splice(i..i, long);
        }
        6 => {
            let i = at(rng, out.len());
            let random: Vec<u8> = (0..rng.below(16)).map(|_| rng.next_u64() as u8).collect();
            out.splice(i..i, random);
        }
        _ => {
            // Two halves swapped.
            let i = at(rng, out.len());
            out.rotate_left(i);
        }
    }
    out
}

/// `seed` damaged as text, as [`mutate_bytes`] does, invalid UTF-8 replaced.
pub(crate) fn mutate_text(rng: &mut Rng, seed: &str) -> String {
    String::from_utf8_lossy(&mutate_bytes(rng, seed.as_bytes())).into_owned()
}

/// `seed` with one of its values changed: another type, a huge number, an empty or very
/// long string, removed, or repeated.
pub(crate) fn mutate_json(rng: &mut Rng, seed: &serde_json::Value) -> serde_json::Value {
    use serde_json::{Value, json};
    let mut out = seed.clone();
    let pointer = random_pointer(rng, &out);
    let Some(node) = out.pointer_mut(&pointer) else {
        return out;
    };
    match rng.below(12) {
        0 => *node = Value::Null,
        1 => *node = json!(true),
        2 => *node = json!(u64::MAX),
        3 => *node = json!(-1),
        4 => *node = json!(1.5e300),
        5 => *node = json!(""),
        6 => *node = json!("x".repeat(1 + rng.below(30_000))),
        7 => *node = json!([]),
        8 => *node = json!({}),
        9 => *node = json!(rng.pick(&AWKWARD)),
        10 => match node {
            Value::Array(items) if !items.is_empty() => {
                let first = items[0].clone();
                items.extend(std::iter::repeat_n(first, 1 + rng.below(40)));
            }
            other => *other = Value::Array(vec![other.clone(), other.clone()]),
        },
        _ => {
            if let Value::Object(map) = node
                && let Some(key) = map.keys().nth(rng.below(map.len().max(1))).cloned()
            {
                map.remove(&key);
            } else {
                *node = Value::Null;
            }
        }
    }
    out
}

/// A JSON pointer (RFC 6901) to a value of `value`, down a random path.
fn random_pointer(rng: &mut Rng, value: &serde_json::Value) -> String {
    let mut pointer = String::new();
    let mut node = value;
    for _ in 0..rng.below(6) {
        match node {
            serde_json::Value::Object(map) if !map.is_empty() => {
                let Some((key, next)) = map.iter().nth(rng.below(map.len())) else {
                    break;
                };
                pointer.push('/');
                pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
                node = next;
            }
            serde_json::Value::Array(items) if !items.is_empty() => {
                let i = rng.below(items.len());
                pointer.push_str(&format!("/{i}"));
                node = &items[i];
            }
            _ => break,
        }
    }
    pointer
}

/// The key id the tests' encrypted tracks name.
pub(crate) const KEY_ID: [u8; 16] = *b"made-up-key-id-1";

/// A box: its 32-bit size, its type, its body (ISO/IEC 14496-12 §4.2).
fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let size = u32::try_from(8 + body.len()).expect("a small box");
    [&size.to_be_bytes()[..], kind, body].concat()
}

/// A full box: version 0, `flags`, then `body`.
fn full_box(kind: &[u8; 4], flags: u32, body: &[u8]) -> Vec<u8> {
    boxed(kind, &[&flags.to_be_bytes()[..], body].concat())
}

/// The movie box of one AAC track at 44.1 kHz, protected `cenc` (ISO/IEC 23001-7) with
/// [`KEY_ID`] and 16-byte IVs.
fn encrypted_movie() -> Vec<u8> {
    let tkhd = [&[0; 8][..], &1u32.to_be_bytes(), &[0; 68]].concat();
    let mdhd = [&[0; 8][..], &44_100u32.to_be_bytes(), &[0; 8]].concat();
    let tenc = [&[0, 0, 1, 16][..], &KEY_ID].concat();
    let sinf = boxed(
        b"sinf",
        &[
            boxed(b"frma", b"mp4a"),
            full_box(
                b"schm",
                0,
                &[&b"cenc"[..], &0x0001_0000u32.to_be_bytes()].concat(),
            ),
            boxed(b"schi", &full_box(b"tenc", 0, &tenc)),
        ]
        .concat(),
    );
    let enca = [&[0; 6][..], &1u16.to_be_bytes(), &[0; 20], &sinf].concat();
    let stsd = full_box(
        b"stsd",
        0,
        &[&1u32.to_be_bytes()[..], &boxed(b"enca", &enca)].concat(),
    );
    let mdia = boxed(
        b"mdia",
        &[
            full_box(b"mdhd", 0, &mdhd),
            boxed(b"minf", &boxed(b"stbl", &stsd)),
        ]
        .concat(),
    );
    let trak = boxed(b"trak", &[full_box(b"tkhd", 7, &tkhd), mdia].concat());
    let trex = [1u32, 1, 0, 0, 0].map(u32::to_be_bytes).concat();
    let mvex = boxed(b"mvex", &full_box(b"trex", 0, &trex));
    boxed(b"moov", &[trak, mvex].concat())
}

/// A fragment of `count` samples of `size` bytes, each sample 1024 frames long, with an IV
/// of its own, its data counted from its `moof`.
fn fragment(sequence: u32, count: u32, size: u32) -> Vec<u8> {
    // Default-base-is-moof, a default duration and a default size.
    let tfhd = full_box(
        b"tfhd",
        0x02_0018,
        &[1u32, 1024, size].map(u32::to_be_bytes).concat(),
    );
    let ivs: Vec<u8> = (0..count).flat_map(|n| [n as u8; 16]).collect();
    let senc = full_box(b"senc", 0, &[&count.to_be_bytes()[..], &ivs].concat());
    let mfhd = full_box(b"mfhd", 0, &sequence.to_be_bytes());
    // The trun is 20 bytes: its header, its count and its data offset.
    let moof_size = 8 + mfhd.len() + 8 + tfhd.len() + 20 + senc.len();
    let data_offset = u32::try_from(moof_size + 8).expect("a small moof");
    let trun = full_box(
        b"trun",
        0x1,
        &[count, data_offset].map(u32::to_be_bytes).concat(),
    );
    let moof = boxed(
        b"moof",
        &[mfhd, boxed(b"traf", &[tfhd, trun, senc].concat())].concat(),
    );
    let mdat = boxed(b"mdat", &vec![0; (count * size) as usize]);
    [moof, mdat].concat()
}

/// A track of `fragments` fragments of `per_fragment` samples of `size` bytes, as a
/// download brings it, its samples "encrypted" by XOR with 0xFF; and what a player should
/// read of it once decrypted by [`xor_decrypt`]: made to read as a plain MP4, each byte of a
/// sample its offset modulo 251.
pub(crate) fn encrypted_track(fragments: u32, per_fragment: u32, size: u32) -> (Vec<u8>, Vec<u8>) {
    let mut file = encrypted_movie();
    for sequence in 1..=fragments {
        file.extend(fragment(sequence, per_fragment, size));
    }
    let (movie, samples) = cenc::index(&file).expect("the test track indexes");
    let mut clear = file.clone();
    cenc::make_clear(&mut clear, &movie, &samples);
    for sample in samples.iter() {
        for at in sample.offset..sample.offset + sample.size {
            clear[at] = (at % 251) as u8;
            file[at] = clear[at] ^ 0xFF;
        }
    }
    (file, clear)
}

/// Decrypts the tests' tracks: XOR with 0xFF.
pub(crate) fn xor_decrypt() -> cenc::Decrypt {
    Box::new(|bytes, _| {
        bytes.iter_mut().for_each(|b| *b ^= 0xFF);
        Ok(())
    })
}
