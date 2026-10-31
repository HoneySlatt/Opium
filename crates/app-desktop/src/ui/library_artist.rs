//! An artist as the library has them, as Apple Music's app shows one: the
//! photo and name, Play and Shuffle over their songs in the library, then
//! their albums and their singles and EPs there, newest first.

use egui::{Id, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{Album, Collection, ItemId};

use crate::app::{App, Loaded, Page};
use crate::fonts::{Weight, font};
use crate::icons;

use super::grid::{CARD_TEXT, CardClick, CoverCard, Grid, cover_card};
use super::search::{message, section_head};
use super::{PageButton, artwork, format_date, glow, page_buttons, paint_icon, paint_text, text};

const PHOTO_TOP: f32 = 60.0;
const PHOTO: f32 = 200.0;
/// From the top of the page to the first section.
const BODY_TOP: f32 = PHOTO_TOP + PHOTO + 48.0;
const SECTION_HEAD: f32 = 40.0;
const SECTION_GAP: f32 = 36.0;

pub fn show(app: &mut App, ui: &mut Ui, pad: f32, id: &ItemId) -> f32 {
    let theme = app.theme.clone();
    // Their releases on every service the library has them on, one copy
    // of each: what has come, while the rest comes.
    let mut gathered: Option<Vec<Album>> = None;
    let mut error = None;
    for twin in app.artist_twins(id) {
        match app.artist_albums.get(&twin) {
            Some(Loaded::Ready(albums)) => {
                gathered
                    .get_or_insert_with(Vec::new)
                    .extend(albums.iter().cloned());
            }
            Some(Loaded::Failed(e)) => error = Some(e.clone()),
            _ => {}
        }
    }
    let albums = gathered.map(|all| {
        let copies = music_core::dedupe::albums(&all);
        let kept: Vec<Album> = all
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !copies.contains(i))
            .map(|(_, a)| a)
            .collect();
        std::sync::Arc::new(kept)
    });
    // A failure shows only when nothing came.
    let error = error.filter(|_| albums.is_none());
    let artist = app.library_artist(id).cloned();
    // Singles and EPs apart, as Apple names them; each newest first.
    let (mut long, mut short): (Vec<Album>, Vec<Album>) = albums
        .as_deref()
        .map(|a| a.to_vec())
        .unwrap_or_default()
        .into_iter()
        .partition(|a| !(a.title.ends_with(" - Single") || a.title.ends_with(" - EP")));
    let newest = |a: &Album, b: &Album| b.released.cmp(&a.released);
    long.sort_by(newest);
    short.sort_by(newest);
    // Spotify's: their picture across the top under their name, Play,
    // Shuffle and a pill to their page under it.
    let spotify = theme.layout.header == crate::theme::Header::Banner;
    let singles = if spotify {
        "Singles and EPs"
    } else {
        "Singles & EPs"
    };
    let sections: Vec<(&str, Vec<Album>)> = [("Albums", long), (singles, short)]
        .into_iter()
        .filter(|(_, a)| !a.is_empty())
        .collect();

    let width = ui.available_width() - 2.0 * pad;
    let grid = Grid::new(width);
    let hero = (ui.available_width() * 0.3).clamp(340.0, 500.0);
    let body_top = if spotify {
        hero + super::artist::BANNER_ACTIONS
    } else {
        BODY_TOP
    };
    let mut total = body_top + 104.0;
    for (_, albums) in &sections {
        total += SECTION_HEAD + grid.height(albums.len()) + SECTION_GAP;
    }
    if albums.is_none() {
        total += SECTION_HEAD + grid.height(grid.columns());
    }

    let mut clicked: Option<(Album, CardClick)> = None;
    let mut play: Option<bool> = None;
    let mut play_page = false;
    let mut profile = false;
    let output = egui::ScrollArea::vertical()
        .id_salt(("library-artist", &id.id))
        .auto_shrink(false)
        .show(ui, |ui| {
            let (content, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
            let left = content.left() + pad;
            if spotify {
                let top = Rect::from_min_size(content.min, vec2(content.width(), hero));
                super::artist::banner_fade(ui, &theme, theme.surface1, top);
                let url = artist
                    .as_ref()
                    .and_then(|a| a.artwork.as_ref())
                    .map(|a| a.url(1600));
                let texture = url.and_then(|url| {
                    crate::covers::seen(ui.ctx(), &url);
                    match egui::Image::new(url).load_for_size(ui.ctx(), top.size()) {
                        Ok(egui::load::TexturePoll::Ready { texture }) => Some(texture),
                        _ => None,
                    }
                });
                super::artist::paint_picture(ui, &theme, texture, true, top);
                let name = artist.as_ref().map_or("", |a| a.name.as_str());
                super::artist::banner_name(ui, name, left, width, top.bottom());
                let (p, pill) = super::artist::banner_actions(
                    app,
                    ui,
                    &theme,
                    left,
                    top.bottom(),
                    "Artist page",
                    "library-artist",
                );
                play_page = p;
                profile = pill;
            } else {
                glow(
                    ui,
                    &theme,
                    Rect::from_min_size(content.min, vec2(content.width(), 460.0)),
                );
                header(
                    ui,
                    &theme,
                    &artist,
                    id,
                    left,
                    width,
                    content.top(),
                    &mut profile,
                    &mut play,
                );
            }
            let mut y = content.top() + body_top;
            if let Some(e) = &error {
                message(ui, &theme, left, y, width, "Couldn't load this artist", e);
                return;
            }
            if albums.is_none() {
                section_head(ui, &theme, left, y, "Albums");
                let top = y + SECTION_HEAD;
                for i in 0..grid.columns() {
                    ui.painter()
                        .rect_filled(grid.cell(i, left, top), 8.0, theme.hover());
                }
                return;
            }
            for (title, albums) in &sections {
                section_head(ui, &theme, left, y, title);
                let top = y + SECTION_HEAD;
                for (i, album) in albums.iter().enumerate() {
                    let art = grid.cell(i, left, top);
                    if !ui.is_rect_visible(art.expand2(vec2(0.0, CARD_TEXT))) {
                        continue;
                    }
                    let sub = if spotify {
                        release_line(album)
                    } else {
                        album
                            .released
                            .as_deref()
                            .and_then(format_date)
                            .or_else(|| album.year.map(|y| y.to_string()))
                            .unwrap_or_default()
                    };
                    let card = CoverCard {
                        url: album.artwork.as_ref().map(|a| a.url(376)),
                        title: &album.title,
                        sub: &sub,
                        round: false,
                        explicit: album.explicit,
                        addable: false,
                        menu: true,
                    };
                    let card_id = Id::unique(("library-artist-card", *title, i));
                    if let Some(click) = cover_card(ui, &theme, art, &card, card_id) {
                        clicked = Some((album.clone(), click));
                    }
                }
                y = top + grid.height(albums.len()) + SECTION_GAP;
            }
        });

    let now = ui.input(|i| i.time);
    if play_page || play.is_some() {
        // Every song of theirs in the library, album by album.
        let all: Vec<Album> = sections.iter().flat_map(|(_, a)| a.clone()).collect();
        let songs = app.songs_of(&all);
        if songs.is_empty() {
            app.notify("Their songs come with the library, still loading", now);
        } else if play_page {
            app.play_page(songs, now);
        } else if let Some(shuffle) = play {
            app.play_shuffled_or_not(songs, shuffle, now);
        }
    }
    if profile {
        // By the catalog's id when the library knows it.
        let catalog = artist.as_ref().map_or_else(|| id.clone(), |a| a.id.clone());
        app.go(Page::Artist(catalog));
    }
    match clicked {
        Some((album, CardClick::Open)) => app.go(Page::Album(album.id)),
        Some((album, CardClick::Enqueue(how))) => app.enqueue_album(album.id, how, now),
        Some((album, CardClick::AddToLibrary)) => {
            app.add_collection_to_library(&Collection::Album(album), now);
        }
        None => {}
    }
    output.state.clamped_offset().y
}

/// Spotify's line under a release: `2023 • Album`, or Single, or EP.
fn release_line(album: &Album) -> String {
    let kind = if album.title.ends_with(" - Single") {
        "Single"
    } else if album.title.ends_with(" - EP") {
        "EP"
    } else {
        "Album"
    };
    album
        .year
        .map_or(kind.to_string(), |y| format!("{y} • {kind}"))
}

/// The photo, round, and beside it the name, leading to their page on
/// Apple Music, and Play and Shuffle, as Apple Music's app has them.
#[allow(clippy::too_many_arguments)]
fn header(
    ui: &mut Ui,
    theme: &crate::theme::Theme,
    artist: &Option<music_core::model::Artist>,
    id: &ItemId,
    left: f32,
    width: f32,
    top: f32,
    profile: &mut bool,
    play: &mut Option<bool>,
) {
    let photo = Rect::from_min_size(pos2(left, top + PHOTO_TOP), vec2(PHOTO, PHOTO));
    let url = artist
        .as_ref()
        .and_then(|a| a.artwork.as_ref())
        .map(|a| a.url(400));
    let missing = url.is_none();
    artwork(ui, url, photo, PHOTO / 2.0, theme);
    if missing {
        paint_icon(ui, icons::MIC, photo.center(), PHOTO / 3.0, theme.muted());
    }
    let x = photo.right() + 32.0;
    let name = artist.as_ref().map_or("", |a| a.name.as_str());
    let g = text(
        ui,
        name,
        font(40.0, Weight::Bold),
        theme.text,
        width - (x - left),
    );
    let mid = photo.center().y;
    // The name leads to the artist's whole page on Apple Music.
    let size = g.size();
    let link = Rect::from_min_size(pos2(x, mid - 46.0 - size.y / 2.0), size);
    let hit = ui.interact(link, Id::unique(("artist-profile", &id.id)), Sense::click());
    if hit.hovered() {
        ui.painter().hline(
            link.x_range(),
            link.bottom() - 4.0,
            egui::Stroke::new(2.0, theme.text),
        );
    }
    paint_text(ui, g, x, mid - 46.0, egui::Align::Min, theme.text);
    if hit
        .on_hover_text("Apple Music profile")
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        *profile = true;
    }
    if let Some(button) = page_buttons(ui, theme, pos2(x, mid - 10.0), "library-artist", false) {
        *play = Some(button == PageButton::Shuffle);
    }
}
