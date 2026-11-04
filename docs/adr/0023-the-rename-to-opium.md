# 0023. The app is named Opium, everywhere, with no compatibility kept

- Status: Accepted
- Date: 2026-11-04

## Context

The app was called "Music": its crates (`music-core`, `music-ffi`), its binary, its folders
(`~/.config/music`…), its names on the network, on D-Bus and in the keyring, its Android package
(`io.github.honeyslatt.music`) and its repository (`HoneySlatt/Music`). The name had been noted
as a concern before any public release: Apple's own app bears it. The owner chose "Opium", with a
new icon, on 2026-11-04. The rename was first planned for the switch to GPUI
([ADR-0022](0022-gpui-tried-and-abandoned.md)), then taken on its own. Nothing has been released:
the only installed copies are the owner's own.

## Decision

**Everything named after the app is named Opium** (commit `8e8784d`):

- the crates `opium-core` and `opium-ffi` (and every Rust path `opium_core::…`), the binary
  `opium` (still built by the package `app-desktop`), the window's title and Wayland app id,
  `packaging/opium.desktop` and its new icon `packaging/opium.svg`;
- the folders: `opium/` under the config, cache and data folders;
- the keyring item (label "Opium downloads", attributes `application=opium`, `purpose=downloads`);
- the sealed files' magic number (`OPIUMSL1`);
- Opium Connect: the mDNS service `_opium-connect._tcp`, the handshake's prologue, the iroh
  protocol `opium-connect/1`;
- the MPRIS name `org.mpris.MediaPlayer2.opium`, the PipeWire node `opium`;
- the environment variables `OPIUM_SCREENSHOT…` and `OPIUM_BROWSER`;
- the provider of the app's own playlists, `ProviderKind::Opium`;
- the Android package and application id `io.github.honeyslatt.opium`, the vault's key alias;
- the GitHub repository, `HoneySlatt/Opium`.

**The services' own words stay theirs**: Apple Music, "New Music", music videos, and Apple's genre
"Music" (`MUSIC_GENRE` in `apple-music`).

**No compatibility code**: no reading of the old folders, no old names accepted on the network or
the bus. With nothing released, the only data under the old names are the owner's, **migrated
once, by hand, on both devices together**: the folders moved, the keyring item tagged again, the
sealed files' magic rewritten (it is not authenticated: only the chunks are), and the shared state's
`Music` provider converted to `Opium`. On the phone, the new package is a new app: reinstalled,
signed in again, paired again; its downloads are lost, which the owner accepted.

## Alternatives considered

- **Keep "Music"**: the concern above stays for any release.
- **Rename with compatibility code** (read the old folders, accept both names on the network):
  with no release, there is no user to migrate but the owner, whose data is moved once instead.
- **Rename at the switch to another toolkit**, as first planned: that switch is now far off
  ([ADR-0022](0022-gpui-tried-and-abandoned.md)).

## Consequences

- An old build and a new one do not see each other on the network nor on the bus, and do not read
  each other's files or downloads.
- History before the rename (commits, ADR contexts, comments citing commit messages) still says
  "Music"; documents written after it say Opium.
- The owner's own launcher entries and system configuration outside the repository are theirs to
  change.
