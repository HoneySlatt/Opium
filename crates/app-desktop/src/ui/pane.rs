//! M3E's side pane: Playing, Lyrics and Queue as connected segmented
//! buttons, then the song playing large with cards about it (its signal
//! path, its album, what comes next), its lyrics, or the queue.

use egui::{Color32, CornerRadius, Id, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{FavouriteKind, Track};

use crate::app::{App, Page, Side};
use crate::color::{alpha, mix};
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::motion::{Spring, spring};
use super::signal::Tone;
use super::{artwork, fading_artwork, icon_button, paint_icon, paint_text, text, thumb_url};

/// What the toolbar floating over the pane's bottom takes of it.
const UNDER_TOOLBAR: f32 = 110.0;

/// The pane's width in a window `window` wide.
pub fn width(window: f32) -> f32 {
    if window < 1280.0 { 320.0 } else { 360.0 }
}

pub fn show(app: &mut App, ui: &mut Ui, shows: Side) {
    let theme = app.theme.clone();
    let full = ui.max_rect();
    let head = Rect::from_min_max(
        pos2(full.left() + 20.0, full.top() + 14.0),
        pos2(full.right() - 14.0, full.top() + 54.0),
    );
    let tabs = Rect::from_min_max(head.min, pos2(head.right() - 40.0 - 8.0, head.bottom()));
    if let Some(tab) = segmented(ui, &theme, tabs, shows) {
        app.side = Some(tab);
    }
    let close = Rect::from_min_max(pos2(head.right() - 40.0, head.top()), head.max);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(close));
    if icon_button(
        &mut child,
        &theme,
        icons::X,
        40.0,
        20.0,
        theme.subtext0,
        true,
    )
    .on_hover_text("Close the panel")
    .clicked()
    {
        app.side = None;
    }

    let body = Rect::from_min_max(pos2(full.left(), head.bottom() + 14.0), full.max);
    let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
    body_ui.set_clip_rect(body.intersect(ui.clip_rect()));
    match shows {
        Side::NowPlaying => playing(app, &mut body_ui, &theme),
        Side::Lyrics => {
            let lines = Rect::from_min_max(body.min, body.max - vec2(0.0, UNDER_TOOLBAR - 24.0));
            super::lyrics::show(
                app,
                &mut body_ui,
                &theme,
                lines,
                24.0,
                lines.height(),
                "pane-lyrics",
            );
        }
        Side::Queue | Side::History => {
            let rows = Rect::from_min_max(body.min + vec2(12.0, 0.0), body.max - vec2(12.0, 0.0));
            let mut rows_ui = body_ui.new_child(egui::UiBuilder::new().max_rect(rows));
            super::drawer::queue(app, &mut rows_ui, &theme, "pane-queue");
        }
    }
}

/// The three tabs joined: the selected one in the accent and fully round,
/// the others with 8 px inner corners. Returns the tab clicked.
fn segmented(ui: &Ui, theme: &Theme, rect: Rect, shows: Side) -> Option<Side> {
    let selected = match shows {
        Side::History => Side::Queue,
        s => s,
    };
    let tabs = [
        (Side::NowPlaying, "Playing"),
        (Side::Lyrics, "Lyrics"),
        (Side::Queue, "Queue"),
    ];
    let w = (rect.width() - 2.0 * 2.0) / 3.0;
    let mut clicked = None;
    for (i, (tab, label)) in tabs.into_iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(rect.left() + i as f32 * (w + 2.0), rect.top()),
            vec2(w, rect.height()),
        );
        let on = tab == selected;
        let t = spring(
            ui,
            ("pane-tab", i),
            if on { 1.0 } else { 0.0 },
            Spring::Fast,
        )
        .clamp(0.0, 1.0);
        let full = rect.height() / 2.0;
        let corner = |outer: bool| {
            let base = if outer { full } else { 8.0 };
            (base + (full - base) * t).round() as u8
        };
        let (first, last) = (i == 0, i == tabs.len() - 1);
        let radius = CornerRadius {
            nw: corner(first),
            sw: corner(first),
            ne: corner(last),
            se: corner(last),
        };
        let response = ui
            .interact(r, Id::unique(("pane-tab", label)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let mut fill = mix(theme.accent, theme.surface0, t);
        if response.hovered() && !on {
            fill = mix(theme.text, fill, 0.08);
        }
        ui.painter().rect_filled(r, radius, fill);
        let ink = mix(theme.on_accent(), theme.subtext0, t);
        let g = text(ui, label, font(13.5, Weight::SemiBold), ink, w - 12.0);
        paint_text(ui, g, r.center().x, r.center().y, egui::Align::Center, ink);
        if response.clicked() && !on {
            clicked = Some(tab);
        }
    }
    clicked
}

