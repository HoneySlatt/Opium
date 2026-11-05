//! The movie box: tracks, their sample entries and protection (ISO/IEC 14496-12 §8,
//! ISO/IEC 23001-7 §8).

use crate::Error;
use crate::boxes::{Body, Children, Found, Header, Kind, find, skip};

/// The movie box of a file: what every fragment needs to be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Movie {
    /// Every track of the file, in the order of their `trak` boxes.
    pub tracks: Vec<Track>,
    /// The `pssh` boxes found in the movie box, in order.
    pub pssh: Vec<Pssh>,
    /// Where the `moov` box ends: the first byte after it.
    pub end: usize,
    /// Where the `moov` box starts.
    pub(crate) at: usize,
}

/// One track of the movie.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    /// Its `track_ID`, which each fragment's `tfhd` names.
    pub id: u32,
    /// Units per second of its durations, from `mdhd`.
    pub timescale: u32,
    /// Its sample entries, in the order of `stsd`; a fragment names one by its index plus one.
    pub entries: Vec<SampleEntry>,
    /// The `trex` values, all zero when the movie has no `trex` for this track.
    pub defaults: TrackDefaults,
    /// The `seig` sample group descriptions of its `stbl`, which fragments may name.
    pub(crate) groups: Vec<Group>,
}

/// A track's defaults for its fragments, from `trex` (14496-12 §8.8.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackDefaults {
    /// Which sample entry the fragments use, counted from 1; 0 when there is no `trex`.
    pub sample_description_index: u32,
    /// The duration of a sample whose `trun` and `tfhd` give none, in the track's timescale.
    pub sample_duration: u32,
    /// The size in bytes of a sample whose `trun` and `tfhd` give none.
    pub sample_size: u32,
    /// The flags of a sample whose `trun` and `tfhd` give none (14496-12 §8.8.3.1): kept as
    /// read, since nothing here needs them to locate or decrypt a sample.
    pub sample_flags: u32,
}

/// One sample entry of a track's `stsd`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SampleEntry {
    /// Where the entry's box starts.
    pub at: usize,
    /// The original format: what `frma` names for an encrypted entry, the entry's own type
    /// otherwise.
    pub format: [u8; 4],
    /// How its samples are encrypted; `None` for a clear entry.
    pub protection: Option<Protection>,
    /// The `AudioSpecificConfig` of its `esds`, when it has one.
    pub audio_config: Option<Vec<u8>>,
    /// Where the entry's child boxes start, after its fixed fields.
    pub(crate) children_at: usize,
}

/// How an entry's samples are encrypted, from `schm` and `tenc`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Protection {
    /// The scheme `schm` names: which cipher and mode decrypt the samples.
    pub scheme: Scheme,
    /// The `default_KID`; all zero when the keys are named elsewhere.
    pub key_id: [u8; 16],
    /// Bytes of IV each sample carries: 0, 8 or 16.
    pub iv_size: u8,
    /// The IV every sample shares when `iv_size` is 0: 8 or 16 bytes.
    pub constant_iv: Option<Vec<u8>>,
    /// The crypt:skip pattern; 0:0 means none.
    pub pattern: Pattern,
}

/// A protection scheme of ISO/IEC 23001-7.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Scheme {
    /// AES-128-CTR.
    Cenc,
    /// AES-128-CTR with a pattern.
    Cens,
    /// AES-128-CBC.
    Cbc1,
    /// AES-128-CBC with a pattern and a constant IV.
    Cbcs,
}

/// A crypt:skip pattern of 16-byte blocks (23001-7 §10.4); 0:0 means every block is encrypted.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Pattern {
    /// How many 16-byte blocks are encrypted at the start of each crypt:skip cycle.
    pub crypt: u8,
    /// How many 16-byte blocks then stay clear before the next cycle starts.
    pub skip: u8,
}

