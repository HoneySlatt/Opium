//! Home and Radio: shelves as Apple Music's web player and Cider show them.
//! On Home, the playlists made for the user come first as tall cards on their
//! artwork's colour; on Radio, the stations on air as large covers. Every
//! other shelf is a row of covers.

use egui::{Color32, Id, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{Collection, Shelf};

use crate::app::{App, CollectionChoice, Loaded, Page, Shelves as Which};
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::grid::{
    CARD_TEXT, CardClick, SHELF_HEAD, ShelfCard, cover_shelf, shelf_scroll, song_columns,
    song_columns_height,
};
use super::menu::CollectionFacts;
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

/// The stations on air, on Radio.
const LIVE: f32 = 230.0;
/// The editors' picks on New: their badge, name and line above a large cover.
const PICK: f32 = 340.0;
/// A pick's wide picture, at Apple's 16:9.
const PICK_HEIGHT: f32 = PICK * 9.0 / 16.0;
const PICK_WORDS: f32 = 64.0;
const PICK_GAP: f32 = 20.0;
/// The best new songs: four to a column, as in Cider.
const NEW_SONG_ROWS: usize = 4;

pub fn show(app: &mut App, ui: &mut Ui, pad: f32) -> f32 {
    page(app, ui, pad, Which::Home)
}

pub fn new_music(app: &mut App, ui: &mut Ui, pad: f32) -> f32 {
    page(app, ui, pad, Which::New)
}

pub fn radio(app: &mut App, ui: &mut Ui, pad: f32) -> f32 {
    page(app, ui, pad, Which::Radio)
}

fn page(app: &mut App, ui: &mut Ui, pad: f32, which: Which) -> f32 {
    let theme = app.theme.clone();
    let loaded = match which {
        Which::Home => &app.home,
        Which::New => &app.new_music,
        Which::Radio => &app.radio,
    };
    let (title, salt) = match which {
        Which::Home => ("Home", "home"),
        Which::New => ("New", "new"),
        Which::Radio => ("Radio", "radio"),
    };
    let mut see_all = None;
    let shelves = match loaded {
        Some(Loaded::Ready(shelves)) => Ok(Some(shelves.clone())),
        Some(Loaded::Failed(e)) => Err(e.clone()),
        _ => Ok(None),
    };
    let width = ui.available_width() - 2.0 * pad;
    let height = |shelf: &Shelf| {
        SHELF_HEAD
            + match (shelf.hero, which) {
                _ if !shelf.songs.is_empty() => song_columns_height(NEW_SONG_ROWS),
                (true, Which::Home) => HERO + HERO_FOOT,
                (true, Which::New) => PICK_WORDS + PICK_HEIGHT,
                (true, Which::Radio) => LIVE + CARD_TEXT,
                (false, _) => COVER + CARD_TEXT,
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
    let mut go = None;
    let output = egui::ScrollArea::vertical()
        .id_salt(salt)
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
            if theme.layout.sidebar == crate::theme::Sidebar::Library {
                // No pages in that sidebar: the three are chips, as
                // Spotify's All, Music and Podcasts.
                if let Some(page) = chips(ui, &theme, pos2(left, content.top() + TITLE_Y), which) {
                    go = Some(page);
                }
            } else {
                let g = text(ui, title, font(22.0, Weight::Bold), theme.text, width);
                paint_text(
                    ui,
                    g,
                    left,
                    content.top() + TITLE_Y,
                    egui::Align::Min,
                    theme.text,
                );
            }
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
                    let head = format!("Couldn't load {title}");
                    message(ui, &theme, left, y, width, &head, e);
                    return;
                }
                Ok(None) => {
                    for i in 0..5 {
                        let card = Rect::from_min_size(
                            pos2(left + i as f32 * (HERO + HERO_GAP), y + SHELF_HEAD),
                            vec2(HERO, HERO + HERO_FOOT),
                        );
                        if card.right() <= left + width {
                            super::loading::shape(ui, &theme, card, 12.0);
                        }
                    }
                    super::loading::spinner(ui, &theme, pos2(left + width / 2.0, y + 120.0));
                    return;
                }
                Ok(Some(shelves)) => shelves,
            };
            if shelves.is_empty() {
                let detail =
                    "Nothing to suggest yet. Listen a little, then refresh.";
                message(ui, &theme, left, y, width, "Nothing here yet", detail);
                return;
            }
            for (n, shelf) in shelves.iter().enumerate() {
                let h = height(shelf);
                // Shelves off screen are skipped, so their covers do not load yet.
                let visible = (y - content.top())..(y - content.top() + h);
                if visible.end > viewport.top() && visible.start < viewport.bottom() {
                    let at = pos2(left, y);
                    let id = Id::unique(("shelf", salt, n));
                    let facts: Vec<CollectionFacts> = shelf
                        .items
                        .iter()
                        .map(|c| app.collection_facts(c))
                        .collect();
                    let cards = || -> Vec<ShelfCard> {
                        shelf
                            .items
                            .iter()
                            .zip(&facts)
                            .map(|(c, &facts)| ShelfCard { facts, ..card(c) })
                            .collect()
                    };
                    let clicked = match (shelf.hero, which) {
                        _ if !shelf.songs.is_empty() => {
                            section_head(ui, &theme, at.x, at.y, &shelf.title);
                            let row = Rect::from_min_size(
                                pos2(at.x, at.y + SHELF_HEAD),
                                vec2(width, song_columns_height(NEW_SONG_ROWS)),
                            );
                            let arrows = pos2(at.x + width, at.y + 11.0);
                            song_columns(
                                app,
                                ui,
                                &theme,
                                row,
                                arrows,
                                &shelf.songs,
                                id,
                                false,
                                NEW_SONG_ROWS,
                            );
                            None
                        }
                        (true, Which::Home)
                            if theme.layout.cards == crate::theme::Cards::Shapes =>
                        {
                            carousel(ui, &theme, at, width, shelf, &facts, id)
                        }
                        (true, Which::Home) => hero_shelf(ui, &theme, at, width, shelf, &facts, id),
                        (true, Which::New) => picks_shelf(ui, &theme, at, width, shelf, &facts, id),
                        (true, Which::Radio) => {
                            if more(ui, &theme, at, width, shelf, LIVE, id) {
                                see_all = Some(shelf.title.clone());
                            }
                            cover_shelf(ui, &theme, at, width, &shelf.title, &cards(), LIVE, id)
                        }
                        (false, _) => {
                            if more(ui, &theme, at, width, shelf, COVER, id) {
                                see_all = Some(shelf.title.clone());
                            }
                            cover_shelf(ui, &theme, at, width, &shelf.title, &cards(), COVER, id)
                        }
                    };
                    if let Some((i, click)) = clicked {
                        act(app, &shelf.items[i], click, ui.input(|i| i.time));
                    }
                }
                y += h;
            }
        });
    if refresh {
        app.refresh(which);
    }
    if let Some(page) = go {
        app.go(page);
    }
    if let Some(title) = see_all {
        app.go(Page::Shelf(which, title));
    }
    output.state.clamped_offset().y
}

