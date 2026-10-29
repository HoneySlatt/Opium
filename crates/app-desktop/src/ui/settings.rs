//! Settings: the services and their sign-in, the colours, the interface scale,
//! and what the app is. Only settings that do something are here.

use egui::{Id, Rect, Sense, Stroke, Ui, pos2, vec2};

use crate::app::{App, Connection};
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::settings::Appearance;
use crate::theme::Theme;

use super::{glow, paint_icon, paint_text, text};

const TITLE_Y: f32 = 66.0;
const BODY_TOP: f32 = 104.0;
/// The page is no wider than this, as in the design.
const MAX_WIDTH: f32 = 760.0;
const ROW: f32 = 68.0;
const GROUP_GAP: f32 = 32.0;
/// A group's title and the line under it.
const GROUP_HEAD: f32 = 52.0;
/// The theme cards: padding, preview, name, line, swatches, padding.
const THEMES: f32 = 14.0 + 10.0 + 74.0 + 10.0 + 18.0 + 18.0 + 4.0 + 14.0 + 12.0 + 14.0;

pub fn show(app: &mut App, ui: &mut Ui, pad: f32) -> f32 {
    let theme = app.theme.clone();
    let width = (ui.available_width() - 2.0 * pad).min(MAX_WIDTH);
    let total = BODY_TOP + 6.0 * (GROUP_HEAD + GROUP_GAP) + 3.0 * ROW + THEMES + 6.0 * ROW + 104.0;
    let output = egui::ScrollArea::vertical()
        .id_salt("settings")
        .auto_shrink(false)
        .show(ui, |ui| {
            let (content, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
            glow(
                ui,
                &theme,
                Rect::from_min_size(content.min, vec2(content.width(), 460.0)),
            );
            let left = content.left() + pad;
            let g = text(ui, "Settings", font(22.0, Weight::Bold), theme.text, width);
            paint_text(
                ui,
                g,
                left,
                content.top() + TITLE_Y,
                egui::Align::Min,
                theme.text,
            );
            let mut y = content.top() + BODY_TOP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Services",
                "Each service keeps its own sign-in, on this computer only.",
            );
            let card = Rect::from_min_size(pos2(left, y), vec2(width, 3.0 * ROW));
            panel(ui, &theme, card, 3);
            services(app, ui, &theme, card);
            y = card.bottom() + GROUP_GAP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Library",
                "What joins your Apple Music library.",
            );
            let card = Rect::from_min_size(pos2(left, y), vec2(width, 2.0 * ROW));
            panel(ui, &theme, card, 2);
            let on = app.favourites_to_library();
            if switch(
                ui,
                &theme,
                pos2(card.right() - 16.0, card.top() + ROW / 2.0),
                on,
                "favourites-to-library",
            ) {
                app.set_favourites_to_library(!on);
            }
            labels(
                ui,
                &theme,
                pos2(card.left() + 16.0, card.top() + ROW / 2.0),
                "Add favourites to the library",
                "A song you favourite joins your library if it isn't there yet.",
                width - 32.0 - 60.0,
            );
            let on = app.playlist_songs_to_library();
            if switch(
                ui,
                &theme,
                pos2(card.right() - 16.0, card.top() + 1.5 * ROW),
                on,
                "playlist-songs-to-library",
            ) {
                app.set_playlist_songs_to_library(!on);
            }
            labels(
                ui,
                &theme,
                pos2(card.left() + 16.0, card.top() + 1.5 * ROW),
                "Add playlist songs to the library",
                "A song you add to a playlist joins your library if it isn't there yet.",
                width - 32.0 - 60.0,
            );
            y = card.bottom() + GROUP_GAP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Appearance",
                "Amane recolours Music along with the rest of the desktop.",
            );
            let card = Rect::from_min_size(pos2(left, y), vec2(width, THEMES));
            panel(ui, &theme, card, 1);
            appearance(app, ui, &theme, card);
            y = card.bottom() + GROUP_GAP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Artwork",
                "Covers that move, where Apple made them: on album pages and in Now Playing.",
            );
            let card = Rect::from_min_size(pos2(left, y), vec2(width, 2.0 * ROW));
            panel(ui, &theme, card, 2);
            let on = app.animated_artwork();
            if switch(
                ui,
                &theme,
                pos2(card.right() - 16.0, card.top() + ROW / 2.0),
                on,
                "animated-artwork",
            ) {
                app.set_animated_artwork(!on);
            }
            labels(
                ui,
                &theme,
                pos2(card.left() + 16.0, card.top() + ROW / 2.0),
                "Animated artwork",
                "Play a cover's video when it has one. It takes a little more power.",
                width - 32.0 - 60.0,
            );
            let quality =
                motion_quality(app, ui, &theme, card.right() - 16.0, card.top() + 1.5 * ROW);
            labels(
                ui,
                &theme,
                pos2(card.left() + 16.0, card.top() + 1.5 * ROW),
                "Animated artwork quality",
                "Automatic fetches each video at the size it takes on screen.",
                quality - card.left() - 32.0,
            );
            y = card.bottom() + GROUP_GAP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Interface",
                "How large everything is drawn, on top of the screen's own scale.",
            );
            let card = Rect::from_min_size(pos2(left, y), vec2(width, ROW));
            panel(ui, &theme, card, 1);
            scale(app, ui, &theme, card);
            y = card.bottom() + GROUP_GAP;

            y = group(ui, &theme, left, y, width, "About", "");
            let card = Rect::from_min_size(pos2(left, y), vec2(width, ROW));
            panel(ui, &theme, card, 1);
            labels(
                ui,
                &theme,
                pos2(card.left() + 16.0, card.center().y),
                &format!("Music {}", env!("CARGO_PKG_VERSION")),
                "GPL-3.0-or-later. Widevine and CENC code adapted from Kopuz (EUPL-1.2).",
                width - 32.0,
            );
        });
    output.state.clamped_offset().y
}

