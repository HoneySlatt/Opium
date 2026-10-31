//! What's on its way, as the theme's motion has it: Apple Music's spinner,
//! or Spotify's grey shapes of what comes, pulsing.

use egui::{Pos2, Rect, Stroke, Ui, vec2};

use crate::color::alpha;
use crate::theme::{Motion, Theme};

/// A shape of what loads: grey and breathing in Spotify's, none in Apple
/// Music's, whose [`spinner`] stands for them all.
pub fn shape(ui: &Ui, theme: &Theme, rect: Rect, radius: f32) {
    if theme.layout.motion != Motion::Spotify {
        return;
    }
    let t = ui.input(|i| i.time) as f32;
    // From full to 45 % and back, every 1.8 s.
    let breath = 0.725 + 0.275 * (t * std::f32::consts::TAU / 1.8).cos();
    let color = theme.hover();
    ui.painter().rect_filled(
        rect,
        radius,
        alpha(color, color.a() as f32 / 255.0 * breath),
    );
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(33));
}

/// Apple Music's spinner at `center`, eight spokes with the lit one going
/// round; nothing in Spotify's, whose shapes say it.
pub fn spinner(ui: &Ui, theme: &Theme, center: Pos2) {
    if theme.layout.motion == Motion::Spotify {
        return;
    }
    let r = 14.0;
    let width = r * 0.22;
    let lit = ((ui.input(|i| i.time) / 0.1) as usize) % 8;
    for k in 0..8 {
        let a = (k as f32 * 45.0 - 90.0).to_radians();
        let dir = vec2(a.cos(), a.sin());
        let behind = (lit + 8 - k) % 8;
        let color = alpha(theme.subtext0, 1.0 - behind as f32 * 0.11);
        ui.painter().line_segment(
            [center + dir * (r * 0.45), center + dir * (r - width / 2.0)],
            Stroke::new(width, color),
        );
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(50));
}
