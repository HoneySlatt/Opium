//! The music files of this device, in the folders the user chose, as a
//! service like the others: their tags make the songs, albums and artists,
//! their pictures the covers. They play as they are.

mod scan;
mod tags;
mod watch;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use async_trait::async_trait;
use music_core::model::{
    Album, Artist, ArtistDetail, AudioQuality, Collection, Cursor, ItemId, Page, Playlist, ProviderKind,
    SearchResults, Track,
};
use music_core::text::fold;
use music_core::{Error, Provider, Result};

pub use scan::{EXTENSIONS, is_music};
pub use tags::{Cover, TagEdit, Tags, lyrics_from, lyrics_of, read_tags, read_tags_from, write_tags, write_tags_to};
pub use watch::Watching;

pub use scan::Entry;
use scan::Index;

const ALBUM: &str = "album:";
const ARTIST: &str = "artist:";
/// What search shows of each kind, at most.
const FOUND: usize = 50;

/// The files of `folders`, indexed.
pub struct Local {
    folders: Vec<PathBuf>,
    index_path: PathBuf,
    covers: PathBuf,
    index: RwLock<Arc<Index>>,
    /// Held while reading the folders, so that two reads at once (a change
    /// watched while tags are written) go one after the other: the later
    /// one then sees what the earlier wrote rather than undoing it.
    reading: Mutex<()>,
}

/// A file to play.
pub struct Audio {
    pub file: std::fs::File,
    pub len: u64,
    /// `flac`, `m4a`…: a hint for the decoder.
    pub extension: String,
    pub quality: Option<AudioQuality>,
}

impl Local {
    /// The library of `folders` as it was indexed last, at once; `dir` keeps
    /// the index and the covers.
    pub fn open(folders: Vec<PathBuf>, dir: &Path) -> Self {
        let index_path = dir.join("index.json");
        let index = Index::read(&index_path);
        Self {
            folders,
            covers: dir.join("covers"),
            index_path,
            index: RwLock::new(Arc::new(index)),
            reading: Mutex::new(()),
        }
    }

    /// A library of the files `entries` tell of, read elsewhere: the phone's,
    /// from its media index.
    pub fn from_entries(entries: Vec<Entry>) -> Self {
        Self {
            folders: Vec::new(),
            index_path: PathBuf::new(),
            covers: PathBuf::new(),
            index: RwLock::new(Arc::new(Index {
                entries,
                skipped: Vec::new(),
            })),
            reading: Mutex::new(()),
        }
    }

    pub fn folders(&self) -> &[PathBuf] {
        &self.folders
    }

    fn index(&self) -> Arc<Index> {
        self.index.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Read the folders again, the files unchanged as they were kept;
    /// whether the library changed. Waits on the disk: off the async threads.
    pub fn rescan(&self) -> bool {
        // The phone's come read already.
        if self.folders.is_empty() {
            return false;
        }
        let _reading = self.reading.lock().unwrap_or_else(|e| e.into_inner());
        let before = self.index();
        let (index, changed) = scan::scan(&self.folders, &before, &self.covers);
        if changed || index.skipped != before.skipped {
            let _ = index.write(&self.index_path);
            *self.index.write().unwrap_or_else(|e| e.into_inner()) = Arc::new(index);
        }
        changed
    }

    /// Watch the folders, calling `changed` once a change has settled.
    pub fn watch(&self, changed: impl Fn() + Send + 'static) -> Option<Watching> {
        watch::watch(&self.folders, changed).ok()
    }

    /// How many songs there are.
    pub fn count(&self) -> usize {
        self.index().entries.len()
    }

    /// Every song, in the folders' order.
    pub fn tracks(&self) -> Vec<Track> {
        let index = self.index();
        let covers = album_covers(&index.entries);
        index.entries.iter().map(|e| track(e, &covers)).collect()
    }

    /// The file `id` names, to play.
    pub fn audio(&self, id: &ItemId) -> Result<Audio> {
        audio(id, &self.index())
    }

    /// The covers' folder, for files opened from elsewhere.
    pub fn covers(&self) -> &Path {
        &self.covers
    }
}

/// A file opened from elsewhere (the file manager, the command line), as a
/// song; `covers` keeps its cover. None if it isn't music played here.
pub fn file_track(path: &Path, covers: &Path) -> Option<Track> {
    let meta = std::fs::metadata(path).ok()?;
    let entry = scan::read_file(path, meta.len(), 0, 0, covers)?;
    Some(track(&entry, &HashMap::new()))
}

/// The file `id` names, to play, with no library to say its quality.
pub fn file_audio(id: &ItemId) -> Result<Audio> {
    audio(id, &Index::default())
}

/// The music files `paths` name, a folder for those under it, in order.
pub fn files_in(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut found: Vec<PathBuf> = scan::walk(std::slice::from_ref(path))
                .into_iter()
                .map(|(p, _, _)| p)
                .collect();
            found.sort();
            files.extend(found);
        } else if is_music(path) {
            files.push(path.clone());
        }
    }
    files
}

