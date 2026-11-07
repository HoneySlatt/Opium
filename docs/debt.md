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
- **What a tag write leaves** (`local::tag_writes`, since 2026-11-07): on the desktop, the file
  replaced by its copy loses its extended attributes, ACLs and owner, and a second hard link to it
  keeps the old tags; a folder the user can't write refuses the copy, so its files' tags are no
  longer written even where the files could be; a crash during a write leaves a whole copy of the
  song in its folder, hidden (`.opium-tags-…`), which nothing removes. On the phone, a write cut
  short is put back only once Android lets Opium open the file again, at a start or at that file's
  next change; the copies of a file deleted meanwhile stay in the app's own folder, and so does the
  original of a file changed since some other way (`<id>.kept`), which nothing shows the user: a
  song a power cut left with blocks of zeros is taken for one, and stays damaged. An undo that fails
  at a start says nothing; the reason shows only in that song's next change. Tracked by Stage 7,
  which reworks tag edits (old copies kept, undo).
- **Two instances can run at once** (seen on 2026-11-07): an Opium started while another held the
  MPRIS name ran on without it, and once the other quit, nobody held the name. Media keys then miss
  it, and a new launch starts a second instance, which overwrites the first's files
  ([ADR-0018](adr/0018-one-instance-through-the-session-bus.md)). Cause not looked for. A risk to
  data, kept as debt by the owner's decision of 2026-11-07; tracked by M1.8, the stage audit.
- **Away from home cannot be turned off**: the desktop always asks for it (`desktop::devices`,
  `set_away(true)`), so n0's relays and address lookup are reached even before any device is paired;
  Settings only changes the relay. The README said it could be turned off until 2026-11-06. It stays
  until the owner decides whether it may be turned off, open in the [ROADMAP](ROADMAP.md#open) for
  Stage 8.
- **CI checks the phone's Rust only**: `scripts/check.sh android` runs clippy on `opium-ffi`; no
  check builds the Kotlin bindings, the Kotlin or the APK, so Kotlin that no longer matches the core
  passes CI. Built by hand (`build-core.sh`, `gradle assembleRelease`). No stage tracks it yet: in
  the PLAN's [Ideas](PLAN.md#ideas), for Stage 2.
- **The dependencies are not checked**: `cargo deny` finds three advisories against `zstd`
  (RUSTSEC-2026-0391, -0398, -0403), six unmaintained crates and a yanked `async-executor`; a
  passing `deny.toml` would ignore them or update `Cargo.lock`, the owner's call at Stage 2.
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
  - Played by the owner on 2026-11-06, on the owner's profile, everything working by the owner's
    report; what was played was not listed one by one. A playlist's station (`ra.cp-pl.u-…`) has
    no button in Opium: its id's acceptance is tested against the local server only.
  - `take_licence` has no test: it needs a CDM session, which tests never load.
  - `fetch_track`'s check of a song's id has no direct test: it needs a CDM. `protected`'s has,
    and both go through `song_and_binaural`, which takes the checked kind.
  - On a 401, while Apple's web player cannot be reached at all, each request reads it again and
    may wait the ten-second connection timeout in turn: a renewal that meets no connection is not
    recorded. Older than M1.12; the reading before `exp` is now bounded.
  - Between encodes of the same bit rate, the first is played, where the code before played the
    last; the answers seen had one of each rate.
  - The token's search takes 1.2 to 1.3 µs more once warm, within the 4.5 µs the owner
    accepted; a process's first, 2 to 12 µs more.
  - Station ids are checked as one path segment: `ra.<number>`, `ra.cp-pl.u-…` and `ra.q-…` pass;
    other forms Apple may use were not met.
  - A string `songId` of `"0"` counts as a catalog id, as §1 reads digits; the code before took
    it too.
  - Two tests lean on their machine: one dates a reading 5 minutes back on the monotonic clock,
    which fails, saying so, on a machine up for less; one counts readings in flight, the second
    started by `join`'s order of polling.
  - A station's calls still in flight when enough songs came are dropped: their songs are lost,
    the station's cursor moved past them, as the rounds of four did before.
- **Left by M1.13** (the whole tree compared with Kopuz;
  [evidence](audits/2026-11-06-kopuz-whole-tree.md)):
  - `DevTools` in `browser-signin` is still adapted from Kopuz, and rests on the EUPL's
    compatibility clause; the owner kept it
    ([ADR-0033](adr/0033-the-browser-sign-ins-pipe-kept-as-adapted-from-kopuz.md)). Another
    licence than the GPL needs it written again first.
