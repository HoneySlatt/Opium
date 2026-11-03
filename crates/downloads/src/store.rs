//! Where downloaded songs are kept, and the list of them: each song a
//! sealed file of an opaque name, the list a manifest beside them, written
//! whole each time it changes.
//!
//! A song that came through a service's DRM plays only while that service's
//! subscription was confirmed in the last [`PROTECTED_FOR`]: each time the
//! app is online it is confirmed again, and once the subscription ends, or
//! the service is signed out of, its songs stop playing.

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use music_core::model::{ItemId, ProviderKind, Track};
use serde::{Deserialize, Serialize};

use crate::sealed::{self, Key};

/// How long a song that came through DRM plays without the subscription
/// being confirmed again, in seconds: 30 days, as Apple Music has it.
pub const PROTECTED_FOR: u64 = 30 * 24 * 60 * 60;

const MANIFEST: &str = "manifest.json";
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

#[derive(Default, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    items: Vec<Item>,
    /// When each service last confirmed its subscription, in seconds since
    /// the epoch, by its name.
    confirmed: BTreeMap<String, u64>,
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
    /// The store in `dir`, made if missing. A manifest that doesn't read is
    /// kept aside and the store starts empty; files the manifest doesn't
    /// name (a download cut short) go.
    pub fn open(dir: &Path, key: Key) -> io::Result<Self> {
        private_dir(dir)?;
        let path = dir.join(MANIFEST);
        let manifest = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Manifest>(&bytes) {
                Ok(m) => m,
                Err(_) => {
                    std::fs::rename(&path, dir.join("manifest.corrupt.json"))?;
                    Manifest::default()
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => Manifest::default(),
            Err(e) => return Err(e),
        };
        let store = Self {
            dir: dir.to_owned(),
            key,
            items: manifest
                .items
                .into_iter()
                .map(|item| (item.track.id.clone(), item))
                .collect(),
            confirmed: manifest.confirmed,
        };
        store.sweep();
        Ok(store)
    }

    /// Take away the files no song is kept in.
    fn sweep(&self) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        let kept: std::collections::HashSet<&str> =
            self.items.values().map(|i| i.file.as_str()).collect();
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let song = name.ends_with(".sealed") || name.ends_with(".part");
            if song && !kept.contains(name) {
                let _ = std::fs::remove_file(entry.path());
            }
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
        }
        let json = serde_json::to_vec(&Out {
            version: VERSION,
            items,
            confirmed: &self.confirmed,
        })
        .map_err(io::Error::other)?;
        let part = self.dir.join("manifest.json.part");
        let mut file = File::create(&part)?;
        file.write_all(&json)?;
        file.sync_all()?;
        std::fs::rename(part, self.dir.join(MANIFEST))
    }

    pub fn get(&self, id: &ItemId) -> Option<&Item> {
        self.items.get(id)
    }

    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.items.values()
    }

    pub fn standing(&self, item: &Item, now: u64) -> Standing {
        if !item.protected {
            return Standing::Plays;
        }
        match self.confirmed.get(&service(item.track.id.provider)) {
            Some(at) if now < at + PROTECTED_FOR => Standing::PlaysUntil(at + PROTECTED_FOR),
            _ => Standing::Expired,
        }
    }

    /// The song kept as `id`, opened, if it is kept and plays.
    pub fn open_song(&self, id: &ItemId, now: u64) -> Option<io::Result<Open>> {
        let item = self.items.get(id)?;
        if self.standing(item, now) == Standing::Expired {
            return None;
        }
        Some((|| {
            let file = File::open(self.dir.join(&item.file))?;
            let reader = sealed::Reader::new(file, &self.key)?;
            if reader.len() != item.len {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "the kept song isn't whole",
                ));
            }
            Ok(Open {
                len: reader.len(),
                reader,
                extension: item.extension.clone(),
                priming: item.priming,
                kbps: item.kbps,
                spatial: item.spatial,
            })
        })())
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

    /// Keep the song written into `pending`, in place of one kept before as
    /// the same track. A song that came through DRM confirms its service's
    /// subscription: the service just licensed it.
    pub fn keep(&mut self, pending: Pending, song: Song, now: u64) -> io::Result<&Item> {
        let (file, len) = pending.finish(&self.dir)?;
        if song.protected {
            self.confirmed.insert(service(song.track.id.provider), now);
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
        if let Some(old) = self.items.insert(id.clone(), item) {
            let _ = std::fs::remove_file(self.dir.join(old.file));
        }
        self.save()?;
        Ok(&self.items[&id])
    }

    pub fn remove(&mut self, id: &ItemId) -> io::Result<()> {
        if let Some(item) = self.items.remove(id) {
            self.save()?;
            let _ = std::fs::remove_file(self.dir.join(item.file));
        }
        Ok(())
    }

    /// Every song of `provider` gone: it was signed out of.
    pub fn remove_service(&mut self, provider: ProviderKind) -> io::Result<()> {
        let gone: Vec<ItemId> = self
            .items
            .keys()
            .filter(|id| id.provider == provider)
            .cloned()
            .collect();
        let files: Vec<String> = gone
            .iter()
            .filter_map(|id| self.items.remove(id))
            .map(|i| i.file)
            .collect();
        self.confirmed.remove(&service(provider));
        self.save()?;
        for file in files {
            let _ = std::fs::remove_file(self.dir.join(file));
        }
        Ok(())
    }

    /// `provider` said whether its subscription is active, at `now`: its
    /// protected songs play for [`PROTECTED_FOR`] more, or no longer.
    pub fn subscription(
        &mut self,
        provider: ProviderKind,
        active: bool,
        now: u64,
    ) -> io::Result<()> {
        if active {
            self.confirmed.insert(service(provider), now);
        } else {
            self.confirmed.remove(&service(provider));
        }
        self.save()
    }
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
        store.remove_service(ProviderKind::AppleMusic).unwrap();
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
}
