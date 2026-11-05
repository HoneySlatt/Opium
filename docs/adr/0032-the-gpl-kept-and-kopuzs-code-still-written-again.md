# 0032. The GPL is kept, and Kopuz's code is still written again

- Status: Accepted; amends [0030](0030-the-kopuz-code-rewritten-in-a-clean-room.md) (the licence
  kept, M1.13 a check that nothing of Kopuz's is left) and confirms
  [0001](0001-gpl-3-0-or-later.md)
- Date: 2026-11-06

## Context

[ADR-0030](0030-the-kopuz-code-rewritten-in-a-clean-room.md) decided that the code adapted from
Kopuz (EUPL-1.2) is written again in a clean room, so that Opium could leave GPL-3.0-or-later for
"MIT OR Apache-2.0" at M1.13. M1.10 and M1.11 are done; M1.12 is under way.

On 2026-11-06 the owner decided to keep GPL-3.0-or-later, Opium's licence since
[ADR-0001](0001-gpl-3-0-or-later.md), and to go on removing Kopuz's code all the same, to the same
standard as if Opium were moving to MIT. Weighed the same day: Apache-2.0 alone, rejected for the
reasons below.

## Decision

- **Opium stays under GPL-3.0-or-later.** M1.13 changes no licence.
- **The clean-room rewrite goes on as ADR-0030 says**: M1.12 for `apple-music`, the same method,
  the same token comparison, the same evidence.
- **M1.13 checks that nothing of Kopuz's is left**: the whole tree compared with Kopuz, the
  credits that name Kopuz for code made true (the desktop's and the phone's about screens, the
  README, the notices), the CHANGELOG. Kopuz stays credited for the facts it found about Apple's
  service, which are not code.

Why the rewrite goes on without a change of licence:

- no code of Opium's then depends on the EUPL's compatibility clause, whose reach is debated, and
  that was ADR-0001's reason for the GPL;
- the rewritten code does better than Kopuz where the [licence
  audit](../audits/2026-11-05-licence.md) found it lacking;
- the owner, sole author of what remains, stays free to choose another licence later.

## Alternatives considered

- **MIT OR Apache-2.0, as ADR-0030 decided**: code would move freely between Opium, uwui and
  SlimeEngine, but Opium's code could be taken into closed software, and the copyleft would be
  lost. The owner chose the GPL.
- **Apache-2.0 alone**: one licence and a patent grant for everyone, but code that could not go
  into uwui or SlimeEngine as it is, incompatible with GPL-2.0-only projects, and a patent grant
  that covers only the contributors' patents, the owner's alone today.
- **Stopping the rewrite after M1.11**, since the GPL accepts the EUPL code: less work, but code
  resting on a debated clause, Kopuz's weaknesses in `apple-music` kept, and no free choice of
  licence later.

## Consequences

- M1.13's goal and deliveries change in the PLAN; ADR-0030's consequences on the licence's change
  no longer apply.
- uwui, under MIT OR Apache-2.0, can still be used by Opium: both licences are compatible with the
  GPL. Code goes from uwui into Opium, not from Opium into uwui without the owner relicensing it.
- The ROADMAP's licence row and Stage 1's exit name the GPL.
