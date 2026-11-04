//! M3E's player: a floating toolbar, 70 px high and fully round. The song
//! on the left with its cover turning, the transport round a shaped Play
//! over the wavy progress in the middle, the quality, lyrics, queue and
//! volume on the right.

use std::f32::consts::TAU;
use std::time::Duration;

use egui::{Color32, CornerRadius, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use music_core::model::FavouriteKind;

use crate::app::{App, SignalAnchor};
use crate::color::{alpha, mix};
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::fab;
use super::menu::{TrackFacts, track_menu};
use super::motion::{Spring, spring};
use super::player_bar::shown_position;
use super::shapes::Shape;
use super::{explicit_badge, format_time, paint_icon, paint_text, text, thumb_url};

const HEIGHT: f32 = 70.0;

/// The toolbar's layer, which the window puts over the panes' rounded
/// corners, which it reaches, once they are drawn.
pub fn layer() -> egui::LayerId {
    egui::LayerId::new(egui::Order::Middle, Id::unique("toolbar"))
}

/// The toolbar over `span`, the main pane and the side pane together;
/// `side` says whether the side pane shows, where lyrics and the queue open.
pub fn show(app: &mut App, ui: &mut Ui, span: Rect, side: bool) {
    let theme = app.theme.clone();
    let bar = Rect::from_center_size(
        pos2(span.center().x, span.bottom() - 8.0 - HEIGHT / 2.0),
        vec2((span.width() - 16.0).min(1240.0), HEIGHT),
    );

    // Above the page and the side pane, under menus and windows.
    super::register_layer(ui.ctx(), layer(), bar, true);
    let mut layer = ui.new_child(egui::UiBuilder::new().max_rect(bar).layer_id(layer()));
    layer.set_clip_rect(bar.expand(48.0));
    let ui = &mut layer;
    let painter = ui.painter();
    for (y, blur, a) in [(12, 40, 128), (2, 6, 77)] {
        painter.add(
            egui::Shadow {
                offset: [0, y],
                blur,
                spread: 0,
                color: Color32::from_black_alpha(a),
            }
            .as_shape(bar, HEIGHT / 2.0),
        );
    }
    painter.rect_filled(bar, HEIGHT / 2.0, theme.surface0);
    // Clicks on the bar's background must not reach the page underneath.
    ui.interact(bar, Id::unique("toolbar-bg"), Sense::click());

    // Three columns, `1fr · minmax(320, 460) · 1fr`, 16 apart.
    let inner = Rect::from_min_max(
        pos2(bar.left() + 6.0, bar.top()),
        pos2(bar.right() - 14.0, bar.bottom()),
    );
    let room = inner.width() - 32.0;
    let middle = room.min(460.0).max(320.0_f32.min(room));
    let column = (room - middle) / 2.0;
    let left = Rect::from_min_size(inner.min, vec2(column, HEIGHT));
    let centre = Rect::from_min_size(pos2(left.right() + 16.0, inner.top()), vec2(middle, HEIGHT));
    let right = Rect::from_min_max(pos2(centre.right() + 16.0, inner.top()), inner.max);

    song(app, ui, &theme, left);
    transport(app, ui, &theme, centre);
    tools(app, ui, &theme, right, side);
}

/// A round icon button, 40 px, whose corners close in while pressed.
fn button(
    ui: &mut Ui,
    theme: &Theme,
    icon: Icon,
    glyph: f32,
    color: Color32,
    enabled: bool,
) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(40.0, 40.0), sense);
    let pressed = enabled && response.is_pointer_button_down_on();
    let corner = spring(
        ui,
        ("toolbar-press", response.id),
        if pressed { 12.0 } else { 20.0 },
        Spring::Fast,
    );
    if enabled && response.hovered() {
        let fill = if pressed {
            theme.press()
        } else {
            theme.hover()
        };
        ui.painter()
            .rect_filled(rect, corner.clamp(0.0, 20.0), fill);
    }
    let color = if enabled { color } else { alpha(color, 0.38) };
    paint_icon(ui, icon, rect.center(), glyph, color);
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

/// On, a button's icon is in primary with a dot under it.
fn dot(ui: &Ui, theme: &Theme, rect: Rect, on: bool) {
    if on {
        ui.painter().circle_filled(
            pos2(rect.center().x, rect.bottom() - 7.0),
            2.0,
            theme.tonal.primary,
        );
    }
}

