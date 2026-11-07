# 0036. The phone's CI job blocks, even when Google fails

- Status: Accepted
- Date: 2026-11-07

## Context

CI's `android` job runs `scripts/check.sh android` in `nix develop .#android` at every push to
`main` and every pull request (M1.5). The shell's Android SDK is Google's and unfree, so
cache.nixos.org does not hold it: each run downloads it from Google's server, at the versions
that `flake.lock`'s nixpkgs asks for.

On 2026-11-04 and 2026-11-05 Google's server answered 404 for two of those versions; by
2026-11-07 it served them again. During such a gap the job is red through no change of Opium's,
and AGENTS.md has a red CI on `main` fixed before any new work.

## Decision

**The job runs at every push and pull request, and a red caused by Google counts as any red**, by
the owner's decision of 2026-11-07. The remedy is to move `flake.lock`'s nixpkgs to a revision
whose `androidenv` asks for versions Google serves, with the owner's go, as it changes every tool
of both shells.

## Alternatives considered

- **Every push, but a download from Google failing does not block work**: the work goes on through
  Google's gaps, at the cost of an exception to the rule on a red CI, and of a red that people learn
  to ignore.
- **The weekly run only**, as fuzzing: about 8 to 11 minutes of the private repository's quota
  saved per push, but the code built only for Android (`cfg(target_os = "android")`), which the
  `check` job never compiles, would be checked once a week.

## Consequences

- A gap at Google may stop the work on `main` until nixpkgs is moved or Google serves the
  version again; CONTRIBUTING.md says what to do.
- Each push spends about 8 more minutes of CI, 11 when `Cargo.lock` or `flake.lock` changed
  ([measured](../measurements.md#the-android-shell-and-cis-android-job-m15-2026-11-07)).
