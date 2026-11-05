# Roadmap

> **Decided by the owner on 2026-11-04**, when the project came under its method; its quality bar on
> 2026-11-05, and its order the same day
> ([ADR-0029](adr/0029-opium-waits-for-slime-ui-then-moves-first.md),
> [ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)); the move to MIT OR
> Apache-2.0 the same day too ([ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md)). Living document. It defines
> direction, ordering and exit criteria. It is not a task list: the current plan lives in
> `docs/PLAN.md`, and decisions live in `docs/adr/`. Stages will be
> merged, reduced, reordered or dropped as the project evolves; each such change is recorded in an
> ADR.

## 1. Vision

Opium is one music player for the computer and the phone, over every service its user has: Apple
Music, SoundCloud, their own Navidrome server and their own files, in one library, one search and
the same pages ([ADR-0007](adr/0007-one-library-and-the-best-copy.md),
[ADR-0016](adr/0016-the-services.md)). It looks and moves like the best players do, Apple Music's
and Spotify's, in looks its user can change ([ADR-0011](adr/0011-themes-as-toml-files-of-component-variants.md)).

"Excellent" means, for its user:

- each song from its best copy, at the best quality its service allows, reaching the sound card
  unchanged where the platform allows it, while other programs keep playing
  ([ADR-0005](adr/0005-pipewire-at-each-tracks-own-rate.md));
- the computer and the phone as one: the same history and playlists, each controlling the other,
  at home and away ([ADR-0010](adr/0010-opium-connect-on-the-local-network.md),
  [ADR-0014](adr/0014-shared-state-merged-field-by-field.md), [ADR-0015](adr/0015-away-from-home-through-iroh.md));
- the lowest draw of power: nothing drawn at rest, battery measured on the phone;
- honest with the services and with the user: what a licence allows, nothing more
  ([ADR-0017](adr/0017-downloads-sealed-and-tied-to-the-subscription.md)).

## 2. Principles

1. **Quality before speed.** A feature lands when it is as good as the services' own apps make it,
   or it waits.
2. **Measured, not guessed.** Speed, memory and power claims come with figures, kept in the
   repository, taken the same way before and after.
3. **Nothing drawn at rest**, on the desktop as on the phone.
4. **Other programs' sound keeps playing.** No output mode silences them.
5. **Honest towards the services**: verified against the real service, sources cited, their DRM
   and their bot guards met, never broken.
6. **The user's data is safe**: secrets private, files written whole, downloads never lost by an
   update.
7. **One core for both apps**: what both do is written once, in Rust.

## Quality bar

> **Decided by the owner on 2026-11-05** ([ADR-0025](adr/0025-the-quality-bar.md)). Taken from the method's default bars for every project, for
> interfaces and desktop applications and for mobile applications, adapted to Opium; the sound row
> comes from the bar for audio. "Today" gives what was measured on 2026-11-05 on the high desktop
> tier, or says plainly that it was not measured. The figures themselves live in
> [`measurements.md`](measurements.md). No budget is yet anchored on a figure measured from a
> reference app, Spotify's battery on the phone aside. The desktop window is frozen until the move to
> uwui, the owner's UI framework ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)): the rows checked "with uwui" are worked on then.
> The phone's Compose screens are frozen until uwui replaces them
> ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)): its interface rows are worked on then, on uwui.

References: Apple Music's apps and Spotify's, on the computer and the phone, for looks, motion and
responsiveness; Spotify's Android app for the battery used while playing; beyond them, the most
refined current interfaces, in motion (springs, shapes that morph, transitions between elements),
materials, type and responsiveness. For sound: each track bit-perfect at its own rate while other
programs keep playing ([ADR-0005](adr/0005-pipewire-at-each-tracks-own-rate.md)).

Hardware, from the lowest tier supported to the highest; release builds. Aiming at the top never
narrows who can run Opium: on the lowest tier it holds its frame rate, answers at once and every
feature works; it only loses detail (animated covers, effects).

