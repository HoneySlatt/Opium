# Architecture

How the code is organised now, for a developer who has never seen it. Decisions and their reasons
are in [`adr/`](adr/README.md); the contracts of each item are in the code's documentation.

Opium is two programs over one Rust core: a desktop app for Linux, drawn with egui, and an Android
app in Kotlin with Jetpack Compose, which calls the core through UniFFI
([ADR-0009](adr/0009-the-android-app-over-the-rust-core.md)). Everything is one Cargo workspace
(`Cargo.toml`, edition 2024, GPL-3.0-or-later) and one Gradle project (`android/`).

## Parts

### The core, shared by both apps

- **`crates/opium-core`**: the model (`model.rs`: tracks, albums, artists, playlists, stations,
  `ItemId` with its `ProviderKind`), the `Provider` trait every service implements
  (`provider.rs`), the play queue and its `Handoff` (`queue.rs`), the best-copy matching
  (`dedupe.rs`, [ADR-0007](adr/0007-one-library-and-the-best-copy.md)), themes
  (`theme.rs` and `assets/themes/`, [ADR-0011](adr/0011-themes-as-toml-files-of-component-variants.md)),
  cover palettes and colour maths, and `Growing`, a reader of a file still downloading. No network,
  no UI. Its `ffi` feature derives the types for UniFFI.
- **`crates/apple-music`**: Apple Music through its web player's private API: the developer
  token from the web player (`auth.rs`), the sign-in (`signin`), library, catalog, search, radio,
  lyrics, animated covers, and playback through `webPlayback`, a Widevine licence and CENC
  (`playback.rs`) ([ADR-0002](adr/0002-apple-music-through-its-web-player.md)).
- **`crates/soundcloud`**: SoundCloud through `api-v2`: the client id read from soundcloud.com,
  tokens and their renewal, originals, HLS streams clear or encrypted (`stream.rs`), and the
  `Browser` seam through which every change goes ([ADR-0012](adr/0012-soundcloud-changes-through-a-browser-seam.md)).
- **`crates/navidrome`**: any Subsonic/OpenSubsonic server, with salted-token requests; files
  streamed as they are.
- **`crates/local`**: the device's own music files as a service: scanning and watching folders,
  tags read and written (`lofty`), covers, `.lrc` lyrics. On the phone, Android's media index fills
  it instead of a scan.
- **`crates/lyrics`**: LRCLIB, NetEase Cloud Music and KuGou, and the LRC format
  ([ADR-0008](adr/0008-the-lyrics-sources.md)).
- **`crates/services`**: the services and files as one: an item goes to its own service, a search
  to all, and what several have shows once.
- **`crates/cenc`**: encrypted fragmented MP4 (ISO BMFF and CENC), written from
  [its specification](specs/fmp4-cenc.md): the movie box (`Movie`), every sample's place and
  decryption inputs in a compact table (`Indexer`, `SampleTable`), the file made to read as a plain
  MP4 (`make_clear`); and `AudioStream`, which reads as the plain MP4 while a CDM decrypts ahead in
  the background. Fuzzed by mutation tests and by `cargo fuzz` (`crates/cenc/fuzz`).
- **`crates/widevine`**: a browser's Widevine CDM found, loaded and driven, written from
  [its specification](specs/widevine-host.md): one thread named `cdm` owns the instance, fires its
  timers in time and serves requests by priority (licences before decrypts), each decrypt one
  sample in place; a thin C++ layer (`shim/cdm_11.cc`) only passes interface 11's calls across a C
  boundary (`shim/boundary.h`). Without C++ on Android, where loading fails
  ([ADR-0004](adr/0004-the-widevine-cdm-borrowed-from-the-browser.md),
  [ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md)).
- **`crates/browser-signin`**: signing in through a Chromium-family browser on a profile of the
  app's own, its cookies read over DevTools pipes ([ADR-0003](adr/0003-sign-in-through-the-users-browser.md)).
- **`crates/synced`**: what the user's devices keep in common (history, Opium's own playlists, the
  session, shared settings), kept on disk and merged field by field
  ([ADR-0014](adr/0014-shared-state-merged-field-by-field.md)).
- **`crates/connect`**: Opium Connect: finding, pairing, encrypted connections, control, handoffs,
  sharing `synced`'s state, lending songs; on the local network and through iroh
  ([ADR-0010](adr/0010-opium-connect-on-the-local-network.md),
  [ADR-0015](adr/0015-away-from-home-through-iroh.md)). Its `ffi` feature exposes it to Kotlin.
