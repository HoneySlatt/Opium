# Plan

This file says what is being worked on now: the milestones of the current stage, their status, how
each one is checked; the known debt is in [`debt.md`](debt.md). Direction beyond this stage is in
[`ROADMAP.md`](ROADMAP.md); decisions are in [`adr/`](adr/README.md). A *stage* is a step of the
roadmap; a *milestone* is a small, verifiable step of a stage.

Every milestone ends with the definition of done in [`AGENTS.md`](../AGENTS.md#definition-of-done).

## Current

- **Stage 1 — Under the method is done** (2026-11-12): every milestone, its audit and the audit's
  fixes, in [the history](history.md#stage-1--under-the-method-done-on-2026-11-12).
- **Opium waits for its port in uwui's first stage**, after SlimeEngine's Stage 6d
  ([ADR-0035](adr/0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md)). Meanwhile it
  fixes a loss of data, a crash, a security flaw or a service that stops working, and builds the
  three milestones below, in their order, as the owner decided on 2026-11-16 after the
  [architecture audit](audits/2026-11-16-architecture.md#decisions-for-the-owner)
  ([ADR-0048](adr/0048-formats-and-connects-versions-before-the-port.md) for the second; the first
  writes the rule whole).
- **Now: [Documentation under the method's new rules](#documentation-under-the-methods-new-rules)**,
  not started; then [Formats and versions](#formats-and-versions), then
  [Errors and the log](#errors-and-the-log).
- **Then Stage 5**, the move to uwui
  ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)), opened by a design
  session for one shared layer of state and actions ([ROADMAP](ROADMAP.md#stage-5--on-uwui)), and
  the stages after it in the [ROADMAP's order](ROADMAP.md#4-stages).
- **Still due with the owner**: a SoundCloud song served encrypted, not yet played after the
  clean-room rewrite.

## Documentation under the method's new rules

Not started. No code of the app changes.

- **Goal.** Each topic is read in one ADR; the documents follow the method's rules of 2026-11-16; a
  stranger finds how to report a flaw in private and how to file an issue.
- **Delivers.**
  - Each chain of ADRs that amend one another (38 ADRs today) replaced by one ADR per topic that
    supersedes the chain and states its rule whole, nothing added nor dropped. The ADR on the port
    states the work done while Opium waits: these three milestones, and why
    ([ADR-0048](adr/0048-formats-and-connects-versions-before-the-port.md)).
  - From the method's templates: the rules of `docs/adr/README.md` and `scripts/adr-index.py`,
    which writes its index; `changelog.d/` and `scripts/changelog.py`; both checked by
    `scripts/check.sh`.
  - `ARCHITECTURE.md` opened by a page that a newcomer reads first.
  - `AGENTS.md` aligned with the method's template: the rule on abstractions, the definition of
    done checking the affected architecture contracts and scenarios, ADRs superseded rather than
    amended, `changelog.d/`. It stays under 1,500 words by dropping the lines that repeat the
    owner's global rules: files added by their paths, no stash or reset over another's work, no
    commit that fails the checks, everything in English.
  - `SECURITY.md`, through GitHub's private reporting of vulnerabilities, turned on in the
    repository's settings; templates for issues and pull requests. The code of conduct and SPDX
    headers wait for Stage 2.
  - The measuring scripts on Niri: `startup.py map` reads Niri's event stream, `CONTRIBUTING.md`
    names Niri; `rest.sh`, `sample.py`, `startup.py` and `memwatch.py` record the load of other
    programs before and while they run ([debt](debt.md#not-waiting-for-a-stage)).
- **Done when.** `scripts/check.sh` passes with its new steps; no accepted ADR is amended by
  another; a fresh reviewer has read each new ADR against the chain it replaces and found nothing
  lost or added.
- **How to test.** `scripts/check.sh`; `startup.py map` run once in a headless Niri or on the
  owner's desktop with the owner's go.
- **Measured.** Nothing but the documents' sizes.

## Formats and versions

Not started. Decided on 2026-11-16 ([ADR-0048](adr/0048-formats-and-connects-versions-before-the-port.md)),
from the [architecture audit](audits/2026-11-16-architecture.md#before-the-first-release) (A1 to A3).

- **Goal.** Every file Opium keeps reads its older versions and is never lost to a newer build; two
  devices one version apart still pair and sync, and the older knows it should update; a service
  an older device does not know passes through it unchanged.
- **Decided by the owner on 2026-11-16.**
  - Each kept file carries its own version.
  - A build that finds a profile written by a newer one refuses it and writes nothing, as Firefox
    does; going back means restoring a backup.
  - A cache of another version is thrown away and rebuilt, never migrated.
  - "This profile comes from a newer Opium" and "this other device must be updated" are reported by
    the core only, to the log of [Errors and the log](#errors-and-the-log) and to the terminal: the
    frozen windows show nothing until uwui.
- **Delivers.**
  - The list of every file both apps keep, with its owner and its version (the desktop's files,
    the phone's preferences and theme), in `ARCHITECTURE.md`.
  - A version in each, a migration per step, a backup before a migration writes, the refusal of a
    newer profile.
  - Connect's hello with a range of versions, the common highest used.
  - `ProviderKind` and the shared state keeping a service they do not know.
  - An ADR for the conventions a newcomer would question.
- **Done when.** Each format is read at each of its versions by a test on files written by
  today's build, kept in the tests; a test of two wire versions pairs and syncs; a message naming
  an unknown service passes an older device's merge unchanged; an older build given a newer profile
  refuses it and leaves every file as it was.
- **How to test.** `scripts/check.sh` and `scripts/check.sh android`; the desktop on a throwaway
  profile carrying files from today's build; the ROADMAP's
  [scenarios](ROADMAP.md#architecture-and-stability-scenarios) "a phone one version behind", "an
  upgrade, then a downgrade" and "a service added".
- **Measured.** The start-up to the first useful frame, before and after: a version check must not
  slow it.
- **By hand, with the owner.** The new desktop and APK installed together on the owner's devices;
  history and playlists checked on both.

## Errors and the log

Not started. Decided on 2026-11-16, from the
[architecture audit](audits/2026-11-16-architecture.md#before-the-first-release) (A5, A6).

- **Goal.** The interface can tell an error's kind, so that it can be translated with uwui; one log
  a user can attach to an issue, secrets stripped in one place.
- **Delivers.**
  - Kinds in `opium_core::provider::Error` and the phone's `OpiumError`: offline, a session that
    lapsed, a subscription that ended, a song not available in the user's country, a service that
    limits requests, an answer not understood, and those the services' code shows are needed. Each
    keeps its English text, so the frozen windows show what they show today.
  - `tracing` and `tracing-subscriber`, the owner's go given on 2026-11-16: both are already in
    `Cargo.lock` (`tracing` through iroh, zbus and winit; `tracing-subscriber` only through `loom`,
    for tests), and become the app's own. A writer of Opium's keeps two files of bounded size in
    the state folder, on both apps, and strips secrets before writing. The `eprintln!` calls move
    to it.
  - An ADR for the error kinds and the log.
- **Done when.** Each service maps its failures to the kinds, tested; a test shows that a token, a
  password or a signed address never reaches the log file; a flood of messages leaves the log
  within its bound.
- **How to test.** `scripts/check.sh` and `scripts/check.sh android`; the desktop on a throwaway
  profile, offline and online.
- **Measured.** The wake-ups at rest and the start-up, before and after: the log must add neither.

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
