# 0031. Opium moves to uwui during SlimeEngine's Stage 6d, on the phone too

- Status: Accepted; amends [0029](0029-opium-waits-for-slime-ui-then-moves-first.md) (the move
  during Stage 6d, not after it), [0022](0022-gpui-tried-and-abandoned.md) (before Slatty's move),
  [0028](0028-the-phones-compose-screens-frozen-until-measured.md) (uwui on the phone without
  condition), [0026](0026-the-egui-window-frozen-until-the-move.md) (the window instrumented to
  measure its frames) and [0027](0027-macrobenchmark-for-the-phones-measurements-at-stage-3.md) (the
  Macrobenchmark in Stage 5); amended by
  [0032](0032-the-gpl-kept-and-kopuzs-code-still-written-again.md) (Opium keeps the GPL, which
  uwui's licence allows it to use)
- Date: 2026-11-05

## Context

Opium waits for SlimeEngine's UI framework, then moves to it before anything else
([ADR-0029](0029-opium-waits-for-slime-ui-then-moves-first.md)). The phone was to take the
framework only if a prototype matched or beat Jetpack Compose
([ADR-0028](0028-the-phones-compose-screens-frozen-until-measured.md)).

On 2026-11-05 the owner decided uwui's name and scope in SlimeEngine's repository (its ADR-0178 to
ADR-0182). What of it changes Opium's plan:

- The framework is named **uwui**, formerly slime-ui (SlimeEngine's ADR-0179). It moves to a
  repository of its own, under MIT OR Apache-2.0 (its ADR-0181).
- uwui is the base of the interfaces of all of the owner's projects. Four programs are ported to it
  during SlimeEngine's Stage 6d, before its API is declared stable, in this order:
  1. Opium on the desktop;
  2. Slatty, the launcher;
  3. Opium's Android prototype, measured against Compose;
  4. Magnolia, a media application.
- On Android, uwui becomes Opium's interface whatever the prototype shows, and is fixed until it
  matches Compose.
- uwui is embedded in a host, and its core never owns the event loop:
  - on the desktop, a host on winit, which uwui provides, runs the whole program;
  - on Android, the program stays a Kotlin program, which keeps its lifecycle, services, media
    session, Android Auto, MediaDrm and notifications. uwui draws in a surface that Kotlin gives
    it, so that screens move one at a time.
- The host's API carries:
  - the surface, created, lost, created again and resized, with its scale;
  - input: the pointer, multi-touch, the keyboard and gamepads, and the system's keyboard with
    composition;
  - the lifecycle (pause, resume, low memory);
  - accessibility, through an AccessKit adapter for each platform;
  - the clipboard;
  - requests for frames, so that nothing is drawn at rest.
- uwui's targets are the desktop on Linux and Windows, Android phones, and the web in a canvas;
  macOS and iOS are within reach, without a port. Android TV, consoles, Android Auto and CarPlay are
  out of its scope: they stay interfaces the system imposes, which the Rust core and the native
  layer feed.
- On Android, uwui draws through Vulkan, with GLES as its fallback, by one drawing path.
- The look is the same everywhere, from themes in TOML files. A small layer for each platform gives
  the behaviours its users expect: a scroll's physics, Android's predictive back, a selection's
  handles and the long press, haptics. Input comes from the foundations, with one model of focus
  (Tab and directions).
- **The exit bar of each port** (SlimeEngine's ADR-0178): each program runs on uwui at least as
  well as on its present framework, on figures measured first. These are start-up, frame times
  while scrolling and animating, and memory.
- Before the API is frozen, a trial on Android, bounded to two milestones and thrown away. It
  hosts uwui in a Kotlin activity and covers:
  - the surface lost and made again, and rotation;
  - the system's keyboard with composition;
  - suspend and resume;
  - scrolling by touch;
  - TalkBack;
  - frame times and power on the owner's phone, in a release build.

  A trial on the web, of the same size, follows.
- The lower tiers, none measured: a 2015 laptop (two cores, Intel HD 520, 4 GB, 1080p at 60 Hz),
  and an entry-level Android 14 phone (Helio G99 class, 4 GB, 90 Hz).
- uwui is under MIT OR Apache-2.0. Its own repository comes during Stage 6d, after it is decoupled
  from the engine. Until then a program depends on it from SlimeEngine's repository, under the
  names `uwui` and `uwui-render` from SlimeEngine's milestone M6c.N on.

## Decision

**The framework is uwui.** Opium's documents name it so from now on. Earlier ADRs keep the name
of their date.

**The desktop moves during SlimeEngine's Stage 6d**, as the first of its four ports, before uwui's
API is declared stable. The standby of ADR-0029 ends when SlimeEngine plans Stage 6d's ports, not
when the stage ends, so that Stage 1 closes (M1.4, M1.5, M1.14, then the audit M1.8) before Opium's
port starts. Stage 5, the move, still comes right after Stage 1. A need missing in uwui still goes
to uwui, under its rules, when it serves others too; one of Opium's alone stays in Opium.

**Opium's figures under egui are measured before Stage 6d's port**: start-up, frame times while
scrolling and animating, and memory, on the high desktop tier, and on the phone under Compose.
Frame times were left unmeasurable by M1.7, because the window records none and cannot be driven.
Measuring them is the one change allowed to the frozen window
([ADR-0026](0026-the-egui-window-frozen-until-the-move.md)), besides its fixes: a frame-time record
and a scripted scroll, removed with egui.

**Opium's logic never depends on uwui**, as it never depended on egui
([ADR-0019](0019-the-workings-apart-from-the-drawing.md)). Until uwui has its own repository,
Opium depends on it from SlimeEngine's, by path or git revision, under its new names.

**On the phone, uwui replaces Compose without condition.** The phone's prototype, SlimeEngine's
third port, made in Stage 5, is still measured against the Compose screens in smoothness, start-up
and power, on the owner's phone. The measurement no longer chooses the toolkit: it lists what uwui
must fix until it matches Compose. The Compose screens stay frozen until uwui replaces them, screen
by screen. The Kotlin system layer stays and goes on: playback, MediaDrm, the media session, Android
Auto, the services, downloads.

## Alternatives considered

- **Moving after Stage 6d, as ADR-0029 decided**: uwui's API would be stable when Opium arrives, but
  Opium's needs would come too late to shape it. The owner chose to port before the API freezes.
- **The phone on uwui only if it matches Compose, as ADR-0028 decided**: the phone never served
  worse than today, but it could stay on Compose for ever, with a second interface to maintain.

## Consequences

- Opium's port shapes uwui's API: what it lacks is added there, during the stage.
- Until uwui matches Compose on the phone, the phone may be served less well than by Compose for a
  time; the measurement says by how much, and what to fix.
- Stage 3 no longer decides a toolkit. The Macrobenchmark of
  [ADR-0027](0027-macrobenchmark-for-the-phones-measurements-at-stage-3.md) comes with the
  prototype, in Stage 5, before Stage 3.
- Slatty no longer moves before Opium: the ROADMAP's Stage 5 changes accordingly.
- A milestone measures the egui window's frame times, start-up and memory before the port, and
  the phone's under Compose; it is planned in `docs/PLAN.md`.
- uwui shares Opium's intended licence, MIT OR Apache-2.0, so depending on it raises no question
  of licence.
