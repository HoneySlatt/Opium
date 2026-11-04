# 0020. D-Bus spoken over async-io, not tokio

- Status: Accepted
- Date: 2026-11-04

## Context

Several parts of the desktop app speak to the desktop over D-Bus: the keyring that keeps the
downloads' key (`oo7`, [ADR-0017](0017-downloads-sealed-and-tied-to-the-subscription.md)), the XDG
portals' file windows (`ashpd`: choosing a playlist's cover, choosing music folders), and the media
controls (`mpris-server`). `ashpd` can be built for tokio or for async-io, not for both at once,
and `oo7` brings `ashpd` with it. The choice was first forced by the window GPUI drew
([ADR-0022](0022-gpui-tried-and-abandoned.md)), whose `ashpd` was on async-io; what keeps it since
that window is gone is that the file windows wait on their answer without a runtime of their own.

## Decision

**Everything speaking to the desktop over D-Bus runs on async-io** (commits `eea0220`, `09b67ff`):
`ashpd` with its `async-io` feature in `app-desktop`, `oo7` with its `async-std` feature in
`downloads`. The keyring item is found by the same attributes as before, so the downloads' key did
not change.

## Alternatives considered

- **tokio for both**: `ashpd` on tokio needs a tokio runtime around each file window, and `oo7` on
  tokio pulls `ashpd`'s tokio feature, which cannot coexist with the async-io one.
- **Each crate on the runtime it prefers**: does not build; the feature conflict is in `ashpd`.

## Consequences

- A new D-Bus dependency is chosen, or configured, for async-io.
- The backend's own work stays on tokio ([ADR-0019](0019-the-workings-apart-from-the-drawing.md));
  the two runtimes live side by side.
