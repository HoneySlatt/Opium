//! Locating and decrypting the CENC samples of a fragmented MP4, as Apple Music
//! and SoundCloud serve them.
//!
//! Adapted from Kopuz (<https://github.com/Kopuz-org/kopuz>, revision a58c385,
//! `daemon/server/src/applemusic/cenc.rs`), Copyright (c) 2026 Temidaradev and the
//! Kopuz contributors, EUPL-1.2, redistributed under the GPL-3.0-or-later through
//! the EUPL's compatibility clause.

fn u32be(d: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}
fn u64be(d: &[u8], o: usize) -> u64 {
    u64::from_be_bytes([
        d[o],
        d[o + 1],
        d[o + 2],
        d[o + 3],
        d[o + 4],
        d[o + 5],
        d[o + 6],
        d[o + 7],
    ])
}

const ENCA: u32 = u32::from_be_bytes(*b"enca");
const ENCV: u32 = u32::from_be_bytes(*b"encv");
const STSD: u32 = u32::from_be_bytes(*b"stsd");
const MOOV: u32 = u32::from_be_bytes(*b"moov");
const MOOF: u32 = u32::from_be_bytes(*b"moof");
const MDAT: u32 = u32::from_be_bytes(*b"mdat");
const TRAK: u32 = u32::from_be_bytes(*b"trak");
const TKHD: u32 = u32::from_be_bytes(*b"tkhd");
const SINF: u32 = u32::from_be_bytes(*b"sinf");
const SCHI: u32 = u32::from_be_bytes(*b"schi");
const TENC: u32 = u32::from_be_bytes(*b"tenc");
const TRAF: u32 = u32::from_be_bytes(*b"traf");
const SENC: u32 = u32::from_be_bytes(*b"senc");
const TRUN: u32 = u32::from_be_bytes(*b"trun");
const TFHD: u32 = u32::from_be_bytes(*b"tfhd");

fn read_box(data: &[u8], pos: usize) -> Option<(usize, usize, usize)> {
    if pos + 8 > data.len() {
        return None;
    }
    let size = u32be(data, pos) as usize;
    if size == 1 {
        if pos + 16 > data.len() {
            return None;
        }
        let ext = usize::try_from(u64be(data, pos + 8)).ok()?;
        let body_start = pos + 16;
        let body_end = pos.checked_add(ext)?;
        // A size too small for its own header would never move on.
        if ext < 16 || body_end > data.len() {
            return None;
        }
        Some((body_start, body_end, ext))
    } else if size >= 8 {
        let body_start = pos + 8;
        let body_end = pos + size;
        if body_end > data.len() {
            return None;
        }
        Some((body_start, body_end, size))
    } else {
        None
    }
}

fn box_type(data: &[u8], pos: usize) -> u32 {
    u32::from_be_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
}

fn find_child(
    data: &[u8],
    body_start: usize,
    body_end: usize,
    target: u32,
) -> Option<(usize, usize, usize)> {
    let mut pos = body_start;
    while pos < body_end {
        let (bs, be, total) = match read_box(data, pos) {
            Some(v) => v,
            None => break,
        };
        if box_type(data, pos) == target {
            return Some((bs, be, total));
        }
        pos += total;
    }
    None
}

fn find_deep(data: &[u8], start: usize, end: usize, target: u32) -> Option<(usize, usize)> {
    let mut pos = start;
    while pos < end {
        let (bs, be, total) = match read_box(data, pos) {
            Some(v) => v,
            None => break,
        };
        if box_type(data, pos) == target {
            return Some((bs, be));
        }
        if let Some(found) = find_deep(data, bs, be, target) {
            return Some(found);
        }
        pos += total;
    }
    None
}

/// Each child as `(box_start, body_start, body_end)`.
///
/// `body_end` comes from [`read_box`], which is the only place that knows
/// whether the box carried a 32- or 64-bit size. Deriving it at the call site
/// as `body_start + total - 8` is right for a 32-bit box and eight bytes too
/// long for a 64-bit one, which walks the child scan into the next sibling.
fn find_all_children(
    data: &[u8],
    body_start: usize,
    body_end: usize,
) -> Vec<(usize, usize, usize)> {
    let mut result = Vec::new();
    let mut pos = body_start;
    while pos < body_end {
        let (bs, be, total) = match read_box(data, pos) {
            Some(v) => v,
            None => break,
        };
        result.push((pos, bs, be));
        pos += total;
    }
    result
}

#[derive(Debug, Clone)]
struct TrackInfo {
    track_id: u32,
    default_iv_size: u8,
}

/// The tracks, the `enca`/`encv` boxes, and the sample entry counts above
/// one, as [`Init`] keeps them.
type TrackBoxes = (Vec<TrackInfo>, Vec<usize>, Vec<usize>);

