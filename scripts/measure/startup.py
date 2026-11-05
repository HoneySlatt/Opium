#!/usr/bin/env python3
"""Times the warm start of the release build.

usage: scripts/measure/startup.py <profile-dir> <runs> <out.json> [map]

Each run starts the app through run.sh on $WAYLAND_DISPLAY and records:
- shot_ms: exec to the first frame saved, with OPIUM_SCREENSHOT_AFTER=0. An upper bound of the
  first frame: it adds egui's read-back and the PNG's encoding. The app quits by itself.
- map_ms, with `map`: exec to the window first listed by the compositor, read from Umbriel's event
  stream (`umbriel subscribe windows`), so on the owner's desktop only. The run is then ended by
  SIGTERM to its own pid; nothing is asked of the compositor.
Warm means the binary and the library's cache are already in the page cache; a cold start needs
root to drop it, and is not measured here.
"""
import json
import os
import statistics
import subprocess
import sys
import tempfile
import threading
import time

profile, runs, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
use_map = len(sys.argv) > 4 and sys.argv[4] == "map"
run_sh = os.path.join(os.path.dirname(os.path.abspath(__file__)), "run.sh")
shots = tempfile.mkdtemp(prefix="opium-startup.")
results = []


def watch_map(stream, seen):
    for line in stream:
        moment = time.monotonic()
        try:
            windows = json.loads(line)["data"]
        except (ValueError, KeyError):
            continue
        if any(w.get("app_id") == "opium" for w in windows):
            seen["map"] = moment
            return


def watch_shot(stream, seen):
    for line in stream:
        if "screenshot saved" in line:
            seen["shot"] = time.monotonic()


for run in range(runs):
    seen = {}
    events = None
    if use_map:
        events = subprocess.Popen(["umbriel", "subscribe", "windows"], stdout=subprocess.PIPE, text=True)
        events.stdout.readline()  # the windows already there
        threading.Thread(target=watch_map, args=(events.stdout, seen), daemon=True).start()
    extra = [] if use_map else [f"OPIUM_SCREENSHOT={shots}/start-{run}.png", "OPIUM_SCREENSHOT_AFTER=0"]
    start = time.monotonic()
    app = subprocess.Popen([run_sh, profile, *extra], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    threading.Thread(target=watch_shot, args=(app.stderr, seen), daemon=True).start()
    if use_map:
        deadline = time.monotonic() + 20
        while "map" not in seen and time.monotonic() < deadline:
            time.sleep(0.005)
        app.terminate()  # SIGTERM to the test instance's own pid
    app.wait(timeout=30)
    if events:
        events.kill()
    result = {
        "map_ms": (seen["map"] - start) * 1000 if "map" in seen else None,
        "shot_ms": (seen["shot"] - start) * 1000 if "shot" in seen else None,
    }
    results.append(result)
    print(run, {k: round(v, 1) if v else v for k, v in result.items()}, flush=True)
    time.sleep(2)

with open(out, "w") as f:
    json.dump(results, f, indent=1)
for key in ("map_ms", "shot_ms"):
    values = sorted(r[key] for r in results if r[key] is not None)
    if values:
        print(f"{key}: median {statistics.median(values):.1f} min {values[0]:.1f} "
              f"max {values[-1]:.1f} n {len(values)}")
