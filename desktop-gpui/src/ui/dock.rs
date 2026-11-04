//! The player along the window's bottom, as Spotify's and the egui window's
//! `ui/dock.rs`: the song on the left with its quality and favourite, the
//! controls over the position in the middle, the panels, the devices and
//! the volume on the right. Here it shows what the last session left; its
//! buttons are drawn and lit under the pointer, but play nothing yet.

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, Context, Div, InteractiveElement, IntoElement, ParentElement, Styled, Window, div,
};
use music_core::model::FavouriteKind;

use crate::app::{Music, Side};
use crate::color::{WHITE, black_alpha};
use crate::fonts::Weight;
use crate::icons::{self, Icon};
use crate::theme::{Lyrics, Panel, Theme};

use super::{
    Align, artwork, disc, explicit_badge, format_time, icon_at, icon_button, label_cut, place,
    text_at, text_width, thumb_url, z,
};

pub const HEIGHT: f32 = 72.0;
/// How tall the bar is: taller by its strip while another device plays
/// (28 points), which this slice doesn't follow yet.
pub fn height(_app: &Music) -> f32 {
    HEIGHT
}

/// The bar, `width` points wide.
pub fn show(app: &Music, window: &mut Window, cx: &mut Context<Music>, width: f32) -> AnyElement {
    let theme = app.theme.clone();
    let h = HEIGHT;
    let third = (width * 0.3).min(420.0);
    // As Spotify's: about a quarter of the window.
    let middle_width = (width * 0.27)
        .min(width - 2.0 * third - 32.0)
        .clamp(240.0, 722.0);
    div()
        .absolute()
        .left_0()
        .bottom_0()
        .w(z(width))
        .h(z(h))
        .bg(theme.crust)
        // Clicks on the bar's background don't reach what is under it.
        .id("dock")
        .occlude()
        .child(now_playing(app, window, cx, &theme, 8.0, third - 8.0, h))
        .child(controls(
            app,
            &theme,
            width / 2.0 - middle_width / 2.0,
            middle_width,
        ))
        .child(right_side(
            app,
            &theme,
            width - third,
            third - 8.0,
            h,
            width < super::NARROW,
        ))
        .into_any_element()
}

/// The cover, the title, the artists and the quality, then the favourite,
/// in an area from `x`, `w` wide.
fn now_playing(
    app: &Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    x: f32,
    w: f32,
    h: f32,
) -> Div {
    let mut out = place(x, 0.0, w, h);
    let shown = app.shown();
    let Some(track) = shown.loading.clone().or(shown.track.clone()) else {
        return out;
    };
    let center = h / 2.0;
    // The cover opens Now Playing (still to come), as the floating bar's does.
    out = out.child(
        place(0.0, center - 28.0, 56.0, 56.0)
            .id("dock-art")
            .group("dock-art")
            .cursor_pointer()
            .child(artwork(
                window,
                cx,
                thumb_url(&track).as_deref(),
                56.0,
                4.0,
                theme,
            ))
            .child(
                place(0.0, 0.0, 56.0, 56.0)
                    .rounded(z(4.0))
                    .bg(black_alpha(115))
                    .invisible()
                    .group_hover("dock-art", |s| s.visible())
                    .child(icon_at(icons::EXPAND, 28.0, 28.0, 18.0, WHITE)),
            ),
    );
    let station = track.id.id.starts_with("ra.");
    let favourite = app.is_favourite(FavouriteKind::Song, &track.id);

    let left = 56.0 + 14.0;
    // Room for the favourite after the words.
    let width = (w - left - 44.0).max(0.0);
    let badge = super::quality_badge(&track);
    let lines = if badge.is_some() { 3.0 } else { 2.0 };
    let mut y = center - (lines - 1.0) * 10.0;

    // The title, which opens its album.
    let title_width = width - if track.explicit { 17.0 } else { 0.0 };
    let title_w = text_width(window, &track.title, 14.0, Weight::Medium).min(title_width);
    out = out.child(link(
        "dock-title",
        theme,
        left,
        y,
        14.0,
        Weight::Medium,
        track.title.clone(),
        title_width,
        theme.text,
        track.album_id.is_some(),
    ));
    if track.explicit {
        out = out.child(explicit_badge(theme, left + title_w + 5.0 + 6.0, y));
    }
    y += 20.0;

    // The artists, which open the first one's page.
    let artists = track.artists.join(", ");
    let artists_w = text_width(window, &artists, 12.0, Weight::Regular).min(width);
    out = out.child(link(
        "dock-artist",
        theme,
        left,
        y,
        12.0,
        Weight::Regular,
        artists,
        width,
        theme.subtext0,
        !station,
    ));
    y += 20.0;

    // Lossless or Dolby Atmos, in the accent; a click would tell the signal
    // path, still to come.
    if let Some(badge) = badge {
        out = out.child(text_at(
            left,
            y,
            11.0,
            Align::Min,
            label_cut(badge, 11.0, Weight::Medium, theme.accent, width),
        ));
    }

    // The favourite, as Spotify's saved mark.
    if !station {
        let words_end = left + width.min(title_w.max(artists_w));
        let (mx, my) = (words_end + 26.0, center);
        let (text, subtext) = (theme.text, theme.subtext0);
        let mark = place(mx - 16.0, my - 16.0, 32.0, 32.0)
            .id("dock-favourite")
            .group("dock-favourite")
            .cursor_pointer();
        out = out.child(if favourite {
            mark.child(disc(16.0, 16.0, 8.5, theme.accent).child(icon_at(
                icons::CHECK,
                8.5,
                8.5,
                11.0,
                theme.on_accent(),
            )))
        } else {
            mark.child(
                place(16.0 - 9.0, 16.0 - 9.0, 18.0, 18.0).child(
                    gpui::svg()
                        .data(icons::CIRCLE_PLUS.bytes())
                        .size(z(18.0))
                        .text_color(subtext)
                        .group_hover("dock-favourite", move |s| s.text_color(text)),
                ),
            )
        });
    }
    out
}

