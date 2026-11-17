# Architecture audit, 2026-11-17

Whether Opium is built so that contributors can understand it, work on it and change it without
weakening the rest; what is solid, what only looks so in the documents, what is missing, and what
costs more the later it is done. The owner asked for it after the method's rules of 2026-11-16. It
checks the [audit of 2026-11-16](2026-11-16-architecture.md) rather than taking it as given. It
fixed nothing; open items go to the [known debt](../debt.md), the decisions are at the end.

## Scope

- **Commit**: `10fc464` on `main`, clean.
- **Dimensions**: architecture and evolvability; stability and recovery; data safety; security of
  sessions and pairings; what a contributor meets; platforms and the port to uwui.
- **How**: the documents, the owner's project method of 2026-11-16 (its sources checked identical
  to what the agents read), the code paths and their tests. Three reviewers on the
  strongest model read the code with open questions and without the expected conclusions: state and
  concurrency; data and compatibility; contributors and platforms. The lead checked a dozen of their
  findings in the code; all held. A fourth, the challenger, attacked the recommendations; its
  objections changed them (below).
- **Not done**: nothing was built, run or measured, and no test was written. Every finding is read
  in the code; the races (SEC-1 to SEC-3, ARC-3, ERR-4) are traced, not reproduced. Apple's
  reactions (SEC-5, ERR-3's 403) are hypotheses for the owner to check on the real service.

## What holds

- The dependency rules are enforced by the build: no egui below `app-desktop`; `opium-ffi` without
  `player`, `motion` or `desktop`, which the Android job compiles. The graph in
  [ARCHITECTURE](../ARCHITECTURE.md#dependencies) matches every manifest.
- `opium_core::files::write_whole` (`files.rs:64`): random partial file opened `create_new`, file and
  folder flushed, mode from creation, tested against two writers and a stop before the rename.
  A file that does not read is put aside, never written over.
- `synced`: a clock that never goes back (`lib.rs:615`), implausible times refused, playlist ids from
  other devices checked before they name a file; the merge ignores order and repetition, by reading
  and by 58 example tests (see DON-9).
- The desktop's sign-out guards for connects and token renewals (`SignOuts`, `Connects`,
  `backend.rs:721`, `1251-1283`, `1534-1541`), tested in `backend/tests.rs` (but see SEC-2).
- The audio callback: atomics and an `rtrb` ring only, no lock or I/O (`player/src/output.rs`); a
  stalled engine replaced (`lib.rs:249-283`, tested); the next track opened off the engine and
  checked by generation.
- The CDM's host: one owner thread, licences before decrypts, shutdown and panic answer every
  request waiting.
- Connect's transport: clear frames capped at 64 KiB and sealed ones at 32 MiB, deadlines per piece
  and per step, backoff, 64 strangers at most, unpaired senders dropped.
- Sealed downloads: AES-GCM per 64 KiB chunk, the last chunk marked against truncation, the folder
  locked, no sweep of files a lost manifest may name; the 30 days count only the clock's forward
  motion.
- Secrets: `without_url()` wherever an address carries a signature; Navidrome's password never
  kept; `scrub_kept` on every cache read; no core dump by default.
- CI: actions pinned by SHA, `--locked`, exact cache keys, Gradle's SHA-256 verification, no git
  dependency; tests hermetic (a fake CDM, a local Apple server, mutation tests, weekly fuzzing).
- The documents say plainly what falls short.

## Findings

Each finding gives where, the mechanism, the consequence, its status and the check that would prove
a fix. "New" means neither the audit of 2026-11-16 nor `debt.md` held it.

### Sessions, security, honesty towards the services

- **SEC-1. On the phone, a sign-out is not final.** New, but for SoundCloud's renewal (debt).
  `opium-ffi/src/lib.rs:327`, `370`, `395` install a service after an `await` with no generation;
  `Core.kt:183-189`, `275-322`. A connect that ends after `signOut` puts the account back; `attempt`
  then shows `Connected` (`Core.kt:303-312`), `OpiumApp.kt:121-126` reloads the library and confirms
  the downloads' subscription for an account the user left, against
  [ADR-0017](../adr/0017-downloads-sealed-and-tied-to-the-subscription.md); Kotlin's counter cannot
  catch that confirmation, asked after the sign-out was counted (`Downloads.kt:235-242`).
  Navidrome's credentials are written back into the vault `signOut` emptied (`Core.kt:292`,
  `navidrome_kept`, `lib.rs:880`), and survive restarts. Security; confidence medium to high.
  Check: an `opium-ffi` test holding Navidrome's `ping`: connect, disconnect, release, and
  `services().navidrome` is `None` and nothing is kept.
