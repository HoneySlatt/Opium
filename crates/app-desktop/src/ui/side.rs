//! The column beside the page, as Spotify's: the song playing, large, with
//! its signal, its album and what plays next; or the queue and what played.

use egui::{Id, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{FavouriteKind, Track};

use crate::app::{App, Page, Side};
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::{artwork, icon_button, paint_icon, paint_text, text, thumb_url};

pub const WIDTH: f32 = 340.0;

pub fn show(app: &mut App, ui: &mut Ui, shows: Side) {
    let theme = app.theme.clone();
    let full = ui.max_rect();
    let head = Rect::from_min_size(full.min, vec2(full.width(), 60.0));

    // The heading: where the song plays from, or the queue's tabs.
    match shows {
        Side::NowPlaying => {
            let title = app
                .playing_from()
                .cloned()
                .map(|p| app.title_of(&p))
                .filter(|t| !t.is_empty())
                .or_else(|| app.shown().track.and_then(|t| t.album))
                .unwrap_or_default();
            let g = text(
                ui,
                &title,
                font(16.0, Weight::Bold),
                theme.text,
                head.width() - 80.0,
            );
            let rect = Rect::from_min_size(
                pos2(head.left() + 16.0, head.center().y - g.size().y / 2.0),
                g.size(),
            );
            let hit = ui.interact(rect, Id::unique("side-from"), Sense::click());
            let color = theme.text;
            if hit.hovered() {
                ui.painter()
                    .hline(rect.x_range(), rect.bottom(), egui::Stroke::new(1.0, color));
            }
            paint_text(ui, g, rect.left(), rect.center().y, egui::Align::Min, color);
            if hit
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                app.go_to_playing();
            }
        }
        Side::Queue | Side::History => {
            let mut x = head.left() + 16.0;
            for (tab, label) in [(Side::Queue, "Queue"), (Side::History, "Recently played")] {
                let on = tab == shows;
                let w = text(ui, label, font(16.0, Weight::Bold), theme.text, 200.0)
                    .size()
                    .x;
                let rect = Rect::from_min_size(pos2(x, head.center().y - 14.0), vec2(w, 28.0));
                x = rect.right() + 20.0;
                let hit = ui
                    .interact(rect, Id::unique(("side-tab", label)), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                let color = if on || hit.hovered() {
                    theme.text
                } else {
                    theme.subtext0
                };
                let g = text(ui, label, font(16.0, Weight::Bold), color, 200.0);
                paint_text(ui, g, rect.left(), rect.center().y, egui::Align::Min, color);
                if on {
                    ui.painter().hline(
                        rect.x_range(),
                        rect.bottom() + 2.0,
                        egui::Stroke::new(2.0, theme.accent),
                    );
                }
                if hit.clicked() {
                    app.side = Some(tab);
                }
            }
        }
    }
    let close = Rect::from_center_size(
        pos2(head.right() - 16.0 - 16.0, head.center().y),
        vec2(32.0, 32.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(close));
    if icon_button(
        &mut child,
        &theme,
        icons::X,
        32.0,
        16.0,
        theme.subtext0,
        true,
    )
    .on_hover_text("Close")
    .clicked()
    {
        app.side = None;
    }

    let body = Rect::from_min_max(
        pos2(full.left() + 8.0, head.bottom()),
        full.max - vec2(8.0, 0.0),
    );
    let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
    body_ui.set_clip_rect(body.intersect(ui.clip_rect()));
    match shows {
        Side::NowPlaying => now_playing(app, &mut body_ui, &theme),
        Side::Queue => super::drawer::queue(app, &mut body_ui, &theme, "side-queue"),
        Side::History => {
            egui::ScrollArea::vertical()
                .id_salt("side-history")
                .auto_shrink(false)
                .show(&mut body_ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    super::drawer::history(app, ui, &theme);
                });
        }
    }
}

/// The song playing, large, then cards about it.
fn now_playing(app: &mut App, ui: &mut Ui, theme: &Theme) {
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
    egui::ScrollArea::vertical()
        .id_salt("side-now")
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let width = ui.available_width() - 8.0;
            let left = ui.max_rect().left() + 8.0;

            // The cover, as wide as the column.
            let (rect, _) = ui.allocate_exact_size(vec2(width + 8.0, width + 16.0), Sense::hover());
            let art = Rect::from_min_size(pos2(left, rect.top()), vec2(width - 8.0, width - 8.0));
            let url = track.artwork.as_ref().map(|a| a.url(720));
            artwork(ui, url, art, 8.0, theme);

            // Title and artists, the favourite beside them.
            let (rect, _) = ui.allocate_exact_size(vec2(width + 8.0, 64.0), Sense::hover());
            let words = width - 8.0 - 40.0;
            let g = text(
                ui,
                &track.title,
                font(22.0, Weight::Bold),
                theme.text,
                words,
            );
            let title = Rect::from_min_size(pos2(left, rect.top() + 4.0), g.size());
            let hit = ui.interact(title, Id::unique("side-title"), Sense::click());
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
            let g = text(
                ui,
                &track.artists.join(", "),
                font(15.0, Weight::Regular),
                theme.subtext0,
                words,
            );
            let artists = Rect::from_min_size(pos2(left, title.bottom() + 4.0), g.size());
            let hit = ui.interact(artists, Id::unique("side-artists"), Sense::click());
            let station = track.id.id.starts_with("ra.");
            let color = if hit.hovered() && !station {
                theme.text
            } else {
                theme.subtext0
            };
            let g = text(
                ui,
                &track.artists.join(", "),
                font(15.0, Weight::Regular),
                color,
                words,
            );
            paint_text(ui, g, left, artists.center().y, egui::Align::Min, color);
            if !station
                && hit
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
            {
                let now = ui.input(|i| i.time);
                app.track_choice(track.clone(), crate::app::TrackChoice::GoToArtist, now);
            }
            if !track.id.id.starts_with("ra.") {
                let favourite = app.is_favourite(FavouriteKind::Song, &track.id);
                let mark = Rect::from_center_size(
                    pos2(left + width - 8.0 - 14.0, title.center().y + 6.0),
                    vec2(32.0, 32.0),
                );
                let hit = ui
                    .interact(mark, Id::unique("side-favourite"), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(if favourite {
                        "Remove from Favourites"
                    } else {
                        "Favourite"
                    });
                if favourite {
                    ui.painter()
                        .circle_filled(mark.center(), 10.0, theme.accent);
                    paint_icon(ui, icons::CHECK, mark.center(), 13.0, theme.on_accent());
                } else {
                    let color = if hit.hovered() {
                        theme.text
                    } else {
                        theme.subtext0
                    };
                    paint_icon(ui, icons::CIRCLE_PLUS, mark.center(), 21.0, color);
                }
                if hit.clicked() {
                    app.toggle_favourite(FavouriteKind::Song, track.id.clone());
                }
            }
            ui.add_space(12.0);

            // How it reaches the ears.
            if let Some((label, _)) = super::quality_label(&track) {
                let badge = super::quality_badge(&track);
                let detail = match badge {
                    Some(badge) => format!("{badge} · {label}"),
                    None => label,
                };
                let open = card(
                    ui,
                    theme,
                    width,
                    "Signal",
                    "Signal path",
                    64.0,
                    |ui, inner| {
                        let y = inner.top() + 14.0;
                        let mut x = inner.left();
                        if super::plays_in_atmos(&track) {
                            super::dolby_logo(ui, pos2(x + 9.0, y), 11.0, theme.text);
                            x += 24.0;
                        }
                        let g = text(
                            ui,
                            &detail,
                            font(14.0, Weight::Medium),
                            theme.text,
                            inner.right() - x,
                        );
                        paint_text(ui, g, x, y, egui::Align::Min, theme.text);
                    },
                );
                if open {
                    app.signal_open = Some(crate::app::SignalAnchor::PlayerBar);
                }
            }

            // The album it is from.
            if let (Some(album), Some(id)) = (track.album.clone(), track.album_id.clone()) {
                let open = card(
                    ui,
                    theme,
                    width,
                    "From the album",
                    "Open album",
                    72.0,
                    |ui, inner| {
                        let cover = Rect::from_min_size(inner.min, vec2(40.0, 40.0));
                        artwork(ui, thumb_url(&track), cover, 4.0, theme);
                        let x = cover.right() + 12.0;
                        let room = inner.right() - x;
                        let g = text(ui, &album, font(14.0, Weight::Medium), theme.text, room);
                        paint_text(
                            ui,
                            g,
                            x,
                            cover.center().y - 9.0,
                            egui::Align::Min,
                            theme.text,
                        );
                        let g = text(
                            ui,
                            &track.artists.join(", "),
                            font(12.5, Weight::Regular),
                            theme.subtext0,
                            room,
                        );
                        paint_text(
                            ui,
                            g,
                            x,
                            cover.center().y + 10.0,
                            egui::Align::Min,
                            theme.subtext0,
                        );
                    },
                );
                if open {
                    app.go(Page::Album(id));
                }
            }

            // What plays next.
            if let Some(next) = next {
                let open = card(
                    ui,
                    theme,
                    width,
                    "Next in queue",
                    "Open queue",
                    72.0,
                    |ui, inner| {
                        next_row(ui, theme, inner, &next);
                    },
                );
                if open {
                    app.side = Some(Side::Queue);
                }
            }
            ui.add_space(16.0);
        });
}

