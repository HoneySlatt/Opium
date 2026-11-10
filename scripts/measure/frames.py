#!/usr/bin/env python3
"""Runs one of the window's scripted scenarios and sums up its frames, its start and its memory.

usage: scripts/measure/frames.py <profile-dir> <scenario> <runs> <out-dir> [<refresh in Hz>]
       scripts/measure/frames.py --again <scenario> <runs> <out-dir> [<refresh in Hz>]

<scenario> is one of `crates/app-desktop/src/measure.rs`'s: start, scroll-songs, scroll-albums,
animate. Each run starts the release build through run.sh on $WAYLAND_DISPLAY, cut off the network
(`unshare --user --map-current-user --net`), with OPIUM_MEASURE set; the app writes its frame
record and closes itself. Meanwhile its memory is read from /proc every 0.25 s. A run that has
not ended after 400 s is ended by SIGTERM to its own pid, and counts as failed, record or not. `--again` runs
nothing: it sums up again the files that earlier runs left in <out-dir>, keeping the load average
read when they ended (the system's, over 1, 5 and 15 minutes, the measured app's work included),
or none if no earlier summary is there.

For every run, in <out-dir>: the record (<scenario>-<n>.csv), the memory with the time the program
was started (<scenario>-<n>-mem.json) and the app's standard error (<scenario>-<n>.log); then
<scenario>.json, the figures of every run and of all runs together:
- start_ms: from just before the program is started to the end of the first useful frame's work
  (its start plus its CPU time). The library comes from its index, so a profile's first run, which
  scans the folder, is not a warm start: run a `start` first and drop it.
- after_useful_ms: the first useful frame's interval to the next frame, which its own work makes
  long; it is left out of the phase that follows.
- for each phase of the scenario: the CPU time of every frame in it (eframe's: interface,
  tessellation and drawing calls), and the interval of each frame the frame before asked for at
  once (the record's `continued`), the time from the frame before's start, which is what the
  display shows; the interval is held when it is one period of <refresh in Hz> (240 by default);
  a frame later than 1.5 periods missed at least one vsync;
- memory: VmRSS and RssAnon in the first sample after the first useful frame began (within 0.25 s),
  their highest after it, and VmHWM, the highest VmRSS the kernel saw.
A run is left out of the figures together, and said on standard error, if it never showed the
library, if a frame was begun but not drawn (`undrawn`: the window hidden, or the compositor
withholding frames), if its scenario did not end by itself
(`ended`), if a `scroll-songs` run did not scroll the whole list (see `scroll_reach`: the record's
`rows_points`, the height of the list's rows, against the distance `scrolled`; the list is
the library's, without copies of a song across services), if it was not drawn at OPIUM_MEASURE_PIXELS_PER_POINT (1.65 by default: the owner's
scale 1.5 at the interface's 100 %), or, with OPIUM_MEASURE_PIXELS set (3840x2160), if its
window had another size. Refuses to start if an Opium
already runs. Run inside `nix develop`.
"""
import json
import os
import signal
import statistics
import subprocess
import sys
import threading
import time

again = sys.argv[1] == "--again"
if again:
    profile = None
    scenario, runs, out, *rest = sys.argv[2:]
else:
    profile, scenario, runs, out, *rest = sys.argv[1:]
runs = int(runs)
refresh = float(rest[0]) if rest else 240.0
period_ms = 1000.0 / refresh
pixels = os.environ.get("OPIUM_MEASURE_PIXELS")
# By default the owner's display at scale 1.5, the interface at 100 % (which draws at 1.1).
PIXELS_PER_POINT = float(os.environ.get("OPIUM_MEASURE_PIXELS_PER_POINT", "1.65"))
run_sh = os.path.join(os.path.dirname(os.path.abspath(__file__)), "run.sh")


