# AGENTS.md

Instructions for AI agents working in this repository. Read this file fully at the start of every
session.

## Project

Opium is a music player for the computer and the phone that puts Apple Music, SoundCloud, a
Navidrome (OpenSubsonic) server and the user's own music files in one library, in the spirit of
Apple Music and Cider, for a listener who wants each source at its best quality. The desktop app is
Rust with egui, for Linux, playing through PipeWire at each track's own sample rate; the Android app
is Kotlin with Jetpack Compose over the same Rust core, through UniFFI. Paired devices share their
history, Opium's own playlists and where the music stopped, and control each other (Opium
Connect); songs can be kept for offline listening, sealed. Licensed GPL-3.0-or-later. Its real state:
in daily use by its owner on one computer and one phone, never released; the APK is signed with
Android's debug key.

Apple Music and SoundCloud are reached through their web players' private APIs, and their protected
streams through a Widevine CDM borrowed from the user's browser. Opium handles people's accounts,
their sign-ins and their downloads: **safety and honesty towards the services come before
features**.

It is developed primarily by AI agents; the owner sets its direction and validates architecture
decisions. Anyone arriving in the repository, at any point in its life, must be able to understand
all of it from the repository alone. Documentation is part of every deliverable, held to the same
standard as the code.

## Read before working

1. `docs/PLAN.md`: the current stage, its milestones and their status. The source of truth for what
   to do now.
2. `docs/ROADMAP.md`: direction, stage order, exit criteria. An overview, not a commitment.
3. `docs/ARCHITECTURE.md`: the crates, their roles, their dependencies and the main flows.
4. `docs/adr/`: decisions already made. Do not decide them again.

You have no memory of previous sessions. These files are the project's memory: what is not written
there, the next session will not know.

## Scope

- Work only on the stage and milestone marked as current in `docs/PLAN.md`.
- Do not start later stages, and do not add abstractions that prepare for them.
- If a task needs something out of scope, stop and say so.

The roadmap sets direction and order; it does not say what will be built, nor how. Build something
because the current milestone needs it, never because the roadmap mentions it. When experience
contradicts the roadmap, propose a change; once accepted, record it in an ADR. The desktop window
stays on egui until Opium moves to SlimeEngine's UI framework
([ADR-0022](docs/adr/0022-gpui-tried-and-abandoned.md)): do not start another interface framework.

## Environment and commands

Development happens on NixOS, under Wayland. Everything runs inside the Nix development shells:
`nix develop` (loaded by direnv through `.envrc`) for the workspace, with Rust, PipeWire, FFmpeg,
libopus, and cmake and perl for BoringSSL; `nix develop .#android` for the phone, with Rust's
Android targets, `cargo-ndk`, the Android SDK and NDK, JDK 17 and Gradle 9.

