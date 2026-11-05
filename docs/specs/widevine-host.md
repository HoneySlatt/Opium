# The Widevine CDM host: what Opium does with the browser's CDM

The behaviour that `crates/widevine` must have, on the desktop. It is written from Chromium's
published CDM interface (`crates/widevine/vendor/content_decryption_module.h`, BSD-3-Clause, the
only source to read besides this file), from the W3C Encrypted Media Extensions
(<https://www.w3.org/TR/encrypted-media/>), and from what Opium observed of the CDM. It is the only
input of the clean-room rewrite ([README](README.md)), and says what the code must do, not how to
structure it. What may and may not be done with the CDM is decided in
[ADR-0004](../adr/0004-the-widevine-cdm-borrowed-from-the-browser.md): this file never goes beyond
it.

## 1. Finding a CDM

The CDM is Google's `libwidevinecdm.so`, which a browser downloaded into the user's profile. Opium
never downloads, ships or modifies one.

- `WIDEVINE_CDM`, when set, names the file to use, and nothing else is looked for.
- Otherwise, under the config folder (`XDG_CONFIG_HOME`, else `~/.config`), each Chromium-family
  browser keeps it at `<user data>/WidevineCdm/<version>/_platform_specific/linux_x64/` (on arm64,
  `linux_arm64`). The user-data folders, in alphabetical order: `BraveSoftware/Brave-Browser*`,
  `chromium`, `google-chrome`, `google-chrome-beta`, `google-chrome-unstable`, `microsoft-edge*`,
  `opera*`, `vivaldi`, `vivaldi-snapshot`.
- Google Chrome also installs one with the browser, at
  `/opt/google/chrome/WidevineCdm/_platform_specific/linux_x64/`; its version is in the
  `manifest.json` of `/opt/google/chrome/WidevineCdm/`.
- Flatpak browsers keep the same tree under `~/.var/app/<application id>/config/`
  (`com.brave.Browser`, `com.google.Chrome`, `com.microsoft.Edge`, `com.vivaldi.Vivaldi`,
  `org.chromium.Chromium`), and Firefox's under `~/.var/app/org.mozilla.firefox/.mozilla/firefox/`.
- Firefox keeps it at `~/.mozilla/firefox/<profile>/gmp-widevinecdm/<version>/`.
- Among the candidates, the highest **version** wins, compared as numbers part by part
  (`4.9.12.0` < `4.10.3.0`), never as text. A Chromium component's version is the `version` of the
  `manifest.json` beside `_platform_specific` when there is one, else its folder's name; Firefox's,
  its folder's name. A candidate whose version does not read as numbers is set aside. Between equal
  versions, the first path in byte order wins, so that the choice is the same on every run.
- Every candidate considered, and why one was set aside, can be listed for a diagnostic.
- A candidate must be a regular file once symbolic links are resolved, wherever they lead; a
  broken link or a folder is set aside.

## 2. Loading it

- The library is opened with immediate binding and kept local to its handle (`RTLD_NOW |
  RTLD_LOCAL`), so that its symbols never replace the program's.
- It must export `InitializeCdmModule_4`, `CreateCdmInstance` and `GetCdmVersion`; otherwise it is
  "not a CDM". `InitializeCdmModule_4` is called once per process, after the library is opened and
  before the first `CreateCdmInstance`. `DeinitializeCdmModule` is called at shutdown when exported.
- `RTLD_LOCAL` and immediate binding differ from today's loading (`RTLD_GLOBAL`): the change is
  checked with the owner's CDM (§11) before it is kept.
- `GetCdmVersion` is kept for diagnostics.
- One instance per process, of interface version 11 (`ContentDecryptionModule_11`), key system
  `com.widevine.alpha`, asked for through `CreateCdmInstance` with a host lookup that answers only
  version 11 (`Host_11`). An instance of another version is refused: "interface not supported".
- The C++ that implements the header's interfaces is specific to one interface version, and does
  nothing but pass calls and callbacks between the CDM and Rust. Everything else (timers, promises,
  waits, key statuses, errors, threads) is in Rust and knows no interface version, so that version
  12, already in the header, can be added beside 11 without touching it.
- `Initialize(allow_distinctive_identifier = false, allow_persistent_state = false,
  use_hw_secure_codecs = false)`; the CDM answers through `OnInitialized(success)`. A failed
  initialisation leaves no instance: the instance is destroyed, the next attempt creates a new one
  from the library already open, and the error is reported each time, never a success.
- Loading is idempotent: loading again returns the same instance; loading another path while one
  is loaded is refused.

## 3. The host's duties (`Host_11`)

The CDM calls the host back on the CDM thread (§7), during a call or, for answers that come
later, from the timers the thread fires. The host:

- **`Allocate(capacity)`**: a buffer of at least that capacity, freed when the CDM calls its
  `Destroy`.
- **`SetTimer(delay_ms, context)`**: `TimerExpired(context)` is called on the CDM thread, as soon
  as possible once `delay_ms` has passed, and never before. The thread wakes for it, whether or not
  a request is waiting.
- **`GetCurrentWallTime()`**: seconds since the Unix epoch, as a double.
- **Promises**: each call taking a `promise_id` gets a fresh id; the answer
  (`OnResolveNewSessionPromise`, `OnResolvePromise`, `OnResolveKeyStatusPromise`,
  `OnRejectPromise`) is matched to its call by that id. A rejection keeps its `Exception`, its
  `system_code` and its message, which the error carries.
- **`OnSessionMessage(session, type, message)`**: kept with its type. A `kLicenseRequest` is the
  challenge for the licence server. A `kIndividualizationRequest` means the CDM has no device
  certificate: it is **never** sent anywhere, and the session fails with "not provisioned". What to
  tell the user then is not known yet, since the CDM is given no storage of the browser's: it is
  found with the owner, and written here. `kLicenseRenewal` and
  `kLicenseRelease` are reported, not acted on.
- **`OnSessionKeysChange(session, has_additional_usable_key, keys)`**: each key id's status is
  kept: `kUsable`, `kInternalError`, `kExpired`, `kOutputRestricted`, `kOutputDownscaled`,
  `kStatusPending`, `kReleased`. Statuses may come before or after the licence's promise resolves,
  and may move from `kStatusPending` to a final one: a licence is accepted once the promise has
  resolved and the key id the caller needs is `kUsable`, within the licence's deadline (§4); a
  final status other than `kUsable` for it is the error, named.
- A key that becomes `kExpired` later, during a long track, makes its decrypts fail with "key
  expired", so that the caller can open a new session and ask for a new licence. Renewals
  (`kLicenseRenewal`) are reported, not sent: sending them to a service needs the owner's check
  against it.
- **`OnExpirationChange`**: kept, for diagnostics.
- **`OnSessionClosed(session)`**: marks the session closed.
- **`CreateFileIO`**: none (returns null): no persistent state is given to the CDM.
- **`RequestStorageId(version)`**: answered with no storage id (`OnStorageId(version, null, 0)`).
- **`QueryOutputProtectionStatus`, `EnableOutputProtection`, `SendPlatformChallenge`**: not
  answered, as today. Answering them with anything is a change of what Opium tells the CDM about
  the machine, and needs the owner's decision and a check against the real services.
- **`OnDeferredInitializationDone`, `ReportMetrics`**: ignored (no decoder is used).

## 4. Waiting for the CDM

Every call that answers through a callback waits for that answer, firing due timers meanwhile and
sleeping until the next one is due (never spinning), up to a deadline in time:

| Step | Deadline | Error when it passes |
|---|---|---|
| Initialisation | 10 s | "the CDM did not initialise" |
| A session's request (`CreateSessionAndGenerateRequest`) | 10 s | "the CDM made no licence request" |
| A licence (`UpdateSession`) | 10 s | "the CDM did not take the licence" |
| Closing (`CloseSession`) | 5 s | logged, not an error for the caller |

A deadline passed is an error naming the step; it never hangs a thread for ever.

## 5. Sessions

- A session is **temporary** (`kTemporary`), opened with **CENC** init data (`kCenc`): a `pssh`
  box version 0 or 1 whose system id is Widevine's, `edef8ba9-79d6-4ace-a3c8-27dcd51d21ed`.
- Opening one yields the session's id and its licence request (the challenge bytes).
- The caller sends the challenge to the service's licence server
  ([apple-music-playback.md](apple-music-playback.md), SoundCloud's own), and gives the answer back
  to the session (`UpdateSession`). The content keys stay inside the CDM; Opium never sees them.
- Dropping a session closes it (`CloseSession`, then `OnSessionClosed`), and its keys go with it;
  decrypts already queued for it are answered first. How the close went is logged.
- Several sessions may be open at once (a track playing, the next one preparing, a download).

### Building the init data for a key id

When a file names its key id but carries no Widevine `pssh` (Apple's catalog songs), the init data
is a `pssh` built for that key id:

- box header: size, `pssh`, version 0, flags 0; Widevine's system id; the data's size; the data;
- the data is a `WidevinePsshData` protocol buffer with, in field order: field 1 (`algorithm`,
  varint) = 1 (AES-CTR); field 2 (`key_id`, bytes) = the 16-byte key id; field 3 (`provider`,
  string) empty; field 6 (`policy`, string) empty. Lengths are varints.
- These are the bytes Opium sends today, which Apple's licence server accepts. Whether fields 1, 3
  and 6 are needed is not known: a message with field 2 alone is tried against Apple with the
  owner, and kept if it works; until then, these bytes stay.

## 6. Decrypting a sample

- One call decrypts one sample: its bytes, the key id, the IV, the subsample runs, the scheme and
  its pattern ([fmp4-cenc.md](fmp4-cenc.md) §5).
- The IV given to the CDM is always 16 bytes: an 8-byte IV is followed by eight zero bytes (CENC
  §9.1); a constant IV of 8 bytes likewise.
- No subsamples means the whole sample is protected. The runs must add up to the sample's size;
  otherwise the call is refused before reaching the CDM.
- The scheme is `kCenc` (AES-CTR, no pattern) or `kCbcs` (AES-CBC, with the crypt and skip block
  counts); others are refused.
- The cleartext has the sample's length; it is written over the caller's bytes, in place, with one
  copy from the CDM's buffer at most, and that buffer destroyed.
- `InputBuffer_2`'s `timestamp` is 0: it only comes back in the decrypted block, which Opium does
  not read.
- The CDM's `Status` is mapped exactly: `kNoKey` (2) "no key for this content", `kDecryptError` (4)
  "decryption failed", `kInitializationError` (3), `kNeedMoreData` (1), `kDecodeError` (5),
  `kDeferredInitialization` (6) each named; a cleartext of another length is an error.

## 7. Threads

As Chromium's own CDM host does, one thread owns the CDM:

- **One CDM thread** creates the instance, makes every call into it, receives every callback, fires
  the timers and holds the host's state. No other thread touches them.
- Callers on any thread send it requests (open a session, give a licence, decrypt, close) and wait
  for the answer. The requests wait in a queue by priority: a session's opening, licence and closing
  go before decrypts, so that a track decrypting delays the licence of the track the user just
  picked by one sample's decrypt at most (about 1.5 ms).
- A decrypt request is one sample, never a whole track.
- With no request and no timer due, the thread sleeps until the next timer or request: nothing
  runs at rest.
- A callback from the CDM into Rust never panics (a panic across the C++ boundary is undefined
  behaviour): every Rust function the C++ calls catches what could go wrong and records an error.
- The cleartext is written to a buffer that never overlaps the ciphertext the CDM reads, then copied
  over the caller's bytes.

## 8. Shutdown

Before the process exits, `shutdown()` is called:

- requests waiting or sent afterwards get "shut down";
- the CDM thread finishes the call under way, closes the sessions still open, destroys the
  instance (`Destroy`), calls `DeinitializeCdmModule` when exported, and ends. All of it is bounded
  by five seconds in total: `shutdown()` returns when the thread has ended, or after those five
  seconds;
- the library is never unloaded: no thread can be inside it when the process's exit handlers run.

## 9. Errors

Each error names its step and keeps what the CDM said:

- the library could not be opened (with the loader's message); not a CDM (which symbol is
  missing); interface 11 not supported; initialisation failed or timed out;
- not provisioned; init data refused (with the CDM's message); licence refused (with the CDM's
  message and system code); the needed key not usable (with its status);
- each decrypt status of §6; a deadline passed (§4); shut down (§8).

## 10. Android

On Android the crate exists with the same types, but the system's MediaDrm decrypts inside the
player ([ADR-0009](../adr/0009-the-android-app-over-the-rust-core.md)): loading a CDM fails with
"no CDM on this platform", and no C++ is built.

## 11. What is measured

On the owner's machine and with the owner's go (a licence needs a signed-in account), measured
again **before** the rewrite and after it, the same way:

- the time from opening a session to a usable key;
- the CDM time per sample over a whole track (about 1.5 ms, 11.6 times real time, when commit
  `33a995e` measured it: an old figure, taken again first);
- the CDM thread's wake-ups a second while paused, with a session open: honoured timers must not
  add any beyond the CDM's own.

Without an account, with the owner's go to load the browser's CDM: loading, initialising, a
licence request for a made-up key id, and the timers and storage requests the CDM makes meanwhile
(an example the rewrite brings, `examples/probe.rs`).

## 12. The crate's public interface

These names are fixed: the callers are moved to them. Everything not listed here is private.

| Item | What it gives |
|---|---|
| `widevine::find_cdm() -> Option<PathBuf>` | The CDM to use (§1) |
| `widevine::cdm_candidates() -> Vec<Candidate>` | Every candidate considered (§1) |
| `Candidate { path: PathBuf, version: Option<String>, set_aside: Option<String> }` | `set_aside`: why it was, when it was |
| `widevine::Cdm::load(path: &Path) -> Result<Cdm, widevine::Error>` | §2; `Cdm` is `Clone + Copy + Send + Sync`, a handle to the one instance |
| `Cdm::open_session(&self, init_data: &[u8]) -> Result<(Session, Vec<u8>), widevine::Error>` | The session and its licence request (§5) |
| `Session::give_licence(&self, licence: &[u8], key_id: &[u8; 16]) -> Result<(), widevine::Error>` | Ready once that key is usable (§3) |
| `Session::decrypt(&self, data: &mut [u8], sample: &Protected<'_>) -> Result<(), widevine::Error>` | One sample, in place (§6); `Session` is `Send + Sync` |
| `Protected<'a> { key_id: &'a [u8; 16], iv: &'a [u8; 16], subsamples: &'a [(u32, u32)], scheme: Scheme }` | Subsample runs as (clear, protected) bytes |
| `Scheme { Cenc, Cbcs { crypt: u8, skip: u8 } }` | |
| `widevine::pssh_for_key(key_id: &[u8; 16]) -> Vec<u8>` | §5 |
| `widevine::SYSTEM_ID: [u8; 16]` | Widevine's system id |
| `widevine::shutdown()` | §8 |
| `widevine::Error` | §9, with `Display`, every case named |

## 13. What the clean side is given

An export with no history, built by the dirty side:

- this file, [fmp4-cenc.md](fmp4-cenc.md) §5 for what a sample's inputs are, and
  [README.md](README.md);
- `crates/widevine/vendor/`: Chromium's header, its export header and `LICENSE.chromium`;
- `crates/widevine/Cargo.toml`, a workspace `Cargo.toml` of its own with the root's
  `[workspace.package]`, and the part of `Cargo.lock` its build needs (`cc`, `thiserror`);
- no `src/`, no `shim/`, no `build.rs`, no `examples/`.

The callers outside the crate are moved to §12's names by the dirty side, after the clean side's
code is in: `apple-music` (`playback.rs`, `radio.rs`, `examples/am.rs`), `soundcloud`
(`stream.rs`, `examples/sc.rs`), `desktop` (`backend.rs`, `backend/downloading.rs`), `app-desktop`
(`main.rs`) and `downloads` (`examples/spike.rs`).

## 14. Tests the rewrite brings

Without a CDM (in `cargo test`, everywhere):

- the candidates found in a folder tree built in the test: each browser's layout, Flatpak's,
  Firefox's, versions compared as numbers, a broken symbolic link, a folder in place of the file;
- the `pssh` bytes for a key id, pinned; a key id of another length refused;
- a decrypt refused before the CDM when the subsample runs do not add up to the sample;
- the host's timer queue: timers fire in order of their due time, never before it, and a wait
  passes its deadline with the named error;
- the queue's priority: a licence request sent while decrypts wait is served before them;
- promises matched by id, a rejection's message carried into the error; key statuses arriving
  before the promise, after it, and pending then usable;
- the CDM driven by a fake written in Rust behind the same C boundary, so that the state machine is
  tested in every order of callbacks, including callbacks for a session already closed;
- each CDM `Status` mapped to its error (kNoKey to "no key").

With the owner's CDM, by hand and with the owner's go: a library that is not a CDM refused with
the missing symbol named; and the behaviour of §11.
