# 2026-11-05: `cenc` written again in a clean room (M1.10)

- **Scope**: `crates/cenc`, written again from [its specification](../specs/fmp4-cenc.md) by
  [the method](../specs/README.md) of [ADR-0030](../adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md);
  the callers moved to its names; the token comparison afterwards.
- **Starting commit**: `4522e68`. The old `fmp4.rs` and its notice are deleted.
- **Sides**:
  - dirty side: the session that read Kopuz and the adapted code, and wrote the specification
    (M1.9), the export, `stream.rs`'s move to the new names, the callers and this file;
  - clean side, three new agents in turn, on two different models: round 1 wrote the crate;
    round 2 made it index faster; round 3 fixed what the milestone's review found.

  No clean agent had read Kopuz or the adapted files, before or during.
- This is how a team usually shows that new code is its own; it is not legal advice.

## The export

A new git repository outside Opium's tree, with no history: one commit of the allowed files, then
`Cargo.lock` (commit `6e97f71`):

- `docs/specs/README.md` and `docs/specs/fmp4-cenc.md`;
- the workspace's `Cargo.toml`, with the root's `[workspace.package]`; `crates/cenc/Cargo.toml`;
- `crates/cenc/src/stream.rs`, Opium's own, already on §11's names, its tests building real files;
- `crates/cenc/src/lib.rs`, empty but for the stream's exports;
- `flake.nix` and `flake.lock`, for the nightly shell `.#fuzz`, built beforehand, with
  `libfuzzer-sys` 0.4.13 already in Cargo's cache, so that no network was needed.

