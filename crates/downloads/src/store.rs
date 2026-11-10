//! Where downloaded songs are kept, and the list of them: each song a
//! sealed file of an opaque name, the list a manifest beside them, written
//! whole each time it changes.
//!
//! A song that came through a service's DRM plays only while that service's
//! subscription was confirmed in the last [`PROTECTED_FOR`]: each time the
//! app is online it is confirmed again, and once the subscription ends, or
//! the service is signed out of, its songs stop playing. The store judges
//! each service's songs at the latest time it has seen since that service
//! last confirmed, so that a clock set back offline doesn't bring an expired
//! song back; only that service's confirmation, online, takes its time back
//! ([`Store::subscription`]). A clock that reads more than [`CLOCK_SLACK`]
//! before a confirmation finds that service's songs expired too, so that a
//! confirmation made with the clock far ahead doesn't hold for good.
//!
//! A kept song's sealed file is deleted when the song is removed or kept
//! again, and otherwise only by the sweep [`Store::open`] makes, under the
//! rule it states, so that no song is lost with a manifest that doesn't
//! read.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use opium_core::files::put_aside;
use opium_core::model::{ItemId, ProviderKind, Track};
use serde::{Deserialize, Serialize};

use crate::sealed::{self, Key};

/// How long a song that came through DRM plays without the subscription
/// being confirmed again, in seconds: 30 days, as Apple Music has it.
pub const PROTECTED_FOR: u64 = 30 * 24 * 60 * 60;

/// How far before a service's confirmation the clock may read and its
/// protected songs still play: a day, for a clock put right after it.
pub const CLOCK_SLACK: u64 = 24 * 60 * 60;

const MANIFEST: &str = "manifest.json";
/// Where a manifest that doesn't read is put aside: this name, or this name
/// followed by `.1`, `.2`… ([`put_aside`]).
const PUT_ASIDE: &str = "manifest.corrupt.json";
const VERSION: u32 = 1;

/// A downloaded song.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub track: Track,
    /// Its sealed file, in the store's folder.
    file: String,
    /// How long the song is, in bytes, unsealed.
    pub len: u64,
    /// `m4a`, `flac`…: a hint for the decoder.
    pub extension: String,
    /// Frames the encoder put before the music, which the file doesn't say
    /// (Apple's 2112).
    pub priming: u32,
    /// It came through the service's DRM: it plays while the subscription
    /// is confirmed.
    pub protected: bool,
    pub kbps: Option<u32>,
    pub spatial: bool,
    /// When it was downloaded, in seconds since the epoch.
    pub added: u64,
}

/// What a download says of the song it brought, to keep it.
pub struct Song {
    pub track: Track,
    pub extension: String,
    pub priming: u32,
    pub protected: bool,
    pub kbps: Option<u32>,
    pub spatial: bool,
}

/// The list of the songs kept, in `manifest.json`, written whole each time.
///
/// `seen` and `unnamed_songs` came within version 1, and are optional: an
/// older build reads such a manifest whole, ignores them, and drops them when
/// it writes it again; it may then sweep the sealed files `unnamed_songs`
/// kept.
#[derive(Default, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    items: Vec<Item>,
    /// When each service last confirmed its subscription, in seconds since
    /// the epoch, by its name.
    confirmed: BTreeMap<String, u64>,
    /// By service's name, the latest time the store has seen since that
    /// service last confirmed, in seconds since the epoch ([`Store::seen`]).
    #[serde(default)]
    seen: BTreeMap<String, u64>,
    /// The folder was once found with sealed files beside no manifest read
    /// whole (none, or one put aside): any sealed file this manifest doesn't
    /// name may be one of its songs, and is never swept. Never cleared, by
    /// design: nothing tells such a file from a leftover.
    #[serde(default)]
    unnamed_songs: bool,
}

/// Whether a kept song plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    Plays,
    /// Until this time, in seconds since the epoch.
    PlaysUntil(u64),
    /// Its service's subscription wasn't confirmed in time.
    Expired,
}

/// A kept song, opened to play.
pub struct Open {
    pub reader: sealed::Reader<File>,
    pub len: u64,
    pub extension: String,
    pub priming: u32,
    pub kbps: Option<u32>,
    pub spatial: bool,
}

pub struct Store {
    dir: PathBuf,
    key: Key,
    items: HashMap<ItemId, Item>,
    confirmed: BTreeMap<String, u64>,
    /// By service's name, the latest time the store has seen, in seconds
    /// since the epoch: the largest `now` it was given since that service's
    /// last confirmation, which sets it to its own `now`, lower or not. Its
    /// songs are judged no earlier. Behind a lock so that judging, through
    /// `&self`, raises it.
    seen: Mutex<BTreeMap<String, u64>>,
    /// The `seen` the manifest on disk holds: as read, or as last written.
    seen_saved: Mutex<BTreeMap<String, u64>>,
    /// Kept in the manifest: [`Manifest::unnamed_songs`].
    unnamed_songs: bool,
}

fn service(provider: ProviderKind) -> String {
    format!("{provider:?}")
}

/// A folder only the user can open, made if missing.
fn private_dir(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
}

