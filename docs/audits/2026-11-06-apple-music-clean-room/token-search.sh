#!/usr/bin/env bash
# Times the developer token's search, M1.12 (docs/measurements.md), on a page and a script of
# Apple's web player saved outside the repository:
#
#   docs/audits/2026-11-06-apple-music-clean-room/token-search.sh <page.html> <index.js>
#
# Inside nix develop. The code before is its two regular expressions; the code after is
# crates/apple-music/src/auth.rs's search functions, copied into the bench crate beside this
# script (token-search/) and made public there, for timing only. Prints the steady state (101
# searches each, alternately, the expressions built once) and the first search of a process (30
# processes each, alternately, the expressions built then, as at Opium's start).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
page=$(realpath "$1") script=$(realpath "$2")
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp -r "$here/token-search/." "$work/"
cp "$here/../../../Cargo.lock" "$work/"
python3 -I - "$here/../../../crates/apple-music/src/auth.rs" "$work/src/new_search.rs" <<'PY'
import re, sys
source = open(sys.argv[1]).read()
source = source[: source.index("#[cfg(feature = \"fuzzing\")]")]
out = [
    "// Copied from crates/apple-music/src/auth.rs for timing only: the searches, made public.\n",
    "use std::sync::LazyLock;\nuse base64::Engine;\n"
    "use base64::engine::general_purpose::URL_SAFE_NO_PAD;\nuse regex::Regex;\n",
]
for name in ["MAIN_SCRIPT", "JWT_SHAPED"]:
    out.append(re.search(r"static " + name + r":.*?\);\n", source, re.S).group(0))
for name in ["claims", "decoded_json", "main_script", "continues_an_address", "continues_a_path",
             "first_token", "first_token_counting", "names_es256", "is_base64url"]:
    out.append("pub " + re.search(r"^fn " + name + r"\b.*?^\}\n", source, re.S | re.M).group(0))
open(sys.argv[2], "w").write("\n".join(out))
PY
(cd "$work" && cargo build --release --offline -q)
echo "steady state:"
"$work/target/release/token-search-bench" "$page" "$script"
echo "first search of a process:"
for _ in $(seq 30); do
    for side in old new; do
        echo "$side $("$work/target/release/cold" "$side" "$page" "$script")"
    done
done | python3 -I -c '
import collections, sys
times = collections.defaultdict(list)
for line in sys.stdin:
    side, nanos = line.split()
    times[side].append(int(nanos) / 1e3)
for side in ("old", "new"):
    t = sorted(times[side])
    print(f"  {side}: median {t[len(t) // 2]:.1f} µs, p95 {t[int(len(t) * 0.95)]:.1f} µs")'
