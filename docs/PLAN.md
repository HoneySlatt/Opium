# Plan

This file says what is being worked on now: the milestones of the current stage, their status, how
each one is checked; the known debt is in [`debt.md`](debt.md). Direction beyond this stage is in
[`ROADMAP.md`](ROADMAP.md); decisions are in [`adr/`](adr/README.md). A *stage* is a step of the
roadmap; a *milestone* is a small, verifiable step of a stage.

Every milestone ends with the definition of done in [`AGENTS.md`](../AGENTS.md#definition-of-done).

## Current

- **Stage 1 — Under the method is done** (2026-11-12): every milestone, its audit and the audit's
  fixes, in [the history](history.md#stage-1--under-the-method-done-on-2026-11-12).
- **Opium waits for its port in uwui's first stage**, after SlimeEngine's Stage 6d, fixing only a
  loss of data, a crash, a security flaw or a service that stops working
  ([ADR-0035](adr/0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md)), but for the
  milestone below ([ADR-0048](adr/0048-formats-and-connects-versions-before-the-port.md)).
- **Now: [Formats and versions](#formats-and-versions)**, not started.
- **Then Stage 5**, the move to uwui
  ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)), planned when it
  starts, and the stages after it in the [ROADMAP's order](ROADMAP.md#4-stages).
- **Still due with the owner**: a SoundCloud song served encrypted, not yet played after the
  clean-room rewrite.
- **The [architecture audit of 2026-11-16](audits/2026-11-16-architecture.md#decisions-for-the-owner)**
  waits for the owner's decisions, one at a time; the first, formats and Connect's versions, was
  taken on 2026-11-16. Next: the desktop's app id, errors and the log, the terms for contributions,
  one shared layer before Stage 5, the quality bar's scenarios, `AGENTS.md` aligned with the
  method.

## Formats and versions

Not started. Decided on 2026-11-16 ([ADR-0048](adr/0048-formats-and-connects-versions-before-the-port.md)),
from the [architecture audit](audits/2026-11-16-architecture.md#before-the-first-release) (A1 to A3).

- **Goal.** Every file Opium keeps reads its older versions and is never lost to a newer one; two
  devices one version apart still pair and sync, and the older knows it should update; a service
  an older device does not know passes through it unchanged.
- **Delivers.**
  - First, the conventions, put to the owner one at a time: how a file names its version; what a
    build does with a file newer than it knows; whether a cache of another version is thrown away
    and rebuilt rather than migrated; how two devices agree on Connect's version; how an unknown
    service is kept; whether the frozen windows say "update this device" or only the core reports
    it until uwui.
  - The list of every file both apps keep, with its owner and its version (the desktop's files,
    the phone's preferences and theme), in `ARCHITECTURE.md`.
  - A version in each, a migration per step, a backup before a migration writes.
  - Connect's hello with a range of versions, the common highest used.
  - `ProviderKind` and the shared state keeping a service they do not know.
- **Done when.** Each format is read at each of its versions by a test on files written by
  today's build, kept in the tests; a test of two wire versions pairs and syncs; a message naming
  an unknown service passes an older device's merge unchanged; a downgrade leaves every file
  readable again by the newer build.
- **How to test.** `scripts/check.sh` and `scripts/check.sh android`; the desktop on a throwaway
  profile carrying files from today's build; the audit's scenarios "a phone one version behind",
  "an upgrade, then a downgrade" and "a service added".
- **Measured.** The start-up to the first useful frame, before and after: a version check must not
  slow it.
- **By hand, with the owner.** The new desktop and APK installed together on the owner's devices;
  history and playlists checked on both.

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
  phone comes with the phone's prototype, in Stage 5 ([ADR-0027](adr/0027-macrobenchmark-for-the-phones-measurements-at-stage-3.md), [ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)).

## Known debt

In [`debt.md`](debt.md).

Finished stages and milestones move to [`history.md`](history.md), so that this file holds the
current stage only; `scripts/check.sh` fails if this file passes 2,000 words.
