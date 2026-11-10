# 0041. A paused protected song's session closes after a long pause

- Status: Accepted; amends [0038](0038-opium-measured-against-the-market.md) (when a paused song's
  CDM session is closed)
- Date: 2026-11-10

## Context

A protected song keeps its CDM session open while it plays, and while paused. The CDM's own timer
then wakes Opium once a second, and the decrypt thread, which waits with a 250 ms timeout, four
times a second. [The debt](../debt.md) recorded that the owner kept the session open, as closing it
costs a licence when the song resumes (75 to 340 ms); [ADR-0038](0038-opium-measured-against-the-market.md)
then said that a session left open while paused is closed. The [stage 1
audit](../audits/2026-11-10-stage-1.md) found the two in conflict.

## Decision

**A paused song's session closes once the pause has lasted a while**, minutes rather than seconds,
the length set when it is built, and opens again with a licence when the song resumes. **The
decrypt thread waits without a timeout**, every change it waits for already waking it. Built in
Stage 5, with the rest of the work for no wake-up at rest.

## Alternatives considered

- **Closing at every pause**, ADR-0038 to the letter: no wake-up from the first second, and a
  licence asked of Apple at every resume, even after two seconds.
- **Keeping it open**: an instant resume always, and a wake-up a second for as long as the pause
  lasts, against the bar's 0 at rest.

## Consequences

- A short pause resumes at once; a long one costs a licence when it resumes.
- The debt's line on the CDM's wake-up while a session is open is replaced by this decision.
