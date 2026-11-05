//! Files built box by box for the tests, from the layouts of the specification's §2 to §5.
//!
//! A [`FileSpec`] says what a file holds; [`build`] gives its bytes and the samples the crate
//! should find in it, worked out here apart from the crate's own reading. [`build`] can also give
//! the file as it should read once made clear (§6), for a byte-for-byte comparison.
//!
//! This file is included by path from the integration tests too, so it uses nothing of the crate.

/// The key id the layouts use when nothing says otherwise.
pub const KEY: [u8; 16] = [0x22; 16];
/// A second key, for Spatial Audio's several entries and `seig` groups.
pub const OTHER_KEY: [u8; 16] = [0x33; 16];
/// Any Widevine-looking system id.
pub const SYSTEM_ID: [u8; 16] = [0xED; 16];

/// A box with a 32-bit size.
pub fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let size = u32::try_from(8 + body.len()).expect("a small box");
    [&size.to_be_bytes()[..], kind, body].concat()
}

/// A box with `size == 1` and a 64-bit `largesize`.
pub fn boxed64(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let size = 16 + body.len() as u64;
    [&1u32.to_be_bytes()[..], kind, &size.to_be_bytes(), body].concat()
}

/// A box with `size == 0`, which runs to the end of its container.
pub fn open_box(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    [&0u32.to_be_bytes()[..], kind, body].concat()
}

/// A `uuid` box with its extended type.
pub fn uuid_box(extended: &[u8; 16], body: &[u8]) -> Vec<u8> {
    let size = u32::try_from(24 + body.len()).expect("a small box");
    [&size.to_be_bytes()[..], b"uuid", extended, body].concat()
}

/// A full box: version and flags open its body.
pub fn full(kind: &[u8; 4], version: u8, flags: u32, body: &[u8]) -> Vec<u8> {
    let head = flags.to_be_bytes();
    boxed(
        kind,
        &[&[version, head[1], head[2], head[3]][..], body].concat(),
    )
}

/// A sample entry of `stsd`.
#[derive(Clone, Debug)]
pub struct Entry {
    /// `enca`, `encv`, `mp4a` or another type.
    pub kind: [u8; 4],
    /// What `frma` names, for an encrypted entry.
    pub format: [u8; 4],
    pub scheme: [u8; 4],
    pub iv_size: u8,
    pub key_id: [u8; 16],
    /// Makes the `tenc` version 1, with this crypt:skip pattern.
    pub pattern: Option<(u8, u8)>,
    pub constant_iv: Option<Vec<u8>>,
    /// `default_isProtected`.
    pub protected: bool,
    /// The `AudioSpecificConfig` an `esds` carries.
    pub audio_config: Option<Vec<u8>>,
    /// QuickTime's sound description version: 0, 1 or 2.
    pub quicktime_version: u16,
}

impl Entry {
    /// An `enca` entry of scheme `cenc` with `iv_size`-byte IVs.
    pub fn cenc(iv_size: u8) -> Self {
        Self {
            kind: *b"enca",
            format: *b"mp4a",
            scheme: *b"cenc",
            iv_size,
            key_id: KEY,
            pattern: None,
            constant_iv: None,
            protected: true,
            audio_config: Some(vec![0x12, 0x10]),
            quicktime_version: 0,
        }
    }

    /// An `enca` entry of scheme `cbcs`: a constant IV and a crypt:skip pattern.
    pub fn cbcs(pattern: (u8, u8), constant_iv: Vec<u8>) -> Self {
        Self {
            scheme: *b"cbcs",
            iv_size: 0,
            pattern: Some(pattern),
            constant_iv: Some(constant_iv),
            ..Self::cenc(0)
        }
    }

    /// A clear `mp4a` entry.
    pub fn clear() -> Self {
        Self {
            kind: *b"mp4a",
            ..Self::cenc(0)
        }
    }

    pub fn with_key(mut self, key_id: [u8; 16]) -> Self {
        self.key_id = key_id;
        self
    }

    fn encrypted(&self) -> bool {
        matches!(&self.kind, b"enca" | b"encv")
    }
}

/// A `seig` sample group description.
#[derive(Clone, Debug)]
pub struct Seig {
    pub protected: bool,
    pub iv_size: u8,
    pub key_id: [u8; 16],
    pub pattern: (u8, u8),
    pub constant_iv: Option<Vec<u8>>,
}

impl Seig {
    pub fn key(key_id: [u8; 16], iv_size: u8) -> Self {
        Self {
            protected: true,
            iv_size,
            key_id,
            pattern: (0, 0),
            constant_iv: None,
        }
    }

    pub fn clear() -> Self {
        Self {
            protected: false,
            iv_size: 0,
            key_id: [0; 16],
            pattern: (0, 0),
            constant_iv: None,
        }
    }

    fn bytes(&self) -> Vec<u8> {
        let mut entry = vec![
            0,
            (self.pattern.0 << 4) | self.pattern.1,
            u8::from(self.protected),
            self.iv_size,
        ];
        entry.extend(self.key_id);
        if let Some(iv) = &self.constant_iv {
            entry.push(iv.len() as u8);
            entry.extend(iv);
        }
        entry
    }
}

/// A `pssh` box.
#[derive(Clone, Debug)]
pub struct Pssh {
    pub key_ids: Option<Vec<[u8; 16]>>,
    pub data: Vec<u8>,
}

impl Pssh {
    fn bytes(&self) -> Vec<u8> {
        let mut body = SYSTEM_ID.to_vec();
        if let Some(ids) = &self.key_ids {
            body.extend((ids.len() as u32).to_be_bytes());
            for id in ids {
                body.extend(id);
            }
        }
        body.extend((self.data.len() as u32).to_be_bytes());
        body.extend(&self.data);
        full(b"pssh", u8::from(self.key_ids.is_some()), 0, &body)
    }
}

#[derive(Clone, Debug)]
pub struct TrackSpec {
    pub id: u32,
    pub timescale: u32,
    pub entries: Vec<Entry>,
    /// `trex`: description index, duration, size, flags.
    pub trex: Option<[u32; 4]>,
    /// The `seig` descriptions of the track's `stbl`.
    pub groups: Vec<Seig>,
    /// The version of that `sgpd`: 0 (fixed 20-byte entries), 1 or 2.
    pub sgpd_version: u8,
    /// `tkhd` and `mdhd` of version 1, with 64-bit times.
    pub version1_headers: bool,
    /// A video track: entries laid out with the 78 bytes of visual fields.
    pub visual: bool,
}

