//! M3E's Now Playing: over the cover's drifting colours, the cover on the
//! left (radius 48, a little smaller when paused) with the song, the large
//! wavy progress, the times round the quality and the controls round a
//! 96 px Play; on the right the lyrics or what plays next.

use egui::{Color32, CornerRadius, Id, Rect, Sense, Ui, pos2, vec2};
use opium_core::model::FavouriteKind;

use crate::app::{App, Loaded, Page};
use crate::color::alpha;
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::menu::{TrackFacts, track_menu_button};
use super::motion::{Spring, spring};
use super::{
    artwork, explicit_badge, fading_artwork, format_time, paint_icon, paint_text, text, thumb_url,
};

pub fn show(app: &mut App, ui: &mut Ui) {
    let theme = app.theme.clone();
    let full = ui.max_rect();
    ui.painter().rect_filled(full, 0.0, theme.base);
    super::now_playing::backdrop(app, ui, &theme, full);
    top_buttons(app, ui, &theme, full);

    let shown = app.shown();
    let Some(track) = shown.track.clone() else {
        let g = text(
            ui,
            "Nothing playing",
            font(18.0, Weight::SemiBold),
            theme.subtext0,
            full.width(),
        );
        paint_text(
            ui,
            g,
            full.center().x,
            full.center().y,
            egui::Align::Center,
            theme.subtext0,
        );
        return;
    };

    // Two columns, 1 : 1.1, 48 apart, inside 72 64 48; one when narrow.
    let inner = Rect::from_min_max(full.min + vec2(64.0, 72.0), full.max - vec2(64.0, 48.0));
    let narrow = full.width() < super::NARROW;
    let left_w = if narrow {
        inner.width()
    } else {
        (inner.width() - 48.0) / 2.1
    };
    let left = Rect::from_min_size(inner.min, vec2(left_w, inner.height()));
    let right = Rect::from_min_max(pos2(left.right() + 48.0, inner.top()), inner.max);

    // The cover and what goes under it, centred in the column.
    let side = left.width().min(full.height() * 0.52);
    let block_h = side + 28.0 + 72.0 + 40.0 + 24.0 + 18.0 + 96.0;
    let top = left.center().y - block_h / 2.0;
    let x = left.center().x - side / 2.0;
    let slot = Rect::from_min_size(pos2(x, top), vec2(side, side));
    let t = spring(
        ui,
        "np-cover",
        if shown.playing { 1.0 } else { 0.0 },
        Spring::Slow,
    );
    let art = Rect::from_center_size(slot.center(), slot.size() * (0.94 + 0.06 * t));
    ui.painter().add(
        egui::Shadow {
            offset: [0, 18],
            blur: 40,
            spread: 0,
            color: Color32::from_black_alpha(102),
        }
        .as_shape(art, 48.0),
    );
    fading_artwork(
        ui,
        "np-m3e",
        track.artwork.as_ref().map(|a| a.url(1000)),
        art,
        48.0,
        &theme,
    );

    // The title, the artist and the album as links, the heart.
    let mut y = slot.bottom() + 28.0;
    let room = side - 56.0 - 8.0;
    let title_room = room - if track.explicit { 20.0 } else { 0.0 };
    let g = text(
        ui,
        &track.title,
        font(32.0, Weight::Bold),
        theme.text,
        title_room,
    );
    let title_end = x + g.size().x;
    paint_text(ui, g, x, y + 19.0, egui::Align::Min, theme.text);
    if track.explicit {
        explicit_badge(ui, &theme, pos2(title_end + 8.0 + 6.0, y + 19.0));
    }
    let station = track.id.id.starts_with("ra.");
    let mut lx = x;
    let artists = track.artists.join(", ");
    let mut links: Vec<(String, Option<Page>)> = vec![(artists, None)];
    if let (Some(album), Some(id)) = (&track.album, &track.album_id) {
        links.push((album.clone(), Some(Page::Album(id.clone()))));
    }
    let ly = y + 38.0 + 14.0;
    for (k, (label, page)) in links.into_iter().enumerate() {
        if k > 0 {
            let g = text(
                ui,
                "  ·  ",
                font(17.0, Weight::Regular),
                theme.subtext0,
                40.0,
            );
            let w = g.size().x;
            paint_text(ui, g, lx, ly, egui::Align::Min, theme.subtext0);
            lx += w;
        }
        let g = text(
            ui,
            &label,
            font(17.0, Weight::Regular),
            theme.subtext0,
            (x + room - lx).max(0.0),
        );
        let link = Rect::from_min_size(pos2(lx, ly - g.size().y / 2.0), g.size());
        lx = link.right();
        let hit = ui.interact(link, Id::unique(("np-link", k)), Sense::click());
        let color = if hit.hovered() && !station {
            theme.text
        } else {
            theme.subtext0
        };
        if hit.hovered() && !station {
            ui.painter()
                .hline(link.x_range(), link.bottom(), egui::Stroke::new(1.0, color));
        }
        paint_text(ui, g, link.left(), ly, egui::Align::Min, color);
        if !station
            && hit
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
        {
            app.now_playing = false;
            match page {
                Some(page) => app.go(page),
                None => {
                    let now = ui.input(|i| i.time);
                    app.track_choice(track.clone(), crate::app::TrackChoice::GoToArtist, now);
                }
            }
        }
    }
    if !station {
        let favourite = app.is_favourite(FavouriteKind::Song, &track.id);
        let heart = Rect::from_min_size(pos2(x + side - 56.0, y + 8.0), vec2(56.0, 56.0));
        let (icon, color) = if favourite {
            (icons::HEART_FILL, theme.tonal.on_secondary_container)
        } else {
            (icons::HEART, theme.text)
        };
        let fill = if favourite {
            theme.tonal.secondary_container
        } else {
            theme.surface1
        };
        ui.painter().circle_filled(heart.center(), 28.0, fill);
        if super::fab::round(ui, &theme, heart, "np-heart", icon, 22.0, color)
            .on_hover_text(if favourite {
                "Remove from Favourites"
            } else {
                "Favourite"
            })
            .clicked()
        {
            app.toggle_favourite(FavouriteKind::Song, track.id.clone());
        }
    }
    y += 72.0;

    // The large wavy progress, the times and the quality under it.
    let duration = shown.duration.filter(|d| !d.is_zero());
    let wave = Rect::from_min_size(pos2(x, y), vec2(side, 40.0));
    super::toolbar::wavy(app, ui, &theme, wave, duration, shown.playing, true);
    y += 40.0;
    let position = super::player_bar::shown_position(app, ui);
    let small = font(12.5, Weight::Regular);
    let g = text(
        ui,
        &format_time(position),
        small.clone(),
        theme.subtext0,
        80.0,
    );
    paint_text(ui, g, x, y + 10.0, egui::Align::Min, theme.subtext0);
    let rest = match duration {
        Some(total) => format!("-{}", format_time(total.saturating_sub(position))),
        None => "LIVE".into(),
    };
    let g = text(ui, &rest, small, theme.subtext0, 80.0);
    paint_text(ui, g, x + side, y + 10.0, egui::Align::Max, theme.subtext0);
    let mut chip = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_center_size(
                // The chip's own 4 px above it.
                pos2(x + side / 2.0, y + 10.0 - 2.0),
                vec2(side - 160.0, 32.0),
            ))
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    super::toolbar::quality(app, &mut chip, &theme, &track);
    y += 24.0 + 18.0;

    // Shuffle, previous, Play, next, repeat.
    let mid = y + 48.0;
    let cx = x + side / 2.0;
    let dim = theme.text;
    let on = |yes: bool| if yes { theme.tonal.primary } else { dim };
    let play = Rect::from_center_size(pos2(cx, mid), vec2(96.0, 96.0));
    if super::fab::fab(
        ui,
        &theme,
        play,
        "np-play",
        shown.playing,
        shown.loading.is_some(),
        shown.has_track,
    )
    .clicked()
    {
        app.toggle_play();
    }
    if pill(
        ui,
        &theme,
        pos2(play.left() - 12.0, mid),
        true,
        "np-prev",
        icons::BACK_FILL,
    ) {
        app.previous();
    }
    if pill(
        ui,
        &theme,
        pos2(play.right() + 12.0, mid),
        false,
        "np-next",
        icons::FORWARD_FILL,
    ) {
        app.next();
    }
    let shuffle = Rect::from_center_size(
        pos2(play.left() - 12.0 - 84.0 - 8.0 - 24.0, mid),
        vec2(48.0, 48.0),
    );
    if super::fab::round(
        ui,
        &theme,
        shuffle,
        "np-shuffle",
        icons::SHUFFLE,
        22.0,
        on(shown.shuffle),
    )
    .on_hover_text("Shuffle")
    .clicked()
    {
        app.toggle_shuffle();
    }
    let (repeat_icon, repeat_on) = match shown.repeat {
        opium_core::Repeat::Off => (icons::REPEAT, false),
        opium_core::Repeat::All => (icons::REPEAT, true),
        opium_core::Repeat::One => (icons::REPEAT_ONE, true),
    };
    let repeat = Rect::from_center_size(
        pos2(play.right() + 12.0 + 84.0 + 8.0 + 24.0, mid),
        vec2(48.0, 48.0),
    );
    if super::fab::round(
        ui,
        &theme,
        repeat,
        "np-repeat",
        repeat_icon,
        22.0,
        on(repeat_on),
    )
    .on_hover_text("Repeat")
    .clicked()
    {
        app.cycle_repeat();
    }

    if !narrow {
        right_column(app, ui, &theme, right);
    }
}