/// Home, New and Radio as chips from `at` (their left, centred on its y), the
/// page shown filled. The one clicked.
fn chips(ui: &Ui, theme: &Theme, at: egui::Pos2, which: Which) -> Option<Page> {
    let mut x = at.x;
    let f = font(14.0, Weight::Regular);
    let mut clicked = None;
    for (kind, label, page) in [
        (Which::Home, "Home", Page::Home),
        (Which::New, "New", Page::New),
        (Which::Radio, "Radio", Page::Radio),
    ] {
        let w = text(ui, label, f.clone(), theme.text, 200.0).size().x;
        let rect = Rect::from_min_size(pos2(x, at.y - 16.0), vec2(w + 24.0, 32.0));
        x = rect.right() + 8.0;
        let response = ui
            .interact(rect, Id::unique(("home-chip", label)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let on = kind == which;
        let (fill, ink) = if on {
            (theme.text, theme.base)
        } else if response.hovered() {
            (theme.surface2, theme.text)
        } else {
            (theme.surface1, theme.text)
        };
        ui.painter().rect_filled(rect, 16.0, fill);
        let g = text(ui, label, f.clone(), ink, 200.0);
        paint_text(
            ui,
            g,
            rect.center().x,
            rect.center().y,
            egui::Align::Center,
            ink,
        );
        if response.clicked() && !on {
            clicked = Some(page);
        }
    }
    clicked
}

/// A chevron after a shelf's title when its covers run past the width, as
/// Apple Music shows it; the title and the chevron lead to the whole shelf.
fn more(
    ui: &mut Ui,
    theme: &Theme,
    at: egui::Pos2,
    width: f32,
    shelf: &Shelf,
    size: f32,
    id: Id,
) -> bool {
    let fits = ((width + super::grid::SHELF_GAP) / (size + super::grid::SHELF_GAP)).floor();
    if shelf.title.is_empty() || shelf.items.len() as f32 <= fits {
        return false;
    }
    let title = text(
        ui,
        &shelf.title,
        super::search::heading(theme),
        theme.text,
        600.0,
    );
    let y = at.y + 11.0;
    let chevron = pos2(at.x + title.size().x + 4.0 + 8.0, y + 1.0);
    let rect = Rect::from_min_max(pos2(at.x, y - 14.0), pos2(chevron.x + 10.0, y + 14.0));
    let response = ui.interact(rect, id.with("see-all"), Sense::click());
    let color = if response.hovered() {
        theme.text
    } else {
        theme.muted()
    };
    super::paint_icon(ui, icons::RIGHT, chevron, 16.0, color);
    response
        .on_hover_text("See All")
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// Do what a click on a shelf's cover asked: open it, queue it, or tune in.
pub(super) fn act(app: &mut App, item: &Collection, click: CardClick, now: f64) {
    match (click, item) {
        (CardClick::Menu(CollectionChoice::Opened), Collection::Station(_)) => {}
        (_, Collection::Station(s)) => app.play_station(s),
        (CardClick::Open, Collection::Album(a)) => app.go(Page::Album(a.id.clone())),
        (CardClick::Open, Collection::Playlist(p)) => app.open_playlist(p),
        (CardClick::Enqueue(how), Collection::Album(a)) => {
            app.enqueue_album(a.id.clone(), how, now);
        }
        (CardClick::Enqueue(how), Collection::Playlist(p)) => app.enqueue_playlist(p, how, now),
        (CardClick::Menu(choice), item) => app.collection_choice(item, choice, now),
    }
}

pub(super) fn card(item: &Collection) -> ShelfCard {
    match item {
        Collection::Album(a) => ShelfCard {
            title: a.title.clone(),
            sub: a.artists.join(", "),
            url: a.artwork.as_ref().map(|art| art.url(400)),
            explicit: a.explicit,
            facts: CollectionFacts::default(),
        },
        Collection::Playlist(p) => ShelfCard {
            title: p.name.clone(),
            sub: p.curator.clone().unwrap_or_else(|| "Playlist".into()),
            url: p.artwork.as_ref().map(|art| art.url(400)),
            explicit: false,
            facts: CollectionFacts::default(),
        },
        Collection::Station(s) => ShelfCard {
            title: s.name.clone(),
            sub: s.description.clone().unwrap_or_else(|| "Station".into()),
            url: s.artwork.as_ref().map(|art| art.url(400)),
            explicit: false,
            facts: CollectionFacts::default(),
        },
    }
}

/// M3E's multi-browse carousel: one large item with its words and a Play,
/// one medium and one small, radius 28. A click on the medium or the small
/// one makes it the large one, the widths moving on the slow spring.
fn carousel(
    ui: &mut Ui,
    theme: &Theme,
    at: egui::Pos2,
    width: f32,
    shelf: &Shelf,
    facts: &[CollectionFacts],
    id: Id,
) -> Option<(usize, CardClick)> {
    use super::motion::{Spring, spring};
    const MEDIUM: f32 = 168.0;
    const SMALL: f32 = 72.0;
    const GAP: f32 = 8.0;
    section_head(ui, theme, at.x, at.y, &shelf.title);
    let count = shelf.items.len();
    let focus_id = id.with("focus");
    let mut focus = ui
        .data(|d| d.get_temp::<usize>(focus_id))
        .unwrap_or(0)
        .min(count.saturating_sub(1));

    // The arrows step through the picks.
    for (k, (icon, step)) in [(icons::LEFT, -1i32), (icons::RIGHT, 1)]
        .into_iter()
        .enumerate()
    {
        let center = pos2(at.x + width - 16.0 - (1 - k) as f32 * 40.0, at.y + 11.0);
        let rect = Rect::from_center_size(center, vec2(32.0, 32.0));
        let enabled = if step < 0 {
            focus > 0
        } else {
            focus + 1 < count
        };
        ui.painter().circle_filled(center, 16.0, theme.surface1);
        let color = if enabled {
            theme.subtext0
        } else {
            crate::color::alpha(theme.subtext0, 0.38)
        };
        let response =
            super::fab::round(ui, theme, rect, &format!("{id:?}-{k}"), icon, 18.0, color);
        if enabled && response.clicked() {
            focus = (focus as i32 + step) as usize;
        }
    }

    let row = Rect::from_min_size(pos2(at.x, at.y + SHELF_HEAD), vec2(width, HERO + HERO_FOOT));
    let shown = (count - focus).min(3);
    let large = width
        - match shown {
            3 => MEDIUM + SMALL + 2.0 * GAP,
            2 => MEDIUM + GAP,
            _ => 0.0,
        };
    let mut clicked = None;
    let mut x = row.left();
    for (i, item) in shelf.items.iter().enumerate() {
        let target = match i.checked_sub(focus) {
            Some(0) => large,
            Some(1) => MEDIUM,
            Some(2) => SMALL,
            _ => 0.0,
        };
        let w = spring(ui, ("carousel", id, i), target, Spring::Slow).max(0.0);
        if w < 1.0 {
            continue;
        }
        let rect = Rect::from_min_size(
            pos2(x, row.top()),
            vec2(w.min(row.right() - x), row.height()),
        );
        x += w + GAP;
        if rect.width() < 1.0 {
            continue;
        }
        let (name, sub, art) = match item {
            Collection::Playlist(p) => (&p.name, p.description.clone(), p.artwork.as_ref()),
            Collection::Album(a) => (&a.title, Some(a.artists.join(", ")), a.artwork.as_ref()),
            Collection::Station(s) => (&s.name, s.description.clone(), s.artwork.as_ref()),
        };
        let kind = match item {
            Collection::Playlist(_) => "Playlist",
            Collection::Album(_) => "Album",
            Collection::Station(_) => "Station",
        };
        let response = ui.interact(rect, id.with(i), Sense::click());
        let big = i == focus;
        cover_fill(ui, theme, art.map(|a| a.url(720)), rect, 28.0);
        // On the large one, its words over a shade rising from the bottom.
        let words = spring(
            ui,
            ("carousel-words", id, i),
            if big { 1.0 } else { 0.0 },
            Spring::Fast,
        )
        .clamp(0.0, 1.0);
        if words > 0.01 {
            let shade = Rect::from_min_max(
                pos2(rect.left(), rect.top() + rect.height() * 0.45),
                rect.max,
            );
            // Its bottom corners rounded as the item's: a convex outline,
            // each point as dark as it is low.
            let r = 28.0;
            let mut outline = vec![shade.left_top(), shade.right_top()];
            for (c, from) in [
                (pos2(shade.right() - r, shade.bottom() - r), 0.0),
                (pos2(shade.left() + r, shade.bottom() - r), 90.0),
            ] {
                for k in 0..=8 {
                    let a = (from + k as f32 * 90.0 / 8.0_f32).to_radians();
                    outline.push(c + vec2(a.cos(), a.sin()) * r);
                }
            }
            let mut mesh = egui::Mesh::default();
            for p in &outline {
                let t = ((p.y - shade.top()) / shade.height()).clamp(0.0, 1.0);
                mesh.colored_vertex(*p, Color32::from_black_alpha((184.0 * words * t) as u8));
            }
            for k in 1..outline.len() as u32 - 1 {
                mesh.add_triangle(0, k, k + 1);
            }
            ui.painter().add(mesh);
            let white = |a: f32| Color32::from_white_alpha((255.0 * a * words) as u8);
            let lift = 10.0 * (1.0 - words);
            let fab = Rect::from_min_size(
                pos2(
                    rect.right() - 28.0 - 72.0,
                    rect.bottom() - 24.0 - 72.0 + lift,
                ),
                vec2(72.0, 72.0),
            );
            let room = (fab.left() - 16.0 - rect.left() - 28.0).max(0.0);
            let tx = rect.left() + 28.0;
            let mut y = rect.bottom() - 24.0 + lift;
            if let Some(sub) = sub.as_deref().and_then(|s| s.lines().next()) {
                let g = text(
                    ui,
                    sub,
                    font(14.0, Weight::Regular),
                    white(0.82),
                    room.min(460.0),
                );
                let h = g.size().y;
                paint_text(ui, g, tx, y - h / 2.0, egui::Align::Min, white(0.82));
                y -= h + 4.0;
            }
            let g = text(ui, name, font(32.0, Weight::Bold), white(1.0), room);
            let h = g.size().y;
            paint_text(ui, g, tx, y - h / 2.0, egui::Align::Min, white(1.0));
            y -= h;
            let g = text(ui, kind, font(13.0, Weight::Medium), white(0.8), room);
            let h = g.size().y;
            paint_text(ui, g, tx, y - h / 2.0, egui::Align::Min, white(0.8));
            if big
                && super::fab::fab(
                    ui,
                    theme,
                    fab,
                    &format!("{id:?}-play-{i}"),
                    false,
                    false,
                    true,
                )
                .clicked()
            {
                clicked = Some((i, CardClick::Enqueue(crate::app::Enqueue::Now)));
            }
        }
        let shown = facts.get(i).copied().unwrap_or_default();
        if let Some(choice) = super::menu::collection_menu(&response, theme, shown) {
            clicked = Some((i, CardClick::Menu(choice)));
        } else if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
            && clicked.is_none()
        {
            if big {
                clicked = Some((i, CardClick::Open));
            } else {
                focus = i;
            }
        }
    }
    ui.data_mut(|d| d.insert_temp(focus_id, focus));
    clicked
}

/// The picture at `url` filling `rect` as a cover does, cropped to its
/// shape, with rounded corners.
fn cover_fill(ui: &Ui, theme: &Theme, url: Option<String>, rect: Rect, radius: f32) {
    let texture = url.as_ref().and_then(|url| {
        crate::covers::seen(ui.ctx(), url);
        let side = (rect.width().max(rect.height()) * ui.pixels_per_point()).round() as u32;
        let size = egui::load::SizeHint::Size {
            width: side,
            height: side,
            maintain_aspect_ratio: true,
        };
        match ui
            .ctx()
            .try_load_texture(url, egui::TextureOptions::LINEAR, size)
        {
            Ok(egui::load::TexturePoll::Ready { texture }) => Some(texture),
            _ => None,
        }
    });
    let Some(texture) = texture else {
        ui.painter().rect_filled(rect, radius, theme.surface0);
        return;
    };
    // The square cover's middle, as wide as the rect asks.
    let aspect = rect.width() / rect.height();
    let uv = if aspect < 1.0 {
        let w = aspect;
        Rect::from_min_max(pos2(0.5 - w / 2.0, 0.0), pos2(0.5 + w / 2.0, 1.0))
    } else {
        let h = 1.0 / aspect;
        Rect::from_min_max(pos2(0.0, 0.5 - h / 2.0), pos2(1.0, 0.5 + h / 2.0))
    };
    ui.painter().add(
        egui::epaint::RectShape::filled(rect, radius, Color32::WHITE).with_texture(texture.id, uv),
    );
}

/// Tall cards: the artwork, then a foot in its colour with the name and the
/// description, the art fading into it.
fn hero_shelf(
    ui: &mut Ui,
    theme: &Theme,
    at: egui::Pos2,
    width: f32,
    shelf: &Shelf,
    facts: &[CollectionFacts],
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
            Collection::Station(s) => (&s.name, s.description.clone(), s.artwork.as_ref()),
        };
        let response = row_ui.interact(rect, id.with(i), Sense::click());
        // Lift a little on hover, as Cider does.
        let rect = rect.translate(vec2(0.0, if response.hovered() { -2.0 } else { 0.0 }));
        let foot_color = art
            .and_then(|a| a.background)
            .map_or(theme.surface1, |rgb| {
                Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
            });
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
        let shown = facts.get(i).copied().unwrap_or_default();
        if let Some(choice) = super::menu::collection_menu(&response, theme, shown) {
            clicked = Some((i, CardClick::Menu(choice)));
        } else if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            clicked = Some((i, CardClick::Open));
        }
    }
    clicked
}

