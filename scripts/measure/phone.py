#!/usr/bin/env python3
"""Measures the phone's app under Compose over adb: start, frames while scrolling and moving
between pages, memory.

usage: scripts/measure/phone.py <runs> <out-dir>

Run with `adb` on the PATH (`nix shell nixpkgs#android-tools`) and the owner's phone plugged in,
with the release build installed (CONTRIBUTING.md, "The phone over adb"). It plays nothing, and
changes nothing but what is shown. In order:
1. checks: one device; the app not DEBUGGABLE; Opium or the home screen in front; Opium neither
   playing nor downloading, since a cold start ends its process, and with it the music or the
   download;
2. cold starts, <runs> times: `am force-stop`, then `am start -W` (TotalTime: the system's time
   from the intent to the first frame drawn);
3. hot starts, <runs> times: Home, then `am start -W` again, the process still alive;
4. memory at rest, 5 s after a start: `dumpsys meminfo` (total PSS and RSS);
5. Songs and Albums, rows of the Library tab: `dumpsys gfxinfo` reset, 12 flings up the list and
   12 down, and `dumpsys gfxinfo` read: the frames, the janky ones, the percentiles of frame time,
   the missed vsyncs;
6. moving between pages: the tabs Home, New, Radio and Library tapped in turn, 4 rounds, gfxinfo
   likewise;
7. memory after it all, then Home.
Before each cold start, tap or key, it checks again that Opium is the focused app, playing and
downloading nothing, and stops if not: the owner may have taken the phone back. The labels are found in a dump of the interface
(`uiautomator dump`), read only when Opium has the focus, and deleted from the phone at once. Only
the navigation is tapped, each label within the heights where the Stock theme's layout of
2026-11-07 puts it (on a 1080x2424 screen): a label found elsewhere, which could be a song's, or
another theme's layout, stops the script.
Writes <out-dir>/phone.json and prints its figures.
"""
import json
import os
import re
import statistics
import subprocess
import sys
import time

PACKAGE = "io.github.honeyslatt.opium"
ACTIVITY = f"{PACKAGE}/.MainActivity"
runs, out = int(sys.argv[1]), sys.argv[2]
os.makedirs(out, exist_ok=True)


def adb(*args):
    return subprocess.run(["adb", *args], check=True, capture_output=True, text=True).stdout


def shell(command):
    return adb("shell", command)


def opium_in_front():
    return f"{PACKAGE}/" in shell("dumpsys window | grep mCurrentFocus")


def require_front(what):
    """Stops unless Opium is the focused app, plays nothing and downloads nothing: the owner may
    have taken the phone back, and a cold start would end the music or the download."""
    if not opium_in_front():
        focus = shell("dumpsys window | grep mCurrentFocus").strip()
        sys.exit(f"stopped before {what}: Opium is not the focused app (the owner may have the "
                 f"phone): {focus}")
    require_idle(what)


def require_idle(what):
    if playing():
        sys.exit(f"stopped before {what}: Opium is playing")
    if "DownloadService" in shell(f"dumpsys activity services {PACKAGE} | grep ServiceRecord || true"):
        sys.exit(f"stopped before {what}: Opium is downloading")


def playing():
    sessions = shell("dumpsys media_session")
    for block in sessions.split("package=")[1:]:
        if block.startswith(PACKAGE) and "state=PLAYING" in block:
            return True
    return False


def start():
    text = shell(f"am start -W -n {ACTIVITY}")
    total = re.search(r"TotalTime: (\d+)", text)
    state = re.search(r"LaunchState: (\w+)", text)
    return (int(total.group(1)) if total else None), (state.group(1) if state else None)


def memory():
    text = shell(f"dumpsys meminfo {PACKAGE}")
    pss = re.search(r"TOTAL PSS:\s+(\d+)", text)
    rss = re.search(r"TOTAL RSS:\s+(\d+)", text)
    return {"pss_mib": round(int(pss.group(1)) / 1024, 1) if pss else None,
            "rss_mib": round(int(rss.group(1)) / 1024, 1) if rss else None}


