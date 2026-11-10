# 0044. Navidrome's password, as it is used

- Status: Accepted; amends [0013](0013-secrets-and-what-is-kept-on-disk.md) (Navidrome's password)
- Date: 2026-11-10

## Context

[ADR-0013](0013-secrets-and-what-is-kept-on-disk.md) says Navidrome's password "is used once", to
make Subsonic's salted token, and kept nowhere. The [stage 1 audit](../audits/2026-11-10-stage-1.md)
(S9) found it was not true when written: the password is also posted to Navidrome's own API to open
a session (`Navidrome::open_session`, `POST /auth/login`), which a playlist's cover needs, and the
app asks the user for it again when that session lapses. The session's token is kept with the
salted token. An address typed without a scheme became `http://`, so the password, and the salted
token, which signs in as well as the password does, crossed the network in clear.

## Decision

**The password is used, never kept**: to make the salted token at sign-in, and to open a session on
Navidrome's own API, at sign-in and each time that session lapses, when the user types it again.
It is kept in no file, no setting and no log. **The salted token and the session's token are
kept** with the server's address, as ADR-0013 keeps tokens (the desktop's `navidrome/credentials.json`,
the phone's vault).

**An address typed without a scheme is reached over HTTPS**, unless it is on the local network
(a private IPv4 address, a link-local or unique local IPv6 one, `localhost`, or a name ending in
`.local`), which is reached over HTTP, as a server at home is. A scheme typed is always followed.

## Alternatives considered

- **HTTPS always, unless `http://` is typed**: simplest and safest, but a server at home without a
  certificate fails until the user types the scheme, and the error shows in the frozen window.
- **HTTP kept, and written down**: no change of behaviour, the leak to a server on the internet
  left possible.

## Consequences

- An address kept before this change keeps the scheme it was given; nothing is migrated.
- A server on the internet typed without a scheme and serving HTTP only fails to connect; the user
  types `http://` to accept the risk.
- An address holding a user and a password (`https://me:pass@host`) is refused (S2), so the
  password never reaches a kept address.
- The owner chose this on 2026-11-10; built in M1.15.
