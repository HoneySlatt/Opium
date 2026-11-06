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
  - clean side, four new agents in turn: round 1 wrote the items; round 2 changed what the
    milestone's review found and wrote the `cargo fuzz` targets; round 3 changed what the final
    review found; round 4 closed the gaps the last review found in the tests. None had read
    Kopuz or the adapted files, before or during.
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

## The briefs

Word for word, but for the path of the project's folder:
[round 1](2026-11-06-apple-music-clean-room/brief.md),
[round 2](2026-11-06-apple-music-clean-room/brief-round-2.md),
[round 3](2026-11-06-apple-music-clean-room/brief-round-3.md),
[round 4](2026-11-06-apple-music-clean-room/brief-round-4.md). Each allows only the export, the public
standards the specification cites and the dependencies' documentation built inside the export, and
forbids the rest of the disk, Opium's repository and its history, `/nix/store` and `~/.cargo`
read directly, the network but `127.0.0.1`, a CDM, an account, and other agents.

## Questions and answers

The clean side asked no question that stopped it. Its assumptions where the specification is
silent, each kept by the dirty side, the first five written into the specification (commit
`10e31ba`):

- `isLibrary` says whether the song was asked for by a library id, as Opium's callers pass it, even
  with a catalog `adamId`: §5's "that id" read the other way.
- A page or a script answering a failure status is out of reach, a network error: kept in round 1,
  then undone after the review (below), for the readings it made repeat.
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
- Round 2: the reading before `exp` bounded by its own time stamp beside `renew_developer_token`;
  `protected` and `fetch_track` check the song's id first, and `song_and_binaural` takes the
  kind checked; the crate's `fuzzing` feature and its `#[doc(hidden)]` module.
- Two lines reflowed by `rustfmt`.

The clean side also found that `cenc`'s `AudioStream::downloading` reserved memory for whatever
length a server announced: a huge one aborted the app. Fixed by the dirty side in `cenc`, Opium's
own, with a test that panicked first ("capacity overflow") (commit `316a8ee`). No other
reservation in the tree takes a length from a server.

## The tool calls

Read by the dirty side from the transcript, with a script that lists every call, every absolute
path and every command that could reach the network:

| Round | Calls | Reads outside the export | Network | Notes |
|---|---|---|---|---|
| 1 | 81: 67 shell, 10 reads, 4 writes | none | none | Every `nix` call `--offline`, every `cargo` call offline. The absolute paths named outside the export are the tests' addresses (`/v1/me/library/songs`, `/assets/index~ab.js`, …), `/dev/null` and `/usr/bin/env`. Dependencies' interfaces read from `cargo doc --offline` under the export's `target/`. No subagent. |
| 2 | 67: 63 shell, 2 writes, one tool lookup and one stop | none | none | The same. The stop ended its own fuzz build, which the agent tool had moved to the background. No subagent. |
| 3 | 40: 38 shell, 1 read, 1 edit | none | none | The same. No subagent. |
| 4 | 32: 30 shell, 2 edits | none | none | The same. No subagent. |

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
| Kopuz `a58c385` | 0 | 27 | 66 | 49 |
| Kopuz `1d8b323` | 0 | 27 | 66 | 49 |
| The lines removed for the export | 1 | 49 | 12 | 155 |
| The adapted files whole | 3 | 49 | 23 | 156 |
| The rest of the tree | 4 | 49 | 44 | 131 |

Read by hand:

