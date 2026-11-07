# 0037. uwui's decisions of 2026-11-07, taken into Opium

- Status: Accepted; amends [0011](0011-themes-as-toml-files-of-component-variants.md) (M3E built on
  uwui's official theme; Stock gains glass), [0025](0025-the-quality-bar.md) (uwui's bars for
  applications enter Opium's) and [0034](0034-what-opium-is-to-become.md) (glass and its motion
  earlier for one screen; actions for users' agents, native; macOS planned after uwui's 1.0)
- Date: 2026-11-07

## Context

On 2026-11-07 the owner took decisions on uwui in SlimeEngine's repository, where uwui's decisions
are kept until its code moves: SlimeEngine's ADR-0213 (uwui's reach, its lead, its themes and
agents), ADR-0215 (measured against the market), ADR-0218 (one core for agents) and ADR-0220 (Stage
6d's milestones widened). ADR-0214 keeps uwui under MIT OR Apache-2.0, paid for by patrons from its
1.0, which Opium, under the GPL, can depend on as before. Opium is one of the programs uwui's first
stage ports, its desktop and a prototype of its phone measured against Compose
([ADR-0035](0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md)). What those decisions
touch in Opium:

- **The lead.** A program on uwui is measured beside a *blank window*, a program that only clears
  its window and presents it. Its *own cost* is what it adds above that window. uwui's own cost must
  be at most a third of the best rival's, on memory, the start, the processor, power, a frame's time
  and the package's size. Opium's table of songs is one of uwui's four screens of reference, against
  Spotify and Cider.
- **A fifth screen of reference, rich in effects**: glass over moving content, a background blurred,
  shapes that morph, a transition on springs from one screen to the next. SlimeEngine's ADR-0213
  proposes Opium's screen of the song playing "in Opium's Glass theme", for the owner to choose.
  Opium has no Glass theme: its themes are Stock, Spotify and M3E
  ([ADR-0011](0011-themes-as-toml-files-of-component-variants.md)). Opium's
  [ADR-0034](0034-what-opium-is-to-become.md) puts glass, the cover flying between screens and
  gestures that drive animations in Stage 6, after the move to uwui. The screen comes in uwui's
  first stage, which is Opium's Stage 5.
- **Official themes.** uwui's core ships one sober look. Glass, Material 3 Expressive and Pro become
  official themes of uwui, each a crate of its own in uwui's repository, its images of reference in
  uwui's CI. Keeping a theme in Opium and copying it was rejected, as copies drift. The glass
  material stays an effect of uwui's core (SlimeEngine's ADR-0171), so a theme uses it without the
  Glass theme. Opium's `m3e.toml` holds the generic look and parts proper to a music player: the
  player bar, the wave of progress, the rail with its side panel and floating toolbar. Every commit
  to it is under the owner's identity, and nothing in it comes from Kopuz.
- **Users' agents.** An application declares its actions once: a name, typed parameters, a
  description, a permission and a family. Before uwui's 1.0, a local MCP server lets the user's
  assistant call them on the desktop. The user consents agent by agent. Each action goes through
  the interface's own commands, so it can be undone and is logged. No secret is given. Adapters for
  Android's AppFunctions, Apple's App Intents and Windows' registry of agents come when a pilot needs
  them; Android's comes in the stage where Opium ships on Android with uwui. Opium's
  [ADR-0034](0034-what-opium-is-to-become.md) already plans the `opium` command (`opium play`,
  `opium next`, `opium queue add`) at Stage 6, which asks the same things of the app. Some actions
  would change a real account at a service: a like, a playlist, the library.
- **macOS at uwui's 1.0.** uwui supports macOS at its 1.0, checked by hand in the owner's virtual
  machine (macOS 26, Intel, an AMD GPU), and on a Mac with Apple's chips when one is at hand. An
  application that uses only uwui's APIs runs on Linux, Windows, macOS and the web with no line
  written for one platform. Opium's sound, its CDM, its tokens and its single instance are not
  uwui's: they stay code of Opium's for each platform. Stage 10 says macOS is "eventual", "once uwui
  supports it and a Mac can check it".
- **uwui measured as the engine is.** SlimeEngine's ADR-0215 applies to applications on uwui a
  light sensor on the screen, which the owner buys, timing input to photon; blind tests by people;
  budgets of compiling; and hours of use without a crash, crash reports staying on the machine. A
  *test of signature* checks that applications on uwui are not recognised as such more often than
  chance. Opium's quality bar has none of these.

## Decision

The owner's decisions of 2026-11-07.

**The fifth screen: Opium's song playing, in Stock, with glass.** The owner chooses it as uwui's
fifth screen of reference. Stock gains glass, through uwui's core's material. For that screen alone,
glass, the cover flying to it from the screen before, and the transitions on springs come in Stage 5
instead of Stage 6; the rest of Stage 6 stays where it is. SlimeEngine records the choice of the
screen.

