#!/usr/bin/env bash
# Runs every `cargo fuzz` target of the project for a while each, as CI does every week
# (ADR-0030). Run it inside the nightly shell: `nix develop .#fuzz -c scripts/fuzz.sh [seconds]`,
# 60 seconds a target by default.
#
# A fuzz crate is any `fuzz` folder beside a crate's Cargo.toml (crates/cenc/fuzz). A crash leaves
# its input in that crate's artifacts/ folder and fails the script.
set -euo pipefail
cd "$(dirname "$0")/.."

seconds=${1:-60}
for crate in crates/*/fuzz; do
    [ -f "$crate/Cargo.toml" ] || continue
    for target in $(cd "$crate" && cargo fuzz list); do
        printf '\n==> %s: %s, %s s\n' "$crate" "$target" "$seconds"
        (cd "$crate" && cargo fuzz run "$target" -- -max_total_time="$seconds")
    done
done
