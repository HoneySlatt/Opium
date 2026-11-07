# 0011. Themes as TOML files that choose a variant of each component

- Status: Accepted; amended by [0034](0034-what-opium-is-to-become.md)
  (the Spotify theme renamed; users' themes' format at Stage 5; uwui's high contrast), then by
  [0037](0037-uwuis-decisions-of-2026-11-07-taken-into-opium.md) (M3E built on uwui's official
  theme; Stock gains glass)
- Date: 2026-10-31

## Context

The first window had one look, Apple Music's, recoloured by the user's desktop theme switch, Amane,
through a `theme.toml` it writes. The owner then wanted drastically different looks beside it, a
Spotify one first, without forking the interface, and the same looks on the phone. A look is more
than colours: Spotify's has its library in a sidebar, search in the top bar, a docked player bar
and a Now Playing panel, where Apple Music's has none of them in the same place.

## Decision

**A theme is a TOML file** (`opium_core::theme`, commits `aec53d8`, `f0b1d79`): a `name`, a
`[layout]` table of the choices both apps make (header, rows, cards, favourite, headings, motion),
`[layout.desktop]` and `[layout.mobile]` for each app's own (on the desktop: window, sidebar, top
bar, player bar, panel, lyrics, glow, Now Playing; on the phone: tabs, mini player, Now Playing,
search, library), and `[colors]`, with `[colors.light]` for a theme that has light colours too.

**Each choice names a variant of a component, not a colour**: the apps draw a different component
for each value. Whatever a file leaves out is Stock's. A malformed colour, an unknown key or an
unknown choice rejects the file and says why (`deny_unknown_fields`).

**Three themes are built in** (`crates/opium-core/assets/themes/`): Stock (Apple Music's look,
Carbonfox's black with Apple Music's pink), Spotify, and M3E (Material 3 Expressive, with a fixed
violet tonal scheme). The user's own are files in the config folder's `themes/`, read again as they
change; the phone imports them, and the desktop shares its own with the paired phone over Opium
Connect.

**Colours come from the theme or from Amane**, a setting apart from the theme, so that a theme's
layout can be worn in the desktop's colours.

**The colour roles are named after Amane's keys** (`base`, `mantle`, `surface0`…, `accent`), plus
the phone's own roles and Material's tonal roles where a theme draws with them.

## Alternatives considered

- **A hard-coded look per theme in the code**: each new look a fork of the interface, and none of
  them the user's to make.
- **Colours only**: Spotify's layout cannot be reached by recolouring Apple Music's.

## Consequences

- Every component exists in as many variants as the themes ask, on both platforms: a change to a
  page is checked in each theme (Stock, Spotify, M3E).
- The theme format is shared by both apps and parsed once, in Rust.
- A UI framework that replaces egui must let a theme choose a variant of each component
  (ROADMAP, the move to SlimeEngine's UI framework).
