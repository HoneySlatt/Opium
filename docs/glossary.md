# Glossary

The terms this project defines, in alphabetical order. Each says what it means here, and links to
where it is decided or built.

**Away from home.** Paired devices that are not on the same network, reaching each other through
iroh, straight or through a relay ([ADR-0015](adr/0015-away-from-home-through-iroh.md)).

**Best copy.** Of the copies of one song or album that several sources have, the one the library
shows and plays: the device's own file, then the Navidrome server's, then Apple Music's, then
SoundCloud's ([ADR-0007](adr/0007-one-library-and-the-best-copy.md), `opium_core::dedupe`).

**Browser seam.** The way every change to SoundCloud (a like, a follow, a playlist edit) is sent:
through a `soundcloud::Browser` that sends it as the user's browser would, so that SoundCloud's bot
guard lets it through; on the desktop `desktop::soundcloud_browser::ChromeLike`, on the phone a
hidden WebView ([ADR-0012](adr/0012-soundcloud-changes-through-a-browser-seam.md)).

**CDM.** A Content Decryption Module: here Google's Widevine library, which decrypts Apple Music's
and SoundCloud's protected streams. The desktop borrows the user's browser's; the phone uses its
own through MediaDrm ([ADR-0004](adr/0004-the-widevine-cdm-borrowed-from-the-browser.md)).

**CENC.** Common Encryption: the encrypted fragmented MP4 in which Apple Music and SoundCloud serve
their protected audio (`crates/cenc`).

**Clean room.** Code written again by an agent that never read the code it replaces, from a
specification only, so that the new code owes nothing to the old one's expression; its side that
read the old code, and wrote the specification, is the *dirty side*
([ADR-0030](adr/0030-the-kopuz-code-rewritten-in-a-clean-room.md), [method](specs/README.md)).

**Expired.** The standing of a downloaded song that came through a service's DRM once that
service's subscription was not confirmed in the last 30 days, or the user signed out of it
(`downloads::store::Standing`, [ADR-0017](adr/0017-downloads-sealed-and-tied-to-the-subscription.md)).

**First useful frame.** The first frame that shows the library, not only the window: on the
desktop, Songs drawn with its rows from the cached library, as M1.14's measuring mode marks it
(`measure.rs`). Start-up is timed to the end of its work.

**Guest.** Someone on the local network who joins the queue by a code, without pairing and without
seeing the owner's data; planned at Stage 8, its trust decided by an ADR first
([ADR-0034](adr/0034-what-opium-is-to-become.md)).

**Handoff.** A queue handed from one device to another, or kept for later: its songs, their order,
the song playing and where in it, shuffle and repeat (`opium_core::Handoff`). The *session* is the
last handoff kept.

**Lend, borrow.** A song one device downloads for another over a connection of their own: the
phone borrows from the computer the Apple Music and SoundCloud songs it cannot download itself,
and seals them (`connect::lend`, [ADR-0017](adr/0017-downloads-sealed-and-tied-to-the-subscription.md)).

**M3E.** The built-in theme in Material 3 Expressive's style, with a fixed violet tonal scheme,
beside Stock and Spotify ([ADR-0011](adr/0011-themes-as-toml-files-of-component-variants.md)).

