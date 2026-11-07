# 0035. The port in uwui's first stage, Stage 1 finished first

- Status: Accepted; amends [0031](0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md) (when
  the port comes, and when the standby ends)
- Date: 2026-11-07

## Context

[ADR-0031](0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md) had Opium's desktop ported to
uwui during SlimeEngine's Stage 6d, as the first of its four ports, and kept Opium on standby until
that stage planned its ports; Stage 1 was to close between the wake and the port.

SlimeEngine changed the plan since. Its ADR-0194 and ADR-0203 put the programs of uwui's first stage
after Stage 6d, which builds uwui's foundations and moves its code to its own repository (M6d.11);
its ADR-0206 lists them: Opium on the desktop and its Android prototype, ported, Pluto's client, and
Slatty. Opium's figures under egui and Compose are measured before its port. On 2026-11-07, Stage 6d
stood at M6d.1c3: uwui has no desktop host yet (M6d.7), so no program can run on it. Its state model
was chosen meanwhile, signals (SlimeEngine's ADR-0205), checked against Opium's list of needs among
others.

The standby's end, "when Stage 6d plans its ports", no longer comes at a time anyone can name.

## Decision

**Opium's desktop and its Android prototype are ported in uwui's first stage**, after SlimeEngine's
Stage 6d, as SlimeEngine's ADR-0203 and ADR-0206 have it, not during Stage 6d. Opium's Stage 5
starts when that stage takes Opium's port.

**The standby ends on 2026-11-07 for Stage 1's remaining milestones**, by the owner's decision of
that day: M1.4's phone part (with the owner), M1.5, M1.14 and the stage audit M1.8, so that Opium is
ready when uwui is. After M1.8, Opium waits again, under the same rules as before (only a loss of
data, a crash, a security flaw or a broken service fixed), until its port starts.

## Alternatives considered

- **Starting the port's groundwork now**: taking the interface's state and logic out of `app.rs`
  into a model shaped for signals. The port would go faster, but it changes the frozen window
  ([ADR-0026](0026-the-egui-window-frozen-until-the-move.md)) before uwui's API is stable, at the
  risk of doing it twice.
- **Staying on standby until uwui's first stage**: nothing done in vain, but the measurements and
  the audit fall at the last moment, and the figures uwui must match would be taken in a hurry.

## Consequences

- AGENTS.md, the PLAN and the ROADMAP say the port comes in uwui's first stage, and that Stage 1 is
  being finished.
- M1.14's figures are taken before uwui can run Opium, so they wait unchanged until the port is
  measured against them; the frozen window keeps the one change ADR-0031 allows, for M1.14.
- The needs Opium marks as indispensable gate the end of uwui's first stage (uwui's ROADMAP), not
  of Stage 6d: where ADR-0034 and `ui-framework-needs.md` said "the end of Stage 6d", read "the end
  of uwui's first stage".
- Where SlimeEngine's plan and Opium's disagree again, SlimeEngine's decides when uwui is ready, and
  Opium records the change.
