#!/usr/bin/env bash
# Starts a headless Weston for a measurement, so that the test window is always shown and never
# touches the desktop:
#
#   scripts/measure/weston.sh <socket> [<width>x<height>] [<refresh in mHz>] [kiosk]
#
# Defaults: 2560x1440 at 240000 mHz, scale 1, rendered on the GPU (--renderer=gl), the screen never
# blanked. With `kiosk`, Weston's kiosk shell shows each window full screen. Prints Weston's pid; end it with `kill -TERM <pid>`, that pid only: other agents may run Westons
# of their own. Then run the app with
# WAYLAND_DISPLAY=<socket>. Weston comes from the development shell.
set -euo pipefail
socket=$1
size=${2:-2560x1440}
refresh=${3:-240000}
shell=desktop
[[ ${4:-} == kiosk ]] && shell=kiosk
dir=$(mktemp -d "${TMPDIR:-/tmp}/opium-weston.XXXXXX")
cat > "$dir/weston.ini" <<'INI'
[core]
idle-time=0
[shell]
panel-position=none
background-color=0xff000000
INI
weston --backend=headless --renderer=gl --shell="$shell" --width="${size%x*}" --height="${size#*x}" \
    --refresh-rate="$refresh" --socket="$socket" --config="$dir/weston.ini" \
    --log="$dir/weston.log" </dev/null >/dev/null 2>&1 &
pid=$!
disown
for _ in $(seq 100); do
    [[ -S ${XDG_RUNTIME_DIR:?}/$socket ]] && break
    sleep 0.1
done
[[ -S $XDG_RUNTIME_DIR/$socket ]] || { echo "weston did not start; see $dir/weston.log" >&2; exit 1; }
echo "$pid"
