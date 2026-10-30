//! Search: the query as the title with the source filter, Apple Music or the
//! library to search, the top result beside the first songs, then shelves of
//! albums and playlists.

use egui::{Id, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{Artwork, Collection, FavouriteKind, ItemId, SearchResults, Track};

use crate::app::{App, Loaded, Page, SearchScope};
use crate::fonts::{Weight, font};
use crate::theme::Theme;

use super::grid::{CardClick, CoverCard, Grid, ShelfCard, cover_card, cover_shelf, round_shelf};
use super::songs::{source_chips, sources_shown};
use super::tracks::{self, Columns, RowAction, RowState};
use super::{artwork, glow, paint_icon, paint_text, text};
use crate::icons;

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
const CARD_HEIGHT: f32 = CARD + super::grid::CARD_TEXT;

/// The best match, shown large.
enum Top {
    Artist(ItemId, String, Option<Artwork>),
    Album(ItemId, String, String, Option<Artwork>),
    Playlist(usize),
}

/// A cover in a shelf.
struct Card {
    title: String,
    sub: String,
    art: Option<Artwork>,
    open: Open,
    explicit: bool,
}

#[derive(Clone)]
enum Open {
    Album(ItemId),
    Artist(ItemId),
    Playlist(usize),
}

pub fn show(app: &mut App, ui: &mut Ui, pad: f32, query: &str) -> f32 {
    let theme = app.theme.clone();
    let loaded = match app.search_results(query) {
        Some(Loaded::Ready(results)) => Ok(Some(results.clone())),
        Some(Loaded::Failed(e)) => Err(e.clone()),
        _ => Ok(None),
    };
    let width = ui.available_width() - 2.0 * pad;
    let side_by_side = width >= TOP_RESULT_LEFT + COLUMN_GAP + SONGS_MIN;
    let grid = Grid::new(width);
    let unfolded = |ui: &Ui, name: &str| {
        let id = unfold_id(app, query, name);
        (
            id,
            ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(false),
        )
    };
    let (songs_id, all_songs) = unfolded(ui, "Songs");

    // Lay the sections out first: the scroll area needs the height.
    let (top, all_tracks, sections) = match &loaded {
        Ok(Some(r)) => (top_result(r), r.tracks.clone(), sections(r)),
        _ => (None, Vec::new(), Vec::new()),
    };
    let songs: Vec<Track> = if all_songs {
        all_tracks.clone()
    } else {
        all_tracks.iter().take(SONGS_SHOWN).cloned().collect()
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
    // Each section of covers: a shelf, or all of them in a grid once its
    // title is clicked.
    let sections: Vec<(Section, Id, bool)> = sections
        .into_iter()
        .filter(|s| !s.cards.is_empty())
        .map(|s| {
            let (id, open) = unfolded(ui, s.title);
            (s, id, open)
        })
        .collect();
    let section_height = |s: &Section, open: bool| {
        SECTION_GAP
            + SECTION_HEAD
            + if open {
                grid.height(s.cards.len())
            } else {
                CARD_HEIGHT
            }
    };
    let empty = !loading && top.is_none() && songs.is_empty() && sections.is_empty();
    let total = BODY_TOP
        + if empty { 160.0 } else { columns_height }
        + sections
            .iter()
            .map(|(s, _, open)| section_height(s, *open))
            .sum::<f32>()
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
            let title_rect = Rect::from_min_size(
                pos2(left, content.top() + 70.0 + 16.5 - title.size().y / 2.0),
                title.size(),
            );
            paint_text(
                ui,
                title,
                left,
                content.top() + 70.0 + 16.5,
                egui::Align::Min,
                theme.text,
            );
            let chosen = app.search_source.is_some();
            let (sources, _) = sources_shown(ui, &theme, Id::unique("search"), title_rect, chosen);
            if sources
                && let Some(source) = source_chips(
                    ui,
                    &theme,
                    left + width,
                    content.top() + 73.0,
                    app.search_source,
                )
            {
                app.set_search_source(source);
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
                message(ui, &theme, left, y, width, "Couldn't search", e);
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
                // The first six; the title shows the others, when there are.
                if all_tracks.len() > SONGS_SHOWN {
                    let at = songs_rect.left_top();
                    if fold_head(ui, &theme, at, "Songs", all_songs, songs_id) {
                        ui.ctx().data_mut(|d| d.insert_temp(songs_id, !all_songs));
                    }
                } else {
                    section_head(ui, &theme, songs_rect.left(), songs_rect.top(), "Songs");
                }
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

            for (section, id, open) in &sections {
                y += SECTION_GAP;
                let at = pos2(left, y);
                let clicked = if *open {
                    covers_grid(app, ui, &theme, &grid, at, section, *id, query)
                } else {
                    shelf(app, ui, &theme, at, width, section, *id, query)
                };
                // More than a row holds: the title shows them all.
                if *open || section.cards.len() > grid.columns() {
                    if fold_head(ui, &theme, at, section.title, *open, *id) {
                        ui.ctx().data_mut(|d| d.insert_temp(*id, !*open));
                    }
                } else {
                    section_head(ui, &theme, at.x, at.y, section.title);
                }
                if let Some((i, click)) = clicked {
                    card_clicked(app, ui, section, i, click, query);
                }
                y += section_height(section, *open) - SECTION_GAP;
            }
        });
    output.state.clamped_offset().y
}

/// Where a section of the current search keeps whether it shows all it found.
fn unfold_id(app: &App, query: &str, name: &str) -> Id {
    Id::unique((
        "search-unfolded",
        query,
        app.search_scope,
        app.search_source,
        name,
    ))
}

/// A section's title that shows all it found, or folds them back: a chevron
/// after it, right when folded, down when open. Returns whether it was clicked.
fn fold_head(ui: &Ui, theme: &Theme, at: egui::Pos2, label: &str, open: bool, id: Id) -> bool {
    let g = text(ui, label, font(18.0, Weight::SemiBold), theme.text, 400.0);
    let end = at.x + g.size().x;
    paint_text(ui, g, at.x, at.y + 11.0, egui::Align::Min, theme.text);
    let response = ui.interact(
        Rect::from_min_max(pos2(at.x, at.y - 4.0), pos2(end + 26.0, at.y + 26.0)),
        id.with("head"),
        Sense::click(),
    );
    let colour = if response.hovered() {
        theme.text
    } else {
        theme.muted()
    };
    let chevron = if open { icons::DOWN } else { icons::RIGHT };
    paint_icon(ui, chevron, pos2(end + 13.0, at.y + 11.0), 15.0, colour);
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// A section of covers: albums, artists or playlists.
struct Section {
    title: &'static str,
    cards: Vec<Card>,
    round: bool,
}

/// Albums, the artists after the top result, playlists.
fn sections(r: &SearchResults) -> Vec<Section> {
    vec![
        Section {
            title: "Albums",
            cards: album_cards(r),
            round: false,
        },
        Section {
            title: "Artists",
            cards: artist_cards(r),
            round: true,
        },
        Section {
            title: "Playlists",
            cards: playlist_cards(r),
            round: false,
        },
    ]
}

/// An artist first, else an album, else a playlist.
fn top_result(r: &SearchResults) -> Option<Top> {
    if let Some(a) = r.artists.first() {
        return Some(Top::Artist(a.id.clone(), a.name.clone(), a.artwork.clone()));
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
            explicit: a.explicit,
            open: Open::Album(a.id.clone()),
        })
        .collect()
}

/// The artists after the first, which is the top result.
fn artist_cards(r: &SearchResults) -> Vec<Card> {
    r.artists
        .iter()
        .skip(1)
        .map(|a| Card {
            title: a.name.clone(),
            sub: "Artist".into(),
            art: a.artwork.clone(),
            open: Open::Artist(a.id.clone()),
            explicit: false,
        })
        .collect()
}

fn playlist_cards(r: &SearchResults) -> Vec<Card> {
    r.playlists
        .iter()
        .enumerate()
        .map(|(i, p)| Card {
            title: p.name.clone(),
            // Who made it: Apple's editors, or a SoundCloud user.
            sub: p.curator.clone().unwrap_or_else(|| "Apple Music".into()),
            art: p.artwork.clone(),
            open: Open::Playlist(i),
            explicit: false,
        })
        .collect()
}

/// Open a result: its album, or a playlist of the current search.
fn open(app: &mut App, what: Open, query: &str) {
    match what {
        Open::Album(id) => app.go(Page::Album(id)),
        Open::Artist(id) => app.go(Page::Artist(id)),
        Open::Playlist(i) => {
            let playlist = match app.search_results(query) {
                Some(Loaded::Ready(r)) => r.playlists.get(i).cloned(),
                _ => None,
            };
            if let Some(playlist) = playlist {
                app.open_playlist(&playlist);
            }
        }
    }
}

pub(super) fn section_head(ui: &Ui, theme: &Theme, left: f32, top: f32, label: &str) {
    let g = text(ui, label, font(18.0, Weight::SemiBold), theme.text, 400.0);
    paint_text(ui, g, left, top + 11.0, egui::Align::Min, theme.text);
}

pub(super) fn message(
    ui: &Ui,
    theme: &Theme,
    left: f32,
    top: f32,
    width: f32,
    title: &str,
    detail: &str,
) {
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
        Top::Artist(id, name, art) => (
            "ARTIST",
            name.clone(),
            None,
            art.clone(),
            true,
            Some(Open::Artist(id.clone())),
        ),
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
            let Some(p) = r.playlists.get(*i).cloned() else {
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
            favourite: app.is_favourite(FavouriteKind::Song, &track.id),
            in_library: app.track_in_library(track),
            in_playlist: false,
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
            RowAction::Menu(choice) => {
                let track = track.clone();
                app.track_choice(track, choice, ui.input(|i| i.time));
            }
        }
    }
}

