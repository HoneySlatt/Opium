//! The folders read: each music file's tags and cover, kept in an index on
//! the disk so a launch shows the library at once, and only the files changed
//! since are read again.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use lofty::file::{AudioFile, FileType, TaggedFileExt};
use lofty::picture::PictureType;
use lofty::tag::{Accessor, ItemKey, Tag};
use music_core::model::Codec;
use serde::{Deserialize, Serialize};

/// The files read as music: what the player decodes.
pub const EXTENSIONS: [&str; 10] = [
    "mp3", "flac", "wav", "aif", "aiff", "aifc", "m4a", "mp4", "aac", "ogg",
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

    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let partial = path.with_extension("part");
        std::fs::write(&partial, serde_json::to_vec(self)?)?;
        std::fs::rename(partial, path)
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

/// The music files under `folders`, with their size and when they changed;
/// hidden folders are left out.
pub(crate) fn walk(folders: &[PathBuf]) -> Vec<(PathBuf, u64, u64)> {
    let mut found = Vec::new();
    let mut todo: Vec<PathBuf> = folders.to_vec();
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
                let Ok(meta) = std::fs::metadata(&path) else { continue };
                if meta.is_file() {
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
    let known: HashMap<&str, &Entry> = before.entries.iter().map(|e| (e.path.as_str(), e)).collect();
    let skipped_before: HashMap<&str, (u64, u64)> =
        before.skipped.iter().map(|(p, size, modified)| (p.as_str(), (*size, *modified))).collect();
    let files = walk(folders);
    let mut kept = Vec::with_capacity(files.len());
    let mut skipped = Vec::new();
    let mut fresh = Vec::new();
    for (path, size, modified) in files {
        let name = path.to_string_lossy();
        if skipped_before.get(name.as_ref()) == Some(&(size, modified)) {
            skipped.push((name.into_owned(), size, modified));
            continue;
        }
        match known.get(name.as_ref()) {
            Some(entry) if entry.size == size && entry.modified == modified => kept.push((*entry).clone()),
            entry => fresh.push((path, size, modified, entry.map(|e| e.added))),
        }
    }
    // Read on every core: tags are mostly waiting on the disk.
    let next = AtomicUsize::new(0);
    let read = Mutex::new(Vec::with_capacity(fresh.len()));
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).min(8);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((path, size, modified, added)) = fresh.get(i) else {
                    return;
                };
                let entry = read_file(path, *size, *modified, added.unwrap_or(*modified), covers)
                    .ok_or_else(|| (path.to_string_lossy().into_owned(), *size, *modified));
                read.lock().unwrap_or_else(|e| e.into_inner()).push(entry);
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
    folder_covers(&mut entries, covers);
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let changed = entries != before.entries;
    (Index { entries, skipped }, changed)
}

/// The file at `path` as an entry, its tags read; none if it isn't music
/// the player decodes.
pub fn read_file(path: &Path, size: u64, modified: u64, added: u64, covers: &Path) -> Option<Entry> {
    let file = lofty::read_from_path(path).ok()?;
    let props = file.properties();
    let codec = match file.file_type() {
        FileType::Flac => Codec::Flac,
        FileType::Mpeg => Codec::Mp3,
        FileType::Wav | FileType::Aiff => Codec::Wav,
        FileType::Vorbis => Codec::Vorbis,
        FileType::Aac => Codec::Aac,
        // Apple Lossless keeps its bit depth; AAC has none.
        FileType::Mp4 if props.bit_depth().is_some() => Codec::Alac,
        FileType::Mp4 => Codec::Aac,
        // Opus and the rest: the player doesn't decode them.
        _ => return None,
    };
    let tag = file.primary_tag().or_else(|| file.first_tag());
    let text = |key: ItemKey| tag.and_then(|t| t.get_string(key)).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned);
    let mut artists: Vec<String> = tag
        .map(|t| t.get_strings(ItemKey::TrackArtist).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect())
        .unwrap_or_default();
    if artists.is_empty() {
        artists.extend(text(ItemKey::AlbumArtist));
    }
    let title = tag
        .and_then(|t| t.title())
        .map(|t| t.trim().to_owned())
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
        album_artist: text(ItemKey::AlbumArtist),
        album: tag.and_then(|t| t.album()).map(|a| a.trim().to_owned()).filter(|a| !a.is_empty()),
        track: tag.and_then(|t| t.track()),
        disc: tag.and_then(|t| t.disk()),
        year: tag.and_then(|t| t.date()).map(|d| d.year).filter(|&y| y > 0),
        genre: tag.and_then(|t| t.genre()).map(|g| g.trim().to_owned()).filter(|g| !g.is_empty()),
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
fn embedded_cover(tag: &Tag, covers: &Path) -> Option<String> {
    let pictures = tag.pictures();
    let picture = pictures
        .iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| pictures.first())?;
    keep_cover(picture.data(), covers)
}

/// `bytes`, a picture, made square in `covers`; its path.
pub fn keep_cover(bytes: &[u8], covers: &Path) -> Option<String> {
    let path = covers.join(format!("{:016x}.jpg", fnv(bytes)));
    if !path.exists() {
        let square = music_core::cover::square(bytes)?;
        std::fs::create_dir_all(covers).ok()?;
        let partial = path.with_extension("part");
        std::fs::write(&partial, square).ok()?;
        std::fs::rename(&partial, &path).ok()?;
    }
    Some(path.to_string_lossy().into_owned())
}

/// The files with no picture of their own take their folder's: `cover.jpg`,
/// `folder.png` and the like.
fn folder_covers(entries: &mut [Entry], covers: &Path) {
    let mut found: HashMap<PathBuf, Option<String>> = HashMap::new();
    for entry in entries.iter_mut().filter(|e| e.cover.is_none()) {
        let Some(dir) = Path::new(&entry.path).parent() else { continue };
        entry.cover = found
            .entry(dir.to_owned())
            .or_insert_with(|| folder_cover(dir, covers))
            .clone();
    }
}

fn folder_cover(dir: &Path, covers: &Path) -> Option<String> {
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
    keep_cover(&std::fs::read(path).ok()?, covers)
}

/// FNV-1a, 64 bits.
pub fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ *b as u64).wrapping_mul(0x0000_0100_0000_01b3))
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

    #[test]
    fn a_file_read_once_is_kept_until_it_changes() {
        let dir = std::env::temp_dir().join(format!("local-scan-{}", std::process::id()));
        let music = dir.join("music");
        std::fs::create_dir_all(music.join(".hidden")).unwrap();
        std::fs::write(music.join("Song.wav"), wav()).unwrap();
        // Not music the tags can be read from: skipped, and not read again.
        std::fs::write(music.join("broken.mp3"), b"not an mp3").unwrap();
        std::fs::write(music.join(".hidden").join("x.flac"), b"x").unwrap();
        assert_eq!(walk(std::slice::from_ref(&music)).len(), 2, "hidden folders are left out");
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
}
