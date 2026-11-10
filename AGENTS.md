# AGENTS.md

Rules for AI agents working in this repository. Read this file fully at the start of every session.

## Project

- Opium is a music player for the computer and the phone: Apple Music, SoundCloud, Navidrome and the
  user's own files in one library ([ADR-0016](docs/adr/0016-the-services.md)).
- Desktop: Rust, egui, Linux, PipeWire at each track's rate. Phone: Kotlin and Compose over the same
  Rust core.
- Paired devices share history and playlists (Opium Connect); downloads stay sealed.
- GPL-3.0-or-later, in daily use, aimed at the general public
  ([ADR-0034](docs/adr/0034-what-opium-is-to-become.md)); never released.
- Opium handles people's accounts and sign-ins: **safety and honesty towards the services come
  before features**.

## Read before working

- `docs/PLAN.md`, *Current* section and current milestone: what to do now.
- When planning or touching a decision: `docs/ROADMAP.md`, `docs/ARCHITECTURE.md`, `docs/adr/`.
- Before running anything: [`CONTRIBUTING.md`](CONTRIBUTING.md),
  [known pitfalls](CONTRIBUTING.md#known-pitfalls) first.

## Scope

- Work on the current milestone or what the owner asks; a request that changes the plan goes into
  the PLAN.
- Start no later stage, nor prepare abstractions for one.
- Until the port to uwui
  ([ADR-0035](docs/adr/0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md)), work beyond
  the current milestone fixes only data loss, a crash, a security flaw or a broken service, unless
  the owner asks otherwise.
- When experience contradicts the roadmap, propose a change; once accepted, record it in an ADR.
- `crates/app-desktop` is frozen until the move to uwui, the owner's UI framework
  ([ADR-0026](docs/adr/0026-the-egui-window-frozen-until-the-move.md)): change it only for data loss,
  a crash, a security flaw or M1.14's measuring. Start no other interface framework.
- The phone's Compose screens (`ui/`) are frozen likewise
  ([ADR-0031](docs/adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)); its Kotlin system
  layer is not.

## Commands

```sh
nix develop
scripts/check.sh
cargo test --workspace
cargo run -p app-desktop
scripts/check.sh android     # this line and the next in `nix develop .#android`
./android/build-core.sh && (cd android && gradle assembleRelease)
```

What each does is in [`CONTRIBUTING.md#commands`](CONTRIBUTING.md#commands). The app runs on a
[throwaway profile](CONTRIBUTING.md#a-throwaway-profile), in a headless Weston where it can; end a
test instance by its pid.

### Never on your own

Ask the owner first, every time:

- No change on a real account (likes, ratings, library, playlists, sign-in) or to the owner's tags.
- No purchase, no developer example on the owner's sign-ins (`am`, `sc`, `spike`), nor `probe`,
  which loads the owner's CDM.
- No run [on a copy of the owner's profile](CONTRIBUTING.md#tests-on-a-copy-of-the-owners-profile):
  it talks to the real services and may renew their tokens.
- No sound on the owner's speakers; never take the sound card exclusively (ALSA `hw:`).
- No touch on the phone without [its checks](CONTRIBUTING.md#the-phone-over-adb), no song played
  there, no debug APK.
- No change to the downloads or the keyring item "Opium downloads"; no device paired, unpaired or
  forgotten; never the owner's `connect.json` in a test profile.
- No DRM worked around beyond the app: no decrypted song at rest, no capture of the decoded sound,
  no other CDM or keys.
- No anti-bot measure got around: no refused route retried, no captcha, no fingerprint or address
  changed ([ADR-0012](docs/adr/0012-soundcloud-changes-through-a-browser-seam.md)).

The owner allows closing Opium for a test: MPRIS Quit only, started again after
([how](CONTRIBUTING.md#the-owners-own-window)). Never kill it, nor close another agent's instance or
a window through the compositor.

## Definition of done

- `scripts/check.sh` passes.
- The program was run on a throwaway profile and behaved as expected.
- New behaviour is tested where practical.
- A visual change is looked at in each built-in theme, at the quality bar's scales.
- The budgets the milestone touches are measured and met, in `docs/measurements.md`.
- `docs/PLAN.md` gives the new status.
- Every affected document is updated and reread as a newcomer.
- Every decision a newcomer would question is an ADR.

Never report as working what was not verified, services and phone included: say what was not, and
why. Never weaken a check to make it pass (a skipped test, a raised budget, a silenced lint)
without the owner's go.

## Architecture rules

- `crates/desktop` and below name no egui type; `crates/app-desktop` draws; fix the workings there
  once ([ADR-0019](docs/adr/0019-the-workings-apart-from-the-drawing.md)).
- Both apps share Rust through `crates/opium-ffi`, which never depends on `player`, `motion`,
  `desktop` or `app-desktop`.
- A service is a crate implementing `opium_core::Provider`, on `opium-core`, leaf crates and
  `lyrics` only.
- The window's thread never waits: a `Request` to the backend, a `Reply` back, `desktop::Wake` to
  redraw.
- Nothing drawn at rest: animations ask for frames at a capped rate, decorative ones stop unfocused.
- D-Bus is spoken over async-io ([ADR-0020](docs/adr/0020-d-bus-over-async-io.md)).

## Rules the code depends on

### Secrets and privacy

- Tokens in user-only files (`desktop::backend::write_private`), the downloads' key in the keyring,
  the phone's in its vault.
- A secret is never printed, logged, put in an error or a kept address; Navidrome's password is
  kept nowhere ([ADR-0013](docs/adr/0013-secrets-and-what-is-kept-on-disk.md)).
- Lyrics sites besides LRCLIB: opt-in ([ADR-0008](docs/adr/0008-the-lyrics-sources.md)).
- Opium Connect hears only devices paired on the local network
  ([ADR-0015](docs/adr/0015-away-from-home-through-iroh.md)).

### Settings and data

- `settings.toml` is rewritten whole on every change: never edit it while Opium runs, nor without a
  backup ([settings care](CONTRIBUTING.md#settings-care)).
- An unreadable downloads manifest is put aside, never swept; a sealed file is never rewritten in
  the clear ([ADR-0017](docs/adr/0017-downloads-sealed-and-tied-to-the-subscription.md)).
- The shared state's merge ignores order and repetition, deletions are final; a change to it extends
  `crates/synced/src/tests.rs`
  ([ADR-0014](docs/adr/0014-shared-state-merged-field-by-field.md)).
- One instance at a time, or two overwrite each other's files ([ADR-0018](docs/adr/0018-one-instance-through-the-session-bus.md)).
- No code reads the old "Music" names ([ADR-0023](docs/adr/0023-the-rename-to-opium.md)).
- Tests write only in their own folder under `std::env::temp_dir()`.

### Honesty towards the services

- A service's behaviour is "verified" only against the real service, with the owner; fakes never
  prove it.
- Reverse-engineered behaviour cites its source (Kopuz, SoundCloud's web app, Chromium's CDM
  header); copied code is GPL-compatible, source and revision noted
  ([ADR-0002](docs/adr/0002-apple-music-through-its-web-player.md)).
- SoundCloud's changes go through `soundcloud::Browser` only; reads stay plain
  ([ADR-0012](docs/adr/0012-soundcloud-changes-through-a-browser-seam.md)).
- When Apple moves its developer token, the error names the step that broke.

### DRM

- The CDM is the browser's, never shipped, driven through the CDM-11 interface in temporary
  sessions; the phone's is MediaDrm
  ([ADR-0004](docs/adr/0004-the-widevine-cdm-borrowed-from-the-browser.md)).
- A licence is the ceiling: AAC 256 and binaural Spatial Audio from Apple, the CDM's pace and
  output rules.
- Protected audio is never kept in the clear; offline it plays only within 30 days of a confirmed
  subscription ([ADR-0017](docs/adr/0017-downloads-sealed-and-tied-to-the-subscription.md)).

## Code rules

- Plain code: names that say what things are, one job per function, the obvious
  solution over the clever one.
- No abstraction without a second use, nor standard library reimplemented without a measurement.
- A new dependency, service or host the app talks to needs a reason, an alternative weighed, a
  GPL-compatible licence and the owner's go.
- Every `unsafe` block has a `// SAFETY:`. Comments say why; no commented-out code.
- Fix a bug at its cause, after a test that fails; look for it elsewhere.
- Edge cases are tested (empty, huge, malformed, concurrent, interrupted); tests are
  deterministic.
- The user's data is never lost: versioned formats, writes aside then renamed, a backup first.
- Reuse helpers; one way to do a thing; delete dead code; follow the ADRs' conventions.
- Report drift from `docs/ARCHITECTURE.md` or an ADR; do not work around it.

## Documentation

- Written for a newcomer: plain words, the essential first, as short as complete.
- Describe the code as it is now, limits and debt plainly.
- No reference to conversations the reader cannot see; coined terms go in `docs/glossary.md`.
- Every public item says what it does, its invariants, errors and panics.
- Each fact has one home others link to; an ADR's body never changes, only its status.
- `AGENTS.md` stays within 1,500 words and `docs/PLAN.md` within 2,000; `scripts/check.sh` fails
  beyond.

## Commits

- One logical change per commit, in English, its subject in the imperative, saying what changes for
  the user.
- Never commit code that fails the checks. A red CI on `main` is fixed before any new work.

## When to stop and ask

Before a decision no ADR covers, a request against the roadmap or an ADR, a wrong plan, a growing
milestone, a large rewrite, or a check needing the owner's accounts, phone, speakers or window:
give the options, plain trade-offs and a long-term recommendation, one decision at a time. Never
work around a problem.

## Reporting

- Once the strongest model's reviewing agent has checked the milestone against this file, the ADRs
  and the quality bar, report what was done, choices and alternatives, what to read first, figures,
  debt and next step, on the owner's go.
- A stage ends with a stage audit, a release follows one: [`docs/audits/`](docs/audits/README.md).
- Ideas outside the current work go to the PLAN's [Ideas](docs/PLAN.md#ideas).

## Working with several agents

- One milestone per agent, as `docs/PLAN.md` assigns it.
- Shared files (`docs/PLAN.md`, `docs/ARCHITECTURE.md`, `Cargo.toml`, `Cargo.lock`, `flake.nix`):
  one agent at a time.
- Commit your files by their paths, never with `git add -A`.
- Never stash, reset or check out over another agent's tree; use your own worktree and target.

## Language

Everything in the repository is in English.
