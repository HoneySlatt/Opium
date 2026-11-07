# Changelog

Notable changes, for the people who use the project. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

Nothing has been released yet. What Opium does today:

- **One library** of Apple Music, SoundCloud, a Navidrome (OpenSubsonic) server and your own music
  files, with one search and the same pages; a song several services have shows once and plays
  from the best copy.
- **Sound at its best**: on the computer, each track at its own sample rate through PipeWire,
  gapless, while other programs keep playing; Apple Music's AAC 256 and Spatial Audio, SoundCloud's
  original uploads where allowed, your server's and your own files as they are.
- **Synced lyrics**, word by word from Apple Music, line by line from your files, your server or
  LRCLIB (NetEase and KuGou if you turn them on).
- **Three looks**, Stock, Spotify and M3E, on the computer and the phone, and themes of your own as
  TOML files.
- **Opium Connect**: paired devices share their history, Opium's own playlists and where the music
  stopped, control each other and hand the music over, at home and away.
- **Offline**: songs, albums and playlists downloaded and kept sealed; songs that came through a
  service's DRM play while its subscription was confirmed in the last 30 days.
- **On the computer**: Now Playing, the queue, media keys and desktop widgets (MPRIS), keyboard
  shortcuts, files opened from the file manager, tags edited as in iTunes.
- **On the phone** (Android 14 and later): the same library and pages, Android Auto, and the
  audio chip's offload for songs that are not encrypted.
- **Tags written safely**: a song's tags are edited on a copy first. On the computer, the change is
  made on a copy that replaces the file once whole, so a crash or a full disk can no longer damage
  the song. On the phone, the original is kept aside while the file is written; a change cut short
  leaves the song half written until Opium can open it again, at its next start or at that song's
  next change, and then puts it back.
- The app is renamed from Music to Opium: its folders, its package and its names on the network
  change, with no migration.
- **Written again**: the parts once adapted from Kopuz (the Widevine host, the encrypted MP4
  reader, some of the Apple Music client) are written again from specifications, and sturdier
  against damaged files. Only part of the browser sign-in is still adapted from Kopuz (EUPL-1.2),
  and the about screens say so; Opium stays under GPL-3.0-or-later. Kopuz is still credited for
  finding the way to Apple Music's web player.
