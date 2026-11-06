#!/usr/bin/env bash
# The token comparison of M1.12 (docs/specs/README.md, "The check afterwards"), with M1.10's
# token_compare.py. Only part of crates/apple-music was written again, so the targets are the clean
# side's own lines: every line its commits added to the crate in the export, file by file, taken
# from the export's history (its repository, given as the first argument). They are compared:
#
# - against Kopuz at the revision Opium cited and at its latest, whole;
# - against the adapted items as they were last, the lines the dirty side removed for the export;
# - against the adapted files whole, as they were last;
# - against the rest of the tree, crates/apple-music left out.
#
# Writes comparison.md beside this script (or $REPORT). Run from anywhere inside nix develop:
# `docs/audits/2026-11-06-apple-music-clean-room/compare.sh <export repository>`. Kopuz's sources
# come from GitHub through `nix flake prefetch`, into the Nix store.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
export_repo="$(cd "$1" && pwd)"
cd "$here/../../.."

store_path() {
    nix flake prefetch --json "github:Kopuz-org/kopuz/$1" |
        python3 -I -c 'import json, sys; print(json.load(sys.stdin)["storePath"])'
}
cited=$(store_path a58c385)
latest=$(store_path 1d8b323)

# The export as the dirty side made it, and the code the clean side handed back.
given=f6dd554
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# The clean side's lines: what its commits added, by file (a whole new file is all added).
mkdir -p "$work/new"
for file in $(git -C "$export_repo" diff --name-only "$given" HEAD -- crates/apple-music | grep '\.rs$'); do
    git -C "$export_repo" diff --unified=0 "$given" HEAD -- "$file" |
        sed -n 's/^+//p' | grep -v '^++ ' >"$work/new/$(basename "$file")" || true
done

# The adapted files as they were last, before the rewrite, and the lines removed from them.
adapted=3d7b225
mkdir -p "$work/adapted" "$work/removed"
for file in auth.rs convert.rs lib.rs playback.rs; do
    git show "$adapted:crates/apple-music/src/$file" >"$work/adapted/$file"
done
for file in convert.rs lib.rs playback.rs; do
    git -C "$export_repo" show "$given:crates/apple-music/src/$file" >"$work/given-$file"
    diff "$work/adapted/$file" "$work/given-$file" | sed -n 's/^< //p' >"$work/removed/$file" || true
done
cp "$work/adapted/auth.rs" "$work/removed/auth.rs"

mapfile -t targets < <(find "$work/new" -type f -name '*.rs' | sort)

origins=()
for revision in a58c385 1d8b323; do
    root=$cited
    [ "$revision" = 1d8b323 ] && root=$latest
    for file in "$root"/daemon/server/src/applemusic/*.rs; do
        origins+=("kopuz-$revision $(basename "$file")=$file")
    done
done
for file in "$work"/removed/*; do
    origins+=("removed@$adapted $(basename "$file")=$file")
done

report=${REPORT:-$here/comparison.md}
python3 -I "$here/../2026-11-05-cenc-clean-room/token_compare.py" "$report" \
    --target "${targets[@]}" \
    --source "kopuz-a58c385=$cited" "kopuz-1d8b323=$latest" "removed@$adapted=$work/removed" \
    "adapted@$adapted=$work/adapted" "tree=." \
    --exclude crates/apple-music \
    --origin "${origins[@]}"
# The clean side's lines are named by their file, without the temporary folder they were put in.
sed -i "s#$work/new/#clean side's lines of #g" "$report"
