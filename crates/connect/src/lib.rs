//! Music Connect: the user's devices on the same network find each other,
//! show what each one plays, control it, and hand the music over from one
//! to another, as Spotify's devices do. No server is in between: each
//! device plays with its own sign-ins, and tells the others only what it
//! plays.
//!
//! Devices are paired once, with a code one shows and the other's user
//! types in; only paired devices are listened to, and what they say to
//! each other is encrypted.
//!
//! What they keep in common (the history, Music's own playlists: see the
//! `synced` crate) goes both ways each time two connect, and each change as
//! it is made: a device off meanwhile catches up the next time.
//!
//! Paired devices away from each other connect through iroh: see `away`.

mod away;
mod lend;
mod pair;
mod store;
mod wire;

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub use music_core::Handoff;
use music_core::Repeat;
use music_core::model::{Station, Track};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

pub use lend::{Borrowed, Lender, Lent};
use store::Store;
use synced::{Digest, Kept, Shared};

#[cfg(feature = "ffi")]
uniffi::setup_scaffolding!();

/// Where a device listens, if it is free; the firewall lets it in.
pub const PORT: u16 = 47820;
const SERVICE: &str = "_music-connect._tcp.local.";
const VERSION: u32 = 1;

/// How many of the songs coming up are shown elsewhere.
const UPCOMING_SHOWN: usize = 100;
/// A connection says something this often, and is given up when silent
/// three times as long (`wire::IDLE`): the other device left the network.
const PING: Duration = Duration::from_secs(15);
/// How long the greetings and each step of a handshake may take.
pub(crate) const STEP: Duration = Duration::from_secs(5);
/// How long a song lent may go without a piece of it passing: a device
/// that left the network without saying so would otherwise hold the
/// connection, the reader and a thread of the pool for good.
pub(crate) const LENDING: Duration = Duration::from_secs(30);
/// How long reaching a device through iroh may take: finding its relay,
/// then it.
const AWAY: Duration = Duration::from_secs(20);
/// How long a code waits to be typed in.
const PAIRING: Duration = Duration::from_secs(120);
/// A paired device in sight is reached again this often, and less as it
/// keeps not answering.
const REDIAL: Duration = Duration::from_secs(5);
/// How long pairing is refused after one that came to nothing, doubling
/// with each in a row up to the most.
const PAIRING_PAUSE: Duration = Duration::from_secs(30);
const PAIRING_PAUSE_MAX: Duration = Duration::from_secs(3600);
/// The most devices kept in sight: a network announcing more is ignored.
const SEEN_MAX: usize = 64;
/// A position this far from where it should be by now is told again.
const DRIFT_MS: u64 = 1500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum DeviceKind {
    Computer,
    Phone,
}

impl DeviceKind {
    fn as_str(self) -> &'static str {
        match self {
            DeviceKind::Computer => "computer",
            DeviceKind::Phone => "phone",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Device {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
}

/// What a device plays, as it tells the others.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Playing {
    pub track: Option<Track>,
    pub playing: bool,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub shuffle: bool,
    pub repeat: Repeat,
    /// From 0 to 1; `None` where it isn't to be set from another device.
    pub volume: Option<f32>,
    /// A station on air: no end to it, nowhere to seek.
    pub live: bool,
}

impl Playing {
    /// Where it is by `now`, having been at `position_ms` `at`.
    fn at(&self, at: Instant, now: Instant) -> u64 {
        let mut ms = self.position_ms;
        if self.playing {
            ms += now.saturating_duration_since(at).as_millis() as u64;
        }
        self.duration_ms.map_or(ms, |d| ms.min(d))
    }
}

/// What one device asks of another's player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Command {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    Seek {
        position_ms: u64,
    },
    SetVolume {
        volume: f32,
    },
    SetShuffle {
        on: bool,
    },
    SetRepeat {
        repeat: Repeat,
    },
    /// Play the `k`-th song coming up.
    Jump {
        k: u32,
    },
    RemoveUpcoming {
        k: u32,
    },
    /// Move the `from`-th song coming up to be the `to`-th.
    MoveUpcoming {
        from: u32,
        to: u32,
    },
    ClearUpcoming,
    PlayList {
        tracks: Vec<Track>,
        start: u32,
    },
    PlayNext {
        tracks: Vec<Track>,
    },
    AddToQueue {
        tracks: Vec<Track>,
    },
    PlayStation {
        station: Station,
    },
    /// Stop, and hand what played over to the device asking.
    HandOver,
}

/// Another device, as this one knows it.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Peer {
    pub device: Device,
    pub paired: bool,
    /// Talking with this one now.
    pub connected: bool,
    /// On the network now, as far as is known.
    pub nearby: bool,
    /// What it plays, the position as of now; once connected.
    pub playing: Option<Playing>,
    pub upcoming: Vec<Track>,
}

/// A theme file the desktop shares with the paired phones: its id (the
/// file's stem) and its text, which the phone reads itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct SharedTheme {
    pub id: String,
    pub source: String,
}

/// What the app hears.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Event {
    /// The devices, or what they play, changed: [`Node::peers`] again.
    Changed,
    /// A device asks this one's player for something.
    Asked { from: String, command: Command },
    /// A device handed its music over: play it from here.
    Handed { from: String, handoff: Handoff },
    /// `device` asks to pair: show `code` until [`Event::Paired`].
    ShowCode { device: Device, code: String },
    /// Pairing with `device` began: ask for the code it shows, then
    /// [`Node::enter_code`].
    AskCode { device: Device },
    /// Pairing with `device` ended, with `error` if it didn't go through.
    Paired {
        device: Device,
        error: Option<String>,
    },
    /// A device shares its themes: all of them, those gone are gone there.
    Themes {
        from: String,
        themes: Vec<SharedTheme>,
    },
    /// What the devices keep in common changed, from another device: read
    /// it again.
    Synced,
}

#[derive(Serialize, Deserialize)]
struct Hello {
    version: u32,
    device: Device,
    pair: bool,
    /// To borrow a song (see `lend`), not to talk.
    #[serde(default)]
    borrow: bool,
    /// It sends what it keeps in common in short first (a [`Message::Digest`]),
    /// for the other to send only what differs.
    #[serde(default)]
    digests: bool,
}

#[derive(Serialize, Deserialize)]
enum Message {
    Playing(Playing),
    Upcoming(Vec<Track>),
    Command(Command),
    Take(Handoff),
    Themes(Vec<SharedTheme>),
    /// All this device keeps in common, as it connects, or a change to it;
    /// or, once a digest came, what differs of it.
    Shared(Shared),
    /// What this device keeps in common in short, as it connects.
    Digest(Digest),
    /// Where this device is reached away from home: its id on iroh.
    Away(String),
    /// Another device paired with this one, and the key to know it by:
    /// paired with one device, a new one is with all of them.
    Introduce {
        device: Device,
        key: String,
        away: Option<String>,
    },
    Ping,
}