def watch_memory(pid, samples, stop):
    while not stop.is_set():
        moment = time.time()
        try:
            with open(f"/proc/{pid}/status") as status:
                fields = dict(line.split(":", 1) for line in status if ":" in line)
        except OSError:
            return
        sample = {"t": moment}
        for key in ("VmRSS", "RssAnon", "RssFile", "VmHWM"):
            if key in fields:
                sample[key] = int(fields[key].split()[0]) / 1024  # kB to MiB
        samples.append(sample)
        stop.wait(0.25)


def measure(run):
    """Runs the scenario once and leaves its files in <out-dir>."""
    record = os.path.join(out, f"{scenario}-{run}.csv")
    if os.path.exists(record):
        os.remove(record)
    samples, stop = [], threading.Event()
    with open(os.path.join(out, f"{scenario}-{run}.log"), "w") as log:
        started = time.time()
        app = subprocess.Popen(
            ["unshare", "--user", "--map-current-user", "--net", run_sh, profile,
             f"OPIUM_MEASURE={scenario}", f"OPIUM_MEASURE_OUT={record}"],
            stdout=subprocess.DEVNULL, stderr=log)
        watcher = threading.Thread(target=watch_memory, args=(app.pid, samples, stop), daemon=True)
        watcher.start()
        timed_out = False
        try:
            app.wait(timeout=400)
        except subprocess.TimeoutExpired:
            timed_out = True
            app.send_signal(signal.SIGTERM)  # its own pid: unshare and run.sh exec the app
            app.wait()
        stop.set()
        watcher.join()
    with open(os.path.join(out, f"{scenario}-{run}-mem.json"), "w") as mem:
        json.dump({"started": started, "timed_out": timed_out, "samples": samples}, mem)


def read_record(path):
    facts, frames = {}, []
    with open(path) as record:
        for line in record:
            line = line.strip()
            if line.startswith("#"):
                if "=" in line:
                    key, value = line[1:].strip().split("=", 1)
                    facts[key] = value
            elif line and not line.startswith("start_us"):
                start, cpu, phase, continued, scrolled = line.split(",")
                frames.append({"start": int(start) / 1e6, "cpu_ms": int(cpu) / 1e3 if cpu else None,
                               "phase": phase, "continued": continued == "1",
                               "scrolled": float(scrolled)})
    return facts, frames


def percentile(values, p):
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(round(p / 100 * (len(ordered) - 1))))]


def spread(values):
    if not values:
        return None
    return {"n": len(values), "median": round(statistics.median(values), 3),
            "p99": round(percentile(values, 99), 3), "max": round(max(values), 3)}


def phases(frames):
    """Each phase's CPU times, of every frame in it, and intervals, of the frames the frame before
    asked for at once. The interval after the first useful frame measures that frame's own work,
    and stays out."""
    found = {}
    for before, frame in zip(frames, frames[1:]):
        if frame["phase"] in ("loading", "useful", "end"):
            continue
        phase = found.setdefault(frame["phase"], {"cpu": [], "interval": []})
        if frame["continued"] and before["phase"] != "useful":
            phase["interval"].append((frame["start"] - before["start"]) * 1e3)
        if frame["cpu_ms"] is not None:
            phase["cpu"].append(frame["cpu_ms"])
    return found


def summary(found):
    result = {}
    for name, phase in found.items():
        intervals = phase["interval"]
        result[name] = {
            "cpu_ms": spread(phase["cpu"]),
            "interval_ms": spread(intervals),
            "missed": sum(1 for i in intervals if i > 1.5 * period_ms),
            "over_8_33_ms": sum(1 for i in intervals if i > 8.33),
            "cpu_over_period": sum(1 for c in phase["cpu"] if c > period_ms),
        }
    return result


