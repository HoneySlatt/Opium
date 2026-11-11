//! The folders read: each music file's tags and cover, kept in an index on
//! the disk so a launch shows the library at once, and only the files changed
//! since are read again.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use lofty::file::{AudioFile, FileType, TaggedFileExt};
use lofty::picture::PictureType;
use lofty::tag::{ItemKey, Tag};
use opium_core::files::{Mode, write_whole};
use opium_core::model::Codec;
use serde::{Deserialize, Serialize};

/// The files read as music: what the player decodes.
pub const EXTENSIONS: [&str; 12] = [
    "mp3", "flac", "wav", "aif", "aiff", "aifc", "m4a", "mp4", "aac", "ogg", "oga", "opus",
];

/// Pictures in an album's folder taken as its cover, the first found.
const FOLDER_COVERS: [&str; 4] = ["cover", "folder", "front", "album"];

/// One music file, as its tags say. `path` names it: a path, or the
/// phone's address of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub path: String,
    pub size: u64,
    /// When the file last changed, in seconds: read again once it differs.
    pub modified: u64,
    /// When it joined the library, in seconds.
    pub added: u64,
    pub title: String,
    pub artists: Vec<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub track: Option<u32>,
    pub disc: Option<u32>,
    pub year: Option<u16>,
    pub genre: Option<String>,
    pub duration_ms: u64,
    pub codec: Codec,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub bitrate: Option<u32>,
    /// Its cover, made square in the covers' folder; or an address.
    pub cover: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Index {
    pub entries: Vec<Entry>,
    /// The files that couldn't be read as music, by path, with their size
    /// and when they changed: not read again until they do.
    #[serde(default)]
    pub skipped: Vec<(String, u64, u64)>,
}

impl Index {
    pub fn read(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    /// Write the index to `path`, its folder made if missing: whole or not
    /// at all, even with two writes at once, as a cache, which a scan makes
    /// again ([`write_whole`], [`Mode::Cache`]).
    ///
    /// # Errors
    ///
    /// The folder can't be made, or the index written: `path` is then as it
    /// was.
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        write_whole(path, &serde_json::to_vec(self)?, Mode::Cache)
    }
}

fn secs(time: std::io::Result<SystemTime>) -> u64 {
    time.ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs())
}

