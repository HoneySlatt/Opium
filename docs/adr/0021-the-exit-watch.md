# 0021. The exit watch

- Status: Accepted; amended by [0024](0024-the-exit-watch-aborts-for-a-core-dump.md) (the process
  ends by an abort, for a core dump)
- Date: 2026-11-04

## Context

Closing the desktop app now and then left the process behind: the window gone, the process still
there until killed by hand. While it hung, starting Opium again only brought the hung instance
forward ([ADR-0018](0018-one-instance-through-the-session-bus.md)), so the user could not reopen
it. The cause was not found: it did not happen in a dozen tries. The teardown involves the window,
the player's engine, Opium Connect's goodbyes and the Widevine CDM's own teardown at exit.

## Decision

**Once what the app keeps is on disk, it ends within five seconds, torn down or not**
(`desktop::exit`, commit `575eb28`):

- `on_exit` saves the session, the settings and the shared state, then calls `exit::leaving()`,
  which starts a watch thread.
- **Five seconds later** (`GRACE`: the player waits half a second for its engine, the other devices
  a second to say goodbye), if the process is still there, the watch notes where the teardown stood
  (`exit::reached` marks each stage) and what each remaining thread waits on (its name, `wchan` and
  system call, from `/proc/self/task`), appends it to `exit.log` in the cache folder and to stderr,
  then ends the process with `_exit(0)`, not `exit`, which could run into whatever held the
  teardown.

## Alternatives considered

- **Finding and fixing the hang first**: not reproduced; meanwhile users could not reopen the app.
- **Killing the process at once after saving**: no teardown at all, and nothing learnt of the hang.

## Consequences

- The app always ends, and `exit.log` collects what is needed to find the cause; the hang itself is
  still open (PLAN, known debt). Once found and fixed, the watch can stay as a safety net or go, by
  an ADR.
- Anything that must be on disk at exit is written before `leaving()` is called.
- `_exit` skips destructors and `atexit` handlers: nothing may rely on them.
