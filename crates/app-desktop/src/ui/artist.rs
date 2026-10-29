//! An artist, laid out as Cider does: the round photo, the name and the play
//! buttons centred over the photo's colours; the latest release beside the top
//! songs in sideways columns; then shelves of albums and of singles and EPs.

use egui::{Id, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{Album, ArtistDetail, ItemId};

use crate::app::{App, Loaded, Page};
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::detail::cover_wash;
use super::grid::{CARD_TEXT, SHELF_HEAD, ShelfCard, cover_shelf, shelf_scroll};
use super::search::{message, section_head};
use super::{artwork, explicit_badge, format_date, paint_icon, paint_text, text, thumb_url};

const PHOTO_TOP: f32 = 56.0;
const PHOTO: f32 = 220.0;
/// From the top of the page to the first section.
const HERO: f32 = PHOTO_TOP + PHOTO + 20.0 + 58.0 + 16.0 + 56.0 + 40.0;
const SECTION_GAP: f32 = 36.0;
/// Top songs: three rows to a column, as in Cider.
const SONG_ROWS: usize = 3;
const SONG_ROW: f32 = 56.0;
const SONG_COLUMN: f32 = 300.0;
const SONG_GAP: f32 = 24.0;
/// The latest release beside them: its cover and the words next to it.
const LATEST_ART: f32 = 150.0;
const LATEST_WIDTH: f32 = LATEST_ART + 16.0 + 140.0;
const SHELF_CARD: f32 = 200.0;

pub fn show(app: &mut App, ui: &mut Ui, pad: f32, id: &ItemId) -> f32 {
    let theme = app.theme.clone();
    let (artist, error) = match app.artists.get(id) {
        Some(Loaded::Ready(a)) => (Some(a.clone()), None),
        Some(Loaded::Failed(e)) => (None, Some(e.clone())),
        _ => (None, None),
    };
    let width = ui.available_width() - 2.0 * pad;
    let top_songs = artist
        .as_ref()
        .is_some_and(|a| !a.top_songs.is_empty() || a.latest.is_some());
    let shelves: Vec<(&str, &[Album])> = artist
        .as_ref()
        .map(|a| {
            vec![
                ("Albums", a.albums.as_slice()),
                ("Singles & EPs", a.singles.as_slice()),
            ]
        })
        .unwrap_or_default();
    let shelf_height = SHELF_HEAD + SHELF_CARD + CARD_TEXT + SECTION_GAP;
    let mut total = HERO;
    if top_songs {
        total += SHELF_HEAD + SONG_ROWS as f32 * SONG_ROW + SECTION_GAP;
    }
    total += shelves.iter().filter(|(_, a)| !a.is_empty()).count() as f32 * shelf_height;
    total += 104.0;

    let colors = super::backdrop(app);
    let output = egui::ScrollArea::vertical()
        .id_salt(("artist", &id.id))
        .auto_shrink(false)
        .show(ui, |ui| {
            let (content, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
            if let Some(colors) = colors {
                cover_wash(
                    ui,
                    &theme,
                    Rect::from_min_size(content.min, vec2(content.width(), 680.0)),
                    colors,
                );
            }
            let left = content.left() + pad;
            let center = content.center().x;
            let photo = Rect::from_center_size(
                pos2(center, content.top() + PHOTO_TOP + PHOTO / 2.0),
                vec2(PHOTO, PHOTO),
            );
            ui.painter().add(
                egui::Shadow {
                    offset: [0, 16],
                    blur: 44,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(100),
                }
                .as_shape(photo.shrink(8.0), PHOTO / 2.0),
            );
            let url = artist
                .as_ref()
                .and_then(|a| a.artist.artwork.as_ref())
                .map(|a| a.url(480));
            artwork(ui, url, photo, PHOTO / 2.0, &theme);

            let mut y = content.top() + HERO;
            if let Some(e) = &error {
                message(ui, &theme, left, y, width, "Couldn't load this artist", e);
                return;
            }
            let Some(artist) = &artist else {
                return;
            };
            let name = text(
                ui,
                &artist.artist.name,
                font(48.0, Weight::Bold),
                theme.text,
                width,
            );
            let name_y = photo.bottom() + 20.0 + 29.0;
            paint_text(ui, name, center, name_y, egui::Align::Center, theme.text);
            if let Some(shuffle) =
                play_buttons(ui, &theme, pos2(center, name_y + 29.0 + 16.0 + 28.0))
            {
                app.play_shuffled_or_not(artist.top_songs.clone(), shuffle, ui.input(|i| i.time));
            }

            if top_songs {
                songs_section(app, ui, &theme, artist, pos2(left, y), width);
                y += SHELF_HEAD + SONG_ROWS as f32 * SONG_ROW + SECTION_GAP;
            }
            for (title, albums) in &shelves {
                if albums.is_empty() {
                    continue;
                }
                let cards: Vec<ShelfCard> = albums
                    .iter()
                    .map(|a| ShelfCard {
                        title: a.title.clone(),
                        sub: date_line(a),
                        url: a.artwork.as_ref().map(|art| art.url(400)),
                    })
                    .collect();
                let shelf_id = Id::unique(("artist-shelf", *title, &artist.artist.id.id));
                if let Some(i) = cover_shelf(
                    ui,
                    &theme,
                    pos2(left, y),
                    width,
                    title,
                    &cards,
                    SHELF_CARD,
                    shelf_id,
                ) {
                    app.go(Page::Album(albums[i].id.clone()));
                }
                y += shelf_height;
            }
        });
    output.state.clamped_offset().y
}

/// `MAR 27, 2026`, or the year alone.
fn date_line(album: &Album) -> String {
    album
        .released
        .as_deref()
        .and_then(format_date)
        .or_else(|| album.year.map(|y| y.to_string()))
        .unwrap_or_default()
}

/// The big round Play and Shuffle beside it, the pair centred on `center`.
/// Returns `Some(shuffle)` when one was clicked.
fn play_buttons(ui: &mut Ui, theme: &Theme, center: egui::Pos2) -> Option<bool> {
    let mut clicked = None;
    let (play, shuffle, gap) = (28.0, 20.0, 16.0);
    let left = center.x - (2.0 * play + gap + 2.0 * shuffle) / 2.0;
    let buttons = [
        (
            pos2(left + play, center.y),
            play,
            icons::PLAY_FILL,
            22.0,
            false,
            "Play",
        ),
        (
            pos2(left + 2.0 * play + gap + shuffle, center.y),
            shuffle,
            icons::SHUFFLE,
            17.0,
            true,
            "Shuffle",
        ),
    ];
    for (c, radius, icon, size, shuffle, label) in buttons {
        let rect = Rect::from_center_size(c, vec2(radius * 2.0, radius * 2.0));
        let response = ui.interact(rect, Id::unique(("artist-play", label)), Sense::click());
        let fill = theme.text_a(if response.hovered() { 0.16 } else { 0.10 });
        ui.painter().circle_filled(c, radius, fill);
        // The play triangle sits a little right of centre to look centred.
        let nudge = if shuffle { 0.0 } else { 2.0 };
        paint_icon(ui, icon, c + vec2(nudge, 0.0), size, theme.text);
        if response
            .on_hover_text(label)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            clicked = Some(shuffle);
        }
    }
    clicked
}

/// The latest release on the left, then the top songs three to a column,
/// scrolling sideways.
fn songs_section(
    app: &mut App,
    ui: &mut Ui,
    theme: &Theme,
    artist: &ArtistDetail,
    at: egui::Pos2,
    width: f32,
) {
    let mut x = at.x;
    let rows_top = at.y + SHELF_HEAD;
    if let Some(latest) = &artist.latest {
        let art = Rect::from_min_size(
            pos2(
                x,
                rows_top + (SONG_ROWS as f32 * SONG_ROW - LATEST_ART) / 2.0,
            ),
            vec2(LATEST_ART, LATEST_ART),
        );
        let response = ui.interact(art, Id::unique("artist-latest"), Sense::click());
        artwork(
            ui,
            latest.artwork.as_ref().map(|a| a.url(320)),
            art,
            8.0,
            theme,
        );
        if response.hovered() {
            ui.painter()
                .rect_filled(art, 8.0, egui::Color32::from_black_alpha(40));
        }
        let tx = art.right() + 16.0;
        let room = LATEST_WIDTH - LATEST_ART - 16.0;
        let muted = theme.muted();
        let mut ty = art.center().y - 30.0;
        let g = text(
            ui,
            &date_line(latest),
            font(11.0, Weight::SemiBold),
            muted,
            room,
        );
        paint_text(ui, g, tx, ty, egui::Align::Min, muted);
        ty += 20.0;
        let g = text(
            ui,
            &latest.title,
            font(14.0, Weight::Medium),
            theme.text,
            room,
        );
        paint_text(ui, g, tx, ty, egui::Align::Min, theme.text);
        ty += 20.0;
        if let Some(count) = latest.track_count {
            let songs = format!("{count} Song{}", if count == 1 { "" } else { "s" });
            let g = text(ui, &songs, font(12.0, Weight::Regular), muted, room);
            paint_text(ui, g, tx, ty, egui::Align::Min, muted);
        }
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            app.go(Page::Album(latest.id.clone()));
        }
        x += LATEST_WIDTH + 28.0;
    }
    if artist.top_songs.is_empty() {
        return;
    }

    let room = at.x + width - x;
    section_head(ui, theme, x, at.y, "Top Songs");
    let row = Rect::from_min_size(pos2(x, rows_top), vec2(room, SONG_ROWS as f32 * SONG_ROW));
    let columns = artist.top_songs.len().div_ceil(SONG_ROWS);
    let stride = SONG_COLUMN + SONG_GAP;
    let page = ((room + SONG_GAP) / stride).floor().max(1.0) * stride;
    let id = Id::unique(("artist-songs", &artist.artist.id.id));
    let offset = shelf_scroll(
        ui,
        theme,
        id,
        pos2(at.x + width, at.y + 11.0),
        row,
        columns as f32 * stride - SONG_GAP,
        page,
    );

    let mut row_ui = ui.new_child(egui::UiBuilder::new().max_rect(row));
    row_ui.set_clip_rect(row.intersect(ui.clip_rect()));
    let mut play = None;
    for (i, track) in artist.top_songs.iter().enumerate() {
        let (column, line) = (i / SONG_ROWS, i % SONG_ROWS);
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
        let room = cell.right() - 8.0 - tx;
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
        let sub = track
            .album
            .clone()
            .unwrap_or_else(|| track.artists.join(", "));
        let g = text(&row_ui, &sub, font(12.0, Weight::Regular), muted, room);
        paint_text(
            &row_ui,
            g,
            tx,
            cell.center().y + 9.0,
            egui::Align::Min,
            muted,
        );
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            play = Some(i);
        }
    }
    if let Some(i) = play {
        app.play_list(artist.top_songs.clone(), i);
    }
}