/// A group's title and its line of explanation; returns where its card goes.
fn group(ui: &Ui, theme: &Theme, left: f32, top: f32, width: f32, title: &str, blurb: &str) -> f32 {
    let g = text(ui, title, font(18.0, Weight::SemiBold), theme.text, width);
    paint_text(ui, g, left, top + 11.0, egui::Align::Min, theme.text);
    if blurb.is_empty() {
        return top + GROUP_HEAD - 18.0;
    }
    let muted = theme.muted();
    let g = text(ui, blurb, font(13.0, Weight::Regular), muted, width);
    paint_text(ui, g, left, top + 33.0, egui::Align::Min, muted);
    top + GROUP_HEAD
}

/// A card of `rows` rows, with hairlines between them.
fn panel(ui: &Ui, theme: &Theme, rect: Rect, rows: usize) {
    ui.painter().rect(
        rect,
        12.0,
        theme.text_a(0.04),
        Stroke::new(1.0, theme.line()),
        egui::StrokeKind::Inside,
    );
    for i in 1..rows {
        let y = rect.top() + i as f32 * ROW;
        ui.painter()
            .hline(rect.x_range(), y, Stroke::new(1.0, theme.line()));
    }
}

/// A setting's name and, under it, what it does.
fn labels(ui: &Ui, theme: &Theme, at: egui::Pos2, name: &str, detail: &str, width: f32) {
    let g = text(ui, name, font(13.5, Weight::Medium), theme.text, width);
    paint_text(ui, g, at.x, at.y - 9.0, egui::Align::Min, theme.text);
    let muted = theme.muted();
    let g = text(ui, detail, font(12.0, Weight::Regular), muted, width);
    paint_text(ui, g, at.x, at.y + 10.0, egui::Align::Min, muted);
}