/// A `pssh` box (23001-7 §8.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pssh {
    /// The DRM system the box is meant for, as its 16-byte UUID.
    pub system_id: [u8; 16],
    /// The key ids a version 1 box lists; empty for version 0.
    pub key_ids: Vec<[u8; 16]>,
    /// The system's own data, as the box holds it: opaque to this crate.
    pub data: Vec<u8>,
}

/// A `seig` sample group description (23001-7 §6), which overrides the `tenc` for its samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Group {
    pub protected: bool,
    pub iv_size: u8,
    pub key_id: [u8; 16],
    pub pattern: Pattern,
    /// The constant IV, padded; `constant_iv_size` says how many bytes the file gave.
    pub constant_iv: [u8; 16],
    pub constant_iv_size: u8,
}

const SEIG: Kind = *b"seig";

impl Movie {
    /// Reads the movie box of `data`, the file's bytes known so far from its start.
    ///
    /// Boxes before the `moov` are skipped. Returns `None` until the whole `moov` has arrived;
    /// a `moov` of size 0, which runs to the end of the file, is never whole here. Every `trak`
    /// is read; a `pssh` found in the `moov` is reported.
    ///
    /// # Errors
    ///
    /// A malformed box: a size below its header or past its container, a nest deeper than 16, a
    /// count larger than the bytes left, a `tenc` or `seig` with an IV size other than 0, 8 or
    /// 16, an 8-byte IV under a CBC scheme, an encrypted entry missing its `sinf`, `schm` or
    /// `tenc`, a scheme other than `cenc`, `cens`, `cbc1` or `cbcs`.
    pub fn read(data: &[u8]) -> Result<Option<Movie>, Error> {
        read(data, false, &mut 0)
    }
}

/// Reads the movie box; `whole` says the data is the whole file, so that a `moov` of size 0 ends
/// with it. `steps` counts the boxes and entries read.
pub(crate) fn read(data: &[u8], whole: bool, steps: &mut usize) -> Result<Option<Movie>, Error> {
    let mut at = 0;
    loop {
        let header = match find(data, at)? {
            Found::Cut => return Ok(None),
            Found::Open { kind, body } => {
                if !whole {
                    return Ok(None);
                }
                Header {
                    at,
                    kind,
                    body,
                    end: data.len(),
                }
            }
            Found::Sized(header) => {
                if header.end > data.len() {
                    return Ok(None);
                }
                header
            }
        };
        *steps += 1;
        if header.kind == *b"moov" {
            return moov(data, header, steps).map(Some);
        }
        at = header.end;
    }
}

fn moov(data: &[u8], header: Header, steps: &mut usize) -> Result<Movie, Error> {
    let mut tracks = Vec::new();
    let mut pssh = Vec::new();
    let mut defaults = Vec::new();
    let mut children = Children::of(data, header, 1)?;
    while let Some(child) = children.next(steps)? {
        match &child.kind {
            b"trak" => tracks.push(trak(data, child, children.depth, steps)?),
            b"mvex" => mvex(data, child, children.depth, steps, &mut defaults)?,
            b"pssh" => pssh.push(parse_pssh(data, child)?),
            _ => skip(data, child, children.depth, steps)?,
        }
    }
    for track in &mut tracks {
        if let Some((_, values)) = defaults.iter().find(|(id, _)| *id == track.id) {
            track.defaults = *values;
        }
    }
    Ok(Movie {
        tracks,
        pssh,
        end: header.end,
        at: header.at,
    })
}

fn trak(data: &[u8], header: Header, depth: u8, steps: &mut usize) -> Result<Track, Error> {
    let mut id = None;
    let mut media = None;
    let mut children = Children::of(data, header, depth)?;
    while let Some(child) = children.next(steps)? {
        match &child.kind {
            b"tkhd" => id = Some(track_id(data, child)?),
            b"mdia" => media = Some(mdia(data, child, children.depth, steps)?),
            _ => skip(data, child, children.depth, steps)?,
        }
    }
    let missing = |what| Error::new(header.at, header.kind, format!("has no `{what}`"));
    let id = id.ok_or_else(|| missing("tkhd"))?;
    let media = media.ok_or_else(|| missing("mdia"))?;
    Ok(Track {
        id,
        timescale: media.timescale,
        entries: media.entries,
        defaults: TrackDefaults::default(),
        groups: media.groups,
    })
}