/// A device seen on the network, or connected.
struct Seen {
    device: Device,
    addrs: Vec<SocketAddr>,
    /// Its name on the network, which says when it leaves.
    fullname: Option<String>,
    /// It answers on the network now. When it stops (a phone in Doze, its
    /// Wi-Fi gone a moment), its last addresses are kept to dial again.
    announced: bool,
    link: Option<Link>,
    /// A task is reaching it, or talking with it.
    dialing: bool,
    failures: u32,
    retry_at: Instant,
    playing: Option<(Playing, Instant)>,
    upcoming: Vec<Track>,
}

impl Seen {
    fn new(device: Device) -> Self {
        Self {
            device,
            addrs: Vec::new(),
            fullname: None,
            announced: false,
            link: None,
            dialing: false,
            failures: 0,
            retry_at: Instant::now(),
            playing: None,
            upcoming: Vec::new(),
        }
    }
}

struct Link {
    serial: u64,
    /// The device that made the connection.
    dialer: String,
    tx: mpsc::UnboundedSender<Arc<[u8]>>,
    /// When the device last said something on it: pinged every
    /// [`PING`], a link silent longer is likely dead (the other end
    /// suspended, or gone from the Wi-Fi) though not timed out yet.
    heard: Instant,
}

impl Link {
    fn alive(&self) -> bool {
        self.heard.elapsed() < PING + Duration::from_secs(5)
    }
}

/// What this device last told the others.
#[derive(Default)]
struct Local {
    playing: Option<(Playing, Instant)>,
    upcoming: Vec<Track>,
    themes: Vec<SharedTheme>,
}

struct Inner {
    me: Device,
    store: Mutex<Store>,
    seen: Mutex<HashMap<String, Seen>>,
    local: Mutex<Local>,
    /// Waiting for the code the user types in.
    asking: Mutex<Option<oneshot::Sender<String>>>,
    /// Showing a code: dropped, it calls pairing off.
    showing: Mutex<Option<oneshot::Sender<()>>>,
    /// Pairings that came to nothing in a row, and until when no other is
    /// taken: each doubles the wait.
    pairing_pause: Mutex<(u32, Option<Instant>)>,
    kept: Kept,
    /// This device on iroh, to reach the paired ones away from home, while
    /// it is wanted there: see [`Node::set_away`].
    away: Mutex<Option<iroh::Endpoint>>,
    /// Its key there, and so its id; none off the network (tests).
    secret: Option<[u8; 32]>,
    relay: Option<String>,
    want_away: std::sync::atomic::AtomicBool,
    /// One change at a time to whether it is on iroh.
    switching: tokio::sync::Mutex<()>,
    events: Box<dyn Fn(Event) + Send + Sync>,
    /// What lends the paired devices songs, if this one does.
    lender: Mutex<Option<Arc<dyn Lender>>>,
    serial: AtomicU64,
    rt: tokio::runtime::Handle,
}

/// A connection to another device: on the network, or through iroh.
trait Stream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static> Stream for T {}
type Conn = Box<dyn Stream>;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn failed(what: &str) -> io::Error {
    io::Error::other(what.to_owned())
}

pub(crate) async fn within<T>(limit: Duration, f: impl Future<Output = io::Result<T>>) -> io::Result<T> {
    timeout(limit, f)
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "no answer"))?
}

/// This device on the network: found by the others, reaching the paired
/// ones, telling them what plays here and hearing what they play.
pub struct Node {
    inner: Arc<Inner>,
    port: u16,
    rt: Option<tokio::runtime::Runtime>,
    mdns: Option<mdns_sd::ServiceDaemon>,
}

impl Node {
    /// Go on the network as `name`, keeping the pairings in `dir`, and
    /// sharing what `kept` keeps; away from home through `relay`, n0's when
    /// `None`. `events` is called from the node's own threads.
    pub fn start(
        name: String,
        kind: DeviceKind,
        dir: &Path,
        kept: Kept,
        relay: Option<String>,
        events: impl Fn(Event) + Send + Sync + 'static,
    ) -> Self {
        Self::launch(name, kind, dir, kept, relay, Box::new(events), true)
    }