- **SEC-2. On the desktop, a sign-out leaves work alive, and its own fence has holes.** New.
  - Untracked tasks (`keep_album`, `load_playlist`, `apple_lists`, `backend.rs:2117-2590`) write the
    cache after `cache::clear`; their replies are dropped by the window only for a service turned
    off (`app.rs:4976`, `service_on` reads `services_off`), which a sign-out is not.
  - A browser sign-in in flight writes its token unchecked (`sign_in`, `backend.rs:1206`), then
    `Request::Reconnect` clears the sign-out's mark (`backend.rs:1710`); SoundCloud likewise
    (`1012-1025`, `1886`).
  - The signed-out account's albums come back in the window and on disk; after a sign-in to another
    account, account A's library may show for account B. Privacy; confidence high on the mechanism.
  Check: a test of `run` with a fake provider that waits: no reply for it after `SignOut`, an empty
  cache folder, no token written.
- **SEC-3. The phone's library loads are not called off when a service is forgotten**
  (`Library.kt:110-138`, `360-410`): `songs-APPLE_MUSIC.json` comes back. New; privacy.
- **SEC-4. Forgetting a device leaves the devices it introduced paired.** New.
  `Message::Introduce` (`connect/src/lib.rs:1270`) takes a device and a key the introducer chose,
  without asking the user; `Store::forget` (`store.rs:229`) removes one id, though each entry keeps
  `by`. Forgetting is local only (`lib.rs:710-718`): a lost phone keeps syncing with the other paired
  devices. An introduced attacker stays paired, may send commands, read the shared state and borrow
  decrypted songs (`lend`), away from home too. Security; confidence medium to high.
- **SEC-5. Apple's licences in bursts while playing.** New. Each `Fetch` is a task never called off
  (`backend.rs:2451`); a song the pointer rests on for 0.3 s is warmed (`app.rs:180`), and the warm
  asks for a licence (`backend.rs:2487-2501`). Ten skips may send about ten licence requests in a
  second or two; the CDM's queue serves them first come, so the wanted one waits. Downloads space
  theirs a second apart (`downloads/src/queue.rs:349-367`); every licence of both apps goes through
  `AppleMusic::license` (`apple-music/src/playback.rs:722`). Honesty towards the services; Apple's
  reaction not known.
