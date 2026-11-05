//! Fragments: where each sample is (ISO/IEC 14496-12 §8.8) and what decrypts it
//! (ISO/IEC 23001-7 §6, §7; 14496-12 §8.7.8, §8.7.9, §8.9).

use std::collections::HashMap;

use crate::Error;
use crate::boxes::{Body, Children, Header, Kind, skip};
use crate::claims::Claims;
use crate::movie::{Group, SampleEntry, Track, sgpd};
use crate::table::{Description, SampleTable};

/// Why a fragment could not be indexed.
pub(crate) enum Fault {
    Malformed(Error),
    /// Bytes the fragment needs have not arrived: the file up to `needs` is wanted.
    Incomplete {
        needs: usize,
    },
}

impl From<Error> for Fault {
    fn from(error: Error) -> Self {
        Fault::Malformed(error)
    }
}

/// Where a fragment's samples go, and what they are read against.
pub(crate) struct Context<'a> {
    pub tracks: &'a [Track],
    pub table: &'a mut SampleTable,
    /// The descriptions already in the table, so that equal ones are stored once.
    pub descriptions: &'a mut HashMap<Description, u32>,
    /// The bytes of the file that samples own: a sample with bytes is justified by them alone.
    pub claims: &'a mut Claims,
    /// How many more zero-size samples the `moof` justifies: one a byte of it, all its `trun`s
    /// together.
    pub zero_size_left: usize,
    pub steps: &'a mut usize,
}

impl Context<'_> {
    /// The index of `description` in the table, added if new; `blame` names the box in the error
    /// when the descriptions can no longer be indexed.
    fn intern(&mut self, description: Description, blame: Header) -> Result<u32, Error> {
        if let Some(&index) = self.descriptions.get(&description) {
            return Ok(index);
        }
        let index = self.table.push_description(description).ok_or_else(|| {
            Error::new(
                blame.at,
                blame.kind,
                "brings more distinct protection settings than can be indexed",
            )
        })?;
        self.descriptions.insert(description, index);
        Ok(index)
    }

    /// Gives the samples of the `trun` read in `body` the bytes `start..end`, which no other
    /// sample may own.
    fn own(&mut self, start: usize, end: usize, body: &Body<'_>) -> Result<(), Error> {
        if self.claims.claim(start, end) {
            return Ok(());
        }
        Err(body.fault(format!(
            "has samples on the bytes {start:#x}..{end:#x}, some of which another sample owns"
        )))
    }

    /// Counts `count` more zero-size samples of the `moof`, from the `trun` read in `body`.
    fn take_zero_size(&mut self, count: usize, body: &Body<'_>) -> Result<(), Error> {
        let left = self.zero_size_left;
        self.zero_size_left = left.checked_sub(count).ok_or_else(|| {
            body.fault(format!(
                "declares {count} zero-size samples where its `moof` has bytes for {left} more"
            ))
        })?;
        Ok(())
    }
}

const SEIG: Kind = *b"seig";

/// Indexes the samples of the `moof` at `header`, which is whole, into the context's table.
///
/// On an error the table holds the samples added so far: the caller drops them.
pub(crate) fn moof(data: &[u8], header: Header, cx: &mut Context<'_>) -> Result<(), Fault> {
    let mut previous_end = None;
    let mut children = Children::of(data, header, 1)?;
    while let Some(child) = children.next(cx.steps)? {
        match &child.kind {
            b"traf" => {
                let end = traf(data, child, children.depth, header, previous_end, cx)?;
                previous_end = Some(end);
            }
            _ => skip(data, child, children.depth, cx.steps)?,
        }
    }
    Ok(())
}

/// The boxes of a `traf` that matter, located before anything is read, since their order is not
/// fixed.
#[derive(Default)]
struct TrafBoxes {
    tfhd: Option<Header>,
    truns: Vec<Header>,
    senc: Option<Header>,
    saiz: Option<Header>,
    saio: Option<Header>,
    /// The `sbgp` of grouping type `seig`.
    sbgp: Option<Header>,
    /// The descriptions of the `sgpd` of grouping type `seig`.
    groups: Vec<Group>,
}

