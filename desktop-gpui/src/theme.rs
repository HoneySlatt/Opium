//! A theme, as the egui window's `theme.rs`: the colour roles of
//! `design/…/DESIGN.md`, named after Amane's `C[]` keys so a theme file maps
//! one to one, the values derived from them, and the layout, which picks how
//! each part of the window is drawn. In GPUI's colours.
//!
//! Theme files are read by the core, which the phone shares (see
//! `music_core::theme`): Stock and Spotify are built in, and every file in
//! `~/.config/music/themes/` is one more.

use std::path::PathBuf;

use gpui::Rgba;
use music_core::theme::{BUILT_IN, ThemeFile};
#[allow(unused_imports)]
pub use music_core::theme::{
    Cards, Favourite, Glow, Header, Headings, LyricsPlace as Lyrics, Motion, NowPlaying, Panel,
    PlayerBar, Rows, Sidebar, TopBar, Window,
};

use crate::color::{BLACK, WHITE, alpha, hex, luminance, mix, rgb};

/// How each part of the window is drawn: the choices both apps make and
/// the desktop's own, together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub window: Window,
    pub sidebar: Sidebar,
    pub top_bar: TopBar,
    pub player_bar: PlayerBar,
    pub panel: Panel,
    pub header: Header,
    pub lyrics: Lyrics,
    pub glow: Glow,
    pub cards: Cards,
    pub favourite: Favourite,
    pub now_playing: NowPlaying,
    pub rows: Rows,
    pub headings: Headings,
    pub motion: Motion,
}

impl Layout {
    pub const STOCK: Layout = Layout {
        window: Window::Flat,
        sidebar: Sidebar::Navigation,
        top_bar: TopBar::Page,
        player_bar: PlayerBar::Floating,
        panel: Panel::Drawer,
        header: Header::Cover,
        lyrics: Lyrics::Drawer,
        glow: Glow::Accent,
        cards: Cards::Covers,
        favourite: Favourite::Star,
        now_playing: NowPlaying::Lyrics,
        rows: Rows::Single,
        headings: Headings::Small,
        motion: Motion::Apple,
    };

    fn of(file: &ThemeFile) -> Layout {
        let (shared, own) = (file.layout, file.desktop);
        Layout {
            window: own.window,
            sidebar: own.sidebar,
            top_bar: own.top_bar,
            player_bar: own.player_bar,
            panel: own.panel,
            header: shared.header,
            lyrics: own.lyrics,
            glow: own.glow,
            cards: shared.cards,
            favourite: shared.favourite,
            now_playing: own.now_playing,
            rows: shared.rows,
            headings: shared.headings,
            motion: shared.motion,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub layout: Layout,
    pub light: bool,
    pub base: Rgba,
    pub mantle: Rgba,
    pub crust: Rgba,
    pub surface0: Rgba,
    pub surface1: Rgba,
    pub surface2: Rgba,
    pub overlay0: Rgba,
    pub overlay1: Rgba,
    pub text: Rgba,
    pub subtext0: Rgba,
    pub accent: Rgba,
    pub mauve: Rgba,
    pub red: Rgba,
    pub green: Rgba,
    pub yellow: Rgba,
    /// Material's tonal roles: as the theme file gives them, else derived
    /// from the accent (see [`Tonal::from_accent`]).
    pub tonal: Tonal,
}

/// Material 3's tonal roles, which M3E draws with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tonal {
    pub primary: Rgba,
    pub on_primary: Rgba,
    pub secondary_container: Rgba,
    pub on_secondary_container: Rgba,
    pub tertiary_container: Rgba,
    pub on_tertiary_container: Rgba,
}

impl Tonal {
    /// From an accent, as the core derives them (the phone's alike).
    pub fn from_accent(accent: Rgba) -> Tonal {
        let [r, g, b, _] = crate::color::bytes(accent);
        Tonal::of_core(music_core::theme::Tonal::from_accent(
            u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b),
        ))
    }

    fn of_core(t: music_core::theme::Tonal) -> Tonal {
        Tonal {
            primary: rgb(t.primary),
            on_primary: rgb(t.on_primary),
            secondary_container: rgb(t.secondary_container),
            on_secondary_container: rgb(t.on_secondary_container),
            tertiary_container: rgb(t.tertiary_container),
            on_tertiary_container: rgb(t.on_tertiary_container),
        }
    }

    /// A theme file's, what it leaves out derived from its accent.
    fn of(p: &music_core::theme::Palette) -> Tonal {
        Tonal::of_core(p.tonal())
    }
}

impl Theme {
    /// M3E's look, in the parts no choice of its own names (the rows, the
    /// menus): the expressive motion goes with it.
    pub fn expressive(&self) -> bool {
        self.layout.motion == Motion::Expressive
    }

    /// Carbonfox's black and neutral greys, Apple Music's pink as the accent.
    pub const STOCK: Theme = Theme {
        layout: Layout::STOCK,
        light: false,
        base: rgb(0x161616),
        mantle: rgb(0x1e1e1e),
        crust: rgb(0x101010),
        surface0: rgb(0x242424),
        surface1: rgb(0x2e2e2e),
        surface2: rgb(0x3a3a3a),
        overlay0: rgb(0x6e7074),
        overlay1: rgb(0x9a9ca0),
        text: rgb(0xf2f4f8),
        subtext0: rgb(0xb6b8bb),
        accent: rgb(0xfa2d48),
        mauve: rgb(0xfb5c74),
        red: rgb(0xff453a),
        green: rgb(0x4ade80),
        yellow: rgb(0xfbbf24),
        // As `Tonal::from_accent` makes them of the pink.
        tonal: Tonal {
            primary: rgb(0xffa6a4),
            on_primary: rgb(0x490007),
            secondary_container: rgb(0x59302f),
            on_secondary_container: rgb(0xffdbd9),
            tertiary_container: rgb(0x553e0e),
            on_tertiary_container: rgb(0xf2e2c7),
        },
    };

