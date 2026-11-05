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
- The entry's `songId`, a string or a number, names the catalog song a library song was matched
  to; `-1`, `0`, an empty string or none means there is none (an upload).

## 3. Choosing what to play

The entry's `assets` is an array of objects, each with a `flavor` (a string) and a `URL`.

- **An upload**: exactly one asset, with no `flavor` and a non-empty `URL`. It is played as it is,
  without a licence.
- **A catalog song**: among the assets whose `flavor` contains `:ctrp` (AES-CTR, which Widevine can
  open; `:cbcp` ones are FairPlay's) and whose `URL` is not empty, the one with the highest bit
  rate, the number after `:ctrp` (`28:ctrp256` is 256 kbps, `32:ctrp64` is 64). Its `URL` is an HLS
  media playlist.
- No asset at all is the error "webPlayback returned no assets"; assets but none of `ctrp` is "no
  Widevine asset (only FairPlay's)".

## 4. Reading the catalog song's media playlist (RFC 8216)

- The playlist is fetched with the asset request (`playback.rs`'s own code, given in the export),
  its HTTP status checked.
- The key is the first segment's first `EXT-X-KEY` whose `METHOD` is not `NONE`. Its `URI` has the
  form `<prefix>,<base64>`: the text after the first comma, in standard base64 with its padding, is
  the 16-byte key id. The whole `URI` is kept: the licence request names it.
- Its `EXT-X-MAP` `URI` is the whole fragmented MP4 (Apple maps every segment onto one file). It is
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
  - `isLibrary`: whether that id is a library id;
  - `user-initiated`: `true`.
- An HTTP 401 or 403 is "not signed in"; another failure status names it.
- The answer is JSON. An `errorCode` other than 0 is "licence refused (errorCode N)"; otherwise
  `license` is the licence, base64, given to the CDM session; missing or undecodable is "licence
  response without a licence".
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
- The result is a path and query relative to `https://amp-api.music.apple.com`.

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

## 8. Lyrics

For a catalog song, `GET /v1/catalog/<storefront>/songs/<id>/syllable-lyrics` (word-timed), then if
it gives none, `.../lyrics` (line-timed); each answer's `data[].attributes.ttml` is TTML, parsed by
`lyrics.rs` (unchanged). An upload (`i.` id) has none.

## 9. The developer token

Apple's web player carries a developer token, a JWT (RFC 7519), in its main script.

- `GET https://music.apple.com/`: the page names its main script, an address of the form
  `/assets/index~<hash>.js` (Apple has also used `index-<hash>.js`; both are looked for).
- `GET` that script: the token is the first JWT in it, three base64url segments joined by dots,
  whose first segment decodes to a JSON header naming `"alg": "ES256"`.
- Its payload's `exp` (seconds since the epoch) says when it expires: Opium renews it ten minutes
  before that time, besides renewing it on a 401 as today.
- Each step that fails names itself: the page out of reach (its status), no script named, no token
  in the script, a token that does not decode.

## 10. Tests

- The conversion tests of `convert.rs` use made-up songs: titles, artists, durations and ids
  invented for the test, none taken from another project.
- The HTTP behaviours above are tested against a local server written for the tests (on
  `std::net::TcpListener`, no new dependency): the 401 that renews the token, a 429 with
  `Retry-After`, a redirect off Apple's hosts refused, a licence refused, a playlist without a key.

## 11. Better than before, where this rewrite can

- A download without a `Content-Length` plays once fully read, instead of failing with "no length".
- A playlist removal naming no song is refused before any request (fixed on 2026-11-05, before the
  rewrite).

## 12. What is written again, and what the clean side is given

The items below are removed by the dirty side, their tests with them, and written again by the
clean side under the names given. Their callers in the export already use these names.

| Behaviour | Section | New name, in the file it lives in |
|---|---|---|
| Telling the two kinds of ids apart | §1 | `IdKind { Catalog, Library }` and `fn id_kind(id: &str) -> Option<IdKind>`, `playback.rs` |
| The `webPlayback` body | §2 | `fn playback_request(id: &str, kind: IdKind) -> serde_json::Value`, `playback.rs` |
| Choosing what to play | §3 | `enum Source { Upload(String), Encode { playlist: String, kbps: u32 } }` and `fn choose_source(entry: &serde_json::Value) -> Result<Source>`, `playback.rs` |
| Reading the media playlist | §4 | `fn catalog_file(playlist_url: &str, body: &[u8]) -> Option<EncryptedFile>`, `playback.rs` (`EncryptedFile` and `Key` are Opium's own and stay) |
| Taking the licence into a session | §5 | `async fn take_licence(&self, session: &Session, challenge: &[u8], adam_id: &str, is_library: bool, key_uri: &str) -> Result<()>`, `playback.rs` |
| The next page's address | §6 | `fn next_page_path(next: &str, first: &str) -> String`, `lib.rs` (`at_offset` calls it) |
| A station's next songs | §7 | the body of `station_tracks`, `lib.rs` |
| Lyrics' order | §8 | the body of `lyrics_of`, `lib.rs` |
| The developer token | §9 | `pub async fn developer_token(http: &reqwest::Client) -> Result<String>`, `auth.rs` (whole file) |
| The ids a change may carry | — | `fn apple_id(id: &str) -> Result<&str>` in `lib.rs`: written on 2026-11-05 by the dirty side as a stopgap, written again: a catalog number, or one to three lowercase letters, a dot and one or more ASCII letters, digits or dashes; anything else refused before a request that changes the account |
| The conversion tests | §10 | new tests in `convert.rs` |

The export holds: this file and [README.md](README.md); the crate's `Cargo.toml` and sources with
the items above removed; `opium-core`, `browser-signin`, `cenc` and `widevine` as built at M1.11's
end, which are Opium's own or already written again; a workspace `Cargo.toml` and `Cargo.lock` for
them.

