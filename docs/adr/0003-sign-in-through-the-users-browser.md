# 0003. Signing in through the user's browser, on a profile of Opium's own

- Status: Accepted
- Date: 2026-10-29

## Context

Apple Music and SoundCloud keep the user's token in cookies of their web players
([ADR-0002](0002-apple-music-through-its-web-player.md), [ADR-0012](0012-soundcloud-changes-through-a-browser-seam.md)).
Their sign-in pages are full web flows, with two-factor checks and captchas, which a program cannot
fill in itself. Chromium's cookie store on disk is encrypted, with a format and a key storage that
keep changing.

## Decision

**On the desktop, the user signs in in a Chromium-family browser that Opium starts on a profile of
its own** (`crates/browser-signin`): Brave, Chromium, Chrome, Vivaldi or Edge, the first found on
`PATH`, or the one `OPIUM_BROWSER` names. The profiles are kept under the config folder,
`apple-music/browser-profile` and `soundcloud/browser-profile`, readable by the user only.

**The cookies are read live from the browser's DevTools, over `--remote-debugging-pipe`**: two
pipes only the browser and Opium hold, not a port another program on the machine could connect to
during the sign-in (commit `f073144`). Opium asks for the cookies until the token shows up, then
keeps it in a file only the user can read ([ADR-0013](0013-secrets-and-what-is-kept-on-disk.md)).

**The profile is kept between sign-ins**, so that a browser still signed in hands over a fresh
token without typing. Signing out deletes the tokens, the profile and the library kept from the
account (commit `4bf0551`).

**On the phone, the sign-in runs in a WebView** (`ui/connect/Connect.kt`), and the tokens are kept
in the app's vault, encrypted with an Android Keystore key (`data/Vault.kt`).

## Alternatives considered

- **Reading Chromium's cookie database on disk**: its encryption and format change between
  versions; reading the cookies live avoids both.
- **DevTools over a TCP port** (`--remote-debugging-port`), the first version: any local program
  could have connected to it during the sign-in.
- **Asking the user to paste a token**: error-prone, and it hands secrets through the clipboard.

## Consequences

- A Chromium-family browser is a requirement of the desktop app (README).
- The SoundCloud profile is also the one whose `datadome` cookie lets changes through
  ([ADR-0012](0012-soundcloud-changes-through-a-browser-seam.md)); deleting it signs the user out
  of more than the token.
- Signing in acts on the owner's real account: an agent never does it on its own (AGENTS.md).
