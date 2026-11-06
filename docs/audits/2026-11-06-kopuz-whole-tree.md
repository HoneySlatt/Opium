# 2026-11-06: Opium's whole tree compared with Kopuz (M1.13)

- **Scope**: Opium's tracked text files against Kopuz (EUPL-1.2) at the revision Opium
  cited, `a58c385` (2026-10-28), and at its latest, `1d8b323` (2026-11-04), by
  [the method](../specs/README.md#the-clean-room-rewrite)'s check afterwards
  ([ADR-0030](../adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md),
  [ADR-0032](../adr/0032-the-gpl-kept-and-kopuzs-code-still-written-again.md)); the credits that
  name Kopuz made true.
- **Starting commit**: `72d99e4`, after M1.10 to M1.12 wrote `cenc`, `widevine` and
  `apple-music`'s derived items again.
- **Found**, two parts the [licence audit](2026-11-05-licence.md) had missed:
  - `DevTools` in `crates/browser-signin`, still adapted from Kopuz. The owner kept it, named
    ([ADR-0033](../adr/0033-the-browser-sign-ins-pipe-kept-as-adapted-from-kopuz.md)).
  - The test helper `boxed` of `player/src/gapless.rs`, Kopuz's nearly word for word, and its
    variant in `apple-music/src/radio.rs`: the owner had both take the form M1.10's clean side
    wrote in `cenc`'s tests.
  - Nothing else is owed to Kopuz.
- This is how a team usually shows that code is its own; it is not legal advice.

## The comparison

[`compare.sh`](2026-11-06-kopuz-whole-tree/compare.sh) fetches Kopuz at both revisions with
`nix flake prefetch` and runs M1.10's
[`token_compare.py`](2026-11-05-cenc-clean-room/token_compare.py), extended for this milestone:

- **`--all-text`**: Kotlin, JSON, shell, Python, Nix, TOML, YAML, Fluent, desktop entries, XML, SVG
  and Markdown are read too, each by its own rules (comments, strings). Only Rust, C, C++, Kotlin
  and Java are compared with identifiers made the same: elsewhere nearly every word is an
  identifier, and every text would look alike. Without the option, the script reads as before:
  M1.11's report, run again with the old and the new script, came out the same byte for byte.
- **An origin that is a folder** is searched whole: its compound names, its strings of three
  words or more (Fluent's messages too), its comment and prose phrases of six words, and its data
  (literals of ten characters or more with a digit, numbers of six digits or more, literals of
  six characters or more in its tests), each with the files that hold it on both sides.
- **Files identical byte for byte**, binaries included, are listed at the end of the report.

The targets are Opium's 424 tracked files in those languages, this audit's included. Left out:
Chromium's two headers, which both trees vendor; the earlier comparisons' reports, which quote
code already compared; the lock files, which list dependencies; the licence texts; the dotfiles
(`.envrc`, `.gitignore`). Runs made only of the headers' tokens are left out. The sources are
Kopuz's 1 122 and 1 137 files in the same languages, its vendored `gpui` included; its CSS,
JavaScript, SQL, property lists and service files are not read, Opium having none. The full
report is [`comparison.md`](2026-11-06-kopuz-whole-tree/comparison.md); it takes under two
minutes and 1.1 GB.

| Against | Raw: runs of 50 or more | Raw: longest | Normalised: runs of 30 or more | Normalised: longest |
|---|---|---|---|---|
| Kopuz `a58c385` | 2 | 104 | 2 075 | 140 |
| Kopuz `1d8b323` | 2 | 104 | 2 078 | 140 |

The report's "longest below" column reads one less than the threshold whenever a run reaches it:
it tells nothing here.

## Read by hand

**The raw runs.**

- 104 tokens, `desktop/src/mpris.rs` against Kopuz's `daemon/player/src/systemint/linux.rs`: the
  `PlayerInterface` methods `minimum_rate` to `can_seek`, in the order `mpris-server`'s trait and
  its own examples declare them, with the obvious answers (`Ok(1.0)`, `Ok(true)`). The rest of
  both files differs: Opium's `can_seek` follows the track's length, its `can_quit` is true, its
  volume is real.
- 52 tokens, `connect/src/away.rs` against Kopuz's `shared/proto/src/pipe.rs`: tokio's
  `AsyncRead::poll_read` signature and the one way to pass it on to an inner stream.
- Below the threshold, the 18 other raw runs of 35 tokens or more (`--raw 35`) were read too:
  - `desktop/src/mpris.rs` 463–472 and 472–480 (49 and 39): `mpris-server`'s `RootInterface`
    methods and their obvious answers;
  - `connect/src/away.rs` 80–94 (41, 46, 48): tokio's `AsyncWrite` methods passed on;
  - `app-desktop/src/app.rs` 3151–3162 (43): the window's history, below;
  - `navidrome/src/types.rs` 40–46 (43): Subsonic's fields;
  - `widevine/src/pssh.rs` 4–6 and 58–61 (42, 35): Widevine's system id, a public UUID, and a
    test's expected `pssh` box, its header then that id, in ISO 23001-7's order;
  - `navidrome/src/resume.rs` 102–104 and 162–165 (40, 41): a test's local server,
    `bind("127.0.0.1:0")`, `local_addr`, `spawn`;
  - `player/src/lib.rs` 99–111 and 130–132 (37, 37, 35): `std::io`'s `Read` and `Seek` passed
    on, a live stream refusing to seek, for symphonia's `MediaSource`;
  - `local/src/scan.rs` 69–72 (36): `create_dir_all` of a path's parent before writing;
  - `navidrome/src/lib.rs` 85–89 (36): a `Debug` that hides secrets;
  - `opium-core/src/color.rs` 5–11 (35): sRGB's transfer function, its standard constants;
  - `browser-signin/src/lib.rs` (38): the condition below.

**The normalised runs**: 1 976 ranges of Opium's, each found at one or both revisions (1 977
before `boxed` was replaced). Each was
read on both sides, by 14 agents in parallel, about 140 runs each, with Opium's and Kopuz's lines
side by side: [`runs.py`](2026-11-06-kopuz-whole-tree/runs.py) prepares the parts and joins the
reading back; the reading, run by run, is
[`review.tsv`](2026-11-06-kopuz-whole-tree/review.tsv). Two aids came with each run, and set no
threshold: the share of identifiers the same on both sides, and the period of its shape when it
repeats, as a list does. The session that wrote this file read again every run classed `similar`
or `suspect`, and the 192 whose identifiers agree for 60 % or more and that have no period. The
agents wrote many `idiom` reasons by family (import lists, fields, test modules): a few such
lines say "names differ" of a run whose names are the same, standard-library imports; the class
holds, the sentence is too general.

| Class in `review.tsv` | Runs | What they are |
|---|---|---|
| `idiom` | 1 771 | `use` and Kotlin `import` lists, struct and data-class fields, derives, `match` on `Option`, iterator chains, test modules, `Mutex` poisoning, `read_dir` filtered by extension |
| `api`: an API or a standard | 199 | `mpris-server`'s, tokio's, symphonia's, `std::io`'s traits; Chromium's CDM interface; Apple's, SoundCloud's and Subsonic's JSON fields; ISO BMFF; Material 3's role names; sRGB |
| `similar`, explained | 5 | Below |
| `suspect`: owed to Kopuz | 1 | `browser-signin`, below |

226 of them pair Opium's Kotlin `import` lists with those of `CertificateVerifier.kt`, a file
Kopuz vendors from `rustls-platform-verifier`: only the shape of an import line is shared.

The five similar runs, each read again with both files, and two more read closely:

- **The window's history** (`app-desktop/src/app.rs` against `apps/kopuz/store/src/nav.rs`):
  `can_go_back`, `can_go_forward` and `go_back` over two stacks with `std::mem::replace`, a
  browser's history in its plain form; the bodies around differ.
- **The decrypting stream's end** (`cenc/src/stream.rs` against
  `daemon/server/src/stream/range_source.rs`): a `Shared` of a `Mutex` and a `Condvar` named
  `changed`, and a `Drop` that sets a flag and wakes the waiters, the condition variable's usual
  form; the states and their flags differ.
