//! Library → Songs: the title, the source filter, Play and Shuffle, and a
//! sortable track list that only lays out the rows on screen.

use egui::{Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use music_core::model::{ProviderKind, Track};

use crate::app::{App, Connection, SortKey};
use crate::color::{alpha, mix};
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::{
    artwork, explicit_badge, format_time, glow, paint_icon, paint_text, quality_label, quality_tag,
    text, thumb_url,
};

const ROW: f32 = 52.0;
const HEAD: f32 = 32.0;
const TABLE_TOP: f32 = 179.0;

/// Column x ranges inside a row, after the design's grid:
/// `36px 3fr 2fr 2fr 116px 52px 36px`, gap 14, padding 10.
struct Columns {
    number: (f32, f32),
    title: (f32, f32),
    artist: (f32, f32),
    album: (f32, f32),
    source: (f32, f32),
    duration: (f32, f32),
}

impl Columns {
    fn new(left: f32, width: f32) -> Self {
        let gap = 14.0;
        let fixed = 36.0 + 116.0 + 52.0 + 36.0 + gap * 6.0 + 20.0;
        let fr = ((width - fixed) / 7.0).max(0.0);
        let mut x = left + 10.0;
        let mut take = |w: f32| {
            let range = (x, x + w);
            x += w + gap;
            range
        };
        Self {
            number: take(36.0),
            title: take((3.0 * fr).max(200.0)),
            artist: take((2.0 * fr).max(120.0)),
            album: take((2.0 * fr).max(120.0)),
            source: take(116.0),
            duration: take(52.0),
        }
    }
}

pub fn show(app: &mut App, ui: &mut Ui, pad: f32) -> f32 {
    rebuild_order(app);
    let theme = app.theme.clone();
    let rows = app.songs_view.order.len();
    let foot = 18.0 + 16.0 + 18.0;
    let total = TABLE_TOP + HEAD + 5.0 + rows as f32 * ROW + foot + 104.0;

    let output = egui::ScrollArea::vertical()
        .id_salt("songs")
        .auto_shrink(false)
        .show_viewport(ui, |ui, viewport| {
            let (content, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
            glow(
                ui,
                &theme,
                Rect::from_min_size(content.min, vec2(content.width(), 460.0)),
            );
            let left = content.left() + pad;
            let width = content.width() - 2.0 * pad;
            header(app, ui, &theme, content.top(), left, width);

            let columns = Columns::new(left, width);
            let table_top = content.top() + TABLE_TOP;
            table_head(app, ui, &theme, &columns, table_top, left, width);
            let rows_top = table_top + HEAD + 5.0;

            if rows == 0 {
                empty(app, ui, &theme, rows_top, left, width);
                return;
            }
            let first = ((viewport.top() - (TABLE_TOP + HEAD + 5.0)) / ROW)
                .floor()
                .max(0.0) as usize;
            let last = (((viewport.bottom() - (TABLE_TOP + HEAD + 5.0)) / ROW)
                .ceil()
                .max(0.0) as usize)
                .min(rows);
            for position in first..last {
                let top = rows_top + position as f32 * ROW;
                let rect = Rect::from_min_size(pos2(left, top), vec2(width, ROW));
                row(app, ui, &theme, &columns, rect, position);
            }
            let foot_text = if app.songs_done {
                format!("{rows} songs")
            } else {
                format!("{rows} songs so far, loading…")
            };
            let galley = text(
                ui,
                &foot_text,
                font(12.0, Weight::Regular),
                theme.muted(),
                width,
            );
            paint_text(
                ui,
                galley,
                left + 10.0,
                rows_top + rows as f32 * ROW + 26.0,
                egui::Align::Min,
                theme.muted(),
            );
        });
    keyboard(app, ui);
    output.state.clamped_offset().y
}

fn header(app: &mut App, ui: &mut Ui, theme: &Theme, top: f32, left: f32, width: f32) {
    let title = text(ui, "Songs", font(30.0, Weight::Bold), theme.text, width);
    paint_text(
        ui,
        title,
        left,
        top + 70.0 + 16.5,
        egui::Align::Min,
        theme.text,
    );

    // Source filter, right-aligned.
    let chips: [(Option<ProviderKind>, Option<Icon>, &str); 4] = [
        (None, None, "All"),
        (
            Some(ProviderKind::AppleMusic),
            Some(icons::AM),
            "Apple Music",
        ),
        (
            Some(ProviderKind::SoundCloud),
            Some(icons::CLOUD),
            "SoundCloud",
        ),
        (
            Some(ProviderKind::Navidrome),
            Some(icons::SERVER),
            "Navidrome",
        ),
    ];
    let font_chip = font(13.0, Weight::Regular);
    let widths: Vec<f32> = chips
        .iter()
        .map(|(_, icon, label)| {
            text(ui, label, font_chip.clone(), theme.text, 300.0)
                .size()
                .x
                + 24.0
                + if icon.is_some() { 19.0 } else { 0.0 }
        })
        .collect();
    let mut x = left + width - widths.iter().sum::<f32>() - 6.0 * (chips.len() - 1) as f32;
    for ((source, icon, label), w) in chips.into_iter().zip(widths) {
        let rect = Rect::from_min_size(pos2(x, top + 73.0), vec2(w, 28.0));
        x += w + 6.0;
        let response = ui.interact(rect, Id::unique(("source-filter", label)), Sense::click());
        let on = app.songs_view.source == source;
        let (fill, fg, border) = if on {
            (Some(theme.text), theme.base, theme.text)
        } else if response.hovered() {
            (Some(theme.hover()), theme.text, theme.line())
        } else {
            (None, theme.subtext0, theme.line())
        };
        if let Some(fill) = fill {
            ui.painter().rect_filled(rect, 14.0, fill);
        }
        ui.painter().rect_stroke(
            rect,
            14.0,
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        let mut cx = rect.left() + 12.0;
        if let Some(icon) = icon {
            let icon_color = if on { theme.base } else { theme.muted() };
            paint_icon(ui, icon, pos2(cx + 6.5, rect.center().y), 13.0, icon_color);
            cx += 19.0;
        }
        let galley = text(ui, label, font_chip.clone(), fg, 300.0);
        paint_text(ui, galley, cx, rect.center().y, egui::Align::Min, fg);
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            app.songs_view.source = source;
            app.songs_view.selected = None;
        }
    }

    // Play and Shuffle.
    let play = Rect::from_min_size(pos2(left, top + 125.0), vec2(88.0, 36.0));
    let shuffle = Rect::from_min_size(pos2(play.right() + 10.0, top + 125.0), vec2(107.0, 36.0));
    let has_songs = !app.songs_view.order.is_empty();
    if button(ui, theme, play, icons::PLAY_FILL, "Play", true) && has_songs {
        if app.queue.shuffle() {
            app.toggle_shuffle();
        }
        app.play_list(shown_tracks(app), 0);
    }
    if button(ui, theme, shuffle, icons::SHUFFLE, "Shuffle", false) && has_songs {
        let tracks = shown_tracks(app);
        let start = (ui.input(|i| i.time) * 1000.0) as usize % tracks.len();
        if !app.queue.shuffle() {
            app.queue.set_shuffle(true);
        }
        app.play_list(tracks, start);
    }
}

/// A 36 px button: accent-filled for the primary action, else a faint fill.
fn button(ui: &mut Ui, theme: &Theme, rect: Rect, icon: Icon, label: &str, primary: bool) -> bool {
    let response = ui.interact(rect, Id::unique(("page-button", label)), Sense::click());
    let (fill, fg) = if primary {
        let fill = if response.hovered() {
            mix(theme.accent, theme.text, 0.88)
        } else {
            theme.accent
        };
        (fill, theme.on_accent())
    } else {
        let fill = theme.text_a(if response.hovered() { 0.14 } else { 0.09 });
        (fill, theme.text)
    };
    let rect = if response.is_pointer_button_down_on() {
        rect.shrink(0.4)
    } else {
        rect
    };
    ui.painter().rect_filled(rect, 9.0, fill);
    paint_icon(
        ui,
        icon,
        pos2(rect.left() + 20.0 + 7.0, rect.center().y),
        14.0,
        fg,
    );
    let galley = text(ui, label, font(13.0, Weight::Medium), fg, rect.width());
    paint_text(
        ui,
        galley,
        rect.left() + 20.0 + 14.0 + 8.0,
        rect.center().y,
        egui::Align::Min,
        fg,
    );
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn table_head(
    app: &mut App,
    ui: &mut Ui,
    theme: &Theme,
    columns: &Columns,
    top: f32,
    left: f32,
    width: f32,
) {
    let y = top + HEAD / 2.0;
    let f = font(11.5, Weight::Medium);
    let heads = [
        (columns.title, "Title", Some(SortKey::Title)),
        (columns.artist, "Artist", Some(SortKey::Artist)),
        (columns.album, "Album", Some(SortKey::Album)),
        (columns.source, "Source", None),
    ];
    for ((x0, x1), label, key) in heads {
        let galley = text(ui, label, f.clone(), theme.muted(), x1 - x0);
        let rect = Rect::from_min_size(pos2(x0, y - 8.0), vec2(galley.size().x + 12.0, 16.0));
        let mut color = theme.muted();
        if let Some(key) = key {
            let response = ui.interact(rect, Id::unique(("sort", label)), Sense::click());
            if response.hovered() {
                color = theme.text;
            }
            if response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                cycle_sort(app, key);
            }
            if let Some((sorted, ascending)) = app.songs_view.sort
                && sorted == key
            {
                let arrow = if ascending { "▲" } else { "▼" };
                let g = text(ui, arrow, font(8.0, Weight::Regular), color, 20.0);
                paint_text(
                    ui,
                    g,
                    x0 + galley.size().x + 4.0,
                    y,
                    egui::Align::Min,
                    color,
                );
            }
        }
        paint_text(ui, galley, x0, y, egui::Align::Min, color);
    }
    // The duration column is headed by a clock and sorts by length.
    let (_, x1) = columns.duration;
    let clock = Rect::from_center_size(pos2(x1 - 8.0, y), vec2(16.0, 16.0));
    let response = ui.interact(
        clock.expand(4.0),
        Id::unique(("sort", "duration")),
        Sense::click(),
    );
    let color = if response.hovered() {
        theme.text
    } else {
        theme.muted()
    };
    paint_icon(ui, icons::CLOCK, clock.center(), 13.0, color);
    if response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        cycle_sort(app, SortKey::Duration);
    }
    ui.painter().hline(
        left..=left + width,
        top + HEAD - 0.5,
        Stroke::new(1.0, theme.line()),
    );
}

fn cycle_sort(app: &mut App, key: SortKey) {
    app.songs_view.sort = match app.songs_view.sort {
        Some((k, true)) if k == key => Some((key, false)),
        Some((k, false)) if k == key => None,
        _ => Some((key, true)),
    };
    app.songs_view.selected = None;
}

fn row(app: &mut App, ui: &mut Ui, theme: &Theme, columns: &Columns, rect: Rect, position: usize) {
    let index = app.songs_view.order[position];
    let response = ui.interact(rect, Id::unique(("song", index)), Sense::click());
    let track = &app.songs[index];
    let now = app.now.as_ref().is_some_and(|n| n.id == track.id) && app.status.token.is_some();
    let selected = app.songs_view.selected == Some(position);

    let fill = if selected {
        Some(alpha(theme.accent, 0.16))
    } else if response.contains_pointer() {
        Some(theme.hover())
    } else if position % 2 == 1 {
        Some(theme.text_a(0.025))
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, 8.0, fill);
    }
    let y = rect.center().y;

    // Number, or a play button on hover, or bars for the playing track.
    let (n0, n1) = columns.number;
    let n_center = pos2((n0 + n1) / 2.0, y);
    let mut play_clicked = false;
    // `contains_pointer`, not `hovered`: over the play button the row is no
    // longer the hovered widget, and the button must not vanish under the click.
    if response.contains_pointer() {
        let button = Rect::from_center_size(n_center, vec2(28.0, 28.0));
        let hit = ui.interact(button, Id::unique(("song-play", index)), Sense::click());
        if hit.hovered() {
            ui.painter().circle_filled(n_center, 14.0, theme.hover());
        }
        paint_icon(ui, icons::PLAY_FILL, n_center, 13.0, theme.text);
        play_clicked = hit.clicked();
    } else if now {
        bars(ui, theme, n_center, app.is_playing());
    } else {
        let g = text(
            ui,
            &(position + 1).to_string(),
            font(13.0, Weight::Regular),
            theme.muted(),
            36.0,
        );
        paint_text(ui, g, n_center.x, y, egui::Align::Center, theme.muted());
    }

    // Artwork, title and the E badge.
    let (t0, t1) = columns.title;
    let art = Rect::from_min_size(pos2(t0, y - 18.0), vec2(36.0, 36.0));
    artwork(ui, thumb_url(track), art, 5.0, theme);
    let tx = art.right() + 12.0;
    let title_color = if now { theme.accent } else { theme.text };
    let max = t1 - tx - if track.explicit { 18.0 } else { 0.0 };
    let title = text(
        ui,
        &track.title,
        font(13.0, Weight::Medium),
        title_color,
        max,
    );
    let title_end = tx + title.size().x;
    paint_text(ui, title, tx, y, egui::Align::Min, title_color);
    if track.explicit {
        explicit_badge(ui, theme, pos2(title_end + 6.0 + 6.0, y));
    }

    // Artist and album.
    for ((c0, c1), value) in [
        (columns.artist, track.artists.join(", ")),
        (columns.album, track.album.clone().unwrap_or_default()),
    ] {
        let g = text(
            ui,
            &value,
            font(13.0, Weight::Regular),
            theme.subtext0,
            c1 - c0,
        );
        paint_text(ui, g, c0, y, egui::Align::Min, theme.subtext0);
    }

    // Source icon and quality tag.
    let (s0, _) = columns.source;
    let icon = match track.id.provider {
        ProviderKind::AppleMusic => icons::AM,
        ProviderKind::SoundCloud => icons::CLOUD,
        ProviderKind::Navidrome => icons::SERVER,
    };
    paint_icon(ui, icon, pos2(s0 + 6.5, y), 13.0, theme.muted());
    if let Some((label, lossless)) = quality_label(track) {
        quality_tag(ui, theme, &label, lossless, pos2(s0 + 13.0 + 6.0, y));
    }

    // Duration.
    let (_, d1) = columns.duration;
    if let Some(duration) = track.duration {
        let g = text(
            ui,
            &format_time(duration),
            font(13.0, Weight::Regular),
            theme.muted(),
            52.0,
        );
        paint_text(ui, g, d1, y, egui::Align::Max, theme.muted());
    }

    if response.clicked() {
        app.songs_view.selected = Some(position);
    }
    if response.double_clicked() || play_clicked {
        app.songs_view.selected = Some(position);
        let tracks = shown_tracks(app);
        app.play_list(tracks, position);
    }
}

