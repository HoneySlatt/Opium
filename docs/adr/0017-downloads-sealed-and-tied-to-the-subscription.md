# 0017. Downloads sealed with the app's key, and tied to the subscription

- Status: Accepted; amended by [0034](0034-what-opium-is-to-become.md)
  (a collection's available part shown; an offline mode by hand) and
  [0047](0047-the-thirty-days-counted-on-the-clocks-forward-motion.md) (how the 30 days are counted)
- Date: 2026-11-03

## Context

The owner wanted songs kept for offline listening, inside the app, as Apple Music and Spotify do.
Navidrome's files and SoundCloud's original uploads come unencrypted. Apple Music's songs and
SoundCloud's encrypted streams come through Widevine, whose licences on the desktop are temporary
([ADR-0004](0004-the-widevine-cdm-borrowed-from-the-browser.md)): keeping them playable offline
means keeping them decrypted in some form. On the phone (Widevine L1), a spike on 2026-11-03 found
that Apple and SoundCloud give streaming licences only: an offline licence request is refused.

## Decision

**Every downloaded song is kept sealed with the app's key** (`crates/downloads`, commits `40aab8d`,
`4f7dd68`):

- **The sealed format** (`sealed.rs`): AES-256-GCM in chunks of 64 KiB, each sealed on its own with
  a nonce made of a random prefix and the chunk's index, so that the player seeks anywhere; the
  last chunk is sealed as the last, so a file cut short does not read as whole. The header starts
  with a magic number naming the format.
- **The key** is made once and kept in the desktop's keyring (`keyring.rs`,
  [ADR-0013](0013-secrets-and-what-is-kept-on-disk.md)); on the phone, in the app's vault.
- **The store** (`store.rs`): a folder only the user can open, under the data folder
  (`opium/downloads`), each song a sealed file of an opaque name, and a manifest beside them,
  written whole and renamed into place. A manifest that does not read is put aside, without
  sweeping away the files it no longer names; a song sealed under a key the keyring no longer has
  plays from its service instead (commit `4bce119`).

**A song that came through a service's DRM plays while that service's subscription was confirmed
in the last 30 days** (`store::PROTECTED_FOR`): Apple Music's is asked each time the app connects
(`AppleMusic::subscribed`, `/v1/me/account?meta=subscription`); SoundCloud's counts as confirmed
while signed in. An ended subscription, or a sign-out, makes those songs `Expired`. Songs that
came without DRM play always.

**The phone downloads DRM songs through the computer**: it asks the paired desktop, which fetches
and decrypts the song with its CDM and lends it over an Opium Connect connection of its own; the
phone seals what comes, as it seals its own downloads (`connect::lend`, commits `54d034b`,
`40043cb`). Unencrypted songs the phone downloads itself.

**The CDM's pace is accepted**: Apple's songs download no faster than the CDM decrypts (about 11.6
times real time, commit `33a995e`); the next song's licence and first bytes are fetched meanwhile.

## Alternatives considered

- **Encrypted bytes kept with a persistent licence**: the desktop's CDM sessions are temporary, and
  the phone's offline licence requests were refused by both services.
- **Decrypted files kept in the clear**: anyone could copy them out of the app, and they would
  outlive the subscription.
- **Capturing the decoded sound, or a software CDM of one's own**: refused.
- **Getting around the CDM's pace** (faking its clock, several CDMs at once): refused.

## Consequences

- Downloads play only in Opium, on the device that holds the key; losing the key loses them.
- The 30-day rule depends on the network: offline longer than that, DRM songs stop until the app
  confirms again.
- The phone needs the computer, paired and reachable, to download Apple Music's songs.
- How much of this the first public release exposes is the owner's decision (ROADMAP).