A search of the export for Kopuz's and `fmp4.rs`'s names found only the specifications' README,
which names Kopuz and `fmp4.rs` as what the method replaces. Round 2 got the measuring program
`crates/cenc/examples/index_cost.rs` (Opium's own) and the first round's code (commit `7701052`).

## The briefs

Word for word, but for the path of the project's folder:
[round 1](2026-11-05-cenc-clean-room/brief.md), [round 2](2026-11-05-cenc-clean-room/brief-round-2.md),
[round 3](2026-11-05-cenc-clean-room/brief-round-3.md). Each one allows only the export, and
forbids the rest of the disk, Opium's repository and its history, reading `/nix/store` or
`~/.cargo` except through the compiler and Nix, the network, and other agents.

## Questions and answers

The clean side asked no question that stopped it. Its answers to the gaps in the specification,
the dirty side's choice on each:

- **Written into the specification before the hand-over** (commit `39bb0fa`, from moving
  `stream.rs` to the new names): `make_clear` may be called again as fragments arrive, touching only
  fragments indexed; the indexer never reads again the movie box or a fragment indexed; how far a
  step says the file is whole; the traits of the public types; the table holding clear samples too.
- **Round 2** (commit `7701052`): §10 got the figures to beat, from the kept measuring program.
- **Round 3** (commit `958ad1f`): §9 got how a fragment's samples are bounded by the file's
  bytes, after the review showed a `moof` of 66 KB could ask for 8 GB of table.
- **Assumptions the clean side made**, all kept:
  - private fields in `Track`, `Movie` and `SampleEntry` for what the indexer and `make_clear` need
    once the `moov` is clear; the public fields are exactly §11's;
  - `TrackDefaults`' fields named after `trex`'s; a clear sample reports scheme `cenc`, pattern
    0:0, zero key id and IV;
  - `audio_config` holds the raw `DecoderSpecificInfo`, unparsed;
  - `encv` read with its 78-byte layout, other unknown entries kept opaque;
  - `sbgp` must map exactly the `traf`'s samples; duplicate `tfhd`, `senc`, `saiz`, `saio` or
    `seig` `sbgp` is an error; `saiz` without `saio` is an error; `senc` wins over them;
  - zero-size samples need no `mdat` but lie within the file;
  - `write_key_ids` counts entries over every track in file order;
  - Spatial Audio's entries collapse when they are several, all encrypted, of the same format and
    `AudioSpecificConfig`;
  - `Movie::read` never yields a `moov` of size 0 while a file downloads; `index` accepts it;
  - a `pssh` in a `moof` is made `free`, not reported.
- **Kept, but debt** ([debt.md](../debt.md)): a `tenc` with `default_isProtected` 0 is an error;
  a protected sample with no IV is an error, where the radio once used the playlist's IV.

## What the clean side changed in Opium's own file

`stream.rs`, one condition: a read waiting for a sample now fails only when the decrypt thread
itself failed, not when the download did. Before, a download that failed made the read fail
at once, though the samples it had brought could still be decrypted: a race, which the new test
`a_download_cut_inside_a_fragment_fails_where_it_was_cut` showed 3 to 17 times in 20. The dirty
side kept the change and wrote it as a let-chain.

## What the milestone's review found

A reviewer that wrote neither side read the milestone against `AGENTS.md`, ADR-0030, the
specification and this evidence, and compared the new code with Kopuz and `fmp4.rs` itself, down
to 20 raw and 22 normalised tokens. It found the independence sound, and:

- the table could grow with the square of the input (many `trun`s of zero-size samples, or
  pointing at the same bytes): fixed by the clean side in round 3, with tests;
- `AudioStream` panicked on a download ending in a `moof` of size 0, which `Indexer::finish` alone
  makes whole, and would have served it encrypted: fixed by the dirty side in `stream.rs`, Opium's
  own, with the test `a_download_ending_in_an_open_moof_reads_its_samples_decrypted`, and a fuzz
  target over the stream added in round 3;
- undocumented public items, a stale doc comment, the comment phrase below: round 3.

Round 3's fuzz target over the stream found two more faults in `stream.rs`, which the clean side
left to the dirty side, as its brief said: a step that indexed a whole fragment, then met a
malformed one, left the samples unnoted and the decrypt thread panicking; a fragment without
samples was not made clear. Both fixed, each with a test that failed before
(`a_step_that_meets_a_malformed_fragment_keeps_the_whole_ones_before_it`,
`a_fragment_without_samples_is_made_clear_too`); the target then ran 300 s, 3.58 million inputs,
without a fault. Its questions were answered in the specification (commit `81e8181`): the
fragments indexed are made clear with or without samples; a step that meets a malformed fragment
keeps the whole ones before it; no byte is owned by two samples, across fragments too.

The rest of the review:
- the Spatial Audio path read the movie box from the file's first 16 KiB: it now reads the init
  segment at the range its playlist gives; the live radio made empty frames of zero-size samples:
  they are skipped.

## The tool calls

Read by the dirty side from the transcripts, with a script that lists every call and every absolute
path:

| Round | Calls | Reads outside the export | Network | Notes |
|---|---|---|---|---|
| 1 | 55: 45 shell, 9 writes, 1 read | none | none | Fuzz seeds were written once to the Nix shell's `TMPDIR`, then deleted; the agent said so itself. Every `nix` and `cargo` call was offline. |
| 2 | 48: 43 shell, 5 reads | none | none | A throwaway worktree of the export, under its `target/`, timed the starting commit; removed. `GLIBC_TUNABLES` used once to diagnose the allocator. |
| 3 | 61: 60 shell, 1 write | none | none | Crash inputs of the new fuzz target kept under the export's `target/` and `artifacts/`, both ignored. |

No call named Kopuz, `fmp4.rs` other than the specification's file name, Opium's folder, the home
folder, `/nix/store` or `~/.cargo`.

## The token comparison

[`compare.sh`](2026-11-05-cenc-clean-room/compare.sh) runs
[`token_compare.py`](2026-11-05-cenc-clean-room/token_compare.py) over every `.rs` file of
`crates/cenc` (the clean code, `stream.rs`, the example, the tests and the fuzz targets), against
Kopuz at `a58c385` and `1d8b323`, `fmp4.rs` at `4522e68`, and the rest of the tree. The full report
is [`comparison.md`](2026-11-05-cenc-clean-room/comparison.md); it shows only the new code's tokens.

Calibration: `fmp4.rs` against Kopuz finds 542 raw tokens (the audit's 590, with another
tokenizer); Opium's own `soundcloud`, `navidrome` and `synced` (6 687 lines, which owe Kopuz
nothing) share with Kopuz 43 raw tokens at most, and 225 normalised runs of 30 or more, the longest
102.

