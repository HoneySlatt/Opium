# 0051. What Opium builds while it waits for uwui

- Status: Accepted; supersedes [0048](0048-formats-and-connects-versions-before-the-port.md);
  supersedes in part [0035](0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md) (what is
  done while Opium waits)
- Date: 2026-11-17

## Context

Opium waits for its port in uwui's first stage, after SlimeEngine's Stage 6d, which still has many
milestones ahead. [ADR-0035](0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md) allowed
only a fix for a loss of data, a crash, a security flaw or a broken service meanwhile;
[ADR-0048](0048-formats-and-connects-versions-before-the-port.md) added, by the owner's decision of
2026-11-16, the formats' and Connect's versions, and the PLAN two more milestones, the documentation
and the errors and log.

The [architecture audit of 2026-11-17](../audits/2026-11-17-architecture.md) found:

- **No application layer** (ARC-1). The desktop's lives in the egui window (`App` owns the queue,
  the player, MPRIS and Connect; its retries run on the frame loop); the phone's in its Kotlin system
  layer (`data/`, `playback/`), which is not frozen; `opium-ffi` is thin. The copies already differ,
  and the phone lacks the desktop's sign-out fence.
- **Sign-outs that are not final** on both apps (SEC-1 to SEC-3): late connects, sign-ins, writes and
  replies put a signed-out account back, and on the phone confirm its protected downloads again.
- **Kotlin is checked by no CI** (CTB-1), though the phone's fix crosses that boundary.
- **The formats' work is larger than one milestone**: types of their own for what crosses versions,
  since `ProviderKind` is the UniFFI enum the frozen screens iterate (DON-5); unknown fields kept
  (DON-6); Connect's downgrade (DON-4).
- **No check of parity** for the port: nothing records what the app does at the boundaries the port
  replaces.

uwui's state model is chosen (signals, SlimeEngine's ADR-0205); its declaration of actions is not
settled yet.

## Decision

**While it waits, Opium builds these milestones, in this order**, beside fixes for a loss of data, a
crash, a security flaw or a broken service:

1. **Kotlin's CI**: the Android job builds the core's bindings, the Kotlin and the APK, and runs the
   JVM tests.
2. **Final sign-outs**: nothing started before a sign-out takes effect after it, on both apps,
   through one fence shared by `desktop::backend` and `opium-ffi`; the devices a forgotten device
   introduced unpaired on this device; Apple's licences paced
   ([ADR-0052](0052-apples-licences-paced-in-one-place.md)).
3. **The documentation under the method's rules of 2026-11-16.**
4. **The files' formats and versions.**
5. **Connect's versions**, with a forgotten device forgotten everywhere
   ([ADR-0053](0053-a-forgotten-device-forgotten-everywhere.md)).
6. **Errors and the log.**
7. **Parity tests**: what the app does at `desktop::backend`'s `Request`/`Reply` and at
   `opium-ffi`'s `Session`, recorded over fake services, for the port to match.
8. **The design of the shared layer** that needs no toolkit: the services' lifecycle, who owns and
   calls off each task, the policies both apps share, the errors' kinds. How state binds to the
   interface and how actions are declared wait for uwui's, at Stage 5. Nothing of it is built during
   the wait but the fence of milestone 2.

**The formats' conventions** stay as the owner decided on 2026-11-16: each kept file carries its own
version; a build that finds a profile written by a newer one refuses it and writes nothing, going
back meaning a backup restored; a cache of another version is thrown away and rebuilt; "this profile
comes from a newer Opium" and "this other device must be updated" are said by the core only, to the
log and the terminal, as the frozen windows show nothing new until uwui.

**The frozen windows change only mechanically**: no milestone of the wait gives the egui window or
the Compose screens anything new to show.

## Alternatives considered

- **The design at Stage 5's start**, as the ROADMAP had it: no work added now, but the fixes and
  milestones of the wait written twice, and the design made under the port's pressure.
- **The shared layer designed and built now**, all of it outside the frozen windows: more
  duplication gone before the port, but the desktop's retries and queue sit in the frozen window,
  and how actions are declared is not settled in uwui.
- **The order of 2026-11-16** (documentation first): the security fixes would wait behind the
  documents, and the phone's would be checked by hand.
- **One milestone for the formats and Connect**, as ADR-0048 had it: one install of both apps at its
  end, but a milestone too large to check at once.

## Consequences

- The PLAN holds these milestones; the ROADMAP's Stage 5 starts from the design instead of a
  session of its own.
- [ADR-0035](0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md) rejected taking the
  interface's state out of `app.rs` before uwui: that still holds, as nothing here changes the
  window.
- The fence of milestone 2 sets an API before the design; the design adopts it or replaces it, and
  says so.
- The owner installs the new desktop and APK together at the end of milestones 4 and 5, so that both
  speak the same versions.

## Validation and reconsideration

Each milestone is checked by its tests: the sign-outs by a fake service held during a sign-out on
both apps; the formats by files written by today's build kept in the tests; Connect by two versions
of the wire, an edited hello refused; the parity by the tests themselves, run again at the port. To
be put to the owner again if uwui's first stage starts before these milestones end, or if one of them
grows past what can be checked at once.
