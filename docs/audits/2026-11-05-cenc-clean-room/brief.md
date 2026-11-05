# The clean side's brief

Given to the agent that wrote `crates/cenc`, a new agent with no earlier context, on 2026-11-05.
Word for word, but for the path of the project's folder, redacted. The export it names was at
commit `6e97f71` of its own repository.

---

You are the **clean side** of a clean-room rewrite. You write a Rust crate, `cenc`, from a
specification alone. Nobody has shown you, and you must never look at, the code it replaces or the
program that code was adapted from. Read this brief to the end before doing anything.

## Where you work, and the only things you may read

Your workspace is `/tmp/opium-cenc-cleanroom`, a git repository with no history but the export
commits. You may read and write **only inside it**. It holds:

- `docs/specs/fmp4-cenc.md`: the specification, your one source of behaviour. Read it whole.
- `docs/specs/README.md`: how the clean room works.
- `crates/cenc/Cargo.toml`, `crates/cenc/src/lib.rs` (nearly empty), and
  `crates/cenc/src/stream.rs`: the project's own code, already using the public names of the
  specification's §11, with its tests. It is the main user of what you write.
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

## What to deliver

1. **The crate**, in `crates/cenc/src/`, giving exactly the public interface of the
   specification's §11 (names, signatures, fields, and the traits listed under the table);
   everything else private. Split it into modules as you see fit. No dependency.
2. **`stream.rs` passes its tests unchanged.** Change it only if the specification forces you to,
   and say why in your report.
3. **The tests of §13**, in `cargo test`:
   - each box layout and each malformed case §13 lists, on files built by the tests;
   - the deterministic mutation test: a seeded generator written in the tests (no dependency), at
     least several thousand mutated files, steps counted to catch a hang, never a clock;
   - an integration test of its own (`crates/cenc/tests/…`) with a counting global allocator: no
     input makes the parser allocate more than the input justifies;
   - **the budget of §10**, in that integration test or another: on a synthetic file of Apple's
     shape (9 000 samples in 100 fragments, about 800 bytes each, scheme `cenc`, 8-byte IVs, one
     subsample each), the heap kept by the table (what `Indexer::samples()` holds after the whole
     file, capacity included) is **at most 48 bytes a sample**, asserted;
   - making clear changes only the types and counts §6 names, byte for byte, and can be called
     again as §6 says.
4. **`cargo fuzz` targets**, in a crate `crates/cenc/fuzz` outside the workspace (its own
   `[workspace]` table in its `Cargo.toml`), on `libfuzzer-sys = "0.4.13"` and `cenc` by path:
   at least `Movie::read`, the whole-file `index` followed by `make_clear`, and the `Indexer` fed
   the same bytes in growing prefixes cut where the input says. Build and run each one briefly in
   the nightly shell (below), and commit its `Cargo.lock`.
5. **A report** as your final message: the files and their roles; how the design meets §9 and §10;
   each §13 item and the test that covers it; the bytes a sample measured; the fuzz runs (target,
   time, executions, findings); every question you met and the assumption you made; every change
   to `stream.rs`; anything left undone.

## Questions

When the specification does not answer a question of behaviour, do not guess silently. If you can
go on without the answer, write down your assumption and go on; it goes in your report. If you
cannot, stop and end your turn with a list headed `QUESTIONS`, each one about behaviour, and
nothing else. The answer will come in the specification, and you will be told to read it again.

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
