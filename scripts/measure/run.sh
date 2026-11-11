#!/usr/bin/env bash
# Starts the release build on a profile of its own, for a measurement:
#
#   scripts/measure/run.sh <profile-dir> [VAR=value...]
#
# The profile's config, cache and data folders are <profile-dir>/{config,cache,data}. The session
# bus is pointed nowhere, so no MPRIS or keyring runs (CONTRIBUTING.md, "What the folders do not
# redirect"); the profile's own lock keeps one instance on it. No Widevine CDM is loaded unless
# WIDEVINE_CDM names one (a path that does not exist by default), as the search would otherwise
# find the owner's under $HOME. The window opens on $WAYLAND_DISPLAY: a headless Weston from
# weston.sh, or the desktop. Run inside `nix develop`, which
# sets the libraries the window loads. The script execs the app, so its pid is the app's: end the
# run with `kill -TERM <pid>`, never through the compositor.
set -euo pipefail
cd "$(dirname "$0")/../.."
profile=$1
shift
: "${WAYLAND_DISPLAY:?set WAYLAND_DISPLAY, for instance to the socket of weston.sh}"
exec env XDG_CONFIG_HOME="$profile/config" XDG_CACHE_HOME="$profile/cache" \
    XDG_DATA_HOME="$profile/data" DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent \
    WIDEVINE_CDM="${WIDEVINE_CDM:-/nonexistent}" "$@" "${OPIUM_BIN:-target/release/opium}"
