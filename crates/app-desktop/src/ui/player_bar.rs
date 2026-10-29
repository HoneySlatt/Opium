//! The floating player bar, modelled on Cider's: transport and time on the left,
//! the "LCD" (artwork, title, quality, position hairline) in the middle, volume
//! and the drawer buttons on the right. 44 px high, at most 1000 px wide.

use std::time::Duration;

use egui::{Color32, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use music_core::model::FavouriteKind;

use crate::app::App;
use crate::color::{alpha, mix};
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::menu::{TrackFacts, track_menu};

use super::{
    artwork, explicit_badge, format_time, icon_button, paint_text, quality_label, text, thumb_url,
};

pub fn show(app: &mut App, ui: &mut Ui, main: Rect) {
    let theme = app.theme.clone();
    let width = (main.width() - 40.0).min(1000.0);
    let bar = Rect::from_center_size(
        pos2(main.center().x, main.bottom() - 14.0 - 22.0),
        vec2(width, 44.0),
    );

    // Above the page, but under menus and windows, which may reach over it.
    let mut layer = ui.new_child(egui::UiBuilder::new().max_rect(bar).layer_id(
        egui::LayerId::new(egui::Order::Middle, Id::unique("player-bar")),
    ));
    let ui = &mut layer;
    let painter = ui.painter();
    painter.add(
        egui::Shadow {
            offset: [0, 12],
            blur: 32,
            spread: 0,
            color: Color32::from_black_alpha(120),
        }
        .as_shape(bar.shrink(4.0), 11.0),
    );
    painter.rect(
        bar,
        11.0,
        theme.mantle,
        Stroke::new(1.0, theme.text_a(0.05)),
        egui::StrokeKind::Inside,
    );
    // Clicks on the bar's background must not reach the page underneath.
    ui.interact(bar, Id::unique("player-bar-bg"), Sense::click());

    let inner = bar.shrink2(vec2(6.0, 0.0));
    let controls = transport(app, ui, &theme, inner);
    let right = right_side(app, ui, &theme, inner);
    let lcd = Rect::from_min_max(
        pos2(controls + 8.0, bar.center().y - 18.0),
        pos2(right - 8.0, bar.center().y + 18.0),
    );
    if lcd.width() > 120.0 {
        now_playing(app, ui, &theme, lcd);
    }
}

/// Shuffle, previous, play/pause, next, repeat, then the time. Returns its right edge.
fn transport(app: &mut App, ui: &mut Ui, theme: &Theme, inner: Rect) -> f32 {
    let dim = theme.text_a(0.82);
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    row.spacing_mut().item_spacing.x = 0.0;
    let has_track = app.queue.current().is_some();

    let shuffle_color = if app.queue.shuffle() {
        theme.accent
    } else {
        dim
    };
    if icon_button(
        &mut row,
        theme,
        icons::SHUFFLE,
        30.0,
        15.0,
        shuffle_color,
        true,
    )
    .on_hover_text("Shuffle")
    .clicked()
    {
        app.toggle_shuffle();
    }
    if icon_button(
        &mut row,
        theme,
        icons::BACK_FILL,
        30.0,
        16.0,
        dim,
        has_track,
    )
    .clicked()
    {
        app.previous();
    }
    let (icon, tip) = if app.is_playing() {
        (icons::PAUSE_FILL, "Pause")
    } else {
        (icons::PLAY_FILL, "Play")
    };
    if app.loading().is_some() {
        // In the play button's place while the track is on its way.
        let (rect, _) = row.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
        let mut child = row.new_child(egui::UiBuilder::new().max_rect(rect));
        child
            .put(
                Rect::from_center_size(rect.center(), vec2(16.0, 16.0)),
                egui::Spinner::new().size(16.0).color(theme.text),
            )
            .on_hover_text("Loading");
    } else if icon_button(&mut row, theme, icon, 32.0, 18.0, theme.text, has_track)
        .on_hover_text(tip)
        .clicked()
    {
        app.toggle_play();
    }
    if icon_button(
        &mut row,
        theme,
        icons::FORWARD_FILL,
        30.0,
        16.0,
        dim,
        has_track,
    )
    .clicked()
    {
        app.next();
    }
    let (repeat_icon, repeat_color) = match app.queue.repeat() {
        music_core::Repeat::Off => (icons::REPEAT, dim),
        music_core::Repeat::All => (icons::REPEAT, theme.accent),
        music_core::Repeat::One => (icons::REPEAT_ONE, theme.accent),
    };
    if icon_button(&mut row, theme, repeat_icon, 30.0, 15.0, repeat_color, true)
        .on_hover_text("Repeat")
        .clicked()
    {
        app.cycle_repeat();
    }

    let position = shown_position(app, ui);
    let time = match app.duration() {
        _ if app.loading().is_some() => "Loading…".into(),
        Some(total) if app.now.is_some() => format!(
            "{} / -{}",
            format_time(position),
            format_time(total.saturating_sub(position))
        ),
        // A station on air has no end.
        None if app.is_playing() => format!("{} · LIVE", format_time(position)),
        _ => "0:00 / -0:00".into(),
    };
    row.add_space(10.0);
    let galley = text(
        &row,
        &time,
        font(12.0, Weight::Regular),
        theme.muted(),
        200.0,
    );
    let (rect, _) = row.allocate_exact_size(vec2(galley.size().x, 20.0), Sense::hover());
    paint_text(
        &row,
        galley,
        rect.left(),
        rect.center().y,
        egui::Align::Min,
        theme.muted(),
    );
    rect.right() + 4.0
}

/// Mute, volume, queue and Now Playing. Returns its left edge.
fn right_side(app: &mut App, ui: &mut Ui, theme: &Theme, inner: Rect) -> f32 {
    let dim = theme.text_a(0.82);
    let width = 30.0 + 6.0 + 84.0 + 8.0 + 30.0 + 30.0;
    let area = Rect::from_min_max(pos2(inner.right() - width, inner.top()), inner.max);
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(area)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    row.spacing_mut().item_spacing.x = 0.0;
    let mute_icon = if app.muted || app.volume == 0.0 {
        icons::VOL_MUTE
    } else {
        icons::VOL
    };
    if icon_button(&mut row, theme, mute_icon, 30.0, 16.0, dim, true)
        .on_hover_text(if app.muted { "Unmute" } else { "Mute" })
        .clicked()
    {
        app.toggle_mute();
    }
    row.add_space(6.0);
    volume_slider(app, &mut row, theme);
    row.add_space(8.0);
    let queue_color = if app.drawer_shows(false) {
        theme.accent
    } else {
        dim
    };
    if icon_button(&mut row, theme, icons::QUEUE, 30.0, 16.0, queue_color, true)
        .on_hover_text("Playing next")
        .clicked()
    {
        app.toggle_drawer(false);
    }
    if icon_button(&mut row, theme, icons::EXPAND, 30.0, 15.0, dim, true)
        .on_hover_text("Now Playing")
        .clicked()
    {
        app.now_playing = true;
    }
    area.left()
}

fn volume_slider(app: &mut App, ui: &mut Ui, theme: &Theme) {
    let (rect, response) = ui.allocate_exact_size(vec2(84.0, 14.0), Sense::click_and_drag());
    if let Some(pointer) = response.interact_pointer_pos()
        && (response.dragged() || response.clicked())
    {
        app.set_volume((pointer.x - rect.left()) / rect.width());
    }
    let level = if app.muted { 0.0 } else { app.volume };
    let rail = Rect::from_center_size(rect.center(), vec2(rect.width(), 3.0));
    let painter = ui.painter();
    painter.rect_filled(rail, 3.0, theme.text_a(0.14));
    let fill = Rect::from_min_size(rail.min, vec2(rail.width() * level, rail.height()));
    painter.rect_filled(fill, 3.0, theme.text_a(0.55));
    let knob = if response.hovered() || response.dragged() {
        theme.text
    } else {
        mix(theme.text, theme.mantle, 0.70)
    };
    painter.circle_filled(
        pos2(rail.left() + rail.width() * level, rail.center().y),
        5.5,
        knob,
    );
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(format!("Volume {:.0} %", level * 100.0));
}

/// The playing position, or where the seek hairline is being dragged to.
fn shown_position(app: &App, ui: &Ui) -> Duration {
    ui.ctx()
        .data(|d| d.get_temp::<Duration>(Id::unique("seek-preview")))
        .unwrap_or(app.status.position)
}

fn now_playing(app: &mut App, ui: &mut Ui, theme: &Theme, lcd: Rect) {
    // The track picked shows at once, while it loads.
    let Some(track) = app.loading().cloned().or_else(|| app.now.clone()) else {
        return;
    };
    let art = Rect::from_min_size(
        pos2(lcd.left() + 2.0, lcd.center().y - 15.0),
        vec2(30.0, 30.0),
    );
    artwork(ui, thumb_url(&track), art, 3.0, theme);
    // The track's menu, as in the lists, over the artwork and the words (a
    // station has none). The artwork and the album link sense clicks of their
    // own, so they carry the menu too.
    let chip_right = lcd.right() - 4.0;
    let words = Rect::from_min_max(lcd.min, pos2(chip_right - 80.0, lcd.max.y));
    let area = ui.interact(words, Id::unique("player-track"), Sense::click());
    let (has_album, favourite, in_library) = (
        track.album_id.is_some(),
        app.is_favourite(FavouriteKind::Song, &track.id),
        app.track_in_library(&track),
    );
    let removable = track.library_id.is_some();
    let facts = || TrackFacts {
        has_album,
        favourite,
        in_library,
        removable,
        in_playlist: false,
    };
    let station = track.id.id.starts_with("ra.");
    let mut choice = None;
    if !station {
        choice = choice.or(track_menu(&area, theme, facts()));
    }
    // The artwork opens Now Playing, with an expand icon on hover.
    let open = ui.interact(art, Id::unique("player-art"), Sense::click());
    if !station {
        choice = choice.or(track_menu(&open, theme, facts()));
    }
    if open.hovered() {
        ui.painter()
            .rect_filled(art, 3.0, Color32::from_black_alpha(115));
        super::paint_icon(ui, icons::EXPAND, art.center(), 14.0, Color32::WHITE);
    }
    if open
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        app.now_playing = true;
    }

    // The quality chip, right-aligned.
    let chip_left = quality_chip(app, ui, theme, &track, chip_right, lcd.center().y);

    let left = art.right() + 10.0;
    let width = (chip_left - 10.0 - left).max(0.0);
    let mut title_width = width;
    if track.explicit {
        title_width -= 17.0;
    }
    let title = text(
        ui,
        &track.title,
        font(12.5, Weight::Medium),
        theme.text,
        title_width,
    );
    let title_end = left + title.size().x;
    paint_text(
        ui,
        title,
        left,
        lcd.center().y - 7.0,
        egui::Align::Min,
        theme.text,
    );
    if track.explicit {
        explicit_badge(ui, theme, pos2(title_end + 5.0 + 6.0, lcd.center().y - 7.0));
    }
    let album = track.album.clone().unwrap_or_default();
    let subtitle = if album.is_empty() {
        track.artists.join(", ")
    } else {
        format!("{album} - {}", track.artists.join(", "))
    };
    let galley = text(
        ui,
        &subtitle,
        font(11.5, Weight::Regular),
        theme.muted(),
        width,
    );
    // The subtitle links to the album.
    let y = lcd.center().y + 8.0;
    let link = Rect::from_min_size(pos2(left, y - galley.size().y / 2.0), galley.size());
    let mut color = theme.muted();
    if let Some(album) = &track.album_id {
        let hit = ui.interact(link, Id::unique("player-album"), Sense::click());
        if !station {
            choice = choice.or(track_menu(&hit, theme, facts()));
        }
        if hit.hovered() {
            color = theme.text;
            ui.painter()
                .hline(link.x_range(), link.bottom(), Stroke::new(1.0, color));
        }
        if hit
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            app.go(crate::app::Page::Album(album.clone()));
        }
    }
    paint_text(ui, galley, left, y, egui::Align::Min, color);

    seek_hairline(app, ui, theme, lcd);
    if let Some(choice) = choice {
        app.track_choice(track, choice, ui.input(|i| i.time));
    }
}

