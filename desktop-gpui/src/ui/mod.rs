//! The window: sidebar, page, top bar and player bar, after `design/…/DESIGN.md`,
//! as the egui window's `ui/mod.rs` lays them out. Its numbers are egui's
//! points; [`z`] turns them into GPUI's pixels at the interface's scale, as
//! egui's zoom did.

pub mod detail;
pub mod dock;
pub mod grid;
pub mod library;
pub mod library_artist;
mod paint;
pub mod songs;
pub mod tracks;
pub mod window_bar;

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use gpui::prelude::FluentBuilder;

use gpui::{
    App, BoxShadow, ClickEvent, Div, ElementId, InteractiveElement, IntoElement, ObjectFit,
    ParentElement, Pixels, Rgba, SharedString, Stateful, StatefulInteractiveElement, Styled,
    StyledImage, Window, div, img, linear_color_stop, linear_gradient, point, px,
};
use music_core::model::Track;

use crate::app::{Music, Page};
use crate::color::{BLACK, WHITE, alpha, lerp, mix};
use crate::fonts::{Weight, line_height};
use crate::icons::{self, Icon};
use crate::theme::Theme;

/// The gap between cards, in a theme of cards.
pub const GAP: f32 = 8.0;
/// The cards' corners.
pub const RADIUS: f32 = 8.0;
/// Below this window width the sidebar collapses and padding tightens.
pub const NARROW: f32 = 1100.0;
pub const TOP_BAR: f32 = 52.0;

/// Points to pixels: egui's zoom, the interface scale times
/// `desktop::settings::SCALE_UNIT`.
static ZOOM: AtomicU32 = AtomicU32::new(0x3f8c_cccd); // 1.1

pub fn set_zoom(zoom: f32) {
    ZOOM.store(zoom.to_bits(), Ordering::Relaxed);
}

pub fn zoom() -> f32 {
    f32::from_bits(ZOOM.load(Ordering::Relaxed))
}

/// `points` of egui's, in GPUI's pixels.
pub fn z(points: f32) -> Pixels {
    px(points * zoom())
}

/// A box at `x`, `y` (points, in its parent), `w` by `h`.
pub fn place(x: f32, y: f32, w: f32, h: f32) -> Div {
    div().absolute().left(z(x)).top(z(y)).w(z(w)).h(z(h))
}

/// How text is set: Geist at `size` points in `weight`, one line, its box
/// as tall as egui's galley of it.
pub fn label(s: impl Into<SharedString>, size: f32, weight: Weight, color: Rgba) -> Div {
    div()
        .flex_none()
        .text_size(z(size))
        .line_height(z(line_height(size)))
        .font_weight(weight.gpui())
        .text_color(color)
        .whitespace_nowrap()
        .child(s.into())
}

/// …cut with an ellipsis past `max` points, as egui's `truncate_at_width`.
pub fn label_cut(
    s: impl Into<SharedString>,
    size: f32,
    weight: Weight,
    color: Rgba,
    max: f32,
) -> Div {
    label(s, size, weight, color)
        .max_w(z(max.max(0.0)))
        .truncate()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Min,
    Center,
    Max,
}

/// `child`, a line of text `size` points high, vertically centred on `y`
/// from `x`, with `align` horizontally: egui's `paint_text`.
pub fn text_at(x: f32, y: f32, size: f32, align: Align, child: impl IntoElement) -> Div {
    let h = line_height(size);
    let top = y - h / 2.0;
    // Wide boxes either side of `x` hold it centred or ending there.
    const ROOM: f32 = 4000.0;
    match align {
        Align::Min => div().absolute().left(z(x)).top(z(top)).child(child),
        Align::Center => div()
            .absolute()
            .left(z(x - ROOM / 2.0))
            .w(z(ROOM))
            .top(z(top))
            .flex()
            .justify_center()
            .child(child),
        Align::Max => div()
            .absolute()
            .left(z(x - ROOM))
            .w(z(ROOM))
            .top(z(top))
            .flex()
            .justify_end()
            .child(child),
    }
}

