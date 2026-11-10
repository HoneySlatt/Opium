//! What the user's devices keep in common: the songs played lately, the
//! playlists made in Opium itself, where the music last stopped, on
//! whichever device, when the library's newest songs were first seen, the
//! playlists songs were last added to, and the user's settings.
//!
//! Each device keeps its own copy, and changes it while alone. When two
//! meet, each takes the other's in, and they come out the same whatever was
//! changed where, and in whatever order and however often they meet: a song
//! played on one is in both histories, songs added to a playlist on each
//! are all in it, the latest name given wins, and what is deleted or
//! cleared stays so, however late a device hears of it.

mod history;
mod place;
mod playlists;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::time::{SystemTime, UNIX_EPOCH};

use opium_core::Handoff;
use opium_core::files::put_aside;
use opium_core::model::Track;
use serde::{Deserialize, Serialize};

pub use history::History;
pub use playlists::{Playlist, PlaylistSong, Playlists};

/// When a change was made, and on which device: of two changes to the same
/// thing, the later wins, and the device decides between two made at the
/// same time.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Stamp {
    /// Milliseconds since 1970, on the device's clock or later; for the
    /// songs played before the devices shared their history, a count from 1
    /// up, below any real play (see [`Kept::open`]).
    pub at: u64,
    pub device: String,
}

/// A value and when it was given: the one given last wins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Latest<T> {
    pub at: Stamp,
    pub value: T,
}

/// `theirs` instead of `mine` if given later: whether it was.
pub(crate) fn latest<T>(mine: &mut Option<Latest<T>>, theirs: Option<Latest<T>>) -> bool {
    if let Some(theirs) = theirs
        && mine.as_ref().is_none_or(|m| theirs.at > m.at)
    {
        *mine = Some(theirs);
        return true;
    }
    false
}

/// When a time another device says is worth believing. These two fields are
/// the only ones a device's own clock decides outright — everywhere else the
/// hybrid clock orders changes — so a device booting with its clock in 1970,
/// or years ahead, would otherwise pin every song's "first seen" and the
/// playlists last added to for good, on every device, unrecoverably.
fn plausible(at: u64, seconds: bool) -> bool {
    /// 2000-01-01, before which no device of this app's time ran.
    const EARLIEST: u64 = 946_684_800;
    /// A day's room for a clock a little ahead, or a timezone's worth.
    const AHEAD: u64 = 24 * 60 * 60;
    let (earliest, now) = if seconds {
        (EARLIEST, now() / 1000)
    } else {
        (EARLIEST * 1000, now())
    };
    let ahead = if seconds { AHEAD } else { AHEAD * 1000 };
    at >= earliest && at <= now.saturating_add(ahead)
}

/// `theirs` taken in, of two times for a song the earliest: whether any was.
fn earliest(mine: &mut BTreeMap<String, u64>, theirs: BTreeMap<String, u64>) -> bool {
    let mut changed = false;
    for (id, at) in theirs {
        if !plausible(at, true) {
            continue;
        }
        let known = mine.entry(id).or_insert(u64::MAX);
        if at < *known {
            *known = at;
            changed = true;
        }
    }
    changed
}

/// `theirs` taken in, of two times for a playlist the latest, the latest
/// [`USED`] kept: whether that changed anything.
fn latest_used(mine: &mut BTreeMap<String, u64>, theirs: BTreeMap<String, u64>) -> bool {
    let before = mine.clone();
    for (id, at) in theirs.into_iter().filter(|&(_, at)| plausible(at, false)) {
        let known = mine.entry(id).or_insert(0);
        *known = (*known).max(at);
    }
    // The same whichever device trims: by time, then by id.
    let mut all: Vec<(String, u64)> = std::mem::take(mine).into_iter().collect();
    all.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    *mine = all.into_iter().take(USED).collect();
    *mine != before
}

/// All that is kept in common, or some changes to it: what one device
/// sends another.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Shared {
    #[serde(default)]
    pub history: History,
    #[serde(default)]
    pub playlists: Playlists,
    /// The queue and where in it the music last stopped, paused or left:
    /// the device that played last says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session: Option<Latest<Handoff>>,
    /// When each new library song was first seen on any of the devices,
    /// by the service's id for it, in seconds since 1970: the earliest
    /// wins. It dates Recently Added where the service doesn't.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    seen: BTreeMap<String, u64>,
    /// The playlists songs were last added to, by id, and when, in ms since
    /// 1970: the latest [`USED`] of them.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    used: BTreeMap<String, u64>,
    /// Settings alike on every device, by name, as text: the last given wins.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    settings: BTreeMap<String, Latest<String>>,
}

/// [`Shared`] but its playlists, serialized as it is, borrowed.
#[derive(Serialize)]
struct Rest<'a> {
    history: &'a History,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: &'a Option<Latest<Handoff>>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    seen: &'a BTreeMap<String, u64>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    used: &'a BTreeMap<String, u64>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    settings: &'a BTreeMap<String, Latest<String>>,
}

