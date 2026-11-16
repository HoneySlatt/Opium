# 0049. The app's names, and its desktop id

- Status: Accepted; supersedes [0023](0023-the-rename-to-opium.md)
- Date: 2026-11-16

## Context

[ADR-0023](0023-the-rename-to-opium.md) named everything after the app "Opium", with no
compatibility kept for the old name "Music". It gave the desktop window the Wayland app id `opium`
(`crates/app-desktop/src/main.rs:49`, `packaging/opium.desktop`).

The [architecture audit of 2026-11-16](../audits/2026-11-16-architecture.md#before-the-first-release)
(A4) found that Flathub refuses that id. Flathub's
[requirements](https://docs.flathub.org/docs/for-app-authors/requirements), read on 2026-11-16:
an id in reverse-DNS form; for a project hosted on GitHub, `io.github.<user>.<repository>`, the
user's part in lowercase, the repository's as it is; the id must match the MetaInfo file's; a
rename means submitting the app again, and Flathub accepts few renames. A Flatpak's id also names
its data folder (`~/.var/app/<id>`). The phone's id, `io.github.honeyslatt.opium`, already follows
this form. Nothing has been released.

## Decision

**Everything named after the app is named Opium**, with no compatibility code for the old name:

- the crates `opium-core` and `opium-ffi` (and every Rust path `opium_core::…`), the binary
  `opium` (built by the package `app-desktop`), the window's title;
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

**The desktop's id is `io.github.honeyslatt.Opium`**: the Wayland app id, the `.desktop` file's
name, the MetaInfo file's id and the Flatpak's id. It replaces `opium` when the app is first
packaged for Flathub, at Stage 2, or sooner if `main.rs` is opened for a reason of its own; until
then the code keeps `opium` (`app-desktop` is frozen,
[ADR-0026](0026-the-egui-window-frozen-until-the-move.md)).

**The services' own words stay theirs**: Apple Music, "New Music", music videos, and Apple's genre
"Music" (`MUSIC_GENRE` in `apple-music`).

**No code reads the old names**: no old folder, no old name on the network or the bus. The owner's
data was moved by hand once, on both devices together, on 2026-11-04 (commit `8e8784d`); the
phone's downloads were lost then, which the owner accepted.

## Alternatives considered

- **Keep "Music"**: Apple's own app bears it.
- **Rename with compatibility code**: with no release, there was no user to migrate but the owner.
- **A desktop id on a domain of the owner's own**: it would not depend on GitHub, but costs a
  domain that must never lapse, and the phone's id would change too.
- **Keep `opium` as the desktop's id**: Flathub refuses it.

## Consequences

- An old build and a new one do not see each other on the network nor on the bus, and do not read
  each other's files or downloads.
- History before the rename still says "Music"; documents written after it say Opium.
- The desktop's id stays tied to GitHub: if the repository left GitHub, keeping the id would need an
  exception from Flathub.
- The folders inside the app keep the name `opium/`: the id changes no path outside a Flatpak.

## Validation and reconsideration

Checked at Stage 2: the Flatpak builds under this id, its MetaInfo passes Flathub's linter, and the
window shows its icon in a desktop that matches app ids to `.desktop` files.
