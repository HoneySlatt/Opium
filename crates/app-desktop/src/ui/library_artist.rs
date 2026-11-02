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
    let expressive = theme.layout.header == crate::theme::Header::Expressive;
    let here = app.page_playing();
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
    } else if expressive {
        EXPRESSIVE_TOP
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
            } else if expressive {
                // `2 albums · 1 single or EP`.
                let counts = sections
                    .iter()
                    .map(|(title, a)| {
                        let n = a.len();
                        let what = match (*title == "Albums", n == 1) {
                            (true, true) => "album",
                            (true, false) => "albums",
                            (false, true) => "single or EP",
                            (false, false) => "singles and EPs",
                        };
                        format!("{n} {what}")
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                let top = Rect::from_min_size(content.min, vec2(content.width(), EXPRESSIVE_TOP));
                expressive_header(
                    ui,
                    &theme,
                    &artist,
                    top,
                    pad,
                    &counts,
                    here,
                    &mut play_page,
                    &mut profile,
                    &mut play,
                );
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
                    super::loading::shape(ui, &theme, grid.cell(i, left, top), 8.0);
                }
                super::loading::spinner(ui, &theme, pos2(left + width / 2.0, top + 60.0));
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
                        facts: app.album_facts(album),
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
        Some((album, CardClick::Menu(choice))) => {
            app.collection_choice(&Collection::Album(album), choice, now);
        }
        None => {}
    }
    output.state.clamped_offset().y
}

/// From the top of M3E's page to its first section: 48 px, the picture,
/// 28 px, the buttons' row.
const EXPRESSIVE_TOP: f32 = 48.0 + 248.0 + 28.0 + 100.0;

/// M3E's header, as the artist page's: their picture in the Sunny shape,
/// the eyebrow chip, the name in display type (to their page on the
/// service) over what the library has of them; the shaped Play, Shuffle and
/// their page.
#[allow(clippy::too_many_arguments)]
fn expressive_header(
    ui: &mut Ui,
    theme: &crate::theme::Theme,
    artist: &Option<music_core::model::Artist>,
    top: Rect,
    pad: f32,
    counts: &str,
    here: bool,
    play_page: &mut bool,
    profile: &mut bool,
    play: &mut Option<bool>,
) {
    use super::shapes::Shape;
    glow(
        ui,
        theme,
        Rect::from_min_size(top.min, vec2(top.width(), 460.0)),
    );
    let art = Rect::from_min_size(pos2(top.left() + pad, top.top() + 48.0), vec2(248.0, 248.0));
    let sunny = Shape::Sunny.radii();
    match artist
        .as_ref()
        .and_then(|a| a.artwork.as_ref())
        .map(|a| a.url(500))
    {
        Some(url) => sunny.image(ui, &url, art, 0.0, 0.0, theme.surface0),
        None => {
            sunny.fill(ui, art, 0.0, theme.surface0);
            paint_icon(ui, icons::MIC, art.center(), 80.0, theme.muted());
        }
    }
    let x = art.right() + 32.0;
    let room = (top.right() - pad - x).max(0.0);
    let mut y = art.bottom() - 10.0;
    if !counts.is_empty() {
        let g = text(
            ui,
            counts,
            font(14.0, Weight::Regular),
            theme.subtext0,
            room,
        );
        paint_text(ui, g, x, y, egui::Align::Min, theme.subtext0);
        y -= 10.0 + 12.0;
    } else {
        y += 10.0;
    }
    let name = artist.as_ref().map_or("", |a| a.name.as_str());
    let largest = (ui.ctx().content_rect().width() * 0.052).clamp(40.0, 72.0);
    let name_font = [largest, 56.0, 48.0, 40.0, 32.0]
        .into_iter()
        .filter(|s| *s <= largest)
        .map(|s| font(s, Weight::Bold))
        .find(|f| {
            text(ui, name, f.clone(), theme.text, f32::INFINITY)
                .size()
                .x
                <= room
        })
        .unwrap_or(font(32.0, Weight::Bold));
    let g = text(ui, name, name_font, theme.text, room);
    let size = g.size();
    let link = Rect::from_min_size(pos2(x, y - size.y), size);
    let hit = ui.interact(link, Id::unique("m3e-library-artist-name"), Sense::click());
    if hit.hovered() {
        ui.painter().hline(
            link.x_range(),
            link.bottom() - 6.0,
            egui::Stroke::new(2.0, theme.text),
        );
    }
    paint_text(ui, g, x, y - size.y / 2.0, egui::Align::Min, theme.text);
    if hit
        .on_hover_text("Artist page")
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        *profile = true;
    }
    y -= size.y + 14.0;
    let logo = artist.as_ref().map(|a| icons::service(a.id.provider));
    let g = text(
        ui,
        "Artist · In your library",
        font(12.5, Weight::SemiBold),
        theme.text,
        room,
    );
    let logo_w = if logo.is_some() { 22.0 } else { 0.0 };
    let chip = Rect::from_min_size(
        pos2(x, y - 28.0),
        vec2(12.0 + logo_w + g.size().x + 12.0, 28.0),
    );
    ui.painter()
        .rect_filled(chip, 14.0, crate::color::alpha(theme.base, 0.55));
    if let Some(logo) = logo {
        paint_icon(
            ui,
            logo,
            pos2(chip.left() + 12.0 + 7.0, chip.center().y),
            14.0,
            theme.text,
        );
    }
    paint_text(
        ui,
        g,
        chip.left() + 12.0 + logo_w,
        chip.center().y,
        egui::Align::Min,
        theme.text,
    );

    let mid = art.bottom() + 28.0 + 4.0 + 36.0;
    let fab = Rect::from_min_size(pos2(top.left() + pad, mid - 36.0), vec2(72.0, 72.0));
    if super::fab::fab(ui, theme, fab, "library-artist-play", here, false, true).clicked() {
        if here {
            *play_page = true;
        } else {
            *play = Some(false);
        }
    }
    let shuffle = super::fab::tonal(
        ui,
        theme,
        pos2(fab.right() + 12.0, mid),
        "library-artist-shuffle",
        icons::SHUFFLE,
        "Shuffle",
    );
    if shuffle.clicked() {
        *play = Some(true);
    }
    let page = Rect::from_min_size(
        pos2(shuffle.rect.right() + 12.0, mid - 28.0),
        vec2(56.0, 56.0),
    );
    if super::fab::round(
        ui,
        theme,
        page,
        "library-artist-page",
        icons::USER,
        24.0,
        theme.subtext0,
    )
    .on_hover_text("Artist page")
    .clicked()
    {
        *profile = true;
    }
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
