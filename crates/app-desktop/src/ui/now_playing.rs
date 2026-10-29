//! Now Playing: the whole window, over the cover's colours. The artwork, title,
//! position, transport and quality on the left; Playing Next and Lyrics on the
//! right (hidden when the window is narrow).

use std::time::Duration;

use egui::{Color32, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use music_core::Repeat;
use music_core::model::ProviderKind;

use crate::app::{App, NpTab, Page, SignalAnchor};
use crate::color::alpha;
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::{
    Blob, NARROW, artwork, drawer, explicit_badge, format_time, icon_button, paint_icon,
    paint_text, quality_label, signal, text, wash_with,
};

pub fn show(app: &mut App, ui: &mut Ui) {
    let theme = app.theme.clone();
    let full = ui.max_rect();
    ui.painter().rect_filled(full, 0.0, theme.base);
    backdrop(app, ui, &theme, full);

    let close = Rect::from_min_size(full.min + vec2(18.0, 14.0), vec2(36.0, 36.0));
    let mut close_ui = ui.new_child(egui::UiBuilder::new().max_rect(close));
    if icon_button(
        &mut close_ui,
        &theme,
        icons::DOWN,
        36.0,
        22.0,
        theme.subtext0,
        true,
    )
    .on_hover_text("Close Now Playing")
    .clicked()
    {
        app.now_playing = false;
        return;
    }

    let narrow = full.width() < NARROW;
    let inner = if narrow {
        Rect::from_min_max(full.min + vec2(32.0, 56.0), full.max - vec2(32.0, 32.0))
    } else {
        Rect::from_min_max(full.min + vec2(64.0, 64.0), full.max - vec2(64.0, 48.0))
    };
    let gap = 56.0;
    let right_width = if narrow {
        0.0
    } else {
        440.0_f32.min((inner.width() - gap - 380.0).max(340.0))
    };
    let left = Rect::from_min_max(
        inner.min,
        pos2(
            inner.right() - if narrow { 0.0 } else { right_width + gap },
            inner.bottom(),
        ),
    );
    left_column(app, ui, &theme, left, full.height());
    if !narrow {
        let right = Rect::from_min_max(
            pos2(inner.right() - right_width, inner.top() + 12.0),
            inner.max,
        );
        right_column(app, ui, &theme, right);
    }
}

/// The cover's three colours as four large blobs across the window, drifting
/// slowly, under a vignette of the background. No blurred artwork, so a dark
/// cover cannot muddy a light theme.
fn backdrop(app: &mut App, ui: &Ui, theme: &Theme, full: Rect) {
    let colors = app
        .now
        .as_ref()
        .and_then(|t| t.artwork.as_ref())
        .map(|a| a.url(96))
        .and_then(|url| app.palette(&url))
        .map(|p| p.map(|c| crate::palette::fit(c, theme.light)))
        .unwrap_or([theme.accent, theme.mauve, theme.accent]);
    let [c1, c2, c3] = colors;
    // 40 s one way, then back.
    let t = ui.input(|i| i.time) as f32;
    let drift = (t / 40.0 * std::f32::consts::PI).sin() * 0.04;
    let blob = |x: f32, y: f32, rx: f32, ry: f32, color: Color32, alpha: f32| Blob {
        center: vec2(x + drift, y - drift * 0.6),
        radii: vec2(rx, ry),
        color,
        alpha,
        fade: 0.85,
    };
    let blobs = [
        blob(0.22, 0.34, 0.42, 0.52, c1, 0.95),
        blob(0.78, 0.22, 0.40, 0.50, c2, 0.85),
        blob(0.70, 0.86, 0.46, 0.56, c3, 0.80),
        blob(0.12, 0.92, 0.36, 0.46, c2, 0.60),
    ];
    let vignette = |f: egui::Vec2| {
        let d = ((f - vec2(0.30, 0.40)) / vec2(1.2, 1.0)).length() * 2.0;
        0.6 * d.min(1.0)
    };
    wash_with(ui, full, theme.base, &blobs, vignette);
    ui.ctx().request_repaint_after(Duration::from_millis(50));
}

fn left_column(app: &mut App, ui: &mut Ui, theme: &Theme, column: Rect, window_height: f32) {
    let Some(track) = app.now.clone() else {
        let g = text(
            ui,
            "Nothing playing",
            font(18.0, Weight::SemiBold),
            theme.subtext0,
            column.width(),
        );
        paint_text(
            ui,
            g,
            column.center().x,
            column.center().y,
            egui::Align::Center,
            theme.subtext0,
        );
        return;
    };
    let size = 400.0_f32.min(window_height * 0.46).min(column.width());
    let block = size + 28.0 + 26.0 + 22.0 + 18.0 + 14.0 + 16.0 + 12.0 + 56.0 + 14.0 + 24.0;
    let top = column.center().y - block / 2.0;
    let x = column.center().x - size / 2.0;

    // The artwork, a little smaller while paused.
    let paused = ui
        .ctx()
        .animate_bool_with_time(Id::unique("np-paused"), !app.is_playing(), 0.3);
    let scale = 1.0 - 0.06 * paused;
    let art = Rect::from_center_size(
        pos2(x + size / 2.0, top + size / 2.0),
        vec2(size, size) * scale,
    );
    ui.painter().add(
        egui::Shadow {
            offset: [0, 30],
            blur: 70,
            spread: 0,
            color: Color32::from_black_alpha(150),
        }
        .as_shape(art.shrink(18.0), 14.0),
    );
    artwork(
        ui,
        track.artwork.as_ref().map(|a| a.url(600)),
        art,
        14.0,
        theme,
    );

    // Title, with the E badge, then artist — album, the album a link.
    let mut y = top + size + 28.0;
    let title = text(
        ui,
        &track.title,
        font(22.0, Weight::Bold),
        theme.text,
        size - 24.0,
    );
    let title_end = x + title.size().x;
    paint_text(ui, title, x, y + 13.0, egui::Align::Min, theme.text);
    if track.explicit {
        explicit_badge(ui, theme, pos2(title_end + 8.0 + 6.0, y + 13.0));
    }
    y += 26.0;
    let artist = track.artists.join(", ");
    let a = text(
        ui,
        &artist,
        font(16.0, Weight::Regular),
        theme.subtext0,
        size,
    );
    let artist_w = a.size().x;
    paint_text(ui, a, x, y + 11.0, egui::Align::Min, theme.subtext0);
    if let Some(album) = &track.album {
        let dash = text(ui, " — ", font(16.0, Weight::Regular), theme.subtext0, 40.0);
        let dash_w = dash.size().x;
        paint_text(
            ui,
            dash,
            x + artist_w,
            y + 11.0,
            egui::Align::Min,
            theme.subtext0,
        );
        let ax = x + artist_w + dash_w;
        let g = text(
            ui,
            album,
            font(16.0, Weight::Regular),
            theme.subtext0,
            (x + size - ax).max(0.0),
        );
        let link = Rect::from_min_size(pos2(ax, y + 11.0 - g.size().y / 2.0), g.size());
        let mut color = theme.subtext0;
        if let Some(id) = &track.album_id {
            let hit = ui.interact(link, Id::unique("np-album"), Sense::click());
            if hit.hovered() {
                color = theme.text;
                ui.painter()
                    .hline(link.x_range(), link.bottom(), Stroke::new(1.0, color));
            }
            if hit
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                app.now_playing = false;
                app.go(Page::Album(id.clone()));
            }
        }
        paint_text(ui, g, ax, y + 11.0, egui::Align::Min, color);
    }
    y += 22.0 + 18.0;

    // Position and times.
    let position = seek_bar(
        app,
        ui,
        theme,
        Rect::from_min_size(pos2(x, y), vec2(size, 14.0)),
    );
    y += 14.0;
    let total = app.duration().unwrap_or_default();
    let muted = theme.muted();
    let g = text(
        ui,
        &format_time(position),
        font(11.5, Weight::Regular),
        muted,
        80.0,
    );
    paint_text(ui, g, x, y + 8.0, egui::Align::Min, muted);
    let remaining = format!("-{}", format_time(total.saturating_sub(position)));
    let g = text(ui, &remaining, font(11.5, Weight::Regular), muted, 80.0);
    paint_text(ui, g, x + size, y + 8.0, egui::Align::Max, muted);
    y += 16.0 + 12.0;

    // Transport, spread across the column.
    transport(
        app,
        ui,
        theme,
        Rect::from_min_size(pos2(x, y), vec2(size, 56.0)),
    );
    y += 56.0 + 14.0;

    quality(app, ui, theme, &track, pos2(x + size / 2.0, y + 12.0));
}

