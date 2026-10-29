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
    /// The interface scale, 1.0 for 100 % (which draws at [`SCALE_UNIT`]);
    /// Ctrl +, Ctrl − and Ctrl 0 change it. `None` is 100 %.
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

/// What 100 % draws at: the design's sizes, 10 % larger (the user's call).
pub const SCALE_UNIT: f32 = 1.1;
