//! Home: the shelves Apple Music recommends, as its web player and Cider show
//! them. The playlists made for the user come first as tall cards on their
//! artwork's colour; every other shelf is a row of covers.

use egui::{Color32, Id, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{Collection, Shelf};

use crate::app::{App, Loaded, Page};
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::grid::{CARD_TEXT, CardClick, SHELF_HEAD, ShelfCard, cover_shelf, shelf_scroll};
use super::search::{message, section_head};
use super::{artwork, glow, icon_button, paint_text, text};

const TITLE_Y: f32 = 66.0;
const BODY_TOP: f32 = 104.0;
const SECTION_GAP: f32 = 34.0;
const COVER: f32 = 190.0;
const HERO: f32 = 240.0;
/// The coloured foot under a hero card's artwork.
const HERO_FOOT: f32 = 78.0;
const HERO_GAP: f32 = 20.0;

pub fn show(app: &mut App, ui: &mut Ui, pad: f32) -> f32 {
    let theme = app.theme.clone();
    let shelves = match &app.home {
        Some(Loaded::Ready(shelves)) => Ok(Some(shelves.clone())),
        Some(Loaded::Failed(e)) => Err(e.clone()),
        _ => Ok(None),
    };
    let width = ui.available_width() - 2.0 * pad;
    let height = |shelf: &Shelf| {
        SHELF_HEAD
            + if shelf.hero {
                HERO + HERO_FOOT
            } else {
                COVER + CARD_TEXT
            }
            + SECTION_GAP
    };
    let total = BODY_TOP
        + match &shelves {
            Ok(Some(s)) => s.iter().map(height).sum::<f32>(),
            _ => 200.0,
        }
        + 104.0;

    let mut refresh = false;
    let output = egui::ScrollArea::vertical()
        .id_salt("home")
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
            let g = text(ui, "Home", font(22.0, Weight::Bold), theme.text, width);
            paint_text(
                ui,
                g,
                left,
                content.top() + TITLE_Y,
                egui::Align::Min,
                theme.text,
            );
            let button = Rect::from_center_size(
                pos2(left + width - 16.0, content.top() + TITLE_Y),
                vec2(32.0, 32.0),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(button));
            if icon_button(
                &mut child,
                &theme,
                icons::REFRESH,
                32.0,
                16.0,
                theme.subtext0,
                true,
            )
            .on_hover_text("Refresh")
            .clicked()
            {
                refresh = true;
            }

            let mut y = content.top() + BODY_TOP;
            let shelves = match &shelves {
                Err(e) => {
                    message(ui, &theme, left, y, width, "Couldn't load Home", e);
                    return;
                }
                Ok(None) => {
                    for i in 0..5 {
                        let card = Rect::from_min_size(
                            pos2(left + i as f32 * (HERO + HERO_GAP), y + SHELF_HEAD),
                            vec2(HERO, HERO + HERO_FOOT),
                        );
                        if card.right() <= left + width {
                            ui.painter().rect_filled(card, 12.0, theme.hover());
                        }
                    }
                    return;
                }
                Ok(Some(shelves)) => shelves,
            };
            if shelves.is_empty() {
                let detail =
                    "Apple Music has nothing to suggest yet. Listen a little, then refresh.";
                message(ui, &theme, left, y, width, "Nothing here yet", detail);
                return;
            }
            for (n, shelf) in shelves.iter().enumerate() {
                let h = height(shelf);
                // Shelves off screen are skipped, so their covers do not load yet.
                let visible = (y - content.top())..(y - content.top() + h);
                if visible.end > viewport.top() && visible.start < viewport.bottom() {
                    let at = pos2(left, y);
                    let id = Id::unique(("home-shelf", n));
                    let clicked = if shelf.hero {
                        hero_shelf(ui, &theme, at, width, shelf, id)
                    } else {
                        let cards: Vec<ShelfCard> = shelf.items.iter().map(card).collect();
                        cover_shelf(ui, &theme, at, width, &shelf.title, &cards, COVER, id)
                    };
                    if let Some((i, click)) = clicked {
                        let item = &shelf.items[i];
                        let now = ui.input(|i| i.time);
                        match (click, item) {
                            (CardClick::Open, _) => open(app, item),
                            (CardClick::Enqueue(how), Collection::Album(a)) => {
                                app.enqueue_album(a.id.clone(), how, now);
                            }
                            (CardClick::Enqueue(how), Collection::Playlist(p)) => {
                                app.enqueue_playlist(p, how, now);
                            }
                        }
                    }
                }
                y += h;
            }
        });
    if refresh {
        app.refresh_home();
    }
    output.state.clamped_offset().y
}

