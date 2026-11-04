//! Spotify's banner over a page, as the egui window's `ui/detail.rs` draws
//! it: the part the Songs page uses (its Liked Songs tile, the title, the
//! user and the count; Play and Shuffle under it). Album and playlist pages
//! will use the rest.

use gpui::prelude::FluentBuilder;
use gpui::{Context, Div, InteractiveElement, ParentElement, Rgba, Styled, Window, div};

use crate::app::Music;
use crate::color::{BLACK, WHITE, black_alpha, mix, white_alpha};
use crate::fonts::Weight;
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::{
    Align, disc, icon_button, label_cut, loved_tile, place, shadow, text_at, text_width,
    vertical_gradient, z,
};

/// What the header shows.
pub struct Header {
    pub kind: &'static str,
    pub title: String,
    pub by: Option<String>,
    pub meta: Vec<(Option<Icon>, String)>,
    /// The library's songs: Spotify's Liked Songs tile for a cover.
    pub loved: bool,
}

/// The band behind Spotify's Liked Songs.
pub const SONGS_BAND: Rgba = crate::color::rgb(0x5038a0);

/// The banner's height, down to under the cover, in Spotify's layout.
pub const BANNER: f32 = 52.0 + 24.0 + 232.0 + 24.0;
/// The row of buttons under it.
pub const ACTIONS: f32 = 104.0;

/// The Songs page's banner, as Spotify's Liked Songs: its tile, Playlist,
/// the title, the user and the count; Play and Shuffle under it. In a box
/// `width` points wide, its top the page's.
#[allow(clippy::too_many_arguments)]
pub fn songs_banner(
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    width: f32,
    pad: f32,
    count: usize,
    shuffle: bool,
    here: bool,
) -> Div {
    let user = std::env::var("USER").unwrap_or_default();
    let mut chars = user.chars();
    let user: String = chars
        .next()
        .map(|f| f.to_uppercase().chain(chars).collect())
        .unwrap_or_default();
    let header = Header {
        kind: "Playlist",
        title: "Songs".into(),
        by: Some(user),
        meta: vec![(
            Some(icons::NOTE),
            format!("{count} TRACK{}", if count == 1 { "" } else { "S" }),
        )],
        loved: true,
    };
    banner_header(
        window, cx, theme, width, pad, &header, SONGS_BAND, shuffle, here,
    )
}

