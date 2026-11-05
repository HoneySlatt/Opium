# 0030. The code adapted from Kopuz is written again in a clean room

- Status: Accepted; amends [0004](0004-the-widevine-cdm-borrowed-from-the-browser.md) (the shape of
  the CDM's host) and [0029](0029-opium-waits-for-slime-ui-then-moves-first.md) (the standby lifted
  for these milestones)
- Date: 2026-11-05

## Context

On 2026-11-05 the owner decided that Opium leaves GPL-3.0-or-later for "MIT OR Apache-2.0", at the
user's choice, as uwui and SlimeEngine did (their ADR-0181 and ADR-0182). The code written again
must also do better than Kopuz wherever it can.

The GPL was chosen because code adapted from Kopuz (EUPL-1.2) came in under the EUPL's
compatibility clause ([ADR-0001](0001-gpl-3-0-or-later.md)). The [licence audit of
2026-11-05](../audits/2026-11-05-licence.md) found that this code is the only thing that stops a
permissive licence. It sits in:

- `crates/widevine`: about 88 % of the C++ shim, 55 % of `native.rs` and 60 % of `build.rs`, the
  last two without a notice;
- `crates/cenc/src/fmp4.rs`: about half;
- `crates/apple-music`: 5 to 12 % of `playback.rs`, `lib.rs`, `auth.rs` and `convert.rs`.

The audit also lists what Kopuz does badly there, and Opium with it: a box of size 0 or a deep
nesting, a hang, a crash or a sample left encrypted; the IV size guessed; one track only; the CDM's
"no key" misreported; timers fired in a loop instead of in time; a failed start of the CDM taken for
a success.

Copyright protects code as written, not the requests, formats and protocols it carries out. Code
written again without Kopuz's structure, order or names can take any licence. This is the owner's
reading, not legal advice.

## Decision

**A clean-room rewrite.** Every part derived from Kopuz is written again by an agent that never
read Kopuz nor the adapted code, from specifications that describe behaviour only
([`docs/specs/`](../specs/README.md)):

- all of `crates/cenc/src/fmp4.rs` ([fmp4-cenc.md](../specs/fmp4-cenc.md));
- the whole of `crates/widevine` ([widevine-host.md](../specs/widevine-host.md));
- the identified items of `crates/apple-music`
  ([apple-music-playback.md](../specs/apple-music-playback.md)).

The method is in [the specifications' README](../specs/README.md):

- the clean side reads an allow-list, exported without history;
- the call sites are renamed beforehand;
- the questions and answers are written down;
- a token comparison of the new code with Kopuz and with the adapted files follows, read by hand
  above 50 raw or 30 normalised tokens;
- the clean side's brief, its tool calls and the comparison are kept as evidence.

The adapted files and their notices are deleted once their replacement passes.

**The CDM's host: thin C++, the logic in Rust, on one thread.** A C++ layer, specific to one
version of the CDM interface, implements the interface's classes and only passes calls and
callbacks to Rust. One Rust thread owns the CDM, as Chromium's host does. It serves requests from a
queue by priority (licences before decrypts), fires the CDM's timers in time, sleeps when nothing is
due, and destroys the instance at shutdown. This amends ADR-0004's shim, which held the logic in
C++.

**Parsers fuzzed two ways**:

- deterministic mutation tests inside `cargo test`, with a fixed seed and no new dependency, in CI
  at every push;
- `cargo fuzz` targets for the MP4 and CENC boxes, the HLS playlists and Apple's answers. They need
  a nightly toolchain from rust-overlay, already an input of the flake, in a shell of their own, and
  the `libfuzzer-sys` dependency, in a crate outside the workspace. They run by hand and weekly in
  CI.

This settles the ROADMAP's open decision on a nightly toolchain, for fuzzing. Miri is decided when
it is first needed.

**Now, in Stage 1.** The standby of ADR-0029 is lifted for these milestones only, one a session:

| Milestone | What it does |
|---|---|
| M1.9 | The inventory and the specifications |
| M1.10 | `cenc` (its in-place decrypting shapes the CDM's API) |
| M1.11 | `widevine`, its baseline measured again with the owner first |
| M1.12 | `apple-music` |
| M1.13 | The licence changed, by an ADR that amends ADR-0001 |

The stage audit M1.8 comes after, over the new code.

## Alternatives considered

- **Keeping the GPL**: nothing to do, but Opium apart from uwui and SlimeEngine, which the owner
  moved to MIT OR Apache-2.0, and code would not move freely between them.
- **A plain rewrite by the same side that read Kopuz, then the comparison**: cheaper, but its
  author knows the old structure and names, and nothing but the comparison would show the new code
  owes nothing to them.
- **Asking Kopuz's authors to relicense their code**: one message, but a public step on the owner's
  part, every contributor of those files to agree, and no improvement of the code.
- **The derived files under MPL-2.0**, which the EUPL-1.2's appendix lists as compatible, the rest
  under MIT OR Apache-2.0: nothing to write again, but part of Opium stays under a copyleft, the
  clause's reach is debated, and nothing gets better.
- **The host's logic in C++, as today**: fewer crossings between languages, but logic in a second
  language that cannot be tested without the real CDM.
- **No C++, the interface's tables of virtual functions built by hand in Rust**: no C++ compiler,
  but the layout of C++ objects, which differs under Windows (Stage 4), and far more `unsafe` code.
- **A lock per call, as today**: no thread of its own, but callbacks entering Rust under the lock
  invite deadlocks and aliasing, and no timer fires between calls.
- **Mutation tests alone, or `cargo fuzz` alone**: the first finds only cases near the ones written;
  the second runs too rarely to catch a regression at once.
- **After the standby, or as a stage of its own**: the standby kept whole, or a clearer roadmap, but
  the stage audit stale on these crates, or one audit more.

## Consequences

- Until M1.13, Opium stays under the GPL: the licence changes only once nothing derived remains, and
  the whole tree's comparison is recorded.
- Versions before M1.13 stay under GPL-3.0-or-later, the public history included.
- Behaviour verified today against Apple and SoundCloud must be verified again with the owner after
  the rewrite: fakes do not prove a service's behaviour (AGENTS.md).
- The CDM's figures are taken again with the owner before M1.11. The figure of 1.5 ms per sample is
  old.
- The rewrite may still carry what a model remembers of public code: the comparison is how that is
  caught.
- Fuzzing adds a nightly shell and a dependency outside the workspace. `libfuzzer-sys` is under
  permissive licences, MIT OR Apache-2.0, with LLVM's NCSA for libFuzzer itself; this is checked
  when it is added.
