# Measurements

What the benchmarks and the measurements by hand found, against the budgets of the quality bar in
[`ROADMAP.md`](ROADMAP.md#quality-bar). One section per milestone or machine, newest first. A
section names the machine, the build, the command, and the figures with their spread.

Unless a section says otherwise, the builds measured are release builds (`nix build` or
`cargo build --release -p app-desktop` on the desktop, `gradle assembleRelease` on the phone), and
the desktop runs on a [throwaway profile](../CONTRIBUTING.md#a-throwaway-profile). The scripts
cited live in [`scripts/measure/`](../scripts/measure/), each described at its head
([CONTRIBUTING.md](../CONTRIBUTING.md#measuring)).

## CI on GitHub (2026-11-05)

Runner: GitHub's `ubuntu-24.04`. Workflow: [`ci.yml`](../.github/workflows/ci.yml) at commit
`7d4336a`, run `41950023132`, with the disk freed and the tests built without debug info.

| Run | Cache | Duration | Cache saved |
|---|---|---|---|
| Cold | None | 24 min 3 s | 1.1 GB |
| Warm | Hit | Not measured yet | — |

The run before (`41929905546`, without these changes) filled the runner's disk after 31 minutes.

## Baseline of the desktop app, before any budget (2026-11-05)

Machine: AMD Ryzen 7 5800X (8 cores, 16 threads); AMD Radeon RX 6800/6800 XT/6900 XT (Navi 21,
`1002:73bf`), Mesa 26.2.5 radeonsi (the app renders with OpenGL through EGL: `libgallium`,
`libEGL_mesa` are mapped); Linux 7.3.0; compositor umbriel. Displays: DP-2 ASUS PG32UCDM
3840x2160 at 240 Hz, scale 1.5 (2560x1440 logical); DP-3 MSI G244F 1920x1080 at 180 Hz, scale 1.0.
Build: release, commit `7a749d0` (`nix develop -c cargo build --release -p app-desktop`, clean tree),
binary `target/release/opium`. Library: 8553 songs (the owner's, from the cache).

Conditions: a copy of the owner's config (without `connect.json`, `muted = true`) and cache, an
empty data folder, the session bus pointed nowhere (`DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent`,
so no MPRIS, no keyring, no single-instance check), the network on (so Opium Connect ran, as a new
unpaired device), `WIDEVINE_CDM` set. Nothing was played. The owner's own Opium was not running.
During most runs a game and other agents' Rust builds were running (load average 14 to 25); the
start-up and power runs were made after they stopped (load average about 2).

Case 1 = no session shown (`session.json` removed and `shared.session` set to null in the copy);
case 2 = the last song shown, paused (the normal state after launch).

| Budget | Default | Measured | |
|---|---|---|---|
| Wake-ups at rest, interface threads (main, Mesa, clipboard) | 0/s | 0/s in every run (12 runs, 60 s or 30 s each) | met |
| Wake-ups at rest, whole process | 0/s | mean 8.4/s (runs 7.1 to 9.9, n=12 runs); per second median 5, p95 19, max 101 (n=630 s) | missed |
| CPU at rest | none | 0.67 ms/s median (0.5 to 1.2 ms/s, n=12 runs), 0.05 to 0.12 % of one core | |
| Warm start, exec to window mapped | 150 ms | median 172.6 ms, min 168.9, max 356.9 (n=10) | missed |
| Warm start, exec to first frame read back | 150 ms | median 223.6 ms, min 216.6, max 251.1 (n=8), upper bound | missed |
| Memory at rest after start (VmRSS) | none | 339 to 371 MiB (RssAnon 208 to 241, RssFile 124 to 140), n=12 runs | |
| Memory, Songs / Albums open | none | Songs 361 to 376 MiB; Albums 346 to 352 MiB; highest VmHWM seen 383 MiB | |
| GPU power at rest, open minus closed | none | +0 W: medians closed 36 / 35 / 38 W, open 37 / 35 / 35 W (3 pairs of 30 s at 10 Hz); noise about ±2 W | met |
| Frames drawn at rest | 0 | main thread 0 wake-ups at rest, so 0 frames (indirect) | met |
| Animation and scroll frame times | display rate, p99 | not measurable without code or input injection | not measured |

### 1. At rest: CPU time and wake-ups

Method: `scripts/measure/sample.py <pid> 60` after 30 s to settle: the per-second delta of
`voluntary_ctxt_switches + nonvoluntary_ctxt_switches` summed over `/proc/<pid>/task/*/status`,
kept per thread name (the main thread apart), and `utime + stime` from `/proc/<pid>/stat` over the
run; `top -b -d 60 -n 2 -p <pid>` alongside agreed (0.0 to 0.1 %).

| State | Runs | Mean wake-ups/s (per run) | Per-second median / p95 / max | CPU ms/s |
|---|---|---|---|---|
| Hidden or covered, real compositor (minimized; offscreen column; floating under a maximized window) | 6 | 8.45 (7.35 to 9.93) | 5 / 19 / 101 | 0.67 (0.50 to 1.00) |
| Visible, unfocused, headless Weston 2560x1440 at 240 Hz, scale 1 | 3 | 8.38 (8.03 to 9.48) | 5 / 38 / 79 | 1.00 (0.83 to 1.17) |
| Visible and focused, real compositor, DP-3 (180 Hz, scale 1), 30 s runs | 3 | 7.46 (7.07 to 7.80) | 5 / 14 / 56 | 1.00 (0.67 to 1.00) |
| Case 1 only (no session shown) | 7 | 8.38 (7.35 to 9.48) | 5 / 19 / 87 | 0.67 |
| Case 2 only (song shown, paused) | 2 | 8.98 (8.03 to 9.93) | 5 / 38 / 101 | 0.92 |

Case 2 is the same as case 1: the shown song has no sink until it is played, so the player thread
keeps its 1 s timer.

Wake-ups per thread at rest (median of the runs, all states alike):

| Thread | Wake-ups/s | Cause |
|---|---|---|
| `connect` (Opium Connect's tokio runtime) | 3.8 (3.0 to 5.5); bursts of 40 to 100 in one second every 15 to 20 s | LAN, iroh and redial timers (`REDIAL` 5 s, `PING` 15 s) |
| `blocking-1` | 2.00, exactly | the `blocking` crate 1.7.0 (through zbus and async-fs): its idle thread waits 500 ms and no longer exits under the thread limit, so it loops forever. 1.6.2 let it exit. |
| `theme-watch` | 1.00, exactly | `app.rs` `watch_theme`: polls the theme files' times every second |
| `player` | 1.00, exactly | `player/src/engine.rs` `idle()`: 1 s `recv_timeout` with no sink; 250 ms (4/s) once a song is paused with a sink (read in the code, not measured: it needs playing) |
| `mDNS_daemon` | 0.3 (0.2 to 0.85) | mdns-sd's daemon, a 5 s tick plus answers |
| `tokio-rt-worker` x16, `connect-mdns` | under 0.1 | |
| main thread, Mesa threads (`opium:gl0`, `cs0`, `sh*`, `disk$*`), `smithay-clipboard`, `backend`, `synced-save`, `notify-rs` | 0 | |

The floor in a quiet second is 4/s (blocking-1 2, theme-watch 1, player 1); Connect and mDNS add
the bursts.

Focus: a focused-and-idle run was made only on DP-3 (3 runs of 30 s, during the power pairs); the
first attempts on DP-2 were disturbed by the owner's pointer (bursts of 400 to 1500 wake-ups/s on the
main and `smithay-clipboard` threads, which wake on each Wayland event). With no input, focused and
unfocused do not differ.

Network cut (`unshare --user --map-current-user --net`, 1 run, covered window): 41.5/s and
1.3 % CPU; the `connect` thread alone 37/s, in bursts of about 700 wake-ups over 4 s every 20 s.

### 2. Paused with a song shown

Identical to case 1 (table above): 8.98/s against 8.38/s, within the spread set by Connect's
bursts; no thread wakes more.

### 3. Warm start-up

Method: `scripts/measure/startup.py`: `umbriel subscribe windows` is opened first; the time is
taken just before `run.sh` starts (bash, then `env`, then the binary), and again when the
compositor's event stream first lists a window of app id `opium`. A second series sets
`OPIUM_SCREENSHOT_AFTER=0` and records when "screenshot saved" appears on stderr. These runs closed
the window through a compositor command; that can reach the owner's focused window, so the script
kept now ends each run by SIGTERM to its own pid, and the measurements run in a headless Weston
(`scripts/measure/weston.sh`) where they can. Profile of case 2; window on DP-2 (240 Hz, scale 1.5) as a new column. Load average about 2.

- Exec to window mapped (the compositor has the first committed buffer): median 172.6 ms, min 168.9,
  max 356.9, n=10 (values 169, 171, 171, 171, 172, 173, 174, 175, 195, 357). Includes bash and
  `env` start (a few ms), the single-instance check failing fast on the dead bus, eframe, EGL and
  Mesa's start, the app's construction, the first frame, and the event's trip through the
  compositor.
- Exec to first frame saved: median 223.6 ms, min 216.6, max 251.1, n=8. An upper bound of the first
  frame: it adds egui's read-back and the PNG encoding. That first frame is not yet the Songs page:
  the page opens only once the playlists are loaded; "Songs page drawn" was not timed.
- Close to exit (not asked, seen with the compositor's close): about 1.08 s to the process's end
  (exec to exit 1252.9 ms median minus 172.6 ms to map; n=10, spread 1249 to 1448 ms).
- Cold start: not measured; dropping the page cache needs root.

### 4. Memory

Method: `scripts/measure/memwatch.py`: `VmRSS`, `RssAnon`, `RssFile`, `VmHWM` from `/proc/<pid>/status`
every 0.25 s for 95 s from launch. Pages through the screenshot mode with `OPIUM_SCREENSHOT_AFTER=100000`
(the run stays open), on a headless Weston where the window is always shown (window 1440x900,
scale 1; Songs and Albums checked on a capture, then deleted).

| State | VmRSS MiB | RssAnon | RssFile | Runs |
|---|---|---|---|---|
| Home at rest, 30 to 90 s after start | 339 to 371 | 208 to 241 | 124 to 140 | 12 |
| Songs page open (8553 songs) | 361 to 376 | 230 to 249 | 129 to 134 | 3 |
| Albums grid open | 346 to 352 | 215 to 222 | 129 | 2 |
| Highest VmHWM seen (after pointer use, Home) | 383 | | | |

RSS reaches about 370 MiB within 8 s of the start and stays flat. The Albums grid at scale 1 in a
1440x900 window shows few covers; at the owner's scale 1.5 and full screen it would load more:
not measured. Scrolling (the heaviest realistic load) was not measured: no input injection.
`heaptrack` was not run.

### 5. GPU power at rest

Method: `scripts/measure/power.py`: `power1_average` of `card1` (hwmon2) and `gpu_busy_percent`, at 10 Hz
for 30 s; three pairs, app closed then open at rest (case 1, floating and focused on DP-3, after
30 s), load average about 2.

| Pair | Closed median / mean W | Open median / mean W | Busy % mean closed / open |
|---|---|---|---|
| 1 | 36 / 36.4 | 37 / 37.4 | 1.8 / 3.2 |
| 2 | 35 / 35.1 | 35 / 36.0 | 0.3 / 1.6 |
| 3 | 38 / 38.6 | 35 / 34.6 | 4.6 / 0.3 |

Difference: 0 W within a noise of about ±2 W (the sensor's step is 1 W; the desktop's other
windows move the closed baseline by 3 W). A first baseline taken while the game ran read
89.5 W median (35 to 169 W, p5 to p95): unusable, discarded.

### 6. Frame times of scrolling and animations

Not measured.

- The app renders through OpenGL (EGL + radeonsi), so Mesa's `GALLIUM_HUD` applies:
  `GALLIUM_HUD=fps,frametime GALLIUM_HUD_PERIOD=0 GALLIUM_HUD_DUMP_DIR=<dir> GALLIUM_HUD_VISIBLE=false`
  should write one value per frame. One try produced empty files, but in a run whose window got no
  frame callbacks; it was not repeated. `frametime` there is the time between presents, not the
  frame's cost.
- Without input injection, nothing scrolls, and no animation runs at rest. The only animations
  that run without input are the loading spinner (fixed at 30 Hz, `ui/loading.rs`) and the moving
  covers (at the video's own rate).
- A proxy: with the screenshot mode's 200 ms repaint (5 frames/s at most), the main thread spent
  about 840 to 970 ms per minute on the Songs page and about 270 ms on the Albums grid: roughly
  3 ms of CPU per frame on Songs and 1 ms on Albums (the frame count is inferred, not counted).
  A 240 Hz frame is 4.17 ms.
- What would be needed: frame timing recorded in the app (eframe's `Frame::info().cpu_usage` and
  the present times, written as a trace), and a scripted scroll or animation in the screenshot
  mode (an `OPIUM_SCREENSHOT_SCROLL`-like variable), or an input-injection protocol in the
  compositor. The egui window being frozen, these come with slime-ui's frame statistics and MCP
  server instead ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)).

### Not measured, and why

- Cold start: needs root to drop caches.
- Scrolling: frame times, memory and power; no input injection and no frame statistics.
- Animation frame times (springs, transitions): same.
- Paused with a sink (after a song was played): needs playing; the code says 4/s more on `player`.
- With the session bus (MPRIS, keyring, AT-SPI): the bus was pointed nowhere, as CONTRIBUTING.md
  asks for a copied profile; the zbus threads' rest behavior is not in these figures.
- With pairing (the owner's `connect.json`): forbidden; Connect ran as an unpaired new device.
- At scale 1.5 with the window visible on DP-2: a test window could not be shown there without
  moving the owner's windows; the visible runs are on DP-3 (scale 1), placed through compositor
  commands, and on a headless Weston (scale 1). The scripts kept use the headless Weston only.

Runs counted: six covered or minimized runs, three on the headless Weston, three focused (during the
power pairs). Left out: runs disturbed by the owner's pointer, runs whose screenshot variables were
mis-passed, and runs where the screenshot mode's repaints or Weston's idle blanking changed what was
measured. The raw files were not kept in the repository; the scripts that made them are in
`scripts/measure/`.

## The phone, 2026-11-03

Phone: the owner's Pixel on GrapheneOS, at 120 Hz; its exact model was not recorded. Build: a release
APK built before the rename (the app then named Music), signed with the debug key; its exact commit
was not recorded. These figures came before the quality bar and before
[ADR-0025](adr/0025-the-quality-bar.md); they were kept in the owner's notes and are copied here.

| Measure | Method | Figure |
|---|---|---|
| Cold start | `adb shell am start -W` (`TotalTime`) | 600 to 750 ms, about 300 ms of it the app's own; the rest is GrapheneOS starting the process |
| Playing, screen off, a Navidrome or local song | CPU ticks over 60 s from `/proc/<pid>/stat`, over adb | about 1,500 to 1,900 ticks a minute through MediaCodec; 130 to 340 after the audio chip's offload (`d8055e7`) |
| Playing, screen off, Apple Music (Widevine) | As above | 1,500 to 1,900 ticks a minute: protected streams cannot be offloaded |
| Playing Dolby Atmos | As above | about 2,800 ticks a minute; since `1c51392`, Atmos plays only on headphones |
| Spotify, lossless, for comparison | As above | 844 ticks a minute |
| Frame statistics | `adb shell dumpsys gfxinfo` | used while tuning; no figure kept |

Known: GrapheneOS compiles apps fully ahead of time (`dexopt status=speed`), so baseline profiles
bring nothing there. Asynchronous or synchronous MediaCodec, dynamic scheduling and float output were
each tried and reverted: none explained the decoding's cost. Not known: the songs' exact formats,
the screen's state for the start-up runs, and the runs' spread.

<!-- ## M1.7 — <title> (YYYY-MM-DD)

Machine: <CPU, GPU, display>. Build: <profile, commit>. Command: `<command>`.

| Budget | Target | Measured | |
|---|---|---|---|
| <name> | <target> | <median, p99, spread> | met / missed | -->