fn extract_track_info(data: &[u8], init_end: usize) -> Result<TrackBoxes, String> {
    let mut track_infos = Vec::new();
    let mut enca_positions = Vec::new();
    let mut several_entries = Vec::new();

    let (moov_body_start, moov_body_end) = match find_deep(data, 0, init_end, MOOV) {
        Some(v) => v,
        None => return Ok((track_infos, enca_positions, several_entries)),
    };

    for (trak_box_start, trak_body_start, trak_body_end) in
        find_all_children(data, moov_body_start, moov_body_end)
    {
        if box_type(data, trak_box_start) != TRAK {
            continue;
        }

        let track_id = find_child(data, trak_body_start, trak_body_end, TKHD)
            .filter(|(bs, be, _)| bs < be)
            .and_then(|(s, be, _)| {
                let offset = if data[s] == 0 { 12 } else { 20 };
                (s + offset + 4 <= be).then(|| u32be(data, s + offset))
            })
            .unwrap_or(0);

        if let Some((stsd_bs, stsd_be)) = find_deep(data, trak_body_start, trak_body_end, STSD) {
            if stsd_bs + 8 <= stsd_be && u32be(data, stsd_bs + 4) > 1 {
                several_entries.push(stsd_bs + 4);
            }
            let entries_start = stsd_bs + 8;
            let mut epos = entries_start;
            while epos < stsd_be {
                let (es, ee, etotal) = match read_box(data, epos) {
                    Some(v) => v,
                    None => break,
                };
                let etype = box_type(data, epos);

                if etype == ENCA || etype == ENCV {
                    let children_start = es + 28;
                    let default_iv = get_tenc_iv_size(data, children_start, ee);
                    track_infos.push(TrackInfo {
                        track_id,
                        default_iv_size: default_iv,
                    });
                    enca_positions.push(epos);
                }
                epos += etotal;
            }
        }
        break;
    }

    Ok((track_infos, enca_positions, several_entries))
}

fn get_tenc_iv_size(data: &[u8], enca_body_start: usize, enca_body_end: usize) -> u8 {
    if let Some((sinf_bs, sinf_be, _)) = find_child(data, enca_body_start, enca_body_end, SINF)
        && let Some((schi_bs, schi_be, _)) = find_child(data, sinf_bs, sinf_be, SCHI)
        && let Some((tenc_bs, _, _)) = find_child(data, schi_bs, schi_be, TENC)
        && tenc_bs + 8 <= data.len()
    {
        return data[tenc_bs + 7];
    }
    16
}

/// Per-sample decryption inputs read out of a `senc` box: each sample's 16-byte
/// IV, paired with its subsample layout as `(clear_bytes, encrypted_bytes)`
/// runs (empty when the sample is encrypted whole).
type SencSamples = (Vec<[u8; 16]>, Vec<Vec<(u16, u32)>>);

fn parse_senc(iv_size: u8, sample_count: u32, raw_data: &[u8], use_subsample: bool) -> SencSamples {
    if iv_size == 0 && sample_count == 0 {
        return (vec![], vec![]);
    }

    if use_subsample {
        if iv_size != 0
            && let Some(result) = try_parse_senc(iv_size, sample_count, raw_data, true)
        {
            return result;
        }
        for try_size in [0u8, 8, 16] {
            if try_size == iv_size {
                continue;
            }
            if let Some(result) = try_parse_senc(try_size, sample_count, raw_data, true) {
                return result;
            }
        }
    } else {
        if iv_size != 0
            && let Some(result) = try_parse_senc(iv_size, sample_count, raw_data, false)
        {
            return result;
        }
        for try_size in [0u8, 8, 16] {
            if try_size == iv_size {
                continue;
            }
            if let Some(result) = try_parse_senc(try_size, sample_count, raw_data, false) {
                return result;
            }
        }
    }
    (vec![], vec![])
}

fn try_parse_senc(
    iv_size: u8,
    sample_count: u32,
    raw_data: &[u8],
    use_subsample: bool,
) -> Option<SencSamples> {
    let count = sample_count as usize;
    let mut pos = 0usize;
    let mut ivs = Vec::with_capacity(count);
    let mut subs = Vec::with_capacity(count);

    for _ in 0..count {
        if iv_size > 0 {
            if raw_data.len().saturating_sub(pos) < iv_size as usize {
                return None;
            }
            let mut iv = [0u8; 16];
            iv[..iv_size as usize].copy_from_slice(&raw_data[pos..pos + iv_size as usize]);
            ivs.push(iv);
            pos += iv_size as usize;
        }
        if use_subsample {
            if raw_data.len().saturating_sub(pos) < 2 {
                return None;
            }
            let n = u16::from_be_bytes([raw_data[pos], raw_data[pos + 1]]) as usize;
            pos += 2;
            if raw_data.len().saturating_sub(pos) < n * 6 {
                return None;
            }
            let mut patterns = Vec::with_capacity(n);
            for _ in 0..n {
                let clear = u16::from_be_bytes([raw_data[pos], raw_data[pos + 1]]);
                let protected = u32::from_be_bytes([
                    raw_data[pos + 2],
                    raw_data[pos + 3],
                    raw_data[pos + 4],
                    raw_data[pos + 5],
                ]);
                patterns.push((clear, protected));
                pos += 6;
            }
            subs.push(patterns);
        } else {
            subs.push(vec![]);
        }
    }

    if pos != raw_data.len() {
        return None;
    }

    Some((ivs, subs))
}