/// What a `mdia` gives its track.
struct Media {
    timescale: u32,
    entries: Vec<SampleEntry>,
    groups: Vec<Group>,
}

fn track_id(data: &[u8], header: Header) -> Result<u32, Error> {
    let mut body = Body::of(data, header);
    let version = body.version()?;
    body.skip(3, "flags")?;
    body.skip(timed_fields(version), "creation and modification times")?;
    body.u32("track_ID")
}

/// The bytes of the two times that open `tkhd` and `mdhd`: 64 bits each in version 1.
fn timed_fields(version: u8) -> usize {
    if version == 1 { 16 } else { 8 }
}

fn mdia(data: &[u8], header: Header, depth: u8, steps: &mut usize) -> Result<Media, Error> {
    let mut timescale = None;
    let mut samples = None;
    let mut children = Children::of(data, header, depth)?;
    while let Some(child) = children.next(steps)? {
        match &child.kind {
            b"mdhd" => timescale = Some(parse_timescale(data, child)?),
            b"minf" => samples = Some(minf(data, child, children.depth, steps)?),
            _ => skip(data, child, children.depth, steps)?,
        }
    }
    let missing = |what| Error::new(header.at, header.kind, format!("has no `{what}`"));
    let timescale = timescale.ok_or_else(|| missing("mdhd"))?;
    let (entries, groups) = samples.ok_or_else(|| missing("minf"))?;
    Ok(Media {
        timescale,
        entries,
        groups,
    })
}

fn parse_timescale(data: &[u8], header: Header) -> Result<u32, Error> {
    let mut body = Body::of(data, header);
    let version = body.version()?;
    body.skip(3, "flags")?;
    body.skip(timed_fields(version), "creation and modification times")?;
    body.u32("timescale")
}

type Samples = (Vec<SampleEntry>, Vec<Group>);

fn minf(data: &[u8], header: Header, depth: u8, steps: &mut usize) -> Result<Samples, Error> {
    let mut samples = None;
    let mut children = Children::of(data, header, depth)?;
    while let Some(child) = children.next(steps)? {
        match &child.kind {
            b"stbl" => samples = Some(stbl(data, child, children.depth, steps)?),
            _ => skip(data, child, children.depth, steps)?,
        }
    }
    samples.ok_or_else(|| Error::new(header.at, header.kind, "has no `stbl`"))
}

fn stbl(data: &[u8], header: Header, depth: u8, steps: &mut usize) -> Result<Samples, Error> {
    let mut entries = None;
    let mut groups = Vec::new();
    let mut children = Children::of(data, header, depth)?;
    while let Some(child) = children.next(steps)? {
        match &child.kind {
            b"stsd" => entries = Some(stsd(data, child, children.depth, steps)?),
            b"sgpd" => {
                if let Some(found) = sgpd(data, child, steps)? {
                    groups = found;
                }
            }
            _ => skip(data, child, children.depth, steps)?,
        }
    }
    let entries = entries.ok_or_else(|| Error::new(header.at, header.kind, "has no `stsd`"))?;
    Ok((entries, groups))
}

fn stsd(
    data: &[u8],
    header: Header,
    depth: u8,
    steps: &mut usize,
) -> Result<Vec<SampleEntry>, Error> {
    let mut body = Body::of(data, header);
    body.version()?;
    body.skip(3, "flags")?;
    let count = body.u32("entry_count")?;
    let fits = (count as usize)
        .checked_mul(8)
        .is_some_and(|n| n <= body.left());
    if !fits {
        return Err(body.fault(format!(
            "declares {count} sample entries in {} bytes",
            body.left()
        )));
    }
    let mut entries = Vec::with_capacity(count as usize);
    let mut children = body.children(depth)?;
    for n in 0..count {
        let Some(child) = children.next(steps)? else {
            return Err(body.fault(format!("declares {count} sample entries but holds {n}")));
        };
        entries.push(sample_entry(data, child, children.depth, steps)?);
    }
    Ok(entries)
}

