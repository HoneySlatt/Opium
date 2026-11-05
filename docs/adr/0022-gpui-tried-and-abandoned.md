# 0022. GPUI tried and abandoned; egui stays until SlimeEngine's UI framework runs applications

- Status: Accepted; amends [0006](0006-egui-for-the-desktop-window.md) (egui stays the window until
  the move to SlimeEngine's UI framework); amended by [0026](0026-the-egui-window-frozen-until-the-move.md)
  (the window frozen until then)
- Date: 2026-11-04

## Context

On 2026-11-04 the owner asked whether the desktop window should move from egui to GPUI, the
framework of the Zed editor: the same interface, finer animations, the least power drawn, and
Windows later. A spike, kept on the branch `spike/gpui` (commit `b748901`, never merged), drew M3E's
Now Playing twice, with the app's own egui code and with GPUI from Zed's `main`, and measured both
on the owner's desktop (a 240 Hz monitor at 1.5×; `spikes/gpui-now-playing/NOTES.md` on that
branch):

| Case | egui, CPU | GPUI, CPU | Frames/s, egui / GPUI |
|---|---|---|---|
| Playing, window in use | 29.5 % | 51.8 % | 240 / 240 |
| Paused, window in use | 0.00 % | 0.07 % | 0 / 0 |
| Playing, window in the background | 0.00 % | 6.5 % | 0 / 28 |
| Animated cover, 25 fps | 9.1 % | 12.4 % | 25 / 25 |

GPUI drew at scale 2 on the 1.5× monitor (1.78 times egui's pixels), had no in-place texture update
(0.80 ms per animated-cover frame against egui's 0.002 ms), and its small text was softer. The
owner chose to migrate anyway, and the port began the same day in a Cargo workspace of its own
(`desktop-gpui/`, commit `8d75008`: GPUI pins dependencies egui's cannot share a lockfile with), on a
fork of GPUI patched for Wayland's fractional scale and for dithered gradients. It reached the Songs
page, playback, navigation, the album and playlist pages, and the cover grids (commits `6e4a5a7`,
`e0d038c`, `9ef9e97`, `0fee1fd`), about 20 000 lines.

The same day, the owner set GPUI aside for all of their applications: they are to move onto the UI
framework of their own game engine, SlimeEngine, once it runs desktop applications. SlimeEngine
decided this in its ADR-0170 and ADR-0171 and its Stage 6d: applications become the framework's
third client, beside the editor and games; a launcher of games, Slatty, is the pilot, ported during
the stage; this music player follows after the stage, in its own repository.

## Decision

**The GPUI window is removed** (commit `9d784f5`), with its workspace, its build rules and the
libraries the dev shell linked for it. **It is kept under the tag `gpui-window`** (on `529af16`), and
the spike stays on its branch.

**egui's window stays the one window** ([ADR-0006](0006-egui-for-the-desktop-window.md)) until
Opium moves to SlimeEngine's UI framework, after SlimeEngine's Stage 6d and after Slatty's port.
Until then, no other toolkit is started.

**The separation of the workings from the drawing stays**
([ADR-0019](0019-the-workings-apart-from-the-drawing.md)): it is what the next toolkit stands on.

## Alternatives considered

- **Finishing the GPUI port**: a fork to maintain against Zed's churn, more processor time while
  animating on a fractional-scale screen, and a second interface stack among the owner's
  applications.
- **Staying on egui for good**: no identity for agents to read and drive, no screen-reader names,
  and a framework apart from the owner's other applications.
- **Moving to SlimeEngine's framework now**: it does not yet run applications; its Stage 6d builds
  what they need.

## Consequences

- Work on the window stays on egui; fixes to the workings go to `crates/desktop`.
- What the app needs from a UI framework, and what egui gives it today, is to be stated in the
  repository for SlimeEngine's Stage 6d, whose exit asks that every need the music player marks as
  indispensable exist (ROADMAP).
- Two app bugs the spike found were fixed on `main`: M3E's wavy progress bar and turning shapes
  drew at the display's rate in the background (commits `941cd25`, `5b5e709`).
- The rename to Opium, first planned for the switch to GPUI, was taken on its own
  ([ADR-0023](0023-the-rename-to-opium.md)).
