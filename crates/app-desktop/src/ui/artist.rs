//! An artist, laid out as Cider does: the round photo, the name and the play
//! buttons centred over the photo's colours; the latest release beside the top
//! songs in sideways columns; then shelves of albums and of singles and EPs.

use egui::{Id, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{Album, ArtistDetail, FavouriteKind, ItemId};

use crate::app::{App, Loaded, Page};
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::detail::cover_wash;
use super::grid::{
    CARD_TEXT, CardClick, SHELF_HEAD, SONG_COLUMNS_HEIGHT, ShelfCard, cover_shelf, song_columns,
};
use super::search::{message, section_head};
use super::{artwork, format_date, paint_icon, paint_text, text};

const PHOTO_TOP: f32 = 56.0;
const PHOTO: f32 = 220.0;
/// From the top of the page to the first section.
const HERO: f32 = PHOTO_TOP + PHOTO + 20.0 + 58.0 + 16.0 + 56.0 + 40.0;
const SECTION_GAP: f32 = 36.0;
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
        total += SHELF_HEAD + SONG_COLUMNS_HEIGHT + SECTION_GAP;
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
            let favourite = app.is_favourite(FavouriteKind::Artist, &artist.artist.id);
            let buttons_at = pos2(center, name_y + 29.0 + 16.0 + 28.0);
            match hero_buttons(ui, &theme, buttons_at, favourite) {
                Some(HeroButton::Play) => {
                    app.play_shuffled_or_not(artist.top_songs.clone(), false, ui.input(|i| i.time));
                }
                Some(HeroButton::Shuffle) => {
                    app.play_shuffled_or_not(artist.top_songs.clone(), true, ui.input(|i| i.time));
                }
                Some(HeroButton::Favourite) => {
                    app.toggle_favourite(FavouriteKind::Artist, artist.artist.id.clone());
                }
                None => {}
            }

            if top_songs {
                songs_section(app, ui, &theme, artist, pos2(left, y), width);
                y += SHELF_HEAD + SONG_COLUMNS_HEIGHT + SECTION_GAP;
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
                if let Some((i, click)) = cover_shelf(
                    ui,
                    &theme,
                    pos2(left, y),
                    width,
                    title,
                    &cards,
                    SHELF_CARD,
                    shelf_id,
                ) {
                    let id = albums[i].id.clone();
                    match click {
                        CardClick::Open => app.go(Page::Album(id)),
                        CardClick::Enqueue(how) => {
                            app.enqueue_album(id, how, ui.input(|i| i.time));
                        }
                    }
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

/// What a round button under the name asked for.
#[derive(Clone, Copy, PartialEq)]
enum HeroButton {
    Play,
    Shuffle,
    Favourite,
}

/// Shuffle, the big round Play centred on `center`, and the favourite star.
fn hero_buttons(
    ui: &mut Ui,
    theme: &Theme,
    center: egui::Pos2,
    favourite: bool,
) -> Option<HeroButton> {
    let (big, small, gap) = (28.0, 20.0, 16.0);
    let side = big + gap + small;
    let star = if favourite {
        icons::STAR_FILL
    } else {
        icons::STAR
    };
    let buttons = [
        (
            center - vec2(side, 0.0),
            small,
            icons::SHUFFLE,
            17.0,
            HeroButton::Shuffle,
            "Shuffle",
        ),
        (
            center,
            big,
            icons::PLAY_FILL,
            22.0,
            HeroButton::Play,
            "Play",
        ),
        (
            center + vec2(side, 0.0),
            small,
            star,
            17.0,
            HeroButton::Favourite,
            if favourite {
                "Remove from Favourites"
            } else {
                "Favourite"
            },
        ),
    ];
    let mut clicked = None;
    for (c, radius, icon, size, button, label) in buttons {
        let rect = Rect::from_center_size(c, vec2(radius * 2.0, radius * 2.0));
        let response = ui.interact(rect, Id::unique(("artist-hero", label)), Sense::click());
        let fill = theme.text_a(if response.hovered() { 0.16 } else { 0.10 });
        ui.painter().circle_filled(c, radius, fill);
        let color = if button == HeroButton::Favourite && favourite {
            theme.red
        } else {
            theme.text
        };
        // The triangle is drawn off-centre in its icon already, so that it
        // looks centred.
        paint_icon(ui, icon, c, size, color);
        if response
            .on_hover_text(label)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            clicked = Some(button);
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
            pos2(x, rows_top + (SONG_COLUMNS_HEIGHT - LATEST_ART) / 2.0),
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
    let row = Rect::from_min_size(pos2(x, rows_top), vec2(room, SONG_COLUMNS_HEIGHT));
    let id = Id::unique(("artist-songs", &artist.artist.id.id));
    song_columns(
        app,
        ui,
        theme,
        row,
        pos2(at.x + width, at.y + 11.0),
        &artist.top_songs,
        id,
        true,
    );
}
