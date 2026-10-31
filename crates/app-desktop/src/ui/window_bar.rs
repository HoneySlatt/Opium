//! The bar across the window's top, as Spotify's: back and forward on the
//! left, Home and Search in the middle, the account on the right.

use egui::{Id, Rect, Sense, Stroke, Ui, pos2, vec2};

use crate::app::{App, Page};
use crate::fonts::{Weight, font};
use crate::icons;

use super::{artwork, icon_button, paint_icon, text};

pub const HEIGHT: f32 = 64.0;

pub fn show(app: &mut App, ui: &mut Ui) {
    let theme = app.theme.clone();
    let bar = ui.max_rect();
    let y = bar.center().y;

    let mut left = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_size(
                pos2(bar.left() + 16.0, y - 16.0),
                vec2(80.0, 32.0),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    left.spacing_mut().item_spacing.x = 4.0;
    let can_back = app.can_go_back();
    if icon_button(
        &mut left,
        &theme,
        icons::LEFT,
        32.0,
        20.0,
        theme.subtext0,
        can_back,
    )
    .on_hover_text("Go back")
    .clicked()
    {
        app.go_back();
    }
    let can_forward = app.can_go_forward();
    if icon_button(
        &mut left,
        &theme,
        icons::RIGHT,
        32.0,
        20.0,
        theme.subtext0,
        can_forward,
    )
    .on_hover_text("Go forward")
    .clicked()
    {
        app.go_forward();
    }

    // Home and Search, centred on the window.
    let field = (bar.width() - 2.0 * 160.0 - 56.0).clamp(220.0, 474.0);
    let start = bar.center().x - (48.0 + 8.0 + field) / 2.0;
    let home = Rect::from_center_size(pos2(start + 24.0, y), vec2(48.0, 48.0));
    let response = ui
        .interact(home, Id::unique("window-home"), Sense::click())
        .on_hover_text("Home")
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let at_home = matches!(app.page, Page::Home | Page::New | Page::Radio) && !app.lyrics_page;
    let fill = if response.hovered() {
        theme.surface1
    } else {
        theme.surface0
    };
    ui.painter().circle_filled(home.center(), 24.0, fill);
    let color = if at_home || response.hovered() {
        theme.text
    } else {
        theme.subtext0
    };
    paint_icon(ui, icons::HOME, home.center(), 22.0, color);
    if response.clicked() {
        app.go(Page::Home);
    }
    search(
        app,
        ui,
        Rect::from_min_size(pos2(home.right() + 8.0, y - 24.0), vec2(field, 48.0)),
    );

    // The account: its picture, which opens Settings.
    let avatar = Rect::from_center_size(pos2(bar.right() - 16.0 - 24.0, y), vec2(48.0, 48.0));
    let response = ui
        .interact(avatar, Id::unique("window-account"), Sense::click())
        .on_hover_text("Settings")
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    ui.painter().circle_filled(
        avatar.center(),
        24.0,
        if response.hovered() {
            theme.surface1
        } else {
            theme.surface0
        },
    );
    let picture = Rect::from_center_size(avatar.center(), vec2(32.0, 32.0));
    match &app.profile_picture {
        Some(p) => artwork(ui, Some(p.url(64)), picture, 16.0, &theme),
        None => {
            let name = std::env::var("USER").unwrap_or_default();
            super::sidebar::avatar_disc(ui, &theme, picture.center(), &name);
        }
    }
    if response.clicked() {
        app.go(Page::Settings);
    }
}

/// The search field, as a pill.
fn search(app: &mut App, ui: &mut Ui, rect: Rect) {
    let theme = app.theme.clone();
    let id = Id::unique("window-search");
    let focused = ui.ctx().memory(|m| m.has_focus(id));
    let hovered = ui.rect_contains_pointer(rect);
    let painter = ui.painter();
    let fill = if hovered || focused {
        theme.surface1
    } else {
        theme.surface0
    };
    let stroke = if focused {
        Stroke::new(2.0, theme.text)
    } else if hovered {
        Stroke::new(1.0, theme.text_a(0.12))
    } else {
        Stroke::NONE
    };
    painter.rect(rect, 24.0, fill, stroke, egui::StrokeKind::Inside);
    paint_icon(
        ui,
        icons::SEARCH,
        pos2(rect.left() + 26.0, rect.center().y),
        20.0,
        if focused { theme.text } else { theme.subtext0 },
    );
    // A click anywhere on the pill puts the cursor in it.
    if ui
        .interact(rect, id.with("pill"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::Text)
        .clicked()
    {
        app.focus_search = true;
    }
    let body = font(15.0, Weight::Regular);
    let line = text(ui, "W", body.clone(), theme.text, f32::INFINITY)
        .size()
        .y;
    let field = Rect::from_min_max(
        pos2(rect.left() + 50.0, rect.center().y - line / 2.0),
        pos2(rect.right() - 20.0, rect.center().y + line / 2.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field));
    let response = child.add(
        egui::TextEdit::singleline(&mut app.search)
            .id(id)
            .hint_text(egui::RichText::new("What do you want to play?").color(theme.subtext0))
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
