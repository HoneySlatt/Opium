//! An artist as the library has them, as the egui window's
//! `ui/library_artist.rs` shows one in Spotify's look: their picture across
//! the top under their name, Play, Shuffle and a pill to their page under
//! it, then their albums and their singles and EPs there, newest first.
//! Apple Music's and M3E's headers come with their themes.

use std::cell::RefCell;
use std::sync::Arc;

use gpui::{
    ClickEvent, Context, Div, ElementId, InteractiveElement, ObjectFit, ParentElement, Rgba,
    StatefulInteractiveElement, Styled, StyledImage, Window, div, img,
};
use music_core::model::{Album, ItemId};

use crate::app::{Loaded, Music, Page};
use crate::color::{BLACK, WHITE, black_alpha, mix};
use crate::fonts::{Weight, line_height};
use crate::icons;
use crate::theme::{Header, Theme};

use super::grid::{CARD_TEXT, CardClick, CoverCard, Grid, cover_card, scrolled_page};
use super::{
    Align, disc, glow, icon_button, label, label_cut, loading_shape, message, place, section_head,
    text_at, text_width, vertical_gradient, z,
};

const PHOTO_TOP: f32 = 60.0;
const PHOTO: f32 = 200.0;
/// From the top of the page to the first section.
const BODY_TOP: f32 = PHOTO_TOP + PHOTO + 48.0;
const SECTION_HEAD: f32 = 40.0;
const SECTION_GAP: f32 = 36.0;

/// The row of Play, Shuffle and the pill under the picture, as the egui
/// window's artist page has it.
const BANNER_ACTIONS: f32 = 104.0;

/// Their releases in the two sections the page shows, each newest first.
#[derive(Clone)]
struct Shelves {
    albums: Arc<Vec<Album>>,
    singles: Arc<Vec<Album>>,
}

/// Their releases as the page shows them, none while nothing has come, and
/// what went wrong when nothing did.
#[derive(Clone, Default)]
struct Releases {
    kept: Option<Shelves>,
    error: Option<String>,
}

/// What a twin of theirs had when the page was last built.
enum Had {
    Coming,
    /// The very list, kept so that a list freed since can't be taken for it.
    Albums(Arc<Vec<Album>>),
    Failed(String),
}

impl PartialEq for Had {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Coming, Self::Coming) => true,
            (Self::Albums(a), Self::Albums(b)) => Arc::ptr_eq(a, b),
            (Self::Failed(a), Self::Failed(b)) => a == b,
            _ => false,
        }
    }
}

/// The page's releases and what they were built from: the artist, and the
/// list each of their twins had.
struct Kept {
    id: ItemId,
    from: Vec<(ItemId, Had)>,
    releases: Releases,
}

thread_local! {
    /// What the page built last. Gathering a whole discography, dropping its
    /// copies and sorting it is work for a list that changed, not for every
    /// frame of a page standing still.
    static KEPT: RefCell<Option<Kept>> = const { RefCell::new(None) };
}

/// Their releases on every service the library has them on, one copy of
/// each, built again only once one of those lists is another.
fn releases(app: &mut Music, id: &ItemId) -> Releases {
    let from: Vec<(ItemId, Had)> = app
        .artist_twins(id)
        .into_iter()
        .map(|twin| {
            let had = match app.artist_albums.get(&twin) {
                Some(Loaded::Ready(albums)) => Had::Albums(albums.clone()),
                Some(Loaded::Failed(e)) => Had::Failed(e.clone()),
                _ => Had::Coming,
            };
            (twin, had)
        })
        .collect();
    let kept = KEPT.with_borrow(|kept| match kept {
        Some(kept) if kept.id == *id && kept.from == from => Some(kept.releases.clone()),
        _ => None,
    });
    if let Some(releases) = kept {
        return releases;
    }
    let mut gathered: Option<Vec<Album>> = None;
    let mut error = None;
    for (_, had) in &from {
        match had {
            Had::Albums(albums) => {
                gathered
                    .get_or_insert_with(Vec::new)
                    .extend(albums.iter().cloned());
            }
            Had::Failed(e) => error = Some(e.clone()),
            Had::Coming => {}
        }
    }
    // Singles and EPs apart, as Apple names them; each newest first.
    let kept = gathered.map(|all| {
        let copies = music_core::dedupe::albums(&all);
        let (mut long, mut short): (Vec<Album>, Vec<Album>) = all
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !copies.contains(i))
            .map(|(_, a)| a)
            .partition(|a| !(a.title.ends_with(" - Single") || a.title.ends_with(" - EP")));
        let newest = |a: &Album, b: &Album| b.released.cmp(&a.released);
        long.sort_by(newest);
        short.sort_by(newest);
        Shelves {
            albums: Arc::new(long),
            singles: Arc::new(short),
        }
    });
    let releases = Releases {
        // A failure shows only when nothing came.
        error: error.filter(|_| kept.is_none()),
        kept,
    };
    KEPT.set(Some(Kept {
        id: id.clone(),
        from,
        releases: releases.clone(),
    }));
    releases
}

