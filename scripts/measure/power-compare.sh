#!/usr/bin/env bash
# Compares the GPU's power with the app closed and open at rest, in pairs.
#
#   scripts/measure/power-compare.sh <profile-dir> <out-dir> [pairs]
#
# For each pair (3 by default): 30 s of power.py with no Opium running; then the release build
# started through run.sh on $WAYLAND_DISPLAY, 30 s to settle, 30 s of power.py beside sample.py,
# and the run ended with SIGTERM to its own pid. A difference within the sensor's noise (about
# ±2 W on the owner's desktop) reads as none. Refuses to start if an Opium already runs.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
profile=$1 out=$2 pairs=${3:-3}
mkdir -p "$out"
for i in $(seq "$pairs"); do
    if pgrep -ax opium; then
        echo "an Opium already runs: whose? (CONTRIBUTING.md, \"The owner's own window\")" >&2
        exit 1
    fi
    python3 -I "$here/power.py" 30 "$out/power-closed-$i.json"
    "$here/run.sh" "$profile" > "$out/run-power-$i.log" 2>&1 &
    pid=$!
    sleep 30
    python3 -I "$here/sample.py" "$pid" 30 "$out/power-open-$i-wake.json" | head -1 &
    python3 -I "$here/power.py" 30 "$out/power-open-$i.json"
    wait "$!" || true
    kill -TERM "$pid"
    timeout 10 tail --pid="$pid" -f /dev/null
done