/// The cover as a circle turning while it plays, the title and the artists;
/// a click opens Now Playing.
fn song(app: &mut App, ui: &mut Ui, theme: &Theme, column: Rect) {
    let shown = app.shown();
    let Some(track) = shown.loading.clone().or(shown.track.clone()) else {
        return;
    };
    let cover = Rect::from_min_size(
        pos2(column.left() + 4.0, column.center().y - 25.0),
        vec2(50.0, 50.0),
    );
    let words_left = cover.right() + 12.0;
    let room = (column.right() - 8.0 - words_left).max(0.0);
    let title_room = if track.explicit { room - 18.0 } else { room };
    let title = text(
        ui,
        &track.title,
        font(14.5, Weight::SemiBold),
        theme.text,
        title_room,
    );
    let artists = track.artists.join(", ");
    let artist = text(
        ui,
        &artists,
        font(13.0, Weight::Regular),
        theme.subtext0,
        room,
    );
    let words = title.size().x + if track.explicit { 18.0 } else { 0.0 };
    let width = words.max(artist.size().x);
    let pill = Rect::from_min_max(
        pos2(column.left(), cover.top() - 4.0),
        pos2(
            (words_left + width + 8.0).min(column.right()),
            cover.bottom() + 4.0,
        ),
    );
    let response = ui
        .interact(pill, Id::unique("toolbar-song"), Sense::click())
        .on_hover_text("Now Playing")
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.hovered() {
        ui.painter()
            .rect_filled(pill, pill.height() / 2.0, theme.hover());
    }

    let spin = fab::turning(ui, "toolbar-cover", shown.playing, 24.0);
    let circle = Shape::Circle.radii();
    match thumb_url(&track) {
        Some(url) => circle.image(ui, &url, cover, 0.0, spin, theme.surface1),
        None => {
            circle.fill(ui, cover, 0.0, theme.surface1);
            paint_icon(ui, icons::NOTE, cover.center(), 20.0, theme.subtext0);
        }
    }
    let title_end = words_left + title.size().x;
    paint_text(
        ui,
        title,
        words_left,
        cover.center().y - 9.0,
        egui::Align::Min,
        theme.text,
    );
    if track.explicit {
        explicit_badge(
            ui,
            theme,
            pos2(title_end + 6.0 + 6.0, cover.center().y - 9.0),
        );
    }
    paint_text(
        ui,
        artist,
        words_left,
        cover.center().y + 10.0,
        egui::Align::Min,
        theme.subtext0,
    );

    // The track's menu, as in the lists (a station has none).
    let station = track.id.id.starts_with("ra.");
    let facts = TrackFacts {
        file: track.id.provider == music_core::model::ProviderKind::Local,
        has_album: track.album_id.is_some(),
        favourite: app.is_favourite(FavouriteKind::Song, &track.id),
        in_library: app.track_in_library(&track),
        removable: track.library_id.is_some(),
        in_playlist: false,
        download: app.download_offer(&track.id),
    };
    let choice = if station {
        None
    } else {
        track_menu(&response, theme, facts)
    };
    if response.clicked() {
        app.now_playing = true;
    }
    if let Some(choice) = choice {
        app.track_choice(track, choice, ui.input(|i| i.time));
    }
}

