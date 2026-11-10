# 0039. The exit watch leaves no core dump unless asked

- Status: Accepted; amends [0024](0024-the-exit-watch-aborts-for-a-core-dump.md) (what the abort
  leaves)
- Date: 2026-11-10

## Context

When the desktop app's teardown hangs, the exit watch ends it after 5 s with `abort()`, so that
`systemd-coredump` keeps a dump of every thread to find the hang with
([ADR-0024](0024-the-exit-watch-aborts-for-a-core-dump.md)). The [stage 1
audit](../audits/2026-11-10-stage-1.md) found that the dump holds the whole of Opium's memory: the
services' tokens, the downloads' key and decrypted audio, readable on disk by the user and root.
That breaks [ADR-0013](0013-secrets-and-what-is-kept-on-disk.md) (the downloads' key is never in a
file) and the DRM rules (no decrypted song at rest). Nothing in the code keeps secrets out of a
dump.

## Decision

**Opium makes itself not dumpable at start** (`prctl(PR_SET_DUMPABLE, 0)` on Linux), so that the
exit watch's abort, or any crash, leaves no core dump. **A variable of the environment asks for
dumps again**, for whoever debugs the hang on a throwaway profile; its name is set when it is
built. The abort and `exit.log` stay. Built in M1.15.

## Alternatives considered

- **`_exit` again**, as before ADR-0024: no dump ever, and the hang in eframe's teardown no longer
  debuggable.
- **Keeping the dump**: secrets and decrypted audio left on disk after each hang, against the
  project's rules.

## Consequences

- A hang leaves `exit.log` only, unless the variable is set; the hang is debugged on a throwaway
  profile, where no real secret is in memory.
- `crates/desktop` takes the `libc` crate again for one call.
- A crash of any kind leaves no dump either, so a crash report of Opium's own (an idea in the
  [PLAN](../PLAN.md#ideas)) matters more.