fn locate(data: &[u8], traf: Header, depth: u8, steps: &mut usize) -> Result<TrafBoxes, Error> {
    let mut found = TrafBoxes::default();
    let mut children = Children::of(data, traf, depth)?;
    while let Some(child) = children.next(steps)? {
        match &child.kind {
            b"tfhd" => set_once(&mut found.tfhd, child, traf)?,
            b"trun" => found.truns.push(child),
            b"senc" => set_once(&mut found.senc, child, traf)?,
            b"saiz" => {
                if aux_is_cenc(data, child)? {
                    set_once(&mut found.saiz, child, traf)?;
                }
            }
            b"saio" => {
                if aux_is_cenc(data, child)? {
                    set_once(&mut found.saio, child, traf)?;
                }
            }
            b"sbgp" => {
                if grouping_type(data, child)? == SEIG {
                    set_once(&mut found.sbgp, child, traf)?;
                }
            }
            b"sgpd" => {
                if let Some(groups) = sgpd(data, child, steps)? {
                    found.groups = groups;
                }
            }
            _ => skip(data, child, children.depth, steps)?,
        }
    }
    Ok(found)
}

fn set_once(slot: &mut Option<Header>, found: Header, parent: Header) -> Result<(), Error> {
    if slot.is_some() {
        return Err(Error::new(
            parent.at,
            parent.kind,
            format!("has two `{}` boxes", found.kind.escape_ascii()),
        ));
    }
    *slot = Some(found);
    Ok(())
}

/// Whether a `saiz` or `saio` describes CENC auxiliary information: it does unless its flags
/// name another `aux_info_type`.
fn aux_is_cenc(data: &[u8], header: Header) -> Result<bool, Error> {
    let mut body = Body::of(data, header);
    body.version()?;
    if body.flags()? & 1 == 0 {
        return Ok(true);
    }
    let kind: [u8; 4] = body.array("aux_info_type")?;
    Ok(matches!(&kind, b"cenc" | b"cbcs" | b"cens" | b"cbc1"))
}

fn grouping_type(data: &[u8], header: Header) -> Result<Kind, Error> {
    let mut body = Body::of(data, header);
    body.version()?;
    body.skip(3, "flags")?;
    body.array("grouping_type")
}

/// What a `tfhd` says (14496-12 §8.8.7).
struct TrackHeader {
    track_id: u32,
    base_data_offset: Option<usize>,
    description_index: Option<u32>,
    default_duration: Option<u32>,
    default_size: Option<u32>,
    default_base_is_moof: bool,
}

fn tfhd(data: &[u8], header: Header) -> Result<TrackHeader, Error> {
    let mut body = Body::of(data, header);
    body.version()?;
    let flags = body.flags()?;
    let track_id = body.u32("track_ID")?;
    let base_data_offset = if flags & 0x1 != 0 {
        let offset = body.u64("base_data_offset")?;
        Some(usize::try_from(offset).map_err(|_| {
            body.fault(format!(
                "has a base data offset of {offset}, more than this machine can address"
            ))
        })?)
    } else {
        None
    };
    let description_index = (flags & 0x2 != 0)
        .then(|| body.u32("sample_description_index"))
        .transpose()?;
    let default_duration = (flags & 0x8 != 0)
        .then(|| body.u32("default_sample_duration"))
        .transpose()?;
    let default_size = (flags & 0x10 != 0)
        .then(|| body.u32("default_sample_size"))
        .transpose()?;
    if flags & 0x20 != 0 {
        body.u32("default_sample_flags")?;
    }
    Ok(TrackHeader {
        track_id,
        base_data_offset,
        description_index,
        default_duration,
        default_size,
        default_base_is_moof: flags & 0x020000 != 0,
    })
}

/// What a `traf`'s samples share, resolved from its `tfhd` and the movie.
struct TrafInfo<'a> {
    /// What data offsets count from.
    base: usize,
    head: TrackHeader,
    track: &'a Track,
    entry: &'a SampleEntry,
    /// The entry's index in its track, from 0.
    entry_index: u32,
}