/// How wide a line of text is set, in points, as egui's galley size.
pub fn text_width(window: &Window, s: &str, size: f32, weight: Weight) -> f32 {
    if s.is_empty() {
        return 0.0;
    }
    let mut font = gpui::font(crate::fonts::FAMILY);
    font.weight = weight.gpui();
    let run = gpui::TextRun {
        len: s.len(),
        font,
        color: gpui::black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let line = window
        .text_system()
        .shape_line(s.to_owned().into(), z(size), &[run], None);
    f32::from(line.width) / zoom()
}

/// `s` as egui's `truncate_at_width` sets it in `max` points: whole if it
/// fits, else as much of it as fits with an ellipsis after. GPUI's own
/// truncation cuts sooner, which matters where something follows the text.
pub fn elide(window: &Window, s: &str, size: f32, weight: Weight, max: f32) -> String {
    if text_width(window, s, size, weight) <= max {
        return s.to_owned();
    }
    let ends: Vec<usize> = s.char_indices().map(|(i, _)| i).skip(1).collect();
    // The most characters that fit with the ellipsis.
    let (mut fit, mut over) = (0, ends.len());
    while fit < over {
        let mid = (fit + over).div_ceil(2);
        if text_width(window, &format!("{}…", &s[..ends[mid - 1]]), size, weight) <= max {
            fit = mid;
        } else {
            over = mid - 1;
        }
    }
    match fit {
        0 => "…".to_owned(),
        n => format!("{}…", &s[..ends[n - 1]]),
    }
}

/// An icon `size` points square centred on `x`, `y`.
pub fn icon_at(icon: Icon, x: f32, y: f32, size: f32, color: Rgba) -> Div {
    place(x - size / 2.0, y - size / 2.0, size, size).child(icon.element(size, color))
}

/// A disc of `r` points round `x`, `y`.
pub fn disc(x: f32, y: f32, r: f32, color: Rgba) -> Div {
    place(x - r, y - r, 2.0 * r, 2.0 * r)
        .rounded_full()
        .bg(color)
}

/// A square icon button `size` points wide: no background until hovered,
/// the glyph then in the text's colour; disabled, faded and inert. Its
/// place is the caller's.
pub fn icon_button(
    id: impl Into<ElementId>,
    theme: &Theme,
    icon: Icon,
    size: f32,
    glyph: f32,
    color: Rgba,
    enabled: bool,
) -> Stateful<Div> {
    let id = id.into();
    let group: SharedString = format!("icon-button-{id}").into();
    let color = if enabled {
        color
    } else {
        alpha(color, 0.35 * color.a)
    };
    let hover_color = theme.text;
    let button = div()
        .id(id)
        .group(group.clone())
        .flex_none()
        .size(z(size))
        .rounded(z(8.0))
        .flex()
        .items_center()
        .justify_center()
        .child(
            gpui::svg()
                .data(icon.bytes())
                .size(z(glyph))
                .text_color(color)
                .when(enabled, |s| {
                    s.group_hover(group, move |s| s.text_color(hover_color))
                }),
        );
    if !enabled {
        return button;
    }
    let (hover, press) = (theme.hover(), theme.press());
    button
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .active(move |s| s.bg(press))
}

/// The "E" badge after explicit titles, centred on `x`, `y`.
pub fn explicit_badge(theme: &Theme, x: f32, y: f32) -> Div {
    place(x - 6.0, y - 6.0, 12.0, 12.0)
        .rounded(z(2.5))
        .bg(mix(theme.subtext0, theme.base, 0.55))
        .child(text_at(
            6.0,
            6.0,
            8.5,
            Align::Center,
            label("E", 8.5, Weight::Bold, theme.base),
        ))
}

/// Whether `track` plays as Spatial Audio, its Dolby Atmos mix.
pub fn plays_in_atmos(track: &music_core::model::Track) -> bool {
    track
        .quality
        .as_ref()
        .is_some_and(|q| q.codec == music_core::model::Codec::Other("Dolby Atmos".into()))
}

/// The badge a track's quality earns, as Apple Music gives them: Lossless or
/// Dolby Atmos, none for a lossy stream.
pub fn quality_badge(track: &music_core::model::Track) -> Option<&'static str> {
    if plays_in_atmos(track) {
        Some("Dolby Atmos")
    } else if track.quality.as_ref()?.codec.is_lossless() {
        Some("Lossless")
    } else {
        None
    }
}