impl Store {
    /// The store in `dir`, made if missing.
    ///
    /// A manifest that doesn't read (damaged, or a later version's) is put
    /// aside as `manifest.corrupt.json`, or that name followed by `.1`,
    /// `.2`… when taken, and the store starts empty.
    ///
    /// Then the files no kept song names are swept: leftovers of a download
    /// or a save cut short (`.part`) at every opening; sealed files only
    /// when the manifest was read whole, while no manifest put aside lies in
    /// the folder, and in a folder never found with sealed files beside no
    /// manifest read whole. Each of those may mean the sealed files are
    /// songs a lost manifest named, so they stay for the user to recover.
    ///
    /// # Errors
    ///
    /// The folder can't be made or listed; the manifest can't be read (but
    /// for a missing one) or put aside.
    pub fn open(dir: &Path, key: Key) -> io::Result<Self> {
        private_dir(dir)?;
        let path = dir.join(MANIFEST);
        let (manifest, read_whole) = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Manifest>(&bytes) {
                Ok(m) if m.version <= VERSION => (m, true),
                read => {
                    let why = read
                        .err()
                        .map_or_else(|| format!("version {VERSION} at most"), |e| e.to_string());
                    let aside = put_aside(&path, &dir.join(PUT_ASIDE))?;
                    eprintln!(
                        "downloads: {} doesn't read ({why}): put aside as {}, the songs left alone",
                        path.display(),
                        aside.display()
                    );
                    (Manifest::default(), false)
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => (Manifest::default(), false),
            Err(e) => return Err(e),
        };
        // Listed after the manifest is put aside, so as to see it.
        let names = file_names(dir)?;
        let holds_songs = names.iter().any(|name| name.ends_with(".sealed"));
        // Kept on in the manifest, so that deleting a copy put aside later
        // doesn't let the sweep take its songs.
        let unnamed_songs = manifest.unnamed_songs || (!read_whole && holds_songs);
        let put_aside_here = names.iter().any(|name| name.starts_with(PUT_ASIDE));
        let store = Self {
            dir: dir.to_owned(),
            key,
            items: manifest
                .items
                .into_iter()
                .map(|item| (item.track.id.clone(), item))
                .collect(),
            confirmed: manifest.confirmed,
            seen: Mutex::new(manifest.seen.clone()),
            seen_saved: Mutex::new(manifest.seen),
            unnamed_songs,
        };
        store.sweep(&names, !unnamed_songs && !put_aside_here);
        Ok(store)
    }

    /// Take away, among `names`, the folder's, the files no song is kept
    /// in: leftovers (`.part`), and with `songs`, sealed files.
    fn sweep(&self, names: &[String], songs: bool) {
        let kept: HashSet<&str> = self.items.values().map(|i| i.file.as_str()).collect();
        for name in names {
            let unnamed_song = songs && name.ends_with(".sealed") && !kept.contains(name.as_str());
            if name.ends_with(".part") || unnamed_song {
                let _ = std::fs::remove_file(self.dir.join(name));
            }
        }
    }

    /// Note that the clock reads `now`: each confirmed service's latest time
    /// seen becomes the later of it and `now`.
    fn note_reading(&self, now: u64) {
        let mut seen = lock(&self.seen);
        for name in self.confirmed.keys() {
            let at = seen.entry(name.clone()).or_default();
            *at = (*at).max(now);
        }
    }

    fn save(&self) -> io::Result<()> {
        let mut items: Vec<&Item> = self.items.values().collect();
        items.sort_by_key(|i| (i.added, i.file.as_str()));
        #[derive(Serialize)]
        struct Out<'a> {
            version: u32,
            items: Vec<&'a Item>,
            confirmed: &'a BTreeMap<String, u64>,
            seen: &'a BTreeMap<String, u64>,
            unnamed_songs: bool,
        }
        let seen = lock(&self.seen).clone();
        let json = serde_json::to_vec(&Out {
            version: VERSION,
            items,
            confirmed: &self.confirmed,
            seen: &seen,
            unnamed_songs: self.unnamed_songs,
        })
        .map_err(io::Error::other)?;
        let part = self.dir.join("manifest.json.part");
        let mut file = File::create(&part)?;
        file.write_all(&json)?;
        file.sync_all()?;
        std::fs::rename(part, self.dir.join(MANIFEST))?;
        *lock(&self.seen_saved) = seen;
        Ok(())
    }