/// The file `id` names, its quality as the index knows it.
fn audio(id: &ItemId, index: &Index) -> Result<Audio> {
    if id.provider != ProviderKind::Local {
        return Err(Error::Protocol("not a file of this device".into()));
    }
    let path = Path::new(&id.id);
    let file = std::fs::File::open(path).map_err(|e| Error::Playback(format!("{}: {e}", path.display())))?;
    let len = file.metadata().map_err(|e| Error::Playback(e.to_string()))?.len();
    let quality = index
        .entries
        .iter()
        .find(|e| e.path == id.id)
        .map(quality);
    Ok(Audio {
        file,
        len,
        extension: path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default(),
        quality,
    })
}

fn quality(e: &Entry) -> AudioQuality {
    AudioQuality {
        codec: e.codec.clone(),
        sample_rate: e.sample_rate,
        bit_depth: e.bit_depth,
        bitrate_kbps: e.bitrate,
    }
}

/// Who an entry's album is by: its album artist, else its first artist.
fn album_artist(e: &Entry) -> &str {
    e.album_artist
        .as_deref()
        .or(e.artists.first().map(String::as_str))
        .unwrap_or("Unknown Artist")
}

fn album_key(e: &Entry) -> Option<String> {
    let album = e.album.as_deref()?;
    Some(format!("{ALBUM}{}|{}", fold(album_artist(e)), fold(album)))
}

fn artist_id(name: &str) -> ItemId {
    ItemId::new(ProviderKind::Local, format!("{ARTIST}{}", fold(name)))
}

/// A cover kept at `path`, or at an address (the phone's covers).
fn artwork(path: &str) -> music_core::model::Artwork {
    music_core::model::Artwork {
        url_template: if path.contains("://") {
            path.to_owned()
        } else {
            format!("file://{path}")
        },
        background: None,
    }
}

/// Each album's cover: the first of its files that has one.
fn album_covers(entries: &[Entry]) -> HashMap<String, String> {
    let mut covers = HashMap::new();
    for e in entries {
        if let (Some(key), Some(cover)) = (album_key(e), &e.cover) {
            covers.entry(key).or_insert_with(|| cover.clone());
        }
    }
    covers
}

fn track(e: &Entry, covers: &HashMap<String, String>) -> Track {
    let key = album_key(e);
    let cover = e
        .cover
        .as_ref()
        .or_else(|| key.as_ref().and_then(|k| covers.get(k)));
    Track {
        id: ItemId::new(ProviderKind::Local, e.path.clone()),
        title: e.title.clone(),
        artists: if e.artists.is_empty() {
            vec![album_artist(e).to_owned()]
        } else {
            e.artists.clone()
        },
        album: e.album.clone(),
        album_id: key.map(|k| ItemId::new(ProviderKind::Local, k)),
        // Android's index says 0 for a file it couldn't read the length
        // of: unknown rather than a song of no length, which would match
        // no copy of itself anywhere.
        duration: (e.duration_ms > 0).then(|| std::time::Duration::from_millis(e.duration_ms)),
        track_number: e.track,
        disc_number: e.disc,
        artwork: cover.map(|c| artwork(c)),
        explicit: false,
        quality: Some(quality(e)),
        in_library: true,
        library_id: None,
    }
}