/// The fixed fields of a sample entry before its child boxes (14496-12 §8.5.2, §12.1.3, §12.2.3),
/// which say how it is laid out.
enum Layout {
    Audio,
    Visual,
    /// A format this crate does not look into: the entry is reported by its type alone.
    Opaque,
}

fn layout(kind: Kind) -> Layout {
    match &kind {
        b"enca" | b"mp4a" => Layout::Audio,
        b"encv" => Layout::Visual,
        _ => Layout::Opaque,
    }
}

/// How many bytes of fixed fields a sample entry of `kind` has before its child boxes: 28 for
/// audio, which QuickTime's `version` 1 and 2 (the 16-bit field at body offset 8) lengthen by 16
/// and 36; 78 for video. `None` for a format this crate does not look into.
pub(crate) fn fixed_fields(kind: Kind, version: u16) -> Option<usize> {
    match layout(kind) {
        Layout::Audio => Some(match version {
            1 => 44,
            2 => 64,
            _ => 28,
        }),
        Layout::Visual => Some(78),
        Layout::Opaque => None,
    }
}

fn sample_entry(
    data: &[u8],
    header: Header,
    depth: u8,
    steps: &mut usize,
) -> Result<SampleEntry, Error> {
    let mut body = Body::of(data, header);
    body.skip(6, "reserved bytes")?;
    body.u16("data_reference_index")?;
    let version = match layout(header.kind) {
        Layout::Audio => body.u16("version")?,
        Layout::Visual | Layout::Opaque => 0,
    };
    let Some(fixed) = fixed_fields(header.kind, version) else {
        return Ok(SampleEntry {
            at: header.at,
            format: header.kind,
            protection: None,
            audio_config: None,
            children_at: header.end,
        });
    };
    let mut body = Body::of(data, header);
    body.skip(fixed, "fixed fields")?;
    let children_at = body.at();
    let mut protection = None;
    let mut audio_config = None;
    let mut children = body.children(depth)?;
    while let Some(child) = children.next(steps)? {
        match &child.kind {
            b"sinf" => protection = Some(sinf(data, child, children.depth, steps)?),
            b"esds" => audio_config = esds(data, child, steps)?,
            _ => skip(data, child, children.depth, steps)?,
        }
    }
    let encrypted = matches!(&header.kind, b"enca" | b"encv");
    let (format, protection) = match protection {
        Some((format, protection)) if encrypted => (format, Some(protection)),
        Some(_) => (header.kind, None),
        None if encrypted => return Err(body.fault("has no `sinf`")),
        None => (header.kind, None),
    };
    Ok(SampleEntry {
        at: header.at,
        format,
        protection,
        audio_config,
        children_at,
    })
}

fn sinf(
    data: &[u8],
    header: Header,
    depth: u8,
    steps: &mut usize,
) -> Result<([u8; 4], Protection), Error> {
    let mut format = None;
    let mut scheme = None;
    let mut tenc_box = None;
    let mut children = Children::of(data, header, depth)?;
    while let Some(child) = children.next(steps)? {
        match &child.kind {
            b"frma" => format = Some(Body::of(data, child).array("data_format")?),
            b"schm" => scheme = Some(schm(data, child)?),
            b"schi" => {
                let mut inner = Children::of(data, child, children.depth)?;
                while let Some(grandchild) = inner.next(steps)? {
                    match &grandchild.kind {
                        b"tenc" => tenc_box = Some(grandchild),
                        _ => skip(data, grandchild, inner.depth, steps)?,
                    }
                }
            }
            _ => skip(data, child, children.depth, steps)?,
        }
    }
    let missing = |what| Error::new(header.at, header.kind, format!("has no `{what}`"));
    let format = format.ok_or_else(|| missing("frma"))?;
    let scheme = scheme.ok_or_else(|| missing("schm"))?;
    let tenc_box = tenc_box.ok_or_else(|| missing("tenc"))?;
    Ok((format, tenc(data, tenc_box, scheme)?))
}

