# 0005. PipeWire as the only output, at each track's own rate

- Status: Accepted
- Date: 2026-10-29

## Context

Opium is made for a listener who wants each source played at its best: a 24-bit, 96 kHz file
should reach the sound card as it is, not resampled to whatever rate the desktop's mixer runs at.
The same listener needs other programs' sound (a game, a browser, a call) to keep playing alongside
the music.

The first player (commit `9208d65`) opened the card through ALSA in exclusive mode. It silenced
every other program, and once left WirePlumber without the card afterwards: the user's other
programs fell back to another output until WirePlumber was restarted.

## Decision

**The desktop plays through a PipeWire stream, and through nothing else** (`crates/player`,
commit `7b75bce`).

**The stream asks the graph to run at the source's rate**: `node.rate` is the track's rate and
`node.force-rate` is `0`, which forces the graph to that rate, so the music is not resampled; what
else plays is resampled instead (`output.rs`). If the graph still cannot follow, PipeWire resamples
at its best quality (`resample.quality = 14`).

**Samples go out as 32-bit floats**, as PipeWire mixes them, so a 24-bit source passes exactly.
Volume is applied in the stream; at 100 % the samples pass untouched. Pausing, seeking and volume
changes ramp over a few milliseconds instead of clicking.

**The player tells what reaches the card**: it watches the PipeWire graph (`graph.rs`) for the
device it plays into, the other programs mixed into it and the device's software volume, and
reports the graph's real rate, which the signal path shows the user.

**A track queued with the same rate and channel count follows without a gap** (`gapless.rs`).

## Alternatives considered

- **Exclusive ALSA output** (`hw:` device, device reservation): bit-perfect, but other programs go
  silent, and it once broke WirePlumber's routing. Removed.
- **A PipeWire stream at the graph's own rate**: simplest, but every track whose rate differs is
  resampled.

## Consequences

- PipeWire is a hard requirement on Linux (README).
- While a track plays at 44.1 kHz, other programs' 48 kHz sound is resampled by PipeWire.
- The phone does not use this crate: Android plays through ExoPlayer
  ([ADR-0009](0009-the-android-app-over-the-rust-core.md)).
- A Windows port needs an output of its own (ROADMAP); this crate is Linux-only.
- The player has no way to wake the window: the window asks it for its state every 250 ms while
  music plays (PLAN, known debt).
