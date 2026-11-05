# Encrypted fragmented MP4: what Opium reads and changes

The behaviour that `crates/cenc` must have. It is written from the public standards and from what
Apple Music's and SoundCloud's files contain, without reference to any other program's code; it is
the only input of the clean-room rewrite ([README](README.md)). It says what the code must do, not
how to structure it.

Standards cited:

- **ISO/IEC 14496-12** (ISO base media file format, "BMFF"): boxes, movie and fragments.
- **ISO/IEC 23001-7** (Common Encryption, "CENC"): protection boxes and schemes.
- **ISO/IEC 14496-1** §7.2.6 (descriptors) and **14496-3** §1.6.2.1 (`AudioSpecificConfig`).

## 1. What the files are

Both services serve one audio track as one fragmented MP4 file, or as byte ranges of one:

- a movie box `moov` (the *init*) with one `trak`, whose sample entry is `enca` (encrypted audio);
- then fragments, each a `moof` followed by its `mdat`; one AAC-LC frame per sample.

Variations met in practice, all to be handled:

- **Apple's catalog songs**: scheme `cenc` (AES-CTR), a `tenc` naming the key, 8- or 16-byte IVs per
  sample in `senc`, subsamples present or not. About 9 000 samples in 7 MB for four minutes.
- **Apple's Spatial Audio** (enhanced HLS): several sample entries in one `stsd`, one per key, all
  the same audio; each fragment's `tfhd` names its entry through `sample_description_index`. The
  init's `tenc` key ids are all zero; the playlist names the keys (see
  [apple-music-playback.md](apple-music-playback.md)).
- **Apple's live radio**: scheme `cbcs` (AES-CBC), the crypt:skip pattern and a constant IV in
  `tenc`, segments arriving one at a time.
- **SoundCloud's protected tracks**: scheme `cenc`, one key.
- **A user's upload on Apple Music**: possibly not encrypted at all (sample entry `mp4a`).

## 2. Reading boxes (14496-12 §4.2)

- A box starts with a 32-bit big-endian `size` and a four-character `type`.
- `size == 1`: a 64-bit `largesize` follows the type; the header is 16 bytes.
- `size == 0`: the box runs to the end of its container (for a top-level box, to the end of the
  data known so far, and it is only complete once the whole file is known).
- `type == "uuid"`: a 16-byte extended type follows; such boxes are skipped.
- A *full box* adds one byte of `version` and three bytes of `flags` after the header. In this
  file, a box's **body** is what follows its type (or its `largesize`, or its extended type): for a
  full box, the body starts with `version` and `flags`, and offsets "in the body" count from there.
- A size smaller than its own header, or one running past its container, makes the box malformed:
  it is reported, never read past. Arithmetic on sizes and offsets is checked: a 64-bit size on a
  32-bit target that does not fit is malformed, not truncated.
- Nesting is bounded (16 levels is far beyond any real file); deeper is malformed.
- Box order inside a container is not assumed beyond what the standard requires; unknown boxes are
  skipped by their size.

## 3. The init: tracks and their protection

Path: `moov` → `trak` → `mdia` → `minf` → `stbl` → `stsd`. Every `trak` is read, not only the first.

- `tkhd` (full box): `track_ID` is at body offset 12 for version 0, 20 for version 1 (after the
  creation and modification times and their widths).
- `mdhd` (full box): the track's `timescale` (offset 12 in version 0, 20 in version 1).
- `stsd` (full box): a 32-bit `entry_count`, then that many sample entries.
- A sample entry starts with 6 reserved bytes and a 16-bit `data_reference_index`. An audio sample
  entry (`mp4a`, `enca`, …) then has 20 bytes of audio fields before its child boxes: 28 bytes of
  body in all. Its first 16-bit field is a version: QuickTime's version 1 adds 16 bytes before the
  children, its version 2 adds 36; both are read. A visual one has 70 more bytes (78 in all). Only
  audio is required; video entries are recognised and skipped.
