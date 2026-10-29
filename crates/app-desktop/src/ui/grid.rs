//! Cover grids: the library's Albums, Artists and Recently Added pages, and the
//! cover card and grid layout the artist page and search share.

use egui::{Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use music_core::model::{
    Collection, FavouriteKind, ItemId, Playlist, ProviderKind, Station, Track,
};

use crate::app::{App, Enqueue, Loaded, Page, Shelves};
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::menu::{TrackFacts, track_menu, track_menu_button};
use super::songs::source_chips;
use super::{artwork, explicit_badge, glow, icon_button, paint_icon, paint_text, text, thumb_url};

/// Songs in columns: three rows to a column, as in Cider.
const SONG_ROW: f32 = 56.0;
const SONG_COLUMN: f32 = 300.0;
const SONG_GAP: f32 = 24.0;
/// How tall a block of song columns is, three rows to a column (an artist's
/// top songs).
pub(super) const SONG_COLUMNS_HEIGHT: f32 = 3.0 * SONG_ROW;

/// How tall a block of song columns of `rows` rows is.
pub(super) fn song_columns_height(rows: usize) -> f32 {
    rows as f32 * SONG_ROW
}

/// Grid covers are at least this wide, 26 apart, rows 40 apart, as in Cider.
const CARD_MIN: f32 = 190.0;
const CARD_GAP: f32 = 26.0;
const ROW_GAP: f32 = 40.0;
/// The title and the line under a cover.
pub(super) const CARD_TEXT: f32 = 50.0;
/// Where the grid starts, under the title.
const GRID_TOP: f32 = 100.0;
/// The title's centre line, just under the top bar.
const TITLE_Y: f32 = 66.0;

/// What a cover card shows.
pub(super) struct CoverCard<'a> {
    pub url: Option<String>,
    pub title: &'a str,
    pub sub: &'a str,
    /// An artist: round, with the name centred.
    pub round: bool,
    /// An E after the title.
    pub explicit: bool,
}

/// What a click on a cover asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum CardClick {
    Open,
    /// From the right-click menu of an album or a playlist.
    Enqueue(Enqueue),
}

/// A cover with its title and a muted line under it; `art` is the cover's
/// square. Albums and playlists (square covers) have a right-click menu.
pub(super) fn cover_card(
    ui: &Ui,
    theme: &Theme,
    art: Rect,
    card: &CoverCard,
    id: Id,
) -> Option<CardClick> {
    let response = ui.interact(art, id, Sense::click());
    let radius = if card.round { art.width() / 2.0 } else { 8.0 };
    let missing = card.url.is_none();
    artwork(ui, card.url.clone(), art, radius, theme);
    if missing {
        let icon = if card.round { icons::MIC } else { icons::DISC };
        paint_icon(ui, icon, art.center(), art.width() / 3.0, theme.muted());
    }
    if response.hovered() {
        ui.painter()
            .rect_filled(art, radius, egui::Color32::from_black_alpha(40));
    }
    ui.painter().rect_stroke(
        art,
        radius,
        Stroke::new(1.0, theme.line()),
        egui::StrokeKind::Inside,
    );
    let (x, align) = if card.round {
        (art.center().x, egui::Align::Center)
    } else {
        (art.left(), egui::Align::Min)
    };
    let width = art.width();
    let explicit = card.explicit && !card.round;
    let badge = if explicit { 18.0 } else { 0.0 };
    let g = text(
        ui,
        card.title,
        font(13.5, Weight::Regular),
        theme.text,
        width - badge,
    );
    let title_end = x + g.size().x;
    paint_text(ui, g, x, art.bottom() + 22.0, align, theme.text);
    if explicit {
        explicit_badge(ui, theme, pos2(title_end + 4.0 + 6.0, art.bottom() + 22.0));
    }
    let muted = theme.muted();
    let g = text(ui, card.sub, font(12.0, Weight::Regular), muted, width);
    paint_text(ui, g, x, art.bottom() + 39.0, align, muted);
    if !card.round
        && let Some(how) = super::menu::collection_menu(&response, theme)
    {
        return Some(CardClick::Enqueue(how));
    }
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
        .then_some(CardClick::Open)
}