- **Against Kopuz**: no run of 28 raw tokens. Of the 66 normalised runs, 64 fall in code that
  does nothing of the kind (Kopuz's interface, store, database, player, YouTube clients): `use`
  lists, struct literals, `iter().filter(…)` chains. One, 36 tokens, falls in Kopuz's Apple
  lyrics file (`daemon/server/src/lyrics/apple_music.rs`): the head of the tests' local server's
  request reader against a reader of Kopuz's, `fn …(stream: &TcpStream) -> Option<…> { let mut
  reader = BufReader::new(stream); let mut line = String::new();`, the standard library's way to
  read an HTTP request line. Another, 37 tokens, falls in Kopuz's `api.rs`: the head of round 3's
  `script_address` against that of Kopuz's paging merge, `fn …(…: &str, …: &str) -> String { let
  (…, …) = match ….…('?') { Some(…) =>`, a function of two strings that splits one at its `?`;
  the two do different jobs, and their bodies share nothing. No string and no comment phrase in
  common; the comparison searches the lyrics file's names too.
- **Against the lines removed**:
  - `apple_id` (51 raw, 155 normalised): nearly the same as the stopgap the dirty side wrote on
    2026-11-05. Its error message, "not an Apple Music id: {id:?}", is the stopgap's word for
    word, though it was not among the clean side's inputs: the export holds it nowhere, the
    specification included. Both sides are agents of the same kind given the same rule, and
    wrote it the same way. The stopgap was the dirty side's own, not Kopuz's (Kopuz guards an
    empty id only, and shares no run of 28 raw tokens with the new code): nothing of Kopuz's
    reached the clean side this way. It is the one place where the clean side's text matches
    text it was not given. The fuzz target `ids` restates the same rule (33 and 32 normalised).
  - `take_licence` (94 normalised) and `ask_licence`'s head and body (57 and 34): signatures that
    §12 fixes, the JSON keys of §5 in the order §5 lists them, and the one call `widevine`'s
    interface allows, its error wrapped as §5 says.
  - `catalog_file`'s head (61): §12's signature, then `m3u8-rs`'s parsing and the first
    segment's `key`, as §4 says.
  - The lyrics' loop (53): `fetch` into `types::LyricsResponse`, which §8 names.
  - `choose_source`'s head (32 and 32), `next_page_path`'s (30) and `id_kind`'s rule (30): §12's
    signatures, §3's error message, §1's rule.
- **Against the adapted files whole**: the same, and the client's building moved out of `connect`
  unchanged (141 raw); `next_station_songs` reading a status and a body as `fetch`, Opium's own,
  does (62 raw).
- **Against the tree**: `test_support.rs`'s fragmented MP4 builder follows `cenc`'s own test
  builders (68, 66, 65 and 52 raw), which the clean side read in the export; Opium's idioms.
- **Names** shared with Kopuz's Apple Music files: the standard library's, `serde_json`'s,
  `m3u8-rs`'s, Apple's JSON fields, `AppleMusic`, `USER_AGENT` and `asset_url`.
- **Strings**: with Kopuz none; with the removed lines, the specification's own error messages
  ("webPlayback returned no assets", "no Widevine asset (only FairPlay's)", "licence refused",
  "licence response without a licence", "the station offered no songs") and `apple_id`'s.
- **The expressions**: the token's, `eyJ[A-Za-z0-9_-]*\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+`,
  resembles Kopuz's `eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+`: a JWT's shape (RFC
  7519, a JSON header's `{"` in base64url). The script's, since round 3,
  `/assets/index[~-][^/?#"'<>\t\n\x0C\r ]+\.js`, is close to Kopuz's `/assets/index~[^/]+\.js`,
  a negated class then `.js`: §9 asks, after the final review, to find every form the code before
  found, and the code before was this expression. Each is one token to the comparison; the new
  one leaves out the characters no address holds, and the code around them checks what Kopuz's
  does not (the token's header ES256; the script asked of `music.apple.com` under `/assets/`).

Nothing above the thresholds is owed to Kopuz.

## The milestone's review, and round 2

A reviewer that wrote neither side read the milestone against `AGENTS.md`, ADR-0030 and ADR-0032,
the specification and this evidence, re-ran the comparison to the same report, and compared the
next page's addresses of the old and new code on thirteen Apple-shaped `next`s: the same. It found
nothing owed to Kopuz, nothing that would stop Apple Music from playing, and:

- **the token's renewal could repeat at every request**: a failure status of the web player was
  made a network error, which the renewal never records, so near the token's `exp` each request
  read the page again, a library's 85 pages up to 200 MB. The specification was corrected (a
  status is a protocol error again, as before; a reading before `exp` at most once in five
  minutes, whatever comes of it); round 2, with tests of a web player failing by status, by
  connection, and serving the same token;
- **a playlist's station refused**: `ra.cp-pl.u-…` has a dot after its prefix, which `apple_id`
  refuses, as the dirty side's specification asked. Now a station's id must be one path segment;
  round 2;
- **the main script found only in one tag's form**, stricter than the code before: now its address
  anywhere in the page, a query kept; round 2;
- **an id of no known kind reached the catalog's Spatial Audio lookup and a warm-up**: now refused
  before any request, with `spatial` on and off; round 2;
- ADR-0030's `cargo fuzz` targets left as debt without the owner's decision: put to the owner, who
  chose to make them; round 2, eight targets in `crates/apple-music/fuzz`, through a `fuzzing`
  feature off by default;
- the evidence: the 62-token run and the run in Kopuz's lyrics file explained wrongly, the
  `apple_id` message's origin said too softly: corrected above. A behaviour change unsaid: between
  encodes of the same bit rate, the first is played, where the code before played the last; the
  answers seen had one encode of each rate, but that was not checked for every song;
- the documents: the README and ADR-0031 still on MIT, M1.12 called done before playing, two
  constants told apart by one letter, a date and a median unsaid, smaller points: fixed.

Round 2's assumptions, kept: a script's address counts only standing alone (not after a letter,
digit or `- . _ ~ % / : @`, nor before a letter, digit or `- . _ ~ % /`), its query ends at a
quote, `<`, `>` or any space, a fragment dropped, entities left as written; a reading before
`exp` goes through the existing renewal, so it is skipped too within five minutes of a 401's
reading that got an answer; a station's id refused is a protocol error. `fetch_track`'s check has
no direct test (it needs a CDM); `song_and_binaural` takes the checked kind, so no path goes
around it.

## The final review, and round 3

A second reviewer, new, read the milestone again after round 2. It re-ran the comparison to the
same report, built and ran the fuzz targets, found every finding of the first review fixed, and
found:

- **a defect**: the reading before `exp` went through the 401's renewal and started its five
  minutes, so when Apple serves the old token until its `exp`, the first refusal after it read
  nothing, and Opium stayed signed out up to five minutes. Shown by a test of its own. The
  specification was corrected (a reading before `exp` never counts towards a 401's five minutes),
  round 3, with a test across `exp` that failed before;
- **the main script refused in forms the code before found** (`//music.apple.com/…`, another
  host, `/us/assets/…`, a dotted name such as `index~a1.min.js`), though §9 said "as tolerant".
  Now every form the code before found is found, but for names holding a space, a quote, `<`,
  `>`, `?` or `#`, which no address holds; the script is always asked of
  `https://music.apple.com`; round 3, with tests of each form;
- **a matched `songId` unchecked**, which goes into an address and the licence: now taken only
  when it is a catalog id; round 3;
- the `fuzzing` feature linted only weekly: now in `scripts/check.sh`; the PLAN calling M1.12 done
  in one place and not another; a figure given as 1.6 µs that was 1.7, and a p95 of 2.4 µs more
  unsaid; the token's timer unnamed and not kept: fixed, the timer now
  [`token-time.sh`](2026-11-06-apple-music-clean-room/token-time.sh).

Round 3's assumptions, kept: after `.js` only a query is looked at, so `index~ab.jsx` reads as
`index~ab.js`, as the code before read it; a `\` in the script's name is sent as `%5C`, so the
path stays under `/assets/`; a numeric `songId` is a whole number from 1 to `u64::MAX`, a string
`"0"` a catalog id by §1's digits; one reading at a time of either kind, under one lock.

## The last review, and round 4

A third reviewer, new, checked round 3. It found no defect: each fix holds, and its test fails
with the fix undone. It found two gaps in the tests (the 401's five minutes, and one reading at a
time of either kind, each removable with every test still passing), a test that waited on the
wall clock, and wording too broad or stale in the documents, since corrected. Round 4 wrote the
tests, and a test-only seam for the clock (`now`, the real clock in every other build). The dirty
side then removed each check in turn: without the 401's five minutes, two tests failed; without
the shared lock, one; restored, all passed.

## Checked

- `scripts/check.sh` passes: the crate's tests, `cenc`'s, every other test, lints, documentation
  and links; clippy with the `fuzzing` feature; `scripts/fuzz.sh 20` ran every target of both fuzz
  crates without a crash (35 thousand to 3.5 million runs each).
- Measured with the owner's go, alternately with the old code: the same cleartext; a station's
  batch 43 % quicker; the developer token, the library and a song's start the same within the
  network's spread; the token's search, final code, 1.2 to 1.3 µs slower once warm and 2 to 12 µs
  at a process's first, within the 4.5 µs the owner accepted
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
- `cargo fuzz` targets over the same readers, run weekly;
- a song's id of no known kind, and a station's id that is not one path segment, are refused before
  any request; the developer token's text is checked to be an ES256 token; a `next` cannot change
  the host; a reading of the token before its `exp` is bounded to one in five minutes;
- a station gathers with up to six calls in flight, bounded to 32, and ends early when it repeats
  itself.