/// Previous or next: a 72 × 56 pill, 84 wide and less round while pressed,
/// from `edge` (its right edge when `from_right`). Whether it was clicked.
fn pill(
    ui: &Ui,
    theme: &Theme,
    edge: egui::Pos2,
    from_right: bool,
    salt: &str,
    icon: Icon,
) -> bool {
    let id = Id::unique(("np-pill", salt));
    let base = |w: f32| {
        if from_right {
            Rect::from_min_max(pos2(edge.x - w, edge.y - 28.0), pos2(edge.x, edge.y + 28.0))
        } else {
            Rect::from_min_max(pos2(edge.x, edge.y - 28.0), pos2(edge.x + w, edge.y + 28.0))
        }
    };
    let response = ui
        .interact(base(72.0), id, Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let pressed = response.is_pointer_button_down_on();
    let w = spring(
        ui,
        ("np-pill-w", salt),
        if pressed { 84.0 } else { 72.0 },
        Spring::Fast,
    );
    let r = spring(
        ui,
        ("np-pill-r", salt),
        if pressed { 16.0 } else { 28.0 },
        Spring::Fast,
    );
    let rect = base(w);
    let mut fill = alpha(theme.text, 0.12);
    if response.hovered() {
        fill = alpha(theme.text, 0.18);
    }
    ui.painter().rect_filled(rect, r.clamp(0.0, 28.0), fill);
    paint_icon(ui, icon, rect.center(), 28.0, theme.text);
    response.clicked()
}

/// Close on the left; Play On and the song's menu on the right, as tonal
/// round buttons.
fn top_buttons(app: &mut App, ui: &mut Ui, theme: &Theme, full: Rect) {
    let close = Rect::from_min_size(full.min + vec2(24.0, 20.0), vec2(40.0, 40.0));
    ui.painter()
        .circle_filled(close.center(), 20.0, theme.surface1);
    if super::fab::round(ui, theme, close, "np-close", icons::DOWN, 22.0, theme.text)
        .on_hover_text("Close Now Playing")
        .clicked()
    {
        app.now_playing = false;
    }
    let more = Rect::from_min_size(
        pos2(full.right() - 24.0 - 40.0, full.top() + 20.0),
        vec2(40.0, 40.0),
    );
    let devices = more.translate(vec2(-48.0, 0.0));
    ui.painter()
        .circle_filled(devices.center(), 20.0, theme.surface1);
    let shown = app.shown();
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(devices));
    super::devices::button(app, &mut child, theme, &shown);
    let Some(track) = shown.track.clone().filter(|t| !t.id.id.starts_with("ra.")) else {
        return;
    };
    ui.painter()
        .circle_filled(more.center(), 20.0, theme.surface1);
    let open = super::fab::round(ui, theme, more, "np-more", icons::MORE, 20.0, theme.text)
        .on_hover_text("More");
    let facts = TrackFacts {
        file: track.id.provider == opium_core::model::ProviderKind::Local,
        has_album: track.album_id.is_some(),
        favourite: app.is_favourite(FavouriteKind::Song, &track.id),
        in_library: app.track_in_library(&track),
        removable: track.library_id.is_some(),
        in_playlist: false,
        download: app.download_offer(&track.id),
    };
    if let Some(choice) = track_menu_button(&open, theme, facts) {
        app.track_choice(track, choice, ui.input(|i| i.time));
    }
}

