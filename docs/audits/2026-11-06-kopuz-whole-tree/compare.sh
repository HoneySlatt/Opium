#!/usr/bin/env bash
# The token comparison of M1.13 (docs/specs/README.md, "The check afterwards"): Opium's whole tree
# against Kopuz's, at the revision Opium cited and at its latest, with M1.10's token_compare.py.
#
# - The targets are every tracked text file the script can read (`--all-text`: Rust, C++, Kotlin,
#   JSON, shell, Python, Nix, TOML, YAML, Fluent, desktop entries, XML, SVG, Markdown), but
#   Chromium's headers, which both trees vendor, and the earlier comparisons' reports, which quote
#   code already compared. Lock files, licence texts and dotfiles are not read.
# - Runs made only of tokens of Chromium's headers, which both sides must use, are left out.
# - Kopuz's names, strings, comment and prose phrases and test data are searched in the targets.
# - Files identical byte for byte, binaries included, are listed at the end.
#
# Writes comparison.md beside this script (or $REPORT). Run from anywhere inside nix develop:
# `docs/audits/2026-11-06-kopuz-whole-tree/compare.sh`. Kopuz's sources come from GitHub through
# `nix flake prefetch`, into the Nix store.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
cd "$here/../../.."

store_path() {
    nix flake prefetch --json "github:Kopuz-org/kopuz/$1" |
        python3 -I -c 'import json, sys; print(json.load(sys.stdin)["storePath"])'
}
cited=$(store_path a58c385)
latest=$(store_path 1d8b323)

extensions='\.(rs|c|cc|cpp|h|kt|kts|java|json|sh|toml|yml|yaml|nu|ftl|properties|pro|desktop|py|nix|xml|svg|md)$'
mapfile -t targets < <(git ls-files | grep -E "$extensions" |
    grep -v -E '^crates/widevine/vendor/.*\.h$|^docs/audits/[^/]+/comparison\.md$' | sort)

report=${REPORT:-$here/comparison.md}
python3 -I "$here/../2026-11-05-cenc-clean-room/token_compare.py" "$report" --all-text \
    --target "${targets[@]}" \
    --source "kopuz-a58c385=$cited" "kopuz-1d8b323=$latest" \
    --forced crates/widevine/vendor/content_decryption_module.h crates/widevine/vendor/content_decryption_module_export.h \
    --origin "kopuz-a58c385=$cited" "kopuz-1d8b323=$latest"

# Files the same byte for byte: every tracked file of Opium's against every file of Kopuz's.
{
    printf '\n## Files identical byte for byte\n\n'
    for revision in a58c385 1d8b323; do
        root=$cited
        [ "$revision" = 1d8b323 ] && root=$latest
        # sha256sum writes 64 digits and two spaces, then the path.
        (cd "$root" && find . -type f -print0 | xargs -0 sha256sum) | sed 's#^\(.\{66\}\)\./#\1#' >"$report.kopuz"
        git ls-files -z | xargs -0 sha256sum >"$report.opium"
        # An empty file is the same as every other, and holds nothing to copy.
        awk -v revision="$revision" -v empty="$(sha256sum </dev/null | cut -c1-64)" '
            $1 == empty { next }
            NR == FNR { theirs[$1] = (theirs[$1] ? theirs[$1] "`, `" : "") substr($0, 67); next }
            $1 in theirs { printf "- kopuz-%s: `%s` is `%s`\n", revision, substr($0, 67), theirs[$1] }
        ' "$report.kopuz" "$report.opium" | sort
    done
    rm -f "$report.kopuz" "$report.opium"
} >>"$report"
