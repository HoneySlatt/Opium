//! Cover grids, as the egui window's `ui/grid.rs`: the library's Albums,
//! Artists and Recently Added pages, one shelf of Home, New or Radio in
//! full, and the cover card, grid layout and shelves the album and artist
//! pages share. Spotify's tiles and Apple Music's covers; M3E's shaped
//! cards come with its theme.

use std::sync::Arc;
use std::time::Instant;

use gpui::prelude::FluentBuilder;
use gpui::{
    ClickEvent, Context, Div, ElementId, InteractiveElement, ParentElement, ScrollEvent,
    SharedString, Stateful, StatefulInteractiveElement, Styled, Window, div,
};
use music_core::model::{Collection, ItemId, Playlist, ProviderKind, Station};

use crate::app::{Loaded, Music, Page, Shelves};
use crate::color::{WHITE, black_alpha, mix};
use crate::fonts::Weight;
use crate::icons;
use crate::theme::{Cards, Theme};

use super::songs::{source_chips, sources_shown};
use super::{
    Align, artwork_sized, disc, elide, explicit_badge, glow, icon_at, icon_button, label,
    label_cut, loading_shape, message, place, section_head, shadow, text_at, text_width, z,
};

/// Grid covers are at least this wide, 26 apart, rows 40 apart, as in Cider.
const CARD_MIN: f32 = 190.0;
const CARD_GAP: f32 = 26.0;
const ROW_GAP: f32 = 40.0;
/// The title and the line under a cover.
pub const CARD_TEXT: f32 = 50.0;
/// Where the grid starts, under the title.
const GRID_TOP: f32 = 100.0;
/// The title's centre line, just under the top bar.
const TITLE_Y: f32 = 66.0;

/// What a cover card shows.
pub struct CoverCard<'a> {
    pub url: Option<String>,
    pub title: &'a str,
    pub sub: &'a str,
    /// An artist: round, with the name centred.
    pub round: bool,
    /// An E after the title.
    pub explicit: bool,
    /// An album or a playlist: its Play shows. (Its right-click menu comes
    /// with the menus.)
    pub menu: bool,
}

/// What a click on a cover asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CardClick {
    Open,
    /// Its play button: played now.
    Enqueue,
}

