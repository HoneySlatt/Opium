# 0036. The phone's CI job blocks, even when Google fails

- Status: Accepted
- Date: 2026-11-07

## Context

CI's `android` job runs `scripts/check.sh android` in `nix develop .#android` at every push to
`main` and every pull request that changes more than Markdown, and weekly (M1.5). The shell's
Android SDK is Google's and unfree, so cache.nixos.org holds none of it but an old `sdk-tools`
archive: each run downloads the rest from Google's server. `flake.nix` pins the NDK, the
build-tools and the platform; nixpkgs' `androidenv`, pinned by `flake.lock`, picks the other
tools' versions (CMake, the command-line tools, platform-tools).

On 2026-11-04 and 2026-11-05 Google's server answered 404 for two of those versions; by
2026-11-07 it served them again. During such a gap the job is red through no change of Opium's,
and AGENTS.md has a red CI on `main` fixed before any new work.

## Decision

**The job runs at every push and pull request, and a red caused by Google counts as any red**, by
the owner's decision of 2026-11-07. The remedy, with the owner's go, is to move the version
Google no longer serves where it is chosen: in `flake.nix`, to a version `androidenv` lists, or
in `flake.lock`, to a nixpkgs whose `androidenv` asks for versions Google serves, which changes
every tool of both shells.

## Alternatives considered

- **Every push, but a download from Google failing does not block work**: the work goes on through
  Google's gaps, at the cost of an exception to the rule on a red CI, and of a red that people learn
  to ignore.
- **The weekly run only**, as fuzzing: 7 to 10 minutes of the private repository's quota
  saved per push, but the code built only for Android (`cfg(target_os = "android")`), which the
  `check` job never compiles, would be checked once a week.

## Consequences

- A gap at Google may stop the work on `main` until the version is moved or Google serves it
  again; CONTRIBUTING.md says what to do.
- Each push spends 7 more minutes of CI, 10 when the job's cache starts cold: when `Cargo.lock`,
  a manifest of the workspace, `scripts/check.sh` or the Android shell changed
  ([measured](../measurements.md#the-android-shell-and-cis-android-job-m15-2026-11-07)).