    fn launch(
        name: String,
        kind: DeviceKind,
        dir: &Path,
        kept: Kept,
        relay: Option<String>,
        events: Box<dyn Fn(Event) + Send + Sync>,
        on_network: bool,
    ) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("connect")
            .enable_all()
            .build()
            .expect("a runtime for Music Connect");
        let store = Store::open(dir);
        let secret = on_network.then(|| store.secret()).flatten();
        let me = Device {
            id: store.id().to_owned(),
            name,
            kind,
        };
        let inner = Arc::new(Inner {
            me,
            store: Mutex::new(store),
            seen: Mutex::default(),
            local: Mutex::default(),
            asking: Mutex::default(),
            showing: Mutex::default(),
            pairing_pause: Mutex::default(),
            kept,
            away: Mutex::default(),
            secret,
            relay,
            want_away: Default::default(),
            switching: Default::default(),
            events,
            lender: Mutex::default(),
            serial: AtomicU64::new(0),
            rt: rt.handle().clone(),
        });
        let listener = rt.block_on(async {
            match TcpListener::bind(("0.0.0.0", PORT)).await {
                Ok(l) => Ok(l),
                Err(_) => TcpListener::bind(("0.0.0.0", 0)).await,
            }
        });
        let port = match listener {
            Ok(listener) => {
                let port = listener.local_addr().map_or(PORT, |a| a.port());
                rt.spawn(accept(inner.clone(), listener));
                port
            }
            Err(e) => {
                eprintln!("music connect: can't listen: {e}");
                PORT
            }
        };
        rt.spawn(redial(inner.clone()));
        let mdns = on_network
            .then(|| {
                discover(&inner, port)
                    .map_err(|e| eprintln!("music connect: can't look for devices: {e}"))
                    .ok()
            })
            .flatten();
        Self {
            inner,
            port,
            rt: Some(rt),
            mdns,
        }
    }

    /// Where it listens: the port a firewall must let in.
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn me(&self) -> Device {
        self.inner.me.clone()
    }

    /// The devices seen or paired, by name.
    pub fn peers(&self) -> Vec<Peer> {
        let paired: Vec<Device> = lock(&self.inner.store).paired().cloned().collect();
        let seen = lock(&self.inner.seen);
        let now = Instant::now();
        let mut peers: Vec<Peer> = seen
            .values()
            .map(|s| Peer {
                device: s.device.clone(),
                paired: paired.iter().any(|d| d.id == s.device.id),
                // A link gone quiet is a device gone, before it is dropped.
                connected: s.link.as_ref().is_some_and(Link::alive),
                nearby: s.link.is_some() || (s.announced && !s.addrs.is_empty()),
                playing: s.playing.as_ref().map(|(p, at)| Playing {
                    position_ms: p.at(*at, now),
                    ..p.clone()
                }),
                upcoming: s.upcoming.clone(),
            })
            .collect();
        for device in paired {
            if !seen.contains_key(&device.id) {
                peers.push(Peer {
                    device,
                    paired: true,
                    connected: false,
                    nearby: false,
                    playing: None,
                    upcoming: Vec::new(),
                });
            }
        }
        peers.sort_by(|a, b| a.device.name.cmp(&b.device.name));
        peers
    }

    /// Tell the others what plays here. Called as often as wanted: only a
    /// change goes out, and the position only when it jumped.
    pub fn publish(&self, playing: Playing) {
        let now = Instant::now();
        {
            let mut local = lock(&self.inner.local);
            if let Some((sent, at)) = &local.playing {
                let same = Playing {
                    position_ms: sent.position_ms,
                    ..playing.clone()
                } == *sent;
                if same && sent.at(*at, now).abs_diff(playing.position_ms) <= DRIFT_MS {
                    return;
                }
            }
            local.playing = Some((playing.clone(), now));
        }
        self.inner.broadcast(&Message::Playing(playing), None);
    }

    /// Tell the others what comes up here, once it changed.
    pub fn publish_upcoming(&self, mut upcoming: Vec<Track>) {
        upcoming.truncate(UPCOMING_SHOWN);
        {
            let mut local = lock(&self.inner.local);
            if local.upcoming == upcoming {
                return;
            }
            local.upcoming = upcoming.clone();
        }
        self.inner.broadcast(&Message::Upcoming(upcoming), None);
    }

    /// Share these themes with the devices, now and as they connect, once
    /// they changed.
    pub fn share_themes(&self, themes: Vec<SharedTheme>) {
        {
            let mut local = lock(&self.inner.local);
            if local.themes == themes {
                return;
            }
            local.themes = themes.clone();
        }
        self.inner.broadcast(&Message::Themes(themes), None);
    }

    /// Tell the devices connected of a change made here to what they keep
    /// in common; those not connected hear of it as they connect.
    pub fn share(&self, change: Shared) {
        self.sharer().share(change);
    }

    /// What tells the devices of changes, for another part of the app.
    pub fn sharer(&self) -> Sharer {
        Sharer(self.inner.clone())
    }

    /// Ask `to`'s player for something; false if it isn't connected.
    pub fn send(&self, to: &str, command: Command) -> bool {
        self.inner.send(to, &Message::Command(command))
    }

    /// Hand the music over to `to`, to play from where it is here.
    pub fn hand_off(&self, to: &str, handoff: Handoff) -> bool {
        self.inner.send(to, &Message::Take(handoff))
    }

    /// Pair with `id`: it shows a code, and [`Event::AskCode`] asks for it.
    pub fn pair(&self, id: &str) {
        let inner = self.inner.clone();
        let id = id.to_owned();
        self.inner.rt.spawn(async move {
            let device = lock(&inner.seen).get(&id).map(|s| s.device.clone());
            let Some(device) = device else {
                return;
            };
            let error = match inner.clone().ask(&device).await {
                Ok(()) => None,
                Err(e) => Some(e.to_string()),
            };
            (inner.events)(Event::Paired { device, error });
            inner.reach(&id);
        });
    }

    /// The code the other device shows, typed in.
    pub fn enter_code(&self, code: String) {
        if let Some(tx) = lock(&self.inner.asking).take() {
            let _ = tx.send(code);
        }
    }

    /// Call off pairing, here and on the other device.
    pub fn cancel_pairing(&self) {
        lock(&self.inner.asking).take();
        lock(&self.inner.showing).take();
    }

    /// Be reachable away from home, and reach the paired devices away, or
    /// not: a phone is while the app shows, to spare its battery the
    /// relay's keeping in touch. Going there, it reaches them at once.
    pub fn set_away(&self, on: bool) {
        self.inner.want_away.store(on, Ordering::Relaxed);
        self.inner.rt.spawn(self.inner.clone().switch_away());
    }

    /// The paired computers connected now, which may lend songs.
    pub fn lenders(&self) -> Vec<String> {
        lock(&self.inner.seen)
            .iter()
            .filter(|(_, s)| s.device.kind == DeviceKind::Computer && s.link.as_ref().is_some_and(Link::alive))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Borrow `track` from `from`, a paired device lending songs: what it
    /// is, and its bytes as they come.
    pub async fn borrow(&self, from: &str, track: Track, original: bool) -> io::Result<(Lent, Borrowed)> {
        self.inner
            .rt
            .spawn(self.inner.clone().borrow(from.to_owned(), track, original))
            .await
            .map_err(io::Error::other)?
    }

    /// Unpair `id`: it is no longer listened to.
    pub fn forget(&self, id: &str) {
        lock(&self.inner.store).forget(id);
        if let Some(seen) = lock(&self.inner.seen).get_mut(id) {
            seen.link = None;
            seen.playing = None;
            seen.upcoming.clear();
        }
        (self.inner.events)(Event::Changed);
    }
}

/// Tells the devices connected of changes to what they keep in common, as
/// [`Node::share`] does; it doesn't keep the node going.
#[derive(Clone)]
pub struct Sharer(Arc<Inner>);

impl Sharer {
    /// Lend the paired devices songs through `lender`, as they ask.
    pub fn lend(&self, lender: Arc<dyn Lender>) {
        *lock(&self.0.lender) = Some(lender);
    }

    pub fn share(&self, change: Shared) {
        if !change.is_empty() {
            self.0.broadcast(&Message::Shared(change), None);
        }
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        if let Some(mdns) = self.mdns.take() {
            let _ = mdns.shutdown();
        }
        if let Some(rt) = self.rt.take() {
            // Gone from iroh at once, rather than once the others give up.
            if let Some(away) = lock(&self.inner.away).take() {
                rt.block_on(async { timeout(Duration::from_secs(1), away.close()).await.ok() });
            }
            rt.shutdown_background();
        }
    }
}

impl Inner {
    /// On iroh or off it, as last wanted.
    async fn switch_away(self: Arc<Self>) {
        let _one = self.switching.lock().await;
        let want = self.want_away.load(Ordering::Relaxed);
        if want == lock(&self.away).is_some() {
            return;
        }
        if !want {
            let endpoint = lock(&self.away).take();
            if let Some(endpoint) = endpoint {
                endpoint.close().await;
            }
            return;
        }
        let Some(secret) = self.secret else {
            return;
        };
        let Some(endpoint) = away::endpoint(secret, self.relay.as_deref()).await else {
            return;
        };
        *lock(&self.away) = Some(endpoint.clone());
        tokio::spawn(accept_away(self.clone(), endpoint));
        // Not left to the next try, nor to a wait grown with failures.
        self.in_sight_away();
        let ids: Vec<String> = {
            let store = lock(&self.store);
            let mut seen = lock(&self.seen);
            seen.iter_mut()
                .filter(|(id, s)| s.link.is_none() && store.away(id).is_some())
                .map(|(id, s)| {
                    s.failures = 0;
                    s.retry_at = Instant::now();
                    id.clone()
                })
                .collect()
        };
        for id in ids {
            self.reach(&id);
        }
    }

