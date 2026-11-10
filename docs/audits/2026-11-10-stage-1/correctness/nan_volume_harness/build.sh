#!/usr/bin/env bash
# Cuts fill() and its helpers out of the player's output.rs as they are now, and builds and runs
# the harness in a temporary folder, so that nothing is written in the repository. Run inside the
# development shell, after `cargo build -p player` (it links the rtrb that build made).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(git -C "$here" rev-parse --show-toplevel)
out=$(mktemp -d)
cp "$here/main.rs" "$out/"
sed -n 63,194p "$root/crates/player/src/output.rs" >"$out/output_fill.rs"
deps="${CARGO_TARGET_DIR:-$root/target}/debug/deps"
rtrb=$(ls "$deps"/librtrb-*.rlib | head -1)
rustc --edition 2024 -O "$out/main.rs" -L "dependency=$deps" --extern "rtrb=$rtrb" -o "$out/harness"
"$out/harness"
