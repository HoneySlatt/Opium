//! A theme: the colour roles of `design/…/DESIGN.md`, named after Amane's
//! `C[]` keys so a theme file maps one to one, the values derived from them,
//! and the layout, which picks how each part of the window is drawn.
//!
//! Theme files are read by the core, which the phone shares (see
//! `opium_core::theme`): Stock and Spotify are built in, and every file in
//! `~/.config/opium/themes/` is one more.

use std::path::PathBuf;

use egui::Color32;
use opium_core::theme::ThemeFile;
pub use opium_core::theme::{
    Cards, Favourite, Glow, Header, Headings, LyricsPlace as Lyrics, Motion, NowPlaying, Panel,
    PlayerBar, Rows, Sidebar, TopBar, Window,
};

use crate::color::{alpha, hex, luminance, mix};

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
    pub base: Color32,
    pub mantle: Color32,
    pub crust: Color32,
    pub surface0: Color32,
    pub surface1: Color32,
    pub surface2: Color32,
    pub overlay0: Color32,
    pub overlay1: Color32,
    pub text: Color32,
    pub subtext0: Color32,
    pub accent: Color32,
    pub mauve: Color32,
    pub red: Color32,
    pub green: Color32,
    pub yellow: Color32,
    /// Material's tonal roles: as the theme file gives them, else derived
    /// from the accent (see [`Tonal::from_accent`]).
    pub tonal: Tonal,
}

/// Material 3's tonal roles, which M3E draws with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tonal {
    pub primary: Color32,
    pub on_primary: Color32,
    pub secondary_container: Color32,
    pub on_secondary_container: Color32,
    pub tertiary_container: Color32,
    pub on_tertiary_container: Color32,
}

impl Tonal {
    /// From an accent, as the core derives them (the phone's alike).
    pub fn from_accent(accent: Color32) -> Tonal {
        let [r, g, b, _] = accent.to_srgba_unmultiplied();
        Tonal::of_core(opium_core::theme::Tonal::from_accent(
            u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b),
        ))
    }

    fn of_core(t: opium_core::theme::Tonal) -> Tonal {
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
    fn of(p: &opium_core::theme::Palette) -> Tonal {
        Tonal::of_core(p.tonal())
    }
}

use opium_core::theme::BUILT_IN;

const fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
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

    /// Where Amane's `switch-theme.sh` writes the theme for Opium.
    pub fn amane_path() -> PathBuf {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_default();
        config.join("opium/theme.toml")
    }

    /// The Amane theme, if its file is there and valid.
    pub fn amane() -> Option<Self> {
        Self::from_toml(&std::fs::read_to_string(Self::amane_path()).ok()?)
    }

    /// A theme file: its name and the theme. What it leaves out is Stock's;
    /// a malformed colour or an unknown layout choice rejects it, with why.
    pub fn from_theme_file(source: &str) -> Result<(String, Self), String> {
        let file = opium_core::theme::parse(source)?;
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

    /// Every theme to choose from: the built-in ones, then the files in
    /// [`Theme::themes_dir`] by name. Each comes with its id (the file's
    /// stem), its name, and the theme or what is wrong with its file.
    pub fn catalogue() -> Vec<(String, String, Result<Theme, String>)> {
        let mut all: Vec<_> = BUILT_IN
            .iter()
            .map(|(id, source)| {
                let (name, theme) =
                    Self::from_theme_file(source).expect("the built-in themes are valid");
                (id.to_string(), name, Ok(theme))
            })
            .collect();
        let mut own: Vec<_> = std::fs::read_dir(Self::themes_dir())
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|e| e == "toml"))
            .filter_map(|path| {
                let id = path.file_stem()?.to_str()?.to_string();
                if BUILT_IN.iter().any(|(built, _)| *built == id) {
                    return None;
                }
                let read = std::fs::read_to_string(&path).map_err(|e| e.to_string());
                Some(match read.and_then(|s| Self::from_theme_file(&s)) {
                    Ok((name, theme)) => {
                        let name = if name.is_empty() { id.clone() } else { name };
                        (id, name, Ok(theme))
                    }
                    Err(why) => (id.clone(), id, Err(why)),
                })
            })
            .collect();
        own.sort_by_key(|a| a.1.to_lowercase());
        all.extend(own);
        all
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

    pub fn hover(&self) -> Color32 {
        alpha(self.text, 0.07)
    }

    pub fn press(&self) -> Color32 {
        alpha(self.text, 0.12)
    }

    /// Hairlines.
    pub fn line(&self) -> Color32 {
        alpha(self.text, 0.09)
    }

    /// Tertiary text.
    pub fn muted(&self) -> Color32 {
        mix(self.subtext0, self.base, 0.78)
    }

    /// Text and glyphs on an accent fill: black on a light accent, else white.
    /// White holds up to a quarter of white's luminance, as on Apple Music's
    /// pink; past that, black.
    pub fn on_accent(&self) -> Color32 {
        if luminance(self.accent) > 0.25 {
            Color32::BLACK
        } else {
            Color32::WHITE
        }
    }

    /// `text` at `p` opacity, the design's most common tint.
    pub fn text_a(&self, p: f32) -> Color32 {
        alpha(self.text, p)
    }

    /// egui's own widgets (scroll bars, text edits, selection) in these colours.
    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = if self.light {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
        visuals.panel_fill = self.base;
        visuals.window_fill = self.mantle;
        visuals.extreme_bg_color = self.surface0;
        visuals.faint_bg_color = self.hover();
        visuals.override_text_color = Some(self.text);
        visuals.selection.bg_fill = alpha(self.accent, 0.18);
        visuals.selection.stroke.color = self.accent;
        visuals.hyperlink_color = self.accent;
        visuals.widgets.noninteractive.bg_stroke.color = self.line();
        visuals.widgets.inactive.bg_fill = self.surface0;
        visuals.widgets.inactive.weak_bg_fill = self.surface0;
        visuals.widgets.hovered.weak_bg_fill = self.hover();
        visuals.widgets.active.weak_bg_fill = self.press();
        visuals.text_cursor.stroke.color = self.accent;
        ctx.set_visuals(visuals);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROSE_PINE: &str = r##"
[colors]
# Rosé Pine (written by switch-theme.sh)
light = false
base = "#191724"
mantle = "#1f1d2e"
crust = "#191724"
surface0 = "#26233a"
surface1 = "#403d52"
surface2 = "#524f67"
overlay0 = "#6e6a86"
overlay1 = "#908caa"
text = "#e0def4"
subtext0 = "#908caa"
accent = "#c4a7e7"
mauve = "#c4a7e7"
red = "#eb6f92"
green = "#9ccfd8"
yellow = "#f6c177"
"##;

    #[test]
    fn an_amane_file_becomes_a_theme() {
        let theme = Theme::from_toml(ROSE_PINE).expect("valid theme");
        assert!(!theme.light);
        assert_eq!(theme.base, Color32::from_rgb(0x19, 0x17, 0x24));
        assert_eq!(theme.accent, Color32::from_rgb(0xc4, 0xa7, 0xe7));
        // A light accent takes dark text.
        assert_eq!(theme.on_accent(), Color32::BLACK);
    }

    #[test]
    fn the_stock_file_is_stock_and_spotify_its_own() {
        assert_eq!(Theme::named("stock"), Some(Theme::STOCK));
        let spotify = Theme::named("spotify").expect("built in");
        assert_eq!(spotify.layout.sidebar, Sidebar::Library);
        assert_eq!(spotify.accent, Color32::from_rgb(0x1e, 0xd7, 0x60));
        // Black on Spotify's green, as its play button.
        assert_eq!(spotify.on_accent(), Color32::BLACK);
    }

    #[test]
    fn a_theme_file_leaves_the_rest_to_stock() {
        let (name, theme) = Theme::from_theme_file(
            "[layout.desktop]\nplayer_bar = \"docked\"\n[colors]\naccent = \"#00ff00\"",
        )
        .unwrap();
        assert_eq!(name, "");
        assert_eq!(theme.layout.player_bar, PlayerBar::Docked);
        assert_eq!(theme.layout.sidebar, Sidebar::Navigation);
        assert_eq!(theme.accent, Color32::from_rgb(0, 255, 0));
        assert_eq!(theme.base, Theme::STOCK.base);
        assert!(Theme::from_theme_file("[layout.desktop]\nsidebar = \"nope\"").is_err());
    }

    #[test]
    fn amane_recolours_a_layout() {
        let amane = Theme::from_toml(ROSE_PINE).unwrap();
        let spotify = Theme::named("spotify").unwrap().recoloured(&amane);
        assert_eq!(spotify.layout.window, Window::Cards);
        assert_eq!(spotify.base, amane.base);
    }

    #[test]
    fn a_missing_or_malformed_role_rejects_the_file() {
        assert!(Theme::from_toml(&ROSE_PINE.replace("yellow = \"#f6c177\"\n", "")).is_none());
        assert!(Theme::from_toml(&ROSE_PINE.replace("#e0def4", "e0def4")).is_none());
        assert!(Theme::from_toml("not toml [").is_none());
    }
}
