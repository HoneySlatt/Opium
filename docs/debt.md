# Known debt

What is known to fall short, each item short: why it stays, and where it is tracked. The current
work is in [`PLAN.md`](PLAN.md); a stage's audit adds what it leaves open. Triaged whole by the
[stage 1 audit](audits/2026-11-10-stage-1.md) on 2026-11-10; its findings are named by their
number there (L1, S2, W3…), and their failing tests are kept beside it.

## Paid by M1.15

The audit's losses of data, flaws, crashes and broken services, with the small items the owner
asked to pay at once ([M1.15](PLAN.md#m115--the-audits-fixes)):

- **Losses of data**: every download deleted after a manifest put aside (L1); two instances at
  once, which take the MPRIS name from each other and overwrite each other's files, the debt's
  incident of 2026-11-07, whose exact way is not known (L2); settings that stop saving, or are
  replaced without a copy (L3); an unreadable shared state that pushes every device's history out
  (L4); a second file set aside replacing the first, the user never told (L5); a sign-out that
  deletes the service's downloads, against ADR-0017 (L6); the phone's vault, whose failed read
  deletes downloads and sign-ins (L7); a song downloaded again lost on a failed save (L8); the
  playlists' move to their own file (L9); **what a tag write leaves**: a power cut that leaves zeros
  past the old end makes a song stay half written on the phone, its original hidden (L10); the
  settings' unknown keys dropped by a build that goes back (L11, the settings only); the phone's
  themes overwritten by the desktop's (L12); writes not flushed before their rename (L13); the
  phone's caches sharing one partial file per list; a scan's embedded cover lost to a shared
  partial file.
- **Flaws**: the exit watch's core dump ([ADR-0039](adr/0039-the-exit-watch-leaves-no-core-dump-unless-asked.md),
  S1); Navidrome's user and password in kept addresses (S2); SoundCloud's write paths (S3); a
  paired device's playlist id naming a file (S4); the 30 days beaten by a clock set back (S5); the
  phone's TLS verifier fetched from a branch (S7); the zstd advisories and the yanked
  `async-executor`, by one compatible update (S8); ADR-0013 corrected on Navidrome's password (S9).
- **Crashes and broken services**: `volume = nan` (W1); Apple's paging, and SoundCloud's uploads,
  without end (W2); Connect's playlists over the message cap (W3); a long list played on a device
  (W4); an LRC offset and an edit list that overflow (W5); SoundCloud's confirmation lost at start
  (W6); the backend waiting on Apple and SoundCloud (W7); an empty `XDG_CONFIG_HOME` (W8);
  `frames.py`, which passes a scroll that stops short (W12).
- **Small items**: the test tone `player/tests/data/tone.opus`, whose origin is to be recorded (its
  tags say FFmpeg's Opus encoder); each licence logging its expiry twice; `Cdm::load` answering Ok
  once loaded even if the `cdm` thread has since ended, `give_licence` and `decrypt` leaving "not
  initialised" out of their documentation, "the CDM closed the session" said when the host closed
  it; a test that needs five minutes of uptime; two comments pointing into the plan at text that
  left it; the documents the audit found wrong, frozen code's false comments included; the
  nineteen tests the audit wrote for gaps in the tests.

## Waiting for Stage 5, the move to uwui

- **Opium wakes at rest**: 7.4 times a second online, 27 to 33 offline, its window's threads aside.
  iroh, bound even with nothing paired, wakes most
  ([ADR-0040](adr/0040-away-from-home-only-while-a-device-is-paired.md) binds it only while a device
  is paired); the idle `blocking` thread (1.6.2 does not keep one); the player's idle timeout;
  mDNS's address check; the theme watch's poll, in `app-desktop`.
- **Paused after playing**, the PipeWire stream runs on silence, keeping the device awake and the
  graph at the song's rate; a protected song's session and decrypt thread keep waking
  ([ADR-0041](adr/0041-a-paused-protected-songs-session-closes-after-a-long-pause.md)). **Playing**,
  the engine wakes every 10 ms. Neither measured: they need sound.
- **The player cannot wake the window**: the window asks it for its state every 250 ms while music
  plays, or while a paired device plays.
- **The desktop's backend in one function** (`desktop::backend::run`, 949 lines, untested), which
  routes items by hand where `services::Services` does; "write aside, then rename" written 16 times;
  the resuming reader in three service crates; `connect` on `rand` 0.8 for one call; `desktop`'s
  second HTTP client for one cover, fetched with a blocking call in the media controls' loop;
  unused dependencies in `soundcloud`, `local` and `desktop`; 820 public items or fields without
  documentation.
- **Memory at rest**: 202 to 208 MiB, where the budget is 150.
- **Frozen with `app-desktop`** ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)):
  - Space toggles playback again on each key repeat while held (`ui/mod.rs:922`);
  - files dropped on the window do nothing under Wayland, where winit 0.30 has no drag and drop;
  - Opium cannot be driven by an agent: nothing scrolls, clicks or types, but for M1.14's scripted
    scenarios (`measure.rs`), which only measure;
  - a comment gives the turning cover twenty seconds a turn; the cookie takes 14, the toolbar's
    cover 24 (`ui/fab.rs:28`);
  - animations that go on without the focus (`ui/lyrics.rs`, `ui/loading.rs`), animated covers too;
  - one thread per cover file, unbounded, a refused one panicking the window's thread
    (`ui/thumbs.rs`); WebP and GIF covers not decoded; a file's cover read on the window's thread;
  - the window's thread checking every visible file on disk at each frame; the Songs page's costs
    at 100,000 songs and its search cache keyed by the song count;
  - no Korean among the fallback fonts (`fonts.rs`); no colour emoji;
  - no screen-reader names, focus ring or Tab order, no text reaching a screen reader; 50 pairs of
    colours under WCAG AA, in both apps; no reduced motion, transparency or contrast read;
  - favourites on a Navidrome song or a file showing an error;
  - a failed service retried every minute for ever, offline included;
  - the exit hang inside eframe's teardown ([ADR-0021](adr/0021-the-exit-watch.md),
    [ADR-0024](adr/0024-the-exit-watch-aborts-for-a-core-dump.md));
  - the interface's state and logic mixed with egui in `app.rs`, taken out as part of the move.
