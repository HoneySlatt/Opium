# 0040. Away from home only while a device is paired

- Status: Accepted; amends [0015](0015-away-from-home-through-iroh.md) (when the desktop is
  reachable away from home)
- Date: 2026-11-10

## Context

Opium Connect reaches paired devices away from home through iroh and n0's relays
([ADR-0015](0015-away-from-home-through-iroh.md)), and the desktop "always is" reachable that way.
Its iroh secret is made at the first start, so the endpoint is bound whatever is paired. The
[stage 1 audit](../audits/2026-11-10-stage-1.md) found that it is the main cause of Opium waking at
rest: iroh's network report every 20 to 26 s, its relay ping every 15 s, an endless redial offline;
the process wakes 7.4 times a second online and 27 to 33 offline, where the quality bar asks for 0.
And n0 is contacted before any device is paired, which serves no one, since only paired devices
are let in.

## Decision

**iroh is bound only while a paired device has an iroh id**, and closed when the last one is
forgotten. With nothing paired, Opium contacts no relay. A paired device still reaches the desktop
away from home whenever it could before. Built in Stage 5, with the rest of the work for no
wake-up at rest; the budget for the relay's ping while a device is paired is set there.

## Alternatives considered

- **The same change in M1.15**: the gain at once, but it is neither a loss of data nor a flaw, which
  is what [ADR-0035](0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md) lets through
  before uwui.
- **iroh always bound, Connect given a budget of wake-ups of its own**: nothing changes, and the bar's
  0 at rest is never met; n0 is contacted with nothing paired.

## Consequences

- Most of Connect's wake-ups at rest go for a user with nothing paired.
- With a device paired, the relay's ping every 15 s remains: "Opium Connect wakes only when the
  network speaks to it" cannot hold then, and Stage 5 gives it a budget.
- The debt "Away from home cannot be turned off" narrows to the case of a device paired.
