//! Where an AAC track's music starts and ends in its MP4. Encoders put
//! priming frames before the music (2112 for Apple's, 1024 for FFmpeg's) and
//! pad the last frame after it; a player that plays every frame leaves a gap
//! between songs meant to run into each other.
//!
//! An MP4 says so in its edit list (`elst`), or in iTunes' `iTunSMPB` tag;
//! Apple Music's own streams say nothing, their priming being the rule
//! (see [`crate::Source::with_priming`]), and end their last frame early.

use std::io::{Read, Seek, SeekFrom};

/// The music's place, in `timescale` units per second: frames, when it is
/// the sample rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Trim {
    /// What comes before the music.
    pub priming: u64,
    /// The music's length, when the file says.
    pub length: Option<u64>,
    /// `None` for frames.
    pub timescale: Option<u32>,
}

/// How far into the file the movie box may start for it to be read: one
/// further on (after the media) would mean waiting for a download's end.
const MOOV_WITHIN: u64 = 8 << 20;
/// A movie box larger than this isn't one worth reading.
const MOOV_MAX: u64 = 16 << 20;

/// The trim an MP4 says, if it says one; `reader` is left at its start.
pub(crate) fn mp4_trim<R: Read + Seek + ?Sized>(reader: &mut R) -> Option<Trim> {
    let found = find_moov(reader);
    reader.seek(SeekFrom::Start(0)).ok()?;
    parse_moov(&found?)
}

fn find_moov<R: Read + Seek + ?Sized>(reader: &mut R) -> Option<Vec<u8>> {
    let mut pos = 0u64;
    while pos <= MOOV_WITHIN {
        reader.seek(SeekFrom::Start(pos)).ok()?;
        let mut head = [0u8; 8];
        reader.read_exact(&mut head).ok()?;
        let mut size = u64::from(u32::from_be_bytes(head[..4].try_into().ok()?));
        let mut header = 8;
        if size == 1 {
            let mut large = [0u8; 8];
            reader.read_exact(&mut large).ok()?;
            size = u64::from_be_bytes(large);
            header = 16;
        }
        // To the end of the file, or broken.
        if size < header {
            return None;
        }
        if &head[4..] == b"moov" {
            if size > MOOV_MAX {
                return None;
            }
            let mut body = vec![0u8; usize::try_from(size - header).ok()?];
            reader.read_exact(&mut body).ok()?;
            return Some(body);
        }
        pos = pos.checked_add(size)?;
    }
    None
}

/// The boxes in `data`, by type and body.
fn boxes(data: &[u8]) -> impl Iterator<Item = (&[u8], &[u8])> {
    let mut pos = 0usize;
    std::iter::from_fn(move || {
        let head = data.get(pos..pos + 8)?;
        let size = u32::from_be_bytes(head[..4].try_into().ok()?) as usize;
        let (header, size) = match size {
            1 => {
                let large = data.get(pos + 8..pos + 16)?;
                (
                    16,
                    usize::try_from(u64::from_be_bytes(large.try_into().ok()?)).ok()?,
                )
            }
            0 => (8, data.len() - pos),
            s => (8, s),
        };
        if size < header {
            return None;
        }
        let body = data.get(pos + header..pos.checked_add(size)?)?;
        pos += size;
        Some((&head[4..8], body))
    })
}

fn child<'a>(data: &'a [u8], kind: &[u8]) -> Option<&'a [u8]> {
    boxes(data).find(|(k, _)| *k == kind).map(|(_, body)| body)
}

fn be32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn be64(data: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(data.get(at..at + 8)?.try_into().ok()?))
}

/// A full box's timescale: `mvhd` and `mdhd` keep it at the same place.
fn timescale(full_box: &[u8]) -> Option<u32> {
    match full_box.first()? {
        1 => be32(full_box, 20),
        _ => be32(full_box, 12),
    }
}

fn parse_moov(moov: &[u8]) -> Option<Trim> {
    itunes_smpb(moov).or_else(|| edit_list(moov))
}

/// iTunes' ` 00000000 PPPPPPPP QQQQQQQQ LLLLLLLLLLLLLLLL …`: the priming,
/// the padding and the music's length, in frames, in hex.
fn itunes_smpb(moov: &[u8]) -> Option<Trim> {
    let meta = child(child(moov, b"udta")?, b"meta")?;
    // `meta` is a full box: its children follow its version and flags.
    let ilst = child(meta.get(4..)?, b"ilst")?;
    let text = boxes(ilst)
        .filter(|(kind, _)| *kind == b"----")
        .find_map(|(_, item)| {
            let name = child(item, b"name")?;
            if name.get(4..)? != b"iTunSMPB" {
                return None;
            }
            // Its version and flags, then a type and a locale.
            let data = child(item, b"data")?;
            std::str::from_utf8(data.get(8..)?).ok()
        })?;
    let fields: Vec<u64> = text
        .split_whitespace()
        .map(|f| u64::from_str_radix(f, 16).ok())
        .collect::<Option<_>>()?;
    Some(Trim {
        priming: *fields.get(1)?,
        length: fields.get(3).copied().filter(|&l| l > 0),
        timescale: None,
    })
}