- An encrypted entry (`enca`, `encv`) holds a `sinf`:
  - `frma`: the original format (`mp4a` for these services), which the entry takes back once
    clear;
  - `schm` (full box): `scheme_type` (`cenc`, `cbcs`, `cens`, `cbc1`) and its version;
  - `schi` → `tenc` (23001-7 §8.2, full box):
    - one reserved byte;
    - version 0: one more reserved byte; version ≥ 1: `default_crypt_byte_block` (high nibble)
      and `default_skip_byte_block` (low nibble);
    - `default_isProtected` (1 byte), `default_Per_Sample_IV_Size` (1 byte: 0, 8 or 16);
    - `default_KID` (16 bytes);
    - when protected and the IV size is 0: `default_constant_IV_size` (1 byte: 8 or 16) and the
      constant IV.
- `esds` inside the sample entry (ISO/IEC 14496-14 §5.6): a full box, so four bytes of version and
  flags, then the descriptors of 14496-1 §7.2.6: each with a tag byte and a length of one
  to four bytes, seven bits each, the high bit saying another follows. `ES_Descriptor` (tag 3):
  16-bit id, a flags byte, then a 16-bit `dependsOn_ES_ID` if flag 0x80, a URL of length-prefixed
  bytes if 0x40, a 16-bit `OCR_ES_Id` if 0x20, then its children. `DecoderConfigDescriptor`
  (tag 4): 13 bytes, then children. `DecoderSpecificInfo` (tag 5): the `AudioSpecificConfig`.
- The `AudioSpecificConfig` (14496-3 §1.6.2.1): five bits of object type (31 escapes to 32 plus
  six more bits), four bits of sampling frequency index (table 1.18: 96000, 88200, 64000, 48000,
  44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350; 15 means a 24-bit explicit rate
  follows), four bits of channel configuration. For object types 5 (SBR) and 29 (PS), an extension
  sampling frequency index (or explicit rate) follows, then the core object type: the rate to
  report is the extension's, the output rate, not the core's. Implicit SBR, which nothing in the
  configuration says, is not detected: the decoder reports that rate. Whether Apple's `ctrp64`
  flavour is HE-AAC is not verified.
- `mvex` → `trex` (full box): per track, the fragment defaults: `default_sample_description_index`,
  `default_sample_duration`, `default_sample_size`, `default_sample_flags`.
- `pssh` boxes may sit in `moov` or `moof`: they are reported (system id, key ids for version 1,
  and data), never needed to locate samples.

What the init yields: per track, its id, timescale, each sample entry (its original format,
scheme, pattern, IV size, constant IV, key id, `AudioSpecificConfig`), its `trex` defaults and its
`seig` sample group descriptions; and the `pssh` boxes.

## 4. Fragments: where each sample is (14496-12 §8.8)

For each `moof`, each `traf`:

- `tfhd` (full box) gives the `track_ID`, then, as its flags say, in this order:
  `0x000001` base data offset (64 bits), `0x000002` sample description index (32 bits, from 1),
  `0x000008` default duration, `0x000010` default size, `0x000020` default flags;
  `0x010000` duration is empty; `0x020000` default base is moof.
- `tfdt` (full box): the fragment's decode time (32 bits in version 0, 64 in version 1).
- One or more `trun` (full box): `sample_count`, then as its flags say `0x000001` a signed 32-bit
  data offset, `0x000004` first sample flags; then per sample, as flagged, `0x000100` duration,
  `0x000200` size, `0x000400` flags, `0x000800` composition time offset (signed in version 1).
- Each value missing from `trun` comes from `tfhd`, else from `trex`.
- **The base of data offsets** (§8.8.7.1): the `tfhd` base data offset when present; else the start
  of the `moof` when `default-base-is-moof` is set, and for the first `traf` of a `moof` in any
  case; else the end of the previous `traf`'s data. A `trun`'s data offset is added to that base. A
  first `trun` without a data offset starts at the base itself; a later one continues where the
  previous `trun`'s data ended.
- A sample's bytes must lie inside an `mdat` that is known; one that does not is malformed. The
  `mdat` need not follow its `moof` immediately, and may have a 64-bit size: offsets are absolute
  in the file.
- Zero-size samples are allowed and have nothing to decrypt.

