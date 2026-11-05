# 0014. What the devices share, merged field by field

- Status: Accepted
- Date: 2026-10-31

## Context

The owner wanted the play history and Opium's own playlists (of songs from any service) to be the
same on every device, for every user of the app, with no server of Opium's
([ADR-0010](0010-opium-connect-on-the-local-network.md)). Devices are often off or away from each
other; each must keep working alone and catch up when they meet, in whatever order, however often,
without one device's copy overwriting another's changes.

## Decision

**Each device keeps its own copy of what is shared, and two devices that meet merge each other's**
(`crates/synced`, commit `61c668a`), so that both come out the same whatever was changed where and
in whatever order they meet. The rules, by kind of data:

- **A change carries a stamp**, the time on a hybrid clock and the device: of two changes to the
  same thing the later wins, the device deciding a tie.
- **History** (`history.rs`): each song once, at its last play; the latest 100 kept. Clearing the
  history is a stamp of its own: everything played before it is gone on every device, however late
  a device hears of it.
- **Opium's own playlists** (`playlists.rs`): each song in a playlist is an entry of its own, so
  the same song added on two devices at once stays twice; an entry taken out stays out. Each entry
  has a place, a key that sorts it among the others and can always be found between two
  (`place.rs`), so moving one song changes only its own place. A name or description: the latest
  wins. A playlist deleted stays deleted.
- **Settings shared between devices** (the lyrics sites and their order, favourites and playlist
  songs joining the library, Recently Added by date, SoundCloud's originals, the relay): each a
  value whose latest wins. Spatial Audio and settings of the device itself are not shared.
- **Where the music last stopped** (the session): the latest stop wins; a device opens on it, and
  an idle one follows the one playing.
- **When each new library song was first seen** on any device (the earliest wins), which dates
  Recently Added where the service does not, and **the playlists songs were last added to** (the
  latest three): the only fields a device's own clock decides outright, so a time too far from now
  is not believed (commit `5717743`).

**They travel over Opium Connect**: in full when two devices connect, as each change is made
afterwards, and as digests first, so that a large library's playlists are sent only when they
differ (commit `75162f5`). Playlists are kept in a file of their own, `playlists.json`, beside
`shared.json`.

## Alternatives considered

- **The latest copy wins whole**: simpler, but a device that was off would wipe what the others did
  meanwhile.
- **A hosted server holding the one copy**: rejected with the server of
  [ADR-0015](0015-away-from-home-through-iroh.md).

## Consequences

- Every change to the shared data must keep the merge the same whatever the order and however often
  it runs: tested in `crates/synced/src/tests.rs`, which a change extends.
- Deletions are final: there is no undo of a deleted playlist across devices.
- A test window that shares the owner's device identity (a copied `connect.json`) merges into the
  owner's real state and can overwrite the phone's newer stop (CONTRIBUTING.md).
- The renamed playlists' provider (`Opium`, formerly `Music`) needs the owner's state converted
  once by hand ([ADR-0023](0023-the-rename-to-opium.md)).