/// The position bar: 3 px, 5 px in the accent with a knob on hover. Returns
/// the position to show, which follows the pointer while dragging.
fn seek_bar(app: &mut App, ui: &mut Ui, theme: &Theme, hit: Rect) -> Duration {
    let total = app.duration().filter(|d| !d.is_zero());
    let response = ui.interact(hit, Id::unique("np-seek"), Sense::click_and_drag());
    let preview_id = Id::unique("np-seek-preview");
    let mut position = ui
        .ctx()
        .data(|d| d.get_temp::<Duration>(preview_id))
        .unwrap_or(app.status.position);
    if let (Some(total), Some(pointer)) = (total, response.interact_pointer_pos()) {
        let at = total.mul_f32(((pointer.x - hit.left()) / hit.width()).clamp(0.0, 1.0));
        if response.dragged() {
            ui.ctx().data_mut(|d| d.insert_temp(preview_id, at));
            position = at;
        }
        if response.drag_stopped() || response.clicked() {
            app.seek(at);
            ui.ctx().data_mut(|d| d.remove::<Duration>(preview_id));
            position = at;
        }
    }
    let active = response.hovered() || response.dragged();
    let fraction = total.map_or(0.0, |t| {
        (position.as_secs_f32() / t.as_secs_f32()).clamp(0.0, 1.0)
    });
    let thickness = if active { 5.0 } else { 3.0 };
    let rail = Rect::from_center_size(hit.center(), vec2(hit.width(), thickness));
    ui.painter().rect_filled(rail, 3.0, theme.text_a(0.14));
    let fill = Rect::from_min_size(rail.min, vec2(rail.width() * fraction, thickness));
    ui.painter()
        .rect_filled(fill, 3.0, if active { theme.accent } else { theme.text });
    if active {
        ui.painter()
            .circle_filled(pos2(fill.right(), rail.center().y), 5.5, theme.text);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
    position
}

fn transport(app: &mut App, ui: &mut Ui, theme: &Theme, row: Rect) {
    let has_track = app.queue.current().is_some();
    let centers: Vec<f32> = (0..5)
        .map(|i| row.left() + 20.0 + (row.width() - 40.0) * i as f32 / 4.0)
        .collect();
    let y = row.center().y;
    let button = |ui: &mut Ui, cx: f32, size: f32| {
        let rect = Rect::from_center_size(pos2(cx, y), vec2(size, size));
        ui.new_child(egui::UiBuilder::new().max_rect(rect))
    };
    let shuffle_color = if app.queue.shuffle() {
        theme.accent
    } else {
        theme.subtext0
    };
    if icon_button(
        &mut button(ui, centers[0], 40.0),
        theme,
        icons::SHUFFLE,
        40.0,
        20.0,
        shuffle_color,
        true,
    )
    .on_hover_text("Shuffle")
    .clicked()
    {
        app.toggle_shuffle();
    }
    if icon_button(
        &mut button(ui, centers[1], 40.0),
        theme,
        icons::BACK_FILL,
        40.0,
        28.0,
        theme.text,
        has_track,
    )
    .clicked()
    {
        app.previous();
    }
    // The big play button: a round hover instead of the square one.
    let play_rect = Rect::from_center_size(pos2(centers[2], y), vec2(56.0, 56.0));
    let play = ui.interact(play_rect, Id::unique("np-play"), Sense::click());
    if play.hovered() {
        ui.painter()
            .circle_filled(play_rect.center(), 28.0, theme.hover());
    }
    let icon = if app.is_playing() {
        icons::PAUSE_FILL
    } else {
        icons::PLAY_FILL
    };
    paint_icon(ui, icon, play_rect.center(), 34.0, theme.text);
    if play
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        app.toggle_play();
    }
    if icon_button(
        &mut button(ui, centers[3], 40.0),
        theme,
        icons::FORWARD_FILL,
        40.0,
        28.0,
        theme.text,
        has_track,
    )
    .clicked()
    {
        app.next();
    }
    let (repeat_icon, repeat_color) = match app.queue.repeat() {
        Repeat::Off => (icons::REPEAT, theme.subtext0),
        Repeat::All => (icons::REPEAT, theme.accent),
        Repeat::One => (icons::REPEAT_ONE, theme.accent),
    };
    if icon_button(
        &mut button(ui, centers[4], 40.0),
        theme,
        repeat_icon,
        40.0,
        20.0,
        repeat_color,
        true,
    )
    .on_hover_text("Repeat")
    .clicked()
    {
        app.cycle_repeat();
    }
}

