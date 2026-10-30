//! Now Playing, as Cider's full screen player: the artwork the window's
//! height on the left, fading into the cover's colours; the lyrics in large
//! type on the right; the song and its controls in a small card at the
//! bottom left. While the lyrics run and the pointer rests, the card and the
//! buttons fade away.

use egui::{Color32, Id, Mesh, Rect, Sense, Stroke, Ui, pos2, vec2};
use music_core::model::{FavouriteKind, Track};

use crate::app::{App, Loaded};
use crate::color::alpha;
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::menu::{TrackFacts, track_menu_button};
use super::{Blob, NARROW, explicit_badge, icon_button, lyrics, paint_text, text, wash_with};

/// Where the lyrics begin, as a fraction of the window's width.
const LYRICS_LEFT: f32 = 0.52;
/// How long the pointer rests before the controls fade, in seconds.
const IDLE: f32 = 3.0;
const CARD: egui::Vec2 = vec2(420.0, 64.0);

pub fn show(app: &mut App, ui: &mut Ui) {
    let theme = app.theme.clone();
    let full = ui.max_rect();
    ui.painter().rect_filled(full, 0.0, theme.base);
    backdrop(app, ui, &theme, full);

    let Some(track) = app.shown().track else {
        let g = text(
            ui,
            "Nothing playing",
            font(18.0, Weight::SemiBold),
            theme.subtext0,
            full.width(),
        );
        let center = full.center();
        paint_text(
            ui,
            g,
            center.x,
            center.y,
            egui::Align::Center,
            theme.subtext0,
        );
        buttons(app, ui, &theme, full, 1.0);
        return;
    };
    let narrow = full.width() < NARROW;
    let lyrics_left = full.left() + full.width() * LYRICS_LEFT;
    cover(app, ui, &track, full, narrow, lyrics_left);

    let lyrics = lyrics::ready(app);
    if !narrow {
        let body = Rect::from_min_max(
            pos2(lyrics_left, full.top() + full.height() * 0.08),
            pos2(full.right() - 24.0, full.bottom()),
        );
        if lyrics.is_some() {
            let size = (body.width() * 0.046).clamp(22.0, 42.0);
            lyrics::show(app, ui, &theme, body, size, full.height(), "np-lyrics");
        } else if !matches!(app.lyrics(&track), Loaded::Loading) {
            title(ui, &theme, &track, body, full);
        }
    }

    // The card sits under the artwork, the middle of what shows of it.
    let card_x = if narrow {
        full.center().x
    } else {
        (full.left() + lyrics_left) / 2.0
    };
    let size = vec2(CARD.x.min(full.width() - 32.0), CARD.y);
    let card = Rect::from_center_size(pos2(card_x, full.bottom() - 30.0 - size.y / 2.0), size);

    // Hidden while the lyrics run and the pointer rests away from the card.
    let running = lyrics.is_some_and(|l| l.synced) && app.shown().playing;
    let idle = ui.input(|i| i.pointer.time_since_last_movement()) > IDLE;
    let on_card = ui.rect_contains_pointer(card.expand(8.0));
    let reveal = ui.ctx().animate_bool_with_time(
        Id::unique("np-controls"),
        !(running && idle) || on_card,
        0.5,
    );
    if reveal == 0.0 {
        ui.ctx().set_cursor_icon(egui::CursorIcon::None);
        return;
    }
    controls(app, ui, &theme, &track, card, reveal);
    buttons(app, ui, &theme, full, reveal);
}

