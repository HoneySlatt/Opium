# 0042. CI keeps one job of checks

- Status: Accepted
- Date: 2026-11-10

## Context

[The ROADMAP](../ROADMAP.md#open) left CI's time to be decided at Stage 1: one job, or the checks
split across jobs if a run passes the owner's limit. The quality bar asks for a warm run, whose
cache hit, under 20 minutes. The [stage 1 audit](../audits/2026-11-10-stage-1.md) read CI's last ten
runs: the `check` job took 7 to 10 minutes in most of them and 22 and 25 minutes twice, cold by
their length; the `android` job beside it about 7 minutes; the weekly fuzzing 67 minutes.

## Decision

**CI keeps its form**: one `check` job running `scripts/check.sh`, the `android` job beside it, the
fuzzing on Mondays. **The checks are split across jobs only if a warm run passes 20 minutes.**

## Alternatives considered

- **Splitting the checks now** (clippy, tests and documentation in jobs of their own): a quicker
  answer when the cache is cold, for more minutes of CI, each job building again, and more to keep
  up, for a budget already met.

## Consequences

- A cold run still waits about 25 minutes.
- The figure is read again at each stage's audit; a warm run past 20 minutes opens the split.
