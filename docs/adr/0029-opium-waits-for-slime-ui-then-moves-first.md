# 0029. Opium waits for slime-ui, then moves to it before anything else

- Status: Accepted; amends [0026](0026-the-egui-window-frozen-until-the-move.md) (no work meanwhile)
  and the ROADMAP's order (Stage 5 comes right after Stage 1); amended by
  [0030](0030-the-kopuz-code-rewritten-in-a-clean-room.md) (the standby lifted for the clean-room
  milestones), [0031](0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md) (the move during
  Stage 6d), [0034](0034-what-opium-is-to-become.md) (the order after the phone) and
  [0035](0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md) (the standby's end)
- Date: 2026-11-05

## Context

The egui window is frozen until the move to slime-ui, SlimeEngine's UI framework
([ADR-0026](0026-the-egui-window-frozen-until-the-move.md)). slime-ui runs applications at the end
of SlimeEngine's Stage 6d, which needs every indispensable need of
[`ui-framework-needs.md`](../ui-framework-needs.md) (SlimeEngine's ADR-0170). The ROADMAP put the
move last, as Stage 5, after the first release, the phone and Windows, and ADR-0026 sent the work
meanwhile to the shared crates, the phone's system layer and the documents.

On 2026-11-05 the owner chose to stop Opium once M1.6 is done, and to start again with the move.
slime-ui is the owner's own framework, shared by SlimeEngine's editor and games, by Slatty and by
Opium, so that what one of them needs serves the others.

## Decision

**Opium is on standby from the end of M1.6 until SlimeEngine's Stage 6d is done.** No milestone
runs meanwhile. Opium stays in daily use: a loss of data, a crash, a security flaw, or a service
that stops working is still fixed.

**At the wake, Stage 1 closes first**: the phone under its new name (M1.4), the Android shell from
an empty store (M1.5), then the stage audit (M1.8), so that the audit is current when the move
starts.

**Then the move, Stage 5, before Stages 2, 3 and 4.** egui is replaced whole by slime-ui; the first
release, the phone and Windows follow, in that order. The stages keep their numbers, as ADRs cite
them.

**A need the move finds missing in slime-ui goes to slime-ui**, in SlimeEngine's repository and
under its rules, when it is general: it serves the editor, games or another application too, or
costs them nothing (SlimeEngine's ADR-0170). A need of Opium's alone stays in Opium. Neither is
worked around in the other.

## Alternatives considered

- **Keep working meanwhile on the shared crates and the phone's system layer** (ADR-0026): the
  known debt shrinks sooner, but the owner's attention goes to slime-ui, and the phone's system
  layer may change again with its toolkit (Stage 3).
- **Close Stage 1 before the standby**: the project's state known at the pause, but the audit
  would mostly find egui's debt, which leaves with the move, and would be stale at the wake; M1.5
  also waits on Google's Android repository.
- **Keep the move last**: the first release and Windows would ship egui, and Windows' window would
  be ported twice.

## Consequences

- The ROADMAP's decision "Windows before or after the move" is settled: after, ported once.
- The first release (0.1.0) ships on slime-ui; the move has no release of its own.
- The known debt waits for the wake, the shared crates' included.
- Stage 1 stays open through the standby.