/// Covers in as many columns as fit, sharing the width.
pub(super) struct Grid {
    columns: usize,
    card: f32,
}

impl Grid {
    pub fn new(width: f32) -> Self {
        let columns = (((width + CARD_GAP) / (CARD_MIN + CARD_GAP)).floor() as usize).max(1);
        let card = (width - CARD_GAP * (columns - 1) as f32) / columns as f32;
        Self { columns, card }
    }

    fn row_height(&self) -> f32 {
        self.card + CARD_TEXT + ROW_GAP
    }

    pub fn height(&self, count: usize) -> f32 {
        let rows = count.div_ceil(self.columns);
        rows as f32 * self.row_height() - if rows > 0 { ROW_GAP } else { 0.0 }
    }

    /// The cover square of item `i`.
    pub fn cell(&self, i: usize, left: f32, top: f32) -> Rect {
        let (row, column) = (i / self.columns, i % self.columns);
        Rect::from_min_size(
            pos2(
                left + column as f32 * (self.card + CARD_GAP),
                top + row as f32 * self.row_height(),
            ),
            vec2(self.card, self.card),
        )
    }

    /// The items of `count` that the viewport shows, for a grid starting at
    /// `top` (in the viewport's coordinates).
    fn visible(&self, viewport: Rect, top: f32, count: usize) -> std::ops::Range<usize> {
        let row = |y: f32| ((y - top) / self.row_height()).max(0.0);
        let first = row(viewport.top()).floor() as usize * self.columns;
        let last = (row(viewport.bottom()).ceil() as usize * self.columns).min(count);
        first.min(last)..last
    }
}

/// Where a shelf is scrolled to, and where it is heading.
#[derive(Clone, Copy, Default)]
struct ShelfScroll {
    offset: f32,
    target: f32,
}