## 5. Each sample's decryption inputs (23001-7)

- **IVs and subsamples** come from `senc` (§7.2, full box): flag `0x000002` says subsamples are
  present; `sample_count`, which must equal the number of samples of the `traf`, all its `trun`s
  together; per sample an IV of the sample's IV size (none when it is 0, the constant IV then
  serving), then if flagged a 16-bit `subsample_count` and that many pairs of 16-bit clear bytes and
  32-bit protected bytes.
- **The IV size** is the sample's `seig` group's when it has one (below), else its sample entry's
  `tenc`'s. It is never guessed: an `senc` whose length does not match it exactly is malformed.
- **Where there is no `senc`**, `saiz` and `saio` (§8.7.8, §8.7.9, full boxes) locate the same
  auxiliary information elsewhere in the file. When their flag `0x000001` is set, each starts with a
  32-bit `aux_info_type` (`cenc`, `cbcs`, …) and a 32-bit `aux_info_type_parameter`, which shift
  what follows. `saiz`: a default sample info size, then the sample count, then, only when the
  default is 0, one size byte per sample. `saio`: an entry count, then offsets of 32 bits (version
  0) or 64 (version 1). In a `traf`, there is one offset, or one per `trun`: each gives where its
  run's first sample's information is, the others following it back to back, of the sizes `saiz`
  gives. Offsets in a `traf` count from the same base as the data offsets (above); in `moov`, they
  are absolute in the file. Each sample's information is laid out as in `senc`: its IV, then, when
  its size is larger than the IV, a 16-bit subsample count and the pairs. Neither service is known
  to use them today; they are read so that a change does not break playback.
- **Sample groups** (§6; 14496-12 §8.9): an `sbgp` of grouping type `seig` maps runs of samples to a
  group description index; an `sgpd` of the same type lists the descriptions. Index 0 means no group
  (the `tenc` applies); 1 to 0xFFFF names an entry of the init's `sgpd` for that track (in `stbl`);
  0x10001 and above name entry (index − 0x10000) of the `traf`'s own `sgpd`. Index 0x10000 itself is
  malformed. An `sgpd` of version 1 gives a `default_length`, and when it is 0 each entry is
  preceded by its 32-bit `description_length` (constant IVs make entries of different lengths);
  version 2 adds a `default_sample_description_index`, which changes nothing here; version 0, which
  the standard deprecates, has entries of `seig`'s fixed 20 bytes, and no constant IV. A `seig`
  entry holds: one reserved byte; one byte of crypt (high nibble) and skip (low nibble) blocks;
  `isProtected` (1 byte); `Per_Sample_IV_Size` (1 byte); the key id (16 bytes); and when protected
  with an IV size of 0, a constant IV size (1 byte) and the constant IV. It overrides the `tenc` for
  its samples.
- **Subsample checks**: the sum of a sample's clear and protected bytes must equal its size; a
  sample with no subsamples is protected whole.
- **The IV given on**: always 16 bytes. An 8-byte IV (allowed for `cenc` and `cens`) is followed by
  eight zero bytes: it is the upper half of the counter block (23001-7 §9.1). The CBC schemes
  (`cbc1`, `cbcs`) take 16-byte IVs only; an 8-byte IV or constant IV under them is malformed.
