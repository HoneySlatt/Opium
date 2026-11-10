# 0018. One instance, found through its MPRIS name on the session bus

- Status: Accepted; amended by [0043](0043-one-instance-per-profile-held-by-a-lock.md)
  (the one instance held by a lock)
- Date: 2026-11-04

## Context

The desktop app tells the desktop what it plays through MPRIS, on the D-Bus session bus, under the
name `org.mpris.MediaPlayer2.opium` (`desktop::mpris`, since commit `8c6721d`): media keys,
`playerctl` and bars' player widgets talk to it. Started a second time, the app opened a second
window: the two shared the settings, the playlists, the history and the downloads, each rewriting
whole files from its own copy, so whichever wrote last silently took the other's changes away.
Files opened from the file manager also need to reach the instance already running.

## Decision

**Before opening a window, the app looks for itself on the session bus** (`main.rs`,
`mpris::raise_running`, `mpris::open_in_running`, commit `1f29653`):

- **started without files**, it asks the running instance to come forward (MPRIS `Raise`) and
  exits;
- **started with files**, it hands them to the running instance (MPRIS `OpenUri`), brings it
  forward and exits;
- **when none runs**, it opens its window and takes the name.

**The program holding the name is asked whether it is Opium** (its `DesktopEntry` and `Identity`)
before anything is handed to it: any program may take the name first (commit `804dee9`).

## Alternatives considered

None were written down. A lock file, the obvious other way, would tell that another instance runs
but could neither bring it forward nor hand it files, which the MPRIS name already on the bus does.

## Consequences

- A test build started while the owner's Opium runs only raises the owner's window, and exits with
  nothing printed. An agent either closes the owner's window gracefully first or runs the test on
  another session bus (CONTRIBUTING.md).
- With no session bus, the check finds nothing and the app starts; MPRIS is then unavailable.
- A Windows port needs another way to find the running instance (ROADMAP).