/// An on/off switch ending at `right`, as the design draws it: the accent
/// when on, the knob sliding across. Returns whether it was clicked.
fn switch(ui: &mut Ui, theme: &Theme, right_center: egui::Pos2, on: bool, salt: &str) -> bool {
    let rect = Rect::from_min_max(
        pos2(right_center.x - 38.0, right_center.y - 11.0),
        pos2(right_center.x, right_center.y + 11.0),
    );
    let response = ui.interact(rect, Id::unique(("switch", salt)), Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(Id::unique(("switch-anim", salt)), on, 0.18);
    let off = theme.surface2;
    ui.painter()
        .rect_filled(rect, 11.0, crate::color::mix(off, theme.accent, t));
    let x = rect.left() + 11.0 + t * 16.0;
    ui.painter()
        .circle_filled(pos2(x, rect.center().y), 8.0, egui::Color32::WHITE);
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// A small button on a faint fill, ending at `right`. Returns whether it was
/// clicked.
fn button(ui: &mut Ui, theme: &Theme, right: f32, y: f32, label: &str, enabled: bool) -> bool {
    let g = text(ui, label, font(12.5, Weight::Medium), theme.text, 300.0);
    let rect = Rect::from_min_max(
        pos2(right - g.size().x - 28.0, y - 15.0),
        pos2(right, y + 15.0),
    );
    let response = ui.interact(rect, Id::unique(("settings-button", label)), Sense::click());
    let hovered = enabled && response.hovered();
    let fill = theme.text_a(if hovered { 0.14 } else { 0.08 });
    ui.painter().rect_filled(rect, 8.0, fill);
    let color = if enabled { theme.text } else { theme.muted() };
    paint_text(ui, g, rect.left() + 14.0, y, egui::Align::Min, color);
    enabled
        && response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
}

fn services(app: &mut App, ui: &mut Ui, theme: &Theme, card: Rect) {
    let rows: [(Icon, &str); 3] = [
        (icons::AM, "Apple Music"),
        (icons::CLOUD, "SoundCloud"),
        (icons::SERVER, "Navidrome"),
    ];
    for (i, (icon, name)) in rows.into_iter().enumerate() {
        let y = card.top() + i as f32 * ROW + ROW / 2.0;
        let tile = Rect::from_center_size(pos2(card.left() + 16.0 + 20.0, y), vec2(40.0, 40.0));
        let active = i == 0;
        let tile_fill = if active {
            theme.accent
        } else {
            theme.text_a(0.08)
        };
        let glyph = if active {
            theme.on_accent()
        } else {
            theme.muted()
        };
        ui.painter().rect_filled(tile, 10.0, tile_fill);
        paint_icon(ui, icon, tile.center(), 20.0, glyph);
        let x = tile.right() + 14.0;
        let right = card.right() - 16.0;
        if !active {
            let detail = if i == 1 {
                "AAC 160 through SoundCloud's API. Comes in a later version."
            } else {
                "Your own server, original files up to 24-bit. Comes in a later version."
            };
            labels(ui, theme, pos2(x, y), name, detail, right - x - 20.0);
            continue;
        }

        let detail = match &app.connection {
            Connection::Connected => format!(
                "Storefront {} · AAC 256 kbps at best (ALAC is FairPlay-only)",
                app.storefront.as_deref().unwrap_or("?").to_uppercase()
            ),
            Connection::Connecting => "Connecting…".to_owned(),
            Connection::NotSignedIn => {
                "Not signed in. Signing in opens Apple's page in Brave Origin.".to_owned()
            }
            Connection::Failed(e) => e.clone(),
        };
        let (label, enabled) = if app.signing_in {
            ("Waiting for you in the browser…", false)
        } else if app.connection == Connection::Connected {
            ("Sign in again", true)
        } else {
            ("Sign in", app.connection != Connection::Connecting)
        };
        // The button first, so the words know how much room is left.
        let g = text(ui, label, font(12.5, Weight::Medium), theme.text, 300.0);
        let button_left = right - g.size().x - 28.0;
        if button(ui, theme, right, y, label, enabled) {
            app.sign_in();
        }
        let mut state_right = button_left - 14.0;
        if app.connection == Connection::Connected {
            let g = text(
                ui,
                "Connected",
                font(12.0, Weight::Medium),
                theme.subtext0,
                100.0,
            );
            let w = g.size().x;
            paint_text(ui, g, state_right, y, egui::Align::Max, theme.subtext0);
            ui.painter()
                .circle_filled(pos2(state_right - w - 9.0, y), 3.5, theme.green);
            state_right -= w + 20.0;
        }
        labels(ui, theme, pos2(x, y), name, &detail, state_right - x - 12.0);
    }
}

fn appearance(app: &mut App, ui: &mut Ui, theme: &Theme, card: Rect) {
    let amane = Theme::amane();
    let chosen = app.appearance();
    let options = [
        (
            Appearance::Stock,
            "Stock",
            "Black and violet, from Pluto.",
            Some(Theme::STOCK),
        ),
        (
            Appearance::Amane,
            "Amane",
            if amane.is_some() {
                "Follows switch-theme.sh."
            } else {
                "Amane's theme file isn't there yet."
            },
            amane,
        ),
    ];
    let inner = card.shrink(14.0);
    let w = (inner.width() - 12.0) / 2.0;
    for (i, (kind, name, detail, colors)) in options.into_iter().enumerate() {
        let rect = Rect::from_min_size(
            pos2(inner.left() + i as f32 * (w + 12.0), inner.top()),
            vec2(w, inner.height()),
        );
        let enabled = colors.is_some();
        let response = ui.interact(rect, Id::unique(("theme-option", name)), Sense::click());
        let on = chosen == kind;
        if enabled && response.hovered() && !on {
            ui.painter().rect_filled(rect, 12.0, theme.hover());
        }
        let border = if on {
            Stroke::new(2.0, theme.accent)
        } else {
            Stroke::new(1.0, theme.line())
        };
        ui.painter()
            .rect_stroke(rect, 12.0, border, egui::StrokeKind::Inside);
        let preview = Rect::from_min_size(rect.min + vec2(10.0, 10.0), vec2(w - 20.0, 74.0));
        let x = preview.left();
        let mut y = preview.bottom() + 10.0;
        match &colors {
            Some(c) => paint_preview(ui, preview, c),
            None => {
                ui.painter().rect_filled(preview, 8.0, theme.text_a(0.04));
            }
        }
        let g = text(ui, name, font(13.5, Weight::SemiBold), theme.text, w - 20.0);
        paint_text(ui, g, x, y + 9.0, egui::Align::Min, theme.text);
        y += 18.0;
        let muted = theme.muted();
        let g = text(ui, detail, font(12.0, Weight::Regular), muted, w - 20.0);
        paint_text(ui, g, x, y + 9.0, egui::Align::Min, muted);
        y += 18.0 + 4.0;
        if let Some(c) = &colors {
            for (k, color) in [c.base, c.surface1, c.text, c.accent, c.mauve]
                .into_iter()
                .enumerate()
            {
                let swatch = Rect::from_min_size(pos2(x + k as f32 * 18.0, y), vec2(14.0, 14.0));
                ui.painter().rect(
                    swatch,
                    4.0,
                    color,
                    Stroke::new(1.0, theme.line()),
                    egui::StrokeKind::Inside,
                );
            }
        }
        if enabled
            && response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            && !on
        {
            app.set_appearance(kind, ui.ctx());
        }
    }
}

/// A small picture of the app in `c`'s colours: the sidebar, a title, a line,
/// the player bar.
fn paint_preview(ui: &Ui, rect: Rect, c: &Theme) {
    let p = ui.painter();
    let at = |fx: f32, fy: f32| pos2(rect.left() + fx * rect.width(), rect.top() + fy);
    p.rect_filled(rect, 8.0, c.base);
    p.rect_filled(
        Rect::from_min_max(
            rect.min,
            pos2(rect.left() + 0.28 * rect.width(), rect.bottom()),
        ),
        egui::CornerRadius {
            nw: 8,
            sw: 8,
            ne: 0,
            se: 0,
        },
        c.crust,
    );
    p.rect_filled(
        Rect::from_min_size(at(0.36, 14.0), vec2(0.28 * rect.width(), 8.0)),
        4.0,
        c.text,
    );
    p.rect_filled(
        Rect::from_min_size(at(0.36, 30.0), vec2(0.18 * rect.width(), 6.0)),
        3.0,
        c.subtext0,
    );
    let bar = Rect::from_min_max(
        pos2(rect.left() + 0.36 * rect.width(), rect.bottom() - 22.0),
        pos2(rect.right() - 0.10 * rect.width(), rect.bottom() - 10.0),
    );
    p.rect_filled(bar, 6.0, c.mantle);
    p.rect_filled(
        Rect::from_min_size(bar.min, vec2(0.4 * bar.width(), bar.height())),
        6.0,
        c.accent,
    );
}

/// Automatic, Low and High, ending at `right`; returns where they begin.
fn motion_quality(app: &mut App, ui: &mut Ui, theme: &Theme, right: f32, y: f32) -> f32 {
    use crate::settings::MotionQuality;
    let chosen = app.motion_quality();
    let enabled = app.animated_artwork();
    let options = [
        (MotionQuality::Automatic, "Automatic"),
        (MotionQuality::Low, "Low"),
        (MotionQuality::High, "High"),
    ];
    let body = font(12.5, Weight::Medium);
    let widths: Vec<f32> = options
        .iter()
        .map(|(_, label)| text(ui, label, body.clone(), theme.text, 200.0).size().x + 24.0)
        .collect();
    let mut x = right - widths.iter().sum::<f32>() - 6.0 * (options.len() - 1) as f32;
    let left = x;
    for ((quality, label), w) in options.into_iter().zip(widths) {
        let rect = Rect::from_min_size(pos2(x, y - 14.0), vec2(w, 28.0));
        x += w + 6.0;
        let response = ui.interact(rect, Id::unique(("motion-quality", label)), Sense::click());
        let on = quality == chosen;
        let (fill, fg) = if on {
            (theme.text_a(if enabled { 0.9 } else { 0.4 }), theme.base)
        } else if enabled && response.hovered() {
            (theme.text_a(0.14), theme.text)
        } else {
            (
                theme.text_a(0.08),
                if enabled { theme.text } else { theme.muted() },
            )
        };
        ui.painter().rect_filled(rect, 14.0, fill);
        let g = text(ui, label, body.clone(), fg, w);
        paint_text(
            ui,
            g,
            rect.center().x,
            rect.center().y,
            egui::Align::Center,
            fg,
        );
        if enabled
            && response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
        {
            app.set_motion_quality(quality);
        }
    }
    left
}

fn scale(app: &mut App, ui: &mut Ui, theme: &Theme, card: Rect) {
    let y = card.center().y;
    let right = card.right() - 16.0;
    let current = app.scale();
    // From the right: Reset, +, the value, −.
    if button(ui, theme, right, y, "Reset", true) {
        app.set_scale(None);
    }
    let g = text(ui, "Reset", font(12.5, Weight::Medium), theme.text, 300.0);
    let mut x = right - g.size().x - 28.0 - 12.0;
    let step = |ui: &mut Ui, x: f32, icon: Icon, salt: &str| {
        let rect = Rect::from_center_size(pos2(x - 15.0, y), vec2(30.0, 30.0));
        let response = ui.interact(rect, Id::unique(("scale", salt)), Sense::click());
        let fill = theme.text_a(if response.hovered() { 0.14 } else { 0.08 });
        ui.painter().rect_filled(rect, 8.0, fill);
        paint_icon(ui, icon, rect.center(), 14.0, theme.text);
        response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
    };
    if step(ui, x, icons::PLUS, "bigger") {
        app.set_scale(Some(current + 0.1));
    }
    x -= 30.0 + 8.0;
    let value = format!("{:.0} %", current * 100.0);
    let g = text(ui, &value, font(13.0, Weight::Medium), theme.text, 80.0);
    paint_text(ui, g, x - 30.0, y, egui::Align::Center, theme.text);
    x -= 60.0 + 8.0;
    if step(ui, x, icons::MINUS, "smaller") {
        app.set_scale(Some(current - 0.1));
    }
    x -= 30.0;
    labels(
        ui,
        theme,
        pos2(card.left() + 16.0, y),
        "Interface scale",
        "Ctrl + and Ctrl − change it too; Ctrl 0 goes back to 100 %.",
        x - card.left() - 32.0,
    );
}