pub fn format_time(d: Duration) -> String {
    let s = d.as_secs();
    format!("{}:{:02}", s / 60, s % 60)
}

/// Artwork URLs share one size so the list, the sidebar and the player bar hit
/// the same cached image.
pub fn thumb_url(track: &music_core::model::Track) -> Option<String> {
    track.artwork.as_ref().map(|a| a.url(120))
}

/// Artwork from a URL template in a box `size` points square, rounded
/// `radius`: the fill while it loads, the cover over it once in.
pub fn artwork(
    window: &mut Window,
    cx: &mut App,
    url: Option<&str>,
    size: f32,
    radius: f32,
    theme: &Theme,
) -> Div {
    artwork_sized(window, cx, url, size, size, radius, theme)
}

/// …in a box `w` by `h` points, the picture stretched to it: a music
/// video's still.
pub fn artwork_sized(
    window: &mut Window,
    cx: &mut App,
    url: Option<&str>,
    w: f32,
    h: f32,
    radius: f32,
    theme: &Theme,
) -> Div {
    let frame = div()
        .flex_none()
        .w(z(w))
        .h(z(h))
        .rounded(z(radius))
        .bg(theme.surface0);
    let Some(url) = url else {
        return frame;
    };
    let pixels = w.max(h) * zoom() * window.scale_factor();
    let Some(cover) = crate::covers::Cover::drawn(url, pixels) else {
        return frame;
    };
    crate::covers::seen(&cover);
    // Loaded, the cover; until then (or failing), the fill alone.
    let Some(Ok(image)) = window.use_asset::<crate::covers::CoverLoader>(&cover, cx) else {
        return frame;
    };
    frame.child(
        img(image)
            .w(z(w))
            .h(z(h))
            .rounded(z(radius))
            .object_fit(ObjectFit::Fill),
    )
}

/// Spotify's Liked Songs violet, for the library's songs.
pub const LOVED: [Rgba; 2] = [crate::color::rgb(0x450af5), crate::color::rgb(0xc4efd9)];

/// A rounded square `size` points wide of `from` at its top left to `to`
/// at its bottom right, as egui's mesh of it (see [`paint::diagonal`]).
pub fn gradient(window: &Window, size: f32, radius: f32, from: Rgba, to: Rgba) -> Div {
    let pixels = (size * zoom() * window.scale_factor()).round() as u32;
    div().flex_none().size(z(size)).child(
        img(paint::diagonal(from, to, pixels))
            .size(z(size))
            .rounded(z(radius))
            .object_fit(ObjectFit::Fill),
    )
}

/// The library's songs as Spotify draws Liked Songs: a white heart on violet.
pub fn loved_tile(window: &Window, size: f32) -> Div {
    gradient(window, size, 4.0, LOVED[0], LOVED[1])
        .relative()
        .child(icon_at(
            icons::HEART,
            size / 2.0,
            size / 2.0,
            size * 0.4,
            crate::color::WHITE,
        ))
}

