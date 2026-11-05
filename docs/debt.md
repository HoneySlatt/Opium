# Known debt

What is known to fall short, each item short: why it stays, and where it is tracked. The current
work is in [`PLAN.md`](PLAN.md); a stage's audit adds what it leaves open.

- **Opium wakes 8.4 times a second at rest**, its window's threads aside
  ([causes](measurements.md#1-at-rest-cpu-time-and-wake-ups)): the idle `blocking` thread, the
  player's idle timeout and Opium Connect are shared work; the theme watch's poll, in
  `app-desktop`, waits for uwui.
- **The player cannot wake the window**: the window asks it for its state every 250 ms while music
  plays. The player's `Wake` is shared work; the window's side waits for uwui.
- **Waiting for uwui**, as `app-desktop` is frozen
  ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)):
  - Space toggles playback again on each key repeat while held (`ui/mod.rs:922`);
  - files dropped on the window do nothing under Wayland, where winit 0.30 has no drag and drop;
  - Opium cannot be driven by an agent: nothing scrolls, clicks or types;
  - a comment says the cookie turns in twenty seconds; it turns in 14 (`ui/fab.rs:28`);
  - animations that go on without the focus (`ui/lyrics.rs`, `ui/loading.rs`);
  - one thread per cover file, unbounded (`ui/thumbs.rs`); WebP and GIF covers not decoded;
  - no Korean among the fallback fonts (`fonts.rs`); no colour emoji;
  - no screen-reader names, focus ring or Tab order;
  - the exit hang inside eframe's teardown ([ADR-0021](adr/0021-the-exit-watch.md),
    [ADR-0024](adr/0024-the-exit-watch-aborts-for-a-core-dump.md));
  - the interface's state and logic mixed with egui in `app.rs`, taken out as part of the move.
- **The dependencies are not checked**: `cargo deny` finds three advisories against `zstd`
  (RUSTSEC-2026-0391, -0398, -0403), six unmaintained crates and a yanked `async-executor`; a
  passing `deny.toml` would ignore them or update `Cargo.lock`, the owner's call at Stage 2.
- **`unsafe` without `// SAFETY:`**: `File::from_raw_fd` (`opium-ffi/src/lib.rs`); the CDM shim's,
  until M1.11.
- **Public items without documentation**: not counted yet.
- **Left on purpose by the [2026-10-31 audit](audits/2026-10-31-full.md)**: SoundCloud's clear HLS
  joined whole before it plays; the player's real-time scratch buffer reallocated, 32-bit sources
  through `f32`, a hang on closing during a stuck read.
- **The phone, from the same audit** (ROADMAP, Stage 3): Opium Connect's node built on the main
  thread, which is system work; the page tint recomposing for 450 ms, the whole app recomposing,
  accessibility labels and touch targets, which wait for uwui on the phone
  ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)).
- **For the first release** (ROADMAP, Stage 2):
  - the APK is signed with Android's debug key;
  - the phone lets cleartext HTTP go to any host;
  - the Gradle project has no wrapper;
  - no list of third-party licences exists: the Material icons of Android Auto's buttons, the
    logos' CC0 and Apache-2.0 and UniFFI's bindings among them
    ([licence audit](audits/2026-11-05-licence.md));
  - the shell's FFmpeg is built with GPL parts, which a distributed binary must not link;
  - the test tone `player/tests/data/tone.opus` has no recorded origin;
  - the examples `am fetch`, `am live` and `sc fetch` write decrypted protected audio to a file.
- **Until M1.10** ([licence
  audit](audits/2026-11-05-licence.md#4-what-kopuz-does-badly-here-and-opium-with-it)), in
  `cenc/src/fmp4.rs`: a box of size 0 stops the walk, leaving a last fragment's samples encrypted
  but handed on as clear; nesting is unbounded (a deep file overflows the stack); counts read from
  the file size loops and allocations up to four billion; a negative data offset wraps; only the
  first track is read; the IV size is guessed; an `mdat` must follow its `moof`, and the `tfhd`'s
  base offset is ignored; reads are checked against the buffer, not the box; an `senc` whose count
  differs reuses the last IV, and missing subsamples become "whole sample encrypted"; subsamples
  larger than their sample go to the CDM; `saiz`/`saio` without `senc` decrypt to noise silently;
  AAC's `roll` groups are relabelled `free` with the encryption's.
- **Until M1.11**, in `widevine`: the CDM's "no key" is reported as a decrypt failure; a CDM whose
  start failed passes for loaded at the next attempt; the CDM's timers fire in a loop, not in time;
  a session's close is not checked; CDM versions are sorted as text; promise ids are not matched,
  key statuses and the CDM's rejection messages are dropped, and `RequestStorageId` gets no answer;
  the library is loaded `RTLD_GLOBAL` and the instance never destroyed; 14 `unsafe` without
  `// SAFETY:`.
- **Until M1.12**, in `apple-music`: a download without a `Content-Length` fails the track.