fn schm(data: &[u8], header: Header) -> Result<Scheme, Error> {
    let mut body = Body::of(data, header);
    body.version()?;
    body.skip(3, "flags")?;
    let kind: [u8; 4] = body.array("scheme_type")?;
    body.u32("scheme_version")?;
    match &kind {
        b"cenc" => Ok(Scheme::Cenc),
        b"cens" => Ok(Scheme::Cens),
        b"cbc1" => Ok(Scheme::Cbc1),
        b"cbcs" => Ok(Scheme::Cbcs),
        _ => Err(body.fault(format!(
            "names the scheme `{}`, which is not one of cenc, cens, cbc1 or cbcs",
            kind.escape_ascii()
        ))),
    }
}

fn tenc(data: &[u8], header: Header, scheme: Scheme) -> Result<Protection, Error> {
    let mut body = Body::of(data, header);
    let version = body.version()?;
    body.skip(3, "flags")?;
    body.u8("reserved byte")?;
    let blocks = body.u8("pattern byte")?;
    let pattern = if version >= 1 {
        Pattern::from_byte(blocks)
    } else {
        Pattern::default()
    };
    let protected = body.u8("default_isProtected")?;
    let iv_size = body.u8("default_Per_Sample_IV_Size")?;
    let key_id = body.array("default_KID")?;
    if protected == 0 {
        return Err(body.fault("says its samples are not protected by default"));
    }
    check_iv_size(&body, scheme, iv_size, "per-sample")?;
    let constant_iv = if iv_size == 0 {
        let size = body.u8("default_constant_IV_size")?;
        check_constant_iv_size(&body, scheme, size)?;
        Some(body.bytes(size as usize, "default_constant_IV")?.to_vec())
    } else {
        None
    };
    Ok(Protection {
        scheme,
        key_id,
        iv_size,
        constant_iv,
        pattern,
    })
}

impl Pattern {
    /// The crypt blocks in the high nibble, the skip blocks in the low one.
    pub(crate) fn from_byte(byte: u8) -> Self {
        Self {
            crypt: byte >> 4,
            skip: byte & 0xF,
        }
    }
}

impl Scheme {
    /// Whether the scheme is a CBC one, which takes 16-byte IVs only.
    pub(crate) fn is_cbc(self) -> bool {
        matches!(self, Scheme::Cbc1 | Scheme::Cbcs)
    }
}

/// A per-sample IV size must be 0, 8 or 16; 8 is not allowed under a CBC scheme.
pub(crate) fn check_iv_size(
    body: &Body<'_>,
    scheme: Scheme,
    size: u8,
    which: &str,
) -> Result<(), Error> {
    match size {
        0 | 16 => Ok(()),
        8 if !scheme.is_cbc() => Ok(()),
        8 => Err(body.fault(format!(
            "gives an 8-byte {which} IV under a CBC scheme, which takes 16-byte IVs only"
        ))),
        _ => Err(body.fault(format!("gives a {which} IV size of {size}, not 0, 8 or 16"))),
    }
}

/// A constant IV is 8 or 16 bytes; 16 under a CBC scheme.
pub(crate) fn check_constant_iv_size(
    body: &Body<'_>,
    scheme: Scheme,
    size: u8,
) -> Result<(), Error> {
    match size {
        16 => Ok(()),
        8 if !scheme.is_cbc() => Ok(()),
        8 => Err(body
            .fault("gives an 8-byte constant IV under a CBC scheme, which takes 16-byte IVs only")),
        _ => Err(body.fault(format!("gives a constant IV size of {size}, not 8 or 16"))),
    }
}