/// `h` points of `top` at the top to `bottom` at the bottom, each byte
/// straight between them as egui's mesh blends them: in bands, each a
/// gradient of its own, GPUI's being made in another space.
pub fn vertical_gradient(w: Option<f32>, h: f32, top: Rgba, bottom: Rgba) -> Div {
    const BANDS: usize = 8;
    let mut column = div().flex_none().flex().flex_col().h(z(h));
    column = match w {
        Some(w) => column.w(z(w)),
        None => column.w_full(),
    };
    column.children((0..BANDS).map(|i| {
        let (t0, t1) = (i as f32 / BANDS as f32, (i + 1) as f32 / BANDS as f32);
        div()
            .flex_none()
            .w_full()
            .h(z(h / BANDS as f32))
            .bg(linear_gradient(
                180.0,
                linear_color_stop(lerp(top, bottom, t0), 0.0),
                linear_color_stop(lerp(top, bottom, t1), 1.0),
            ))
    }))
}

/// The glow at the top of pages without a cover, `width` by `height`
/// points, as the theme has it: Spotify's grey from the top, M3E's tonal
/// containers fading into the page, else the accent's and mauve's.
pub fn glow(window: &Window, theme: &Theme, width: f32, height: f32) -> Div {
    use crate::theme::Glow;
    use paint::Blob;
    let blob = |center, radii, color, alpha, fade| Blob {
        center,
        radii,
        color,
        alpha,
        fade,
    };
    let (blobs, fade_to_base) = match theme.layout.glow {
        Glow::Grey => (
            vec![blob((0.5, -0.6), (1.4, 1.3), theme.surface2, 0.55, 0.9)],
            false,
        ),
        // M3E's: the accent, the secondary and the tertiary containers
        // across the top, fading into the page.
        Glow::Tonal => (
            vec![
                blob(
                    (0.12, 0.0),
                    (0.60, 0.75),
                    mix(theme.accent, theme.base, 0.55),
                    0.9,
                    0.7,
                ),
                blob(
                    (0.52, 0.0),
                    (0.45, 0.65),
                    mix(theme.tonal.secondary_container, theme.base, 0.8),
                    0.9,
                    0.7,
                ),
                blob(
                    (0.92, 0.06),
                    (0.45, 0.70),
                    mix(theme.tonal.tertiary_container, theme.base, 0.7),
                    0.9,
                    0.7,
                ),
            ],
            true,
        ),
        _ => (
            vec![
                blob((0.08, -0.10), (0.55, 0.90), theme.accent, 0.26, 0.70),
                blob((0.62, -0.25), (0.45, 0.80), theme.mauve, 0.20, 0.72),
            ],
            false,
        ),
    };
    // Smooth as it is, a pixel in four either way is enough: the GPU
    // stretches it, blending as the mesh would.
    let pixels = |points: f32| ((points * zoom() * window.scale_factor()) / 4.0).ceil() as u32;
    place(0.0, 0.0, width, height).child(
        img(paint::wash(
            theme.base,
            &blobs,
            fade_to_base,
            pixels(width),
            pixels(height),
        ))
        .size_full()
        .object_fit(ObjectFit::Fill),
    )
}

/// A shape of what loads, `w` by `h` points at `x`, `y`: grey in Spotify's,
/// none in Apple Music's, whose spinner stands for them all (still to
/// come, as is the shapes' breathing).
pub fn loading_shape(theme: &Theme, x: f32, y: f32, w: f32, h: f32, radius: f32) -> Div {
    if theme.layout.motion != crate::theme::Motion::Spotify {
        return div();
    }
    place(x, y, w, h).rounded(z(radius)).bg(theme.hover())
}

/// How large a section's title is set, as the theme's headings are.
pub fn heading(theme: &Theme) -> (f32, Weight) {
    match theme.layout.headings {
        crate::theme::Headings::Small => (18.0, Weight::SemiBold),
        crate::theme::Headings::Large => (24.0, Weight::Bold),
    }
}

/// A section's title at `left`, its top at `top`, as egui's `section_head`.
pub fn section_head(theme: &Theme, left: f32, top: f32, title: &str) -> Div {
    let (size, weight) = heading(theme);
    text_at(
        left,
        top + 11.0,
        size,
        Align::Min,
        label_cut(title.to_owned(), size, weight, theme.text, 600.0),
    )
}