impl TrackSpec {
    pub fn audio(id: u32, entries: Vec<Entry>) -> Self {
        Self {
            id,
            timescale: 44_100,
            entries,
            trex: Some([1, 1024, 0, 0]),
            groups: Vec::new(),
            sgpd_version: 1,
            version1_headers: false,
            visual: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MovieSpec {
    pub tracks: Vec<TrackSpec>,
    pub pssh: Vec<Pssh>,
    /// Boxes to put in the `moov` that the crate should skip.
    pub extra: Vec<u8>,
    /// Give the `moov` a 64-bit size.
    pub largesize: bool,
}

/// One sample of a run.
#[derive(Clone, Debug)]
pub struct SampleSpec {
    pub size: u32,
    /// Its own duration, when the run carries durations.
    pub duration: u32,
    /// Its IV, of the IV size its entry or group names: empty under a constant IV.
    pub iv: Vec<u8>,
    pub subsamples: Vec<(u16, u32)>,
}

impl SampleSpec {
    pub fn new(size: u32, iv: Vec<u8>) -> Self {
        Self {
            size,
            duration: 1024,
            iv,
            subsamples: Vec::new(),
        }
    }
}

/// A `trun`.
#[derive(Clone, Debug)]
pub struct RunSpec {
    pub samples: Vec<SampleSpec>,
    /// Carry a data offset, computed to where the run's data lies.
    pub data_offset: bool,
    pub per_sample_duration: bool,
    pub per_sample_size: bool,
    /// Carry sample flags and composition offsets too, with `first_sample_flags`.
    pub all_fields: bool,
}

impl RunSpec {
    /// `count` samples of `size` bytes with `iv_size`-byte IVs, sizes in the `trun`.
    pub fn of(count: u32, size: u32, iv_size: usize) -> Self {
        let samples = (0..count)
            .map(|n| SampleSpec::new(size, vec![n as u8 + 1; iv_size]))
            .collect();
        Self {
            samples,
            data_offset: true,
            per_sample_duration: false,
            per_sample_size: true,
            all_fields: false,
        }
    }

    fn flags(&self) -> u32 {
        let mut flags = 0;
        if self.data_offset {
            flags |= 0x1;
        }
        if self.per_sample_duration {
            flags |= 0x100;
        }
        if self.per_sample_size {
            flags |= 0x200;
        }
        if self.all_fields {
            flags |= 0x4 | 0x400 | 0x800;
        }
        flags
    }
}

/// How a `traf` carries its IVs and subsamples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aux {
    Senc,
    /// `saiz` and `saio`, the information laid in the `mdat` before the samples.
    SaizSaio,
    /// An `senc` of `sample_count` 0, flags 0: what Apple's live radio sends under a constant IV.
    EmptySenc,
    None,
}

#[derive(Clone, Debug)]
pub struct TrafSpec {
    pub track_id: u32,
    /// `sample_description_index` in the `tfhd`, from 1.
    pub description_index: Option<u32>,
    pub default_duration: Option<u32>,
    pub default_size: Option<u32>,
    /// Carry a `base_data_offset`: the absolute offset of the `traf`'s data.
    pub explicit_base: bool,
    pub default_base_is_moof: bool,
    pub runs: Vec<RunSpec>,
    pub aux: Aux,
    /// `sbgp` entries of type `seig`: sample counts and group description indices.
    pub sbgp: Vec<(u32, u32)>,
    /// The `traf`'s own `seig` descriptions, named from 0x10001.
    pub sgpd: Vec<Seig>,
}

impl TrafSpec {
    pub fn new(track_id: u32, runs: Vec<RunSpec>) -> Self {
        Self {
            track_id,
            description_index: None,
            default_duration: Some(1024),
            default_size: None,
            explicit_base: false,
            default_base_is_moof: true,
            runs,
            aux: Aux::Senc,
            sbgp: Vec::new(),
            sgpd: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FragmentSpec {
    pub trafs: Vec<TrafSpec>,
    /// Boxes between the `moof` and its `mdat`.
    pub gap: Vec<u8>,
    /// Give the `mdat` a 64-bit size.
    pub mdat64: bool,
    /// Put the `mdat` before the `moof`: data offsets are then negative.
    pub mdat_first: bool,
    /// A `pssh` in the `moof`.
    pub pssh: bool,
}

impl FragmentSpec {
    pub fn new(trafs: Vec<TrafSpec>) -> Self {
        Self {
            trafs,
            gap: Vec::new(),
            mdat64: false,
            mdat_first: false,
            pssh: false,
        }
    }

    /// One `traf` of one run of `count` samples of `size` bytes, IVs of `iv_size` bytes.
    pub fn simple(track_id: u32, count: u32, size: u32, iv_size: usize) -> Self {
        Self::new(vec![TrafSpec::new(
            track_id,
            vec![RunSpec::of(count, size, iv_size)],
        )])
    }
}

#[derive(Clone, Debug, Default)]
pub struct FileSpec {
    /// Boxes before the `moov`: an `ftyp`.
    pub leading: Vec<u8>,
    pub movie: MovieSpec,
    pub fragments: Vec<FragmentSpec>,
    /// Bytes after the last fragment: an index, a `free` box.
    pub trailing: Vec<u8>,
}

/// What the crate should find of a sample.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expected {
    pub offset: usize,
    pub size: usize,
    pub duration: u32,
    pub fragment: usize,
    pub entry: u32,
    pub key_id: [u8; 16],
    pub iv: [u8; 16],
    pub iv_size: u8,
    pub subsamples: Vec<(u32, u32)>,
    pub protected: bool,
    pub scheme: [u8; 4],
    pub pattern: (u8, u8),
}

/// A file built, and the samples it holds.
#[derive(Clone, Debug)]
pub struct Built {
    pub bytes: Vec<u8>,
    pub samples: Vec<Expected>,
    /// Where each fragment's `moof` starts.
    pub fragments: Vec<usize>,
}

/// Builds `spec`; `cleared` gives the file as it should read once made clear (§6).
pub fn build(spec: &FileSpec, cleared: bool) -> Built {
    let mut bytes = spec.leading.clone();
    bytes.extend(moov(&spec.movie, cleared));
    let mut samples = Vec::new();
    let mut fragments = Vec::new();
    for fragment in &spec.fragments {
        let at = bytes.len();
        let built = build_fragment(fragment, &spec.movie, at, cleared);
        bytes.extend(built.bytes);
        samples.extend(built.samples);
        fragments.push(built.moof_at);
    }
    bytes.extend(&spec.trailing);
    Built {
        bytes,
        samples,
        fragments,
    }
}

/// A clear `ftyp`.
pub fn ftyp() -> Vec<u8> {
    boxed(
        b"ftyp",
        &[&b"iso6"[..], &0u32.to_be_bytes(), b"iso6", b"cenc"].concat(),
    )
}

fn moov(spec: &MovieSpec, cleared: bool) -> Vec<u8> {
    let mut body = Vec::new();
    for pssh in &spec.pssh {
        body.extend(free_if(cleared, pssh.bytes()));
    }
    for track in &spec.tracks {
        body.extend(trak(track, cleared));
    }
    let mut mvex = Vec::new();
    for track in &spec.tracks {
        if let Some([index, duration, size, flags]) = track.trex {
            let index = if cleared && collapses(track) {
                1
            } else {
                index
            };
            let mut trex = track.id.to_be_bytes().to_vec();
            trex.extend(index.to_be_bytes());
            trex.extend(duration.to_be_bytes());
            trex.extend(size.to_be_bytes());
            trex.extend(flags.to_be_bytes());
            mvex.extend(full(b"trex", 0, 0, &trex));
        }
    }
    body.extend(boxed(b"mvex", &mvex));
    body.extend(&spec.extra);
    if spec.largesize {
        boxed64(b"moov", &body)
    } else {
        boxed(b"moov", &body)
    }
}

/// Whether a track's entries are the same audio under several keys (§6).
fn collapses(track: &TrackSpec) -> bool {
    let Some(first) = track.entries.first() else {
        return false;
    };
    track.entries.len() > 1
        && track.entries.iter().all(|entry| {
            entry.encrypted()
                && entry.format == first.format
                && entry.audio_config == first.audio_config
        })
}

fn free_if(cleared: bool, mut box_bytes: Vec<u8>) -> Vec<u8> {
    if cleared {
        box_bytes[4..8].copy_from_slice(b"free");
    }
    box_bytes
}

fn trak(track: &TrackSpec, cleared: bool) -> Vec<u8> {
    let version = u8::from(track.version1_headers);
    let times = if track.version1_headers { 16 } else { 8 };
    let mut tkhd = vec![0; times];
    tkhd.extend(track.id.to_be_bytes());
    tkhd.extend(vec![0; 68]);
    let mut mdhd = vec![0; times];
    mdhd.extend(track.timescale.to_be_bytes());
    mdhd.extend(vec![0; if track.version1_headers { 12 } else { 8 }]);
    let stsd = stsd(track, cleared);
    let mut stbl = stsd;
    if !track.groups.is_empty() {
        stbl.extend(free_if(cleared, sgpd(&track.groups, track.sgpd_version)));
    }
    let minf = boxed(b"minf", &boxed(b"stbl", &stbl));
    let mdia = boxed(b"mdia", &[full(b"mdhd", version, 0, &mdhd), minf].concat());
    boxed(b"trak", &[full(b"tkhd", version, 7, &tkhd), mdia].concat())
}

fn stsd(track: &TrackSpec, cleared: bool) -> Vec<u8> {
    let count = if cleared && collapses(track) {
        1
    } else {
        track.entries.len() as u32
    };
    let mut body = count.to_be_bytes().to_vec();
    for entry in &track.entries {
        body.extend(sample_entry(entry, track.visual, cleared));
    }
    full(b"stsd", 0, 0, &body)
}

fn sample_entry(entry: &Entry, visual: bool, cleared: bool) -> Vec<u8> {
    let mut body = vec![0; 6];
    body.extend(1u16.to_be_bytes());
    if visual {
        body.extend(vec![0; 70]);
    } else {
        body.extend(entry.quicktime_version.to_be_bytes());
        body.extend(vec![0; 18]);
        body.extend(vec![
            0;
            match entry.quicktime_version {
                1 => 16,
                2 => 36,
                _ => 0,
            }
        ]);
    }
    if let Some(config) = &entry.audio_config {
        body.extend(esds(config));
    }
    if entry.encrypted() {
        body.extend(free_if(cleared, sinf(entry)));
    }
    let kind = if cleared && entry.encrypted() {
        &entry.format
    } else {
        &entry.kind
    };
    boxed(kind, &body)
}

/// An `esds` whose `DecoderSpecificInfo` is `config`, its lengths in the two-byte form.
pub fn esds(config: &[u8]) -> Vec<u8> {
    let specific = descriptor(5, config);
    let decoder = descriptor(
        4,
        &[
            &[0x40, 0x15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0][..],
            &specific,
        ]
        .concat(),
    );
    let es = descriptor(3, &[&[0, 1, 0][..], &decoder].concat());
    full(b"esds", 0, 0, &es)
}

fn descriptor(tag: u8, body: &[u8]) -> Vec<u8> {
    let length = body.len();
    [
        &[tag, 0x80 | (length >> 7) as u8, (length & 0x7F) as u8][..],
        body,
    ]
    .concat()
}

fn sinf(entry: &Entry) -> Vec<u8> {
    let (version, pattern_byte) = match entry.pattern {
        Some((crypt, skip)) => (1, (crypt << 4) | skip),
        None => (0, 0),
    };
    let mut tenc = vec![0, pattern_byte, u8::from(entry.protected), entry.iv_size];
    tenc.extend(entry.key_id);
    if let Some(iv) = &entry.constant_iv {
        tenc.push(iv.len() as u8);
        tenc.extend(iv);
    }
    boxed(
        b"sinf",
        &[
            boxed(b"frma", &entry.format),
            full(b"schm", 0, 0, &[&entry.scheme[..], &[0, 1, 0, 0]].concat()),
            boxed(b"schi", &full(b"tenc", version, 0, &tenc)),
        ]
        .concat(),
    )
}

/// An `sgpd` of type `seig`: version 0 has fixed 20-byte entries, version 1 a `default_length`
/// of 0 and each entry's length before it, version 2 a default sample description index too.
fn sgpd(groups: &[Seig], version: u8) -> Vec<u8> {
    let mut body = b"seig".to_vec();
    if version >= 1 {
        body.extend(0u32.to_be_bytes());
    }
    if version >= 2 {
        body.extend(0u32.to_be_bytes());
    }
    body.extend((groups.len() as u32).to_be_bytes());
    for group in groups {
        let mut entry = group.bytes();
        if version == 0 {
            entry.truncate(20);
        } else {
            body.extend((entry.len() as u32).to_be_bytes());
        }
        body.extend(entry);
    }
    full(b"sgpd", version, 0, &body)
}

/// The protection a sample ends up with.
struct Resolved {
    protected: bool,
    iv_size: u8,
    key_id: [u8; 16],
    pattern: (u8, u8),
    constant_iv: Option<Vec<u8>>,
}

struct BuiltFragment {
    bytes: Vec<u8>,
    samples: Vec<Expected>,
    moof_at: usize,
}

/// Where a `traf`'s pieces lie, once the `moof`'s size is known.
struct TrafLayout {
    /// Where each run's data starts, in the file.
    run_starts: Vec<usize>,
    /// Where the `saiz`/`saio` information starts, in the file.
    aux_at: usize,
    data_end: usize,
}

fn build_fragment(
    spec: &FragmentSpec,
    movie: &MovieSpec,
    at: usize,
    cleared: bool,
) -> BuiltFragment {
    // The data offsets depend on the moof's size, which does not depend on their values: build
    // once to measure, then again with the offsets right.
    let probe = moof(spec, movie, at, &guess_layouts(spec, at, 0), cleared);
    let mdat_header = if spec.mdat64 { 16 } else { 8 };
    let (moof_at, mdat_at) = if spec.mdat_first {
        let mdat_len = mdat_header + payload_len(spec);
        (at + mdat_len + spec.gap.len(), at)
    } else {
        (at, at + probe.len() + spec.gap.len())
    };
    let layouts = guess_layouts(spec, moof_at, mdat_at + mdat_header);
    let moof = moof(spec, movie, moof_at, &layouts, cleared);
    assert_eq!(
        moof.len(),
        probe.len(),
        "the moof's size does not depend on offsets"
    );
    let payload = payload(spec, &layouts);
    let mdat = if spec.mdat64 {
        boxed64(b"mdat", &payload)
    } else {
        boxed(b"mdat", &payload)
    };
    let bytes = if spec.mdat_first {
        [mdat, spec.gap.clone(), moof].concat()
    } else {
        [moof, spec.gap.clone(), mdat].concat()
    };
    let samples = expected(spec, movie, moof_at, &layouts);
    BuiltFragment {
        bytes,
        samples,
        moof_at,
    }
}

fn aux_len(traf: &TrafSpec) -> usize {
    if traf.aux != Aux::SaizSaio {
        return 0;
    }
    traf.runs
        .iter()
        .flat_map(|run| &run.samples)
        .map(sample_info_len)
        .sum()
}

fn sample_info_len(sample: &SampleSpec) -> usize {
    sample.iv.len()
        + if sample.subsamples.is_empty() {
            0
        } else {
            2 + 6 * sample.subsamples.len()
        }
}

fn payload_len(spec: &FragmentSpec) -> usize {
    spec.trafs
        .iter()
        .map(|traf| {
            aux_len(traf)
                + traf
                    .runs
                    .iter()
                    .flat_map(|run| &run.samples)
                    .map(|sample| sample.size as usize)
                    .sum::<usize>()
        })
        .sum()
}

/// Lays the `mdat`'s payload out: for each `traf`, its auxiliary information then its samples.
fn guess_layouts(spec: &FragmentSpec, _moof_at: usize, mdat_body: usize) -> Vec<TrafLayout> {
    let mut at = mdat_body;
    spec.trafs
        .iter()
        .map(|traf| {
            let aux_at = at;
            at += aux_len(traf);
            let mut run_starts = Vec::new();
            for run in &traf.runs {
                run_starts.push(at);
                at += run.samples.iter().map(|s| s.size as usize).sum::<usize>();
            }
            TrafLayout {
                run_starts,
                aux_at,
                data_end: at,
            }
        })
        .collect()
}

fn payload(spec: &FragmentSpec, layouts: &[TrafLayout]) -> Vec<u8> {
    let mut out = Vec::new();
    for (traf, layout) in spec.trafs.iter().zip(layouts) {
        if traf.aux == Aux::SaizSaio {
            for sample in traf.runs.iter().flat_map(|run| &run.samples) {
                out.extend(sample_info(sample, true));
            }
        }
        for (run, _) in traf.runs.iter().zip(&layout.run_starts) {
            for (n, sample) in run.samples.iter().enumerate() {
                out.extend(std::iter::repeat_n(0xA0 ^ n as u8, sample.size as usize));
            }
        }
    }
    out
}

/// A sample's IV then, `subsampled` or when it has any, its subsample runs.
fn sample_info(sample: &SampleSpec, only_if_present: bool) -> Vec<u8> {
    let mut out = sample.iv.clone();
    if only_if_present && sample.subsamples.is_empty() {
        return out;
    }
    out.extend((sample.subsamples.len() as u16).to_be_bytes());
    for (clear, protected) in &sample.subsamples {
        out.extend(clear.to_be_bytes());
        out.extend(protected.to_be_bytes());
    }
    out
}

fn moof(
    spec: &FragmentSpec,
    movie: &MovieSpec,
    moof_at: usize,
    layouts: &[TrafLayout],
    cleared: bool,
) -> Vec<u8> {
    let mut body = full(b"mfhd", 0, 0, &1u32.to_be_bytes());
    if spec.pssh {
        body.extend(free_if(
            cleared,
            Pssh {
                key_ids: None,
                data: vec![7; 4],
            }
            .bytes(),
        ));
    }
    let mut previous_end = None;
    for (n, (traf, layout)) in spec.trafs.iter().zip(layouts).enumerate() {
        let track = movie
            .tracks
            .iter()
            .find(|track| track.id == traf.track_id)
            .expect("the traf's track is in the movie");
        let base = if traf.explicit_base {
            layout.run_starts.first().copied().unwrap_or(layout.aux_at)
        } else if traf.default_base_is_moof || n == 0 {
            moof_at
        } else {
            previous_end.expect("a previous traf")
        };
        body.extend(traf_box(traf, track, base, layout, cleared));
        previous_end = Some(layout.data_end);
    }
    boxed(b"moof", &body)
}

fn traf_box(
    traf: &TrafSpec,
    track: &TrackSpec,
    base: usize,
    layout: &TrafLayout,
    cleared: bool,
) -> Vec<u8> {
    let mut flags = 0;
    let mut tfhd = traf.track_id.to_be_bytes().to_vec();
    if traf.explicit_base {
        flags |= 0x1;
        tfhd.extend((base as u64).to_be_bytes());
    }
    if let Some(index) = traf.description_index {
        flags |= 0x2;
        let index = if cleared && collapses(track) {
            1
        } else {
            index
        };
        tfhd.extend(index.to_be_bytes());
    }
    if let Some(duration) = traf.default_duration {
        flags |= 0x8;
        tfhd.extend(duration.to_be_bytes());
    }
    if let Some(size) = traf.default_size {
        flags |= 0x10;
        tfhd.extend(size.to_be_bytes());
    }
    if traf.default_base_is_moof {
        flags |= 0x02_0000;
    }
    let mut body = full(b"tfhd", 0, flags, &tfhd);
    body.extend(full(b"tfdt", 1, 0, &0u64.to_be_bytes()));
    let mut run_end = base;
    for (run, start) in traf.runs.iter().zip(&layout.run_starts) {
        let mut trun = (run.samples.len() as u32).to_be_bytes().to_vec();
        if run.data_offset {
            let offset = i32::try_from(*start as i64 - base as i64).expect("a near offset");
            trun.extend(offset.to_be_bytes());
        } else {
            assert_eq!(
                *start, run_end,
                "a run without an offset lies where the previous ended"
            );
        }
        if run.all_fields {
            trun.extend(0x0200_0000u32.to_be_bytes());
        }
        for sample in &run.samples {
            if run.per_sample_duration {
                trun.extend(sample.duration.to_be_bytes());
            }
            if run.per_sample_size {
                trun.extend(sample.size.to_be_bytes());
            }
            if run.all_fields {
                trun.extend(0u32.to_be_bytes());
                trun.extend((-1i32).to_be_bytes());
            }
        }
        let version = u8::from(run.all_fields);
        body.extend(full(b"trun", version, run.flags(), &trun));
        run_end = start + run.samples.iter().map(|s| s.size as usize).sum::<usize>();
    }
    let samples: Vec<&SampleSpec> = traf.runs.iter().flat_map(|run| &run.samples).collect();
    match traf.aux {
        Aux::Senc => {
            let subsampled = samples.iter().any(|s| !s.subsamples.is_empty());
            let mut senc = (samples.len() as u32).to_be_bytes().to_vec();
            for sample in &samples {
                senc.extend(sample.iv.clone());
                if subsampled {
                    senc.extend(sample_info(sample, false)[sample.iv.len()..].to_vec());
                }
            }
            body.extend(free_if(
                cleared,
                full(b"senc", 0, if subsampled { 2 } else { 0 }, &senc),
            ));
        }
        Aux::SaizSaio => {
            let sizes: Vec<u8> = samples.iter().map(|s| sample_info_len(s) as u8).collect();
            let default = if sizes.iter().all(|s| *s == sizes[0]) {
                sizes[0]
            } else {
                0
            };
            let mut saiz = vec![default];
            saiz.extend((samples.len() as u32).to_be_bytes());
            if default == 0 {
                saiz.extend(&sizes);
            }
            body.extend(free_if(cleared, full(b"saiz", 0, 0, &saiz)));
            let mut saio = 1u32.to_be_bytes().to_vec();
            // The probe build lays the data at 0: the difference wraps there, and is right once
            // the layout is known.
            saio.extend((layout.aux_at.wrapping_sub(base) as u32).to_be_bytes());
            body.extend(free_if(cleared, full(b"saio", 0, 0, &saio)));
        }
        Aux::EmptySenc => {
            body.extend(free_if(cleared, full(b"senc", 0, 0, &0u32.to_be_bytes())));
        }
        Aux::None => {}
    }
    if !traf.sbgp.is_empty() {
        let mut sbgp = b"seig".to_vec();
        sbgp.extend((traf.sbgp.len() as u32).to_be_bytes());
        for (count, group) in &traf.sbgp {
            sbgp.extend(count.to_be_bytes());
            sbgp.extend(group.to_be_bytes());
        }
        body.extend(free_if(cleared, full(b"sbgp", 0, 0, &sbgp)));
    }
    if !traf.sgpd.is_empty() {
        body.extend(free_if(cleared, sgpd(&traf.sgpd, 1)));
    }
    boxed(b"traf", &body)
}

fn expected(
    spec: &FragmentSpec,
    movie: &MovieSpec,
    moof_at: usize,
    layouts: &[TrafLayout],
) -> Vec<Expected> {
    let mut out = Vec::new();
    for (traf, layout) in spec.trafs.iter().zip(layouts) {
        let track = movie
            .tracks
            .iter()
            .find(|track| track.id == traf.track_id)
            .expect("the traf's track is in the movie");
        let trex = track.trex.unwrap_or([0; 4]);
        let index = traf.description_index.unwrap_or(trex[0]);
        let entry = &track.entries[index as usize - 1];
        let groups: Vec<u32> = if traf.sbgp.is_empty() {
            Vec::new()
        } else {
            traf.sbgp
                .iter()
                .flat_map(|(count, group)| std::iter::repeat_n(*group, *count as usize))
                .collect()
        };
        let mut nth = 0;
        for (run, start) in traf.runs.iter().zip(&layout.run_starts) {
            let mut at = *start;
            for sample in &run.samples {
                let group = groups.get(nth).copied().unwrap_or(0);
                let resolved = resolve(entry, track, traf, group);
                let (iv, iv_size) = match (&resolved.constant_iv, resolved.iv_size) {
                    (Some(iv), 0) => (iv.clone(), iv.len() as u8),
                    _ => (sample.iv.clone(), sample.iv.len() as u8),
                };
                let mut padded = [0; 16];
                let kept = iv.len().min(16);
                padded[..kept].copy_from_slice(&iv[..kept]);
                let duration = if run.per_sample_duration {
                    sample.duration
                } else {
                    traf.default_duration.unwrap_or(trex[1])
                };
                out.push(Expected {
                    offset: at,
                    size: sample.size as usize,
                    duration,
                    fragment: moof_at,
                    entry: index - 1,
                    key_id: if resolved.protected {
                        resolved.key_id
                    } else {
                        [0; 16]
                    },
                    iv: if resolved.protected { padded } else { [0; 16] },
                    iv_size: if resolved.protected { iv_size } else { 0 },
                    subsamples: sample
                        .subsamples
                        .iter()
                        .map(|(clear, protected)| (u32::from(*clear), *protected))
                        .collect(),
                    protected: resolved.protected,
                    scheme: if resolved.protected {
                        entry.scheme
                    } else {
                        *b"cenc"
                    },
                    pattern: if resolved.protected {
                        resolved.pattern
                    } else {
                        (0, 0)
                    },
                });
                at += sample.size as usize;
                nth += 1;
            }
        }
    }
    out
}

fn resolve(entry: &Entry, track: &TrackSpec, traf: &TrafSpec, group: u32) -> Resolved {
    let seig = match group {
        0 => None,
        1..=0xFFFF => Some(&track.groups[group as usize - 1]),
        _ => Some(&traf.sgpd[(group - 0x10001) as usize]),
    };
    match seig {
        None => Resolved {
            protected: entry.encrypted(),
            iv_size: entry.iv_size,
            key_id: entry.key_id,
            pattern: entry.pattern.unwrap_or((0, 0)),
            constant_iv: entry.constant_iv.clone(),
        },
        Some(seig) => Resolved {
            protected: seig.protected,
            iv_size: seig.iv_size,
            key_id: seig.key_id,
            pattern: seig.pattern,
            constant_iv: seig.constant_iv.clone(),
        },
    }
}

/// A plain file: one `cenc` track with 16-byte IVs, `fragments` fragments of `count` samples of
/// `size` bytes, IVs and no subsamples in `senc`.
pub fn plain(fragments: usize, count: u32, size: u32) -> FileSpec {
    FileSpec {
        leading: ftyp(),
        movie: MovieSpec {
            tracks: vec![TrackSpec::audio(1, vec![Entry::cenc(16)])],
            ..MovieSpec::default()
        },
        fragments: (0..fragments)
            .map(|_| FragmentSpec::simple(1, count, size, 16))
            .collect(),
        trailing: Vec::new(),
    }
}

/// Apple's live radio (§5): one `cbcs` track with a 1:9 pattern and a constant IV, two fragments
/// of five samples, each with an `senc` that counts 0 samples.
pub fn radio() -> FileSpec {
    let mut radio = plain(2, 5, 64);
    radio.movie.tracks[0].entries[0] = Entry::cbcs((1, 9), vec![0x44; 16]);
    for fragment in &mut radio.fragments {
        *fragment = FragmentSpec::simple(1, 5, 64, 0);
        fragment.trafs[0].aux = Aux::EmptySenc;
    }
    radio
}

/// The layouts of §13, each named: every box layout of §2 to §5 on a file of its own.
pub fn corpus() -> Vec<(&'static str, FileSpec)> {
    let mut files = vec![("plain", plain(2, 3, 50))];

    let mut sizes64 = plain(1, 2, 40);
    sizes64.movie.largesize = true;
    sizes64.movie.extra = [uuid_box(&[9; 16], &[1, 2, 3]), boxed(b"udta", &[])].concat();
    sizes64.fragments[0].mdat64 = true;
    sizes64.fragments[0].gap = uuid_box(&[8; 16], &[]);
    sizes64.trailing = open_box(b"free", &[0; 12]);
    files.push(("sizes64", sizes64));

    let mut two_tracks = plain(1, 2, 40);
    two_tracks
        .movie
        .tracks
        .push(TrackSpec::audio(2, vec![Entry::clear()]));
    let mut second = TrafSpec::new(2, vec![RunSpec::of(3, 20, 0)]);
    second.default_base_is_moof = false;
    second.runs[0].data_offset = false;
    second.aux = Aux::None;
    two_tracks.fragments[0].trafs.push(second);
    files.push(("two_tracks", two_tracks));

    let mut two_runs = plain(1, 2, 40);
    let mut later = RunSpec::of(3, 30, 16);
    later.data_offset = false;
    later.per_sample_duration = true;
    later.samples[2].duration = 500;
    two_runs.fragments[0].trafs[0].runs.push(later);
    files.push(("two_runs", two_runs));

    let mut base_offset = plain(1, 3, 40);
    base_offset.fragments[0].trafs[0].explicit_base = true;
    base_offset.fragments[0].trafs[0].default_base_is_moof = false;
    base_offset.fragments[0].trafs[0].runs[0].data_offset = false;
    files.push(("base_offset", base_offset));

    let mut mdat_gap = plain(2, 3, 40);
    mdat_gap.fragments[1].gap = boxed(b"free", &[0; 20]);
    mdat_gap.fragments[1].mdat64 = true;
    files.push(("mdat_gap", mdat_gap));

    let mut mdat_first = plain(2, 3, 40);
    mdat_first.fragments[0].mdat_first = true;
    files.push(("mdat_first", mdat_first));

    let mut iv8 = plain(1, 4, 100);
    iv8.movie.tracks[0].entries[0] = Entry::cenc(8);
    iv8.fragments[0] = FragmentSpec::simple(1, 4, 100, 8);
    for (n, sample) in iv8.fragments[0].trafs[0].runs[0]
        .samples
        .iter_mut()
        .enumerate()
    {
        sample.subsamples = vec![(10 + n as u16, 60), (5, 25 - n as u32)];
    }
    files.push(("iv8_subsamples", iv8));

    let mut cbcs = plain(1, 3, 64);
    cbcs.movie.tracks[0].entries[0] = Entry::cbcs((1, 9), vec![0x44; 16]);
    cbcs.fragments[0] = FragmentSpec::simple(1, 3, 64, 0);
    cbcs.fragments[0].trafs[0].aux = Aux::None;
    files.push(("cbcs_constant_iv", cbcs));

    let mut cbcs_senc = plain(1, 3, 64);
    cbcs_senc.movie.tracks[0].entries[0] = Entry::cbcs((0, 0), vec![0x44; 16]);
    cbcs_senc.fragments[0] = FragmentSpec::simple(1, 3, 64, 0);
    cbcs_senc.fragments[0].trafs[0].runs[0].samples[1].subsamples = vec![(4, 60)];
    files.push(("cbcs_senc", cbcs_senc));

    files.push(("radio_empty_senc", radio()));

    let mut saiz = plain(1, 3, 50);
    saiz.fragments[0].trafs[0].aux = Aux::SaizSaio;
    saiz.fragments[0].trafs[0].runs[0].samples[0].subsamples = vec![(2, 48)];
    files.push(("saiz_saio", saiz));

    let mut seig = plain(1, 5, 40);
    seig.movie.tracks[0].groups = vec![Seig::clear(), Seig::key(OTHER_KEY, 8)];
    seig.fragments[0].trafs[0].runs[0].samples[1].iv = Vec::new();
    seig.fragments[0].trafs[0].runs[0].samples[2].iv = vec![9; 8];
    seig.fragments[0].trafs[0].runs[0].samples[3].iv = vec![7; 16];
    seig.fragments[0].trafs[0].sbgp = vec![(1, 0), (1, 1), (1, 2), (1, 0x10001), (1, 0)];
    seig.fragments[0].trafs[0].sgpd = vec![Seig {
        pattern: (3, 7),
        ..Seig::key([0x55; 16], 16)
    }];
    files.push(("seig", seig));

    let mut seig_v0 = plain(1, 2, 40);
    seig_v0.movie.tracks[0].groups = vec![Seig::key(OTHER_KEY, 16)];
    seig_v0.movie.tracks[0].sgpd_version = 0;
    seig_v0.fragments[0].trafs[0].sbgp = vec![(2, 1)];
    files.push(("seig_version0", seig_v0));

    let mut spatial = plain(3, 2, 40);
    spatial.movie.tracks[0].entries = vec![
        Entry::cenc(16).with_key([0; 16]),
        Entry::cenc(16).with_key([0; 16]),
        Entry::cenc(16).with_key([0; 16]),
    ];
    spatial.movie.pssh = vec![Pssh {
        key_ids: Some(vec![KEY, OTHER_KEY]),
        data: vec![1, 2, 3],
    }];
    for (n, fragment) in spatial.fragments.iter_mut().enumerate() {
        fragment.trafs[0].description_index = Some(n as u32 + 1);
    }
    files.push(("spatial", spatial));

    let mut clear = plain(2, 3, 40);
    clear.movie.tracks[0].entries[0] = Entry::clear();
    for fragment in &mut clear.fragments {
        *fragment = FragmentSpec::simple(1, 3, 40, 0);
        fragment.trafs[0].aux = Aux::None;
    }
    files.push(("clear", clear));

    let mut trex_defaults = plain(1, 4, 48);
    trex_defaults.movie.tracks[0].trex = Some([1, 960, 48, 0]);
    let traf = &mut trex_defaults.fragments[0].trafs[0];
    traf.default_duration = None;
    traf.runs[0].per_sample_size = false;
    files.push(("trex_defaults", trex_defaults));

    let mut fields = plain(1, 3, 40);
    let run = &mut fields.fragments[0].trafs[0].runs[0];
    run.all_fields = true;
    run.per_sample_duration = true;
    run.samples[0].duration = 1000;
    run.samples[2].duration = 300;
    files.push(("per_sample_fields", fields));

    let mut pssh = plain(1, 2, 40);
    pssh.movie.pssh = vec![
        Pssh {
            key_ids: None,
            data: vec![0xAB; 6],
        },
        Pssh {
            key_ids: Some(vec![KEY]),
            data: Vec::new(),
        },
    ];
    pssh.fragments[0].pssh = true;
    files.push(("pssh", pssh));

    let mut quicktime = plain(1, 2, 40);
    quicktime.movie.tracks[0].entries[0].quicktime_version = 1;
    quicktime.movie.tracks.push(TrackSpec::audio(
        2,
        vec![Entry {
            quicktime_version: 2,
            audio_config: None,
            ..Entry::cenc(16)
        }],
    ));
    quicktime.fragments.push(FragmentSpec::simple(2, 2, 40, 16));
    files.push(("quicktime_versions", quicktime));

    let mut v1 = plain(1, 2, 40);
    v1.movie.tracks[0].version1_headers = true;
    v1.movie.tracks[0].timescale = 48_000;
    files.push(("version1_headers", v1));

    let mut video = plain(1, 2, 40);
    let mut visual = TrackSpec::audio(
        2,
        vec![Entry {
            kind: *b"encv",
            format: *b"avc1",
            audio_config: None,
            ..Entry::cenc(16)
        }],
    );
    visual.visual = true;
    video.movie.tracks.push(visual);
    video.fragments.push(FragmentSpec::simple(2, 2, 300, 16));
    files.push(("encv", video));

    let mut opaque = plain(1, 2, 40);
    opaque.movie.tracks.push(TrackSpec::audio(
        2,
        vec![Entry {
            kind: *b"Opus",
            audio_config: None,
            ..Entry::clear()
        }],
    ));
    let mut traf = TrafSpec::new(2, vec![RunSpec::of(2, 30, 0)]);
    traf.aux = Aux::None;
    opaque.fragments.push(FragmentSpec::new(vec![traf]));
    files.push(("opaque_entry", opaque));

    files
}

/// The layout of [`corpus`] named `name`.
pub fn named(name: &str) -> FileSpec {
    corpus()
        .into_iter()
        .find(|(found, _)| *found == name)
        .map(|(_, spec)| spec)
        .unwrap_or_else(|| panic!("the corpus has a layout named {name}"))
}

/// A file of Apple's shape (§10): 9 000 samples in 100 fragments, about 800 bytes each, scheme
/// `cenc`, 8-byte IVs, one subsample each.
pub fn apple_shape() -> FileSpec {
    let mut spec = plain(0, 0, 0);
    spec.movie.tracks[0].entries[0] = Entry::cenc(8);
    spec.movie.tracks[0].trex = Some([1, 1024, 0, 0]);
    for fragment in 0..100u32 {
        let mut run = RunSpec::of(90, 800, 8);
        for (n, sample) in run.samples.iter_mut().enumerate() {
            let size = 780 + ((fragment as usize * 7 + n * 13) % 41) as u32;
            sample.size = size;
            sample.iv = ((fragment as u64) << 32 | n as u64).to_be_bytes().to_vec();
            sample.subsamples = vec![(7, size - 7)];
        }
        spec.fragments
            .push(FragmentSpec::new(vec![TrafSpec::new(1, vec![run])]));
    }
    spec
}

/// Where the first box of type `kind` starts: the four bytes before its type. The layouts put
/// no box types in their sample bytes, so the first occurrence is the box.
pub fn offset_of(bytes: &[u8], kind: &[u8; 4]) -> usize {
    bytes
        .windows(4)
        .position(|window| window == kind)
        .expect("the box is in the file")
        - 4
}

/// Writes a 32-bit value at `at`.
pub fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_be_bytes());
}

/// The top-level boxes of `bytes`, read leniently: start, end and type of each.
pub fn top_level_boxes(bytes: &[u8]) -> Vec<(usize, usize, [u8; 4])> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 8 <= bytes.len() {
        let size = u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
        let kind = [bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]];
        let end = match size {
            0 => bytes.len(),
            1 if at + 16 <= bytes.len() => {
                let large = u64::from_be_bytes(bytes[at + 8..at + 16].try_into().unwrap());
                match usize::try_from(large) {
                    Ok(large) if large >= 16 => at.saturating_add(large),
                    _ => break,
                }
            }
            n if n >= 8 => at + n as usize,
            _ => break,
        };
        out.push((at, end.min(bytes.len()), kind));
        if end >= bytes.len() {
            break;
        }
        at = end;
    }
    out
}

