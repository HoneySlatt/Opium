#!/usr/bin/env bash
# Makes a copy of the owner's profile for a measurement on the real library:
#
#   scripts/measure/copy-profile.sh <dir> [nosession]
#
# Only with the owner's go (AGENTS.md, "Never on your own"): the copy holds the owner's tokens, and a
# run with the network on talks to the real services and may renew them. It follows CONTRIBUTING.md,
# "Tests on a copy of the owner's profile": `connect.json` removed, `muted = true`, an empty data
# folder. `nosession` also removes the song shown at start. Delete <dir> when done.
set -euo pipefail
dir=$1
config=${XDG_CONFIG_HOME:-$HOME/.config}/opium
cache=${XDG_CACHE_HOME:-$HOME/.cache}/opium
mkdir -p "$dir/config" "$dir/cache" "$dir/data"
cp -a "$config" "$dir/config/opium"
cp -a "$cache" "$dir/cache/opium"
rm -f "$dir/config/opium/connect.json"
settings=$dir/config/opium/settings.toml
# Replace the key, never add a second one (CONTRIBUTING.md, "Settings care").
if grep -q '^muted = ' "$settings"; then
    sed -i 's/^muted = .*/muted = true/' "$settings"
else
    printf 'muted = true\n' | cat - "$settings" > "$settings.new" && mv "$settings.new" "$settings"
fi
grep -q '^muted = true$' "$settings"
if [[ ${2:-} == nosession ]]; then
    rm -f "$dir/config/opium/session.json"
    python3 -I -c '
import json, sys
path = sys.argv[1]
state = json.load(open(path))
state["shared"]["session"] = None
json.dump(state, open(path, "w"))' "$dir/config/opium/shared.json"
fi
echo "copied to $dir"
