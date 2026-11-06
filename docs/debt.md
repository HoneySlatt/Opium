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
- **`unsafe` without `// SAFETY:`**: `File::from_raw_fd` (`opium-ffi/src/lib.rs`).
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
  - Played with the owner on 2026-11-05: an Apple Music song in stereo, one in Spatial Audio, and
    the live radio Apple Music 1, which first failed on an empty `senc` the specification called
    malformed, fixed through the clean room ([evidence](audits/2026-11-05-cenc-clean-room.md)).
    SoundCloud's protected stream is not played yet: the album tried was served as previews only,
    the account having no Go+.
  - The new parser is stricter than the old: a `tenc` whose `default_isProtected` is 0, or stray
    bytes after a sample entry's boxes, are errors. Apple's songs, Spatial Audio and the radio
    played; other files of theirs may still meet them.
  - A downloading stream makes the file clear again at each new fragment, over every fragment
    indexed: time in the square of the fragments. 0.45 ms in all for a four-minute song, 0.76 s
    over a four-hour download, its last calls 0.24 ms each under the stream's lock
    ([measured](measurements.md#indexing-an-encrypted-track-m110-2026-11-05)). A way to make
    clear only the fragments just indexed needs a public item more in `cenc`.
  - The desktop's about screen still credits Kopuz for CENC code; it is frozen
    ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)), and M1.13 corrects it.
- **Left by M1.11** (`widevine` written again; [evidence](audits/2026-11-05-widevine-clean-room.md)):
  - Checked with the owner's CDM: loading, initialising, a licence request, the figures before
    and after ([measured](measurements.md#the-cdm-m111-2026-11-05)); played with the owner, on
    the owner's profile: Apple Music in stereo and in Spatial Audio, the live radio. Not yet:
    SoundCloud's protected stream (above). A licence now waits for the exact key id its decrypts
    use: a wrong one costs ten seconds before "the CDM did not take the licence".
  - The live radio's rotated keys now ask for the key id their PSSH names; whether Apple's
    stations rotate keys is not known: none rotated while the radio played.
  - Each licence logs its expiry twice ("now expires at …", then "at 0"): two lines a song on
    standard error, more than the log needs.
  - A buffer the CDM sets on a failed decrypt is not destroyed: the header says to ignore it, and
    the clean side chose a rare leak over a possible double free.
  - Quitting right after the CDM is loaded takes up to 0.85 s more once the window has closed:
    the CDM's teardown waits for a thread of its own. Kept whole, the owner's choice.
  - One wake-up a second at rest while a session is open, the CDM's own timer fired in time;
    accepted by the owner. Closing a paused song's session would end it, at the cost of a
    licence when it resumes.
  - What to tell the user when the CDM is not provisioned is not known yet: the error says only
    "not provisioned".
  - The C++ is compiled with warnings as errors: a newer compiler than the flake's could stop the
    build on a new warning.
  - Opening a session and giving a licence block the async task that calls them, a tokio worker
    with it, 75 to 340 ms, and ten seconds when a licence never gives the key wanted. Older than
    M1.11; the worst case is new.
  - `Cdm::load` answers Ok once loaded even if the `cdm` thread has since ended; the next call
    fails with "the CDM thread ended". `give_licence` and `decrypt` leave "not initialised" out of
    their documentation, and "the CDM closed the session" is said when the host closed it.
- **The screenshot mode may never save**: on 2026-11-05 the release build saved in one of nine
  runs, on the desktop or in a headless Weston, old code and new alike; it waits for frames the
  compositor may not send. The cause is not known. The visual checks of the definition of done and
  M1.14 lean on it; frozen with `app-desktop` ([pitfall](../CONTRIBUTING.md#known-pitfalls)).
- **Left by M1.12** (`apple-music`'s derived items written again;
  [evidence](audits/2026-11-06-apple-music-clean-room.md)):
  - Not played yet on the new code: Apple Music in stereo and Spatial Audio, the live radio, a
    library song, a station; due with the owner. Measured on the owner's account, without sound.
  - `take_licence` has no test: it needs a CDM session, which tests never load.
  - ADR-0030's `cargo fuzz` targets for the HLS playlists and Apple's answers are not made: the
    readers are private, and a target needs a public seam. Seeded mutation tests in `cargo test`
    cover them.
  - Near the developer token's expiry, while Apple's web player is out of reach, every request
    tries to read it again first: a renewal that meets a network error is not recorded. The same
    holds on a 401, older than M1.12.
  - The token's search takes 4.5 µs more once warm, accepted by the owner.
  - A station's calls still in flight when enough songs came are dropped: their songs are lost,
    the station's cursor moved past them, as the rounds of four did before.