/// One encrypted sample: where it sits in the file, and what the CDM needs to
/// turn it into cleartext.
///
/// Offsets are absolute and identical in ciphertext and cleartext — CENC is
/// size-preserving and every box is copied verbatim — which is what lets a
/// sample be decrypted in isolation, in place, in any order.
#[derive(Debug, Clone)]
pub struct EncryptedSample {
    /// Where the `moof` of this sample's fragment begins.
    pub fragment: usize,
    pub start: usize,
    pub len: usize,
    pub iv: [u8; 16],
    pub subs: Vec<(u16, u32)>,
    /// Which of the track's sample entries its fragment names, from 0:
    /// Apple's enhanced files have one per key.
    pub description: u32,
}

impl EncryptedSample {
    pub fn end(&self) -> usize {
        self.start + self.len
    }
}

/// Everything needed to decrypt a track without walking its boxes again.
#[derive(Debug, Default)]
pub struct Fmp4Layout {
    /// `enca`/`encv` box offsets to be relabelled `mp4a` so decoders accept the
    /// now-cleartext track.
    pub enca_positions: Vec<usize>,
    /// The sample entry counts above one, as [`Init`] keeps them.
    pub several_entries: Vec<usize>,
    /// Every encrypted sample, in file order.
    pub samples: Vec<EncryptedSample>,
}

/// Walk the fMP4 once and record where every encrypted sample lives.
///
/// Pure parsing — no CDM calls — so it's cheap (~10ms for a 7 MB track) and can
/// run before the first byte is needed.
pub fn index_fmp4(data: &[u8]) -> Result<Fmp4Layout, String> {
    let init = Init::read(data)?.ok_or_else(|| "no moov".to_string())?;
    let mut layout = Fmp4Layout {
        enca_positions: init.enca_positions.clone(),
        several_entries: init.several_entries.clone(),
        samples: Vec::new(),
    };
    init.fragments(data, init.end, &mut layout.samples);
    Ok(layout)
}

/// What the movie box says of the tracks, to index their fragments with.
#[derive(Debug, Clone)]
pub struct Init {
    track_infos: Vec<TrackInfo>,
    /// `enca`/`encv` box offsets, as in [`Fmp4Layout`].
    pub enca_positions: Vec<usize>,
    /// Where a track's sample entry count is above one: Apple's enhanced
    /// files have one entry per key, all the same audio, and decoders that
    /// take a single entry refuse them. Once decrypted, the first stands
    /// for all: see [`one_sample_entry`].
    pub several_entries: Vec<usize>,
    /// Where the movie box ends: the fragments follow.
    pub end: usize,
}

impl Init {
    /// The movie box of `data`, once it is all there: `None` until then.
    pub fn read(data: &[u8]) -> Result<Option<Self>, String> {
        let mut pos = 0;
        while pos + 8 <= data.len() {
            let Some((_, be, total)) = read_box(data, pos) else {
                return Ok(None);
            };
            if box_type(data, pos) == MOOV {
                let (track_infos, enca_positions, several_entries) = extract_track_info(data, be)?;
                return Ok(Some(Self {
                    track_infos,
                    enca_positions,
                    several_entries,
                    end: be,
                }));
            }
            pos += total;
        }
        Ok(None)
    }

    /// Index the fragments of `data` from `pos` into `samples`, each fragment
    /// once its `moof` and `mdat` are both there; returns where the first
    /// fragment not yet whole begins, to go on from when more has come.
    pub fn fragments(
        &self,
        data: &[u8],
        mut pos: usize,
        samples: &mut Vec<EncryptedSample>,
    ) -> usize {
        while pos + 8 <= data.len() {
            let Some((moof_bs, moof_be, moof_total)) = read_box(data, pos) else {
                break;
            };
            if box_type(data, pos) != MOOF {
                pos += moof_total;
                continue;
            }
            let moof_pos = pos;
            let mdat_pos = moof_pos + moof_total;
            let Some((_, mdat_be, mdat_total_size)) = read_box(data, mdat_pos) else {
                break;
            };
            if box_type(data, mdat_pos) != MDAT {
                pos = moof_be;
                continue;
            }
            self.fragment(
                data,
                moof_pos,
                moof_bs,
                moof_be,
                mdat_pos,
                mdat_total_size,
                samples,
            );
            pos = mdat_be;
        }
        pos
    }

