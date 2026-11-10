# 0015. Away from home, paired devices meet through iroh

- Status: Accepted; amends [0010](0010-opium-connect-on-the-local-network.md) (paired devices also
  reach each other away from home); amended by [0040](0040-away-from-home-only-while-a-device-is-paired.md)
- Date: 2026-11-01

## Context

Opium Connect worked only on the same network ([ADR-0010](0010-opium-connect-on-the-local-network.md)):
a phone on mobile data could neither see the computer at home nor share its history and playlists
with it ([ADR-0014](0014-shared-state-merged-field-by-field.md)). The owner wanted it to work away
from home, for every user of the app, without a server of Opium's to pay for, and with the phone
behind a VPN of its own.

## Decision

**Paired devices away from each other connect through iroh** (`crates/connect/src/away.rs`,
commit `a8e5590`), which finds a way between them: straight through the routers when it can,
through a relay otherwise, n0's public relays unless the user sets another in Settings → Devices.

- **iroh is the transport only**: the same Noise connection as at home runs over it, readable only
  by paired devices. Opium speaks its own protocol name on it (`opium-connect/1`).
- **Each device has a key on iroh**, kept in `connect.json`; devices exchange their iroh ids
  encrypted, over a connection at home. **Pairing stays on the local network**: devices meet at home
  once, then reach each other anywhere.
- **What iroh publishes of a device is the relay it uses, not its address**, and only paired
  devices know its id.
- **The phone is reachable away from home only while the app shows and a minute after** (commit
  `3a8b03f`), so that keeping in touch with the relay does not keep its radio awake; the desktop
  always is.

## Alternatives considered

- **A VPN such as Tailscale**: Android allows one VPN at a time, and the phone already runs its own.
- **Port forwarding on the home router**: a burden on every user, and not always possible.
- **A hosted mailbox for everyone**: its cost would be Opium's to carry. A relay that cannot read
  what passes is possible later.

## Consequences

- With n0's relays, a third party carries the (encrypted) traffic and sees which relay a device
  uses; a user can set a relay of their own.
- iroh grew the APK from about 22 to about 34 MB, and the phone reconnects in about two seconds
  when the app comes back to the front over mobile data (both noted when it was added, not
  measured again since).