/// Whether `path` names a file read as music.
pub fn is_music(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// The folders to walk, each once: one chosen inside another (`/music` and
/// `/music/rock`) would otherwise have its files found twice, each song
/// showing twice and its album counting double.
pub(crate) fn roots(folders: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for folder in folders {
        let folder = folder.canonicalize().unwrap_or_else(|_| folder.clone());
        if roots.iter().any(|kept: &PathBuf| folder.starts_with(kept)) {
            continue;
        }
        roots.retain(|kept| !kept.starts_with(&folder));
        roots.push(folder);
    }
    roots
}

/// The music files under `folders`, with their size and when they changed;
/// hidden folders are left out.
pub(crate) fn walk(folders: &[PathBuf]) -> Vec<(PathBuf, u64, u64)> {
    let mut found = Vec::new();
    let mut todo: Vec<PathBuf> = roots(folders);
    while let Some(dir) = todo.pop() {
        let Ok(items) = std::fs::read_dir(&dir) else {
            continue;
        };
        for item in items.flatten() {
            let path = item.path();
            let hidden = item.file_name().to_string_lossy().starts_with('.');
            let Ok(kind) = item.file_type() else { continue };
            if kind.is_dir() {
                if !hidden {
                    todo.push(path);
                }
            } else if !hidden && is_music(&path) {
                // Through a link, as what it points to.
                let Ok(meta) = std::fs::metadata(&path) else {
                    continue;
                };
                // A name that isn't text can't be told apart from another
                // of its kind once said, so it is left out rather than
                // listed as a song that won't open.
                if meta.is_file() && path.to_str().is_some() {
                    found.push((path, meta.len(), secs(meta.modified())));
                }
            }
        }
    }
    found
}

/// Read `folders` again: the files kept as they were reused, the others
/// read; whether anything changed.
pub fn scan(folders: &[PathBuf], before: &Index, covers: &Path) -> (Index, bool) {
    let covers = Covers::new(covers);
    let known: HashMap<&str, &Entry> = before
        .entries
        .iter()
        .map(|e| (e.path.as_str(), e))
        .collect();
    let skipped_before: HashMap<&str, (u64, u64)> = before
        .skipped
        .iter()
        .map(|(p, size, modified)| (p.as_str(), (*size, *modified)))
        .collect();
    // A folder that can't be read at all — a drive unplugged, a share not
    // mounted — says nothing about its songs, so what was read of it last
    // time stays as it was rather than its half of the library going (and
    // coming back as newly added once the drive returns).
    let away: Vec<PathBuf> = roots(folders)
        .into_iter()
        .filter(|root| std::fs::read_dir(root).is_err())
        .collect();
    let under_away = |path: &str| away.iter().any(|root| Path::new(path).starts_with(root));
    let files = walk(folders);
    let mut kept = Vec::with_capacity(files.len());
    let mut skipped = Vec::new();
    let mut fresh = Vec::new();
    if !away.is_empty() {
        kept.extend(
            before
                .entries
                .iter()
                .filter(|e| under_away(&e.path))
                .cloned(),
        );
        skipped.extend(
            before
                .skipped
                .iter()
                .filter(|(path, _, _)| under_away(path))
                .cloned(),
        );
    }
    for (path, size, modified) in files {
        let name = path.to_string_lossy();
        if skipped_before.get(name.as_ref()) == Some(&(size, modified)) {
            skipped.push((name.into_owned(), size, modified));
            continue;
        }
        match known.get(name.as_ref()) {
            Some(entry) if entry.size == size && entry.modified == modified => {
                kept.push((*entry).clone())
            }
            entry => fresh.push((path, size, modified, entry.map(|e| e.added))),
        }
    }
    // Read on every core: tags are mostly waiting on the disk.
    let next = AtomicUsize::new(0);
    let read = Mutex::new(Vec::with_capacity(fresh.len()));
    let workers = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(8);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((path, size, modified, added)) = fresh.get(i) else {
                        return;
                    };
                    let entry =
                        read_file(path, *size, *modified, added.unwrap_or(*modified), &covers)
                            .ok_or_else(|| (path.to_string_lossy().into_owned(), *size, *modified));
                    read.lock().unwrap_or_else(|e| e.into_inner()).push(entry);
                }
            });
        }
    });
    let mut entries = kept;
    for entry in read.into_inner().unwrap_or_else(|e| e.into_inner()) {
        match entry {
            Ok(entry) => entries.push(entry),
            Err(file) => skipped.push(file),
        }
    }
    folder_covers(&mut entries, &covers);
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let changed = entries != before.entries;
    (Index { entries, skipped }, changed)
}

