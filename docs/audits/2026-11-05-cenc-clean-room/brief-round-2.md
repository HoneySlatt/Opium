# The clean side's brief, second round

Given to a new agent, with no earlier context, which never read Kopuz nor the adapted code, on
2026-11-05, after the first round's code was found slower than the code it replaces. Word for
word, but for the path of the project's folder, redacted. The export was at commit `7701052` of
its own repository.

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

## The task: index as fast as before

The crate passes its checks and keeps 46.2 bytes a sample, within the budget of §10. But it indexes
more slowly than the code it replaces, and §10 says neither figure may get worse:

- `cargo run --release --offline -p cenc --example index_cost` (inside `nix develop --offline`)
  reports the best and median of 50 runs of `cenc::index` on a file of Apple's shape;
- the code before took **0.445 to 0.457 ms** at best on this machine; the crate now takes **0.52 to
  0.54 ms**, measured alternately with it under the same load.

Make the crate's best time **0.45 ms or less** with that program, keeping:

- the public interface of §11 exactly, and every behaviour of the specification;
- at most 48 bytes a sample in the table, the counting allocator's bounds, and every test;
- no `unsafe`, no dependency, no lint silenced, no test weakened or removed.

No profiler is available here (the kernel refuses `perf`). Time what you change with the program
above or with throwaway code under `target/`, which you delete. Look first at what runs once per
sample. Do not change `index_cost.rs`. If the target cannot be reached without breaking one of the
conditions above, stop and say what it would cost.

## What to deliver

- The faster crate, committed as below, every check passing, and each fuzz target built and run
  60 seconds again.
- A report as your final message: what was slow and why, each change and what it gained, the
  figures before and after with the program above (best and median, three runs), the bytes a
  sample, the fuzz runs, any question met and the assumption made.

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