/// A card under the song: its title and link on top, `body` under them.
/// Whether the link was clicked.
pub(super) fn card(
    ui: &mut Ui,
    theme: &Theme,
    width: f32,
    title: &str,
    link: &str,
    body: f32,
    paint: impl FnOnce(&mut Ui, Rect),
) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(width + 8.0, 48.0 + body + 12.0), Sense::hover());
    let rect = Rect::from_min_size(
        pos2(rect.left() + 8.0, rect.top()),
        vec2(width - 8.0, 48.0 + body),
    );
    ui.painter().rect_filled(rect, 8.0, theme.surface0);
    let g = text(
        ui,
        title,
        font(15.0, Weight::Bold),
        theme.text,
        rect.width() - 120.0,
    );
    paint_text(
        ui,
        g,
        rect.left() + 16.0,
        rect.top() + 26.0,
        egui::Align::Min,
        theme.text,
    );
    let g = text(ui, link, font(13.0, Weight::Medium), theme.subtext0, 120.0);
    let link_rect = Rect::from_min_size(
        pos2(
            rect.right() - 16.0 - g.size().x,
            rect.top() + 26.0 - g.size().y / 2.0,
        ),
        g.size(),
    );
    let hit = ui
        .interact(link_rect, Id::unique(("side-card", title)), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let color = if hit.hovered() {
        theme.text
    } else {
        theme.subtext0
    };
    if hit.hovered() {
        ui.painter().hline(
            link_rect.x_range(),
            link_rect.bottom(),
            egui::Stroke::new(1.0, color),
        );
    }
    let g = text(ui, link, font(13.0, Weight::Medium), color, 120.0);
    paint_text(
        ui,
        g,
        link_rect.left(),
        link_rect.center().y,
        egui::Align::Min,
        color,
    );
    let inner = Rect::from_min_max(
        pos2(rect.left() + 16.0, rect.top() + 48.0),
        pos2(rect.right() - 16.0, rect.bottom() - 12.0),
    );
    paint(ui, inner);
    hit.clicked()
}

pub(super) fn next_row(ui: &Ui, theme: &Theme, inner: Rect, track: &Track) {
    let cover = Rect::from_min_size(inner.min, vec2(40.0, 40.0));
    artwork(ui, thumb_url(track), cover, 4.0, theme);
    let x = cover.right() + 12.0;
    let room = inner.right() - x;
    let g = text(
        ui,
        &track.title,
        font(14.0, Weight::Medium),
        theme.text,
        room,
    );
    paint_text(
        ui,
        g,
        x,
        cover.center().y - 9.0,
        egui::Align::Min,
        theme.text,
    );
    let g = text(
        ui,
        &track.artists.join(", "),
        font(12.5, Weight::Regular),
        theme.subtext0,
        room,
    );
    paint_text(
        ui,
        g,
        x,
        cover.center().y + 10.0,
        egui::Align::Min,
        theme.subtext0,
    );
}
