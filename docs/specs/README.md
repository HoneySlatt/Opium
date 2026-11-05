# Specifications

What some parts of Opium must do, written apart from their code: the formats they read, the
requests they send, their steps and their errors. They are the only input of a clean-room rewrite,
and stay afterwards as the reference for the protocols they describe.

| Specification | Crate | Why it exists |
|---|---|---|
| [fmp4-cenc.md](fmp4-cenc.md) | `crates/cenc` | Encrypted fragmented MP4, from ISO/IEC 14496-12 and 23001-7 |
| [widevine-host.md](widevine-host.md) | `crates/widevine` | Driving the browser's Widevine CDM, from Chromium's CDM interface |
| [apple-music-playback.md](apple-music-playback.md) | `crates/apple-music` | The parts of the Apple Music client written again |

## The clean-room rewrite

Code adapted from Kopuz (EUPL-1.2) is replaced by code written from these files alone, so that
Opium can leave the GPL ([ADR-0030](../adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md);
the inventory in [the licence audit](../audits/2026-11-05-licence.md)).

- **Two sides.** The *dirty side* read Kopuz's code and its adaptation in Opium, and wrote these
  specifications, describing behaviour only: no code, no names of functions, types or variables,
  no comments, error strings or test data taken from Kopuz. The *clean side* is an agent that has
  never read Kopuz's code nor its adaptation; it writes the new code from these files, the public
  standards they cite, and the parts of Opium that are Opium's own.
- **An allow-list, not a ban.** The clean side reads only what it is given: these specifications;
  Chromium's CDM header; and the Opium files each rewrite names, which are Opium's own. It is given
  them as an export (`git archive` of those files into a new repository with no history), never the
  project's repository, whose history and other documents (the licence audit, the older ADRs)
  describe the adapted code. Its brief forbids reading anything else, Kopuz's sources and the
  network included; the full log of its tool calls is read afterwards to check it.
- **Prepared by the dirty side first.** Each specification fixes the public names of what it
  describes, and lists what its export holds. In the export, the code given already uses those
  names, and the removed items' tests are gone: no name of Kopuz's reaches the clean side. The
  callers left outside the export are moved to the new names by the dirty side, afterwards.
- **A question** the specifications do not answer goes back to the dirty side, which answers it in
  these files, as behaviour; the questions and answers are kept with the evidence.
- **The check afterwards.** A token comparison of the new code with Kopuz at both revisions, with
  Opium's adapted files as they were, and with the whole tree:
  - once on the raw tokens, once with every identifier made the same;
  - sequences of Chromium's header, which both sides must use, are left out;
  - every shared run of 50 raw tokens or 30 normalised ones is read by hand and explained, or
    rewritten. For calibration: on 2026-11-05, Opium's own code shared at most 46 raw tokens with
    Kopuz, and `fmp4.rs` 590.
  - A search for Kopuz's names, comments, error strings and test data.
  - A reviewer that wrote neither side compares them.
- **The evidence**, kept in [`../audits/`](../audits/README.md): the clean side's brief, its
  questions and their answers, a summary of its tool calls (paths outside the repository
  redacted), the comparison's script and report.

This is how a team usually shows that new code is its own; it is not legal advice.
