//! Library → Songs: the title, the source filter, Play and Shuffle, and a
//! sortable track list that only lays out the rows on screen.

use egui::{Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use music_core::model::{FavouriteKind, ProviderKind, Track};

use crate::app::{App, Connection, Page, SortKey};
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::tracks::{self, Columns, RowAction, RowState};
use super::{glow, page_buttons, paint_icon, paint_text, text};

const TABLE_TOP: f32 = 179.0;

pub fn show(app: &mut App, ui: &mut Ui, pad: f32) -> f32 {
    rebuild_order(app);
    let theme = app.theme.clone();
    let rows = app.songs_view.order.len();
    let foot = 18.0 + 16.0 + 18.0;
    let total = TABLE_TOP + tracks::HEAD_SPACE + rows as f32 * tracks::ROW + foot + 104.0;

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

            let columns = Columns::new(left, width, true);
            let table_top = content.top() + TABLE_TOP;
            let sort = Some(app.songs_view.sort);
            if let Some(key) = tracks::head(ui, &theme, &columns, table_top, left, width, sort) {
                app.songs_view.sort = tracks::cycle(app.songs_view.sort, key);
                app.songs_view.selected = None;
            }
            let rows_top = table_top + tracks::HEAD_SPACE;

            if rows == 0 {
                empty(app, ui, &theme, rows_top, left, width);
                return;
            }
            for position in tracks::visible(viewport, TABLE_TOP + tracks::HEAD_SPACE, rows) {
                let top = rows_top + position as f32 * tracks::ROW;
                let rect = Rect::from_min_size(pos2(left, top), vec2(width, tracks::ROW));
                let track = &app.songs[app.songs_view.order[position]];
                let now = app.is_current(track);
                let state = RowState {
                    number: Some(position as u32 + 1),
                    selected: app.songs_view.selected == Some(position),
                    now,
                    playing: now && app.is_playing(),
                    favourite: app.is_favourite(FavouriteKind::Song, &track.id),
                    in_library: app.track_in_library(track),
                    in_playlist: false,
                    list: "songs",
                    position,
                };
                match tracks::row(ui, &theme, &columns, rect, track, state) {
                    RowAction::None => {}
                    RowAction::Select => app.songs_view.selected = Some(position),
                    RowAction::Play => {
                        app.songs_view.selected = Some(position);
                        app.play_list(shown_tracks(app), position);
                    }
                    RowAction::OpenAlbum(id) => app.go(Page::Album(id)),
                    RowAction::Menu(choice) => {
                        let track = track.clone();
                        app.track_choice(track, choice, ui.input(|i| i.time));
                    }
                }
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
                rows_top + rows as f32 * tracks::ROW + 26.0,
                egui::Align::Min,
                theme.muted(),
            );
        });
    if let Some(position) = tracks::keyboard(ui, rows, &mut app.songs_view.selected) {
        app.play_list(shown_tracks(app), position);
    }
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

    // The search, right-aligned, then the source filter left of it.
    let search = super::page_search(app, ui, theme, left + width, top + 71.0, "Search Songs");
    if search.changed {
        app.songs_view.selected = None;
    }
    if let Some(source) = source_chips(
        ui,
        theme,
        search.left - 10.0,
        top + 73.0,
        app.songs_view.source,
    ) {
        app.songs_view.source = source;
        app.songs_view.selected = None;
    }

    // Play and Shuffle.
    if !app.songs_view.order.is_empty()
        && let Some(button) = page_buttons(ui, theme, pos2(left, top + 125.0), "songs", false)
    {
        let shuffle = button == super::PageButton::Shuffle;
        app.play_shuffled_or_not(shown_tracks(app), shuffle, ui.input(|i| i.time));
    }
}

fn empty(app: &App, ui: &Ui, theme: &Theme, top: f32, left: f32, width: f32) {
    let (title, detail) = match &app.connection {
        Connection::Connecting => ("Loading your library…", String::new()),
        Connection::NotSignedIn => (
            "Apple Music isn't connected",
            "Sign in from Settings, under your name at the bottom of the sidebar.".to_owned(),
        ),
        Connection::Failed(e) => ("Couldn't reach Apple Music", e.clone()),
        Connection::Connected if !app.songs_done => ("Loading your library…", String::new()),
        Connection::Connected if !app.page_search.trim().is_empty() => (
            "No songs found",
            format!("None match “{}”.", app.page_search.trim()),
        ),
        Connection::Connected => (
            "No songs here",
            "Nothing matches this source filter.".to_owned(),
        ),
    };
    if title.starts_with("Loading") {
        tracks::skeleton(ui, theme, top, left, width, 8);
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
    let query = crate::app::fold(app.page_search.trim());
    let key = Some((app.songs.len(), view.sort, view.source, query.clone()));
    if view.order_key == key {
        return;
    }
    view.order_key = key;
    let songs = &app.songs;
    // Every word searched for is in the title, the artists or the album.
    let words: Vec<&str> = query.split_whitespace().collect();
    let found = |t: &Track| {
        words.is_empty() || {
            let text = crate::app::fold(&format!(
                "{} {} {}",
                t.title,
                t.artists.join(" "),
                t.album.as_deref().unwrap_or_default()
            ));
            words.iter().all(|w| text.contains(w))
        }
    };
    let mut order: Vec<usize> = (0..songs.len())
        .filter(|&i| view.source.is_none_or(|s| songs[i].id.provider == s) && found(&songs[i]))
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

/// The source filter chips (All · Apple Music · SoundCloud · Navidrome), ending
/// at `right`. Returns the source clicked.
pub(super) fn source_chips(
    ui: &Ui,
    theme: &Theme,
    right: f32,
    top: f32,
    current: Option<ProviderKind>,
) -> Option<Option<ProviderKind>> {
    let mut clicked = None;
    let chips: [(Option<ProviderKind>, Option<Icon>, &str); 4] = [
        (None, None, "All"),
        (
            Some(ProviderKind::AppleMusic),
            Some(icons::APPLE_MUSIC),
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
    let mut x = right - widths.iter().sum::<f32>() - 6.0 * (chips.len() - 1) as f32;
    for ((source, icon, label), w) in chips.into_iter().zip(widths) {
        let rect = Rect::from_min_size(pos2(x, top), vec2(w, 28.0));
        x += w + 6.0;
        let response = ui.interact(rect, Id::unique(("source-filter", label)), Sense::click());
        let on = current == source;
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
            clicked = Some(source);
        }
    }
    clicked
}
