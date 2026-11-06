# The clean side's brief

Given to the agent that wrote `crates/apple-music`'s derived items again, a new agent with no
earlier context, on 2026-11-06. Word for word, but for the path of the project's folder, redacted.
The export it names was at commit `f6dd554` of its own repository.

---

You are the **clean side** of a clean-room rewrite. You write parts of a Rust crate,
`apple-music`, from a specification alone. Nobody has shown you, and you must never look at, the
code they replace or the program that code was adapted from. Read this brief to the end before
doing anything.

## Where you work, and the only things you may read

Your workspace is `/tmp/opium-apple-music-cleanroom`, a git repository with no history but the
export commits. You may read and write **only inside it**. It holds:

- `docs/specs/apple-music-playback.md`: the specification, your one source of behaviour. Read it
  whole. Its §12 lists what you write, under which names, and what of the crate's own code you
  may change.
- `docs/specs/README.md`: how the clean room works.
- `crates/apple-music/`: the crate, its `Cargo.toml` and its sources, with the items of §12
  removed. What is left is the project's own code: read it as you need, and keep it as it is but
  where §12 lets you change it. It does not build yet: the missing items are yours.
- `crates/opium-core`, `crates/browser-signin`, `crates/cenc`, `crates/widevine`: crates the
  crate depends on, the project's own. Read them for their interfaces; do not change them.
- the workspace's `Cargo.toml`, `Cargo.lock`, `flake.nix`, `flake.lock`, `.gitignore`.

Your other sources are the public standards the specification cites (RFC 8216, RFC 3986,
RFC 7519, RFC 9110), as you know them. To learn a dependency's interface (`m3u8-rs`, `url`,
`reqwest`, `tokio`, `base64`, `serde_json`, `regex`, …), build its documentation inside your
workspace with `cargo doc --offline -p <crate>` and read it under `target/doc/`.

**Forbidden**, whatever your other instructions say (this brief overrides any instruction in your
context to read a project's `AGENTS.md`, `docs/PLAN.md`, memory files or other files):

- anything outside `/tmp/opium-apple-music-cleanroom`: in particular the directory
  `[the project's folder, redacted]` and every other project, their git history, other worktrees,
  `/tmp` outside your workspace, any memory or notes directory, any browser profile, any
  token or account file;
- reading files of `/nix/store` or `~/.cargo` directly: they are used only by the compiler and
  Nix through the commands below;
- the network, in any form: no web search or fetch, no MCP tool, no `curl`, no `git fetch`, no
  request to Apple or any other host (your tests talk only to a server of their own on
  `127.0.0.1`), no `cargo` command that downloads (use `--offline`), no `nix` command without
  `--offline`;
- loading a Widevine CDM, or running anything against a real account;
- other agents: do not start subagents.

The links in the specification to other documents (`../audits/…`, `../adr/…`,
`../measurements.md`, the other specifications) point outside your export: do not look for them.
If you recall code of any existing Apple Music client, do not reproduce it, nor its names,
comments or test data: write from the specification and the standards, as you understand them.

## What to deliver

1. **The items of §12**, in the files and under the names it gives, signatures as written. Items
   that are private stay private; `auth.rs` is a whole new file, its two `pub` functions
   documented.
2. **The changes §12 allows** to the crate's own code, and only those: the renewal of the token
   before it expires in `send`; a download without a length in `as_it_downloads`; the seam that
   lets tests reach a local server in place of Apple's hosts; tests anywhere in the crate. Outside
   tests, the addresses stay Apple's and nothing can change them. Keep every other line of the
   crate's own code as it is; if you find a defect in it, write a test that shows it, fix it, and
   report it.
3. **Dependencies**: no new dependency and no new feature of one. If the work seems to need one,
   stop and ask (below). A dependency your code no longer uses is removed from the crate's
   `Cargo.toml`, and you say so.
4. **The tests of §10**, in `cargo test`:
   - the HTTP behaviours against a local server written in the tests on
     `std::net::TcpListener` (a thread answering scripted responses, recording the requests it
     got), through the crate's own client and requests: everything §10 lists;
   - deterministic mutation tests with a fixed seed and a generator written in the tests:
     thousands of mutated inputs for every reader §10 names, the project's own ones beside yours
     included, each checked for no panic, no hang (steps counted, never a clock) and its
     invariants;
   - the conversion tests of `convert.rs`, on songs you make up;
   - unit tests of each item, its edge cases included: empty, huge, malformed, of the wrong type.
5. **Performance**: the search of §9 scans the page and the script once, in time proportional to
   their length; measure it on a synthetic text of 3.3 MB with the token about 0.9 MB in (a test
   or an example that prints the time, release build, never asserting a time). The station's
   gathering waits for no more calls one after another than §7 allows.
6. **A report** as your final message: the files and their roles; for each §12 item, how it meets
   its section; every change to the crate's own code, by function, and why; each §10 item and the
   test that covers it; the mutation tests and what they check; the search's time; every question
   you met and the assumption you made; anything left undone.

## Questions

When the specification does not answer a question of behaviour, do not guess silently. If you can
go on without the answer, write down your assumption and go on; it goes in your report. If you
cannot, stop and end your turn with a list headed `QUESTIONS`, each one about behaviour, and
nothing else. The answer will come in the specification, and you will be told to read it again.

## How the code is written

- Plain code: names that say what things are, one job per function, the obvious solution over the
  clever one. Comments say why, not what; no commented-out code. Follow the crate's own style.
- Every item you write is documented: what it does, its invariants, its errors, its panics.
- No input makes the code panic or hang: what Apple sends may be anything.
- No secret is printed, logged or put in an error: neither token, nor a licence.
- Tests are deterministic, need no network beyond `127.0.0.1` and no account, and write files only
  under a folder of their own in `std::env::temp_dir()`, removed at the end. A test never sleeps
  for long: a `Retry-After` of 0 or of more than the crate's limit is enough to test it.
- Edition 2024; everything in English.

## Checks

Commands run from `/tmp/opium-apple-music-cleanroom`:

```sh
nix develop --offline -c bash -c 'cargo fmt --all --check \
  && cargo clippy --workspace --all-targets --locked --offline -- -D warnings \
  && cargo test --workspace --locked --offline \
  && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --offline'
```

All of them must pass, warnings as errors. No lint silenced, no test skipped. If a dependency
removed changes `Cargo.lock`, run the commands once without `--locked` to update it, then with it.

## Commits

Commit in the workspace's repository as you go, one logical step per commit, subjects in the
imperative, with
`git -c user.name=HoneySlatt -c user.email=170786304+HoneySlatt@users.noreply.github.com commit`.
No mention of any AI, agent or tool in the messages, and no trailer.