**Measuring mode.** The desktop window recording its frames and running a scripted scenario on its
own (`OPIUM_MEASURE`), so that uwui is held to the same figures; M1.14's, removed with egui
([how](../CONTRIBUTING.md#the-measuring-mode)).

**Opium Connect.** How the user's devices find each other, pair, show what each plays, control
each other, hand the music over and keep their shared state in common, on the local network and
away from home (`crates/connect`, [ADR-0010](adr/0010-opium-connect-on-the-local-network.md)).

**Opium's own playlists.** Playlists made in Opium itself, of songs from any service, kept alike on
every paired device (`ProviderKind::Opium`, [ADR-0014](adr/0014-shared-state-merged-field-by-field.md)).

**Optional piece.** A part of Opium that a build can leave out whole, as a Cargo feature, planned at
Stage 2: each online service, the Widevine layer, and six more pieces that talk beyond Opium, which
[ADR-0034](adr/0034-what-opium-is-to-become.md) lists. *Native* parts are always built, perhaps off
by default ([ADR-0034](adr/0034-what-opium-is-to-become.md)).

**Pairing.** Making two devices trust each other once: one shows a six-digit code, the user types
it on the other, and the two derive a key of their own with SPAKE2
([ADR-0010](adr/0010-opium-connect-on-the-local-network.md)).

**Playable.** What the desktop's backend hands the player for a song: an Apple Music stream
decrypting ahead, a file still downloading, a file of the computer, a station's stream, or a sealed
download (`desktop::backend::Playable`).

**Player without a screen.** `opium-headless`, a program on the same core without uwui, playing in
a room on a Raspberry Pi and driven by the other devices through Opium Connect; planned at Stage 8
([ADR-0034](adr/0034-what-opium-is-to-become.md)).

**Provider.** A source of music as the core sees it: a crate implementing `opium_core::Provider`
(Apple Music, SoundCloud, Navidrome, the device's files), with the `ProviderKind` that names it.

**Sealed download.** A song kept for offline listening, encrypted with the app's key in chunks of
64 KiB, playable only by Opium on the device that holds the key
([ADR-0017](adr/0017-downloads-sealed-and-tied-to-the-subscription.md), `downloads::sealed`).

**Shared state.** What the user's devices keep in common: the history, Opium's own playlists, the
session, when new songs were first seen, the playlists last added to, and some settings; each
device keeps its own copy and merges the others' (`synced::Shared`, `synced::Kept`,
[ADR-0014](adr/0014-shared-state-merged-field-by-field.md)).

**Signal path.** What the app shows of the way a song reaches the ears: the source's format, the
rate the PipeWire graph runs at, the other programs mixed in and the device's volume on the
desktop (`player::OutputInfo`, `player::Mixing`); the phone's own chain on Android.

**Spatial Audio.** Apple's binaural rendering of a song's Dolby Atmos mix, for headphones, which
Opium plays from the song's enhanced HLS ([ADR-0002](adr/0002-apple-music-through-its-web-player.md)).

**Specification.** A file of [`docs/specs/`](specs/README.md) saying what a part of Opium must do
(formats, requests, steps, errors) apart from its code: the only input of a clean-room rewrite.

**Station.** A radio station: live (on air now), an episode (a show that aired, from its start), or
a station of songs that the service picks one after the other (`opium_core::model::Station`).

**Stock.** The built-in theme in Apple Music's look, in Carbonfox's black with Apple Music's pink;
what a theme file leaves out is Stock's ([ADR-0011](adr/0011-themes-as-toml-files-of-component-variants.md)).

**Throwaway profile.** Config, cache and data folders of a test's own, with no session bus and no
network, on which an agent runs the app without touching the owner's data
([CONTRIBUTING.md](../CONTRIBUTING.md#a-throwaway-profile)).

**Token comparison.** The check after a clean-room rewrite: the new code's tokens compared with
the old code's and Kopuz's, raw and with identifiers made alike, every long shared run read by hand
([method](specs/README.md)).

**uwui.** The owner's UI framework, formerly slime-ui, made in SlimeEngine's repository and then
in its own, under MIT OR Apache-2.0. Opium's desktop window and phone screens move to it during
SlimeEngine's Stage 6d ([ADR-0031](adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)).

**Wake.** The callback through which the desktop's workings ask the window for a new frame when
something it shows has changed (`desktop::Wake`, [ADR-0019](adr/0019-the-workings-apart-from-the-drawing.md)).

**Workings.** The desktop app apart from its drawing: the services, the player's sources, what is
kept on disk, the media controls and the devices, in `crates/desktop`
([ADR-0019](adr/0019-the-workings-apart-from-the-drawing.md)).
