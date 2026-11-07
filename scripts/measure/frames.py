#!/usr/bin/env python3
"""Runs one of the window's scripted scenarios and sums up its frames, its start and its memory.

usage: scripts/measure/frames.py <profile-dir> <scenario> <runs> <out-dir> [<refresh in Hz>]

<scenario> is one of `crates/app-desktop/src/measure.rs`'s: start, scroll-songs, scroll-albums,
animate. Each run starts the release build through run.sh on $WAYLAND_DISPLAY, cut off the network
(`unshare --user --map-current-user --net`), with OPIUM_MEASURE set; the app writes its frame
record and closes itself. Meanwhile its memory is read from /proc every 0.25 s. A run that has
not ended after 400 s is ended by SIGTERM to its own pid, and counts as failed.

For every run, in <out-dir>: the record (<scenario>-<n>.csv), the memory (<scenario>-<n>-mem.json)
and the app's standard error (<scenario>-<n>.log); then <scenario>.json, the figures of every run
and of all runs together:
- start_ms: from just before the program is started to the end of the first useful frame's work
  (its start plus its CPU time). The library comes from its index, so a profile's first run, which
  scans the folder, is not a warm start: run a `start` first and drop it.
- for each phase of the scenario, over the frames the frame before asked for at once (the record's
  `continued`): their CPU time (eframe's: interface, tessellation and drawing calls) and their
  interval, the time from the frame before's start, which is what the display shows; the interval
  is held when it is one period of <refresh in Hz> (240 by default); a frame later than 1.5
  periods missed at least one;
- memory: VmRSS and RssAnon at the first useful frame, their highest during the scenario, and
  VmHWM, the highest VmRSS the kernel saw.
- the window's size in pixels, from the record: with OPIUM_MEASURE_PIXELS set (3840x2160), a run
  whose window had another size is said on standard error.
Refuses to start if an Opium already runs. Run inside `nix develop`.
"""
import json
import os
import signal
import statistics
import subprocess
import sys
import threading
import time

profile, scenario, runs, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
refresh = float(sys.argv[5]) if len(sys.argv) > 5 else 240.0
period_ms = 1000.0 / refresh
pixels = os.environ.get("OPIUM_MEASURE_PIXELS")
run_sh = os.path.join(os.path.dirname(os.path.abspath(__file__)), "run.sh")
os.makedirs(out, exist_ok=True)
if subprocess.run(["pgrep", "-ax", "opium"], stdout=subprocess.DEVNULL).returncode == 0:
    sys.exit("an Opium already runs: whose? (CONTRIBUTING.md, \"The owner's own window\")")
if "WAYLAND_DISPLAY" not in os.environ:
    sys.exit("set WAYLAND_DISPLAY, for instance to the socket of weston.sh")


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
    """Each phase's frames that the frame before asked for at once: CPU times and intervals."""
    found = {}
    for before, frame in zip(frames, frames[1:]):
        if not frame["continued"] or frame["phase"] in ("loading", "useful", "end"):
            continue
        phase = found.setdefault(frame["phase"], {"cpu": [], "interval": []})
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


results, every = [], {}
for run in range(runs):
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
        try:
            app.wait(timeout=400)
        except subprocess.TimeoutExpired:
            app.send_signal(signal.SIGTERM)  # its own pid: unshare and run.sh exec the app
            app.wait()
        stop.set()
        watcher.join()
    with open(os.path.join(out, f"{scenario}-{run}-mem.json"), "w") as mem:
        json.dump(samples, mem)
    if not os.path.exists(record):
        print(f"run {run}: no record (see {scenario}-{run}.log)", file=sys.stderr)
        results.append({"run": run, "failed": True})
        continue
    facts, frames = read_record(record)
    useful = next(f for f in frames if f["phase"] == "useful")
    after = [s for s in samples if s["t"] >= useful["start"]]
    at_useful = after[0] if after else {}
    found = phases(frames)
    if pixels and facts.get("window_pixels") != pixels:
        print(f"run {run}: window {facts.get('window_pixels')}, not {pixels}", file=sys.stderr)
    for name, phase in found.items():
        every.setdefault(name, {"cpu": [], "interval": []})
        every[name]["cpu"] += phase["cpu"]
        every[name]["interval"] += phase["interval"]
    result = {
        "run": run,
        "facts": facts,
        "start_ms": round((useful["start"] + (useful["cpu_ms"] or 0) / 1e3 - started) * 1e3, 1),
        "useful_cpu_ms": useful["cpu_ms"],
        "frames": len(frames),
        "phases": summary(found),
        "memory_mib": {
            "rss_at_useful": at_useful.get("VmRSS"),
            "anon_at_useful": at_useful.get("RssAnon"),
            "rss_highest": max((s.get("VmRSS", 0) for s in after), default=None),
            "anon_highest": max((s.get("RssAnon", 0) for s in after), default=None),
            "hwm": max((s.get("VmHWM", 0) for s in samples), default=None),
        },
    }
    results.append(result)
    print(json.dumps({k: result[k] for k in ("run", "start_ms", "useful_cpu_ms", "memory_mib")}))
    for name, phase in result["phases"].items():
        print(f"  {name}: {json.dumps(phase)}")

done = [r for r in results if not r.get("failed")]
together = {
    "start_ms": spread([r["start_ms"] for r in done]),
    "phases": summary(every),
    "rss_highest_mib": spread([r["memory_mib"]["rss_highest"] for r in done if r["memory_mib"]["rss_highest"]]),
}
with open(os.path.join(out, f"{scenario}.json"), "w") as summed:
    json.dump({"scenario": scenario, "refresh_hz": refresh, "load_average": os.getloadavg(),
               "runs": results, "together": together}, summed, indent=1)
print("together:", json.dumps(together))