    pub fn get(&self, id: &ItemId) -> Option<&Item> {
        self.items.get(id)
    }

    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.items.values()
    }

    /// How `item` stands when the clock reads `now`: judged at the later of
    /// `now` and the latest time the store has seen since its service last
    /// confirmed, which `now` then raises. A song that came without DRM
    /// always plays; a protected one is `Expired` too when the clock reads
    /// more than [`CLOCK_SLACK`] before its service's confirmation.
    ///
    /// When it finds a protected song `Expired` while the manifest's latest
    /// time doesn't reach the end of a subscription's 30 days that the clock
    /// has passed, it saves the manifest, on the caller's thread, so that a
    /// restart with the clock set back finds it `Expired` too; a failed save
    /// is printed. Where it may not wait, [`Downloads`](crate::queue::Downloads)
    /// judges through `Store::judge` and makes that write on a blocking
    /// thread.
    pub fn standing(&self, item: &Item, now: u64) -> Standing {
        let standing = self.judge(item, now);
        if standing == Standing::Expired {
            self.keep_expiry_seen();
        }
        standing
    }

    /// As [`Store::standing`], without the write: the caller makes it,
    /// through [`Store::keep_expiry_seen`], where it may wait.
    pub(crate) fn judge(&self, item: &Item, now: u64) -> Standing {
        self.note_reading(now);
        if !item.protected {
            return Standing::Plays;
        }
        let name = service(item.track.id.provider);
        let Some(&at) = self.confirmed.get(&name) else {
            return Standing::Expired;
        };
        // A confirmation the clock hasn't reached was made with the clock
        // ahead, or the clock was set back since: neither is trusted.
        if now.saturating_add(CLOCK_SLACK) < at {
            return Standing::Expired;
        }
        let judged = lock(&self.seen).get(&name).copied().unwrap_or(now).max(now);
        let until = at.saturating_add(PROTECTED_FOR);
        if judged < until {
            Standing::PlaysUntil(until)
        } else {
            Standing::Expired
        }
    }

    /// Whether the latest time seen has passed the end of a subscription's
    /// 30 days that the manifest's latest time doesn't reach: a restart with
    /// the clock set back would bring its protected songs back.
    pub(crate) fn expiry_unsaved(&self) -> bool {
        let seen = lock(&self.seen);
        let saved = lock(&self.seen_saved);
        self.confirmed.iter().any(|(name, at)| {
            let until = at.saturating_add(PROTECTED_FOR);
            let seen = seen.get(name).copied().unwrap_or(0);
            let saved = saved.get(name).copied().unwrap_or(0);
            saved < until && until <= seen
        })
    }

    /// Save the manifest if [`Store::expiry_unsaved`]; a failed save is
    /// printed, and tried again at the next judgement that needs it.
    pub(crate) fn keep_expiry_seen(&self) {
        if self.expiry_unsaved()
            && let Err(e) = self.save()
        {
            eprintln!("downloads: couldn't keep the time seen: {e}");
        }
    }

    /// The song kept as `id`, opened, if it is kept and plays at `now`, as
    /// [`Store::standing`] judges it, with its write.
    pub fn open_song(&self, id: &ItemId, now: u64) -> Option<io::Result<Open>> {
        let item = self.items.get(id)?;
        if self.standing(item, now) == Standing::Expired {
            return None;
        }
        Some(self.open_item(item))
    }

    /// `item`, one of the store's, opened whatever its standing.
    pub(crate) fn open_item(&self, item: &Item) -> io::Result<Open> {
        let file = File::open(self.dir.join(&item.file))?;
        let mut reader = sealed::Reader::new(file, &self.key)?;
        if reader.len() != item.len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the kept song isn't whole",
            ));
        }
        reader.opens()?;
        Ok(Open {
            len: reader.len(),
            reader,
            extension: item.extension.clone(),
            priming: item.priming,
            kbps: item.kbps,
            spatial: item.spatial,
        })
    }

    /// A new sealed file to write a song into, kept once
    /// [`Store::keep`] is given it.
    pub fn begin(&self) -> io::Result<Pending> {
        let name = format!("{:032x}", rand::random::<u128>());
        let part = self.dir.join(format!("{name}.part"));
        let file = File::create(&part)?;
        Ok(Pending {
            name,
            part,
            writer: sealed::Writer::new(file, &self.key)?,
            len: 0,
        })
    }

    /// Keep the song written into `pending`, at `now`, in place of one kept
    /// before as the same track, whose file goes once the manifest no longer
    /// names it. A song that came through DRM confirms its service's
    /// subscription, as [`Store::subscription`] does: the service just
    /// licensed it.
    ///
    /// # Errors
    ///
    /// The song can't be sealed whole: its `.part` file stays, swept at the
    /// next opening. The manifest can't be saved: the store then names the
    /// copy kept before, if any, which plays on, and the new sealed file
    /// goes.
    pub fn keep(&mut self, pending: Pending, song: Song, now: u64) -> io::Result<&Item> {
        let (file, len) = pending.finish(&self.dir)?;
        if song.protected {
            self.note_subscription(song.track.id.provider, true, now);
        }
        let id = song.track.id.clone();
        let item = Item {
            track: song.track,
            file,
            len,
            extension: song.extension,
            priming: song.priming,
            protected: song.protected,
            kbps: song.kbps,
            spatial: song.spatial,
            added: now,
        };
        let before = self.items.insert(id.clone(), item);
        if let Err(e) = self.save() {
            // The manifest on disk still names the copy kept before: so does
            // the store, and the new file, which nothing names, goes.
            let new = match before {
                Some(before) => self.items.insert(id, before),
                None => self.items.remove(&id),
            };
            if let Some(new) = new {
                let _ = std::fs::remove_file(self.dir.join(new.file));
            }
            return Err(e);
        }
        if let Some(before) = before {
            let _ = std::fs::remove_file(self.dir.join(before.file));
        }
        Ok(&self.items[&id])
    }

    pub fn remove(&mut self, id: &ItemId) -> io::Result<()> {
        if let Some(item) = self.items.remove(id) {
            self.save()?;
            let _ = std::fs::remove_file(self.dir.join(item.file));
        }
        Ok(())
    }

    /// `provider` said whether its subscription is active, at `now`: its
    /// protected songs play for [`PROTECTED_FOR`] more, counted from `now`,
    /// or no longer. A sign-out says it is not.
    ///
    /// A confirmation is the service's word, online: it sets that service's
    /// latest time seen to `now`, lower or not, so that one clock reading far
    /// ahead doesn't hold for good; another service's is left alone. The
    /// clock is still trusted at the moment of a confirmation: set back
    /// then, the service's songs play 30 days from that reading, gaining at
    /// most how far the clock was set back; set ahead then, they play only
    /// while the clock reads no more than [`CLOCK_SLACK`] before it, until
    /// the next confirmation. Not active, `now` is only a reading, judged as
    /// any other.
    ///
    /// # Errors
    ///
    /// The manifest can't be saved; the store keeps what was said, saved
    /// with the next change.
    pub fn subscription(
        &mut self,
        provider: ProviderKind,
        active: bool,
        now: u64,
    ) -> io::Result<()> {
        self.note_subscription(provider, active, now);
        self.save()
    }

    /// As [`Store::subscription`], kept in memory until the manifest is
    /// next saved.
    pub(crate) fn note_subscription(&mut self, provider: ProviderKind, active: bool, now: u64) {
        let name = service(provider);
        if active {
            self.confirmed.insert(name.clone(), now);
            lock(&self.seen).insert(name, now);
        } else {
            self.note_reading(now);
            self.confirmed.remove(&name);
            lock(&self.seen).remove(&name);
        }
    }

    /// Note that the clock reads `now`, and save the manifest with the
    /// latest time the store has seen, so that it judges no earlier after a
    /// restart, a clock set back meanwhile.
    ///
    /// # Errors
    ///
    /// The manifest can't be saved.
    pub(crate) fn note_time(&self, now: u64) -> io::Result<()> {
        self.note_reading(now);
        self.save()
    }
}

