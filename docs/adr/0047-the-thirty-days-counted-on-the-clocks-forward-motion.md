# 0047. The thirty days counted on the clock's forward motion

- Status: Accepted; amends [0017](0017-downloads-sealed-and-tied-to-the-subscription.md) (how the 30
  days are counted)
- Date: 2026-11-11

## Context

[ADR-0017](0017-downloads-sealed-and-tied-to-the-subscription.md) lets a song that came through a
service's DRM play offline while that service's subscription was confirmed in the last 30 days,
judged on the wall clock. The [stage 1 audit](../audits/2026-11-10-stage-1.md) found that a clock
set back played an expired song again (S5), and proposed keeping the latest time seen. M1.15 built
that first; its closing review then showed that a clock set back *before* the 30 days ended still
played on without end, as the latest time seen never reached the end.

## Decision

**The store counts, per service, how far the clock has moved forward since that service's last
confirmation**, and a protected song is `Expired` once that count reaches 30 days
(`downloads::store`). Each reading adds the time since the one before when the clock moved forward,
and nothing when it moved back. A confirmation starts the count again at 0. The clock is read at
each start, confirmation, download and song opened to play; the count is kept in the downloads'
manifest (its `elapsed` field, within version 1, which older builds ignore), saved at each start and
confirmation, when it reaches 30 days, and each time it grows by an hour.

## Alternatives considered

- **The latest time seen**, as the audit proposed and M1.15 first built: a clock set back before the
  end gains without limit.
- **Reading the clock on a timer** while the app runs: the time while it is open with nothing played
  would count too, for a wake-up the quality bar refuses at rest.
- **The service's own time**, from its answers: only online, which is when a confirmation already
  resets the count.

## Consequences

- A clock set back hides the time between two readings: while the app is closed, or open with
  nothing downloaded played, and up to an hour since the last save. The manifest is plain JSON, which
  a user can edit. Both are in [the debt](../debt.md); the count is a guard against a clock moved,
  not against a user who sets out to defeat it.
- A clock that was behind and is put right counts the jump: protected songs may expire until the next
  confirmation, which is the safe side.
- A confirmation made with the clock far ahead holds 30 days of forward motion once the clock is
  right, not 30 days from that false time.
