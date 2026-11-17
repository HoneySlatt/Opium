# 0052. Apple's licences paced in one place

- Status: Accepted
- Date: 2026-11-17

## Context

Apple Music plays through its web player's private API, as a browser would
([ADR-0002](0002-apple-music-through-its-web-player.md)); every protected song needs a Widevine
licence from Apple. The [architecture audit of 2026-11-17](../audits/2026-11-17-architecture.md)
(SEC-5) found:

- the desktop warms a song the pointer rests on for 0.3 s (`app.rs`, `WARM_AFTER`), and the warm
  asks for its licence (`desktop::backend`, `Request::Warm`);
- a fetch to play is never called off once a newer one comes;
- so ten quick skips may ask Apple for about ten licences in a second or two, which no browser does;
  the CDM serves them first come, so the licence wanted waits behind the others;
- the downloads' queue spaces its own licences a second apart (`downloads::queue`), and every licence
  of both apps goes through one function, `AppleMusic::license` (`apple-music/src/playback.rs`).

How Apple answers such bursts is not known.

## Decision

**A warm stops before the licence**: it reads what it can without one, and asks for the licence only
once the song is played.

**A fetch to play that a newer one supersedes is called off**, by its token.

**Apple's licences are paced in `AppleMusic::license`**, for every caller of both apps: a licence at
most every second, as the downloads' queue spaces them today. The downloads' queue then keeps no
pacing of its own.

## Alternatives considered

- **Calling off without pacing**: no skip slowed, but bursts stay possible when skipping fast.
- **Leaving it until Apple's reaction is checked**: nothing changes, but a burst that Apple's guards
  notice would be learned from a refused account.

## Consequences

- A skip made within a second of the last licence may wait up to a second for its own.
- The pacing has one home, tested once, and a new caller cannot bypass it.
- The figures of the burst and of the time to sound are measured when the time to sound is, with
  uwui.

## Validation and reconsideration

A fake Apple server counts the licence requests over ten skips: two at most. Reconsidered if Apple's
web player is seen asking for licences faster, or if the time to sound misses its budget because of
the pacing.
