#!/usr/bin/env bash
# Runs builds of the CDM's measuring program in turn, the same way, for a before-and-after:
#
#   scripts/measure/cdm-cost.sh <out-dir> <song id> <rounds> <name>=<binary>...
#
# Each round runs every binary once, in the order given, with the default 30 s at rest; each run
# writes <out-dir>/<name>-<round>.txt. A binary is a release build of
# `crates/apple-music/examples/cdm_cost.rs`, copied aside, the old one built from the commit
# before the change. Every run asks Apple for one licence on the owner's account: only with the
# owner's go (AGENTS.md, "Never on your own"). Runs are ten seconds apart, so that licences are
# never asked in a burst. Nothing is played; nothing is written but the reports.
set -euo pipefail
out=$1 song=$2 rounds=$3
shift 3
mkdir -p "$out"
for round in $(seq "$rounds"); do
    for build in "$@"; do
        name=${build%%=*} binary=${build#*=}
        echo "round $round: $name"
        "$binary" "$song" >"$out/$name-$round.txt" 2>&1 || echo "  failed, see $out/$name-$round.txt"
        sleep 10
    done
done