/// The page, `width` by `height` points in the main card.
#[allow(clippy::too_many_arguments)]
pub fn show(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    width: f32,
    height: f32,
    pad: f32,
    id: &ItemId,
) -> Div {
    let theme = app.theme.clone();
    // Their releases on every service the library has them on, one copy
    // of each: what has come, while the rest comes.
    let releases = releases(app, id);
    let artist = app.library_artist(id).cloned();
    // Spotify's: their picture across the top under their name, Play,
    // Shuffle and a pill to their page under it.
    let spotify = theme.layout.header == Header::Banner;
    let singles = if spotify {
        "Singles and EPs"
    } else {
        "Singles & EPs"
    };
    let sections: Vec<(&str, Arc<Vec<Album>>)> = match &releases.kept {
        Some(shelves) => [
            ("Albums", shelves.albums.clone()),
            (singles, shelves.singles.clone()),
        ]
        .into_iter()
        .filter(|(_, a)| !a.is_empty())
        .collect(),
        None => Vec::new(),
    };

    let room = width - 2.0 * pad;
    let grid = Grid::new(room);
    let hero = (width * 0.3).clamp(340.0, 500.0);
    let body_top = if spotify {
        hero + BANNER_ACTIONS
    } else {
        BODY_TOP
    };
    let mut total = body_top + 104.0;
    for (_, albums) in &sections {
        total += SECTION_HEAD + grid.height(albums.len()) + SECTION_GAP;
    }
    if releases.kept.is_none() {
        total += SECTION_HEAD + grid.height(grid.columns());
    }
    let page = app.page.clone();
    let scroll = app.page_scroll(&page);
    let offset = (-f32::from(scroll.offset().y) / super::zoom()).max(0.0);

    let left = pad;
    let mut content = div().relative().w(z(width)).h(z(total));
    if spotify {
        content = content.child(banner_fade(&theme, theme.surface1, width, hero));
        let url = artist
            .as_ref()
            .and_then(|a| a.artwork.as_ref())
            .map(|a| a.url(1600));
        content = content.child(picture(window, cx, &theme, url.as_deref(), width, hero));
        let name = artist.as_ref().map_or("", |a| a.name.as_str());
        content = content.child(banner_name(window, name, left, room, hero));
        let catalog = artist.as_ref().map_or_else(|| id.clone(), |a| a.id.clone());
        content = content.child(banner_actions(
            app,
            window,
            cx,
            &theme,
            left,
            hero,
            "Artist page",
            sections.iter().flat_map(|(_, a)| a.to_vec()).collect(),
            catalog,
        ));
    } else {
        content = content.child(glow(window, &theme, width, 460.0));
    }
    let mut y = body_top;
    if let Some(e) = &releases.error {
        content = content.child(message(
            &theme,
            left,
            y,
            room,
            "Couldn't load this artist",
            e,
        ));
    } else if releases.kept.is_none() {
        content = content.child(section_head(&theme, left, y, "Albums"));
        let top = y + SECTION_HEAD;
        for i in 0..grid.columns() {
            let (x, y, w, h) = grid.cell(i, left, top);
            content = content.child(loading_shape(&theme, x, y, w, h, 8.0));
        }
    } else {
        for (title, albums) in &sections {
            content = content.child(section_head(&theme, left, y, title));
            let top = y + SECTION_HEAD;
            for (i, album) in albums.iter().enumerate() {
                let art = grid.cell(i, left, top);
                // Out of sight, not laid out.
                if art.1 + art.3 + CARD_TEXT < offset || art.1 > offset + height {
                    continue;
                }
                let sub = release_line(album);
                let card = CoverCard {
                    url: album.artwork.as_ref().map(|a| a.url(376)),
                    title: &album.title,
                    sub: &sub,
                    round: false,
                    explicit: album.explicit,
                    menu: true,
                };
                let album_id = album.id.clone();
                content = content.child(cover_card(
                    window,
                    cx,
                    &theme,
                    art,
                    &card,
                    ElementId::Name(format!("library-artist-card-{title}-{i}").into()),
                    move |app, click, _| match click {
                        CardClick::Open => app.go(Page::Album(album_id.clone())),
                        CardClick::Enqueue => app.enqueue_album(album_id.clone()),
                    },
                ));
            }
            y = top + grid.height(albums.len()) + SECTION_GAP;
        }
    }
    scrolled_page(
        &theme,
        &page,
        &scroll,
        width,
        offset,
        content,
        app.page_title(),
    )
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

/// Under the picture, `hero` points tall and `width` wide, `color` fading
/// into the page.
fn banner_fade(theme: &Theme, color: Rgba, width: f32, hero: f32) -> Div {
    let shade = mix(mix(BLACK, color, 0.4), theme.base, 0.8);
    place(0.0, hero, width, 260.0).child(vertical_gradient(Some(width), 260.0, shade, theme.base))
}

/// Their photo filling the top `width` by `hero` points, a little above
/// its middle, where faces are; darkening at its foot under the name.
fn picture(
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    url: Option<&str>,
    width: f32,
    hero: f32,
) -> Div {
    let mut out = place(0.0, 0.0, width, hero)
        .overflow_hidden()
        .bg(theme.surface0);
    let pixels = width * super::zoom() * window.scale_factor();
    let image = url
        .and_then(|url| crate::covers::Cover::drawn(url, pixels))
        .and_then(|cover| {
            crate::covers::seen(&cover);
            match window.use_asset::<crate::covers::CoverLoader>(&cover, cx) {
                Some(Ok(image)) => Some(image),
                _ => None,
            }
        });
    if let Some(image) = image {
        let size = image.size(0);
        let (tw, th) = (size.width.0 as f32, size.height.0 as f32);
        if tw > 0.0 && th > 0.0 {
            let scale = (width / tw).max(hero / th);
            let (dw, dh) = (tw * scale, th * scale);
            out = out.child(
                place(-(dw - width) / 2.0, -(dh - hero) * 0.3, dw, dh)
                    .child(img(image).size_full().object_fit(ObjectFit::Fill)),
            );
        }
    }
    out.child(vertical_gradient(
        Some(width),
        hero * 0.4,
        black_alpha(60),
        black_alpha(0),
    ))
    .child(
        place(0.0, hero * 0.4, width, hero * 0.6).child(vertical_gradient(
            Some(width),
            hero * 0.6,
            black_alpha(0),
            black_alpha(140),
        )),
    )
}

/// The name over the picture's foot at `bottom`, as large as it fits.
fn banner_name(window: &Window, name: &str, left: f32, width: f32, bottom: f32) -> Div {
    let size = [96.0, 72.0, 56.0, 40.0]
        .into_iter()
        .find(|&size| text_width(window, name, size, Weight::Bold) <= width)
        .unwrap_or(40.0);
    let y = bottom - 28.0 - line_height(size) / 2.0;
    text_at(
        left,
        y,
        size,
        Align::Min,
        label_cut(name.to_owned(), size, Weight::Bold, WHITE, width),
    )
}

/// Under the picture's foot at `bottom`: Play in the accent, Shuffle a
/// switch, green with a dot while on, as Spotify's, and an outlined pill
/// saying `pill`, to their page on the service. Play plays every song of
/// theirs in the library from `albums`, album by album.
#[allow(clippy::too_many_arguments)]
fn banner_actions(
    app: &Music,
    window: &Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    left: f32,
    bottom: f32,
    pill: &'static str,
    albums: Vec<Album>,
    catalog: ItemId,
) -> Div {
    let mid = bottom + BANNER_ACTIONS / 2.0;
    let mut out = div().absolute().left_0().top_0().size_full();
    let (px, py) = (left + 28.0, mid);
    let lit = mix(theme.accent, WHITE, 0.85);
    out = out.child(
        disc(px, py, 28.0, theme.accent)
            .id("library-artist-play")
            .cursor_pointer()
            .hover(move |s| s.bg(lit).left(z(px - 30.0)).top(z(py - 30.0)).size(z(60.0)))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                let songs = this.songs_of(&albums);
                if songs.is_empty() {
                    this.notify("Their songs come with the library, still loading");
                } else {
                    this.play_page(songs);
                }
                this.acted(window, cx);
            }))
            .child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        (if app.page_playing() {
                            icons::PAUSE_FILL
                        } else {
                            icons::PLAY_FILL
                        })
                        .element(24.0, theme.on_accent()),
                    ),
            ),
    );
    let (sx, shuffling) = (left + 56.0 + 16.0, app.shown().shuffle);
    let color = if shuffling {
        theme.accent
    } else {
        theme.subtext0
    };
    out = out.child(
        place(sx, mid - 20.0, 40.0, 40.0).child(
            icon_button(
                "library-artist-shuffle",
                theme,
                icons::SHUFFLE,
                40.0,
                26.0,
                color,
                true,
            )
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.toggle_shuffle();
                this.acted(window, cx);
            })),
        ),
    );
    if shuffling {
        out = out.child(disc(sx + 20.0, mid + 17.0, 2.0, theme.accent));
    }
    let w = text_width(window, pill, 14.0, Weight::Bold) + 32.0;
    let (text, border) = (theme.text, theme.text_a(0.5));
    out.child(
        place(sx + 40.0 + 16.0, mid - 16.0, w, 32.0)
            .id("library-artist-pill")
            .cursor_pointer()
            .rounded(z(16.0))
            .border(z(1.0))
            .border_color(border)
            .hover(move |s| s.border_color(text))
            .child(text_at(
                w / 2.0,
                16.0,
                14.0,
                Align::Center,
                label(pill, 14.0, Weight::Bold, theme.text),
            ))
            // By the catalog's id when the library knows it. (The page
            // itself comes later.)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.go(Page::Artist(catalog.clone()));
                cx.notify();
            })),
    )
}