/// Spotify's header: a band of the cover's colour darkening downwards, the
/// cover at its bottom left, beside it what it is, the title in large type
/// and a line of details; under the band, a large round Play and the other
/// buttons on the colour fading into the page.
#[allow(clippy::too_many_arguments)]
fn banner_header(
    window: &mut Window,
    _cx: &mut Context<Music>,
    theme: &Theme,
    width: f32,
    pad: f32,
    header: &Header,
    color: Rgba,
    shuffle: bool,
    here: bool,
) -> Div {
    let shade = |c: Rgba, p: f32| mix(BLACK, c, p);
    let mut out = div()
        .absolute()
        .left_0()
        .top_0()
        .w(z(width))
        .h(z(BANNER + ACTIONS));
    // The band, rounded at the card's top as egui's corners painted over it.
    out = out.child(place(0.0, 0.0, width, BANNER).child(vertical_gradient(
        Some(width),
        BANNER,
        color,
        shade(color, 0.5),
    )));
    out = out.child(place(0.0, BANNER, width, 232.0).child(vertical_gradient(
        Some(width),
        232.0,
        mix(shade(color, 0.4), theme.base, 0.8),
        theme.base,
    )));

    let (ax, ay, side) = (pad, BANNER - 24.0 - 232.0, 232.0);
    let art = place(ax, ay, side, side)
        .rounded(z(4.0))
        .shadow(shadow(8.0, 24.0, black_alpha(130)));
    out = out.child(if header.loved {
        art.child(loved_tile(window, side))
    } else {
        art.bg(theme.surface0)
    });

    // What it is, the title as large as it fits, the details.
    let x = ax + side + 24.0;
    let room = (width - pad - x).max(0.0);
    let title_size = [96.0, 72.0, 56.0, 40.0, 32.0]
        .into_iter()
        .find(|&size| text_width(window, &header.title, size, Weight::Bold) <= room)
        .unwrap_or(32.0);
    let details = banner_details(header);
    let bottom = ay + side;
    let line_y = bottom - 10.0;
    // The artist (or the playlist's maker) first, with their photo.
    let mut lx = x;
    if let Some(by) = &header.by {
        let w = text_width(window, by, 14.0, Weight::Bold).min(room - (lx - x));
        out = out.child(text_at(
            lx,
            line_y,
            14.0,
            Align::Min,
            label_cut(by.clone(), 14.0, Weight::Bold, WHITE, room - (lx - x)),
        ));
        lx += w;
    }
    if !details.is_empty() {
        let line = if header.by.is_some() {
            format!(" • {details}")
        } else {
            details
        };
        let soft = white_alpha(200);
        out = out.child(text_at(
            lx,
            line_y,
            14.0,
            Align::Min,
            label_cut(line, 14.0, Weight::Regular, soft, (x + room - lx).max(0.0)),
        ));
    }
    let mut y = line_y - 12.0 - 8.0;
    let h = crate::fonts::line_height(title_size);
    out = out.child(text_at(
        x,
        y - h / 2.0,
        title_size,
        Align::Min,
        label_cut(header.title.clone(), title_size, Weight::Bold, WHITE, room),
    ));
    y -= h + 2.0;
    out = out.child(text_at(
        x,
        y - 8.0,
        14.0,
        Align::Min,
        label_cut(header.kind, 14.0, Weight::Medium, WHITE, room),
    ));

    // The buttons: Play, large and round in the accent, then the others.
    let mid = BANNER + ACTIONS / 2.0;
    let (px, py) = (pad + 28.0, mid);
    let lit = mix(theme.accent, WHITE, 0.85);
    // Play grows by two points and lightens under the pointer; its click
    // would play the page, which this slice doesn't.
    out = out.child(
        disc(px, py, 28.0, theme.accent)
            .id("banner-play")
            .cursor_pointer()
            .hover(move |s| s.bg(lit).left(z(px - 30.0)).top(z(py - 30.0)).size(z(60.0)))
            .child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        (if here {
                            icons::PAUSE_FILL
                        } else {
                            icons::PLAY_FILL
                        })
                        .element(24.0, theme.on_accent()),
                    ),
            ),
    );
    // Shuffle is a switch, green with a dot while on, as Spotify's.
    let bx = pad + 56.0 + 16.0;
    let color = if shuffle {
        theme.accent
    } else {
        theme.subtext0
    };
    out = out.child(place(bx, mid - 20.0, 40.0, 40.0).child(icon_button(
        "banner-shuffle",
        theme,
        icons::SHUFFLE,
        40.0,
        26.0,
        color,
        true,
    )));
    out.when(shuffle, |out| {
        out.child(disc(bx + 20.0, mid + 17.0, 2.0, theme.accent))
    })
}

/// The header's details as Spotify writes them: `2026 • 12 songs, 43 min
/// 7 sec`, then the quality's badge. No genre, as Spotify.
fn banner_details(header: &Header) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (icon, label) in &header.meta {
        if label.is_empty() || *icon == Some(icons::MIC) {
            continue;
        }
        let digits = label.starts_with(|c: char| c.is_ascii_digit());
        // A length of the details': said whole after the count instead.
        if *icon == Some(icons::CLOCK) && !label.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if digits && label.contains("TRACK") {
            parts.push(
                label
                    .to_lowercase()
                    .replace(" tracks", " songs")
                    .replace(" track", " song"),
            );
        } else if digits {
            parts.push(label.clone());
        } else {
            // LOSSLESS, DOLBY ATMOS: each word capitalised.
            let mut after_letter = false;
            parts.push(
                label
                    .chars()
                    .map(|c| {
                        let out = if after_letter {
                            c.to_lowercase().next().unwrap_or(c)
                        } else {
                            c
                        };
                        after_letter = c.is_alphabetic();
                        out
                    })
                    .collect(),
            );
        }
    }
    parts.join(" • ")
}