```sh
scripts/check.sh                                  # every check of the definition of done; what CI runs
cargo build --workspace
cargo test --workspace                            # no network, PipeWire, display or sign-in needed
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo run -p app-desktop                          # the desktop app, binary `opium`: see the test profile below
cargo build --release -p app-desktop              # target/release/opium
nix build                                         # the packaged desktop app: result/bin/opium

# The phone, in `nix develop .#android`:
./android/build-core.sh                           # the core for arm64 and its Kotlin bindings
(cd android && gradle assembleRelease)            # android/app/build/outputs/apk/release/app-release.apk
scripts/check.sh android                          # clippy on the phone's core
```

`scripts/check.sh` runs, in order: formatting, clippy with warnings as errors, the build and the
tests, the code documentation with warnings as errors, and the links between Markdown files when
lychee is installed. CI runs it unchanged on every push to `main` and every pull request
(`.github/workflows/ci.yml`). Run it before reporting a milestone done. The Gradle project has no
wrapper: `gradle` is the dev shell's.

**The screenshot mode.** `OPIUM_SCREENSHOT=<file.png>` saves the window once it has had time to
load, then quits; `OPIUM_SCREENSHOT_AFTER=<seconds>` sets the wait. Others open a page or a state
first: `OPIUM_SCREENSHOT_PAGE` (`playlist:<id>`, `album:<id>`, `artist:<id>`, `search:<query>`,
`shelf:home|new|radio:<title>`, `page:home|new|radio|albums|artists|recent|songs|settings|favourites`),
`OPIUM_SCREENSHOT_DRAWER`, `_LYRICS`, `_NP`, `_SIGNAL`, `_DOWNLOADED`;
`OPIUM_SCREENSHOT_PLAY=<row>` and `OPIUM_SCREENSHOT_STATION=<ra.…>` play out loud, and
`OPIUM_SCREENSHOT_DOWNLOAD` downloads the page opened (`crates/app-desktop/src/screenshot.rs`). It
compares the image to nothing: look at it.

**Developer examples.** `cargo run -p widevine --example probe` loads the browser's CDM and asks it
for a licence challenge, offline. The others use the owner's real sign-ins, kept in the config
folder: `cargo run -p apple-music --example am -- …`, `cargo run -p soundcloud --example sc -- …`
(which may renew SoundCloud's tokens on disk, under the running app) and
`cargo run -p downloads --features keyring --example spike -- …`. Each file's header lists its
commands; they are run only with the owner's go (see "Things an agent must not do on its own").

### Running Opium without touching the owner's data

The app keeps its files under `$XDG_CONFIG_HOME/opium` (settings, themes, sign-ins, browser
profiles, `connect.json`, `shared.json`, `playlists.json`, the session), `$XDG_CACHE_HOME/opium`
(the library as last read, covers, animated covers, the files' index, `exit.log`) and
`$XDG_DATA_HOME/opium` (downloads). Every one of these follows the three variables. What they do
**not** redirect:

- **the session bus**: the single-instance check, so a test build started while the owner's
  Opium runs only raises the owner's window and exits ([ADR-0018](docs/adr/0018-one-instance-through-the-session-bus.md));
  the MPRIS name; and **the keyring**, whose one item "Opium downloads" holds the key every
  download is sealed with;
- **the network**: Opium Connect announces the app on the local network and through iroh. A fresh
  profile is a new device on the owner's phone; a copied `connect.json` *is* the owner's computer,
  which merges into the phone's real history, playlists and session;
- **the sound card**: PipeWire plays on the owner's speakers;
- **the Widevine CDM**, found under `$XDG_CONFIG_HOME`: a test profile finds none unless
  `WIDEVINE_CDM` names it.

So a test run uses a throwaway profile, no session bus and no network. Inside `nix develop`, with
`WAYLAND_DISPLAY` naming the compositor's socket in `$XDG_RUNTIME_DIR`:

```sh
cargo build -p app-desktop
test=$(mktemp -d /tmp/opium-test.XXXXXX)
unshare --user --map-current-user --net env \
    XDG_CONFIG_HOME="$test/config" XDG_CACHE_HOME="$test/cache" XDG_DATA_HOME="$test/data" \
    DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent \
    OPIUM_SCREENSHOT="$test/shot.png" OPIUM_SCREENSHOT_AFTER=6 \
    target/debug/opium
