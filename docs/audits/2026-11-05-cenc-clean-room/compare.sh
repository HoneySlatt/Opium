#!/usr/bin/env bash
# The token comparison of M1.10 (docs/specs/README.md, "The check afterwards"): every source file
# of crates/cenc, against Kopuz at the revision Opium cited and at its latest, against the old
# fmp4.rs, and against the rest of the tree. Writes comparison.md beside this script.
#
# Run from anywhere inside nix develop: `docs/audits/2026-11-05-cenc-clean-room/compare.sh`.
# Kopuz's sources come from GitHub through `nix flake prefetch`, into the Nix store.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here/../../.."

store_path() {
    nix flake prefetch --json "github:Kopuz-org/kopuz/$1" |
        python3 -I -c 'import json, sys; print(json.load(sys.stdin)["storePath"])'
}
cited=$(store_path a58c385)
latest=$(store_path 1d8b323)

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# The adapted file as it was last, before the rewrite.
mkdir -p "$work/fmp4"
git show 4522e68:crates/cenc/src/fmp4.rs >"$work/fmp4/fmp4.rs"

mapfile -t targets < <(find crates/cenc -name '*.rs' -not -path '*/target/*' | sort)

python3 -I "$here/token_compare.py" "$here/comparison.md" \
    --target "${targets[@]}" \
    --source "kopuz-a58c385=$cited" "kopuz-1d8b323=$latest" "fmp4.rs@4522e68=$work/fmp4" "tree=." \
    --exclude crates/cenc \
    --origin "kopuz-a58c385 cenc.rs=$cited/daemon/server/src/applemusic/cenc.rs" \
    "kopuz-1d8b323 cenc.rs=$latest/daemon/server/src/applemusic/cenc.rs" \
    "fmp4.rs@4522e68=$work/fmp4/fmp4.rs"