# What the egui window's scroll does, from crates/app-desktop/src/measure.rs and ui/songs.rs.
# A row of the Songs list, in points (ui/tracks.rs `ROW`): for records that predate `rows_points`.
SONGS_ROW_POINTS = 52.0
# The most a page can be taller than its rows: its header and foot, with the banner layout
# (ui/songs.rs: detail::BANNER + ACTIONS, tracks::HEAD_SPACE, a foot of 52 and 104 below it).
# The stock layout's is 372.
LIST_EXTRA_POINTS = 629.0
# How fast the scroll goes, in points a second (measure.rs `SCROLL_SPEED`).
SCROLL_SPEED = 4000.0
# A frame this late, in seconds, is far beyond the display's pace (4 ms at 240 Hz).
LATE_FRAME = 0.25
# The scroll counts a frame's whole push once the page took any of it, so the last one may
# overshoot the end by up to a frame's push: SCROLL_SPEED x LATE_FRAME = 1,000 points.
LAST_STEP_POINTS = SCROLL_SPEED * LATE_FRAME


def scroll_reach(facts):
    """Why a `scroll-songs` run's distance does not cover its list; empty if it does.

    The record says the height of the list's rows in points (`rows_points`; for an older record,
    its `songs` x SONGS_ROW_POINTS, said on standard error), the window's size (`window_points`,
    "WxH") and how far the scroll went (`scrolled`). The page ends at its rows plus a header and
    a foot (372 points, 629 with the banner layout) less one window's height, and a run that
    went the whole way counted at least that distance (every push the page took counts whole),
    and at most one frame's push more. So a fit run has `scrolled` between `rows_points` less
    the window's height and `rows_points` plus 629 plus LAST_STEP_POINTS (the push of a frame
    0.25 s late at 4,000 points a second: 1,000). Shorter is a scroll that stopped; longer, one
    counted twice (the audit's runs: 7,066 and 411,933 stopped, 853,559 went on, of 441,064).

    `rows_points` counts the library, not the rows laid out: it holds for a library without
    copies of a song across services, fully loaded before the scroll, as library.py makes it.
    """
    try:
        window = float(facts["window_points"].split("x")[1])
        scrolled = float(facts["scrolled"])
        if "rows_points" in facts:
            rows = float(facts["rows_points"])
        else:
            rows = float(facts["songs"]) * SONGS_ROW_POINTS
            print("the record has no rows_points: the list's height taken as songs x "
                  f"{SONGS_ROW_POINTS:.0f} points", file=sys.stderr)
    except (KeyError, IndexError, ValueError):
        return ["the record does not say the list's height (rows_points, or songs), the window's "
                "(window_points) or how far it scrolled: the run is left out"]
    least, most = rows - window, rows + LIST_EXTRA_POINTS + LAST_STEP_POINTS
    # Written so that a NaN fails.
    if not least <= scrolled <= most:
        return [f"scrolled {scrolled:.0f} points, where the list's {rows:.0f} should take "
                f"{least:.0f} to {most:.0f}"]
    return []


def unfit(facts):
    """Why a run's figures may not stand with the others; empty if they may."""
    reasons = []
    if scenario == "scroll-songs":
        reasons += scroll_reach(facts)
    if facts.get("undrawn", "0") != "0":
        reasons.append(f"{facts['undrawn']} frames begun but not drawn")
    if facts.get("ended") not in ("done", "end"):
        reasons.append(f"ended: {facts.get('ended')}")
    if abs(float(facts.get("pixels_per_point", 0)) - PIXELS_PER_POINT) > 0.005:
        reasons.append(f"{facts.get('pixels_per_point')} pixels a point, not {PIXELS_PER_POINT}")
    if pixels and facts.get("window_pixels") != pixels:
        reasons.append(f"window {facts.get('window_pixels')}, not {pixels}")
    return reasons