rm -rf "$test"
```

It writes only under `$test`; it says that the media controls, the keyring (so downloads) and
Opium Connect are unavailable, which is expected. Without the screenshot variables the window stays
until closed. (Checked on 2026-11-04 with the build of the commit before the rename, under its old
names.)

A check that needs the owner's signed-in services uses a **copy of the owner's config folder** as
`XDG_CONFIG_HOME`, with the owner's standing leave, and only this way:

- run as above but without `unshare … --net`, the services needing the network; the session bus
  still pointed nowhere;
- only while the owner's Opium is closed (below), and never while another agent's test runs;
- `connect.json` deleted from the copy before the run and after it: the run then shows on the
  owner's phone as a new device, never to be paired;
- `muted = true` in the copy's `settings.toml`, replacing the key if it is there;
- `WIDEVINE_CDM` set to the browser's `libwidevinecdm.so`;
- no change made in the app: everything there acts on the real accounts;
- the copy deleted when done: it holds the owner's tokens. SoundCloud's tokens last about a day and
  the copy may renew them; tell the owner if it did, as their own may then need a new sign-in.

**The owner's own window.** If the owner's Opium is open and in the way, it may be closed for the
test, gracefully, and started again after:

```sh
pgrep -ax opium                       # whose? the owner's installed program, or a build in a tree
playerctl --player=opium quit         # MPRIS Quit: it saves, then ends within 5 s (ADR-0021)
pgrep -x opium || echo closed
# … the test …
setsid -f <the program pgrep showed> >/dev/null 2>&1   # started again, detached
```

Never `kill` it: killing skips what it saves on closing. A build running from a working tree may be
another agent's test: leave it, and ask.

### The phone

The phone is the owner's daily device, reached over `adb` only with the owner's go. Install the
release build only (`adb install -r android/app/build/outputs/apk/release/app-release.apk`), then
check that `adb shell dumpsys package io.github.honeyslatt.opium | grep pkgFlags` shows no
`DEBUGGABLE`. Before each batch of taps (`adb shell input`), check that Opium is the focused app
(`adb shell dumpsys window | grep mCurrentFocus`), right before the taps, one check per batch.

## Definition of done

A milestone is done only when all of the following hold:

- `scripts/check.sh` passes.
- The program was actually run and behaved as expected, on a throwaway profile as above.
- New behaviour is covered by tests where a test is practical; a bug fix starts with a failing
  test.
- `docs/PLAN.md` reflects the new status.
- Every document affected by the change is updated, and was reread as a newcomer would read it.
- Any decision taken is recorded as an ADR.

Never report that something works without having verified it. If you could not verify, say exactly
what was not verified and why; a behaviour of Apple Music, SoundCloud, Navidrome or the phone that
was not checked for real is said so.

## Things an agent must not do on its own

These act on the owner's accounts, devices, data or ears. Ask the owner first, every time:

- **Act on a real account.** No favourites, likes, follows, ratings ("suggest less"), library
  additions or removals, playlists made, edited or deleted, on Apple Music, SoundCloud or the
  Navidrome server; no sign-in or sign-out (signing out deletes the tokens and the browser's
  profile); no tags written into the owner's own files. The developer examples `am`, `sc` and
  `spike` run on the owner's sign-ins: ask before any of their commands.
- **Buy anything**: no subscription, purchase or paid plan, anywhere.
- **Play sound on the owner's speakers.** `OPIUM_SCREENSHOT_PLAY`, `_STATION`, `am play`, `am live`,
  or anything that plays, only once the owner said yes. Never take the sound card exclusively
  (an ALSA `hw:` device): it once broke WirePlumber's routing for every other program.
- **Edit `settings.toml` while Opium runs**, or at all without a backup taken first.
- **Touch the phone** without the checks above, or play a song there: it replaces the owner's
  queue.
- **Install a debug APK** on the phone.
- **Work around DRM beyond what the app does**: no decrypted copy of a protected song at rest (the
  examples' `am fetch`, `am live` and `sc fetch` write one), no capture of the decoded sound, no
  CDM or keys other than the browser's and the phone's, no getting around the CDM's pace or its
  output rules.
- **Change or delete downloads** (the data folder's `opium/downloads`, the phone's) **or the keyring
  item** "Opium downloads".
- **Get around a service's anti-bot measures.** SoundCloud's bot guard is met only through the
  browser seam; when it refuses, the owner passes its check in the profile's browser. Never retry a
  refused route, solve a captcha, or change fingerprints or addresses to get through.
- **Pair, unpair or forget a device**, or give a test profile the owner's `connect.json`.
- **Kill the owner's Opium**, or close a test instance that is not yours.
- **Add a dependency, a service or a host the app talks to.**

## Architecture rules

- **The workings stay apart from the drawing** ([ADR-0019](docs/adr/0019-the-workings-apart-from-the-drawing.md)):
  the services, the player's sources, what is kept on disk, the media controls and the devices live
  in `crates/desktop` and the crates below it, which name no egui type; `crates/app-desktop` draws.
  A fix to the workings is made there, once.
- **What both apps share is Rust**, reachable from `crates/opium-ffi`. Nothing the phone's core
  depends on may need the desktop: `player`, `motion`, `desktop` and `app-desktop` stay out of it,
  and `scripts/check.sh android` checks it builds.
- **A service is a crate implementing `opium_core::Provider`**, depending on `opium-core` and the
  leaf crates only; `crates/services` puts them together.
- **Nothing slow runs on the window's thread**: it sends a `Request` to the backend and takes the
  `Reply`; whatever runs elsewhere wakes the window through `desktop::Wake`.
- **Nothing is drawn at rest**: an animation asks for its next frame at a capped rate, and the
  decorative ones stop while the window has lost the focus, as the playing row's bars and M3E's
  wave do.
- **D-Bus is spoken over async-io** ([ADR-0020](docs/adr/0020-d-bus-over-async-io.md)).

## Rules the code depends on

### Secrets and privacy

- Tokens live in files only the user can read, written through `desktop::backend::write_private`
  (`0600` in `0700` folders, aside then renamed); the downloads' key lives in the keyring only; on
  the phone, both in the vault ([ADR-0013](docs/adr/0013-secrets-and-what-is-kept-on-disk.md)).
- A secret is never printed, logged, put in an error message or in a kept address: a Navidrome
  cover's address is kept unsigned and signed only as it is fetched; Apple Music's token goes to
  Apple's hosts only.
- The Navidrome password is used once, to make the salted token, and kept nowhere.
- Lyrics sites other than LRCLIB stay off unless the user turns them on: they are told every song
  looked up ([ADR-0008](docs/adr/0008-the-lyrics-sources.md)).
- Only paired devices are listened to over Opium Connect; pairing happens on the local network
  only.

### Files and data safety

- **`settings.toml` is rewritten whole from memory on every change**, so an edit made while the app
  runs is lost. A file that does not read is kept beside as `settings.toml.bad`, and only its
  settings that read are kept. To change a key in a test profile, replace it; never add a second
  one (a duplicate key once made the whole file unreadable and the user's settings were lost).
- Everything kept is written aside, then renamed into place.
- Downloads: a manifest that does not read is put aside, never swept; a sealed file is never
  rewritten in the clear ([ADR-0017](docs/adr/0017-downloads-sealed-and-tied-to-the-subscription.md)).
- The shared state's merge must give the same result whatever the order and however often it runs,
  and deletions stay final ([ADR-0014](docs/adr/0014-shared-state-merged-field-by-field.md)); a
  change to it extends `crates/synced/src/tests.rs`.
- One instance at a time ([ADR-0018](docs/adr/0018-one-instance-through-the-session-bus.md)): two
  would overwrite each other's files.
- No code reads the old "Music" names ([ADR-0023](docs/adr/0023-the-rename-to-opium.md)).
- Tests write only under `std::env::temp_dir()`, in a folder of their own that they remove, never
  in the real folders.

### Honesty towards the services

- A behaviour of Apple Music, SoundCloud or Navidrome is "verified" only once checked against the
  real service, by or with the owner. Tests with fakes never justify saying it works with the
  service; when you could not check something for real, say so.
- Cite where a reverse-engineered behaviour comes from, in the code that relies on it: Kopuz for
  Apple's web player, the CDM and CENC ([ADR-0002](docs/adr/0002-apple-music-through-its-web-player.md)),
  SoundCloud's web app for its endpoints, Chromium's header for the CDM interface. Code from
  GPL-compatible projects is welcome with a note naming its source, revision and licence; never copy
  code that has no licence.
- SoundCloud's changes go through the browser seam (`soundcloud::Browser`) and nothing else
  ([ADR-0012](docs/adr/0012-soundcloud-changes-through-a-browser-seam.md)); reads stay plain.
- Apple's developer token is read from the web player; when Apple moves it, the error says which
  step broke.

### DRM

- The Widevine CDM is the user's browser's, never shipped, loaded through the CDM-11 interface by
  the shim, in temporary sessions; on the phone, the system's MediaDrm
  ([ADR-0004](docs/adr/0004-the-widevine-cdm-borrowed-from-the-browser.md)).
- What a licence allows is the ceiling: AAC 256 and binaural Spatial Audio from Apple, the CDM's
  own decryption pace, its output rules.
- Protected audio is never kept in the clear: a download is sealed, and a protected song plays only
  while its service's subscription was confirmed in the last 30 days
  ([ADR-0017](docs/adr/0017-downloads-sealed-and-tied-to-the-subscription.md)).

## Code rules

- No abstraction without a second concrete use. No trait, generic or layer added "just in case".
- Do not reimplement what the standard library provides without a measurement that justifies it.
- Every new dependency needs a justification: why, which alternative was considered, its licence and
  its compatibility with the GPL-3.0. Ask the owner before adding one.
- Every `unsafe` block carries a `// SAFETY:` comment explaining why it is sound.
- Comments explain why, not what. No commented-out code.
- Follow the conventions that ADRs set. If no ADR covers a case, propose one instead of deciding
  silently.