/// The `DecoderSpecificInfo` of an `esds` (ISO/IEC 14496-14 §5.6, 14496-1 §7.2.6): the
/// `AudioSpecificConfig`, when the descriptors hold one.
fn esds(data: &[u8], header: Header, steps: &mut usize) -> Result<Option<Vec<u8>>, Error> {
    let mut body = Body::of(data, header);
    body.version()?;
    body.skip(3, "flags")?;
    let mut descriptors = Descriptors::new(data, header, body.at(), header.end);
    while let Some(descriptor) = descriptors.next(steps)? {
        if descriptor.tag != 3 {
            continue;
        }
        let mut es = Body::of(data, descriptor.header);
        es.u16("ES_ID")?;
        let flags = es.u8("ES flags")?;
        if flags & 0x80 != 0 {
            es.u16("dependsOn_ES_ID")?;
        }
        if flags & 0x40 != 0 {
            let length = es.u8("URLlength")?;
            es.skip(length as usize, "URL")?;
        }
        if flags & 0x20 != 0 {
            es.u16("OCR_ES_Id")?;
        }
        let mut inner = Descriptors::new(data, header, es.at(), descriptor.header.end);
        while let Some(config) = inner.next(steps)? {
            if config.tag != 4 {
                continue;
            }
            let mut decoder = Body::of(data, config.header);
            decoder.skip(13, "DecoderConfigDescriptor fields")?;
            let mut specific = Descriptors::new(data, header, decoder.at(), config.header.end);
            while let Some(info) = specific.next(steps)? {
                if info.tag == 5 {
                    return Ok(Some(data[info.header.body..info.header.end].to_vec()));
                }
            }
        }
    }
    Ok(None)
}

/// A descriptor of 14496-1 §7.2.6: a tag byte, a length of one to four bytes of seven bits,
/// the high bit saying another follows, then its body.
struct Descriptor {
    tag: u8,
    /// The descriptor's body as a box-like range, for reading it field by field.
    header: Header,
}

/// The descriptors laid out in `[at, end)` of an `esds`.
struct Descriptors<'a> {
    data: &'a [u8],
    esds: Header,
    at: usize,
    end: usize,
}

impl<'a> Descriptors<'a> {
    fn new(data: &'a [u8], esds: Header, at: usize, end: usize) -> Self {
        Self {
            data,
            esds,
            at,
            end,
        }
    }

    fn fault(&self, what: String) -> Error {
        Error::new(self.esds.at, self.esds.kind, what)
    }

    fn next(&mut self, steps: &mut usize) -> Result<Option<Descriptor>, Error> {
        if self.at >= self.end {
            return Ok(None);
        }
        *steps += 1;
        let header = Header {
            body: self.at,
            end: self.end,
            ..self.esds
        };
        let mut body = Body::of(self.data, header);
        let tag = body.u8("descriptor tag")?;
        let mut length: usize = 0;
        let mut more = true;
        let mut bytes = 0;
        while more {
            if bytes == 4 {
                return Err(self.fault("has a descriptor length of more than four bytes".into()));
            }
            let byte = body.u8("descriptor length")?;
            more = byte & 0x80 != 0;
            length = (length << 7) | usize::from(byte & 0x7F);
            bytes += 1;
        }
        let body_at = body.at();
        let body_end = body_at
            .checked_add(length)
            .filter(|end| *end <= self.end)
            .ok_or_else(|| self.fault(format!("has a descriptor of tag {tag} running past it")))?;
        self.at = body_end;
        Ok(Some(Descriptor {
            tag,
            header: Header {
                body: body_at,
                end: body_end,
                ..header
            },
        }))
    }
}

fn mvex(
    data: &[u8],
    header: Header,
    depth: u8,
    steps: &mut usize,
    defaults: &mut Vec<(u32, TrackDefaults)>,
) -> Result<(), Error> {
    let mut children = Children::of(data, header, depth)?;
    while let Some(child) = children.next(steps)? {
        match &child.kind {
            b"trex" => {
                let mut body = Body::of(data, child);
                body.version()?;
                body.skip(3, "flags")?;
                let id = body.u32("track_ID")?;
                let values = TrackDefaults {
                    sample_description_index: body.u32("default_sample_description_index")?,
                    sample_duration: body.u32("default_sample_duration")?,
                    sample_size: body.u32("default_sample_size")?,
                    sample_flags: body.u32("default_sample_flags")?,
                };
                defaults.push((id, values));
            }
            _ => skip(data, child, children.depth, steps)?,
        }
    }
    Ok(())
}

