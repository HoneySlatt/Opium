# 0053. A forgotten device forgotten everywhere

- Status: Accepted; supersedes in part [0010](0010-opium-connect-on-the-local-network.md) (a
  device forgotten)
- Date: 2026-11-17

## Context

Opium Connect introduces a device paired with one to the others, with a key made for the two;
a device the user forgot is never introduced again
([ADR-0010](0010-opium-connect-on-the-local-network.md)). The
[architecture audit of 2026-11-17](../audits/2026-11-17-architecture.md) (SEC-4) found that
forgetting is local only: `connect::Store::forget` removes one id on the device where the user
forgets it; the devices that device introduced stay paired, though each entry keeps who introduced
it (`by`); the other paired devices go on talking to the forgotten one and pass it the shared state.
A lost phone, forgotten on the computer, could still sync through a tablet, and a device it
introduced could still send commands and borrow songs.

## Decision

**A device forgotten on one of the user's devices is forgotten on every paired device, for good**,
as a deletion of the shared state is: each device that hears of it unpairs it and never introduces
it again, whatever order the news comes in.

**The devices it introduced are unpaired with it**, on each device, unless the user paired them
directly.

**It is built in the milestone of Connect's versions**, which changes the wire already; the
milestone of final sign-outs unpairs the introduced devices on the device where the user forgets
one.

## Alternatives considered

- **Local only, documented**: "forget it on each device"; no change to the wire, but a lost device
  stays trusted wherever the user forgets to.
- **Built now, in the milestone of final sign-outs**: sooner, but a change to the wire before
  Connect knows versions.

## Consequences

- A device forgotten by mistake must be paired again on every device.
- A forgotten device that never hears of it keeps its own pairings, but no device of the user's
  answers it any more.
- The trust of a guest, open for Stage 8, starts from this rule.

## Validation and reconsideration

A test of three devices: the user forgets the third on the first; the second unpairs it, whatever
order the messages come in, and so do the devices it introduced. Reconsidered with the guest's trust,
at Stage 8.