/// The song playing: its cover, which shrinks a little when paused, its
/// title, then the cards.
fn playing(app: &mut App, ui: &mut Ui, theme: &Theme) {
    let shown = app.shown();
    let Some(track) = shown.loading.clone().or(shown.track.clone()) else {
        let g = text(
            ui,
            "Nothing playing",
            font(14.0, Weight::Regular),
            theme.subtext0,
            ui.available_width(),
        );
        let at = ui.max_rect().center_top() + vec2(0.0, 40.0);
        paint_text(ui, g, at.x, at.y, egui::Align::Center, theme.subtext0);
        return;
    };
    let next = app.upcoming_shown().next().cloned();
    let playing = shown.playing;
    egui::ScrollArea::vertical()
        .id_salt("pane-playing")
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let left = ui.max_rect().left() + 20.0;
            let width = ui.available_width() - 40.0;
            ui.add_space(4.0);

            // The cover: radius 28, and 94 % with radius 48 while paused.
            let (rect, _) = ui.allocate_exact_size(vec2(width + 40.0, width), Sense::hover());
            let t = spring(
                ui,
                "pane-cover",
                if playing { 1.0 } else { 0.0 },
                Spring::Slow,
            );
            let side = width * (0.94 + 0.06 * t);
            let radius = 48.0 + (28.0 - 48.0) * t.clamp(0.0, 1.0);
            let art =
                Rect::from_center_size(pos2(left + width / 2.0, rect.center().y), vec2(side, side));
            ui.painter().add(
                egui::Shadow {
                    offset: [0, 18],
                    blur: 40,
                    spread: 0,
                    color: Color32::from_black_alpha(102),
                }
                .as_shape(art, radius),
            );
            let url = track.artwork.as_ref().map(|a| a.url(720));
            fading_artwork(ui, "pane", url, art, radius, theme);

            ui.add_space(18.0);
            title(app, ui, theme, &track, left, width);
            ui.add_space(18.0);

            signal_card(app, ui, theme, &track, left, width);
            if let (Some(album), Some(id)) = (track.album.clone(), track.album_id.clone()) {
                let (_, inner) = card(ui, theme, left, width, "From the album", 64.0);
                if row(
                    ui,
                    theme,
                    inner,
                    thumb_url(&track),
                    16.0,
                    &album,
                    &track.artists.join(", "),
                    "pane-album",
                ) {
                    app.go(Page::Album(id));
                }
            }
            if let Some(next) = next {
                let (rect, inner) = card(ui, theme, left, width, "Next in queue", 56.0);
                // Open queue, a text button on the heading's right.
                let g = text(
                    ui,
                    "Open queue",
                    font(13.0, Weight::SemiBold),
                    theme.tonal.primary,
                    120.0,
                );
                let button = Rect::from_min_size(
                    pos2(
                        rect.right() - 18.0 - g.size().x - 24.0,
                        rect.top() + 18.0 + 8.0 - 14.0,
                    ),
                    vec2(g.size().x + 24.0, 28.0),
                );
                let hit = ui
                    .interact(button, Id::unique("pane-open-queue"), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if hit.hovered() {
                    ui.painter()
                        .rect_filled(button, 14.0, alpha(theme.tonal.primary, 0.08));
                }
                paint_text(
                    ui,
                    g,
                    button.center().x,
                    button.center().y,
                    egui::Align::Center,
                    theme.tonal.primary,
                );
                if hit.clicked() {
                    app.side = Some(Side::Queue);
                }
                let row_rect = Rect::from_min_size(inner.min, vec2(inner.width(), 56.0));
                if row(
                    ui,
                    theme,
                    row_rect,
                    thumb_url(&next),
                    12.0,
                    &next.title,
                    &next.artists.join(", "),
                    "pane-next",
                ) {
                    app.jump_to(0);
                }
            }
            ui.add_space(UNDER_TOOLBAR);
        });
}