/// Sideways scrolling for a shelf: arrows ending at `arrows` (the right end of
/// its title row), sideways scrolling over `row`, eased. `content` is the
/// shelf's full width, `page` how far an arrow moves. Returns the offset.
pub(super) fn shelf_scroll(
    ui: &mut Ui,
    theme: &Theme,
    id: Id,
    arrows: egui::Pos2,
    row: Rect,
    content: f32,
    page: f32,
) -> f32 {
    let max = (content - row.width()).max(0.0);
    let mut scroll: ShelfScroll = ui.ctx().data(|d| d.get_temp(id)).unwrap_or_default();
    if max > 0.0 {
        let buttons = [
            (icons::LEFT, -page, scroll.target > 0.0),
            (icons::RIGHT, page, scroll.target < max),
        ];
        for (i, (icon, step, enabled)) in buttons.into_iter().enumerate() {
            let rect = Rect::from_center_size(
                pos2(arrows.x - 14.0 - (1 - i) as f32 * 32.0, arrows.y),
                vec2(28.0, 28.0),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
            if round_arrow(&mut child, theme, icon, enabled) {
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
    scroll.offset
}

/// Cider's shelf arrows: round, on a faint fill.
fn round_arrow(ui: &mut Ui, theme: &Theme, icon: icons::Icon, enabled: bool) -> bool {
    let rect = ui.max_rect();
    let fill = theme.text_a(if enabled { 0.08 } else { 0.04 });
    ui.painter()
        .circle_filled(rect.center(), rect.width() / 2.0, fill);
    icon_button(ui, theme, icon, 28.0, 15.0, theme.subtext0, enabled).clicked()
}

/// A cover in a shelf.
pub(super) struct ShelfCard {
    pub title: String,
    pub sub: String,
    pub url: Option<String>,
    pub explicit: bool,
}

/// A titled row of `size` px covers that scrolls sideways. Returns the cover
/// clicked, and how.
#[allow(clippy::too_many_arguments)]
pub(super) fn cover_shelf(
    ui: &mut Ui,
    theme: &Theme,
    at: egui::Pos2,
    width: f32,
    title: &str,
    cards: &[ShelfCard],
    size: f32,
    id: Id,
) -> Option<(usize, CardClick)> {
    super::search::section_head(ui, theme, at.x, at.y, title);
    let row = Rect::from_min_size(pos2(at.x, at.y + SHELF_HEAD), vec2(width, size + CARD_TEXT));
    let stride = size + SHELF_GAP;
    let content = cards.len() as f32 * stride - SHELF_GAP;
    // A page is as many whole covers as fit.
    let page = ((width + SHELF_GAP) / stride).floor().max(1.0) * stride;
    let offset = shelf_scroll(
        ui,
        theme,
        id,
        pos2(at.x + width, at.y + 11.0),
        row,
        content,
        page,
    );

    let mut row_ui = ui.new_child(egui::UiBuilder::new().max_rect(row));
    row_ui.set_clip_rect(row.intersect(ui.clip_rect()));
    let mut clicked = None;
    for (i, card) in cards.iter().enumerate() {
        let x = row.left() + i as f32 * stride - offset;
        if x + size < row.left() || x > row.right() {
            continue;
        }
        let art = Rect::from_min_size(pos2(x, row.top()), vec2(size, size));
        let cover = CoverCard {
            url: card.url.clone(),
            title: &card.title,
            sub: &card.sub,
            round: false,
            explicit: card.explicit,
        };
        if let Some(click) = cover_card(&row_ui, theme, art, &cover, id.with(i)) {
            clicked = Some((i, click));
        }
    }
    clicked
}

/// One shelf of Home, New or Radio in full: every cover, in a grid.
pub fn shelf(app: &mut App, ui: &mut Ui, pad: f32, which: Shelves, title: &str) -> f32 {
    let theme = app.theme.clone();
    let shelf = match app.shelves(which) {
        Some(Loaded::Ready(shelves)) => Ok(shelves.iter().find(|s| s.title == title).cloned()),
        Some(Loaded::Failed(e)) => Err(e.clone()),
        _ => Ok(None),
    };
    let loading =
        matches!(shelf, Ok(None)) && !matches!(app.shelves(which), Some(Loaded::Ready(_)));
    let items = match &shelf {
        Ok(Some(shelf)) => shelf.items.clone(),
        _ => Vec::new(),
    };
    let width = ui.available_width() - 2.0 * pad;
    let grid = Grid::new(width);
    let grid_height = if loading {
        grid.height(grid.columns * 2)
    } else {
        grid.height(items.len()).max(60.0)
    };
    let total = GRID_TOP + grid_height + 104.0;

    let mut clicked = None;
    let output = egui::ScrollArea::vertical()
        .id_salt(("shelf-page", title))
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
            let g = text(ui, title, font(22.0, Weight::Bold), theme.text, width);
            paint_text(
                ui,
                g,
                left,
                content.top() + TITLE_Y,
                egui::Align::Min,
                theme.text,
            );
            let top = content.top() + GRID_TOP;
            match &shelf {
                Err(e) => {
                    super::search::message(ui, &theme, left, top, width, "Couldn't load it", e);
                    return;
                }
                Ok(None) if loading => {
                    for i in 0..grid.columns * 2 {
                        ui.painter()
                            .rect_filled(grid.cell(i, left, top), 8.0, theme.hover());
                    }
                    return;
                }
                Ok(None) => {
                    let detail =
                        "Apple Music took this shelf away. It may be back after a refresh.";
                    super::search::message(ui, &theme, left, top, width, "Gone", detail);
                    return;
                }
                Ok(Some(_)) => {}
            }
            for position in grid.visible(viewport, GRID_TOP, items.len()) {
                let card = super::home::card(&items[position]);
                let cover = CoverCard {
                    url: card.url,
                    title: &card.title,
                    sub: &card.sub,
                    round: false,
                    explicit: card.explicit,
                };
                let id = Id::unique(("shelf-card", title, position));
                let art = grid.cell(position, left, top);
                if let Some(click) = cover_card(ui, &theme, art, &cover, id) {
                    clicked = Some((position, click));
                }
            }
        });
    if let Some((i, click)) = clicked {
        super::home::act(app, &items[i], click, ui.input(|i| i.time));
    }
    output.state.clamped_offset().y
}

/// A shelf's title row, and the gap between its covers.
pub(super) const SHELF_HEAD: f32 = 34.0;
pub(super) const SHELF_GAP: f32 = 20.0;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Albums,
    Artists,
    Recent,
}

/// Where a click on a cover leads.
enum Open {
    Album(ItemId),
    Artist(ItemId),
    Playlist(Playlist),
    Station(Station),
}

pub fn albums(app: &mut App, ui: &mut Ui, pad: f32) -> f32 {
    page(app, ui, pad, Kind::Albums)
}

pub fn artists(app: &mut App, ui: &mut Ui, pad: f32) -> f32 {
    page(app, ui, pad, Kind::Artists)
}

pub fn recently_added(app: &mut App, ui: &mut Ui, pad: f32) -> f32 {
    page(app, ui, pad, Kind::Recent)
}

fn page(app: &mut App, ui: &mut Ui, pad: f32, kind: Kind) -> f32 {
    let theme = app.theme.clone();
    let source = app.grid_source.filter(|_| kind != Kind::Artists);
    let keep = |provider: ProviderKind| source.is_none_or(|s| s == provider);
    // The items shown, as indices into the list.
    let (shown, done): (Vec<usize>, bool) = match kind {
        Kind::Albums => (
            indices(&app.library_albums.items, |a| keep(a.id.provider)),
            app.library_albums.done,
        ),
        Kind::Artists => (
            (0..app.library_artists.items.len()).collect(),
            app.library_artists.done,
        ),
        Kind::Recent => (
            indices(&app.recently_added.items, |item| {
                keep(item_id(item).provider)
            }),
            app.recently_added.done,
        ),
    };
    let (title, one, many) = match kind {
        Kind::Albums => ("Albums", "album", "albums"),
        Kind::Artists => ("Artists", "artist", "artists"),
        Kind::Recent => ("Recently Added", "item", "items"),
    };
    let width = ui.available_width() - 2.0 * pad;
    let grid = Grid::new(width);
    let loading = shown.is_empty() && !done;
    let grid_height = if loading {
        grid.height(grid.columns * 2)
    } else {
        grid.height(shown.len()).max(60.0)
    };
    let total = GRID_TOP + grid_height + 18.0 + 16.0 + 18.0 + 104.0;

    let mut clicked = None;
    let output = egui::ScrollArea::vertical()
        .id_salt(("grid", title))
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
            let g = text(ui, title, font(22.0, Weight::Bold), theme.text, width);
            paint_text(
                ui,
                g,
                left,
                content.top() + TITLE_Y,
                egui::Align::Min,
                theme.text,
            );
            if kind != Kind::Artists
                && let Some(source) = source_chips(
                    ui,
                    &theme,
                    left + width,
                    content.top() + TITLE_Y - 14.0,
                    app.grid_source,
                )
            {
                app.grid_source = source;
            }

            let top = content.top() + GRID_TOP;
            if loading {
                for i in 0..grid.columns * 2 {
                    let art = grid.cell(i, left, top);
                    let radius = if kind == Kind::Artists {
                        art.width() / 2.0
                    } else {
                        8.0
                    };
                    ui.painter().rect_filled(art, radius, theme.hover());
                }
                return;
            }
            if shown.is_empty() {
                let (head, detail) = if source.is_some() {
                    (
                        "Nothing from this source",
                        "Connect it in Settings, or pick another source.".to_owned(),
                    )
                } else {
                    (
                        "Nothing here yet",
                        format!("Your library has no {many} yet."),
                    )
                };
                let g = text(ui, head, font(15.0, Weight::SemiBold), theme.text, width);
                paint_text(ui, g, left, top + 20.0, egui::Align::Min, theme.text);
                let g = text(
                    ui,
                    &detail,
                    font(13.0, Weight::Regular),
                    theme.subtext0,
                    width,
                );
                paint_text(ui, g, left, top + 44.0, egui::Align::Min, theme.subtext0);
                return;
            }

            for position in grid.visible(viewport, GRID_TOP, shown.len()) {
                let art = grid.cell(position, left, top);
                let id = Id::unique(("grid-card", title, position));
                let (card_title, sub, url, open) = card(app, kind, shown[position]);
                let explicit =
                    matches!(&open, Open::Album(_)) && explicit_album(app, kind, shown[position]);
                let cover = CoverCard {
                    url,
                    title: &card_title,
                    sub: &sub,
                    round: kind == Kind::Artists,
                    explicit,
                };
                if let Some(click) = cover_card(ui, &theme, art, &cover, id) {
                    clicked = Some((open, click));
                }
            }

            let count = shown.len();
            let foot = format!(
                "{count} {}{}",
                if count == 1 { one } else { many },
                if done { "" } else { " so far, loading…" }
            );
            let muted = theme.muted();
            let g = text(ui, &foot, font(12.0, Weight::Regular), muted, width);
            paint_text(
                ui,
                g,
                left,
                top + grid.height(count) + 18.0 + 8.0,
                egui::Align::Min,
                muted,
            );
        });
    let now = ui.input(|i| i.time);
    match clicked {
        Some((Open::Album(id), CardClick::Open)) => app.go(Page::Album(id)),
        Some((Open::Artist(id), CardClick::Open)) => app.go(Page::Artist(id)),
        Some((Open::Playlist(playlist), CardClick::Open)) => app.open_playlist(&playlist),
        Some((Open::Station(station), _)) => app.play_station(&station),
        Some((Open::Album(id), CardClick::Enqueue(how))) => app.enqueue_album(id, how, now),
        Some((Open::Playlist(p), CardClick::Enqueue(how))) => app.enqueue_playlist(&p, how, now),
        Some((Open::Artist(_), CardClick::Enqueue(_))) | None => {}
    }
    output.state.clamped_offset().y
}

