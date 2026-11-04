//! Your Library, as Spotify's sidebar and the egui window's `ui/library.rs`:
//! the playlists, albums or artists of the library with their covers,
//! chosen by chips. Songs is pinned over the playlists as Spotify pins
//! Liked Songs, Recently Added over the albums. Collapsed, only the covers.

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Rgba,
    StatefulInteractiveElement, Styled, Window, div,
};
use music_core::model::ProviderKind;

use crate::app::{LibraryFilter as Filter, Music, Page};
use crate::color::{WHITE, alpha};
use crate::fonts::Weight;
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::{
    Align, artwork, disc, icon_at, icon_button, label, label_cut, place, text_at, text_width, z,
};

pub const WIDTH: f32 = 300.0;
pub const COLLAPSED: f32 = 72.0;

/// A row's picture.
enum Art {
    Cover(Option<String>),
    /// A pinned page's: its icon on a gradient.
    Tile(Icon, [Rgba; 2]),
}

struct Row {
    title: String,
    subtitle: String,
    art: Art,
    /// Artists are round.
    round: bool,
    page: Page,
}

/// The column, `width` by `height` points.
pub fn show(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    width: f32,
    height: f32,
    collapsed: bool,
) -> AnyElement {
    let theme = app.theme.clone();
    let filter = app.library_filter;
    let mut out = div()
        .absolute()
        .left_0()
        .top_0()
        .size_full()
        .id("library")
        .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
            this.library_hovered = *hovered;
            cx.notify();
        }));

    // The heading. Collapsed, the library's icon opens it again; open, a
    // button to fold it shows left of the title while the pointer is over
    // the library, as Spotify's.
    let head_h = 60.0;
    let mut title_left = 16.0;
    if collapsed || app.library_hovered {
        let (cx0, size, glyph, icon) = if collapsed {
            (width / 2.0, 40.0, 22.0, icons::LIBRARY)
        } else {
            (12.0 + 16.0, 32.0, 18.0, icons::PANEL)
        };
        let (text, subtext) = (theme.text, theme.subtext0);
        out = out.child(
            place(cx0 - size / 2.0, head_h / 2.0 - size / 2.0, size, size)
                .id("library-fold")
                .group("library-fold")
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    gpui::svg()
                        .data(icon.bytes())
                        .size(z(glyph))
                        .text_color(subtext)
                        .group_hover("library-fold", move |s| s.text_color(text)),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.sidebar_collapsed = !collapsed;
                    cx.notify();
                })),
        );
        if !collapsed {
            title_left = 12.0 + 32.0 + 6.0;
        }
    }
    if !collapsed {
        let color = theme.text_a(0.92);
        out = out.child(text_at(
            title_left,
            head_h / 2.0,
            16.0,
            Align::Min,
            label_cut("Your Library", 16.0, Weight::Bold, color, 200.0),
        ));
    }
    // New playlist (its window is still to come), and the grid of what the
    // chip shows.
    let (px, py) = if collapsed {
        (width / 2.0, head_h + 16.0)
    } else {
        (
            width
                - 16.0
                - 16.0
                - if filter == Filter::Playlists {
                    0.0
                } else {
                    40.0
                },
            head_h / 2.0,
        )
    };
    let lit = theme.surface2;
    out = out.child(
        disc(px, py, 16.0, theme.surface1)
            .id("library-plus-disc")
            .hover(move |s| s.bg(lit))
            .child(icon_button(
                "library-plus",
                &theme,
                icons::PLUS,
                32.0,
                16.0,
                theme.text,
                true,
            )),
    );
    if !collapsed && filter != Filter::Playlists {
        out = out.child(
            place(width - 32.0 - 16.0, head_h / 2.0 - 16.0, 32.0, 32.0).child(
                icon_button(
                    "library-grid",
                    &theme,
                    icons::GRID,
                    32.0,
                    16.0,
                    theme.subtext0,
                    true,
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.go(match filter {
                        Filter::Albums => Page::Albums,
                        _ => Page::Artists,
                    });
                    cx.notify();
                })),
            ),
        );
    }

    let mut top = head_h;
    if collapsed {
        top = py + 16.0 + 12.0;
    } else {
        let chips = [
            (Filter::Playlists, "Playlists"),
            (Filter::Albums, "Albums"),
            (Filter::Artists, "Artists"),
        ];
        let mut x = 16.0;
        for (kind, name) in chips {
            let w = text_width(window, name, 14.0, Weight::Regular) + 24.0;
            let on = filter == kind;
            let (fill, ink) = if on {
                (theme.text, theme.base)
            } else {
                (theme.surface1, theme.text)
            };
            let lit = theme.surface2;
            out = out.child(
                place(x, top, w, 32.0)
                    .id(name)
                    .cursor_pointer()
                    .rounded(z(16.0))
                    .bg(fill)
                    .when(!on, |c| c.hover(move |s| s.bg(lit)))
                    .child(text_at(
                        w / 2.0,
                        16.0,
                        14.0,
                        Align::Center,
                        label(name, 14.0, Weight::Regular, ink),
                    ))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.library_filter = kind;
                        cx.notify();
                    })),
            );
            x += w + 8.0;
        }
        top += 32.0 + 12.0;
    }

    // The rows, only those in sight laid out.
    let pinned_count = pinned(app, filter).len();
    let listed = match filter {
        Filter::Playlists => app.playlists.len(),
        // The library's albums and artists come with their pages.
        Filter::Albums | Filter::Artists => 0,
    };
    let row_height = if collapsed { 56.0 } else { 64.0 };
    // As egui's `show_rows`: each row at its own place, `row_height` apart
    // to the fraction of a point, those in sight laid out. (GPUI's
    // `uniform_list` would space them by their height snapped to the
    // device's pixels, drifting from egui's row after row.)
    let list_h = (height - top).max(0.0);
    let count = pinned_count + listed;
    let scroll = &app.library_scroll[filter as usize];
    let offset = (-f32::from(scroll.offset().y) / super::zoom()).max(0.0);
    let first = (offset / row_height).floor() as usize;
    let last = (((offset + list_h) / row_height).ceil() as usize).min(count);
    let mut pins: Vec<Option<Row>> = pinned(app, filter).into_iter().map(Some).collect();
    let mut rows = div().relative().w(z(width)).h(z(count as f32 * row_height));
    for i in first.min(last)..last {
        let row = match i.checked_sub(pins.len()) {
            None => pins[i].take().expect("each pinned row once"),
            Some(k) => listed_row(app, k),
        };
        rows = rows.child(
            div()
                .absolute()
                .left_0()
                .top(z(i as f32 * row_height))
                .child(row_ui(
                    app, window, cx, &theme, row, i, width, row_height, collapsed,
                )),
        );
    }
    out.child(
        place(0.0, top, width, list_h).child(
            div()
                .id(("library-rows", filter as usize))
                .size_full()
                .overflow_y_scroll()
                .track_scroll(scroll)
                .child(rows),
        ),
    )
    .into_any_element()
}