/// A copy in short: two devices whose digests are alike have it alike, and
/// those meeting send each other only the parts whose digests differ
/// rather than all of it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Digest {
    rest: u64,
    /// Each playlist's, by id.
    playlists: BTreeMap<String, u64>,
}

/// `value`'s digest, the same on every device: FNV-1a over its JSON.
fn digest(value: &impl Serialize) -> u64 {
    struct Fnv(u64);
    impl std::io::Write for Fnv {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            for b in bytes {
                self.0 = (self.0 ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut fnv = Fnv(0xcbf2_9ce4_8422_2325);
    let _ = serde_json::to_writer(&mut fnv, value);
    fnv.0
}

/// As many playlists as Add to Playlist shows as the last ones.
pub const USED: usize = 3;

impl Shared {
    pub fn is_empty(&self) -> bool {
        *self == Shared::default()
    }

    /// Take `other` in: whether anything changed.
    pub fn merge(&mut self, other: Shared) -> bool {
        let (playlists, rest) = self.merge_parts(other);
        playlists || rest
    }

    /// Take `other` in: whether the playlists changed, and whether the rest did.
    fn merge_parts(&mut self, other: Shared) -> (bool, bool) {
        let history = self.history.merge(other.history);
        let playlists = self.playlists.merge(other.playlists);
        let session = latest(&mut self.session, other.session);
        let seen = earliest(&mut self.seen, other.seen);
        let used = latest_used(&mut self.used, other.used);
        let mut settings = false;
        for (name, value) in other.settings {
            let mut mine = self.settings.remove(&name);
            settings |= latest(&mut mine, Some(value));
            self.settings.extend(mine.map(|m| (name, m)));
        }
        (playlists, history || session || seen || used || settings)
    }

    /// All but the playlists, as kept and digested apart from them.
    fn rest(&self) -> Rest<'_> {
        Rest {
            history: &self.history,
            session: &self.session,
            seen: &self.seen,
            used: &self.used,
            settings: &self.settings,
        }
    }

    pub fn digest(&self) -> Digest {
        Digest {
            rest: digest(&self.rest()),
            playlists: self.playlists.digests(),
        }
    }

    /// What of this copy isn't as `theirs` says another device has it:
    /// what to send it, the rest being alike.
    pub fn unlike(&self, theirs: &Digest) -> Shared {
        let mut unlike = if digest(&self.rest()) == theirs.rest {
            Shared::default()
        } else {
            Shared {
                history: self.history.clone(),
                playlists: Playlists::default(),
                session: self.session.clone(),
                seen: self.seen.clone(),
                used: self.used.clone(),
                settings: self.settings.clone(),
            }
        };
        unlike.playlists = self.playlists.unlike(&theirs.playlists);
        unlike
    }

    fn newest(&self) -> u64 {
        self.history
            .newest()
            .max(self.playlists.newest())
            .max(self.session.as_ref().map(|s| &s.at))
            .max(self.settings.values().map(|s| &s.at).max())
            .map_or(0, |s| s.at)
    }

    /// This copy, or change, cut into parts of at most `max` bytes each as
    /// JSON, for a device that takes messages only so long; none if it is
    /// empty.
    ///
    /// Taken in by another device in any order, any number of times, the
    /// parts come to the same as the whole: the merge ignores order and
    /// repetition (ADR-0014), and each piece a part holds merges on its
    /// own. The pieces: the history, the last session, the playlists used
    /// last and the settings, each whole; each song's first sighting; each
    /// playlist's name, description, cover and deletion together; and each
    /// song in a playlist, so that one playlist can span parts. A piece
    /// longer than `max` on its own goes alone, in a part over `max`: none
    /// comes near in use (a history of 100 songs, a queue of 100, a cover).
    pub fn into_parts(self, max: usize) -> Vec<Shared> {
        let Shared {
            history,
            playlists,
            session,
            seen,
            used,
            settings,
        } = self;
        let mut parts = Parts::new(max);
        if history != History::default() {
            let len = json_len(&history) + PIECE_SLACK;
            parts.room(len).history = history;
        }
        if session.is_some() {
            let len = json_len(&session) + PIECE_SLACK;
            parts.room(len).session = session;
        }
        if !used.is_empty() {
            let len = json_len(&used) + PIECE_SLACK;
            parts.room(len).used = used;
        }
        if !settings.is_empty() {
            let len = json_len(&settings) + PIECE_SLACK;
            parts.room(len).settings = settings;
        }
        for (id, at) in seen {
            let len = json_len(&id) + json_len(&at) + PIECE_SLACK;
            parts.room(len).seen.insert(id, at);
        }
        playlists.cut(&mut parts);
        parts.finish()
    }

    /// Leaves out the playlists whose id this app doesn't make, as a paired
    /// device may send them: an id is sixteen hexadecimal digits in lower
    /// case, up to 64 tolerated (see `is_playlist_id`), as a playlist's id
    /// names its cover's file on this device. What another device sends
    /// goes through this before [`Kept::merge`]. Returns how many it left
    /// out.
    pub fn leave_out_bad_ids(&mut self) -> usize {
        self.playlists.leave_out_bad_ids()
    }
}

/// Whether `id` is a playlist's id as this app makes them: [`new_id`]
/// makes sixteen hexadecimal digits in lower case, and has since playlists
/// began. Up to 64 such digits pass, as tolerance of a length that might
/// change, not a plan to change it. Such an id names a file inside a folder
/// as it is: no `/`, `\` or `.`, never empty, and no `-`, which ends the
/// id in its cover's file name.
pub(crate) fn is_playlist_id(id: &str) -> bool {
    (1..=64).contains(&id.len()) && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The bytes `value` takes as JSON, counted rather than kept.
pub(crate) fn json_len(value: &impl Serialize) -> usize {
    struct Count(usize);
    impl std::io::Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0);
    let _ = serde_json::to_writer(&mut count, value);
    count.0
}

/// What a piece of a part takes as JSON besides its own: its key, a colon,
/// a comma; and for a song in a playlist, besides its playlist's id, what
/// holds the playlist's songs (`:{"songs":{` and `}}`), counted with each
/// of its songs rather than once a part.
pub(crate) const PIECE_SLACK: usize = 16;

/// The most an empty part takes as JSON:
/// `{"history":{},"playlists":{"lists":{}},"seen":{}}` and room to spare.
const PART_BASE: usize = 64;

/// A copy being cut into parts: see [`Shared::into_parts`].
pub(crate) struct Parts {
    max: usize,
    done: Vec<Shared>,
    part: Shared,
    /// The most bytes `part` takes as JSON: [`PART_BASE`] and what each of
    /// its pieces was said to take.
    len: usize,
    /// Whether `part` holds a piece yet.
    holds: bool,
}

impl Parts {
    fn new(max: usize) -> Self {
        Self {
            max,
            done: Vec::new(),
            part: Shared::default(),
            len: PART_BASE,
            holds: false,
        }
    }

    /// The part to put a piece in that takes `len` bytes of JSON there,
    /// its key and punctuation counted: the part being filled while the
    /// piece fits in `max`, else a new one, which a piece longer than
    /// `max` has to itself.
    pub(crate) fn room(&mut self, len: usize) -> &mut Shared {
        if self.holds && self.len + len > self.max {
            self.done.push(std::mem::take(&mut self.part));
            self.len = PART_BASE;
        }
        self.len += len;
        self.holds = true;
        &mut self.part
    }

    fn finish(mut self) -> Vec<Shared> {
        if self.holds {
            self.done.push(self.part);
        }
        self.done
    }
}

/// This device's copy, kept on disk as it changes. Clones share it.
#[derive(Clone)]
pub struct Kept {
    state: Arc<Mutex<State>>,
    /// Asks for the copy to be written.
    save: mpsc::Sender<Save>,
    /// The playlists read from disk, after the rest.
    loaded: Arc<Loaded>,
    /// Where the playlists' chosen covers are kept as files, for the apps
    /// to show as they show any other.
    covers: std::path::PathBuf,
}

/// Whether something done in the background is.
#[derive(Default)]
struct Loaded {
    done: Mutex<bool>,
    ready: std::sync::Condvar,
}

impl Loaded {
    fn finish(&self) {
        *lock(&self.done) = true;
        self.ready.notify_all();
    }

    fn wait(&self) {
        let mut done = lock(&self.done);
        while !*done {
            done = self.ready.wait(done).unwrap_or_else(|e| e.into_inner());
        }
    }
}

struct State {
    me: String,
    /// The last stamp's time: a change made here is always later than all
    /// those it knows of, even with this device's clock behind.
    clock: u64,
    shared: Shared,
    /// The playlists changed since they were last written: kept in a file
    /// of their own, much the largest, they are written only then.
    playlists_changed: bool,
    /// The playlists are in `shared.json`, as kept before they had a file
    /// of their own, and not written to theirs yet: `shared.json` keeps
    /// them until they are.
    moving: bool,
    /// Whether `shared.json` may be written: not when it was there, didn't
    /// read and couldn't be put aside.
    write_rest: bool,
    /// Whether `playlists.json` may be written, likewise.
    write_playlists: bool,
}

/// `shared.json`: this device's id and its copy, but for the playlists,
/// in `playlists.json` (in it too before they had a file of their own).
#[derive(Deserialize)]
struct Saved {
    me: String,
    #[serde(default)]
    shared: Shared,
}

/// [`Saved`] as written: `shared` a [`Rest`], or the whole [`Shared`]
/// while the playlists are moving to their own file.
#[derive(Serialize)]
struct SavedRef<'a, S> {
    me: &'a str,
    shared: S,
}

/// What reading a file kept here found: see [`read_kept`].
enum Found<T> {
    Read(T),
    /// No file: the first time.
    Missing,
    /// A file that didn't read, put aside: the copy starts without it.
    PutAside,
    /// A file that didn't read and couldn't be put aside: never to be
    /// written over, the copy kept in memory only.
    Stuck,
}

/// `path` read through `parse`. A file there that can't be read, or that
/// `parse` refuses, is put aside rather than written over, as `<name>.bad`
/// or, when taken, the first of `<name>.bad.1`, `.2`… free
/// ([`put_aside`]); what was done is said on the standard error.
fn read_kept<T>(path: &Path, parse: impl FnOnce(&[u8]) -> Option<T>) -> Found<T> {
    let why = match std::fs::read(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Found::Missing,
        Err(e) => format!("can't be read: {e}"),
        Ok(bytes) => match parse(&bytes) {
            Some(value) => return Found::Read(value),
            None => "isn't as this build keeps it".to_owned(),
        },
    };
    match put_aside(path, &path.with_extension("json.bad")) {
        Ok(aside) => {
            eprintln!(
                "synced: {} {why}: put aside as {}",
                path.display(),
                aside.display()
            );
            Found::PutAside
        }
        Err(e) => {
            eprintln!(
                "synced: {} {why}, and can't be put aside ({e}): left as it is, never written \
                 over; what it would keep is kept in memory only until Opium closes",
                path.display()
            );
            Found::Stuck
        }
    }
}

/// What was kept, read again without the songs of a service this build
/// doesn't know (one taken out of the app): those history entries and
/// playlist rows go, and a last session holding any is forgotten.
fn without_unknown_services(bytes: &[u8]) -> Option<Saved> {
    use serde_json::Value;
    let mut saved: Value = serde_json::from_slice(bytes).ok()?;
    let shared = saved.get_mut("shared")?;
    if shared.get("session").is_some_and(unknown) {
        shared["session"] = Value::Null;
    }
    prune(shared);
    serde_json::from_value(saved).ok()
}

/// The playlists kept, read again so too.
fn playlists_without_unknown_services(bytes: &[u8]) -> Option<Playlists> {
    let mut playlists: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    prune(&mut playlists);
    serde_json::from_value(playlists).ok()
}

fn unknown(value: &serde_json::Value) -> bool {
    use serde_json::Value;
    match value {
        Value::Object(map) => map.iter().any(|(key, v)| {
            (key == "provider"
                && serde_json::from_value::<opium_core::model::ProviderKind>(v.clone()).is_err())
                || unknown(v)
        }),
        Value::Array(items) => items.iter().any(unknown),
        _ => false,
    }
}

/// The history's entries and the playlists' rows, at any depth: each
/// holds a `track`; those of an unknown service go.
fn prune(value: &mut serde_json::Value) {
    if let serde_json::Value::Object(map) = value {
        map.retain(|_, v| v.get("track").is_none_or(|track| !unknown(track)));
        map.values_mut().for_each(prune);
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

fn new_id() -> String {
    format!("{:016x}", rand::random::<u64>())
}

impl State {
    fn new(me: String, shared: Shared) -> Self {
        let clock = shared.newest();
        Self {
            me,
            clock,
            shared,
            playlists_changed: false,
            moving: false,
            write_rest: true,
            write_playlists: true,
        }
    }

    /// `tracks`, the songs played before the devices shared their history,
    /// the last first, as plays older than any real one: stamped 1, 2… up
    /// to the last played, rather than now. Stamped now, they would rank
    /// above every play a paired device made before now, and push those out
    /// of the latest [`history::KEPT`] once the devices meet. Ranked so,
    /// the merge's rules stay as they are: each song once at its latest
    /// stamp, the latest kept, and a clear made anywhere, being later,
    /// takes them out too.
    fn played_before(&mut self, tracks: Vec<Track>) {
        let tracks: Vec<Track> = tracks.into_iter().take(history::KEPT).collect();
        let count = tracks.len() as u64;
        for (i, track) in tracks.into_iter().enumerate() {
            let at = Stamp {
                at: count - i as u64,
                device: self.me.clone(),
            };
            self.merge(Shared {
                history: History::play(track, at),
                ..Shared::default()
            });
        }
    }

    fn stamp(&mut self, now: u64) -> Stamp {
        self.clock = now.max(self.clock + 1);
        Stamp {
            at: self.clock,
            device: self.me.clone(),
        }
    }

    fn merge(&mut self, other: Shared) -> bool {
        let (playlists, rest) = self.shared.merge_parts(other);
        self.playlists_changed |= playlists;
        self.clock = self.clock.max(self.shared.newest());
        playlists || rest
    }

    /// Make `change` here; it is also what to send, with whether anything
    /// changed.
    fn apply(&mut self, change: Shared) -> (Shared, bool) {
        let changed = self.merge(change.clone());
        (change, changed)
    }

    /// The music stopped, or moved on, at `session`: a change only if it
    /// isn't where it last stopped already, so that a device opened and
    /// closed without playing doesn't pass for the last one played.
    fn left(&mut self, mut session: Handoff, now: u64) -> (Shared, bool) {
        // Taken up again, it starts paused.
        session.playing = false;
        if self
            .shared
            .session
            .as_ref()
            .is_some_and(|s| s.value == session)
        {
            return (Shared::default(), false);
        }
        let at = self.stamp(now);
        self.apply(Shared {
            session: Some(Latest { at, value: session }),
            ..Shared::default()
        })
    }

    fn played(&mut self, track: Track, now: u64) -> (Shared, bool) {
        let at = self.stamp(now);
        self.apply(Shared {
            history: History::play(track, at),
            ..Shared::default()
        })
    }

    fn clear_history(&mut self, now: u64) -> (Shared, bool) {
        let at = self.stamp(now);
        self.apply(Shared {
            history: History::clear(at),
            ..Shared::default()
        })
    }

    fn create_playlist(
        &mut self,
        id: &str,
        name: String,
        description: String,
        tracks: Vec<Track>,
        now: u64,
    ) -> (Shared, bool) {
        let at = self.stamp(now);
        self.apply(Shared {
            playlists: Playlists::create(id, name, description, tracks, new_id, at),
            ..Shared::default()
        })
    }

    fn edit_playlist(
        &mut self,
        id: &str,
        name: String,
        description: String,
        now: u64,
    ) -> (Shared, bool) {
        let at = self.stamp(now);
        self.apply(Shared {
            playlists: Playlists::edit(id, name, description, at),
            ..Shared::default()
        })
    }

    fn cover_playlist(&mut self, id: &str, cover: Option<String>, now: u64) -> (Shared, bool) {
        let at = self.stamp(now);
        self.apply(Shared {
            playlists: Playlists::cover(id, cover, at),
            ..Shared::default()
        })
    }

    fn delete_playlist(&mut self, id: &str, now: u64) -> (Shared, bool) {
        let at = self.stamp(now);
        self.apply(Shared {
            playlists: Playlists::delete(id, at),
            ..Shared::default()
        })
    }

    fn add_to_playlist(&mut self, id: &str, tracks: Vec<Track>, now: u64) -> (Shared, bool) {
        let at = self.stamp(now);
        let playlists = self.shared.playlists.add(id, tracks, new_id, at);
        self.apply(Shared {
            playlists,
            ..Shared::default()
        })
    }

    fn remove_from_playlist(&mut self, id: &str, entries: &[String]) -> (Shared, bool) {
        self.apply(Shared {
            playlists: Playlists::remove(id, entries),
            ..Shared::default()
        })
    }

    fn move_in_playlist(&mut self, id: &str, entry: &str, to: usize, now: u64) -> (Shared, bool) {
        let at = self.stamp(now);
        let playlists = self.shared.playlists.move_to(id, entry, to, at);
        self.apply(Shared {
            playlists,
            ..Shared::default()
        })
    }
}

impl Kept {
    /// What `dir` keeps. The first time, with no `shared.json` there,
    /// `history` gives the songs played before, the last first, to start
    /// with, ranked below every real play (see [`Stamp::at`]).
    ///
    /// A `shared.json` or `playlists.json` there that doesn't read is put
    /// aside, as `shared.json.bad` or, when taken, `shared.json.bad.1`, `.2`…
    /// (and `playlists.json.bad` likewise), and the copy starts without
    /// what it held; for `shared.json`, with a new device id and no history
    /// from before, as that is no first run. One that can't be put aside is
    /// left as it is and never written over; the copy then keeps what would
    /// go in it in memory only. Each is said on the standard error. The
    /// folder is made if missing. Never fails.
    pub fn open(dir: &Path, history: impl FnOnce() -> Vec<Track>) -> Self {
        // There once opened, rather than once first written to: an error
        // shows as the writes'.
        let _ = std::fs::create_dir_all(dir);
        let path = dir.join("shared.json");
        let found = read_kept(&path, |bytes| {
            serde_json::from_slice::<Saved>(bytes)
                .ok()
                .or_else(|| without_unknown_services(bytes))
                .filter(|s| !s.me.is_empty())
        });
        let first_run = matches!(found, Found::Missing);
        let write_rest = !matches!(found, Found::Stuck);
        let (saved, read) = match found {
            Found::Read(saved) => (saved, true),
            _ => (
                Saved {
                    me: new_id(),
                    shared: Shared::default(),
                },
                false,
            ),
        };
        // In `shared.json` still, as before they had a file of their own:
        // moved to it.
        let moving = !saved.shared.playlists.is_empty();
        let mut state = State::new(saved.me, saved.shared);
        state.playlists_changed = moving;
        state.moving = moving;
        state.write_rest = write_rest;
        if first_run {
            state.played_before(history());
        }
        let state = Arc::new(Mutex::new(state));
        let loaded = Arc::new(Loaded::default());
        // The playlists, much the largest, read meanwhile: what needs them
        // waits for them, not the app's start.
        let lists = dir.join("playlists.json");
        let (kept, done) = (state.clone(), loaded.clone());
        let load = move || {
            let found = read_kept(&lists, |bytes| {
                serde_json::from_slice::<Playlists>(bytes)
                    .ok()
                    .or_else(|| playlists_without_unknown_services(bytes))
            });
            {
                let mut state = lock(&kept);
                match found {
                    Found::Read(playlists) => {
                        state.merge(Shared {
                            playlists,
                            ..Shared::default()
                        });
                        state.playlists_changed = moving;
                    }
                    Found::Stuck => state.write_playlists = false,
                    Found::Missing | Found::PutAside => {}
                }
            }
            done.finish();
        };
        if std::thread::Builder::new()
            .name("synced-load".into())
            .spawn(load.clone())
            .is_err()
        {
            load();
        }
        let (save, saves) = mpsc::channel();
        let (kept, done) = (state.clone(), loaded.clone());
        let _ = std::thread::Builder::new()
            .name("synced-save".into())
            .spawn(move || {
                // Not before they are all in.
                done.wait();
                keep(&kept, &path, &saves)
            });
        if !read || moving {
            let _ = save.send(Save::Now);
        }
        Self {
            state,
            save,
            loaded,
            covers: dir.join("covers"),
        }
    }

    /// This device, in the stamps of its changes.
    pub fn me(&self) -> String {
        lock(&self.state).me.clone()
    }

    /// The songs played lately, the last first.
    pub fn history(&self) -> Vec<Track> {
        lock(&self.state).shared.history.tracks()
    }

    pub fn playlists(&self) -> Vec<Playlist> {
        let all = self.with_playlists().shared.playlists.all();
        all.into_iter().map(|p| self.with_cover(p)).collect()
    }

    pub fn playlist(&self, id: &str) -> Option<Playlist> {
        let playlist = self.with_playlists().shared.playlists.get(id)?;
        Some(self.with_cover(playlist))
    }

    /// `playlist` with its chosen cover kept as a file, written the first
    /// time it is asked for (the one before it gone). The file is named by
    /// the playlist's id, so a playlist whose id this app doesn't make,
    /// which might name a file outside the covers' folder, shows its first
    /// song's instead.
    fn with_cover(&self, mut playlist: Playlist) -> Playlist {
        let Some(data) = playlist.cover_data.take() else {
            return playlist;
        };
        if !is_playlist_id(&playlist.id) {
            return playlist;
        }
        let name = format!("{}-{:016x}.jpg", playlist.id, digest(&data));
        let path = self.covers.join(&name);
        if !path.exists() {
            use base64::Engine;
            let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&data) else {
                return playlist;
            };
            if let Ok(old) = std::fs::read_dir(&self.covers) {
                let mine = format!("{}-", playlist.id);
                for file in old.flatten() {
                    if file.file_name().to_string_lossy().starts_with(&mine) {
                        let _ = std::fs::remove_file(file.path());
                    }
                }
            }
            let _ = std::fs::create_dir_all(&self.covers);
            let partial = path.with_extension("part");
            if std::fs::write(&partial, bytes)
                .and_then(|()| std::fs::rename(&partial, &path))
                .is_err()
            {
                return playlist;
            }
        }
        playlist.cover = Some(format!("file://{}", path.display()));
        playlist
    }

    /// Where the music last stopped, and on which device: its id in the
    /// stamp, [`Kept::me`] if here.
    pub fn session(&self) -> Option<(Handoff, Stamp)> {
        lock(&self.state)
            .shared
            .session
            .as_ref()
            .map(|s| (s.value.clone(), s.at.clone()))
    }

    /// Everything kept: what a device gets on meeting this one.
    pub fn everything(&self) -> Shared {
        self.with_playlists().shared.clone()
    }

    /// This copy in short, for a device meeting this one.
    pub fn digest(&self) -> Digest {
        self.with_playlists().shared.digest()
    }

    /// What a device whose copy is `theirs` in short lacks of this one.
    pub fn unlike(&self, theirs: &Digest) -> Shared {
        self.with_playlists().shared.unlike(theirs)
    }

    /// Take in what another device sent: whether anything changed here.
    /// Written once what it sends stops coming for a moment (a second, five
    /// at most: see `Save::Soon`), or at the next [`Kept::flush`].
    pub fn merge(&self, other: Shared) -> bool {
        let changed = self.with_playlists().merge(other);
        if changed {
            let _ = self.save.send(Save::Soon);
        }
        changed
    }

    /// The copy, its playlists read.
    fn with_playlists(&self) -> MutexGuard<'_, State> {
        self.loaded.wait();
        lock(&self.state)
    }

    fn change(&self, make: impl FnOnce(&mut State) -> (Shared, bool)) -> Shared {
        let (change, changed) = make(&mut lock(&self.state));
        if changed {
            let _ = self.save.send(Save::Now);
        }
        change
    }

    /// `track` played here, now. Each change made returns what changed, to
    /// send the other devices.
    pub fn played(&self, track: Track) -> Shared {
        self.change(|s| s.played(track, now()))
    }

    pub fn clear_history(&self) -> Shared {
        self.change(|s| s.clear_history(now()))
    }

    /// The music stopped here, or moved on, at `session`.
    pub fn left(&self, session: Handoff) -> Shared {
        self.change(|s| s.left(session, now()))
    }

    /// Where the music stopped as kept here before the devices shared it,
    /// `at` (the file's time) in ms since 1970: taken if none is shared
    /// yet, so that of the devices' own the latest wins as they meet.
    pub fn take_old_session(&self, session: Handoff, at: u64) {
        self.change(|s| {
            if s.shared.session.is_some() {
                return (Shared::default(), false);
            }
            let at = Stamp {
                at,
                device: s.me.clone(),
            };
            let session = Handoff {
                playing: false,
                ..session
            };
            s.apply(Shared {
                session: Some(Latest { at, value: session }),
                ..Shared::default()
            })
        });
    }

    /// When each of `ids` was first seen on any device, of those known.
    pub fn seen(&self, ids: &[String]) -> HashMap<String, u64> {
        let state = lock(&self.state);
        ids.iter()
            .filter_map(|id| Some((id.clone(), *state.shared.seen.get(id)?)))
            .collect()
    }

    /// Songs first seen here at those times: what is earlier than known,
    /// to send the other devices.
    pub fn first_seen(&self, times: Vec<(String, u64)>) -> Shared {
        self.change(|s| {
            let earlier: BTreeMap<String, u64> = times
                .into_iter()
                .filter(|(id, at)| s.shared.seen.get(id).is_none_or(|known| at < known))
                .collect();
            s.apply(Shared {
                seen: earlier,
                ..Shared::default()
            })
        })
    }

    /// The playlists songs were last added to, by id, the last first.
    pub fn used(&self) -> Vec<String> {
        let state = lock(&self.state);
        let mut used: Vec<(&String, &u64)> = state.shared.used.iter().collect();
        used.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        used.into_iter().map(|(id, _)| id.clone()).collect()
    }

    /// Songs added now to the playlists `ids`, the first last of all.
    pub fn use_playlists(&self, ids: &[String]) -> Shared {
        let at = now();
        self.change(|s| {
            let used = ids
                .iter()
                .enumerate()
                .map(|(i, id)| (id.clone(), at.saturating_sub(i as u64)))
                .collect();
            s.apply(Shared {
                used,
                ..Shared::default()
            })
        })
    }

    /// The settings alike on every device, by name.
    pub fn settings(&self) -> BTreeMap<String, String> {
        lock(&self.state)
            .shared
            .settings
            .iter()
            .map(|(name, s)| (name.clone(), s.value.clone()))
            .collect()
    }

    /// Setting `name` set to `value` here, now: nothing if it is so already.
    pub fn set_setting(&self, name: &str, value: String) -> Shared {
        self.change(|s| {
            if s.shared
                .settings
                .get(name)
                .is_some_and(|v| v.value == value)
            {
                return (Shared::default(), false);
            }
            let at = s.stamp(now());
            s.apply(Shared {
                settings: BTreeMap::from([(name.to_owned(), Latest { at, value })]),
                ..Shared::default()
            })
        })
    }

    /// Setting `name` as it was here before the devices shared it: taken if
    /// none is shared yet, and given way to by any set since, anywhere.
    pub fn offer_setting(&self, name: &str, value: String) -> Shared {
        self.change(|s| {
            if s.shared.settings.contains_key(name) {
                return (Shared::default(), false);
            }
            let at = Stamp {
                at: 0,
                device: s.me.clone(),
            };
            s.apply(Shared {
                settings: BTreeMap::from([(name.to_owned(), Latest { at, value })]),
                ..Shared::default()
            })
        })
    }

    /// On disk now, before the app closes; a few seconds at most.
    pub fn flush(&self) {
        let (done, wait) = mpsc::channel();
        if self.save.send(Save::Flush(done)).is_ok() {
            let _ = wait.recv_timeout(std::time::Duration::from_secs(3));
        }
    }

    /// A new playlist; its id with the change.
    pub fn create_playlist(
        &self,
        name: String,
        description: String,
        tracks: Vec<Track>,
    ) -> (String, Shared) {
        let id = new_id();
        let change = {
            self.loaded.wait();
            self.change(|s| s.create_playlist(&id, name, description, tracks, now()))
        };
        (id, change)
    }

    /// Playlist `id` named `name`, described by `description`.
    pub fn edit_playlist(&self, id: &str, name: String, description: String) -> Shared {
        {
            self.loaded.wait();
            self.change(|s| s.edit_playlist(id, name, description, now()))
        }
    }

    /// Playlist `id`'s cover `jpeg`, the same on every device; none: its
    /// first song's again.
    pub fn cover_playlist(&self, id: &str, jpeg: Option<&[u8]>) -> Shared {
        use base64::Engine;
        let cover = jpeg.map(|j| base64::engine::general_purpose::STANDARD.encode(j));
        self.loaded.wait();
        self.change(|s| s.cover_playlist(id, cover, now()))
    }

    pub fn delete_playlist(&self, id: &str) -> Shared {
        {
            self.loaded.wait();
            self.change(|s| s.delete_playlist(id, now()))
        }
    }

    /// `tracks` added at the end of playlist `id`.
    pub fn add_to_playlist(&self, id: &str, tracks: Vec<Track>) -> Shared {
        {
            self.loaded.wait();
            self.change(|s| s.add_to_playlist(id, tracks, now()))
        }
    }

    /// The songs of `entries` taken out of playlist `id`.
    pub fn remove_from_playlist(&self, id: &str, entries: &[String]) -> Shared {
        {
            self.loaded.wait();
            self.change(|s| s.remove_from_playlist(id, entries))
        }
    }

    /// The song of `entry` moved to be the `to`-th of playlist `id`,
    /// counted without it.
    pub fn move_in_playlist(&self, id: &str, entry: &str, to: usize) -> Shared {
        {
            self.loaded.wait();
            self.change(|s| s.move_in_playlist(id, entry, to, now()))
        }
    }
}

/// What the copy's writer is asked.
enum Save {
    /// A change made here: written at once.
    Now,
    /// What another device sent, which comes in parts, a large library's
    /// in many (see [`Shared::into_parts`]): written once nothing more came
    /// for [`QUIET`], or [`SOON_AT_MOST`] after the first, rather than all
    /// of the playlists again after each part. Nothing is lost meanwhile
    /// that the other device doesn't still hold.
    Soon,
    /// Written at once, and the sender told once it is.
    Flush(mpsc::Sender<()>),
}

/// How long the writer waits for more of what another device sends.
const QUIET: std::time::Duration = std::time::Duration::from_secs(1);
/// The longest it waits so.
const SOON_AT_MOST: std::time::Duration = std::time::Duration::from_secs(5);

/// Write the copy to `path` each time it changed, once the changes in a
/// row are in (see [`Save`]): until no copy is left to change. A file that
/// may not be written (see [`State::write_rest`]) is left alone.
fn keep(state: &Mutex<State>, path: &Path, saves: &mpsc::Receiver<Save>) {
    while let Ok(first) = saves.recv() {
        let since = std::time::Instant::now();
        let mut soon = matches!(first, Save::Soon);
        let mut waiting: Vec<mpsc::Sender<()>> = Vec::new();
        let mut next = Ok(first);
        while let Ok(save) = next {
            match save {
                Save::Soon => {}
                Save::Now => soon = false,
                Save::Flush(done) => {
                    waiting.push(done);
                    soon = false;
                }
            }
            let left = SOON_AT_MOST.saturating_sub(since.elapsed());
            next = if soon && !left.is_zero() {
                saves.recv_timeout(QUIET.min(left)).map_err(|_| ())
            } else {
                saves.try_recv().map_err(|_| ())
            };
        }
        // Serialized as it stands, rather than copied first. The playlists
        // first: a power cut between the two leaves them in both files,
        // read again alike.
        let playlists = {
            let mut state = lock(state);
            let changed = state.write_playlists && std::mem::take(&mut state.playlists_changed);
            changed
                .then(|| serde_json::to_vec(&state.shared.playlists).ok())
                .flatten()
        };
        if let Some(bytes) = playlists {
            let written = write(&path.with_file_name("playlists.json"), &bytes);
            let mut state = lock(state);
            if written {
                state.moving = false;
            } else {
                state.playlists_changed = true;
            }
        }
        // Moving and not written to their own file yet: still in this one,
        // or they would be in neither.
        let rest = {
            let state = lock(state);
            let me = state.me.as_str();
            match (state.write_rest, state.moving) {
                (false, _) => None,
                (true, true) => serde_json::to_vec(&SavedRef {
                    me,
                    shared: &state.shared,
                })
                .ok(),
                (true, false) => serde_json::to_vec(&SavedRef {
                    me,
                    shared: state.shared.rest(),
                })
                .ok(),
            }
        };
        if let Some(bytes) = rest {
            write(path, &bytes);
        }
        for done in waiting {
            let _ = done.send(());
        }
    }
}

/// Whether it was written.
fn write(path: &Path, bytes: &[u8]) -> bool {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let partial = path.with_extension("json.part");
    let written = {
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        // On disk before it replaces the last: a power cut midway leaves
        // one or the other whole.
        options
            .open(&partial)
            .and_then(|mut f| f.write_all(bytes).and_then(|()| f.sync_all()))
    };
    written.is_ok() && std::fs::rename(&partial, path).is_ok()
}

#[cfg(test)]
mod tests;
