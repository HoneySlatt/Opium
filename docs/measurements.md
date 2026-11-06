# Measurements

What the benchmarks and the measurements by hand found, against the budgets of the quality bar in
[`ROADMAP.md`](ROADMAP.md#quality-bar). One section per milestone or machine, newest first. A
section names the machine, the build, the command, and the figures with their spread.

Unless a section says otherwise, the builds measured are release builds (`nix build` or
`cargo build --release -p app-desktop` on the desktop, `gradle assembleRelease` on the phone), and
the desktop runs on a [throwaway profile](../CONTRIBUTING.md#a-throwaway-profile). The scripts
cited live in [`scripts/measure/`](../scripts/measure/), each described at its head
([CONTRIBUTING.md](../CONTRIBUTING.md#measuring)).

## Apple Music's steps, M1.12 (2026-11-05 and 2026-11-06)

How long Apple Music's steps take Opium on the owner's account, before and after
`crates/apple-music`'s derived items were written again
([M1.12](PLAN.md#m110-to-m112--kopuzs-code-written-again)). Machine: AMD Ryzen 7 5800X, Linux
7.3.0, the owner's network. Command: `scripts/measure/cdm-cost.sh`, which runs release builds of
[`am_cost.rs`](../crates/apple-music/examples/am_cost.rs), with the owner's go: one licence and one
station batch on the owner's account a run, nothing played, nothing written to disk. Song
1463409605 and its station; a library of 8 499 songs.

- *Before, first*: five runs of the code at `c132830`, ten seconds apart, at a load average of 8.8
  rising to 16.6 (another build ran meanwhile).
- *Before* and *after*, on 2026-11-06: five rounds, each running the old code (`c132830`) then the
  new one (`316a8ee`, the clean side's first round), the same program, at a load average of 0.9 to
  0.6. Its second and third rounds, after the reviews, changed none of these requests for Apple's
  real answers: the search of the page and the script, measured apart below, an id's check, and
  how often the token is read again near its `exp`, which none of these runs came near.

| What | Before, first (5 runs) | Before (5 rounds), median and range | After (5 rounds), median and range |
|---|---|---|---|
| The developer token, on a client of its own | 507 to 791 ms | 506 ms, 495 to 529 | 534 ms, 508 to 646 |
| Connecting: the token and the storefront | 772 to 886 ms | 780 ms, 728 to 830 | 756 ms, 720 to 891 |
| The library's first page of 100 songs | 1 266 to 1 641 ms | 1 601 ms, 1 079 to 1 766 | 1 096 ms, 1 070 to 1 689 |
| Every song of the library | 7.3 to 12.2 s | 11.2 s, 7.6 to 31.6 | 10.3 s, 7.3 to 11.9 |
| A station's batch of 20 | 1 162 to 2 129 ms | 2 252 ms, 1 525 to 2 608 | **1 278 ms**, 1 065 to 1 395 |
| The song ready to play (webPlayback to the first samples decrypted) | 1 035 to 1 459 ms | 1 065 ms, 1 009 to 1 257 | 1 027 ms, 987 to 1 211 |
| Its first MiB of cleartext | `d467d7d1094aa53a` every run | the same | the same |

- **A station's batch is 43 % quicker**: up to six calls in flight, as many as the songs still
  missing need, instead of rounds of four that each waited for their slowest call. Every batch
  held 20 distinct songs on both sides.
- **The developer token**, alone, ten times each, alternately, on 2026-11-06, the clean side's
  first round against `c132830`
  ([`token-time.sh`](audits/2026-11-06-apple-music-clean-room/token-time.sh); Apple's public
  page, no account), at a load
  average of 17 falling to 6.5: median before 487.8 ms (460 to 626), after 494.4 ms (460 to 616). The
  28 ms of the rounds above are the network's: the two sides' spreads overlap, and the search
  itself costs microseconds.
- The library, its first page and a song's start go through the same requests as before; their
  differences are the network's.
- **Searching the page and the script for the token**, apart, on Apple's page (2.4 MB) and script
  (3.3 MB) of 2026-11-05, saved outside the repository, release builds, by
  [`token-search.sh`](audits/2026-11-06-apple-music-clean-room/token-search.sh), alternately:
  - round 1's code, which read the page's `<script>` tags: a process's first search the same as
    before (459 and 533 µs before, 459 and 521 µs after, medians of 30), and once warm 4.5 µs more
    (15.0 → 19.5 and 16.6 → 21.1 µs, p95 16.3 → 21.1 and 17.1 → 21.5 µs); accepted by the owner;
  - round 2's code, which found the address standing alone anywhere in the page: a process's
    first search 459 to 470 µs before, 280 to 311 µs after; once warm 1.0 to 1.7 µs more in
    median (p95 up to 2.4 µs more).
  - **The final code**, round 3's, which finds the address after whatever precedes it, its name
    with dots: three runs at a load average of 7 to 8 (other work on the machine). A process's
    first search: 577 to 588 µs before, 583 to 597 µs after (medians of 30; p95 683 to 759 µs
    before, 683 to 772 µs after): **2 to 12 µs more**, the wider expression's building. Once
    warm, medians of 101: 16.3 → 17.6, 17.1 → 18.3 and 17.3 → 18.5 µs, **1.2 to 1.3 µs more**,
    the token's header decoded to check that it is ES256; p95 34.4 → 36.5, 21.7 → 20.4 and
    23.8 → 24.9 µs, noisy at that load. Within the 4.5 µs the owner accepted. In round 2's runs,
    warm times fell into two groups, 15 or 28 µs, both sides alike, from one run to the next:
    where the process ran, not the code; round 3's runs showed the lower group only.
  - The figure first given, 0.08 ms for the code before, built its expressions in every one of 30
    searches in a row.

## The CDM, M1.11 (2026-11-05)

What the browser's CDM costs Opium, before and after `crates/widevine` was written again
([M1.11](PLAN.md#m110-to-m112--kopuzs-code-written-again)); what is measured is set in
[`specs/widevine-host.md`](specs/widevine-host.md#11-what-is-measured) §11. Machine: AMD Ryzen 7
5800X, Linux 7.3.0. CDM: Widevine 4.10.3112.0, from Brave's profile. Command:
`scripts/measure/cdm-cost.sh`, which runs release builds of
[`cdm_cost.rs`](../crates/apple-music/examples/cdm_cost.rs), with the owner's go: one licence on
the owner's account a run, nothing played, nothing written to disk. Song 1463409605, 7.18 MB,
9 057 samples, 210.3 s of AAC.

- *Before, first*: five runs of the code at `133176e`, load average 2.6 to 3.6.
- *Before* and *after*: five rounds, each running the old code (built from `fce9347`) then the new
  one (`380bebc`), the same program but for its `cdm` module. The first round ran at a load
  average of 18 (another build), and is left out on both sides below: before, a session to a
  usable key in 1 084 ms and a decrypt's median 60.9 µs; after, 612 ms and 52.2 µs (p95 155 µs,
  p99 3.0 ms).

| What | Before, first (5 runs) | Before (rounds 2 to 5) | After (rounds 2 to 5) |
|---|---|---|---|
| Loading and initialising the CDM | 7.9 ms on the first run, 0.9 to 1.0 ms after | 0.9 to 1.2 ms | 1.7 to 1.9 ms |
| A session to a usable key | 573 to 612 ms | 573 to 589 ms | 566 to 584 ms |
| … of which the CDM: opening the session | 74 to 82 ms | 74 to 80 ms | 75 to 78 ms |
| … of which the licence server | 195 to 231 ms | 200 to 254 ms | 191 to 310 ms |
| … of which the CDM: taking the licence | 270 to 336 ms | 247 to 309 ms | 185 to 304 ms |
| A decrypt in place: median | 33 to 42 µs | 31.9 to 32.9 µs | 41.0 to 41.6 µs |
| A decrypt in place: p95, p99 | 62 to 93 µs, 0.89 to 1.6 ms | 47 to 56 µs, 0.99 to 1.08 ms | 71 to 82 µs, 0.98 to 1.02 ms |
| A decrypt in place: max | 196 to 197 ms | 197 ms | 196 ms |
| The whole song decrypted | 14.20 s every run | 14.20 s | 14.20 s |
| The cleartext's digest | `680d56e025ce09e0` | the same | the same |
| Wake-ups at rest, a session open, but the measuring thread's | 0 a second | 0 a second | 1.00 to 1.07 a second, all on the `cdm` thread |
| The session closed and the CDM shut down, after 30 s at rest | not measured | 0.16 to 0.25 ms | 0.30 to 0.34 ms |

- **The CDM paces its decrypts.** Most calls take tens of microseconds, but a few wait inside the
  CDM up to 197 ms, so that a song always takes the same time: 14.20 s for this one, about
  505 KB/s or 15 times real time. The mean, 1.57 ms a sample, is the pace, not the work; the old
  figure "1.5 ms a sample, 11.6 times real time" was this mean. The host cannot change that pace.
- **A decrypt's median grows by 8.8 µs** (32.5 to 41.3 µs, the rounds' means), the round trip to
  the `cdm` thread: within the budget of 15 µs the owner set. With the fake CDM, the round trip
  adds 6.5 to 7.1 µs.
- **A decrypt's p95 grows by about 25 µs** (47–56 to 71–82 µs): waking the `cdm` thread sometimes
  costs more than its median. The p99, the max and the whole song are unchanged. The owner
  accepted it.
- **Loading grows by about 0.8 ms**, once a process, on the backend's thread: the `cdm` thread
  started, and the calls sent to it. The owner accepted it, and the teardown after a while at rest
  growing from 0.2 to 0.3 ms.
- **One wake-up a second at rest, a session open**: the CDM sets a timer of about a second while a
  session is open (48 in a run), which the old host never fired and the new one fires in time, as
  ADR-0030 asks. The owner accepted it. With no session open, nothing wakes.
- **Quitting**: right after the CDM is loaded, its teardown takes 0.44 to 0.85 s (the probe), its
  module waiting for a thread of its own; after a while at rest, 0.3 ms. The window is gone by then.
  The owner chose to keep the teardown whole.
- A licence asked while a track decrypts waits behind one decrypt call: up to about 200 ms when
  that call is one the CDM paces.
- A licence's figures are the same within their noise; the CDM's share of a licence varies from
  run to run on both sides.

## Indexing an encrypted track, M1.10 (2026-11-05)

What indexing an Apple-shaped file costs, before and after `crates/cenc` was written again
([M1.10](PLAN.md#m110-to-m112--kopuzs-code-written-again)); the budget is in
[`specs/fmp4-cenc.md`](specs/fmp4-cenc.md#10-performance) §10. Machine: AMD Ryzen 7 5800X, Linux
7.3.0, load average 0.8 to 4. Command: `cargo run --release -p cenc --example index_cost
[fragments]` ([`index_cost.rs`](../crates/cenc/examples/index_cost.rs)), inside `nix develop`.

The file is synthetic, built by the program: 7.39 MB, 9 000 samples of 700 to 900 bytes in 100
fragments, scheme `cenc`, 8-byte IVs, one subsample each; with 6 000 fragments, four hours. *Kept*
is the heap the result of indexing holds while it lives, counted by a counting allocator:
capacity, not length, so a `Vec` grown by doubling counts whole. Before and after were run
alternately, five times each against each round, the old code built from commit `39bb0fa` beside
the new.

| What | Before (`fmp4.rs`) | After | Budget |
|---|---|---|---|
| Indexing the whole file, best of 50 | 0.445 to 0.461 ms (medians 0.459 to 0.474) | 0.317 to 0.335 ms (medians 0.330 to 0.343) | not worse: met |
| The table kept | 1 222.3 KiB in 9 002 blocks: 139.1 bytes a sample | 406.3 KiB in 11 blocks: 46.2 bytes a sample | at most 48 bytes a sample: met |
| A download, 64 KiB at a time, made clear at each new fragment | not measured (another interface) | 0.77 to 0.79 ms in all | none |
| Four hours (443.5 MB, 540 000 samples): indexing, best of 5 | not measured | 25.8 ms | in proportion to the size |
| Four hours: the table kept | not measured | 23.8 MiB, 46.2 bytes a sample | under 26 MB: met |
| Four hours: a download | not measured | 763 ms in all | none |

- Indexing four hours takes 80 times as long as four minutes, for 60 times the samples: near
  proportion; the 443 MB file no longer fits any cache.
- A download costs more than indexing whole because the file is made clear again at each new
  fragment, over every fragment indexed so far: the time grows with the square of the fragments
  ([debt](debt.md)). For a four-minute song, 0.45 ms in all.
- The licence audit's figures, 0.37 ms and 632 KiB plus 140 KiB in 9 000 allocations
  ([audit §5](audits/2026-11-05-licence.md#5-measured)), came from a throwaway program on a 7.21 MB
  file and counted the table by its length (9 000 samples of 72 bytes); this kept program counts
  what the heap really holds.
- The first clean-room version indexed in 0.52 to 0.54 ms, slower than before; a second round made
  it 0.32 ms, and the third, which bounds the samples by the file's bytes, kept it so
  ([evidence](audits/2026-11-05-cenc-clean-room.md)).

## CI on GitHub (2026-11-05)

Runner: GitHub's `ubuntu-24.04`. Workflow: [`ci.yml`](../.github/workflows/ci.yml) at commit
`7d4336a`, run `41950023132`, with the disk freed and the tests built without debug info.

| Run | Cache | Duration | Cache saved |
|---|---|---|---|
| Cold | None | 24 min 3 s | 1.1 GB |
| Warm (run `41952855207`) | Hit | 9 min 25 s: the disk freed in 3 min 46 s, the cache restored in 53 s, `scripts/check.sh` in 4 min 35 s | Not saved again (hit) |

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
  compositor. The egui window being frozen, these were left to slime-ui's frame statistics and MCP
  server ([ADR-0026](adr/0026-the-egui-window-frozen-until-the-move.md)); since ADR-0031, M1.14
  adds them to the egui window to measure it before the move.

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