/// `mutex` locked; a thread that panicked holding it left a map whose every
/// state is whole, so its poison is ignored.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The names of the files in `dir`; those that aren't UTF-8, none of the
/// store's, are left out.
fn file_names(dir: &Path) -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        // A listing cut short could miss a manifest put aside.
        if let Ok(name) = entry?.file_name().into_string() {
            names.push(name);
        }
    }
    Ok(names)
}

/// A song being written, sealed, into the store.
pub struct Pending {
    name: String,
    part: PathBuf,
    writer: sealed::Writer<File>,
    len: u64,
}

impl Pending {
    /// Seal the end, make it last on the disk, and name it as kept.
    fn finish(self, dir: &Path) -> io::Result<(String, u64)> {
        let file = self.writer.finish()?;
        file.sync_all()?;
        let name = format!("{}.sealed", self.name);
        std::fs::rename(&self.part, dir.join(&name))?;
        Ok((name, self.len))
    }

    /// Give up on it: the file goes.
    pub fn abandon(self) {
        let _ = std::fs::remove_file(&self.part);
    }
}

impl Write for Pending {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.len += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn track(provider: ProviderKind, id: &str) -> Track {
        Track {
            id: ItemId::new(provider, id),
            title: format!("Song {id}"),
            artists: vec!["Someone".into()],
            album: None,
            album_id: None,
            duration: None,
            track_number: None,
            disc_number: None,
            artwork: None,
            explicit: false,
            quality: None,
            in_library: true,
            library_id: None,
        }
    }

    fn song(provider: ProviderKind, id: &str, protected: bool) -> Song {
        Song {
            track: track(provider, id),
            extension: "m4a".into(),
            priming: 2112,
            protected,
            kbps: Some(256),
            spatial: false,
        }
    }

    struct Dir(PathBuf);
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn dir() -> Dir {
        Dir(std::env::temp_dir().join(format!("downloads-{:016x}", rand::random::<u64>())))
    }

    fn keep(store: &mut Store, song: Song, data: &[u8], now: u64) {
        let mut pending = store.begin().unwrap();
        pending.write_all(data).unwrap();
        store.keep(pending, song, now).unwrap();
    }

