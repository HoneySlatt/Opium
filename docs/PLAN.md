# Plan

This file says what is being worked on now: the milestones of the current stage, their status, how
each one is checked, and the known debt. Direction beyond this stage is in
[`ROADMAP.md`](ROADMAP.md); decisions are in [`adr/`](adr/README.md). A *stage* is a step of the
roadmap; a *milestone* is a small, verifiable step of a stage.

Every milestone ends with the definition of done in [`AGENTS.md`](../AGENTS.md#definition-of-done).

## Current

- Stage 1 — Under the method, started on 2026-11-04. M1.1, M1.2, M1.3 and M1.7 are done.
- M1.3: the first CI run ran out of disk while linking the tests; the workflow now frees the disk,
  builds without debug info and keeps its cache even when a check fails. CI is green on `main`
  (2026-11-05: 24 minutes cold, 9 min 25 s warm, under the 20-minute budget). M1.3 is done.
- M1.4: the rename has landed on the owner's computer and its folder; the phone remains.
- The egui window is frozen until the move to SlimeEngine's UI framework, slime-ui
  ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)), and the phone's Compose screens
  until Stage 3 measures slime-ui against Compose ([ADR-0028](adr/0028-the-phones-compose-screens-frozen-until-measured.md)). Work goes to the shared
  crates, the phone's Kotlin system layer and the documents.
- Next: M1.6; M1.4's phone and M1.5 once Google's Android repository answers again; then M1.8,
  the stage audit, which closes the stage.

## Status

