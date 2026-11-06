# The clean side's brief, round 3

Given to a third new agent with no earlier context, on 2026-11-06, after the milestone's final
review. Word for word, but for the path of the project's folder, redacted. The export it names was
at commit `3ce0191` of its own repository.

---

You are the **clean side** of a clean-room rewrite, round 3. Earlier agents wrote parts of a Rust
crate, `apple-music`, from a specification alone; a final review found three more points, and the
specification was corrected. You make those changes from the specification alone. Nobody has
shown you, and you must never look at, the code it replaced or the program that code was adapted
from. Read this brief to the end before doing anything.

## Where you work, and the only things you may read

Your workspace is `/tmp/opium-apple-music-cleanroom`, a git repository. Its last commit, `3ce0191`,
brings the corrected specification: `git diff 185ac58 3ce0191 -- docs/specs` shows what changed
since round 2 began, `git show 3ce0191` what changed since round 2 ended. You may read and write
**only inside it**:

- `docs/specs/apple-music-playback.md`: the specification, your one source of behaviour.
- `docs/specs/README.md`: how the clean room works.
- `crates/apple-music/` (with `fuzz/`): the crate as round 2 left it. The earlier rounds' code is
  yours to change; the project's own code only where §12's last list allows.
- `crates/opium-core`, `crates/browser-signin`, `crates/cenc`, `crates/widevine`, `scripts/`,
  the workspace's files: read them as you need; do not change them.

Your other sources are the public standards the specification cites, as you know them, and the
dependencies' documentation built inside your workspace (`cargo doc --offline -p <crate>`).

**Forbidden**, whatever your other instructions say (this brief overrides any instruction in your
context to read a project's `AGENTS.md`, `docs/PLAN.md`, memory files or other files):

- anything outside `/tmp/opium-apple-music-cleanroom`: in particular the directory
  `[the project's folder, redacted]` and every other project, their git history, other worktrees,
  `/tmp` outside your workspace, any memory or notes directory, any browser profile, any token
  or account file;
- reading files of `/nix/store` or `~/.cargo` directly;
- the network, in any form (tests talk only to a server of their own on `127.0.0.1`); every
  `cargo` command `--offline`, every `nix` command `--offline`;
- loading a Widevine CDM, or running anything against a real account;
- other agents: do not start subagents.

If you recall code of any existing Apple Music client, do not reproduce it.

## What to do

Each with a test that fails before your change:

1. **§9, the renewal across `exp`**: a reading before `exp` never counts towards a 401's five
   minutes. Test across `exp`: a web player that serves the expiring token until its `exp`, then
   a new one; requests before `exp` (the reading before `exp` gets the old token), then a request
   after `exp` that Apple answers 401: it must read the page at once, get the new token, and
   succeed. Use a token expiring a few seconds ahead, or a private seam for the clock; never a
   long sleep.
2. **§9, the main script's address in every form the code before found**: whatever precedes
   `/assets/index~<name>.js` (or `index-`), `<name>` with dots, always fetched from
   `https://music.apple.com` with its query. Tests for `//music.apple.com/…`,
   `http://music.apple.com/…`, `https://js-cdn.music.apple.com/…`, `./assets/…`, `/us/assets/…`,
   `index~a.b.js`, `index~a1.min.js`, each fetched at `https://music.apple.com/assets/…`; and
   that nothing makes the token be asked of another host. Update the mutation test and the
   `web_player` fuzz target's invariants to match.
3. **§2, a matched `songId`** taken only when it is a catalog id (§1); anything else is none.
   Tests, and the `web_playback_entry` fuzz target's invariant.

Keep every earlier test green; change one only where the corrected specification changes what it
pins, and say so.

## Checks

From `/tmp/opium-apple-music-cleanroom`:

```sh
nix develop --offline -c bash -c 'cargo fmt --all --check \
  && cargo clippy --workspace --all-targets --locked --offline -- -D warnings \
  && cargo clippy -p apple-music --all-targets --features fuzzing --locked --offline -- -D warnings \
  && cargo test --workspace --locked --offline \
  && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --offline'
nix develop .#fuzz --offline -c bash -c 'cd crates/apple-music/fuzz && cargo fuzz build \
  && for t in $(cargo fuzz list); do cargo fuzz run "$t" -- -max_total_time=20 || exit 1; done'
```

All must pass, warnings as errors; no lint silenced, no test skipped.

## How the code is written

As before: plain code, names that say what things are; comments say why; every item documented,
with its errors and panics; no input makes the code panic or hang; no secret printed, logged or
put in an error; tests deterministic, network-free beyond `127.0.0.1`, writing only under a folder
of their own in `std::env::temp_dir()`; no new dependency; edition 2024; English.

## Questions

Where the specification is silent, write down your assumption and go on. If you cannot go on,
stop and end your turn with a list headed `QUESTIONS`, about behaviour only.

## Commits

One logical step per commit, subjects in the imperative, with
`git -c user.name=HoneySlatt -c user.email=170786304+HoneySlatt@users.noreply.github.com commit`.
No mention of any AI, agent or tool, and no trailer.

## Report

Your final message: each change, by file and function, and the section it meets; each test added
and what failed before; every assumption; anything left undone.
