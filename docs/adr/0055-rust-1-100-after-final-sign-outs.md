# 0055. Rust 1.100 after Final sign-outs

- Status: Accepted
- Date: 2026-11-18

## Context

The [flake](../../flake.nix) takes the desktop shell and the Nix package's Rust from nixpkgs.
Android takes its stable toolchain from rust-overlay. The [locked revisions](../../flake.lock)
select Rust 1.99.0 and 1.99.1 respectively. They can move independently.

[Rust 1.100](https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/) adds a Cargo warning for unused
dependencies and an optional minimum publication age for dependency resolution. It also changes
the layout of intermediate build artifacts. The decision draws on source inspection. Audio
performance and build compatibility with Rust 1.100 have yet to be measured or tested.

## Decision

**Adopt Rust 1.100 after [Final sign-outs](../PLAN.md#final-sign-outs)** in a dedicated toolchain
update, once both toolchain sources can provide it. Move the desktop shell, the Nix package and
the Android shell together. Consider a minimum publication age, allocator changes and
profile-guided optimization separately.

## Alternatives considered

- **Upgrade now.** Cargo's new tooling would be available sooner, while Final sign-outs would
  have a moving build baseline across both platforms.
- **Upgrade with uwui.** This would defer the Cargo improvements and make it harder to tell
  whether a regression comes from the port or the compiler update.

## Validation and reconsideration

Run the desktop and Android checks and build the Nix package. The Android checks cover the
Rust core, Kotlin bindings, JVM tests and release APK. Check the
[audit harness](../audits/2026-11-10-stage-1/correctness/nan_volume_harness/build.sh), which reads
an intermediate library from Cargo's old directory layout. Keep the current pins if the
upgrade cannot pass the checks on both platforms.