/// Indexes one `traf`; `previous_end` is where the previous `traf`'s data ended. Returns where
/// this one's ends.
fn traf(
    data: &[u8],
    header: Header,
    depth: u8,
    moof: Header,
    previous_end: Option<usize>,
    cx: &mut Context<'_>,
) -> Result<usize, Fault> {
    let boxes = locate(data, header, depth, cx.steps)?;
    let tfhd_box = boxes
        .tfhd
        .ok_or_else(|| Error::new(header.at, header.kind, "has no `tfhd`"))?;
    let head = tfhd(data, tfhd_box)?;
    let track = cx
        .tracks
        .iter()
        .find(|track| track.id == head.track_id)
        .ok_or_else(|| {
            Error::new(
                tfhd_box.at,
                tfhd_box.kind,
                format!(
                    "names track {}, which the movie does not have",
                    head.track_id
                ),
            )
        })?;
    let index = head
        .description_index
        .unwrap_or(track.defaults.sample_description_index);
    let entry = index
        .checked_sub(1)
        .and_then(|i| track.entries.get(i as usize))
        .ok_or_else(|| {
            Error::new(
                tfhd_box.at,
                tfhd_box.kind,
                format!(
                    "names sample entry {index} of a track with {} entries",
                    track.entries.len()
                ),
            )
        })?;
    let base = match (head.base_data_offset, previous_end) {
        (Some(base), _) => base,
        (None, Some(end)) if !head.default_base_is_moof => end,
        (None, _) => moof.at,
    };
    let info = TrafInfo {
        base,
        head,
        track,
        entry,
        entry_index: index - 1,
    };

    let first = cx.table.len();
    let mut run_end = base;
    let mut run_lengths = Vec::with_capacity(boxes.truns.len());
    for (n, trun_box) in boxes.truns.iter().enumerate() {
        let before = cx.table.len();
        run_end = trun(data, *trun_box, n == 0, run_end, &info, cx)?;
        run_lengths.push(cx.table.len() - before);
    }
    let count = cx.table.len() - first;

    match boxes.sbgp {
        Some(sbgp_box) => assign_groups(data, sbgp_box, first, count, &info, &boxes.groups, cx)?,
        None => {
            let own = cx.intern(describe(&info, None, header)?, header)?;
            for sample in first..first + count {
                cx.table.set_description(sample, own);
            }
        }
    }

    match (boxes.senc, boxes.saiz, boxes.saio) {
        (Some(senc_box), _, _) => senc(data, senc_box, first, count, cx)?,
        (None, Some(saiz_box), Some(saio_box)) => {
            aux_info(data, saiz_box, saio_box, first, &run_lengths, base, cx)?;
        }
        (None, Some(lone), None) | (None, None, Some(lone)) => {
            let other = if lone.kind == *b"saiz" {
                "saio"
            } else {
                "saiz"
            };
            return Err(
                Error::new(lone.at, lone.kind, format!("has no `{other}` beside it")).into(),
            );
        }
        (None, None, None) => {
            for sample in first..first + count {
                let description = cx.table.description(sample);
                if description.protected && description.iv_size > 0 {
                    return Err(Error::new(
                        header.at,
                        header.kind,
                        "has protected samples but no `senc`, nor `saiz` and `saio`",
                    )
                    .into());
                }
            }
        }
    }
    Ok(run_end)
}

