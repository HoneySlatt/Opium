# 0013. Secrets, and what is kept on disk

- Status: Accepted
- Date: 2026-10-31

## Context

Opium holds the keys to its user's accounts: Apple Music's `media-user-token`, SoundCloud's OAuth
tokens and its bot guard's cookie, the Navidrome server's address, user and salted token, the
browsers' profiles with their cookies, the Opium Connect keys, and the key that seals downloads. It
also keeps the user's history, playlists and settings. An audit on 2026-10-31 found secrets that
could leave through error messages and stored addresses, and files other local users could read.

## Decision

**Tokens are kept in files only the user can read**: written with mode `0600` into folders of mode
`0700`, aside then renamed over the old one (`desktop::backend::write_private`), under the config
folder: `apple-music/media-user-token`, `soundcloud/tokens.json` and `soundcloud/datadome`,
`navidrome/credentials.json`. The sign-in browsers' profiles, the settings, the history, the
session and the shared state are made private the same way, those made before included
(commit `5abe880`).

**The Navidrome password is used once**, to make Subsonic's salted token
(`md5(password + salt)`), and kept nowhere (`crates/navidrome`).

**The key that seals downloads lives in the desktop's keyring**, the Secret Service, as one item
(`downloads::keyring`, attributes `application=opium`, `purpose=downloads`), never in a file
([ADR-0017](0017-downloads-sealed-and-tied-to-the-subscription.md)). On the phone, sign-ins and the
downloads key are in the app's vault, sealed with an Android Keystore key (`data/Vault.kt`).

**Secrets go nowhere else**:

- never printed, logged or put in an error message: SoundCloud's errors no longer carry the
  addresses asked (commit `5abe880`), a station that fails says so without its signed address
  (commit `1e8fe42`);
- a Navidrome cover's address is kept and shown unsigned, and signed only as it is fetched
  (commit `5228297`);
- Apple Music's token goes to Apple's hosts only: a redirect elsewhere is refused (commit
  `1e8fe42`).

**What is kept is written whole or not at all**: aside, then renamed into place, for the settings,
the session, the shared state, the downloads' manifest and the covers.

## Alternatives considered

- **Signed Navidrome addresses kept as they are**, as before commit `5228297`: every kept list and
  cover cache carried the server's credentials.
- **Every token in the keyring**: not recorded as weighed. Only the downloads key is there today; an
  agent proposes the move to the owner rather than making it.

## Consequences

- A copy of the config folder holds tokens: a test profile copied from it is deleted after use
  (AGENTS.md).
- Anything new that holds a secret follows the same rules; a review checks errors and logs for
  addresses with tokens in them.
- The keyring item is shared by every run on the owner's session: agents never change or delete it
  (AGENTS.md).