/// `secs` since 1970 as RFC 3339, in UTC.
fn rfc3339(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rest = secs % 86_400;
    // Howard Hinnant's days-to-civil.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// The albums, each with its songs in order.
fn albums(index: &Index) -> Vec<(Album, Vec<&Entry>)> {
    let mut by_key: HashMap<String, Vec<&Entry>> = HashMap::new();
    for e in &index.entries {
        if let Some(key) = album_key(e) {
            by_key.entry(key).or_default().push(e);
        }
    }
    let mut albums: Vec<(Album, Vec<&Entry>)> = by_key
        .into_iter()
        .map(|(key, mut songs)| {
            songs.sort_by(|a, b| {
                (a.disc.unwrap_or(1), a.track.unwrap_or(u32::MAX), &a.title)
                    .cmp(&(b.disc.unwrap_or(1), b.track.unwrap_or(u32::MAX), &b.title))
            });
            let first = songs[0];
            let artist = album_artist(first).to_owned();
            let album = Album {
                id: ItemId::new(ProviderKind::Local, key),
                title: first.album.clone().unwrap_or_default(),
                artist_id: Some(artist_id(&artist)),
                artists: vec![artist],
                artist_artwork: None,
                released: None,
                genre: songs.iter().find_map(|s| s.genre.clone()),
                description: None,
                catalog_id: None,
                in_favourites: None,
                year: songs.iter().filter_map(|s| s.year).max(),
                artwork: songs.iter().find_map(|s| s.cover.as_deref()).map(artwork),
                track_count: Some(songs.len() as u32),
                explicit: false,
                added: songs.iter().map(|s| s.added).min().map(rfc3339),
                motion: None,
                copyright: None,
                mastered: false,
            };
            (album, songs)
        })
        .collect();
    albums.sort_by(|(a, _), (b, _)| {
        (fold(&a.artists[0]), a.year, fold(&a.title)).cmp(&(fold(&b.artists[0]), b.year, fold(&b.title)))
    });
    albums
}

/// The artists the albums are by, and those of songs in no album.
fn artists(index: &Index) -> Vec<Artist> {
    let mut seen: HashMap<String, String> = HashMap::new();
    for e in &index.entries {
        let name = album_artist(e);
        seen.entry(fold(name)).or_insert_with(|| name.to_owned());
    }
    let mut artists: Vec<(String, String)> = seen.into_iter().collect();
    artists.sort();
    artists
        .into_iter()
        .map(|(_, name)| Artist {
            id: artist_id(&name),
            name,
            artwork: None,
            in_favourites: None,
            library_id: None,
        })
        .collect()
}

fn page<T>(items: Vec<T>) -> Page<T> {
    Page { items, next: None }
}

fn mine(id: &ItemId) -> Result<()> {
    if id.provider == ProviderKind::Local {
        Ok(())
    } else {
        Err(Error::Protocol(format!("not a file of this device: {id}")))
    }
}

/// Whether each of `words` is in `text`, folded.
fn matches(words: &[String], text: &str) -> bool {
    let text = fold(text);
    words.iter().all(|w| text.contains(w.as_str()))
}

#[async_trait]
impl Provider for Local {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Local
    }

    async fn search(&self, query: &str) -> Result<SearchResults> {
        self.search_library(query).await
    }

    async fn search_library(&self, query: &str) -> Result<SearchResults> {
        let words: Vec<String> = fold(query).split_whitespace().map(str::to_owned).collect();
        if words.is_empty() {
            return Ok(SearchResults::default());
        }
        let index = self.index();
        let covers = album_covers(&index.entries);
        let tracks = index
            .entries
            .iter()
            .filter(|e| {
                let album = e.album.as_deref().unwrap_or_default();
                matches(&words, &format!("{} {} {album}", e.title, e.artists.join(" ")))
            })
            .take(FOUND)
            .map(|e| track(e, &covers))
            .collect();
        let albums = albums(&index)
            .into_iter()
            .map(|(a, _)| a)
            .filter(|a| matches(&words, &format!("{} {}", a.title, a.artists.join(" "))))
            .take(FOUND)
            .collect();
        let artists = artists(&index)
            .into_iter()
            .filter(|a| matches(&words, &a.name))
            .take(FOUND)
            .collect();
        Ok(SearchResults {
            tracks,
            albums,
            artists,
            playlists: Vec::new(),
        })
    }

    async fn library_tracks(&self, _cursor: Option<&Cursor>) -> Result<Page<Track>> {
        Ok(page(self.tracks()))
    }

    async fn library_tracks_newest(&self, _cursor: Option<&Cursor>) -> Result<Page<Track>> {
        let index = self.index();
        let covers = album_covers(&index.entries);
        let mut entries: Vec<&Entry> = index.entries.iter().collect();
        entries.sort_by_key(|e| std::cmp::Reverse(e.added));
        Ok(page(entries.into_iter().map(|e| track(e, &covers)).collect()))
    }

    async fn library_albums(&self, _cursor: Option<&Cursor>) -> Result<Page<Album>> {
        Ok(page(albums(&self.index()).into_iter().map(|(a, _)| a).collect()))
    }

    async fn library_playlists(&self, _cursor: Option<&Cursor>) -> Result<Page<Playlist>> {
        Ok(page(Vec::new()))
    }

    async fn library_artists(&self, _cursor: Option<&Cursor>) -> Result<Page<Artist>> {
        Ok(page(artists(&self.index())))
    }

    async fn recently_added(&self, _cursor: Option<&Cursor>) -> Result<Page<Collection>> {
        let mut albums: Vec<Album> = albums(&self.index()).into_iter().map(|(a, _)| a).collect();
        albums.sort_by(|a, b| b.added.cmp(&a.added));
        Ok(page(albums.into_iter().map(Collection::Album).collect()))
    }

    async fn album(&self, album: &ItemId) -> Result<Album> {
        mine(album)?;
        albums(&self.index())
            .into_iter()
            .find(|(a, _)| a.id == *album)
            .map(|(a, _)| a)
            .ok_or_else(|| Error::Protocol("this album's files are gone".into()))
    }

    async fn album_tracks(&self, album: &ItemId) -> Result<Vec<Track>> {
        mine(album)?;
        let index = self.index();
        let covers = album_covers(&index.entries);
        albums(&index)
            .into_iter()
            .find(|(a, _)| a.id == *album)
            .map(|(_, songs)| songs.into_iter().map(|e| track(e, &covers)).collect())
            .ok_or_else(|| Error::Protocol("this album's files are gone".into()))
    }

    async fn artist(&self, artist: &ItemId) -> Result<ArtistDetail> {
        mine(artist)?;
        let index = self.index();
        let name = artists(&index)
            .into_iter()
            .find(|a| a.id == *artist)
            .ok_or_else(|| Error::Protocol("this artist's files are gone".into()))?;
        let key = fold(&name.name);
        let covers = album_covers(&index.entries);
        let (mut own, mut appears_on) = (Vec::new(), Vec::new());
        for (album, songs) in albums(&index) {
            if fold(&album.artists[0]) == key {
                own.push(album);
            } else if songs.iter().any(|s| s.artists.iter().any(|a| fold(a) == key)) {
                appears_on.push(album);
            }
        }
        own.sort_by_key(|a| std::cmp::Reverse(a.year));
        // The newest songs first, as no plays are counted here.
        let mut songs: Vec<&Entry> = index
            .entries
            .iter()
            .filter(|e| fold(album_artist(e)) == key || e.artists.iter().any(|a| fold(a) == key))
            .collect();
        songs.sort_by_key(|e| (std::cmp::Reverse(e.year), e.album.clone(), e.disc, e.track));
        let artwork = own.iter().find_map(|a| a.artwork.clone());
        let (singles, albums): (Vec<Album>, Vec<Album>) =
            own.into_iter().partition(|a| a.track_count.is_some_and(|n| n <= 3));
        Ok(ArtistDetail {
            artist: Artist { artwork, ..name },
            top_songs: songs.into_iter().take(10).map(|e| track(e, &covers)).collect(),
            latest: None,
            albums,
            singles,
            compilations: Vec::new(),
            appears_on,
            similar: Vec::new(),
            essentials: Vec::new(),
            videos: Vec::new(),
            playlists: Vec::new(),
            motion: None,
            banner: None,
        })
    }

    /// An `.lrc` beside the file, else the lyrics in its tags; none for the
    /// phone's, which the app reads itself.
    async fn lyrics(&self, track: &ItemId) -> Result<Option<music_core::model::Lyrics>> {
        mine(track)?;
        let title = self
            .index()
            .entries
            .iter()
            .find(|e| e.path == track.id)
            .map(|e| e.title.clone())
            .unwrap_or_default();
        Ok(lyrics_of(Path::new(&track.id), &title))
    }

    async fn playlist_tracks(&self, _playlist: &ItemId, _cursor: Option<&Cursor>) -> Result<Page<Track>> {
        Err(Error::Protocol("files have no playlists of their own".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_written_as_rfc_3339() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_793_750_400), "2026-11-04T00:00:00Z");
        assert_eq!(rfc3339(951_782_400 + 3_661), "2000-02-29T01:01:01Z");
    }

    fn entry(path: &str, title: &str, artist: &str, album: Option<&str>, track: u32) -> Entry {
        Entry {
            path: path.into(),
            size: 1,
            modified: 1,
            added: track as u64,
            title: title.into(),
            artists: vec![artist.into()],
            album_artist: None,
            album: album.map(str::to_owned),
            track: Some(track),
            disc: None,
            year: Some(2020),
            genre: None,
            duration_ms: 1000,
            codec: music_core::model::Codec::Flac,
            sample_rate: Some(44_100),
            bit_depth: Some(16),
            bitrate: None,
            cover: (track == 2).then(|| "/covers/a.jpg".into()),
        }
    }

    fn local(entries: Vec<Entry>) -> Local {
        Local {
            folders: Vec::new(),
            index_path: PathBuf::new(),
            covers: PathBuf::new(),
            index: RwLock::new(Arc::new(Index {
                entries,
                skipped: Vec::new(),
            })),
            reading: Mutex::new(()),
        }
    }

    #[tokio::test]
    async fn songs_make_albums_and_artists_as_their_tags_say() {
        let library = local(vec![
            entry("/m/b.flac", "Second", "Ébène", Some("Disc"), 2),
            entry("/m/a.flac", "First", "Ébène", Some("Disc"), 1),
            entry("/m/c.flac", "Alone", "Other", None, 1),
        ]);
        let albums = library.library_albums(None).await.unwrap().items;
        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].track_count, Some(2));
        assert_eq!(albums[0].artwork.as_ref().unwrap().url_template, "file:///covers/a.jpg");
        let songs = library.album_tracks(&albums[0].id).await.unwrap();
        assert_eq!(songs.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(), ["First", "Second"]);
        // The album's cover for its songs without one.
        assert!(songs[0].artwork.is_some());
        let artists = library.library_artists(None).await.unwrap().items;
        assert_eq!(artists.len(), 2);
        let found = library.search_library("ebene sec").await.unwrap();
        assert_eq!(found.tracks.len(), 1);
        assert_eq!(found.tracks[0].title, "Second");
        let page = library.artist(&artists[0].id).await.unwrap();
        assert_eq!(page.artist.name, "Ébène");
        assert_eq!((page.albums.len() + page.singles.len(), page.top_songs.len()), (1, 2));
        assert!(page.artist.artwork.is_some(), "their album's cover");
        let newest = library.library_tracks_newest(None).await.unwrap().items;
        assert_eq!(newest[0].title, "Second");
    }

    #[test]
    fn tags_written_are_read_back_and_the_index_follows() {
        let dir = std::env::temp_dir().join(format!("local-tags-{}", std::process::id()));
        let music = dir.join("music");
        std::fs::create_dir_all(&music).unwrap();
        let path = music.join("song.wav");
        std::fs::write(&path, scan::tests::wav()).unwrap();
        let library = Local::open(vec![music.clone()], &dir.join("cache"));
        library.rescan();
        assert_eq!(library.tracks()[0].title, "song");
        // A picture made a square JPEG as a picked one is.
        let cover = music_core::cover::square(&image_png()).expect("a cover");
        let edit = TagEdit {
            title: Some("Renamed".into()),
            artist: Some("Someone".into()),
            album: Some("An Album".into()),
            album_artist: Some("Someone".into()),
            track: Some(Some(4)),
            year: Some(Some(1999)),
            cover: Cover::Set(cover),
            ..TagEdit::default()
        };
        write_tags(&path, &edit).unwrap();
        let tags = read_tags(&path).unwrap();
        assert_eq!(
            (tags.title.as_str(), tags.artist.as_str(), tags.album.as_str(), tags.track, tags.year),
            ("Renamed", "Someone", "An Album", Some(4), Some(1999))
        );
        assert!(library.rescan(), "the file changed");
        let track = &library.tracks()[0];
        assert_eq!((track.title.as_str(), track.track_number), ("Renamed", Some(4)));
        assert!(track.artwork.is_some(), "its cover");
        // Cleared again.
        write_tags(&path, &TagEdit { title: Some(String::new()), cover: Cover::Removed, ..TagEdit::default() }).unwrap();
        assert_eq!(read_tags(&path).unwrap().title, "");
        library.rescan();
        assert_eq!(library.tracks()[0].title, "song");
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// A small PNG.
    fn image_png() -> Vec<u8> {
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(8, 8, image::Rgb([200, 40, 90]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        png.into_inner()
    }

    #[tokio::test]
    async fn a_files_lyrics_come_from_beside_it_else_from_its_tags() {
        let dir = std::env::temp_dir().join(format!("local-lyrics-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("song.wav");
        std::fs::write(&path, scan::tests::wav()).unwrap();
        let library = Local::open(vec![dir.clone()], &dir.join("cache"));
        library.rescan();
        let id = ItemId::new(ProviderKind::Local, path.to_string_lossy());
        assert!(library.lyrics(&id).await.unwrap().is_none());
        // Its tags', plain.
        {
            use lofty::file::{AudioFile, TaggedFileExt};
            let mut file = lofty::read_from_path(&path).unwrap();
            if file.primary_tag().is_none() {
                file.insert_tag(lofty::tag::Tag::new(file.primary_tag_type()));
            }
            file.primary_tag_mut()
                .unwrap()
                .insert_text(lofty::tag::ItemKey::UnsyncLyrics, "First words\nSecond words".into());
            file.save_to_path(&path, lofty::config::WriteOptions::default()).unwrap();
        }
        let tagged = library.lyrics(&id).await.unwrap().unwrap();
        assert!(!tagged.synced);
        assert_eq!(tagged.lines[0].text, "First words");
        // An .lrc beside it comes first, timed, line by line.
        std::fs::write(path.with_extension("lrc"), "[00:01.00]<00:01.00>Hello <00:01.50>world\n[00:03.00]Next\n").unwrap();
        let beside = library.lyrics(&id).await.unwrap().unwrap();
        assert!(beside.synced);
        assert_eq!(beside.lines[0].text, "Hello world");
        assert!(beside.lines.iter().all(|l| l.words.is_empty()));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_wav_file_plays_from_its_path() {
        let dir = std::env::temp_dir().join(format!("local-lib-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Tone.wav");
        std::fs::write(&path, scan::tests::wav()).unwrap();
        let track = file_track(&path, &dir.join("covers")).unwrap();
        assert_eq!(track.title, "Tone");
        let audio = audio(&track.id, &Index::default()).unwrap();
        assert_eq!(audio.extension, "wav");
        assert_eq!(audio.len, std::fs::metadata(&path).unwrap().len());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn a_tag_of_another_kind_isnt_lost_when_one_field_is_written() {
        // An MP3 whose only tag is an ID3v1: editing the album used to
        // leave an ID3v2 holding the album alone, read as untitled.
        use lofty::config::WriteOptions;
        use lofty::file::TaggedFileExt;
        use lofty::tag::{Accessor, TagExt};
        let dir = std::env::temp_dir().join(format!("local-v1-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("song.mp3");
        std::fs::write(&path, scan::tests::mp3()).unwrap();
        {
            let v1 = lofty::id3::v1::Id3v1Tag {
                title: Some("Real Title".into()),
                artist: Some("Real Artist".into()),
                album: Some("Real Album".into()),
                ..Default::default()
            };
            let mut file = std::fs::OpenOptions::new().read(true).write(true).open(&path).unwrap();
            v1.save_to(&mut file, WriteOptions::default()).unwrap();
        }
        assert_eq!(read_tags(&path).unwrap().title, "Real Title", "read before");

        write_tags(&path, &TagEdit { album: Some("Edited Album".into()), ..TagEdit::default() }).unwrap();
        let tags = read_tags(&path).unwrap();
        assert_eq!(
            (tags.title.as_str(), tags.artist.as_str(), tags.album.as_str()),
            ("Real Title", "Real Artist", "Edited Album")
        );
        // The file's own kind of tag now holds it all, so that another
        // player reading only that one shows the song as it is.
        let read = lofty::read_from_path(&path).unwrap();
        let primary = read.primary_tag().expect("an ID3v2 of its own");
        assert_eq!(
            (
                primary.title().as_deref(),
                primary.artist().as_deref(),
                primary.album().as_deref()
            ),
            (Some("Real Title"), Some("Real Artist"), Some("Edited Album"))
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

}