## Consistency rules

- Before writing a helper, type or pattern, search the code for an existing one. Reuse or extend it.
- One way to do a given thing. If you find two, unify them or report it.
- Prefer less code. Delete dead code, unused parameters and unused features.
- Keep changes local to the milestone. Unrelated cleanup goes in a separate commit, or in a note in
  `docs/PLAN.md`.
- If the code has drifted from `docs/ARCHITECTURE.md` or an ADR, report it. Do not silently work
  around it.
- A change to a page is looked at in each built-in theme (Stock, Spotify, M3E), and at the
  display's fractional scale.

## Documentation

### Standard

The reader is a competent developer who has never seen this project. From the repository alone,
they must be able to understand what every part does and why it is built that way, then build, test
and change it.

- Documentation describes the code as it is now, not as it is intended to be. Wrong documentation
  is worse than missing documentation.
- No reference to past sessions or conversations the reader cannot see; every term the project
  coins is defined where it first appears and in `docs/glossary.md`.
- State limits and known debt plainly. Do not present a partial feature as complete.
- Every public item has documentation that says what it does, its invariants, its errors and its
  panics.
- A change that leaves a document wrong is not finished.

### One home per fact

A fact written in two places drifts. Each kind of fact has one home, and other documents link to it:

- **The code's documentation**: what an item does, its contracts, its limits and their figures.
- **`docs/ARCHITECTURE.md`**: the flows across the parts, without the figures the code's
  documentation holds.