/// What a page says instead of what it couldn't show: a title, a line
/// under it.
pub fn message(theme: &Theme, left: f32, top: f32, width: f32, title: &str, detail: &str) -> Div {
    div()
        .absolute()
        .left_0()
        .top_0()
        .size_full()
        .child(text_at(
            left,
            top + 20.0,
            15.0,
            Align::Min,
            label_cut(title.to_owned(), 15.0, Weight::SemiBold, theme.text, width),
        ))
        .child(text_at(
            left,
            top + 44.0,
            13.0,
            Align::Min,
            label_cut(
                detail.to_owned(),
                13.0,
                Weight::Regular,
                theme.subtext0,
                width,
            ),
        ))
}

/// The bar over a page scrolled down, as egui's top bar turns solid: the
/// page's colour and its title.
pub fn scrolled_bar(theme: &Theme, width: f32, title: String) -> Div {
    place(0.0, 0.0, width, TOP_BAR)
        .bg(alpha(theme.base, 0.96))
        .child(hline(0.0, TOP_BAR - 0.5, width, theme.line()))
        .child(text_at(
            width / 2.0,
            TOP_BAR / 2.0,
            13.0,
            Align::Center,
            label_cut(title, 13.0, Weight::SemiBold, theme.text, 400.0),
        ))
}

/// egui's `Shadow { offset, blur, spread: 0, color }`.
pub fn shadow(offset_y: f32, blur: f32, color: Rgba) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: color.into(),
        offset: point(px(0.0), z(offset_y)),
        blur_radius: z(blur / 2.0),
        spread_radius: px(0.0),
        inset: false,
    }]
}

/// A card's corners painted over in `color`, so that it reads as a card
/// with `r` corners whatever was drawn in it: a ring `GAP` wide round it,
/// whose inner edge is the card's rounded one. The pointer goes through.
pub fn round_off(w: f32, h: f32, color: Rgba, r: f32) -> Div {
    place(-GAP, -GAP, w + 2.0 * GAP, h + 2.0 * GAP)
        .border(z(GAP))
        .border_color(color)
        .rounded(z(r + GAP))
}

/// A lit hairline `w` points wide at `x`, `y`.
pub fn hline(x: f32, y: f32, w: f32, color: Rgba) -> Div {
    place(x, y - 0.5, w, 1.0).bg(color)
}

/// How tall Spotify's bar over a scrolled page is.
pub const BANNER_BAR: f32 = 64.0;

/// Spotify's bar over a page scrolled past its banner, `shown` of the way
/// in: the page's colour, a round Play and the title, over the page. Its
/// Play plays the page, or pauses it.
pub fn banner_bar(
    cx: &mut gpui::Context<Music>,
    theme: &Theme,
    width: f32,
    color: Rgba,
    title: &str,
    shown: f32,
    playing: bool,
) -> Div {
    let bar = BANNER_BAR;
    let lit = mix(theme.accent, WHITE, 0.85);
    let (px, py) = (40.0 + 24.0, bar / 2.0);
    div()
        .absolute()
        .left_0()
        .top_0()
        .size_full()
        .opacity(shown)
        .child(
            // The bar takes the pointer over it, not the rows under it.
            place(0.0, 0.0, width, bar)
                .id("banner-bar")
                .block_mouse_except_scroll()
                .bg(mix(BLACK, color, 0.6)),
        )
        .child(
            disc(px, py, 24.0, theme.accent)
                .id("banner-bar-play")
                .cursor_pointer()
                .hover(move |s| s.bg(lit))
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    let tracks = page_tracks(this);
                    this.play_page(tracks);
                    this.acted(window, cx);
                }))
                .child(icon_at(
                    if playing {
                        icons::PAUSE_FILL
                    } else {
                        icons::PLAY_FILL
                    },
                    24.0,
                    24.0,
                    20.0,
                    theme.on_accent(),
                )),
        )
        .child(text_at(
            px + 24.0 + 12.0,
            py,
            24.0,
            Align::Min,
            label_cut(title.to_owned(), 24.0, Weight::Bold, WHITE, width - 200.0),
        ))
}