/// The title, the artists under it, the heart on the right.
fn title(app: &mut App, ui: &mut Ui, theme: &Theme, track: &Track, left: f32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width + 40.0, 52.0), Sense::hover());
    let station = track.id.id.starts_with("ra.");
    let words = width - if station { 0.0 } else { 48.0 };
    let g = text(
        ui,
        &track.title,
        font(24.0, Weight::Bold),
        theme.text,
        words,
    );
    let title = Rect::from_min_size(pos2(left, rect.top()), g.size());
    let hit = ui.interact(title, Id::unique("pane-title"), Sense::click());
    if hit.hovered() && track.album_id.is_some() {
        ui.painter().hline(
            title.x_range(),
            title.bottom(),
            egui::Stroke::new(1.5, theme.text),
        );
    }
    paint_text(ui, g, left, title.center().y, egui::Align::Min, theme.text);
    if let Some(album) = &track.album_id
        && hit
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
    {
        app.go(Page::Album(album.clone()));
    }
    let artists = track.artists.join(", ");
    let g = text(
        ui,
        &artists,
        font(15.0, Weight::Regular),
        theme.subtext0,
        words,
    );
    let line = Rect::from_min_size(pos2(left, title.bottom() + 2.0), g.size());
    let hit = ui.interact(line, Id::unique("pane-artists"), Sense::click());
    let color = if hit.hovered() && !station {
        theme.text
    } else {
        theme.subtext0
    };
    paint_text(ui, g, left, line.center().y, egui::Align::Min, color);
    if !station
        && hit
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
    {
        let now = ui.input(|i| i.time);
        app.track_choice(track.clone(), crate::app::TrackChoice::GoToArtist, now);
    }
    if station {
        return;
    }
    let favourite = app.is_favourite(FavouriteKind::Song, &track.id);
    let mark = Rect::from_min_size(pos2(left + width - 40.0, rect.top()), vec2(40.0, 40.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(mark));
    let (icon, color) = if favourite {
        (icons::HEART_FILL, theme.tonal.primary)
    } else {
        (icons::HEART, theme.subtext0)
    };
    if icon_button(&mut child, theme, icon, 40.0, 22.0, color, true)
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

/// A card: `mantle`, radius 28, its heading in small letters, `body` high
/// under it. Returns the card and the room for its body.
fn card(
    ui: &mut Ui,
    theme: &Theme,
    left: f32,
    width: f32,
    heading: &str,
    body: f32,
) -> (Rect, Rect) {
    let height = 18.0 + 16.0 + 12.0 + body + 18.0;
    let (slot, _) = ui.allocate_exact_size(vec2(width + 40.0, height + 12.0), Sense::hover());
    let rect = Rect::from_min_size(pos2(left, slot.top()), vec2(width, height));
    ui.painter().rect_filled(rect, 28.0, theme.mantle);
    let g = text(
        ui,
        heading,
        font(13.0, Weight::SemiBold),
        theme.subtext0,
        width - 140.0,
    );
    paint_text(
        ui,
        g,
        rect.left() + 18.0,
        rect.top() + 18.0 + 8.0,
        egui::Align::Min,
        theme.subtext0,
    );
    let inner = Rect::from_min_max(
        pos2(rect.left() + 18.0, rect.top() + 18.0 + 16.0 + 12.0),
        pos2(rect.right() - 18.0, rect.bottom() - 18.0),
    );
    (rect, inner)
}

/// A cover, a title and a line under it; the whole row clickable.
#[allow(clippy::too_many_arguments)]
fn row(
    ui: &Ui,
    theme: &Theme,
    rect: Rect,
    url: Option<String>,
    radius: f32,
    title: &str,
    sub: &str,
    salt: &str,
) -> bool {
    let hit = ui
        .interact(rect.expand(6.0), Id::unique(salt), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if hit.hovered() {
        ui.painter()
            .rect_filled(rect.expand(6.0), 16.0, theme.hover());
    }
    let side = rect.height().min(64.0) - if radius < 16.0 { 12.0 } else { 0.0 };
    let cover = Rect::from_min_size(
        pos2(rect.left(), rect.center().y - side / 2.0),
        vec2(side, side),
    );
    artwork(ui, url, cover, radius, theme);
    let x = cover.right() + 14.0;
    let room = rect.right() - x;
    let g = text(ui, title, font(14.5, Weight::SemiBold), theme.text, room);
    paint_text(
        ui,
        g,
        x,
        rect.center().y - 9.0,
        egui::Align::Min,
        theme.text,
    );
    let g = text(ui, sub, font(13.0, Weight::Regular), theme.subtext0, room);
    paint_text(
        ui,
        g,
        x,
        rect.center().y + 10.0,
        egui::Align::Min,
        theme.subtext0,
    );
    hit.clicked()
}

/// The signal path: whether it is bit-perfect, why not, and each step as a
/// tonal tile joined to the next by a line.
fn signal_card(app: &App, ui: &mut Ui, theme: &Theme, track: &Track, left: f32, width: f32) {
    let assessment = super::signal::assess(app);
    let steps = super::signal::steps(app, track);
    let perfect = assessment.perfect();
    let reason = (!perfect).then(|| assessment.issues.join(" · "));
    let reason_g = reason.as_ref().map(|r| {
        ui.painter().layout(
            r.clone(),
            font(12.5, Weight::Regular),
            theme.yellow,
            width - 36.0,
        )
    });
    const STEP: f32 = 48.0;
    let body = reason_g.as_ref().map_or(0.0, |g| g.size().y + 10.0) + STEP * steps.len() as f32;
    let (rect, inner) = card(ui, theme, left, width, "Signal path", body);

    // The status on the heading's right.
    let (ok_fill, warn_fill) = (
        mix(theme.green, theme.mantle, 0.22),
        mix(theme.yellow, theme.mantle, 0.22),
    );
    let (fill, ink, icon, label) = if perfect {
        (ok_fill, theme.green, icons::CHECK, "Bit-perfect")
    } else {
        (warn_fill, theme.yellow, icons::VOL, "Not bit-perfect")
    };
    let g = text(ui, label, font(12.5, Weight::SemiBold), ink, 160.0);
    let chip = Rect::from_min_size(
        pos2(
            rect.right() - 18.0 - (12.0 + 14.0 + 6.0 + g.size().x + 12.0),
            rect.top() + 18.0 + 8.0 - 14.0,
        ),
        vec2(12.0 + 14.0 + 6.0 + g.size().x + 12.0, 28.0),
    );
    ui.painter().rect_filled(chip, 14.0, fill);
    paint_icon(
        ui,
        icon,
        pos2(chip.left() + 12.0 + 7.0, chip.center().y),
        14.0,
        ink,
    );
    paint_text(
        ui,
        g,
        chip.left() + 12.0 + 14.0 + 6.0,
        chip.center().y,
        egui::Align::Min,
        ink,
    );

    let mut y = inner.top();
    if let Some(g) = reason_g {
        let h = g.size().y;
        ui.painter()
            .galley(pos2(inner.left(), y - 4.0), g, theme.yellow);
        y += h + 10.0;
    }
    let tile = |y: f32| Rect::from_min_size(pos2(inner.left(), y + 6.0), vec2(36.0, 36.0));
    // The line first, under the tiles.
    if steps.len() > 1 {
        let x = inner.left() + 18.0;
        let from = tile(y).bottom();
        let to = tile(y + STEP * (steps.len() - 1) as f32).top();
        ui.painter().rect_filled(
            Rect::from_min_max(pos2(x - 1.0, from), pos2(x + 1.0, to)),
            0.0,
            theme.surface2,
        );
    }
    for step in &steps {
        let t = tile(y);
        let (fill, ink) = match step.tone {
            Tone::Neutral => (theme.surface1, theme.subtext0),
            Tone::Ok => (ok_fill, theme.green),
            Tone::Warn => (warn_fill, theme.yellow),
        };
        ui.painter().rect_filled(t, 12.0, fill);
        paint_icon(ui, step.icon, t.center(), 18.0, ink);
        let x = t.right() + 12.0;
        let room = inner.right() - x;
        let g = text(
            ui,
            &step.title,
            font(13.5, Weight::SemiBold),
            theme.text,
            room,
        );
        let line = step.lines.iter().find(|l| !l.is_empty());
        let dy = if line.is_some() { 9.0 } else { 0.0 };
        paint_text(ui, g, x, t.center().y - dy, egui::Align::Min, theme.text);
        if let Some(line) = line {
            let g = text(ui, line, font(12.5, Weight::Regular), theme.subtext0, room);
            paint_text(
                ui,
                g,
                x,
                t.center().y + 9.0,
                egui::Align::Min,
                theme.subtext0,
            );
        }
        y += STEP;
    }
}
