# 0054. A minimal export at the first release

- Status: Accepted
- Date: 2026-11-17

## Context

What Opium is to become puts the export of everything, in open formats, in Stage 7, at 0.5
([ADR-0034](0034-what-opium-is-to-become.md)). The phone keeps nothing in Android's backups or
transfers (`allowBackup="false"`, `data_extraction_rules.xml`), so that its sign-ins and pairings
never leave it. The [architecture audit of 2026-11-17](../audits/2026-11-17-architecture.md) found
that a user with only the phone then loses Opium's own playlists and the history on a new phone,
and that a paired device is the only other copy until 0.5.

## Decision

**A minimal export and import of Opium's own playlists and of the history**, holding no secret, a
sign-in or a pairing: on the desktop at 0.1.0 (Stage 2), on the phone at Stage 3. The full export of
Stage 7 stays.

## Alternatives considered

- **Android's backup of `shared.json` and `playlists.json` alone**: no button to add, but it rests
  on the phone's backup service, which some phones have not, and it leaves the desktop as it is.
- **Nothing before Stage 7**: no work, and data a user can lose for two releases.

## Consequences

- Stage 2 and Stage 3 each gain a function; its format is versioned like the kept files.
- What an import does with playlists and plays already there is designed at Stage 2.

## Validation and reconsideration

An export read back into an empty profile shows the same playlists and history. Reconsidered when Stage 7's export is designed, which may absorb it.