/// Shuffle, previous, the Play button, next and repeat, over the progress
/// and the times.
fn transport(app: &mut App, ui: &mut Ui, theme: &Theme, column: Rect) {
    let shown = app.shown();
    let has_track = shown.has_track;
    let dim = theme.subtext0;
    let width = 4.0 * 40.0 + 42.0 + 12.0 + 4.0 * 2.0;
    let row_rect = Rect::from_center_size(
        pos2(column.center().x, column.top() + 26.0),
        vec2(width, 42.0),
    );
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    row.spacing_mut().item_spacing.x = 2.0;

    let color = |on: bool| if on { theme.tonal.primary } else { dim };
    let response = button(
        &mut row,
        theme,
        icons::SHUFFLE,
        18.0,
        color(shown.shuffle),
        true,
    )
    .on_hover_text("Shuffle");
    dot(&row, theme, response.rect, shown.shuffle);
    if response.clicked() {
        app.toggle_shuffle();
    }
    if button(&mut row, theme, icons::BACK_FILL, 20.0, dim, has_track)
        .on_hover_text("Previous")
        .clicked()
    {
        app.previous();
    }
    row.add_space(6.0);
    let (rect, _) = row.allocate_exact_size(vec2(42.0, 42.0), Sense::hover());
    let play = fab::fab(
        &mut row,
        theme,
        rect,
        "toolbar-play",
        shown.playing,
        shown.loading.is_some(),
        has_track,
    );
    if play.clicked() {
        app.toggle_play();
    }
    row.add_space(6.0);
    if button(&mut row, theme, icons::FORWARD_FILL, 20.0, dim, has_track)
        .on_hover_text("Next")
        .clicked()
    {
        app.next();
    }
    let (icon, on) = match shown.repeat {
        music_core::Repeat::Off => (icons::REPEAT, false),
        music_core::Repeat::All => (icons::REPEAT, true),
        music_core::Repeat::One => (icons::REPEAT_ONE, true),
    };
    let response = button(&mut row, theme, icon, 18.0, color(on), true).on_hover_text("Repeat");
    dot(&row, theme, response.rect, on);
    if response.clicked() {
        app.cycle_repeat();
    }

    // The times either side of the progress.
    let y = column.top() + 54.0;
    let position = shown_position(app, ui);
    let duration = shown
        .duration
        .filter(|d| !d.is_zero() && shown.track.is_some());
    let small = font(12.0, Weight::Regular);
    let g = text(ui, &format_time(position), small.clone(), dim, 60.0);
    paint_text(ui, g, column.left() + 38.0, y, egui::Align::Max, dim);
    let left = match duration {
        Some(total) => format!("-{}", format_time(total.saturating_sub(position))),
        None if shown.playing => "LIVE".into(),
        None => String::new(),
    };
    let g = text(ui, &left, small, dim, 60.0);
    paint_text(ui, g, column.right() - 38.0, y, egui::Align::Min, dim);
    let wave = Rect::from_min_max(
        pos2(column.left() + 48.0, y - 10.0),
        pos2(column.right() - 48.0, y + 10.0),
    );
    wavy(app, ui, theme, wave, duration, shown.playing, false);
}

