# History

The stages that are done, newest first, then the milestones done in the current stage, with what
each delivered, its figures and the decisions taken on the way. Stage 1 is the current stage and no
stage is done yet; the current work is in [`PLAN.md`](PLAN.md).

## Stage 1 — Under the method (in progress)

### Milestones done

#### M1.1 — The project's memory in the repository

`AGENTS.md`, `CONTRIBUTING.md`, the documents of `docs/` and `CHANGELOG.md`, with the owner's go on
the ROADMAP and the ADRs.

#### M1.2 — The checks green

The workspace formatted, dead variants removed, links fixed, and `synced`'s flaky test fixed
(`d1c8543`), passing 30 runs in a row.

#### M1.3 — CI green

The workflow on GitHub, in the same Nix shell; the runner's disk freed, tests built without debug
info, the cache kept on failure (since M1.5, on success only). On 2026-11-05: 24 minutes cold, 9 min
25 s warm, under the 20-minute budget ([measurements](measurements.md)).

#### M1.4 — The rename landed

The rename ([ADR-0023](adr/0023-the-rename-to-opium.md), `8e8784d`), the repository renamed
`HoneySlatt/Opium` (`8e82ab7`). On the computer, the owner's folders, keyring item, sealed files'
magic and the shared state's provider moved. On the phone, the new package installed on 2026-11-07,
a release build, then signed in and paired by the owner, whose history and playlists match the
computer's by the owner's report of 2026-11-07, not checked by an agent. The old
`io.github.honeyslatt.music` is still installed with its data, for the owner to remove.

#### M1.5 — The phone's core checked

