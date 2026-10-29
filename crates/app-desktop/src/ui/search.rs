//! Search: the query as the title with the source filter, Apple Music or the
//! library to search, the top result beside the first songs, then shelves of
//! albums and playlists.

use std::sync::Arc;

use egui::{Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use music_core::model::{Artwork, ItemId, ProviderKind, SearchResults, Track};

use crate::app::{App, Loaded, Page, SearchScope};
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::songs::source_chips;
use super::tracks::{self, Columns, RowAction, RowState};
use super::{artwork, glow, icon_button, paint_text, text};

/// Where the sections start, under the title and the scope switch.
const BODY_TOP: f32 = 172.0;
const SECTION_GAP: f32 = 34.0;
/// A section's title and the space under it.
const SECTION_HEAD: f32 = 34.0;
const TOP_RESULT_LEFT: f32 = 380.0;
const TOP_RESULT_HEIGHT: f32 = 156.0;
const COLUMN_GAP: f32 = 28.0;
/// The song rows need this much beside the top result; narrower, they go under it.
const SONGS_MIN: f32 = 650.0;
const SONGS_SHOWN: usize = 6;
const CARD: f32 = 188.0;
const CARD_GAP: f32 = 18.0;
const CARD_HEIGHT: f32 = CARD + 8.0 + 18.0 + 16.0;

/// The best match, shown large.
enum Top {
    Artist(String, Option<Artwork>),
    Album(ItemId, String, String, Option<Artwork>),
    Playlist(usize),
}

/// A cover in a shelf.
struct Card {
    title: String,
    sub: String,
    art: Option<Artwork>,
    open: Open,
}

#[derive(Clone)]
enum Open {
    Album(ItemId),
    Playlist(usize),
}

pub fn show(app: &mut App, ui: &mut Ui, pad: f32, query: &str) -> f32 {
    let theme = app.theme.clone();
    let loaded = match app.search_results(query) {
        Some(Loaded::Ready(results)) => Ok(Some(filtered(results, app.search_source))),
        Some(Loaded::Failed(e)) => Err(e.clone()),
        _ => Ok(None),
    };
    let width = ui.available_width() - 2.0 * pad;
    let side_by_side = width >= TOP_RESULT_LEFT + COLUMN_GAP + SONGS_MIN;

    // Lay the sections out first: the scroll area needs the height.
    let (top, songs, albums, playlists) = match &loaded {
        Ok(Some(r)) => (
            top_result(r),
            r.tracks
                .iter()
                .take(SONGS_SHOWN)
                .cloned()
                .collect::<Vec<_>>(),
            album_cards(r),
            playlist_cards(r),
        ),
        _ => (None, Vec::new(), Vec::new(), Vec::new()),
    };
    let loading = matches!(loaded, Ok(None));
    let song_rows = if loading { SONGS_SHOWN } else { songs.len() };
    let top_height = if top.is_some() || loading {
        SECTION_HEAD + TOP_RESULT_HEIGHT
    } else {
        0.0
    };
    let songs_height = if song_rows > 0 {
        SECTION_HEAD + song_rows as f32 * tracks::ROW
    } else {
        0.0
    };
    let columns_height = if side_by_side {
        top_height.max(songs_height)
    } else if top_height > 0.0 && songs_height > 0.0 {
        top_height + SECTION_GAP + songs_height
    } else {
        top_height + songs_height
    };
    let shelves = [("Albums", &albums), ("Playlists", &playlists)];
    let shelf_count = shelves.iter().filter(|(_, c)| !c.is_empty()).count();
    let empty = !loading && top.is_none() && songs.is_empty() && shelf_count == 0;
    let total = BODY_TOP
        + if empty { 160.0 } else { columns_height }
        + shelf_count as f32 * (SECTION_GAP + SECTION_HEAD + CARD_HEIGHT)
        + 104.0
        + 40.0;

    let output = egui::ScrollArea::vertical()
        .id_salt(("search", query))
        .auto_shrink(false)
        .show_viewport(ui, |ui, _viewport| {
            let (content, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
            glow(
                ui,
                &theme,
                Rect::from_min_size(content.min, vec2(content.width(), 460.0)),
            );
            let left = content.left() + pad;
            let title = text(
                ui,
                &format!("“{query}”"),
                font(30.0, Weight::Bold),
                theme.text,
                (width - 420.0).max(120.0),
            );
            paint_text(
                ui,
                title,
                left,
                content.top() + 70.0 + 16.5,
                egui::Align::Min,
                theme.text,
            );
            if let Some(source) = source_chips(
                ui,
                &theme,
                left + width,
                content.top() + 73.0,
                app.search_source,
            ) {
                app.search_source = source;
                app.detail_selected = None;
            }

            let scopes = [
                (SearchScope::AppleMusic, "Apple Music"),
                (SearchScope::Library, "Library"),
            ];
            let at = pos2(left, content.top() + 118.0);
            if let Some(scope) = super::drawer::tabs(ui, &theme, at, &scopes, app.search_scope) {
                app.set_search_scope(scope);
                app.detail_selected = None;
            }

            let mut y = content.top() + BODY_TOP;
            if let Err(e) = &loaded {
                message(ui, &theme, left, y, width, "Couldn't search Apple Music", e);
                return;
            }
            if empty {
                let place = match app.search_scope {
                    SearchScope::AppleMusic => "on Apple Music",
                    SearchScope::Library => "in your library",
                };
                let detail = format!(
                    "Nothing {place} matches “{query}”. Check the spelling, or try fewer words."
                );
                message(ui, &theme, left, y, width, "No results", &detail);
                return;
            }

            // The top result, and the first songs beside it or under it.
            let (top_rect, songs_rect) = if side_by_side {
                (
                    Rect::from_min_size(pos2(left, y), vec2(TOP_RESULT_LEFT, top_height)),
                    Rect::from_min_size(
                        pos2(left + TOP_RESULT_LEFT + COLUMN_GAP, y),
                        vec2(width - TOP_RESULT_LEFT - COLUMN_GAP, songs_height),
                    ),
                )
            } else {
                let below = if top_height > 0.0 {
                    top_height + SECTION_GAP
                } else {
                    0.0
                };
                (
                    Rect::from_min_size(
                        pos2(left, y),
                        vec2(TOP_RESULT_LEFT.min(width), top_height),
                    ),
                    Rect::from_min_size(pos2(left, y + below), vec2(width, songs_height)),
                )
            };
            if top_height > 0.0 {
                section_head(ui, &theme, top_rect.left(), top_rect.top(), "Top Result");
                let card = Rect::from_min_size(
                    pos2(top_rect.left(), top_rect.top() + SECTION_HEAD),
                    vec2(top_rect.width(), TOP_RESULT_HEIGHT),
                );
                match &top {
                    Some(top) => top_card(app, ui, &theme, card, top, query),
                    None => {
                        ui.painter().rect_filled(card, 12.0, theme.hover());
                    }
                }
            }
            if song_rows > 0 {
                section_head(ui, &theme, songs_rect.left(), songs_rect.top(), "Songs");
                let rows_top = songs_rect.top() + SECTION_HEAD;
                if loading {
                    tracks::skeleton(
                        ui,
                        &theme,
                        rows_top,
                        songs_rect.left(),
                        songs_rect.width(),
                        SONGS_SHOWN,
                    );
                } else {
                    song_list(app, ui, &theme, songs_rect, rows_top, &songs);
                }
            }
            y += columns_height;

            for (title, cards) in shelves {
                if cards.is_empty() {
                    continue;
                }
                y += SECTION_GAP;
                shelf(app, ui, &theme, pos2(left, y), width, title, cards, query);
                y += SECTION_HEAD + CARD_HEIGHT;
            }
        });
    output.state.clamped_offset().y
}

/// The results from the chosen source only.
fn filtered(results: &Arc<SearchResults>, source: Option<ProviderKind>) -> Arc<SearchResults> {
    let Some(source) = source else {
        return results.clone();
    };
    Arc::new(SearchResults {
        tracks: results
            .tracks
            .iter()
            .filter(|t| t.id.provider == source)
            .cloned()
            .collect(),
        albums: results
            .albums
            .iter()
            .filter(|a| a.id.provider == source)
            .cloned()
            .collect(),
        artists: results
            .artists
            .iter()
            .filter(|a| a.id.provider == source)
            .cloned()
            .collect(),
        playlists: results
            .playlists
            .iter()
            .filter(|p| p.id.provider == source)
            .cloned()
            .collect(),
    })
}

/// An artist first, else an album, else a playlist.
fn top_result(r: &SearchResults) -> Option<Top> {
    if let Some(a) = r.artists.first() {
        return Some(Top::Artist(a.name.clone(), a.artwork.clone()));
    }
    if let Some(a) = r.albums.first() {
        return Some(Top::Album(
            a.id.clone(),
            a.title.clone(),
            a.artists.join(", "),
            a.artwork.clone(),
        ));
    }
    (!r.playlists.is_empty()).then_some(Top::Playlist(0))
}

fn album_cards(r: &SearchResults) -> Vec<Card> {
    r.albums
        .iter()
        .map(|a| Card {
            title: a.title.clone(),
            sub: match a.year {
                Some(year) => format!("{} · {year}", a.artists.join(", ")),
                None => a.artists.join(", "),
            },
            art: a.artwork.clone(),
            open: Open::Album(a.id.clone()),
        })
        .collect()
}

fn playlist_cards(r: &SearchResults) -> Vec<Card> {
    r.playlists
        .iter()
        .enumerate()
        .map(|(i, p)| Card {
            title: p.name.clone(),
            sub: "Apple Music".into(),
            art: p.artwork.clone(),
            open: Open::Playlist(i),
        })
        .collect()
}

/// Open a result: its album, or a playlist of the current search.
fn open(app: &mut App, what: Open, query: &str) {
    match what {
        Open::Album(id) => app.go(Page::Album(id)),
        Open::Playlist(i) => {
            let playlist = match app.search_results(query) {
                Some(Loaded::Ready(r)) => filtered(r, app.search_source).playlists.get(i).cloned(),
                _ => None,
            };
            if let Some(playlist) = playlist {
                app.open_playlist(&playlist);
            }
        }
    }
}

fn section_head(ui: &Ui, theme: &Theme, left: f32, top: f32, label: &str) {
    let g = text(ui, label, font(18.0, Weight::SemiBold), theme.text, 400.0);
    paint_text(ui, g, left, top + 11.0, egui::Align::Min, theme.text);
}

fn message(ui: &Ui, theme: &Theme, left: f32, top: f32, width: f32, title: &str, detail: &str) {
    let g = text(ui, title, font(15.0, Weight::SemiBold), theme.text, width);
    paint_text(ui, g, left, top + 20.0, egui::Align::Min, theme.text);
    let g = text(
        ui,
        detail,
        font(13.0, Weight::Regular),
        theme.subtext0,
        width,
    );
    paint_text(ui, g, left, top + 44.0, egui::Align::Min, theme.subtext0);
}

/// The top result: large artwork (round for an artist), what it is, its name.
fn top_card(app: &mut App, ui: &mut Ui, theme: &Theme, card: Rect, top: &Top, query: &str) {
    let (kind, title, sub, art, round, target) = match top {
        Top::Artist(name, art) => ("ARTIST", name.clone(), None, art.clone(), true, None),
        Top::Album(id, title, by, art) => (
            "ALBUM",
            title.clone(),
            Some(by.clone()),
            art.clone(),
            false,
            Some(Open::Album(id.clone())),
        ),
        Top::Playlist(i) => {
            let Some(Loaded::Ready(r)) = app.search_results(query) else {
                return;
            };
            let Some(p) = filtered(r, app.search_source).playlists.get(*i).cloned() else {
                return;
            };
            (
                "PLAYLIST",
                p.name,
                None,
                p.artwork,
                false,
                Some(Open::Playlist(*i)),
            )
        }
    };
    // Artist pages come later: until then an artist is not a link.
    let response = ui.interact(card, Id::unique("search-top"), Sense::click());
    let hovered = target.is_some() && response.hovered();
    ui.painter()
        .rect_filled(card, 12.0, theme.text_a(if hovered { 0.08 } else { 0.05 }));
    let art_rect = Rect::from_min_size(card.min + vec2(18.0, 18.0), vec2(120.0, 120.0));
    artwork(
        ui,
        art.map(|a| a.url(240)),
        art_rect,
        if round { 60.0 } else { 8.0 },
        theme,
    );
    let x = art_rect.right() + 18.0;
    let room = card.right() - 18.0 - x;
    let cy = card.center().y;
    let block = if sub.is_some() { 66.0 } else { 46.0 };
    let mut y = cy - block / 2.0;
    let muted = theme.muted();
    let g = text(ui, kind, font(11.0, Weight::SemiBold), muted, room);
    paint_text(ui, g, x, y + 7.0, egui::Align::Min, muted);
    y += 18.0;
    let g = text(ui, &title, font(22.0, Weight::Bold), theme.text, room);
    paint_text(ui, g, x, y + 14.0, egui::Align::Min, theme.text);
    y += 30.0;
    if let Some(sub) = sub {
        let g = text(ui, &sub, font(13.0, Weight::Regular), muted, room);
        paint_text(ui, g, x, y + 9.0, egui::Align::Min, muted);
    }
    if let Some(target) = target
        && response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
    {
        open(app, target, query);
    }
}

fn song_list(app: &mut App, ui: &mut Ui, theme: &Theme, area: Rect, top: f32, songs: &[Track]) {
    let columns = Columns::new(area.left(), area.width(), false);
    for (position, track) in songs.iter().enumerate() {
        let rect = Rect::from_min_size(
            pos2(area.left(), top + position as f32 * tracks::ROW),
            vec2(area.width(), tracks::ROW),
        );
        let now = app.is_current(track);
        let state = RowState {
            number: Some(position as u32 + 1),
            selected: app.detail_selected == Some(position),
            now,
            playing: now && app.is_playing(),
            list: "search",
            position,
        };
        match tracks::row(ui, theme, &columns, rect, track, state) {
            RowAction::None => {}
            RowAction::Select => app.detail_selected = Some(position),
            RowAction::Play => {
                app.detail_selected = Some(position);
                app.play_list(songs.to_vec(), position);
            }
            RowAction::OpenAlbum(id) => app.go(Page::Album(id)),
        }
    }
}

/// Where a shelf is scrolled to, and where it is heading.
#[derive(Clone, Copy, Default)]
struct ShelfScroll {
    offset: f32,
    target: f32,
}

/// A row of covers that scrolls sideways: with the arrows by its title, or
/// sideways scrolling over it.
#[allow(clippy::too_many_arguments)]
fn shelf(
    app: &mut App,
    ui: &mut Ui,
    theme: &Theme,
    at: egui::Pos2,
    width: f32,
    title: &str,
    cards: &[Card],
    query: &str,
) {
    section_head(ui, theme, at.x, at.y, title);
    let row = Rect::from_min_size(pos2(at.x, at.y + SECTION_HEAD), vec2(width, CARD_HEIGHT));
    let content = cards.len() as f32 * (CARD + CARD_GAP) - CARD_GAP;
    let max = (content - width).max(0.0);
    let id = Id::unique(("search-shelf", title, query));
    let mut scroll: ShelfScroll = ui.ctx().data(|d| d.get_temp(id)).unwrap_or_default();

    if max > 0.0 {
        // A page is as many whole covers as fit.
        let page = ((width + CARD_GAP) / (CARD + CARD_GAP)).floor().max(1.0) * (CARD + CARD_GAP);
        let arrows = [
            (icons::LEFT, -page, scroll.target > 0.0),
            (icons::RIGHT, page, scroll.target < max),
        ];
        for (i, (icon, step, enabled)) in arrows.into_iter().enumerate() {
            let rect = Rect::from_center_size(
                pos2(at.x + width - 14.0 - (1 - i) as f32 * 30.0, at.y + 11.0),
                vec2(28.0, 28.0),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
            if icon_button(&mut child, theme, icon, 28.0, 16.0, theme.subtext0, enabled).clicked() {
                scroll.target += step;
            }
        }
        if ui.rect_contains_pointer(row) {
            scroll.target -= ui.input(|i| i.smooth_scroll_delta.x);
        }
    }
    scroll.target = scroll.target.clamp(0.0, max);
    let dt = ui.input(|i| i.stable_dt).min(0.1);
    let step = scroll.target - scroll.offset;
    scroll.offset += step * (1.0 - (-dt * 12.0).exp());
    if step.abs() < 0.5 {
        scroll.offset = scroll.target;
    } else {
        ui.ctx().request_repaint();
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, scroll));

    let mut row_ui = ui.new_child(egui::UiBuilder::new().max_rect(row));
    row_ui.set_clip_rect(row.intersect(ui.clip_rect()));
    let muted = theme.muted();
    for (i, card) in cards.iter().enumerate() {
        let x = row.left() + i as f32 * (CARD + CARD_GAP) - scroll.offset;
        if x + CARD < row.left() || x > row.right() {
            continue;
        }
        let art = Rect::from_min_size(pos2(x, row.top()), vec2(CARD, CARD));
        let response = row_ui.interact(art, id.with(i), Sense::click());
        artwork(
            &row_ui,
            card.art.as_ref().map(|a| a.url(376)),
            art,
            8.0,
            theme,
        );
        if response.hovered() {
            row_ui
                .painter()
                .rect_filled(art, 8.0, egui::Color32::from_black_alpha(40));
        }
        row_ui.painter().rect_stroke(
            art,
            8.0,
            Stroke::new(1.0, theme.line()),
            egui::StrokeKind::Inside,
        );
        let g = text(
            &row_ui,
            &card.title,
            font(13.0, Weight::Medium),
            theme.text,
            CARD,
        );
        paint_text(
            &row_ui,
            g,
            x,
            art.bottom() + 8.0 + 9.0,
            egui::Align::Min,
            theme.text,
        );
        let g = text(&row_ui, &card.sub, font(12.0, Weight::Regular), muted, CARD);
        paint_text(
            &row_ui,
            g,
            x,
            art.bottom() + 8.0 + 18.0 + 8.0,
            egui::Align::Min,
            muted,
        );
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            open(app, card.open.clone(), query);
        }
    }
}