/// M3E's wavy progress: the part played a wave in primary, moving while it
/// plays and flattening when paused, a gap, the rest flat, a stop dot at the
/// end and a bar at the position. Click or drag to seek. `big`, Now
/// Playing's, is thicker and taller.
pub(super) fn wavy(
    app: &mut App,
    ui: &mut Ui,
    theme: &Theme,
    rect: Rect,
    duration: Option<Duration>,
    playing: bool,
    big: bool,
) {
    let (stroke, amplitude, wavelength, gap, handle) = if big {
        (6.0, 5.0, 44.0, 8.0, 28.0)
    } else {
        (4.0, 3.2, 32.0, 6.0, 22.0)
    };

    // How tall the wave stands, and where it is along.
    let id = Id::unique(("wave", big));
    let now = ui.input(|i| i.time);
    let (mut amp, mut phase, then) = ui.data(|d| d.get_temp::<(f32, f32, f64)>(id)).unwrap_or((
        if playing { 1.0 } else { 0.0 },
        0.0,
        now,
    ));
    let dt = (now - then).min(0.1) as f32;
    let target = if playing { 1.0 } else { 0.0 };
    amp += (target - amp) * (dt * 6.0).min(1.0);
    if playing {
        phase = (phase - dt * 4.2) % TAU;
    }
    if playing || (amp - target).abs() > 0.002 {
        ui.ctx().request_repaint();
    }
    ui.data_mut(|d| d.insert_temp(id, (amp, phase, now)));

    let cy = rect.center().y;
    let (start, end) = (rect.left() + stroke / 2.0, rect.right() - stroke / 2.0);
    let painter = ui.painter();
    let flat = |painter: &egui::Painter, from: f32, to: f32, color: Color32| {
        if to > from {
            let r = Rect::from_min_max(
                pos2(from - stroke / 2.0, cy - stroke / 2.0),
                pos2(to + stroke / 2.0, cy + stroke / 2.0),
            );
            painter.rect_filled(r, stroke / 2.0, color);
        }
    };
    let Some(total) = duration else {
        // A station on air: no position along it.
        flat(painter, start, end, theme.tonal.secondary_container);
        return;
    };

    let response = ui.interact(
        rect.expand2(vec2(0.0, 6.0)),
        Id::unique(("seek", big)),
        Sense::click_and_drag(),
    );
    let preview_id = Id::unique("seek-preview");
    let at = |x: f32| total.mul_f32(((x - rect.left()) / rect.width()).clamp(0.0, 1.0));
    if let Some(pointer) = response.interact_pointer_pos() {
        if response.dragged() {
            ui.data_mut(|d| d.insert_temp(preview_id, at(pointer.x)));
        }
        if response.drag_stopped() || response.clicked() {
            app.seek(at(pointer.x));
            ui.data_mut(|d| d.remove::<Duration>(preview_id));
        }
    }
    let dragging = response.dragged();
    let position = shown_position(app, ui);
    let p = (position.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0);
    let x = start + p * (end - start);
    let played_end = (x - gap / 2.0 - if dragging { 2.0 } else { 0.0 }).max(start);

    let painter = ui.painter();
    let y = |x: f32| cy + amplitude * amp * ((x - rect.left()) / wavelength * TAU + phase).sin();
    if played_end > start {
        let mut points: Vec<_> = (0..)
            .map(|i| start + 2.0 * i as f32)
            .take_while(|&px| px < played_end)
            .map(|px| pos2(px, y(px)))
            .collect();
        points.push(pos2(played_end, y(played_end)));
        let color = theme.tonal.primary;
        // Round ends, as the stroke's caps.
        painter.circle_filled(points[0], stroke / 2.0, color);
        painter.circle_filled(*points.last().unwrap(), stroke / 2.0, color);
        painter.add(egui::Shape::line(points, Stroke::new(stroke, color)));
    }
    flat(painter, x + gap, end, theme.tonal.secondary_container);
    painter.circle_filled(pos2(end, cy), stroke / 2.0 - 0.5, theme.tonal.primary);
    painter.rect_filled(
        Rect::from_center_size(pos2(x, cy), vec2(4.0, handle)),
        2.0,
        theme.tonal.primary,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
}

/// The quality label, lyrics, queue, mute and the volume, from the right.
fn tools(app: &mut App, ui: &mut Ui, theme: &Theme, column: Rect, side: bool) {
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_center_size(
                column.center(),
                vec2(column.width(), 40.0),
            ))
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    row.spacing_mut().item_spacing.x = 2.0;
    let shown = app.shown();
    let dim = theme.subtext0;

    if ui.ctx().content_rect().width() >= 1360.0 {
        volume(app, &mut row, theme, &shown);
    }
    let muted = shown.muted || shown.volume == Some(0.0);
    if button(
        &mut row,
        theme,
        if muted { icons::VOL_MUTE } else { icons::VOL },
        19.0,
        dim,
        shown.volume.is_some(),
    )
    .on_hover_text(if shown.muted { "Unmute" } else { "Mute" })
    .clicked()
    {
        app.toggle_mute();
    }

    let color = |on: bool| if on { theme.tonal.primary } else { dim };
    use crate::app::Side;
    let queue_on = if side {
        matches!(app.side, Some(Side::Queue | Side::History))
    } else {
        app.drawer_shows(false)
    };
    let response =
        button(&mut row, theme, icons::QUEUE, 19.0, color(queue_on), true).on_hover_text("Queue");
    dot(&row, theme, response.rect, queue_on);
    if response.clicked() {
        if side {
            app.side = if queue_on { None } else { Some(Side::Queue) };
        } else {
            app.toggle_drawer(false);
        }
    }
    let lyrics_on = if side {
        app.side == Some(Side::Lyrics)
    } else {
        app.drawer_shows(true)
    };
    let response =
        button(&mut row, theme, icons::MIC, 19.0, color(lyrics_on), true).on_hover_text("Lyrics");
    dot(&row, theme, response.rect, lyrics_on);
    if response.clicked() {
        if side {
            app.side = if lyrics_on { None } else { Some(Side::Lyrics) };
        } else {
            app.toggle_drawer(true);
        }
    }

    if let Some(track) = shown.track.as_ref() {
        quality(app, &mut row, theme, track);
    }
}