/// Text that leads somewhere: underlined under the pointer, as egui's
/// hairline under the galley; a link in `subtext` also lights up.
#[allow(clippy::too_many_arguments)]
fn link(
    id: &'static str,
    theme: &Theme,
    x: f32,
    y: f32,
    size: f32,
    weight: Weight,
    s: String,
    max: f32,
    color: gpui::Rgba,
    leads: bool,
) -> Div {
    let words = label_cut(s, size, weight, color, max);
    if !leads {
        return text_at(x, y, size, Align::Min, words);
    }
    let text = theme.text;
    text_at(
        x,
        y,
        size,
        Align::Min,
        div()
            .id(id)
            .group(id)
            .relative()
            .cursor_pointer()
            .child(words.group_hover(id, move |s| s.text_color(text)))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(z(crate::fonts::line_height(size) - 0.5))
                    .h(z(1.0))
                    .bg(text)
                    .invisible()
                    .group_hover(id, |s| s.visible()),
            ),
    )
}

/// Shuffle, previous, play, next and repeat over the position, in an area
/// from `x`, `w` wide.
fn controls(app: &Music, theme: &Theme, x: f32, w: f32) -> Div {
    let shown = app.shown();
    let has_track = shown.has_track;
    let dim = theme.subtext0;
    let y = 24.0;
    let mid = w / 2.0;
    let mut out = place(x, 0.0, w, HEIGHT);
    let mut buttons: Vec<AnyElement> = Vec::new();
    let mut button = |id: &'static str, dx: f32, icon: Icon, color, enabled: bool| {
        buttons.push(
            place(mid + dx - 16.0, y - 16.0, 32.0, 32.0)
                .child(icon_button(id, theme, icon, 32.0, 16.0, color, enabled))
                .into_any_element(),
        );
        if color == theme.accent {
            // Spotify's dot under a button that is on.
            buttons.push(disc(mid + dx, y + 13.0, 2.0, theme.accent).into_any_element());
        }
    };
    let shuffle = if shown.shuffle { theme.accent } else { dim };
    button("dock-shuffle", -88.0, icons::SHUFFLE, shuffle, true);
    button("dock-previous", -44.0, icons::SKIP_BACK, dim, has_track);
    button("dock-next", 44.0, icons::SKIP_FORWARD, dim, has_track);
    let (repeat_icon, repeat) = match shown.repeat {
        music_core::Repeat::Off => (icons::REPEAT, dim),
        music_core::Repeat::All => (icons::REPEAT, theme.accent),
        music_core::Repeat::One => (icons::REPEAT_ONE, theme.accent),
    };
    button("dock-repeat", 88.0, repeat_icon, repeat, true);
    out = out.children(buttons);

    // Play: a round button in the text's colour, a point larger under the
    // pointer.
    let fill = if has_track {
        theme.text
    } else {
        theme.text_a(0.4)
    };
    let icon = if shown.playing {
        icons::PAUSE_FILL
    } else {
        icons::PLAY_FILL
    };
    out = out.child(
        disc(mid, y, 16.0, fill)
            .id("dock-play")
            .when(has_track, |d| {
                d.cursor_pointer()
                    .hover(move |s| s.left(z(mid - 17.0)).top(z(y - 17.0)).size(z(34.0)))
            })
            .flex()
            .items_center()
            .justify_center()
            .child(icon.element(16.0, theme.base)),
    );

    // The position: the time played, the bar, the length.
    let y = 52.0;
    let small = 11.5;
    let (played, length) = match shown.duration {
        Some(total) if shown.track.is_some() => (format_time(shown.position), format_time(total)),
        None if shown.playing => (format_time(shown.position), "LIVE".into()),
        _ => ("-:--".into(), "-:--".into()),
    };
    out = out
        .child(text_at(
            32.0,
            y,
            small,
            Align::Max,
            label_cut(played, small, Weight::Regular, theme.subtext0, 60.0),
        ))
        .child(text_at(
            w - 32.0,
            y,
            small,
            Align::Min,
            label_cut(length, small, Weight::Regular, theme.subtext0, 60.0),
        ));
    let rail_w = w - 80.0;
    let fraction = match shown.duration.filter(|d| !d.is_zero()) {
        Some(total) => (shown.position.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0),
        None => 0.0,
    };
    out.child(bar(
        "dock-seek",
        theme,
        40.0,
        y,
        rail_w,
        fraction,
        shown.duration.is_some_and(|d| !d.is_zero()),
    ))
}

