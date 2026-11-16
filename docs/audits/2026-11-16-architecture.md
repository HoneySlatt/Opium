# Architecture audit, 2026-11-16

What in Opium's structure costs more the later it is dealt with: for its contributors, for its
evolution, and for its stability. The owner asked for it after the method added two audit
dimensions, *architecture and evolvability* and *stability and recovery*, which the
[Stage 1 audit](2026-11-10-stage-1.md) did not cover. It fixed nothing and built nothing. Open
items go to the [known debt](../debt.md); the decisions they need are listed at the end, for the
owner.

## Scope

- **Commit**: `683039b` on `main`.
- **Dimensions**: architecture and evolvability; stability and recovery; what a contributor meets.
- **How**: one agent read the documents and the code paths cited below. Each finding gives the line
  that shows it. Nothing was run or measured, no test was written, and no second reviewer checked
  it: the findings are read, not proven by failing tests.

## What holds

- The crates follow responsibilities, the dependency graph has no cycle, and its rules are written
  down ([ARCHITECTURE](../ARCHITECTURE.md#dependencies)).
- The workings are apart from the drawing on the desktop
  ([ADR-0019](../adr/0019-the-workings-apart-from-the-drawing.md)): `desktop` names no egui type.
- Every service sits behind `opium_core::Provider`; the services' private APIs stay in their own
  crate.
- Files kept whole are written one way (`opium_core::files::write_whole`); a file that does not read
  is put aside, never written over.
- The shared state merges field by field, ignoring order and repetition, and has its own tests
  ([ADR-0014](../adr/0014-shared-state-merged-field-by-field.md)).
- Opium Connect skips a message it cannot read (`crates/connect/src/lib.rs:1158`), so a later
  version can add messages.
- The decisions are ADRs; the debt is known and triaged; one command checks everything.

## Findings

Ordered by how fast their cost grows. "Before 0.1.0" means a release makes the item permanent:
once people have installed Opium, every change to it needs a migration that works on their data.

### Before the first release

- **A1. Four stored formats carry no version**: the shared state, the pairings, the caches and the
  settings ([debt](../debt.md#waiting-for-stage-7-the-library)). The debt waits for Stage 7. After
  0.1.0, each change to one of them must guess which version wrote the file.
- **A2. Two devices of different versions do not talk.** A connection is refused unless both sides
  have exactly the same `VERSION` (`crates/connect/src/lib.rs:1035`), with the error "not one of
  ours". The phone and the computer update at different times (Flathub, Nix, Obtainium). No range
  is negotiated, and nothing tells the user which device to update.
- **A3. A service added later breaks older devices.** `ProviderKind` has no fallback for a variant
  it does not know (`crates/opium-core/src/model.rs:11`). When Jellyfin comes (Stage 9), an older
  device cannot read a message that holds a single Jellyfin song, and drops it whole. An older build
  that reads a `shared.json` holding one puts the whole file aside.
- **A4. The desktop's identity is not settled.** Its Wayland app id is `opium`
  (`crates/app-desktop/src/main.rs:49`, `packaging/opium.desktop`). Flathub asks for a reverse-DNS
  id, and a Flatpak's id names its data folder and its `.desktop` file. Changing it after a release
  moves users' data. The phone's id (`io.github.honeyslatt.opium`) is settled; its signing key is
  [open](../ROADMAP.md#open): an APK signed with another key cannot update the installed one, and
  reinstalling loses the downloads.
- **A5. Errors are sentences, not kinds.** `opium_core::provider::Error` has four variants, three
  of them carrying an English string (`crates/opium-core/src/provider.rs:9`); the phone's
  `OpiumError` is the same (`crates/opium-ffi/src/lib.rs:93`). French ships at 0.1.0
  ([ADR-0034](../adr/0034-what-opium-is-to-become.md)): a sentence made in a service crate cannot be
  translated. Nor can the interface tell apart offline, a session that lapsed, a subscription that
  ended, a song not available in the user's country or a service that limits requests. Each
  service crate adds errors as it grows, so the change grows with them.
- **A6. No log to attach to an issue.** No crate uses a logging library: 46 `eprintln!` across the
  workspace and one `Log` call on the phone. Under Flatpak nobody sees stderr. A contributor gets
  no levels, no file to attach, and no single place that strips secrets before writing. The rule
  "a secret is never logged" is kept by hand at each call. The crash report is an
  [idea](../PLAN.md#ideas); a log comes first.
- **A7. Contributions from others have no terms.** No sign-off (DCO) or agreement is asked; no
  file carries an SPDX header (0 files); there is no `SECURITY.md` for a private report, though
  Opium handles people's accounts; no code of conduct, issue or pull request template. The licence
  has already been reconsidered once ([ADR-0030](../adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md),
  [ADR-0032](../adr/0032-the-gpl-kept-and-kopuzs-code-still-written-again.md)): after outside
  contributions, changing it needs every contributor's consent.
- **A8. Making the repository public cannot be undone.** What a release exposes of DRM is
  [open](../ROADMAP.md#open) until Stage 2. A public history stays copied once forked. The code that
  loads the browser's CDM and talks to Apple's private API is the part most exposed to a takedown
  request. This needs the owner's decision, with advice the audit cannot give, before anything
  goes public.

### At the move to uwui, Stage 5

- **B1. The interface's logic exists twice, and the workings twice more.** On the desktop, the
  state and logic of the interface sit inside `app-desktop/src/app.rs` (6,432 lines, mixed with
  egui); on the phone, inside the Compose screens (`ui/`, 14,042 lines of Kotlin). Below them, the
  desktop's `desktop::backend` (3,591 lines, its loop one function of 1,060) and the phone's
  `opium-ffi` (2,240 lines) each orchestrate the services, downloads and Connect their own way: two
  glue layers over `downloads` (`desktop/src/backend/downloading.rs`, `opium-ffi/src/downloading.rs`).
  uwui will draw both apps ([ADR-0031](../adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)),
  and three more front ends are planned: `opium-headless` (Stage 8), the `opium` command and the
  actions for users' agents ([ADR-0037](../adr/0037-uwuis-decisions-of-2026-11-07-taken-into-opium.md)).
  If the port moves `app.rs` into uwui as it is, the duplication is kept for the life of the app.
  One toolkit-free Rust layer of state and actions, tested without a window, would serve all five.
  This needs a design session before Stage 5 starts.
- **B2. The platform's code is spread rather than placed.** PipeWire (`player`), D-Bus and MPRIS
  (`desktop::mpris`, `desktop::instance`), the Secret Service (`downloads`, feature `keyring`) and
  XDG folders (`desktop::folders`) are each Linux-only. Windows comes at Stage 4. No seam should be
  built before then (AGENTS.md, *Scope*), but the port can keep each in a module that names its
  platform, so that Stage 4 replaces modules rather than edits callers.
- **B3. Local songs are known by their path** (`crates/local/src/lib.rs:319`). The shared history
  keeps the whole `Track`, so it still shows. But a song moved or renamed loses its place in Opium's
  playlists, and one album on the computer and the phone has two ids. Whether the best-copy
  matching ([ADR-0007](../adr/0007-one-library-and-the-best-copy.md)) is enough for statistics,
  scrobbling and cross-device playlists (Stage 6, Stage 7) is not checked. The ids live in the
  shared state, so they become a format with A1.

### Stability and recovery

Most of these are already in the debt; they are named here because the method now asks every
queue, cache and piece of concurrent work to be bounded, with its behaviour at the limit stated.

- **C1. Unbounded work**: Connect's sender queues every part of a sync before writing the first
  (76 MB at 100,000 songs); one thread per cover file; a failed service retried every minute for
  ever; a connect that a server keeps alive with no overall deadline ([debt](../debt.md)).
- **C2. A downgrade loses data.** An older build drops the manifest's new fields and may sweep the
  sealed files they kept ([debt](../debt.md#waiting-for-stage-7-the-library)). With A1 and A3,
  nothing says which upgrades can be undone; no backup is taken before a format changes.
- **C3. No long-run test**: nothing plays, syncs or downloads for hours and checks that memory,
  threads and files come back to where they started.

### For contributors

- **D1. CI does not build the phone's Kotlin or APK** ([debt](../debt.md#for-the-first-release-stage-2)):
  a contributor who changes the core can break the phone without CI saying so.
- **D2. `deny.toml` is written but not used**
  ([proposal](2026-11-10-stage-1/dependencies/deny.toml)): CI learns of no advisory.
- **D3. CONTRIBUTING mixes two readers.** Its 519 lines hold what any contributor needs and what
  only the owner's machine needs (the owner's window, the phone over adb, measuring on the 4K
  display). A newcomer has no short path from cloning to a first change tested.
- **D4. Large files**: `app.rs` (6,432 lines), `backend.rs` (3,591), `apple-music/src/lib.rs`
  (3,198). Two contributors working there will meet in merges.
- **D5. 820 public items without documentation**, and no lint that keeps new ones documented
  ([debt](../debt.md)).
- **D6. A service change cannot be verified without the owner.** That is right
  (AGENTS.md, *Honesty towards the services*), but no document tells a contributor what to prove
  with fakes and what the owner checks afterwards.

### Drift found on the owner's machine

- **E1. The compositor is now Niri; Umbriel is gone.** `scripts/measure/startup.py` (`map`) still
  runs `umbriel subscribe windows`, which no longer exists; `CONTRIBUTING.md` names Umbriel three
  times (lines 16, 234, 497).
- **E2. Timed measures must record the machine's load** before and while they run, and mark a run
  that load disturbed (the owner's rules, *Running programs*). `frames.py` and `power.py` read it;
  `rest.sh`, `sample.py`, `startup.py` and `memwatch.py` do not.

## Proposed for the quality bar

The method's template now gives the quality bar a table of change and failure scenarios. A first
proposal for the ROADMAP, which the owner decides:

| Scenario | Acceptance criterion | Checked by |
|---|---|---|
| A service added (Jellyfin) | The change touches the new crate, `ProviderKind`, `services` and the interface's list of services; older devices keep syncing | Tracing the code; a test that reads a message with an unknown service |
| Windows' audio output | `player` gets a second output; no caller outside it changes | Tracing the code at Stage 4 |
| A phone one version behind the computer | They pair, sync what both know, and the older says it should update | A test of two versions of the wire |
| An upgrade, then a downgrade | Every format reads its older version; a downgrade either reads the newer file or refuses it without losing it | Migration tests on test data |
| Hours of play, sync and downloads | Memory, threads and open files return to their start; no queue grows past its bound | A long-run test on a throwaway profile |
| The network cut during a sync or a download | State stays valid; the work resumes or stops, bounded | Fault-injection tests |

## Decisions for the owner

One at a time, each with its options:

1. Move "every format versioned", Connect's version range and the unknown service (A1 to A3) from
   Stage 7 into Stage 2, before 0.1.0.
2. The desktop's app id (A4), with the signing key already open at Stage 2.
3. Errors as kinds and one log with secrets stripped (A5, A6): at the port, or before it.
4. The terms for outside contributions (A7) and the repository's visibility (A8).
5. A design session for one shared layer of state and actions before Stage 5 (B1).
6. The scenarios above in the ROADMAP's quality bar.
7. Whether to align `AGENTS.md` with the method's new template: its rule "No abstraction without a
   second use" becomes "an abstraction needs a concrete benefit: reuse, isolation of an unstable
   dependency, protection of an invariant or a test boundary", and the definition of done checks
   the affected architecture contracts and stability scenarios.
