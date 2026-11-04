# Music

A music player for the computer and the phone that puts Apple Music,
SoundCloud, Navidrome and your own music files in one library, in the spirit
of Apple Music and [Cider](https://cider.sh). The desktop app is written in
Rust with [egui](https://github.com/emilk/egui); the Android app in Kotlin
with Jetpack Compose, over the same Rust core.

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
  256, SoundCloud's original uploads when the artist allows downloads, and
  your server's files as they are.
- **Lyrics:** synced lyrics, word by word from Apple Music, line by line
  otherwise. They come from the service, else from LRCLIB, NetEase Cloud Music
  or KuGou.
- **Desktop:** a Now Playing view, a queue, MPRIS (media keys and desktop
  widgets), and keyboard shortcuts.
- **Music Connect:** paired devices share the play history, Music's own
  playlists and where the music stopped, and each can work the other's
  player. On the same network they find each other over mDNS and talk on TCP
  port 47820 (so allow 47820/tcp and 5353/udp through the firewall); away
  from home, they reach each other through [iroh](https://www.iroh.computer)'s
  relays, with the same encryption, which you can turn off or point at your
  own relay in Settings → Devices.
- **Offline:** songs can be downloaded, kept sealed with a key in the
  desktop's keyring, and a song that came through a service's DRM plays while
  that service's subscription was confirmed in the last 30 days.

## Requirements

- Linux, with **PipeWire**: it is the only audio output, so without it
  nothing plays.
- A Chromium-family browser (Brave, Chromium, Chrome, Vivaldi or Edge) to sign
  in to Apple Music and SoundCloud. The app opens it on a profile of its own.
  Set `MUSIC_BROWSER` to choose one.
- Apple Music and SoundCloud's protected streams need the Widevine CDM that
  Chrome, Brave, Vivaldi or Firefox installs. Play any DRM video once in that
  browser so the CDM gets its certificate. Set `WIDEVINE_CDM` to point at a
  `libwidevinecdm.so` elsewhere.
- An Apple Music subscription for Apple Music, a SoundCloud account, and a
  [Navidrome](https://www.navidrome.org) server (or any OpenSubsonic server),
  as you like. Each service can be turned off in Settings.
- For the phone: Android 14 or later on an arm64 device. Android 14 is the
  first that plays bit-perfect over USB, which is the point of the app.

## Build

With Nix:

```sh
nix run github:HoneySlatt/Music
```

Or with Cargo. Besides a Rust toolchain you need `pkg-config`, a C and C++
compiler (the Widevine shim is C++), FFmpeg, PipeWire and libopus to link
against, and `cmake`, `perl` and `git` to build BoringSSL from source:

```sh
cargo build --release -p app-desktop
./target/release/music
```

Settings live in `~/.config/music`; covers and animated artwork are cached
in `~/.cache/music`; downloaded songs in `~/.local/share/music`.

### The Android app

The Rust core is shared through UniFFI, so it is built first, then the APK:

```sh
nix develop .#android
./android/build-core.sh
cd android && gradle assembleRelease
```

The APK is in `android/app/build/outputs/apk/release`. It is **signed with
Android's debug key** for now, so it is not fit to hand to anyone else.

## Credits

- [Kopuz](https://github.com/Kopuz-org/kopuz) (EUPL-1.2) for the way to Apple
  Music's web player.
- [Geist](https://vercel.com/font) fonts (SIL Open Font License),
  [Lucide](https://lucide.dev) icons (ISC), the SoundCloud and Apple Music marks
  from [Simple Icons](https://simpleicons.org) (CC0), and the Navidrome logo
  from [its website](https://github.com/navidrome/website) (Apache-2.0).

Music is not affiliated with Apple, SoundCloud or Navidrome. Their names and
logos belong to their owners.

## Licence

GPL-3.0-or-later. See [LICENSE](LICENSE).
