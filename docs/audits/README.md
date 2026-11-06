# Audits

One file per audit, named `YYYY-MM-DD-<scope>.md`: its scope and starting commit, the dimensions it
covered, each finding with how it was verified, the owner's decisions, and what was fixed. Open
items go to the known debt, [`../debt.md`](../debt.md).

| Audit | Scope |
|---|---|
| [2026-10-31](2026-10-31-full.md) | The whole app, before the method; fixed the same day |
| [2026-11-05](2026-11-05-licence.md) | What stood between Opium and MIT OR Apache-2.0, a licence since given up ([ADR-0032](../adr/0032-the-gpl-kept-and-kopuzs-code-still-written-again.md)); what Kopuz does badly in the code written again (M1.9) |
| [2026-11-05](2026-11-05-cenc-clean-room.md) | `cenc` written again in a clean room: the export, the briefs, the questions, the tool calls and the token comparison (M1.10) |
| [2026-11-05](2026-11-05-widevine-clean-room.md) | `widevine` written again in a clean room: the same evidence, and the first runs on the owner's CDM (M1.11) |
| [2026-11-06](2026-11-06-apple-music-clean-room.md) | `apple-music`'s derived items written again in a clean room: the same evidence, and the figures on the owner's account (M1.12) |

The next is the stage audit that closes Stage 1 ([M1.8](../PLAN.md#m18--the-stage-audit)).
