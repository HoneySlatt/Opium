#!/usr/bin/env bash
# Measures the app at rest: start it, let it settle, sample it, end it by its own pid.
#
#   scripts/measure/rest.sh <label> <profile-dir> <out-dir> [VAR=value...]
#
# Starts the release build through run.sh on $WAYLAND_DISPLAY (a headless Weston from weston.sh,
# so that the window is shown and nothing touches the desktop), waits 30 s, then for 60 s records
# its wake-ups and CPU time (sample.py), its memory over 95 s from the start (memwatch.py) and
# `top`. Writes <out-dir>/<label>.json, mem-<label>.json, top-<label>.txt and run-<label>.log, then
# ends the run with SIGTERM to its own pid. Refuses to start if an Opium already runs.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
label=$1 profile=$2 out=$3
shift 3
mkdir -p "$out"
if pgrep -ax opium; then
    echo "an Opium already runs: whose? (CONTRIBUTING.md, \"The owner's own window\")" >&2
    exit 1
fi
"$here/run.sh" "$profile" "$@" > "$out/run-$label.log" 2>&1 &
pid=$!
python3 -I "$here/memwatch.py" "$pid" 95 "$out/mem-$label.json" > "$out/mem-$label.txt" &
memwatch=$!
sleep 30
top -b -d 60 -n 2 -p "$pid" | grep -E "^ *$pid" | tail -1 > "$out/top-$label.txt" &
top=$!
python3 -I "$here/sample.py" "$pid" 60 "$out/$label.json"
wait "$memwatch" "$top" || true
head -1 "$out/mem-$label.txt"
kill -TERM "$pid"
timeout 10 tail --pid="$pid" -f /dev/null