/// The `ftyp` and `moov` of a file with one clear track, id 1, and no fragment.
fn clear_movie() -> Vec<u8> {
    let spec = FileSpec {
        leading: ftyp(),
        movie: MovieSpec {
            tracks: vec![TrackSpec::audio(1, vec![Entry::clear()])],
            ..MovieSpec::default()
        },
        ..FileSpec::default()
    };
    build(&spec, false).bytes
}

/// The length of a `moof` that [`moof_of_runs`] makes of runs `runs_len` bytes long in all.
fn moof_of_runs_len(runs_len: usize) -> usize {
    // The moof's header, the mfhd, the traf's header, and a tfhd with a default size.
    8 + 16 + 8 + 20 + runs_len
}

/// A `moof` of one `traf` for track 1 holding `truns`: its `tfhd` gives every sample
/// `default_size` bytes, and counts data offsets from the `moof`.
fn moof_of_runs(default_size: u32, truns: &[Vec<u8>]) -> Vec<u8> {
    let mut tfhd = 1u32.to_be_bytes().to_vec();
    tfhd.extend(default_size.to_be_bytes());
    let traf = [full(b"tfhd", 0, 0x02_0010, &tfhd), truns.concat()].concat();
    let moof = boxed(
        b"moof",
        &[
            full(b"mfhd", 0, 0, &1u32.to_be_bytes()),
            boxed(b"traf", &traf),
        ]
        .concat(),
    );
    assert_eq!(
        moof.len(),
        moof_of_runs_len(truns.iter().map(Vec::len).sum())
    );
    moof
}

