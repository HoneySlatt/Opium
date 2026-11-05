# 0026. The egui window is frozen until the move to SlimeEngine's UI framework

- Status: Accepted; amends [0006](0006-egui-for-the-desktop-window.md) and
  [0022](0022-gpui-tried-and-abandoned.md) (the window frozen until the move)
- Date: 2026-11-05

## Context

The desktop window, `crates/app-desktop`, is drawn with egui until Opium moves to SlimeEngine's UI
framework ([ADR-0022](0022-gpui-tried-and-abandoned.md)), which is coming soon. The quality bar
measured on 2026-11-05 ([ADR-0025](0025-the-quality-bar.md), [`measurements.md`](../measurements.md))
found work in the window: its smoothness and frame times could not be measured, as nothing drives
it and it records no frame times; its memory at rest and its warm start miss their budgets; it has
no accessibility names. Work in egui now would be redone on the new framework.

The same day, the owner first chose to grow the screenshot mode into a scripted driver (scroll,
click, type, keys, and frame times recorded), so that an agent could drive and measure the window
without a human, as the method asks. An MCP server inside the egui window was rejected then: much
code in egui, thrown away at the move, as the new framework has an MCP server of its own. The
scripted mode was then dropped too, for the reason below.

## Decision

**`crates/app-desktop` is frozen until the move to SlimeEngine's UI framework.** It changes only
to fix a loss of data, a crash or a security flaw.

**What waits for the move**: everything else in the window: smoothness, the window's memory,
accessibility, fonts, animations, the window's start-up, and the exit hang inside eframe's teardown.

**Where the work goes meanwhile**: the shared workings (`crates/desktop`, `player`, `connect`,
`synced`, `opium-core`, the services), the Android app and the documents.

**Driving Opium by an agent comes with the new framework's MCP server.** Until then, the
frame-time and input-to-response budgets stay unmeasurable.

## Alternatives considered

- **A scripted screenshot mode** (scroll, click, type, keys, frame times): chosen, then dropped the
  same day; code in egui, thrown away at the move.
- **An MCP server inside the egui window**: more code in egui, thrown away likewise.
- **Working on the window as before**: each fix redone on the new framework.

## Consequences

- The budgets about the window stay missed or unmeasured until the move: frame times, input to
  response, the window's memory and start-up, accessibility.
- Debt in the shared crates stays planned work: the idle `blocking` thread, the player's idle
  timeout and its wake of the window, Opium Connect's wake-ups.
- The theme watch's poll, in `app-desktop`, waits for the move.
- The interface's state and logic are taken out of `app.rs` as part of the move (ROADMAP, Stage 5),
  not before.
