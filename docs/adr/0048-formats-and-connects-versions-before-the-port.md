# 0048. Formats and Connect's versions before the port

- Status: Superseded by [0051](0051-what-opium-builds-while-it-waits-for-uwui.md)
- Date: 2026-11-16

## Context

The [architecture audit of 2026-11-16](../audits/2026-11-16-architecture.md#before-the-first-release)
found three gaps that a release makes permanent:

- **A1.** The shared state, the pairings, the caches and the settings carry no version. The debt
  had them wait for Stage 7, after 0.1.0.
- **A2.** Two devices talk only when their Connect `VERSION` is the same
  (`crates/connect/src/lib.rs:1035`). The phone and the computer update at different times.
- **A3.** `ProviderKind` has no fallback for a service it does not know: an older device drops a
  whole message that names a service added later, and puts aside a whole `shared.json` that
  holds one.

Until the port, [ADR-0035](0035-the-port-in-uwuis-first-stage-stage-1-finished-first.md) and
`AGENTS.md` allow only fixes for a loss of data, a crash, a security flaw or a broken service,
unless the owner asks otherwise. Opium has no user but its owner yet. The port to uwui will change
formats of its own (Stage 5).

## Decision

**The three gaps are closed now, in a milestone of their own while Opium waits for uwui**, by the
owner's decision of 2026-11-16. This is the owner asking otherwise, as `AGENTS.md` allows; the
other limits on the wait stay.

**The conventions are decided at the milestone's start**, each a decision for the owner and an ADR
of its own where a newcomer would question it: how a file names its version, what a build does with
a file newer than it knows, how two devices agree on a version, how an older device keeps a
service it does not know without losing it.

## Alternatives considered

- **At the start of Stage 2, before 0.1.0**: no work during the wait, but the port, which comes
  first, would change formats that have no version yet.
- **Connect's range and the unknown service now, the files' versions at Stage 2**: less work now,
  the port still made over files without a version.
- **Stage 7, as the debt had it**: after 0.1.0, each change of format would have to guess which
  version wrote a file on the users' data.

## Consequences

- One milestone before the port, in [`PLAN.md`](../PLAN.md); the debt's item *Formats without a
  version* is tracked there.
- Only the owner's data is migrated: the cheapest time there will be.
- The owner installs the new desktop and the new APK together once, when the milestone ends, so
  that both speak the new version.
- The format changes of Stage 5 become migrations from a known version.

## Validation and reconsideration

The milestone is checked against the audit's scenarios: a phone one version behind the computer;
an upgrade, then a downgrade; a service added. Each becomes a test on test data. The decision would
be reconsidered only if the work grew beyond one milestone, which is put to the owner before it is
split.
