# 0043. One instance per profile, held by a lock

- Status: Accepted; amends [0018](0018-one-instance-through-the-session-bus.md) (how the one
  instance is held)
- Date: 2026-11-10

## Context

[ADR-0018](0018-one-instance-through-the-session-bus.md) keeps the desktop app to one instance
through its MPRIS name on the session bus. The [stage 1 audit](../audits/2026-11-10-stage-1.md)
(L2) found two ways for two instances to run at once anyway: `main` looks for the name about
170 ms before the window takes it, so a double launch races; and the MPRIS library lets a later
request take the name from its holder, which nothing notices. With no session bus, the check finds
nothing at all. Two instances rewrite the same files whole from their own copies, and the
downloads of one are swept by the other: the owner lost data this way on 2026-11-07.

## Decision

**Two instances on the same files are impossible.** The first thing `main` does is take an
exclusive lock (`flock`) on a file in Opium's config folder, held until the process ends; the
kernel lets it go when the process ends, crashes included.

- **The lock taken**: the app starts. It asks for its MPRIS name without letting a later request
  replace it.
- **The lock held by another**: the app hands its files to the running instance, or asks it to
  come forward, through the session bus as ADR-0018 says, then exits. If the bus cannot reach the
  running instance, it says so and exits all the same.
- **The lock cannot be taken for another reason** (the folder unwritable): the app says why and
  exits, rather than run unguarded.

The lock covers one profile: an instance on another config folder, such as an agent's throwaway
profile, runs beside the owner's, touching none of its files.

## Alternatives considered

- **The bus alone, stricter**: one name of Opium's own, asked for first, never replaced. One
  mechanism, but with no session bus the app must either start unguarded or refuse to start, and
  two sessions sharing one home folder do not see each other.
- **One instance per user**: the lock in the runtime folder. No test instance could then run while
  the owner's Opium runs.

## Consequences

- ADR-0018's "with no session bus, the check finds nothing and the app starts" no longer holds:
  without a bus, the lock still guards the files.
- A test instance on a throwaway profile no longer raises the owner's window; one on the owner's
  profile still does, and exits.
- A config folder on a network file system that ignores `flock` is not guarded; Opium does not
  support one.
- The owner chose this on 2026-11-10; built in M1.15.
