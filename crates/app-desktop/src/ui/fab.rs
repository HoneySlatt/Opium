//! M3E's Play button: in the accent, a squircle while paused and a cookie
//! turning while playing, its shape morphing on the slow spring. The
//! toolbar has it at 42 px, the pages at 72, Now Playing at 96.

use std::f32::consts::TAU;

use egui::{Color32, Id, Rect, Response, Sense, Ui, vec2};

use crate::color::{alpha, mix};
use crate::icons;
use crate::theme::Theme;

use super::motion::{Spring, spring};
use super::paint_icon;
use super::shapes::Shape;

/// An angle that goes round once in `period` seconds while `on`, and stays
/// where it is otherwise.
pub fn turning(ui: &Ui, salt: &str, on: bool, period: f32) -> f32 {
    let id = Id::unique(("turning", salt));
    let now = ui.input(|i| i.time);
    let (mut angle, then) = ui
        .data(|d| d.get_temp::<(f32, f64)>(id))
        .unwrap_or((0.0, now));
    if on {
        let dt = (now - then).min(0.1) as f32;
        angle = (angle + dt * TAU / period) % TAU;
        ui.ctx().request_repaint();
    }
    ui.data_mut(|d| d.insert_temp(id, (angle, now)));
    angle
}

/// The Play button in `rect`; `playing` shows Pause on the turning cookie.
pub fn fab(
    ui: &mut Ui,
    theme: &Theme,
    rect: Rect,
    salt: &str,
    playing: bool,
    loading: bool,
    enabled: bool,
) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(rect, Id::unique(("fab", salt)), sense);
    let pressed = enabled && response.is_pointer_button_down_on();
    let t = spring(
        ui,
        ("fab", salt),
        if playing { 1.0 } else { 0.0 },
        Spring::Slow,
    );
    let scale = spring(
        ui,
        ("fab-press", salt),
        if pressed { 0.9 } else { 1.0 },
        Spring::Fast,
    );
    let turn = turning(ui, salt, playing, 14.0) * t.clamp(0.0, 1.0);
    let shape = Shape::Squircle.radii().toward(&Shape::Cookie9.radii(), t);
    let mut fill = theme.accent;
    if enabled && response.hovered() {
        fill = mix(Color32::WHITE, fill, 0.12);
    }
    if !enabled {
        fill = alpha(fill, 0.38);
    }
    shape.fill(
        ui,
        Rect::from_center_size(rect.center(), rect.size() * scale),
        turn,
        fill,
    );
    let glyph = (rect.width() * 0.46).min(40.0);
    if loading {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        child.put(
            Rect::from_center_size(rect.center(), vec2(glyph, glyph) * 0.9),
            egui::Spinner::new()
                .size(glyph * 0.9)
                .color(theme.on_accent()),
        );
    } else {
        let icon = if playing {
            icons::PAUSE_FILL
        } else {
            icons::PLAY_FILL
        };
        paint_icon(ui, icon, rect.center(), glyph, theme.on_accent());
    }
    let tip = if playing { "Pause" } else { "Play" };
    if enabled {
        response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(tip)
    } else {
        response
    }
}

/// A round icon button in `rect`, whose corners close in to 12 while
/// pressed.
pub fn round(
    ui: &Ui,
    theme: &Theme,
    rect: Rect,
    salt: &str,
    icon: icons::Icon,
    glyph: f32,
    color: Color32,
) -> Response {
    let response = ui
        .interact(rect, Id::unique(("round", salt)), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let pressed = response.is_pointer_button_down_on();
    let full = rect.height() / 2.0;
    let corner = spring(
        ui,
        ("round", salt),
        if pressed { 12.0 } else { full },
        Spring::Fast,
    );
    if response.hovered() {
        let fill = if pressed {
            theme.press()
        } else {
            theme.hover()
        };
        ui.painter()
            .rect_filled(rect, corner.clamp(0.0, full), fill);
    }
    paint_icon(ui, icon, rect.center(), glyph, color);
    response
}

/// A large tonal button: 56 px, the secondary container, an icon and a
/// label, its left middle at `at`.
pub fn tonal(
    ui: &Ui,
    theme: &Theme,
    at: egui::Pos2,
    salt: &str,
    icon: icons::Icon,
    label: &str,
) -> Response {
    let ink = theme.tonal.on_secondary_container;
    let g = super::text(
        ui,
        label,
        crate::fonts::font(16.0, crate::fonts::Weight::SemiBold),
        ink,
        300.0,
    );
    let rect = Rect::from_min_size(
        egui::pos2(at.x, at.y - 28.0),
        vec2(26.0 + 20.0 + 10.0 + g.size().x + 26.0, 56.0),
    );
    let response = ui
        .interact(rect, Id::unique(("tonal", salt)), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let pressed = response.is_pointer_button_down_on();
    let corner = spring(
        ui,
        ("tonal", salt),
        if pressed { 12.0 } else { 28.0 },
        Spring::Fast,
    );
    let mut fill = theme.tonal.secondary_container;
    if response.hovered() {
        fill = mix(ink, fill, 0.08);
    }
    ui.painter()
        .rect_filled(rect, corner.clamp(0.0, 28.0), fill);
    paint_icon(
        ui,
        icon,
        egui::pos2(rect.left() + 26.0 + 10.0, rect.center().y),
        20.0,
        ink,
    );
    super::paint_text(
        ui,
        g,
        rect.left() + 26.0 + 20.0 + 10.0,
        rect.center().y,
        egui::Align::Min,
        ink,
    );
    response
}
