//! What the user's devices keep in common: the songs played lately, the
//! playlists made in Music itself, and where the music last stopped, on
//! whichever device.
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

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::time::{SystemTime, UNIX_EPOCH};

use music_core::Handoff;
use music_core::model::Track;
use serde::{Deserialize, Serialize};

pub use history::History;
pub use playlists::{Playlist, PlaylistSong, Playlists};

/// When a change was made, and on which device: of two changes to the same
/// thing, the later wins, and the device decides between two made at the
/// same time.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Stamp {
    /// Milliseconds since 1970, on the device's clock or later.
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
}

impl Shared {
    pub fn is_empty(&self) -> bool {
        *self == Shared::default()
    }

    /// Take `other` in: whether anything changed.
    pub fn merge(&mut self, other: Shared) -> bool {
        let history = self.history.merge(other.history);
        let playlists = self.playlists.merge(other.playlists);
        let session = latest(&mut self.session, other.session);
        history || playlists || session
    }

    fn newest(&self) -> u64 {
        self.history
            .newest()
            .max(self.playlists.newest())
            .max(self.session.as_ref().map(|s| &s.at))
            .map_or(0, |s| s.at)
    }
}

/// This device's copy, kept on disk as it changes. Clones share it.
#[derive(Clone)]
pub struct Kept {
    state: Arc<Mutex<State>>,
    /// Asks for the copy to be written; with a sender, told once it is.
    save: mpsc::Sender<Option<mpsc::Sender<()>>>,
}

struct State {
    me: String,
    /// The last stamp's time: a change made here is always later than all
    /// those it knows of, even with this device's clock behind.
    clock: u64,
    shared: Shared,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    me: String,
    #[serde(default)]
    shared: Shared,
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
        Self { me, clock, shared }
    }

    fn stamp(&mut self, now: u64) -> Stamp {
        self.clock = now.max(self.clock + 1);
        Stamp {
            at: self.clock,
            device: self.me.clone(),
        }
    }

    fn merge(&mut self, other: Shared) -> bool {
        let changed = self.shared.merge(other);
        self.clock = self.clock.max(self.shared.newest());
        changed
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
        tracks: Vec<Track>,
        now: u64,
    ) -> (Shared, bool) {
        let at = self.stamp(now);
        self.apply(Shared {
            playlists: Playlists::create(id, name, tracks, new_id, at),
            ..Shared::default()
        })
    }

    fn rename_playlist(&mut self, id: &str, name: String, now: u64) -> (Shared, bool) {
        let at = self.stamp(now);
        self.apply(Shared {
            playlists: Playlists::rename(id, name, at),
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
    /// What `dir` keeps. The first time, `history` gives the songs played
    /// before, the last first, to start with.
    pub fn open(dir: &Path, history: impl FnOnce() -> Vec<Track>) -> Self {
        let path = dir.join("shared.json");
        let saved = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Saved>(&b).ok())
            .filter(|s| !s.me.is_empty());
        // There but unreadable: put aside rather than written over.
        if saved.is_none() && path.exists() {
            let _ = std::fs::rename(&path, path.with_extension("json.bad"));
        }
        let fresh = saved.is_none();
        let saved = saved.unwrap_or_else(|| Saved {
            me: new_id(),
            shared: Shared::default(),
        });
        let mut state = State::new(saved.me, saved.shared);
        if fresh {
            let at = now();
            let me = state.me.clone();
            let tracks = history();
            for (i, track) in tracks.into_iter().take(history::KEPT).enumerate().rev() {
                let stamp = Stamp {
                    at: at - i as u64,
                    device: me.clone(),
                };
                state.merge(Shared {
                    history: History::play(track, stamp),
                    ..Shared::default()
                });
            }
        }
        let state = Arc::new(Mutex::new(state));
        let (save, saves) = mpsc::channel();
        let kept = state.clone();
        let _ = std::thread::Builder::new()
            .name("synced-save".into())
            .spawn(move || keep(&kept, &path, &saves));
        if fresh {
            let _ = save.send(None);
        }
        Self { state, save }
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
        lock(&self.state).shared.playlists.all()
    }

    pub fn playlist(&self, id: &str) -> Option<Playlist> {
        lock(&self.state).shared.playlists.get(id)
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
        lock(&self.state).shared.clone()
    }

    /// Take in what another device sent: whether anything changed here.
    pub fn merge(&self, other: Shared) -> bool {
        let changed = lock(&self.state).merge(other);
        if changed {
            let _ = self.save.send(None);
        }
        changed
    }

    fn change(&self, make: impl FnOnce(&mut State) -> (Shared, bool)) -> Shared {
        let (change, changed) = make(&mut lock(&self.state));
        if changed {
            let _ = self.save.send(None);
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

    /// On disk now, before the app closes; a few seconds at most.
    pub fn flush(&self) {
        let (done, wait) = mpsc::channel();
        if self.save.send(Some(done)).is_ok() {
            let _ = wait.recv_timeout(std::time::Duration::from_secs(3));
        }
    }

    /// A new playlist; its id with the change.
    pub fn create_playlist(&self, name: String, tracks: Vec<Track>) -> (String, Shared) {
        let id = new_id();
        let change = self.change(|s| s.create_playlist(&id, name, tracks, now()));
        (id, change)
    }

    pub fn rename_playlist(&self, id: &str, name: String) -> Shared {
        self.change(|s| s.rename_playlist(id, name, now()))
    }

    pub fn delete_playlist(&self, id: &str) -> Shared {
        self.change(|s| s.delete_playlist(id, now()))
    }

    /// `tracks` added at the end of playlist `id`.
    pub fn add_to_playlist(&self, id: &str, tracks: Vec<Track>) -> Shared {
        self.change(|s| s.add_to_playlist(id, tracks, now()))
    }

    /// The songs of `entries` taken out of playlist `id`.
    pub fn remove_from_playlist(&self, id: &str, entries: &[String]) -> Shared {
        self.change(|s| s.remove_from_playlist(id, entries))
    }

    /// The song of `entry` moved to be the `to`-th of playlist `id`,
    /// counted without it.
    pub fn move_in_playlist(&self, id: &str, entry: &str, to: usize) -> Shared {
        self.change(|s| s.move_in_playlist(id, entry, to, now()))
    }
}

/// Write the copy to `path` each time it changed, once the changes in a
/// row are in: until no copy is left to change.
fn keep(state: &Mutex<State>, path: &Path, saves: &mpsc::Receiver<Option<mpsc::Sender<()>>>) {
    while let Ok(first) = saves.recv() {
        let mut waiting: Vec<mpsc::Sender<()>> = first.into_iter().collect();
        while let Ok(more) = saves.try_recv() {
            waiting.extend(more);
        }
        let bytes = {
            let state = lock(state);
            serde_json::to_vec(&Saved {
                me: state.me.clone(),
                shared: state.shared.clone(),
            })
        };
        if let Ok(bytes) = bytes {
            write(path, &bytes);
        }
        for done in waiting {
            let _ = done.send(());
        }
    }
}

fn write(path: &Path, bytes: &[u8]) {
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
    if written.is_ok() {
        let _ = std::fs::rename(&partial, path);
    }
}

#[cfg(test)]
mod tests;
