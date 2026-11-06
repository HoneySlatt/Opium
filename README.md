# Opium

A music player for the computer and the phone that puts Apple Music,
SoundCloud, Navidrome and your own music files in one library, in the spirit
of Apple Music and [Cider](https://cider.sh). The desktop app is written in
Rust with [egui](https://github.com/emilk/egui); the Android app in Kotlin
with Jetpack Compose, over the same Rust core.

## State

- Not released yet: the first release is the [roadmap](docs/ROADMAP.md)'s
  Stage 2. In daily use by its author on one computer and one phone.
- The computer: Linux only. The phone: Android 14 or later, arm64.
- The desktop window and the phone's screens are frozen until they move to
  uwui, the owner's UI framework, fixes for data loss, crashes and security
  aside ([ADR-0026](docs/adr/0026-the-egui-window-frozen-until-the-move.md),
  [ADR-0031](docs/adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)).
- The APK is signed with Android's debug key.
- On standby until SlimeEngine's Stage 6d plans its ports, Opium's first
  ([ADR-0031](docs/adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)).
- The code once adapted from Kopuz was written again in a clean room, but for part of the
  browser sign-in, kept as adapted
  ([ADR-0030](docs/adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md),
  [ADR-0033](docs/adr/0033-the-browser-sign-ins-pipe-kept-as-adapted-from-kopuz.md),
  [evidence](docs/audits/2026-11-06-kopuz-whole-tree.md)). Opium stays under GPL-3.0-or-later
  ([ADR-0032](docs/adr/0032-the-gpl-kept-and-kopuzs-code-still-written-again.md)).
- What is being worked on: [docs/PLAN.md](docs/PLAN.md).

- **One library, one search:** albums, songs, artists and playlists from every
  service share the same pages. A song or album you have on several services
  shows once, and plays from the best copy: your own file, then your Navidrome
  server's, then Apple Music, then SoundCloud.
- **Your files:** choose music folders in Settings and their FLAC, ALAC, WAV,
  AIFF, MP3, AAC, Ogg Vorbis and Opus files join the library, with their tags
  and covers, watched as they change. Their tags can be edited, as in iTunes.
  A file opened from the file manager, the command line or dropped on the
  window plays.
- **Sound:** each track plays at its own sample rate through PipeWire, without
  resampling, while other apps' sound still plays. You get Apple Music's AAC
  256 and its Spatial Audio for headphones, SoundCloud's original uploads when the artist allows downloads, and
  your server's files as they are.
- **Lyrics:** synced lyrics from the song's own source first: Apple Music's,
  word by word; a Navidrome server's; a file's `.lrc` beside it, or its tags.
  Otherwise from lyrics sites, line by line: LRCLIB, and NetEase Cloud Music
  and KuGou if you turn them on in Settings.
- **Desktop:** a Now Playing view, a queue, MPRIS (media keys and desktop
  widgets), and keyboard shortcuts.
- **Opium Connect:** paired devices share the play history, Opium's own
  playlists and where the music stopped, and each can work the other's
  player. On the same network they find each other over mDNS and talk on TCP
  port 47820 (so allow 47820/tcp and 5353/udp through the firewall); away
  from home, they reach each other through [iroh](https://www.iroh.computer)'s
  relays, with the same encryption, which you can turn off or point at your
  own relay in Settings → Devices.
- **Offline:** songs can be downloaded, kept sealed with a key held in the
  desktop's keyring or the phone app's vault, and a song that came through a service's DRM plays while
  that service's subscription was confirmed in the last 30 days.

## Requirements

- Linux, with **PipeWire**: it is the only audio output, so without it
  nothing plays.
- A Chromium-family browser (Brave, Chromium, Chrome, Vivaldi or Edge) to sign
  in to Apple Music and SoundCloud. The app opens it on a profile of its own.
  Set `OPIUM_BROWSER` to choose one.
- Apple Music and SoundCloud's protected streams need the Widevine CDM that
  Chrome, Chromium, Brave, Vivaldi or Firefox installs. Play any DRM video once in that
  browser so the CDM gets its certificate. Set `WIDEVINE_CDM` to point at a
  `libwidevinecdm.so` elsewhere.
- An Apple Music subscription for Apple Music, a SoundCloud account, and a
  [Navidrome](https://www.navidrome.org) server (or any OpenSubsonic server),
  as you like. Each service can be turned off in Settings.
- For the phone: Android 14 or later on an arm64 device. Android 14 is the
  first that plays bit-perfect over USB, which is the point of the app.

## Build

With Nix, from a clone of the repository:

```sh
nix run .
```

(`nix run github:HoneySlatt/Opium` works too for those with access to the
repository, which is private.)

Or with Cargo. Besides a Rust toolchain you need `pkg-config`, a C and C++
compiler (the Widevine shim is C++), FFmpeg, PipeWire and libopus to link
against, and `cmake`, `perl` and `git` to build BoringSSL from source:

```sh
cargo build --release -p app-desktop
./target/release/opium
```

Settings live in `~/.config/opium`; covers and animated artwork are cached
in `~/.cache/opium`; downloaded songs in `~/.local/share/opium`.

### The Android app

The Rust core is shared through UniFFI, so it is built first, then the APK:

```sh
nix develop .#android
./android/build-core.sh
cd android && gradle assembleRelease
```

The APK is in `android/app/build/outputs/apk/release`. It is **signed with
Android's debug key** for now, so it is not fit to hand to anyone else.

## Development

[CONTRIBUTING.md](CONTRIBUTING.md) gives the environment, the commands and
the checks in detail. The direction, the current plan and the decisions are
in [docs/ROADMAP.md](docs/ROADMAP.md), [docs/PLAN.md](docs/PLAN.md) and
[docs/adr](docs/adr/README.md); the code's layout in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Credits

- [Kopuz](https://github.com/Kopuz-org/kopuz) (EUPL-1.2), which found the way to
  Apple Music's web player. Part of the browser sign-in
  (`crates/browser-signin`) is adapted from its code.
- [Geist](https://vercel.com/font) fonts (SIL Open Font License),
  [Lucide](https://lucide.dev) icons (ISC), the SoundCloud and Apple Music marks
  from [Simple Icons](https://simpleicons.org) (CC0), and the Navidrome logo
  from [its website](https://github.com/navidrome/website) (Apache-2.0).

Opium is not affiliated with Apple, SoundCloud or Navidrome. Their names and
logos belong to their owners.

## Licence

GPL-3.0-or-later. See [LICENSE](LICENSE).
