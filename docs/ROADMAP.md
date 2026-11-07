# Roadmap

> **Decided by the owner on 2026-11-04**, when the project came under its method; its quality bar on
> 2026-11-05, and its order the same day
> ([ADR-0029](adr/0029-opium-waits-for-slime-ui-then-moves-first.md),
> [ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)); the move to MIT OR
> Apache-2.0 the same day too ([ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md)),
> given up on 2026-11-06 for the GPL
> ([ADR-0032](adr/0032-the-gpl-kept-and-kopuzs-code-still-written-again.md)); what Opium is to
> become, its stages 6 to 10, on 2026-11-06 too
> ([ADR-0034](adr/0034-what-opium-is-to-become.md)). Living document. It defines
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

It is for the general public: made first for its owner, built so that anyone can install and
understand it, on Linux, Windows, later macOS, and Android, with a player without a screen for a
room ([ADR-0034](adr/0034-what-opium-is-to-become.md)). It plays music only. It has no account of
its own and no telemetry. It serves albums heard whole, discovery, playlists in the background and
lyrics equally.

"Excellent" means, for its user:

- each song from its best copy, at the best quality its service allows, reaching the sound card
  unchanged where the platform allows it, while other programs keep playing
  ([ADR-0005](adr/0005-pipewire-at-each-tracks-own-rate.md));
- the computer and the phone as one: the same history and playlists, each controlling the other,
  at home and away ([ADR-0010](adr/0010-opium-connect-on-the-local-network.md),
  [ADR-0014](adr/0014-shared-state-merged-field-by-field.md), [ADR-0015](adr/0015-away-from-home-through-iroh.md));
- the lowest draw of power: nothing drawn at rest, battery measured on the phone;
- a library of 100,000 songs as quick as a small one, on the lowest machine too;
- honest with the services and with the user: what a licence allows, nothing more
  ([ADR-0017](adr/0017-downloads-sealed-and-tied-to-the-subscription.md)).

## 2. Principles

1. **Quality before speed.** A feature lands when it is as good as the services' own apps make it,
   or it waits.
2. **Measured, not guessed.** Speed, memory and power claims come with figures, kept in the
   repository, taken the same way before and after.
3. **Nothing drawn at rest**, on the desktop as on the phone.
4. **Other programs' sound keeps playing.** No output mode silences them, with one exception: the
   phone's bit-perfect USB output, an explicit option, off by default, that warns it silences them
   ([ADR-0034](adr/0034-what-opium-is-to-become.md)).
5. **Honest towards the services**: verified against the real service, sources cited, their DRM
   and their bot guards met, never broken.
6. **The user's data is safe**: secrets private, files written whole, downloads never lost by an
   update.
7. **One core for both apps**: what both do is written once, in Rust.
8. **No telemetry, no account of Opium's**: an outside service beyond the user's music services is
   off until the user turns it on, but for LRCLIB, and for iroh's relays (n0's, or the user's own)
   and n0's address lookup, used from the start
   ([ADR-0034](adr/0034-what-opium-is-to-become.md)).

## Quality bar