/// What plays, as a label: lossless in the tertiary container, a yellow
/// dot when the signal isn't bit-perfect. It opens the signal path.
pub(super) fn quality(app: &mut App, ui: &mut Ui, theme: &Theme, track: &music_core::model::Track) {
    let (label, lossless) = if super::plays_in_atmos(track) {
        ("Dolby Atmos".to_string(), true)
    } else {
        match super::quality_label(track) {
            Some(q) => q,
            None => return,
        }
    };
    let (fill, ink) = if lossless {
        (
            theme.tonal.tertiary_container,
            theme.tonal.on_tertiary_container,
        )
    } else {
        (theme.surface1, theme.subtext0)
    };
    let perfect = super::signal::assess(app).perfect();
    let logo = super::rail::logo(track.id.provider);
    let g = text(ui, &label, font(11.5, Weight::SemiBold), ink, 200.0);
    let width = 10.0
        + logo.map_or(0.0, |_| 14.0 + 5.0)
        + g.size().x
        + if perfect { 0.0 } else { 5.0 + 6.0 }
        + 10.0;
    ui.add_space(4.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 28.0), Sense::click());
    let fill = if response.hovered() {
        mix(Color32::WHITE, fill, 0.08)
    } else {
        fill
    };
    ui.painter().rect_filled(rect, 12.0, fill);
    let mut x = rect.left() + 10.0;
    if let Some(logo) = logo {
        paint_icon(ui, logo, pos2(x + 7.0, rect.center().y), 14.0, ink);
        x += 14.0 + 5.0;
    }
    let w = g.size().x;
    paint_text(ui, g, x, rect.center().y, egui::Align::Min, ink);
    if !perfect {
        ui.painter()
            .circle_filled(pos2(x + w + 5.0 + 3.0, rect.center().y), 3.0, theme.yellow);
    }
    super::signal::remember_anchor(ui, SignalAnchor::PlayerBar, rect);
    if response
        .on_hover_text("Signal path")
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        app.signal_open = match app.signal_open {
            Some(SignalAnchor::PlayerBar) => None,
            _ => Some(SignalAnchor::PlayerBar),
        };
    }
}

/// M3E's slider: a 12 px track, primary up to the handle and the secondary
/// container after it, 6 px either side of a 4 × 28 bar that thins while
/// dragged.
fn volume(app: &mut App, ui: &mut Ui, theme: &Theme, shown: &crate::app::Shown) {
    let sense = if shown.volume.is_some() {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(88.0, 32.0), sense);
    if let Some(pointer) = response.interact_pointer_pos()
        && (response.dragged() || response.clicked())
    {
        app.set_volume((pointer.x - rect.left()) / rect.width());
    }
    let level = if shown.muted {
        0.0
    } else {
        shown.volume.unwrap_or(0.0)
    };
    let x = rect.left() + (level * rect.width()).clamp(2.0, rect.width() - 2.0);
    let track = |from: f32, to: f32| {
        Rect::from_min_max(pos2(from, rect.top() + 10.0), pos2(to, rect.top() + 22.0))
    };
    let painter = ui.painter();
    if x - 6.0 > rect.left() {
        painter.rect_filled(
            track(rect.left(), x - 6.0),
            CornerRadius {
                nw: 6,
                sw: 6,
                ne: 2,
                se: 2,
            },
            theme.tonal.primary,
        );
    }
    if x + 6.0 < rect.right() {
        painter.rect_filled(
            track(x + 6.0, rect.right()),
            CornerRadius {
                nw: 2,
                sw: 2,
                ne: 6,
                se: 6,
            },
            theme.tonal.secondary_container,
        );
    }
    let thick = spring(
        ui,
        "toolbar-volume",
        if response.dragged() { 2.0 } else { 4.0 },
        Spring::Fast,
    );
    painter.rect_filled(
        Rect::from_center_size(pos2(x, rect.center().y), vec2(thick.max(1.0), 28.0)),
        2.0,
        theme.tonal.primary,
    );
    if shown.volume.is_none() {
        response.on_hover_text("Its volume is set on the device itself");
        return;
    }
    let tip = match &shown.on {
        Some(device) => format!("Volume on {} {:.0} %", device.name, level * 100.0),
        None => format!("Volume {:.0} %", level * 100.0),
    };
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tip);
}
