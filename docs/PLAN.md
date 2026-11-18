# Plan

This file says what is being worked on now: the milestones of the current stage, their status, how
each one is checked; the known debt is in [`debt.md`](debt.md). Direction beyond this stage is in
[`ROADMAP.md`](ROADMAP.md); decisions are in [`adr/`](adr/README.md). A *stage* is a step of the
roadmap; a *milestone* is a small, verifiable step of a stage.

Every milestone ends with the definition of done in [`AGENTS.md`](../AGENTS.md#definition-of-done).

## Current

- **Stage 1 — Under the method is done** (2026-11-12): every milestone, its audit and the audit's
  fixes, in [the history](history.md#stage-1--under-the-method-done-on-2026-11-12).
- **Opium waits for its port in uwui's first stage**, after SlimeEngine's Stage 6d. Meanwhile it
  fixes a loss of data, a crash, a security flaw or a service that stops working, and builds the
  eight milestones of [ADR-0051](adr/0051-what-opium-builds-while-it-waits-for-uwui.md) in their
  order, those left below, as the owner decided on 2026-11-17 after the
  [architecture audit](audits/2026-11-17-architecture.md#decisions-for-the-owner).
- **Done: Kotlin's CI** (2026-11-18), in [the history](history.md#kotlins-ci-done-on-2026-11-18).
- **Now: [Final sign-outs](#final-sign-outs)**, not started.
- **Then Stage 5**, the move to uwui, from the design of the last milestone
  ([ROADMAP](ROADMAP.md#stage-5--on-uwui)), and the stages after it in the
  [ROADMAP's order](ROADMAP.md#4-stages).
- **Still due with the owner**: a SoundCloud song served encrypted, not yet played after the
  clean-room rewrite.

Each milestone below starts by reading its findings in the
[audit](audits/2026-11-17-architecture.md); its conventions a newcomer would question are put to the
owner at its start, then written in an ADR.

## Final sign-outs

Not started. Audit SEC-1 to SEC-5.

- **Goal.** Nothing started before a sign-out takes effect after it, on both apps: a connect, a
  sign-in, a token, credential or cache write, a reply, a subscription confirmation.
- **Delivers.**
  - One sign-out generation per service in `services`, used by `desktop::backend` and `opium-ffi`,
    checked where a reply is sent, under the lock of each write, and before a service is installed;
    Kotlin's `attempt` and the vault's writes follow it; the phone's library loads of a service
    called off when it is forgotten.
  - A device forgotten unpairs here the devices it introduced
    ([ADR-0053](adr/0053-a-forgotten-device-forgotten-everywhere.md)).
  - Apple's licences paced in `AppleMusic::license`, the warm stopped before the licence, a
    superseded fetch called off ([ADR-0052](adr/0052-apples-licences-paced-in-one-place.md)).
- **Done when.** Tests with a fake service held during a sign-out, on both apps: no reply, no file,
  no token, no confirmation after it; a fake Apple counts two licences at most over ten skips.
- **How to test.** `scripts/check.sh`, `scripts/check.sh android` and the Android job; the desktop on
  a throwaway profile against a local Navidrome.
- **By hand, with the owner.** A sign-out and a sign-in again on each app, on the owner's accounts.

## Documentation under the method's new rules

Not started. No code of the app changes.

- **Goal.** Each topic is read in one ADR; the documents follow the method's rules of 2026-11-16; a
  stranger finds how to start, how to report a flaw in private and how to file an issue.
- **Delivers.**
  - Each chain of ADRs that amend one another replaced by one ADR per topic that supersedes the
    chain and states its rule whole, nothing added nor dropped.
  - From the method's templates: the rules of `docs/adr/README.md` and `scripts/adr-index.py`;
    `changelog.d/` and `scripts/changelog.py`; both checked by `scripts/check.sh`.
  - `ARCHITECTURE.md` opened by a page a newcomer reads first; its ownership, limits and drift
    corrected (audit CTB-7, DON-11, ARC-5).
  - `AGENTS.md` aligned with the method's template, with room under 1,500 words (audit CTB-4).
  - `SECURITY.md`, through GitHub's private reporting, turned on in the repository's settings;
    templates for issues and pull requests.
  - `CONTRIBUTING.md`: "private" corrected (audit SEC-7); a short section for an outside contributor:
    clone, `nix develop`, `scripts/check.sh`, a run on a throwaway profile, what a pull request
    proves and what the owner checks. The full split waits for Stage 2.
  - The measuring scripts on Niri, recording the load of other programs
    ([debt](debt.md#not-waiting-for-a-stage)).
- **Done when.** `scripts/check.sh` passes with its new steps; no accepted ADR is amended by another;
  a fresh reviewer has read each new ADR against the chain it replaces and found nothing lost or
  added.

## The files' formats and versions

Not started. Audit DON-1, DON-2, DON-5, DON-6, DON-8.

- **Goal.** Every file Opium keeps reads its older versions, keeps what it does not know, and is
  never lost to a newer or an older build.
- **Delivers.**
  - The list of every file both apps keep, its owner, whether it can be rebuilt, its version, in
    `ARCHITECTURE.md`.
  - Its first commit: the JSON today's build writes, kept as tests.
  - A version in each file, a migration per step, a backup before a migration writes, a newer profile
    refused with nothing written ([ADR-0051](adr/0051-what-opium-builds-while-it-waits-for-uwui.md)).
  - Types of their own where data crosses versions (shared state's rows, manifest's entries), keeping
    unknown services, fields and values; ids spelt explicitly, not by Rust's debug form.
  - The downloads' manifest put aside read back, and saved once per removal of many; the files'
    dates of adding kept out of the cache; a `content://` id another phone stamped never played as
    this one's.
- **Done when.** Each format is read at each of its versions; an unknown service, field or value
  passes an older build unchanged; an older build given a newer profile leaves every file as it was.
- **Measured.** The start-up to the first useful frame, before and after.

## Connect's versions

Not started. Audit DON-3, DON-4, SEC-4.

- **Goal.** Two devices one version apart pair and sync, and the older knows it should update; a
  forgotten device is forgotten everywhere.
- **Delivers.** A range of versions in the hello, the common highest used; both hellos' hash in the
  Noise prologue, the ALPN and the prologue's tag fixed; a lowest version accepted; a message's items
  read one by one when the whole fails; the forgetting of
  [ADR-0053](adr/0053-a-forgotten-device-forgotten-everywhere.md).
- **Done when.** A test of two versions of the wire pairs and syncs; an edited hello fails the
  handshake; one unknown item leaves the rest of its message merged; three devices forget one.
- **By hand, with the owner.** The new desktop and APK installed together; history and playlists
  checked on both.

## Errors and the log

Not started. Audit ERR-1 to ERR-3; owner's go on `tracing` and `tracing-subscriber` given on
2026-11-16.

- **Goal.** One log a user can attach to an issue, on both apps, secrets stripped in one place; each
  error's kind known to the core.
- **Delivers.** Kinds in `opium_core::provider::Error` and `OpiumError` (offline, session lapsed,
  subscription ended, not in the user's country, rate limited, not understood, not offered by the
  service), each keeping its English text; `desktop::backend`'s replies unchanged, their kinds
  logged; the "encrypted" match of `opium-ffi` on a kind; a writer of two files of bounded size in
  the state folder; the `eprintln!` calls and the writes whose errors are dropped moved to it.
- **Done when.** Each service maps its failures to the kinds, tested; no token, password or signed
  address reaches the log; a flood leaves the log within its bound.
- **Measured.** The wake-ups at rest and the start-up, before and after.

## Parity tests

Not started.

- **Goal.** What the app does at the boundaries the port replaces is recorded, for the new layer to
  match.
- **Delivers.** Tests over fake services at `desktop::backend`'s `Request`/`Reply` and at
  `opium-ffi`'s `Session`: signing in and out, the library kept and read again, playing, downloading,
  searching; the final sign-outs' tests among them.
- **Done when.** Each scenario runs in `cargo test`, deterministic, without a window or the network.

## The shared layer's design

Not started; builds nothing.

- **Goal.** The half of the one layer of state and actions that needs no toolkit, decided before
  the port: the services' lifecycle, who owns and calls off each task, the policies both apps share
  (retries, a station's refill, a search's pause), the errors' kinds.
- **Delivers.** A design session under the method: options, a challenge, the owner's decisions, ADRs.
  It adopts or replaces the sign-out generation of *Final sign-outs*.
- **Done when.** The ADRs are accepted and the ROADMAP's Stage 5 starts from them.

## Ideas

For the owner to pick; none is planned.

- **A gallery of every component in each theme**, captured off screen: the look approved on
  captures, reference images for CI. On the desktop, with uwui.
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
