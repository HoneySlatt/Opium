# 0001. GPL-3.0-or-later

- Status: Accepted
- Date: 2026-10-29

## Context

Opium reaches Apple Music the way another open-source player, Kopuz
(<https://github.com/Kopuz-org/kopuz>, EUPL-1.2), does: its requests to Apple's web player and its
way of driving a browser's Widevine CDM were the starting point of `apple-music`, `cenc` and
`widevine`. Whatever licence Opium takes must allow that code in, with its attribution kept.

## Decision

**The workspace is licensed GPL-3.0-or-later**, set once in the root `Cargo.toml`
(`[workspace.package] license`) and in `LICENSE`, from the first commit.

**Code adapted from Kopuz is used under the GPL through the EUPL-1.2's compatibility clause**, which
lists the GPL-3.0 among the licences an EUPL work may be redistributed under. Each file that adapts
it says so at its top: the project, the revision, the path it came from, its copyright and its
licence (`crates/apple-music/src/lib.rs` and `playback.rs`, `crates/cenc/src/fmp4.rs`,
`crates/widevine/src/lib.rs` and `shim/widevine_shim.cc`).

## Alternatives considered

- **A permissive licence (MIT, Apache-2.0)**: Kopuz's EUPL code could not have been taken in.
- **EUPL-1.2 itself**: possible, but less known, and its compatibility clause already lets the code
  live under the GPL.

## Consequences

- Any dependency or copied code must be GPL-3.0-compatible; a dependency's licence is part of
  asking to add it (AGENTS.md).
- What is bundled under other licences ships with its texts: Geist (SIL OFL 1.1) and Lucide (ISC)
  are in `crates/app-desktop/assets/` and `android/app/src/main/assets/licenses/`. A full list of
  the third-party licences of the shipped programs does not exist yet (ROADMAP, first release).
- Google's Widevine CDM is never shipped: it is the user's browser's ([ADR-0004](0004-the-widevine-cdm-borrowed-from-the-browser.md)).
