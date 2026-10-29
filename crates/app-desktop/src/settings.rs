//! What the app remembers between runs, in `~/.config/music/settings.toml`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where the colours come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    /// Near-black with Pluto's violet.
    Stock,
    /// Whatever Amane's theme switch last wrote to `~/.config/music/theme.toml`.
    Amane,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Unset: Amane when its theme file exists, Stock otherwise.
    pub appearance: Option<Appearance>,
    /// The interface scale, on top of the screen's own; Ctrl +, Ctrl − and
    /// Ctrl 0 change it. `None` until the user picks one.
    pub ui_scale: Option<f32>,
}

fn path() -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_default();
    config.join("music/settings.toml")
}

impl Settings {
    /// Missing or unreadable settings are the defaults.
    pub fn load() -> Self {
        std::fs::read_to_string(path())
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(s) = toml::to_string(self) {
            let _ = std::fs::write(path, s);
        }
    }
}

/// The scale to start from when the user hasn't chosen one: the design's sizes
/// are CSS pixels, which a 4K screen at scale 1 shows far too small (Cider, the
/// reference, draws about 1.6× there).
pub fn default_scale(ctx: &egui::Context) -> f32 {
    let (monitor, native) = ctx.input(|i| {
        (
            i.viewport().monitor_size,
            i.viewport().native_pixels_per_point,
        )
    });
    let physical_width = monitor.map(|m| m.x * native.unwrap_or(1.0)).unwrap_or(0.0);
    let native = native.unwrap_or(1.0);
    if physical_width >= 3000.0 && native < 1.5 {
        1.5 / native
    } else {
        1.0
    }
}
