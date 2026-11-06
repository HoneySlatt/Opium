# 0016. The services: Apple Music, SoundCloud, Navidrome and the device's own files

- Status: Accepted; amended by [0034](0034-what-opium-is-to-become.md)
  (Jellyfin and Plex to come; each online service an optional piece)
- Date: 2026-11-02

## Context

Opium began with Apple Music (2026-10-29), then took in SoundCloud (2026-10-29) and Navidrome
(2026-10-30). Each new service was to blend into the existing pages, at the best quality the
user's plan allows ([ADR-0007](0007-one-library-and-the-best-copy.md)). A YouTube Music client
was then tried, on the computer and the phone, and dropped: the trial is not in the history, only
the code that wiped what it left on the phone (commit `ff6ada9`) and, once nothing was left, that
code's removal (commit `4bda19b`).

## Decision

**Opium plays from four sources**, each a crate implementing `opium_core::Provider`:

- **Apple Music** (`crates/apple-music`), through its web player
  ([ADR-0002](0002-apple-music-through-its-web-player.md));
- **SoundCloud** (`crates/soundcloud`), through its web player's `api-v2`, its uploaders' original
  files when they allow downloads, else its HLS streams, encrypted or not
  ([ADR-0012](0012-soundcloud-changes-through-a-browser-seam.md));
- **Navidrome, or any server speaking the Subsonic API with OpenSubsonic's additions**
  (`crates/navidrome`), its files played as they are;
- **the device's own music files** (`crates/local` on the computer, Android's media index on the
  phone), with their tags and covers, joined on 2026-11-04.

Each can be turned off in Settings while staying signed in.

**YouTube Music is not one of them**: its trial was removed whole, and nothing of it remains in the
code.

## Alternatives considered

- **YouTube Music**: tried and dropped by the owner. Starting a song took several seconds, below the
  bar the other services set.

## Consequences

- A new service is a new `Provider` crate, a place in the best-copy order and in the services
  crate, and its own sign-in; it is the owner's decision, as every new dependency and host is.
- Removing a service later means wiping what it left on the user's devices, as `ff6ada9` did.