/// The cover's three colours as four large blobs across the window, drifting
/// slowly, under a vignette of the background.
fn backdrop(app: &mut App, ui: &Ui, theme: &Theme, full: Rect) {
    let colors = app
        .shown()
        .track
        .as_ref()
        .and_then(|t| t.artwork.as_ref())
        .map(|a| a.url(96))
        .and_then(|url| app.palette(&url))
        .map(|p| p.map(|c| crate::palette::fit(c, theme.light)))
        .unwrap_or([theme.accent, theme.mauve, theme.accent]);
    let [c1, c2, c3] = colors;
    // 40 s one way, then back.
    let t = ui.input(|i| i.time) as f32;
    let drift = (t / 40.0 * std::f32::consts::PI).sin() * 0.04;
    let blob = |x: f32, y: f32, rx: f32, ry: f32, color: Color32, alpha: f32| Blob {
        center: vec2(x + drift, y - drift * 0.6),
        radii: vec2(rx, ry),
        color,
        alpha,
        fade: 0.85,
    };
    let blobs = [
        blob(0.22, 0.34, 0.42, 0.52, c1, 0.95),
        blob(0.78, 0.22, 0.40, 0.50, c2, 0.85),
        blob(0.70, 0.86, 0.46, 0.56, c3, 0.80),
        blob(0.12, 0.92, 0.36, 0.46, c2, 0.60),
    ];
    let vignette = |f: egui::Vec2| {
        let d = ((f - vec2(0.55, 0.45)) / vec2(1.2, 1.0)).length() * 2.0;
        0.45 * d.min(1.0)
    };
    wash_with(ui, full, theme.base, &blobs, vignette);
    // It drifts while the music plays, a few points a second: the player's
    // own tick of four a second moves it on.
}