> **Decided by the owner on 2026-11-05** ([ADR-0025](adr/0025-the-quality-bar.md)), its tiers,
> accessibility, translation and library size amended on 2026-11-06
> ([ADR-0034](adr/0034-what-opium-is-to-become.md)). Taken from the method's default bars for every
> project, for interfaces and desktop applications and for mobile applications, adapted to Opium;
> the sound row comes from the bar for audio. "Today" gives what was measured on 2026-11-05 on the
> high desktop tier, or says plainly that it was not measured. The figures themselves live in
> [`measurements.md`](measurements.md). No budget is yet anchored on a figure measured from a
> reference app, Spotify's battery on the phone aside. The desktop window is frozen until the move
> to uwui, the owner's UI framework ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)):
> the rows checked "with uwui" are worked on then. The phone's Compose screens are frozen until uwui
> replaces them ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)): its
> interface rows are worked on then, on uwui.

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
| Desktop, lowest | The owner's very modest laptop, the one uwui measures first: an HP Laptop 17-ca2xxx, an AMD Athlon Silver 3050U (2 cores), its integrated Radeon (Vega), 4 GiB on one channel, 3.25 GiB left to programs; its display and storage not yet known ([ADR-0034](adr/0034-what-opium-is-to-become.md)) | Not measured |
| Desktop, high | The owner's desktop: AMD Ryzen 7 5800X (8 cores), an AMD RX 6800-class GPU (Navi 21) through Mesa's OpenGL, a 4K display at 240 Hz, scale 1.5 | 2026-11-05, [baseline](measurements.md#baseline-of-the-desktop-app-before-any-budget-2026-11-05) |
| Phone, lowest | uwui's entry-level phone, today an Android 14 phone: 4 GB of memory, a MediaTek Helio G99-class chip, a 90 Hz screen; it changes with the lowest Android, decided at Stage 3 | Not measured |
| Phone, high | The owner's Pixel on GrapheneOS, 120 Hz | Partly, 2026-11-03, before the rename and before this bar |
| Without a screen | A Raspberry Pi 5, bought when Stage 8 comes, for `opium-headless` | Not measured |

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
| Scrolling and animations | The display's rate held, no dropped frame: on the high tier 240 Hz at 4K and scale 1.5, 99th percentile ≤ 4.17 ms and no frame over 8.33 ms; on the lowest tier its display's rate, ≤ 16.7 ms at 60 Hz | A frame-time trace per animation, from uwui's frame statistics and MCP server | With uwui: unmeasurable before |
| Input to visible response | Within one frame where the work allows | The same trace, with input from the MCP server | With uwui: unmeasurable before |
| The window's thread | Never waits on a disk or the network. Today: not audited | Reading the code; a trace of the thread once instrumented | With uwui |
| Images | Decoded off the window's thread, in a cache bounded in memory that frees the GPU's. Today: one thread per cover file, unbounded ([known debt](debt.md)) | Reading the code; memory under load | With uwui |
| Rendering | Gradients dithered, colours blended correctly, text crisp and pixel-aligned at scales 1, 1.5 and 2, in each theme | Reference images through the screenshot mode | With uwui; in CI once the window renders without a display |
| The look in one place | Each theme's colours, durations, springs, radii and blur in one place, shown by a gallery of every component; the owner approves the look from captures made off screen. Today: colours and component variants in the theme files both apps read ([ADR-0011](adr/0011-themes-as-toml-files-of-component-variants.md)); motion and radii in each app's code; no gallery; the screenshot mode captures a window on screen | Captures of the gallery, or the screenshot mode until then | By the owner, with uwui |
| Accessibility | WCAG 2.2 at level AA, checked: screen readers name every control; the whole app by keyboard, every shortcut remappable; reduced motion and transparency honoured; contrast checked in each theme; uwui's high-contrast theme when the system asks. Today: no control named ([known debt](debt.md)) | Orca; keyboard only; contrast ratios | With uwui |
| Text ready for translation | Strings in one place per app, no sentence built from pieces, plurals by rule, layouts that hold longer text and right-to-left writing. Today: English only, the strings written in the code of both apps | A pseudo-locale with longer and right-to-left text | From Stage 5, English and French on the desktop at 0.1.0, on the phone at Stage 3 ([ADR-0034](adr/0034-what-opium-is-to-become.md)); right-to-left once such a language is planned |
| A library of 100,000 songs | Every budget above holds with 100,000 songs: start-up, memory, scrolling, search, sorting and filtering. Today: measured on the owner's library, about 8,500 songs ([baseline](measurements.md#baseline-of-the-desktop-app-before-any-budget-2026-11-05)) | A library generated for it: files and a fake server | With uwui; again at Stage 7 |
| HDR, wide gamut, variable refresh | Not supported, stated: the interface is drawn in sRGB, and a player's interface gains nothing from HDR; covers, animated ones included, are converted to 8-bit sRGB (`crates/motion`), so HDR or wide-gamut artwork is shown clipped; frames follow the compositor's pace, with no variable-refresh handling of the app's own | — | — |
| Sound | No underrun while playing, under load (scrolling, a build), even when the graph runs 128-frame periods at 48 kHz, the method's default: Opium asks no latency of its own, so another program can set that quantum; each track at its own rate; other programs heard. Today: not measured (it needs sound) | `pw-top`: its error count and rate | By hand, at each milestone touching playback |
| Bad input | Never a crash: parsers of untrusted data fuzzed (the services' answers, MP4 and CENC boxes, HLS playlists, file tags, what paired devices send, themes, settings); `unsafe` under Miri where it can run | `cargo fuzz` in a nightly shell, and mutation tests in `cargo test` ([ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md)); Miri | CI: the mutation tests at every push, `cargo fuzz` weekly, from M1.10 for MP4, CENC and HLS |
| Crash report | A crash leaves a report the user can attach to an issue; nothing leaves the machine without the user's consent. Today: only a hung exit leaves `exit.log` and a core dump ([ADR-0024](adr/0024-the-exit-watch-aborts-for-a-core-dump.md)); no panic hook in either app | Reading the code; a forced panic on a throwaway profile | By hand, once built |
| Iteration | A warm CI run, whose cache hit, under 20 minutes; a cold run measured and recorded, without a budget; the full build and tests budgeted once measured. Today: the `check` job 17 to 28 min cold, 7 min 21 s warm; the `android` job, beside it, 9 min 19 s cold, 6 min 4 s warm, on 2026-11-07 ([measured](measurements.md#the-android-shell-and-cis-android-job-m15-2026-11-07)); the full build and tests not measured | CI's durations; `scripts/check.sh` timed | CI, at each run |
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
| Direction | The general public; music only; one person per installation; no account of Opium's, no telemetry ([ADR-0034](adr/0034-what-opium-is-to-become.md)) |
| Platforms | Linux and Windows, macOS once uwui supports it; Android and Android Auto; a player without a screen, `opium-headless`; iOS kept possible, not planned ([ADR-0034](adr/0034-what-opium-is-to-become.md)) |
| Native or optional | The device's files and LRCLIB native; each online service, the Widevine layer and six pieces that talk beyond Opium optional; everything else native, the sound's treatments, bit-perfect USB, the guest and the notification when the song changes off by default ([ADR-0034](adr/0034-what-opium-is-to-become.md)) |
| Distribution | Flathub and Nix; a signed APK on GitHub; Flathub and Nix update it on Linux, Obtainium or the user on Android, the app itself on Windows and macOS ([ADR-0034](adr/0034-what-opium-is-to-become.md)) |
| Translation | The text ready for translation from Stage 5; English and French on the desktop at 0.1.0, on the phone at Stage 3 ([ADR-0034](adr/0034-what-opium-is-to-become.md)) |
| Licence | GPL-3.0-or-later ([ADR-0001](adr/0001-gpl-3-0-or-later.md)), kept after the code adapted from Kopuz was written again in a clean room, but for part of the browser sign-in ([ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md), [ADR-0032](adr/0032-the-gpl-kept-and-kopuzs-code-still-written-again.md), [ADR-0033](adr/0033-the-browser-sign-ins-pipe-kept-as-adapted-from-kopuz.md)) |
| Name | Opium, everywhere, no compatibility with the old names ([ADR-0023](adr/0023-the-rename-to-opium.md)) |
| Services | Apple Music, SoundCloud, Navidrome (OpenSubsonic), the device's own files ([ADR-0016](adr/0016-the-services.md)); Jellyfin and Plex at Stage 9; ListenBrainz, Last.fm, MusicBrainz, the Cover Art Archive and Discord, each off by default ([ADR-0034](adr/0034-what-opium-is-to-become.md)) |
| One library | The best copy of each song: files, server, Apple Music, SoundCloud ([ADR-0007](adr/0007-one-library-and-the-best-copy.md)) |
| Apple Music | Through its web player, after Kopuz ([ADR-0002](adr/0002-apple-music-through-its-web-player.md)) |
| Sign-in | In the user's browser, on a profile of Opium's own ([ADR-0003](adr/0003-sign-in-through-the-users-browser.md)) |
| DRM | The browser's Widevine CDM through a shim on the desktop, MediaDrm on the phone ([ADR-0004](adr/0004-the-widevine-cdm-borrowed-from-the-browser.md)); from M1.11 a thin C++ layer, the host's logic in Rust on one thread ([ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md)) |
| Desktop audio | PipeWire only, at each track's rate ([ADR-0005](adr/0005-pipewire-at-each-tracks-own-rate.md)) |
| Desktop UI | egui, until the move to uwui in uwui's first stage ([ADR-0035](adr/0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md)); GPUI abandoned ([ADR-0006](adr/0006-egui-for-the-desktop-window.md), [ADR-0022](adr/0022-gpui-tried-and-abandoned.md), [ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)) |
| Phone | Kotlin and Compose over the Rust core through UniFFI; ExoPlayer plays; Android 14 and later, arm64, the lowest Android decided again at Stage 3 ([ADR-0009](adr/0009-the-android-app-over-the-rust-core.md), [ADR-0034](adr/0034-what-opium-is-to-become.md)) |
| Themes | TOML files choosing component variants; Stock, Spotify, M3E built in, the Spotify theme renamed before a public release ([ADR-0011](adr/0011-themes-as-toml-files-of-component-variants.md), [ADR-0034](adr/0034-what-opium-is-to-become.md)) |
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
| Miri | The nightly shell of fuzzing ([ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md)), or another | When `unsafe` code is first put under it |
| CI's time | One job, or the checks split across jobs if a run exceeds the owner's limit | Stage 1 |
| What a release exposes of DRM | The release aims at the public ([ADR-0034](adr/0034-what-opium-is-to-become.md)). The owner's call: the repository's visibility, whether the CDM borrowing and the protected services ship, what becomes of the developer examples that write decrypted audio (`am fetch`, `am live`, `sc fetch`) | Stage 2 |
| The release's signing key | The owner creates the keystore; how the build reads it, kept out of the repository | Stage 2 |
| Third-party licences | `cargo-about` for the Rust crates, and the Gradle dependencies' own list, or one tool for both | Stage 2 |
| Cleartext HTTP on the phone | Only for the Navidrome host the user types (checked in the HTTP client, since Android's network security file cannot name a host chosen at run time), or only for private addresses | Stage 2 |
| Windows audio | WASAPI exclusive, or shared with `MATCH_FORMAT`, through the `wasapi` crate, weighed against principle 4 | Stage 4 |
| The name in the stores | Opium and the Spotify theme's new name, against the chosen stores' rules ([ADR-0034](adr/0034-what-opium-is-to-become.md)) | Stage 2 |
| Flathub | Whether it accepts an app that borrows the browser's CDM, and the CDM reached from Flatpak's sandbox | Stage 2 |
| The lowest Android | As low as uwui allows, aiming at Android 10 or 12 | Stage 3 |
| Users' themes | Opium's TOML, or uwui's format once it has one, the existing themes converted | Stage 5 |
| A crossfade between two rates | One track resampled for the fade, or the fade skipped, against [ADR-0005](adr/0005-pipewire-at-each-tracks-own-rate.md) | Stage 6 |
| Updates on Windows and macOS | uwui's optional crate (SlimeEngine's ADR-0199), or Opium's own | Stage 4 |
| The guest | Its trust model, and the rule that Connect hears only paired devices | Stage 8, by an ADR |
| Away from home turned off | A switch in Settings, so that n0 is reached only when wanted, or none ([known debt](debt.md)) | Stage 8 |
| Protected songs beyond the app | On UPnP renderers, against the licence's output rules; on the Raspberry Pi, a CDM on ARM Linux | Stage 8 |
| macOS | Its stage, and its CDM's host verification | When uwui supports macOS |
| What 1.0 holds | The stages it closes, and what it promises | Stage 9 |
| Extensions | On uwui's isolated WebAssembly, after the themes | After 1.0 |
| The exit watch | Kept as a safety net, or removed once the hang is found ([ADR-0021](adr/0021-the-exit-watch.md)) | When the hang is found |

## 4. Stages

The stages run in the order 1, 5, 2, 3, 6, 4, 7, 8, 9, then 10 once uwui supports macOS: the move to
uwui comes first, in uwui's first stage, after SlimeEngine's Stage 6d
([ADR-0029](adr/0029-opium-waits-for-slime-ui-then-moves-first.md),
[ADR-0035](adr/0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md)); the new functions
come after the phone: listening first, then Windows
([ADR-0034](adr/0034-what-opium-is-to-become.md)). They keep their numbers, which earlier ADRs cite.

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
- The code adapted from Kopuz written again in a clean room, but for part of the browser sign-in,
  Opium still under the GPL ([ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md),
  [ADR-0032](adr/0032-the-gpl-kept-and-kopuzs-code-still-written-again.md),
  [ADR-0033](adr/0033-the-browser-sign-ins-pipe-kept-as-adapted-from-kopuz.md))
- The figures uwui must match, measured under egui and on the Compose phone
  ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md))

**Exit.** `scripts/check.sh` and `scripts/check.sh android` pass locally and CI is green on `main`;
the owner's computer and phone run Opium under its new names with their data; the window's needs
from a UI framework are written down and reviewed (M1.6); the budgets are decided (M1.7); Opium is
left with nothing of Kopuz's but the part the owner kept (M1.13); the figures to beat are measured
(M1.14); the stage audit is recorded and its findings decided (M1.8).

### Stage 5 — On uwui

**Goal.** The desktop window drawn by uwui, the owner's UI framework, egui gone. Opium is one of the
programs uwui's first stage ports, after SlimeEngine's Stage 6d, before uwui's API is declared
stable ([ADR-0035](adr/0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md)). Stage 1 is
finished first; Opium then waits for the port, and this stage comes right after Stage 1
([ADR-0029](adr/0029-opium-waits-for-slime-ui-then-moves-first.md)).

- The interface's state and logic taken out of `app.rs` into a model free of any toolkit
- Each page and component, in each theme's variants, on the framework, with every need marked
  indispensable in [`ui-framework-needs.md`](ui-framework-needs.md)
- Agents read and drive the window through the framework; screen readers name its controls,
  WCAG 2.2 AA checked, uwui's high-contrast theme when the system asks
- The text ready for translation, in English and French
  ([ADR-0034](adr/0034-what-opium-is-to-become.md))
- Every shortcut remappable, and a single-key one can also be turned off, as WCAG asks
- What changes while the user looks changes in place, animated, the item looked at and the selection
  kept
- The budgets held with a library of 100,000 songs, generated for it
- uwui's desktop host on winit runs the program; Opium's logic depends on no part of uwui
  ([ADR-0019](adr/0019-the-workings-apart-from-the-drawing.md))
- A need missing in uwui added there, under its rules, when it serves others too; one of Opium's
  alone built in Opium; neither worked around
- Then, in the same stage of uwui's, a prototype of Opium's phone screens on uwui, in a surface the
  Kotlin app gives it, measured against the same screens in Compose in smoothness, start-up and
  power, with a Macrobenchmark module and Perfetto
  ([ADR-0027](adr/0027-macrobenchmark-for-the-phones-measurements-at-stage-3.md)). uwui is taken
  whatever it shows; the measurement lists what to fix until it matches Compose

**Exit.** Every page of the three themes on the framework, egui removed; nothing drawn at rest under
Wayland and under Xwayland; the display's rate held while scrolling and animating; start-up, frame
times while scrolling and animating, memory and power at rest at least as good as egui's figures of
M1.14, measured the same way, and held with 100,000 songs; WCAG 2.2 AA checked; every screen of the
desktop in English and French; the phone's prototype measured against Compose, and what uwui must
fix recorded.

### Stage 2 — The first release → 0.1.0

**Goal.** Opium can be handed to someone else: a Flatpak, a Nix package and a signed APK that build
reproducibly, carry their licences and expose of DRM what the owner decided.

- The owner's decision on what the release exposes of DRM, recorded as an ADR
- A release signing key for the APK, created by the owner and kept out of the repository
- The third-party licences of both apps gathered and shipped, with the bundled assets' texts
  (Geist's OFL, Lucide's ISC, the logos' own)
- A Gradle wrapper, so the APK builds with a pinned Gradle
- Cleartext HTTP on the phone limited to the Navidrome server
- Each optional piece that [ADR-0034](adr/0034-what-opium-is-to-become.md) lists made a Cargo
  feature, CI building the combinations that ship
- Every token of the desktop in the system's keyring, migrated, with a fallback where no keyring
  answers
- The fewest rights: the desktop's portals, Flatpak's sandbox, no superfluous Android permission
- A Flatpak for Flathub, the Nix package, a signed APK on GitHub; the name and the Spotify theme's
  new name checked against those stores' rules
- A spike of the browser's Widevine CDM on Windows (`VerifyCdmHost`), so that Stage 4's risk is
  known early
- French shipped beside English on the desktop; the phone, still on Compose, gets it at Stage 3
- README and CHANGELOG for 0.1.0
- The dependencies checked (`cargo deny`), once their advisories are fixed or accepted
- A release audit before the tag, recorded in [`audits/`](audits/README.md): the stage audit, plus
  the changelog, the version, signed and reproducible packages, debug symbols kept, a run on each
  platform (no earlier release to update from)

**Exit.** A tagged 0.1.0 whose Flatpak, Nix package and signed APK were installed and run from clean
profiles, the desktop in English and in French; each lists its third-party licences; Windows' CDM
spike recorded; CI is green.

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
- The phone's text in English and French
- The lowest Android decided with uwui, and the lowest phone measured

**Exit.** Every screen on uwui, as good as Compose on M1.14's figures; every page of each theme
exists on the phone; TalkBack names every control; the measurements are in the repository and no worse than at the
stage's start.

### Stage 6 — Listening → 0.3

**Goal.** Everything touched at each listening, as good as the best players
([ADR-0034](adr/0034-what-opium-is-to-become.md)).

- A richer queue: saved queues, undo, no limit of 100, besides today's play next and at the end;
  a sleep timer on the desktop, as the phone has; a shuffle that avoids repeats without
  recommendations
- Offered, off by default: loudness levelled (ReplayGain, EBU R128, by track or album, without
  clipping); a smart crossfade that spares albums without gaps; a parametric equaliser with AutoEq's
  profiles, one per headphone or output
- The signal path, shown today, naming each treatment, and proving the sound unchanged when
  nothing changes it
- Lyrics word by word from Navidrome too (its 0.63 and later), as from the other sources today,
  translated and transliterated where the service gives it, kept with downloads
- An offline mode turned on by hand; a download shows what part of a collection can be kept
- On the phone: bit-perfect USB output from Android 14, an explicit option, off by default; a
  home-screen widget, a quick-settings tile
- On the desktop: a mini-player, an icon in the tray, a notification when the song changes (off by
  default), a start with the session, shortcuts with chords, a palette of commands, the `opium`
  command; Discord, off by default
- Discovery: radios computed on the device from the history and the files, beside the services'
- The cover flying between screens, gestures that drive and interrupt animations, glass materials
  lightened on the lowest tier, haptics on the phone
- At start, Opium back where the user was: page, scroll, selection and the window's size, besides
  the song and the panels that come back today

**Exit.** Each function on both devices where it applies, in each theme; the desktop's sound
unchanged by default, `pw-top` showing no underrun with the equaliser and a crossfade running; the
signal path checked against PipeWire's own graph.

### Stage 4 — Windows → 0.4

**Goal.** Opium's desktop app on Windows, with the same functions and quality bar as on Linux,
after the listening ([ADR-0034](adr/0034-what-opium-is-to-become.md)).

- First, the result of Stage 2's spike: the browser's Widevine CDM on Windows requires its host to
  be verified (`VerifyCdmHost`), which the shim does not do; if Opium cannot verify, Apple Music and
  SoundCloud's protected streams cannot play there, and the stage is reshaped
- Audio through WASAPI (`wasapi` crate), as decided at the stage's start
- The tokens and the downloads' key in Windows' credential store (`keyring`), file windows through
  `rfd`, the media controls through SMTC, a single instance without D-Bus, the folders where Windows
  keeps them
- An installer (Inno Setup), the app updating itself, the `opium` command reaching the app without
  D-Bus, and a CI job on Windows

**Exit.** On a fresh Windows 11, the installed Opium signs in to each service, plays each kind of
song at its own rate, downloads and plays offline, pairs with the phone, and its checks pass in CI
on Windows.

### Stage 7 — The library → 0.5

**Goal.** A library of 100,000 songs kept well, and the user's data theirs
([ADR-0034](adr/0034-what-opium-is-to-become.md)).

- Smart playlists by rules, with a visual editor; statistics and a year in review, computed on the
  device; favourites of Opium's own, shared between devices
- Every play kept and shared: the shared state's format versioned and migrated, its merge still
  ignoring order and repetition ([ADR-0014](adr/0014-shared-state-merged-field-by-field.md))
- A file's old copy kept aside a while when its tags change, and the change undone on demand;
  the tags are already edited on a copy first ([`ARCHITECTURE.md`](ARCHITECTURE.md))
- MusicBrainz and the Cover Art Archive completing tags, covers and credits, off by default
- Everything exported in open formats (JSON, M3U)
- Scrobbling to ListenBrainz and Last.fm, off by default, with a queue that waits for the network;
  their suggestions in discovery

**Exit.** The budgets held at 100,000 songs on the lowest desktop; a format migration tested from
the previous release's files; an export read back.

### Stage 8 — Connect → 0.6

**Goal.** The devices as one, and a room played by a Raspberry Pi
([ADR-0034](adr/0034-what-opium-is-to-become.md)).

- Resuming and handing over, which Connect does today
  ([ADR-0010](adr/0010-opium-connect-on-the-local-network.md)), at their best: the session taken up
  on whichever device is opened next, without a gesture; a song handed over without a gap the ear
  notices
- A guest on the local network joining the queue by a code, after an ADR on its trust
- UPnP and DLNA renderers, protected songs as the stage decides
- `opium-headless`, a program on the same core without uwui, on a Raspberry Pi 5, driven by
  Connect

**Exit.** A song handed over between the computer, the phone and the Pi without a gap the ear
notices; the guest's ADR accepted and its tests passing; the Pi's budgets measured.

### Stage 9 — More services → 0.7

**Goal.** Jellyfin and Plex in the same library, at their best copy
([ADR-0034](adr/0034-what-opium-is-to-become.md)).

- Each a crate implementing `opium_core::Provider`, an optional piece, verified against the real
  server with the owner
- Their place in the best-copy order ([ADR-0007](adr/0007-one-library-and-the-best-copy.md))
- What 1.0 holds, decided

**Exit.** Both servers browsed, searched, played and downloaded, verified against real servers.

### Stage 10 — macOS (eventual)

**Goal.** Opium on macOS once uwui supports it and a Mac can check it
([ADR-0034](adr/0034-what-opium-is-to-become.md)).

- First, a spike of the CDM's host verification on macOS, as on Windows
- Audio through Core Audio at each track's rate, the app updating itself

**Exit.** As Stage 4's, on macOS.

## 5. Release map

| Version | Stage | Content |
|---|---|---|
| — | 1 | Under the method, the rename (no release) |
| — | 5 | On uwui (no release: 0.1.0 ships on it) |
| 0.1.0 | 2 | The first release |
| 0.2 | 3 | The phone on par |
| 0.3 | 6 | Listening |
| 0.4 | 4 | Windows |
| 0.5 | 7 | The library |
| 0.6 | 8 | Connect and the player without a screen |
| 0.7 | 9 | Jellyfin and Plex |
| — | 10 | macOS, when uwui supports it |