    #[allow(clippy::too_many_arguments)]
    fn fragment(
        &self,
        data: &[u8],
        moof_pos: usize,
        moof_bs: usize,
        moof_be: usize,
        mdat_pos: usize,
        mdat_total_size: usize,
        out: &mut Vec<EncryptedSample>,
    ) {
        let moof_start_pos = moof_pos as u64;
        let mdat_body_start = mdat_pos + 8;
        let mdat_payload_offset = mdat_body_start as u64;

        for (traf_pos, traf_bs, traf_be) in find_all_children(data, moof_bs, moof_be) {
            if box_type(data, traf_pos) != TRAF {
                continue;
            }

            let tfhd = find_child(data, traf_bs, traf_be, TFHD);
            let track_id = tfhd
                .as_ref()
                .filter(|(bs, be, _)| bs + 8 <= *be)
                .map(|(s, _, _)| u32be(data, s + 4))
                .unwrap_or(0);

            let Some(ti) = self.track_infos.iter().find(|t| t.track_id == track_id) else {
                continue;
            };
            let description = tfhd
                .as_ref()
                .and_then(|&(bs, be, _)| tfhd_description_at(data, bs, be))
                .map_or(0, |at| u32be(data, at).saturating_sub(1));
            let per_sample_iv_size = ti.default_iv_size;

            let mut traf_ivs: Vec<[u8; 16]> = Vec::new();
            let mut traf_subs: Vec<Vec<(u16, u32)>> = Vec::new();
            if let Some((senc_bs, senc_be, _)) = find_child(data, traf_bs, traf_be, SENC)
                && senc_bs + 8 <= senc_be
            {
                let flags = u32be(data, senc_bs);
                let sample_count = u32be(data, senc_bs + 4);
                let raw = &data[senc_bs + 8..senc_be];
                let use_subsample = (flags & 0x02) != 0;
                let (ivs, subs) = parse_senc(per_sample_iv_size, sample_count, raw, use_subsample);
                traf_ivs = ivs;
                traf_subs = subs;
            }

            let trun = find_child(data, traf_bs, traf_be, TRUN);
            let (trun_data_offset, samples) = match trun {
                Some((trun_bs, trun_be, _)) => parse_trun(
                    data,
                    trun_bs,
                    trun_be,
                    tfhd,
                    moof_start_pos,
                    mdat_payload_offset,
                ),
                None => (0, vec![]),
            };
            if samples.is_empty() {
                continue;
            }

            let mdat_body_len = mdat_total_size.saturating_sub(8);
            if mdat_body_start + mdat_body_len > data.len() {
                continue;
            }

            let mut offset = trun_data_offset;
            let mut iv = [0u8; 16];
            for (i, &sz) in samples.iter().enumerate() {
                let sz = sz as usize;
                if sz == 0 {
                    continue;
                }
                if i < traf_ivs.len() {
                    iv = traf_ivs[i];
                }
                if offset + sz > mdat_body_len {
                    break;
                }
                out.push(EncryptedSample {
                    fragment: moof_pos,
                    start: mdat_body_start + offset,
                    len: sz,
                    iv,
                    subs: traf_subs.get(i).cloned().unwrap_or_default(),
                    description,
                });
                offset += sz;
            }
        }
    }
}

/// How long the last sample of the fragment starting `data` lasts, in the
/// track's timescale: its `trun` entry's, else the `tfhd` default. An AAC
/// encode's last frame is shorter than the others by the padding after the
/// music.
pub fn last_sample_duration(data: &[u8]) -> Option<u32> {
    let (moof_bs, moof_be, _) = read_box(data, 0).filter(|_| box_type(data, 0) == MOOF)?;
    let (traf_bs, traf_be, _) = find_child(data, moof_bs, moof_be, TRAF)?;
    let default = find_child(data, traf_bs, traf_be, TFHD).and_then(|(bs, be, _)| {
        let flags = u32be(data, bs) & 0x00ff_ffff;
        // Version and flags, the track id, then what the flags say is there.
        let mut at = bs + 8;
        if flags & 0x01 != 0 {
            at += 8;
        }
        if flags & 0x02 != 0 {
            at += 4;
        }
        (flags & 0x08 != 0 && at + 4 <= be).then(|| u32be(data, at))
    });
    let (bs, be, _) = find_child(data, traf_bs, traf_be, TRUN)?;
    let flags = u32be(data, bs) & 0x00ff_ffff;
    let count = u32be(data, bs + 4) as usize;
    if flags & 0x100 == 0 || count == 0 {
        return default;
    }
    let header = 8 + 4 * usize::from(flags & 0x01 != 0) + 4 * usize::from(flags & 0x04 != 0);
    let entry = 4 * [0x100, 0x200, 0x400, 0x800]
        .iter()
        .filter(|&&f| flags & f != 0)
        .count();
    let last = bs + header + (count - 1) * entry;
    (last + 4 <= be).then(|| u32be(data, last))
}