- **The schemes**: `cenc` (AES-128-CTR); `cbcs` (AES-128-CBC with a crypt:skip pattern of 16-byte
  blocks, the constant IV starting again at each subsample's protected range); `cens` and `cbc1` are
  reported, and need not be supported by the CDM host. A pattern of 0:0 means no pattern: every
  whole 16-byte block of a protected range is encrypted. In the CBC schemes, a partial block at the
  end of a protected range stays clear.

What a fragment yields, per sample: its absolute offset and size, its duration in the track's
timescale, the sample entry it uses, the key id, the IV (8 or 16 bytes, or the constant one), the
subsample runs, and the scheme with its pattern.

## 6. Making the file clear

Once every sample is decrypted in place (CENC keeps sizes), the file is made to read as a plain
MP4, changing only four-byte types and counts, so that every offset stays true:

- each encrypted sample entry takes back the type its `frma` names;
- inside it, `sinf` becomes `free`;
- in every `moof` and in `moov`: `senc`, `saiz`, `saio`, `pssh`, and the `sbgp`/`sgpd` of type
  `seig` become `free` (ExoPlayer refuses a `saiz` whose track says nothing of encryption);
- where a track has several sample entries that are the same audio under several keys (Spatial
  Audio), `stsd`'s `entry_count` becomes 1, and every sample description index that names an entry
  becomes 1: each `tfhd`'s, and the `trex`'s default. Decoders that take a single entry refuse the
  others.

Nothing else is rewritten; the bytes of samples are only changed by decryption.

## 7. Reading while the file arrives

A file is indexed as it downloads:

- the init is reported only once the whole `moov` has arrived;
- each fragment is reported once its `moof` and every byte its samples reference have arrived;
- indexing resumes from where it stopped, never re-reading earlier fragments;
- a fragment never complete when the download ends is reported as truncated, the earlier ones
  staying usable.

## 8. Other questions the callers ask

- The last sample's duration in a fragment (its `trun` entry, else the defaults): an AAC encode's
  last frame is shorter by the padding after the music.
- The key ids of every sample entry, in their order (Spatial Audio has one per entry).
- Writing a key id into a `tenc` whose `default_KID` is zero, by sample entry, for players that
  take keys from the init (Spatial Audio's init, Apple's live radio).

## 9. Errors and robustness

- No input makes the code panic, loop forever, recurse without bound or allocate more than the
  input justifies (a count is checked against the bytes left before anything is reserved).
- Errors say which box was wrong and at which offset: "the `trun` at 0x1a2b runs past its `traf`",
  not "parse error".
- Malformed parts are reported, never silently skipped in a way that would hand undecrypted bytes
  to the decoder as clear.

## 10. Performance

- One pass over the boxes; the samples' table kept compact (no per-sample heap allocation:
  subsample runs stored in one shared array, indexed per sample).
