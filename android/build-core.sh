#!/usr/bin/env bash
# Builds the Rust core for the phone and its Kotlin bindings, into the app's
# generated sources. Run from `nix develop .#android`.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$here/.."
out="$here/app/build/generated"
export CARGO_TARGET_DIR="$root/target-android"
cd "$root"
cargo ndk -t arm64-v8a --platform 34 -o "$out/jniLibs" build --release -p opium-ffi
lib="$out/jniLibs/arm64-v8a/libopium_ffi.so"
cargo run --quiet -p opium-ffi --bin uniffi-bindgen -- generate \
    --library "$lib" --language kotlin --no-format --out-dir "$out/uniffi"