/// Three accent bars for the playing row, moving while it plays.
fn bars(ui: &Ui, theme: &Theme, center: egui::Pos2, moving: bool) {
    let t = ui.input(|i| i.time) as f32;
    for (k, phase) in [0.0_f32, 1.7, 3.1].into_iter().enumerate() {
        let h = if moving {
            4.0 + 8.0 * (0.5 + 0.5 * (t * 6.0 + phase).sin())
        } else {
            4.0
        };
        let x = center.x - 5.0 + k as f32 * 5.0;
        let rect = Rect::from_min_max(
            pos2(x - 1.5, center.y + 6.0 - h),
            pos2(x + 1.5, center.y + 6.0),
        );
        ui.painter().rect_filled(rect, 1.0, theme.accent);
    }
    if moving {
        ui.ctx().request_repaint();
    }
}

fn empty(app: &App, ui: &Ui, theme: &Theme, top: f32, left: f32, width: f32) {
    let (title, detail) = match &app.connection {
        Connection::Connecting => ("Loading your library…", String::new()),
        Connection::NotSignedIn => (
            "Apple Music isn't connected",
            "Sign in for now with `cargo run -p apple-music --example am -- signin`; the in-app sign-in comes in a later step.".to_owned(),
        ),
        Connection::Failed(e) => ("Couldn't reach Apple Music", e.clone()),
        Connection::Connected if !app.songs_done => ("Loading your library…", String::new()),
        Connection::Connected => ("No songs here", "Nothing matches this source filter.".to_owned()),
    };
    if title.starts_with("Loading") {
        for i in 0..8 {
            let rect =
                Rect::from_min_size(pos2(left, top + i as f32 * (ROW + 2.0)), vec2(width, ROW));
            ui.painter().rect_filled(rect, 8.0, theme.hover());
        }
        return;
    }
    let g = text(
        ui,
        title,
        font(15.0, Weight::SemiBold),
        theme.text,
        width.min(460.0),
    );
    paint_text(ui, g, left + 10.0, top + 50.0, egui::Align::Min, theme.text);
    let g = text(
        ui,
        &detail,
        font(13.0, Weight::Regular),
        theme.subtext0,
        width - 20.0,
    );
    paint_text(
        ui,
        g,
        left + 10.0,
        top + 74.0,
        egui::Align::Min,
        theme.subtext0,
    );
}

