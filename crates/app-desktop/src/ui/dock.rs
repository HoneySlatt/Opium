//! The player along the window's bottom, as Spotify's: the song on the left
//! with its quality and favourite, the controls over the position in the
//! middle, the panels, the devices and the volume on the right.

use std::time::Duration;

use egui::{Color32, Id, Rect, Sense, Ui, pos2, vec2};
use music_core::model::FavouriteKind;

use crate::app::{App, Page, Side, SignalAnchor};
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::{Lyrics, Panel, Theme};

use super::menu::{TrackFacts, track_menu};
use super::{
    artwork, explicit_badge, format_time, icon_button, paint_icon, paint_text, text, thumb_url,
};

pub const HEIGHT: f32 = 80.0;

pub fn show(app: &mut App, ui: &mut Ui) {
    let theme = app.theme.clone();
    let bar = ui.max_rect();
    // Clicks on the bar's background must not reach what is under it.
    ui.interact(bar, Id::unique("dock-bg"), Sense::click());
    let third = (bar.width() * 0.3).min(420.0);
    let song = Rect::from_min_max(
        bar.min + vec2(8.0, 0.0),
        pos2(bar.left() + third, bar.bottom()),
    );
    let tools = Rect::from_min_max(
        pos2(bar.right() - third, bar.top()),
        bar.max - vec2(8.0, 0.0),
    );
    let middle_width = (bar.width() - 2.0 * third - 32.0).clamp(240.0, 722.0);
    let middle = Rect::from_center_size(bar.center(), vec2(middle_width, bar.height()));
    now_playing(app, ui, &theme, song);
    controls(app, ui, &theme, middle);
    right_side(app, ui, &theme, tools, bar.width() < super::NARROW);
}

