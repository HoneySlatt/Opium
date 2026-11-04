# Plan

This file says what is being worked on now: the milestones of the current stage, their status, how
each one is checked, and the known debt. Direction beyond this stage is in
[`ROADMAP.md`](ROADMAP.md); decisions are in [`adr/`](adr/README.md). A *stage* is a step of the
roadmap; a *milestone* is a small, verifiable step of a stage.

Every milestone ends with the definition of done in [`AGENTS.md`](../AGENTS.md#definition-of-done).

## Current

- Stage 1 — Under the method: started on 2026-11-04. The rules, the documents, the check script and
  CI are drafted and wait for the owner's review (M1.1). Measured on the commit before the rename
  (`b2dbd14`), in a worktree of its own: formatting, clippy and the documentation fail, and one test
  fails now and then; the fixes were tried in that worktree and make `scripts/check.sh` pass (M1.2).
  The rename to Opium is committed (`8e8784d`) and the remote is `HoneySlatt/Opium`; the same
  failures stand there (M1.4). Next: the owner's review of the ROADMAP and the ADRs, then M1.2.

## Status

| Milestone | Goal | Status |
|---|---|---|
| [M1.1](#m11--the-projects-memory-in-the-repository) | A session arriving in the repository finds the rules, the decisions, the direction and the plan | Drafted, for the owner's review |
| [M1.2](#m12--the-checks-green) | `scripts/check.sh` passes on `main` | Planned; fixes verified in a worktree |
| [M1.3](#m13--ci-green) | Every push to `main` and every pull request is checked by CI | Planned |
| [M1.4](#m14--the-rename-landed) | The owner's computer and phone run Opium under its new names, with their data | In progress: committed, data to confirm |
| [M1.5](#m15--the-phones-core-checked) | The Android shell builds from an empty store and the phone's core is checked | Planned |
| [M1.6](#m16--what-the-window-needs-from-a-ui-framework) | SlimeEngine's Stage 6d can read, in this repository, what Opium's window needs | Planned |

## Stage 1 — Under the method

Opium came under the owner's project method on 2026-11-04: its memory moves from the owner's notes
into the repository, its checks become one command that CI runs, and the rename decided the same
day ([ADR-0023](adr/0023-the-rename-to-opium.md)) lands. No feature is built in this stage.

### M1.1 — The project's memory in the repository

- **Goal.** Any agent or person arriving in the repository knows the rules, the decisions already
  taken, the direction and what to do now.
- **Delivers.** `AGENTS.md` (tracked; a local `CLAUDE.md` holds only `@AGENTS.md`),
  `docs/ARCHITECTURE.md`, `docs/adr/` (ADRs 0001 to 0023 and their index), `docs/ROADMAP.md`
  (a proposal), this plan, `docs/glossary.md`, `CHANGELOG.md`, `scripts/check.sh`,
  `.github/workflows/ci.yml`; README's links to them.
- **Done when.** The owner has read the ROADMAP and the list of ADRs and given their go; the files
  are committed, one logical change per commit.
- **How to test.** `scripts/check.sh` (its link step once lychee is in the shell, M1.3); a reread
  of each document as a newcomer.
- **By hand.** The owner reviews the ROADMAP, the ADRs and the rules new to `AGENTS.md`.

### M1.2 — The checks green

- **Goal.** `scripts/check.sh` passes, so that from now on a failure means a change broke something.
- **Delivers.** Four changes, each a commit of its own:
  - the workspace formatted with `cargo fmt --all` (31 files, 141 places differ on `8e82ab7`), and
    nothing else in that commit;
  - the two variants never constructed, which clippy refuses, removed: `Shape::Burst`
    (`ui/shapes.rs`, its radius and its place in the test) and `Spring::Effects` (`ui/motion.rs`);
  - the four documentation links to private items made plain code: `Feed` (`cenc/src/stream.rs`),
    `GRACE` (`desktop/src/exit.rs`), `MAGIC` (`downloads/src/sealed.rs`), `client_id`
    (`soundcloud/src/lib.rs`);
  - `synced`'s test `kept_on_disk_from_one_launch_to_the_next` calling `Kept::flush()` before it
    reopens the folder, instead of waiting for `playlists.json` alone: it failed 2 runs in 8, as the
    writer renames `shared.json` after `playlists.json`.
- **Done when.** `scripts/check.sh` passes; the test passes 30 runs in a row.
- **How to test.** `scripts/check.sh`;
  `for i in $(seq 30); do cargo test -q -p synced --lib kept_on_disk; done`.
- **Verified beforehand.** On `b2dbd14`, in a worktree of its own, these four changes made
  `scripts/check.sh` pass (223 tests) and the test pass 30 runs in 30. They are to be made again on
  the renamed tree.

### M1.3 — CI green

- **Goal.** Every push to `main` and every pull request is checked, in the same Nix shell.
- **Delivers.** The workflow running on GitHub; its durations, cold and with the cache, recorded
  here; lychee added to the dev shell, or not, by the owner's decision.
- **Done when.** CI is green on `main`, cold and warm; the durations are recorded here; if a run
  exceeds the owner's time limit, the checks are split across jobs, by an ADR.
- **How to test.** A push to a branch and a pull request.
- **By hand.** The owner allows the workflow on the repository.

### M1.4 — The rename landed

- **Goal.** Opium under its new names on the owner's computer and phone, with their data.
- **Delivers.** The rename committed ([ADR-0023](adr/0023-the-rename-to-opium.md)), done in
  `8e8784d`; the GitHub repository renamed `HoneySlatt/Opium` and the README's addresses with it,
  done in `8e82ab7` as far as the remote and the README go; the owner's data moved
  once, on both devices together: the folders, the keyring item tagged again, the sealed files'
  magic rewritten, the shared state's `Music` provider converted to `Opium`; on the phone, the new
  package installed (release build), signed in and paired again.
- **Done when.** On the computer, Opium starts on the moved folders with its library, its sign-ins
  and its downloads playing; on the phone, Opium is signed in and paired with the computer, and the
  history and playlists match.
- **How to test.** `scripts/check.sh`; a run on a throwaway profile (AGENTS.md).
- **By hand.** The owner moves their data and checks both devices.

### M1.5 — The phone's core checked

- **Goal.** The phone's Rust core is checked like the rest, and its shell builds anywhere.
- **Delivers.** `flake.nix`'s Android shell building from an empty store again: on 2026-11-04,
  Google's server answered 404 for the SDK's `cmake-4.1.2-linux.zip`, which the shell's SDK
  includes, and no built SDK was in the store; then `scripts/check.sh android` run, and a CI job
  for it if the SDK's download fits CI's time.
- **Done when.** `nix develop .#android` builds on a machine that never built it, and
  `scripts/check.sh android` passes.
- **How to test.** `nix develop .#android -c scripts/check.sh android`; `./android/build-core.sh`
  and `gradle assembleRelease`.

### M1.6 — What the window needs from a UI framework

- **Goal.** SlimeEngine's Stage 6d, whose exit asks that every need the music player marks as
  indispensable exist, can read those needs here.
- **Delivers.** A document in `docs/` listing what Opium's window asks of a framework (windows,
  input, drawing, images, motion, lists, text, waking, the system, accessibility, themes), each
  need checked against the code, marked indispensable or wished, with what egui gives today.
- **Done when.** Every need cites the code that asks it; the owner has reviewed it.
- **How to test.** A reread against the code.

## Known debt

- **The player cannot wake the window**: while music plays, the window asks for a frame every
  250 ms to read the player's state (`app.rs`, `logic()`). The player needs a `Wake`, and only the
  player bar redrawn when its second or pixel changes.
- **Some animations go on while the window has lost the focus**: the lyrics' glide and word sweep
  (`ui/lyrics.rs`), the loading spinners and Spotify's pulsing skeletons (`ui/loading.rs`) ask
  for frames without checking it, where the playing row's bars and M3E's wave stop.
- **One thread per cover file**: each cover read from a music folder is decoded on a thread of its
  own (`ui/thumbs.rs`), with no limit on how many run at once.
- **WebP and GIF covers are not decoded**: the `image` crate is built for JPEG and PNG only.
- **Korean is not asked for among the fallback fonts** (`fonts.rs` asks fontconfig for Greek,
  Japanese, Arabic, Hebrew, Thai and Hindi); Chinese relies on the Japanese font.
- **No colour emoji**: only egui's monochrome emoji.
- **Screen readers get no names**: the app draws its own controls and names none (no `WidgetInfo`
  anywhere); there is no focus ring and no Tab order of its own.
- **Nothing is measured in the repository**: no figures for start-up time, CPU at rest, memory or
  power of the whole app; the figures in ADRs were taken once.
- **The exit hang's cause is unknown** ([ADR-0021](adr/0021-the-exit-watch.md)): `exit.log` is to be
  read when it happens again.
- **`unsafe` without `// SAFETY:`**: the calls into the CDM shim (`widevine/src/native.rs`),
  `libc::_exit` (`desktop/src/exit.rs`), `File::from_raw_fd` (`opium-ffi/src/lib.rs`).
- **Public items without documentation**: not counted yet; the method asks for every one.
- **The interface's state and logic are mixed with egui** in `app-desktop/src/app.rs` (about 6 300
  lines), to move out before the window moves to another toolkit (ROADMAP, Stage 5).
- **SoundCloud's clear HLS is joined whole before it plays** (`soundcloud/src/stream.rs`): a long
  mix waits for all its segments. Left on purpose in the 2026-10-31 audit, as the growing reader
  needs the length up front.
- **For the first release** (ROADMAP, Stage 2): the APK is signed with Android's debug key; the
  phone lets cleartext HTTP go to any host (`res/xml/network_security_config.xml`); the Gradle
  project has no wrapper; no list of third-party licences exists; the developer examples
  `am fetch`, `am live` and `sc fetch` write decrypted protected audio to a file.

## Completed stages

None yet.
