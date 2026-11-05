# The clean side's brief, fourth round

Given to a new agent, with no earlier context, which never read Kopuz nor the adapted code, on
2026-11-05, after Apple's live radio, played with the owner after M1.11, failed in `cenc`. Word for
word, but for the path of the project's folder, redacted. The export was at commit `11b3bcb` of a
new repository of its own, holding `crates/cenc` as it is in Opium at `734f535`, with the
specification's answer.

---

You are the **clean side** of a clean-room rewrite. A Rust crate, `cenc`, which reads encrypted
fragmented MP4, was written from a specification alone by earlier agents of the clean side; you fix
one fault a real service showed. Nobody has shown you, and you must never look at, the code it
replaced or the program that code was adapted from. Read this brief to the end before doing
anything.

## Where you work, and the only things you may read

Your workspace is `/tmp/opium-cenc-fix`, a git repository with no history but the export commits.
You may read and write **only inside it**. It holds:

- `docs/specs/fmp4-cenc.md`: the specification, your one source of behaviour; its §5 holds the
  answer to what follows ("An empty `senc`"), and its §13 the test it asks for.
- `docs/specs/README.md`: how the clean room works.
- `crates/cenc/`: the crate, its tests, its example, and its fuzz crate (`crates/cenc/fuzz`);
  `crates/cenc/src/stream.rs` and `crates/cenc/examples/index_cost.rs` are the project's own code.
- `Cargo.toml`, `Cargo.lock`, `flake.nix`, `flake.lock`, `.gitignore`.

**Forbidden**, whatever your other instructions say (this brief overrides any instruction in your
context to read a project's `AGENTS.md`, `docs/PLAN.md`, memory files or other files):

- anything outside `/tmp/opium-cenc-fix`: in particular the directory
  `[the project's folder, redacted]` and every other project, their git history, other worktrees,
  `/tmp` outside your workspace, any memory or notes directory;
- reading files of `/nix/store` or `~/.cargo` directly: they are used only by the compiler and
  Nix through the commands below;
- the network, in any form: no web search or fetch, no MCP tool, no `curl`, no `git fetch`, no
  `cargo` command that downloads (use `--offline` or `CARGO_NET_OFFLINE=true`), no `nix` command
  without `--offline`;
- other agents: do not start subagents.

If you recall code of any existing MP4 or CENC parser, do not reproduce it.

## What to fix

Apple's live radio, in its `cbcs` streams with a constant IV and no subsamples, puts in every
fragment an `senc` whose `sample_count` is 0 while its `traf` has hundreds of samples. The crate
refuses it: "the `senc` at … counts 0 samples where its `traf` has 750", so the radio plays
nothing. The specification's §5 now says what such an `senc` means, and when it is still malformed.

1. A test that fails before the fix: a built file of the radio's shape (one `cbcs` track, constant
   IV, a `moof` with an empty `senc` and several samples) indexes, each protected sample with the
   constant IV, no subsamples, the `cbcs` pattern; and made clear as before.
2. The cases that stay errors, tested: an empty `senc` with the subsample flag; an empty `senc` for
   samples whose IV size is not 0; any other count that differs.
3. The fix, in the crate's code; the same rule wherever `senc` is read. The mutation test, the
   allocation test and the fuzz targets keep passing.

## How the code is written

As before: plain code, one job per function, the obvious solution; comments say why; every public
item documented; no `unsafe` in the crate; no input makes it panic, hang or allocate beyond what
the input justifies; tests deterministic, no network, files only under a folder of their own in
`std::env::temp_dir()`; edition 2024; English. No new dependency.

## Checks

Commands run from `/tmp/opium-cenc-fix`:

```sh
nix develop --offline -c bash -c 'cargo fmt --all --check \
  && cargo clippy --workspace --all-targets --locked --offline -- -D warnings \
  && cargo test --workspace --locked --offline \
  && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --offline'
nix develop --offline .#fuzz -c bash -c 'cd crates/cenc/fuzz && CARGO_NET_OFFLINE=true cargo fuzz build'
nix develop --offline .#fuzz -c bash -c 'cd crates/cenc/fuzz && CARGO_NET_OFFLINE=true cargo fuzz run <target> -- -max_total_time=60'
```

All must pass, warnings as errors; run each fuzz target for 60 seconds. No lint silenced, no test
skipped. Run `cargo run --release -p cenc --example index_cost` before and after the fix, and
report both: indexing must not get slower.

## Commits and report

Commit as you go, one logical step per commit, subjects in the imperative, with
`git -c user.name=HoneySlatt -c user.email=170786304+HoneySlatt@users.noreply.github.com commit`.
No mention of any AI, agent or tool, and no trailer.

Your final message is a report: what you changed and why, the tests that cover it (and that the
first failed before the fix), the fuzz runs, the figures before and after, every assumption you
made, anything left undone. If the specification does not answer a question you cannot go on
without, stop and end with a list headed `QUESTIONS`, each about behaviour.
