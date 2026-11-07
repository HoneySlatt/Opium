# Plan

This file says what is being worked on now: the milestones of the current stage, their status, how
each one is checked; the known debt is in [`debt.md`](debt.md). Direction beyond this stage is in
[`ROADMAP.md`](ROADMAP.md); decisions are in [`adr/`](adr/README.md). A *stage* is a step of the
roadmap; a *milestone* is a small, verifiable step of a stage.

Every milestone ends with the definition of done in [`AGENTS.md`](../AGENTS.md#definition-of-done).

## Current

- Stage 1 — Under the method, started on 2026-11-04. M1.1 to M1.7, M1.9 and M1.10 to M1.14 are
  done; the stage audit M1.8 is next.
- **Awake to finish Stage 1**, since 2026-11-07, by the owner's decision
  ([ADR-0035](adr/0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md)): M1.14, done on
  2026-11-07, then the stage audit M1.8. After it, Opium waits for its port in uwui's first stage,
  after SlimeEngine's Stage 6d, fixing only a loss of data, a crash, a security flaw or a service
  that stops working.
- Kopuz's code is written again but for part of the browser sign-in, which the owner kept, named
  ([ADR-0033](adr/0033-the-browser-sign-ins-pipe-kept-as-adapted-from-kopuz.md)); the GPL kept
  ([ADR-0032](adr/0032-the-gpl-kept-and-kopuzs-code-still-written-again.md)).
- Played with the owner on 2026-11-05, on the new `cenc` and `widevine`: Apple Music in stereo and
  Spatial Audio, the live radio (after a fix to `cenc`); on 2026-11-06, on M1.12's code, by the
  owner, who reported everything working. Still due: a SoundCloud song served encrypted, which the
  owner's account did not get for the album tried.
- Then Stage 5, the move to uwui, when uwui's first stage takes Opium's port; the phone moves to
  uwui without condition ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md));
  the stages after it in the [ROADMAP's order](ROADMAP.md#4-stages).
- **What Opium is to become**, decided by the owner on 2026-11-06 in a design session that built
  nothing ([ADR-0034](adr/0034-what-opium-is-to-become.md)): the general public, the platforms, the
  services, Connect, the library, the sound, the interface, the order of stages 6 to 10. The owner
  lifted the standby for it, documents only. The ROADMAP follows it, and
  [`ui-framework-needs.md`](ui-framework-needs.md#what-adr-0034-asks-of-uwui) lists what it asks of
  uwui; the owner carries that list to SlimeEngine's M6d.1b3b, which chooses how state changes an
  application's interface.
- **Fixed on 2026-11-07**, under the standby's rule on a loss of data: a song's tags are written
  on a copy (`local::tag_writes`), one write at a time. The desktop renames the copy over the
  file. The phone keeps the original aside while it writes the copy over the file, and puts back
  a file whose writing was cut short, at the next start or before that file's next write. Tested
  in `cargo test`, the phone's core checked and its APK built; neither app was used to edit a
  song, and the phone's way back was never cut short on a device.
- The egui window is frozen until the move
  ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)), but for M1.14's instruments; the
  phone's Compose screens until uwui replaces them
  ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)).

## Status