/// Indexes one `trun`'s samples: their offsets, sizes and durations. `previous_end` is where the
/// previous `trun`'s data ended. Returns where this one's ends.
fn trun(
    data: &[u8],
    header: Header,
    first_run: bool,
    previous_end: usize,
    info: &TrafInfo<'_>,
    cx: &mut Context<'_>,
) -> Result<usize, Fault> {
    let base = info.base;
    let head = &info.head;
    let track = info.track;
    let mut body = Body::of(data, header);
    body.version()?;
    let flags = body.flags()?;
    let count = body.u32("sample_count")?;
    let start = if flags & 0x1 != 0 {
        let offset = body.i32("data_offset")?;
        base.checked_add_signed(offset as isize).ok_or_else(|| {
            body.fault(format!(
                "has a data offset of {offset}, before the file's start"
            ))
        })?
    } else if first_run {
        base
    } else {
        previous_end
    };
    if flags & 0x4 != 0 {
        body.u32("first_sample_flags")?;
    }
    let has_duration = flags & 0x100 != 0;
    let has_size = flags & 0x200 != 0;
    let has_flags = flags & 0x400 != 0;
    let has_offset = flags & 0x800 != 0;
    let per_sample = 4 * [has_duration, has_size, has_flags, has_offset]
        .iter()
        .filter(|present| **present)
        .count();
    let default_duration = head
        .default_duration
        .unwrap_or(track.defaults.sample_duration);
    let default_size = head.default_size.unwrap_or(track.defaults.sample_size);
    let count = count as usize;
    let beyond =
        |body: &Body<'_>| body.fault("has samples running past what this machine can address");

    // Before anything is reserved: the count against what justifies it (§9). Samples of a
    // default size own their bytes, which must have arrived; zero-size samples take a byte of
    // their `moof` each. Samples with fields in the `trun` are justified by those bytes first,
    // then, once their sizes are read, in the same way. None of these bounds moves as the file
    // arrives.
    if per_sample > 0 {
        let fits = count
            .checked_mul(per_sample)
            .is_some_and(|needed| needed <= body.left());
        if !fits {
            return Err(body
                .fault(format!("declares {count} samples in {} bytes", body.left()))
                .into());
        }
    } else if default_size > 0 {
        let end = count
            .checked_mul(default_size as usize)
            .and_then(|total| start.checked_add(total))
            .ok_or_else(|| beyond(&body))?;
        if end > data.len() {
            return Err(Fault::Incomplete { needs: end });
        }
        cx.own(start, end, &body)?;
    } else {
        cx.take_zero_size(count, &body)?;
    }

    cx.table.reserve(count);
    let mut zero_size = 0;
    let mut at = start;
    for _ in 0..count {
        *cx.steps += 1;
        let duration = if has_duration {
            body.u32("sample_duration")?
        } else {
            default_duration
        };
        let size = if has_size {
            body.u32("sample_size")?
        } else {
            default_size
        };
        if has_flags {
            body.u32("sample_flags")?;
        }
        if has_offset {
            body.u32("sample_composition_time_offset")?;
        }
        cx.table.push(at, size, duration);
        zero_size += usize::from(size == 0);
        at = at.checked_add(size as usize).ok_or_else(|| beyond(&body))?;
    }
    if per_sample > 0 {
        cx.own(start, at, &body)?;
        cx.take_zero_size(zero_size, &body)?;
    }
    Ok(at)
}

/// The description of the `traf`'s samples, under `group` when a `seig` names one. `blame` is
/// the box an error is laid to.
fn describe(
    info: &TrafInfo<'_>,
    group: Option<&Group>,
    blame: Header,
) -> Result<Description, Error> {
    let entry_index = info.entry_index;
    match (group, &info.entry.protection) {
        (None, None) => Ok(Description::clear(entry_index)),
        (None, Some(protection)) => {
            let mut constant_iv = [0; 16];
            let mut constant_iv_size = 0;
            if let Some(iv) = &protection.constant_iv {
                constant_iv[..iv.len()].copy_from_slice(iv);
                constant_iv_size = iv.len() as u8;
            }
            Ok(Description {
                entry: entry_index,
                protected: true,
                scheme: protection.scheme,
                pattern: protection.pattern,
                iv_size: protection.iv_size,
                key_id: protection.key_id,
                constant_iv,
                constant_iv_size,
            })
        }
        (Some(group), _) if !group.protected => Ok(Description::clear(entry_index)),
        (Some(_), None) => Err(Error::new(
            blame.at,
            blame.kind,
            format!(
                "protects samples of the clear sample entry {}",
                entry_index + 1
            ),
        )),
        (Some(group), Some(protection)) => {
            let eight = group.iv_size == 8 || (group.iv_size == 0 && group.constant_iv_size == 8);
            if protection.scheme.is_cbc() && eight {
                return Err(Error::new(
                    blame.at,
                    blame.kind,
                    "names a description with an 8-byte IV under a CBC scheme, which takes 16-byte IVs only",
                ));
            }
            Ok(Description {
                entry: entry_index,
                protected: true,
                scheme: protection.scheme,
                pattern: group.pattern,
                iv_size: group.iv_size,
                key_id: group.key_id,
                constant_iv: group.constant_iv,
                constant_iv_size: group.constant_iv_size,
            })
        }
    }
}