- **SEC-6. A cancellation in Kotlin never stops the work in Rust.** New. `off()`
  (`opium-ffi/src/lib.rs:110`) is `tokio::spawn(work).await`, the path of 55 async exports; UniFFI
  has no cancellation of its own ([its documentation](https://mozilla.github.io/uniffi-rs/latest/futures.html)).
  Every query typed runs against every service, on mobile data too. `off()` also wraps writes
  (`delete_playlist`), so an abort on drop must keep them whole. Budget.
- **SEC-7. `CONTRIBUTING.md:92` says the repository is private; it is public** (checked with
  `gh repo view` on 2026-11-17). The developer examples that write decrypted audio are public, as the
  audit of 2026-11-16 corrected (A8); only the contradiction is new.

### Data, formats, compatibility

- **DON-1. A manifest that does not read empties the downloads' list.** New in part. `Store::open`
  (`store.rs:259-277`) puts it aside under a numbered name and starts empty, which the queue saves at
  once; the sweep is held (`unnamed_songs`), but nothing reads the aside file back. Triggers: an enum
  value this build does not know inside a track, `version > 1`, damage. Gigabytes of sealed files
  stay on disk, never listed; protected songs need downloading again.
- **DON-2. A step back to an older build deletes silently.** `without_unknown_services`
  (`synced/src/lib.rs:524`) drops the history and playlist rows of a service the build does not
  know, keeps no copy, and the next change writes the pruned file. Another unknown value (`Codec`,
  `Repeat`, `DeviceKind`) puts the whole file aside; for `connect.json` that is a new id, a new iroh
  key and every pairing lost. The audit of 2026-11-16 (A3) said a whole `shared.json` is put aside
  for an unknown service: it is pruned instead.
- **DON-3. Connect drops a whole message for one item it cannot read** (`lib.rs:1158`). A part of
  up to 4 MiB with history, headers and deletions goes; the digests never agree, so it is sent and
  dropped again at each connection, and a playlist's deletion in that part never arrives.
- **DON-4. Connect's version stands in three places**: the hello, sent in clear before the
  handshake (`lib.rs:833`, `1035`), the Noise prologue (`lib.rs:1481`) and iroh's ALPN
  `opium-connect/1` (`away.rs:16`). A differing prologue fails the handshake
  ([Noise, §6](https://noiseprotocol.org/noise.html)). Binding only the version chosen into the
  prologue would not stop a downgrade: a man in the middle edits both hellos and both sides agree on
  the lower version. The hello's flags and the device name a peer stores (`lib.rs:1022`, `1027`) are
  unauthenticated today. iroh accepts several ALPNs (`Builder::alpns`, iroh 1.4.1).
- **DON-5. `Track` is four contracts at once**: the domain type, the UniFFI record, the wire format
  and the kept format (`Shared`, `History`, `Handoff`, playlists, manifest, caches). Persisted keys
  come from `ItemId`'s `Display`, built on the Rust debug spelling (`model.rs:39`); Kotlin spells
  ids its own way (`Playback.kt:1019`, `Downloads.kt:211`). No test pins any JSON. A rename changes
  three formats and the keys: duplicated history, confirmations lost, protected downloads expired.
  `ProviderKind` cannot learn to keep an unknown name: it is the UniFFI enum that the frozen Compose
  screens iterate (`ProviderKind.entries`, `ui/components/Lists.kt:140`) and match exhaustively.
- **DON-6. Unknown fields are lost through an older build.** New. `Track` keeps no field it does not
  know; a history row is `Play { at, track }` (`history.rs:26`). A field a newer build adds is lost
  when an older device stamps the row again, and the merge spreads the poorer row. `settings.rs`
  already does it right (`#[serde(flatten)]`).
- **DON-7. Identities tied to a device or a server.** New in part (B3). A `content://` id from
  another phone is taken for one of this phone's files (`Playback.kt:1030-1038`) and may play another
  song under the old title; `ItemId(Navidrome, id)` names no server; a local cover is a `file://`
  path of the computer that made it. On the desktop a moved file plays through `copy_here`
  (`app.rs:3816`) while its tags are unchanged, which B3 did not say.
- **DON-8. `index.json` is written as a cache but holds the only record of each file's date of
  adding** (`local/src/scan.rs:63-83`, `220`): a power cut or a new field resets Recently Added.
- **DON-9. What the merge tests do not prove**: they are examples in fixed orders; no randomised
  interleaving with clears and deletions; only an unknown provider is tested, not other unknown
  values or fields.
- **DON-10. The keyring's key made again silently** (`downloads/src/keyring.rs:21-34`,
  `create_item(…, replace: true)`); under Flatpak, oo7 uses its file backend (oo7 0.6.0,
  `Keyring::new`). Songs stay marked downloaded and fail offline. Expected in part
  (`sealed.rs:159`); what happens offline is written nowhere.
- **DON-11. What only grows**: the shared state's `seen` (`synced/src/lib.rs:121`, never pruned),
  the playlists' deletion marks, and `shared.json` rewritten and flushed at each play; removing N
  downloads rewrites the manifest N times under the store's lock (`queue.rs:247`, `store.rs:527`).
  Not measured.

### Silent failures

- **ERR-1. Failed writes of the kept state say nothing**: `synced/src/lib.rs:1242`,
  `connect/src/store.rs:239`, `desktop/src/backend.rs:711` (`let _ = write_private`, SoundCloud's
  refresh token, which serves once). A full disk loses history, pairings or the sign-in at exit.
- **ERR-2. Files put aside are said on stderr only and never read again**; on Android stderr goes
  nowhere by default (not checked on a device), so on the phone nothing records them.
- **ERR-3. Errors are sentences in more places than planned**: `desktop::backend::Reply` (41 lines
  with `Result<…, String>`); "this service cannot" is `Error::Protocol(text)` or an empty `Ok`
  (`provider.rs:60-185`); `opium-ffi/src/downloading.rs:172` borrows an encrypted SoundCloud song
  from the computer when the message contains "encrypted", so a translation would change the
  phone's downloads; Apple's 403 becomes `NotSignedIn`.
- **ERR-4. A dead backend or a panicking request task goes unnoticed** (`backend.rs:1067`, `1084`):
  a window that loads nothing, with no message. Likelihood low.

### The application's layer

- **ARC-1. There is no application layer.** The desktop's lives in the window: `App` has 144 fields,
  65 public, owns the queue, the player, MPRIS and Connect, and its retries run on egui's frame loop
  (`retry_due`, `app.rs:507-528`); `pump()` is about 850 lines, `backend::run` 1,061, with 54
  `Request` and 52 `Reply` variants. The phone's lives in its Kotlin system layer (`data/`,
  `playback/`, 5,160 lines, not frozen); `opium-ffi` is a thin per-call API. The audit of 2026-11-16
  (B1) put it in the Compose screens. The copies already differ: retries (10 s doubling to a minute,
  for ever, against 1 to 30 minutes when the network comes back), a station's refill (5 against 3),
  the pause before a search (0.16 s against 300 ms), the sign-out fence (SEC-1).
- **ARC-2. What a service can do is decided by matches in the apps**: `ProviderKind::` 105 times in
  `app.rs`, 120 in `backend.rs`; `Services` has a field per service (`services/src/lib.rs:24`).
- **ARC-3. Whole-list replies apply in the order they arrive** (`Reply::Playlists`,
  `app.rs:5022`): an older list can remove a playlist just made. Playback alone uses tokens, well.
- **ARC-4. The window's thread waits on the shared state at start** (`Backend::send` runs
  `Own::answer`, `backend.rs:1084`; `synced/src/lib.rs:943`), beyond what the debt holds.
- **ARC-5. Downloads waiting are lost at exit** (`queue.rs:62-69`, memory only), with no word.

### Contributors

- **CTB-1. CI builds neither the Nix package nor the Kotlin nor the APK.** The package lists its own
  native libraries (`flake.nix:44-58`): `nix build`, the README's first command, can fail on a green
  `main`. `gradle assembleRelease` reuses whatever core `build-core.sh` last built; the three JVM test
  files run nowhere. The change the phone's sign-out fix makes crosses that boundary.
- **CTB-2. The supported machines are not stated**: "Development happens on NixOS" while CI runs
  Ubuntu with Nix; no `rust-version`, though the code needs a recent rustc (`File::try_lock`, let
  chains); two Rust toolchains, one per shell; aarch64 declared, never checked.
- **CTB-3. Rules that bind everyone sit with the owner's and the agents'** ("no sound on the owner's
  speakers" beside "no DRM worked around"); nothing says what an outside pull request runs, nor what
  stands in for the steps only the owner can take. Known (D3, D6), left for Stage 2.
- **CTB-4. `AGENTS.md` has 1,496 words of 1,500**, checked on every pull request.
- **CTB-5. Only the local files can be checked on fake data**: no recipe for a local Navidrome, no
  downloads without a keyring.
- **CTB-6. Collision points**: `Request`/`Reply`, `run`, `App`, `pump`, one Unreleased section, 40 of
  50 ADRs in chains (their consolidation planned).
- **CTB-7. Drift**: nine dependencies of `app-desktop/Cargo.toml` its sources never name (`wreq`,
  `reqwest`, `mpris-server`, `soundcloud`…); an exported `Apple` object no Kotlin uses
  (`opium-ffi/src/lib.rs:1841`); three enums for a search's scope; ARCHITECTURE puts D-Bus in
  `desktop::instance` (it is a `File::try_lock`), bounds Connect's messages at 4 MiB (the receiver
  takes 32 MiB, `wire.rs:18`), says the downloads' queue fetches one song at a time (the next is
  fetched while one is kept, `queue.rs:369-393`), and gives the services as the owners of the state
  that the window owns (ARC-1).

