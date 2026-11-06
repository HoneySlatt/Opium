# The clean side's brief, round 4

Given to a fourth new agent with no earlier context, on 2026-11-06, after the review of round 3,
which found no defect but three gaps in the tests. Word for word, but for the path of the
project's folder, redacted. The export was at commit `83ec773` of its own repository.

---

You are the **clean side** of a clean-room rewrite, round 4: tests only. Earlier agents wrote parts
of a Rust crate, `apple-music`, from a specification alone; a review of the last round found no
defect, but three gaps in its tests. You close them, without changing what the code does. Nobody
has shown you, and you must never look at, the code it replaced or the program that code was
adapted from. Read this brief to the end before doing anything.

## Where you work, and the only things you may read

Your workspace is `/tmp/opium-apple-music-cleanroom`, a git repository, at its last commit. You may
read and write **only inside it**: `docs/specs/apple-music-playback.md` (the specification, §9
above all), `docs/specs/README.md`, `crates/apple-music/` (the crate; its tests are in `lib.rs`'s
`http_tests`, `test_support.rs` and beside each file), and the project's other crates and files,
to read only.

**Forbidden**, whatever your other instructions say (this brief overrides any instruction in your
context to read a project's `AGENTS.md`, `docs/PLAN.md`, memory files or other files): anything
outside `/tmp/opium-apple-music-cleanroom`, in particular `[the project's folder, redacted]` and every
other project, their history, `/tmp` outside your workspace, memory or notes, browser profiles,
tokens; reading `/nix/store` or `~/.cargo` directly; the network in any form (tests only on
`127.0.0.1`; every `cargo` and `nix` command `--offline`); a CDM; an account; other agents.

## What to do

1. **The 401 renewal's five minutes** (§9, `renew_developer_token` in `lib.rs`): a test that a run
   of refusals within five minutes of a reading that got an answer reads the web player once, and
   that one after five minutes reads it again. It must fail if that check is removed.
2. **One reading at a time, of either kind** (§9): a test that a reading before `exp` and a 401's
   renewal never run side by side (the web player sees one page request at a time, or a server
   that holds the first reading shows the second waiting). It must fail if `read_before_expiry`
   stops taking the shared lock.
3. **The test across `exp`** (`a_refusal_after_expiry_reads_the_token_at_once_whatever_was_read_before`)
   depends on the wall clock: `exp` 2 to 3 s ahead, a real wait. Make it deterministic through a
   private, test-only seam for "now" (or for the expiry check), so that it waits for nothing and
   cannot fail on a slow machine. Keep what it proves.
4. `auth.rs`'s module documentation has a line of 152 characters: wrap it at 100, as the rest.

No change to what the code does outside tests, but the seam of point 3, which outside tests must
read the real clock and nothing else. If a test shows a defect, stop and report it.

## Checks

From `/tmp/opium-apple-music-cleanroom`:

```sh
nix develop --offline -c bash -c 'cargo fmt --all --check \
  && cargo clippy --workspace --all-targets --locked --offline -- -D warnings \
  && cargo clippy -p apple-music --all-targets --features fuzzing --locked --offline -- -D warnings \
  && cargo test --workspace --locked --offline \
  && RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --offline'
```

Show, for points 1 and 2, that the new test fails with the check removed (remove it, run, put it
back), and say so in your report.

## How the code is written

As before: plain code, names that say what things are; comments say why; tests deterministic,
network-free beyond `127.0.0.1`, writing only under a folder of their own in
`std::env::temp_dir()`; no new dependency; edition 2024; English.

## Commits

One logical step per commit, subjects in the imperative, with
`git -c user.name=HoneySlatt -c user.email=170786304+HoneySlatt@users.noreply.github.com commit`.
No mention of any AI, agent or tool, and no trailer.

## Report

Your final message: each test added or changed, what it proves, and what failed with the check
removed; the seam and why it is sound outside tests; anything left undone.