/// The quality chip with its status spelled out: source, dot, label · status.
/// It opens the signal path above it.
fn quality(
    app: &mut App,
    ui: &mut Ui,
    theme: &Theme,
    track: &music_core::model::Track,
    center: egui::Pos2,
) {
    let Some((label, lossless)) = quality_label(track) else {
        return;
    };
    let assessment = signal::assess(app);
    let status = if assessment.perfect() {
        "Bit-perfect".to_owned()
    } else {
        assessment.issues[0].clone()
    };
    let caption = format!("{label} · {status}");
    let color = if lossless {
        crate::color::mix(theme.accent, theme.text, 0.70)
    } else {
        theme.subtext0
    };
    let g = text(ui, &caption, font(11.0, Weight::SemiBold), color, 360.0);
    let width = 8.0 + 12.0 + 6.0 + 7.0 + 6.0 + g.size().x + 8.0;
    let rect = Rect::from_center_size(center, vec2(width, 24.0));
    let response = ui.interact(rect, Id::unique("np-quality"), Sense::click());
    signal::remember_anchor(ui, SignalAnchor::PlayerBar, rect);
    if response.hovered() {
        ui.painter().rect_filled(rect, 6.0, theme.hover());
    }
    let border = if lossless {
        alpha(theme.accent, 0.4)
    } else {
        theme.line()
    };
    ui.painter().rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    let mut cx = rect.left() + 8.0;
    let source = match track.id.provider {
        ProviderKind::AppleMusic => icons::AM,
        ProviderKind::SoundCloud => icons::CLOUD,
        ProviderKind::Navidrome => icons::SERVER,
    };
    paint_icon(ui, source, pos2(cx + 6.0, center.y), 12.0, theme.subtext0);
    cx += 12.0 + 6.0;
    let dot = if assessment.perfect() {
        theme.green
    } else {
        theme.yellow
    };
    ui.painter()
        .circle_filled(pos2(cx + 3.5, center.y), 3.5, dot);
    cx += 7.0 + 6.0;
    paint_text(ui, g, cx, center.y, egui::Align::Min, color);
    if response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        app.signal_open = match app.signal_open {
            Some(_) => None,
            None => Some(SignalAnchor::PlayerBar),
        };
    }
}