/// The cover, the title, the artists and the quality, then the favourite.
fn now_playing(app: &mut App, ui: &mut Ui, theme: &Theme, area: Rect) {
    let shown = app.shown();
    let Some(track) = shown.loading.clone().or(shown.track.clone()) else {
        return;
    };
    let art = Rect::from_min_size(pos2(area.left(), area.center().y - 28.0), vec2(56.0, 56.0));
    artwork(ui, thumb_url(&track), art, 4.0, theme);
    // The cover opens Now Playing, as the floating bar's does.
    let open = ui.interact(art, Id::unique("dock-art"), Sense::click());
    let station = track.id.id.starts_with("ra.");
    let (has_album, favourite, in_library) = (
        track.album_id.is_some(),
        app.is_favourite(FavouriteKind::Song, &track.id),
        app.track_in_library(&track),
    );
    let removable = track.library_id.is_some();
    let facts = || TrackFacts {
        has_album,
        favourite,
        in_library,
        removable,
        in_playlist: false,
    };
    let mut choice = None;
    if !station {
        choice = choice.or(track_menu(&open, theme, facts()));
    }
    if open.hovered() {
        ui.painter()
            .rect_filled(art, 4.0, Color32::from_black_alpha(115));
        paint_icon(ui, icons::EXPAND, art.center(), 18.0, Color32::WHITE);
    }
    if open
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Now Playing")
        .clicked()
    {
        app.now_playing = true;
    }

    let left = art.right() + 14.0;
    // Room for the favourite after the words.
    let width = (area.right() - left - 44.0).max(0.0);
    let badge = super::quality_badge(&track);
    let lines = if badge.is_some() { 3.0 } else { 2.0 };
    let mut y = area.center().y - (lines - 1.0) * 10.0;

    // The title, which opens its album.
    let title_width = width - if track.explicit { 17.0 } else { 0.0 };
    let g = text(
        ui,
        &track.title,
        font(14.0, Weight::Medium),
        theme.text,
        title_width,
    );
    let link = Rect::from_min_size(pos2(left, y - g.size().y / 2.0), g.size());
    let title_end = link.right();
    let hit = ui.interact(link, Id::unique("dock-title"), Sense::click());
    if !station {
        choice = choice.or(track_menu(&hit, theme, facts()));
    }
    if hit.hovered() && track.album_id.is_some() {
        ui.painter().hline(
            link.x_range(),
            link.bottom(),
            egui::Stroke::new(1.0, theme.text),
        );
    }
    if let Some(album) = &track.album_id
        && hit
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
    {
        app.go(Page::Album(album.clone()));
    }
    paint_text(ui, g, left, y, egui::Align::Min, theme.text);
    if track.explicit {
        explicit_badge(ui, theme, pos2(title_end + 5.0 + 6.0, y));
    }
    y += 20.0;

    // The artists, which open the first one's page.
    let mut artists = track.artists.join(", ");
    if let Some(device) = &shown.on {
        artists = format!("On {} · {artists}", device.name);
    }
    let g = text(
        ui,
        &artists,
        font(12.0, Weight::Regular),
        theme.subtext0,
        width,
    );
    let link = Rect::from_min_size(pos2(left, y - g.size().y / 2.0), g.size());
    let mut color = theme.subtext0;
    if !station {
        // The artist's page, once the service says which it is.
        let hit = ui.interact(link, Id::unique("dock-artist"), Sense::click());
        if hit.hovered() {
            color = theme.text;
            ui.painter()
                .hline(link.x_range(), link.bottom(), egui::Stroke::new(1.0, color));
        }
        if hit
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            choice = Some(crate::app::TrackChoice::GoToArtist);
        }
    }
    let g = text(ui, &artists, font(12.0, Weight::Regular), color, width);
    paint_text(ui, g, left, y, egui::Align::Min, color);
    y += 20.0;

    // Lossless or Dolby Atmos, in the accent; a click tells the signal path.
    if let Some(badge) = badge {
        let g = text(ui, badge, font(11.0, Weight::Medium), theme.accent, width);
        let mut rect = Rect::from_min_size(pos2(left, y - g.size().y / 2.0), g.size());
        paint_text(ui, g, left, y, egui::Align::Min, theme.accent);
        if super::plays_in_atmos(&track) {
            super::dolby_logo(ui, pos2(rect.right() + 12.0, y), 9.0, theme.accent);
            rect.max.x += 20.0;
        }
        let hit = ui
            .interact(rect, Id::unique("dock-quality"), Sense::click())
            .on_hover_text("Signal path")
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        super::signal::remember_anchor(ui, SignalAnchor::PlayerBar, rect);
        if hit.clicked() {
            app.signal_open = match app.signal_open {
                Some(SignalAnchor::PlayerBar) => None,
                _ => Some(SignalAnchor::PlayerBar),
            };
        }
    }

    // The favourite, as Spotify's saved mark.
    if !station {
        let words_end = left
            + width.min(
                text(
                    ui,
                    &track.title,
                    font(14.0, Weight::Medium),
                    theme.text,
                    title_width,
                )
                .size()
                .x
                .max(
                    text(ui, &artists, font(12.0, Weight::Regular), theme.text, width)
                        .size()
                        .x,
                ),
            );
        let mark =
            Rect::from_center_size(pos2(words_end + 26.0, area.center().y), vec2(32.0, 32.0));
        let hit = ui
            .interact(mark, Id::unique("dock-favourite"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(if favourite {
                "Remove from Favourites"
            } else {
                "Favourite"
            });
        if favourite {
            ui.painter().circle_filled(mark.center(), 8.5, theme.accent);
            paint_icon(ui, icons::CHECK, mark.center(), 11.0, theme.on_accent());
        } else {
            let color = if hit.hovered() {
                theme.text
            } else {
                theme.subtext0
            };
            paint_icon(ui, icons::CIRCLE_PLUS, mark.center(), 18.0, color);
        }
        if hit.clicked() {
            app.toggle_favourite(FavouriteKind::Song, track.id.clone());
        }
    }
    if let Some(choice) = choice {
        app.track_choice(track, choice, ui.input(|i| i.time));
    }
}

/// Shuffle, previous, play, next and repeat over the position.
fn controls(app: &mut App, ui: &mut Ui, theme: &Theme, area: Rect) {
    let shown = app.shown();
    let has_track = shown.has_track;
    let dim = theme.subtext0;
    let y = area.top() + 30.0;
    let at = |dx: f32| Rect::from_center_size(pos2(area.center().x + dx, y), vec2(32.0, 32.0));
    let button = |ui: &mut Ui, dx: f32, icon: Icon, glyph: f32, color, enabled, tip: &str| {
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(at(dx)));
        let response =
            icon_button(&mut child, theme, icon, 32.0, glyph, color, enabled).on_hover_text(tip);
        if color == theme.accent {
            // Spotify's dot under a button that is on.
            ui.painter()
                .circle_filled(pos2(area.center().x + dx, y + 13.0), 2.0, theme.accent);
        }
        response.clicked()
    };
    let shuffle = if shown.shuffle { theme.accent } else { dim };
    if button(ui, -88.0, icons::SHUFFLE, 16.0, shuffle, true, "Shuffle") {
        app.toggle_shuffle();
    }
    if button(
        ui,
        -44.0,
        icons::BACK_FILL,
        16.0,
        dim,
        has_track,
        "Previous",
    ) {
        app.previous();
    }
    if button(ui, 44.0, icons::FORWARD_FILL, 16.0, dim, has_track, "Next") {
        app.next();
    }
    let (repeat_icon, repeat) = match shown.repeat {
        music_core::Repeat::Off => (icons::REPEAT, dim),
        music_core::Repeat::All => (icons::REPEAT, theme.accent),
        music_core::Repeat::One => (icons::REPEAT_ONE, theme.accent),
    };
    if button(ui, 88.0, repeat_icon, 16.0, repeat, true, "Repeat") {
        app.cycle_repeat();
    }

    // Play: a round button in the text's colour.
    let play = Rect::from_center_size(pos2(area.center().x, y), vec2(32.0, 32.0));
    if shown.loading.is_some() {
        ui.put(
            Rect::from_center_size(play.center(), vec2(18.0, 18.0)),
            egui::Spinner::new().size(18.0).color(theme.text),
        )
        .on_hover_text("Loading");
    } else {
        let sense = if has_track {
            Sense::click()
        } else {
            Sense::hover()
        };
        let response = ui.interact(play, Id::unique("dock-play"), sense);
        let grow = if has_track && response.hovered() {
            1.0
        } else {
            0.0
        };
        let fill = if has_track {
            theme.text
        } else {
            theme.text_a(0.4)
        };
        ui.painter().circle_filled(play.center(), 16.0 + grow, fill);
        let (icon, tip) = if shown.playing {
            (icons::PAUSE_FILL, "Pause")
        } else {
            (icons::PLAY_FILL, "Play")
        };
        paint_icon(ui, icon, play.center(), 16.0, theme.base);
        if response
            .on_hover_text(tip)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            app.toggle_play();
        }
    }

    // The position: the time played, the bar, the length.
    let y = area.top() + 62.0;
    let position = super::player_bar::shown_position(app, ui);
    let small = font(11.5, Weight::Regular);
    let (played, length) = match shown.duration {
        Some(total) if shown.track.is_some() => (format_time(position), format_time(total)),
        None if shown.playing => (format_time(position), "LIVE".into()),
        _ => ("-:--".into(), "-:--".into()),
    };
    let g = text(ui, &played, small.clone(), theme.subtext0, 60.0);
    paint_text(
        ui,
        g,
        area.left() + 40.0,
        y,
        egui::Align::Max,
        theme.subtext0,
    );
    let g = text(ui, &length, small, theme.subtext0, 60.0);
    paint_text(
        ui,
        g,
        area.right() - 40.0,
        y,
        egui::Align::Min,
        theme.subtext0,
    );
    let rail = Rect::from_min_max(
        pos2(area.left() + 48.0, y - 6.0),
        pos2(area.right() - 48.0, y + 6.0),
    );
    seek_bar(app, ui, theme, rail, shown.duration);
}

