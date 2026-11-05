# 0010. Opium Connect on the local network: mDNS, TCP 47820, SPAKE2 and Noise

- Status: Accepted; amended by [0015](0015-away-from-home-through-iroh.md) (paired devices also
  reach each other away from home)
- Date: 2026-10-30

## Context

The user listens on the computer and the phone and wants them to work together, as Spotify's devices
do: the phone shows what the computer plays and controls it, and the music moves from one to the
other. There is no Opium server, and none is wanted: each device has its own sign-ins and plays
with them. Anything else on the home network must neither control a device nor read what passes.

## Decision

**Opium Connect** (`crates/connect`, commit `5e80bf4`): devices find each other on the local
network, show what each plays, control it, and hand the queue over (a *handoff*).

- **Found over mDNS** as `_opium-connect._tcp`, **reached over TCP on port 47820**, or another free
  port if that one is taken.
- **Paired once, with a code**: one device shows six digits, the user types them on the other. The
  two run SPAKE2 over the code and confirm the key with an HMAC (`pair.rs`): the code is good for
  one guess, and what passes on the network gives an onlooker nothing to try codes against. A
  failed pairing doubles the wait before the next, up to an hour (commit `5abe880`).
- **Only paired devices are listened to**, and **everything after the greeting is encrypted with
  Noise `NNpsk0_25519_ChaChaPoly_SHA256`**, the pairing key as the pre-shared key (`wire.rs`):
  each connection has new keys, so what was said stays unreadable should the pairing key leak
  later.
- **No server**: each device plays with its own sign-ins and tells the others only what it plays.
- **A device paired with one is introduced to the others** with a key made for the two (commit
  `fb1d688`); a device the user forgot is never introduced again.
- **What a device keeps** (its id, its paired devices and their keys) is `connect.json` in the
  config folder, readable by the user only.

What the devices keep in common goes over the same connections
([ADR-0014](0014-shared-state-merged-field-by-field.md)), and so do songs one device downloads
for another ([ADR-0017](0017-downloads-sealed-and-tied-to-the-subscription.md)).

## Alternatives considered

None were written down for the local network. A server in between was weighed later, for devices
away from home, and rejected ([ADR-0015](0015-away-from-home-through-iroh.md)).

## Consequences

- The desktop's firewall must let in 47820/tcp and 5353/udp (README).
- A test run of the desktop goes on the network like the real one: a fresh profile shows as a new
  device on the owner's phone, and a copy of `connect.json` would *be* the owner's computer
  (CONTRIBUTING.md).
- Old builds and renamed builds do not talk to each other: the service name, the handshake's
  prologue and the iroh protocol name changed with the rename, with no compatibility kept
  ([ADR-0023](0023-the-rename-to-opium.md)).
