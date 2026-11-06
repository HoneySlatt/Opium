# Apple Music: the parts of `crates/apple-music` written again

The behaviour of the parts of `crates/apple-music` that the clean-room rewrite replaces
([README](README.md)); §12 lists them. The rest of the crate is Opium's own and stays as it is. It
is written from what Apple's web player at <https://music.apple.com> does, as Opium observed it and
as its tests pin it, and from the public standards it cites; it says what the code must do, not how
to structure it. Apple's private API can change without notice: what is here was true on 2026-11-05.

Standards cited: **RFC 8216** (HLS), **RFC 3986** (resolving a relative address), **RFC 7519**
(JWT), **RFC 9110** (HTTP).

## 1. The two kinds of song ids

- A **catalog id** is a decimal number (`1234567890`): a song anyone can play.
- A **library id** names an item of the user's own library: one of the letters `i` (a song), `l`
  (an album), `p` (a playlist) or `a` (an artist), a dot, then one or more ASCII letters, digits or
  dashes (`i.AbC9-x`).
- Anything else is neither, and is refused where an id must be one or the other.
- By playback time, only a user's own upload still carries a library id: songs matched to the
  catalog play by their catalog id.

## 2. Asking the web player for a song

- `POST https://play.music.apple.com/WebObjects/MZPlay.woa/wa/webPlayback`, with the developer
  token as a bearer, the user's token in `x-apple-music-user-token`, and the `Origin`, `Referer`
  and `Cookie: media-user-token=…` that every request of Opium's to Apple carries (`lib.rs`'s own
  code, given in the export).
- The JSON body has one key: `salableAdamId` for a catalog id, `universalLibraryId` for a library
  id; its value is the id as a string.
- The answer's `songList` is an array; its first entry is the song. An empty or missing list is
  the error "webPlayback returned no song".
- The entry's `songId` names the catalog song a library song was matched to: a number above 0, or
  a string that is a catalog id (§1). Anything else, or none, means there is none (an upload): it
  goes into addresses and the licence's body, so it is checked as the id asked for is. It is
  read by `playback.rs`'s own `song_id`, which may be changed for this.
