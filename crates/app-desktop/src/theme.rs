//! A theme: the colour roles of `design/…/DESIGN.md`, named after Amane's
//! `C[]` keys so a theme file maps one to one, the values derived from them,
//! and the layout, which picks how each part of the window is drawn.
//!
//! Themes are TOML files: Stock and Spotify are built in, and every file in
//! `~/.config/music/themes/` is one more. A file has a `name`, a `[layout]`
//! table and a `[colors]` table; whatever it leaves out is Stock's.

use std::path::PathBuf;

use egui::Color32;
use serde::Deserialize;

use crate::color::{alpha, hex, luminance, mix};

/// How each part of the window is drawn. Every choice defaults to Stock's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
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
    };
}

/// The window's parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Window {
    /// Side by side, edge to edge.
    #[default]
    Flat,
    /// Rounded cards with gaps between them on `crust`.
    Cards,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sidebar {
    /// Search, then the pages, the library and the playlists, as Cider does.
    #[default]
    Navigation,
    /// Your Library, as Spotify does: the playlists, albums or artists with
    /// their covers, Search and Home being in the top bar.
    Library,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopBar {
    /// Over the page: back, forward and the drawer's buttons.
    #[default]
    Page,
    /// Across the window: back and forward, Home and Search in the middle,
    /// the account on the right.
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerBar {
    /// A floating bar over the page, as Cider's.
    #[default]
    Floating,
    /// Along the window's bottom: the song on the left, the controls and the
    /// position in the middle, the volume on the right.
    Docked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Panel {
    /// A drawer sliding over the page with Playing Next, History and Lyrics.
    #[default]
    Drawer,
    /// A column beside the page with the song playing or the queue.
    Side,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Header {
    /// A large cover with the details beside it, as Cider.
    #[default]
    Cover,
    /// The cover at the bottom left of a band of its colour, the title in
    /// large type, the buttons under it, as Spotify.
    Banner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lyrics {
    /// In the drawer.
    #[default]
    Drawer,
    /// Over the whole page, on the cover's colour.
    Page,
}

/// What lights the top of the pages without a cover of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Glow {
    /// The accent and its glow, blurred.
    #[default]
    Accent,
    /// A grey fading into the page, as Spotify's.
    Grey,
}

/// How covers in grids and shelves are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cards {
    /// The cover alone, darkened on hover.
    #[default]
    Covers,
    /// On hover, a tile behind the cover and its words, and a round Play
    /// on the cover, as Spotify's.
    Tiles,
}

/// How a favourite song is marked in the lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Favourite {
    /// A red star after its title, as Apple Music's.
    #[default]
    Star,
    /// A check in the accent, as Spotify's saved songs.
    Check,
}

/// The full screen player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NowPlaying {
    /// Cider's: the cover down the left fading into its colours, the
    /// lyrics on the right, the controls in a card.
    #[default]
    Lyrics,
    /// Spotify's: the cover large in the middle on its colour, cards about
    /// the song under it, the player bar kept along the bottom; the lyrics
    /// in its place on demand.
    Cover,
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
}

/// The themes that come with the app, by id.
const BUILT_IN: [(&str, &str); 2] = [
    ("stock", include_str!("../assets/themes/stock.toml")),
    ("spotify", include_str!("../assets/themes/spotify.toml")),
];

const fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

impl Theme {
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
    };

    /// A theme file's `[colors]` table: every role as `#rrggbb`, and `light`.
    /// Anything missing or malformed rejects the whole file.
    pub fn from_toml(source: &str) -> Option<Self> {
        let file: toml::Table = toml::from_str(source).ok()?;
        let colors = file.get("colors")?.as_table()?;
        let color = |key: &str| colors.get(key)?.as_str().and_then(hex);
        Some(Self {
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
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct File {
            name: Option<String>,
            #[serde(default)]
            layout: Layout,
            #[serde(default)]
            colors: toml::Table,
        }
        let file: File = toml::from_str(source).map_err(|e| e.message().to_string())?;
        let mut theme = Theme {
            layout: file.layout,
            ..Theme::STOCK
        };
        for (key, value) in &file.colors {
            if key == "light" {
                theme.light = value.as_bool().ok_or("light is true or false")?;
                continue;
            }
            let color = value
                .as_str()
                .and_then(hex)
                .ok_or_else(|| format!("{key} is not a #rrggbb colour"))?;
            let slot = match key.as_str() {
                "base" => &mut theme.base,
                "mantle" => &mut theme.mantle,
                "crust" => &mut theme.crust,
                "surface0" => &mut theme.surface0,
                "surface1" => &mut theme.surface1,
                "surface2" => &mut theme.surface2,
                "overlay0" => &mut theme.overlay0,
                "overlay1" => &mut theme.overlay1,
                "text" => &mut theme.text,
                "subtext0" => &mut theme.subtext0,
                "accent" => &mut theme.accent,
                "mauve" => &mut theme.mauve,
                "red" => &mut theme.red,
                "green" => &mut theme.green,
                "yellow" => &mut theme.yellow,
                _ => return Err(format!("{key} is not a colour role")),
            };
            *slot = color;
        }
        Ok((file.name.unwrap_or_default(), theme))
    }

    /// This theme's layout in `colors`' colours.
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
            "[layout]\nplayer_bar = \"docked\"\n[colors]\naccent = \"#00ff00\"",
        )
        .unwrap();
        assert_eq!(name, "");
        assert_eq!(theme.layout.player_bar, PlayerBar::Docked);
        assert_eq!(theme.layout.sidebar, Sidebar::Navigation);
        assert_eq!(theme.accent, Color32::from_rgb(0, 255, 0));
        assert_eq!(theme.base, Theme::STOCK.base);
        assert!(Theme::from_theme_file("[layout]\nsidebar = \"nope\"").is_err());
        assert!(Theme::from_theme_file("[layout]\nsidbar = \"library\"").is_err());
        assert!(Theme::from_theme_file("[colors]\nbase = \"blue\"").is_err());
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