fn open(app: &mut App, item: &Collection) {
    match item {
        Collection::Album(a) => app.go(Page::Album(a.id.clone())),
        Collection::Playlist(p) => app.open_playlist(p),
    }
}

fn card(item: &Collection) -> ShelfCard {
    match item {
        Collection::Album(a) => ShelfCard {
            title: a.title.clone(),
            sub: a.artists.join(", "),
            url: a.artwork.as_ref().map(|art| art.url(400)),
        },
        Collection::Playlist(p) => ShelfCard {
            title: p.name.clone(),
            sub: p.curator.clone().unwrap_or_else(|| "Playlist".into()),
            url: p.artwork.as_ref().map(|art| art.url(400)),
        },
    }
}

/// Tall cards: the artwork, then a foot in its colour with the name and the
/// description, the art fading into it.
fn hero_shelf(
    ui: &mut Ui,
    theme: &Theme,
    at: egui::Pos2,
    width: f32,
    shelf: &Shelf,
    id: Id,
) -> Option<(usize, CardClick)> {
    section_head(ui, theme, at.x, at.y, &shelf.title);
    let row = Rect::from_min_size(pos2(at.x, at.y + SHELF_HEAD), vec2(width, HERO + HERO_FOOT));
    let stride = HERO + HERO_GAP;
    let content = shelf.items.len() as f32 * stride - HERO_GAP;
    let page = ((width + HERO_GAP) / stride).floor().max(1.0) * stride;
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
    for (i, item) in shelf.items.iter().enumerate() {
        let x = row.left() + i as f32 * stride - offset;
        if x + HERO < row.left() || x > row.right() {
            continue;
        }
        let rect = Rect::from_min_size(pos2(x, row.top()), vec2(HERO, HERO + HERO_FOOT));
        let (name, sub, art) = match item {
            Collection::Playlist(p) => (&p.name, p.description.clone(), p.artwork.as_ref()),
            Collection::Album(a) => (&a.title, Some(a.artists.join(", ")), a.artwork.as_ref()),
        };
        let response = row_ui.interact(rect, id.with(i), Sense::click());
        // Lift a little on hover, as Cider does.
        let rect = rect.translate(vec2(0.0, if response.hovered() { -2.0 } else { 0.0 }));
        let foot_color = art
            .and_then(|a| a.background)
            .map_or(theme.surface1, |[r, g, b]| Color32::from_rgb(r, g, b));
        let painter = row_ui.painter();
        painter.rect_filled(rect, 12.0, foot_color);
        let art_rect = Rect::from_min_size(rect.min, vec2(HERO, HERO));
        artwork(&row_ui, art.map(|a| a.url(480)), art_rect, 12.0, theme);
        // The art fades into the foot over its last 44 px.
        let fade = Rect::from_min_max(
            pos2(art_rect.left(), art_rect.bottom() - 44.0),
            art_rect.max,
        );
        let mut mesh = egui::Mesh::default();
        let clear =
            Color32::from_rgba_unmultiplied(foot_color.r(), foot_color.g(), foot_color.b(), 0);
        mesh.colored_vertex(fade.left_top(), clear);
        mesh.colored_vertex(fade.right_top(), clear);
        mesh.colored_vertex(fade.right_bottom(), foot_color);
        mesh.colored_vertex(fade.left_bottom(), foot_color);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(mesh);
        let ink = if crate::color::luminance(foot_color) > 0.5 {
            Color32::from_black_alpha(230)
        } else {
            Color32::WHITE
        };
        let soft = Color32::from_rgba_unmultiplied(ink.r(), ink.g(), ink.b(), 190);
        let tx = rect.left() + 14.0;
        let room = HERO - 28.0;
        let g = text(&row_ui, name, font(15.0, Weight::SemiBold), ink, room);
        paint_text(
            &row_ui,
            g,
            tx,
            art_rect.bottom() + 20.0,
            egui::Align::Min,
            ink,
        );
        if let Some(sub) = sub {
            let g = text(&row_ui, &sub, font(12.0, Weight::Regular), soft, room);
            paint_text(
                &row_ui,
                g,
                tx,
                art_rect.bottom() + 42.0,
                egui::Align::Min,
                soft,
            );
        }
        if let Some(how) = super::menu::collection_menu(&response, theme) {
            clicked = Some((i, CardClick::Enqueue(how)));
        } else if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            clicked = Some((i, CardClick::Open));
        }
    }
    clicked
}