/// A file whose one `moof` holds `truns` runs without a field per sample, each declaring as
/// many zero-size samples as the `moof` has bytes: each run alone is justified by the `moof`,
/// all together are not (§9).
pub fn zero_size_runs(truns: usize) -> Vec<u8> {
    let count = u32::try_from(moof_of_runs_len(16 * truns)).expect("a small moof");
    let trun = full(b"trun", 0, 0, &count.to_be_bytes());
    [clear_movie(), moof_of_runs(0, &vec![trun; truns])].concat()
}

/// A file whose one `moof` holds `truns` runs of one-byte samples, sizes from the `tfhd`, each
/// run's data offset pointing at the same `mdat_len` bytes of the one `mdat`: every run owns
/// them all.
pub fn shared_byte_runs(truns: usize, mdat_len: usize) -> Vec<u8> {
    let data_offset = moof_of_runs_len(20 * truns) + 8;
    let mut body = u32::try_from(mdat_len).unwrap().to_be_bytes().to_vec();
    body.extend(u32::try_from(data_offset).unwrap().to_be_bytes());
    let trun = full(b"trun", 0, 0x1, &body);
    [
        clear_movie(),
        moof_of_runs(1, &vec![trun; truns]),
        boxed(b"mdat", &vec![0; mdat_len]),
    ]
    .concat()
}