    /// The paired devices reachable away from home, among those in sight.
    fn in_sight_away(&self) {
        let away: Vec<Device> = {
            let store = lock(&self.store);
            store
                .paired()
                .filter(|d| store.away(&d.id).is_some())
                .cloned()
                .collect()
        };
        let mut seen = lock(&self.seen);
        for device in away {
            seen.entry(device.id.clone())
                .or_insert_with(|| Seen::new(device));
        }
    }

    /// This device's id on iroh, whether it is there now or not.
    fn away_id(&self) -> Option<String> {
        self.secret
            .map(|secret| iroh::SecretKey::from_bytes(&secret).public().to_string())
    }

    fn hello(&self, pair: bool) -> Hello {
        Hello {
            version: VERSION,
            device: self.me.clone(),
            pair,
            borrow: false,
            digests: true,
        }
    }

    /// Borrow `track` from `id`, over a connection of its own.
    async fn borrow(self: Arc<Self>, id: String, track: Track, original: bool) -> io::Result<(Lent, Borrowed)> {
        let key = lock(&self.store)
            .key(&id)
            .ok_or_else(|| failed("not paired"))?;
        let mut stream = self.connect(&id).await?;
        let hello = Hello {
            borrow: true,
            ..self.hello(false)
        };
        wire::write_json(&mut stream, &hello).await?;
        let hello: Hello = within(STEP, wire::read_json(&mut stream)).await?;
        if hello.device.id != id {
            return Err(failed("another device answered"));
        }
        let prologue = prologue(&self.me.id, &id);
        let keys = within(STEP, wire::handshake(&mut stream, true, &key, &prologue)).await?;
        lend::borrow(stream, keys, track, original).await
    }

    fn encode(message: &Message) -> Arc<[u8]> {
        serde_json::to_vec(message).unwrap_or_default().into()
    }

    /// Send `message` to the devices connected, but `not_to`.
    fn broadcast(&self, message: &Message, not_to: Option<&str>) {
        let bytes = Self::encode(message);
        for (id, seen) in lock(&self.seen).iter() {
            if Some(id.as_str()) == not_to {
                continue;
            }
            if let Some(link) = &seen.link {
                let _ = link.tx.send(bytes.clone());
            }
        }
    }

    fn send(&self, to: &str, message: &Message) -> bool {
        let seen = lock(&self.seen);
        // A link gone silent may be dead: what is sent may be lost, and the
        // caller is told so (a hand-over then doesn't stop the music here).
        let Some(link) = seen
            .get(to)
            .and_then(|s| s.link.as_ref())
            .filter(|l| l.alive())
        else {
            return false;
        };
        link.tx.send(Self::encode(message)).is_ok()
    }

    /// Connect to `id` if it is paired, in sight or reachable away from
    /// home, and not connected yet.
    fn reach(self: &Arc<Self>, id: &str) {
        let away = {
            let store = lock(&self.store);
            if store.key(id).is_none() {
                return;
            }
            lock(&self.away).is_some() && store.away(id).is_some()
        };
        {
            let mut seen = lock(&self.seen);
            let Some(s) = seen.get_mut(id) else {
                return;
            };
            if s.link.is_some()
                || s.dialing
                || (s.addrs.is_empty() && !away)
                || s.retry_at > Instant::now()
            {
                return;
            }
            s.dialing = true;
        }
        let inner = self.clone();
        let id = id.to_owned();
        self.rt.spawn(async move {
            let result = inner.clone().dial(&id).await;
            let mut seen = lock(&inner.seen);
            if let Some(s) = seen.get_mut(&id) {
                s.dialing = false;
                if result.is_err() && s.link.is_none() {
                    s.failures += 1;
                    s.retry_at = Instant::now() + REDIAL * 2u32.pow((s.failures - 1).min(6));
                }
            }
        });
    }

    /// Reach `id` on the network, else away from home.
    async fn connect(&self, id: &str) -> io::Result<Conn> {
        if let Ok(stream) = self.reach_any(id).await {
            return Ok(Box::new(stream));
        }
        let to = lock(&self.store).away(id).map(str::to_owned);
        let endpoint = lock(&self.away).clone();
        match (endpoint, to) {
            (Some(away), Some(to)) => Ok(Box::new(within(AWAY, away::dial(&away, &to)).await?)),
            _ => Err(failed("can't reach it")),
        }
    }

    async fn reach_any(&self, id: &str) -> io::Result<TcpStream> {
        let addrs = lock(&self.seen)
            .get(id)
            .map(|s| s.addrs.clone())
            .unwrap_or_default();
        for addr in addrs {
            if let Ok(Ok(stream)) = timeout(STEP, TcpStream::connect(addr)).await {
                let _ = stream.set_nodelay(true);
                return Ok(stream);
            }
        }
        Err(failed("can't reach it"))
    }

    /// Connect to a paired device, and talk with it until either leaves.
    async fn dial(self: Arc<Self>, id: &str) -> io::Result<()> {
        let key = lock(&self.store)
            .key(id)
            .ok_or_else(|| failed("not paired"))?;
        let mut stream = self.connect(id).await?;
        wire::write_json(&mut stream, &self.hello(false)).await?;
        let hello: Hello = within(STEP, wire::read_json(&mut stream)).await?;
        if hello.device.id != id {
            return Err(failed("another device answered"));
        }
        let prologue = prologue(&self.me.id, id);
        let keys = within(STEP, wire::handshake(&mut stream, true, &key, &prologue)).await?;
        lock(&self.store).update(&hello.device);
        // Reached: once this connection ends, the next try comes soon.
        if let Some(s) = lock(&self.seen).get_mut(id) {
            s.failures = 0;
        }
        self.talk(hello.device, hello.digests, stream, keys, true).await
    }

    /// A device connected to this one: to talk, or to pair (on the network
    /// only, not `away` from home).
    async fn answer(self: Arc<Self>, mut stream: Conn, away: bool) -> io::Result<()> {
        let hello: Hello = within(STEP, wire::read_json(&mut stream)).await?;
        if hello.version != VERSION || hello.device.id == self.me.id {
            return Err(failed("not one of ours"));
        }
        if hello.pair && away {
            return Err(failed("pairing is on the same network only"));
        }
        if hello.pair {
            return self.show_code(stream, hello.device).await;
        }
        let key = lock(&self.store)
            .key(&hello.device.id)
            .ok_or_else(|| failed("not paired"))?;
        wire::write_json(&mut stream, &self.hello(false)).await?;
        let prologue = prologue(&hello.device.id, &self.me.id);
        let keys = within(STEP, wire::handshake(&mut stream, false, &key, &prologue)).await?;
        if hello.borrow {
            let lender = lock(&self.lender).clone();
            return lend::lend(stream, keys, lender.as_deref()).await;
        }
        lock(&self.store).update(&hello.device);
        self.talk(hello.device, hello.digests, stream, keys, false).await
    }

