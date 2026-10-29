//! The colour roles of `design/…/DESIGN.md`, named after Amane's `C[]` keys so a
//! theme file maps one to one, and the values derived from them.

use egui::Color32;

use crate::color::{alpha, luminance, mix};

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
    /// Near-black with a slight violet cast, Pluto's violet as the accent.
    pub const STOCK: Theme = Theme {
        light: false,
        base: rgb(0x0b0a0f),
        mantle: rgb(0x15131b),
        crust: rgb(0x070609),
        surface0: rgb(0x1a1822),
        surface1: rgb(0x25222f),
        surface2: rgb(0x322e3d),
        overlay0: rgb(0x5f5a6e),
        overlay1: rgb(0x8f8a9e),
        text: rgb(0xf5f3ff),
        subtext0: rgb(0xaaa5ba),
        accent: rgb(0x7c3aed),
        mauve: rgb(0xa78bfa),
        red: rgb(0xf43f5e),
        green: rgb(0x4ade80),
        yellow: rgb(0xfbbf24),
    };

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
    pub fn on_accent(&self) -> Color32 {
        if luminance(self.accent) > 0.179 {
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