/// Returns its left edge.
fn quality_chip(
    app: &mut App,
    ui: &mut Ui,
    theme: &Theme,
    track: &music_core::model::Track,
    right: f32,
    y: f32,
) -> f32 {
    let Some((label, lossless)) = quality_label(track) else {
        return right;
    };
    let assessment = super::signal::assess(app);
    let perfect = assessment.perfect();
    let color = theme.muted();
    let galley = text(ui, &label, font(10.5, Weight::SemiBold), color, 200.0);
    let dot = if perfect { 0.0 } else { 7.0 + 5.0 };
    let rect = Rect::from_min_max(
        pos2(right - galley.size().x - 14.0 - dot, y - 10.0),
        pos2(right, y + 10.0),
    );
    let response = ui.interact(rect, Id::unique("quality-chip"), Sense::click());
    super::signal::remember_anchor(ui, crate::app::SignalAnchor::PlayerBar, rect);
    let open = app.signal_open == Some(crate::app::SignalAnchor::PlayerBar);
    let fill = if response.hovered() || open {
        theme.text_a(0.11)
    } else if lossless {
        alpha(theme.accent, 0.14)
    } else {
        theme.text_a(0.06)
    };
    ui.painter().rect_filled(rect, 5.0, fill);
    let text_color = if response.hovered() {
        theme.text
    } else {
        color
    };
    if !perfect {
        ui.painter()
            .circle_filled(pos2(rect.left() + 7.0 + 3.5, y), 3.5, theme.yellow);
    }
    paint_text(
        ui,
        galley,
        rect.left() + 7.0 + dot,
        y,
        egui::Align::Min,
        text_color,
    );
    let source = match track.id.provider {
        music_core::model::ProviderKind::AppleMusic => "Apple Music",
        music_core::model::ProviderKind::SoundCloud => "SoundCloud",
        music_core::model::ProviderKind::Navidrome => "Navidrome",
    };
    let clicked = response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(format!(
            "{source} · {label}\n{}",
            super::signal::tooltip(&assessment)
        ))
        .clicked();
    if clicked {
        app.signal_open = if open {
            None
        } else {
            Some(crate::app::SignalAnchor::PlayerBar)
        };
    }
    rect.left()
}