fn right_column(app: &mut App, ui: &mut Ui, theme: &Theme, column: Rect) {
    let labels = [
        (NpTab::PlayingNext, "Playing Next"),
        (NpTab::Lyrics, "Lyrics"),
    ];
    if let Some(tab) = drawer::tabs(ui, theme, column.min, &labels, app.np_tab) {
        app.np_tab = tab;
    }
    let body = Rect::from_min_max(
        pos2(column.left() - 8.0, column.top() + 32.0 + 10.0),
        column.max,
    );
    let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
    body_ui.set_clip_rect(body);
    match app.np_tab {
        NpTab::PlayingNext => drawer::queue(app, &mut body_ui, theme, "np-queue"),
        NpTab::Lyrics => {
            let center = body.center();
            let g = text(
                ui,
                "Lyrics are on their way",
                font(18.0, Weight::SemiBold),
                theme.text,
                body.width(),
            );
            paint_text(
                ui,
                g,
                center.x,
                center.y - 12.0,
                egui::Align::Center,
                theme.text,
            );
            let g = text(
                ui,
                "Synced lyrics from Apple Music come in the next step.",
                font(13.0, Weight::Regular),
                theme.subtext0,
                body.width(),
            );
            paint_text(
                ui,
                g,
                center.x,
                center.y + 14.0,
                egui::Align::Center,
                theme.subtext0,
            );
        }
    }
}
