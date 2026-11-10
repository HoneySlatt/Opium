#!/usr/bin/env bash
# Reproduces docs/debt.md, "Two instances can run at once", with the app itself: a throwaway profile
# shared by the instances, as the owner's are; a private session bus (dbus-run-session); a headless
# Weston and a runtime folder of their own, so no window on the desktop and no PipeWire socket, so
# no sound; no network (unshare --net). Nothing is played. Run inside `nix develop`, from the
# repository's root, after `cargo build -p app-desktop --release`:
#
#   bash docs/audits/2026-11-10-stage-1/correctness/two_instances.sh
#
# OPIUM_BIN picks another build (the debug build is slower to start, which widens the race).
# GAP is the time between the first two launches, 0.1 s by default (a double click); 0 always races.
# Every process started here is ended by its own pid. Not run by the agent that wrote it: the brief
# forbade running the app.
set -euo pipefail

if [[ ${1:-} != --inside ]]; then
    bin=$(realpath "${OPIUM_BIN:-target/release/opium}")
    [[ -x $bin ]] || { echo "no $bin: build it first" >&2; exit 2; }
    test=$(mktemp -d /tmp/opium-two.XXXXXX)
    mkdir -m 700 "$test/run"
    echo "profile and logs in $test"
    exec unshare --user --map-current-user --net \
        env -u WAYLAND_DISPLAY -u DISPLAY XDG_RUNTIME_DIR="$test/run" \
        dbus-run-session -- bash "$0" --inside "$bin" "$test"
fi

bin=$2
test=$3
export XDG_CONFIG_HOME=$test/config XDG_CACHE_HOME=$test/cache XDG_DATA_HOME=$test/data
weston=$(scripts/measure/weston.sh opium-two 1440x900)
export WAYLAND_DISPLAY=opium-two
pids=()

finish() {
    for pid in "${pids[@]}"; do kill -TERM "$pid" 2>/dev/null || true; done
    kill -TERM "$weston" 2>/dev/null || true
}
trap finish EXIT

# The pid of the process holding org.mpris.MediaPlayer2.opium on this bus, or "nobody".
holder() {
    local unique
    unique=$(dbus-send --session --print-reply --dest=org.freedesktop.DBus /org/freedesktop/DBus \
        org.freedesktop.DBus.GetNameOwner string:org.mpris.MediaPlayer2.opium 2>/dev/null |
        awk -F'"' '/string/ { print $2 }') || true
    if [[ -z $unique ]]; then
        echo nobody
        return
    fi
    dbus-send --session --print-reply --dest=org.freedesktop.DBus /org/freedesktop/DBus \
        org.freedesktop.DBus.GetConnectionUnixProcessID "string:$unique" | awk '/uint32/ { print $2 }'
}

say() {
    local line="$1:"
    for name in a b c; do
        local var=pid_$name
        [[ -n ${!var:-} ]] || continue
        if kill -0 "${!var}" 2>/dev/null; then line+=" $name (pid ${!var}) runs;"; else line+=" $name ended;"; fi
    done
    echo "$line the name is held by: $(holder)"
}

launch() {
    "$bin" >"$test/$1.log" 2>&1 &
    printf -v "pid_$1" %s "$!"
    pids+=("$!")
}

# 1. Two launches close together: both look for an Opium (main.rs:22-30) before either has taken
#    the name (app.rs:880, at the end of App::new), so both open a window. The later one to reach
#    Mpris::start takes the name from the other (mpris.rs:273).
launch a
sleep "${GAP:-0.1}"
launch b
sleep 8
say "two launches ${GAP:-0.1} s apart"

# 2. Quit the one holding the name, as `playerctl --player=opium quit` would.
dbus-send --session --type=method_call --dest=org.mpris.MediaPlayer2.opium /org/mpris/MediaPlayer2 \
    org.mpris.MediaPlayer2.Quit
sleep 7 # it ends within the exit watch's 5 s
say "after MPRIS Quit"

# 3. A new launch finds no Opium on the bus, and opens one more window over the same profile.
launch c
sleep 8
say "after a third launch"

echo "logs: $test/{a,b,c}.log ('media controls unavailable' would mean a name refused)"
