# 0024. The exit watch aborts, for a core dump

- Status: Accepted; amends [0021](0021-the-exit-watch.md) (how the watch ends the process); amended by [0039](0039-the-exit-watch-leaves-no-core-dump-unless-asked.md)
- Date: 2026-11-04

## Context

The exit watch ([ADR-0021](0021-the-exit-watch.md)) ended a hung process with `_exit(0)` after
writing `exit.log`. The log caught one hang on 2026-11-04: the teardown had not left eframe (stage
"tearing down", before `run_native` returned), and the Mesa OpenGL driver's threads (`opium:gl0`,
`opium:sh0`, `opium:cs0`…) were still alive beside the app's own. It did not say which call blocked:
the log has each thread's name and kernel wait, not its stack. The hang did not come back in nine
more runs, three of them under gdb, which changes the timing; it cannot be waited for under a
debugger.

## Decision

**The watch ends a hung process with `std::process::abort()`, not `_exit`.** It still writes
`exit.log` first. The abort leaves a core dump of every thread, which `systemd-coredump` keeps and
`coredumpctl gdb` opens, so the next hang, on the owner's machine in daily use, shows the call that
blocked. `libc` leaves `crates/desktop`'s dependencies.

## Alternatives considered

- **Keep `_exit` and wait under gdb**: the hang does not happen under gdb.
- **Have the watch capture the other threads' stacks itself**, through a signal sent to each: code
  that runs in signal handlers inside a process already stuck, to keep in the app for good.
- **A core dump only when an environment variable asks**: the hangs happen in daily use, launched
  without it.

## Consequences

- Each hang leaves a core dump (`coredumpctl list opium`) and a line in `exit.log`; the system logs
  it as a crash. Nothing changes for the user: the window is already closed and everything kept.
- Like `_exit`, `abort` skips destructors and `atexit` handlers: nothing may rely on them.
- Once the hang is found and fixed, whether the watch stays, and how it ends the process, is decided
  again by an ADR (ROADMAP, open decisions).