/// The cover colours behind album and playlist pages, fitted to the theme.
/// (An artist's come with their page.)
pub fn backdrop(app: &mut Music) -> Option<crate::palette::Palette> {
    let url = match &app.page {
        Page::Album(id) => match app.albums.get(id) {
            Some(crate::app::Loaded::Ready((album, _))) => album.artwork.as_ref()?.url(96),
            _ => return None,
        },
        Page::Playlist(id) => app.playlist(id)?.artwork.as_ref()?.url(96),
        _ => return None,
    };
    let light = app.theme.light;
    app.palette(&url)
        .map(|p| p.map(|c| crate::palette::fit_header(c, light)))
}

/// The songs the page shown lists, in its order: what its Play plays.
pub fn page_tracks(app: &mut Music) -> Vec<Track> {
    match app.page {
        Page::Songs => songs::shown_tracks(app),
        _ => detail::shown_tracks(app).to_vec(),
    }
}

/// What a row's click asked for, done by the page shown, whose list it is,
/// as each of egui's pages does the `RowAction` its rows return.
pub fn row_action(app: &mut Music, position: usize, action: tracks::RowAction) {
    match app.page {
        Page::Songs => songs::row_action(app, position, action),
        Page::Album(_) | Page::Playlist(_) => detail::row_action(app, position, action),
        _ => {}
    }
}

