# 0046. The shared state sent in parts

- Status: Accepted; amends [0014](0014-shared-state-merged-field-by-field.md) (how the state
  travels, the plays from before, what a device may name)
- Date: 2026-11-10

## Context

[ADR-0014](0014-shared-state-merged-field-by-field.md) sends the shared state "in full when two
devices connect". The [stage 1 audit](../audits/2026-11-10-stage-1.md) found that past about 43,900
songs in Opium's playlists that one message passes Opium Connect's cap of 32 MiB: the link drops
and retries every 5 s for ever (W3), where the quality bar asks for 100,000 songs. It also found
that a device whose `shared.json` does not read starts again and seeds its history from the
apps' old `history.json`, every song stamped now, which pushes every other device's real plays out
(L4); and that a playlist id a paired device sends is joined to the covers' folder, so that an id
holding `../` writes outside it (S4).

## Decision

- **The state travels in parts** (`synced::Shared::into_parts`, M1.15): the history, the session,
  the playlists used last and the settings each whole; each first-seen time; each playlist's name,
  description, cover and deletion; each of its songs. The parts are packed into messages of at most
  4 MiB, an eighth of the cap. Every rule of ADR-0014 merges field by field, ignoring order and
  repetition, so merging the parts in any order equals merging the whole; a device on an older
  build takes them with its merge as it is.
- **What another device sends is written to disk once nothing more has come for 1 s, at most 5 s
  after it came**; a change made on the device, a flush and the app's exit write at once. A device
  that dies within those seconds loses only what the other device still holds and sends again.
- **Plays from before the devices shared their history are seeded only on a true first run** (no
  `shared.json` at all), stamped from 1 up rather than now, so that they rank below every real play;
  a clear made anywhere removes them too. A `shared.json` that does not read is put aside, never
  taken for a first run.
- **A playlist id from another device is 1 to 64 lowercase hexadecimal digits** (Opium makes 16,
  `synced::new_id`): Opium Connect leaves out a playlist whose id is not, and a cover's file is never
  named after such an id.
- **A long list played on another device is cut** to what its queue keeps (`opium_core::queue::LIMIT`),
  the song asked for among it, following the device's shuffle; so are the songs to play next or to
  add to its queue.

## Alternatives considered

- **Raising the cap**: a phone would hold 60 MB and more in one message at 100,000 songs, and a
  larger library would pass the cap again.
- **A new message for parts**: older builds would not take it; the parts as plain `Shared` messages
  need nothing new.
- **Writing at each part**: a full rewrite of `playlists.json` per part, 19 at 100,000 songs in one
  playlist, each holding the state's lock up to 2 s.

## Consequences

- One playlist changed by one song while two devices were apart is still sent whole at their next
  meeting, in parts: 76 MB at 100,000 songs. ADR-0014's design per playlist is kept.
- A piece larger than the cap on its own (a history of a hundred huge tracks from a misbehaving
  device) is still refused, as before.
- The shuffle used to cut a long list is the device's last known one; a `Playing` already on its
  way can undo a shuffle just asked, so a list is then cut in order and shuffled there. No song is
  lost.
