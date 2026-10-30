//! The colour roles of `design/…/DESIGN.md`, named after Amane's `C[]` keys so a
//! theme file maps one to one, and the values derived from them.

use std::path::PathBuf;

use egui::Color32;

use crate::color::{alpha, hex, luminance, mix};

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
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

const fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

impl Theme {
    /// Carbonfox's black and neutral greys, Apple Music's pink as the accent.
    pub const STOCK: Theme = Theme {
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
    fn a_missing_or_malformed_role_rejects_the_file() {
        assert!(Theme::from_toml(&ROSE_PINE.replace("yellow = \"#f6c177\"\n", "")).is_none());
        assert!(Theme::from_toml(&ROSE_PINE.replace("#e0def4", "e0def4")).is_none());
        assert!(Theme::from_toml("not toml [").is_none());
    }
}
