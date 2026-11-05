#!/usr/bin/env python3
"""Records the GPU's power and busy share at 10 Hz.

usage: scripts/measure/power.py <seconds> <out.json>

Reads power1_average (microwatts) and gpu_busy_percent of the first AMD GPU that exposes them under
/sys/class/drm/card*/device, which needs no root. GPU_DEVICE=/sys/class/drm/cardN/device picks
another. Prints the median, mean, 5th and 95th percentiles; the JSON keeps every sample. The
sensor's step is 1 W: compare runs with the app closed and open, in pairs, rather than one figure.
"""
import glob
import json
import os
import statistics
import sys
import time

secs, out = float(sys.argv[1]), sys.argv[2]


def find_device():
    if "GPU_DEVICE" in os.environ:
        return os.environ["GPU_DEVICE"]
    for device in sorted(glob.glob("/sys/class/drm/card*/device")):
        if glob.glob(f"{device}/hwmon/hwmon*/power1_average") and os.path.exists(
            f"{device}/gpu_busy_percent"
        ):
            return device
    sys.exit("no GPU exposing power1_average and gpu_busy_percent: set GPU_DEVICE")


device = find_device()
sensor = glob.glob(f"{device}/hwmon/hwmon*/power1_average")[0]
power, busy = [], []
start = time.monotonic()
i = 0
while time.monotonic() - start < secs:
    with open(sensor) as f:
        power.append(int(f.read()) / 1e6)
    with open(f"{device}/gpu_busy_percent") as f:
        busy.append(int(f.read()))
    i += 1
    time.sleep(max(0, start + i * 0.1 - time.monotonic()))
q = statistics.quantiles(power, n=20)
summary = {
    "device": device,
    "n": len(power),
    "power_w_median": statistics.median(power),
    "power_w_mean": statistics.mean(power),
    "power_w_p5": q[0],
    "power_w_p95": q[-1],
    "busy_mean": statistics.mean(busy),
}
with open(out, "w") as f:
    json.dump({**summary, "power": power, "busy": busy}, f)
print(" ".join(f"{k}={v:.2f}" if isinstance(v, float) else f"{k}={v}" for k, v in summary.items()))