/// The window, as egui's `ui::show` places its panels: the window's bar,
/// the docked player, the sidebar, the side column, the page between.
pub fn show(app: &mut Music, window: &mut Window, cx: &mut gpui::Context<Music>) -> Div {
    let viewport = window.viewport_size();
    let (w, h) = (
        f32::from(viewport.width) / zoom(),
        f32::from(viewport.height) / zoom(),
    );
    let narrow = w < NARROW;
    let collapsed = app.sidebar_collapsed || narrow;
    let theme = app.theme.clone();
    let layout = theme.layout;
    let cards = layout.window != Window_::Flat;
    let radius = RADIUS;
    // Between the cards, and around them.
    let gap = if cards { GAP } else { 0.0 };
    let mut root = div()
        .relative()
        .size_full()
        .overflow_hidden()
        .bg(theme.crust)
        .font_family(crate::fonts::FAMILY)
        .text_color(theme.text);

    let window_bar = layout.top_bar != TopBar::Page;
    let bar_h = if window_bar { window_bar::HEIGHT } else { 0.0 };
    if window_bar {
        root = root.child(window_bar::show(app, window, cx, w));
    }
    // Under the window's bar, the cards keep only the gap.
    let top = if window_bar { bar_h } else { gap };
    let dock_h = if layout.player_bar == PlayerBar::Docked {
        dock::height(app)
    } else {
        0.0
    };
    let bottom = h - dock_h - gap;

    let library = layout.sidebar == Sidebar::Library;
    let side_w = match (library, collapsed) {
        (true, true) => library::COLLAPSED,
        (true, false) => library::WIDTH,
        (false, true) => 68.0,
        (false, false) => 232.0,
    };
    // The sidebar's card: Your Library (the other looks' sidebars are
    // still to come).
    let card =
        |x: f32, y: f32, cw: f32, ch: f32| place(x, y, cw, ch).rounded(z(radius)).bg(theme.base);
    let sidebar = card(gap, top, side_w, bottom - top).overflow_hidden();
    let sidebar = if library {
        sidebar.child(library::show(
            app,
            window,
            cx,
            side_w,
            bottom - top,
            collapsed,
        ))
    } else {
        sidebar
    };
    root = root.child(if cards {
        sidebar.child(round_off(side_w, bottom - top, theme.crust, radius))
    } else {
        sidebar
    });

    // The column beside the page: closed, a narrow card on the right with a
    // chevron to open it again; open, its space (its screens are still to
    // come).
    let side = layout.panel == Panel::Side && !narrow;
    let mut right = w;
    if side {
        let column = if app.side.is_none() {
            CLOSED_SIDE
        } else {
            SIDE_WIDTH
        };
        let x = w - gap - column;
        right = x;
        let (text, soft) = (theme.text, theme.text_a(0.8));
        let mut card = card(x, top, column, bottom - top).overflow_hidden();
        if app.side.is_none() {
            card = card.child(
                place(0.0, 0.0, column, bottom - top)
                    .id("side-open")
                    .group("side-open")
                    .cursor_pointer()
                    .child(
                        place(column / 2.0 - 10.0, (bottom - top) / 2.0 - 10.0, 20.0, 20.0).child(
                            gpui::svg()
                                .data(icons::LEFT.bytes())
                                .size(z(20.0))
                                .text_color(soft)
                                .group_hover("side-open", move |s| s.text_color(text)),
                        ),
                    ),
            );
        }
        root = root.child(if cards {
            card.child(round_off(column, bottom - top, theme.crust, radius))
        } else {
            card
        });
    }

    // The page's card.
    let x = gap + side_w + gap;
    let (main_w, main_h) = (right - gap - x, bottom - top);
    let pad = if narrow { 24.0 } else { 40.0 };
    let mut main = card(x, top, main_w, main_h).overflow_hidden();
    // The pages still to come leave the card empty.
    match app.page.clone() {
        Page::Songs => main = main.child(songs::show(app, window, cx, main_w, main_h, pad)),
        Page::Album(id) => {
            main = main.child(detail::album(app, window, cx, main_w, main_h, pad, &id))
        }
        Page::Playlist(id) => {
            main = main.child(detail::playlist(app, window, cx, main_w, main_h, pad, &id));
        }
        Page::LibraryArtist(id) => {
            main = main.child(library_artist::show(
                app, window, cx, main_w, main_h, pad, &id,
            ));
        }
        Page::Albums => main = main.child(grid::albums(app, window, cx, main_w, main_h, pad)),
        Page::Artists => main = main.child(grid::artists(app, window, cx, main_w, main_h, pad)),
        Page::RecentlyAdded => {
            main = main.child(grid::recently_added(app, window, cx, main_w, main_h, pad));
        }
        Page::Shelf(which, title) => {
            main = main.child(grid::shelf(
                app, window, cx, main_w, main_h, pad, which, &title,
            ));
        }
        _ => {}
    }
    root = root.child(if cards {
        main.child(round_off(main_w, main_h, theme.crust, radius))
    } else {
        main
    });

    if layout.player_bar == PlayerBar::Docked {
        root = root.child(dock::show(app, window, cx, w));
    }
    if app.dragging.is_some() {
        root = root.child(drag_listeners(cx));
    }
    root = root.child(mouse_navigation(cx));
    root.track_focus(&app.focus).on_key_down(cx.listener(
        |this, e: &gpui::KeyDownEvent, window, cx| {
            keys(this, &e.keystroke, window, cx);
        },
    ))
}

/// While a bar of the dock is dragged, the pointer is followed wherever it
/// goes in the window, and let go of anywhere, as egui's drags are.
fn drag_listeners(cx: &mut gpui::Context<Music>) -> impl IntoElement {
    let entity = cx.entity();
    gpui::canvas(
        |_, _, _| {},
        move |_, _, window, _| {
            let moved = entity.clone();
            window.on_mouse_event(move |e: &gpui::MouseMoveEvent, phase, window, cx| {
                if phase == gpui::DispatchPhase::Bubble {
                    let x = f32::from(e.position.x) / zoom();
                    moved.update(cx, |this, cx| this.drag_to(x, window, cx));
                }
            });
            let up = entity.clone();
            window.on_mouse_event(move |e: &gpui::MouseUpEvent, phase, window, cx| {
                if phase == gpui::DispatchPhase::Bubble && e.button == gpui::MouseButton::Left {
                    let x = f32::from(e.position.x) / zoom();
                    up.update(cx, |this, cx| this.end_drag(x, window, cx));
                }
            });
        },
    )
    .absolute()
    .size_full()
}