/// Relabel `enca`/`encv` as `mp4a` so the decoder reads the track as plain AAC.
/// Only the 4-byte box type changes, so every offset is preserved.
pub fn patch_init(buf: &mut [u8], layout: &Fmp4Layout) {
    for pos in &layout.enca_positions {
        if pos + 8 <= buf.len() {
            buf[pos + 4..pos + 8].copy_from_slice(b"mp4a");
        }
    }
    one_sample_entry(buf, &layout.several_entries);
    if !layout.several_entries.is_empty() {
        let mut fragments: Vec<usize> = layout.samples.iter().map(|s| s.fragment).collect();
        fragments.dedup();
        for moof in fragments {
            first_sample_entry(buf, moof);
        }
    }
}

/// Leave each track of `several_entries` (see [`Init::several_entries`])
/// with its first sample entry only, which then stands for all: its count
/// says one, the others stay behind it unread. Their fragments name the
/// first through [`first_sample_entry`].
pub fn one_sample_entry(buf: &mut [u8], several_entries: &[usize]) {
    for &at in several_entries {
        if let Some(count) = buf.get_mut(at..at + 4) {
            count.copy_from_slice(&1u32.to_be_bytes());
        }
    }
}

/// Have the fragment whose `moof` starts at `moof` name its track's first
/// sample entry, where it names another.
pub fn first_sample_entry(buf: &mut [u8], moof: usize) {
    let Some((moof_bs, moof_be, _)) = read_box(buf, moof).filter(|_| box_type(buf, moof) == MOOF)
    else {
        return;
    };
    let mut at = Vec::new();
    for (traf_pos, traf_bs, traf_be) in find_all_children(buf, moof_bs, moof_be) {
        if box_type(buf, traf_pos) == TRAF
            && let Some((bs, be, _)) = find_child(buf, traf_bs, traf_be, TFHD)
            && let Some(index) = tfhd_description_at(buf, bs, be)
        {
            at.push(index);
        }
    }
    for index in at {
        buf[index..index + 4].copy_from_slice(&1u32.to_be_bytes());
    }
}

/// Where a `tfhd` body from `bs` to `be` holds its sample description
/// index, when it has one: after the track id, and the base data offset
/// when there is one.
fn tfhd_description_at(data: &[u8], bs: usize, be: usize) -> Option<usize> {
    let flags = u32be(data, bs) & 0x00ff_ffff;
    if flags & 0x02 == 0 {
        return None;
    }
    let at = bs + 8 + if flags & 0x01 != 0 { 8 } else { 0 };
    (at + 4 <= be).then_some(at)
}