/// Lyrics or Up next, as two joined tabs, over them.
fn right_column(app: &mut App, ui: &mut Ui, theme: &Theme, column: Rect) {
    let tab_id = Id::unique("np-tab-queue");
    let mut queue = ui.data(|d| d.get_temp::<bool>(tab_id)).unwrap_or(false);
    let tabs = Rect::from_min_size(column.min, vec2(column.width().min(300.0), 40.0));
    let w = (tabs.width() - 2.0) / 2.0;
    for (k, label) in ["Lyrics", "Up next"].into_iter().enumerate() {
        let on = (k == 1) == queue;
        let r = Rect::from_min_size(
            pos2(tabs.left() + k as f32 * (w + 2.0), tabs.top()),
            vec2(w, 40.0),
        );
        let t = spring(ui, ("np-tab", k), if on { 1.0 } else { 0.0 }, Spring::Fast).clamp(0.0, 1.0);
        let corner = |outer: bool| {
            let base = if outer { 20.0 } else { 8.0 };
            (base + (20.0 - base) * t).round() as u8
        };
        let radius = CornerRadius {
            nw: corner(k == 0),
            sw: corner(k == 0),
            ne: corner(k == 1),
            se: corner(k == 1),
        };
        let response = ui
            .interact(r, Id::unique(("np-tab", label)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let fill = crate::color::mix(theme.text, alpha(theme.text, 0.10), t);
        ui.painter().rect_filled(r, radius, fill);
        let ink = crate::color::mix(theme.base, theme.subtext0, t);
        let g = text(ui, label, font(13.5, Weight::SemiBold), ink, w - 12.0);
        paint_text(ui, g, r.center().x, r.center().y, egui::Align::Center, ink);
        if response.clicked() {
            queue = k == 1;
        }
    }
    ui.data_mut(|d| d.insert_temp(tab_id, queue));

    let body = Rect::from_min_max(pos2(column.left(), tabs.bottom() + 18.0), column.max);
    let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
    body_ui.set_clip_rect(body.intersect(ui.clip_rect()));
    if !queue {
        let Some(track) = app.shown().track else {
            return;
        };
        if super::lyrics::ready(app).is_some() || matches!(app.lyrics(&track), Loaded::Loading) {
            super::lyrics::show(
                app,
                &mut body_ui,
                theme,
                body,
                34.0,
                body.height(),
                "np-m3e-lyrics",
            );
        } else {
            let g = text(
                &body_ui,
                "No lyrics for this song",
                font(18.0, Weight::Bold),
                theme.text,
                body.width(),
            );
            paint_text(
                &body_ui,
                g,
                body.left(),
                body.top() + 40.0,
                egui::Align::Min,
                theme.text,
            );
        }
        return;
    }
    up_next(app, &mut body_ui, theme);
}

/// What plays next, a row each: the cover, the title and the artists, the
/// length. A click plays it.
fn up_next(app: &mut App, ui: &mut Ui, theme: &Theme) {
    let upcoming: Vec<_> = app.upcoming_shown().take(30).cloned().collect();
    if upcoming.is_empty() {
        let g = text(
            ui,
            "Nothing next.",
            font(15.0, Weight::Regular),
            theme.subtext0,
            400.0,
        );
        let at = ui.max_rect().left_top() + vec2(8.0, 20.0);
        paint_text(ui, g, at.x, at.y, egui::Align::Min, theme.subtext0);
        return;
    }
    let mut jump = None;
    egui::ScrollArea::vertical()
        .id_salt("np-up-next")
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (k, track) in upcoming.iter().enumerate() {
                let (rect, response) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 68.0), Sense::click());
                if response.hovered() {
                    ui.painter().rect_filled(rect, 16.0, theme.hover());
                }
                let cover = Rect::from_min_size(
                    pos2(rect.left() + 8.0, rect.center().y - 26.0),
                    vec2(52.0, 52.0),
                );
                artwork(ui, thumb_url(track), cover, 12.0, theme);
                let tx = cover.right() + 12.0;
                let room = rect.right() - 8.0 - 48.0 - tx;
                let g = text(
                    ui,
                    &track.title,
                    font(16.0, Weight::Medium),
                    theme.text,
                    room,
                );
                paint_text(
                    ui,
                    g,
                    tx,
                    rect.center().y - 10.0,
                    egui::Align::Min,
                    theme.text,
                );
                let g = text(
                    ui,
                    &track.artists.join(", "),
                    font(13.5, Weight::Regular),
                    theme.subtext0,
                    room,
                );
                paint_text(
                    ui,
                    g,
                    tx,
                    rect.center().y + 11.0,
                    egui::Align::Min,
                    theme.subtext0,
                );
                if let Some(d) = track.duration {
                    let g = text(
                        ui,
                        &format_time(d),
                        font(13.0, Weight::Regular),
                        theme.subtext0,
                        48.0,
                    );
                    paint_text(
                        ui,
                        g,
                        rect.right() - 8.0,
                        rect.center().y,
                        egui::Align::Max,
                        theme.subtext0,
                    );
                }
                if response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    jump = Some(k);
                }
            }
            ui.add_space(24.0);
        });
    if let Some(k) = jump {
        app.jump_to(k);
    }
}