- An id that is neither kind (§1) is refused with an error naming it before any request of the
  song's: before `webPlayback`, the catalog's lookup of its Spatial Audio, and the licence host's
  warm-up (`protected` and `fetch_track`, Opium's own, given in the export).

## 3. Choosing what to play

The entry's `assets` is an array of objects, each with a `flavor` (a string) and a `URL`.

- **An upload**: exactly one asset, with no `flavor` (or a `null` one) and a non-empty `URL`. The caller
  (`playback.rs`'s own code, given in the export) fetches that `URL`: an encrypted media playlist
  (§4) is licensed under the song's own id, anything else is the song's file itself.
- **A catalog song**: among the assets whose `flavor` contains `:ctrp` (AES-CTR, which Widevine can
  open; `:cbcp` ones are FairPlay's) and whose `URL` is not empty, the one with the highest bit
  rate: the rest of the flavour after `:ctrp`, a decimal number (`28:ctrp256` is 256 kbps,
  `32:ctrp64` is 64); the first of them in the list when several share it. An asset whose rest
  does not read as a number that fits in 32 bits is not a candidate. Its `URL` is an HLS media
  playlist.
- No `assets` array, or an empty one, is the error "webPlayback returned no assets"; assets but no
  candidate is "no Widevine asset (only FairPlay's)".

## 4. Reading the catalog song's media playlist (RFC 8216)

- The playlist is fetched with the asset request (`playback.rs`'s own code, given in the export),
  its HTTP status checked.
- It is read with `m3u8-rs`, already in the crate's dependencies, as a media playlist.
- The key is the one in force for the first segment: the last `EXT-X-KEY` before that segment's
  address, as `m3u8-rs` gives it in the segment's `key`. This is how Opium has read Apple's
  playlists in daily use. A `METHOD` of `NONE`, or no `URI`, is no key. Its `URI` has the form
  `<prefix>,<base64>`: the text after the first comma, in standard base64 with its padding, is
  the 16-byte key id. The whole `URI` is kept: the licence request names it.
- The `EXT-X-MAP` in force for the first segment names, in its `URI`, the whole fragmented MP4
  (Apple maps every segment onto one file). It is
  resolved against the playlist's own address as RFC 3986 §5 says (absolute stays as it is; a path
  relative to the playlist's folder; `..` and a leading `/` handled), with the `url` crate already
  in the crate's dependencies.
- A playlist that does not parse, or lacks the key, the map or a decodable key id, is "catalog asset
  without a Widevine key".

## 5. The licence

- `POST https://play.itunes.apple.com/WebObjects/MZPlay.woa/wa/acquireWebPlaybackLicense`, with the
  same headers as §2, and a JSON body:
  - `challenge`: the CDM's licence request, base64 (standard alphabet, padded);
  - `key-system`: `com.widevine.alpha`;
  - `uri`: the key's `URI` from the playlist;
  - `adamId`: the id the song is licensed as (the matched catalog id when there is one, §2);
  - `isLibrary`: whether the song was asked for by a library id (§2), as Opium's own callers
    pass it, even when `adamId` is the catalog id it was matched to;
  - `user-initiated`: `true`.
- The request is sent through `lib.rs`'s own `send`, which renews the developer token on a 401 and
  sends it again once. A 401 or 403 after that is "not signed in"; another failure status names
  it.
- The answer is JSON. An `errorCode` that is a number other than 0, whole or not, is "licence
  refused (errorCode N)"; otherwise `license` is the licence, base64 (standard alphabet, padded);
  missing, empty or undecodable is "licence response without a licence".
- Two uses, two items (§12):
  - asking for the licence alone, which answers its bytes: the phone's MediaDrm takes them itself
    (`AppleMusic::license`, Opium's own, calls it);
  - taking the licence into a CDM session: asked as above, then given to the session for the key
    id it unlocks with `widevine::Session::give_licence`, which waits until that key is usable. A
    failure of the session is a playback error carrying `widevine`'s message.
- The content key stays inside the CDM.

## 6. Paging through the library

Apple's library lists come in pages: an answer's `next` is the address of the following page.
Apple's `next` keeps the path and the `offset` but drops parameters the first request had (`limit`,
`include`, `l`), so later pages would shrink to Apple's default size and lose their inclusions.

- `next` may be a path or a whole address: either is resolved against
  `https://amp-api.music.apple.com`.
- The address of a later page is `next`, plus each query parameter of the first request that `next`
  does not have, with its first request's value. Where both have one, `next`'s value is kept.
  Parameters are compared by their names once percent-decoded (`ids%5Bsongs%5D` is `ids[songs]`).
- Parameters keep their order: `next`'s first, then the added ones in the first request's order.
- `next`'s own query is kept as Apple wrote it. The added parameters are written as the `url`
  crate's form serializer writes them (`application/x-www-form-urlencoded`: `ids[songs]` becomes
  `ids%5Bsongs%5D`, a space `+`).
- The result is a path and query relative to `https://amp-api.music.apple.com`, whatever host a
  whole `next` names: a page is only ever asked of that host. With no parameter at all, the result
  is the path alone, with no `?`. A `next` or a first request that does not resolve gives `next`
  as it is, with a `/` before it when it has none: put after the API's address, text such as
  `evil.example:99` would otherwise change the host.
- `at_offset` (Opium's own, given in the export) builds the address of a page at a given offset
  with it, as the library's pages are asked 32 at a time.

## 7. A station's next songs

`POST https://amp-api.music.apple.com/v1/me/stations/next-tracks/<station id>`, with the
`Media-User-Token` header and an empty body (`Content-Length: 0`), answers a few songs (usually
two). Concurrent calls share the station's cursor; the same song may come twice; a station never
ends.

Gathering about `count` songs (the caller asks for 10 to 25) must:

- be quick: the first songs of a station start playing while the rest arrive, so calls run
  concurrently, as few in flight as keeps the wait short;
- be bounded: at most six calls in flight and at most 32 calls in all; a station repeating
  itself, or answering nothing new, ends the gathering before that budget runs out;
- hold each song once (by id), in the order the songs came, and at most `count`;
- fail only when no song came at all: with the first failure met (signed out, out of reach), else
  "the station offered no songs".

And:

- A `count` of 0 sends nothing and answers no song.
- The station's id goes into the address unescaped, so it must stay one path segment: one or
  more ASCII letters, digits, dashes, dots or underscores, and neither `.` nor `..`. Anything
  else is refused before any request. Apple's station ids are not all of `apple_id`'s form: a
  playlist's station is `ra.cp-pl.u-<letters and digits>`, a dot after its prefix's name.
- Each call goes through `lib.rs`'s own `send`, which renews the developer token on a 401. A 401
  or 403 after that is "not signed in"; another failure status names the path and the status.
- An answer is a list of songs (`data`), each read as `types::Song` and made a `Track` by
  `convert::track`, both Opium's own. An answer that does not read is a protocol error naming the
  path.
- On 2026-11-05, the code before this rewrite, which sent rounds of four calls at once and waited
  for each round's last answer, gathered five batches of 20 on one station in 1.2 to 2.1 s, every
  song distinct. The new gathering must be no slower: with every call answering in the same
  time, it never waits for more calls one after another than those rounds did. It is measured
  against Apple afterwards, with the owner.

## 8. Lyrics

For a catalog song, `GET /v1/catalog/<storefront>/songs/<id>/syllable-lyrics` (word-timed), then if
it gives none, `.../lyrics` (line-timed); each answer's `data[].attributes.ttml` is TTML, parsed by
`lyrics.rs` (unchanged). An upload (`i.` id) has none.

- Both are read with `lib.rs`'s own `fetch` (a 404 is no answer) into `types::LyricsResponse`.
- An endpoint "gives none" when it answers 404, or no entry with TTML, or TTML that `lyrics.rs`
  cannot read: the next endpoint is asked. Any other failure ends the search with that error.
- The first TTML found in an answer is the one used.

## 9. The developer token

Apple's web player carries a developer token, a JWT (RFC 7519), in its main script.

- `GET https://music.apple.com/`, with the crate's one HTTP client (its user agent and its
  redirect policy, `lib.rs`'s own code). On 2026-11-05 Apple answered a 301 to a storefront's page
  (`/us/new`), which the client follows, then a page of 2.4 MB.
- The page names its main script: on 2026-11-05 in a `<script type="module" … src="…">`. It is
  the first text anywhere in the page, in a tag or not, of the form `/assets/index~<name>.js`
  (Apple has also used `index-<name>.js`; both are looked for), whatever comes before it (a
  scheme and a host, `//`, `.`, a folder such as `/us`), `<name>` one or more characters but
  `/`, `?`, `#`, a quote, `<`, `>` or a space (dots included: `index~a1.min.js`); a query right
  after it (`?`, then what follows up to a quote, a space, `<` or `>`) is kept. The script is
  always fetched from `https://music.apple.com`, at `/assets/index~<name>.js` (or `-`) and that
  query, whatever host or folder the page wrote before it: the token is never asked of another
  host. Every form the code before this rewrite found whose name holds none of the characters
  above, it finds; the code before found the path anywhere, those characters included, and
  fetched it from `https://music.apple.com`. A name with a space, a quote, `<`, `>`, `?` or `#`
  is no script's address.
- `GET` that script (3.3 MB on 2026-11-05): the token is the first JWT in it, three base64url
  segments joined by dots, whose first segment decodes to a JSON header naming `"alg": "ES256"`.
  Text shaped like a JWT whose header does not decode, or names another algorithm, is passed
  over. On 2026-11-05 the script held three ES256 JWTs (headers `typ`, `alg`, `kid`; payloads
  `iss`, `iat`, `exp`, `root_https_origin`), about 0.9 MB in; the first is the one that works,
  in daily use.
- Its payload's `exp` (seconds since the epoch) says when it expires (on 2026-11-05, about 70
  days after it was issued). A request about to be sent within ten minutes of that time first
  reads the token again, the old token kept when the reading fails. One such reading at a time,
  and at most one every five minutes, whatever its outcome, a failed connection included: the old
  token is still good until its `exp`, and a web player out of reach must not cost every request
  a reading. A token without a readable `exp` is renewed on a 401 only. The renewal on a 401 is
  `lib.rs`'s own: one reading at a time, at most one every five minutes after the page or the
  script answered. A reading before `exp` never counts towards a 401's five minutes: when Apple
  still serves the old token until its `exp`, then a new one, the first 401 after `exp` must read
  the page at once and get it, and is tested so, across `exp`.
- Each step that fails names itself: the page out of reach, no script named, the script out of
  reach, no token in the script, a payload that does not decode. No connection, or an answer cut
  short, is a network error; a page or a script that answers a failure status is a protocol
  error naming the status, as before this rewrite: the renewal on a 401 then waits its five
  minutes instead of reading the page again at every request.
- Searching the page and the script is a scan in time proportional to their length. On the files
  of 2026-11-05, in a release build: a process's first search about 0.58 ms before this rewrite
  and 2 to 12 µs more after; once its expressions are built, about 17 µs before and 1.2 to 1.3 µs
  more after, the ES256 check's cost, within the 4.5 µs the owner accepted
  ([measurements](../measurements.md#apple-musics-steps-m112-2026-11-05-and-2026-11-06)).

## 10. Tests

- The conversion tests of `convert.rs` use made-up songs: titles, artists, durations and ids
  invented for the test, none taken from another project.
- The HTTP behaviours above are tested against a local server written for the tests (on
  `std::net::TcpListener`, no new dependency), through the crate's own client and requests, its
  redirect policy included:
  - the developer token: read from a page and a script (§9), each failing step named, a token
    about to expire read again before a request, a 401 that renews it and sends the request again
    once;
  - a 429 with `Retry-After`: waited for and asked again once when the wait is short (`lib.rs`'s
    own `fetch`), not when it is long;
  - a redirect off Apple's hosts not followed;
  - a song asked (§2), its asset chosen (§3), its playlist read (§4), its licence asked and refused
    (§5), a playlist without a key;
  - the library's pages (§6): later pages asked with the first request's parameters;
  - a station (§7): the bounds on calls, in flight and in all, repeats dropped, an end before the
    budget when nothing new comes, and the failures;
  - lyrics (§8): the second endpoint asked only when the first gives none.
- The local server stands in for Apple's hosts in tests only (`#[cfg(test)]`): outside tests, the
  addresses are Apple's, and nothing can change them.
- Deterministic mutation tests, in `cargo test`, with a fixed seed and a generator written in the
  tests (no new dependency), over everything that reads what Apple sends: the ids, the
  `webPlayback` entry and its assets, the media playlist, the licence's answer, `next`, the page
  and the script of §9, a station's answer. Seeded inputs mutated thousands of times (bytes
  flipped, cut, repeated, fields of another type, huge numbers, empty and very long strings): no
  panic, no hang, and each result keeps its invariants (a key id of 16 bytes, an address on
  Apple's host, a bit rate read from the flavour, at most `count` songs, each once).
- The same mutation tests also cover Opium's own readers beside these, in the export: the
  enhanced playlist, its master's variant, the ExoPlayer playlists, the map's range, the AAC
  configuration's rate and the `songId`.
- `cargo fuzz` targets for the same readers ([ADR-0030](../adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md)):
  a crate `crates/apple-music/fuzz` outside the workspace, as `crates/cenc/fuzz` is, built in the
  nightly shell (`nix develop .#fuzz`) and run weekly by `scripts/fuzz.sh`, which runs every
  `crates/*/fuzz`. The readers are private: a crate feature `fuzzing`, off by default, exposes
  entry points to them in a `#[doc(hidden)]` module, and nothing else. Each target checks the
  invariants the mutation tests check.
- Tests also cover the renewal's bounds: a web player that keeps failing, by status and by
  connection, and one that keeps serving a token about to expire, each costing at most one
  reading in five minutes before the token's `exp`.

## 11. Better than before, where this rewrite can

- A download without a `Content-Length` (`as_it_downloads`, Opium's own, given in the export) is
  read whole, then indexed and decrypted, instead of failing with "no length". A download that
  has one streams as today.
- A playlist removal naming no song is refused before any request (fixed on 2026-11-05, before the
  rewrite).
- A song's id of neither kind (§2) and a station's id that is not one path segment (§7) are
  refused before any request.
- The developer token is read again before it expires (§9), and the script's text is checked to be
  a token (§9).

## 12. What is written again, and what the clean side is given

The items below are removed by the dirty side, their tests with them, and written again by the
clean side under the names given. Their callers in the export already use these names.

| Behaviour | Section | New name, in the file it lives in |
|---|---|---|
| Telling the two kinds of ids apart | §1 | `#[derive(Debug, Clone, Copy, PartialEq, Eq)] enum IdKind { Catalog, Library }` and `fn id_kind(id: &str) -> Option<IdKind>`, `playback.rs` |
| The `webPlayback` body | §2 | `fn playback_request(id: &str) -> Result<serde_json::Value>`, `playback.rs`: an id of neither kind is a playback error naming it |
| Choosing what to play | §3 | `#[derive(Debug, PartialEq)] enum Source { Upload(String), Encode { playlist: String, kbps: u32 } }` and `fn choose_source(entry: &serde_json::Value) -> Result<Source>`, `playback.rs` |
| Reading the media playlist | §4 | `fn catalog_file(playlist_url: &str, body: &[u8]) -> Option<EncryptedFile>`, `playback.rs` (`EncryptedFile` and `Key` are Opium's own and stay) |
| Asking for a licence | §5 | `async fn ask_licence(&self, challenge: &[u8], adam_id: &str, is_library: bool, key_uri: &str) -> Result<Vec<u8>>`, `playback.rs` |
| Taking the licence into a session | §5 | `async fn take_licence(&self, session: &Session, challenge: &[u8], adam_id: &str, is_library: bool, key_uri: &str, key_id: &[u8; 16]) -> Result<()>`, `playback.rs` |
| The next page's address | §6 | `fn next_page_path(next: &str, first: &str) -> String`, `lib.rs` (`at_offset` calls it) |
| A station's next songs | §7 | the body of `station_tracks`, `lib.rs` |
| Lyrics' order | §8 | the body of `lyrics_of`, `lib.rs` |
| The developer token | §9 | `pub async fn developer_token(http: &reqwest::Client) -> Result<String>` and `pub fn expires_at(token: &str) -> Option<std::time::SystemTime>` (its `exp`; `None` when it has none that reads), `auth.rs` (whole file) |
| The ids a change may carry | — | `fn apple_id(id: &str) -> Result<&str>` in `lib.rs`: written on 2026-11-05 by the dirty side as a stopgap, written again: a catalog number, or one to three lowercase letters, a dot and one or more ASCII letters, digits or dashes; anything else refused before a request that changes the account |
| The conversion tests | §10 | new tests in `convert.rs` |

The clean side also changes Opium's own code in the export, only where these sections ask, and
says where in its report:

- `send` in `lib.rs`, and the renewal's bookkeeping beside `renew_developer_token`: the renewal
  before the token expires and its bounds (§9);
- `protected` and `fetch_track` in `playback.rs`: a song's id checked before any request (§2);
- `song_id` in `playback.rs`: a matched `songId` taken only when it is a catalog id (§2);
- `as_it_downloads` in `playback.rs`: a download without a length (§11);
- the crate's `Cargo.toml` and a `fuzzing` module for the `cargo fuzz` targets, and the fuzz crate
  itself (§10);
- the addresses of Apple's hosts, and the building of the client, as the tests' seam needs (§10);
- tests anywhere in the crate.

The export holds: this file and [README.md](README.md); the crate's `Cargo.toml` (without its
dev-dependency on `player`, which only its examples use) and sources with the items above removed,
not its examples; `opium-core`, `browser-signin`, `cenc` and `widevine` as built at M1.11's end,
which are Opium's own or already written again; a workspace `Cargo.toml` and `Cargo.lock` for
them.
