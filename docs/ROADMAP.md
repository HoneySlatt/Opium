# Roadmap

> **Proposal, not yet decided by the owner** (drafted on 2026-11-04, when the project came under
> its method). Living document. It defines direction, ordering and exit criteria. It is not a task
> list: the current plan lives in `docs/PLAN.md`, and decisions live in `docs/adr/`. Stages will be
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

## 3. Decisions

### Fixed

| Topic | Decision |
|---|---|
| Licence | GPL-3.0-or-later ([ADR-0001](adr/0001-gpl-3-0-or-later.md)) |
| Name | Opium, everywhere, no compatibility with the old names ([ADR-0023](adr/0023-the-rename-to-opium.md)) |
| Services | Apple Music, SoundCloud, Navidrome (OpenSubsonic), the device's own files ([ADR-0016](adr/0016-the-services.md)) |
| One library | The best copy of each song: files, server, Apple Music, SoundCloud ([ADR-0007](adr/0007-one-library-and-the-best-copy.md)) |
| Apple Music | Through its web player, after Kopuz ([ADR-0002](adr/0002-apple-music-through-its-web-player.md)) |
| Sign-in | In the user's browser, on a profile of Opium's own ([ADR-0003](adr/0003-sign-in-through-the-users-browser.md)) |
| DRM | The browser's Widevine CDM through a shim on the desktop, MediaDrm on the phone ([ADR-0004](adr/0004-the-widevine-cdm-borrowed-from-the-browser.md)) |
| Desktop audio | PipeWire only, at each track's rate ([ADR-0005](adr/0005-pipewire-at-each-tracks-own-rate.md)) |
| Desktop UI | egui, until SlimeEngine's UI framework runs applications; GPUI abandoned ([ADR-0006](adr/0006-egui-for-the-desktop-window.md), [ADR-0022](adr/0022-gpui-tried-and-abandoned.md)) |
| Phone | Kotlin and Compose over the Rust core through UniFFI; ExoPlayer plays; Android 14 and later, arm64 ([ADR-0009](adr/0009-the-android-app-over-the-rust-core.md)) |
| Themes | TOML files choosing component variants; Stock, Spotify, M3E built in ([ADR-0011](adr/0011-themes-as-toml-files-of-component-variants.md)) |
| Devices | Opium Connect: mDNS, TCP 47820, SPAKE2, Noise; iroh away from home; state merged field by field ([ADR-0010](adr/0010-opium-connect-on-the-local-network.md), [ADR-0014](adr/0014-shared-state-merged-field-by-field.md), [ADR-0015](adr/0015-away-from-home-through-iroh.md)) |
| Offline | Sealed downloads, DRM songs tied to a subscription confirmed in 30 days ([ADR-0017](adr/0017-downloads-sealed-and-tied-to-the-subscription.md)) |
| Lyrics | The song's own source, then LRCLIB, other sites opt-in ([ADR-0008](adr/0008-the-lyrics-sources.md)) |

### Open

Each is decided by an ADR at the stage where it becomes necessary, not earlier.

| Topic | Candidates | Decide at |
|---|---|---|
| Links checked in CI | lychee added to the dev shell, or none | Stage 1 |
| CI's time | One job, or the checks split across jobs if a run exceeds the owner's limit | Stage 1 |
| What a release exposes of DRM | The owner's call: the repository's visibility, whether the CDM borrowing and the protected services ship, what becomes of the developer examples that write decrypted audio (`am fetch`, `am live`, `sc fetch`) | Stage 2 |
| The release's signing key | The owner creates the keystore; how the build reads it, kept out of the repository | Stage 2 |
| Third-party licences | `cargo-about` for the Rust crates, and the Gradle dependencies' own list, or one tool for both | Stage 2 |
| Cleartext HTTP on the phone | Only for the Navidrome host the user types (checked in the HTTP client, since Android's network security file cannot name a host chosen at run time), or only for private addresses | Stage 2 |
| Windows before or after the move to SlimeEngine's UI framework | Port the egui window, or wait and port once | End of Stage 3 |
| Windows audio | WASAPI exclusive, or shared with `MATCH_FORMAT`, through the `wasapi` crate, weighed against principle 4 | Stage 4 |
| The exit watch | Kept as a safety net, or removed once the hang is found ([ADR-0021](adr/0021-the-exit-watch.md)) | When the hang is found |

## 4. Stages

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

**Exit.** `scripts/check.sh` and `scripts/check.sh android` pass locally and CI is green on `main`;
the owner's computer and phone run Opium under its new names with their data.

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

**Exit.** A tagged 0.1.0 whose Linux package (`nix build`) and signed APK were installed and run
from clean profiles; both list their third-party licences; CI is green.

### Stage 3 — The phone on par

**Goal.** The phone does what the computer does, as smoothly and as frugally as measured, and its
known debt is cleared or recorded.

- The phone's M3E completed where its design has parts the phone lacks
- The Android debt left by the 2026-10-31 audit decided one by one: Opium Connect's node built on
  the main thread, recompositions, accessibility labels, touch targets
- Battery and smoothness measured against the reference player, before and after each change

**Exit.** Every page of each theme exists on the phone; TalkBack names every control; the
measurements are in the repository and no worse than at the stage's start.

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

### Stage 5 — On SlimeEngine's UI framework

**Goal.** The desktop window drawn by SlimeEngine's UI framework, egui gone, once that framework
runs applications (its Stage 6d) and after Slatty, its pilot, has moved
([ADR-0022](adr/0022-gpui-tried-and-abandoned.md)).

- The interface's state and logic taken out of `app.rs` into a model free of any toolkit
- Each page and component, in each theme's variants, on the framework
- Agents read and drive the window through the framework; screen readers name its controls

**Exit.** Every page of the three themes on the framework, egui removed; nothing drawn at rest under
Wayland and under Xwayland; the display's rate held while scrolling and animating; memory at start
and power at rest no worse than egui's, measured the same way.

## 5. Release map

| Version | Stage | Content |
|---|---|---|
| — | 1 | Under the method, the rename (no release) |
| 0.1.0 | 2 | The first release |
| 0.2 | 3 | The phone on par |
| 0.3 | 4 | Windows |
| 0.4 | 5 | On SlimeEngine's UI framework |