/// The sound track's edit list: where its media starts playing, and for how
/// long.
fn edit_list(moov: &[u8]) -> Option<Trim> {
    let movie_scale = timescale(child(moov, b"mvhd")?)?;
    let trak = boxes(moov)
        .filter(|(k, _)| *k == b"trak")
        .find(|(_, trak)| {
            child(trak, b"mdia")
                .and_then(|mdia| child(mdia, b"hdlr"))
                .and_then(|hdlr| hdlr.get(8..12))
                == Some(b"soun".as_slice())
        })?;
    let trak = trak.1;
    let media_scale = timescale(child(child(trak, b"mdia")?, b"mdhd")?)?;
    let elst = child(child(trak, b"edts")?, b"elst")?;
    let version = *elst.first()?;
    let count = be32(elst, 4)? as usize;
    let entry = if version == 1 { 20 } else { 12 };
    // The first edit that plays media (an empty one, -1, only delays it).
    (0..count).find_map(|i| {
        let at = 8 + i * entry;
        let (duration, start) = if version == 1 {
            (be64(elst, at)?, be64(elst, at + 8)? as i64)
        } else {
            (
                u64::from(be32(elst, at)?),
                i64::from(be32(elst, at + 4)? as i32),
            )
        };
        let priming = u64::try_from(start).ok()?;
        let length = (duration > 0 && movie_scale > 0)
            .then(|| duration * u64::from(media_scale) / u64::from(movie_scale));
        Some(Trim {
            priming,
            length,
            timescale: Some(media_scale),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let size = u32::try_from(8 + body.len()).expect("a small box");
        [&size.to_be_bytes()[..], kind, body].concat()
    }

    fn full(version: u8, rest: &[u8]) -> Vec<u8> {
        let mut b = vec![version, 0, 0, 0];
        b.extend_from_slice(rest);
        b
    }

    /// `mvhd`/`mdhd` version 0: two times, then the timescale.
    fn header(kind: &[u8; 4], scale: u32) -> Vec<u8> {
        let mut rest = vec![0u8; 8];
        rest.extend_from_slice(&scale.to_be_bytes());
        rest.extend_from_slice(&[0u8; 8]);
        boxed(kind, &full(0, &rest))
    }

    #[test]
    fn the_edit_list_says_where_the_music_starts() {
        let mut elst = 1u32.to_be_bytes().to_vec();
        elst.extend_from_slice(&0u32.to_be_bytes());
        elst.extend_from_slice(&1024u32.to_be_bytes());
        elst.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        // Its pre-defined word, then the handler.
        let mut hdlr = vec![0u8; 4];
        hdlr.extend_from_slice(b"soun");
        let mdia = [header(b"mdhd", 44_100), boxed(b"hdlr", &full(0, &hdlr))].concat();
        let trak = [
            boxed(b"edts", &boxed(b"elst", &full(0, &elst))),
            boxed(b"mdia", &mdia),
        ]
        .concat();
        let moov = [header(b"mvhd", 1000), boxed(b"trak", &trak)].concat();
        let file = [boxed(b"ftyp", b"iso6"), boxed(b"moov", &moov)].concat();
        let trim = mp4_trim(&mut std::io::Cursor::new(file)).unwrap();
        assert_eq!(
            trim,
            Trim {
                priming: 1024,
                length: None,
                timescale: Some(44_100)
            }
        );
    }

    #[test]
    fn itunes_says_it_in_frames() {
        let smpb = b" 00000000 00000840 000001CA 00000000001B8000 00000000";
        let mut data = vec![0u8; 8];
        data.extend_from_slice(smpb);
        let item = [
            boxed(b"mean", &full(0, b"com.apple.iTunes")),
            boxed(b"name", &full(0, b"iTunSMPB")),
            boxed(b"data", &data),
        ]
        .concat();
        let ilst = boxed(b"ilst", &boxed(b"----", &item));
        let moov = boxed(b"udta", &boxed(b"meta", &full(0, &ilst)));
        let trim = parse_moov(&moov).unwrap();
        assert_eq!(
            trim,
            Trim {
                priming: 2112,
                length: Some(0x1B_8000),
                timescale: None
            }
        );
    }
}