/// The position as a hairline along the LCD's bottom: 2 px, 4 px with a knob
/// and in the accent on hover. Click or drag to seek.
fn seek_hairline(app: &mut App, ui: &mut Ui, theme: &Theme, lcd: Rect) {
    let Some(total) = app.duration().filter(|d| !d.is_zero()) else {
        return;
    };
    let hit = Rect::from_min_max(
        pos2(lcd.left() + 2.0, lcd.bottom() - 4.0),
        pos2(lcd.right() - 4.0, lcd.bottom() + 4.0),
    );
    let response = ui.interact(hit, Id::unique("seek"), Sense::click_and_drag());
    let preview_id = Id::unique("seek-preview");
    let at = |x: f32| total.mul_f32(((x - hit.left()) / hit.width()).clamp(0.0, 1.0));
    if let Some(pointer) = response.interact_pointer_pos() {
        if response.dragged() {
            ui.ctx()
                .data_mut(|d| d.insert_temp(preview_id, at(pointer.x)));
        }
        if response.drag_stopped() || response.clicked() {
            app.seek(at(pointer.x));
            ui.ctx().data_mut(|d| d.remove::<Duration>(preview_id));
        }
    }
    let active = response.hovered() || response.dragged();
    let position = shown_position(app, ui);
    let fraction = (position.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0);
    let thickness = if active { 4.0 } else { 2.0 };
    let rail = Rect::from_center_size(hit.center(), vec2(hit.width(), thickness));
    let painter = ui.painter();
    painter.rect_filled(rail, thickness / 2.0, theme.text_a(0.07));
    let fill = Rect::from_min_size(rail.min, vec2(rail.width() * fraction, thickness));
    let fill_color = if active {
        theme.accent
    } else {
        theme.text_a(0.75)
    };
    painter.rect_filled(fill, thickness / 2.0, fill_color);
    if active {
        painter.circle_filled(pos2(fill.right(), rail.center().y), 5.5, theme.text);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
}