- Measured before and after the rewrite, on a synthetic file of Apple's shape (four minutes,
  9 000 samples, 7 MB, subsamples present): the time to index the whole file (0.37 ms on
  2026-11-05, [the licence audit](../audits/2026-11-05-licence.md#5-measured)) and the memory of the
  table (632 KiB, plus 140 KiB in 9 000 allocations of subsample runs). Neither may get worse.
- **Budget**: at most 48 bytes a sample in the table, subsample runs included, measured on the same
  file; a four-hour track (a DJ set: about 460 MB, 540 000 samples) then takes under 26 MB, and
  indexes in time proportional to its size.

## 11. The crate's public interface

These names are fixed: the callers are moved to them. Everything not listed here is private.

| Item | What it gives |
|---|---|
| `cenc::Movie::read(data: &[u8]) -> Result<Option<Movie>, cenc::Error>` | `None` until the whole `moov` is there; then the movie (§3) |
| `Movie { tracks: Vec<Track>, pssh: Vec<Pssh>, end: usize }` | `end`: where the `moov` ends |
| `Track { id: u32, timescale: u32, entries: Vec<SampleEntry>, defaults: TrackDefaults }` | `defaults`: the `trex` values |
| `SampleEntry { at: usize, format: [u8; 4], protection: Option<Protection>, audio_config: Option<Vec<u8>> }` | `at`: where the entry's box starts; `format`: the original format |
| `Protection { scheme: Scheme, key_id: [u8; 16], iv_size: u8, constant_iv: Option<Vec<u8>>, pattern: Pattern }` | From `schm` and `tenc` |
| `Scheme { Cenc, Cens, Cbc1, Cbcs }`, `Pattern { crypt: u8, skip: u8 }` | |
| `Pssh { system_id: [u8; 16], key_ids: Vec<[u8; 16]>, data: Vec<u8> }` | |
| `cenc::Indexer::new(movie: &Movie) -> Indexer` | Indexes fragments as bytes arrive (§7) |
| `Indexer::advance(&mut self, data: &[u8]) -> Result<usize, cenc::Error>` | `data` is everything known so far, from the file's start; returns where the first fragment not yet whole begins |
| `Indexer::finish(&mut self, data: &[u8]) -> Result<(), cenc::Error>` | The download has ended: a fragment still incomplete is the error |
| `Indexer::samples(&self) -> &SampleTable` | |
| `SampleTable::len`, `is_empty`, `get(i) -> Option<Sample<'_>>`, `iter()` | In file order |
| `Sample<'a> { offset: usize, size: usize, duration: u32, fragment: usize, entry: u32, key_id: [u8; 16], iv: [u8; 16], iv_size: u8, subsamples: &'a [(u32, u32)], protected: bool, scheme: Scheme, pattern: Pattern }` | `iv` is padded to 16 bytes (§5); `iv_size` says how many came from the file; `entry` counts from 0; `fragment`: where its `moof` starts; `subsamples` as (clear, protected) bytes, the shape `widevine` takes, so that nothing is converted per sample |
| `cenc::index(data: &[u8]) -> Result<(Movie, SampleTable), cenc::Error>` | A whole file at once |
| `cenc::make_clear(buf: &mut [u8], movie: &Movie, samples: &SampleTable)` | §6 |
| `cenc::last_sample_duration(fragment: &[u8]) -> Option<u32>` | §8 |
| `cenc::write_key_ids(init: &mut [u8], ids: &[[u8; 16]])` | §8: the zero key ids, entry by entry, in order |
| `cenc::Error { offset: usize, kind: [u8; 4], what: String }` with `Display` | §9: which box, where, what |

Apple's live radio hands each segment over alone, its own `moov`'s movie given: offsets then count
from the segment's start. A download hands over one growing file: offsets count from the file's
start.

`AudioStream` (`src/stream.rs`) is Opium's own and keeps its role: a downloaded track readable as a
file while a background thread decrypts it. Its decrypting function becomes
`Box<dyn FnMut(&mut [u8], &Sample<'_>) -> Result<(), String> + Send>`, which decrypts the sample's
bytes **in place** in the stream's own buffer: one copy less per sample.

## 12. What the clean side is given

An export with no history, built by the dirty side:

- this file and [README.md](README.md);
- `crates/cenc/Cargo.toml`, a workspace `Cargo.toml` of its own with the root's
  `[workspace.package]`, and `Cargo.lock`'s part that the crate needs (none: it has no dependency);
- `crates/cenc/src/stream.rs`, Opium's own, its uses of the old types already renamed to §11's;
- `crates/cenc/src/lib.rs` emptied of the old exports, and no `fmp4.rs`.

The callers outside the crate are moved to §11's names by the dirty side, after the clean side's
code is in: `apple-music` (`playback.rs`, `radio.rs`), `soundcloud` (`stream.rs`) and their
examples.

## 13. Tests the rewrite brings

Besides today's tests of `src/stream.rs`, each passing:

- each box layout of §2 to §5 on a file built in the test: 32- and 64-bit sizes, size 0, `uuid`,
  two tracks, two `trun`s in a `traf`, two `traf`s in a `moof`, base data offset, `default-base-is-
  moof`, an `mdat` not right after its `moof`, a negative data offset, `trex` defaults, IV sizes 0
  (constant IV), 8 and 16, subsamples present or not, `saiz`/`saio` without `senc`, a `seig` group,
  several sample entries (Spatial Audio), `cbcs` with its pattern, an unencrypted `mp4a` track;
- each malformed case of §9 an error, never a panic: sizes below their header or past their
  container, nesting 17 deep, counts larger than the bytes left, subsamples not adding up to the
  sample, an `senc` whose count differs from its `traf`'s samples, an `senc` whose length does not
  fit its IV size, an 8-byte IV under `cbcs`, a sample outside every `mdat`;
- a deterministic mutation test in `cargo test`: thousands of small files made from the test files
  by flipping, cutting and repeating bytes of their `moov` and `moof` boxes (not of the audio) with
  a seeded generator, each indexed without a panic; a hang is caught by counting the parser's steps,
  not by a clock; an allocation beyond the input's size is caught by a counting allocator, in an
  integration test of its own;
- making a file clear changes only the types and counts §6 names, byte for byte.
