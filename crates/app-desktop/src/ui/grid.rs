//! Cover grids: the library's Albums, Artists and Recently Added pages, and the
//! cover card and grid layout the artist page and search share.

use egui::{Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use music_core::model::{ItemId, LibraryItem, Playlist, ProviderKind};

use crate::app::{App, Page};
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::songs::source_chips;
use super::{artwork, glow, paint_icon, paint_text, text};

/// Grid covers are at least this wide, 20 apart, rows 26 apart.
const CARD_MIN: f32 = 158.0;
const CARD_GAP: f32 = 20.0;
const ROW_GAP: f32 = 26.0;
/// The title and the line under a cover.
const CARD_TEXT: f32 = 42.0;
/// Where the grid starts, under the title.
const GRID_TOP: f32 = 132.0;

/// What a cover card shows.
pub(super) struct CoverCard<'a> {
    pub url: Option<String>,
    pub title: &'a str,
    pub sub: &'a str,
    /// An artist: round, with the name centred.
    pub round: bool,
}

/// A cover with its title and a muted line under it; `art` is the cover's
/// square. Returns whether it was clicked.
pub(super) fn cover_card(ui: &Ui, theme: &Theme, art: Rect, card: &CoverCard, id: Id) -> bool {
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
    let g = text(
        ui,
        card.title,
        font(13.0, Weight::Medium),
        theme.text,
        width,
    );
    paint_text(ui, g, x, art.bottom() + 17.0, align, theme.text);
    let muted = theme.muted();
    let g = text(ui, card.sub, font(12.0, Weight::Regular), muted, width);
    paint_text(ui, g, x, art.bottom() + 34.0, align, muted);
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
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
            let g = text(ui, title, font(30.0, Weight::Bold), theme.text, width);
            paint_text(
                ui,
                g,
                left,
                content.top() + 70.0 + 16.5,
                egui::Align::Min,
                theme.text,
            );
            if kind != Kind::Artists
                && let Some(source) = source_chips(
                    ui,
                    &theme,
                    left + width,
                    content.top() + 73.0,
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
                let cover = CoverCard {
                    url,
                    title: &card_title,
                    sub: &sub,
                    round: kind == Kind::Artists,
                };
                if cover_card(ui, &theme, art, &cover, id) {
                    clicked = Some(open);
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
    match clicked {
        Some(Open::Album(id)) => app.go(Page::Album(id)),
        Some(Open::Artist(id)) => app.go(Page::Artist(id)),
        Some(Open::Playlist(playlist)) => app.open_playlist(&playlist),
        None => {}
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

fn item_id(item: &LibraryItem) -> &ItemId {
    match item {
        LibraryItem::Album(a) => &a.id,
        LibraryItem::Playlist(p) => &p.id,
    }
}

/// Title, muted line, cover and target of item `i` of a grid.
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
            LibraryItem::Album(a) => album(a),
            LibraryItem::Playlist(p) => (
                p.name.clone(),
                "Playlist".into(),
                p.artwork.as_ref().map(|art| art.url(376)),
                Open::Playlist(p.clone()),
            ),
        },
    }
}