def sum_up(run):
    """One run's figures, read from its files; `failed` says why they stand apart, if they do."""
    record = os.path.join(out, f"{scenario}-{run}.csv")
    if not os.path.exists(record):
        print(f"run {run}: no record (see {scenario}-{run}.log)", file=sys.stderr)
        return {"run": run, "failed": "no record"}
    with open(os.path.join(out, f"{scenario}-{run}-mem.json")) as mem:
        memory = json.load(mem)
    started, samples = memory["started"], memory["samples"]
    facts, frames = read_record(record)
    reasons = unfit(facts)
    if memory.get("timed_out"):
        reasons.append("ended by SIGTERM after 400 s")
    if reasons:
        print(f"run {run}: left out: {'; '.join(reasons)}", file=sys.stderr)
    useful = next((i for i, f in enumerate(frames) if f["phase"] == "useful"), None)
    if useful is None:
        print(f"run {run}: no first useful frame (ended: {facts.get('ended')})", file=sys.stderr)
        return {"run": run, "failed": f"no first useful frame; ended: {facts.get('ended')}"}
    first = frames[useful]
    after = [s for s in samples if s["t"] >= first["start"]]
    at_useful = after[0] if after else {}
    following = frames[useful + 1]["start"] if useful + 1 < len(frames) else None
    return {
        "run": run,
        "failed": "; ".join(reasons) or None,
        "facts": facts,
        "start_ms": round((first["start"] + (first["cpu_ms"] or 0) / 1e3 - started) * 1e3, 1),
        "useful_cpu_ms": first["cpu_ms"],
        "after_useful_ms": round((following - first["start"]) * 1e3, 3) if following else None,
        "frames": len(frames),
        "found": phases(frames),
        "memory_mib": {
            "rss_at_useful": at_useful.get("VmRSS"),
            "anon_at_useful": at_useful.get("RssAnon"),
            "rss_highest": max((s.get("VmRSS", 0) for s in after), default=None),
            "anon_highest": max((s.get("RssAnon", 0) for s in after), default=None),
            "hwm": max((s.get("VmHWM", 0) for s in samples), default=None),
        },
    }


if not again:
    os.makedirs(out, exist_ok=True)
    if subprocess.run(["pgrep", "-ax", "opium"], stdout=subprocess.DEVNULL).returncode == 0:
        sys.exit("an Opium already runs: whose? (CONTRIBUTING.md, \"The owner's own window\")")
    if "WAYLAND_DISPLAY" not in os.environ:
        sys.exit("set WAYLAND_DISPLAY, for instance to the socket of weston.sh")
    for run in range(runs):
        measure(run)

results = [sum_up(run) for run in range(runs)]
every = {}
for result in results:
    found = result.pop("found", None)
    if found is None:
        continue
    result["phases"] = summary(found)
    if not result["failed"]:
        for name, phase in found.items():
            every.setdefault(name, {"cpu": [], "interval": []})
            every[name]["cpu"] += phase["cpu"]
            every[name]["interval"] += phase["interval"]
    print(json.dumps({k: result.get(k) for k in ("run", "failed", "start_ms", "useful_cpu_ms",
                                                 "after_useful_ms", "memory_mib")}))
    for name, phase in result["phases"].items():
        print(f"  {name}: {json.dumps(phase)}")

done = [r for r in results if not r["failed"]]
together = {
    "start_ms": spread([r["start_ms"] for r in done]),
    "after_useful_ms": spread([r["after_useful_ms"] for r in done if r["after_useful_ms"]]),
    "phases": summary(every),
    "rss_highest_mib": spread([r["memory_mib"]["rss_highest"] for r in done
                               if r["memory_mib"]["rss_highest"]]),
    "hwm_mib": spread([r["memory_mib"]["hwm"] for r in done if r["memory_mib"]["hwm"]]),
}
summed_path = os.path.join(out, f"{scenario}.json")
load = os.getloadavg()
if again:
    load = None
    if os.path.exists(summed_path):
        with open(summed_path) as earlier:
            load = json.load(earlier).get("load_average")
with open(summed_path, "w") as summed:
    json.dump({"scenario": scenario, "refresh_hz": refresh, "load_average": load,
               "runs": results, "together": together}, summed, indent=1)
print("together:", json.dumps(together))