- **The cover's change**, two runs (`local/src/tags.rs` against `shared/api/src/mutations.rs` and
  `daemon/reader/src/metadata.rs`): an enum of three cases, kept, removed and set to bytes, and a
  loop `while !tag.pictures().is_empty() { tag.remove_picture(0); }`, the one way to clear the
  pictures, which `lofty` has no call for. The edit's fields and its numbers' handling differ.
- **A growing download's reader** (`opium-core/src/growing.rs` against Kopuz's
  `range_source.rs`): a blocking `Read` that copies what has come and waits for more, its textbook
  form; one buffer here, chunks there.
- **The LRC reader** (`lyrics/src/lrc.rs` against `daemon/server/src/lyrics/lrc.rs`, classed
  `api`): the tags `[…]` taken off a line's head in a loop, as the format lays them; names,
  splitting and the handling of other tags differ.
- **The test helper `boxed`**, in six files. It came into Opium on 2026-10-29 with the tests
  adapted from Kopuz's `cenc.rs`. In `player/src/gapless.rs` it was still Kopuz's body nearly
  word for word, a 72-token run, and `apple-music/src/radio.rs` had a variant of it. Four files
  already had another form, written by M1.10's clean side: `u32::try_from(8 + body.len())`, then
  `concat()`. With the owner's go, the two took that form; the run is gone, their tests pass.

**Names**: 1 066 compound identifiers in common at `a58c385`. They are the standard library's, the
crates' (`mpris-server`, symphonia, lofty, tokio, `m3u8-rs`), Chromium's CDM interface's, Android's,
the services' JSON fields, and the words of any music player (`album_id`, `track_number`). The names
of Kopuz's old shim (`wv_open`, `wv_decrypt`) and `index_fmp4` are found only in the documents that
describe the code before M1.10 (ADR-0004, the licence audit).

