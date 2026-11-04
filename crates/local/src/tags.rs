//! A file's tags, as the Info window shows them and writes them back.

use std::path::Path;

use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::tag::{Accessor, ItemKey, Tag};

/// What a file's tags say, an empty text for none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tags {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: String,
    pub track: Option<u32>,
    pub disc: Option<u32>,
    pub year: Option<u16>,
}

/// What a cover becomes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Cover {
    #[default]
    Kept,
    Removed,
    /// A JPEG, the front cover from now on.
    Set(Vec<u8>),
}

/// What to change in a file's tags: each field given is written (an empty
/// text, a `None` number clears it), the others are left as they are.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagEdit {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub track: Option<Option<u32>>,
    pub disc: Option<Option<u32>>,
    pub year: Option<Option<u16>>,
    pub cover: Cover,
}

impl TagEdit {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The tags of the file at `path`, as written in it.
pub fn read_tags(path: &Path) -> Option<Tags> {
    read_tags_from(&mut std::fs::File::open(path).ok()?)
}

/// The tags of `file`, open: the phone's, from Android's media index.
pub fn read_tags_from(file: &mut std::fs::File) -> Option<Tags> {
    let file = lofty::read_from(file).ok()?;
    Some(tags_of(&file))
}

/// What a file says, its own kind of tag first and anything it also carries
/// behind it, field by field: an MP3 whose ID3v2 is there but empty still
/// shows what its ID3v1 says.
pub(crate) fn tags_of(file: &lofty::file::TaggedFile) -> Tags {
    let tags: Vec<&Tag> = file
        .primary_tag()
        .into_iter()
        .chain(file.tags().iter().filter(|t| {
            file.primary_tag()
                .is_none_or(|p| p.tag_type() != t.tag_type())
        }))
        .collect();
    let text = |pick: fn(&Tag) -> Option<std::borrow::Cow<'_, str>>| {
        tags.iter()
            .filter_map(|t| pick(t))
            .map(|v| v.trim().to_owned())
            .find(|v| !v.is_empty())
            .unwrap_or_default()
    };
    let number = |pick: fn(&Tag) -> Option<u32>| tags.iter().find_map(|t| pick(t));
    Tags {
        title: text(Accessor::title),
        artist: text(Accessor::artist),
        album: text(Accessor::album),
        album_artist: tags
            .iter()
            .filter_map(|t| t.get_string(ItemKey::AlbumArtist))
            .map(|v| v.trim().to_owned())
            .find(|v| !v.is_empty())
            .unwrap_or_default(),
        genre: text(Accessor::genre),
        track: number(Accessor::track),
        disc: number(Accessor::disk),
        year: tags
            .iter()
            .filter_map(|t| t.date())
            .map(|d| d.year)
            .find(|&y| y > 0),
    }
}

/// Write `edit` into the file at `path`, in its own kind of tag.
pub fn write_tags(path: &Path, edit: &TagEdit) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    write_tags_to(&mut file, edit)
}

/// Write `edit` into `file`, open to read and write: the phone's.
pub fn write_tags_to(open: &mut std::fs::File, edit: &TagEdit) -> Result<(), String> {
    use std::io::Seek;
    let mut file = lofty::read_from(open).map_err(|e| e.to_string())?;
    if file.primary_tag().is_none() {
        // What the file already says, carried over: a tag of another kind
        // (an MP3 with only an ID3v1) would otherwise be left behind an
        // empty one of its own kind, and the song read as untitled.
        let mut seed = match file.first_tag() {
            Some(carried) => {
                let mut tag = carried.clone();
                tag.re_map(file.primary_tag_type());
                tag
            }
            None => Tag::new(file.primary_tag_type()),
        };
        seed.remove_empty();
        file.insert_tag(seed);
    }
    let tag = file
        .primary_tag_mut()
        .ok_or("this file takes no tags")?;
    let set = |tag: &mut Tag, value: &Option<String>, put: fn(&mut Tag, String), clear: fn(&mut Tag)| {
        match value.as_deref().map(str::trim) {
            Some("") => clear(tag),
            Some(text) => put(tag, text.to_owned()),
            None => {}
        }
    };
    set(tag, &edit.title, Tag::set_title, Tag::remove_title);
    set(tag, &edit.artist, Tag::set_artist, Tag::remove_artist);
    set(tag, &edit.album, Tag::set_album, Tag::remove_album);
    set(tag, &edit.genre, Tag::set_genre, Tag::remove_genre);
    set(
        tag,
        &edit.album_artist,
        |t, v| {
            t.insert_text(ItemKey::AlbumArtist, v);
        },
        |t| t.remove_key(ItemKey::AlbumArtist),
    );
    match edit.track {
        Some(Some(n)) => tag.set_track(n),
        Some(None) => tag.remove_track(),
        None => {}
    }
    match edit.disc {
        Some(Some(n)) => tag.set_disk(n),
        Some(None) => tag.remove_disk(),
        None => {}
    }
    match edit.year {
        Some(Some(year)) => tag.set_date(lofty::tag::items::Timestamp {
            year,
            ..Default::default()
        }),
        Some(None) => tag.remove_date(),
        None => {}
    }
    match &edit.cover {
        Cover::Kept => {}
        Cover::Removed => {
            while !tag.pictures().is_empty() {
                tag.remove_picture(0);
            }
        }
        Cover::Set(jpeg) => {
            tag.remove_picture_type(PictureType::CoverFront);
            tag.push_picture(
                Picture::unchecked(jpeg.clone())
                    .pic_type(PictureType::CoverFront)
                    .mime_type(MimeType::Jpeg)
                    .build(),
            );
        }
    }
    open.rewind().map_err(|e| e.to_string())?;
    file.save_to(open, WriteOptions::default())
        .map_err(|e| e.to_string())
}

/// The lyrics of the file at `path`: an `.lrc` of the same name beside it,
/// else those in its tags. Line by line, as every source's but Apple Music's.
pub fn lyrics_of(path: &Path, title: &str) -> Option<opium_core::model::Lyrics> {
    let beside = ["lrc", "LRC"]
        .iter()
        .find_map(|ext| std::fs::read_to_string(path.with_extension(ext)).ok())
        .and_then(|text| lyrics::lrc::parse(&text, title));
    beside
        .or_else(|| lyrics_from(&mut std::fs::File::open(path).ok()?, title))
        .map(line_by_line)
}

/// The lyrics in the tags of `file`, open: the phone's.
pub fn lyrics_from(file: &mut std::fs::File, title: &str) -> Option<opium_core::model::Lyrics> {
    let file = lofty::read_from(file).ok()?;
    let tag = file.primary_tag().or_else(|| file.first_tag())?;
    [ItemKey::Lyrics, ItemKey::UnsyncLyrics]
        .into_iter()
        .find_map(|key| tag.get_string(key))
        .and_then(|text| lyrics::lrc::parse(text, title))
        .map(line_by_line)
}

fn line_by_line(mut lyrics: opium_core::model::Lyrics) -> opium_core::model::Lyrics {
    for line in &mut lyrics.lines {
        line.words.clear();
    }
    lyrics
}
