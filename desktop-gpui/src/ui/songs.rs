//! Library → Songs, as the egui window's `ui/songs.rs`: in Spotify's look,
//! the banner of its Liked Songs, the source menu, Downloaded and the
//! search on the right of its buttons, and the songs, the last added first,
//! only the rows in sight laid out.

use gpui::prelude::FluentBuilder;
use gpui::{
    ClickEvent, Context, Div, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div,
};
use music_core::model::{FavouriteKind, ProviderKind};

use crate::app::{Connection, Music};
use crate::color::{BLACK, WHITE, mix};
use crate::fonts::Weight;
use crate::icons;
use crate::theme::{Header, Motion, Theme};

use super::detail::{self, ACTIONS, BANNER, SONGS_BAND};
use super::tracks::{self, Columns, RowState};
use super::{
    Align, TOP_BAR, disc, hline, icon_at, icon_button, label, label_cut, place, text_at,
    text_width, z,
};

const TABLE_TOP: f32 = 179.0;

/// The page in the main card, `width` by `height` points.
pub fn show(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    width: f32,
    height: f32,
    pad: f32,
) -> Div {
    app.rebuild_order();
    let theme = app.theme.clone();
    // Spotify's layout shows the songs as its Liked Songs playlist.
    let banner = theme.layout.header == Header::Banner;
    // Spotify's Liked Songs, and M3E's songs as it: the last added first.
    let newest_only = banner || theme.expressive();
    if newest_only {
        app.want_newest_songs();
    }
    let table_offset = if banner { BANNER + ACTIONS } else { TABLE_TOP };
    let rows = app.songs_view.order.len();
    let foot = 18.0 + 16.0 + 18.0;
    let total = table_offset + tracks::HEAD_SPACE + rows as f32 * tracks::ROW + foot + 104.0;
    let offset = (-f32::from(app.songs_scroll.offset().y) / super::zoom()).max(0.0);

    let left = pad;
    let list_width = width - 2.0 * pad;
    let mut content = div().relative().w(z(width)).h(z(total));
    if banner {
        content = content.child(banner_header(app, window, cx, &theme, width, pad));
    }
    // The page title and its buttons of the other looks are their own pages'
    // business, still to come.

    let columns = Columns::new(0.0, list_width, true);
    let table_top = table_offset;
    // Spotify's look and M3E keep the last added first: their titles don't sort.
    let sort = (!newest_only).then_some(app.songs_view.sort);
    content = content.child(
        place(left, table_top, list_width, tracks::HEAD)
            .child(tracks::head(&theme, &columns, 0.0, list_width, sort, cx)),
    );
    let rows_top = table_top + tracks::HEAD_SPACE;

    if rows == 0 {
        content = content.child(empty(app, &theme, rows_top, left, list_width));
    } else {
        let range = tracks::visible(offset, offset + height, rows_top, rows);
        for position in range {
            let track = app.listed()[app.songs_view.order[position]].clone();
            let now = app.is_current(&track);
            let state = RowState {
                number: Some(position as u32 + 1),
                selected: app.songs_view.selected == Some(position),
                now,
                playing: now && app.is_playing(),
                favourite: app.is_favourite(FavouriteKind::Song, &track.id),
                download: app.badge(&track.id),
                unplayable: app.unplayable(&track),
                position,
            };
            let top = rows_top + position as f32 * tracks::ROW;
            content = content.child(
                div()
                    .absolute()
                    .left(z(left))
                    .top(z(top))
                    .child(tracks::row(
                        window, cx, &theme, &columns, list_width, &track, state,
                    )),
            );
        }
        let foot_text = if app.songs_view.downloaded {
            match rows {
                1 => "1 song downloaded".to_owned(),
                n => format!("{n} songs downloaded"),
            }
        } else if app.songs_done {
            format!("{rows} songs")
        } else {
            format!("{rows} songs so far, loading…")
        };
        content = content.child(text_at(
            left + 10.0,
            rows_top + rows as f32 * tracks::ROW + 26.0,
            12.0,
            Align::Min,
            label_cut(foot_text, 12.0, Weight::Regular, theme.muted(), list_width),
        ));
    }

    let mut page = div().absolute().left_0().top_0().size_full().child(
        div()
            .id("songs")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&app.songs_scroll)
            .child(content),
    );

    // The bar over the page once scrolled: the page's colour and title.
    if offset > 24.0 {
        page = page.child(
            place(0.0, 0.0, width, TOP_BAR)
                .bg(crate::color::alpha(theme.base, 0.96))
                .child(hline(0.0, TOP_BAR - 0.5, width, theme.line()))
                .child(text_at(
                    width / 2.0,
                    TOP_BAR / 2.0,
                    13.0,
                    Align::Center,
                    label("Songs", 13.0, Weight::SemiBold, theme.text),
                )),
        );
    }
    if banner {
        // Past the banner, Spotify's bar with its Play, and the column
        // titles kept under it.
        let bar = super::BANNER_BAR;
        let shown = ((offset - (BANNER - 2.0 * bar)) / 48.0).clamp(0.0, 1.0);
        if shown > 0.0 {
            let mut layer = banner_bar(
                &theme,
                width,
                SONGS_BAND,
                "Songs",
                shown,
                app.page_playing(),
            );
            if offset > table_offset - bar {
                layer = layer.child(
                    place(left - 16.0, bar, list_width + 32.0, tracks::HEAD).bg(mix(
                        theme.surface0,
                        theme.base,
                        0.5,
                    )),
                );
                layer = layer.child(
                    place(left, bar, list_width, tracks::HEAD)
                        .child(tracks::head(&theme, &columns, 0.0, list_width, None, cx)),
                );
            }
            page = page.child(layer);
        }
    }
    page
}