/// A cover with its title and a muted line under it; `art` is the cover's
/// rect (x, y, w, h) in the parent. A click is handed to `on`, with how.
#[allow(clippy::too_many_arguments)]
pub fn cover_card(
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    art: (f32, f32, f32, f32),
    card: &CoverCard,
    id: ElementId,
    on: impl Fn(&mut Music, CardClick, &mut Context<Music>) + Clone + 'static,
) -> Stateful<Div> {
    let (x, y, w, h) = art;
    let tiles = theme.layout.cards == Cards::Tiles;
    let radius = match (card.round, tiles) {
        (true, _) => w / 2.0,
        (false, true) => 6.0,
        (false, false) => 8.0,
    };
    // A tile takes the cover and its words; else the cover alone.
    let (hx, hy) = if tiles { (x - 10.0, y - 10.0) } else { (x, y) };
    let (hw, hh) = if tiles {
        (w + 20.0, h + 10.0 + CARD_TEXT - 4.0)
    } else {
        (w, h)
    };
    // Where the cover is in the hit area.
    let (ax, ay) = (x - hx, y - hy);
    let group: SharedString = format!("card-{id}").into();
    let surface = theme.surface0;
    let open = on.clone();
    let mut out = place(hx, hy, hw, hh)
        .id(id)
        .group(group.clone())
        .cursor_pointer()
        .when(tiles, |t| t.rounded(z(8.0)).hover(move |s| s.bg(surface)))
        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
            open(this, CardClick::Open, cx);
            this.acted(window, cx);
        }));
    out = out.child(place(ax, ay, w, h).child(artwork_sized(
        window,
        cx,
        card.url.as_deref(),
        w,
        h,
        radius,
        theme,
    )));
    if card.url.is_none() {
        let icon = if card.round { icons::MIC } else { icons::DISC };
        out = out.child(icon_at(
            icon,
            ax + w / 2.0,
            ay + h / 2.0,
            h / 3.0,
            theme.muted(),
        ));
    }
    // Spotify's round Play on the cover, under the pointer.
    if tiles && !card.round && card.menu {
        let (px, py) = (ax + w - 30.0, ay + h - 30.0);
        let (accent, lit) = (theme.accent, mix(theme.accent, WHITE, 0.85));
        out = out.child(
            disc(px, py, 24.0, accent)
                .id("card-play")
                .invisible()
                .group_hover(group.clone(), |s| s.visible())
                .shadow(shadow(8.0, 16.0, black_alpha(90)))
                .hover(move |s| s.bg(lit).left(z(px - 25.0)).top(z(py - 25.0)).size(z(50.0)))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    on(this, CardClick::Enqueue, cx);
                    this.acted(window, cx);
                }))
                .child(
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icons::PLAY_FILL.element(20.0, theme.on_accent())),
                ),
        );
    }
    if !tiles {
        out = out.child(
            place(ax, ay, w, h)
                .rounded(z(radius))
                .invisible()
                .bg(black_alpha(40))
                .group_hover(group, |s| s.visible()),
        );
    }
    out = out.child(
        place(ax, ay, w, h)
            .rounded(z(radius))
            .border(z(1.0))
            .border_color(theme.line()),
    );
    let (tx, align) = if card.round {
        (ax + w / 2.0, Align::Center)
    } else {
        (ax, Align::Min)
    };
    let explicit = card.explicit && !card.round;
    let badge = if explicit { 18.0 } else { 0.0 };
    let title = elide(window, card.title, 13.5, Weight::Regular, w - badge);
    let title_end = tx + text_width(window, &title, 13.5, Weight::Regular);
    out = out.child(text_at(
        tx,
        ay + h + 22.0,
        13.5,
        align,
        label(title, 13.5, Weight::Regular, theme.text),
    ));
    if explicit {
        out = out.child(explicit_badge(theme, title_end + 4.0 + 6.0, ay + h + 22.0));
    }
    let muted = theme.muted();
    out.child(text_at(
        tx,
        ay + h + 39.0,
        12.0,
        align,
        label(
            elide(window, card.sub, 12.0, Weight::Regular, w),
            12.0,
            Weight::Regular,
            muted,
        ),
    ))
}

/// Covers in as many columns as fit, sharing the width.
pub struct Grid {
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

    /// How many covers a row holds.
    pub fn columns(&self) -> usize {
        self.columns
    }

    pub fn height(&self, count: usize) -> f32 {
        let rows = count.div_ceil(self.columns);
        rows as f32 * self.row_height() - if rows > 0 { ROW_GAP } else { 0.0 }
    }

    /// The cover square of item `i`: x, y, width and height.
    pub fn cell(&self, i: usize, left: f32, top: f32) -> (f32, f32, f32, f32) {
        let (row, column) = (i / self.columns, i % self.columns);
        (
            left + column as f32 * (self.card + CARD_GAP),
            top + row as f32 * self.row_height(),
            self.card,
            self.card,
        )
    }

    /// The items of `count` that the page shows from `from` to `to` points
    /// down it, for a grid starting at `top`.
    fn visible(&self, from: f32, to: f32, top: f32, count: usize) -> std::ops::Range<usize> {
        let row = |y: f32| ((y - top) / self.row_height()).max(0.0);
        let first = row(from).floor() as usize * self.columns;
        let last = (row(to).ceil() as usize * self.columns).min(count);
        first.min(last)..last
    }
}

/// Where a shelf is scrolled to, and where it is heading.
#[derive(Clone, Copy, Default)]
pub struct ShelfScroll {
    offset: f32,
    target: f32,
    /// When it last moved towards it, while it does.
    moved: Option<Instant>,
}