/// The pages pinned over the chip's list.
fn pinned(app: &Music, filter: Filter) -> Vec<Row> {
    let tile = |title: &str, subtitle: String, icon, colors, page| Row {
        title: title.into(),
        subtitle,
        art: Art::Tile(icon, colors),
        round: false,
        page,
    };
    const PLAIN: [Rgba; 2] = [crate::color::rgb(0x2a2a2a), crate::color::rgb(0x505050)];
    match filter {
        Filter::Playlists => {
            // The library's songs are Spotify's Liked Songs; Apple's own
            // playlist of favourites is listed with the others.
            vec![tile(
                "Songs",
                if app.songs.is_empty() && !app.songs_done {
                    "Playlist".to_owned()
                } else {
                    format!("Playlist · {} songs", app.songs.len())
                },
                icons::HEART,
                super::LOVED,
                Page::Songs,
            )]
        }
        Filter::Albums => vec![tile(
            "Recently Added",
            "Library".into(),
            icons::CLOCK,
            PLAIN,
            Page::RecentlyAdded,
        )],
        Filter::Artists => Vec::new(),
    }
}

/// The `i`th of the library's playlists, as the Playlists chip lists them
/// (the albums' and artists' rows come with their pages).
fn listed_row(app: &Music, i: usize) -> Row {
    let p = &app.playlists[i];
    let by = if p.editable {
        capitalised_user()
    } else {
        p.curator
            .clone()
            .unwrap_or_else(|| service(p.id.provider).into())
    };
    Row {
        title: p.name.clone(),
        subtitle: format!("Playlist · {by}"),
        art: Art::Cover(p.artwork.as_ref().map(|a| a.url(120))),
        round: false,
        page: Page::Playlist(p.id.clone()),
    }
}