    /// Talk with `peer` over a connection both have proved theirs.
    async fn talk(
        self: Arc<Self>,
        peer: Device,
        digests: bool,
        stream: Conn,
        keys: wire::Keys,
        dialed: bool,
    ) -> io::Result<()> {
        let dialer = if dialed { &self.me.id } else { &peer.id }.clone();
        let serial = self.serial.fetch_add(1, Ordering::Relaxed);
        let (tx, mut rx) = mpsc::unbounded_channel::<Arc<[u8]>>();
        {
            let mut seen = lock(&self.seen);
            let s = seen
                .entry(peer.id.clone())
                .or_insert_with(|| Seen::new(peer.clone()));
            s.device = peer.clone();
            // Both dialed at once: each keeps the connection the device
            // with the smaller id made, so they keep the same one.
            // A silent one is likely dead, though: the new one takes over.
            if let Some(old) = &s.link
                && old.dialer != dialer
                && old.dialer == *self.me.id.as_str().min(peer.id.as_str())
                && old.alive()
            {
                return Ok(());
            }
            s.link = Some(Link {
                serial,
                dialer,
                tx: tx.clone(),
                heard: Instant::now(),
            });
            s.failures = 0;
        }
        (self.events)(Event::Changed);
        // What plays here, for it to show at once.
        {
            let local = lock(&self.local);
            if let Some((playing, at)) = &local.playing {
                let now = Playing {
                    position_ms: playing.at(*at, Instant::now()),
                    ..playing.clone()
                };
                let _ = tx.send(Self::encode(&Message::Playing(now)));
            }
            let _ = tx.send(Self::encode(&Message::Upcoming(local.upcoming.clone())));
            if !local.themes.is_empty() {
                let _ = tx.send(Self::encode(&Message::Themes(local.themes.clone())));
            }
        }
        // In short, to be sent what differs, if it knows how; else all of it.
        let _ = tx.send(Self::encode(&if digests {
            Message::Digest(self.kept.digest())
        } else {
            Message::Shared(self.kept.everything())
        }));
        if let Some(away) = self.away_id() {
            let _ = tx.send(Self::encode(&Message::Away(away)));
        }
        self.introduce(&peer, &tx);
        drop(tx);

        let (mut read, mut write) = tokio::io::split(stream);
        let sending = keys.clone();
        let writer = self.rt.spawn(async move {
            let mut nonce = 0;
            let mut ping = tokio::time::interval(PING);
            let ping_bytes = Self::encode(&Message::Ping);
            loop {
                let bytes = tokio::select! {
                    m = rx.recv() => match m {
                        Some(m) => m,
                        // Forgotten, or replaced by another connection.
                        None => break,
                    },
                    _ = ping.tick() => ping_bytes.clone(),
                };
                if wire::send(&mut write, &sending, &mut nonce, &bytes)
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        let mut nonce = 0;
        let ended = loop {
            // The wait is per piece, inside `receive`: a whole message of
            // several megabytes mustn't be cut off for taking longer than
            // one piece may.
            let bytes = match wire::receive(&mut read, &keys, &mut nonce).await {
                Ok(bytes) => bytes,
                Err(e) => break e,
            };
            // Something a later version says is passed over.
            if let Ok(message) = serde_json::from_slice::<Message>(&bytes) {
                self.heard(&peer.id, message);
            }
        };
        writer.abort();
        {
            let mut seen = lock(&self.seen);
            if let Some(s) = seen.get_mut(&peer.id)
                && s.link.as_ref().is_some_and(|l| l.serial == serial)
            {
                s.link = None;
                s.playing = None;
                s.upcoming.clear();
            }
        }
        (self.events)(Event::Changed);
        Err(ended)
    }

    /// `peer`, just connected, and the other paired devices introduced to
    /// each other: to it through `tx`, it to those connected now.
    fn introduce(&self, peer: &Device, tx: &mpsc::UnboundedSender<Arc<[u8]>>) {
        let mut store = lock(&self.store);
        let others: Vec<Device> = store
            .paired()
            .filter(|d| d.id != peer.id)
            .cloned()
            .collect();
        let peer_away = store.away(&peer.id).map(str::to_owned);
        for other in others {
            let key = store.introduction(&peer.id, &other.id);
            let away = store.away(&other.id).map(str::to_owned);
            let _ = tx.send(Self::encode(&Message::Introduce {
                device: other.clone(),
                key: key.clone(),
                away,
            }));
            self.send(
                &other.id,
                &Message::Introduce {
                    device: peer.clone(),
                    key,
                    away: peer_away.clone(),
                },
            );
        }
    }

    fn heard(self: &Arc<Self>, from: &str, message: Message) {
        // Unpaired while it was connected, it is no longer listened to:
        // the connection stands until it falls silent, but what comes over
        // it after is dropped rather than left to work the player.
        if lock(&self.store).key(from).is_none() {
            return;
        }
        if let Some(link) = lock(&self.seen).get_mut(from).and_then(|s| s.link.as_mut()) {
            link.heard = Instant::now();
        }
        match message {
            Message::Playing(playing) => {
                if let Some(s) = lock(&self.seen).get_mut(from) {
                    s.playing = Some((playing, Instant::now()));
                }
                (self.events)(Event::Changed);
            }
            Message::Upcoming(upcoming) => {
                if let Some(s) = lock(&self.seen).get_mut(from) {
                    s.upcoming = upcoming;
                }
                (self.events)(Event::Changed);
            }
            Message::Command(command) => (self.events)(Event::Asked {
                from: from.to_owned(),
                command,
            }),
            Message::Take(handoff) => (self.events)(Event::Handed {
                from: from.to_owned(),
                handoff,
            }),
            Message::Themes(themes) => (self.events)(Event::Themes {
                from: from.to_owned(),
                themes,
            }),
            Message::Shared(shared) => {
                // New here: the other devices connected may not have it
                // either.
                if self.kept.merge(shared.clone()) {
                    (self.events)(Event::Synced);
                    self.broadcast(&Message::Shared(shared), Some(from));
                }
            }
            Message::Digest(theirs) => {
                let unlike = self.kept.unlike(&theirs);
                if !unlike.is_empty() {
                    self.send(from, &Message::Shared(unlike));
                }
            }
            Message::Away(away) => lock(&self.store).set_away(from, away),
            Message::Introduce { device, key, away } => {
                if device.id == self.me.id
                    || !lock(&self.store).introduced(device.clone(), key, away, from)
                {
                    return;
                }
                // Known now: reached on the network once seen there, or at
                // once away from home.
                let id = device.id.clone();
                lock(&self.seen)
                    .entry(id.clone())
                    .or_insert_with(|| Seen::new(device));
                (self.events)(Event::Changed);
                self.reach(&id);
            }
            Message::Ping => {}
        }
    }

    /// Pair with `device`, which shows a code for the user to type in here.
    async fn ask(self: Arc<Self>, device: &Device) -> io::Result<()> {
        let (tx, rx) = oneshot::channel();
        // A new pairing calls off the one before.
        *lock(&self.asking) = Some(tx);
        let mut stream = self.reach_any(&device.id).await?;
        wire::write_json(&mut stream, &self.hello(true)).await?;
        let hello: Hello = within(STEP, wire::read_json(&mut stream)).await?;
        if hello.device.id != device.id {
            return Err(failed("another device answered"));
        }
        let theirs = match within(STEP, wire::read_json(&mut stream)).await? {
            pair::Step::Spake(theirs) => theirs,
            _ => return Err(failed("it said no")),
        };
        (self.events)(Event::AskCode {
            device: hello.device.clone(),
        });
        // Typed in, or called off here or there.
        let mut probe = [0];
        let code = tokio::select! {
            code = timeout(PAIRING, rx) => code
                .map_err(|_| failed("no code was typed in"))?
                .map_err(|_| failed("pairing was called off"))?,
            _ = stream.peek(&mut probe) => return Err(failed("pairing was called off")),
        };
        let (spake, ours) = pair::start(code.trim(), &self.me.id, &device.id, true);
        wire::write_json(&mut stream, &pair::Step::Spake(ours)).await?;
        let key = spake
            .finish(&theirs)
            .map_err(|_| failed("pairing failed"))?;
        wire::write_json(
            &mut stream,
            &pair::Step::Confirm(pair::proof(&key, "asker")),
        )
        .await?;
        match within(STEP, wire::read_json(&mut stream)).await? {
            pair::Step::Confirm(proof) if pair::proves(&key, "shower", &proof) => {}
            _ => return Err(failed("wrong code")),
        }
        let mut store = lock(&self.store);
        store.pair(hello.device, pair::kept(&key));
        drop(store);
        if let Some(s) = lock(&self.seen).get_mut(&device.id) {
            s.retry_at = Instant::now();
            s.failures = 0;
        }
        Ok(())
    }

    /// `peer` asks to pair: show a code, and pair if it is typed in there.
    async fn show_code(self: Arc<Self>, mut stream: Conn, peer: Device) -> io::Result<()> {
        if lock(&self.pairing_pause)
            .1
            .is_some_and(|until| Instant::now() < until)
        {
            return Err(failed("a wrong code was typed: try again in a moment"));
        }
        let (tx, mut called_off) = oneshot::channel::<()>();
        {
            let mut showing = lock(&self.showing);
            if showing.is_some() {
                return Err(failed("already pairing"));
            }
            *showing = Some(tx);
        }
        let result: io::Result<()> = async {
            wire::write_json(&mut stream, &self.hello(true)).await?;
            let code = pair::code();
            let (spake, ours) = pair::start(&code, &peer.id, &self.me.id, false);
            wire::write_json(&mut stream, &pair::Step::Spake(ours)).await?;
            (self.events)(Event::ShowCode {
                device: peer.clone(),
                code,
            });
            let step = tokio::select! {
                step = within(PAIRING, wire::read_json(&mut stream)) => step?,
                _ = &mut called_off => {
                    let _ = wire::write_json(&mut stream, &pair::Step::Refused).await;
                    return Err(failed("pairing was called off"));
                }
            };
            let pair::Step::Spake(theirs) = step else {
                return Err(failed("pairing was called off"));
            };
            let key = spake
                .finish(&theirs)
                .map_err(|_| failed("pairing failed"))?;
            let proved = match within(STEP, wire::read_json(&mut stream)).await? {
                pair::Step::Confirm(proof) => pair::proves(&key, "asker", &proof),
                _ => false,
            };
            if !proved {
                let _ = wire::write_json(&mut stream, &pair::Step::Refused).await;
                return Err(failed("wrong code"));
            }
            wire::write_json(
                &mut stream,
                &pair::Step::Confirm(pair::proof(&key, "shower")),
            )
            .await?;
            lock(&self.store).pair(peer.clone(), pair::kept(&key));
            Ok(())
        }
        .await;
        lock(&self.showing).take();
        // A wrong code, or one never typed (a device on the network holding
        // the code up): each such pairing doubles the wait before the next,
        // up to an hour, so that codes can't be tried one after another.
        {
            let mut pause = lock(&self.pairing_pause);
            if result.is_ok() {
                *pause = (0, None);
            } else {
                pause.0 = (pause.0 + 1).min(8);
                let wait = (PAIRING_PAUSE * 2u32.pow(pause.0 - 1)).min(PAIRING_PAUSE_MAX);
                pause.1 = Some(Instant::now() + wait);
            }
        }
        (self.events)(Event::Paired {
            device: peer,
            error: result.as_ref().err().map(|e| e.to_string()),
        });
        result
    }

    /// What the network said about a device.
    fn found(self: &Arc<Self>, event: mdns_sd::ServiceEvent) {
        match event {
            mdns_sd::ServiceEvent::ServiceResolved(info) => {
                let Some(id) = info.get_property_val_str("id").map(str::to_owned) else {
                    return;
                };
                if id == self.me.id {
                    return;
                }
                let device = Device {
                    name: info.get_property_val_str("name").unwrap_or(&id).to_owned(),
                    kind: match info.get_property_val_str("kind") {
                        Some("phone") => DeviceKind::Phone,
                        _ => DeviceKind::Computer,
                    },
                    id: id.clone(),
                };
                let port = info.get_port();
                let mut addrs: Vec<SocketAddr> = info
                    .get_addresses_v4()
                    .into_iter()
                    .map(|ip| SocketAddr::new(ip.into(), port))
                    .collect();
                addrs.sort();
                // A paired device's name is kept only once it proves itself
                // (see `dial` and `answer`): anyone can announce anything.
                {
                    let mut seen = lock(&self.seen);
                    if !seen.contains_key(&id) && seen.len() >= SEEN_MAX {
                        return;
                    }
                    let s = seen
                        .entry(id.clone())
                        .or_insert_with(|| Seen::new(device.clone()));
                    s.device = device;
                    s.addrs = addrs;
                    s.announced = true;
                    s.fullname = Some(info.fullname.clone());
                    // Back in sight: worth trying at once.
                    s.retry_at = Instant::now();
                }
                (self.events)(Event::Changed);
                self.reach(&id);
            }
            mdns_sd::ServiceEvent::ServiceRemoved(_, fullname) => {
                let store = lock(&self.store);
                let mut seen = lock(&self.seen);
                for s in seen.values_mut() {
                    if s.fullname.as_deref() == Some(&fullname) {
                        s.announced = false;
                    }
                }
                // Strangers gone are forgotten; paired ones are dialed
                // again where they were.
                seen.retain(|id, s| s.announced || s.link.is_some() || store.key(id).is_some());
                drop((seen, store));
                (self.events)(Event::Changed);
            }
            _ => {}
        }
    }
}

/// Names both ends, the one that dialed first.
fn prologue(dialer: &str, answerer: &str) -> Vec<u8> {
    format!("music-connect {VERSION} {dialer} {answerer}").into_bytes()
}

async fn accept(inner: Arc<Inner>, listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let _ = stream.set_nodelay(true);
                tokio::spawn(inner.clone().answer(Box::new(stream), false));
            }
            Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }
}

/// The paired devices reaching this one through iroh.
async fn accept_away(inner: Arc<Inner>, endpoint: iroh::Endpoint) {
    while let Some(incoming) = endpoint.accept().await {
        let inner = inner.clone();
        tokio::spawn(async move {
            let paired = |away: &str| lock(&inner.store).away_of(away).is_some();
            let stream = within(AWAY, away::answer(incoming, paired)).await?;
            inner.answer(Box::new(stream), true).await
        });
    }
}

/// Reach again, now and then, the paired devices in sight, and those away
/// from home.
async fn redial(inner: Arc<Inner>) {
    let mut every = tokio::time::interval(REDIAL);
    loop {
        every.tick().await;
        if lock(&inner.away).is_some() {
            inner.in_sight_away();
        }
        let ids: Vec<String> = lock(&inner.seen).keys().cloned().collect();
        for id in ids {
            inner.reach(&id);
        }
    }
}

/// Be found on the network, and look for the others.
fn discover(inner: &Arc<Inner>, port: u16) -> Result<mdns_sd::ServiceDaemon, mdns_sd::Error> {
    let mdns = mdns_sd::ServiceDaemon::new()?;
    let me = &inner.me;
    let host = format!("music-{}.local.", me.id);
    let properties = [
        ("id", me.id.as_str()),
        ("name", me.name.as_str()),
        ("kind", me.kind.as_str()),
    ];
    let info = mdns_sd::ServiceInfo::new(SERVICE, &me.id, &host, "", port, &properties[..])?
        .enable_addr_auto();
    mdns.register(info)?;
    let events = mdns.browse(SERVICE)?;
    let inner = inner.clone();
    let _ = std::thread::Builder::new()
        .name("connect-mdns".into())
        .spawn(move || {
            while let Ok(event) = events.recv() {
                inner.found(event);
            }
        });
    Ok(mdns)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: &str) -> Track {
        Track {
            id: music_core::model::ItemId::new(music_core::model::ProviderKind::AppleMusic, id),
            title: id.into(),
            artists: Vec::new(),
            album: None,
            album_id: None,
            duration: None,
            track_number: None,
            disc_number: None,
            artwork: None,
            explicit: false,
            quality: None,
            in_library: false,
            library_id: None,
        }
    }