/// Sideways scrolling for the shelf `id`: arrows ending at `arrows` (the
/// right end of its title row), eased, as egui's. `room` is how wide its
/// row is, `content` the shelf's full width, `page` how far an arrow
/// moves. Its offset, the arrows, and how far it can go.
#[allow(clippy::too_many_arguments)]
fn shelf_scroll(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    id: &str,
    arrows: (f32, f32),
    room: f32,
    content: f32,
    page: f32,
) -> (f32, Div, f32) {
    let max = (content - room).max(0.0);
    let scroll = app.shelf_scroll.entry(id.to_owned()).or_default();
    scroll.target = scroll.target.clamp(0.0, max);
    let now = Instant::now();
    let dt = scroll
        .moved
        .map_or(1.0 / 60.0, |at| (now - at).as_secs_f32())
        .min(0.1);
    let step = scroll.target - scroll.offset;
    scroll.offset += step * (1.0 - (-dt * 12.0).exp());
    if step.abs() < 0.5 {
        scroll.offset = scroll.target;
        scroll.moved = None;
    } else {
        scroll.moved = Some(now);
        window.request_animation_frame();
    }
    let (offset, target) = (scroll.offset, scroll.target);
    let mut out = div().absolute().left_0().top_0().size_full();
    if max > 0.0 {
        let buttons = [
            (icons::LEFT, -page, target > 0.0),
            (icons::RIGHT, page, target < max),
        ];
        for (i, (icon, step, enabled)) in buttons.into_iter().enumerate() {
            let (bx, by) = (arrows.0 - 14.0 - (1 - i) as f32 * 32.0, arrows.1);
            let fill = theme.text_a(if enabled { 0.08 } else { 0.04 });
            let shelf = id.to_owned();
            let button = icon_button(
                ElementId::Name(format!("{id}-arrow-{i}").into()),
                theme,
                icon,
                28.0,
                15.0,
                theme.subtext0,
                enabled,
            )
            .rounded_full();
            let button = if enabled {
                button.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    let scroll = this.shelf_scroll.entry(shelf.clone()).or_default();
                    scroll.target = (scroll.target + step).clamp(0.0, max);
                    cx.notify();
                }))
            } else {
                button
            };
            out = out.child(disc(bx, by, 14.0, fill).child(button));
        }
    }
    (offset, out, max)
}

/// A cover in a shelf.
pub struct ShelfCard {
    pub title: String,
    pub sub: String,
    pub url: Option<String>,
    pub explicit: bool,
}

/// A titled row of `size` px covers at `at` that scrolls sideways; a click
/// on one is handed to `on`, with its place and how.
#[allow(clippy::too_many_arguments)]
pub fn cover_shelf(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    at: (f32, f32),
    width: f32,
    title: &str,
    cards: &[ShelfCard],
    size: f32,
    id: &str,
    on: impl Fn(&mut Music, usize, CardClick, &mut Context<Music>) + Clone + 'static,
) -> Div {
    shelf_of(
        app,
        window,
        cx,
        theme,
        at,
        width,
        title,
        cards,
        (size, size),
        id,
        on,
    )
}