#[allow(clippy::too_many_arguments)]
fn row_ui(
    app: &Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    row: Row,
    i: usize,
    width: f32,
    height: f32,
    collapsed: bool,
) -> impl IntoElement + use<> {
    // The row's own rect, 8 points in from either side.
    let w = width - 16.0;
    let active = app.page == row.page && !app.lyrics_page;
    let (hover, press) = (theme.hover(), theme.press());
    let mut inner = div()
        .id(("library-row", i))
        .relative()
        .w(z(w))
        .h(z(height))
        .ml(z(8.0))
        .rounded(z(6.0))
        .cursor_pointer()
        .when(active, |r| r.bg(alpha(theme.text, 0.1)))
        .when(!active, |r| r.hover(move |s| s.bg(hover)))
        .active(move |s| s.bg(press))
        .on_click({
            let page = row.page.clone();
            cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.go(page.clone());
                cx.notify();
            })
        });
    let size = 48.0;
    let lead = if collapsed { w / 2.0 } else { 8.0 + size / 2.0 };
    let radius = if row.round { size / 2.0 } else { 4.0 };
    let art = div()
        .absolute()
        .left(z(lead - size / 2.0))
        .top(z(height / 2.0 - size / 2.0));
    inner = inner.child(match &row.art {
        Art::Cover(url) => art.child(artwork(window, cx, url.as_deref(), size, radius, theme)),
        Art::Tile(icon, [from, to]) => art.child(
            super::gradient(window, size, radius, *from, *to)
                .relative()
                .child(icon_at(*icon, size / 2.0, size / 2.0, 20.0, WHITE)),
        ),
    });
    if collapsed {
        return div().w(z(width)).h(z(height)).child(inner);
    }
    let left = lead + size / 2.0 + 12.0;
    let room = w - left - 8.0;
    // The page playing from shows in the accent, as Spotify's.
    let playing = app.is_playing() && app.playing_from() == Some(&row.page);
    let title_color = if playing { theme.accent } else { theme.text };
    inner = inner
        .child(text_at(
            left,
            height / 2.0 - 10.0,
            15.0,
            Align::Min,
            label_cut(row.title, 15.0, Weight::Regular, title_color, room),
        ))
        .child(text_at(
            left,
            height / 2.0 + 11.0,
            13.0,
            Align::Min,
            label_cut(row.subtitle, 13.0, Weight::Regular, theme.subtext0, room),
        ));
    div().w(z(width)).h(z(height)).child(inner)
}

pub fn service(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::AppleMusic => "Apple Music",
        ProviderKind::SoundCloud => "SoundCloud",
        ProviderKind::Navidrome => "Navidrome",
        ProviderKind::Local => "Files",
        ProviderKind::Music => "Music",
    }
}

fn capitalised_user() -> String {
    let name = std::env::var("USER").unwrap_or_default();
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