| Tier | Reference machine | Measured |
|---|---|---|
| Desktop, lowest | A 2015 laptop: 2 cores, 4 threads (Intel Core i5 of the Skylake generation), its integrated Intel HD 520, 4 GB of memory, a hard disk, a 1080p display at 60 Hz, scale 1 | Not measured |
| Desktop, high | The owner's desktop: AMD Ryzen 7 5800X (8 cores), an AMD RX 6800-class GPU (Navi 21) through Mesa's OpenGL, a 4K display at 240 Hz, scale 1.5 | 2026-11-05, [baseline](measurements.md#baseline-of-the-desktop-app-before-any-budget-2026-11-05) |
| Phone, lowest | An entry-level Android 14 phone: 4 GB of memory, a MediaTek Helio G99-class chip, a 90 Hz screen | Not measured |
| Phone, high | The owner's Pixel on GrapheneOS, 120 Hz | Partly, 2026-11-03, before the rename and before this bar |

Unmeasured, never assumed: every lowest tier, Intel and NVIDIA GPUs, Vulkan rendering (the window
draws through OpenGL), X11. No budget can be checked in CI yet: each needs a display, PipeWire or
the phone. Each budget gets a benchmark kept in the repository; one that can run in CI fails
when it exceeds its budget by more than its noise.

| Budget | Target | Measured by | Checked in |
|---|---|---|---|
| Desktop at rest: paused, window shown or hidden | No frame drawn, no CPU time, 0 wake-ups per second of Opium's own: Opium Connect wakes only when the network speaks to it. Today: 8.4 wake-ups a second, none on the window's threads ([at rest](measurements.md#1-at-rest-cpu-time-and-wake-ups)) | `scripts/measure/rest.sh`: `sample.py` and `top` | By hand, at each milestone touching the window or the workings |
| Desktop playing, window hidden or without the focus | No frame drawn; wake-ups only from the audio path. Today: not measured (it needs sound) | As above | By hand, as above |
| Desktop playing, window shown, nothing else moving | A frame only when the progress moves a pixel or the time a second. Today: a frame every 250 ms ([known debt](debt.md)); its cost not measured (it needs sound) | As above, and the frames counted | With uwui: the window's side; the player's wake now |
| Start-up to the first useful frame (the library from its cache) | Warm ≤ 100 ms on the high tier; cold no worse than today. Today: warm 173 ms to the window on screen ([start-up](measurements.md#3-warm-start-up)); cold not measured (it needs root) | `scripts/measure/startup.py`; a mark at the first useful frame, with uwui | With uwui |
| Desktop memory, at rest and scrolling Songs over the whole library with its covers | At rest ≤ 150 MiB resident; scrolling the whole library, no more than today's peak (400 MiB). Today: about 370 MiB at rest ([memory](measurements.md#4-memory)); scrolling not measurable before uwui | `scripts/measure/memwatch.py` (`VmRSS`); `heaptrack` | By hand; the window's share with uwui |
| Desktop power, at rest and while scrolling | At rest no measurable GPU power and no wake-up; playing, the work of the rows above and no more; CPU judged by its time and wake-ups. Today: 0 W at rest ([power](measurements.md#5-gpu-power-at-rest)); scrolling not measurable before uwui | `scripts/measure/power-compare.sh` (`power1_average`); RAPL or `powertop` run by the owner (root) | By hand; scrolling with uwui |
| Scrolling and animations | The display's rate held, no dropped frame: on the high tier 240 Hz at 4K and scale 1.5, 99th percentile ≤ 4.17 ms and no frame over 8.33 ms; on the lowest tier 60 Hz, ≤ 16.7 ms | A frame-time trace per animation, from uwui's frame statistics and MCP server | With uwui: unmeasurable before |
| Input to visible response | Within one frame where the work allows | The same trace, with input from the MCP server | With uwui: unmeasurable before |
| The window's thread | Never waits on a disk or the network. Today: not audited | Reading the code; a trace of the thread once instrumented | With uwui |
| Images | Decoded off the window's thread, in a cache bounded in memory that frees the GPU's. Today: one thread per cover file, unbounded ([known debt](debt.md)) | Reading the code; memory under load | With uwui |
| Rendering | Gradients dithered, colours blended correctly, text crisp and pixel-aligned at scales 1, 1.5 and 2, in each theme | Reference images through the screenshot mode | With uwui; in CI once the window renders without a display |
| The look in one place | Each theme's colours, durations, springs, radii and blur in one place, shown by a gallery of every component; the owner approves the look from captures made off screen. Today: colours and component variants in the theme files both apps read ([ADR-0011](adr/0011-themes-as-toml-files-of-component-variants.md)); motion and radii in each app's code; no gallery; the screenshot mode captures a window on screen | Captures of the gallery, or the screenshot mode until then | By the owner, with uwui |
| Accessibility | Screen readers name every control; the whole app by keyboard; reduced motion and transparency honoured; contrast checked in each theme. Today: no control named ([known debt](debt.md)) | Orca; keyboard only; contrast ratios | With uwui |
| Text ready for translation | Strings in one place per app, no sentence built from pieces, plurals by rule, layouts that hold longer text and right-to-left writing. Today: English only, the strings written in the code of both apps | A pseudo-locale with longer and right-to-left text | Once the owner decides Opium is translated ([open](#open)) |
| HDR, wide gamut, variable refresh | Not supported, stated: the interface is drawn in sRGB, and a player's interface gains nothing from HDR; covers, animated ones included, are converted to 8-bit sRGB (`crates/motion`), so HDR or wide-gamut artwork is shown clipped; frames follow the compositor's pace, with no variable-refresh handling of the app's own | — | — |
| Sound | No underrun while playing, under load (scrolling, a build), even when the graph runs 128-frame periods at 48 kHz, the method's default: Opium asks no latency of its own, so another program can set that quantum; each track at its own rate; other programs heard. Today: not measured (it needs sound) | `pw-top`: its error count and rate | By hand, at each milestone touching playback |
| Bad input | Never a crash: parsers of untrusted data fuzzed (the services' answers, MP4 and CENC boxes, HLS playlists, file tags, what paired devices send, themes, settings); `unsafe` under Miri where it can run | `cargo fuzz` in a nightly shell, and mutation tests in `cargo test` ([ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md)); Miri | CI: the mutation tests at every push, `cargo fuzz` weekly, from M1.10 for MP4, CENC and HLS |
| Crash report | A crash leaves a report the user can attach to an issue; nothing leaves the machine without the user's consent. Today: only a hung exit leaves `exit.log` and a core dump ([ADR-0024](adr/0024-the-exit-watch-aborts-for-a-core-dump.md)); no panic hook in either app | Reading the code; a forced panic on a throwaway profile | By hand, once built |
| Iteration | A warm CI run, whose cache hit, under 20 minutes; a cold run measured and recorded, without a budget; the full build and tests budgeted once measured. Today: the first cold CI run passed 31 minutes before the disk ran out; warm not measured; the full build and tests not measured | CI's durations; `scripts/check.sh` timed | CI, at each run |
| Phone cold start | To the first useful frame, no worse than today; warm ≤ 150 ms. Today: cold 600 to 750 ms on the owner's Pixel, before the rename ([phone](measurements.md#the-phone-2026-11-03)); warm not measured | Macrobenchmark from Stage 3 ([ADR-0027](adr/0027-macrobenchmark-for-the-phones-measurements-at-stage-3.md)); until then `adb shell am start -W` (`TotalTime`) and a Perfetto trace | By hand, on the owner's phone; a mid-range phone unmeasured |
| Phone frame times | The display's rate held while scrolling and animating, no janky frame: 120 Hz on the high tier (99th percentile ≤ 8.33 ms), 90 Hz on the lowest (≤ 11.1 ms). Today: not measured | Macrobenchmark from Stage 3 ([ADR-0027](adr/0027-macrobenchmark-for-the-phones-measurements-at-stage-3.md)); until then Perfetto's frame timeline and `adb shell dumpsys gfxinfo io.github.honeyslatt.opium framestats` | By hand, at Stage 3, on uwui against Compose |
| Phone battery while playing, screen off | No more than Spotify's for the same kind of song, Apple Music's protected songs included (under 844 ticks a minute). Today: 1,500 to 1,900 ticks a minute for Apple Music, before the rename ([phone](measurements.md#the-phone-2026-11-03)) | Macrobenchmark from Stage 3 ([ADR-0027](adr/0027-macrobenchmark-for-the-phones-measurements-at-stage-3.md)); until then CPU ticks from `/proc/<pid>/stat` over 60 s, and Perfetto's power rails where the phone exposes them | By the owner, or with the owner's go, at each milestone touching playback on the phone |
| Phone in the background | No work outside the system's schedulers and the foreground services of playback and downloads; paused, no wake-up of Opium's own. Today: not measured | `/proc/<pid>/status` over adb; a Perfetto trace | By hand |
| Phone memory, at rest and scrolling | No more than today. Today: not measured | `adb shell dumpsys meminfo io.github.honeyslatt.opium` | By hand |
| Phone accessibility | TalkBack names every control; the system's font scale honoured up to its largest, without clipped text; touch targets of at least 48 dp; reduced motion honoured | TalkBack; the largest font scale; the layout inspector | At Stage 3, on uwui |

## 3. Decisions

### Fixed

| Topic | Decision |
|---|---|
| Licence | GPL-3.0-or-later ([ADR-0001](adr/0001-gpl-3-0-or-later.md)) until M1.13, then MIT OR Apache-2.0, once the code adapted from Kopuz is written again in a clean room ([ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md)) |
| Name | Opium, everywhere, no compatibility with the old names ([ADR-0023](adr/0023-the-rename-to-opium.md)) |
| Services | Apple Music, SoundCloud, Navidrome (OpenSubsonic), the device's own files ([ADR-0016](adr/0016-the-services.md)) |
| One library | The best copy of each song: files, server, Apple Music, SoundCloud ([ADR-0007](adr/0007-one-library-and-the-best-copy.md)) |
| Apple Music | Through its web player, after Kopuz ([ADR-0002](adr/0002-apple-music-through-its-web-player.md)) |
| Sign-in | In the user's browser, on a profile of Opium's own ([ADR-0003](adr/0003-sign-in-through-the-users-browser.md)) |
| DRM | The browser's Widevine CDM through a shim on the desktop, MediaDrm on the phone ([ADR-0004](adr/0004-the-widevine-cdm-borrowed-from-the-browser.md)); from M1.11 a thin C++ layer, the host's logic in Rust on one thread ([ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md)) |
| Desktop audio | PipeWire only, at each track's rate ([ADR-0005](adr/0005-pipewire-at-each-tracks-own-rate.md)) |
| Desktop UI | egui, until the move to uwui during SlimeEngine's Stage 6d; GPUI abandoned ([ADR-0006](adr/0006-egui-for-the-desktop-window.md), [ADR-0022](adr/0022-gpui-tried-and-abandoned.md), [ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)) |
| Phone | Kotlin and Compose over the Rust core through UniFFI; ExoPlayer plays; Android 14 and later, arm64 ([ADR-0009](adr/0009-the-android-app-over-the-rust-core.md)) |
| Themes | TOML files choosing component variants; Stock, Spotify, M3E built in ([ADR-0011](adr/0011-themes-as-toml-files-of-component-variants.md)) |
| Devices | Opium Connect: mDNS, TCP 47820, SPAKE2, Noise; iroh away from home; state merged field by field ([ADR-0010](adr/0010-opium-connect-on-the-local-network.md), [ADR-0014](adr/0014-shared-state-merged-field-by-field.md), [ADR-0015](adr/0015-away-from-home-through-iroh.md)) |
| Offline | Sealed downloads, DRM songs tied to a subscription confirmed in 30 days ([ADR-0017](adr/0017-downloads-sealed-and-tied-to-the-subscription.md)) |
| Lyrics | The song's own source, then LRCLIB, other sites opt-in ([ADR-0008](adr/0008-the-lyrics-sources.md)) |
| Quality bar | Tiers and budgets ([ADR-0025](adr/0025-the-quality-bar.md)) |
| Desktop window | Frozen until the move to uwui, but for data loss, crashes, security flaws and M1.14's instruments; agents drive it through uwui then ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md), [ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)) |
| The phone's benchmarks | Macrobenchmark with Perfetto, from Stage 3 ([ADR-0027](adr/0027-macrobenchmark-for-the-phones-measurements-at-stage-3.md)) |
| The phone's screens | uwui, without condition, fixed until it matches Compose; Compose frozen until uwui replaces it, screen by screen; the Kotlin system layer goes on ([ADR-0028](adr/0028-the-phones-compose-screens-frozen-until-measured.md), [ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)) |

### Open

Each is decided by an ADR at the stage where it becomes necessary, not earlier.

| Topic | Candidates | Decide at |
|---|---|---|
| Dependency checks | `cargo deny` with a `deny.toml`, once the advisories it finds are fixed or accepted ([known debt](debt.md)) | Stage 2 |
| Translation | The text made ready for translation now, or English only until a release needs another language | Stage 2 |
| Miri | The nightly shell of fuzzing ([ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md)), or another | When `unsafe` code is first put under it |
| CI's time | One job, or the checks split across jobs if a run exceeds the owner's limit | Stage 1 |
| What a release exposes of DRM | The owner's call: the repository's visibility, whether the CDM borrowing and the protected services ship, what becomes of the developer examples that write decrypted audio (`am fetch`, `am live`, `sc fetch`) | Stage 2 |
| The release's signing key | The owner creates the keystore; how the build reads it, kept out of the repository | Stage 2 |
| Third-party licences | `cargo-about` for the Rust crates, and the Gradle dependencies' own list, or one tool for both | Stage 2 |
| Cleartext HTTP on the phone | Only for the Navidrome host the user types (checked in the HTTP client, since Android's network security file cannot name a host chosen at run time), or only for private addresses | Stage 2 |
| Windows audio | WASAPI exclusive, or shared with `MATCH_FORMAT`, through the `wasapi` crate, weighed against principle 4 | Stage 4 |
| The exit watch | Kept as a safety net, or removed once the hang is found ([ADR-0021](adr/0021-the-exit-watch.md)) | When the hang is found |

## 4. Stages

The stages run in the order 1, 5, 2, 3, 4: the move to uwui comes first, during SlimeEngine's
Stage 6d ([ADR-0029](adr/0029-opium-waits-for-slime-ui-then-moves-first.md),
[ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)). They keep their numbers, which
earlier ADRs cite.

### Stage 1 — Under the method

**Goal.** The repository holds its rules and its memory, its checks pass and CI runs them on every
push, and the rename to Opium is committed with the owner's data moved.

- `AGENTS.md`, `docs/`, `CHANGELOG.md`, `scripts/check.sh` and CI
- The checks green: the workspace formatted once, dead code removed, the documentation's broken
  links fixed, the flaky test fixed
- The rename committed, the repository renamed to `HoneySlatt/Opium`, the owner's data on both
  devices moved to the new names ([ADR-0023](adr/0023-the-rename-to-opium.md))
- The Android dev shell building again from an empty store, and the phone's core checked
- What the desktop app needs from a UI framework, in the repository, for SlimeEngine's Stage 6d
  ([`ui-framework-needs.md`](ui-framework-needs.md))
- The quality bar measured as the app is today, its budgets decided by the owner
- The code adapted from Kopuz written again in a clean room, then Opium under MIT OR Apache-2.0
  ([ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md))
- The figures uwui must match, measured under egui and on the Compose phone
  ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md))

**Exit.** `scripts/check.sh` and `scripts/check.sh android` pass locally and CI is green on `main`;
the owner's computer and phone run Opium under its new names with their data; the window's needs
from a UI framework are written down and reviewed (M1.6); the budgets are decided (M1.7); Opium is
under MIT OR Apache-2.0 with nothing of Kopuz's left (M1.13); the figures to beat are measured
(M1.14); the stage audit is recorded and its findings decided (M1.8).

### Stage 5 — On uwui

**Goal.** The desktop window drawn by uwui, the owner's UI framework, egui gone. Opium is the first
of the four programs SlimeEngine's Stage 6d ports, before uwui's API is declared stable; Slatty
comes after it ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)). Opium is
on standby until the stage plans its ports, and this stage comes right after Stage 1
([ADR-0029](adr/0029-opium-waits-for-slime-ui-then-moves-first.md)).

- The interface's state and logic taken out of `app.rs` into a model free of any toolkit
- Each page and component, in each theme's variants, on the framework, with every need of
  [`ui-framework-needs.md`](ui-framework-needs.md)
- Agents read and drive the window through the framework; screen readers name its controls
- uwui's desktop host on winit runs the program; Opium's logic depends on no part of uwui
  ([ADR-0019](adr/0019-the-workings-apart-from-the-drawing.md))
- A need missing in uwui added there, under its rules, when it serves others too; one of Opium's
  alone built in Opium; neither worked around
- Then, as Stage 6d's third port, a prototype of Opium's phone screens on uwui, in a surface the
  Kotlin app gives it, measured against the same screens in Compose in smoothness, start-up and
  power, with a Macrobenchmark module and Perfetto
  ([ADR-0027](adr/0027-macrobenchmark-for-the-phones-measurements-at-stage-3.md)). uwui is taken
  whatever it shows; the measurement lists what to fix until it matches Compose

**Exit.** Every page of the three themes on the framework, egui removed; nothing drawn at rest under
Wayland and under Xwayland; the display's rate held while scrolling and animating; start-up, frame
times while scrolling and animating, memory and power at rest at least as good as egui's figures of
M1.14, measured the same way; the phone's prototype measured against Compose, and what uwui must
fix recorded.

### Stage 2 — The first release → 0.1.0

**Goal.** Opium can be handed to someone else: a Linux package and a signed APK that build
reproducibly, carry their licences and expose of DRM what the owner decided.

- The owner's decision on what the release exposes of DRM, recorded as an ADR
- A release signing key for the APK, created by the owner and kept out of the repository
- The third-party licences of both apps gathered and shipped, with the bundled assets' texts
  (Geist's OFL, Lucide's ISC, the logos' own)
- A Gradle wrapper, so the APK builds with a pinned Gradle
- Cleartext HTTP on the phone limited to the Navidrome server
- README and CHANGELOG for 0.1.0
- The dependencies checked (`cargo deny`), once their advisories are fixed or accepted
- A release audit before the tag, recorded in [`audits/`](audits/README.md): the stage audit, plus
  the changelog, the version, signed and reproducible packages, debug symbols kept, a run on each
  platform (no earlier release to update from)

**Exit.** A tagged 0.1.0 whose Linux package (`nix build`) and signed APK were installed and run
from clean profiles; both list their third-party licences; CI is green.

### Stage 3 — The phone on par

**Goal.** The phone on uwui, doing what the computer does, as smoothly and as frugally as Compose
did ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)).

- uwui fixed until it matches Compose on the prototype's measurement (Stage 5)
- The screens moved to uwui one at a time; the Kotlin system layer stays (playback, MediaDrm,
  the media session, Android Auto, the services)
- The phone's budgets measured with the Macrobenchmark module
- On uwui: the phone's M3E completed where its design has parts the phone
  lacks; the interface debt of the [2026-10-31 audit](audits/2026-10-31-full.md) (recompositions,
  accessibility labels, touch targets)
- Opium Connect's node no longer built on the main thread (the system layer, not frozen)
- Battery and smoothness measured against the reference player, before and after each change

**Exit.** Every screen on uwui, as good as Compose on M1.14's figures; every page of each theme
exists on the phone; TalkBack names every control; the measurements are in the repository and no worse than at the
stage's start.

### Stage 4 — Windows (eventual)

**Goal.** Opium's desktop app on Windows, with the same functions and quality bar as on Linux.

- First, a spike: the browser's Widevine CDM on Windows requires its host to be verified
  (`VerifyCdmHost`), which the shim does not do; if Opium cannot verify, Apple Music and
  SoundCloud's protected streams cannot play there, and the stage is reshaped
- Audio through WASAPI (`wasapi` crate), as decided at the stage's start
- The downloads' key in Windows' credential store (`keyring`), file windows through `rfd`, the media
  controls through SMTC, a single instance without D-Bus, the folders where Windows keeps them
- An installer (Inno Setup), and a CI job on Windows

**Exit.** On a fresh Windows 11, the installed Opium signs in to each service, plays each kind of
song at its own rate, downloads and plays offline, pairs with the phone, and its checks pass in CI
on Windows.

## 5. Release map

| Version | Stage | Content |
|---|---|---|
| — | 1 | Under the method, the rename (no release) |
| — | 5 | On uwui (no release: 0.1.0 ships on it) |
| 0.1.0 | 2 | The first release |
| 0.2 | 3 | The phone on par |
| 0.3 | 4 | Windows |