- **The screenshot mode may never save**: on 2026-11-05 the release build saved in one of nine
  runs; on 2026-11-07 twice in two, in a headless Weston at 3840x2160 under its kiosk shell; on
  2026-11-10 once in two, the packaged build never. The cause is not known. Frozen with
  `app-desktop` ([pitfall](../CONTRIBUTING.md#known-pitfalls)).
- **Text in decomposed Unicode** does not fold for search; `file://localhost/` is read as a
  relative path; a file opened with a name that is not UTF-8 will not open; when mDNS fails to
  start, discovery is never tried again.

## Waiting for Stage 3, the phone on uwui

- **The phone's screens** (frozen, [ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)):
  TalkBack can neither read nor set the position or a device's volume; 69 of 145 tap targets hold
  no text, 44 are under 48 dp; M3E's queue and carousel only by dragging; contrast; "Remove
  animations" honoured in two places; the account button always "H"; frames when moving between
  tabs; the position read four times a second while paused; the page tint recomposing for 450 ms
  and the whole app recomposing.
- **Android Auto** names Repeat "Repeat" in its three states.
- **Opium Connect's node** is built on the main thread, which is system work; paused in the
  background, Connect's timers keep running.
- **The phone cannot play an Apple Music upload**, which the computer plays.

## Waiting for Stage 7, the library

- **Formats without a version**: the shared state's, the pairings' and the caches' (the settings'
  is M1.15's).
- **A library of 100,000 songs**: 606 bytes of the window's heap a song; the files' search folding
  every song again (27 to 36 ms); a Navidrome file or a SoundCloud original held whole in memory
  while it plays.
- **What a tag write leaves**, beyond M1.15's zeros: on the desktop, the file replaced by its copy
  loses its extended attributes, ACLs and owner, and a second hard link to it keeps the old tags; a
  folder the user can't write refuses the copy; a crash during a write leaves a hidden copy of the
  song (`.opium-tags-…`) that nothing removes. On the phone, a write cut short is put back only once
  Android lets Opium open the file again; copies of a file deleted meanwhile stay in the app's own
  folder, and so does the original of a file changed since some other way (`<id>.kept`), which
  nothing shows the user. An undo that fails at a start says nothing. Neither app was used to edit
  a song; the phone's way back was never cut short on a device. Stage 7 reworks tag edits (old
  copies kept, undo).

## For the first release, Stage 2

- **Dependencies**: no `deny.toml`, so CI learns of no advisory, though the weekly run is meant to
  let a project at rest learn of its dependencies' changes (a proposal passes today:
  [`deny.toml`](audits/2026-11-10-stage-1/dependencies/deny.toml)); six unmaintained crates, three of
  them under zbus 5; two TLS providers in both apps, a third on the desktop; the phone's build
  compiling UniFFI's generator, which [ADR-0009](adr/0009-the-android-app-over-the-rust-core.md)
  names (moving it out needs an ADR).
- **Licences shipped**: no list of third-party licences, the Material icons of Android Auto's
  buttons, the logos' CC0 and Apache-2.0 and UniFFI's bindings among them
  ([licence audit](audits/2026-11-05-licence.md)); neither package ships Opium's own GPL text; the
  Nix package ships no licence file; `btls-sys` declares MIT but links BoringSSL, under Apache-2.0.
