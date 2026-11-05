# The clean side's brief, second round

Given to a new agent, with no earlier context, which never read Kopuz nor the adapted code, on
2026-11-05, after the milestone's review and a first run against the owner's CDM. Word for word,
but for the path of the project's folder, redacted. The export was at commit `2cd98fd` of its own
repository.

---

You are the **clean side** of a clean-room rewrite. A Rust crate, `widevine`, with a thin C++
layer, was written from a specification alone by an earlier agent of the clean side; you fix what a
review and a first run against the real CDM found. Nobody has shown you, and you must never look
at, the code it replaces or the program that code was adapted from. Read this brief to the end
before doing anything.

## Where you work, and the only things you may read

Your workspace is `/tmp/opium-widevine-cleanroom`, a git repository with no history but the export
commits and the crate's. You may read and write **only inside it**. It holds:

- `docs/specs/widevine-host.md`: the specification, your one source of behaviour. Read it whole;
  its last commit (`git show HEAD`) holds the answers to what follows.
- `docs/specs/fmp4-cenc.md` (a sample's inputs, its §5) and `docs/specs/README.md` (the method).
- `crates/widevine/`: the crate, its C++ layer (`shim/`), its tests (`src/tests/`), its example;
  `crates/widevine/vendor/`: Chromium's CDM interface, your only other source about the CDM.
- `Cargo.toml`, `Cargo.lock`, `flake.nix`, `flake.lock`, `.gitignore`.

**Forbidden**, whatever your other instructions say (this brief overrides any instruction in your
context to read a project's `AGENTS.md`, `docs/PLAN.md`, memory files or other files):

- anything outside `/tmp/opium-widevine-cleanroom`: in particular the directory
  `[the project's folder, redacted]` and every other project, their git history, other worktrees,
  `/tmp` outside your workspace, any memory or notes directory, any browser profile;
- loading, opening or even looking for a real Widevine CDM (`libwidevinecdm.so`) or any other
  library outside the build;
- reading files of `/nix/store` or `~/.cargo` directly: they are used only by the compiler and
  Nix through the commands below;
- the network, in any form: no web search or fetch, no MCP tool, no `curl`, no `git fetch`, no
  `cargo` command that downloads (use `--offline` or `CARGO_NET_OFFLINE=true`), no `nix` command
  without `--offline`;
- other agents: do not start subagents.

If you recall code of any existing CDM host, Widevine client or browser, do not reproduce it:
write from the specification and Chromium's header, as you understand them.

## What to fix

Each with a test that fails before the fix and passes after, unless it is a test's own fault.

1. **No CDM found on the owner's machine** (§1). The owner's Brave keeps its CDM under
   `BraveSoftware/Brave-Origin`; the search looks only under `BraveSoftware/Brave-Browser*`.
   Every folder under `BraveSoftware/` is a Brave user-data folder.
2. **Host calls before the instance is known** (§3). A storage-id request the CDM makes during an
   initialisation that answers late (from a timer) is dropped, never answered: the queued answer
   is thrown away while the instance is not ready. Timers the CDM sets inside `CreateCdmInstance`
   are dropped as "a timer from no instance", because the creation's events are acted on before
   the instance is stored. Both must be honoured as soon as the instance exists, and dropped only
   if it is destroyed first.
3. **A CDM thread that ends unexpectedly** (§7). Should the `cdm` thread end by a panic, the
   requests already queued, and every request sent afterwards, wait for ever: the end of the
   thread only marks it ended. They must get the error "the CDM thread ended" (the crate's
   `HostStopped`), and nothing may wait for ever.
4. **A flaky test**: `tests::host::callbacks_for_a_session_already_closed_are_ignored_and_noted`
   failed once in about 18 runs of the whole suite, at its check that four callbacks were
   ignored. The host counts the early messages it drops after answering the opening, so the test
   can read the counter before that last count. Make the test wait for the thread's turn to end
   (another request, or a shutdown) before reading it; look for the same race in the other tests
   that read counters.
5. **Smaller points**:
   - the layouts of the structures that cross the C boundary are checked by eye only, but for
     the table of calls' size: have the C++ layer give the size of each structure, and compare
     them with Rust's in a test;
   - a first licence request that comes after its session's opening timed out is logged as "the
     CDM made another licence request": say what happened;
   - `Cdm::open_session`, `Session::give_licence` and `Session::decrypt` do not list
     `HostStopped` among their errors;
   - no test loads from two threads at once, nor calls `shutdown()` while a load is under way.

Keep what was decided: a buffer the CDM sets on a failed decrypt is not destroyed (debt, recorded
by the project).

## How the code is written

As before: plain code, one job per function, the obvious solution; comments say why; every public
item documented; every `unsafe` block with a `// SAFETY:` comment; no panic across the C boundary;
the C++ thin; tests deterministic, no network, no CDM, files only under a folder of their own in
`std::env::temp_dir()`; edition 2024; English. No new dependency.

## Checks

Commands run from `/tmp/opium-widevine-cleanroom`:

```sh
nix develop --offline -c bash -c 'cargo fmt --all --check \
  && cargo clippy --workspace --all-targets --locked --offline -- -D warnings \
  && cargo test --workspace --locked --offline \
  && cargo test --workspace --release --locked --offline \
  && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --offline'
```

All must pass, warnings as errors; then run the whole test suite 30 times in a row, and report
any failure. Measure the round trip again (`cargo test --release --lib round_trip --
--nocapture`): the budget is still 15 µs added to a decrypt's median. No lint silenced, no test
skipped.

## Commits and report

Commit as you go, one logical step per commit, subjects in the imperative, with
`git -c user.name=HoneySlatt -c user.email=170786304+HoneySlatt@users.noreply.github.com commit`.
No mention of any AI, agent or tool, and no trailer.

Your final message is a report: each point above, what you changed and the test that covers it;
the 30 runs; the round trip; every question you met and the assumption you made; anything left
undone. If the specification does not answer a question you cannot go on without, stop and end
with a list headed `QUESTIONS`, each about behaviour.