    /// A theme file's `[colors]` table: every role as `#rrggbb`, and `light`.
    /// Anything missing or malformed rejects the whole file.
    pub fn from_toml(source: &str) -> Option<Self> {
        let file: toml::Table = toml::from_str(source).ok()?;
        let colors = file.get("colors")?.as_table()?;
        let color = |key: &str| colors.get(key)?.as_str().and_then(hex);
        let accent = color("accent")?;
        Some(Self {
            tonal: Tonal::from_accent(accent),
            layout: Layout::STOCK,
            light: colors
                .get("light")
                .and_then(toml::Value::as_bool)
                .unwrap_or(false),
            base: color("base")?,
            mantle: color("mantle")?,
            crust: color("crust")?,
            surface0: color("surface0")?,
            surface1: color("surface1")?,
            surface2: color("surface2")?,
            overlay0: color("overlay0")?,
            overlay1: color("overlay1")?,
            text: color("text")?,
            subtext0: color("subtext0")?,
            accent: color("accent")?,
            mauve: color("mauve")?,
            red: color("red")?,
            green: color("green")?,
            yellow: color("yellow")?,
        })
    }

    /// Where Amane's `switch-theme.sh` writes the theme for Music.
    pub fn amane_path() -> PathBuf {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_default();
        config.join("music/theme.toml")
    }

    /// The Amane theme, if its file is there and valid.
    pub fn amane() -> Option<Self> {
        Self::from_toml(&std::fs::read_to_string(Self::amane_path()).ok()?)
    }

    /// A theme file: its name and the theme. What it leaves out is Stock's;
    /// a malformed colour or an unknown layout choice rejects it, with why.
    pub fn from_theme_file(source: &str) -> Result<(String, Self), String> {
        let file = music_core::theme::parse(source)?;
        let p = file.colors;
        let theme = Theme {
            layout: Layout::of(&file),
            light: p.light,
            base: rgb(p.base),
            mantle: rgb(p.mantle),
            crust: rgb(p.crust),
            surface0: rgb(p.surface0),
            surface1: rgb(p.surface1),
            surface2: rgb(p.surface2),
            overlay0: rgb(p.overlay0),
            overlay1: rgb(p.overlay1),
            text: rgb(p.text),
            subtext0: rgb(p.subtext0),
            accent: rgb(p.accent),
            mauve: rgb(p.mauve),
            red: rgb(p.red),
            green: rgb(p.green),
            yellow: rgb(p.yellow),
            tonal: Tonal::of(&p),
        };
        Ok((file.name, theme))
    }

    /// This theme's layout in `colors`' colours (their tonal roles derived
    /// from their accent, as Amane's are).
    pub fn recoloured(&self, colors: &Theme) -> Theme {
        Theme {
            layout: self.layout,
            ..colors.clone()
        }
    }

    /// Where the user's own theme files go.
    pub fn themes_dir() -> PathBuf {
        Self::amane_path().with_file_name("themes")
    }

    /// The theme of that id, if it is there and valid.
    pub fn named(id: &str) -> Option<Theme> {
        if let Some((_, source)) = BUILT_IN.iter().find(|(built, _)| *built == id) {
            return Self::from_theme_file(source).ok().map(|(_, t)| t);
        }
        let path = Self::themes_dir().join(format!("{id}.toml"));
        Self::from_theme_file(&std::fs::read_to_string(path).ok()?)
            .ok()
            .map(|(_, t)| t)
    }

    pub fn hover(&self) -> Rgba {
        alpha(self.text, 0.07)
    }

    pub fn press(&self) -> Rgba {
        alpha(self.text, 0.12)
    }

    /// Hairlines.
    pub fn line(&self) -> Rgba {
        alpha(self.text, 0.09)
    }

    /// Tertiary text.
    pub fn muted(&self) -> Rgba {
        mix(self.subtext0, self.base, 0.78)
    }

    /// Text and glyphs on an accent fill: black on a light accent, else white.
    /// White holds up to a quarter of white's luminance, as on Apple Music's
    /// pink; past that, black.
    pub fn on_accent(&self) -> Rgba {
        if luminance(self.accent) > 0.25 {
            BLACK
        } else {
            WHITE
        }
    }

    /// `text` at `p` opacity, the design's most common tint.
    pub fn text_a(&self, p: f32) -> Rgba {
        alpha(self.text, p)
    }
}

/// The theme chosen (Stock if it is gone or broken), in its own colours or
/// in Amane's when they are chosen (or, until it is chosen either way, when
/// Amane's file exists). A missing or invalid Amane file leaves the theme's.
pub fn resolve_theme(settings: &desktop::settings::Settings) -> Theme {
    use desktop::settings::Appearance;
    let theme = settings
        .theme
        .as_deref()
        .and_then(Theme::named)
        .unwrap_or(Theme::STOCK);
    match settings.appearance {
        Some(Appearance::Stock) => theme,
        Some(Appearance::Amane) | None => match Theme::amane() {
            Some(colors) => theme.recoloured(&colors),
            None => theme,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::bytes;

    #[test]
    fn the_stock_file_is_stock_and_spotify_its_own() {
        assert_eq!(Theme::named("stock"), Some(Theme::STOCK));
        let spotify = Theme::named("spotify").expect("built in");
        assert_eq!(spotify.layout.sidebar, Sidebar::Library);
        assert_eq!(bytes(spotify.accent), [0x1e, 0xd7, 0x60, 255]);
        // Black on Spotify's green, as its play button.
        assert_eq!(spotify.on_accent(), BLACK);
    }
}