/// The mouse's back and forward buttons go back and forward between pages,
/// as in a browser: pressed anywhere in the window, as egui reads them.
fn mouse_navigation(cx: &mut gpui::Context<Music>) -> impl IntoElement {
    let entity = cx.entity();
    gpui::canvas(
        |_, _, _| {},
        move |_, _, window, _| {
            let entity = entity.clone();
            window.on_mouse_event(move |e: &gpui::MouseDownEvent, phase, _, cx| {
                use gpui::{MouseButton::Navigate, NavigationDirection};
                if phase != gpui::DispatchPhase::Bubble {
                    return;
                }
                entity.update(cx, |this, cx| match e.button {
                    Navigate(NavigationDirection::Back) => {
                        this.go_back();
                        cx.notify();
                    }
                    Navigate(NavigationDirection::Forward) => {
                        this.go_forward();
                        cx.notify();
                    }
                    _ => {}
                });
            });
        },
    )
    .absolute()
    .size_full()
}

/// How far the volume keys move it.
const VOLUME_STEP: f32 = 0.05;
/// How far Shift with the arrows moves in the song.
const SEEK_STEP: Duration = Duration::from_secs(10);

/// The keys, as egui's `shortcuts` and `player_shortcuts`: Space plays or
/// pauses; as Apple Music's, Ctrl with the arrows for the next or previous
/// song and the volume, Shift with them to move in the song, Ctrl+J to the
/// page playing; then the page's own.
fn keys(
    app: &mut Music,
    key: &gpui::Keystroke,
    window: &mut Window,
    cx: &mut gpui::Context<Music>,
) {
    let m = key.modifiers;
    let ctrl = m.control && !m.shift && !m.alt && !m.platform;
    let shift = m.shift && !m.control && !m.alt && !m.platform;
    match key.key.as_str() {
        "space" => app.toggle_play(),
        "right" if ctrl => app.next(),
        "left" if ctrl => app.previous(),
        "up" | "down" if ctrl => {
            let step = if key.key == "up" {
                VOLUME_STEP
            } else {
                -VOLUME_STEP
            };
            let shown = app.shown();
            let from = if shown.muted {
                0.0
            } else {
                shown.volume.unwrap_or(app.volume)
            };
            app.set_volume(from + step);
        }
        "m" if ctrl => app.toggle_mute(),
        "s" if ctrl => app.toggle_shuffle(),
        "r" if ctrl => app.cycle_repeat(),
        "j" if ctrl => app.go_to_playing(),
        // A live stream has no length, and doesn't move.
        "right" | "left" if shift => {
            let Some(length) = app.shown().duration else {
                return;
            };
            let at = app.shown().position;
            let to = if key.key == "right" {
                at + SEEK_STEP
            } else {
                at.saturating_sub(SEEK_STEP)
            };
            if to < length {
                app.seek(to);
            } else {
                app.next();
            }
        }
        _ if !m.modified() && app.page == Page::Songs => {
            if !songs::keyboard(app, &key.key) {
                return;
            }
        }
        _ if !m.modified() && matches!(app.page, Page::Album(_) | Page::Playlist(_)) => {
            if !detail::keyboard(app, &key.key) {
                return;
            }
        }
        _ => return,
    }
    cx.stop_propagation();
    app.acted(window, cx);
}

/// The side column's widths, as the egui window's `ui/side.rs` has them.
const SIDE_WIDTH: f32 = 340.0;
const CLOSED_SIDE: f32 = 36.0;

use crate::theme::{Panel, PlayerBar, Sidebar, TopBar, Window as Window_};
