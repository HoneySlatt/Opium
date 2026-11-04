# 0019. The desktop's workings apart from its drawing, waking it through `Wake`

- Status: Accepted
- Date: 2026-11-04

## Context

Everything the desktop app did lived in `crates/app-desktop`, mixed with egui: the services and the
player behind the window, the settings, the lists and covers kept on disk, the media controls, the
other devices. A second window, drawn by another toolkit, was about to be built
([ADR-0022](0022-gpui-tried-and-abandoned.md)); it had to stand on the very same workings, and a fix
to them had to be made once.

## Decision

**The desktop's workings are a library of their own, `crates/desktop`**, which knows nothing of the
toolkit that draws the window (commit `efd90fa`):

- `backend`: the services and the player's sources, on a tokio runtime of its own thread; the
  window sends `Request`s and drains `Reply`s;
- `settings`, `cache` (the library as last read), `covers` (the covers kept on disk, their signing
  and the queue they are fetched in), `palette`;
- `mpris` (the media controls and the single instance, [ADR-0018](0018-one-instance-through-the-session-bus.md)),
  `devices` (Opium Connect), `soundcloud_browser` ([ADR-0012](0012-soundcloud-changes-through-a-browser-seam.md)),
  `exit` ([ADR-0021](0021-the-exit-watch.md)).

**They wake the window through `desktop::Wake`**, a plain `Arc<dyn Fn() + Send + Sync>`: every reply,
event or action that changes what the window shows calls it. The window built on egui makes it
from `egui::Context::request_repaint`.

**What stays with the window**: what the toolkit draws and loads (egui's textures and image
loader), the interface's state and the logic still mixed with it (`app-desktop/src/app.rs`).

## Alternatives considered

- **Both windows with their own copy of the workings**: every fix made twice.
- **The workings depending on egui's context to wake the window**: the second toolkit could not
  have used them.

## Consequences

- A fix to the workings is made in `crates/desktop` or below, never in a window.
- No crate but `app-desktop` names an egui type; a crate that did would tie the workings to the
  toolkit again (AGENTS.md, architecture rules).
- The move to another toolkit ([ADR-0022](0022-gpui-tried-and-abandoned.md)) redraws the window
  on these workings. The interface's own state and logic, about 6 300 lines of `app.rs`, are still
  mixed with egui and would move first.
- The player has no `Wake` of its own: the window polls it every 250 ms while music plays
  (PLAN, known debt).
