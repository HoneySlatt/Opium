# 0038. Opium measured against the market

- Status: Accepted; amends [0025](0025-the-quality-bar.md) (rivals named and beaten by a margin;
  frames, sound, battery, library and handoff given figures), [0034](0034-what-opium-is-to-become.md)
  (transitions, notes, quality shown, the library's tools, Android Auto, the watch; Wikipedia and
  Wikidata among the optional pieces) and [0037](0037-uwuis-decisions-of-2026-11-07-taken-into-opium.md)
  (what users' agents may touch)
- Date: 2026-11-08

## Context

The owner wants Opium to be the best music player on the market, and asked on 2026-11-08 for its
quality bar, budgets, ambitions and functions to be set against the market, as SlimeEngine's
ADR-0215 did for the engine. What held before:

- **Budgets without rivals.** [ADR-0025](0025-the-quality-bar.md)'s budgets are absolute. Only the
  phone's battery is compared, with Spotify's. Beating a line by 1 % passes.
- **Where Opium stands.** On the desktop, Opium under egui already does better than Spotify and
  Cider on its start, memory, rest and a song playing (uwui's `measurements.md`). On the phone, it
  used 1,500 to 1,900 ticks of CPU a minute for an Apple Music song on 2026-11-03, against under
  844 for Spotify ([measurements](../measurements.md#the-phone-2026-11-03)). At rest, the desktop
  wakes 8.4 times a second: Opium Connect 3.8, a thread of the `blocking` crate 2, the theme's watch 1
  and the player 1 ([measurements](../measurements.md#1-at-rest-cpu-time-and-wake-ups)).
- **The market, surveyed on 2026-11-08.** No player publishes reproducible figures on the time to
  first sound, the gap between tracks, the battery or a handoff between devices. Gaps are reported
  by users of Apple Music and Spotify, never measured. Apple Music's AutoMix (iOS 26) blends songs on
  their tempo and beat, and skips a blend that would not be clean. Spotify, Amazon and YouTube Music
  build playlists from a request in words; Plexamp removed its own as "not consistent or useful
  enough". Qobuz and Roon lead on notes and credits; Tidal, Qobuz and Apple Music show each track's
  quality; MusicBee and foobar2000 on the care of a collection; Symfonium on Android Auto and on
  sound for Android. Spotify published a median playback latency of 265 ms (Kreitz and Niemelä,
  IEEE P2P 2010).
- **Limits found by a review of the proposal**, by a fresh agent:
  - A window drawn on the GPU already costs its driver's memory, which a light GTK player does not
    pay.
  - A cold Apple Music song needs at least four round trips to Apple before any sound: about 120 to
    240 ms. Searching Apple's catalogue goes through the API that Apple's own player uses.
  - Timing a rival's first sound by recording its output would capture protected sound, against
    AGENTS.md's DRM rules and the services' terms.
  - On a display of fixed rate, a late frame shows one refresh later: a bound of 1.5 frames means
    none late.
  - Android Auto hides an app not installed from Google Play unless the user turns on its
    developer setting "Unknown sources". Opium ships a signed APK on GitHub
    ([ADR-0034](0034-what-opium-is-to-become.md)).
  - Apple states that its cloud library holds up to 100,000 songs (not checked here).

## Decision

The owner's decisions of 2026-11-08.

**Rivals in two families,** each measured from outside on the owner's desktop and Pixel, each
figure with its rival's version, the client measured and its date; the lists are confirmed when
measured:

- resources and sound, against native players: on the desktop DeaDBeeF, Strawberry, Elisa,
  Audacious, Amberol, Supersonic and foobar2000; on the phone Poweramp, Neutron, Symfonium and
  Gramophone;
- experience and functions, against Spotify, Apple Music, Tidal, Qobuz and Roon.

The best rival on each line sets the bar. A measure on the owner's accounts waits for the owner's
consent. foobar2000 is measured under Wine, or on Windows itself at Stage 4, never in a virtual
machine, whose timers and GPU falsify the figures. A rival's terms are checked before its figure is
committed; each figure comes with its method and its raw data.

**The margin, by family.** Against the natives, on memory, the processor while playing, wake-ups at
rest, the battery with the screen off and the start: at least as frugal as the best, line by line,
and equal to the floor where one exists, 0 at rest. Memory counts resident memory and video memory
together; it is compared directly with the natives drawn on the GPU (Amberol, Elisa), and with the
others above uwui's blank window ([ADR-0037](0037-uwuis-decisions-of-2026-11-07-taken-into-opium.md)).
Against the giants, on what depends on Opium alone (a file's or a cached song's first sound, the
next song, a seek, a page opened, a search of the library on the device): 2.5 times faster than the
best. On the network, Opium adds next to nothing above the round trips it cannot avoid, which it
measures and logs. The giants are timed by hand on a test account, their sound never recorded.

**Sound, proven on files.** On files and Navidrome, in a test graph of PipeWire, never the owner's
speakers, and in CI where it runs: no sample of gap or overlap between two tracks of an album, MP3's
and AAC's delay and padding read from their tags; the output identical to the file, sample by
sample, on the desktop up to 24 bits, at full volume, Opium the only stream, and a lossy file against
Opium's own reference decode; no underrun under load. On the phone, identical only through
bit-perfect USB. Protected songs are judged by the signal path and PipeWire's graph, never captured.

**Frames.** On every tier at its display's rate, the desktop's and the phone's: 99.9 % of refreshes
get a new frame, a late frame is never more than one refresh late, and none is missed when a page
first shows, everything it needs prepared before. Measured in scripted phases of a stated length,
with the work of each frame at the 99.9th percentile within a share of the frame set at Stage 5.

**The phone's battery, by source,** with the screen off. Files and Navidrome: at least as frugal as
the best native, the sound offloaded to the chip. Protected songs: at least as frugal as Apple
Music's own Android app on the same song, and as Spotify. Measured first by Perfetto's power rails,
with a stated margin of noise; hours on one charge checked from time to time. Connect's keepalives
and each treatment of the sound get a budget of their own.

**The library.** Every budget holds with 100,000 songs on every tier, and with 500,000 on the high
tier. The 500,000 come from files and Navidrome, generated for the test. The first scan has a budget
of its own, apart from the start.

**Handoff between devices.** The music resumes within 10 ms of where it was left, with at most
50 ms of silence or overlap between the two devices. Measured on files from both devices' output
timestamps, on wired outputs or outputs of known latency, the clocks' error stated; no sound
captured. Bluetooth gets a bound of its own. The target device fetches a protected song's licence
before the handoff. Checked at Stage 8.

**Transitions at AutoMix's level.** Computed on the device from tempo, beat and energy; off by
default; never inside an album without gaps; none where it would not be clean, nor where the next
song has not been analysed. The next song is analysed while it preloads, in memory; only its tempo
and markers are kept, never sound. A transition turns the phone's offload off, and the signal path
says so. Judged blind by people against AutoMix on the same songs, listened to live, never recorded,
the owner's speakers used only with the owner's consent at the time. This replaces the smart
crossfade of [ADR-0034](0034-what-opium-is-to-become.md).

**No AI built in.** Playlists asked for in words are made by the user's own agent, through Opium's
declared actions, beside the radios computed on the device.

**Notes around the music.** The services' editorial notes, MusicBrainz's credits, and Wikipedia's and
Wikidata's biographies and album notes. Wikipedia and Wikidata become one optional piece, off by
default, added to ADR-0034's list. An article is found from the MusicBrainz identifier through
Wikidata, never by name alone; its authors and licence, CC BY-SA, are shown with it; what the option
tells Wikimedia, the artists listened to, is said where it is turned on.

**Quality shown everywhere, discreetly:** in the lists, the album pages and the song playing, in
plain text, of what is actually decoded (as "FLAC 24/96", "AAC 256", "Spatial"), and which copy of a
song plays ([ADR-0007](0007-one-library-and-the-best-copy.md)). No brand's logo. The signal path
keeps the detail.

**The library's tools.** Duplicates found between files and between sources; tags proposed by
MusicBrainz, reviewed album by album, every track shown, each write undone on demand, the old file
kept a while. On the device's own files only. No CD ripping. A fingerprint service (AcoustID) is
decided apart, if ever.

**Android Auto complete,** within Google's rules for driving: the library browsed, voice search, text
search when parked, the queue, the favourites, every source, protected songs included. It stays for
those who find Android Auto's "Unknown sources" setting: Opium ships no store build for it and does
not document the setting.

**The watch.** Wear OS's own media controls, checked to work with Opium. An app of its own is weighed
after 1.0, once uwui says whether it aims at watches.

**Users' agents** ([ADR-0037](0037-uwuis-decisions-of-2026-11-07-taken-into-opium.md)): they may ask
for a download or its removal, confirmed in the window each time, and never touch a setting.
Opium's own playlists and favourites change without confirmation; deleting a playlist is always
confirmed, as a shared deletion is final ([ADR-0014](0014-shared-state-merged-field-by-field.md)).
An undo is a change that compensates, kept in a journal that survives a restart, never overwriting
a newer edit from another device. An agent makes a bounded number of changes in a row. What an
action returns marks the text that comes from outside (titles, notes) as untrusted. "Favourite"
means Opium's own; a service's like stays an account change. Opium says, where an agent is
accepted, that what the agent reads goes to its provider.

## Alternatives considered

- **Spotify and Apple Music alone as rivals**, or **budgets kept absolute**: less to measure, but
  Opium already beats the giants on resources, and the natives beat them on sound and frugality.
- **The natives alone**: no bar for streaming and Connect, which they lack. **One list for all**: a
  local player compared on lines that mean nothing to it.
- **1.5 times more frugal than the best native**: memory against GPU-less players, a debt no work
  would settle. **Beating each line**: a lead of 1 % passes.
- **2.5 times everywhere, with preloading**: more network, battery and licences asked for songs never
  played. **Beating each line on the network**: 1 % passes.
- **Sound proven by hand only**: a regression slips between milestones. **The signal path alone**:
  a claim.
- **99 % of frames and up to two frames**: one stutter in a hundred, visible at 240 Hz. **Every
  frame on time**: one hitch of the system fails it.
- **1.5 times the best battery**, from twice worse today; **no worse than Spotify**: the natives do
  better on files.
- **A million songs everywhere**: memory short on the 3.25 GiB laptop, and an index on disk to
  design early. **100,000 alone**: the largest collections left out.
- **A handoff to the sample**: nearly the rooms played in sync that ADR-0034 left out. **Words
  alone**: nothing measurable.
- **AutoMix with editable curves**: one more editor. **The simple crossfade**: below Apple Music
  today.
- **A local model built in**: hundreds of MB, slow on the laptop, where Plexamp failed. **No
  playlists in words**: below the giants.
- **Notes from the services and MusicBrainz alone**: no biography for files and Navidrome.
- **Quality on the song playing alone**, or **the signal path alone**: the copy that will play not
  seen beforehand.
- **CD ripping too**: a new service and a disc drive to test.
- **Android Auto's essentials**, or **as today**: below Symfonium and Spotify. **Google Play**:
  Android Auto for everyone, but a developer account, the review of car apps, and a real risk of
  refusal for Apple Music and SoundCloud. **The setting documented**: more reach, the owner chose
  not to.
- **A Wear OS app now**: Compose for Wear OS, outside uwui, screens of its own to keep.
- **Agents with settings**, confirmed: a confirmation misread opens an outside service. **Opium's
  playlists confirmed**: an agent tiresome for the commonest use.

## Consequences

- [`ROADMAP.md`](../ROADMAP.md#quality-bar) holds the rivals, the margins and every figure above;
  the functions go to Stages 3, 6, 7 and 8.
- Rest is broken today by Connect's timers, a thread of the `blocking` crate, the theme's watch and
  the player's timer: rest is measured with every session closed, and each cause is fixed or given a
  stated budget when its code is touched. A CDM session left open while paused is closed.
- Measuring the natives and the giants needs tools: a library of 500,000 generated files, a test
  graph of PipeWire in CI, timestamps on both devices, Perfetto's rails.
- Publishing a rival's figures waits for its terms to be read; SlimeEngine's ADR-0214 sends the same
  question to a lawyer.
- AutoMix's blind test needs an Apple device with Apple Music, lent, and the owner's speakers at the
  time; until then, transitions are judged blind on files only.
- [ADR-0034](0034-what-opium-is-to-become.md)'s six optional pieces off by default become seven,
  Wikipedia and Wikidata added; ADR-0005's open question, a crossfade between two rates, now covers transitions.
- Android Auto reaches few users: Opium ships no store build, and Google Play is not planned.
- No bar was set for the hours without a crash, which ADR-0037 sets at Stage 5.
