#!/usr/bin/env python3
"""Records a process's resident memory every 0.25 s.

usage: scripts/measure/memwatch.py <pid> <seconds> <out.json>

Each row is [seconds since start, VmRSS, RssAnon, RssFile, VmHWM], in MiB, from /proc/<pid>/status.
It stops early if the process ends, and prints the last row, the peak and a dozen rows spread over
the run.
"""
import json
import sys
import time

pid, secs, out = int(sys.argv[1]), float(sys.argv[2]), sys.argv[3]
KEYS = ("VmRSS", "RssAnon", "RssFile", "VmHWM")
start = time.monotonic()
rows = []
while time.monotonic() - start < secs:
    try:
        with open(f"/proc/{pid}/status") as f:
            status = dict(line.split(":", 1) for line in f if ":" in line)
    except FileNotFoundError:
        break
    elapsed = round(time.monotonic() - start, 2)
    rows.append([elapsed] + [int(status[k].split()[0]) // 1024 for k in KEYS])
    time.sleep(0.25)
if not rows:
    sys.exit(f"process {pid} not found")
with open(out, "w") as f:
    json.dump(rows, f)
last = rows[-1]
print(f"MiB at end: rss {last[1]} anon {last[2]} file {last[3]}; "
      f"peak rss {max(r[1] for r in rows)}, VmHWM {last[4]}")
for row in rows[:: max(1, len(rows) // 12)]:
    print("  ", row)