**Strings**: 19 of three words or more. Interface words any player writes ("Add to Queue",
"Nothing here yet", "Sign in again"), separators (" · "), Material's shuffle icon (Google's, under
Apache-2.0, as the licence audit records), Cargo's `cfg(target_os = "android")`.

**Phrases**: 18 of six words, one of them this audit quoting a command, one Kopuz's copyright
line in the notice below.

- "only async-signal-safe calls between fork and exec", in `browser-signin`, below.
- "the CDM has no device certificate" (`widevine/src/error.rs`, `docs/specs/widevine-host.md`):
  the specification's words for Chromium's error, which Kopuz's comment names alike.
- "in the file, and what the CDM needs" (the cenc audit): the dirty side's description.
- The command `cargo clippy --workspace --all-targets -- -D warnings`, and plain English ("at the
  end of the song", "in the order they were added").

**Data**: 260 in common at `a58c385`: Apple's addresses (`/v1/me/library/songs`), Subsonic's and
SoundCloud's fields, box types (`b"moof"`), MIME types, the sRGB, Rec. 709 and FNV-1a constants,
colours (`#000000`), Chromium's Flatpak names, the Nix systems, and test words ("Hello world",
"Unknown Artist") in fixtures that differ. No song, id or account of Kopuz's tests.

**Identical files**: only Chromium's `content_decryption_module.h` and
`content_decryption_module_export.h`, Chromium's code under its BSD licence, which Opium takes
from Chromium at a pinned commit (`crates/widevine/vendor/README.md`).

## Owed to Kopuz: the sign-in browser's pipe

`DevTools` in `crates/browser-signin/src/lib.rs` starts the browser with
`--remote-debugging-pipe` and calls it. It came into Opium on 2026-10-30 (`f073144`), two days
after `a58c385`, and follows Kopuz's `daemon/server/src/cookies/cdp.rs`:

- the `SAFETY` comment's first sentence, word for word;
- the descriptors copied above 10 with `fcntl`, then `dup2` onto 3 and 4, under the same four-part
  condition, its error the same;
- the two pipes made in the same order, wrapped as tokio's `pipe::Sender` and a `BufReader` of
  `pipe::Receiver`, beside a call counter;
- answers read up to their NUL, as Chromium's protocol frames them.

Chromium fixes the descriptors 3 and 4 and the NUL; the move above 10, the condition and the
comment are Kopuz's expression. The rest of the crate is Opium's own: the browser's wait until
closed, the cookies' filter by domain, the errors.

The owner was given three choices: a clean-room rewrite, a rewrite by this session, or keeping it,
named. The owner kept it
([ADR-0033](../adr/0033-the-browser-sign-ins-pipe-kept-as-adapted-from-kopuz.md)). The crate's
documentation now carries Kopuz's notice, as the adapted files did.

## The credits

Made true:

- the about screens, desktop (`app-desktop/src/ui/settings.rs`) and phone (`SettingsSheet.kt`):
  "GPL-3.0-or-later. Part of the browser sign-in adapted from Kopuz (EUPL-1.2)." The desktop's
  said "Widevine and CENC code adapted from Kopuz", no longer true; the phone's credited only the
  way to Apple's web player. The phone's core holds the crate too, through `apple-music`. A change
  of text that ADR-0032 decided, the only one this milestone makes to the frozen screens
  (ADR-0026, ADR-0031);
- the README's state and credits, the CHANGELOG, the specifications' README, the ROADMAP's licence
  row and Stage 1's exit.

Kept: Kopuz credited for the facts it found about Apple's service (`apple-music`'s comments, "Kopuz
found the form"; the README), and for the theme watch's idea (`app-desktop/src/app.rs`). The ADRs'
bodies and the earlier audits stay as written. The licence audit listed no other notice: the
adapted files' notices left with their files at M1.10 to M1.12.

## Checked, and not

- Checked: the comparison at both revisions; every run above the thresholds read, and the raw runs
  from 35 tokens; the strings, phrases, data and names; the files identical; M1.11's report
  reproduced by the extended script; `scripts/check.sh`; the release build started on a throwaway
  profile in a headless Weston, without network, and its capture saved; the new about text found
  in that build's binary. A reviewer that wrote neither the code nor this file read 30 runs again,
  and agreed with every class.
- Not checked:
  - the about screens drawn: the screenshot mode opens a page only once playlists have loaded,
    which an empty profile has none of, and cannot scroll to the about card; the phone's APK was
    not rebuilt;
  - Kopuz's revisions between `a58c385` and `1d8b323`, nor those before: code taken from one of
    them and changed since in Kopuz would show only as far as it still matches either end;
  - the files left out above; the reviewer compared the two desktop entries by hand, which share
    only freedesktop's keys;
  - code an agent wrote from memory of other public projects, which a comparison with Kopuz alone
    cannot see.
