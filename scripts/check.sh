#!/usr/bin/env bash
# Runs every automated check of the definition of done (AGENTS.md). CI runs this script unchanged,
# inside the project's development shell (flake.nix), which provides every tool used here:
# `nix develop --no-update-lock-file -c scripts/check.sh` (the flag makes an incomplete flake.lock an
# error instead of a silent update).
#
# `scripts/check.sh android`, run inside `nix develop .#android`, checks the phone's core instead:
# the Rust that the Android app loads, built for the phone. Building the APK itself is not checked.
#
# `scripts/check.sh docs` checks the documents only: the sizes of the files every agent reads at
# every session, and the links between the Markdown files. The default part runs it first.
#
# No test needs the network, PipeWire, a display, the user's session bus, a sign-in or a Widevine
# CDM: the tests of the one instance start a bus of their own (dbus-run-session, from the shell).
# What needs them is checked by hand (CONTRIBUTING.md, "Running Opium without touching the owner's
# data").
#
# Not checked here yet (docs/debt.md; cargo-deny at Stage 2, docs/ROADMAP.md): the dependencies'
# licences and advisories, as cargo-deny finds advisories that only the owner can rule on; and the
# benchmarks, measured by hand for now (docs/measurements.md).
set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n==> %s\n' "$*"; }

part=${1:-desktop}
case "$part" in
    desktop | android | docs) ;;
    *)
        echo "usage: scripts/check.sh [desktop|android|docs]" >&2
        exit 2
        ;;
esac

# Fails when a file that every agent reads at every session outgrows its limit in words
# (AGENTS.md, "Documentation").
check_size() {
    local file=$1 limit=$2 words
    words=$(wc -w <"$file")
    if [ "$words" -gt "$limit" ]; then
        echo "$file has $words words; keep it at most $limit (AGENTS.md, \"Documentation\")." >&2
        exit 1
    fi
    echo "$file: $words words of $limit"
}

check_documents() {
    step "document sizes"
    check_size AGENTS.md 1500
    check_size docs/PLAN.md 2000

    # Offline, anchors included; the development shell provides lychee.
    step "links"
    git ls-files -z '*.md' | xargs -0 lychee --offline --include-fragments --no-progress
}

if [ "$part" = docs ]; then
    check_documents
    step "all checks passed"
    exit 0
fi

if [ "$part" = android ]; then
    : "${ANDROID_NDK_HOME:?run this part inside nix develop .#android}"
    step "the phone's core: clippy for arm64 Android, warnings as errors"
    # The same target folder and API level as android/build-core.sh.
    CARGO_TARGET_DIR=target-android \
        cargo ndk -t arm64-v8a --platform 34 clippy -p opium-ffi --locked -- -D warnings
    step "all checks passed"
    exit 0
fi

check_documents

step "format"
cargo fmt --all --check

step "lints"
cargo clippy --workspace --all-targets --locked -- -D warnings
# apple-music's entry points for its fuzz targets, built only with this feature (ADR-0030).
cargo clippy -p apple-music --all-targets --features fuzzing --locked -- -D warnings

step "build and tests"
cargo test --workspace --locked

# frames.py judges which runs' figures stand (a scroll that stopped short is left out).
step "the measuring scripts' tests"
python3 -I scripts/measure/test_frames.py

step "documentation"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

step "all checks passed"