- **The APK**: signed with Android's debug key; it lets cleartext HTTP go to any host; it ships
  2.25 MiB of iroh libraries that nothing loads; the Gradle project has no wrapper.
- **CI checks the phone's Rust only**: no check builds the Kotlin bindings, the Kotlin or the APK,
  so Kotlin that no longer matches the core passes CI; built by hand (`build-core.sh`,
  `gradle assembleRelease`). With the Gradle wrapper; the PLAN's
  [Ideas](PLAN.md#ideas) hold it.
- **Tests**: no Kotlin test; the player's engine, Apple Music's live radio reader and `opium-ffi`
  untested; SoundCloud's bounded retries untested; Connect's tests listen on port 47820 of the local
  network while they run; no test in release mode; Miri never runs; most parsers of untrusted input
  not fuzzed (only `cenc`'s and `apple-music`'s are).
- **The examples `am fetch`, `am live` and `sc fetch` write decrypted protected audio to a file**,
  against the DRM rules; the owner keeps them until the first release (2026-11-10).
- **FFmpeg**: the shell's is built with GPL parts, GPL-3.0-or-later as a whole, which Opium's
  licence can take; which FFmpeg the packages link is decided at Stage 2.

## Waiting for Stage 8, Connect

- **Away from home cannot be turned off** while a device is paired: the desktop always asks for
  it, and the phone while on screen and for 60 s after; Settings only changes the relay. Whether it
  may be turned off is open in the [ROADMAP](ROADMAP.md#open).

## Left by the clean rooms of M1.10 to M1.13

Watched or decided; each is taken up when its crate is next opened for a reason of its own.

- **`cenc`** ([evidence](audits/2026-11-05-cenc-clean-room.md)): the new parser is stricter than
  the old (a `tenc` whose `default_isProtected` is 0, or stray bytes after a sample entry's boxes,
  are errors); Apple's songs, Spatial Audio and the radio played, other files may still meet them.
  A downloading stream makes the file clear again over every fragment indexed, time in the square
  of the fragments: 0.76 s over a four-hour download
  ([measured](measurements.md#indexing-an-encrypted-track-m110-2026-11-05)); Stage 6.
- **`widevine`** ([evidence](audits/2026-11-05-widevine-clean-room.md)):
  - SoundCloud's protected stream is not played yet: the album tried was served as previews only,
    the account having no Go+ (still due with the owner, in the PLAN).
  - A licence waits for the exact key id its decrypts use: a wrong one costs ten seconds before
    "the CDM did not take the licence".
  - Whether Apple's stations rotate keys is not known: none rotated while the radio played.
  - A buffer the CDM sets on a failed decrypt is not destroyed: the clean side chose a rare leak
    over a possible double free.
  - Quitting right after the CDM is loaded takes up to 0.85 s more: kept, the owner's choice.
  - What to tell the user when the CDM is not provisioned is not known: the error says only "not
    provisioned"; the words are the owner's to choose.
  - The C++ is compiled with warnings as errors: a newer compiler could stop the build on a new
    warning; Stage 2, with Flathub's compiler.
  - Opening a session and giving a licence block the tokio worker that calls them, 75 to 340 ms,
    and ten seconds when a licence never gives the key wanted; with the time to sound, Stage 5.
- **`apple-music`** ([evidence](audits/2026-11-06-apple-music-clean-room.md)):
  - `take_licence` and `fetch_track`'s check of a song's id have no direct test: they need a CDM,
    which tests never load; with a fake CDM, when one exists.
  - On a 401 while Apple's web player cannot be reached, each request reads it again and may wait
    the ten-second timeout in turn; with the time to sound, Stage 5.
  - Station ids are checked as one path segment; other forms Apple may use were not met. A string
    `songId` of `"0"` counts as a catalog id.
  - A test counts readings in flight, the second started by `join`'s order of polling; another, a
    station's budget of calls, failed once on a loaded machine (its server counts a call in flight
    until after its reply).
  - A station's calls still in flight when enough songs came are dropped, their songs lost; Stage 6.
- **`browser-signin`**: `DevTools` is still adapted from Kopuz, under the EUPL's compatibility clause
  ([ADR-0033](adr/0033-the-browser-sign-ins-pipe-kept-as-adapted-from-kopuz.md)).

## Left on purpose by the 2026-10-31 audit

SoundCloud's clear HLS joined whole before it plays; the player's real-time scratch buffer
reallocated past 8,192 frames a channel; no stream cache on the phone (addresses signed per
request, live playlists, a wrong key would play the wrong song). Stage 6, where the sound is
checked.