| Milestone | Goal | Status |
|---|---|---|
| [M1.1](#m11--the-projects-memory-in-the-repository) | A session arriving in the repository finds the rules, the decisions, the direction and the plan | Done |
| [M1.2](#m12--the-checks-green) | `scripts/check.sh` passes on `main` | Done |
| [M1.3](#m13--ci-green) | Every push to `main` and every pull request is checked by CI | Done |
| [M1.4](#m14--the-rename-landed) | The owner's computer and phone run Opium under its new names, with their data | In progress: the computer done |
| [M1.5](#m15--the-phones-core-checked) | The Android shell builds from an empty store and the phone's core is checked | Planned |
| [M1.6](#m16--what-the-window-needs-from-a-ui-framework) | SlimeEngine's Stage 6d can read, in this repository, what Opium's window needs | Planned |
| [M1.7](#m17--the-quality-bar-measured) | The owner decides Opium's budgets with today's figures in hand | Done |
| [M1.8](#m18--the-stage-audit) | The whole project examined before Stage 1 closes | Planned |

## Stage 1 — Under the method

Opium came under the owner's project method on 2026-11-04: its memory moves from the owner's notes
into the repository, its checks become one command that CI runs, the rename decided the same day
([ADR-0023](adr/0023-the-rename-to-opium.md)) lands, and its quality bar is measured. No feature is
built in this stage.

### M1.1 — The project's memory in the repository

- **Goal.** Any agent or person arriving in the repository knows the rules, the decisions already
  taken, the direction and what to do now.
- **Delivers.** `AGENTS.md` (a local `CLAUDE.md` holds only `@AGENTS.md`), `CONTRIBUTING.md`, and
  in `docs/`: the architecture, the ADRs, the ROADMAP, this plan, the history, the measurements,
  the audits, the glossary; `CHANGELOG.md`.
- **Done when.** The owner has given their go on the ROADMAP and the ADRs; the files are committed.
- **How to test.** `scripts/check.sh`, whose last step checks the links; a reread as a newcomer.
- **Measured.** Nothing.

### M1.2 — The checks green

- **Goal.** `scripts/check.sh` passes, so that from now on a failure means a change broke something.
- **Delivers.** The workspace formatted; two variants never constructed removed; four documentation
  links to private items made plain code; `synced`'s flaky test fixed (`d1c8543`).
- **Done when.** `scripts/check.sh` passes; the test passes 30 runs in a row.
- **How to test.** `for i in $(seq 30); do cargo test -q -p synced --lib kept_on_disk; done`.
- **Measured.** Nothing.

### M1.3 — CI green

- **Goal.** Every push to `main` and every pull request is checked, in the same Nix shell.
- **Delivers.** The workflow running on GitHub; every tool of `scripts/check.sh` in the dev shell;
  the runner's disk freed and the tests built without debug info or incremental data, after the
  first run filled the disk while linking the tests (after 31 minutes, clippy taking about 12); the
  cache saved even when a check fails.
- **Done when.** CI is green on `main`. A warm run, whose cache hit, takes under 20 minutes (the
  quality bar's iteration budget), or the checks are split across jobs by an ADR; a cold run is
  measured and recorded, without a budget.
- **How to test.** A push to a branch and a pull request.
- **Measured.** CI's duration, cold and warm, and the cache's size, in
  [`measurements.md`](measurements.md).
- **By hand.** The owner allows the workflow on the repository.

### M1.4 — The rename landed

- **Goal.** Opium under its new names on the owner's computer and phone, with their data.
- **Delivers.** The rename ([ADR-0023](adr/0023-the-rename-to-opium.md), `8e8784d`); the GitHub
  repository renamed `HoneySlatt/Opium` (`8e82ab7`); the owner's data moved on the computer: the
  folders, the keyring item, the sealed files' magic, the shared state's provider. Left: the
  repository's folder renamed; on the phone, the new package installed (release build), signed in
  and paired again.
- **Done when.** On the computer, Opium starts on the moved folders with its library, sign-ins and
  downloads; on the phone, it is signed in and paired, and the history and playlists match.
- **How to test.** `scripts/check.sh`; a run on a
  [throwaway profile](../CONTRIBUTING.md#a-throwaway-profile).
- **Measured.** Nothing.
- **By hand.** The owner moves the folder and the phone's data, and checks both devices.

### M1.5 — The phone's core checked

- **Goal.** The phone's Rust core is checked like the rest, and its shell builds anywhere.
- **Delivers.** The Android shell building from an empty store again: on 2026-11-04, Google's
  server answered 404 for the SDK's `cmake-4.1.2-linux.zip`. Then `scripts/check.sh android` run,
  and a CI job for it if the SDK's download fits CI's time.
- **Done when.** `nix develop .#android` builds on a machine that never built it, and
  `scripts/check.sh android` passes.
- **How to test.** `nix develop .#android -c scripts/check.sh android`; `./android/build-core.sh`
  and `gradle assembleRelease`.
- **Measured.** The CI job's duration, if one is added.

### M1.6 — What the window needs from a UI framework

- **Goal.** SlimeEngine's Stage 6d, whose exit needs every indispensable need of the music player,
  can read those needs here.
- **Delivers.** A document in `docs/` listing what Opium's window asks of a framework (windows,
  input, drawing, images, motion, lists, text, waking, the system, accessibility, themes), each
  need checked against the code, marked indispensable or wished, with what egui gives today.
- **Done when.** Every need cites the code that asks it; the owner has reviewed it.
- **How to test.** A reread against the code.
- **Measured.** Nothing.

### M1.7 — The quality bar measured

- **Goal.** The owner decides Opium's budgets with today's figures in hand, and each later change
  is measured the same way.
- **Delivers.** The desktop's baseline at rest, at start and in memory and GPU power, and the
  phone's figures of 2026-11-03, in [`measurements.md`](measurements.md); the budgets, decided by
  the owner ([ADR-0025](adr/0025-the-quality-bar.md)); the scripts in `scripts/measure/`.
- **Not measurable yet.** Frame times, input to response, and memory and power while scrolling: the
  window cannot be driven and records no frame times, until slime-ui ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)). Cold start
  needs root.
- **Done when.** Every row of the quality bar has a figure or says why it has none; the owner has
  decided the budgets.
- **Measured.** Every budget of the quality bar that can be.

### M1.8 — The stage audit

- **Goal.** Stage 1 closes on a project examined whole, so that Stage 2 starts from known facts.
- **Delivers.** A stage audit over every dimension of the method: correctness; edge cases;
  performance, every budget measured again; resources at rest; security and privacy; data safety;
  tests, with mutation testing (`cargo mutants`), none skipped or flaky; readability; documentation;
  dependencies; accessibility; debt. One fresh reviewer per dimension, each finding verified by a
  test, a measurement or the exact line. Two checks of reading: a fresh agent builds, runs and
  tests Opium from the README alone and answers a newcomer's questions from the documents; another
  explains a module from its links. Recorded in [`audits/`](audits/README.md); open items added to
  the debt below.
- **Done when.** The audit is recorded and the owner has decided on its findings; Stage 1 moves to
  [`history.md`](history.md).
- **Measured.** Every budget of the quality bar, again.
- **By hand.** The owner decides the fixes that change direction.

## Ideas

For the owner to pick; none is planned.

- **A gallery of every component in each theme**, captured off screen: the look approved on
  captures, reference images for CI. On the desktop, with slime-ui.
- **A crash report of the app's own**: a panic hook writing what the user can attach to an issue;
  the phone's now, the desktop window's with slime-ui.
- **The text ready for translation**, if Opium is to be translated: both apps write their strings in
  the code; with slime-ui on the desktop, and on the phone's chosen toolkit.
- Driving Opium by an agent comes with slime-ui's MCP server ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)); a Macrobenchmark
  module for the phone is decided for Stage 3
  ([ADR-0027](adr/0027-macrobenchmark-for-the-phones-measurements-at-stage-3.md)).

## Known debt

- **Opium wakes 8.4 times a second at rest**, its window's threads aside
  ([causes](measurements.md#1-at-rest-cpu-time-and-wake-ups)): the idle `blocking` thread, the
  player's idle timeout and Opium Connect are shared work; the theme watch's poll, in
  `app-desktop`, waits for slime-ui.
- **The player cannot wake the window**: the window asks it for its state every 250 ms while music
  plays. The player's `Wake` is shared work; the window's side waits for slime-ui.
- **Waiting for slime-ui**, as `app-desktop` is frozen ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)):
  - Opium cannot be driven by an agent: nothing scrolls, clicks or types;
  - animations that go on without the focus (`ui/lyrics.rs`, `ui/loading.rs`);
  - one thread per cover file, unbounded (`ui/thumbs.rs`); WebP and GIF covers not decoded;
  - no Korean among the fallback fonts (`fonts.rs`); no colour emoji;
  - no screen-reader names, focus ring or Tab order;
  - the exit hang inside eframe's teardown ([ADR-0021](adr/0021-the-exit-watch.md),
    [ADR-0024](adr/0024-the-exit-watch-aborts-for-a-core-dump.md));
  - the interface's state and logic mixed with egui in `app.rs`, taken out as part of the move.
- **The dependencies are not checked**: `cargo deny` finds three advisories against `zstd`
  (RUSTSEC-2026-0391, -0398, -0403), six unmaintained crates and a yanked `async-executor`; a
  passing `deny.toml` would ignore them or update `Cargo.lock`, the owner's call at Stage 2.
- **`unsafe` without `// SAFETY:`**: the calls into the CDM shim (`widevine/src/native.rs`),
  `File::from_raw_fd` (`opium-ffi/src/lib.rs`).
- **Public items without documentation**: not counted yet.
- **Left on purpose by the [2026-10-31 audit](audits/2026-10-31-full.md)**: SoundCloud's clear HLS
  joined whole before it plays; the player's real-time scratch buffer reallocated, 32-bit sources
  through `f32`, a hang on closing during a stuck read.
- **The phone, from the same audit** (ROADMAP, Stage 3): Opium Connect's node built on the main
  thread, which is system work; the page tint recomposing for 450 ms, the whole app recomposing,
  accessibility labels and touch targets, which wait for the phone's toolkit ([ADR-0028](adr/0028-the-phones-compose-screens-frozen-until-measured.md)).
- **For the first release** (ROADMAP, Stage 2): the APK is signed with Android's debug key; the
  phone lets cleartext HTTP go to any host; the Gradle project has no wrapper; no list of
  third-party licences exists; the examples `am fetch`, `am live` and `sc fetch` write decrypted
  protected audio to a file.

Finished stages move to [`history.md`](history.md), in a few lines each, so that this file holds the
current stage only, under about 2,000 words.
