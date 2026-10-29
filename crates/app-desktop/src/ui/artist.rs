//! An artist: the header with round artwork over its colours, the top songs,
//! then grids of albums and of singles and EPs.

use egui::{Id, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{Album, ArtistDetail, ItemId};

use crate::app::{App, Loaded, Page};
use crate::theme::Theme;

use super::detail::{ART_TOP, Header, HeaderAction, art_size, hero};
use super::search::{cover_card, message, section_head};
use super::tracks::{self, Columns, RowAction, RowState};

const SECTION_GAP: f32 = 34.0;
const SECTION_HEAD: f32 = 34.0;
/// Grid covers are at least this wide, 20 apart, rows 26 apart.
const CARD_MIN: f32 = 158.0;
const CARD_GAP: f32 = 20.0;
const ROW_GAP: f32 = 26.0;
/// The title and the line under a cover.
const CARD_TEXT: f32 = 42.0;

pub fn show(app: &mut App, ui: &mut Ui, pad: f32, id: &ItemId) -> f32 {
    let theme = app.theme.clone();
    let (artist, error) = match app.artists.get(id) {
        Some(Loaded::Ready(a)) => (Some(a.clone()), None),
        Some(Loaded::Failed(e)) => (None, Some(e.clone())),
        _ => (None, None),
    };
    let width = ui.available_width() - 2.0 * pad;
    let grid = Grid::new(width);
    let body_top = ART_TOP + art_size(pad) + 40.0;
    let sections: Vec<(&str, &[Album])> = artist
        .as_ref()
        .map(|a| {
            vec![
                ("Albums", a.albums.as_slice()),
                ("Singles & EPs", a.singles.as_slice()),
            ]
        })
        .unwrap_or_default();
    let songs = artist.as_ref().map_or(0, |a| a.top_songs.len());
    let mut total = body_top;
    if songs > 0 {
        total += SECTION_HEAD + songs as f32 * tracks::ROW + SECTION_GAP;
    }
    for (_, albums) in &sections {
        if !albums.is_empty() {
            total += SECTION_HEAD + grid.height(albums.len()) + SECTION_GAP;
        }
    }
    total += 104.0;

    let header = artist.as_ref().map(|a| header(a));
    let colors = super::backdrop(app);
    let output = egui::ScrollArea::vertical()
        .id_salt(("artist", &id.id))
        .auto_shrink(false)
        .show(ui, |ui| {
            let (content, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
            let left = content.left() + pad;
            let action = hero(ui, &theme, content, pad, header.as_ref(), colors);
            let mut y = content.top() + body_top;
            if let Some(e) = &error {
                message(ui, &theme, left, y, width, "Couldn't load this artist", e);
                return;
            }
            let Some(artist) = &artist else {
                tracks::skeleton(ui, &theme, y + SECTION_HEAD, left, width, 6);
                return;
            };
            if let Some(HeaderAction::Play { shuffle }) = action {
                app.play_shuffled_or_not(artist.top_songs.clone(), shuffle, ui.input(|i| i.time));
            }

            if songs > 0 {
                section_head(ui, &theme, left, y, "Top Songs");
                y += SECTION_HEAD;
                top_songs(app, ui, &theme, artist, left, y, width);
                y += songs as f32 * tracks::ROW + SECTION_GAP;
            }
            for (title, albums) in &sections {
                if albums.is_empty() {
                    continue;
                }
                section_head(ui, &theme, left, y, title);
                y += SECTION_HEAD;
                for (i, album) in albums.iter().enumerate() {
                    let art = grid.cell(i, left, y);
                    let url = album.artwork.as_ref().map(|a| a.url(376));
                    let sub = album.year.map(|y| y.to_string()).unwrap_or_default();
                    let cell = Id::unique(("artist-card", *title, i));
                    if cover_card(ui, &theme, art, url, &album.title, &sub, cell) {
                        app.go(Page::Album(album.id.clone()));
                    }
                }
                y += grid.height(albums.len()) + SECTION_GAP;
            }
        });
    output.state.clamped_offset().y
}

fn header(a: &ArtistDetail) -> Header {
    let count =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let mut meta = Vec::new();
    if !a.albums.is_empty() {
        meta.push((None, count(a.albums.len(), "ALBUM", "ALBUMS")));
    }
    if !a.singles.is_empty() {
        meta.push((
            None,
            count(a.singles.len(), "SINGLE OR EP", "SINGLES & EPS"),
        ));
    }
    meta.push((Some(crate::icons::AM), "APPLE MUSIC".into()));
    Header {
        kind: "Artist",
        title: a.artist.name.clone(),
        by: None,
        by_link: None,
        meta,
        art: a.artist.artwork.as_ref().map(|a| a.url(600)),
        round: true,
    }
}

#[allow(clippy::too_many_arguments)]
fn top_songs(
    app: &mut App,
    ui: &mut Ui,
    theme: &Theme,
    artist: &ArtistDetail,
    left: f32,
    top: f32,
    width: f32,
) {
    let columns = Columns::new(left, width, true);
    for (position, track) in artist.top_songs.iter().enumerate() {
        let rect = Rect::from_min_size(
            pos2(left, top + position as f32 * tracks::ROW),
            vec2(width, tracks::ROW),
        );
        let now = app.is_current(track);
        let state = RowState {
            number: Some(position as u32 + 1),
            selected: app.detail_selected == Some(position),
            now,
            playing: now && app.is_playing(),
            list: "artist",
            position,
        };
        match tracks::row(ui, theme, &columns, rect, track, state) {
            RowAction::None => {}
            RowAction::Select => app.detail_selected = Some(position),
            RowAction::Play => {
                app.detail_selected = Some(position);
                app.play_list(artist.top_songs.clone(), position);
            }
            RowAction::OpenAlbum(id) => app.go(Page::Album(id)),
        }
    }
}

/// Covers in as many columns as fit, sharing the width.
struct Grid {
    columns: usize,
    card: f32,
}

impl Grid {
    fn new(width: f32) -> Self {
        let columns = (((width + CARD_GAP) / (CARD_MIN + CARD_GAP)).floor() as usize).max(1);
        let card = (width - CARD_GAP * (columns - 1) as f32) / columns as f32;
        Self { columns, card }
    }

    fn height(&self, count: usize) -> f32 {
        let rows = count.div_ceil(self.columns);
        rows as f32 * (self.card + CARD_TEXT) + rows.saturating_sub(1) as f32 * ROW_GAP
    }

    /// The cover square of item `i`.
    fn cell(&self, i: usize, left: f32, top: f32) -> Rect {
        let (row, column) = (i / self.columns, i % self.columns);
        Rect::from_min_size(
            pos2(
                left + column as f32 * (self.card + CARD_GAP),
                top + row as f32 * (self.card + CARD_TEXT + ROW_GAP),
            ),
            vec2(self.card, self.card),
        )
    }
}
