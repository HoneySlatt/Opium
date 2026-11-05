//! Rewriting a file in place: making it read as a plain MP4, writing key ids into its `tenc`
//! boxes, and the last sample's duration of a fragment.
//!
//! Only four-byte types and counts change, so that every offset stays true. The walks here are
//! lenient: a box they cannot read ends the walk, and nothing is written there.

use crate::boxes::{
    Header, Kind, Lenient, lenient_at, lenient_kind, lenient_u32, retype, slice, write_u32,
};
use crate::movie::fixed_fields;
use crate::{Movie, SampleTable, Track};

const FREE: Kind = *b"free";
const SEIG: Kind = *b"seig";

/// Makes `buf`, the file `movie` and `samples` describe, read as a plain MP4; its samples are
/// decrypted apart, before or after (CENC keeps sizes).
///
/// Each encrypted sample entry takes back the type its `frma` names and its `sinf` becomes
/// `free`; in the `moov` and in every `moof` the table names, `senc`, `saiz`, `saio`, `pssh` and
/// the `sbgp` and `sgpd` of type `seig` become `free`. Where a track's entries are the same
/// audio under several keys (Spatial Audio: several entries, all encrypted, of one format and
/// one `AudioSpecificConfig`), `stsd` says it has one entry and every description index that
/// names an entry becomes 1. Nothing else changes.
///
/// Called again with the table grown, it changes only the fragments the table names, and leaves
/// as it is what an earlier call changed. A `buf` that does not match `movie` is left alone
/// wherever it does not match.
pub fn make_clear(buf: &mut [u8], movie: &Movie, samples: &SampleTable) {
    let collapsed: Vec<u32> = movie
        .tracks
        .iter()
        .filter(|track| collapses(track))
        .map(|track| track.id)
        .collect();
    clear_movie(buf, movie, &collapsed);
    for fragment in samples.fragments() {
        clear_fragment(buf, fragment.start, &collapsed);
    }
}

/// Whether a track's entries are one audio under several keys, to be shown as one entry.
fn collapses(track: &Track) -> bool {
    let Some(first) = track.entries.first() else {
        return false;
    };
    track.entries.len() > 1
        && track.entries.iter().all(|entry| {
            entry.protection.is_some()
                && entry.format == first.format
                && entry.audio_config == first.audio_config
        })
}

fn clear_movie(buf: &mut [u8], movie: &Movie, collapsed: &[u32]) {
    let Some(moov) = lenient_at(buf, movie.at, *b"moov") else {
        return;
    };
    if moov.end != movie.end {
        return;
    }
    let mut children = Lenient::inside(moov);
    while let Some(child) = children.next(buf) {
        match &child.kind {
            b"pssh" => retype(buf, child, FREE),
            b"trak" => clear_trak(buf, child, movie, collapsed),
            b"mvex" => {
                let mut boxes = Lenient::inside(child);
                while let Some(trex) = boxes.next(buf) {
                    if trex.kind == *b"trex"
                        && lenient_u32(buf, trex.body + 4).is_some_and(|id| collapsed.contains(&id))
                    {
                        write_u32(buf, trex.body + 8, 1);
                    }
                }
            }
            _ => {}
        }
    }
}

fn clear_trak(buf: &mut [u8], trak: Header, movie: &Movie, collapsed: &[u32]) {
    let mut id = None;
    let mut media = None;
    let mut children = Lenient::inside(trak);
    while let Some(child) = children.next(buf) {
        match &child.kind {
            b"tkhd" => id = lenient_track_id(buf, child),
            b"mdia" => media = Some(child),
            _ => {}
        }
    }
    let Some(track) = id.and_then(|id| movie.tracks.iter().find(|track| track.id == id)) else {
        return;
    };
    let Some(stbl) = media
        .and_then(|mdia| child_of(buf, mdia, *b"minf"))
        .and_then(|minf| child_of(buf, minf, *b"stbl"))
    else {
        return;
    };
    let mut children = Lenient::inside(stbl);
    while let Some(child) = children.next(buf) {
        match &child.kind {
            b"stsd" => clear_stsd(buf, child, track, collapsed.contains(&track.id)),
            b"sbgp" | b"sgpd" => free_if_seig(buf, child),
            _ => {}
        }
    }
}

