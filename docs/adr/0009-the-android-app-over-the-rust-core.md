# 0009. The Android app: Kotlin and Compose over the Rust core, through UniFFI

- Status: Accepted; amended by [0034](0034-what-opium-is-to-become.md)
  (the lowest Android decided at Stage 3; bit-perfect USB as an option)
- Date: 2026-10-30

## Context

Opium is for the phone as well as the computer, with the same functions. The services' clients
(Apple Music, SoundCloud, Navidrome), the copy matching, the themes, the shared state and the
downloads' sealing were already Rust crates. Writing them again in Kotlin would double every fix.
On Android, though, the system's own media stack is what plays efficiently and decrypts Widevine in
hardware, and Compose is what draws a native, smooth interface.

## Decision

**The Android app is written in Kotlin with Jetpack Compose** (`android/`), and **calls the Rust
core through UniFFI** (commits `d5901ce`, `02b1f7f`).

- **`crates/opium-ffi` is the one library the app loads** (`cdylib`): it exports the services, the
  library, search, downloads, Connect and the shared state as UniFFI objects and suspending
  functions. `opium-core` and `connect` derive their types for it behind their `ffi` feature.
- **The bindings are generated from the built library**: `android/build-core.sh` builds `opium-ffi`
  for `arm64-v8a` with `cargo-ndk` (Android API 34), then runs the crate's own `uniffi-bindgen`
  binary to write the Kotlin into `android/app/build/generated/`, which Gradle compiles.
- **What the two apps share lives in Rust**: logic that first lived in the desktop app, such as the
  copy matching and the cover palette, moved into `opium-core` before the phone used it
  (commit `67bc976`).
- **The phone plays through ExoPlayer (Media3)**, not through `crates/player`: Apple Music's songs
  as HLS with the phone's own Widevine through MediaDrm, the licence asked through the core
  (`playback/AppleSource.kt`); unencrypted songs through the audio chip's offload where it can.
- **Android 14 (API 34) is the minimum**: the first with bit-perfect USB audio. Only `arm64-v8a` is
  built.
- **No Google Play Services library**: the app runs on Android builds without them.

## Alternatives considered

None were written down at the time; these are the obvious ones, and what the code shows against
them.

- **A second implementation in Kotlin**: every service fixed twice, and drift between the apps.
- **The Rust player on the phone too**: no hardware Widevine and no offload, which ExoPlayer and
  MediaDrm give.

## Consequences

- A change to a type crossing the boundary changes the Kotlin that uses it; `build-core.sh` must be
  run before Gradle.
- The core must build for Android: `widevine` compiles a stub there, and nothing desktop-only
  (`player`, `desktop`, `motion`, `app-desktop`) is a dependency of `opium-ffi`.
- TLS on Android checks certificates against the system's store through
  `rustls-platform-verifier`, whose Kotlin part's version must match `Cargo.lock`
  (`android/app/build.gradle.kts`).
- The APK is signed with Android's debug key until the owner creates a release key (ROADMAP).