def labels():
    """The interface's nodes: text or description, and the centre of their bounds."""
    for _ in range(3):
        require_front("reading the interface")
        # It fails while the interface moves ("could not get idle state"): try again.
        dumped = "dumped to" in shell("uiautomator dump /sdcard/opium-measure-ui.xml")
        try:
            xml = adb("exec-out", "cat", "/sdcard/opium-measure-ui.xml") if dumped else ""
        finally:
            shell("rm -f /sdcard/opium-measure-ui.xml")
        if dumped:
            break
        time.sleep(1)
    else:
        sys.exit("the interface could not be read")
    nodes = []
    for node in re.finditer(r"<node [^>]*>", xml):
        attrs = dict(re.findall(r'([\w-]+)="([^"]*)"', node.group(0)))
        if attrs.get("package") != PACKAGE:
            continue
        x1, y1, x2, y2 = map(int, re.findall(r"\d+", attrs.get("bounds", "[0,0][0,0]")))
        nodes.append({"text": attrs.get("text", ""), "desc": attrs.get("content-desc", ""),
                      "scrollable": attrs.get("scrollable") == "true",
                      "centre": ((x1 + x2) // 2, (y1 + y2) // 2), "size": (x2 - x1) * (y2 - y1)})
    return nodes


def tap(name, top, bottom):
    """Taps the label `name` whose centre lies between the heights `top` and `bottom`, in pixels:
    only the navigation is ever tapped, never a list's row, which could be a song."""
    found = [n for n in labels() if n["text"] == name and top <= n["centre"][1] <= bottom]
    if len(found) != 1:
        sys.exit(f"not one {name!r} between heights {top} and {bottom}: the interface has changed")
    x, y = found[0]["centre"]
    require_front(f"tapping {name}")
    shell(f"input tap {x} {y}")
    time.sleep(1.5)


TABS = ("Home", "New", "Radio", "Library")


def tab(name):
    """Taps the tab `name`, once the four tabs are found side by side along the bottom: a song
    or a station of the same name is never taken for it."""
    nodes = [n for n in labels() if n["text"] in TABS and n["centre"][1] > HEIGHT - 250]
    rows = {n["centre"][1] for n in nodes}
    if len(nodes) != len(TABS) or len(rows) != 1:
        sys.exit("the tabs along the bottom are not where the Stock theme puts them")
    x, y = next(n["centre"] for n in nodes if n["text"] == name)
    require_front(f"tapping the tab {name}")
    shell(f"input tap {x} {y}")
    time.sleep(1.5)


def to_library():
    """Library, by its tab, checked by its title."""
    tab("Library")
    if not any(n["text"] == "Library" and n["centre"][1] < 400 for n in labels()):
        sys.exit("Library did not open")


def gfxinfo():
    text = shell(f"dumpsys gfxinfo {PACKAGE}")
    figures = {}
    for key, pattern in [("frames", r"Total frames rendered: (\d+)"),
                         ("janky", r"Janky frames: (\d+)"),
                         ("p50_ms", r"50th percentile: (\d+)ms"),
                         ("p90_ms", r"90th percentile: (\d+)ms"),
                         ("p95_ms", r"95th percentile: (\d+)ms"),
                         ("p99_ms", r"99th percentile: (\d+)ms"),
                         ("missed_vsync", r"Number Missed Vsync: (\d+)"),
                         ("deadline_missed", r"Number Frame deadline missed: (\d+)"),
                         ("slow_ui_thread", r"Number Slow UI thread: (\d+)"),
                         ("gpu_p50_ms", r"50th gpu percentile: (\d+)ms"),
                         ("gpu_p99_ms", r"99th gpu percentile: (\d+)ms")]:
        found = re.search(pattern, text)
        figures[key] = int(found.group(1)) if found else None
    return figures


def flings(count):
    """Flings up the page, then back down, between the heights 700 and 1700: below the page's
    head, above the player along the bottom. A swipe this fast is never taken for a tap."""
    require_front("scrolling")
    x = WIDTH // 2
    for low, high in ((1700, 700), (700, 1700)):
        for _ in range(count):
            shell(f"input swipe {x} {low} {x} {high} 120")
            time.sleep(0.6)


if len(adb("devices").strip().splitlines()) != 2:
    sys.exit("not exactly one device")
WIDTH, HEIGHT = map(int, re.search(r"(\d+)x(\d+)", shell("wm size")).groups())
if "DEBUGGABLE" in shell(f"dumpsys package {PACKAGE} | grep pkgFlags"):
    sys.exit("a debuggable build is installed: install the release build")
# The phone set aside: Opium or the home screen in front, nothing playing or downloading.
if not opium_in_front() and "launcher" not in shell("dumpsys window | grep mCurrentFocus"):
    sys.exit("another app is in front: the owner may be using the phone")
require_idle("the cold starts")

result = {"device": shell("getprop ro.product.model").strip(),
          "build": shell("getprop ro.build.fingerprint").strip(),
          "app": shell(f"dumpsys package {PACKAGE} | grep -m1 lastUpdateTime").strip(),
          "compiled": shell(f"dumpsys package dexopt | grep -A2 '{PACKAGE}]' | grep -o 'status=[a-z-]*'").strip()}

cold = []
for run in range(runs):
    if run:
        require_front("a cold start")
    shell(f"am force-stop {PACKAGE}")
    time.sleep(3)
    cold.append(start())
    print("cold start", cold[-1], flush=True)
    time.sleep(5)
result["cold_ms"] = [t for t, _ in cold]
result["memory_at_rest"] = memory()

hot = []
for _ in range(runs):
    require_front("going Home")
    shell("input keyevent KEYCODE_HOME")
    time.sleep(2)
    hot.append(start())
    print("hot start", hot[-1], flush=True)
    time.sleep(2)
result["hot_ms"] = [t for t, _ in hot]
result["hot_states"] = sorted({s for _, s in hot if s})

# Songs and Albums: rows of Library, under its title.
for page in ("Songs", "Albums"):
    to_library()
    tap(page, 300, 900)
    shell(f"dumpsys gfxinfo {PACKAGE} reset")
    flings(12)
    result[f"scroll_{page.lower()}"] = gfxinfo()

# Between pages: the tabs along the bottom, in turn.
shell(f"dumpsys gfxinfo {PACKAGE} reset")
for _ in range(4):
    for name in TABS:
        tab(name)
result["pages"] = gfxinfo()
result["memory_after"] = memory()
require_front("going Home")
shell("input keyevent KEYCODE_HOME")

for key in ("cold_ms", "hot_ms"):
    times = [t for t in result[key] if t is not None]
    result[key + "_median"] = statistics.median(times) if times else None
with open(os.path.join(out, "phone.json"), "w") as summed:
    json.dump(result, summed, indent=1)
print(json.dumps(result, indent=1))
