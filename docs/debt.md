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
- **Left by M1.10** (`cenc` written again; [evidence](audits/2026-11-05-cenc-clean-room.md)):
  - Behaviour is checked on built files and on one file of FFmpeg's, never yet on Apple's or
    SoundCloud's: their protected songs, Spatial Audio and the live radio are played with the
    owner after M1.11 and M1.12 ([PLAN](PLAN.md#m110-to-m112--kopuzs-code-written-again)).
  - The live radio's segments once fell back on the playlist's IV when the init gave none; a
    protected sample without a constant IV or a per-sample IV is now an error, as the standard
    asks. Whether Apple's stations ever relied on it is unknown until the radio is played.
  - The new parser is stricter than the old: a `tenc` whose `default_isProtected` is 0, or stray
    bytes after a sample entry's boxes, are errors; Spatial Audio's entries are made one only when
    they share one `AudioSpecificConfig`. Any of them in Apple's files would stop a song, which
    playing them with the owner shows.
  - A downloading stream makes the file clear again at each new fragment, over every fragment
    indexed: time in the square of the fragments. 0.45 ms in all for a four-minute song, 0.76 s
    over a four-hour download, its last calls 0.24 ms each under the stream's lock
    ([measured](measurements.md#indexing-an-encrypted-track-m110-2026-11-05)). A way to make
    clear only the fragments just indexed needs a public item more in `cenc`.
  - The CDM still returns a new buffer (`widevine`), copied into the stream's: decrypting in place
    saves its copy once M1.11 gives the CDM's host an in-place call.
  - The desktop's about screen still credits Kopuz for CENC code; it is frozen
    ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)), and M1.13 rewrites it.
- **Until M1.11**, in `widevine`: the CDM's "no key" is reported as a decrypt failure; a CDM whose
  start failed passes for loaded at the next attempt; the CDM's timers fire in a loop, not in time;
  a session's close is not checked; CDM versions are sorted as text; promise ids are not matched,
  key statuses and the CDM's rejection messages are dropped, and `RequestStorageId` gets no answer;
  the library is loaded `RTLD_GLOBAL` and the instance never destroyed; 14 `unsafe` without
  `// SAFETY:`.
- **Until M1.12**, in `apple-music`: a download without a `Content-Length` fails the track.