/// Spotify's bar over a page scrolled past its banner, `shown` of the way
/// in: the page's colour, a round Play and the title, over the page.
fn banner_bar(
    theme: &Theme,
    width: f32,
    color: gpui::Rgba,
    title: &str,
    shown: f32,
    playing: bool,
) -> Div {
    let bar = super::BANNER_BAR;
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

/// Spotify's banner over the songs, then on the right of its buttons the
/// search and, with more than one service, a menu of them.
fn banner_header(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    width: f32,
    pad: f32,
) -> Div {
    let count = app.songs_view.order.len();
    let (shuffle, here) = (app.shown().shuffle, app.page_playing());
    let mut out = detail::songs_banner(window, cx, theme, width, pad, count, shuffle, here);
    let right = width - pad;
    let top = BANNER + (ACTIONS - 32.0) / 2.0;
    // The page's search, closed: its magnifier. Typing in it comes with
    // the text field.
    let search_left = right - 32.0;
    out = out.child(place(search_left, top, 32.0, 32.0).child(icon_button(
        "page-search",
        theme,
        icons::SEARCH,
        32.0,
        15.0,
        theme.subtext0,
        true,
    )));
    let (chip, chip_left) = downloaded_chip(app, window, cx, theme, search_left - 16.0, top + 2.0);
    out = out.child(chip);
    let services = app.services_in_use();
    if services.len() < 2 {
        return out;
    }
    let name = match app.songs_view.source {
        None => "All services",
        Some(ProviderKind::AppleMusic) => "Apple Music",
        Some(ProviderKind::SoundCloud) => "SoundCloud",
        Some(ProviderKind::Navidrome) => "Navidrome",
        Some(ProviderKind::Local) => "Files",
        Some(ProviderKind::Music) => "Music",
    };
    let w = text_width(window, name, 14.0, Weight::Regular) + 8.0 + 14.0;
    let (text, subtext) = (theme.text, theme.subtext0);
    let x = chip_left - 16.0 - w;
    // Its menu is still to come.
    out.child(
        place(x, top, w, 32.0)
            .id("songs-services")
            .group("songs-services")
            .cursor_pointer()
            .child(text_at(
                0.0,
                16.0,
                14.0,
                Align::Min,
                label(name, 14.0, Weight::Regular, subtext)
                    .group_hover("songs-services", move |s| s.text_color(text)),
            ))
            .child(
                place(w - 14.0, 9.0, 14.0, 14.0).child(
                    gpui::svg()
                        .data(icons::DOWN.bytes())
                        .size(z(14.0))
                        .text_color(subtext)
                        .group_hover("songs-services", move |s| s.text_color(text)),
                ),
            ),
    )
}

/// The Downloaded chip, ending at `right`: a click shows only the songs
/// downloaded, or all again. Returns it and where it starts.
fn downloaded_chip(
    app: &Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    right: f32,
    top: f32,
) -> (impl IntoElement + use<>, f32) {
    let on = app.songs_view.downloaded;
    let w = text_width(window, "Downloaded", 13.0, Weight::Regular) + 24.0 + 19.0;
    let x = right - w;
    let (text, hover) = (theme.text, theme.hover());
    let (fg, icon) = if on {
        (theme.base, theme.base)
    } else {
        (theme.subtext0, theme.muted())
    };
    let chip = place(x, top, w, 28.0)
        .id("songs-downloaded")
        .group("songs-downloaded")
        .cursor_pointer()
        .rounded(z(14.0))
        .border(z(1.0))
        .border_color(if on { theme.text } else { theme.line() })
        .when(on, |c| c.bg(text))
        .when(!on, |c| c.hover(move |s| s.bg(hover)))
        .child(icon_at(
            icons::ARROW_DOWN,
            12.0 + 6.5 - 1.0,
            13.0,
            13.0,
            icon,
        ))
        .child(text_at(
            12.0 + 19.0 - 1.0,
            13.0,
            13.0,
            Align::Min,
            label("Downloaded", 13.0, Weight::Regular, fg).when(!on, |l| {
                l.group_hover("songs-downloaded", move |s| s.text_color(text))
            }),
        ))
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.songs_view.downloaded = !on;
            this.songs_view.selected = None;
            cx.notify();
        }));
    (chip, x)
}