/// A titled row of music videos' stills, `card_width` px wide at 16:9,
/// that scrolls sideways; the one clicked is handed to `on`.
#[allow(clippy::too_many_arguments)]
pub fn video_shelf(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    at: (f32, f32),
    width: f32,
    title: &str,
    cards: &[ShelfCard],
    card_width: f32,
    id: &str,
    on: impl Fn(&mut Music, usize, &mut Context<Music>) + Clone + 'static,
) -> Div {
    let size = (card_width, card_width * 9.0 / 16.0);
    shelf_of(
        app,
        window,
        cx,
        theme,
        at,
        width,
        title,
        cards,
        size,
        id,
        move |app, i, click, cx| {
            if click == CardClick::Open {
                on(app, i, cx);
            }
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn shelf_of(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    at: (f32, f32),
    width: f32,
    title: &str,
    cards: &[ShelfCard],
    size: (f32, f32),
    id: &str,
    on: impl Fn(&mut Music, usize, CardClick, &mut Context<Music>) + Clone + 'static,
) -> Div {
    // Wider than tall: music videos' stills, which have no menu.
    let video = size.0 > size.1;
    let mut out = div()
        .absolute()
        .left_0()
        .top_0()
        .size_full()
        .child(section_head(theme, at.0, at.1, title));
    let row_top = at.1 + SHELF_HEAD;
    let stride = size.0 + SHELF_GAP;
    let content = cards.len() as f32 * stride - SHELF_GAP;
    // A page is as many whole covers as fit.
    let page = ((width + SHELF_GAP) / stride).floor().max(1.0) * stride;
    let (offset, arrows, max) = shelf_scroll(
        app,
        window,
        cx,
        theme,
        id,
        (at.0 + width, at.1 + 11.0),
        width,
        content,
        page,
    );
    out = out.child(arrows);

    // Room for a tile around the covers, in a theme of tiles.
    let around = if theme.layout.cards != Cards::Covers {
        10.0
    } else {
        0.0
    };
    let shelf = id.to_owned();
    let mut row = place(
        at.0 - around,
        row_top - around,
        width + 2.0 * around,
        size.1 + CARD_TEXT + 2.0 * around,
    )
    .id(ElementId::Name(format!("{id}-row").into()))
    .overflow_hidden()
    // Scrolled sideways over the row, as egui reads it.
    .on_scroll(cx.listener(move |this, e: &ScrollEvent, window, cx| {
        let delta = e.delta.pixel_delta(window.line_height());
        let dx = if e.modifiers.shift && f32::from(delta.x) == 0.0 {
            f32::from(delta.y)
        } else {
            f32::from(delta.x)
        };
        if max > 0.0 && dx != 0.0 {
            let scroll = this.shelf_scroll.entry(shelf.clone()).or_default();
            scroll.target = (scroll.target - dx / super::zoom()).clamp(0.0, max);
            cx.notify();
        }
    }));
    for (i, card) in cards.iter().enumerate() {
        let x = i as f32 * stride - offset;
        if x + size.0 < 0.0 || x > width {
            continue;
        }
        let cover = CoverCard {
            url: card.url.clone(),
            title: &card.title,
            sub: &card.sub,
            round: false,
            explicit: card.explicit,
            menu: !video,
        };
        let on = on.clone();
        row = row.child(cover_card(
            window,
            cx,
            theme,
            (x + around, around, size.0, size.1),
            &cover,
            ElementId::Name(format!("{id}-{i}").into()),
            move |app, click, cx| on(app, i, click, cx),
        ));
    }
    out.child(row)
}

/// One shelf of Home, New or Radio in full: every cover, in a grid, only
/// the rows in sight laid out.
#[allow(clippy::too_many_arguments)]
pub fn shelf(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    width: f32,
    height: f32,
    pad: f32,
    which: Shelves,
    title: &str,
) -> Div {
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
    let room = width - 2.0 * pad;
    let grid = Grid::new(room);
    let grid_height = if loading {
        grid.height(grid.columns * 2)
    } else {
        grid.height(items.len()).max(60.0)
    };
    let total = GRID_TOP + grid_height + 104.0;
    let shown = app.page.clone();
    let scroll = app.page_scroll(&shown);
    let offset = (-f32::from(scroll.offset().y) / super::zoom()).max(0.0);

    let left = pad;
    let mut content = div()
        .relative()
        .w(z(width))
        .h(z(total))
        .child(glow(window, &theme, width, 460.0))
        .child(text_at(
            left,
            TITLE_Y,
            22.0,
            Align::Min,
            label_cut(title.to_owned(), 22.0, Weight::Bold, theme.text, room),
        ));
    let top = GRID_TOP;
    match &shelf {
        Err(e) => {
            content = content.child(message(&theme, left, top, room, "Couldn't load it", e));
        }
        Ok(None) if loading => {
            for i in 0..grid.columns * 2 {
                let (x, y, w, h) = grid.cell(i, left, top);
                content = content.child(loading_shape(&theme, x, y, w, h, 8.0));
            }
        }
        Ok(None) => {
            let detail = "Apple Music took this shelf away. It may be back after a refresh.";
            content = content.child(message(&theme, left, top, room, "Gone", detail));
        }
        Ok(Some(_)) => {
            let items = Arc::new(items);
            for position in grid.visible(offset, offset + height, GRID_TOP, items.len()) {
                let card = card_of(&items[position]);
                let cover = CoverCard {
                    url: card.url,
                    title: &card.title,
                    sub: &card.sub,
                    round: false,
                    explicit: card.explicit,
                    menu: true,
                };
                let items = items.clone();
                content = content.child(cover_card(
                    window,
                    cx,
                    &theme,
                    grid.cell(position, left, top),
                    &cover,
                    ElementId::Name(format!("shelf-card-{title}-{position}").into()),
                    move |app, click, _| act(app, &items[position], click),
                ));
            }
        }
    }
    scrolled_page(
        &theme,
        &shown,
        &scroll,
        width,
        offset,
        content,
        app.page_title(),
    )
}

/// The page in its scroll area, and the bar over it once scrolled.
pub fn scrolled_page(
    theme: &Theme,
    shown: &Page,
    scroll: &gpui::ScrollHandle,
    width: f32,
    offset: f32,
    content: Div,
    title: String,
) -> Div {
    let mut page = div().absolute().left_0().top_0().size_full().child(
        div()
            .id(ElementId::Name(format!("{shown:?}").into()))
            .size_full()
            .overflow_y_scroll()
            .track_scroll(scroll)
            .child(content),
    );
    if offset > 24.0 {
        page = page.child(super::scrolled_bar(theme, width, title));
    }
    page
}

/// Do what a click on a shelf's cover asked: open it, play it, or tune in.
fn act(app: &mut Music, item: &Collection, click: CardClick) {
    match (click, item) {
        (_, Collection::Station(s)) => app.play_station(s),
        (CardClick::Open, Collection::Album(a)) => app.go(Page::Album(a.id.clone())),
        (CardClick::Open, Collection::Playlist(p)) => app.open_playlist(p),
        (CardClick::Enqueue, Collection::Album(a)) => app.enqueue_album(a.id.clone()),
        (CardClick::Enqueue, Collection::Playlist(p)) => app.enqueue_playlist(p),
    }
}

/// A shelf's item as its cover shows it.
fn card_of(item: &Collection) -> ShelfCard {
    match item {
        Collection::Album(a) => ShelfCard {
            title: a.title.clone(),
            sub: a.artists.join(", "),
            url: a.artwork.as_ref().map(|art| art.url(400)),
            explicit: a.explicit,
        },
        Collection::Playlist(p) => ShelfCard {
            title: p.name.clone(),
            sub: p.curator.clone().unwrap_or_else(|| "Playlist".into()),
            url: p.artwork.as_ref().map(|art| art.url(400)),
            explicit: false,
        },
        Collection::Station(s) => ShelfCard {
            title: s.name.clone(),
            sub: s.description.clone().unwrap_or_else(|| "Station".into()),
            url: s.artwork.as_ref().map(|art| art.url(400)),
            explicit: false,
        },
    }
}

/// A shelf's title row, and the gap between its covers.
pub const SHELF_HEAD: f32 = 34.0;
pub const SHELF_GAP: f32 = 20.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Albums,
    Artists,
    Recent,
}

/// Where a click on a cover leads.
#[derive(Clone)]
enum Open {
    Album(ItemId),
    Artist(ItemId),
    Playlist(Playlist),
    Station(Station),
}

pub fn albums(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    width: f32,
    height: f32,
    pad: f32,
) -> Div {
    page(app, window, cx, width, height, pad, Kind::Albums)
}

pub fn artists(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    width: f32,
    height: f32,
    pad: f32,
) -> Div {
    page(app, window, cx, width, height, pad, Kind::Artists)
}

pub fn recently_added(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    width: f32,
    height: f32,
    pad: f32,
) -> Div {
    page(app, window, cx, width, height, pad, Kind::Recent)
}

#[allow(clippy::too_many_arguments)]
fn page(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    width: f32,
    height: f32,
    pad: f32,
    kind: Kind,
) -> Div {
    let theme = app.theme.clone();
    let source = app.grid_source.filter(|_| kind != Kind::Artists);
    let keep = |provider: ProviderKind| source.is_none_or(|s| s == provider);
    // The items shown, as indices into the list.
    // Across services, one copy of each; a service's filter shows it whole.
    let copies = match (source, kind) {
        (Some(_), _) => Default::default(),
        (None, Kind::Albums) => app.copies(0),
        (None, Kind::Artists) => app.copies(1),
        (None, Kind::Recent) => app.copies(2),
    };
    let (shown, done): (Vec<usize>, bool) = match kind {
        Kind::Albums => (
            app.albums_in_order()
                .iter()
                .copied()
                .filter(|&i| keep(app.library_albums.items[i].id.provider) && !copies.contains(&i))
                .collect(),
            app.library_albums.done,
        ),
        Kind::Artists => (
            (0..app.library_artists.items.len())
                .filter(|i| !copies.contains(i))
                .collect(),
            app.library_artists.done,
        ),
        Kind::Recent => (
            (0..app.recently_added.items.len())
                .filter(|&i| keep(item_id(&app.recently_added.items[i]).provider))
                .filter(|i| !copies.contains(i))
                .collect(),
            app.recently_added.done,
        ),
    };
    // What the page's search finds comes with the text field: all is shown.
    let (title, one, many) = match kind {
        Kind::Albums => ("Albums", "album", "albums"),
        Kind::Artists => ("Artists", "artist", "artists"),
        Kind::Recent => ("Recently Added", "item", "items"),
    };
    let room = width - 2.0 * pad;
    let grid = Grid::new(room);
    let loading = shown.is_empty() && !done;
    let grid_height = if loading {
        grid.height(grid.columns * 2)
    } else {
        grid.height(shown.len()).max(60.0)
    };
    let total = GRID_TOP + grid_height + 18.0 + 16.0 + 18.0 + 104.0;
    let page = app.page.clone();
    let scroll = app.page_scroll(&page);
    let offset = (-f32::from(scroll.offset().y) / super::zoom()).max(0.0);

    let left = pad;
    let mut content = div()
        .relative()
        .w(z(width))
        .h(z(total))
        .child(glow(window, &theme, width, 460.0));
    let title_w = text_width(window, title, 22.0, Weight::Bold).min(room);
    let title_h = crate::fonts::line_height(22.0);
    content = content.child(text_at(
        left,
        TITLE_Y,
        22.0,
        Align::Min,
        label_cut(title, 22.0, Weight::Bold, theme.text, room),
    ));
    let mut title_end = left + title_w;
    // Albums and Recently Added show their source filter once their title
    // is clicked.
    let services = app.services_in_use();
    let mut sources = false;
    if kind != Kind::Artists {
        let (chevron, shown_now, end) = sources_shown(
            app,
            cx,
            &theme,
            &page,
            (left, TITLE_Y - title_h / 2.0, title_w, title_h),
            &services,
        );
        if let Some(chevron) = chevron {
            content = content.child(chevron);
        }
        (sources, title_end) = (shown_now, end);
    }
    // Albums sort as the menu after the title says. (Its menu comes with
    // the menus.)
    if kind == Kind::Albums {
        content = content.child(place(title_end + 10.0, TITLE_Y - 16.0, 32.0, 32.0).child(
            icon_button(
                "albums-sort",
                &theme,
                icons::SORT,
                32.0,
                16.0,
                theme.subtext0,
                true,
            ),
        ));
    }
    // The search, right-aligned, then the source filter left of it: its
    // magnifier, closed. Typing in it comes with the text field.
    let chips_right = if kind == Kind::Recent {
        left + room
    } else {
        let field = left + room - 32.0;
        content = content.child(place(field, TITLE_Y - 16.0, 32.0, 32.0).child(icon_button(
            "page-search",
            &theme,
            icons::SEARCH,
            32.0,
            15.0,
            theme.subtext0,
            true,
        )));
        field - 10.0
    };
    if sources {
        content = content.child(source_chips(
            window,
            cx,
            &theme,
            chips_right,
            TITLE_Y - 14.0,
            app.grid_source,
            &services,
        ));
    }

    let top = GRID_TOP;
    if loading {
        for i in 0..grid.columns * 2 {
            let (x, y, w, h) = grid.cell(i, left, top);
            let radius = if kind == Kind::Artists { w / 2.0 } else { 8.0 };
            content = content.child(loading_shape(&theme, x, y, w, h, radius));
        }
    } else if shown.is_empty() {
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
        content = content.child(message(&theme, left, top, room, head, &detail));
    } else {
        for position in grid.visible(offset, offset + height, GRID_TOP, shown.len()) {
            let i = shown[position];
            let (card_title, sub, url, open) = card(app, kind, i);
            let explicit = matches!(&open, Open::Album(_)) && explicit_album(app, kind, i);
            let cover = CoverCard {
                url,
                title: &card_title,
                sub: &sub,
                round: kind == Kind::Artists,
                explicit,
                menu: true,
            };
            content = content.child(cover_card(
                window,
                cx,
                &theme,
                grid.cell(position, left, top),
                &cover,
                ElementId::Name(format!("grid-card-{title}-{position}").into()),
                move |app, click, _| match (&open, click) {
                    (Open::Album(id), CardClick::Open) => app.go(Page::Album(id.clone())),
                    // An artist from the library opens as the library has them.
                    (Open::Artist(id), CardClick::Open) => app.go(Page::LibraryArtist(id.clone())),
                    (Open::Playlist(playlist), CardClick::Open) => app.open_playlist(playlist),
                    (Open::Station(station), _) => app.play_station(station),
                    (Open::Album(id), CardClick::Enqueue) => app.enqueue_album(id.clone()),
                    (Open::Playlist(p), CardClick::Enqueue) => app.enqueue_playlist(p),
                    (Open::Artist(_), CardClick::Enqueue) => {}
                },
            ));
        }
        let count = shown.len();
        let foot = format!(
            "{count} {}{}",
            if count == 1 { one } else { many },
            if done { "" } else { " so far, loading…" }
        );
        content = content.child(text_at(
            left,
            top + grid.height(count) + 18.0 + 8.0,
            12.0,
            Align::Min,
            label_cut(foot, 12.0, Weight::Regular, theme.muted(), room),
        ));
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

fn item_id(item: &Collection) -> &ItemId {
    match item {
        Collection::Album(a) => &a.id,
        Collection::Playlist(p) => &p.id,
        Collection::Station(s) => &s.id,
    }
}

/// Whether item `i` of a library page is an album rated explicit.
fn explicit_album(app: &Music, kind: Kind, i: usize) -> bool {
    match kind {
        Kind::Albums => app.library_albums.items[i].explicit,
        Kind::Recent => matches!(&app.recently_added.items[i], Collection::Album(a) if a.explicit),
        Kind::Artists => false,
    }
}

/// Title, muted line, cover and target of item `i` of a grid.
fn card(app: &Music, kind: Kind, i: usize) -> (String, String, Option<String>, Open) {
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
                // The library's own id, which its page asks by.
                Open::Artist(a.library_id.clone().unwrap_or_else(|| a.id.clone())),
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