fn indices<T>(items: &[T], keep: impl Fn(&T) -> bool) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| keep(item))
        .map(|(i, _)| i)
        .collect()
}

fn item_id(item: &Collection) -> &ItemId {
    match item {
        Collection::Album(a) => &a.id,
        Collection::Playlist(p) => &p.id,
        Collection::Station(s) => &s.id,
    }
}

/// Title, muted line, cover and target of item `i` of a grid.
/// Whether item `i` of a library page is an album rated explicit.
fn explicit_album(app: &App, kind: Kind, i: usize) -> bool {
    match kind {
        Kind::Albums => app.library_albums.items[i].explicit,
        Kind::Recent => matches!(&app.recently_added.items[i], Collection::Album(a) if a.explicit),
        Kind::Artists => false,
    }
}

fn card(app: &App, kind: Kind, i: usize) -> (String, String, Option<String>, Open) {
    let album = |a: &music_core::model::Album| {
        let artists = a.artists.join(", ");
        let sub = match a.year {
            Some(year) => format!("{artists} · {year}"),
            None => artists,
        };
        (
            a.title.clone(),
            sub,
            a.artwork.as_ref().map(|art| art.url(376)),
            Open::Album(a.id.clone()),
        )
    };
    match kind {
        Kind::Albums => album(&app.library_albums.items[i]),
        Kind::Artists => {
            let a = &app.library_artists.items[i];
            (
                a.name.clone(),
                String::new(),
                a.artwork.as_ref().map(|art| art.url(376)),
                Open::Artist(a.id.clone()),
            )
        }
        Kind::Recent => match &app.recently_added.items[i] {
            Collection::Album(a) => album(a),
            Collection::Playlist(p) => (
                p.name.clone(),
                "Playlist".into(),
                p.artwork.as_ref().map(|art| art.url(376)),
                Open::Playlist(p.clone()),
            ),
            Collection::Station(s) => (
                s.name.clone(),
                "Station".into(),
                s.artwork.as_ref().map(|art| art.url(376)),
                Open::Station(s.clone()),
            ),
        },
    }
}