/// [`shared_byte_runs`] with each sample's size in its `trun`: the bytes shared are known only
/// once a run is read.
pub fn shared_byte_sized_runs(truns: usize, count: usize) -> Vec<u8> {
    let data_offset = moof_of_runs_len((20 + 4 * count) * truns) + 8;
    let mut body = u32::try_from(count).unwrap().to_be_bytes().to_vec();
    body.extend(u32::try_from(data_offset).unwrap().to_be_bytes());
    body.extend(1u32.to_be_bytes().repeat(count));
    let trun = full(b"trun", 0, 0x201, &body);
    [
        clear_movie(),
        moof_of_runs(1, &vec![trun; truns]),
        boxed(b"mdat", &vec![0; count]),
    ]
    .concat()
}

/// A file of `fragments` fragments of `mdat_len` one-byte samples, sizes from the `tfhd`: the
/// first has its `mdat`, every later one a negative data offset pointing at that same `mdat`.
pub fn fragments_sharing_bytes(fragments: usize, mdat_len: usize) -> Vec<u8> {
    let run = |data_offset: i64| {
        let mut body = u32::try_from(mdat_len).unwrap().to_be_bytes().to_vec();
        body.extend(i32::try_from(data_offset).unwrap().to_be_bytes());
        full(b"trun", 0, 0x1, &body)
    };
    let moof_len = moof_of_runs_len(20);
    let mut file = clear_movie();
    let payload = file.len() + moof_len + 8;
    file.extend(moof_of_runs(1, &[run((moof_len + 8) as i64)]));
    file.extend(boxed(b"mdat", &vec![0; mdat_len]));
    for _ in 1..fragments {
        let moof_at = file.len();
        file.extend(moof_of_runs(1, &[run(payload as i64 - moof_at as i64)]));
    }
    file
}

