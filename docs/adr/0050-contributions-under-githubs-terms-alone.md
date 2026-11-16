# 0050. Contributions under GitHub's terms alone

- Status: Accepted
- Date: 2026-11-16

## Context

Opium is public on GitHub, under GPL-3.0-or-later ([ADR-0001](0001-gpl-3-0-or-later.md)). Its
licence was reconsidered once and kept ([ADR-0032](0032-the-gpl-kept-and-kopuzs-code-still-written-again.md)).
The [architecture audit of 2026-11-16](../audits/2026-11-16-architecture.md#before-the-first-release)
(A7) found that nothing asks more of an outside contribution: no sign-off, no agreement.

## Decision

**A contribution comes under GitHub's terms of service alone**: what is offered to the repository
is licensed under the repository's licence. No sign-off (DCO) and no contributor licence agreement
is asked; CI checks neither.

## Alternatives considered

- **A sign-off (DCO) on every commit**: the author certifies having the right to give the code, as
  the Linux kernel asks. It records where the code comes from, but adds a step to every commit.
- **A contributor licence agreement**: it would let the owner change the licence later, but many
  people refuse to sign one, and it needs a service to collect the signatures.

## Consequences

- Once others have contributed, changing the licence needs each contributor's consent: the GPL
  stays.
- No commit certifies its origin. Reviewing where code comes from, which the clean-room work on
  Kopuz's code showed matters ([ADR-0030](0030-the-kopuz-code-rewritten-in-a-clean-room.md)), is
  left to the review of each pull request.

## Validation and reconsideration

To be put to the owner again only if a contribution's origin is disputed, or if the owner wants to
change the licence before others have contributed.
