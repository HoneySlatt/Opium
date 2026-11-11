# Plan

This file says what is being worked on now: the milestones of the current stage, their status, how
each one is checked; the known debt is in [`debt.md`](debt.md). Direction beyond this stage is in
[`ROADMAP.md`](ROADMAP.md); decisions are in [`adr/`](adr/README.md). A *stage* is a step of the
roadmap; a *milestone* is a small, verifiable step of a stage.

Every milestone ends with the definition of done in [`AGENTS.md`](../AGENTS.md#definition-of-done).

## Current

- Stage 1 — Under the method: M1.15, the stage audit's fixes, is next; every other milestone is
  done, the audit (M1.8) recorded and decided ([audit](audits/2026-11-10-stage-1.md)).
- After it, Opium waits for its port in uwui's first stage, after SlimeEngine's Stage 6d, fixing
  only a loss of data, a crash, a security flaw or a service that stops working
  ([ADR-0035](adr/0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md)).
- Then Stage 5, the move to uwui
  ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)), and the stages after
  it in the [ROADMAP's order](ROADMAP.md#4-stages).
- Still due with the owner: a SoundCloud song served encrypted, not yet played after the clean-room
  rewrite.

## Status

| Milestone | Goal | Status |
|---|---|---|
| [M1.1](history.md#m11--the-projects-memory-in-the-repository) | A session arriving in the repository finds the rules, the decisions, the direction and the plan | Done |
| [M1.2](history.md#m12--the-checks-green) | `scripts/check.sh` passes on `main` | Done |
| [M1.3](history.md#m13--ci-green) | Every push to `main` and every pull request is checked by CI | Done |
| [M1.4](history.md#m14--the-rename-landed) | The owner's computer and phone run Opium under its new names, with their data | Done |
| [M1.5](history.md#m15--the-phones-core-checked) | The Android shell builds from an empty store and the phone's core is checked | Done |
| [M1.6](history.md#m16--what-the-window-needs-from-a-ui-framework) | SlimeEngine's Stage 6d can read, in this repository, what Opium's window needs | Done |
| [M1.7](history.md#m17--the-quality-bar-measured) | The owner decides Opium's budgets with today's figures in hand | Done |
| [M1.8](history.md#m18--the-stage-audit) | The whole project examined before Stage 1 closes | Done |
| [M1.9](history.md#m19--what-stands-between-opium-and-a-permissive-licence) | What stops MIT OR Apache-2.0 is known, and the code to write again is specified | Done |
| [M1.10](history.md#m110-to-m112--kopuzs-code-written-again) | `cenc` written again in a clean room | Done |
| [M1.11](history.md#m110-to-m112--kopuzs-code-written-again) | `widevine` written again in a clean room | Done |
| [M1.12](history.md#m110-to-m112--kopuzs-code-written-again) | `apple-music`'s derived items written again in a clean room | Done |
| [M1.13](history.md#m113--nothing-of-kopuzs-left) | Nothing of Kopuz's left, the GPL kept | Done, one part kept |
| [M1.14](history.md#m114--the-figures-to-beat-on-uwui) | The figures uwui must match: the egui window's and the Compose phone's | Done |
| [M1.15](#m115--the-audits-fixes) | No loss of data, flaw or broken service the audit found is left | In review; the owner's checks by hand open |

## Stage 1 — Under the method

Opium came under the owner's project method on 2026-11-04: its memory moves from the owner's notes
into the repository, its checks become one command that CI runs, the rename decided the same day
([ADR-0023](adr/0023-the-rename-to-opium.md)) lands, and its quality bar is measured. No feature is
built in this stage.

### M1.15 — The audit's fixes

- **Goal.** Stage 1 closes with no loss of data, flaw, crash or broken service left of those the
  [stage 1 audit](audits/2026-11-10-stage-1.md) found, as
  [ADR-0035](adr/0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md) allows before uwui.
- **Delivers.** What the [stage 1 audit](audits/2026-11-10-stage-1.md#the-owners-decisions) asked of M1.15: the losses of data L1 to
  L13 (L6 following [ADR-0017](adr/0017-downloads-sealed-and-tied-to-the-subscription.md), L11 for
  the settings); the flaws S1 ([ADR-0039](adr/0039-the-exit-watch-leaves-no-core-dump-unless-asked.md)),
  S2 to S5, S7, S8 (`cargo update -p zstd -p zstd-safe -p async-executor`) and S9 (an ADR amending
  ADR-0013); the crashes and broken services W1 to W8, the LRC overflow, the phone's caches and the
  scan's covers, and W12; the small items and the documents the audit found wrong; the nineteen
  tests the audit wrote for gaps. `app-desktop` is touched only for a loss of data, a crash or a
  flaw ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)).
- **Done when.** Each finding's test, from the
  [audit's proofs](audits/2026-11-10-stage-1/README.md), fails before its fix and passes after;
  `scripts/check.sh` and CI are green; the app runs on a throwaway profile; the debt and the
  audit's "Fixed" are updated; Stage 1 moves to [`history.md`](history.md).
- **Measured.** Rest and the window's frames, not made worse.
- **By hand.** The owner checks a release APK (the vault, the TLS verifier) and, on the owner's
  account, SoundCloud's changes after S3 and S8.
- **Where it stands (2026-11-11).** Every finding fixed and pushed, CI green, the figures in
  [`measurements.md`](measurements.md#the-audits-fixes-m115-2026-11-11), [the history](history.md#m115--the-audits-fixes)
  written. Next: the closing review; the owner's checks of the release APK, installed, and of
  SoundCloud's changes; then Stage 1 moves to the history.

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