/// The file at `path` as an entry, its tags read, its cover kept in
/// `covers`; none if it isn't music the player decodes.
pub fn read_file(
    path: &Path,
    size: u64,
    modified: u64,
    added: u64,
    covers: &Covers,
) -> Option<Entry> {
    let file = lofty::read_from_path(path).ok()?;
    let props = file.properties();
    let codec = match file.file_type() {
        FileType::Flac => Codec::Flac,
        FileType::Mpeg => Codec::Mp3,
        FileType::Wav | FileType::Aiff => Codec::Wav,
        FileType::Vorbis => Codec::Vorbis,
        FileType::Opus => Codec::Opus,
        FileType::Aac => Codec::Aac,
        // Apple Lossless keeps its bit depth; AAC has none.
        FileType::Mp4 if props.bit_depth().is_some() => Codec::Alac,
        FileType::Mp4 => Codec::Aac,
        // The rest: the player doesn't decode them.
        _ => return None,
    };
    let tag = file.primary_tag().or_else(|| file.first_tag());
    // What the file says, every tag of it read: see [`tags::tags_of`].
    let said = crate::tags::tags_of(&file);
    let mut artists: Vec<String> = tag
        .map(|t| {
            t.get_strings(ItemKey::TrackArtist)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    if artists.is_empty() {
        artists.extend(Some(said.artist.clone()).filter(|a| !a.is_empty()));
    }
    if artists.is_empty() {
        artists.extend(Some(said.album_artist.clone()).filter(|a| !a.is_empty()));
    }
    let title = Some(said.title.clone())
        .filter(|t| !t.is_empty())
        .or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default();
    Some(Entry {
        path: path.to_string_lossy().into_owned(),
        size,
        modified,
        added,
        title,
        artists,
        album_artist: Some(said.album_artist).filter(|a| !a.is_empty()),
        album: Some(said.album).filter(|a| !a.is_empty()),
        track: said.track,
        disc: said.disc,
        year: said.year,
        genre: Some(said.genre).filter(|g| !g.is_empty()),
        duration_ms: props.duration().as_millis() as u64,
        codec,
        sample_rate: props.sample_rate(),
        bit_depth: props.bit_depth(),
        bitrate: props.audio_bitrate(),
        cover: tag.and_then(|t| embedded_cover(t, covers)),
    })
}

/// The tag's front cover, else its first picture, kept square in `covers`
/// under the picture's own hash: the same picture on every track of an
/// album is made once.
fn embedded_cover(tag: &Tag, covers: &Covers) -> Option<String> {
    let pictures = tag.pictures();
    let picture = pictures
        .iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| pictures.first())?;
    covers.keep(picture.data())
}

/// The covers one scan keeps in their folder, each under its picture's own
/// hash: made once however many workers reach it at once ([`Making`]), and
/// one that can't be made tried once in the scan.
pub struct Covers<'a> {
    folder: &'a Path,
    /// The covers this scan couldn't make, by their path: not tried again
    /// until the next scan, which may find the disk no longer full.
    failed: Mutex<HashSet<PathBuf>>,
    /// Makes a picture a cover: [`opium_core::cover::square`], or a test's
    /// own.
    square: fn(&[u8]) -> Option<Vec<u8>>,
}

impl<'a> Covers<'a> {
    /// The covers kept in `folder`, made when first written to, for one
    /// scan or one file.
    pub fn new(folder: &'a Path) -> Self {
        Self {
            folder,
            failed: Mutex::default(),
            square: opium_core::cover::square,
        }
    }

    /// `bytes`, a picture, made square in the folder; its path.
    ///
    /// Made once: a worker that reaches a cover another is making waits for
    /// it, then takes it as made (the scan reads an album's files at once,
    /// each with the same picture). One these covers couldn't make is not
    /// tried again, by the workers waiting for it nor by those after.
    /// Written whole, as a cache ([`write_whole`], [`Mode::Cache`]), so a
    /// reader never finds half a cover. `None` when the picture doesn't
    /// decode or can't be kept.
    pub fn keep(&self, bytes: &[u8]) -> Option<String> {
        let path = self.folder.join(format!("{:016x}.jpg", fnv(bytes)));
        if !made(&path) && !self.make_once(bytes, &path) {
            return None;
        }
        Some(path.to_string_lossy().into_owned())
    }

    /// Make the cover at `path` from `bytes`, unless another worker made it
    /// meanwhile or these covers failed to; whether it is there.
    fn make_once(&self, bytes: &[u8], path: &Path) -> bool {
        let Some(_making) = Making::claim(path) else {
            return true;
        };
        if self.failed().contains(path) {
            return false;
        }
        let made = (self.square)(bytes).is_some_and(|square| {
            std::fs::create_dir_all(self.folder).is_ok()
                && write_whole(path, &square, Mode::Cache).is_ok()
        });
        if !made {
            // Noted while `_making` is held: the workers waiting for it read
            // this once it goes.
            self.failed().insert(path.to_owned());
        }
        made
    }