    #[test]
    fn keeps_a_song_and_plays_it_after_reopening() {
        let dir = dir();
        let key = sealed::new_key();
        let id = ItemId::new(ProviderKind::Navidrome, "1");
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            keep(
                &mut store,
                song(ProviderKind::Navidrome, "1", false),
                b"music",
                10,
            );
        }
        let store = Store::open(&dir.0, key).unwrap();
        let mut open = store.open_song(&id, 99_999_999).unwrap().unwrap();
        assert_eq!(open.len, 5);
        assert_eq!(open.priming, 2112);
        let mut back = Vec::new();
        open.reader.read_to_end(&mut back).unwrap();
        assert_eq!(back, b"music");
        assert!(
            store
                .open_song(&ItemId::new(ProviderKind::Navidrome, "2"), 0)
                .is_none()
        );
    }

    #[test]
    fn a_protected_song_plays_while_the_subscription_is_confirmed() {
        let dir = dir();
        let mut store = Store::open(&dir.0, sealed::new_key()).unwrap();
        let id = ItemId::new(ProviderKind::AppleMusic, "1");
        keep(
            &mut store,
            song(ProviderKind::AppleMusic, "1", true),
            b"x",
            100,
        );
        let item = store.get(&id).unwrap().clone();
        assert_eq!(
            store.standing(&item, 100),
            Standing::PlaysUntil(100 + PROTECTED_FOR)
        );
        assert!(store.open_song(&id, 100 + PROTECTED_FOR - 1).is_some());
        assert!(store.open_song(&id, 100 + PROTECTED_FOR).is_none());
        // Confirmed again later: 30 more days from then.
        store
            .subscription(ProviderKind::AppleMusic, true, 1_000)
            .unwrap();
        assert!(store.open_song(&id, 100 + PROTECTED_FOR).is_some());
        // Ended: no longer.
        store
            .subscription(ProviderKind::AppleMusic, false, 2_000)
            .unwrap();
        assert_eq!(store.standing(&item, 2_000), Standing::Expired);
        // The song is still kept, to play again once subscribed.
        assert!(store.get(&id).is_some());
    }

    #[test]
    fn an_unprotected_song_always_plays() {
        let dir = dir();
        let mut store = Store::open(&dir.0, sealed::new_key()).unwrap();
        keep(
            &mut store,
            song(ProviderKind::Navidrome, "1", false),
            b"x",
            0,
        );
        let item = store.items().next().unwrap().clone();
        assert_eq!(store.standing(&item, u64::MAX / 2), Standing::Plays);
    }

    #[test]
    fn removing_takes_the_file_away() {
        let dir = dir();
        let mut store = Store::open(&dir.0, sealed::new_key()).unwrap();
        keep(
            &mut store,
            song(ProviderKind::AppleMusic, "1", true),
            b"a",
            0,
        );
        keep(
            &mut store,
            song(ProviderKind::Navidrome, "2", false),
            b"b",
            0,
        );
        let files = || {
            std::fs::read_dir(&dir.0)
                .unwrap()
                .filter(|e| e.as_ref().unwrap().path().extension() == Some("sealed".as_ref()))
                .count()
        };
        assert_eq!(files(), 2);
        store
            .remove(&ItemId::new(ProviderKind::AppleMusic, "1"))
            .unwrap();
        assert_eq!(files(), 1);
        store
            .remove(&ItemId::new(ProviderKind::Navidrome, "2"))
            .unwrap();
        assert_eq!(files(), 0);
        assert_eq!(store.items().count(), 0);
    }

    #[test]
    fn a_download_cut_short_is_swept_away() {
        let dir = dir();
        let key = sealed::new_key();
        let store = Store::open(&dir.0, key).unwrap();
        let mut pending = store.begin().unwrap();
        pending.write_all(b"half").unwrap();
        drop(pending);
        drop(store);
        Store::open(&dir.0, key).unwrap();
        assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 0);
    }

    #[test]
    fn a_damaged_manifest_is_put_aside() {
        let dir = dir();
        std::fs::create_dir_all(&dir.0).unwrap();
        std::fs::write(dir.0.join(MANIFEST), b"{ not json").unwrap();
        let store = Store::open(&dir.0, sealed::new_key()).unwrap();
        assert_eq!(store.items().count(), 0);
        assert!(dir.0.join("manifest.corrupt.json").exists());
    }

    #[test]
    fn the_folder_is_the_users_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = dir();
        Store::open(&dir.0, sealed::new_key()).unwrap();
        let mode = std::fs::metadata(&dir.0).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
    }
    #[test]
    fn a_manifest_that_doesnt_read_leaves_the_songs_on_disk() {
        let dir = dir();
        let key = sealed::new_key();
        let id = ItemId::new(ProviderKind::Navidrome, "1");
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            keep(
                &mut store,
                song(ProviderKind::Navidrome, "1", false),
                b"music",
                10,
            );
        }
        // As a later version's manifest would read here, or a damaged one.
        std::fs::write(dir.0.join(MANIFEST), br#"{"version":99,"items":[]}"#).unwrap();
        let store = Store::open(&dir.0, key).unwrap();
        assert!(store.get(&id).is_none(), "it says nothing is kept");
        assert!(dir.0.join("manifest.corrupt.json").exists());
        let sealed: Vec<_> = std::fs::read_dir(&dir.0)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".sealed"))
            .collect();
        assert_eq!(sealed.len(), 1, "the song's file is still there");
    }

    #[test]
    fn a_song_sealed_under_another_key_doesnt_open() {
        let dir = dir();
        let id = ItemId::new(ProviderKind::Navidrome, "1");
        {
            let mut store = Store::open(&dir.0, sealed::new_key()).unwrap();
            keep(
                &mut store,
                song(ProviderKind::Navidrome, "1", false),
                &vec![7u8; 200_000],
                10,
            );
        }
        // The keyring remade: the key is another, the files the same.
        let store = Store::open(&dir.0, sealed::new_key()).unwrap();
        assert!(
            store.open_song(&id, 20).is_some_and(|r| r.is_err()),
            "it fails to open, so the song plays from its service"
        );
    }

    // The stage 1 audit's L1 (a manifest put aside) and S5 (a clock set back).

    fn sealed_files(dir: &Path) -> usize {
        std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".sealed"))
            .count()
    }

    #[test]
    fn a_manifest_put_aside_keeps_its_songs_through_the_next_starts() {
        let dir = dir();
        let key = sealed::new_key();
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            keep(
                &mut store,
                song(ProviderKind::Navidrome, "1", false),
                b"music",
                10,
            );
        }
        std::fs::write(dir.0.join(MANIFEST), b"{ damaged").unwrap();
        {
            // The next start: put aside, then anything saved, as a confirmed subscription is
            // (Apple's at each connection on the desktop, each service's on the phone); a
            // SoundCloud confirmation stands in for it here.
            let mut store = Store::open(&dir.0, key).unwrap();
            store
                .subscription(ProviderKind::SoundCloud, true, 20)
                .unwrap();
        }
        // The start after.
        Store::open(&dir.0, key).unwrap();
        assert_eq!(sealed_files(&dir.0), 1, "the song was swept");
    }

    #[test]
    fn a_clock_set_back_doesnt_bring_an_expired_song_back() {
        let dir = dir();
        let mut store = Store::open(&dir.0, sealed::new_key()).unwrap();
        let confirmed = 1_800_000_000;
        keep(
            &mut store,
            song(ProviderKind::AppleMusic, "1", true),
            b"x",
            confirmed,
        );
        let id = ItemId::new(ProviderKind::AppleMusic, "1");
        let expired = confirmed + PROTECTED_FOR;
        assert!(store.open_song(&id, expired).is_none());
        // Offline, the clock set back a year.
        assert!(store.open_song(&id, expired - 365 * 24 * 60 * 60).is_none());
    }

    // The stage 1 audit's L1 (a manifest put aside) and L8 (a song kept again).

    /// ADR-0017: a manifest that doesn't read is put aside "without
    /// sweeping away the files it no longer names", at the start that finds
    /// it and at every start after.
    #[test]
    fn songs_of_a_manifest_put_aside_are_still_there_at_the_next_start() {
        let dir = dir();
        let key = sealed::new_key();
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            keep(
                &mut store,
                song(ProviderKind::Navidrome, "1", false),
                b"music",
                10,
            );
        }
        std::fs::write(dir.0.join(MANIFEST), b"{ damaged").unwrap();
        drop(Store::open(&dir.0, key).unwrap());
        drop(Store::open(&dir.0, key).unwrap());
        assert_eq!(sealed_files(&dir.0), 1, "the song's file is still there");
    }

    /// The same when, in between, a service confirmed its subscription
    /// (each Apple Music connection does), which writes the manifest anew.
    #[test]
    fn songs_of_a_manifest_put_aside_stay_once_a_subscription_is_confirmed() {
        let dir = dir();
        let key = sealed::new_key();
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            keep(
                &mut store,
                song(ProviderKind::Navidrome, "1", false),
                b"music",
                10,
            );
        }
        // A later version's, whole: it reaches the version check.
        std::fs::write(
            dir.0.join(MANIFEST),
            br#"{"version":99,"items":[],"confirmed":{}}"#,
        )
        .unwrap();
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            store
                .subscription(ProviderKind::AppleMusic, true, 20)
                .unwrap();
        }
        drop(Store::open(&dir.0, key).unwrap());
        assert_eq!(sealed_files(&dir.0), 1, "the song's file is still there");
    }

    /// A song kept again as the same track: when the manifest can't be
    /// written, the copy kept before still plays at the next start.
    #[test]
    fn a_song_kept_again_still_plays_when_the_manifest_cant_be_written() {
        let dir = dir();
        let key = sealed::new_key();
        let id = ItemId::new(ProviderKind::Navidrome, "1");
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            keep(
                &mut store,
                song(ProviderKind::Navidrome, "1", false),
                b"first copy",
                10,
            );
            // As a full disk or an I/O error would: a folder stands where
            // the manifest's new copy goes.
            std::fs::create_dir(dir.0.join("manifest.json.part")).unwrap();
            let mut pending = store.begin().unwrap();
            pending.write_all(b"second copy").unwrap();
            assert!(
                store
                    .keep(pending, song(ProviderKind::Navidrome, "1", false), 20)
                    .is_err()
            );
        }
        std::fs::remove_dir(dir.0.join("manifest.json.part")).unwrap();
        let store = Store::open(&dir.0, key).unwrap();
        assert!(
            store.open_song(&id, 30).is_some_and(|r| r.is_ok()),
            "one copy of the song still opens"
        );
    }

    #[test]
    fn songs_found_without_a_manifest_stay_through_the_next_starts() {
        let dir = dir();
        let key = sealed::new_key();
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            keep(
                &mut store,
                song(ProviderKind::Navidrome, "1", false),
                b"music",
                10,
            );
        }
        std::fs::remove_file(dir.0.join(MANIFEST)).unwrap();
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            assert_eq!(sealed_files(&dir.0), 1, "left alone at the start");
            // A save writes a manifest naming none of them.
            store
                .subscription(ProviderKind::AppleMusic, true, 20)
                .unwrap();
        }
        drop(Store::open(&dir.0, key).unwrap());
        assert_eq!(sealed_files(&dir.0), 1, "left alone at the start after");
    }

    #[test]
    fn a_manifest_put_aside_twice_keeps_both_copies() {
        let dir = dir();
        let key = sealed::new_key();
        std::fs::create_dir_all(&dir.0).unwrap();
        std::fs::write(dir.0.join(MANIFEST), b"{ first").unwrap();
        drop(Store::open(&dir.0, key).unwrap());
        std::fs::write(dir.0.join(MANIFEST), b"{ second").unwrap();
        drop(Store::open(&dir.0, key).unwrap());
        assert_eq!(std::fs::read(dir.0.join(PUT_ASIDE)).unwrap(), b"{ first");
        assert_eq!(
            std::fs::read(dir.0.join(format!("{PUT_ASIDE}.1"))).unwrap(),
            b"{ second"
        );
    }

    #[test]
    fn a_song_that_cant_be_kept_leaves_no_file() {
        let dir = dir();
        let mut store = Store::open(&dir.0, sealed::new_key()).unwrap();
        keep(
            &mut store,
            song(ProviderKind::Navidrome, "1", false),
            b"first copy",
            10,
        );
        std::fs::create_dir(dir.0.join("manifest.json.part")).unwrap();
        for id in ["1", "2"] {
            let mut pending = store.begin().unwrap();
            pending.write_all(b"again").unwrap();
            let kept = store.keep(pending, song(ProviderKind::Navidrome, id, false), 20);
            assert!(kept.is_err());
        }
        assert_eq!(sealed_files(&dir.0), 1, "the first copy's only");
        assert_eq!(
            store
                .get(&ItemId::new(ProviderKind::Navidrome, "1"))
                .unwrap()
                .added,
            10,
            "the store names the copy kept before"
        );
        assert!(
            store
                .get(&ItemId::new(ProviderKind::Navidrome, "2"))
                .is_none()
        );
    }

    #[test]
    fn a_clock_set_back_before_a_restart_doesnt_bring_an_expired_song_back() {
        let dir = dir();
        let key = sealed::new_key();
        let confirmed = 1_800_000_000;
        let id = ItemId::new(ProviderKind::AppleMusic, "1");
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            keep(
                &mut store,
                song(ProviderKind::AppleMusic, "1", true),
                b"x",
                confirmed,
            );
            store.note_time(confirmed + PROTECTED_FOR).unwrap();
        }
        let store = Store::open(&dir.0, key).unwrap();
        assert!(store.open_song(&id, confirmed + 1).is_none());
    }

    const DAY: u64 = 24 * 60 * 60;

    /// A confirmation trusts the clock it is given, set back or not: the 30
    /// days count from it.
    #[test]
    fn a_subscription_confirmed_with_the_clock_set_back_counts_from_its_own_time() {
        let dir = dir();
        let mut store = Store::open(&dir.0, sealed::new_key()).unwrap();
        let confirmed = 1_800_000_000;
        keep(
            &mut store,
            song(ProviderKind::AppleMusic, "1", true),
            b"x",
            confirmed,
        );
        let item = store.items().next().unwrap().clone();
        let expired = confirmed + PROTECTED_FOR;
        assert_eq!(store.standing(&item, expired), Standing::Expired);
        // Online again, the clock a day behind: confirmed as of that day.
        let back = expired - DAY;
        store
            .subscription(ProviderKind::AppleMusic, true, back)
            .unwrap();
        assert_eq!(
            store.standing(&item, back),
            Standing::PlaysUntil(back + PROTECTED_FOR)
        );
        assert_eq!(
            store.standing(&item, back + PROTECTED_FOR),
            Standing::Expired
        );
    }

    /// One far-future clock reading (a bad clock, a mistyped date) must not
    /// let protected songs play offline for good once confirmed again.
    #[test]
    fn one_far_future_reading_poisons_the_store() {
        let dir = dir();
        let key = sealed::new_key();
        let t = 1_800_000_000;
        let mut store = Store::open(&dir.0, key).unwrap();
        keep(
            &mut store,
            song(ProviderKind::AppleMusic, "1", true),
            b"x",
            t,
        );
        let item = store.items().next().unwrap().clone();
        // The clock reads 2100 once.
        let _ = store.standing(&item, 4_100_000_000);
        // Online the next day, the clock right again: confirmed.
        store
            .subscription(ProviderKind::AppleMusic, true, t + DAY)
            .unwrap();
        drop(store);
        // A year later, offline, after a restart.
        let store = Store::open(&dir.0, key).unwrap();
        const { assert!(PROTECTED_FOR < 365 * DAY) };
        assert_eq!(
            store.standing(&item, t + 365 * DAY),
            Standing::Expired,
            "plays a year after its last confirmation"
        );
    }

    /// One service's confirmation leaves another's time seen alone: a
    /// SoundCloud sign-in, with the clock set back and Apple out of reach,
    /// doesn't bring Apple's expired songs back.
    #[test]
    fn one_services_confirmation_doesnt_bring_anothers_expired_songs_back() {
        let dir = dir();
        let key = sealed::new_key();
        let t = 1_800_000_000;
        let mut store = Store::open(&dir.0, key).unwrap();
        keep(
            &mut store,
            song(ProviderKind::AppleMusic, "1", true),
            b"x",
            t,
        );
        let item = store.items().next().unwrap().clone();
        assert_eq!(store.standing(&item, t + 40 * DAY), Standing::Expired);
        drop(store);
        let mut store = Store::open(&dir.0, key).unwrap();
        store
            .subscription(ProviderKind::SoundCloud, true, t + DAY)
            .unwrap();
        assert_eq!(store.standing(&item, t + DAY), Standing::Expired);
        drop(store);
        let store = Store::open(&dir.0, key).unwrap();
        assert_eq!(store.standing(&item, t + 2 * DAY), Standing::Expired);
    }

    /// A confirmation made with the clock far ahead doesn't hold once the
    /// clock is right again; the next confirmation does.
    #[test]
    fn a_confirmation_made_with_the_clock_far_ahead_doesnt_hold() {
        let dir = dir();
        let key = sealed::new_key();
        let t = 1_800_000_000;
        let ahead = 4_100_000_000;
        let mut store = Store::open(&dir.0, key).unwrap();
        keep(
            &mut store,
            song(ProviderKind::AppleMusic, "1", true),
            b"x",
            t,
        );
        let item = store.items().next().unwrap().clone();
        store
            .subscription(ProviderKind::AppleMusic, true, ahead)
            .unwrap();
        drop(store);
        let mut store = Store::open(&dir.0, key).unwrap();
        assert_eq!(store.standing(&item, t + 365 * DAY), Standing::Expired);
        // A clock put right within a day of the confirmation is forgiven.
        assert_eq!(
            store.standing(&item, ahead - DAY + 1),
            Standing::PlaysUntil(ahead + PROTECTED_FOR)
        );
        store
            .subscription(ProviderKind::AppleMusic, true, t + 365 * DAY)
            .unwrap();
        assert_eq!(
            store.standing(&item, t + 366 * DAY),
            Standing::PlaysUntil(t + 365 * DAY + PROTECTED_FOR)
        );
    }

    /// Named for the defect it guards against: an expiry judged during a
    /// session reaches the disk, so that a restart with the clock set back
    /// still finds the song `Expired`.
    #[test]
    fn an_expiry_seen_before_a_restart_is_forgotten() {
        let dir = dir();
        let key = sealed::new_key();
        let t = 1_800_000_000;
        let mut store = Store::open(&dir.0, key).unwrap();
        keep(
            &mut store,
            song(ProviderKind::AppleMusic, "1", true),
            b"x",
            t,
        );
        let item = store.items().next().unwrap().clone();
        assert_eq!(store.standing(&item, t + 31 * DAY), Standing::Expired);
        drop(store);
        // Quit, clock set back to day 20, started again.
        let store = Store::open(&dir.0, key).unwrap();
        assert_eq!(
            store.standing(&item, t + 20 * DAY),
            Standing::Expired,
            "plays again after a restart"
        );
    }

    #[test]
    fn songs_of_a_manifest_put_aside_stay_once_the_copy_put_aside_is_deleted() {
        let dir = dir();
        let key = sealed::new_key();
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            keep(
                &mut store,
                song(ProviderKind::Navidrome, "1", false),
                b"music",
                10,
            );
        }
        std::fs::write(dir.0.join(MANIFEST), b"{ damaged").unwrap();
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            store
                .subscription(ProviderKind::AppleMusic, true, 20)
                .unwrap();
        }
        // The user, or a cleaning tool, deletes the copy put aside.
        std::fs::remove_file(dir.0.join(PUT_ASIDE)).unwrap();
        drop(Store::open(&dir.0, key).unwrap());
        assert_eq!(sealed_files(&dir.0), 1, "the song's file is still there");
    }

    /// As an older build leaves the folder: its manifest put aside, then one
    /// naming none of the songs written whole.
    #[test]
    fn songs_beside_a_manifest_put_aside_by_an_older_build_stay() {
        let dir = dir();
        let key = sealed::new_key();
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            keep(
                &mut store,
                song(ProviderKind::Navidrome, "1", false),
                b"music",
                10,
            );
        }
        std::fs::write(dir.0.join(PUT_ASIDE), b"{ damaged").unwrap();
        std::fs::write(
            dir.0.join(MANIFEST),
            br#"{"version":1,"items":[],"confirmed":{}}"#,
        )
        .unwrap();
        drop(Store::open(&dir.0, key).unwrap());
        assert_eq!(sealed_files(&dir.0), 1, "the song's file is still there");
    }

    #[test]
    fn a_manifest_from_an_older_build_reads_whole() {
        let dir = dir();
        let key = sealed::new_key();
        let id = ItemId::new(ProviderKind::Navidrome, "1");
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            keep(
                &mut store,
                song(ProviderKind::Navidrome, "1", false),
                b"music",
                10,
            );
        }
        // As written before the latest time seen and the unnamed songs.
        let path = dir.0.join(MANIFEST);
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let fields = manifest.as_object_mut().unwrap();
        assert!(fields.remove("seen").is_some());
        assert!(fields.remove("unnamed_songs").is_some());
        std::fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let store = Store::open(&dir.0, key).unwrap();
        assert!(store.open_song(&id, 20).is_some_and(|r| r.is_ok()));
        assert!(!dir.0.join(PUT_ASIDE).exists());
    }
}