/// The position as a 4 px bar, in the accent with a knob on hover. Click or
/// drag to seek.
fn seek_bar(app: &mut App, ui: &mut Ui, theme: &Theme, hit: Rect, duration: Option<Duration>) {
    let rail = Rect::from_center_size(hit.center(), vec2(hit.width(), 4.0));
    let painter = ui.painter();
    painter.rect_filled(rail, 2.0, theme.text_a(0.3));
    let Some(total) = duration.filter(|d| !d.is_zero()) else {
        return;
    };
    let response = ui.interact(hit, Id::unique("dock-seek"), Sense::click_and_drag());
    // Where the floating bar keeps it too: the time follows the drag.
    let preview_id = Id::unique("seek-preview");
    let at = |x: f32| total.mul_f32(((x - hit.left()) / hit.width()).clamp(0.0, 1.0));
    if let Some(pointer) = response.interact_pointer_pos() {
        if response.dragged() {
            ui.ctx()
                .data_mut(|d| d.insert_temp(preview_id, at(pointer.x)));
        }
        if response.drag_stopped() || response.clicked() {
            app.seek(at(pointer.x));
            ui.ctx().data_mut(|d| d.remove::<Duration>(preview_id));
        }
    }
    let active = response.hovered() || response.dragged();
    let position = super::player_bar::shown_position(app, ui);
    let fraction = (position.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0);
    let fill = Rect::from_min_size(rail.min, vec2(rail.width() * fraction, 4.0));
    let painter = ui.painter();
    painter.rect_filled(fill, 2.0, if active { theme.accent } else { theme.text });
    if active {
        painter.circle_filled(pos2(fill.right(), rail.center().y), 6.0, theme.text);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
}

/// The panels' buttons, the devices, the volume, and Now Playing.
fn right_side(app: &mut App, ui: &mut Ui, theme: &Theme, area: Rect, narrow: bool) {
    let layout = theme.layout;
    let side = layout.panel == Panel::Side && !narrow;
    let dim = theme.subtext0;
    let width = if side { 32.0 } else { 0.0 } + 32.0 * 4.0 + 6.0 + 84.0 + 8.0 + 32.0;
    let row_rect = Rect::from_min_max(
        pos2(area.right() - width, area.center().y - 16.0),
        pos2(area.right(), area.center().y + 16.0),
    );
    let mut row = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    row.spacing_mut().item_spacing.x = 0.0;
    let on = |yes: bool| if yes { theme.accent } else { dim };
    let dot = |ui: &Ui, rect: Rect, yes: bool| {
        if yes {
            ui.painter().circle_filled(
                pos2(rect.center().x, rect.bottom() + 1.0),
                2.0,
                theme.accent,
            );
        }
    };
    if side {
        let shows = app.side == Some(Side::NowPlaying);
        let response = icon_button(&mut row, theme, icons::PANEL, 32.0, 16.0, on(shows), true)
            .on_hover_text("Now playing view");
        dot(&row, response.rect, shows);
        if response.clicked() {
            app.toggle_side(Side::NowPlaying);
        }
    }
    let lyrics_on = if layout.lyrics == Lyrics::Page {
        app.lyrics_page
    } else {
        app.drawer_shows(true)
    };
    let response = icon_button(
        &mut row,
        theme,
        icons::LYRICS,
        32.0,
        16.0,
        on(lyrics_on),
        true,
    )
    .on_hover_text("Lyrics");
    dot(&row, response.rect, lyrics_on);
    if response.clicked() {
        if layout.lyrics == Lyrics::Page {
            app.lyrics_page = !app.lyrics_page;
            // Spotify's full screen shows them in the cover's place.
            if layout.now_playing != crate::theme::NowPlaying::Cover {
                app.now_playing = false;
            }
        } else {
            app.toggle_drawer(true);
        }
    }
    let queue_on = if side {
        matches!(app.side, Some(Side::Queue | Side::History))
    } else {
        app.drawer_shows(false)
    };
    let response = icon_button(
        &mut row,
        theme,
        icons::QUEUE,
        32.0,
        16.0,
        on(queue_on),
        true,
    )
    .on_hover_text("Queue");
    dot(&row, response.rect, queue_on);
    if response.clicked() {
        if side {
            app.side = if queue_on { None } else { Some(Side::Queue) };
        } else {
            app.toggle_drawer(false);
        }
    }
    let shown = app.shown();
    super::devices::button(app, &mut row, theme, &shown);
    let mute_icon = if shown.muted || shown.volume == Some(0.0) {
        icons::VOL_MUTE
    } else {
        icons::VOL
    };
    if icon_button(
        &mut row,
        theme,
        mute_icon,
        32.0,
        16.0,
        dim,
        shown.volume.is_some(),
    )
    .on_hover_text(if shown.muted { "Unmute" } else { "Mute" })
    .clicked()
    {
        app.toggle_mute();
    }
    row.add_space(6.0);
    super::player_bar::volume_slider(app, &mut row, theme, &shown);
    row.add_space(8.0);
    let (icon, tip) = if app.now_playing {
        (icons::MINIMIZE, "Exit full screen")
    } else {
        (icons::EXPAND, "Full screen")
    };
    if icon_button(&mut row, theme, icon, 32.0, 15.0, dim, true)
        .on_hover_text(tip)
        .clicked()
    {
        app.now_playing = !app.now_playing;
    }
}
