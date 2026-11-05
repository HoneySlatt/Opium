# 0027. Macrobenchmark for the phone's measurements, at Stage 3

- Status: Accepted; amended by [0031](0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)
  (the module comes with the phone's prototype, in Stage 5)
- Date: 2026-11-05

## Context

The quality bar's phone budgets ([ADR-0025](0025-the-quality-bar.md)) are cold start, frame times
at the display's rate and battery while playing. The method's default for mobile applications
measures them with Macrobenchmark and Perfetto. Today they are taken by hand over `adb`
(`am start -W`, `dumpsys gfxinfo`, CPU ticks from `/proc/<pid>/stat`), on the owner's phone, without
a script that repeats them the same way ([`measurements.md`](../measurements.md)).

## Decision

**A Gradle benchmark module using Jetpack Macrobenchmark**, Google's, under the Apache License 2.0
(checked in the library's published POM), measures cold start, frame times and battery, with
Perfetto traces. It is a development dependency: a module of its own, never in the APK.

**It is added at the ROADMAP's Stage 3**, the phone on par, where the phone's budgets are worked
on. Until then, the phone is measured by hand with `adb` and Perfetto.

## Alternatives considered

- **`adb` and Perfetto only**: no new dependency, but the runs are neither repeatable nor scripted,
  and their spread is not known.

## Consequences

- A new dependency of the Gradle project, outside the shipped app; its licence fits the GPL-3.0.
- The phone's budgets get repeatable runs from Stage 3; before it, their figures carry the
  limits of measurements by hand.
- The benchmark runs on a device, so it is checked by hand, not in CI.
