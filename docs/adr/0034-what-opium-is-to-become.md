# 0034. What Opium is to become

- Status: Accepted; amends [0008](0008-the-lyrics-sources.md) (lyrics word by word, translated, kept
  offline; the sites besides LRCLIB an optional piece),
  [0009](0009-the-android-app-over-the-rust-core.md) (the lowest Android decided with uwui at Stage
  3; bit-perfect USB as an explicit option), [0010](0010-opium-connect-on-the-local-network.md)
  (resuming and handing over at their best; a guest on the local network, its trust designed first),
  [0011](0011-themes-as-toml-files-of-component-variants.md) (the Spotify theme renamed before a
  public release; the format of users' themes decided at Stage 5; high contrast through uwui's
  theme), [0013](0013-secrets-and-what-is-kept-on-disk.md) (every desktop token in the system's
  keyring), [0014](0014-shared-state-merged-field-by-field.md) (every play kept and shared),
  [0016](0016-the-services.md) (Jellyfin and Plex to come; each online service an optional piece;
  five outside services accepted, off by default),
  [0017](0017-downloads-sealed-and-tied-to-the-subscription.md) (a collection's available part
  shown; an offline mode turned on by hand), [0025](0025-the-quality-bar.md) (the lowest desktop
  named after a real machine; a tier without a screen; a library of 100,000 songs),
  [0029](0029-opium-waits-for-slime-ui-then-moves-first.md) (the order after the phone) and
  [0031](0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md) (the lower tiers); amended by
  [0037](0037-uwuis-decisions-of-2026-11-07-taken-into-opium.md) (glass and its motion earlier for
  one screen; actions for users' agents; macOS planned after uwui's 1.0)
- Date: 2026-11-06

## Context

Opium's decisions so far answered the needs of the day: a service, a format, a framework. What Opium
must become in the end, for whom, on what, doing what, had never been asked whole. Two things made
it pressing. Opium is the first program ported to uwui, the owner's UI framework, during
SlimeEngine's Stage 6d ([ADR-0031](0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)), and
that stage's milestone M6d.1b3b is about to decide how an application's state changes its interface:
it should know its first client's needs before it decides. And a long-term direction lets each later
stage be planned against it, not against the last request.