/// The editors' picks, as Apple's New page and Cider lay them out: what it
/// is (`NEW ALBUM`), its name and who made it, above a wide picture with the
/// editors' line along its foot and the cover in its corner; sideways.
fn picks_shelf(
    ui: &mut Ui,
    theme: &Theme,
    at: egui::Pos2,
    width: f32,
    shelf: &Shelf,
    facts: &[CollectionFacts],
    id: Id,
) -> Option<(usize, CardClick)> {
    let row = Rect::from_min_size(
        pos2(at.x, at.y + SHELF_HEAD),
        vec2(width, PICK_WORDS + PICK_HEIGHT),
    );
    let stride = PICK + PICK_GAP;
    let content = shelf.items.len() as f32 * stride - PICK_GAP;
    let page = ((width + PICK_GAP) / stride).floor().max(1.0) * stride;
    // No title: the arrows alone on its line.
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
        if x + PICK < row.left() || x > row.right() {
            continue;
        }
        let frame = Rect::from_min_size(pos2(x, row.top()), vec2(PICK, PICK_WORDS + PICK_HEIGHT));
        let words = card(item);
        let pick = shelf.picks.get(i).cloned().unwrap_or_default();
        let muted = theme.muted();
        let g = text(
            &row_ui,
            &pick.badge,
            font(10.5, Weight::SemiBold),
            muted,
            PICK,
        );
        paint_text(&row_ui, g, x, frame.top() + 8.0, egui::Align::Min, muted);
        let g = text(
            &row_ui,
            &words.title,
            font(14.5, Weight::Medium),
            theme.text,
            PICK,
        );
        paint_text(
            &row_ui,
            g,
            x,
            frame.top() + 27.0,
            egui::Align::Min,
            theme.text,
        );
        let g = text(
            &row_ui,
            &words.sub,
            font(13.5, Weight::Regular),
            theme.subtext0,
            PICK,
        );
        paint_text(
            &row_ui,
            g,
            x,
            frame.top() + 46.0,
            egui::Align::Min,
            theme.subtext0,
        );

        let art = Rect::from_min_size(pos2(x, frame.top() + PICK_WORDS), vec2(PICK, PICK_HEIGHT));
        let response = row_ui.interact(frame, id.with(i), Sense::click());
        match &pick.wide {
            Some(wide) => artwork(&row_ui, Some(wide.url(0)), art, 10.0, theme),
            None => artwork(&row_ui, words.url.clone(), art, 10.0, theme),
        }
        // The editors' line over a shade at the picture's foot.
        if let Some(blurb) = &pick.blurb {
            let foot = Rect::from_min_max(pos2(art.left(), art.bottom() - 70.0), art.max);
            let mut mesh = egui::Mesh::default();
            let clear = Color32::from_black_alpha(0);
            let shade = Color32::from_black_alpha(150);
            mesh.colored_vertex(foot.left_top(), clear);
            mesh.colored_vertex(foot.right_top(), clear);
            mesh.colored_vertex(foot.right_bottom(), shade);
            mesh.colored_vertex(foot.left_bottom(), shade);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            row_ui.painter().add(mesh);
            let room = PICK - 24.0 - if pick.wide.is_some() { 80.0 } else { 0.0 };
            let mut job = egui::text::LayoutJob::single_section(
                blurb.clone(),
                egui::TextFormat {
                    font_id: font(11.5, Weight::Regular),
                    color: Color32::WHITE,
                    line_height: Some(15.0),
                    ..Default::default()
                },
            );
            job.wrap = egui::text::TextWrapping {
                max_width: room,
                max_rows: 2,
                break_anywhere: false,
                overflow_character: Some('…'),
            };
            let g = row_ui.painter().layout_job(job);
            let pos = pos2(art.left() + 12.0, art.bottom() - 12.0 - g.size().y);
            row_ui.painter().galley(pos, g, Color32::WHITE);
        }
        // The cover in the corner, over the wide picture.
        if pick.wide.is_some() {
            let cover = Rect::from_min_size(
                pos2(art.right() - 10.0 - 70.0, art.bottom() - 10.0 - 70.0),
                vec2(70.0, 70.0),
            );
            artwork(&row_ui, words.url.clone(), cover, 6.0, theme);
        }
        if response.hovered() {
            row_ui
                .painter()
                .rect_filled(art, 10.0, Color32::from_black_alpha(30));
        }
        let shown = facts.get(i).copied().unwrap_or_default();
        if let Some(choice) = super::menu::collection_menu(&response, theme, shown) {
            clicked = Some((i, CardClick::Menu(choice)));
        } else if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            clicked = Some((i, CardClick::Open));
        }
    }
    clicked
}