fn parse_trun(
    data: &[u8],
    trun_bs: usize,
    trun_be: usize,
    tfhd: Option<(usize, usize, usize)>,
    moof_start_pos: u64,
    mdat_payload_offset: u64,
) -> (usize, Vec<u32>) {
    if trun_bs + 8 > trun_be {
        return (0, vec![]);
    }

    let trun_flags = u32be(data, trun_bs);
    let sample_count = u32be(data, trun_bs + 4) as usize;
    let mut tpos = trun_bs + 8;

    let mut has_data_offset = false;
    let mut trun_data_offset_i32 = 0i32;
    let mut has_first_sample_flags = false;

    if trun_flags & 0x000001 != 0 && tpos + 4 <= trun_be {
        trun_data_offset_i32 = i32::from_be_bytes(data[tpos..tpos + 4].try_into().unwrap());
        has_data_offset = true;
        tpos += 4;
    }
    if trun_flags & 0x000004 != 0 && tpos + 4 <= trun_be {
        has_first_sample_flags = true;
        tpos += 4;
    }

    let mut durations = Vec::with_capacity(sample_count);
    let mut sizes = Vec::with_capacity(sample_count);
    let mut flags = Vec::with_capacity(sample_count);
    let mut composition_offsets = Vec::with_capacity(sample_count);

    for i in 0..sample_count {
        if trun_flags & 0x000100 != 0 && tpos + 4 <= trun_be {
            durations.push(u32be(data, tpos));
            tpos += 4;
        }
        if trun_flags & 0x000200 != 0 && tpos + 4 <= trun_be {
            sizes.push(u32be(data, tpos));
            tpos += 4;
        }
        if trun_flags & 0x000400 != 0 && tpos + 4 <= trun_be {
            flags.push(u32be(data, tpos));
            tpos += 4;
        } else if i == 0 && has_first_sample_flags {
            flags.push(u32be(
                data,
                trun_bs + 8 + if has_data_offset { 4 } else { 0 },
            ));
        }
        if trun_flags & 0x000800 != 0 && tpos + 4 <= trun_be {
            composition_offsets.push(i32::from_be_bytes(data[tpos..tpos + 4].try_into().unwrap()));
            tpos += 4;
        }
    }

    if sizes.is_empty()
        && sample_count > 0
        && let Some((tfhd_bs, _, _)) = tfhd
    {
        let tfhd_version_flags = u32be(data, tfhd_bs);
        let tfhd_flags = tfhd_version_flags & 0x00FFFFFF;
        let mut off = tfhd_bs + 8;
        if tfhd_flags & 0x000001 != 0 {
            off += 8;
        }
        if tfhd_flags & 0x000002 != 0 {
            off += 4;
        }
        if tfhd_flags & 0x000008 != 0 {
            off += 4;
        }
        if tfhd_flags & 0x000010 != 0 && off + 4 <= data.len() {
            let def_size = u32be(data, off);
            if def_size > 0 {
                sizes = vec![def_size; sample_count];
            }
        }
    }

    let mut data_start: usize = 0;
    if has_data_offset {
        let base_offset = moof_start_pos.wrapping_add(trun_data_offset_i32 as i64 as u64);
        if base_offset >= mdat_payload_offset {
            data_start = (base_offset - mdat_payload_offset) as usize;
        }
    }

    (data_start, sizes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_last_sample_says_how_much_the_frame_was_cut() {
        let be = |v: u32| v.to_be_bytes();
        // tfhd: default-base-is-moof and a default duration of 1024.
        let tfhd = [be(0x0002_0008), be(1), be(1024)].concat();
        // trun: a data offset, then a duration and a size per sample.
        let trun = [
            be(0x301),
            be(2),
            be(100),
            be(1024),
            be(300),
            be(412),
            be(12),
        ]
        .concat();
        let traf = boxed(
            b"traf",
            &[boxed(b"tfhd", &tfhd), boxed(b"trun", &trun)].concat(),
        );
        let moof = boxed(b"moof", &traf);
        assert_eq!(last_sample_duration(&moof), Some(412));
        let plain = [be(0x201), be(1), be(100), be(300)].concat();
        let traf = boxed(
            b"traf",
            &[boxed(b"tfhd", &tfhd), boxed(b"trun", &plain)].concat(),
        );
        assert_eq!(last_sample_duration(&boxed(b"moof", &traf)), Some(1024));
    }

    use super::*;

    /// `size` counts the header, so `body` of length 0 is a legal 8-byte box.
    fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        out
    }

    fn concat(parts: &[Vec<u8>]) -> Vec<u8> {
        parts.iter().flatten().copied().collect()
    }

    /// An init segment whose sole track is encrypted and has id 1, so a
    /// fragment naming that id gets past the track lookup and reaches the
    /// `senc` parse.
    fn encrypted_init() -> Vec<u8> {
        let mut tkhd = vec![0u8; 20];
        tkhd[12..16].copy_from_slice(&1u32.to_be_bytes());

        let stsd = concat(&[vec![0, 0, 0, 0, 0, 0, 0, 1], boxed(b"enca", &[0u8; 28])]);
        concat(&[
            boxed(b"ftyp", b"isom"),
            boxed(
                b"moov",
                &boxed(
                    b"trak",
                    &concat(&[boxed(b"tkhd", &tkhd), boxed(b"stsd", &stsd)]),
                ),
            ),
        ])
    }

    /// Apple's enhanced files: a sample entry per key, each with its own
    /// constant IV, and fragments naming theirs.
    #[test]
    fn several_sample_entries_read_as_one_and_samples_say_theirs() {
        let be = |v: u32| v.to_be_bytes();
        let entry = |iv: u8| {
            let mut tenc = vec![1, 0, 0, 0, 0, 0, 1, 0];
            tenc.extend_from_slice(&[0; 16]);
            tenc.push(16);
            tenc.extend_from_slice(&[iv; 16]);
            let sinf = boxed(b"sinf", &boxed(b"schi", &boxed(b"tenc", &tenc)));
            boxed(b"enca", &concat(&[vec![0u8; 28], sinf]))
        };
        let mut tkhd = vec![0u8; 20];
        tkhd[12..16].copy_from_slice(&1u32.to_be_bytes());
        let stsd = concat(&[be(0).to_vec(), be(2).to_vec(), entry(1), entry(2)]);
        let init = concat(&[
            boxed(b"ftyp", b"isom"),
            boxed(
                b"moov",
                &boxed(
                    b"trak",
                    &concat(&[boxed(b"tkhd", &tkhd), boxed(b"stsd", &stsd)]),
                ),
            ),
        ]);
        // default-base-is-moof, and the second sample entry.
        let tfhd = boxed(b"tfhd", &[be(0x0002_0002), be(1), be(2)].concat());
        let trun_len = 8 + 16;
        let moof_len = 8 + 8 + tfhd.len() + trun_len;
        let trun = boxed(
            b"trun",
            &[be(0x201), be(1), be(moof_len as u32 + 8), be(4)].concat(),
        );
        let moof = boxed(b"moof", &boxed(b"traf", &concat(&[tfhd, trun])));
        let mut data = concat(&[init.clone(), moof, boxed(b"mdat", &[9; 4])]);

        let ivs: Vec<Vec<u8>> = tencs(&data)
            .into_iter()
            .filter_map(|t| t.constant_iv)
            .collect();
        assert_eq!(ivs, [vec![1; 16], vec![2; 16]]);
        let layout = index_fmp4(&data).unwrap();
        assert_eq!(layout.samples.len(), 1);
        assert_eq!(layout.samples[0].description, 1);
        assert_eq!(
            &data[layout.samples[0].start..layout.samples[0].end()],
            &[9; 4]
        );

        patch_init(&mut data, &layout);
        let count = layout.several_entries[0];
        assert_eq!(u32be(&data, count), 1);
        let tfhd = data.windows(4).position(|w| w == b"tfhd").unwrap() + 4;
        assert_eq!(u32be(&data, tfhd + 8), 1);
    }

    /// A box can be well-formed as a box and still too short to hold the fields
    /// its type implies. `index_fmp4` parses bytes straight off the network
    /// under `State`'s mutex, so a panic here poisons the mutex and leaves the
    /// track permanently unplayable — a far worse outcome than the empty index
    /// a malformed box should produce.
    #[test]
    fn short_boxes_do_not_panic_the_indexer() {
        let empty = boxed(b"tkhd", &[]);

        let stunted = boxed(b"tkhd", &[1, 0, 0, 0]);
        for trak_body in [empty, stunted] {
            let data = concat(&[
                boxed(b"ftyp", b"isom"),
                boxed(b"moov", &boxed(b"trak", &trak_body)),
            ]);
            let index = index_fmp4(&data).expect("a moov is present");
            assert!(index.samples.is_empty(), "no moof, so no samples");
        }

        for senc_body in [Vec::new(), vec![0u8; 4], vec![0u8; 7]] {
            let traf = boxed(
                b"traf",
                &concat(&[
                    boxed(b"tfhd", &[0, 0, 0, 0, 0, 0, 0, 1]),
                    boxed(b"senc", &senc_body),
                ]),
            );
            let data = concat(&[
                encrypted_init(),
                boxed(b"moof", &traf),
                boxed(b"mdat", &[0u8; 16]),
            ]);
            let index = index_fmp4(&data).expect("a moov is present");
            assert!(index.samples.is_empty(), "no trun, so no samples");
        }
    }

    /// The fragment walk requires a readable `mdat` after each `moof`, which
    /// puts at least eight bytes beyond every box inside that `moof`. That is
    /// what keeps the fixed-offset reads on `tfhd` and `senc`'s header in
    /// bounds, so the guards there are belt-and-braces rather than load-bearing
    /// — this pins the precondition they lean on.
    #[test]
    fn a_fragment_is_only_walked_when_an_mdat_follows_it() {
        let traf = boxed(b"traf", &boxed(b"tfhd", &[]));
        let truncated = concat(&[encrypted_init(), boxed(b"moof", &traf)]);
        assert!(
            index_fmp4(&truncated)
                .expect("a moov is present")
                .samples
                .is_empty(),
            "a moof with nothing after it is not walked at all"
        );

        let with_mdat = concat(&[truncated.clone(), boxed(b"mdat", &[0u8; 16])]);
        let moof_end = truncated.len();
        assert!(
            moof_end + 8 <= with_mdat.len(),
            "the mdat guarantees eight readable bytes past the end of the moof"
        );
    }
}

