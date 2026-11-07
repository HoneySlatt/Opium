# Architecture decision records

An ADR records one decision: its context, the decision, the alternatives rejected and the
consequences. Once accepted, only its status changes. A later decision that replaces it whole makes
it `Superseded by NNNN`; one that changes part of it makes it `Accepted; amended by NNNN`, and the
newer one `Accepted; amends NNNN (what)`.

New ADRs copy [`0000-template.md`](0000-template.md) and take the next free number.

ADRs 0001 to 0023 were written on 2026-11-04, when the project came under this method: they record
decisions already taken, each dated from the history (`git log`) and checked against the code. Where
an ADR rests on the owner's notes rather than on the history, it says so. Commits before the rename
call the app "Music" ([ADR-0023](0023-the-rename-to-opium.md)).

| ADR | Title | Status |
|---|---|---|
| [0001](0001-gpl-3-0-or-later.md) | GPL-3.0-or-later | Accepted; confirmed by 0032 |
| [0002](0002-apple-music-through-its-web-player.md) | Apple Music through its web player, as Kopuz does | Accepted |
| [0003](0003-sign-in-through-the-users-browser.md) | Signing in through the user's browser, on a profile of Opium's own | Accepted |
| [0004](0004-the-widevine-cdm-borrowed-from-the-browser.md) | The Widevine CDM borrowed from the user's browser, through a C++ shim | Accepted; amended by 0030 |
| [0005](0005-pipewire-at-each-tracks-own-rate.md) | PipeWire as the only output, at each track's own rate | Accepted |
| [0006](0006-egui-for-the-desktop-window.md) | egui for the desktop window | Accepted; amended by 0022 and 0026 |
| [0007](0007-one-library-and-the-best-copy.md) | One library, and the best copy of each song | Accepted |
| [0008](0008-the-lyrics-sources.md) | Lyrics: the song's own source first, then the sites, LRCLIB alone by default | Accepted; amended by 0034 |
| [0009](0009-the-android-app-over-the-rust-core.md) | The Android app: Kotlin and Compose over the Rust core, through UniFFI | Accepted; amended by 0034 |
| [0010](0010-opium-connect-on-the-local-network.md) | Opium Connect on the local network: mDNS, TCP 47820, SPAKE2 and Noise | Accepted; amended by 0015 and 0034 |
| [0011](0011-themes-as-toml-files-of-component-variants.md) | Themes as TOML files that choose a variant of each component | Accepted; amended by 0034 |
| [0012](0012-soundcloud-changes-through-a-browser-seam.md) | SoundCloud's changes go through a browser seam | Accepted |
| [0013](0013-secrets-and-what-is-kept-on-disk.md) | Secrets, and what is kept on disk | Accepted; amended by 0034 |
| [0014](0014-shared-state-merged-field-by-field.md) | What the devices share, merged field by field | Accepted; amended by 0034 |
| [0015](0015-away-from-home-through-iroh.md) | Away from home, paired devices meet through iroh | Accepted; amends 0010 (away from home) |
| [0016](0016-the-services.md) | The services: Apple Music, SoundCloud, Navidrome and the device's own files | Accepted; amended by 0034 |
| [0017](0017-downloads-sealed-and-tied-to-the-subscription.md) | Downloads sealed with the app's key, and tied to the subscription | Accepted; amended by 0034 |
| [0018](0018-one-instance-through-the-session-bus.md) | One instance, found through its MPRIS name on the session bus | Accepted |
| [0019](0019-the-workings-apart-from-the-drawing.md) | The desktop's workings apart from its drawing, waking it through `Wake` | Accepted |
| [0020](0020-d-bus-over-async-io.md) | D-Bus spoken over async-io, not tokio | Accepted |
| [0021](0021-the-exit-watch.md) | The exit watch | Accepted; amended by 0024 |
| [0022](0022-gpui-tried-and-abandoned.md) | GPUI tried and abandoned; egui stays until SlimeEngine's UI framework runs applications | Accepted; amends 0006 (until the move); amended by 0026 and 0031 |
| [0023](0023-the-rename-to-opium.md) | The app is named Opium, everywhere, with no compatibility kept | Accepted |
| [0024](0024-the-exit-watch-aborts-for-a-core-dump.md) | The exit watch aborts, for a core dump | Accepted; amends 0021 |
| [0025](0025-the-quality-bar.md) | The quality bar: tiers and budgets | Accepted; amended by 0034 |
| [0026](0026-the-egui-window-frozen-until-the-move.md) | The egui window is frozen until the move to SlimeEngine's UI framework | Accepted; amends 0006 and 0022; amended by 0029 and 0031 |
| [0027](0027-macrobenchmark-for-the-phones-measurements-at-stage-3.md) | Macrobenchmark for the phone's measurements, at Stage 3 | Accepted; amended by 0031 |
| [0028](0028-the-phones-compose-screens-frozen-until-measured.md) | The phone's Compose screens are frozen until slime-ui is measured on Android | Accepted; amended by 0031 |
| [0029](0029-opium-waits-for-slime-ui-then-moves-first.md) | Opium waits for slime-ui, then moves to it before anything else | Accepted; amends 0026 (no work meanwhile) and the ROADMAP's order; amended by 0030, 0031 and 0034 |
| [0030](0030-the-kopuz-code-rewritten-in-a-clean-room.md) | The code adapted from Kopuz is written again in a clean room | Accepted; amends 0004 (the host's shape) and 0029 (the standby lifted for these milestones); amended by 0032 and 0033 |
| [0031](0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md) | Opium moves to uwui during SlimeEngine's Stage 6d, on the phone too | Accepted; amends 0022, 0026, 0027, 0028 and 0029; amended by 0032, 0034 and 0035 |
| [0032](0032-the-gpl-kept-and-kopuzs-code-still-written-again.md) | The GPL is kept, and Kopuz's code is still written again | Accepted; amends 0030 and 0031, confirms 0001; amended by 0033 |
| [0033](0033-the-browser-sign-ins-pipe-kept-as-adapted-from-kopuz.md) | The browser sign-in's pipe is kept as adapted from Kopuz | Accepted; amends 0030 and 0032 |
| [0034](0034-what-opium-is-to-become.md) | What Opium is to become | Accepted; amends 0008, 0009, 0010, 0011, 0013, 0014, 0016, 0017, 0025, 0029 and 0031 |
| [0035](0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md) | The port in uwui's first stage, Stage 1 finished first | Accepted; amends 0031 |
| [0036](0036-the-phones-ci-job-blocks-even-when-google-fails.md) | The phone's CI job blocks, even when Google fails | Accepted |