fn clear_stsd(buf: &mut [u8], stsd: Header, track: &Track, collapse: bool) {
    if collapse {
        write_u32(buf, stsd.body + 4, 1);
    }
    let mut entries = Lenient::new(stsd.body + 8, stsd.end);
    while let Some(entry_box) = entries.next(buf) {
        let Some(entry) = track.entries.iter().find(|entry| entry.at == entry_box.at) else {
            continue;
        };
        if entry.protection.is_none() {
            continue;
        }
        retype(buf, entry_box, entry.format);
        let mut children = Lenient::new(entry.children_at, entry_box.end);
        while let Some(child) = children.next(buf) {
            if child.kind == *b"sinf" {
                retype(buf, child, FREE);
            }
        }
    }
}

fn clear_fragment(buf: &mut [u8], start: usize, collapsed: &[u32]) {
    let Some(moof) = lenient_at(buf, start, *b"moof") else {
        return;
    };
    let mut children = Lenient::inside(moof);
    while let Some(child) = children.next(buf) {
        match &child.kind {
            b"pssh" => retype(buf, child, FREE),
            b"traf" => {
                let mut boxes = Lenient::inside(child);
                while let Some(inner) = boxes.next(buf) {
                    match &inner.kind {
                        b"tfhd" => collapse_tfhd(buf, inner, collapsed),
                        b"senc" | b"saiz" | b"saio" => retype(buf, inner, FREE),
                        b"sbgp" | b"sgpd" => free_if_seig(buf, inner),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

/// Points a `tfhd` of a collapsed track at the first sample entry, when it names one.
fn collapse_tfhd(buf: &mut [u8], tfhd: Header, collapsed: &[u32]) {
    let Some(flags) = lenient_u32(buf, tfhd.body).map(|word| word & 0xFF_FFFF) else {
        return;
    };
    let Some(id) = lenient_u32(buf, tfhd.body + 4) else {
        return;
    };
    if flags & 0x2 == 0 || !collapsed.contains(&id) {
        return;
    }
    let index_at = tfhd.body + 8 + if flags & 0x1 != 0 { 8 } else { 0 };
    if index_at + 4 <= tfhd.end {
        write_u32(buf, index_at, 1);
    }
}

fn free_if_seig(buf: &mut [u8], header: Header) {
    if lenient_kind(buf, header.body + 4) == Some(SEIG) {
        retype(buf, header, FREE);
    }
}

/// The first child of `parent` of type `kind`.
fn child_of(buf: &[u8], parent: Header, kind: Kind) -> Option<Header> {
    let mut children = Lenient::inside(parent);
    while let Some(child) = children.next(buf) {
        if child.kind == kind {
            return Some(child);
        }
    }
    None
}

fn lenient_track_id(buf: &[u8], tkhd: Header) -> Option<u32> {
    let version = *buf.get(tkhd.body)?;
    let times = if version == 1 { 16 } else { 8 };
    let at = tkhd.body + 4 + times;
    (at + 4 <= tkhd.end).then(|| lenient_u32(buf, at)).flatten()
}

/// Where a sample entry's child boxes start, read from the entry itself.
fn lenient_children_at(buf: &[u8], entry: Header) -> Option<usize> {
    let version = slice(buf, entry.body + 8, 2).map_or(0, |b| u16::from_be_bytes([b[0], b[1]]));
    let fixed = fixed_fields(entry.kind, version)?;
    let at = entry.body + fixed;
    (at <= entry.end).then_some(at)
}

/// Writes `ids` into the `tenc` boxes of `init`'s sample entries whose `default_KID` is all
/// zero: entry `i`, counted over every track's entries in file order, takes `ids[i]`. Entries
/// with a key id already, without a `tenc`, or beyond `ids` are left alone.
pub fn write_key_ids(init: &mut [u8], ids: &[[u8; 16]]) {
    let mut entry_index = 0;
    let mut top = Lenient::new(0, init.len());
    while let Some(moov) = top.next(init) {
        if moov.kind != *b"moov" {
            continue;
        }
        let mut traks = Lenient::inside(moov);
        while let Some(trak) = traks.next(init) {
            if trak.kind != *b"trak" {
                continue;
            }
            let Some(stsd) = child_of(init, trak, *b"mdia")
                .and_then(|mdia| child_of(init, mdia, *b"minf"))
                .and_then(|minf| child_of(init, minf, *b"stbl"))
                .and_then(|stbl| child_of(init, stbl, *b"stsd"))
            else {
                continue;
            };
            let mut entries = Lenient::new(stsd.body + 8, stsd.end);
            while let Some(entry) = entries.next(init) {
                let id = ids.get(entry_index).copied();
                entry_index += 1;
                let (Some(id), Some(children_at)) = (id, lenient_children_at(init, entry)) else {
                    continue;
                };
                let Some(tenc) = child_of(
                    init,
                    Header {
                        body: children_at,
                        ..entry
                    },
                    *b"sinf",
                )
                .and_then(|sinf| child_of(init, sinf, *b"schi"))
                .and_then(|schi| child_of(init, schi, *b"tenc")) else {
                    continue;
                };
                let kid_at = tenc.body + 8;
                if kid_at + 16 <= tenc.end
                    && let Some(kid) = init.get_mut(kid_at..kid_at + 16)
                    && kid.iter().all(|byte| *byte == 0)
                {
                    kid.copy_from_slice(&id);
                }
            }
        }
    }
}

/// The duration of the last sample of `fragment`, a `moof` with the boxes around it: its `trun`
/// entry, else its `tfhd`'s default. `None` when neither says, or there is no `moof`.
pub fn last_sample_duration(fragment: &[u8]) -> Option<u32> {
    let mut top = Lenient::new(0, fragment.len());
    while let Some(moof) = top.next(fragment) {
        if moof.kind != *b"moof" {
            continue;
        }
        let mut last_traf = None;
        let mut children = Lenient::inside(moof);
        while let Some(child) = children.next(fragment) {
            if child.kind == *b"traf" {
                last_traf = Some(child);
            }
        }
        let traf = last_traf?;
        let mut default = None;
        let mut last_trun = None;
        let mut boxes = Lenient::inside(traf);
        while let Some(child) = boxes.next(fragment) {
            match &child.kind {
                b"tfhd" => default = tfhd_default_duration(fragment, child),
                b"trun" => last_trun = Some(child),
                _ => {}
            }
        }
        return last_trun
            .and_then(|trun| trun_last_duration(fragment, trun))
            .or(default);
    }
    None
}

fn tfhd_default_duration(data: &[u8], tfhd: Header) -> Option<u32> {
    let flags = lenient_u32(data, tfhd.body)? & 0xFF_FFFF;
    if flags & 0x8 == 0 {
        return None;
    }
    let mut at = tfhd.body + 8;
    if flags & 0x1 != 0 {
        at += 8;
    }
    if flags & 0x2 != 0 {
        at += 4;
    }
    (at + 4 <= tfhd.end)
        .then(|| lenient_u32(data, at))
        .flatten()
}

fn trun_last_duration(data: &[u8], trun: Header) -> Option<u32> {
    let flags = lenient_u32(data, trun.body)? & 0xFF_FFFF;
    if flags & 0x100 == 0 {
        return None;
    }
    let count = lenient_u32(data, trun.body + 4)?;
    let last = count.checked_sub(1)? as usize;
    let mut at = trun.body + 8;
    if flags & 0x1 != 0 {
        at += 4;
    }
    if flags & 0x4 != 0 {
        at += 4;
    }
    let per_sample = 4 * (flags & 0xF00).count_ones() as usize;
    let at = last.checked_mul(per_sample)?.checked_add(at)?;
    (at + 4 <= trun.end)
        .then(|| lenient_u32(data, at))
        .flatten()
}