/// A track's protection defaults, from its `tenc` box.
#[derive(Debug, Clone, PartialEq)]
pub struct Tenc {
    pub key_id: [u8; 16],
    /// `cbcs`'s crypt:skip blocks; 0:0 encrypts every whole block.
    pub pattern: (u32, u32),
    /// The IV every sample shares, when samples carry none of their own.
    pub constant_iv: Option<Vec<u8>>,
}

/// The body of the first `kind` box anywhere in `data`, found by its four
/// characters: init segments are small and these boxes are unique in them.
fn find_box_anywhere<'a>(data: &'a [u8], kind: &[u8; 4]) -> Option<&'a [u8]> {
    (4..data.len().saturating_sub(4)).find_map(|i| {
        if &data[i..i + 4] != kind {
            return None;
        }
        let size = u32be(data, i - 4) as usize;
        (size >= 8 && i - 4 + size <= data.len()).then(|| &data[i + 4..i - 4 + size])
    })
}

pub fn tenc(init: &[u8]) -> Option<Tenc> {
    read_tenc(find_box_anywhere(init, b"tenc")?)
}

/// Every sample entry's protection, in their order: one per key in
/// Apple's enhanced files (see [`EncryptedSample::description`]).
pub fn tencs(init: &[u8]) -> Vec<Tenc> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(body) = init
        .get(from..)
        .and_then(|rest| find_box_anywhere(rest, b"tenc"))
    {
        // Where this one's body starts, to look on after it.
        let at = body.as_ptr() as usize - init.as_ptr() as usize;
        found.extend(read_tenc(body));
        from = at;
    }
    found
}