/// A 4 point bar `w` wide from `x`, centred on `y`, filled `fraction` of
/// the way: in the accent with a knob under the pointer, when `live`.
fn bar(
    id: &'static str,
    theme: &Theme,
    x: f32,
    y: f32,
    w: f32,
    fraction: f32,
    live: bool,
) -> AnyElement {
    let (accent, text) = (theme.accent, theme.text);
    let rail = place(x, y - 6.0, w, 12.0).child(
        place(0.0, 4.0, w, 4.0)
            .rounded(z(2.0))
            .bg(theme.text_a(0.3)),
    );
    if !live {
        return rail.into_any_element();
    }
    let group = format!("{id}-bar");
    let fill = w * fraction;
    rail.id(id)
        .group(group.clone())
        .cursor_pointer()
        .child(
            place(0.0, 4.0, fill, 4.0)
                .rounded(z(2.0))
                .bg(text)
                .group_hover(group.clone(), move |s| s.bg(accent)),
        )
        .child(
            disc(fill, 6.0, 6.0, text)
                .invisible()
                .group_hover(group, |s| s.visible()),
        )
        .into_any_element()
}

/// The panels' buttons, the devices, the volume, and Now Playing, ending at
/// the area's right, `w` wide from `x`.
fn right_side(app: &Music, theme: &Theme, x: f32, w: f32, h: f32, narrow: bool) -> Div {
    let layout = theme.layout;
    let side = layout.panel == Panel::Side && !narrow;
    let dim = theme.subtext0;
    let width = 32.0 * 4.0 + 6.0 + 84.0 + 8.0 + 32.0;
    let on = |yes: bool| if yes { theme.accent } else { dim };
    let lyrics_on = if layout.lyrics == Lyrics::Page {
        app.lyrics_page
    } else {
        false
    };
    let queue_on = side && matches!(app.side, Some(Side::Queue | Side::History));
    let shown = app.shown();
    let mute_icon = if shown.muted || shown.volume == Some(0.0) {
        icons::VOL_MUTE
    } else {
        icons::VOL
    };
    let devices = if shown.on.is_some() {
        theme.accent
    } else {
        theme.text_a(0.82)
    };
    let level = if shown.muted {
        0.0
    } else {
        shown.volume.unwrap_or(0.0)
    };
    let dot = |yes: bool| {
        div()
            .size(z(32.0))
            .relative()
            .when(yes, |d| d.child(disc(16.0, 32.0 + 1.0, 2.0, theme.accent)))
    };
    div()
        .absolute()
        .left(z(x + w - width))
        .top(z(h / 2.0 - 16.0))
        .w(z(width))
        .h(z(32.0))
        .flex()
        .items_center()
        .child(dot(lyrics_on).child(icon_button(
            "dock-lyrics",
            theme,
            icons::MIC,
            32.0,
            16.0,
            on(lyrics_on),
            true,
        )))
        .child(dot(queue_on).child(icon_button(
            "dock-queue",
            theme,
            icons::QUEUE,
            32.0,
            16.0,
            on(queue_on),
            true,
        )))
        .child(icon_button(
            "dock-devices",
            theme,
            icons::DEVICES,
            30.0,
            16.0,
            devices,
            true,
        ))
        .child(icon_button(
            "dock-mute",
            theme,
            mute_icon,
            32.0,
            16.0,
            dim,
            shown.volume.is_some(),
        ))
        .child(div().flex_none().w(z(6.0)))
        .child(
            div()
                .flex_none()
                .relative()
                .w(z(84.0))
                .h(z(16.0))
                .child(bar(
                    "dock-volume",
                    theme,
                    0.0,
                    8.0,
                    84.0,
                    level,
                    shown.volume.is_some(),
                )),
        )
        .child(div().flex_none().w(z(8.0)))
        .child(icon_button(
            "dock-full-screen",
            theme,
            if app.now_playing {
                icons::MINIMIZE
            } else {
                icons::EXPAND
            },
            32.0,
            15.0,
            dim,
            true,
        ))
}
