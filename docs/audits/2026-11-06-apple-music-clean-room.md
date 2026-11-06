# 2026-11-06: `apple-music`'s derived items written again in a clean room (M1.12)

- **Scope**: the items of `crates/apple-music` derived from Kopuz, written again from [its
  specification](../specs/apple-music-playback.md) by [the method](../specs/README.md) of
  [ADR-0030](../adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md); the GPL kept
  ([ADR-0032](../adr/0032-the-gpl-kept-and-kopuzs-code-still-written-again.md)); the token
  comparison afterwards.
- **Starting commit**: `c132830`; the specification corrected at `8d3a88e`, the figures before at
  `3d7b225`. The adapted items, their tests and the notices of `playback.rs` and `lib.rs` are
  deleted; `auth.rs` is a new file.
- **Sides**:
  - dirty side: the session that read Kopuz and the adapted code; it checked the specification
    against the code and Apple's public page, measured before, made the export, answered the
    assumptions, brought the code back, compared it and wrote this file;
  - clean side: one new agent, which had read neither Kopuz nor the adapted files, before or
    during.
- This is how a team usually shows that new code is its own; it is not legal advice.

## Before the hand-over

The specification, written at M1.9, was read again against the code and, where it describes
Apple, against what Apple does. Corrected in `8d3a88e`:

- **The licence's items**: §12's `take_licence` lacked the key id that `widevine` waits for since
  M1.11, and the phone's MediaDrm needs the licence's bytes alone: two items now, `ask_licence`
  and `take_licence`.
- **The playlist's key**: §4 said "the first `EXT-X-KEY`"; the code that plays Apple's songs every
  day, and `m3u8-rs`, take the one in force for the first segment, the last before it.
- **Uploads**: §3 said they play without a licence; their caller licenses an encrypted one.
- **`songId`**: a string `"0"` names a song in the code, not in §2.
- **What was missing**: how the tests reach a local server; the renewal before `exp` and a
  download without a length, both in code §12 did not hand over; how the next page's added
  parameters are written; the station's errors and its id; the lyrics' "none".
- **Apple's page**, fetched once with the owner's go, without an account: `/` answers a 301 to a
  storefront's page (`/us/new`), which §9 did not say; the page (2.4 MB) names
  `/assets/index~cc245af156.js` in a `<script type="module">`; the script (3.3 MB) holds three
  ES256 JWTs with an `exp`, about 70 days after their `iat`, the first 0.9 MB in. Kept outside the
  repository; the token never printed.

