#!/usr/bin/env bash
# The token comparison of M1.11 (docs/specs/README.md, "The check afterwards"): every source file
# of crates/widevine but Chromium's headers, against Kopuz at the revision Opium cited and at its
# latest, against the adapted files as they were last, and against the rest of the tree. Runs made
# only of tokens of Chromium's headers, which both sides must use, are left out. Writes
# comparison.md beside this script (or $REPORT), with M1.10's token_compare.py.
#
# Run from anywhere inside nix develop: `docs/audits/2026-11-05-widevine-clean-room/compare.sh`.
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
# The adapted files as they were last, before the rewrite.
adapted=fce9347
mkdir -p "$work/adapted"
for file in build.rs src/lib.rs src/native.rs src/stub.rs shim/widevine_shim.cc examples/probe.rs; do
    git show "$adapted:crates/widevine/$file" >"$work/adapted/${file//\//_}"
done

mapfile -t targets < <(find crates/widevine -type f \( -name '*.rs' -o -name '*.cc' -o -name '*.cpp' -o -name '*.h' \) \
    -not -path '*/vendor/*' -not -path '*/target/*' | sort)

origins=()
for revision in a58c385 1d8b323; do
    root=$cited
    [ "$revision" = 1d8b323 ] && root=$latest
    for file in shim/widevine_shim.cc src/applemusic/widevine/mod.rs src/applemusic/widevine/discover.rs \
        src/applemusic/widevine/fetch.rs src/applemusic/widevine/mediadrm.rs; do
        origins+=("kopuz-$revision $(basename "$file")=$root/daemon/server/$file")
    done
done
for file in "$work"/adapted/*; do
    origins+=("$adapted $(basename "$file")=$file")
done

python3 -I "$here/../2026-11-05-cenc-clean-room/token_compare.py" "${REPORT:-$here/comparison.md}" \
    --target "${targets[@]}" \
    --source "kopuz-a58c385=$cited" "kopuz-1d8b323=$latest" "adapted@$adapted=$work/adapted" "tree=." \
    --exclude crates/widevine \
    --forced crates/widevine/vendor/content_decryption_module.h crates/widevine/vendor/content_decryption_module_export.h \
    --origin "${origins[@]}"