### Platforms

- **PLT-1. The Linux-only code is wider than the audit of 2026-11-16 listed.** It would not compile
  on Windows: `downloads` (`private_dir`, `lock_folder`), shared with the phone; `desktop` (file
  modes, `setrlimit`); `browser-signin`'s pipes on fds 3 and 4, a redesign rather than a `cfg`. It
  compiles and degrades silently: `Mode::Private` sets 0600 under `cfg(unix)` only
  (`files.rs:121`). `player`'s public API names no PipeWire type, so Windows' output stays inside it.
- **PLT-2. The phone's open files close when the JVM collects** (`KeptAudio`, never closed by
  `KeptDataSource.close()`). Hypothesis.

## What to think of early

- **Identity**: Navidrome ids qualified by their server, a `content://` id refused when another
  device stamped it, room for a content key later; a migration from a known version once the files
  carry one.
- **Storage at scale**: Stage 7 keeps every play; a whole JSON file rewritten at each play will not
  hold. The versions' convention must let a new part (a log of plays) be added that older builds
  pass over; the store itself is chosen at Stage 7.
- **Text for translation**: no sentence for the user leaves the core, only kinds and data.
- **Backup and export**: the phone keeps nothing in Android's backups (`allowBackup="false"`); a
  user with only the phone loses Opium's playlists on a new phone until Stage 7's export.