/// Songs in columns of `rows` that scroll sideways, as Apple lists an
/// artist's top songs and the best new songs: a click plays from that song,
/// a right click or its "…" opens its menu. Under each title, its album
/// (`show_album`) or its artists.
#[allow(clippy::too_many_arguments)]
pub(super) fn song_columns(
    app: &mut App,
    ui: &mut Ui,
    theme: &Theme,
    row: Rect,
    arrows: egui::Pos2,
    songs: &[Track],
    id: Id,
    show_album: bool,
    rows: usize,
) {
    let room = row.width();
    let columns = songs.len().div_ceil(rows);
    let stride = SONG_COLUMN + SONG_GAP;
    let page = ((room + SONG_GAP) / stride).floor().max(1.0) * stride;
    let offset = shelf_scroll(
        ui,
        theme,
        id,
        arrows,
        row,
        columns as f32 * stride - SONG_GAP,
        page,
    );

    let mut row_ui = ui.new_child(egui::UiBuilder::new().max_rect(row));
    row_ui.set_clip_rect(row.intersect(ui.clip_rect()));
    let mut play = None;
    let mut chosen = None;
    for (i, track) in songs.iter().enumerate() {
        let (column, line) = (i / rows, i % rows);
        let cell = Rect::from_min_size(
            pos2(
                row.left() + column as f32 * stride - offset,
                row.top() + line as f32 * SONG_ROW,
            ),
            vec2(SONG_COLUMN, SONG_ROW),
        );
        if cell.right() < row.left() || cell.left() > row.right() {
            continue;
        }
        let response = row_ui.interact(cell, id.with(i), Sense::click());
        if response.hovered() {
            row_ui.painter().rect_filled(cell, 8.0, theme.hover());
        }
        let now = app.is_current(track);
        let thumb = Rect::from_min_size(
            pos2(cell.left() + 6.0, cell.center().y - 20.0),
            vec2(40.0, 40.0),
        );
        artwork(&row_ui, thumb_url(track), thumb, 4.0, theme);
        if response.hovered() {
            row_ui
                .painter()
                .rect_filled(thumb, 4.0, egui::Color32::from_black_alpha(90));
            paint_icon(
                &row_ui,
                icons::PLAY_FILL,
                thumb.center(),
                14.0,
                egui::Color32::WHITE,
            );
        }
        let tx = thumb.right() + 12.0;
        let room = cell.right() - 36.0 - tx;
        let color = if now { theme.accent } else { theme.text };
        let title_room = room - if track.explicit { 18.0 } else { 0.0 };
        let g = text(
            &row_ui,
            &track.title,
            font(13.0, Weight::Medium),
            color,
            title_room,
        );
        let title_end = tx + g.size().x;
        paint_text(
            &row_ui,
            g,
            tx,
            cell.center().y - 9.0,
            egui::Align::Min,
            color,
        );
        if track.explicit {
            explicit_badge(
                &row_ui,
                theme,
                pos2(title_end + 12.0, cell.center().y - 9.0),
            );
        }
        let muted = theme.muted();
        let sub = match &track.album {
            Some(album) if show_album => album.clone(),
            _ => track.artists.join(", "),
        };
        let g = text(&row_ui, &sub, font(12.0, Weight::Regular), muted, room);
        paint_text(
            &row_ui,
            g,
            tx,
            cell.center().y + 9.0,
            egui::Align::Min,
            muted,
        );
        let favourite = app.is_favourite(FavouriteKind::Song, &track.id);
        if favourite {
            let end = title_end + if track.explicit { 18.0 } else { 0.0 };
            super::tracks::favourite_star(&row_ui, theme, pos2(end + 12.0, cell.center().y - 9.0));
        }
        let facts = || TrackFacts {
            has_album: track.album_id.is_some(),
            favourite,
            in_library: app.track_in_library(track),
            removable: track.library_id.is_some(),
            in_playlist: false,
        };
        // "…" at the end of the row, as in Cider: the same menu, on a click.
        let more =
            Rect::from_center_size(pos2(cell.right() - 18.0, cell.center().y), vec2(26.0, 26.0));
        let more_response = row_ui.interact(more, id.with(("more", i)), Sense::click());
        let more_color = if more_response.hovered() {
            theme.text
        } else {
            theme.text_a(0.35)
        };
        paint_icon(&row_ui, icons::MORE, more.center(), 16.0, more_color);
        if let Some(choice) = track_menu_button(&more_response, theme, facts()) {
            chosen = Some((i, choice));
        } else if let Some(choice) = track_menu(&response, theme, facts()) {
            chosen = Some((i, choice));
        } else if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            play = Some(i);
        }
    }
    if let Some(i) = play {
        app.play_list(songs.to_vec(), i);
    }
    if let Some((i, choice)) = chosen {
        let track = songs[i].clone();
        app.track_choice(track, choice, ui.input(|i| i.time));
    }
}
