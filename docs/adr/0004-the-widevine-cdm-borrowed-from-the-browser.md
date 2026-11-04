# 0004. The Widevine CDM borrowed from the user's browser, through a C++ shim

- Status: Accepted
- Date: 2026-10-29

## Context

Apple Music's songs and SoundCloud's encrypted tracks are Widevine-protected CENC
([ADR-0002](0002-apple-music-through-its-web-player.md)). Decrypting them needs a Widevine Content
Decryption Module (CDM): Google's proprietary library, which cannot be shipped with a GPL program
nor rewritten. Every Chromium-family browser and Firefox downloads one into the user's profile, and
loads it through the CDM interface published in Chromium's sources
(`content_decryption_module.h`). Kopuz drives it that way.

## Decision

**On the desktop, Opium loads the CDM the user's browser installed** (`crates/widevine`):
`widevine::locate` looks under the config folder for Chrome's, Chromium's, Brave's and Vivaldi's
`WidevineCdm/*/_platform_specific/linux_x64/libwidevinecdm.so`, then in Firefox's profiles, newest
version first; `WIDEVINE_CDM` names another one.

**A small C++ host drives it through `cdm::ContentDecryptionModule_11`**
(`shim/widevine_shim.cc`, adapted from Kopuz, built by `build.rs` with `cc` against Chromium's
header in `vendor/`). The Rust side (`native.rs`) calls the shim's `wv_open`, `wv_challenge`,
`wv_update`, `wv_decrypt` and `wv_close`.

**Sessions are temporary and nothing persists**: no distinctive identifier, no persistent state,
no file storage given to the CDM (`CreateFileIO` returns none), `SessionType::kTemporary`. Licences
are asked for each time a song is opened.

**On Android the crate is a stub** (`stub.rs`): the phone's own MediaDrm decrypts, inside
ExoPlayer ([ADR-0009](0009-the-android-app-over-the-rust-core.md)); `build.rs` compiles no shim
there.

## Alternatives considered

- **Shipping a CDM**: not allowed by its licence, and incompatible with the GPL.
- **A software CDM of one's own, or keys extracted from a device**: refused; Opium decrypts only
  through a CDM that Google licenses to the user's browser.
- **Capturing the decoded audio of the web player (PCM)**: refused for the same reason, and it
  would lose the stream's own quality.

## Consequences

- A browser that installed the CDM, and played a DRM video once so that the CDM has its device
  certificate, is a requirement for Apple Music and SoundCloud's protected tracks (README). An
  unprovisioned CDM gives `Error::Unprovisioned`, which says so.
- The CDM is Widevine L3, software: what it decrypts is limited by what the licence allows it, and
  it decrypts at its own pace (about 11.6 times real time, measured in commit `33a995e`). Opium does
  not work around either.
- `widevine::locate` reads `XDG_CONFIG_HOME`: a test run with a config folder of its own finds no
  CDM and needs `WIDEVINE_CDM` (AGENTS.md).
- A Windows port needs the CDM's host verification, which this shim does not do yet (ROADMAP).
- Every `unsafe` call into the shim lacks the `// SAFETY:` comment the code rules ask for
  (PLAN, known debt).
