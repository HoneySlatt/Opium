#!/usr/bin/env python3
"""Samples a process's CPU time and wake-ups, per second and per thread.

usage: scripts/measure/sample.py <pid> <seconds> <out.json>

A wake-up is a context switch, voluntary or not, read from /proc/<pid>/task/*/status each second;
CPU time is utime + stime from /proc/<pid>/stat. The JSON keeps the per-second totals, the
switches of each thread name per second, each thread name's totals and the memory at the end; the
summary printed gives the rate, its median and extremes, and each waking thread. The quality bar's
"at rest" rows are measured with it (docs/ROADMAP.md).
"""
import json, os, sys, time

pid, secs, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
HZ = os.sysconf("SC_CLK_TCK")


def read_tasks():
    tasks = {}
    for tid in os.listdir(f"/proc/{pid}/task"):
        try:
            with open(f"/proc/{pid}/task/{tid}/status") as f:
                st = dict(l.split(":", 1) for l in f if ":" in l)
            with open(f"/proc/{pid}/task/{tid}/stat") as f:
                s = f.read()
            fields = s[s.rindex(")") + 2 :].split()
            cpu = int(fields[11]) + int(fields[12])  # utime + stime
        except (FileNotFoundError, ProcessLookupError):
            continue
        name = st["Name"].strip()
        if int(tid) == pid:
            name += " (main)"
        tasks[tid] = (
            name,
            int(st["voluntary_ctxt_switches"]),
            int(st["nonvoluntary_ctxt_switches"]),
            cpu,
        )
    return tasks


def proc_cpu():
    with open(f"/proc/{pid}/stat") as f:
        s = f.read()
    fields = s[s.rindex(")") + 2 :].split()
    return int(fields[11]) + int(fields[12])


def mem():
    with open(f"/proc/{pid}/status") as f:
        st = dict(l.split(":", 1) for l in f if ":" in l)
    return {k: st[k].strip() for k in ("VmRSS", "RssAnon", "RssFile", "RssShmem", "VmHWM", "Threads")}


prev = read_tasks()
cpu0 = proc_cpu()
t0 = time.monotonic()
per_second = []  # total switches each second
per_second_by_name = []  # {name: switches} each second
per_thread = {}  # tid -> [name, vol, nonvol, cpu ticks]
for i in range(secs):
    time.sleep(max(0, t0 + i + 1 - time.monotonic()))
    cur = read_tasks()
    total = 0
    sec = {}
    for tid, (name, v, nv, c) in cur.items():
        pv = prev.get(tid, (name, 0, 0, 0))  # a new thread counts from its start
        dv, dnv, dc = v - pv[1], nv - pv[2], c - pv[3]
        total += dv + dnv
        if dv + dnv:
            sec[name] = sec.get(name, 0) + dv + dnv
        acc = per_thread.setdefault(tid, [name, 0, 0, 0])
        acc[0] = name
        acc[1] += dv
        acc[2] += dnv
        acc[3] += dc
    per_second.append(total)
    per_second_by_name.append(sec)
    prev = cur
elapsed = time.monotonic() - t0
cpu1 = proc_cpu()
by_name = {}
for tid, (name, v, nv, c) in per_thread.items():
    e = by_name.setdefault(name, {"threads": 0, "vol": 0, "nonvol": 0, "cpu_ticks": 0})
    e["threads"] += 1
    e["vol"] += v
    e["nonvol"] += nv
    e["cpu_ticks"] += c
res = {
    "pid": pid,
    "seconds": elapsed,
    "cpu_ticks": cpu1 - cpu0,
    "cpu_ms": (cpu1 - cpu0) * 1000 / HZ,
    "cpu_percent": (cpu1 - cpu0) / HZ / elapsed * 100,
    "switches_total": sum(per_second),
    "switches_per_s": per_second,
    "per_second_by_name": per_second_by_name,
    "by_name": dict(sorted(by_name.items(), key=lambda kv: -(kv[1]["vol"] + kv[1]["nonvol"]))),
    "mem_end": mem(),
}
json.dump(res, open(out, "w"), indent=1)
ps = sorted(per_second)
print(f"cpu {res['cpu_ms']:.0f} ms over {elapsed:.1f}s ({res['cpu_percent']:.3f}%), switches {res['switches_total']} "
      f"= {res['switches_total']/elapsed:.2f}/s, median/s {ps[len(ps)//2]}, min {ps[0]}, max {ps[-1]}")
for n, e in res["by_name"].items():
    w = e["vol"] + e["nonvol"]
    if w:
        col = sorted(sec.get(n, 0) for sec in per_second_by_name)
        print(f"  {n:22s} x{e['threads']:<3d} {w/elapsed:7.2f}/s  median {col[len(col)//2]}/s max {col[-1]}  (vol {e['vol']}, nonvol {e['nonvol']}, cpu {e['cpu_ticks']} ticks)")
print(" ", res["mem_end"])