The owner answered on 2026-11-06, theme by theme, in a design session that built nothing. The
questions came from the ADRs' open points, the [known debt](../debt.md), a survey of the best
players on the market (Apple Music, Spotify, Tidal, Qobuz, Plexamp, Roon, foobar2000, Strawberry,
Cider, Symfonium, Poweramp, Feishin, Navidrome's own features) and the owner's other projects
(uwui's requirements in SlimeEngine's ADR-0190, its screens of reference in SlimeEngine's ADR-0198,
and Slatty, the owner's game launcher). Opium was on standby
([ADR-0031](0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)); the owner lifted it for this
session, which touches documents only.

## Decision

**Who it is for.** The general public: made first for its owner, built so that anyone can install
and understand it. It plays music only: no podcasts, audiobooks or internet radio stations; the
services' own stations and Apple Music's live radio stay. One person per installation, no profiles.
Opium has no account of its own, ever: only the services' accounts and the devices paired together.
It serves four ways of listening equally: albums from start to finish, discovery, playlists in the
background, and lyrics.

**Platforms.** Linux and Windows, then macOS once uwui supports it and a Mac can check it. Android;
iOS is not planned but kept possible, its core portable (on an iPhone, Apple Music would go through
MusicKit, Apple's own route). Android Auto is kept. A player without a screen, on a Raspberry Pi,
plays in a room and is driven by the other devices. The lowest Android is decided at Stage 3, as low
as uwui allows, aiming at Android 10 or 12 for the public. Tablets and the web are not planned.

**Services and sources.** Jellyfin and Plex join the four sources one day. A service stays a crate
of Opium's, reviewed and tested; no service comes through an extension. MusicBrainz and the Cover
Art Archive may complete the files' tags, covers and credits, when the user turns them on. The
formats read today suffice (FLAC, ALAC, WAV, AIFF, MP3, AAC, Ogg Vorbis, Opus): no DSD, CUE or MQA,
and no object-based Dolby Atmos, which nothing in the owner's chain renders (Apple Music's binaural
Spatial Audio stays).

**Discovery.** From the services' own stations and suggestions, and from radios computed on the
device over the user's history and files. ListenBrainz and Last.fm receive what is played
(scrobbling), each turned on by the user, with a queue that waits for the network; their suggestions
may feed discovery.

**Opium Connect.** It already shares the history and playlists, shows what each device plays, drives
it and hands the queue over ([ADR-0010](0010-opium-connect-on-the-local-network.md)). The owner
wants resuming and handing over at their best: playback resumed on whichever device is opened next,
at the song and the position left, without a gesture; a song handed over live without a gap the ear
notices. A guest on the local network may join the queue by a code, as the owner chose, without
being paired and without seeing the owner's data; its trust model is designed and recorded first,
the code included. Sound may go to UPnP and DLNA renderers; a protected song sent there leaves the
app decrypted, so whether it may is decided against its licence's output rules when UPnP is built.
Away from home, n0's relays by default, the user's own as an option, as
[ADR-0015](0015-away-from-home-through-iroh.md) has it. No Chromecast, AirPlay or rooms played in
sync.

**The library.** It holds 100,000 songs without slowing. Smart playlists, by rules, with a visual
editor; statistics and a year in review, computed on the device; favourites of Opium's own, shared
between devices, no star ratings. Every play is kept and shared, not only the latest 100 songs, in a
format versioned and migrated. Tags, which Opium edits today in place ([known debt](../debt.md)),
are written aside and renamed, the old file kept a while, the change undone on demand. Everything
exports at any time in open formats (JSON, M3U).

**Sound.** Gapless stays the default, and the sound reaches the card unchanged by default where the
platform allows it, as the desktop does today. Three treatments are offered: loudness levelled by
ReplayGain or EBU R128, by track or album, without clipping; a smart crossfade that spares albums
without gaps; a parametric equaliser with AutoEq's profiles, one per headphone or output. The signal
path, shown today, will name each treatment in use; when none is, it proves the sound unchanged. On
the phone, bit-perfect USB output comes as an explicit option, off by default. Android 14 brought
it, and Opium does not use it yet. The option says that it silences other sounds: it is the one
exception decided so far to the principle that other programs keep playing. Below Android 14,
everything works but this option. No visualiser, no voice removal.

**Lyrics and daily use.** Lyrics word by word where the source gives it, as Apple Music's, the
files' and the sites' are today; Navidrome gives them from its 0.63 (its release notes), which Opium
does not read yet. Lyrics are translated and transliterated where the service gives that, and kept
with downloads for offline use. A sleep timer on the desktop, as the phone has; a richer queue
(saved queues, undo, no limit of 100, beside today's play next and at the end); a shuffle that
avoids repeats without slipping recommendations in.

**The interface.** Opium keeps its own themes on uwui (Stock, Spotify, M3E), built from uwui's
components; the Spotify theme is renamed before a public release, its look kept. The desktop and the
phone keep two interfaces, both on uwui, over the same Rust core. What counts most: the cover flying
from one screen to the next, gestures that drive animations and interrupt them without a jump, glass
materials, haptics on the phone. On the desktop, a mini-player and an icon in the system's tray.
When state changes while the user looks (a library syncing, another device editing the queue, search
results arriving service by service), the screen changes in place, animated, what the user looks at
and the selection kept. At start, Opium returns where the user was: besides the song, its position
and the panels, which come back today, the page, the scroll, the selection and the window's size. A
theme file reloads live, as today; no theme editor, no gallery of users' themes, no couch mode.
Whether users write themes in Opium's TOML or uwui's format is decided at Stage 5, once uwui has
one.

**Accessibility and language.** WCAG 2.2 at level AA, checked. One high-contrast theme, uwui's,
chosen when the system asks. Every shortcut remappable, with chords, and a palette of commands. No
global shortcuts beyond the media keys. The text is made ready for translation from the move to
uwui; English and French on the desktop at the first public release, on the phone at Stage 3, other
languages after.

**Offline.** Downloads by hand, an album or a playlist, showing what part of it can be kept (as "23
of 30"). An offline mode turned on by hand shows and plays only what is available and tries no
network. Downloads stay sealed and tied to the subscription
([ADR-0017](0017-downloads-sealed-and-tied-to-the-subscription.md)).

**Machines.** The lowest desktop is the owner's very modest laptop, an HP Laptop 17-ca2xxx (AMD
Athlon Silver 3050U, its Vega GPU, 4 GiB on one channel), the one uwui measures first, in place of a
2015 laptop nobody owns. The lowest phone is uwui's entry-level phone, today an Android 14 phone of
the Helio G99's class. The player without a screen is measured on a Raspberry Pi 5, bought when its
stage comes.

**Privacy and safety.** No telemetry, not even anonymous: nothing about the app's use is sent
anywhere, and a crash report stays on the machine, attached by hand, as the [quality
bar](../ROADMAP.md#quality-bar) already asks. Every token of the desktop moves to the system's
keyring (Secret Service, Windows' credentials), as the phone's sign-ins are already in the vault
that Android's Keystore seals. Opium asks for the fewest rights: the desktop's portals, Flatpak's
sandbox, no superfluous Android permission.

**Distribution and updates.** On Linux, Flathub and Nix; on Android, a signed APK on GitHub, which
Obtainium can watch for new releases. On Linux and Android, Opium never looks for updates itself:
the stores, Obtainium or the user update it. On Windows and macOS, the app updates itself. The name
Opium is kept, checked against the chosen stores' rules before the first release.

**Extensions and control.** Themes stay the only extension until after 1.0, when uwui's isolated
WebAssembly extensions are weighed. A command line, with commands such as `opium play`, `opium next`
and `opium queue add`, drives the running app from a terminal or a script, on the same machine.
Discord may show the song playing, turned on by the user. Connect, synchronisation and sealed
downloads stay in Opium until a second program needs them; Navidrome stays the server, the player
without a screen serves nothing.

**Around the system.** On the phone, a home-screen widget and a tile in the quick settings, beside
the sharing of a song's link that it has today. On the desktop, files dropped on the window under
Wayland too (opening them already works), a notification when the song changes (off by default), a
start with the session. No deep links (`opium://` addresses).

**Native or optional.** Native means always built, perhaps off by default; optional means a piece a
build can leave out whole. The device's files and LRCLIB are native. Apple Music, SoundCloud,
Navidrome, Jellyfin, Plex and the Widevine layer are optional pieces, built by default: a public
build without DRM, or without a service that breaks, needs no change elsewhere. Six more pieces are
optional, off by default when built: ListenBrainz, Last.fm, MusicBrainz with the Cover Art Archive,
Discord, the lyrics sites besides LRCLIB, and UPnP. This list is the whole of the optional pieces.
Everything else is native, Opium Connect with iroh's relays included: the queue, the timer, the
shuffle, the three treatments, the signal path, bit-perfect USB, the smart playlists, the
statistics, the favourites, the tags, the export, resuming and handing over, the guest, translation,
accessibility, the shortcuts and the palette, the mini-player, the tray, the notification when the
song changes, the widget and the `opium` command. Of these, the three treatments, bit-perfect USB,
the guest and the notification when the song changes are off by default. The player without a screen
is a program of its own on the same core, `opium-headless`, without uwui.

**Order.** The first public release, 0.1.0, is today's Opium with its desktop on uwui, clean,
translated into French and accessible, with Stage 2's decisions; the phone, still on Compose then,
gets French with uwui at Stage 3. New functions come after, release by release: after the phone on
uwui, listening first (with a spike of Widevine on Windows already in Stage 2), then Windows, then
the library, then Connect with the player without a screen, then Jellyfin and Plex; macOS when uwui
allows. Of the new needs, only those the quality bar asks of 0.1.0 belong to the move to uwui, and
so to the end of SlimeEngine's Stage 6d: WCAG 2.2 AA with the high-contrast theme, the translation,
remappable shortcuts, and lists of 100,000 rows changed in place. The cover flying, the gestures,
glass, haptics and the full return at start come with the listening, at Stage 6.

**Left open**, each decided when it becomes necessary:

| Decision | Decided at |
|---|---|
| What a public release exposes of DRM, now that the release aims at the public | Stage 2 |
| The name and the Spotify theme's new name, against the chosen stores' rules | Stage 2 |
| Whether Flathub accepts an app that borrows the browser's CDM, and the CDM from inside the sandbox | Stage 2 |
| The lowest Android version, as low as uwui allows | Stage 3 |
| The format of users' themes, Opium's TOML or uwui's | Stage 5 |
| How a crossfade meets two tracks at different rates, against [ADR-0005](0005-pipewire-at-each-tracks-own-rate.md) | Stage 6 |
| How Windows and macOS update the app: uwui's optional crate (SlimeEngine's ADR-0199) or Opium's | Stage 4 |
| The guest's trust model, and the rule that Connect hears only paired devices | Stage 8, by an ADR |
| Protected songs on UPnP, and on the player without a screen (a CDM on ARM Linux) | Stage 8 |
| macOS: its stage, its CDM's host verification | When uwui supports macOS |
| What 1.0 holds | At Stage 9 |
| Extensions, on uwui's WebAssembly | After 1.0 |

## Alternatives considered

- **Opium for its owner first, open to others like them, or for the owner and close friends
  alone**: fewer duties of support, translation and safety towards strangers, and the DRM question
  stays small. The owner chose the general public.
- **One interface for the desktop and the phone**, adapting to size and input: a function written
  once, but each screen thought for both; the owner kept two interfaces.
- **More services now (Qobuz, Tidal, Bandcamp)**: true lossless for audiophiles, but unofficial
  access, possible DRM and blocking; Jellyfin is open and Plex's access official, neither with DRM.
- **Services through extensions**: more services without the project, but a stable API to keep, and
  extensions that could get around protections.
- **An account of Opium's**: backups without a device switched on, but a server to host and
  personal data to keep.
- **Telemetry, anonymous and opt-in**: real reports, but a server and a privacy policy; the user
  attaches a crash report by hand instead.
- **Profiles, a guest mode whose plays leave the owner's history untouched, Chromecast, AirPlay,
  rooms in sync, podcasts, radio stations, voice removal, a visualiser, a theme editor, a couch
  mode, star ratings, DSD**: each weighed against its cost and left out.
- **Every piece native**: one build to test, but no public build without DRM without changing code.
- **The player without a screen as a mode of Opium**: one program, but it would carry the whole
  interface onto a Pi.
- **Windows right after the phone**, or after everything: the public of Windows sooner, or a
  perfect Linux first; the owner put it after the listening, with its risk known early.

## Consequences

- AGENTS.md's project line says Opium is aimed at the general public. The ROADMAP gains its
  direction, its stages 6 to 10 and its open decisions; the quality bar its tiers and its library
  of 100,000 songs.
- [`ui-framework-needs.md`](../ui-framework-needs.md#what-adr-0034-asks-of-uwui) lists what these
  needs ask of uwui, the general apart from Opium's own, for SlimeEngine's Stage 6d to read. The
  owner carries it there; this repository does not touch SlimeEngine's or uwui's.
- Each optional piece listed above becomes a Cargo feature, and CI builds the combinations that
  ship.
- Keeping every play changes the shared state's format: versioned, migrated, the merge still
  ignoring order and repetition, `crates/synced/src/tests.rs` extended
  ([ADR-0014](0014-shared-state-merged-field-by-field.md)).
- Moving every token to the keyring needs a migration, and a fallback where no keyring answers.
- The guest breaks today's rule that Connect hears only paired devices: the rule holds until the
  guest's ADR replaces it.
- The phone's bit-perfect USB output is an exception, written down, to the principle that other
  programs keep playing; Windows' exclusive mode stays weighed at Stage 4.
- Five new outside services are accepted, each off by default: ListenBrainz, Last.fm, MusicBrainz,
  the Cover Art Archive and Discord. Jellyfin and Plex are servers the user chooses; UPnP speaks to
  devices on the local network.
- Tags are written in place today, against the rule that writes go aside and are renamed: recorded
  in the [known debt](../debt.md), a loss of data the standby allows fixing, proposed to the owner
  as the next fix.
- The public target raises the weight of Stage 2's DRM decision, of the name, and of the stores'
  rules; none of them is decided here.
- The phone's lowest tier is the same class as before; if the lowest Android goes below 14 at
  Stage 3, the reference phone changes with it.
