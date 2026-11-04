# Music

A desktop music player that puts Apple Music, SoundCloud, Navidrome and your
own music files in one library, in the spirit of Apple Music and [Cider](https://cider.sh). Written in
Rust with [egui](https://github.com/emilk/egui).

- **One library, one search:** albums, songs, artists and playlists from every
  service share the same pages. A song or album you have on several services
  shows once, and plays from the best copy: your own file, then your Navidrome
  server's, then Apple Music, then SoundCloud.
- **Your files:** choose music folders in Settings and their FLAC, ALAC, WAV,
  AIFF, MP3, AAC and Ogg Vorbis files join the library, with their tags and
  covers, watched as they change. A file opened from the file manager, the
  command line or dropped on the window plays.
- **Sound:** each track plays at its own sample rate through PipeWire, without
  resampling, while other apps' sound still plays. You get Apple Music's AAC
  256, SoundCloud's original uploads when the artist allows downloads, and
  your server's files as they are.
- **Lyrics:** synced lyrics, word by word when the source has the timing. They
  come from the service, else from LRCLIB, NetEase Cloud Music or KuGou.
- **Desktop:** a Now Playing view, a queue, MPRIS (media keys and desktop
  widgets), and keyboard shortcuts.

## Requirements

- Linux with PipeWire.
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

## Build

With Nix:

```sh
nix run github:HoneySlatt/Music
```

Or with Cargo, given FFmpeg, PipeWire and `pkg-config`:

```sh
cargo build --release -p app-desktop
./target/release/music
```

Settings live in `~/.config/music`; covers and animated artwork are cached
in `~/.cache/music`.

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
