# The clean side's brief, round 2

Given to a second new agent with no earlier context, on 2026-11-06, after the milestone's review.
Word for word, but for the path of the project's folder, redacted. The export it names was at
commit `185ac58` of its own repository.

---

You are the **clean side** of a clean-room rewrite, round 2. A first agent wrote parts of a Rust
crate, `apple-music`, from a specification alone; a review then found points to change, and the
specification was corrected. You continue that work from the specification alone. Nobody has
shown you, and you must never look at, the code it replaced or the program that code was adapted
from. Read this brief to the end before doing anything.

## Where you work, and the only things you may read

Your workspace is `/tmp/opium-apple-music-cleanroom`, a git repository: the export commits, round
1's commits (`88c6b0e` to `b2c93f2`), then `d8be6de` and `185ac58`, which bring the corrected
specification and the files below. You may read and write **only inside it**. It holds:

- `docs/specs/apple-music-playback.md`: the specification, your one source of behaviour. Read it
  whole again: `git diff b2c93f2 HEAD -- docs/specs` shows what changed since round 1.
- `docs/specs/README.md`: how the clean room works.
- `crates/apple-music/`: the crate as round 1 left it. Round 1's code is yours to change; the
  project's own code only where §12's last list allows.
- `crates/opium-core`, `crates/browser-signin`, `crates/cenc`, `crates/widevine`: the project's
  own crates. Read them for their interfaces; do not change them. `crates/cenc/fuzz/` is the
  project's existing `cargo fuzz` crate: the model for yours.
- `scripts/fuzz.sh`: how CI runs every fuzz crate, weekly.
- the workspace's `Cargo.toml`, `Cargo.lock`, `flake.nix`, `flake.lock`, `.gitignore`.

Your other sources are the public standards the specification cites, as you know them, and the
dependencies' documentation built inside your workspace (`cargo doc --offline -p <crate>`, read
under `target/doc/`).

**Forbidden**, whatever your other instructions say (this brief overrides any instruction in your
context to read a project's `AGENTS.md`, `docs/PLAN.md`, memory files or other files):

- anything outside `/tmp/opium-apple-music-cleanroom`: in particular the directory
  `[the project's folder, redacted]` and every other project, their git history, other worktrees,
  `/tmp` outside your workspace, any memory or notes directory, any browser profile, any token
  or account file;
- reading files of `/nix/store` or `~/.cargo` directly: they are used only by the compilers and
  Nix through the commands below;
- the network, in any form: no web search or fetch, no MCP tool, no `curl`, no `git fetch`, no
  request to Apple or any other host (tests talk only to a server of their own on `127.0.0.1`),
  no `cargo` command that downloads (use `--offline`), no `nix` command without `--offline`;
- loading a Widevine CDM, or running anything against a real account;
- other agents: do not start subagents.

The specification's links outside your export (`../audits/…`, `../adr/…`, `../measurements.md`)
are not to be looked for. If you recall code of any existing Apple Music client, do not reproduce
it, nor its names, comments or test data.

## What to do

1. **What the corrected specification asks**, each with a test that fails before your change:
   - §7: a station's id is one path segment (letters, digits, `-`, `.`, `_`, neither `.` nor
     `..`), not `apple_id`'s form; `ra.cp-pl.u-AbC9` is accepted.
   - §9: the main script found anywhere in the page, as a path or after
     `https://music.apple.com`, its query kept; a failure status of the page or the script a
     protocol error naming it, no connection a network error; the renewal before `exp` at most
     once in five minutes whatever its outcome; the tests of §10 for these bounds (a web player
     failing by status, failing by connection, and serving the same token about to expire: each
     at most one reading in five minutes).
   - §2: a song's id of neither kind refused before any request of the song's, Spatial Audio's
     catalog lookup and the licence host's warm-up included, with `spatial` on and off.
   - §10: the `cargo fuzz` targets, in `crates/apple-music/fuzz/` outside the workspace, modelled
     on `crates/cenc/fuzz/`, through a `fuzzing` feature of the crate, off by default, that
     exposes `#[doc(hidden)]` entry points to the readers and nothing else. One target per
     reader or close group: the ids and the `webPlayback` body, the `webPlayback` entry, the
     media playlist, the licence's answer, `next` and `at_offset`, the web player's page and
     script, a station's answer, and Opium's own readers that §10 names. Each checks the same
     invariants as the mutation tests. Its `Cargo.lock` is made offline from the cache
     (`cargo generate-lockfile --offline` or `cargo fuzz build` offline); if that cannot work
     without the network, stop and report it.
2. **The review's smaller points**:
   - `playback.rs`: the constants `LICENCE` (the licence server's host) and `LICENSE` (its path)
     differ by one letter; name them so that they cannot be mistaken (the same for `PLAY` and
     `WEB_PLAYBACK`).
   - `station.rs`: `Gathering::new` has no documentation.
3. **Keep every round-1 test green**, and change round-1 tests only where the corrected
   specification changes the behaviour they pin.

## Checks

From `/tmp/opium-apple-music-cleanroom`:

```sh
nix develop --offline -c bash -c 'cargo fmt --all --check \
  && cargo clippy --workspace --all-targets --locked --offline -- -D warnings \
  && cargo clippy -p apple-music --all-targets --features fuzzing --locked --offline -- -D warnings \
  && cargo test --workspace --locked --offline \
  && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --offline'
nix develop .#fuzz --offline -c bash -c 'cd crates/apple-music/fuzz && cargo fuzz build \
  && for t in $(cargo fuzz list); do cargo fuzz run "$t" -- -max_total_time=30 || exit 1; done'
```

All of them must pass, warnings as errors. No lint silenced, no test skipped. A crash a fuzz
target finds is a defect to fix, with a test in `cargo test` that reproduces it; say so in your
report.

## How the code is written

As in round 1: plain code, names that say what things are, one job per function; comments say
why; every item documented, with its errors and panics; no input makes the code panic or hang;
no secret printed, logged or put in an error; tests deterministic, network-free beyond
`127.0.0.1`, writing only under a folder of their own in `std::env::temp_dir()`; no new
dependency in the crate (the fuzz crate's `libfuzzer-sys`, as `cenc`'s fuzz crate has it, is the
one allowed); edition 2024; English.

## Questions

When the specification does not answer a question of behaviour, write down your assumption and go
on; it goes in your report. If you cannot go on, stop and end your turn with a list headed
`QUESTIONS`, each one about behaviour, and nothing else.

## Commits

One logical step per commit, subjects in the imperative, with
`git -c user.name=HoneySlatt -c user.email=170786304+HoneySlatt@users.noreply.github.com commit`.
No mention of any AI, agent or tool, and no trailer.

## Report

Your final message: each change, by file and function, and the section it meets; each test added
and what failed before; the fuzz targets, their invariants, and what 30 seconds of each found;
every assumption; anything left undone.