The Android shell builds from an empty Nix store again, no SDK version changed: Google's 404s of
2026-11-04 and 2026-11-05 had passed. Built in 8 min 30 s, 11 GB, its six SDK archives downloaded
from Google and matching nixpkgs' hashes; `scripts/check.sh android` passes, the release APK builds.
CI's `android` job downloads the SDK at each run, so a version gone from Google fails it: 9 min 19 s
cold, 6 min 4 s warm, beside the `check` job
([measured](measurements.md#the-android-shell-and-cis-android-job-m15-2026-11-07)). `opium-ffi`'s
`unsafe` block and its `unsafe(no_mangle)` say why they are sound. The shell sets `ANDROID_SDK_ROOT`
and `ANDROID_NDK_ROOT` too, which a CI runner set to its own SDK. CI checks no Kotlin nor the APK
([debt](debt.md)). A red caused by Google blocks as any other
([ADR-0036](adr/0036-the-phones-ci-job-blocks-even-when-google-fails.md)).

#### M1.6 — What the window needs from a UI framework

[`ui-framework-needs.md`](ui-framework-needs.md): 113 needs, 108 indispensable, each citing the
code, the marks confirmed by the owner on 2026-11-05.

#### M1.7 — The quality bar measured

The desktop's baseline and the phone's figures of 2026-11-03 in
[`measurements.md`](measurements.md), the budgets decided by the owner
([ADR-0025](adr/0025-the-quality-bar.md)), the scripts in `scripts/measure/`. Frame times and input
to response stayed unmeasurable under egui until M1.14.

#### M1.9 — What stands between Opium and a permissive licence

[The licence audit](audits/2026-11-05-licence.md): the code adapted from Kopuz file by file,
everything else checked, what Kopuz does badly, indexing measured (0.37 ms for 7.2 MB).
[`specs/`](specs/README.md): the clean-room method and three specifications, attacked by a fresh
agent and corrected. [ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md), its four
questions decided by the owner, and
[ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md) for uwui. A playlist
removal without an id refused (`2645376`).

#### M1.10 to M1.12 — Kopuz's code written again

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
- **M1.10, done on 2026-11-05.** `cenc` written by two clean agents; mutation tests and four
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

#### M1.13 — Nothing of Kopuz's left

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

#### M1.15 — The audit's fixes

- **Goal.** Stage 1 closes with no loss of data, flaw, crash or broken service left of those the
  [stage 1 audit](audits/2026-11-10-stage-1.md) found.
- **Fixed and pushed by 2026-11-11**, finding by finding in [the audit's "Fixed"](audits/2026-11-10-stage-1.md#fixed),
  each from its failing test where one could be written:
  - no download, setting, play, playlist or pairing lost to a file that does not read, a failed
    save, a power cut, a sign-out or two instances: one instance per profile
    ([ADR-0043](adr/0043-one-instance-per-profile-held-by-a-lock.md)), every kept file written
    whole through one function, files set aside under names of their own;
  - the flaws closed, as far as each holds: no core dump ([ADR-0045](adr/0045-no-core-dump-through-the-core-size-limit.md),
    which replaced ADR-0039's way when a search found it would break the file windows), Navidrome's
    password out of addresses and over HTTPS off the local network
    ([ADR-0044](adr/0044-navidromes-password-as-it-is-used.md)), SoundCloud's and a paired
    device's ids checked, the 30 days counted on the clock's forward motion (a clock set back gains only the time between
    two readings: while the app is closed, or open with nothing downloaded played), the phone's
    dependencies verified by Gradle, three advisories updated;
  - Connect syncs a library of any size, in parts
    ([ADR-0046](adr/0046-the-shared-state-sent-in-parts.md)); the files are served at once while the
    services connect (10.4 s to 0.4 s on a stalled network);
  - the phone's first tests, on the JVM; the nineteen tests the audit wrote for gaps; the documents
    it found wrong.
- **Measured** ([figures](measurements.md#the-audits-fixes-m115-2026-11-11)): rest and the window's
  frames no worse. A regression found on the way, Home left loading at rest in 12 runs of 40 by the
  new order of the start's answers, was fixed in the frozen window, with the owner's go.
- **By hand, with the owner**: a release APK built at `114fc3f` was installed on the phone on
  2026-11-11, not debuggable, and started with no crash in its log; the one built at `0655a43`,
  with M1.15's last changes to the phone's core (its JVM tests and `scripts/check.sh android`
  passing there), is not installed yet: the phone was not connected;
  the owner checks its sign-ins, a desktop theme and a playback, and, on the owner's account,
  SoundCloud's changes after S3 and S8.
- **Left**, each under its stage in [the debt](debt.md): S6 (until Stage 2), the frozen screens'
  findings, the work for no wake-up at rest, and what M1.15 found and did not fix.

#### M1.14 — The figures to beat on uwui

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
    scrolling, none while the drawer and Now Playing move; stalls of up to 57 ms when the Albums
    grid first shows; 199 to 232 MiB after start, 395 MiB at most.
  - On the phone under Compose (`phone.py`): cold start 543 ms, 99th percentile of frame time 7 to
    8 ms scrolling, 19 ms between tabs, 273 to 380 MiB.
  - A headless Weston proved unfit for frame times: it paced transitions badly where the owner's
    display did not ([pitfall](../CONTRIBUTING.md#known-pitfalls)).
  - Not measured: each frame's GPU time, a cold start of the desktop, the phone's first useful
    frame.

#### M1.8 — The stage audit

- **Goal.** Stage 1 closes on a project examined whole, so that the next stage starts from known
  facts.
- **Delivers.** A stage audit over every dimension of the method: correctness; edge cases;
  performance, every budget that can be checked today measured again; resources at rest; security
  and privacy; data safety; tests, with mutation testing (`cargo mutants`), none skipped or flaky;
  readability; documentation; dependencies; accessibility; debt. One fresh reviewer per dimension,
  each finding verified by a test, a measurement or the exact line. Two checks of reading: a fresh
  agent builds, runs and tests Opium from the README alone and answers a newcomer's questions from
  the documents; another explains a module from its links. Recorded in
  [`audits/`](audits/README.md); open items added to the [known debt](debt.md).
- **Done when.** The audit is recorded and the owner has decided on its findings; Stage 1 moves to
  [`history.md`](history.md).
- **Measured.** Every budget of the quality bar that can be checked today, again. The owner set the
  audit's scope on 2026-11-10: mutation testing leaves out `app-desktop` and `opium-ffi`, the rivals
  are not measured ([audit](audits/2026-11-10-stage-1.md#scope)).
- **By hand.** The owner decides the fixes that change direction.
- **Done on 2026-11-10.** [The audit](audits/2026-11-10-stage-1.md), its proofs kept beside it:
  13 losses of data, 9 flaws, 12 cases of wrong behaviour, the budgets measured again on the
  desktop, its 4K display and the phone, mutation testing run twice (the first run's shared target
  made it unreadable). Five rounds of closing review. The owner's decisions: M1.15 fixes the losses,
  flaws, crashes and broken services; ADR-0039 to ADR-0042; the examples that write decrypted
  audio kept until Stage 2; `probe` only with the owner's go; the rest given a stage in
  [the debt](debt.md).

### Decisions and events

- **Opium measured against the market**, decided by the owner on 2026-11-08 in a design session that
  built nothing ([ADR-0038](adr/0038-opium-measured-against-the-market.md)): rivals in two families,
  the native players for resources and sound, the giants for experience; at least as frugal as the
  best native, 2.5 times faster than the best giant on what depends on Opium alone; figures for
  frames, sound, battery, the library and a handoff; transitions at AutoMix's level, notes from
  Wikipedia, each song's quality shown, the library's tools, Android Auto complete. The ROADMAP
  follows it.
- **uwui's decisions of 2026-11-07**, SlimeEngine's ADR-0213 to ADR-0220, taken into Opium the same
  day by the owner, documents only
  ([ADR-0037](adr/0037-uwuis-decisions-of-2026-11-07-taken-into-opium.md)): Opium's song playing, in
  Stock with glass, is uwui's fifth screen of reference, its effects brought to Stage 5; M3E is
  built on uwui's official theme; Opium's actions for users' agents are declared at Stage 5, a
  change to an account confirmed each time; macOS's Stage 10 comes after uwui's 1.0; uwui's bars for
  applications join the quality bar, their figures set at Stage 5. The ROADMAP and
  [`ui-framework-needs.md`](ui-framework-needs.md#what-uwuis-decisions-of-2026-11-07-ask) follow it.
- **Fixed on 2026-11-07**, under the standby's rule on a loss of data: a song's tags are written
  on a copy (`local::tag_writes`), one write at a time. The desktop renames the copy over the
  file. The phone keeps the original aside while it writes the copy over the file, and puts back
  a file whose writing was cut short, at the next start or before that file's next write. Tested
  in `cargo test`, the phone's core checked and its APK built; neither app was used to edit a
  song, and the phone's way back was never cut short on a device.
- **Awake to finish Stage 1**, since 2026-11-07, by the owner's decision
  ([ADR-0035](adr/0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md)): M1.14, done on
  2026-11-07, then the stage audit M1.8.
- **What Opium is to become**, decided by the owner on 2026-11-06 in a design session that built
  nothing ([ADR-0034](adr/0034-what-opium-is-to-become.md)): the general public, the platforms, the
  services, Connect, the library, the sound, the interface, the order of stages 6 to 10. The owner
  lifted the standby for it, documents only. The ROADMAP follows it, and
  [`ui-framework-needs.md`](ui-framework-needs.md#what-adr-0034-asks-of-uwui) lists what it asks of
  uwui; the owner carries that list to SlimeEngine's M6d.1b3b, which chooses how state changes an
  application's interface.
- Kopuz's code is written again but for part of the browser sign-in, which the owner kept, named
  ([ADR-0033](adr/0033-the-browser-sign-ins-pipe-kept-as-adapted-from-kopuz.md)); the GPL kept
  ([ADR-0032](adr/0032-the-gpl-kept-and-kopuzs-code-still-written-again.md)).
- Played with the owner on 2026-11-05, on the new `cenc` and `widevine`: Apple Music in stereo and
  Spatial Audio, the live radio (after a fix to `cenc`); on 2026-11-06, on M1.12's code, by the
  owner, who reported everything working.

<!-- ## Stage 1 — <title> → <release> (YYYY-MM-DD)

<what it delivered, in a few lines>. Decisions: [ADR-0001](adr/0001-title.md). -->