/// Gives the `traf`'s samples their descriptions as its `sbgp` maps them to `seig` groups, those
/// of the track's `sgpd` or of `traf_groups`, the fragment's own.
fn assign_groups(
    data: &[u8],
    header: Header,
    first: usize,
    count: usize,
    info: &TrafInfo<'_>,
    traf_groups: &[Group],
    cx: &mut Context<'_>,
) -> Result<(), Error> {
    let movie_groups = &info.track.groups;
    let mut body = Body::of(data, header);
    let version = body.version()?;
    body.skip(3, "flags")?;
    body.array::<4>("grouping_type")?;
    if version == 1 {
        body.u32("grouping_type_parameter")?;
    }
    let entries = body.u32("entry_count")?;
    let fits = (entries as usize)
        .checked_mul(8)
        .is_some_and(|needed| needed <= body.left());
    if !fits {
        return Err(body.fault(format!(
            "declares {entries} entries in {} bytes",
            body.left()
        )));
    }
    let mut cursor: usize = 0;
    for _ in 0..entries {
        *cx.steps += 1;
        let run = body.u32("sample_count")? as usize;
        let group = body.u32("group_description_index")?;
        let end = cursor
            .checked_add(run)
            .filter(|end| *end <= count)
            .ok_or_else(|| {
                body.fault(format!("maps more samples than the {count} of its `traf`"))
            })?;
        let found = match group {
            0 => None,
            1..=0xFFFF => Some(movie_groups.get(group as usize - 1).ok_or_else(|| {
                body.fault(format!(
                    "names description {group}, which the track's `sgpd` does not have"
                ))
            })?),
            0x10000 => {
                return Err(body.fault("names description 0x10000, which no `sgpd` holds"));
            }
            _ => Some(traf_groups.get((group - 0x10001) as usize).ok_or_else(|| {
                body.fault(format!(
                    "names description {group:#x}, which the fragment's `sgpd` does not have"
                ))
            })?),
        };
        let description = cx.intern(describe(info, found, header)?, header)?;
        for sample in first + cursor..first + end {
            cx.table.set_description(sample, description);
        }
        cursor = end;
    }
    if cursor != count {
        return Err(body.fault(format!(
            "maps {cursor} samples where its `traf` has {count}"
        )));
    }
    Ok(())
}

/// Reads the IVs and subsamples of a `senc` (23001-7 §7.2) for samples `first..first + count`.
fn senc(
    data: &[u8],
    header: Header,
    first: usize,
    count: usize,
    cx: &mut Context<'_>,
) -> Result<(), Error> {
    let mut body = Body::of(data, header);
    body.version()?;
    let subsampled = body.flags()? & 0x2 != 0;
    let declared = body.u32("sample_count")?;
    if declared as usize != count {
        return Err(body.fault(format!(
            "counts {declared} samples where its `traf` has {count}"
        )));
    }
    for sample in first..first + count {
        *cx.steps += 1;
        let iv_size = cx.table.description(sample).iv_size as usize;
        let iv_at = push_iv(&mut body, iv_size, cx)?;
        let (runs_at, runs) = if subsampled {
            subsamples(&mut body, sample, sample - first, cx)?
        } else {
            (0, 0)
        };
        cx.table.set_aux(sample, iv_at, runs_at, runs);
    }
    if body.left() != 0 {
        return Err(body.fault(format!(
            "has {} bytes left after its {count} samples: its length does not fit their IV size",
            body.left()
        )));
    }
    Ok(())
}

/// Reads a sample's IV from `body` into the table; where it went.
fn push_iv(body: &mut Body<'_>, iv_size: usize, cx: &mut Context<'_>) -> Result<u32, Error> {
    let iv = body.bytes(iv_size, "IV")?;
    cx.table
        .push_iv(iv)
        .ok_or_else(|| body.fault("brings more IV bytes than can be indexed"))
}

/// Reads a sample's subsample runs from `body` into the table: where they went and how many.
/// `nth` is the sample's index in its `traf`, for the error.
fn subsamples(
    body: &mut Body<'_>,
    sample: usize,
    nth: usize,
    cx: &mut Context<'_>,
) -> Result<(u32, u16), Error> {
    let runs = body.u16("subsample_count")?;
    if usize::from(runs) * 6 > body.left() {
        return Err(body.fault(format!(
            "declares {runs} subsamples in {} bytes",
            body.left()
        )));
    }
    let runs_at = cx
        .table
        .runs_end()
        .ok_or_else(|| body.fault("brings more subsample runs than can be indexed"))?;
    cx.table.reserve_runs(usize::from(runs));
    let mut total: u64 = 0;
    for _ in 0..runs {
        *cx.steps += 1;
        let clear = body.u16("BytesOfClearData")?;
        let protected = body.u32("BytesOfProtectedData")?;
        total += u64::from(clear) + u64::from(protected);
        cx.table.push_run(u32::from(clear), protected);
    }
    let size = cx.table.size(sample);
    if runs > 0 && total != u64::from(size) {
        return Err(body.fault(format!(
            "gives sample {nth} subsamples of {total} bytes in all where the sample has {size}"
        )));
    }
    Ok((runs_at, runs))
}

