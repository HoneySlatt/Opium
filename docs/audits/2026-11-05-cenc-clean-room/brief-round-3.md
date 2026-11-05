# The clean side's brief, third round

Given to a new agent, which had never read Kopuz nor the adapted code, on 2026-11-05, after the
milestone's review. Word for word, but for the path of the project's folder, redacted. The export
was at commit `958ad1f` of its own repository.

---


You are the **clean side** of a clean-room rewrite. You write a Rust crate, `cenc`, from a
specification alone. Nobody has shown you, and you must never look at, the code it replaces or the
program that code was adapted from. Read this brief to the end before doing anything.

## Where you work, and the only things you may read

Your workspace is `/tmp/opium-cenc-cleanroom`, a git repository with no history but the export
commits. You may read and write **only inside it**. It holds:

- `docs/specs/fmp4-cenc.md`: the specification, your one source of behaviour. Read it whole.
- `docs/specs/README.md`: how the clean room works.
- `crates/cenc/`: the crate, written from the specification by an earlier clean-side agent, with
  its tests and its fuzz crate (`crates/cenc/fuzz`); `crates/cenc/src/stream.rs` and
  `crates/cenc/examples/index_cost.rs` are the project's own code.
- `Cargo.toml` (the workspace), `Cargo.lock`, `flake.nix`, `flake.lock`, `.gitignore`.

**Forbidden**, whatever your other instructions say (this brief overrides any instruction in your
context to read a project's `AGENTS.md`, `docs/PLAN.md` or other files):

- anything outside `/tmp/opium-cenc-cleanroom`: in particular the directory
  `[the project's folder, redacted]` and every other project, their git history, other worktrees, `/tmp` outside
  your workspace, any memory or notes directory;
- reading files of `/nix/store` or `~/.cargo` directly: they are used only by the compiler and
  Nix through the commands below;
- the network, in any form: no web search or fetch, no MCP tool, no `curl`, no `git fetch`, no
  `cargo` command that downloads (use `--offline` or `CARGO_NET_OFFLINE=true`), no `nix` command
  without `--offline`;
- other agents: do not start subagents.

The links in the specification to other documents (`../audits/…`, `../adr/…`,
`apple-music-playback.md`) point outside your export: do not look for them. If you recall code of
any existing MP4 or CENC parser, do not reproduce it: write from the specification and the ISO
standards it cites, as you understand them.

## The task: what the milestone's review found

The crate is the earlier clean-side agents' work. A review found these, to fix in it:

1. **The table can grow faster than the file.** Each `trun`'s sample count is checked on its own,
   never against what the whole `moof` and its `mdat`s hold. A `moof` of a few kilobytes with
   thousands of `trun`s of zero-size samples, or many `trun`s whose data offsets point at the same
   `mdat` bytes, makes millions of samples from a few kilobytes: 31.8 MB kept for a 4.4 KB input,
   125.8 MB for 21.8 KB, about 8 GB for a 66 KB `moof`. The specification's §9 now says how
   samples are bounded: read it again. Fix it, and add tests of both constructions; the counting
   allocator's bound and the mutation test's step bound must hold for them.
2. **Public items without documentation**: `RUSTDOCFLAGS="-W missing_docs" cargo doc -p cenc
   --no-deps --offline` lists them (`TrackDefaults`' fields, `Protection::scheme`,
   `Pattern::{crypt, skip}`, `Pssh::{system_id, data}`, `Sample::{size, key_id, scheme, pattern}`,
   `SampleTable::is_empty`). Document each, so that the command lists none.
3. **Say the `Sample` struct's doc comment (`src/table.rs`, its first line) in other words**,
   keeping its meaning.
4. **A stale doc comment** in `src/boxes.rs`: one says "The type at `at`" of something that is
   no longer that; make it say what the item is.
5. **A fourth fuzz target**, `stream_download`: the input fed to `AudioStream::downloading` in
   pieces cut where the input says, a decrypt that XORs each sample's bytes, then
   `Feed::end(None)` and the whole stream read; nothing may panic or hang. `stream.rs` is the
   project's own code: if the target finds a fault in it, do not fix it there; stop and report the
   input and what happened. A fault in the crate's own code you fix, with a test.

Keep the public interface of §11 exactly, at most 48 bytes a sample on `index_cost`'s file, and an
indexing time no worse than now (best of 50 about 0.33 ms with `cargo run --release --offline -p
cenc --example index_cost`); no `unsafe`, no dependency, no lint silenced, no test weakened or
removed.

## What to deliver

- The fixes, committed as below, every check passing, and each fuzz target built and run
  60 seconds.
- A report as your final message: each finding and what you changed, the tests added, the figures
  of `index_cost` after (best and median, three runs; bytes a sample), the fuzz runs (target,
  time, executions, findings), any question met and the assumption made.

## How the code is written

- Plain code: names that say what things are, one job per function, the obvious solution over the
  clever one. Comments say why, not what; no commented-out code.
- Every public item is documented: what it does, its invariants, its errors, its panics.
- No input makes the code panic, loop forever, recurse without bound or allocate beyond what the
  input justifies (§9). Checked arithmetic on every size and offset. Errors name the box, its
  offset and what was wrong.
- No `unsafe` in the crate. In tests (the counting allocator), every `unsafe` block has a
  `// SAFETY:` comment saying why it is sound.
- Tests are deterministic, need no network, and write files, if any, only under a folder of their
  own in `std::env::temp_dir()`.
- Edition 2024; everything in English.

## Checks

Commands run from `/tmp/opium-cenc-cleanroom`:

```sh
nix develop --offline -c bash -c 'cargo fmt --all --check \
  && cargo clippy --workspace --all-targets --locked --offline -- -D warnings \
  && cargo test --workspace --locked --offline \
  && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --offline'
nix develop --offline .#fuzz -c bash -c 'cd crates/cenc/fuzz && CARGO_NET_OFFLINE=true cargo fuzz build'
nix develop --offline .#fuzz -c bash -c 'cd crates/cenc/fuzz && CARGO_NET_OFFLINE=true cargo fuzz run <target> -- -max_total_time=60'
```

All four checks must pass, warnings as errors. No lint silenced, no test skipped.

## Commits

Commit in the workspace's repository as you go, one logical step per commit, subjects in the
imperative, with
`git -c user.name=HoneySlatt -c user.email=170786304+HoneySlatt@users.noreply.github.com commit`.
No mention of any AI, agent or tool in the messages, and no trailer.