/// The artwork, or its animated cover, the window's height a side from the
/// left edge, fading out towards the lyrics and a little at the bottom. In a
/// narrow window it fills the width and fades at the bottom only.
fn cover(app: &mut App, ui: &Ui, track: &Track, full: Rect, narrow: bool, lyrics_left: f32) {
    let side = if narrow {
        full.width().max(full.height())
    } else {
        full.height()
    };
    let art = Rect::from_min_size(
        pos2(full.center().x - side / 2.0, full.top()),
        vec2(side, side),
    );
    let art = if narrow {
        art
    } else {
        art.translate(vec2(full.left() - art.left(), 0.0))
    };
    // Clear to 40 % of the side before where it ends, then gone.
    let end = if narrow {
        f32::INFINITY
    } else {
        (lyrics_left + full.width() * 0.04).min(art.right())
    };
    let fade_from = end - 0.40 * side;

    let pixels = side * ui.ctx().pixels_per_point();
    let motion = app.album_motion(track);
    let texture = motion
        .as_deref()
        .and_then(|master| app.motion_frame("now-playing", master, pixels, ui.ctx()))
        .or_else(|| {
            let size = if pixels > 1500.0 { 3000 } else { 1500 };
            let url = track.artwork.as_ref()?.url(size);
            crate::covers::seen(ui.ctx(), &url);
            match egui::Image::new(url).load_for_size(ui.ctx(), art.size()) {
                Ok(egui::load::TexturePoll::Ready { texture }) => Some(texture),
                _ => None,
            }
        });
    let Some(texture) = texture else { return };

    let (cols, rows) = (32, 16);
    let mut mesh = Mesh::with_texture(texture.id);
    for row in 0..=rows {
        for col in 0..=cols {
            let f = vec2(col as f32 / cols as f32, row as f32 / rows as f32);
            let p = art.min + f * art.size();
            let across = 1.0 - smooth(((p.x - fade_from) / (end - fade_from)).clamp(0.0, 1.0));
            let down = 1.0 - 0.45 * smooth(((f.y - 0.70) / 0.30).clamp(0.0, 1.0));
            mesh.vertices.push(egui::epaint::Vertex {
                pos: p,
                uv: f.to_pos2(),
                color: Color32::WHITE.gamma_multiply(across * down),
            });
        }
    }
    let w = (cols + 1) as u32;
    for row in 0..rows as u32 {
        for col in 0..cols as u32 {
            let i = row * w + col;
            mesh.add_triangle(i, i + 1, i + w);
            mesh.add_triangle(i + 1, i + w + 1, i + w);
        }
    }
    ui.painter_at(full).add(mesh);
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Without lyrics, the song's title and artist in their place.
fn title(ui: &Ui, theme: &Theme, track: &Track, body: Rect, full: Rect) {
    let y = full.top() + full.height() * 0.42;
    let g = text(
        ui,
        &track.title,
        font(44.0, Weight::Bold),
        theme.text,
        body.width() - 24.0,
    );
    paint_text(ui, g, body.left() + 12.0, y, egui::Align::Min, theme.text);
    let g = text(
        ui,
        &track.artists.join(", "),
        font(24.0, Weight::SemiBold),
        theme.subtext0,
        body.width() - 24.0,
    );
    paint_text(
        ui,
        g,
        body.left() + 12.0,
        y + 46.0,
        egui::Align::Min,
        theme.subtext0,
    );
}

/// The card: the title and artist, then the favourite star, the transport
/// and the song's menu; the position along its foot.
fn controls(app: &mut App, ui: &mut Ui, theme: &Theme, track: &Track, card: Rect, reveal: f32) {
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(card));
    ui.set_opacity(reveal);
    let ui = &mut ui;
    ui.painter().add(
        egui::Shadow {
            offset: [0, 10],
            blur: 30,
            spread: 0,
            color: Color32::from_black_alpha(90),
        }
        .as_shape(card, 16.0),
    );
    ui.painter().rect(
        card,
        16.0,
        alpha(theme.crust, 0.72),
        Stroke::new(1.0, theme.text_a(0.10)),
        egui::StrokeKind::Inside,
    );

    // From the right: the menu, next, play, previous, the star.
    let mid = card.center().y - 2.0;
    let mut x = card.right() - 12.0;
    let mut slot = |width: f32| {
        let rect = Rect::from_min_size(pos2(x - width, mid - width / 2.0), vec2(width, width));
        x -= width + 2.0;
        rect
    };
    let more = slot(32.0);
    let next = slot(32.0);
    let play = slot(36.0);
    let previous = slot(32.0);
    let star = slot(32.0);
    let words_right = x - 8.0;

    let station = track.id.id.starts_with("ra.");
    let favourite = app.is_favourite(FavouriteKind::Song, &track.id);
    let has_track = app.shown().has_track;
    let button = |ui: &mut Ui, rect: Rect| ui.new_child(egui::UiBuilder::new().max_rect(rect));
    if !station {
        let (icon, color) = if favourite {
            (icons::STAR_FILL, theme.red)
        } else {
            (icons::STAR, theme.subtext0)
        };
        if icon_button(&mut button(ui, star), theme, icon, 32.0, 16.0, color, true)
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
    let transport = [
        (previous, icons::BACK_FILL, 20.0),
        (play, icons::PLAY_FILL, 24.0),
        (next, icons::FORWARD_FILL, 20.0),
    ];
    for (k, (rect, icon, glyph)) in transport.into_iter().enumerate() {
        let icon = if k == 1 && app.shown().playing {
            icons::PAUSE_FILL
        } else {
            icon
        };
        let enabled = k == 1 || has_track;
        if icon_button(
            &mut button(ui, rect),
            theme,
            icon,
            rect.width(),
            glyph,
            theme.text,
            enabled,
        )
        .clicked()
        {
            match k {
                0 => app.previous(),
                1 => app.toggle_play(),
                _ => app.next(),
            }
        }
    }
    if !station {
        let open = icon_button(
            &mut button(ui, more),
            theme,
            icons::MORE,
            32.0,
            16.0,
            theme.subtext0,
            true,
        );
        let facts = TrackFacts {
            has_album: track.album_id.is_some(),
            favourite,
            in_library: app.track_in_library(track),
            removable: track.library_id.is_some(),
            in_playlist: false,
        };
        if let Some(choice) = track_menu_button(&open, theme, facts) {
            app.track_choice(track.clone(), choice, ui.input(|i| i.time));
        }
    }

    // The title, with the E badge, over the artist.
    let left = card.left() + 18.0;
    let width = (words_right - left).max(0.0);
    let g = text(
        ui,
        &track.title,
        font(14.0, Weight::SemiBold),
        theme.text,
        width - if track.explicit { 20.0 } else { 0.0 },
    );
    let end = left + g.size().x;
    paint_text(ui, g, left, mid - 9.0, egui::Align::Min, theme.text);
    if track.explicit {
        explicit_badge(ui, theme, pos2(end + 6.0 + 6.0, mid - 9.0));
    }
    let g = text(
        ui,
        &track.artists.join(", "),
        font(12.5, Weight::Regular),
        theme.subtext0,
        width,
    );
    paint_text(ui, g, left, mid + 10.0, egui::Align::Min, theme.subtext0);

    position(app, ui, theme, card);
}

/// The position as a hairline along the card's foot, which seeks when
/// clicked or dragged.
fn position(app: &mut App, ui: &mut Ui, theme: &Theme, card: Rect) {
    let Some(total) = app.shown().duration.filter(|d| !d.is_zero()) else {
        return;
    };
    let rail = Rect::from_min_max(
        pos2(card.left() + 18.0, card.bottom() - 7.0),
        pos2(card.right() - 18.0, card.bottom() - 5.0),
    );
    let hit = rail.expand2(vec2(0.0, 5.0));
    let response = ui.interact(hit, Id::unique("np-seek"), Sense::click_and_drag());
    let mut at = app.shown().position;
    if let Some(pointer) = response.interact_pointer_pos() {
        at = total.mul_f32(((pointer.x - rail.left()) / rail.width()).clamp(0.0, 1.0));
        if response.drag_stopped() || response.clicked() {
            app.seek(at);
        }
    }
    let fraction = (at.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0);
    let active = response.hovered() || response.dragged();
    ui.painter().rect_filled(rail, 1.0, theme.text_a(0.14));
    let fill = Rect::from_min_size(rail.min, vec2(rail.width() * fraction, rail.height()));
    ui.painter()
        .rect_filled(fill, 1.0, theme.text_a(if active { 0.9 } else { 0.55 }));
    if active {
        ui.painter()
            .circle_filled(pos2(fill.right(), rail.center().y), 4.0, theme.text);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
}

/// Top left, in a pill: close Now Playing, and full screen in or out.
fn buttons(app: &mut App, ui: &mut Ui, theme: &Theme, full: Rect, reveal: f32) {
    let pill = Rect::from_min_size(full.min + vec2(18.0, 14.0), vec2(76.0, 40.0));
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(pill));
    ui.set_opacity(reveal);
    ui.painter()
        .rect_filled(pill, 20.0, alpha(theme.crust, 0.78));
    let fullscreen = ui.input(|i| i.viewport().fullscreen.unwrap_or(false));
    let close = Rect::from_center_size(pill.left_center() + vec2(22.0, 0.0), vec2(32.0, 32.0));
    let full_rect = Rect::from_center_size(pill.right_center() - vec2(22.0, 0.0), vec2(32.0, 32.0));
    let mut close_ui = ui.new_child(egui::UiBuilder::new().max_rect(close));
    if icon_button(
        &mut close_ui,
        theme,
        icons::DOWN,
        32.0,
        18.0,
        theme.text,
        true,
    )
    .on_hover_text("Close Now Playing")
    .clicked()
    {
        app.now_playing = false;
    }
    let mut full_ui = ui.new_child(egui::UiBuilder::new().max_rect(full_rect));
    if icon_button(
        &mut full_ui,
        theme,
        icons::EXPAND,
        32.0,
        15.0,
        theme.text,
        true,
    )
    .on_hover_text(if fullscreen {
        "Exit Full Screen"
    } else {
        "Full Screen"
    })
    .clicked()
    {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
    }
}
