//! The bar across the window's top, as Spotify's and the egui window's
//! `ui/window_bar.rs`: back and forward on the left, Home and Search in the
//! middle, the account on the right.

use gpui::{
    ClickEvent, Context, Div, InteractiveElement, ParentElement, StatefulInteractiveElement,
    Styled, Window, div,
};

use crate::app::{Music, Page};
use crate::fonts::{Weight, line_height};
use crate::icons;

use super::{Align, artwork, disc, icon_at, icon_button, label, place, text_at, z};

pub const HEIGHT: f32 = 64.0;

/// The bar, `width` points wide.
pub fn show(app: &Music, window: &mut Window, cx: &mut Context<Music>, width: f32) -> Div {
    let theme = app.theme.clone();
    let y = HEIGHT / 2.0;
    let mut out = place(0.0, 0.0, width, HEIGHT).bg(theme.crust);

    // Back and forward, 4 points apart.
    out = out.child(
        place(16.0, y - 16.0, 80.0, 32.0)
            .flex()
            .items_center()
            .gap(z(4.0))
            .child(
                icon_button(
                    "window-back",
                    &theme,
                    icons::LEFT,
                    32.0,
                    20.0,
                    theme.subtext0,
                    app.can_go_back(),
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.go_back();
                    cx.notify();
                })),
            )
            .child(
                icon_button(
                    "window-forward",
                    &theme,
                    icons::RIGHT,
                    32.0,
                    20.0,
                    theme.subtext0,
                    app.can_go_forward(),
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.go_forward();
                    cx.notify();
                })),
            ),
    );

    // Home and Search, centred on the window.
    let field = (width - 2.0 * 160.0 - 56.0).clamp(220.0, 474.0);
    let start = width / 2.0 - (48.0 + 8.0 + field) / 2.0;
    let at_home = matches!(app.page, Page::Home | Page::New | Page::Radio) && !app.lyrics_page;
    let (text, subtext, lit) = (theme.text, theme.subtext0, theme.surface1);
    out = out.child(
        disc(start + 24.0, y, 24.0, theme.surface0)
            .id("window-home")
            .group("window-home")
            .cursor_pointer()
            .hover(move |s| s.bg(lit))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.go(Page::Home);
                cx.notify();
            }))
            .child(
                place(24.0 - 11.0, 24.0 - 11.0, 22.0, 22.0).child(
                    gpui::svg()
                        .data(icons::HOME.bytes())
                        .size(z(22.0))
                        .text_color(if at_home { text } else { subtext })
                        .group_hover("window-home", move |s| s.text_color(text)),
                ),
            ),
    );

    // The search field, as a pill. Typing in it, and its results, come with
    // the text field and the search page.
    let hairline = theme.text_a(0.12);
    let (sx, sw) = (start + 48.0 + 8.0, field);
    out = out.child(
        place(sx, y - 24.0, sw, 48.0)
            .id("window-search")
            .cursor_text()
            .rounded(z(24.0))
            .bg(theme.surface0)
            .border(z(1.0))
            .border_color(crate::color::TRANSPARENT)
            .hover(move |s| s.bg(lit).border_color(hairline))
            .child(icon_at(
                icons::SEARCH,
                26.0 - 1.0,
                24.0 - 1.0,
                20.0,
                theme.subtext0,
            ))
            // egui's hint takes the style's body text, 12.5 points, at the
            // top of a field as tall as a line at 15.
            .child(text_at(
                50.0 - 1.0,
                24.0 - 1.0 - line_height(15.0) / 2.0 + line_height(12.5) / 2.0,
                12.5,
                Align::Min,
                label(
                    "What do you want to play?",
                    12.5,
                    Weight::Regular,
                    theme.subtext0,
                ),
            )),
    );

    // The account: its picture, which opens Settings (whose page is still
    // to come).
    let (ax, ay) = (width - 16.0 - 24.0, y);
    let picture = match &app.profile_picture {
        Some(p) => artwork(window, cx, Some(&p.url(64)), 32.0, 16.0, &theme),
        None => avatar_disc(window, &theme),
    };
    out.child(
        disc(ax, ay, 24.0, theme.surface0)
            .id("window-account")
            .cursor_pointer()
            .hover(move |s| s.bg(lit))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.go(Page::Settings);
                cx.notify();
            }))
            .child(place(8.0, 8.0, 32.0, 32.0).child(picture)),
    )
}

/// The user's initial on the accent fading into its glow, 30 points round,
/// as the egui window's `sidebar::avatar_disc`, in a box 32 points square.
fn avatar_disc(window: &mut Window, theme: &crate::theme::Theme) -> Div {
    let name = std::env::var("USER").unwrap_or_default();
    let initial: String = name
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default();
    div().size(z(32.0)).relative().child(
        place(1.0, 1.0, 30.0, 30.0)
            .child(super::gradient(
                window,
                30.0,
                15.0,
                theme.accent,
                theme.mauve,
            ))
            .child(text_at(
                15.0,
                15.0,
                13.0,
                Align::Center,
                label(initial, 13.0, Weight::SemiBold, theme.on_accent()),
            )),
    )
}
