# 0033. The browser sign-in's pipe is kept as adapted from Kopuz

- Status: Accepted; amends [0030](0030-the-kopuz-code-rewritten-in-a-clean-room.md) (one adapted
  part left) and [0032](0032-the-gpl-kept-and-kopuzs-code-still-written-again.md) (M1.13's
  outcome)
- Date: 2026-11-06

## Context

[ADR-0030](0030-the-kopuz-code-rewritten-in-a-clean-room.md) had the code adapted from Kopuz
(EUPL-1.2) written again in a clean room, from the [licence audit](../audits/2026-11-05-licence.md)'s
list: `cenc`, `widevine` and parts of `apple-music`. M1.10 to M1.12 did so.
[ADR-0032](0032-the-gpl-kept-and-kopuzs-code-still-written-again.md) kept the GPL and gave M1.13 a
check that nothing of Kopuz's is left.

M1.13's comparison of the whole tree with Kopuz ([evidence](../audits/2026-11-06-kopuz-whole-tree.md))
found one more part, which the licence audit had missed: `DevTools` in
`crates/browser-signin/src/lib.rs`, which starts the sign-in browser with
`--remote-debugging-pipe`, and calls it. It follows Kopuz's `daemon/server/src/cookies/cdp.rs` at
`a58c385`:

- the same first sentence of the `SAFETY` comment;
- the same descriptors moved above 10, then onto 3 and 4, under the same condition;
- the same pipes in the same order, wrapped the same way, with a call counter.

It came into Opium on 2026-10-30, two days after that revision of Kopuz. It is under 100 lines; the
rest of the crate (waiting for the browser to close, the cookies' filter, the errors) is Opium's
own.

## Decision

**The part is kept as it is, adapted from Kopuz, and said so**: a notice in the crate's
documentation, with Kopuz's revision, file, copyright and licence; the about screens on both apps
and the README name it. It stays under the GPL through the EUPL's compatibility clause, as
[ADR-0001](0001-gpl-3-0-or-later.md) allows. The owner decided so on 2026-11-06.

## Alternatives considered

- **A clean-room rewrite, as M1.10 to M1.12**: a short specification of Chromium's pipe protocol,
  an export without these lines, a new agent, a comparison and its evidence. Nothing of Kopuz's
  would be left, but at the cost of one more round, for under 100 lines whose behaviour Chromium
  fixes.
- **A rewrite by the session that found it**: quicker, but its author had read Kopuz's file;
  ADR-0030 rejected such rewrites, and only the comparison would show the new code owes nothing.

## Consequences

- Opium is not free of Kopuz's code: M1.13 closes with this part named, not removed. ADR-0030's
  goal, no code derived from Kopuz, holds everywhere else.
- Opium keeps resting on the EUPL's compatibility clause for these lines, the debated clause that
  ADR-0032 wanted no code to depend on. A later choice of licence other than the GPL needs them
  written again first.
- The phone's core holds the crate too, through `apple-music`, though the phone signs in through a
  WebView: its about screen says the same as the desktop's.
- A clean-room rewrite stays possible at any time, by ADR-0030's method.
