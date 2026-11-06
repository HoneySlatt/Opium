# 0008. Lyrics: the song's own source first, then the sites, LRCLIB alone by default

- Status: Accepted; amended by [0034](0034-what-opium-is-to-become.md)
  (word by word, translated, offline; the other sites an optional piece)
- Date: 2026-10-30; LRCLIB alone by default since 2026-10-31

## Context

Synced lyrics are a large part of Now Playing, word by word where the timing allows. Apple Music has
them for most songs; SoundCloud has none; a Navidrome server and the user's files have them only
when someone put them there. Sites share synced lyrics for any song: LRCLIB (open, made for this),
NetEase Cloud Music and KuGou. Each site asked is told every song the user looks up, and NetEase and
KuGou are asked from China.

## Decision

**A song's lyrics come from its own source first**: Apple Music's TTML, word by word or line by line
(`crates/apple-music/src/lyrics.rs`); a Navidrome server's (`getLyricsBySongId`); a file's `.lrc`
of the same name beside it, else the lyrics in its tags (`crates/local`, commit `5fd8b4c`).

**Otherwise the sites are asked** (`crates/lyrics`, commit `32df707`), matched by title, artist and
length within three seconds so that another version's lyrics are not taken.

**Only LRCLIB is on unless the user turns the others on** (commit `5abe880`, settings
`lyrics_on`); the order the sites are asked in, and whether they come before the song's own source,
are settings too, shared between the user's devices.

**The sites' lyrics are shown line by line**: their timing of words is too rough to follow
(commits `6987da3`, `871ad2b`). Apple Music's stay word by word.

## Alternatives considered

- **All three sites on by default**: more songs covered, but every song looked up told to two
  services in China without the user choosing it.
- **The service's lyrics only**: SoundCloud songs and most files would have none.

## Consequences

- Lyrics need the network, except a file's own.
- A site that changes its API breaks only its own source; the others stay.
- LRCLIB asks clients to say who they are: `crates/lyrics` sends a user agent naming the app,
  which the rename changes ([ADR-0023](0023-the-rename-to-opium.md)).