fn keyboard(app: &mut App, ui: &Ui) {
    if ui.ctx().egui_wants_keyboard_input() {
        return;
    }
    let rows = app.songs_view.order.len();
    if rows == 0 {
        return;
    }
    let (down, up, enter) = ui.input(|i| {
        (
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::Enter),
        )
    });
    let selected = &mut app.songs_view.selected;
    if down {
        *selected = Some(selected.map_or(0, |s| (s + 1).min(rows - 1)));
    }
    if up {
        *selected = Some(selected.map_or(0, |s| s.saturating_sub(1)));
    }
    if enter && let Some(position) = app.songs_view.selected {
        let tracks = shown_tracks(app);
        app.play_list(tracks, position);
    }
}

/// The tracks as listed, to become the queue.
fn shown_tracks(app: &App) -> Vec<Track> {
    app.songs_view
        .order
        .iter()
        .map(|&i| app.songs[i].clone())
        .collect()
}

fn rebuild_order(app: &mut App) {
    let view = &mut app.songs_view;
    let key = Some((app.songs.len(), view.sort, view.source));
    if view.order_key == key {
        return;
    }
    view.order_key = key;
    let songs = &app.songs;
    let mut order: Vec<usize> = (0..songs.len())
        .filter(|&i| view.source.is_none_or(|s| songs[i].id.provider == s))
        .collect();
    if let Some((by, ascending)) = view.sort {
        let text_cmp = |a: &str, b: &str| a.to_lowercase().cmp(&b.to_lowercase());
        order.sort_by(|&a, &b| {
            let (a, b) = (&songs[a], &songs[b]);
            let ord = match by {
                SortKey::Title => text_cmp(&a.title, &b.title),
                SortKey::Artist => text_cmp(&a.artists.join(", "), &b.artists.join(", ")),
                SortKey::Album => text_cmp(
                    a.album.as_deref().unwrap_or(""),
                    b.album.as_deref().unwrap_or(""),
                ),
                SortKey::Duration => a.duration.cmp(&b.duration),
            };
            if ascending { ord } else { ord.reverse() }
        });
    }
    if order.len() != view.order.len() {
        view.selected = None;
    }
    view.order = order;
}