fn empty(app: &Music, theme: &Theme, top: f32, left: f32, width: f32) -> Div {
    let (title, detail) = match &app.connection {
        _ if app.songs_view.downloaded && app.page_search.trim().is_empty() => (
            "No downloads yet",
            "Download a song, an album or a playlist from its menu to play it offline.".to_owned(),
        ),
        Connection::Connecting => ("Loading your library…", String::new()),
        Connection::NotSignedIn => (
            "Apple Music isn't connected",
            "Sign in from Settings, under your name at the bottom of the sidebar.".to_owned(),
        ),
        Connection::Failed(e) => ("Couldn't reach Apple Music", e.clone()),
        Connection::Connected if !app.songs_done => ("Loading your library…", String::new()),
        Connection::Connected if !app.page_search.trim().is_empty() => (
            "No songs found",
            format!("None match “{}”.", app.page_search.trim()),
        ),
        Connection::Connected => (
            "No songs here",
            "Nothing matches this source filter.".to_owned(),
        ),
    };
    if title.starts_with("Loading") {
        // Spotify's grey rows of what comes; Apple Music's spinner over
        // them is still to come.
        if theme.layout.motion == Motion::Spotify {
            return tracks::skeleton(theme, top, left, width, 8);
        }
        return div();
    }
    div()
        .absolute()
        .left_0()
        .top_0()
        .size_full()
        .child(text_at(
            left + 10.0,
            top + 50.0,
            15.0,
            Align::Min,
            label_cut(title, 15.0, Weight::SemiBold, theme.text, width.min(460.0)),
        ))
        .child(text_at(
            left + 10.0,
            top + 74.0,
            13.0,
            Align::Min,
            label_cut(detail, 13.0, Weight::Regular, theme.subtext0, width - 20.0),
        ))
}