- **Actions for agents** are the application layer: declared on the interface's methods, they
  would inherit ARC-1.
- **The APK's signing key**, before the first APK given to anyone: another key means a reinstall,
  and the downloads lost.
- **The sound's treatments** of Stage 6: the desktop's engine decodes one source at a time, the
  phone plays through ExoPlayer; equal treatments on both mean writing them twice or the phone's
  sound through Rust, which touches MediaDrm. Decided at Stage 6's design.

## The method's rules of 2026-11-16

- **Opium already has**: ARCHITECTURE's invariants, limits and evolution; the ROADMAP's scenarios;
  the ADR template's validation section.
- **The PLAN has**: one ADR per topic, the generated index, `changelog.d/`, ARCHITECTURE's first
  page, `AGENTS.md` aligned, the measures recording the machine's load.
- **Beyond them**: ARCHITECTURE's ownership is wrong and its limits incomplete (CTB-7, DON-11,
  ARC-5); the scenarios' table has no evidence column; no scenario for a sign-out during work, a
  cancellation, a full disk or a lost keyring, a device away for long, a downgrade of the manifest.
- **Not for contributors**: the owner's tools (Niri, `agent-headless`, `owner-desktop`, measuring
  beside a game), the delegation, the word limits, which are agents' budgets.

## The challenge

The challenger found no reason to reopen the design of the shared layer during the wait, and
narrowed it to the half that needs no toolkit. It changed the rest:

- The desktop's fence has holes (SEC-2): copying it to the phone would copy them. "Final" means
  nothing started before a sign-out takes effect after it: a connect, a sign-in, a token, credential
  or cache write, a reply, a subscription confirmation.
- Kotlin's CI comes before the phone's fix, which crosses that boundary.
- `ProviderKind` cannot keep an unknown name (DON-5): what crosses versions needs types of its own;
  unknown fields are lost too (DON-6).
- Connect needs both hellos' hash in the prologue and a minimum version (DON-4).
- `Reply` is not changed: the frozen window would change for nothing seen; kinds go to the log.
- The identity scheme waits for files with versions; the `content://` refusal needs no new format.
- Parity tests at the boundaries the port replaces can be written during the wait.
- One licence pacer, in `AppleMusic::license`, for every caller.

## Decisions for the owner

Put one at a time on 2026-11-17, each decided the same day:

1. **The shared layer**: designed during the wait, the half that needs no toolkit; only the shared
   sign-out fence built now; the rest at the port
   ([ADR-0051](../adr/0051-what-opium-builds-while-it-waits-for-uwui.md)).
2. **The order**: Kotlin's CI, then a milestone of final sign-outs, before the documentation
   ([ADR-0051](../adr/0051-what-opium-builds-while-it-waits-for-uwui.md)).
3. **Formats and versions in two milestones**: the files, then Connect
   ([ADR-0051](../adr/0051-what-opium-builds-while-it-waits-for-uwui.md)).
4. **Apple's licences**: the warm stops before the licence, a superseded fetch is called off, one
   pacer in `AppleMusic::license` ([ADR-0052](../adr/0052-apples-licences-paced-in-one-place.md)).
5. **Parity tests**: a milestone of their own during the wait
   ([ADR-0051](../adr/0051-what-opium-builds-while-it-waits-for-uwui.md)).
6. **Contributors**: in the documentation milestone, "private" corrected and a short section to
   start with; the full split at Stage 2.
7. **Export**: a minimal export and import of Opium's playlists and history at 0.1.0 on the
   desktop, at Stage 3 on the phone ([ADR-0054](../adr/0054-a-minimal-export-at-the-first-release.md)).
8. **A forgotten device** forgotten on every paired device, built with Connect's versions
   ([ADR-0053](../adr/0053-a-forgotten-device-forgotten-everywhere.md)).

Proposed, not decided: the scenarios above for the quality bar, and their evidence column.