/// The album or playlist a card stands for.
fn collection(app: &App, open: &Open, query: &str) -> Option<Collection> {
    let Some(Loaded::Ready(r)) = app.search_results(query) else {
        return None;
    };
    match open {
        Open::Album(id) => r
            .albums
            .iter()
            .find(|a| a.id == *id)
            .cloned()
            .map(Collection::Album),
        Open::Playlist(k) => r.playlists.get(*k).cloned().map(Collection::Playlist),
        Open::Artist(_) => None,
    }
}

/// A section's covers in a row that scrolls sideways (its title is drawn
/// over it, to be clicked); returns the cover clicked, and how.
#[allow(clippy::too_many_arguments)]
fn shelf(
    app: &App,
    ui: &mut Ui,
    theme: &Theme,
    at: egui::Pos2,
    width: f32,
    section: &Section,
    id: Id,
    query: &str,
) -> Option<(usize, CardClick)> {
    let items: Vec<ShelfCard> = section
        .cards
        .iter()
        .map(|c| ShelfCard {
            title: c.title.clone(),
            sub: c.sub.clone(),
            url: c.art.as_ref().map(|a| a.url(376)),
            explicit: c.explicit,
            addable: collection(app, &c.open, query).is_some_and(|c| app.can_add_to_library(&c)),
        })
        .collect();
    let id = id.with("shelf");
    if section.round {
        round_shelf(ui, theme, at, width, "", &items, CARD, id).map(|i| (i, CardClick::Open))
    } else {
        cover_shelf(ui, theme, at, width, "", &items, CARD, id)
    }
}