- **`crates/downloads`**: the sealed format, the store and its manifest, the download queue, and
  (feature `keyring`) the key in the desktop's keyring
  ([ADR-0017](adr/0017-downloads-sealed-and-tied-to-the-subscription.md)).

### The desktop app

- **`crates/player`**: decoding (symphonia, libopus for Opus) on an engine thread into a ring of
  `f32` frames, and a PipeWire stream that plays it at the source's rate; gapless queueing; a watch
  on the PipeWire graph for the signal path ([ADR-0005](adr/0005-pipewire-at-each-tracks-own-rate.md)).
  Linux only.
- **`crates/motion`**: animated covers, short videos decoded in a loop by FFmpeg into RGBA frames.
- **`crates/desktop`**: the desktop app's workings, apart from how they are drawn
  ([ADR-0019](adr/0019-the-workings-apart-from-the-drawing.md)): `backend` (the services and the
  player's sources on a tokio runtime, `Request`/`Reply`, and `backend/downloading.rs`), `settings`,
  `cache` (the library as last read), `covers` (covers on disk, their signing and fetch queue),
  `mpris` (media controls and the single instance, [ADR-0018](adr/0018-one-instance-through-the-session-bus.md)),
  `devices` (Opium Connect on the desktop), `soundcloud_browser`, `palette`, `exit`
  ([ADR-0021](adr/0021-the-exit-watch.md)). It knows no toolkit: it wakes the window through
  `desktop::Wake`.
- **`crates/app-desktop`** (binary `opium`): the window, drawn with egui on eframe
  ([ADR-0006](adr/0006-egui-for-the-desktop-window.md)): `main.rs` (single instance, window),
  `app.rs` (the interface's state and much of its logic, about 6 300 lines), `ui/` (one module per
  page or part: songs, grids, album and playlist pages, Now Playing, lyrics, settings, the M3E
  toolbar and panes…), `theme.rs`, `fonts.rs`, `icons.rs`, `covers.rs` (egui's cover loader),
  `screenshot.rs` (the screenshot mode). About 29 000 lines.

### The Android app

- **`crates/opium-ffi`**: the one library the phone loads (`libopium_ffi.so`), exporting the
  services, the library, search, downloads (`downloading.rs`), Connect and the shared state as
  UniFFI objects and suspending functions; and the `uniffi-bindgen` binary that writes their Kotlin
  bindings.
- **`android/`**: the Kotlin app (`android/app/src/main/java/io/github/honeyslatt/opium/`):
  `data/` (the core's session, the library, settings, downloads, the vault, the SoundCloud WebView
  seam), `playback/` (ExoPlayer, the media session service, Apple's source with MediaDrm, Android
  Auto's library), `ui/` (Compose screens per theme), `core/Tls.kt` (the platform verifier's setup).
  `android/build-core.sh` builds the core and its bindings into `android/app/build/generated/`.

## Dependencies

Arrows read "depends on". No cycles; development-only dependencies are left out.

```
opium-core, cenc, widevine, browser-signin, player, motion    (none in the workspace)
lyrics, synced, navidrome, downloads ──► opium-core
local                                ──► opium-core, lyrics
apple-music, soundcloud              ──► opium-core, browser-signin, cenc, widevine
connect                              ──► opium-core, synced
services                             ──► opium-core, apple-music, soundcloud, navidrome, local

desktop     ──► opium-core, apple-music, soundcloud, navidrome, local, lyrics, services,
                browser-signin, widevine, player, motion, connect, synced, downloads (keyring)
app-desktop ──► desktop, and the same crates directly
opium-ffi   ──► opium-core (ffi), apple-music, soundcloud, navidrome, local, lyrics, services,
                connect (ffi), synced, downloads
```

Rules that keep it so:

- Only `app-desktop` names egui types; `desktop` and everything below it know no toolkit.
- Nothing `opium-ffi` depends on may need the desktop: `player`, `motion`, `desktop` and
  `app-desktop` stay out of the phone's build, and `widevine` builds no C++ for Android.
- The service crates (`apple-music`, `soundcloud`, `navidrome`, `local`) depend on `opium-core`,
  the leaf crates and `lyrics` only (`local` reads `.lrc` files through it), never on each other
  nor on `services`.

## Main flows

### Start-up of the desktop app

1. `main` looks on the session bus for a running Opium: if one is there, it is raised (or handed
   the files named on the command line) and this process exits.
2. eframe opens the window; `App::new` loads the settings, resolves the theme, opens the shared
   state (`synced::Kept`), starts Opium Connect (`devices::Devices::start`), starts the backend
   with a `Wake` made from egui's context, then the player and the MPRIS server.
3. The backend's thread starts its tokio runtime, makes the private folders private, connects each
   service from what is kept on disk (Apple's token, SoundCloud's tokens, the Navidrome credentials,
   the music folders), and answers with the library as last read (`cache`) before reading it again.

### A song, from a click to PipeWire

1. A double click on a row sets the queue (`opium_core::Queue`) and sends `Request::Fetch` with a
   token for the song.
2. The backend checks the downloads first (`backend/downloading.rs`); otherwise the song's service
   makes it playable: for Apple Music, `webPlayback`, the Widevine licence through the CDM, and a
   `cenc::AudioStream` that decrypts ahead; for Navidrome and SoundCloud's originals, a `Growing`
   reader that plays while the file downloads; for files, the file.
3. It answers `Reply::Fetched` with a `Playable` and wakes the window.
4. The window's `logic()` takes the reply and calls `Player::play` with a `player::Source`.
5. The player's engine decodes into a ring of `f32` frames; its PipeWire stream, which asked the
   graph for the source's rate (`node.force-rate`), plays from the ring.
6. The next song is fetched ahead (`Request::Warm`) and handed over with `Player::set_next`, which
   follows without a gap when the rate and channels match.

### A reply waking the window

Everything off the window's thread (the backend, MPRIS, Opium Connect, the covers' fetches, the
file portals, the theme watch) sends on a channel, then calls its `Wake`. eframe then runs
`App::logic`, which drains each channel (`pump`, `media_controls`, `other_devices`) and updates the
state; `ui()` draws if the window shows. `logic()` runs even while the window is hidden. The player
has no wake: while music plays, `logic()` asks for a frame every 250 ms to read its state.

### Opium Connect

The node (`connect::Node`, on a one-thread tokio runtime of its own) listens on TCP 47820,
announces itself and browses over mDNS, and redials paired devices; away from home it uses iroh.
A connection proves pairing through Noise with the pairing key. On connecting, two devices
exchange digests, then whatever of `synced::Shared` differs; afterwards each change made by
`synced::Kept` goes out as it is made. Events (a device seen, a command, a change received) come
back to the window through a channel and its `Wake`. A handoff carries the queue (`Handoff`) to
the device that takes over.

### A download, sealed and played back

1. Download from a menu: the song joins the queue of `downloads::queue`, which fetches one song at a
   time through the app's `Fetcher` (on the desktop, as playing would, the CDM decrypting).
2. The bytes are written through `sealed::Writer` into a `.part` file of an opaque name, then kept:
   the manifest is written aside and renamed into place.
3. Played back: the backend finds the song in the store, `Store::open_song` checks its standing
   (a DRM song plays while its service's subscription was confirmed in the last 30 days), and the
   player reads it through `sealed::Reader`, which decrypts chunk by chunk and seeks anywhere.
4. Each time Apple Music connects, `AppleMusic::subscribed` confirms the subscription again.

### The Android side

Kotlin creates the core's `Session` (`data/Core.kt`), which holds the services, and calls its
suspending functions; UniFFI runs them on the core's multi-thread runtime. The core answers with
the model's records. ExoPlayer plays: Apple's songs as HLS with the phone's Widevine through
MediaDrm, the licence asked through the core (`playback/AppleSource.kt`); Navidrome's and
SoundCloud's from their addresses; the phone's files from Android's media index. Downloads use the
same `downloads` crate through `opium-ffi`, sealed with a key the vault keeps; Apple's and
SoundCloud's encrypted songs are borrowed from the paired computer (`connect::lend`).

## Where to start reading

1. `crates/opium-core/src/model.rs` and `provider.rs`: what a song, an album and a service are.
2. `crates/desktop/src/backend.rs`: the `Request` and `Reply` enums, then `Backend::start` and
   `playable`.
3. `crates/app-desktop/src/app.rs`: `App::new`, then `logic()` near the end of the file.
4. `crates/player/src/lib.rs` and `output.rs`: what reaches the sound card, and how.
5. For the phone: `crates/opium-ffi/src/lib.rs` and `android/app/src/main/java/io/github/honeyslatt/opium/data/Core.kt`.