/// Files whose `trun`s declare more samples than their bytes justify (§9), at the size of a
/// few kilobytes and of tens: each would make the table grow far faster than the file.
pub fn more_samples_than_bytes() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("zero_size_runs", zero_size_runs(200)),
        ("zero_size_runs, large", zero_size_runs(3000)),
        ("shared_byte_runs", shared_byte_runs(200, 1000)),
        ("shared_byte_runs, large", shared_byte_runs(3000, 20_000)),
        ("shared_byte_sized_runs", shared_byte_sized_runs(50, 20)),
        (
            "fragments_sharing_bytes",
            fragments_sharing_bytes(100, 1000),
        ),
        (
            "fragments_sharing_bytes, large",
            fragments_sharing_bytes(1000, 20_000),
        ),
    ]
}

/// The malformed files of §13, each named: every one an error, never a panic.
pub fn broken() -> Vec<(&'static str, Vec<u8>)> {
    let plain_file = build(&plain(2, 3, 50), false).bytes;
    let mut files = Vec::new();

    let mut file = plain_file.clone();
    let tkhd = offset_of(&file, b"tkhd");
    put_u32(&mut file, tkhd, 4);
    files.push(("size_below_header", file));

    let mut file = plain_file.clone();
    put_u32(&mut file, tkhd, 100_000);
    files.push(("size_past_container", file));

    let mut file = plain_file.clone();
    put_u32(&mut file, 0, 3);
    files.push(("top_level_size_below_header", file));

    let mut nested = boxed(b"trak", &[]);
    for _ in 0..15 {
        nested = boxed(b"trak", &nested);
    }
    files.push(("nesting_17", [ftyp(), boxed(b"moov", &nested)].concat()));

    let mut file = plain_file.clone();
    let stsd = offset_of(&file, b"stsd");
    put_u32(&mut file, stsd + 12, 0xFFFF_FFFF);
    files.push(("stsd_count_huge", file));

    let mut file = plain_file.clone();
    let trun = offset_of(&file, b"trun");
    put_u32(&mut file, trun + 12, 0x00FF_FFFF);
    files.push(("trun_count_huge", file));

    let seig_file = build(&named("seig"), false).bytes;
    let mut file = seig_file.clone();
    let sgpd = offset_of(&file, b"sgpd");
    put_u32(&mut file, sgpd + 20, 0xFFFF_FFF0);
    files.push(("sgpd_count_huge", file));

    let mut file = seig_file.clone();
    let sbgp = offset_of(&file, b"sbgp");
    put_u32(&mut file, sbgp + 16, 0xFFFF_FFF0);
    files.push(("sbgp_count_huge", file));

    let mut file = seig_file.clone();
    put_u32(&mut file, sbgp + 20, 2);
    files.push(("sbgp_maps_too_few", file));

    let mut file = seig_file;
    put_u32(&mut file, sbgp + 24, 0x10000);
    files.push(("group_index_0x10000", file));

    let pssh_file = build(&named("pssh"), false).bytes;
    let mut file = pssh_file;
    let pssh = offset_of(&file, b"pssh");
    let second = pssh + 8 + 4 + 16 + 4 + 6;
    put_u32(&mut file, second + 28, 0xFFFF_FFF0);
    files.push(("pssh_kid_count_huge", file));

    let subsampled = build(&named("iv8_subsamples"), false).bytes;
    let mut file = subsampled.clone();
    let senc = offset_of(&file, b"senc");
    file[senc + 16 + 8] = 0xFF;
    file[senc + 16 + 9] = 0xFF;
    files.push(("subsample_count_huge", file));

    let mut file = subsampled;
    file[senc + 16 + 8 + 2 + 5] += 1;
    files.push(("subsamples_dont_add_up", file));

    let mut file = plain_file.clone();
    let senc = offset_of(&file, b"senc");
    put_u32(&mut file, senc + 12, 4);
    files.push(("senc_count_mismatch", file));

    let radio_file = build(&radio(), false).bytes;
    let senc = offset_of(&radio_file, b"senc");
    let mut file = radio_file.clone();
    file[senc + 11] = 0x2;
    files.push(("empty_senc_subsampled", file));

    let mut file = radio_file.clone();
    put_u32(&mut file, senc + 12, 2);
    files.push(("radio_senc_count_mismatch", file));

    let mut file = radio_file;
    file.splice(senc + 16..senc + 16, [0; 16]);
    put_u32(&mut file, senc, 32);
    let moof = offset_of(&file, b"moof");
    let traf = offset_of(&file, b"traf");
    for at in [moof, traf] {
        let size = u32::from_be_bytes(file[at..at + 4].try_into().unwrap());
        put_u32(&mut file, at, size + 16);
    }
    let trun = offset_of(&file, b"trun");
    let data_offset = u32::from_be_bytes(file[trun + 16..trun + 20].try_into().unwrap());
    put_u32(&mut file, trun + 16, data_offset + 16);
    files.push(("empty_senc_with_bytes_left", file));

    let mut spec = plain(1, 3, 50);
    spec.fragments[0].trafs[0].aux = Aux::EmptySenc;
    files.push(("empty_senc_per_sample_iv", build(&spec, false).bytes));

    let mut spec = radio();
    spec.movie.tracks[0].groups = vec![Seig::key(OTHER_KEY, 16)];
    spec.fragments.truncate(1);
    spec.fragments[0].trafs[0].sbgp = vec![(3, 0), (1, 1), (1, 0)];
    files.push(("empty_senc_seig_per_sample_iv", build(&spec, false).bytes));

    let senc_longer = {
        let mut spec = plain(1, 3, 50);
        spec.fragments[0].trafs[0].runs[0].samples[2].iv = vec![1; 24];
        build(&spec, false).bytes
    };
    files.push(("senc_too_long", senc_longer));

    let senc_shorter = {
        let mut spec = plain(1, 3, 50);
        spec.fragments[0].trafs[0].runs[0].samples[2].iv = vec![1; 8];
        build(&spec, false).bytes
    };
    files.push(("senc_too_short", senc_shorter));

    let mut spec = plain(1, 2, 64);
    spec.movie.tracks[0].entries[0] = Entry {
        scheme: *b"cbcs",
        ..Entry::cenc(8)
    };
    spec.fragments[0] = FragmentSpec::simple(1, 2, 64, 8);
    files.push(("iv8_under_cbcs", build(&spec, false).bytes));

    let mut spec = plain(1, 2, 64);
    spec.movie.tracks[0].entries[0] = Entry::cbcs((1, 9), vec![0x44; 16]);
    spec.movie.tracks[0].groups = vec![Seig::key(OTHER_KEY, 8)];
    spec.fragments[0] = FragmentSpec::simple(1, 2, 64, 8);
    spec.fragments[0].trafs[0].sbgp = vec![(2, 1)];
    files.push(("seig_iv8_under_cbcs", build(&spec, false).bytes));

    let mut spec = plain(1, 2, 64);
    spec.movie.tracks[0].entries[0] = Entry::cbcs((1, 9), vec![0x44; 8]);
    spec.fragments[0] = FragmentSpec::simple(1, 2, 64, 0);
    files.push(("constant_iv8_under_cbcs", build(&spec, false).bytes));

    let mut file = plain_file.clone();
    let trun = offset_of(&file, b"trun");
    put_u32(&mut file, trun + 16, 8);
    files.push(("sample_outside_mdat", file));

    let mut file = plain_file.clone();
    put_u32(&mut file, trun + 16, 100_000);
    files.push(("sample_past_file", file));

    let mut file = plain_file.clone();
    put_u32(&mut file, trun + 16, (-100_000i32) as u32);
    files.push(("sample_before_file", file));

    let mut file = plain_file.clone();
    let tfhd = offset_of(&file, b"tfhd");
    put_u32(&mut file, tfhd + 12, 9);
    files.push(("unknown_track", file));

    let mut file = plain_file.clone();
    let schm = offset_of(&file, b"schm");
    file[schm + 12..schm + 16].copy_from_slice(b"abcd");
    files.push(("unknown_scheme", file));

    let mut file = plain_file.clone();
    let sinf = offset_of(&file, b"sinf");
    file[sinf + 4..sinf + 8].copy_from_slice(b"free");
    files.push(("enca_without_sinf", file));

    let moof_only = {
        let built = build(&plain(1, 2, 40), false).bytes;
        let moov = top_level_boxes(&built)[1];
        [&built[..moov.0], &built[moov.1..]].concat()
    };
    files.push(("no_moov", moof_only));

    let mut file = plain_file.clone();
    let moof = offset_of(&file, b"moof");
    file.truncate(moof + 40);
    files.push(("cut_moof", file));

    let mut file = plain_file.clone();
    file.truncate(file.len() - 10);
    files.push(("cut_mdat", file));

    let mut file = plain_file.clone();
    file.truncate(file.len() - 1);
    file[moof..moof + 4].copy_from_slice(&0u32.to_be_bytes());
    files.push(("open_moof_cut_mdat", file));

    let mut file = plain_file.clone();
    let tenc = offset_of(&file, b"tenc");
    file[tenc + 14] = 0;
    files.push(("tenc_unprotected", file));

    let mut file = plain_file.clone();
    file[tenc + 15] = 7;
    files.push(("tenc_iv_size_7", file));

    let mut spec = plain(1, 2, 40);
    spec.fragments[0].trafs[0].aux = Aux::None;
    files.push(("missing_ivs", build(&spec, false).bytes));

    let saiz_only = {
        let mut spec = plain(1, 3, 50);
        spec.fragments[0].trafs[0].aux = Aux::SaizSaio;
        let mut file = build(&spec, false).bytes;
        let saio = offset_of(&file, b"saio");
        file[saio + 4..saio + 8].copy_from_slice(b"free");
        file
    };
    files.push(("saiz_without_saio", saiz_only));

    let mut file = plain_file.clone();
    put_u32(&mut file, 0, 1);
    files.push(("largesize_cut", file[..12].to_vec()));

    let mut file = plain_file;
    let trex = offset_of(&file, b"trex");
    put_u32(&mut file, trex + 16, 0);
    files.push(("description_index_0", file));

    files.push(("zero_size_runs", zero_size_runs(200)));
    files.push(("shared_byte_runs", shared_byte_runs(200, 1000)));
    files.push(("shared_byte_sized_runs", shared_byte_sized_runs(50, 20)));
    files.push((
        "fragments_sharing_bytes",
        fragments_sharing_bytes(100, 1000),
    ));

    files
}