| Milestone | Goal | Status |
|---|---|---|
| [M1.1](#done) | A session arriving in the repository finds the rules, the decisions, the direction and the plan | Done |
| [M1.2](#done) | `scripts/check.sh` passes on `main` | Done |
| [M1.3](#done) | Every push to `main` and every pull request is checked by CI | Done |
| [M1.4](#done) | The owner's computer and phone run Opium under its new names, with their data | Done |
| [M1.5](#done) | The Android shell builds from an empty store and the phone's core is checked | Done |
| [M1.6](#done) | SlimeEngine's Stage 6d can read, in this repository, what Opium's window needs | Done |
| [M1.7](#done) | The owner decides Opium's budgets with today's figures in hand | Done |
| [M1.8](#m18--the-stage-audit) | The whole project examined before Stage 1 closes | Planned |
| [M1.9](#done) | What stops MIT OR Apache-2.0 is known, and the code to write again is specified | Done |
| [M1.10](#m110-to-m112--kopuzs-code-written-again) | `cenc` written again in a clean room | Done |
| [M1.11](#m110-to-m112--kopuzs-code-written-again) | `widevine` written again in a clean room | Done |
| [M1.12](#m110-to-m112--kopuzs-code-written-again) | `apple-music`'s derived items written again in a clean room | Done |
| [M1.13](#m113--nothing-of-kopuzs-left) | Nothing of Kopuz's left, the GPL kept | Done, one part kept |
| [M1.14](#m114--the-figures-to-beat-on-uwui) | The figures uwui must match: the egui window's and the Compose phone's | Done |

## Stage 1 — Under the method

Opium came under the owner's project method on 2026-11-04: its memory moves from the owner's notes
into the repository, its checks become one command that CI runs, the rename decided the same day
([ADR-0023](adr/0023-the-rename-to-opium.md)) lands, and its quality bar is measured. No feature is
built in this stage.

### Done

- **M1.1 — The project's memory in the repository.** `AGENTS.md`, `CONTRIBUTING.md`, the
  documents of `docs/` and `CHANGELOG.md`, with the owner's go on the ROADMAP and the ADRs.
- **M1.2 — The checks green.** The workspace formatted, dead variants removed, links fixed, and
  `synced`'s flaky test fixed (`d1c8543`), passing 30 runs in a row.
- **M1.3 — CI green.** The workflow on GitHub, in the same Nix shell; the runner's disk freed, tests
  built without debug info, the cache kept on failure (since M1.5, on success only). On
  2026-11-05: 24 minutes cold, 9 min 25 s warm, under the 20-minute budget
  ([measurements](measurements.md)).
- **M1.4 — The rename landed.** The rename ([ADR-0023](adr/0023-the-rename-to-opium.md),
  `8e8784d`), the repository renamed `HoneySlatt/Opium` (`8e82ab7`). On the computer, the owner's
  folders, keyring item, sealed files' magic and the shared state's provider moved. On the phone,
  the new package installed on 2026-11-07, a release build, then signed in and paired by the owner,
  whose history and playlists match the computer's by the owner's report of 2026-11-07, not
  checked by an agent. The old `io.github.honeyslatt.music` is still installed with its data, for
  the owner to remove.
- **M1.5 — The phone's core checked.** The Android shell builds from an empty Nix store again,
  no SDK version changed: Google's 404s of 2026-11-04 and 2026-11-05 had passed. Built in 8 min
  30 s, 11 GB, its six SDK archives downloaded from Google and matching nixpkgs' hashes;
  `scripts/check.sh android` passes, the release APK builds. CI's `android` job downloads the SDK
  at each run, so a version gone from Google fails it: 9 min 19 s cold, 6 min 4 s warm, beside the
  `check` job ([measured](measurements.md#the-android-shell-and-cis-android-job-m15-2026-11-07)).
  `opium-ffi`'s `unsafe` block and its `unsafe(no_mangle)` say why they are sound. The shell sets
  `ANDROID_SDK_ROOT` and `ANDROID_NDK_ROOT` too, which a CI runner set to its own SDK. CI checks
  no Kotlin nor the APK ([debt](debt.md)). A red caused by Google blocks as any other
  ([ADR-0036](adr/0036-the-phones-ci-job-blocks-even-when-google-fails.md)).
- **M1.6 — What the window needs from a UI framework.**
  [`ui-framework-needs.md`](ui-framework-needs.md): 113 needs, 108 indispensable, each citing the
  code, the marks confirmed by the owner on 2026-11-05.
- **M1.7 — The quality bar measured.** The desktop's baseline and the phone's figures of
  2026-11-03 in [`measurements.md`](measurements.md), the budgets decided by the owner
  ([ADR-0025](adr/0025-the-quality-bar.md)), the scripts in `scripts/measure/`. Frame times and
  input to response stayed unmeasurable under egui until M1.14.
- **M1.9 — What stands between Opium and a permissive licence.** [The licence
  audit](audits/2026-11-05-licence.md): the code adapted from Kopuz file by file, everything else
  checked, what Kopuz does badly, indexing measured (0.37 ms for 7.2 MB).
  [`specs/`](specs/README.md): the clean-room method and three specifications, attacked by a fresh
  agent and corrected. [ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md), its four
  questions decided by the owner, and
  [ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md) for uwui. A playlist
  removal without an id refused (`2645376`).

### M1.8 — The stage audit

- **Goal.** Stage 1 closes on a project examined whole, so that Stage 2 starts from known facts.
- **Delivers.** A stage audit over every dimension of the method: correctness; edge cases;
  performance, every budget measured again; resources at rest; security and privacy; data safety;
  tests, with mutation testing (`cargo mutants`), none skipped or flaky; readability; documentation;
  dependencies; accessibility; debt. One fresh reviewer per dimension, each finding verified by a
  test, a measurement or the exact line. Two checks of reading: a fresh agent builds, runs and
  tests Opium from the README alone and answers a newcomer's questions from the documents; another
  explains a module from its links. Recorded in [`audits/`](audits/README.md); open items added to
  the [known debt](debt.md).
- **Done when.** The audit is recorded and the owner has decided on its findings; Stage 1 moves to
  [`history.md`](history.md).
- **Measured.** Every budget of the quality bar, again.
- **By hand.** The owner decides the fixes that change direction.

### M1.10 to M1.12 — Kopuz's code written again

One milestone each, in this order: `cenc`, then `widevine`, then `apple-music`.

- **Goal.** No line derived from Kopuz is left in the crate, and the crate does better than Kopuz
  where the [audit](audits/2026-11-05-licence.md#4-what-kopuz-does-badly-here-and-opium-with-it)
  found it lacking.
- **Delivers.** The new code, written by a clean agent from its specification alone, on an export of
  the files [the method](specs/README.md) allows. The code given is already on the specification's
  names; the dirty side moves the callers outside the export afterwards. The adapted files and their notices are deleted. Each
  milestone also delivers:
  - the tests its specification lists;
  - for `cenc`, the mutation tests and the `cargo fuzz` targets, with their nightly shell;
  - the token comparison's report, and the evidence, in `docs/audits/`.
- **Done when.**
  - Today's tests pass, plus the new ones.
  - The comparison finds no run above its threshold that is not forced by a standard.
  - `scripts/check.sh` passes.
  - Opium plays on a throwaway profile.
  - A reviewer has checked the milestone against this method.
- **By hand, with the owner.**
  - Before M1.11: the CDM's figures taken again (a licence needs the owner's account).
  - After M1.11 and M1.12: Apple Music and SoundCloud's protected songs played, Spatial Audio and
    the live radio included; after M1.12, a library song and a station's too. Fakes do not prove
    a service.
  - Re-taking Chromium's CDM header at a pinned revision needs a download, the owner's go.
- **Measured.** Before and after: indexing and the table's memory (`cenc`); a licence's time, the
  CDM's time per sample and its wake-ups at rest (`widevine`); the developer token, the library, a
  station's batch and a song's start, and the token's search (`apple-music`).
- **M1.10, done on 2026-11-05.** `cenc` written by two clean agents; mutation tests and three
  `cargo fuzz` targets, run weekly (`scripts/fuzz.sh`); indexing 0.45 → 0.33 ms
  ([measurements](measurements.md#indexing-an-encrypted-track-m110-2026-11-05),
  [evidence](audits/2026-11-05-cenc-clean-room.md)).
- **M1.11, done on 2026-11-05.** `widevine` written by two clean agents: one `cdm` thread, a thin
  C++ layer, Chromium's headers pinned; a decrypt's median +8.8 µs and p95 +25 µs, one wake-up a
  second with a session open, all accepted by the owner
  ([measurements](measurements.md#the-cdm-m111-2026-11-05),
  [evidence](audits/2026-11-05-widevine-clean-room.md)).
- **M1.12, done on 2026-11-06.** The specification checked against the code and
  Apple's public page, and corrected; `apple-music`'s derived items written by four clean agents
  in turn, the later three after the reviews. A station's batch 2 252 → 1 278 ms; a token read
  again before its `exp`, at most once in five minutes; a download without a length plays; the HTTP behaviours
  tested against a local server; eight `cargo fuzz` targets; `cenc`'s reservation bounded. The
  token's search 1.2 to 1.3 µs slower once warm, within the 4.5 µs the owner accepted; on round
  1's code, the developer token's reading 6.6 ms slower in median, within the network's spread
  ([measurements](measurements.md#apple-musics-steps-m112-2026-11-05-and-2026-11-06),
  [evidence](audits/2026-11-06-apple-music-clean-room.md)). Opium starts on a throwaway profile;
  played by the owner on the owner's profile, everything working by the owner's report.
- Each found nothing owed to Kopuz; what each left is in [debt.md](debt.md).

### M1.13 — Nothing of Kopuz's left

- **Goal.** No code of Opium's derives from Kopuz, and every document says so. Opium stays under
  GPL-3.0-or-later ([ADR-0032](adr/0032-the-gpl-kept-and-kopuzs-code-still-written-again.md),
  decided on 2026-11-06 in place of MIT OR Apache-2.0).
- **Delivers.**
  - A token comparison of the whole tree with Kopuz at both revisions, recorded in
    [`audits/`](audits/README.md).
  - The credits that name Kopuz for code made true: the about screens on both apps (the desktop's
    says "Widevine and CENC code adapted from Kopuz"), the README, the third-party notices the
    [licence audit](audits/2026-11-05-licence.md) lists, the CHANGELOG. Kopuz stays credited for
    the facts about Apple's service it found.
- **Done when.**
  - M1.10 to M1.12 are done.
  - The comparison finds no run above its threshold owed to Kopuz.
  - No document or screen says that Opium holds Kopuz's code.
- **Measured.** Nothing.
- **Done on 2026-11-06.** The whole tree compared at both revisions, every run above the thresholds
  read by hand ([evidence](audits/2026-11-06-kopuz-whole-tree.md)). One part was found still
  adapted, `DevTools` in `browser-signin`; the owner kept it, named, so the second and third
  criteria hold for all but it
  ([ADR-0033](adr/0033-the-browser-sign-ins-pipe-kept-as-adapted-from-kopuz.md)). The about
  screens, the README and the CHANGELOG say so. The test helper `boxed`, still Kopuz's in two
  test files, took the form M1.10's clean side wrote.

### M1.14 — The figures to beat on uwui

- **Goal.** The port to uwui is judged on figures, as SlimeEngine's ADR-0178 asks: uwui must run
  Opium at least as well as egui does, and later the phone as well as Compose.
- **Delivers.**
  - On the high desktop tier, under egui: start-up, frame times while scrolling and while animating,
    and memory.
  - The window gets a frame-time record and a scripted scroll, the one change allowed to it
    ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)).
  - On the owner's phone, under Compose, in a release build: start-up, frame times, memory.
- **Done when.** The figures are in [`measurements.md`](measurements.md), taken by scripts kept in
  `scripts/measure/`, before uwui's first stage ports Opium.
- **By hand.** The phone, with the owner.
- **Done on 2026-11-07.** [The figures](measurements.md#the-figures-to-beat-on-uwui-m114-2026-11-07):
  - The window's measuring mode (`measure.rs`, [how](../CONTRIBUTING.md#the-measuring-mode)):
    a record of every frame and four scenarios, run by `scripts/measure/frames.py` over a library
    `library.py` generates, the same every time.
  - On the owner's 4K display: 152 ms to the first useful frame; 10 frames late in 85,433 while
    scrolling, none while the drawer and Now Playing move; stalls of up to 56 ms when the Albums
    grid first shows; 199 to 232 MiB after start, 395 MiB at most.
  - On the phone under Compose (`phone.py`): cold start 543 ms, 99th percentile of frame time 7 to
    8 ms scrolling, 19 ms between tabs, 273 to 380 MiB.
  - A headless Weston proved unfit for frame times: it paced transitions badly where the owner's
    display did not ([pitfall](../CONTRIBUTING.md#known-pitfalls)).
  - Not measured: each frame's GPU time, a cold start of the desktop, the phone's first useful
    frame.

## Ideas

For the owner to pick; none is planned.

- **A gallery of every component in each theme**, captured off screen: the look approved on
  captures, reference images for CI. On the desktop, with uwui.
- **CI building the phone's app**: `build-core.sh` and `gradle assembleRelease` in the `android`
  job, so that Kotlin which no longer matches the core fails CI ([debt](debt.md)); with Stage 2's
  Gradle wrapper. Its time is not measured.
- **A crash report of the app's own**: a panic hook writing what the user can attach to an issue;
  the phone's now, the desktop window's with uwui.
- **What Kopuz has and Opium lacks** ([licence audit](audits/2026-11-05-licence.md)): stations
  seeded by a playlist, and Apple's requests in the user's language (Opium asks in English).
- Driving Opium by an agent comes with uwui's MCP server
  ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)); a Macrobenchmark module for the
  phone is decided for Stage 3
  ([ADR-0027](adr/0027-macrobenchmark-for-the-phones-measurements-at-stage-3.md)).

## Known debt

In [`debt.md`](debt.md).

Finished stages move to [`history.md`](history.md), in a few lines each, so that this file holds the
current stage only, under about 2,000 words.