/// Reads the IVs and subsamples that `saiz` and `saio` locate (14496-12 §8.7.8, §8.7.9), for the
/// runs of `run_lengths` samples starting at `first`; offsets count from `base`.
fn aux_info(
    data: &[u8],
    saiz: Header,
    saio: Header,
    first: usize,
    run_lengths: &[usize],
    base: usize,
    cx: &mut Context<'_>,
) -> Result<(), Fault> {
    let count: usize = run_lengths.iter().sum();
    let mut sizes = Body::of(data, saiz);
    sizes.version()?;
    if sizes.flags()? & 1 != 0 {
        sizes.skip(8, "aux_info_type and its parameter")?;
    }
    let default_size = sizes.u8("default_sample_info_size")?;
    let declared = sizes.u32("sample_count")?;
    if declared as usize != count {
        return Err(sizes
            .fault(format!(
                "counts {declared} samples where its `traf` has {count}"
            ))
            .into());
    }
    let each = if default_size == 0 {
        Some(sizes.bytes(count, "sample_info_size")?)
    } else {
        None
    };

    let mut offsets = Body::of(data, saio);
    let version = offsets.version()?;
    if offsets.flags()? & 1 != 0 {
        offsets.skip(8, "aux_info_type and its parameter")?;
    }
    let entries = offsets.u32("entry_count")? as usize;
    if entries != 1 && entries != run_lengths.len() {
        return Err(offsets
            .fault(format!(
                "gives {entries} offsets for {} runs of samples",
                run_lengths.len()
            ))
            .into());
    }
    let width = if version == 0 { 4 } else { 8 };
    if entries
        .checked_mul(width)
        .is_none_or(|needed| needed > offsets.left())
    {
        return Err(offsets
            .fault(format!(
                "declares {entries} offsets in {} bytes",
                offsets.left()
            ))
            .into());
    }

    let mut sample = first;
    let mut at = base;
    for (run, length) in run_lengths.iter().enumerate() {
        if run == 0 || entries > 1 {
            let offset = if version == 0 {
                u64::from(offsets.u32("offset")?)
            } else {
                offsets.u64("offset")?
            };
            at = usize::try_from(offset)
                .ok()
                .and_then(|offset| base.checked_add(offset))
                .ok_or_else(|| {
                    offsets.fault(format!(
                        "has an offset of {offset}, more than this machine can address"
                    ))
                })?;
        }
        for _ in 0..*length {
            *cx.steps += 1;
            let nth = sample - first;
            let size = each.map_or(default_size, |sizes| sizes[nth]) as usize;
            let end = at.checked_add(size).ok_or_else(|| {
                offsets.fault("locates information past what this machine can address")
            })?;
            if end > data.len() {
                return Err(Fault::Incomplete { needs: end });
            }
            let mut info = Body::of(
                data,
                Header {
                    body: at,
                    end,
                    ..saio
                },
            );
            let iv_size = cx.table.description(sample).iv_size as usize;
            if size < iv_size {
                return Err(info
                    .fault(format!(
                        "gives sample {nth} {size} bytes of information, fewer than its {iv_size}-byte IV"
                    ))
                    .into());
            }
            let iv_at = push_iv(&mut info, iv_size, cx)?;
            let (runs_at, runs) = if size > iv_size {
                subsamples(&mut info, sample, nth, cx)?
            } else {
                (0, 0)
            };
            if info.left() != 0 {
                return Err(info
                    .fault(format!(
                        "gives sample {nth} {} bytes of information more than its IV and subsamples take",
                        info.left()
                    ))
                    .into());
            }
            cx.table.set_aux(sample, iv_at, runs_at, runs);
            sample += 1;
            at = end;
        }
    }
    Ok(())
}