- **ADRs**: why a decision was taken and what was rejected. Only their status changes afterwards.
- **`docs/PLAN.md`**: status and known debt, linking to where the facts live.
- **`CHANGELOG.md`**: what changed for users, once, when it changes.

| File | Content | Update when |
|---|---|---|
| `README.md` | What Opium is, its real state, its requirements, how to build and run it | State, requirements or entry points change |
| `docs/ROADMAP.md` | Direction, stage order, exit criteria | A stage is reshaped, merged or dropped (with an ADR) |
| `docs/PLAN.md` | Current stage, milestones, status, known debt | Every milestone |
| `docs/ARCHITECTURE.md` | Crates, responsibilities, dependencies, main flows | Structure or a flow changes |
| `docs/adr/NNNN-title.md` | Context, decision, alternatives rejected, consequences | Each decision |
| `docs/adr/README.md` | The index of the ADRs and their statuses | An ADR is added or its status changes |
| `docs/glossary.md` | Terms the project defines | A term is coined or changes meaning |
| `CHANGELOG.md` | Notable changes, per release | A change users would notice; a release |
| `AGENTS.md` | These rules, the commands, the pitfalls | A command, a rule or a pitfall changes |

## Commits

- One logical change per commit, small enough to review.
- Subject in the imperative, in English, saying what changes for the user. The body explains why
  when it is not obvious, with measurements when the change is about speed or power.
- Do not commit code that fails the checks.

## When to stop and ask

Stop and ask the owner before proceeding when:

- A decision affects the architecture and no ADR covers it.
- You want to add a dependency, a service or a host the app talks to.
- The request contradicts `docs/ROADMAP.md` or an ADR.
- The plan turns out to be wrong, or a milestone is larger than expected.
- You are about to delete or rewrite a significant part of the code.
- A check needs the owner's accounts, phone, speakers or running window.

Propose the options with their trade-offs, in plain terms, and a recommendation for the long term.
Put decisions one at a time. Do not work around a problem to keep going.

## Reporting

At the end of each milestone, report:

- What was done.
- Notable design choices, why, and which alternatives were rejected.
- What deserves to be read first.
- Shortcuts taken and debt introduced.
- The next step, on the owner's go.

A reviewing agent checks the milestone against this file and the ADRs before it is reported done.

## Working with several agents

- Each agent works on one milestone at a time, as `docs/PLAN.md` assigns it.
- Shared files (`docs/PLAN.md`, `docs/ARCHITECTURE.md`, `Cargo.toml`, `Cargo.lock`, `flake.nix`) are
  edited by one agent at a time.
- Commit your own files by their paths, never with `git add -A`, so as not to take another agent's.
- Never `git stash`, reset or check out over a working tree another agent works in; use a worktree
  of your own (`git worktree add`), with a target folder of its own.

## Known pitfalls

- **Closing the app sometimes hung**, the window gone and the process left, which then kept a new
  start from opening. The cause is not found. The exit watch ends the process five seconds after
  everything is saved, appends where the teardown stood to `exit.log` in the cache folder and
  aborts, which leaves a core dump of every thread
  ([ADR-0021](docs/adr/0021-the-exit-watch.md), [ADR-0024](docs/adr/0024-the-exit-watch-aborts-for-a-core-dump.md)):
  read `exit.log`, then `coredumpctl list opium` and `coredumpctl gdb`, before guessing. One hang
  seen was still inside eframe's teardown, with the Mesa OpenGL driver's threads alive.
- **The owner's display runs at a fractional scale (1.5).** Feathered edges and the covers' sizes
  are computed in physical pixels: check a visual change at 1.5 as well as at 1.
- **Exclusive audio breaks WirePlumber.** A program that takes the card through ALSA `hw:` once left
  WirePlumber without it after; `systemctl --user restart wireplumber` brought it back, at another
  volume. Check `wpctl status` after any audio experiment.
- **A debug APK feels like 30 Hz on the phone**: debuggable apps are not compiled ahead of time.
  The release build is signed with the same debug key, so it installs over either.
- **`pgrep -f opium` matches shells** whose command line merely contains the word, yours included.
  Use `pgrep -ax opium`, which matches the process's name, and read the path it prints.
- **The owner's shell is zsh**: an unquoted `$var` is not split into words, and a word starting
  with `=` is replaced by a command's path (`echo ====` fails). Quote, or run scripts with bash.
- **A test build started while Opium runs exits at once, printing nothing**: it raised the running
  window ([ADR-0018](docs/adr/0018-one-instance-through-the-session-bus.md)).
- **With a test `XDG_CONFIG_HOME`, no Widevine CDM is found**: set `WIDEVINE_CDM`.
- **`unshare -rn` breaks D-Bus's authentication**; to cut the network but keep the bus, use
  `unshare --user --map-current-user --net`.
- **Opium Connect's tests listen on `0.0.0.0:47820` when the port is free**, reachable from the
  local network for their few seconds.
- **The session is shared with the phone**: an instance that shares the owner's device identity
  and stops a song overwrites the phone's newer stop.
- **egui paints layers drawn without an `Area` before the others**: a layer drawn over the panes
  must be registered (`ui::register_layer`). **`color::mix(a, b, p)` weights `a` by `p`.**
- **The first build is slow**: `wreq` builds BoringSSL from source, and FFmpeg's bindings run
  bindgen.
- **`synced`'s test `kept_on_disk_from_one_launch_to_the_next` fails now and then** (2 runs in 8 on
  2026-11-04): it waits for `playlists.json` but not for `shared.json` (docs/PLAN.md).

## Language

- Everything in the repository is in English: code, comments, commits, documentation.
- Talk to the owner in French.