/// A small deterministic generator (xorshift64*), for the mutation tests.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// `seed` with one of its `moov` or `moof` boxes damaged: a byte flipped, set, cut, repeated or
/// inserted. The audio is left alone.
pub fn mutate(rng: &mut Rng, seed: &[u8]) -> Vec<u8> {
    let targets: Vec<(usize, usize)> = top_level_boxes(seed)
        .into_iter()
        .filter(|(_, _, kind)| matches!(kind, b"moov" | b"moof"))
        .map(|(start, end, _)| (start, end))
        .collect();
    let (start, end) = if targets.is_empty() {
        (0, seed.len())
    } else {
        targets[rng.below(targets.len())]
    };
    let mut out = seed.to_vec();
    if end <= start {
        return out;
    }
    let at = start + rng.below(end - start);
    let length = 1 + rng.below(16.min(end - at));
    match rng.below(6) {
        0 => out[at] ^= 1 << rng.below(8),
        1 => out[at] = [0x00, 0xFF, 0x7F, 0x80, 0x01][rng.below(5)],
        2 => {
            let value = (rng.next() as u32) >> rng.below(32);
            let end = (at + 4).min(out.len());
            out[at..end].copy_from_slice(&value.to_be_bytes()[..end - at]);
        }
        3 => {
            out.drain(at..at + length);
        }
        4 => {
            let piece = out[at..at + length].to_vec();
            out.splice(at..at, piece);
        }
        _ => {
            let piece: Vec<u8> = (0..length).map(|_| rng.next() as u8).collect();
            out.splice(at..at, piece);
        }
    }
    out
}