**M3E built on uwui's official theme.** Opium takes every generic component's look from uwui's
Material 3 Expressive crate, on the desktop and on the phone. What is proper to a music player, the
player bar, the wave of progress and the composition of Opium's pages, stays in Opium under the GPL,
built on that crate. The generic part goes to uwui under MIT OR Apache-2.0, which the owner, holding
its rights, may give. Stock and Spotify stay Opium's.

**Actions for users' agents, native, writes confirmed.** Opium declares its actions once, on uwui's
declaration; the `opium` command and users' agents share it. It is a native part, always built;
nothing answers an agent before the user has accepted it. Once an agent is accepted, it may play,
edit the queue and move through the app. Any action that changes an account at a service, a like,
a playlist, the library, is confirmed in Opium's window each time. Actions go through the paths the
app uses already, SoundCloud's browser seam among them, and no token, password or key reaches an
agent. The actions are declared at Stage 5, as the interface moves onto uwui's commands; agents on
the desktop reach them once uwui's local MCP server exists. Android's adapter, through AppFunctions,
comes at Stage 3, when the phone moves to uwui. The `opium` command, at Stage 6, runs the same
declared actions.

**macOS planned, after uwui's 1.0.** Stage 10 is no longer eventual: it comes after uwui's 1.0, at
its place in the order. The owner's virtual machine is enough to check it, as for uwui; the
documentation says that Apple's chips are not checked until a Mac with them has been. A spike of
the CDM's host verification on macOS comes first. Opium writes for macOS what uwui does not cover:
the sound through Core Audio, the CDM's host, the tokens in the Keychain, the single instance.

**uwui's bars for applications enter Opium's.** Opium's quality bar takes, now and without a figure:
its own cost above a blank window; input to photon, through the owner's light sensor; blind tests
by people; the budgets of compiling; and hours of use without a crash, counted on the machine and
never sent. Their figures are set at Stage 5, from the first measures on uwui. None is invented
before.

## Alternatives considered

- **The screen in Stock, its effects kept in Stage 6**: Stage 5 stays small, but uwui's first glass
  material is judged without its real client, or on fake data.
- **Another program as the fifth screen**: Opium unchanged, and uwui without a real program to judge
  glass on.
- **A Glass theme of Opium's own**, as SlimeEngine's ADR-0213 worded it: a fourth theme to keep,
  where the owner wants Stock, Apple Music's look, to carry the glass.
- **All of M3E in uwui, the music player's parts included**: nothing left in Opium, but uwui would
  carry components no other program needs.
- **Opium's M3E kept apart from uwui's**: nothing moves, and two M3Es drift, which ADR-0213 rejected.
- **The actions an optional piece**: a build could leave them out, at the cost of one more
  combination for CI, against [ADR-0034](0034-what-opium-is-to-become.md)'s closed list of optional
  pieces.
- **Agents that only read and play**: the safest, but "add this to my playlist" impossible.
- **macOS kept eventual until a Mac with Apple's chips**: no unchecked promise, and Opium without
  macOS while uwui supports it.
- **macOS with Windows, at Stage 4**: the Mac's public sooner, Stage 4 larger and tied to uwui's
  calendar.
- **uwui's bars left to uwui**: less work, and Opium not knowing whether it holds them itself.
- **Their figures now, copied from SlimeEngine's editor** (99.9 % of hours without a crash, 2 s for
  a change seen): clear today, but figures made for another program.

## Consequences

- [`ROADMAP.md`](../ROADMAP.md) holds these decisions: the quality bar's new rows, the fixed and open
  decisions, Stage 5's screen, M3E and actions, Stage 3's adapter, Stage 6's `opium` command on the
  same actions, and Stage 10 planned.
- [`ui-framework-needs.md`](../ui-framework-needs.md#what-uwuis-decisions-of-2026-11-07-ask) lists
  what these decisions ask of uwui, Opium's actions among them, for SlimeEngine's M6d.9b and M6d.10.
- Stage 5 grows by the fifth screen's effects and the actions declared; Stage 6 keeps the rest of
  its motion and glass.
- SlimeEngine's ADR-0213 names "Opium's Glass theme": the choice of the screen, recorded there,
  names Stock. This repository does not touch SlimeEngine's or uwui's.
- The M3E crate's calendar is uwui's; until it exists, Opium's `m3e.toml` stays as it is.
- Whether users' themes take Opium's TOML or uwui's format stays open until Stage 5, now with uwui's
  official themes as crates beside it.
- At Stage 3, before Android's adapter: the lowest Android that AppFunctions needs, and whether any
  agent calls it on GrapheneOS, without Google's services. Neither was checked here.
- Users' agents add a surface to keep safe: consent, rights, confirmation, undo and logs, tested as
  the rest. The rule that Opium talks to no new host holds: the MCP server is local.
- When an agent can attach to an Opium already running (SlimeEngine's ADR-0218), `AGENTS.md` gains a
  rule that no agent attaches to the owner's own Opium without asking.
- The test of signature asks Stock, Spotify and M3E to read as Opium's, not as uwui's look.
