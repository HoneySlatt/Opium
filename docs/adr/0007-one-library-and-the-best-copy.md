# 0007. One library, and the best copy of each song

- Status: Accepted
- Date: 2026-10-30; the device's own files joined, ranked first, on 2026-11-04

## Context

The same song is often on several of the user's services: in their Apple Music library, on their
Navidrome server, liked on SoundCloud, and later as a file of their own. Showing each service in its
own pages would make the user pick a service before a song; showing every copy would list songs two
or three times.

## Decision

**Every service shares the same pages, library and search**; a song, album or artist that several
have shows once (`opium_core::dedupe`, commit `577c504`; `crates/services` gathers the services).

**Songs and albums keep the best copy**, in this order (`dedupe::rank`):

1. the device's own file (`Local`, commit `91d333c`);
2. the Navidrome server's file;
3. Apple Music's AAC 256;
4. SoundCloud's.

**Artists keep Apple Music's**, which has their photos, and its page gathers their releases from
every service (`artist_rank`).

**Copies are matched by their folded title and first artist** (case, accents, `- Single` and `- EP`,
the ways services join several artists), **and songs by their length too**, within three seconds,
so that a remix or a long version stays apart. Copies within one service all stay: they are its own.

## Alternatives considered

- **A page per service**: the user picks a service before a song, as separate apps would.
- **Every copy listed, with its source**: the same song two or three times in every list.

## Consequences

- A file the user owns always wins over a stream; a lossless server copy wins over Apple's AAC.
- A wrong match hides a version: the rules are tested (`dedupe.rs` tests) and every change to them
  needs a test.
- Downloads and the shared playlists name the copy they hold, not the merged song.