| Against | Raw: runs of 50 or more | Raw: longest below | Normalised: runs of 30 or more | Normalised: longest |
|---|---|---|---|---|
| Kopuz `a58c385` | 0 | 29 | 155 | 64 |
| Kopuz `1d8b323` | 0 | 29 | 154 | 67 |
| `fmp4.rs` at `4522e68` | 0 | 32 | 14 | 46 |
| The rest of the tree | 1 (106) | 49 | 95 | 106 |

Read by hand:

- **Against Kopuz**: at each revision, 148 of the 155 runs fall in code that does nothing of the
  kind (its interface, store, database, player, other services and vendored crates): Rust's
  common forms (`#[derive(…)] pub struct` with `Vec` fields, `let mut x = Vec::new();` in a row,
  `match` arms on `Option`s). The seven in its Apple Music folder, the same at both revisions: `struct` declarations whose fields bear §11's and the
  standard's names (`Pssh { system_id, key_ids }`, `key_id`, `iv_size`, `track_id`); the idiom
  `u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])`; and `read_to_end` then
  `assert_eq!`, three times in `stream.rs`'s tests.
- **Against `fmp4.rs`**: `struct` declarations, signatures of the shape
  `fn x(data: &[u8], header: Header) -> Result<T, Error> { let mut body = …`, the same
  `from_be_bytes` idiom, and `#[cfg(test)] mod tests { use super::*; fn boxed(kind: &[u8; 4], body:
  &[u8]) -> Vec<u8>`, the head of the dirty side's own tests in `stream.rs`.
- **Against the tree**: the raw run of 106 is `stream.rs`'s `impl Seek`, the same as in
  `downloads/src/sealed.rs`, both Opium's own; the normalised runs are Opium's idioms.
- **Names**: those the new code shares with Kopuz's `cenc.rs` and with `fmp4.rs` are the standard
  library's, §11's, or the standard's (`base_offset`, `constant_iv`, `body_end`, `moof_len`), and
  `clear_fragment`, a function of the old file's own part (not Kopuz's) which the clean side named
  alike for what it does.
- **Strings**: none in common.
- **Test data**: the 13 bytes `[0x40, 0x15, 0, …, 0]` of a `DecoderConfigDescriptor` in
  `tests/build.rs` are the same as in a Kopuz test (a raw run of 28 tokens, under the threshold):
  object type 0x40 (MPEG-4 audio), stream type 0x15 (audio), then rates left zero, as 14496-1
  §7.2.6.6 lays them out. Opium's own `index_cost.rs` holds the same bytes. No other test data in
  common.
- **Comments**: none in common now; the first version had one phrase of six words. Kopuz's and `fmp4.rs`'s "One encrypted sample: where it
  sits in the file, and what the CDM needs to turn it into cleartext" and the first version of the
  new `table.rs`'s "One sample of the file: where it is, and what the CDM needs to decrypt it". The
  export did not hold it; the model may have met the phrase in public code. The clean side said it
  in other words in round 3, without being shown the original.

Nothing above the thresholds is left unexplained; only the comment was said again. A reviewer that wrote
neither side compared them at the milestone's review ([PLAN](../PLAN.md#m110-to-m112--kopuzs-code-written-again)).

## Checked beyond the built files

A CENC file made by FFmpeg 9.0.2 (ten seconds of AAC-LC, `-encryption_scheme cenc-aes-ctr`, a test
key), with throwaway programs outside the tree:

- `cenc::index` finds its 432 samples at the offsets and sizes `ffprobe` gives, its 8-byte IVs and
  its key id;
- decrypted with `openssl` from the table's IVs, then made clear, it decodes to the same PCM as
  FFmpeg's clear file, bit for bit, with FFmpeg and with Symphonia 0.6.1 (Opium's decoder): 432
  packets, 442 368 frames, no error;
- fed to `AudioStream::downloading` in 4 KiB pieces, a decrypt copying the known cleartext, it reads
  out byte for byte as the file made clear whole.

Not checked: Apple's and SoundCloud's own files, `cbcs`, Spatial Audio and the live radio, which
need the owner's accounts and CDM (with the owner, after M1.12).

## Round 4: Apple's live radio, played with the owner

After M1.11, playing with the owner on the owner's profile, Apple Music in stereo and in Spatial
Audio played, but the live radio Apple Music 1 did not: a throwaway diagnostic (statistics only,
nothing played or written; two licences, with the owner's go) found the same error before and after
M1.11, "the `senc` at 0xb367 counts 0 samples where its `traf` has 750". Apple's radio puts an empty
`senc` in every fragment of its `cbcs` streams, with a constant IV; the specification, the dirty
side's, called it malformed.

- The specification answered first (commit `4f7e597`): an empty `senc`, without the subsample flag,
  carries nothing when every sample of its `traf` has an IV size of 0; malformed otherwise.
- A new export, a new repository holding `crates/cenc` as it was at `734f535` with that answer
  (commit `11b3bcb`), and a new agent of the clean side, which never read Kopuz or `fmp4.rs`:
  [its brief](2026-11-05-cenc-clean-room/brief-round-4.md). It asked no question.
- Its change (commit `0f5cca2`): the rule in the one place `senc` is read; a built file of the
  radio's shape, which failed before; the cases that stay errors (the subsample flag, samples that
  need an IV, a `seig` group giving one its own IV, bytes left over, another count). The four fuzz
  targets ran 60 s each without a fault; indexing is no slower (0.32 ms), the table the same
  46.2 bytes a sample.
- Its tool calls: 36, 34 shell and 2 edits, none outside the export, no network, every `nix` call
  `--offline`.
- The comparison run again ([report](2026-11-05-cenc-clean-room/comparison.md)): the same figures
  against Kopuz and `fmp4.rs`, but three runs of 31 to 36 normalised tokens more, a test's
  `#[derive(…)] pub enum` against unrelated code of Kopuz's interface.
- The radio then played, on the owner's profile.

## Better than Kopuz

What the [licence audit](2026-11-05-licence.md#4-what-kopuz-does-badly-here-and-opium-with-it)
found in the format, each now an error or handled, each tested (`crates/cenc/src/tests/`):

- a box of size 0 runs to its container's end; a 64-bit size below its header is an error;
- nesting is bounded at 16; counts are checked against the bytes left before anything is reserved;
- a negative data offset is checked, not wrapped; reads are bounded by the box, not the buffer;
- an `senc` whose count or length does not match is an error; the IV size is read, never guessed;
- subsamples must add up to their sample;
- every track, every `trun`, `encv`'s own layout, an `mdat` anywhere, the `tfhd`'s base offset;
- `saiz`/`saio` read without `senc`; only `seig` groups made `free`, AAC's `roll` groups kept;
- `schm` read; `cenc`, `cbcs`, `cens` and `cbc1` named; `seig` groups read;
- a download cut inside a fragment is an error, where the old stream served it as clear.

Fuzzing: the mutation tests in `cargo test` (7 440 files, steps counted, allocations bounded), and
four `cargo fuzz` targets (the movie box, a whole file made clear, the indexer fed in prefixes, the
stream fed a download in pieces); they found three faults in the crate and two in `stream.rs`, all
fixed with tests.