fn parse_pssh(data: &[u8], header: Header) -> Result<Pssh, Error> {
    let mut body = Body::of(data, header);
    let version = body.version()?;
    body.skip(3, "flags")?;
    let system_id = body.array("SystemID")?;
    let mut key_ids = Vec::new();
    if version >= 1 {
        let count = body.u32("KID_count")?;
        let fits = (count as usize)
            .checked_mul(16)
            .is_some_and(|n| n <= body.left());
        if !fits {
            return Err(body.fault(format!("declares {count} key ids in {} bytes", body.left())));
        }
        key_ids.reserve_exact(count as usize);
        for _ in 0..count {
            key_ids.push(body.array("KID")?);
        }
    }
    let size = body.u32("DataSize")?;
    let payload = body.bytes(size as usize, "Data")?;
    Ok(Pssh {
        system_id,
        key_ids,
        data: payload.to_vec(),
    })
}

/// A `sgpd` of grouping type `seig` (14496-12 §8.9.3, 23001-7 §6): its descriptions, or `None`
/// for another grouping type.
pub(crate) fn sgpd(
    data: &[u8],
    header: Header,
    steps: &mut usize,
) -> Result<Option<Vec<Group>>, Error> {
    let mut body = Body::of(data, header);
    let version = body.version()?;
    body.skip(3, "flags")?;
    if body.array::<4>("grouping_type")? != SEIG {
        return Ok(None);
    }
    let default_length = if version >= 1 {
        body.u32("default_length")?
    } else {
        0
    };
    if version >= 2 {
        body.u32("default_sample_description_index")?;
    }
    let count = body.u32("entry_count")?;
    let least = match (version, default_length) {
        (0, _) => 20,
        (_, 0) => 4,
        (_, length) => length as usize,
    };
    let fits = (count as usize)
        .checked_mul(least)
        .is_some_and(|n| n <= body.left());
    if !fits {
        return Err(body.fault(format!(
            "declares {count} descriptions in {} bytes",
            body.left()
        )));
    }
    let mut groups = Vec::with_capacity(count as usize);
    for _ in 0..count {
        *steps += 1;
        let length = match (version, default_length) {
            (0, _) => 20,
            (_, 0) => body.u32("description_length")? as usize,
            (_, length) => length as usize,
        };
        let entry = body.bytes(length, "description")?;
        groups.push(seig(&body, entry)?);
    }
    Ok(Some(groups))
}

/// One `seig` description, from its bytes; `sgpd` names the box in errors.
fn seig(sgpd: &Body<'_>, entry: &[u8]) -> Result<Group, Error> {
    let short = || sgpd.fault("has a description shorter than its fields");
    let fixed = entry.get(..20).ok_or_else(short)?;
    let pattern = Pattern::from_byte(fixed[1]);
    let protected = fixed[2] != 0;
    let iv_size = fixed[3];
    let mut key_id = [0; 16];
    key_id.copy_from_slice(&fixed[4..20]);
    if !matches!(iv_size, 0 | 8 | 16) {
        return Err(sgpd.fault(format!(
            "gives a per-sample IV size of {iv_size}, not 0, 8 or 16"
        )));
    }
    let mut constant_iv = [0; 16];
    let mut constant_iv_size = 0;
    if protected && iv_size == 0 {
        let size = *entry.get(20).ok_or_else(short)?;
        if !matches!(size, 8 | 16) {
            return Err(sgpd.fault(format!("gives a constant IV size of {size}, not 8 or 16")));
        }
        let iv = entry.get(21..21 + size as usize).ok_or_else(short)?;
        constant_iv[..iv.len()].copy_from_slice(iv);
        constant_iv_size = size;
    }
    Ok(Group {
        protected,
        iv_size,
        key_id,
        pattern,
        constant_iv,
        constant_iv_size,
    })
}
