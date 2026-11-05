# 0028. The phone's Compose screens are frozen until slime-ui is measured on Android

- Status: Accepted
- Date: 2026-11-05

## Context

The Android app draws its screens with Jetpack Compose (`android/app/src/main/java/…/opium/ui/`)
over a system layer in Kotlin (`playback/`, `data/`, `core/`) and the Rust core
([ADR-0009](0009-the-android-app-over-the-rust-core.md)). SlimeEngine's ADR-0173 plans, after its
Stage 6d, a prototype of this player's phone screens on slime-ui, SlimeEngine's UI framework,
measured against the same screens in Compose in smoothness, start-up and power: Android gets the
framework only if it matches or beats Compose. Work on the Compose screens before that measurement
could be thrown away. The desktop window is frozen for the same reason
([ADR-0026](0026-the-egui-window-frozen-until-the-move.md)).

## Decision

**The Compose screens are frozen until the measurement decides the phone's toolkit.** They change
only to fix a loss of data, a crash or a security flaw.

**The Android system layer is not frozen**: playback (ExoPlayer, MediaDrm and Widevine, the audio
chip's offload), the MediaSession and its notification, Android Auto, the foreground services,
downloads, and the battery work. A Rust interface would call these Android services too, so they
survive either way.

**Stage 3 starts with the measurement** (ROADMAP), then brings the phone on par on the toolkit that
wins. The M3E screens and the TalkBack labels wait for it. Macrobenchmark
([ADR-0027](0027-macrobenchmark-for-the-phones-measurements-at-stage-3.md)) is how the measurement
and the phone's budgets are taken.

## Alternatives considered

- **Working on the Compose screens meanwhile**: the phone improves sooner, but the work may be
  redone on slime-ui.
- **Freezing the whole Android app**: the system layer, which survives either toolkit, would stop
  too, and the battery work with it.

## Consequences

- The phone's interface debt waits: recompositions, accessibility labels, touch targets, the M3E
  screens.
- The battery work, playback and Opium Connect on the phone go on.
- If Compose wins the measurement, the screens thaw, and Stage 3 goes on in Compose.
