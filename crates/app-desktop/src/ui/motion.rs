//! How what's shown comes in, as the theme's motion has it: Apple Music's
//! fades in rising a little, Spotify's only fades, quicker.

use egui::{Id, Ui, UiBuilder, vec2};

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
