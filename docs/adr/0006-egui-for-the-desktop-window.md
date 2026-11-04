# 0006. egui for the desktop window

- Status: Accepted; amended by [0022](0022-gpui-tried-and-abandoned.md) (egui stays until Opium
  moves to SlimeEngine's UI framework)
- Date: 2026-10-29

## Context

The desktop app was to look like Apple Music on the Mac, in the spirit of Cider, from a design
approved before any code: a sidebar, a floating player bar, a queue drawer, Now Playing, pages tinted
by their cover's colours, Geist, Lucide icons. It had to be written in Rust, beside the services'
crates, and run on Linux under Wayland, at a 4K screen's fractional scale, drawing nothing while
nothing moves.

## Decision

**The desktop window is drawn with egui, run by eframe 0.37 on wgpu** (`crates/app-desktop`,
commit `361e28e`).

- **One system window**; menus, dialogs, the drawer, popovers and tooltips are drawn inside it.
- **The app draws its own controls** from egui's painter: shapes, meshes for the cover gradients and
  glows, its own springs (`ui/motion.rs`), Lucide SVGs tinted at their size (`icons.rs`), Geist in
  four weights (`fonts.rs`). It uses almost none of egui's own widgets.
- **What must go on while the window is hidden runs in eframe's `logic()`**, apart from `ui()`:
  the services' replies, the player, MPRIS and the other devices.
- **Frames are drawn only when something changes**: replies from the workings wake the window
  ([ADR-0019](0019-the-workings-apart-from-the-drawing.md)), and animations ask for their next
  frame at a capped rate.

## Alternatives considered

None are recorded from the time. GPUI was weighed and tried later, then abandoned
([ADR-0022](0022-gpui-tried-and-abandoned.md)).

## Consequences

- egui lays everything out again at each frame and keeps no identity for its elements: agents cannot
  read or drive the window, and screen readers get no names, as the app gives none
  (PLAN, known debt).
- The whole interface is the app's own drawing, about 29 000 lines in `crates/app-desktop`;
  `app.rs` alone holds the state and much of its logic beside egui (about 6 300 lines).
- Text needs the system's fonts for scripts Geist lacks, found through `fc-list` (`fonts.rs`).
- Moving to another toolkit means redrawing the window; the workings already live apart from it
  ([ADR-0019](0019-the-workings-apart-from-the-drawing.md)).
