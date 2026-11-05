# 2026-11-05: `widevine` written again in a clean room (M1.11)

- **Scope**: `crates/widevine`, written again from [its specification](../specs/widevine-host.md)
  by [the method](../specs/README.md) of
  [ADR-0030](../adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md); the callers moved to its
  names; the token comparison afterwards.
- **Starting commit**: `fce9347`. The adapted files (`src/lib.rs`, `src/native.rs`, `src/stub.rs`,
  `shim/widevine_shim.cc`, `build.rs`, `examples/probe.rs`) and their notices are deleted.
- **Sides**:
  - dirty side: the session that read Kopuz and the adapted code; it measured the CDM before,
    pinned Chromium's headers, wrote the specification's figures and budget, the export, the
    callers' move and this file;
  - clean side, two new agents in turn: round 1 wrote the crate; round 2 fixed what the milestone's
    review and the first run against the owner's CDM found. Neither had read Kopuz or the adapted
    files, before or during.
- This is how a team usually shows that new code is its own; it is not legal advice.

## Before the hand-over

- **The CDM measured** with the owner's go, five runs on one song, by a program kept in the
  repository (`crates/apple-music/examples/cdm_cost.rs`, run by `scripts/measure/cdm-cost.sh`)
  ([figures](../measurements.md#the-cdm-m111-2026-11-05)). It showed that the CDM paces its
  decrypts (a few calls wait inside it up to 197 ms), which the specification did not say: §7 and
  §11 were corrected, and the owner set the budget (commit `fce9347`).
- **Chromium's headers pinned**: with the owner's go, the repository
  <https://chromium.googlesource.com/chromium/cdm> was cloned outside the tree; both headers are
  byte for byte its files at `8f62f3ba6f06483f3908f9cce4f8d931800c9d7c`, now recorded in
  `crates/widevine/vendor/README.md` (commit `e88a477`). Nothing in them changed.

## The export

A new git repository outside Opium's tree, with no history: one commit of the allowed files, then
`Cargo.lock` cut down to what the crate builds with (commit `368dad2`):

- `docs/specs/README.md`, `docs/specs/widevine-host.md`, `docs/specs/fmp4-cenc.md`;
- `crates/widevine/vendor/` (both headers, `LICENSE.chromium`, the README pinning them) and
  `crates/widevine/Cargo.toml`;
- the workspace's `Cargo.toml` with the root's `[workspace.package]`, `Cargo.lock`, `flake.nix`,
  `flake.lock` and `.gitignore`.

No `src/`, `shim/`, `build.rs` or `examples/`: nothing of the adapted code. A search of the export
for Kopuz's and the adapted files' names found only the specifications' README, which names Kopuz
as what the method replaces, and two uses of the English word "locate" in `fmp4-cenc.md`.

## The briefs

Word for word, but for the path of the project's folder:
[round 1](2026-11-05-widevine-clean-room/brief.md),
[round 2](2026-11-05-widevine-clean-room/brief-round-2.md). Each allows only the export, and forbids the rest of the disk, Opium's repository and its
history, any browser profile, loading or looking for a real CDM, reading `/nix/store` or
`~/.cargo` except through the compiler and Nix, the network, and other agents.

## Questions and answers

The clean side asked no question that stopped it. Round 2's points were answered in the
specification first (commit `588435a`): every folder under `BraveSoftware/` is searched; the CDM
may call its host from its creation on; a CDM thread that ends answers every caller; the
teardown's time, kept by the owner. Its assumptions where the specification is
silent, and the dirty side's choice on each:

- **Kept**:
  - an empty sample succeeds without reaching the CDM;
  - `WIDEVINE_CDM` set but empty counts as unset; a manifest with no readable version falls back
    to the folder's name, one above 64 KiB is not read; `4.10` equals `4.10.0`;
  - Chromium components are looked for on x86_64 and aarch64 only; a relative
    `XDG_CONFIG_HOME` is ignored, as the XDG specification asks;
  - each key-status callback replaces the session's list; a status the header does not define
    is not usable and not final; once a licence's promise settles, a rejection wins over any
    status;
  - a key held as expired fails its decrypts before the CDM is called; a session the CDM closes
    on its own fails its later calls with a new error, "the CDM closed the session";
  - up to 8 messages that come before a session's id are kept until it is named; sessions that
    end "not provisioned" or "no licence request" are closed; an empty or repeated session id is
    "init data refused";
  - `Allocate` and the null `CreateFileIO` answered in the C++ layer, as the specification's thin
    layer allows; the host lookup answering version 11 only;
  - the library opened on the caller's thread, so its static initialisers run there; every other
    call on the `cdm` thread; requests sent during initialisation wait in the queue;
  - dropping a `Session` does not wait for its close, which is logged;
  - a decrypt's caller waits for the CDM's call to return, even during shutdown, since the CDM
    reads the caller's memory;
  - the CDM's expiry time logged, not stored;
  - errors beyond §9: not initialised, the CDM thread stopped or not started, another CDM
    loaded, too large for the CDM, the session closed by the CDM;
  - the C++ always compiled at `-O2`, since Nix's toolchain sets `_FORTIFY_SOURCE`, which warns
    without optimisation.
  - round 2: a failed instance is treated as destroyed (its timers and storage answers dropped);
    a storage-id request made inside `CreateCdmInstance` is answered after `Initialize` is called;
    a licence request is "late" when no opening waits for its session any more; the sizes of the
    structures crossing the C boundary are compared, their offsets checked by the C++ layer's
    `static_assert`s against Chromium's; the host's log lines are kept for tests in test builds
    only.
- **Kept, but debt** ([debt.md](../debt.md)): after a failed decrypt, a buffer the CDM set anyway
  is not destroyed, a leak the clean side chose over a possible double free; the header says only
  that the block "should be ignored".

## What the dirty side changed outside the export

The callers, moved to §12's names (commit `5162b64`):

- `apple-music`: a playlist's key id is now `[u8; 16]`, so that a key id of another length is no
  Widevine key at all; the catalog's and Spatial Audio's decrypt closures decrypt in place, the
  copy back from a new buffer gone; each licence waits for the key its decrypts use; the live
  radio decrypts each sample in place in its segment, a playlist IV not of 16 bytes an error.
- `soundcloud`: the stream is indexed before the licence is given, so that the licence waits for
  the key its first protected sample names.
- `desktop`, the examples `am`, `sc`, `spike` and `cdm_cost`: `find_cdm` and `Cdm::load`.

After the review (commits `edca263`, `4765e2e`, and the final review's tests): the live radio's
rotated key is licensed for the key id its PSSH names, and the playlist's IV is read only for a
sample without its own, both now small functions with tests, as is a key id of another length; the
examples call `widevine::shutdown()` before they exit.

Each source keeps the scheme it used before: `cenc` for Apple's catalog songs and SoundCloud,
`cbcs` with the file's pattern for Spatial Audio and the live radio.

## The tool calls

Read by the dirty side from the transcripts, with a script that lists every call and every absolute
path:

| Round | Calls | Reads outside the export | Network | Notes |
|---|---|---|---|---|
| 1 | 114: 69 shell, 32 writes, 9 reads, 4 edits | none | none | One `pkill` ended the agent's own test binary, which hung before the shutdown fault it then fixed. The only path outside the export named in a call is `/opt/google/chrome`, written into the code as §1 asks, never read. |
| 2 | 65: 55 shell, 5 reads, 5 edits | one, see below | none | Temporary worktrees for the "before" figures stayed inside the export, removed afterwards. |

Every `nix` call was `--offline`, every `cargo` call offline. Round 2 read once a file the agent
tool itself had written outside the export: the output of the agent's own command printing the
crate's files, set aside by the tool because it was long. It held only the export's files. No
call named Kopuz, the adapted files, Opium's folder, a browser profile, a CDM library,
`/nix/store` or `~/.cargo`. No subagent was started.

## The token comparison

[`compare.sh`](2026-11-05-widevine-clean-room/compare.sh) runs M1.10's
[`token_compare.py`](2026-11-05-cenc-clean-room/token_compare.py) over every source file of
`crates/widevine` but Chromium's headers (the Rust, the C++ and its header, the tests and the
example), against Kopuz at `a58c385` and `1d8b323`, the adapted files at `fce9347`, and the rest
of the tree. Runs made only of tokens of Chromium's headers are left out. The full report is
[`comparison.md`](2026-11-05-widevine-clean-room/comparison.md).

Calibration: the same script over the adapted files before the rewrite finds 1 709 raw tokens in
a row shared with Kopuz's shim, and 179 with its `mod.rs`.

| Against | Raw: runs of 50 or more | Raw: longest below | Normalised: runs of 30 or more | Normalised: longest |
|---|---|---|---|---|
| Kopuz `a58c385` | 0 | 42 | 105 | 119 |
| Kopuz `1d8b323` | 0 | 42 | 105 | 119 |
| The adapted files at `fce9347` | 0 | 46 | 9 | 101 |
| The rest of the tree | 0 | 30 | 59 | 54 |

Read by hand:

- **The longest raw runs**, below the threshold: Widevine's system id as a 16-byte constant (42
  and 45 tokens; its name `SYSTEM_ID` is fixed by §12, its bytes are a fact), and the `pssh` box
  written field after field (46 tokens), in the order ISO/IEC 23001-7 §8.1 and §5 of the
  specification give it.
- **Against Kopuz**: 8 of the 105 normalised runs fall in its Widevine code, the same at both
  revisions: a test's end and the next test's head (`assert!(…); } #[test] fn …() { let mut`, 30); the `pssh` box and the system id above, with `#[cfg(test)] mod tests { use super::*;
  #[test] fn` after them (the run of 119), and the one-line getters and setters of the header's
  `cdm::Buffer` and `cdm::DecryptedBlock` (`Data`, `SetSize`, `Size`, `DecryptedBuffer`,
  `SetTimestamp`, `Timestamp`), whose names and signatures the header imposes and whose bodies
  return or set one field. The other 97 fall in code that does nothing of the kind (its
  interface, store, database, player, other services and a vendored crate): struct literals of
  `Vec::new()` fields, `use` lists, `iter().filter(…).count()`, `iter().find(…)`.
- **Against the adapted files**: the same `pssh` box, system id and getters; a directory listing
  (`.map(|entry| entry.path()).filter(|path| path.is_dir()).collect()`, then a sort); two `use`
  lists.
- **Against the tree**: Opium's own idioms; none above 54 normalised tokens.
- **Names**: those the new code shares with Kopuz's and the adapted files' are the header's (every
  `cdm::` class, method and field), the standard library's, §12's, and the method names
  `keys_changed` and `session_closed` in `host.rs`, the header's `OnSessionKeysChange` and
  `OnSessionClosed` in snake case, where Kopuz has two `bool` fields. `GetHost`, the host lookup
  `CreateCdmInstance` takes (the header's `GetCdmHostFunc`), has the same name in the old shim.
  `NotACdm` says §9's "not a CDM". `opt_level` is `cc`'s, used for the same reason, found again:
  `_FORTIFY_SOURCE`. The C++ library's name `widevine_shim` and its folder `shim/` are those of
  the old build, which §13 names ("no `shim/`").
- **Strings**: none in common.
- **Comments**: one phrase in common with Kopuz, "the CDM has no device certificate", which is
  §3's own words; one with the old example, the command `cargo run -p widevine --example probe`.
- **Test data**: the manifest of a Chromium component in `manifest.rs`'s test (`"name":
  "WidevineCdm"`, a `platforms` entry for `linux`, `x64`, `_platform_specific/linux_x64/`) is the
  same as in a Kopuz test: it is the format Google's component ships, as the owner's own
  `manifest.json` shows, and a raw string, one token, which the comparison cannot see. A test of
  `find.rs` uses the version `4.10.2891.0`, as Kopuz's tests do: a version Google released, not in
  the export, which the model knew. No other.

Nothing above the thresholds is left unexplained.

## The milestone's review

A reviewer that wrote neither side read the milestone against `AGENTS.md`, ADR-0030, the
specification and this evidence, and compared the new code with Kopuz and the adapted files itself,
down to 20 raw and 22 normalised tokens. It found no unsound `unsafe` and nothing owed to Kopuz,
and:

- the evidence ahead of the facts (the debt, the figures after, the PLAN): written after the runs;
- a storage-id request made while the CDM initialises, and timers set inside its creation,
  dropped; a CDM thread that ends by a panic leaving its callers waiting; a flaky test: round 2,
  each with a test that failed before;
- the live radio's rotated key licensed for the old key id; a playlist IV of another length
  failing every segment; the examples exiting without `shutdown()`: fixed by the dirty side;
- the structures' layouts checked by eye, a misleading log line, errors left out of the
  documentation, no test of loads from two threads or of a shutdown during a load: round 2;
- the manifest test data, the library's name, and a p99 worded as added time: said above.

A second reviewer, new, read the milestone again after round 2 and the documents. It found the
crate sound and nothing owed to Kopuz, re-ran the comparison to the same figures, and found: a
decrypt's p95 grown beyond the budget, which the documents said was met (put to the owner, who
accepted it); the radio's fixes untested (tests added); the screenshot mode's failure recorded only
as a pitfall (now in the debt); three stale places in the specification (`RTLD_LOCAL` kept, the
teardown's range, host calls before `OnInitialized`); and smaller points, fixed in the documents or
left as debt.

## With the owner's CDM

With the owner's go, Widevine 4.10.3112.0 from the owner's Brave:

- **The first run found no CDM**: §1 named `BraveSoftware/Brave-Browser*` only, where the owner's
  Brave keeps it under `BraveSoftware/Brave-Origin`. The old code looked under `BraveSoftware/*`.
  The specification was wrong, the dirty side's; round 2 searches every Brave. Found since.
- **The probe** (`examples/probe.rs`): loaded `RTLD_NOW | RTLD_LOCAL` and initialised in 3 to 13
  ms; a licence request of 1 708 bytes for a made-up key id in 83 to 151 ms; no timer and no
  storage request meanwhile; the output protection's status asked, not answered, as before. A
  library that is not a CDM (zlib) is refused: "not a CDM, the library does not export
  InitializeCdmModule_4".
- **The teardown right after loading takes 0.44 to 0.85 s**: `DeinitializeCdmModule` waits for a
  thread of the CDM's own. After a while at rest it takes 0.3 ms. Kept whole, the owner's choice.
- **The figures after**, alternately with the old code, and the same cleartext from both, digest
  for digest: [measurements](../measurements.md#the-cdm-m111-2026-11-05).
- **Opium on a throwaway profile**, release build, in a headless Weston, no network, a session bus
  of its own: it starts, loads the CDM, and, `Quit` sent over MPRIS 15 s later, ends in 61 ms
  with the CDM shut down (its teardown is short once the CDM has rested); the old build the same
  way in 60 ms. It plays nothing: sound and a signed-in account need the owner. The screenshot mode, which ends a run by itself, saved no
  capture in the headless Weston with either build, and on the desktop's compositor once in nine
  runs, by the old build (0 of 5 for the new, 1 of 4 for the old): it waits for frames the
  compositor did not send ([pitfall](../../CONTRIBUTING.md#known-pitfalls)).

## Checked

- `scripts/check.sh` passes: the crate's 78 tests and its doc test, the callers' tests, lints,
  documentation and links; round 2 ran the crate's suite 30 times in a row without a failure.
- For Android, `cargo check --target aarch64-linux-android -p widevine` with warnings as errors,
  through a toolchain of the store with Android's standard library (the `.#android` shell does not
  build while Google's repository answers 404): it builds, and no C++ is compiled. The callers
  were not built for Android: their C dependencies need the NDK.
- The round trip through the host with the fake CDM, release build: 6.5 to 7.1 µs added to a
  decrypt's median, 9.4 to 10.2 µs at p99 through the host; the budget is 15 µs added.

## Better than Kopuz

What the [licence audit](2026-11-05-licence.md#4-what-kopuz-does-badly-here-and-opium-with-it)
found in the CDM's host, each now handled and tested (`crates/widevine/src/tests/`):

- the CDM's statuses mapped one by one: "no key for this content" for `kNoKey`;
- a failed initialisation destroys the instance and is reported at every attempt;
- timers fire in time, never before, from a queue; every wait has a deadline and names its step;
  at rest the thread sleeps;
- promise ids are fresh and matched; key statuses are kept, and a licence is ready once the
  needed key is usable; the CDM's rejection message and system code reach the error;
  `RequestStorageId` is answered;
- the library is loaded `RTLD_LOCAL`, the instance destroyed at shutdown, the module ended;
- a session's close is waited for in the CDM thread and logged;
- licences, sessions and closes go before decrypts, in a queue, without a lock across calls;
- every `unsafe` has its `// SAFETY:`; no panic crosses the C boundary;
- CDM versions are compared as numbers; Flatpak's folders, Edge, Opera and Vivaldi's snapshot are
  known; the CDM is never downloaded.

Not checked: Apple's and SoundCloud's protected songs played end to end, Spatial Audio and the live
radio; they are played with the owner after M1.12.