    fn failed(&self) -> std::sync::MutexGuard<'_, HashSet<PathBuf>> {
        // A worker that panicked left the set whole: one path in or not.
        self.failed.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Whether the cover at `path` is made: there and not empty, as a power cut
/// can leave a cache written without a flush ([`Mode::Cache`]).
fn made(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|kept| kept.len() > 0)
}

/// The covers being made now, by their path, and the signal that one is
/// done.
static MAKING: (Mutex<Vec<PathBuf>>, Condvar) = (Mutex::new(Vec::new()), Condvar::new());

/// A cover this worker makes: others that reach it wait until it is
/// dropped, made or not.
struct Making<'a>(&'a Path);

impl<'a> Making<'a> {
    /// The cover at `path` to make, once no other worker makes it; `None`
    /// if, by then, it is there.
    fn claim(path: &'a Path) -> Option<Self> {
        let (making, done) = &MAKING;
        let mut making = making.lock().unwrap_or_else(|e| e.into_inner());
        while making.iter().any(|other| other == path) {
            making = done.wait(making).unwrap_or_else(|e| e.into_inner());
        }
        if made(path) {
            return None;
        }
        making.push(path.to_owned());
        Some(Self(path))
    }
}

impl Drop for Making<'_> {
    fn drop(&mut self) {
        let (making, done) = &MAKING;
        making
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|other| other != self.0);
        done.notify_all();
    }
}

/// The files with no picture of their own take their folder's: `cover.jpg`,
/// `folder.png` and the like.
fn folder_covers(entries: &mut [Entry], covers: &Covers) {
    let mut found: HashMap<PathBuf, Option<String>> = HashMap::new();
    for entry in entries.iter_mut().filter(|e| e.cover.is_none()) {
        let Some(dir) = Path::new(&entry.path).parent() else {
            continue;
        };
        entry.cover = found
            .entry(dir.to_owned())
            .or_insert_with(|| folder_cover(dir, covers))
            .clone();
    }
}

fn folder_cover(dir: &Path, covers: &Covers) -> Option<String> {
    let pictures = std::fs::read_dir(dir).ok()?.flatten().filter_map(|item| {
        let path = item.path();
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        let stem = path.file_stem()?.to_str()?.to_ascii_lowercase();
        (matches!(ext.as_str(), "jpg" | "jpeg" | "png")).then_some((stem, path))
    });
    let mut best: Option<(usize, PathBuf)> = None;
    for (stem, path) in pictures {
        if let Some(rank) = FOLDER_COVERS.iter().position(|name| stem == *name)
            && best.as_ref().is_none_or(|(r, _)| rank < *r)
        {
            best = Some((rank, path));
        }
    }
    let (_, path) = best?;
    covers.keep(&std::fs::read(path).ok()?)
}