    fn playing(position_ms: u64, playing: bool) -> Playing {
        Playing {
            position_ms,
            playing,
            duration_ms: Some(10_000),
            ..Playing::default()
        }
    }

    #[test]
    fn the_position_moves_on_while_playing_only_and_stops_at_the_end() {
        let at = Instant::now();
        let later = at + Duration::from_secs(3);
        assert_eq!(playing(1000, true).at(at, later), 4000);
        assert_eq!(playing(1000, false).at(at, later), 1000);
        assert_eq!(playing(9000, true).at(at, later), 10_000);
    }

    fn pair_up(typed: impl Fn(&str) -> String) -> (Option<String>, Option<String>, bool) {
        let dir = std::env::temp_dir().join(format!(
            "connect-pair-{}-{}",
            std::process::id(),
            typed("0")
        ));
        let (atx, arx) = std::sync::mpsc::channel();
        let (btx, brx) = std::sync::mpsc::channel();
        let a = Node::launch(
            "A".into(),
            DeviceKind::Phone,
            &dir.join("a"),
            Kept::open(&dir.join("a"), Vec::new),
            None,
            Box::new(move |e| {
                let _ = atx.send(e);
            }),
            false,
        );
        let b = Node::launch(
            "B".into(),
            DeviceKind::Computer,
            &dir.join("b"),
            Kept::open(&dir.join("b"), Vec::new),
            None,
            Box::new(move |e| {
                let _ = btx.send(e);
            }),
            false,
        );
        let mut seen = Seen::new(b.me());
        seen.addrs = vec![SocketAddr::from(([127, 0, 0, 1], b.port))];
        lock(&a.inner.seen).insert(b.me().id, seen);
        a.pair(&b.me().id);
        let wait = |rx: &std::sync::mpsc::Receiver<Event>, f: &dyn Fn(&Event) -> bool| loop {
            let e = rx.recv_timeout(Duration::from_secs(5)).expect("no event");
            if f(&e) {
                break e;
            }
        };
        let Event::ShowCode { code, .. } = wait(&brx, &|e| matches!(e, Event::ShowCode { .. }))
        else {
            unreachable!()
        };
        wait(&arx, &|e| matches!(e, Event::AskCode { .. }));
        a.enter_code(typed(&code));
        let paired = |e: Event| match e {
            Event::Paired { error, .. } => error,
            _ => unreachable!(),
        };
        let a_error = paired(wait(&arx, &|e| matches!(e, Event::Paired { .. })));
        let b_error = paired(wait(&brx, &|e| matches!(e, Event::Paired { .. })));
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut connected = false;
        while a_error.is_none() && !connected && Instant::now() < deadline {
            connected = b.peers().iter().any(|p| p.connected);
            std::thread::sleep(Duration::from_millis(20));
        }
        drop((a, b));
        let _ = std::fs::remove_dir_all(dir);
        (a_error, b_error, connected)
    }