fn read_tenc(body: &[u8]) -> Option<Tenc> {
    // version, flags (3), reserved, pattern (version 1) or reserved,
    // is_protected, per-sample IV size, key id (16), then a constant IV.
    if body.len() < 24 {
        return None;
    }
    let pattern = if body[0] >= 1 {
        (u32::from(body[5] >> 4), u32::from(body[5] & 0x0f))
    } else {
        (0, 0)
    };
    let mut key_id = [0u8; 16];
    key_id.copy_from_slice(&body[8..24]);
    let constant_iv = (body[6] == 1 && body[7] == 0 && body.len() > 24)
        .then(|| {
            let n = body[24] as usize;
            body.get(25..25 + n).map(<[u8]>::to_vec)
        })
        .flatten();
    Some(Tenc {
        key_id,
        pattern,
        constant_iv,
    })
}

/// The AAC `AudioSpecificConfig` in an init segment's `esds`.
pub fn audio_specific_config(init: &[u8]) -> Option<Vec<u8>> {
    let body = find_box_anywhere(init, b"esds")?;
    // Full box header, then descriptors: tag, length (1 to 4 bytes of 7 bits).
    let mut pos = 4;
    let read_len = |pos: &mut usize| -> Option<usize> {
        let mut len = 0usize;
        for _ in 0..4 {
            let b = *body.get(*pos)?;
            *pos += 1;
            len = (len << 7) | usize::from(b & 0x7f);
            if b & 0x80 == 0 {
                break;
            }
        }
        Some(len)
    };
    while pos < body.len() {
        let tag = body[pos];
        pos += 1;
        let len = read_len(&mut pos)?;
        match tag {
            // ES_Descriptor: id (2), flags (1), then its children.
            0x03 => {
                let flags = *body.get(pos + 2)?;
                pos += 3;
                if flags & 0x80 != 0 {
                    pos += 2;
                }
                if flags & 0x40 != 0 {
                    pos += 1 + usize::from(*body.get(pos)?);
                }
                if flags & 0x20 != 0 {
                    pos += 2;
                }
            }
            // DecoderConfigDescriptor: 13 bytes, then the specific info.
            0x04 => pos += 13,
            0x05 => return body.get(pos..pos + len).map(<[u8]>::to_vec),
            _ => pos += len,
        }
    }
    None
}

#[cfg(test)]
mod init_tests {
    use super::*;

    fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        out.extend_from_slice(kind);
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn tenc_reads_key_pattern_and_constant_iv() {
        let mut body = vec![1, 0, 0, 0, 0, 0x19, 1, 0];
        body.extend_from_slice(&[7; 16]);
        body.push(16);
        body.extend_from_slice(&[9; 16]);
        let t = tenc(&boxed(b"tenc", &body)).unwrap();
        assert_eq!(t.key_id, [7; 16]);
        assert_eq!(t.pattern, (1, 9));
        assert_eq!(t.constant_iv, Some(vec![9; 16]));
    }

    #[test]
    fn esds_yields_the_audio_specific_config() {
        // ES_Descriptor(id 1, no flags) > DecoderConfig(13 bytes) > specific info.
        let mut body = vec![0, 0, 0, 0, 0x03, 22, 0, 1, 0, 0x04, 17];
        body.extend_from_slice(&[0x40, 0x15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        body.extend_from_slice(&[0x05, 2, 0x12, 0x10]);
        let asc = audio_specific_config(&boxed(b"esds", &body)).unwrap();
        assert_eq!(asc, vec![0x12, 0x10]);
    }
}