/// All of a section's covers, in as many columns as fit.
#[allow(clippy::too_many_arguments)]
fn covers_grid(
    app: &App,
    ui: &mut Ui,
    theme: &Theme,
    grid: &Grid,
    at: egui::Pos2,
    section: &Section,
    id: Id,
    query: &str,
) -> Option<(usize, CardClick)> {
    let top = at.y + SECTION_HEAD;
    let mut clicked = None;
    for (i, c) in section.cards.iter().enumerate() {
        let art = grid.cell(i, at.x, top);
        if !ui.is_rect_visible(art.with_max_y(art.bottom() + super::grid::CARD_TEXT)) {
            continue;
        }
        let card = CoverCard {
            url: c.art.as_ref().map(|a| a.url(376)),
            title: &c.title,
            sub: &c.sub,
            round: section.round,
            explicit: c.explicit,
            addable: collection(app, &c.open, query).is_some_and(|c| app.can_add_to_library(&c)),
            menu: !section.round,
        };
        if let Some(click) = cover_card(ui, theme, art, &card, id.with(("card", i))) {
            clicked = Some((i, click));
        }
    }
    clicked
}

/// What a click on one of a section's covers does: open it, or what its
/// menu asked for.
fn card_clicked(
    app: &mut App,
    ui: &Ui,
    section: &Section,
    i: usize,
    click: CardClick,
    query: &str,
) {
    let target = section.cards[i].open.clone();
    let now = ui.input(|i| i.time);
    let how = match click {
        CardClick::Open => return open(app, target, query),
        CardClick::AddToLibrary => {
            if let Some(item) = collection(app, &target, query) {
                app.add_collection_to_library(&item, now);
            }
            return;
        }
        CardClick::Enqueue(how) => how,
    };
    match &target {
        Open::Album(id) => app.enqueue_album(id.clone(), how, now),
        Open::Playlist(k) => {
            let playlist = match app.search_results(query) {
                Some(Loaded::Ready(r)) => r.playlists.get(*k).cloned(),
                _ => None,
            };
            if let Some(playlist) = playlist {
                app.enqueue_playlist(&playlist, how, now);
            }
        }
        Open::Artist(_) => {}
    }
}
