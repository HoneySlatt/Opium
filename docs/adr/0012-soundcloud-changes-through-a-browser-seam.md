# 0012. SoundCloud's changes go through a browser seam

- Status: Accepted
- Date: 2026-10-31

## Context

Opium talks to SoundCloud through its web player's own API, `api-v2`, with the web player's client
id read from soundcloud.com and the user's token from a browser sign-in (`crates/soundcloud`): the
official API takes no new apps. Reading works with any HTTP client. Changes (likes, follows,
playlist edits) were refused from 2026-10-30: SoundCloud's bot guard, DataDome, answers them
`403` with its `x-dd-b` header and a captcha address.

What was tried and refused, with and without a VPN: the public `api.soundcloud.com`, which refuses
the web player's token; `api-v2` with the sign-in profile's `datadome` cookie; a client imitating
Chrome's TLS and HTTP/2 without the cookie; a `fetch` inside a headless browser on the app's profile.
What worked: a client imitating Chrome (`wreq` with `wreq-util`'s Chrome emulation) carrying a
`datadome` cookie that the user validated by passing the guard's check in the sign-in profile,
kept up to date from each answer's `Set-Cookie`. A refused request spoils the cookie; the user then
passes the check again.

## Decision

**Every change to SoundCloud goes through a `soundcloud::Browser`** (commit `6ddbdda`): the
`SoundCloud` client builds a `Change` (method, address, headers, body) and hands it to the browser
seam, which sends it and returns the `Answer`. A `403` with the guard's header or its captcha
address is recognised as the guard's refusal (`guarded`).

- **On the desktop, `desktop::soundcloud_browser::ChromeLike`** sends the change as Chrome does
  (`wreq`, Chrome emulation), with the `datadome` cookie of the sign-in profile, kept beside the
  tokens and renewed as the guard renews it. When the guard asks for a check anyway, the sign-in
  browser opens on the profile for the user to pass it, and the change is sent again once it is
  closed.
- **On the phone, a WebView no one sees sends it** (`data/SoundCloudWeb.kt`), and a check the guard
  asks for is shown full screen. SoundCloud's WebViews say they are a computer: the mobile site no
  longer keeps the token in a cookie.

**Reads stay on the plain client** (`reqwest`).

## Alternatives considered

- **SoundCloud read-only in Opium**: the owner's first choice while nothing worked; dropped once the
  seam did.
- **The public API (`api.soundcloud.com`)**: refuses the web player's token, even to read.
- **A plain client with the `datadome` cookie**: refused; the guard checks the TLS and HTTP/2
  fingerprint too.
- **Solving or avoiding the guard's checks automatically**: refused; the user passes them.

## Consequences

- The desktop builds BoringSSL from source for `wreq` (cmake, perl, git, a C++ compiler), which
  slows a first build (README).
- Each refused request can get the session flagged further: routes that were refused are not
  retried as they were, and a refusal means asking the user to pass the check in the profile's
  browser (AGENTS.md).
- A new kind of change (a playlist's cover, for instance) goes through the seam too.
