# 0002. Apple Music through its web player, as Kopuz does

- Status: Accepted
- Date: 2026-10-29

## Context

Apple Music is the first service Opium was made for. Apple's official routes for a third-party
player are MusicKit: a developer token signed with a key from a paid developer account, MusicKit JS
on the web and MusicKit for Android, each rendering the music itself. None of them hands a
desktop Rust program the audio to play through its own output at its own quality. Apple's own web
player at `music.apple.com` does everything Opium needs, through a private API
(`amp-api.music.apple.com`) and a Widevine-protected stream. Kopuz, an open-source player,
already drives that private API.

## Decision

**Opium talks to Apple Music as Apple's web player does**, with the request shapes Kopuz uses
(`crates/apple-music`):

- **The developer token is read from the web player's own JavaScript bundle** (`auth.rs`): the
  page names an `index~*.js` bundle, which carries a JWT. When Apple renames or moves it, the error
  says which step broke.
- **The user's token (`media-user-token`) comes from a sign-in in the user's browser**
  ([ADR-0003](0003-sign-in-through-the-users-browser.md)).
- **Songs play through `webPlayback`, a Widevine licence and CENC decryption** (`playback.rs`,
  `crates/cenc`), with the browser's CDM on the desktop
  ([ADR-0004](0004-the-widevine-cdm-borrowed-from-the-browser.md)) and the phone's own MediaDrm on
  Android ([ADR-0009](0009-the-android-app-over-the-rust-core.md)).
- **The best this path reaches is what Apple licenses Widevine for**: the `28:ctrp256` flavour,
  AAC-LC 256 kbps at 44.1 kHz; and Spatial Audio, Apple's binaural rendering of a song's Dolby Atmos
  mix, from the song's enhanced HLS. ALAC is FairPlay-only, and Apple refuses Widevine its key:
  lossless Apple Music is out of reach on this path.
- **Apple's token goes to Apple's hosts only**: a redirect away from them is refused rather than
  followed with the header that carries it (commit `1e8fe42`).

## Alternatives considered

- **MusicKit with a developer account**: official, but it plays through Apple's own renderer, not
  Opium's output, and needs a paid developer account.

## Consequences

- The private API can change without notice; a change breaks Opium until the code follows. Every
  such behaviour is verified only against the real service (AGENTS.md, "Honesty towards the
  services").
- The code adapted from Kopuz carries its attribution and is used under the GPL
  ([ADR-0001](0001-gpl-3-0-or-later.md)).
- The quality ceiling for Apple Music is AAC 256 and binaural Spatial Audio.
