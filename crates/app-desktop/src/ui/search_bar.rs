//! M3E's bar across the window's top: over the rail its toggle and the
//! brand, then back and forward, a large search bar in the middle, Play On
//! and the account on the right.

use egui::{Id, Rect, Sense, Ui, pos2, vec2};

use crate::app::{App, Page};
use crate::color::alpha;
use crate::fonts::{Weight, font};
use crate::icons;

use super::shapes::Shape;
use super::{artwork, icon_button, paint_icon, paint_text, text};

pub fn show(app: &mut App, ui: &mut Ui, rail: f32) {
    let theme = app.theme.clone();
    let bar = ui.max_rect();
    let y = bar.center().y;

    // The rail's toggle, and the brand when the rail is open.
    let toggle = Rect::from_center_size(pos2(bar.left() + 20.0 + 20.0, y), vec2(40.0, 40.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(toggle));
    let collapsed = app.sidebar_collapsed;
    if icon_button(
        &mut child,
        &theme,
        icons::PANEL,
        40.0,
        22.0,
        theme.subtext0,
        true,
    )
    .on_hover_text(if collapsed {
        "Expand the rail"
    } else {
        "Collapse the rail"
    })
    .clicked()
    {
        app.sidebar_collapsed = !collapsed;
    }
    if rail > 200.0 {
        let mark = Rect::from_center_size(pos2(toggle.right() + 12.0 + 15.0, y), vec2(30.0, 30.0));
        Shape::Cookie9.radii().fill(ui, mark, 0.0, theme.accent);
        paint_icon(ui, icons::NOTE, mark.center(), 15.0, theme.on_accent());
        let g = text(ui, "Music", font(20.0, Weight::SemiBold), theme.text, 120.0);
        paint_text(ui, g, mark.right() + 9.0, y, egui::Align::Min, theme.text);
    }

    // Back and forward, past the rail.
    let mut nav = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_size(
                pos2(bar.left() + rail + 8.0, y - 20.0),
                vec2(84.0, 40.0),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    nav.spacing_mut().item_spacing.x = 4.0;
    let can_back = app.can_go_back();
    if icon_button(
        &mut nav,
        &theme,
        icons::LEFT,
        40.0,
        22.0,
        theme.subtext0,
        can_back,
    )
    .on_hover_text("Back")
    .clicked()
    {
        app.go_back();
    }
    let can_forward = app.can_go_forward();
    if icon_button(
        &mut nav,
        &theme,
        icons::RIGHT,
        40.0,
        22.0,
        theme.subtext0,
        can_forward,
    )
    .on_hover_text("Forward")
    .clicked()
    {
        app.go_forward();
    }

    // The search bar, centred on the window, 620 at most.
    let room = bar.width() - 2.0 * (rail + 100.0);
    let width = room.clamp(260.0, 620.0);
    search(
        app,
        ui,
        Rect::from_center_size(pos2(bar.center().x, y), vec2(width, 48.0)),
    );

    // Play On and the account, which opens Settings.
    let shown = app.shown();
    let mut play_on = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_center_size(
        pos2(bar.right() - 8.0 - 40.0 - 4.0 - 20.0, y),
        vec2(40.0, 40.0),
    )));
    super::devices::button(app, &mut play_on, &theme, &shown);
    let avatar = Rect::from_center_size(pos2(bar.right() - 8.0 - 20.0, y), vec2(40.0, 40.0));
    let response = ui
        .interact(avatar, Id::unique("m3e-account"), Sense::click())
        .on_hover_text("Account and settings")
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    match &app.profile_picture {
        Some(p) => artwork(ui, Some(p.url(80)), avatar, 20.0, &theme),
        None => {
            ui.painter()
                .circle_filled(avatar.center(), 20.0, theme.tonal.tertiary_container);
            let name = std::env::var("USER").unwrap_or_default();
            let initial: String = name
                .chars()
                .next()
                .map(|c| c.to_uppercase().collect())
                .unwrap_or_default();
            let ink = theme.tonal.on_tertiary_container;
            let g = text(ui, &initial, font(16.0, Weight::SemiBold), ink, 40.0);
            paint_text(ui, g, avatar.center().x, y, egui::Align::Center, ink);
        }
    }
    if response.hovered() {
        ui.painter()
            .circle_filled(avatar.center(), 20.0, alpha(theme.text, 0.08));
    }
    if response.clicked() {
        app.go(Page::Settings);
    }
}

/// The search bar: `surface0`, fully round; Ctrl F focuses it.
fn search(app: &mut App, ui: &mut Ui, rect: Rect) {
    let theme = app.theme.clone();
    let id = Id::unique("m3e-search");
    let focused = ui.ctx().memory(|m| m.has_focus(id));
    let hovered = ui.rect_contains_pointer(rect);
    ui.painter().rect_filled(rect, 24.0, theme.surface0);
    if hovered && !focused {
        ui.painter()
            .rect_filled(rect, 24.0, alpha(theme.text, 0.05));
    }
    paint_icon(
        ui,
        icons::SEARCH,
        pos2(rect.left() + 28.0, rect.center().y),
        20.0,
        theme.subtext0,
    );
    if ui
        .interact(rect, id.with("pill"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::Text)
        .clicked()
    {
        app.focus_search = true;
    }
    // The shortcut, at the end while nothing is typed.
    if app.search.is_empty() && !focused {
        let hint = text(
            ui,
            "Ctrl F",
            font(12.0, Weight::Medium),
            theme.subtext0,
            80.0,
        );
        let chip = Rect::from_center_size(
            pos2(
                rect.right() - 16.0 - hint.size().x / 2.0 - 8.0,
                rect.center().y,
            ),
            vec2(hint.size().x + 16.0, 22.0),
        );
        ui.painter().rect_stroke(
            chip,
            6.0,
            egui::Stroke::new(1.0, theme.surface2),
            egui::StrokeKind::Inside,
        );
        paint_text(
            ui,
            hint,
            chip.center().x,
            chip.center().y,
            egui::Align::Center,
            theme.subtext0,
        );
    }
    let body = font(16.0, Weight::Regular);
    let line = text(ui, "W", body.clone(), theme.text, f32::INFINITY)
        .size()
        .y;
    let field = Rect::from_min_max(
        pos2(rect.left() + 52.0, rect.center().y - line / 2.0),
        pos2(rect.right() - 80.0, rect.center().y + line / 2.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field));
    let response = child.add(
        egui::TextEdit::singleline(&mut app.search)
            .id(id)
            .hint_text(
                egui::RichText::new("Search Apple Music, SoundCloud and Navidrome")
                    .color(theme.subtext0),
            )
            .font(body)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .desired_width(field.width()),
    );
    if std::mem::take(&mut app.focus_search) {
        response.request_focus();
    }
    if response.changed() {
        app.search_edited = Some(ui.input(|i| i.time));
    }
    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        app.submit_search();
    }
}
