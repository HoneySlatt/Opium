# Contributing

How to set up the environment, the commands in detail, and how a change is made and checked. The
rules for agents are in [`AGENTS.md`](AGENTS.md).

## Environment

Development happens on NixOS, under Wayland. Everything runs inside the Nix development shells of
[`flake.nix`](flake.nix):

- `nix develop`, loaded by direnv through `.envrc`: the workspace's shell. It provides Rust (cargo,
  rustc, clippy, rustfmt, rust-analyzer), pkg-config, PipeWire, FFmpeg and libopus to link against,
  cmake and perl for BoringSSL, libclang for bindgen, git and lychee for the links between
  Markdown files, `dbus-run-session` for the tests of the one instance, and Python, Weston and
  hyperfine for measurements. Every tool `scripts/check.sh` uses comes from it; the measuring
  scripts also use the system's `unshare` and `umbriel`, `adb` for the phone
  (`nix shell nixpkgs#android-tools`) and `heaptrack` for the memory's detail.
- `nix develop .#android`: the phone's shell, x86_64 only. It provides Rust with the Android
  targets, `cargo-ndk`, the Android SDK and NDK (Google's, under its own licence, accepted for this
  shell only), JDK 17 and Gradle 9.
- `nix develop .#fuzz`: a nightly Rust and `cargo-fuzz`, for [fuzzing](#fuzzing) only
  ([ADR-0030](docs/adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md)).

What the app needs besides, at run time (the [README](README.md#requirements) says it for users):

- PipeWire, the only audio output.
- A Chromium-family browser to sign in to Apple Music and SoundCloud; `OPIUM_BROWSER` chooses one.
- The Widevine CDM a browser installed, for protected streams; `WIDEVINE_CDM` names a
  `libwidevinecdm.so` elsewhere.
- For Opium Connect, 47820/tcp and 5353/udp open in the firewall.

The app keeps its files in three folders, each following its XDG variable:

- `$XDG_CONFIG_HOME/opium`: settings, themes, sign-ins, the sign-in browsers' profiles,
  `connect.json`, `shared.json`, `playlists.json`, the playlists' chosen covers (`covers/`), Apple's
  first-seen times (`apple-seen.json`), files that did not read set aside (`*.bad`, `*.bad.1`…),
  the session, and `instance.lock`, which one instance holds per profile ([ADR-0043](docs/adr/0043-one-instance-per-profile-held-by-a-lock.md)).
- `$XDG_CACHE_HOME/opium`: the library as last read, covers, animated covers, the files' index,
  `exit.log`.
- `$XDG_DATA_HOME/opium`: downloads.

A variable unset, empty or not an absolute path counts as unset, as the XDG Base Directory
specification says: `~/.config`, `~/.cache` and `~/.local/share` are used (`desktop::folders`).
With no usable `HOME` either, there is no folder: without a config folder the app refuses to start;
without the others, nothing is cached, the files are not opened, or there are no downloads.

## Commands

### The checks

```sh
scripts/check.sh            # every check of the definition of done; what CI runs
scripts/check.sh android    # inside `nix develop .#android`: the phone's core
scripts/check.sh docs       # the documents only: their sizes and links, in a second
```

- `scripts/check.sh` runs, in order: the documents' part below, formatting, clippy with warnings as
  errors, the build and the tests, and the code documentation with warnings as errors. Its head
  says what it checks and what it does not.
- `scripts/check.sh docs` checks the documents alone. It fails when `AGENTS.md` passes 1,500 words
  or `docs/PLAN.md` 2,000 (`wc -w`): every agent reads both at every session, so their size is
  paid each time. Then it checks the links between the tracked Markdown files (lychee, offline,
  anchors included). Shorten a file that fails by moving its detail to its home: finished
  milestones to [`docs/history.md`](docs/history.md), debt to [`docs/debt.md`](docs/debt.md),
  commands to this file.
- `scripts/check.sh android` runs clippy on `opium-ffi` for arm64 Android, warnings as errors, in
  `target-android`. It checks that the phone's core builds without the desktop's crates. The APK
  itself is not checked.
- CI ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) runs
  `nix develop --no-update-lock-file --command scripts/check.sh` on every push to `main`, every
  pull request, and every Monday at 06:53 UTC, but for a push or a pull request that changes only
  Markdown files. The flag makes an incomplete `flake.lock` an error instead of a silent update.
  A second job runs `scripts/check.sh android` in `nix develop .#android` at the same times.
  cache.nixos.org holds none of Google's SDK but an old `sdk-tools` archive, so each run downloads
  the rest from Google: a version missing from Google's server fails that job, as it did by hand on
  2026-11-04 and 2026-11-05. That red blocks work as any other
  ([ADR-0036](docs/adr/0036-the-phones-ci-job-blocks-even-when-google-fails.md)). The fix, with
  the owner's go, is a version that Google serves, chosen in `flake.nix` where it can be: the NDK,
  the build-tools and the platform are pinned there (`ndk`, `buildTools`, `platformVersions`), and
  CMake and the command-line tools can be (`cmakeVersions`, `cmdLineToolsVersion`, arguments of
  `composeAndroidPackages`), each among the versions nixpkgs' `androidenv` lists in its
  `repo.json`. Only platform-tools has a single version there: for it, a newer nixpkgs in
  `flake.lock`, which also moves the desktop's and the fuzzing shell's tools and the packaged app,
  but not the Android shell's Rust, which rust-overlay pins.
- [`documents.yml`](.github/workflows/documents.yml) runs `scripts/check.sh docs` at every push to
  `main` and on every pull request, documents only included, with lychee from the pinned nixpkgs: a minute or two,
  where the whole check takes ten or more.
- The weekly run tells a project at rest that a dependency or CI's environment has changed: a
  broken build or a moved toolchain, not a new advisory, which nothing checks before Stage 2's
  `cargo deny` ([debt](docs/debt.md)).
  GitHub stops scheduled runs after 60 days without activity on a public repository; a click in
  the Actions tab starts them again. The repository is private: each run, weekly ones included,
  spends GitHub's free quota of minutes.
- CI sets `CARGO_PROFILE_DEV_DEBUG=0` (the tests' binaries without debug info) and
  `CARGO_INCREMENTAL=0` (no incremental data, which a fresh runner never reuses): the same checks in
  a smaller, faster build. Set them locally to reproduce a CI build.
- No test needs the network, PipeWire, a display, the user's session bus, a sign-in or a Widevine
  CDM (the tests of the one instance start a bus of their own). What needs them is checked by hand,
  on a [throwaway profile](#a-throwaway-profile).
- Run `scripts/check.sh` before reporting a milestone done. Lychee reads only tracked files: check a
  new Markdown file with `lychee --offline --include-fragments <file>` until it is committed.

### Build, test, lint

```sh
cargo build --workspace
cargo test --workspace                            # no network, PipeWire, display or sign-in needed
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo run -p app-desktop                          # the desktop app, binary `opium`: on a throwaway profile
cargo build --release -p app-desktop              # target/release/opium
nix build                                         # the packaged desktop app: result/bin/opium
```

The first build is slow: `wreq` builds BoringSSL from source, and FFmpeg's bindings run bindgen.

A crate's unit tests live in it, beside the code (`mod tests`, or a folder such as
`crates/synced/src/tests.rs` or `crates/cenc/src/tests/`); a test that needs a process of its own
(it sets the environment, or starts a bus) goes under `crates/<crate>/tests/`. One test alone:
`cargo test -p <crate> <part of its name>`.

### The phone's build

In `nix develop .#android`:

```sh
./android/build-core.sh                           # the core for arm64 and its Kotlin bindings
(cd android && gradle assembleRelease)            # android/app/build/outputs/apk/release/app-release.apk
(cd android && gradle testDebugUnitTest)          # the JVM tests of android/app/src/test/
```

- `build-core.sh` builds `opium-ffi` with `cargo-ndk` for arm64-v8a at API level 34, into
  `target-android`, then generates the Kotlin bindings with UniFFI into the app's generated
  sources.
- `testDebugUnitTest` runs the app's JVM tests, on JUnit 5 with kotlin-test (the owner's go,
  2026-11-10), both for the tests
  only and never in the APK. They check the data layer's decisions that need no Android
  (`data/Rules.kt`), on the computer: no phone, no emulator. They need the bindings
  `build-core.sh` makes. AGP 9 makes unit tests for the debug build only; the task compiles
  classes and builds no APK. `scripts/check.sh android` does not run them.
- Gradle checks every file it resolves, plugins and test libraries included, against its SHA-256
  in [`android/gradle/verification-metadata.xml`](android/gradle/verification-metadata.xml), and
  fails on any other: a file that changed, or one the list lacks. This pins above all the Kotlin
  half of `rustls-platform-verifier`, served from a GitHub branch that can move. When a dependency
  is added or changes version, write the file again over the build and the tests, every task run
  again so that each resolves what it uses:

  ```sh
  (cd android && gradle --write-verification-metadata sha256 --rerun-tasks assembleRelease testDebugUnitTest)
  ```

  Gradle only adds entries: to drop those no longer used, move the file aside first, then run the
  same command. A change to the file is read in review: only the changed dependencies' entries
  move. A new verifier is checked against the `.sha1` its branch publishes beside each file:
  `--write-verification-metadata sha1`, on a copy of the file, gives the SHA-1 of what Gradle took.
- The Gradle project has no wrapper: `gradle` is the shell's. The shell points Gradle at the SDK's
  own `aapt2`, as Gradle's is built for other Linuxes.
- The release build is signed with Android's debug key, as the debug build is, so either installs
  over the other.
- Always build the release APK for the phone: a debug APK is not compiled ahead of time and feels
  like 30 Hz.

### Developer examples

- `cargo run -p lyrics --example find -- <seconds> <artist> -- <title>` asks every lyrics source
  for one song, NetEase and KuGou included (the app leaves them off by default), and prints what
  each gives and how long it took. It needs the network and no sign-in; each site learns the song.
- The others run only with the owner's go ([AGENTS.md](AGENTS.md#never-on-your-own)), `probe`
  because it loads the owner's CDM, the rest because they use the owner's real sign-ins, kept in
  the config folder:
  - `cargo run -p widevine --example probe` loads the browser's CDM and asks it for a licence
    challenge, offline;
  - `cargo run -p apple-music --example am -- …`; `OPIUM_SPATIAL=1` asks for Spatial Audio;
  - `cargo run -p soundcloud --example sc -- …`, which may renew SoundCloud's tokens on disk, under
    the running app;
  - `cargo run -p downloads --features keyring --example spike -- …`.
- Each file's header lists its commands.
- `am play` and `am live` play out loud.
- `am fetch`, `am live` and `sc fetch` write decrypted protected audio to a file: never run them on
  your own.

### Fuzzing

The parsers of untrusted data are fuzzed two ways
([ADR-0030](docs/adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md)):

- deterministic mutation tests inside `cargo test`, at every push: a seeded generator mutates the
  test files, and the parser's steps and allocations are counted, never timed;
- `cargo fuzz` targets, in a crate of their own beside the crate they fuzz (`crates/cenc/fuzz`,
  `crates/apple-music/fuzz`), outside the workspace and on `libfuzzer-sys`, run by hand and weekly
  in CI. `apple-music`'s readers are private: its `fuzzing` feature, off but for this crate,
  exposes entry points to them.

```sh
nix develop .#fuzz -c scripts/fuzz.sh 60           # every target, 60 s each, as CI does with 300
cd crates/cenc/fuzz && nix develop ../../..#fuzz -c cargo fuzz run index_and_make_clear
cargo fuzz run <target> artifacts/<target>/crash-…  # replays a crash, in the same shell
```

A crash leaves its input in the fuzz crate's `artifacts/`; its fix comes with a test in the
crate's own tests. The corpora (`corpus/`) stay local.

### Measuring

The quality bar's measures by hand ([ROADMAP](docs/ROADMAP.md#quality-bar)) are taken with the
scripts in [`scripts/measure/`](scripts/measure/), each described at its head, on the release
build, inside `nix develop`. Their figures go to [`docs/measurements.md`](docs/measurements.md).

```sh
cargo build --release -p app-desktop
wpid=$(scripts/measure/weston.sh opium-measure-0)            # a headless Weston; prints its pid
export WAYLAND_DISPLAY=opium-measure-0
scripts/measure/rest.sh home <profile> <out>                  # wake-ups, CPU and memory at rest
scripts/measure/startup.py <profile> 10 <out>/startup.json   # warm start to the first frame saved
scripts/measure/power-compare.sh <profile> <out>              # GPU power, closed against open
kill -TERM "$wpid"                                            # that pid only
```

- `<profile>` is a [throwaway profile](#a-throwaway-profile)'s folder, the scripts run under
  `unshare --user --map-current-user --net` to keep it off the network; or a
  [copy of the owner's profile](#tests-on-a-copy-of-the-owners-profile) made by
  `scripts/measure/copy-profile.sh`, with the owner's go. The figures at rest recorded before the
  stage 1 audit were taken online; offline Opium wakes more
  ([measurements](docs/measurements.md#1-at-rest-cpu-time-and-wake-ups)): compare like with like.
- `OPIUM_BIN` names the binary `run.sh` starts (`target/release/opium` by default).
- The scripts end each test instance by SIGTERM to its own pid, and refuse to start while another
  Opium runs. A run ended so never reaches the app's own exit, so it writes no setting that only
  closing saves. Never close a window through the compositor: the command can reach the owner's
  focused window.
- `startup.py … map` times the window's mapping from Umbriel's event stream instead, on the
  desktop.
- The window's frames, its start to the first useful frame and its memory while it scrolls and
  animates are taken by [the measuring mode](#the-measuring-mode), on a library that
  `scripts/measure/library.py` generates, the same every time.
- The phone's start, frames and memory under Compose are taken by `scripts/measure/phone.py`, over
  adb ([the phone](#the-phone-over-adb)); it plays nothing.
- The CDM's costs (a licence, a decrypt, wake-ups at rest) and Apple Music's steps (the developer
  token, the library, a station's batch, a song ready) are taken apart from the window, by
  `scripts/measure/cdm-cost.sh` running builds of `crates/apple-music/examples/cdm_cost.rs` or
  `am_cost.rs` in turn. Each run asks Apple for a licence on the owner's account, and `am_cost`
  for a station's batch, which moves that station's place: with the owner's go only.

### The measuring mode

M1.14's instruments (`crates/app-desktop/src/measure.rs`, removed with egui): the window records
every frame and runs a scenario on its own, then closes. The scroll is injected into egui's own
input; nothing reaches the compositor.

| Variable | Effect |
|---|---|
| `OPIUM_MEASURE=<scenario>` | `start`, `scroll-songs`, `scroll-albums` or `animate`; the window asks to be full screen |
| `OPIUM_MEASURE_OUT=<file>` | The frame record, written when the scenario ends |

```sh
cargo build --release -p app-desktop
scripts/measure/library.py <profile> 8500 1.5           # once, for a display at scale 1.5
WAYLAND_DISPLAY=wayland-0 OPIUM_MEASURE_PIXELS=3840x2160 \
    scripts/measure/frames.py <profile> start 1 <out>    # the first run scans the folder: drop it
WAYLAND_DISPLAY=wayland-0 OPIUM_MEASURE_PIXELS=3840x2160 \
    scripts/measure/frames.py <profile> scroll-songs 3 <out>
```

- `frames.py` runs the scenario off the network and sums up each run's record: the start, each
  phase's CPU times and intervals, the frames missed, the memory. Its head says how. With
  `--again` in place of the profile it runs nothing and sums up again the files left in `<out>`.
- `library.py` makes 8,500 songs in 647 album folders with their covers, which the app shows as 641
  albums, and sets the interface's scale so that the window draws at 1.65 pixels a point, the
  owner's, on a display of the scale given: 1.5 for the owner's, 1 (the default) for a headless
  Weston. `frames.py` leaves out a run drawn at another scale.
- What uwui repeats, to be compared ([the figures](docs/measurements.md#the-figures-to-beat-on-uwui-m114-2026-11-07)):
  - the conditions: the library of `library.py`, nothing played and no song shown, the Stock theme,
    the window full screen on the same display, at 1.65 pixels a point, no variable refresh, the
    GPU's power level as it was (`/sys/class/drm/card1/device/power_dpm_force_performance_level`
    read `high`; how it is set on this machine is not recorded), Mesa and Linux as recorded with the
    figures;
  - the scenarios: Songs opened as soon as the cached library is in (the first useful frame),
    then 3 s at rest; `scroll-songs` scrolls Songs down at 4,000 points a second to its end, the
    pointer at the window's middle, the scroll steady, without rubber band, ended when 10 frames
    in a row move nothing; `scroll-albums` opens Albums after the rest, rests 3 s more, and scrolls
    it likewise; `animate` takes a step each second: Albums, Artists, Home, Songs four times, the
    queue drawer opened and closed three times, Now Playing likewise;
  - the driving: `OPIUM_MEASURE=<scenario>` and `OPIUM_MEASURE_OUT=<file>`, as `run.sh` and
    `frames.py` pass them;
  - the record, as `frames.py` reads it: `#` lines `key=value`, among them `pixels_per_point`,
    `window_points` and `window_pixels` (`WxH`), `ended` (`done`, or `end` for a scroll at its end),
    `undrawn` (0), `scrolled` (the points scrolled in all) and, for `scroll-songs`, `rows_points`
    (the height of Songs' rows, in points): a `scroll-songs` run whose `scrolled` does not reach
    the end of those rows is left out (`frames.py`'s `scroll_reach`, tested by
    `scripts/measure/test_frames.py`, which `scripts/check.sh` runs); then the
    line `start_us,cpu_us,phase,continued,scrolled` and one line a frame drawn: its start in
    microseconds of the Unix epoch; its CPU time, the work of the interface, of turning it into
    triangles and of the drawing calls, without the GPU nor the wait for the display (egui's
    `IntegrationInfo::cpu_usage`); its phase: `loading` before the library, `useful` for the first
    useful frame, `settle` for the 3 s at rest after it, `albums` for the Albums grid's 3 s at rest,
    `scroll` while a scroll goes, `pages`, `drawer` or `now-playing` for `animate`'s steps (the
    frame that takes a step already in its new phase), and `end` for the last; 1 if the frame was
    asked for at once by the pass before (egui's `requested_repaint_last_pass`), else 0; the points
    scrolled so far. `measure.rs`, removed with egui, stays in the history (`git log --
    crates/app-desktop/src/measure.rs`).
- Frame times are taken on the owner's 4K display (DP-3, 240 Hz, scale 1.5) with the owner's go:
  the window covers it for the run. Not in a headless Weston, whose pacing misses frames of its own
  ([pitfalls](#known-pitfalls)). Start and memory can be taken in either.

### The screenshot mode

The desktop app saves its own window once it has had time to load, then quits
(`crates/app-desktop/src/screenshot.rs`). It compares the image to nothing: look at it.

| Variable | Effect |
|---|---|
| `OPIUM_SCREENSHOT=<file.png>` | Turns the mode on; the image's path |
| `OPIUM_SCREENSHOT_AFTER=<seconds>` | The wait before the capture; 8 by default |
| `OPIUM_SCREENSHOT_PAGE=<page>` | Opens a page first: `playlist:<index>`, `library-artist:<index>`, `album:<id>`, `artist:<id>` (Apple Music ids), `search:<query>`, `shelf:home\|new\|radio:<title>`, `page:home\|new\|radio\|albums\|artists\|recent\|songs\|settings\|favourites` |
| `OPIUM_SCREENSHOT_DRAWER` | Opens the queue drawer |
| `OPIUM_SCREENSHOT_LYRICS` | Opens the lyrics page |
| `OPIUM_SCREENSHOT_NP` | Opens Now Playing |
| `OPIUM_SCREENSHOT_SIGNAL` | Opens the signal path's popover, under the top bar's button |
| `OPIUM_SCREENSHOT_DOWNLOADED` | Shows only the downloads in Songs |
| `OPIUM_SCREENSHOT_PLAY=<row>` | Plays a Songs row once the library loaded: **sound, ask the owner** |
| `OPIUM_SCREENSHOT_STATION=<ra.…>` | Tunes in to a station: **sound, ask the owner** |
| `OPIUM_SCREENSHOT_DOWNLOAD` | Downloads the album or playlist opened, once loaded |

- `_DRAWER`, `_LYRICS`, `_NP` and `_SIGNAL` act at start even without `OPIUM_SCREENSHOT`, and the
  window then stays until closed.
- `_PAGE`, `_PLAY`, `_STATION` and `_DOWNLOAD` need `OPIUM_SCREENSHOT`; `_DOWNLOADED` acts only
  with `_PAGE`.
- `_PAGE` waits for the playlists: a profile without any, such as `library.py`'s, stays on Home.
  With `OPIUM_MEASURE=scroll-songs`, which opens Songs itself, a capture shows the list as it
  scrolls.

## Running Opium without touching the owner's data

### What the folders do not redirect

The three XDG variables move every file the app keeps. They do **not** move:

- **the session bus**: the MPRIS name, which an older Opium, from before [ADR-0043](docs/adr/0043-one-instance-per-profile-held-by-a-lock.md),
  lets a newer one take; and **the keyring**, whose one item "Opium downloads" holds the key every download is sealed with;
- **the network**: Opium Connect announces the app on the local network and through iroh. A fresh
  profile is a new device on the owner's phone; a copied `connect.json` *is* the owner's computer,
  which merges into the phone's real history, playlists and session;
- **the sound card**: PipeWire plays on the owner's speakers;
- **the Widevine CDM**, looked for under `$XDG_CONFIG_HOME`, in Google Chrome's install folder and
  under `$HOME` (Flatpak browsers, Firefox): a test profile that keeps `HOME` can find and load the
  owner's. Set `WIDEVINE_CDM` to a path that does not exist to load none, as the recipe below
  does, or to the CDM a test needs, with the owner's go.

### A throwaway profile

A test run uses a throwaway profile, no session bus and no network. Inside `nix develop`, with
`WAYLAND_DISPLAY` naming the compositor's socket in `$XDG_RUNTIME_DIR`:

```sh
cargo build --release -p app-desktop
test=$(mktemp -d /tmp/opium-test.XXXXXX)
unshare --user --map-current-user --net env \
    XDG_CONFIG_HOME="$test/config" XDG_CACHE_HOME="$test/cache" XDG_DATA_HOME="$test/data" \
    DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent WIDEVINE_CDM=/nonexistent \
    OPIUM_SCREENSHOT="$test/shot.png" OPIUM_SCREENSHOT_AFTER=6 \
    target/release/opium
rm -rf "$test"
```

- It writes only under `$test`.
- It prints `media controls unavailable: …`, `the keyring that keeps the downloads' key: …` (so no
  downloads) and `opium connect: can't look for devices: …`, which is expected.
- Without the screenshot variables the window stays until closed.
- Checked on 2026-11-05 under the Opium names, with the debug build on the desktop's compositor: the
  capture saved after about 8.6 s, and only `$test` was written (`config/opium/` with
  `connect.json`, `settings.toml` and `shared.json`; `cache/` with fontconfig's and Mesa's caches;
  `shot.png`).
- It ends without writing `settings.toml` if no setting changed and the window was not closed
  (a SIGTERM, the screenshot mode's own end): the settings are saved as they change and on closing.
- Use the release build, as above: in a headless Weston (`scripts/measure/weston.sh`) the debug
  build's capture never fired in 30 s; the release build's may not either
  ([pitfalls](#known-pitfalls)).

### Tests on a copy of the owner's profile

A check that needs the owner's signed-in services uses a **copy of the owner's config folder** as
`XDG_CONFIG_HOME`, **with the owner's go each time**
([AGENTS.md](AGENTS.md#never-on-your-own)): the run talks to the real services, and may renew the
owner's tokens. `scripts/measure/copy-profile.sh` makes the copy. Only this way:

- run as above but without `unshare … --net`, as the services need the network; the session bus
  still pointed nowhere;
- only while the owner's Opium is closed ([below](#the-owners-own-window)), and never while another
  agent's test runs;
- `connect.json` deleted from the copy before the run and after it: the run then shows on the
  owner's phone as a new device, never to be paired;
- `muted = true` in the copy's `settings.toml`, replacing the key if it is there
  ([settings care](#settings-care));
- `WIDEVINE_CDM` set to the browser's `libwidevinecdm.so`;
- no change made in the app: everything there acts on the real accounts;
- the copy deleted when done: it holds the owner's tokens
  ([ADR-0013](docs/adr/0013-secrets-and-what-is-kept-on-disk.md)).

SoundCloud's tokens last about a day and the copy may renew them. Tell the owner if it did: their
own may then need a new sign-in.

### The owner's own window

The owner allows closing their Opium for a test, only gracefully and started again after:

```sh
pgrep -ax opium                       # whose? the owner's installed program, or a build in a tree
playerctl --player=opium quit         # MPRIS Quit: it saves, then ends within 5 s (ADR-0021)
pgrep -x opium || echo closed
# … the test …
setsid -f <the program pgrep showed> >/dev/null 2>&1   # started again, detached
```

- Never `kill` it: killing skips what it saves on closing.
- A build running from a working tree may be another agent's test: leave it, and ask.

### Settings care

- `settings.toml` is rewritten whole from memory on every change, so an edit made while the app
  runs is lost. A save that fails says why on standard error.
- A file that does not read whole (not UTF-8, a typo, a value this build does not know), or cannot
  be read at all, is moved aside at the start, as `settings.toml.bad`, or `settings.toml.bad.1`,
  `.2`… when that name is taken; standard error names the file used. Its settings that read, or
  the defaults for a file that could not be read, are written in its place at once. A file that
  cannot be moved aside is never written over while the app runs.
- Keys the build does not know, a newer build's, are kept and written back unchanged.
- A music folder whose name is not UTF-8 is written as a `file://` address, its bytes escaped. A
  build older than M1.15 reads it as a folder of that name, which does not exist, and keeps it.
- A `volume` that is not a finite number is dropped (full volume), one outside 0 to 1 is held to it.
- To change a key in a test profile, replace it; never add a second one. A duplicate key once made
  the whole file unreadable, and the user's settings were lost.
- Take a backup before any edit.

## The phone over adb

The phone is the owner's daily device, reached over `adb` only with the owner's go.

- Install the release build only:
  `adb install -r android/app/build/outputs/apk/release/app-release.apk`.
- Then check that `adb shell dumpsys package io.github.honeyslatt.opium | grep pkgFlags` shows no
  `DEBUGGABLE`.
- Before each batch of taps (`adb shell input`), right before it, check that Opium is the focused
  app: `adb shell dumpsys window | grep mCurrentFocus`. One check per batch: the owner may take the
  phone back.
- Never play a song there: it replaces the owner's queue.

## When the app hangs on exit

Closing the app sometimes hung, the window gone and the process left, which then kept a new start
from opening. The cause is not found ([ADR-0021](docs/adr/0021-the-exit-watch.md)).

- The exit watch ends the process five seconds after everything is saved. It appends where the
  teardown stood to `exit.log` in the cache folder, then aborts
  ([ADR-0024](docs/adr/0024-the-exit-watch-aborts-for-a-core-dump.md)). Opium turns core dumps
  off at start, as one would hold its secrets
  ([ADR-0045](docs/adr/0045-no-core-dump-through-the-core-size-limit.md)): the abort leaves no
  dump unless the app was started with `OPIUM_CORE_DUMPS=1`, which only a throwaway profile may
  use.
- Read `exit.log` first, then, from a run on a throwaway profile with `OPIUM_CORE_DUMPS=1`, the core
  dump, before guessing:

  ```sh
  coredumpctl list opium
  coredumpctl gdb opium                 # then `thread apply all bt`
  ```

- One hang seen was still inside eframe's teardown, with the Mesa OpenGL driver's threads alive.

## Known pitfalls

- **The owner's display runs at a fractional scale (1.5).** Feathered edges and the covers' sizes
  are computed in physical pixels: check a visual change at 1.5 as well as at 1.
- **Exclusive audio breaks WirePlumber.** A program that took the card through ALSA `hw:` once left
  WirePlumber without it; `systemctl --user restart wireplumber` brought it back, at another volume.
  Check `wpctl status` after any audio experiment.
- **`pgrep -f opium` matches shells** whose command line merely contains the word, yours included.
  Use `pgrep -ax opium`, which matches the process's name, and read the path it prints.
- **The owner's shell is zsh**: an unquoted `$var` is not split into words, and a word starting with
  `=` is replaced by a command's path (`echo ====` fails). Quote, or run scripts with bash.
- **A test build started on a profile where Opium already runs exits at once**: it raised the
  running window, or says the bus did not reach it ([ADR-0043](docs/adr/0043-one-instance-per-profile-held-by-a-lock.md)). On a throwaway profile it runs beside
  the owner's.
- **A test `XDG_CONFIG_HOME` does not hide the owner's Widevine CDM**: the search also looks in
  Chrome's install folder and under `$HOME`. `WIDEVINE_CDM=/nonexistent` loads none.
- **The screenshot mode may never save**: it waits for frames the compositor may not send. Its
  record of runs is in [the debt](docs/debt.md); the one setting seen to work is a headless Weston at
  3840x2160 under its kiosk shell (`scripts/measure/weston.sh <socket> 3840x2160 <mHz> kiosk`). To end a test run by the app's own exit path, run it with a session bus of its own
  (`unshare --user --map-current-user --net dbus-run-session -- …`) and call MPRIS `Quit` on that
  bus.
- **A headless Weston paces transitions badly**: it adds late frames of its own where the owner's
  display adds none ([measured](docs/measurements.md#the-desktop-window-under-egui)). Take frame
  times on a real display.
- **The 4K display is DP-3 since 2026-11-07** (DP-2 in M1.7's figures): read `umbriel outputs`.
- **End a test process by its own pid**, never by name: `pkill -n weston` once ended another
  agent's headless Weston.
- **`unshare -rn` breaks D-Bus's authentication**; to cut the network but keep the bus, use
  `unshare --user --map-current-user --net`.
- **Opium Connect's tests listen on `0.0.0.0:47820` when the port is free**, reachable from the
  local network for their few seconds.
- **The session is shared with the phone**: an instance that shares the owner's device identity and
  stops a song overwrites the phone's newer stop.
- **Signing out deletes the tokens and the sign-in browser's profile**
  ([ADR-0003](docs/adr/0003-sign-in-through-the-users-browser.md)).
- **egui paints layers drawn without an `Area` before the others**: a layer drawn over the panes
  must be registered (`ui::register_layer`).
- **`color::mix(a, b, p)` weights `a` by `p`.**

## Making a change

1. Pick the current milestone in [`docs/PLAN.md`](docs/PLAN.md).
2. Make the change, with its tests and its documentation.
3. Run `scripts/check.sh`; run the app on a [throwaway profile](#a-throwaway-profile); measure what
   the milestone touches against the [quality bar](docs/ROADMAP.md#quality-bar), into
   [`docs/measurements.md`](docs/measurements.md).
4. Commit one logical change at a time, in English, subject in the imperative.
