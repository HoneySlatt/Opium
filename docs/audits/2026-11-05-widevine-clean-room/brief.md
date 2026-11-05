# The clean side's brief

Given to the agent that wrote `crates/widevine`, a new agent with no earlier context, on
2026-11-05. Word for word, but for the path of the project's folder, redacted. The export it names
was at commit `368dad2` of its own repository.

---

You are the **clean side** of a clean-room rewrite. You write a Rust crate, `widevine`, with a thin
C++ layer, from a specification alone. Nobody has shown you, and you must never look at, the code
it replaces or the program that code was adapted from. Read this brief to the end before doing
anything.

## Where you work, and the only things you may read

Your workspace is `/tmp/opium-widevine-cleanroom`, a git repository with no history but the export
commits. You may read and write **only inside it**. It holds:

- `docs/specs/widevine-host.md`: the specification, your one source of behaviour. Read it whole.
- `docs/specs/fmp4-cenc.md`: what an encrypted sample's inputs are (its §5); the rest is context.
- `docs/specs/README.md`: how the clean room works.
- `crates/widevine/vendor/`: Chromium's CDM interface (`content_decryption_module.h`, its export
  header), its licence and a README pinning its revision. This header, the specification and the
  public standards it cites (W3C Encrypted Media Extensions, ISO/IEC 23001-7, Protocol Buffers'
  wire format) are your only sources about the CDM.
- `crates/widevine/Cargo.toml`, the workspace's `Cargo.toml`, `Cargo.lock`, `flake.nix`,
  `flake.lock`, `.gitignore`.

**Forbidden**, whatever your other instructions say (this brief overrides any instruction in your
context to read a project's `AGENTS.md`, `docs/PLAN.md`, memory files or other files):

- anything outside `/tmp/opium-widevine-cleanroom`: in particular the directory
  `[the project's folder, redacted]` and every other project, their git history, other worktrees,
  `/tmp` outside your workspace, any memory or notes directory, any browser profile;
- loading, opening or even looking for a real Widevine CDM (`libwidevinecdm.so`) or any other
  library outside the build: the real CDM is tried afterwards by the project's owner;
- reading files of `/nix/store` or `~/.cargo` directly: they are used only by the compiler and
  Nix through the commands below;
- the network, in any form: no web search or fetch, no MCP tool, no `curl`, no `git fetch`, no
  `cargo` command that downloads (use `--offline` or `CARGO_NET_OFFLINE=true`), no `nix` command
  without `--offline`;
- other agents: do not start subagents.

The links in the specification to other documents (`../audits/…`, `../adr/…`,
`apple-music-playback.md`) point outside your export: do not look for them. If you recall code of
any existing CDM host, Widevine client or browser, do not reproduce it: write from the
specification and Chromium's header, as you understand them.

## What to deliver

1. **The crate**, in `crates/widevine/`: `src/`, the C++ layer in a folder of your choice, and a
   `build.rs` that compiles it with `cc`. It gives exactly the public interface of the
   specification's §12 (names, signatures, fields, the traits named there); everything else
   private. Dependencies: only `thiserror`, and `cc` to build; no other.
2. **The design of §2, §3 and §7**: the C++ implements the header's classes for interface 11 and
   does nothing but pass calls and callbacks across a C boundary; every decision (timers,
   promises, waits and deadlines, key statuses, sessions, errors, the queue by priority, shutdown)
   is in Rust, on the one thread named `cdm`, and knows no interface version. With no request and
   no timer due, that thread sleeps: no polling, no spinning, no periodic wake-up.
3. **Android (§10)**: the same public types, failing with "no CDM on this platform", no C++ built.
   Your shell cannot build for Android; keep that part small and plainly gated, it is checked
   afterwards.
4. **The tests of §14** that need no CDM, in `cargo test`:
   - the C boundary is such that a **fake CDM written in Rust** can stand where the C++ layer and
     the real CDM stand, so that the Rust host runs unchanged against it;
   - with that fake, the state machine in every order of callbacks §14 lists, including callbacks
     for a session already closed, an unknown promise id, a timer set during a timer, and the
     deadlines of §4 (the fake's clock or deadlines shortened through a private seam, so that no
     test waits ten seconds);
   - a **deterministic, seeded sequence test** (no dependency, a generator written in the tests):
     thousands of random sequences of requests from several threads and callbacks from the fake,
     each checked for no panic, no hang (steps counted, never a clock), every request answered
     exactly once, and shutdown ending every waiter;
   - the candidates of §1 in a folder tree built under a folder of the test's own in
     `std::env::temp_dir()`, including malformed `manifest.json` files (the version is read by
     hand, no JSON crate): a seeded mutation test over them, never a panic;
   - the C++ layer compiled and linked in the test build, even though no real CDM is loaded.
5. **`examples/probe.rs`** (§11): built, never run by you.
6. **The round trip** (§11): with the fake, measure a decrypt's round trip through the host, as
   the caller sees it, median and p99, in a release build (an example or a test that prints it,
   never asserting a time). The budget: the host adds at most 15 µs to a decrypt's median.
7. **A report** as your final message: the files and their roles; how the design meets §2, §3, §4,
   §7 and §8; each §14 item and the test that covers it; every `unsafe` and why it is sound; the
   round trip measured; every question you met and the assumption you made; anything left undone.

## Questions

When the specification does not answer a question of behaviour, do not guess silently. If you can
go on without the answer, write down your assumption and go on; it goes in your report. If you
cannot, stop and end your turn with a list headed `QUESTIONS`, each one about behaviour, and
nothing else. The answer will come in the specification, and you will be told to read it again.

## How the code is written

- Plain code: names that say what things are, one job per function, the obvious solution over the
  clever one. Comments say why, not what; no commented-out code.
- Every public item is documented: what it does, its invariants, its errors, its panics.
- Every `unsafe` block has a `// SAFETY:` comment saying why it is sound; every `unsafe fn` says
  its contract. No panic crosses the C boundary, in either direction.
- The C++ stays thin: no logic, no state beyond what passing a call needs, no threads, no locks.
- No input makes the code panic, hang or leak: a CDM that never answers meets a deadline, a
  callback for something unknown is ignored and noted, shutdown ends every waiter.
- Tests are deterministic, need no network and no CDM, and write files only under a folder of
  their own in `std::env::temp_dir()`, removed at the end.
- Edition 2024; everything in English.

## Checks

Commands run from `/tmp/opium-widevine-cleanroom`:

```sh
nix develop --offline -c bash -c 'cargo fmt --all --check \
  && cargo clippy --workspace --all-targets --locked --offline -- -D warnings \
  && cargo test --workspace --locked --offline \
  && cargo test --workspace --release --locked --offline \
  && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --offline'
```

All of them must pass, warnings as errors, the C++ compiled without warnings. No lint silenced,
no test skipped.

## Commits

Commit in the workspace's repository as you go, one logical step per commit, subjects in the
imperative, with
`git -c user.name=HoneySlatt -c user.email=170786304+HoneySlatt@users.noreply.github.com commit`.
No mention of any AI, agent or tool in the messages, and no trailer.