The figures before were taken then, with the owner's go
([measurements](../measurements.md#apple-musics-steps-m112-2026-11-05-and-2026-11-06)), by a program kept in the
repository, [`am_cost.rs`](../../crates/apple-music/examples/am_cost.rs).

## The export

A new git repository outside Opium's tree, with no history: one commit of the allowed files, then
`Cargo.lock` cut down to what the exported crates build with, no version changed (commit
`f6dd554`):

- `docs/specs/README.md`, `docs/specs/apple-music-playback.md`;
- `crates/apple-music`: its `Cargo.toml`, without the dev-dependency on `player`, and its sources
  with §12's items removed, their tests with them, the notices of `playback.rs` and `lib.rs`, and
  `auth.rs` whole; the callers already on §12's names. Not its examples.
- `crates/opium-core`, `crates/browser-signin`, `crates/cenc` (without `fuzz/`) and
  `crates/widevine`;
- a workspace `Cargo.toml` for those five, `Cargo.lock`, `flake.nix`, `flake.lock`, `.gitignore`.

The export failed to build on exactly the missing items. A search of it for Kopuz's names found
Kopuz named only in the specifications' README and in six comments of Opium's own code that
credit Kopuz with facts about Apple's service (`lib.rs`'s requests' forms, `convert.rs`'s
`favorited` tag), not with code. Names it shares with Kopuz's Apple Music files are Apple's JSON
fields in snake case (`adam_id`, `key_uri`, `is_library`), `m3u8-rs`'s (`media_playlist`) and the
standard library's.

## The brief

Word for word, but for the path of the project's folder:
[brief.md](2026-11-06-apple-music-clean-room/brief.md). It allows only the export, the public
standards the specification cites and the dependencies' documentation built inside the export, and
forbids the rest of the disk, Opium's repository and its history, `/nix/store` and `~/.cargo`
read directly, the network but `127.0.0.1`, a CDM, an account, and other agents.

## Questions and answers

The clean side asked no question that stopped it. Its assumptions where the specification is
silent, each kept by the dirty side, the first five written into the specification (commit
`10e31ba`):

- `isLibrary` says whether the song was asked for by a library id, as Opium's callers pass it, even
  with a catalog `adamId`: §5's "that id" read the other way.
- A page or a script answering a failure status is out of reach, a network error: a renewal that
  meets one is tried again at the next request, rather than taken for a sign-out.
- A `next` that does not resolve gets a `/` before it, instead of staying as it is: put after the
  API's address, `evil.example:99` would change the host. The specification was wrong.
- A `null` flavour is no flavour; an empty `license` is none; an `errorCode` of any number but 0,
  whole or not, is a refusal.
- The lyrics' "first TTML" is the first entry carrying one; if it does not read, the next endpoint
  is asked.
- A station's count of 0 is looked at before its id; three answers in a row that bring nothing new,
  failures counted, stop new calls, those in flight still awaited; two songs a call are counted on;
  "the station offered no songs" and a refused id are protocol errors (a playback error before:
  no caller looks at the kind).
- A JWT's header must begin `eyJ` and not sit inside a longer base64url run; the script's `src`
  must be the path exactly; the page and the script are read up to 64 MiB each.
- A renewal that meets a network error is not recorded, so near the token's expiry each request
  tries the web player again while it is out of reach: the crate's behaviour on a 401 already.
- `give_licence` may block the async thread up to ten seconds, as before.

## What changed outside §12's items

All in the crate, as §12 allows, each read by the dirty side in the export's history:

- `send` reads a token within ten minutes of its `exp`, or past it, again first, through the
  existing renewal; the old token is sent if that fails.
- `as_it_downloads` reads a download without a length whole (resumed if it breaks, up to 512 MiB),
  then indexes and decrypts it; with a length it streams as before.
- The seam: a private `Hosts` holds Apple's four addresses (only `Hosts::apple()` outside tests);
  the client's building moved out of `connect` unchanged; `connect` goes through
  `connect_through`; every `format!("{API}{path}")` became `self.api(path)`. Tests resolve Apple's
  host names to `127.0.0.1` with `reqwest`'s `resolve`, so that the real redirect policy runs.
- Three defects of Opium's own code fixed, each with a test that panicked before ("attempt to
  subtract with overflow"): `range` with a length of 0 or past the largest offset, `padding_of` and
  `lesser_kbps` on playlists whose ranges overflow.
- `thiserror` removed from the crate's dependencies: nothing used it.
- Two lines reflowed by `rustfmt`.

The clean side also found that `cenc`'s `AudioStream::downloading` reserved memory for whatever
length a server announced: a huge one aborted the app. Fixed by the dirty side in `cenc`, Opium's
own, with a test that panicked first ("capacity overflow") (commit `316a8ee`). No other
reservation in the tree takes a length from a server.

## The tool calls

Read by the dirty side from the transcript, with a script that lists every call, every absolute
path and every command that could reach the network:

| Calls | Reads outside the export | Network | Notes |
|---|---|---|---|
| 81: 67 shell, 10 reads, 4 writes | none | none | Every `nix` call `--offline`, every `cargo` call offline. The absolute paths named outside the export are the tests' addresses (`/v1/me/library/songs`, `/assets/index~ab.js`, …), `/dev/null` and `/usr/bin/env`. Dependencies' interfaces read from `cargo doc --offline` under the export's `target/`. No subagent. |

No call named Kopuz, the adapted files, Opium's folder, a browser profile, a token, a CDM,
`/nix/store` or `~/.cargo`.

## The token comparison

[`compare.sh`](2026-11-06-apple-music-clean-room/compare.sh) runs M1.10's
[`token_compare.py`](2026-11-05-cenc-clean-room/token_compare.py). Only part of the crate was
written again, so its targets are the clean side's own lines, every line its commits added, file
by file. They are compared with Kopuz at `a58c385` and `1d8b323` whole, with the lines the dirty
side removed for the export, with the adapted files whole at `3d7b225`, and with the rest of the
tree. The full report is [`comparison.md`](2026-11-06-apple-music-clean-room/comparison.md).

| Against | Raw: runs of 50 or more | Raw: longest below | Normalised: runs of 30 or more | Normalised: longest |
|---|---|---|---|---|
| Kopuz `a58c385` | 0 | 27 | 58 | 49 |
| Kopuz `1d8b323` | 0 | 27 | 58 | 49 |
| The lines removed for the export | 1 | 49 | 10 | 155 |
| The adapted files whole | 3 | 49 | 18 | 156 |
| The rest of the tree | 4 | 49 | 38 | 131 |

Read by hand:

- **Against Kopuz**: no run of 28 raw tokens. The 58 normalised runs fall in code that does
  nothing of the kind (Kopuz's interface, store, database, player, YouTube clients): `use` lists,
  a test server's `BufReader::new(stream)` and `String::new()`, struct literals,
  `iter().filter(…)` chains. None falls in Kopuz's Apple Music code. No string and no comment
  phrase in common.
- **Against the lines removed**:
  - `apple_id` (51 raw, 155 normalised): nearly the same as the stopgap the dirty side wrote on
    2026-11-05, the error message included. That stopgap was not Kopuz's (Kopuz guards an empty
    id only, and shares no run of 28 raw tokens with the new code); §12 gives its rule word for
    word, and both are a model's plainest form of it.
  - `take_licence` (94 normalised) and `ask_licence`'s head and body (57 and 34): signatures that
    §12 fixes, the JSON keys of §5 in the order §5 lists them, and the one call `widevine`'s
    interface allows, its error wrapped as §5 says.
  - `catalog_file`'s head (61): §12's signature, then `m3u8-rs`'s parsing and the first
    segment's `key`, as §4 says.
  - The lyrics' loop (53): `fetch` into `types::LyricsResponse`, which §8 names.
  - `choose_source`'s head (32 and 32), `next_page_path`'s (30) and `id_kind`'s rule (30): §12's
    signatures, §3's error message, §1's rule.
- **Against the adapted files whole**: the same, and the client's building moved out of `connect`
  unchanged (141 raw), the renewal's call in `send` (62 raw).
- **Against the tree**: `test_support.rs`'s fragmented MP4 builder follows `cenc`'s own test
  builders (68, 66, 65 and 52 raw), which the clean side read in the export; Opium's idioms.
- **Names** shared with Kopuz's Apple Music files: the standard library's, `serde_json`'s,
  `m3u8-rs`'s, Apple's JSON fields, `AppleMusic`, `USER_AGENT` and `asset_url`.
- **Strings**: with Kopuz none; with the removed lines, the specification's own error messages
  ("webPlayback returned no assets", "no Widevine asset (only FairPlay's)", "licence refused",
  "licence response without a licence", "the station offered no songs") and `apple_id`'s.
- **The token's expression**: `eyJ[A-Za-z0-9_-]*\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+` resembles
  Kopuz's `eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+`: a JWT's shape (RFC 7519, a JSON
  header's `{"` in base64url), one token to the comparison. The page's search is a tag reader,
  not Kopuz's expression.

Nothing above the thresholds is owed to Kopuz.

## Checked

- `scripts/check.sh` passes: the crate's 118 tests, `cenc`'s 76, every other test, lints,
  documentation and links.
- Measured with the owner's go, alternately with the old code: the same cleartext; a station's
  batch 43 % quicker; the developer token, the library and a song's start the same within the
  network's spread; the token's search 4.5 µs slower once warm, accepted by the owner
  ([measurements](../measurements.md#apple-musics-steps-m112-2026-11-05-and-2026-11-06)).
- Opium on a throwaway profile, release build, in a headless Weston, no network, no session bus:
  it starts, runs 15 s and ends on SIGTERM, writing only in its folder. It talks to no Apple
  service: that needs the owner's account.
- Not verified: `take_licence` has no test (it needs a CDM session); no song was played with the
  new code, nor the radio, Spatial Audio, the library's songs or a station's songs played; the
  phone's Kotlin, which calls the licence through `opium-ffi`, was not built (Google's repository
  answers 404, M1.5).

## Better than Kopuz

What the [licence audit](2026-11-05-licence.md#4-what-kopuz-does-badly-here-and-opium-with-it)
found in Apple Music, and the specification's §11:

- the HTTP behaviours tested against a local server: the token's renewal, before its `exp` and on a
  401; a 429; a redirect off Apple's hosts; a licence refused; a playlist without a key; the
  library's pages; a station's bounds; the lyrics' order;
- mutation tests, seeded, over everything that reads Apple's answers and Opium's readers beside
  them;
- a download without a `Content-Length` plays;
- a song's id of no known kind, and a station's id that is not an Apple id, are refused before any
  request; the developer token's text is checked to be an ES256 token; a `next` cannot change the
  host;
- a station gathers with up to six calls in flight, bounded to 32, and ends early when it repeats
  itself.
