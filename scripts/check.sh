#!/usr/bin/env bash
# Runs every automated check of the definition of done (AGENTS.md). CI runs this script unchanged,
# inside the project's development shell (flake.nix), which provides every tool used here:
# `nix develop --no-update-lock-file -c scripts/check.sh` (the flag makes an incomplete flake.lock an
# error instead of a silent update).
#
# `scripts/check.sh android`, run inside `nix develop .#android`, checks the phone's core instead:
# the Rust that the Android app loads, built for the phone. Building the APK itself is not checked.
#
# No test needs the network, PipeWire, a display, a session bus, a sign-in or a Widevine CDM. What
# needs them is checked by hand (CONTRIBUTING.md, "Running Opium without touching the owner's data").
#
# Not checked here yet (docs/PLAN.md): the dependencies' licences and advisories, as cargo-deny
# finds advisories that only the owner can rule on; and the benchmarks, measured by hand for now
# (docs/measurements.md).
set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n==> %s\n' "$*"; }

part=${1:-desktop}
case "$part" in
    desktop | android) ;;
    *)
        echo "usage: scripts/check.sh [desktop|android]" >&2
        exit 2
        ;;
esac

if [ "$part" = android ]; then
    : "${ANDROID_NDK_HOME:?run this part inside nix develop .#android}"
    step "the phone's core: clippy for arm64 Android, warnings as errors"
    # The same target folder and API level as android/build-core.sh.
    CARGO_TARGET_DIR=target-android \
        cargo ndk -t arm64-v8a --platform 34 clippy -p opium-ffi --locked -- -D warnings
    step "all checks passed"
    exit 0
fi

step "format"
cargo fmt --all --check

step "lints"
cargo clippy --workspace --all-targets --locked -- -D warnings

step "build and tests"
cargo test --workspace --locked

step "documentation"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

# Links between the Markdown files, offline; the development shell provides lychee.
step "links"
git ls-files -z '*.md' | xargs -0 lychee --offline --include-fragments --no-progress

step "all checks passed"