    #[test]
    fn the_code_shown_pairs_and_the_two_connect() {
        let (a, b, connected) = pair_up(|code| code.to_owned());
        assert_eq!((a, b), (None, None));
        assert!(connected);
    }

    #[test]
    fn a_wrong_code_pairs_neither() {
        let (a, b, _) = pair_up(|code| {
            if code == "000000" {
                "1".into()
            } else {
                "000000".into()
            }
        });
        assert!(a.is_some() && b.is_some());
    }

    /// Two nodes on this machine, paired by hand, see each other play and
    /// pass commands: what the network does, short of finding each other.
    #[test]
    fn paired_nodes_talk() {
        let dir = std::env::temp_dir().join(format!("connect-test-{}", std::process::id()));
        let (a_dir, b_dir) = (dir.join("a"), dir.join("b"));
        let (tx, rx) = std::sync::mpsc::channel();
        let (a_kept, b_kept) = (Kept::open(&a_dir, Vec::new), Kept::open(&b_dir, Vec::new));
        // Played on A before they connect.
        a_kept.played(track("1"));
        let (a_tx, a_rx) = std::sync::mpsc::channel();
        let a = Node::launch(
            "A".into(),
            DeviceKind::Phone,
            &a_dir,
            a_kept.clone(),
            None,
            Box::new(move |e| {
                let _ = a_tx.send(e);
            }),
            false,
        );
        let events = Box::new(move |e| {
            let _ = tx.send(e);
        });
        let b = Node::launch(
            "B".into(),
            DeviceKind::Computer,
            &b_dir,
            b_kept.clone(),
            None,
            events,
            false,
        );
        let key = [9; 32];
        lock(&a.inner.store).pair(b.me(), key);
        lock(&b.inner.store).pair(a.me(), key);
        // Where B listens, as the network would have said.
        let mut seen = Seen::new(b.me());
        seen.addrs = vec![SocketAddr::from(([127, 0, 0, 1], b.port))];
        lock(&a.inner.seen).insert(b.me().id, seen);
        a.publish(playing(2000, true));
        let theme = SharedTheme {
            id: "mine".into(),
            source: "name = \"Mine\"".into(),
        };
        a.share_themes(vec![theme.clone()]);
        a.inner.reach(&b.me().id);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !b.peers().iter().any(|p| p.playing.is_some()) {
            assert!(Instant::now() < deadline, "B never heard A");
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(a.send(&b.me().id, Command::Next));
        // The themes and the history as it connected, and the command.
        let (mut got, mut themes, mut synced) = (None, None, false);
        while got.is_none() || themes.is_none() || !synced {
            match rx
                .recv_timeout(Duration::from_secs(5))
                .expect("B heard nothing")
            {
                Event::Asked { command, .. } => got = Some(command),
                Event::Themes { themes: t, .. } => themes = Some(t),
                Event::Synced => synced = true,
                _ => continue,
            }
        }
        assert_eq!(got, Some(Command::Next));
        assert_eq!(themes, Some(vec![theme]));
        assert_eq!(b_kept.history(), a_kept.history());
        // Played on B once connected: A hears of it at once.
        b.share(b_kept.played(track("2")));
        while !matches!(
            a_rx.recv_timeout(Duration::from_secs(5))
                .expect("A heard nothing"),
            Event::Synced
        ) {}
        assert_eq!(a_kept.history().len(), 2);
        assert_eq!(a_kept.history()[0].id.id, "2");
        drop((a, b));
        let _ = std::fs::remove_dir_all(dir);
    }

    struct Lends(Vec<u8>);

    #[async_trait::async_trait]
    impl Lender for Lends {
        async fn lend(&self, track: Track, _: bool) -> Result<(Lent, Box<dyn std::io::Read + Send>), String> {
            if track.id.id == "refused" {
                return Err("not this one".into());
            }
            let lent = Lent {
                len: self.0.len() as u64,
                extension: "m4a".into(),
                priming: 2112,
                protected: true,
                kbps: Some(256),
                spatial: false,
            };
            Ok((lent, Box::new(std::io::Cursor::new(self.0.clone()))))
        }
    }

    /// A phone borrows a song from the computer it is paired with: whole,
    /// in pieces, and told why when it is refused.
    #[test]
    fn a_song_is_borrowed_whole() {
        let dir = std::env::temp_dir().join(format!("connect-lend-{}", std::process::id()));
        let node = |name: &str, kind| {
            let d = dir.join(name);
            Node::launch(name.into(), kind, &d, Kept::open(&d, Vec::new), None, Box::new(|_| {}), false)
        };
        let (phone, computer) = (node("p", DeviceKind::Phone), node("c", DeviceKind::Computer));
        let key = [5; 32];
        lock(&phone.inner.store).pair(computer.me(), key);
        lock(&computer.inner.store).pair(phone.me(), key);
        let mut seen = Seen::new(computer.me());
        seen.addrs = vec![SocketAddr::from(([127, 0, 0, 1], computer.port))];
        lock(&phone.inner.seen).insert(computer.me().id, seen);
        let song: Vec<u8> = (0..1_000_003u32).map(|i| (i * 7) as u8).collect();
        computer.sharer().lend(Arc::new(Lends(song.clone())));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (lent, mut borrowed) = rt
            .block_on(phone.borrow(&computer.me().id, track("1"), false))
            .unwrap();
        assert_eq!((lent.len, lent.priming), (song.len() as u64, 2112));
        let mut got = Vec::new();
        std::io::Read::read_to_end(&mut borrowed, &mut got).unwrap();
        assert_eq!(got, song);
        let refused = rt.block_on(phone.borrow(&computer.me().id, track("refused"), false));
        assert_eq!(refused.err().unwrap().to_string(), "not this one");
        drop((phone, computer));
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A and C, each paired with B alone, are introduced by it: they pair
    /// with each other and connect, without a code.
    #[test]
    fn devices_paired_with_one_are_with_each_other() {
        let dir = std::env::temp_dir().join(format!("connect-introduce-{}", std::process::id()));
        let node = |name: &str, kind| {
            let d = dir.join(name);
            Node::launch(
                name.into(),
                kind,
                &d,
                Kept::open(&d, Vec::new),
                None,
                Box::new(|_| {}),
                false,
            )
        };
        let (a, b, c) = (
            node("a", DeviceKind::Phone),
            node("b", DeviceKind::Computer),
            node("c", DeviceKind::Computer),
        );
        let at = |n: &Node| {
            let mut seen = Seen::new(n.me());
            seen.addrs = vec![SocketAddr::from(([127, 0, 0, 1], n.port))];
            seen
        };
        for (other, key) in [(&a, [1; 32]), (&c, [3; 32])] {
            lock(&other.inner.store).pair(b.me(), key);
            lock(&b.inner.store).pair(other.me(), key);
            lock(&other.inner.seen).insert(b.me().id, at(&b));
            other.inner.reach(&b.me().id);
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while lock(&a.inner.store).key(&c.me().id).is_none()
            || lock(&c.inner.store).key(&a.me().id).is_none()
        {
            assert!(Instant::now() < deadline, "never introduced");
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(
            lock(&a.inner.store).key(&c.me().id),
            lock(&c.inner.store).key(&a.me().id)
        );
        // Seen on the network, as mDNS would say: they connect.
        lock(&a.inner.seen).insert(c.me().id, at(&c));
        a.inner.reach(&c.me().id);
        while !c
            .peers()
            .iter()
            .any(|p| p.device.id == a.me().id && p.connected)
        {
            assert!(Instant::now() < deadline, "A and C never connected");
            std::thread::sleep(Duration::from_millis(20));
        }
        // Forgotten on A: not introduced again.
        a.forget(&c.me().id);
        let key = lock(&b.inner.store).introduction(&a.me().id, &c.me().id);
        assert!(!lock(&a.inner.store).introduced(c.me(), key, None, &b.me().id));
        drop((a, b, c));
        let _ = std::fs::remove_dir_all(dir);
    }
}