/// FNV-1a, 64 bits.
pub fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ *b as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn music_files_are_known_by_their_extension_in_any_case() {
        assert!(is_music(Path::new("/a/b/Song.FLAC")));
        assert!(is_music(Path::new("/a/b/song.m4a")));
        assert!(!is_music(Path::new("/a/b/cover.jpg")));
        assert!(!is_music(Path::new("/a/b/notes")));
    }

    /// A second of 16-bit stereo silence at 44.1 kHz, as a WAV file.
    pub(crate) fn wav() -> Vec<u8> {
        let data = 44_100 * 4;
        let mut bytes = Vec::new();
        bytes.extend(b"RIFF");
        bytes.extend((36 + data as u32).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(44_100u32.to_le_bytes());
        bytes.extend((44_100u32 * 4).to_le_bytes());
        bytes.extend(4u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend((data as u32).to_le_bytes());
        bytes.resize(bytes.len() + data, 0);
        bytes
    }

    /// Ten frames of MPEG-1 Layer III at 44.1 kHz, 128 kbps: enough of an
    /// MP3 for the tags to be read and written.
    pub(crate) fn mp3() -> Vec<u8> {
        let mut bytes = Vec::new();
        for _ in 0..10 {
            bytes.extend([0xFF, 0xFB, 0x90, 0x00]);
            bytes.resize(bytes.len() + 417 - 4, 0);
        }
        bytes
    }

    #[test]
    fn a_file_read_once_is_kept_until_it_changes() {
        let dir = std::env::temp_dir().join(format!("local-scan-{}", std::process::id()));
        let music = dir.join("music");
        std::fs::create_dir_all(music.join(".hidden")).unwrap();
        std::fs::write(music.join("Song.wav"), wav()).unwrap();
        // Not music the tags can be read from: skipped, and not read again.
        std::fs::write(music.join("broken.mp3"), b"not an mp3").unwrap();
        std::fs::write(music.join(".hidden").join("x.flac"), b"x").unwrap();
        assert_eq!(
            walk(std::slice::from_ref(&music)).len(),
            2,
            "hidden folders are left out"
        );
        let covers = dir.join("covers");
        let (index, changed) = scan(std::slice::from_ref(&music), &Index::default(), &covers);
        assert!(changed);
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.skipped.len(), 1);
        let song = &index.entries[0];
        assert_eq!(song.title, "Song", "untagged: named after the file");
        assert_eq!(song.codec, Codec::Wav);
        assert_eq!((song.sample_rate, song.bit_depth), (Some(44_100), Some(16)));
        assert!((990..=1010).contains(&song.duration_ms));
        let (again, changed) = scan(std::slice::from_ref(&music), &index, &covers);
        assert!(!changed);
        assert_eq!(again.entries, index.entries);
        std::fs::remove_file(music.join("Song.wav")).unwrap();
        let (gone, changed) = scan(std::slice::from_ref(&music), &again, &covers);
        assert!(changed && gone.entries.is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn a_folder_chosen_inside_another_is_walked_once() {
        let dir = std::env::temp_dir().join(format!("local-roots-{}", std::process::id()));
        let music = dir.join("music");
        let rock = music.join("rock");
        std::fs::create_dir_all(&rock).unwrap();
        std::fs::write(rock.join("song.wav"), wav()).unwrap();
        let found = walk(&[music.clone(), rock.clone()]);
        assert_eq!(found.len(), 1, "the song is found once");
        // Whichever way round they were chosen.
        assert_eq!(walk(&[rock, music]).len(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_folder_out_of_reach_keeps_the_songs_it_had() {
        let dir = std::env::temp_dir().join(format!("local-away-{}", std::process::id()));
        let music = dir.join("music");
        let covers = dir.join("covers");
        std::fs::create_dir_all(&music).unwrap();
        std::fs::create_dir_all(&covers).unwrap();
        std::fs::write(music.join("song.wav"), wav()).unwrap();
        let (index, _) = scan(std::slice::from_ref(&music), &Index::default(), &covers);
        assert_eq!(index.entries.len(), 1);
        let added = index.entries[0].added;

        // The drive away: the folder is gone, not emptied.
        std::fs::remove_dir_all(&music).unwrap();
        let (after, changed) = scan(std::slice::from_ref(&music), &index, &covers);
        assert_eq!(after.entries.len(), 1, "its songs stay as they were");
        assert_eq!(
            after.entries[0].added, added,
            "and keep when they were added"
        );
        assert!(!changed);

        // Emptied for real, with the folder there: the song goes.
        std::fs::create_dir_all(&music).unwrap();
        let (empty, changed) = scan(&[music], &index, &covers);
        assert!(empty.entries.is_empty() && changed);
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// `bytes` kept as a cover in `covers` as a lone file's is, in a
    /// [`Covers`] of its own: how the stage 1 audit's tests below reach it.
    fn keep_cover(bytes: &[u8], covers: &Path) -> Option<String> {
        Covers::new(covers).keep(bytes)
    }

    /// A folder of the test's own under the temporary folder, made empty.
    fn own_folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("local-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// How long a claim that should wait is given to return all the same.
    const STILL_WAITING: std::time::Duration = std::time::Duration::from_millis(300);

    #[test]
    fn a_second_claim_waits_for_the_first_then_finds_the_cover_made() {
        let dir = own_folder("claim-made");
        let path = dir.join("cover.jpg");
        let first = Making::claim(&path).expect("nobody makes it yet");
        let (said, heard) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            scope.spawn(|| said.send(Making::claim(&path).is_some()).unwrap());
            assert!(
                heard.recv_timeout(STILL_WAITING).is_err(),
                "the second claim didn't wait for the first"
            );
            std::fs::write(&path, b"made").unwrap();
            drop(first);
            assert!(!heard.recv().unwrap(), "claimed though it was made");
        });
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_second_claim_waits_for_the_first_then_makes_what_it_failed_to() {
        let dir = own_folder("claim-failed");
        let path = dir.join("cover.jpg");
        let first = Making::claim(&path).expect("nobody makes it yet");
        let (said, heard) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            scope.spawn(|| said.send(Making::claim(&path).is_some()).unwrap());
            assert!(
                heard.recv_timeout(STILL_WAITING).is_err(),
                "the second claim didn't wait for the first"
            );
            // Failed: dropped with no cover made.
            drop(first);
            assert!(heard.recv().unwrap(), "not claimed though nobody made it");
        });
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_second_claim_waits_for_the_first_then_makes_what_it_panicked_on() {
        let dir = own_folder("claim-panicked");
        let path = dir.join("cover.jpg");
        let path = path.as_path();
        let (claimed, first_claimed) = std::sync::mpsc::channel();
        let (go, panic_now) = std::sync::mpsc::channel::<()>();
        let (said, heard) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            // Owned here, so that a failed assertion lets the maker go.
            let go = go;
            let first = scope.spawn(move || {
                let _making = Making::claim(path).expect("nobody makes it yet");
                claimed.send(()).unwrap();
                panic_now.recv().unwrap();
                // A panic, without the message a test's would print.
                std::panic::resume_unwind(Box::new("the maker panicked"));
            });
            first_claimed.recv().unwrap();
            scope.spawn(|| said.send(Making::claim(path).is_some()).unwrap());
            assert!(
                heard.recv_timeout(STILL_WAITING).is_err(),
                "the second claim didn't wait for the first"
            );
            go.send(()).unwrap();
            assert!(first.join().is_err());
            assert!(
                heard.recv().unwrap(),
                "not claimed after the maker panicked"
            );
        });
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Eight workers at once, as an album's first files reach the same
    /// picture: what each was told. Each `square` call takes a tenth of a
    /// second, so that every worker reaches the cover while it is made.
    fn eight_at_once(covers: &Covers, bytes: &[u8]) -> Vec<Option<String>> {
        let together = std::sync::Barrier::new(8);
        std::thread::scope(|scope| {
            let workers: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        together.wait();
                        covers.keep(bytes)
                    })
                })
                .collect();
            workers.into_iter().map(|w| w.join().unwrap()).collect()
        })
    }

    #[test]
    fn a_cover_reached_by_eight_workers_at_once_is_made_once() {
        static MADE: AtomicUsize = AtomicUsize::new(0);
        fn square(bytes: &[u8]) -> Option<Vec<u8>> {
            MADE.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(100));
            Some(bytes.to_vec())
        }
        let dir = own_folder("made-once");
        let covers = Covers {
            square,
            ..Covers::new(&dir)
        };
        let told = eight_at_once(&covers, b"a picture");
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(told.iter().all(Option::is_some), "{told:?}");
        assert_eq!(MADE.load(Ordering::SeqCst), 1, "times the cover was made");
    }

    /// A cache is written without a flush: a power cut can leave the cover's
    /// file empty, which the next scan makes again rather than take as made.
    #[test]
    fn a_cover_left_empty_by_a_power_cut_is_made_again() {
        fn square(bytes: &[u8]) -> Option<Vec<u8>> {
            Some(bytes.to_vec())
        }
        let dir = own_folder("left-empty");
        let covers = Covers {
            square,
            ..Covers::new(&dir)
        };
        let path = covers.keep(b"a picture").expect("made");
        std::fs::write(&path, b"").unwrap();
        let again = Covers {
            square,
            ..Covers::new(&dir)
        }
        .keep(b"a picture");
        let kept = std::fs::read(&path).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(again.as_deref(), Some(path.as_str()));
        assert_eq!(kept, b"a picture", "the cover made again");
    }

    #[test]
    fn a_cover_that_cant_be_made_is_tried_once_in_a_scan() {
        static TRIED: AtomicUsize = AtomicUsize::new(0);
        fn square(_: &[u8]) -> Option<Vec<u8>> {
            TRIED.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(100));
            None
        }
        let dir = own_folder("failed-once");
        let covers = Covers {
            square,
            ..Covers::new(&dir)
        };
        let told = eight_at_once(&covers, b"not a picture");
        // The album's later files, in the same scan.
        let later: Vec<_> = (0..4).map(|_| covers.keep(b"not a picture")).collect();
        let in_this_scan = TRIED.load(Ordering::SeqCst);
        // The next scan tries again.
        let next_scan = Covers {
            square,
            ..Covers::new(&dir)
        };
        let again = next_scan.keep(b"not a picture");
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(
            told.iter()
                .chain(&later)
                .chain([&again])
                .all(Option::is_none)
        );
        assert_eq!(in_this_scan, 1, "times this scan tried the picture");
        assert_eq!(TRIED.load(Ordering::SeqCst), 2, "the next scan didn't try");
    }

    // Stage 1 audit, W11: a cover made by eight threads at once left some songs without it.
    #[test]
    fn the_same_cover_kept_by_eight_threads_at_once_is_kept_for_each() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_fn(640, 640, |x, y| image::Rgb([x as u8, y as u8, 90]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let png = png.into_inner();
        let mut without = 0;
        for round in 0..20 {
            let covers = std::env::temp_dir()
                .join(format!("local-covers-race-{}-{round}", std::process::id()));
            let start = std::sync::Barrier::new(8);
            let kept: Vec<Option<String>> = std::thread::scope(|s| {
                let handles: Vec<_> = (0..8)
                    .map(|_| {
                        s.spawn(|| {
                            start.wait();
                            keep_cover(&png, &covers)
                        })
                    })
                    .collect();
                handles.into_iter().map(|h| h.join().unwrap()).collect()
            });
            without += kept.iter().filter(|k| k.is_none()).count();
            let _ = std::fs::remove_dir_all(&covers);
        }
        assert_eq!(without, 0, "tracks left without their cover");
    }

    // Stage 1 audit, W11: an album's cover was made by every worker at once, and some lost it.
    /// An album's cover, embedded in each of its files, reached by eight of the scan's workers at
    /// once: each file is told where the cover is kept.
    #[test]
    fn an_albums_cover_reached_by_every_worker_at_once_is_kept_for_each_file() {
        let dir = std::env::temp_dir().join(format!("local-cover-once-{}", std::process::id()));
        // As covers come: 1200 px, so that making it square takes as long as it does for real.
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_fn(1200, 1200, |x, y| image::Rgb([x as u8, y as u8, 90]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let picture = png.into_inner();
        let mut without = 0;
        for round in 0..10 {
            let covers = dir.join(format!("covers-{round}"));
            let together = std::sync::Barrier::new(8);
            let kept: Vec<Option<String>> = std::thread::scope(|scope| {
                let workers: Vec<_> = (0..8)
                    .map(|_| {
                        scope.spawn(|| {
                            together.wait();
                            keep_cover(&picture, &covers)
                        })
                    })
                    .collect();
                workers.into_iter().map(|w| w.join().unwrap()).collect()
            });
            without += kept.iter().filter(|k| k.is_none()).count();
        }
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(without, 0, "of 80 files, {without} told of no cover");
    }
}
