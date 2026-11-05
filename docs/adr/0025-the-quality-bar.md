# 0025. The quality bar: tiers and budgets

- Status: Accepted
- Date: 2026-11-05

## Context

The method asks each project for references, budgets measured against the top of its field, and
the machines it aims at, from the lowest it supports to the highest. Opium had none. A baseline was
measured on the owner's desktop on 2026-11-05 ([`measurements.md`](../measurements.md)): no frame
and no wake-up of the window's threads at rest, but 8.4 wake-ups a second of the process (Opium
Connect, an idle `blocking` thread, the theme watch's poll, the player's idle timeout); a warm
start of 173 ms to the window on screen; 339 to 383 MiB resident; no measurable GPU power at rest.
Frame times could not be measured: the egui window records none. The phone was measured partly on
2026-11-03, before the rename.

## Decision

**The tiers.** On the desktop, from a 2015 laptop (2 cores and 4 threads, an Intel HD 520, 4 GB of
memory, a hard disk, 1080p at 60 Hz) to the owner's desktop (4K at 240 Hz, scale 1.5). On the
phone, from an entry-level Android 14 phone of 2023 (4 GB, a Helio G99-class chip, 90 Hz) to the
owner's Pixel (120 Hz). On the lowest tier Opium holds its frame rate, answers at once and every
feature works; it only loses detail.

**The budgets**, in [`ROADMAP.md`](../ROADMAP.md#quality-bar), where each lives with how it is
measured. The owner chose, beyond the method's defaults:

- **At rest, 0 wake-ups of Opium's own**, Opium Connect included: it wakes only when the network
  speaks to it.
- **A warm start of at most 100 ms** on the high tier, below the method's default of 150 ms.
- **At most 150 MiB resident at rest**, so that the 4 GB tier keeps room; the peak while scrolling
  no higher than today's.
- **The display's rate held**: 240 Hz on the high desktop tier, 60 Hz on the lowest; 120 Hz and
  90 Hz on the phone's.
- **Playing costs only playback**: nothing drawn while hidden; while shown, a frame only when the
  progress moves a pixel or the time a second.
- **The phone's battery while playing no worse than Spotify's** for every kind of song, Apple
  Music's protected songs included.
- A CI job under 20 minutes; full accessibility; HDR stated as unsupported; bad input never a crash.

## Alternatives considered

- **Budgets equal to today's figures**: guards against regressions, sets no direction.
- **A higher lowest tier** (a 2018 laptop with 8 GB and an SSD): easier to meet, narrows who can run
  Opium.
- **An allowance of wake-ups for Opium Connect**: simpler, against the rule that nothing runs while
  there is nothing to do.

## Consequences

- Today Opium misses several budgets: the wake-ups at rest, the warm start, the memory at rest, the
  phone's battery with Apple Music. Each becomes work to plan, with its benchmark.
- The lowest tiers are unmeasured until a machine of each is available; they are never assumed.
- Frame times need instrumentation in the window before the frame budgets can be checked.
