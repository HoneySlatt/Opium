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
    // Spotify's layout shows the songs as its Liked Songs playlist.
    let banner = theme.layout.header == crate::theme::Header::Banner;
    // Spotify's Liked Songs: the last added first.
    if banner {
        app.want_newest_songs();
    }
    let table_offset = if banner {
        super::detail::BANNER + super::detail::ACTIONS
    } else {
        TABLE_TOP
    };
    let rows = app.songs_view.order.len();
    let foot = 18.0 + 16.0 + 18.0;
    let total = table_offset + tracks::HEAD_SPACE + rows as f32 * tracks::ROW + foot + 104.0;

    let output = egui::ScrollArea::vertical()
        .id_salt("songs")
        .auto_shrink(false)
        .show_viewport(ui, |ui, viewport| {
            let (content, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
            let left = content.left() + pad;
            let width = content.width() - 2.0 * pad;
            if banner {
                banner_header(app, ui, &theme, content, pad);
            } else {
                glow(
                    ui,
                    &theme,
                    Rect::from_min_size(content.min, vec2(content.width(), 460.0)),
                );
                header(app, ui, &theme, content.top(), left, width);
            }

            let columns = Columns::new(left, width, true);
            let table_top = content.top() + table_offset;
            let sort = Some(app.songs_view.sort);
            if let Some(key) = tracks::head(ui, &theme, &columns, table_top, left, width, sort) {
                app.songs_view.sort = tracks::cycle(app.songs_view.sort, key);
                app.songs_view.selected = None;
            }
            let rows_top = table_top + tracks::HEAD_SPACE;
            tracks::scroll_to_selected(ui, rows_top, left, width);

            if rows == 0 {
                empty(app, ui, &theme, rows_top, left, width);
                return;
            }
            // Carried out after the rows: it may take songs out of the list
            // they index.
            let mut chosen = None;
            for position in tracks::visible(viewport, table_offset + tracks::HEAD_SPACE, rows) {
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
                    RowAction::Menu(choice) => chosen = Some((track.clone(), choice)),
                }
            }
            if let Some((track, choice)) = chosen {
                app.track_choice(track, choice, ui.input(|i| i.time));
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
    let offset = output.state.clamped_offset().y;
    if banner {
        // Past the banner, Spotify's bar with its Play, and the column
        // titles kept under it, still sorting.
        let main = output.inner_rect;
        let bar = super::BANNER_BAR;
        let shown = ((offset - (super::detail::BANNER - 2.0 * bar)) / 48.0).clamp(0.0, 1.0);
        let here = app.page_playing();
        let (mut layer, play) = super::banner_bar(
            ui,
            &theme,
            main,
            super::detail::SONGS_BAND,
            "Songs",
            shown,
            here,
        );
        if play {
            app.play_page(shown_tracks(app), ui.input(|i| i.time));
        }
        if offset > table_offset - bar {
            let left = main.left() + pad;
            let width = main.width() - 2.0 * pad;
            let top = main.top() + bar;
            layer.painter().rect_filled(
                Rect::from_min_max(
                    pos2(left - 16.0, top),
                    pos2(left + width + 16.0, top + tracks::HEAD),
                ),
                0.0,
                crate::color::mix(theme.surface0, theme.base, 0.5),
            );
            let columns = Columns::new(left, width, true);
            let sort = Some(app.songs_view.sort);
            if let Some(key) = tracks::head(&mut layer, &theme, &columns, top, left, width, sort) {
                app.songs_view.sort = tracks::cycle(app.songs_view.sort, key);
                app.songs_view.selected = None;
            }
        }
    }
    offset
}

/// Spotify's banner over the songs, then on the right of its buttons the
/// search and, with more than one service, a menu of them.
fn banner_header(app: &mut App, ui: &mut Ui, theme: &Theme, content: Rect, pad: f32) {
    let count = app.songs_view.order.len();
    let (shuffle, here) = (app.shown().shuffle, app.page_playing());
    match super::detail::songs_banner(ui, theme, content, pad, count, shuffle, here) {
        Some(super::detail::SongsBanner::Play) => {
            app.play_page(shown_tracks(app), ui.input(|i| i.time));
        }
        Some(super::detail::SongsBanner::ToggleShuffle) => app.toggle_shuffle(),
        None => {}
    }
    let left = content.left() + pad;
    let width = content.width() - 2.0 * pad;
    let top = content.top() + super::detail::BANNER + (super::detail::ACTIONS - 32.0) / 2.0;
    let search = super::page_search(app, ui, theme, left + width, top, "Search Songs");
    if search.changed {
        app.songs_view.selected = None;
    }
    let services = app.services_in_use();
    if services.len() < 2 {
        return;
    }
    let label = match app.songs_view.source {
        None => "All services",
        Some(ProviderKind::AppleMusic) => "Apple Music",
        Some(ProviderKind::SoundCloud) => "SoundCloud",
        Some(ProviderKind::Navidrome) => "Navidrome",
        Some(ProviderKind::Music) => "Music",
    };
    let f = font(14.0, Weight::Regular);
    let w = text(ui, label, f.clone(), theme.text, 200.0).size().x + 8.0 + 14.0;
    let rect = Rect::from_min_size(pos2(search.left - 16.0 - w, top), vec2(w, 32.0));
    let response = ui
        .interact(rect, Id::unique("songs-services"), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let color = if response.hovered() {
        theme.text
    } else {
        theme.subtext0
    };
    let g = text(ui, label, f, color, 200.0);
    paint_text(ui, g, rect.left(), rect.center().y, egui::Align::Min, color);
    paint_icon(
        ui,
        icons::DOWN,
        pos2(rect.right() - 7.0, rect.center().y),
        14.0,
        color,
    );
    if let Some(source) =
        super::menu::sources_menu(&response, theme, app.songs_view.source, &services)
    {
        app.set_songs_source(source);
    }
}

fn header(app: &mut App, ui: &mut Ui, theme: &Theme, top: f32, left: f32, width: f32) {
    let title = text(ui, "Songs", font(30.0, Weight::Bold), theme.text, width);
    let title_rect = Rect::from_min_size(
        pos2(left, top + 70.0 + 16.5 - title.size().y / 2.0),
        title.size(),
    );
    paint_text(
        ui,
        title,
        left,
        top + 70.0 + 16.5,
        egui::Align::Min,
        theme.text,
    );
    let services = app.services_in_use();
    let (sources, _) = sources_shown(ui, theme, Id::unique("songs"), title_rect, &services);

    // The search, right-aligned, then the source filter left of it.
    let search = super::page_search(app, ui, theme, left + width, top + 71.0, "Search Songs");
    if search.changed {
        app.songs_view.selected = None;
    }
    if sources
        && let Some(source) = source_chips(
            ui,
            theme,
            search.left - 10.0,
            top + 73.0,
            app.songs_view.source,
            &services,
        )
    {
        app.set_songs_source(source);
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
    // Spotify's look lists the last added first, unless a column is sorted.
    let newest = (app.theme.layout.header == crate::theme::Header::Banner && view.sort.is_none())
        .then_some(app.newest_version);
    let key = Some((
        app.songs.len(),
        view.sort,
        view.source,
        query.clone(),
        newest,
    ));
    if view.order_key == key {
        return;
    }
    view.order_key = key;
    let songs = &app.songs;
    // Made again only when songs come or go, not at each key typed.
    if view
        .prepared
        .as_ref()
        .is_none_or(|p| p.count != songs.len())
    {
        view.prepared = Some(crate::app::PreparedSongs {
            count: songs.len(),
            copies: music_core::dedupe::songs(songs),
            folded: songs
                .iter()
                .map(|t| {
                    crate::app::fold(&format!(
                        "{} {} {}",
                        t.title,
                        t.artists.join(" "),
                        t.album.as_deref().unwrap_or_default()
                    ))
                })
                .collect(),
        });
    }
    let prepared = view.prepared.as_ref().expect("just made");
    // Every word searched for is in the title, the artists or the album.
    let words: Vec<&str> = query.split_whitespace().collect();
    let found = |i: usize| words.is_empty() || words.iter().all(|w| prepared.folded[i].contains(w));
    // Across services, one copy of a song; within one, all of its own.
    let mut order: Vec<usize> = (0..songs.len())
        .filter(|&i| {
            view.source.is_none_or(|s| songs[i].id.provider == s)
                && (view.source.is_some() || !prepared.copies.contains(&i))
                && found(i)
        })
        .collect();
    if let Some((by, ascending)) = view.sort {
        // Each song's key made once, not at each comparison.
        let text = |t: &Track| match by {
            SortKey::Title => t.title.to_lowercase(),
            SortKey::Artist => t.artists.join(", ").to_lowercase(),
            SortKey::Album => t.album.as_deref().unwrap_or("").to_lowercase(),
            SortKey::Duration => String::new(),
        };
        match by {
            SortKey::Duration => order.sort_by_key(|&i| songs[i].duration),
            _ => order.sort_by_cached_key(|&i| text(&songs[i])),
        }
        if !ascending {
            order.reverse();
        }
    } else if newest.is_some() {
        // Those not listed among the newest yet, after.
        order.sort_by_key(|&i| {
            app.newest
                .get(&songs[i].id)
                .copied()
                .unwrap_or((u8::MAX, 0))
        });
    }
    if order.len() != view.order.len() {
        view.selected = None;
    }
    view.order = order;
}

/// Whether page `id` shows its source filter: hidden until its title (`title`,
/// with a chevron after it) is clicked, as on the phone; a click folds it
/// again. With a single service of `services`, there is nothing to choose
/// between: no chevron. Returns that, and where the title or chevron ends.
pub(super) fn sources_shown(
    ui: &Ui,
    theme: &Theme,
    id: Id,
    title: Rect,
    services: &[ProviderKind],
) -> (bool, f32) {
    if services.len() < 2 {
        return (false, title.right());
    }
    let id = id.with("sources");
    let shown: bool = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(false);
    let end = title.right() + 30.0;
    let response = ui.interact(
        Rect::from_min_max(title.min, pos2(end, title.max.y)),
        id,
        Sense::click(),
    );
    let colour = if response.hovered() {
        theme.text
    } else {
        theme.muted()
    };
    let chevron = if shown { icons::DOWN } else { icons::RIGHT };
    paint_icon(
        ui,
        chevron,
        pos2(title.right() + 17.0, title.center().y),
        18.0,
        colour,
    );
    if response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        ui.ctx().data_mut(|d| d.insert_temp(id, !shown));
    }
    (shown, end)
}

/// The source filter chips (All · Apple Music · SoundCloud · Navidrome), the
/// services of `services` only, ending at `right`. Returns the source
/// clicked.
pub(super) fn source_chips(
    ui: &Ui,
    theme: &Theme,
    right: f32,
    top: f32,
    current: Option<ProviderKind>,
    services: &[ProviderKind],
) -> Option<Option<ProviderKind>> {
    let mut clicked = None;
    let mut chips: Vec<(Option<ProviderKind>, Option<Icon>, &str)> = vec![
        (None, None, "All"),
        (
            Some(ProviderKind::AppleMusic),
            Some(icons::APPLE_MUSIC),
            "Apple Music",
        ),
        (
            Some(ProviderKind::SoundCloud),
            Some(icons::SOUNDCLOUD),
            "SoundCloud",
        ),
        (
            Some(ProviderKind::Navidrome),
            Some(icons::NAVIDROME),
            "Navidrome",
        ),
    ];
    chips.retain(|(source, _, _)| source.is_none_or(|s| services.contains(&s)));
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
