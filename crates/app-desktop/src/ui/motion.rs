//! How what's shown comes in, as the theme's motion has it: Apple Music's
//! fades in rising a little, Spotify's only fades, quicker.

use egui::{Color32, Id, Ui, UiBuilder, vec2};

use crate::theme::{Motion, Theme};

/// How far `what`, shown in `salt`'s place, has come in since it was
/// something else: 0 to 1, slowing as it ends.
pub fn arrival<T: Clone + PartialEq + Send + Sync + 'static>(
    ui: &Ui,
    theme: &Theme,
    salt: &str,
    what: &T,
) -> f32 {
    let id = Id::unique(("arrival", salt));
    let now = ui.input(|i| i.time);
    let since = ui.data_mut(|d| match d.get_temp::<(T, f64)>(id) {
        Some((shown, at)) if &shown == what => at,
        _ => {
            d.insert_temp(id, (what.clone(), now));
            now
        }
    });
    let secs = match theme.layout.motion {
        Motion::Apple => 0.28,
        Motion::Spotify => 0.16,
        // Material's page enter: a 380 ms fade and a 14 px rise.
        Motion::Expressive => 0.38,
    };
    let t = ((now - since) as f32 / secs).clamp(0.0, 1.0);
    if t < 1.0 {
        ui.ctx().request_repaint();
    }
    1.0 - (1.0 - t).powi(3)
}

/// What's shown in `salt`'s place comes in again when next shown.
pub fn again<T: 'static>(ui: &Ui, salt: &str) {
    ui.data_mut(|d| d.remove::<(T, f64)>(Id::unique(("arrival", salt))));
}

/// Draws `content` in a child of `ui` (always, so that what it keeps by
/// its ids stays), coming in by `t`: faded, and in Apple Music's `rise`
/// points lower until it is in.
pub fn arriving<R>(
    ui: &mut Ui,
    theme: &Theme,
    salt: &str,
    t: f32,
    rise: f32,
    content: impl FnOnce(&mut Ui) -> R,
) -> R {
    let down = match theme.layout.motion {
        Motion::Apple => rise * (1.0 - t),
        Motion::Spotify => 0.0,
        Motion::Expressive => 14.0 * (1.0 - t),
    };
    let rect = ui.max_rect().translate(vec2(0.0, down));
    let clip = ui.clip_rect();
    let mut child = ui.new_child(UiBuilder::new().id_salt(salt).max_rect(rect));
    child.set_clip_rect(clip);
    if t < 1.0 {
        child.set_opacity(t);
    }
    content(&mut child)
}

/// Another theme or appearance: the window starts in the base colour of
/// the one before and the new one shows through it, as it fades.
pub fn theme_change(ui: &Ui, theme: &Theme) {
    let id = Id::unique("theme-change");
    let now = ui.input(|i| i.time);
    let from = ui.data_mut(|d| {
        let kept = d.get_temp::<(Theme, Option<(Color32, f64)>)>(id);
        let from = match kept {
            Some((shown, from)) if &shown == theme => from,
            Some((before, _)) => Some((before.base, now)),
            None => None,
        };
        d.insert_temp(id, (theme.clone(), from));
        from
    });
    let Some((base, at)) = from else { return };
    let t = ((now - at) as f32 / 0.38).clamp(0.0, 1.0);
    if t >= 1.0 {
        return;
    }
    let veil = egui::LayerId::new(egui::Order::Foreground, Id::unique("theme-veil"));
    ui.ctx().layer_painter(veil).rect_filled(
        ui.ctx().content_rect(),
        0.0,
        crate::color::alpha(base, 1.0 - t * t * (3.0 - 2.0 * t)),
    );
    ui.ctx().request_repaint();
}

/// Material 3 Expressive's springs: spatial ones (size, position, shape,
/// corners) overshoot a little, effects (colour, opacity) don't.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Spring {
    /// Spatial, fast: stiffness 800, damping ratio 0.6.
    Fast,
    /// Spatial, slow: 380, 0.8.
    Slow,
    /// Effects: 1600, critically damped.
    Effects,
}

impl Spring {
    fn constants(self) -> (f32, f32) {
        match self {
            Spring::Fast => (800.0, 0.6),
            Spring::Slow => (380.0, 0.8),
            Spring::Effects => (1600.0, 1.0),
        }
    }
}

/// A value of `salt`'s that follows `target` on `spring`, from where it was
/// last frame; the window is drawn again until it rests. The first time,
/// it is at `target` already.
pub fn spring(
    ui: &Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    target: f32,
    spring: Spring,
) -> f32 {
    let id = Id::unique(("spring", salt));
    let now = ui.input(|i| i.time);
    let (mut x, mut v, then) = ui
        .data(|d| d.get_temp::<(f32, f32, f64)>(id))
        .unwrap_or((target, 0.0, now));
    let (k, zeta) = spring.constants();
    let c = 2.0 * zeta * k.sqrt();
    // Small steps, so a long frame doesn't throw it off.
    let mut left = ((now - then) as f32).min(0.1);
    while left > 0.0 {
        let dt = left.min(1.0 / 240.0);
        v += (-k * (x - target) - c * v) * dt;
        x += v * dt;
        left -= dt;
    }
    let resting = (x - target).abs() < 1e-3 && v.abs() < 1e-2;
    if resting {
        (x, v) = (target, 0.0);
    } else {
        ui.ctx().request_repaint();
    }
    ui.data_mut(|d| d.insert_temp(id, (x, v, now)));
    x
}
