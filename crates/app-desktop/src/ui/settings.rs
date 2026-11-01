//! Settings: the services and their sign-in, the colours, the interface scale,
//! and what the app is. Only settings that do something are here.

use egui::{Id, Rect, Sense, Stroke, Ui, pos2, vec2};

use music_core::model::ProviderKind;

use crate::app::{App, Connection, SoundCloudState};
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::settings::Appearance;
use crate::theme::Theme;

use super::{glow, paint_icon, paint_text, text};

const TITLE_Y: f32 = 66.0;
const BODY_TOP: f32 = 104.0;
/// The page is no wider than this, as in the design.
const MAX_WIDTH: f32 = 760.0;
const ROW: f32 = 68.0;
const GROUP_GAP: f32 = 32.0;
/// A group's title and the line under it.
const GROUP_HEAD: f32 = 52.0;
/// The theme cards: padding, preview, name, line, swatches, padding.
const THEMES: f32 = 14.0 + 10.0 + 74.0 + 10.0 + 18.0 + 18.0 + 4.0 + 14.0 + 12.0 + 14.0;

pub fn show(app: &mut App, ui: &mut Ui, pad: f32) -> f32 {
    let theme = app.theme.clone();
    let width = (ui.available_width() - 2.0 * pad).min(MAX_WIDTH);
    let paired = app.devices.peers.iter().filter(|p| p.paired).count().max(1);
    let total = BODY_TOP
        + 11.0 * (GROUP_HEAD + GROUP_GAP)
        + 3.0 * ROW
        + THEMES * (1 + theme_rows()) as f32
        + (13 + paired) as f32 * ROW
        + 104.0;
    let output = egui::ScrollArea::vertical()
        .id_salt("settings")
        .auto_shrink(false)
        .show(ui, |ui| {
            let (content, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
            glow(
                ui,
                &theme,
                Rect::from_min_size(content.min, vec2(content.width(), 460.0)),
            );
            let left = content.left() + pad;
            let g = text(ui, "Settings", font(22.0, Weight::Bold), theme.text, width);
            paint_text(
                ui,
                g,
                left,
                content.top() + TITLE_Y,
                egui::Align::Min,
                theme.text,
            );
            let mut y = content.top() + BODY_TOP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Services",
                "Each service keeps its own sign-in, on this computer only.",
            );
            let card = Rect::from_min_size(pos2(left, y), vec2(width, 3.0 * ROW));
            panel(ui, &theme, card, 3);
            services(app, ui, &theme, card);
            y = card.bottom() + GROUP_GAP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Library",
                "What joins your Apple Music library, and how Recently Added is sorted.",
            );
            let card = Rect::from_min_size(pos2(left, y), vec2(width, 3.0 * ROW));
            panel(ui, &theme, card, 3);
            let on = app.favourites_to_library();
            if switch(
                ui,
                &theme,
                pos2(card.right() - 16.0, card.top() + ROW / 2.0),
                on,
                "favourites-to-library",
            ) {
                app.set_favourites_to_library(!on);
            }
            labels(
                ui,
                &theme,
                pos2(card.left() + 16.0, card.top() + ROW / 2.0),
                "Add favourites to the library",
                "A song you favourite joins your library if it isn't there yet.",
                width - 32.0 - 60.0,
            );
            let on = app.playlist_songs_to_library();
            if switch(
                ui,
                &theme,
                pos2(card.right() - 16.0, card.top() + 1.5 * ROW),
                on,
                "playlist-songs-to-library",
            ) {
                app.set_playlist_songs_to_library(!on);
            }
            labels(
                ui,
                &theme,
                pos2(card.left() + 16.0, card.top() + 1.5 * ROW),
                "Add playlist songs to the library",
                "A song you add to a playlist joins your library if it isn't there yet.",
                width - 32.0 - 60.0,
            );
            let on = app.recent_by_date();
            if switch(
                ui,
                &theme,
                pos2(card.right() - 16.0, card.top() + 2.5 * ROW),
                on,
                "recent-by-date",
            ) {
                app.set_recent_by_date(!on);
            }
            labels(
                ui,
                &theme,
                pos2(card.left() + 16.0, card.top() + 2.5 * ROW),
                "Recently Added by date",
                "Every service's together, newest first. Off: Apple Music's, then SoundCloud's, then Navidrome's.",
                width - 32.0 - 60.0,
            );
            y = card.bottom() + GROUP_GAP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Spatial Audio",
                "How Apple Music's songs mixed in Dolby Atmos play.",
            );
            let card = Rect::from_min_size(pos2(left, y), vec2(width, ROW));
            panel(ui, &theme, card, 1);
            let on = app.spatial_audio();
            if switch(
                ui,
                &theme,
                pos2(card.right() - 16.0, card.center().y),
                on,
                "spatial-audio",
            ) {
                app.set_spatial_audio(!on);
            }
            labels(
                ui,
                &theme,
                pos2(card.left() + 16.0, card.center().y),
                "Dolby Atmos",
                "Apple's binaural rendering of the Atmos mix, for headphones (AAC 256, 48 kHz). Off, these songs play in stereo.",
                width - 32.0 - 60.0,
            );
            y = card.bottom() + GROUP_GAP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "SoundCloud",
                "Where its tracks play from.",
            );
            let card = Rect::from_min_size(pos2(left, y), vec2(width, ROW));
            panel(ui, &theme, card, 1);
            let on = app.soundcloud_originals();
            if switch(
                ui,
                &theme,
                pos2(card.right() - 16.0, card.center().y),
                on,
                "soundcloud-originals",
            ) {
                app.set_soundcloud_originals(!on);
            }
            labels(
                ui,
                &theme,
                pos2(card.left() + 16.0, card.center().y),
                "Play the uploader's own file",
                "When the artist allows downloads, their file plays (often WAV or FLAC, lossless) instead of the AAC stream. SoundCloud counts it as a download, not a play.",
                width - 32.0 - 60.0,
            );
            y = card.bottom() + GROUP_GAP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Lyrics",
                "Where a song's lyrics come from when its service has none, in this order.",
            );
            let card = Rect::from_min_size(pos2(left, y), vec2(width, 4.0 * ROW));
            panel(ui, &theme, card, 4);
            lyrics_sites(app, ui, &theme, card);
            y = card.bottom() + GROUP_GAP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Theme",
                "How the window is laid out and coloured. Each TOML file in ~/.config/music/themes/ is one more.",
            );
            let card = Rect::from_min_size(
                pos2(left, y),
                vec2(width, THEMES * theme_rows() as f32),
            );
            panel(ui, &theme, card, 1);
            themes(app, ui, &theme, card);
            y = card.bottom() + GROUP_GAP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Colours",
                "Amane recolours Music along with the rest of the desktop.",
            );
            let card = Rect::from_min_size(pos2(left, y), vec2(width, THEMES));
            panel(ui, &theme, card, 1);
            appearance(app, ui, &theme, card);
            y = card.bottom() + GROUP_GAP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Artwork",
                "Covers that move, where Apple made them: on album pages and in Now Playing.",
            );
            let card = Rect::from_min_size(pos2(left, y), vec2(width, 2.0 * ROW));
            panel(ui, &theme, card, 2);
            let on = app.animated_artwork();
            if switch(
                ui,
                &theme,
                pos2(card.right() - 16.0, card.top() + ROW / 2.0),
                on,
                "animated-artwork",
            ) {
                app.set_animated_artwork(!on);
            }
            labels(
                ui,
                &theme,
                pos2(card.left() + 16.0, card.top() + ROW / 2.0),
                "Animated artwork",
                "Play a cover's video when it has one. It takes a little more power.",
                width - 32.0 - 60.0,
            );
            let quality =
                motion_quality(app, ui, &theme, card.right() - 16.0, card.top() + 1.5 * ROW);
            labels(
                ui,
                &theme,
                pos2(card.left() + 16.0, card.top() + 1.5 * ROW),
                "Animated artwork quality",
                "Automatic fetches each video at the size it takes on screen.",
                quality - card.left() - 32.0,
            );
            y = card.bottom() + GROUP_GAP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Interface",
                "How large everything is drawn, on top of the screen's own scale.",
            );
            let card = Rect::from_min_size(pos2(left, y), vec2(width, ROW));
            panel(ui, &theme, card, 1);
            scale(app, ui, &theme, card);
            y = card.bottom() + GROUP_GAP;

            y = group(
                ui,
                &theme,
                left,
                y,
                width,
                "Devices",
                "Your phone plays here and controls this computer, once paired on the same Wi-Fi; away from it, through a relay.",
            );
            let card = Rect::from_min_size(
                pos2(left, y),
                vec2(width, (paired + 1) as f32 * ROW),
            );
            panel(ui, &theme, card, paired + 1);
            devices(app, ui, &theme, card);
            relay(app, ui, &theme, card);
            y = card.bottom() + GROUP_GAP;

            y = group(ui, &theme, left, y, width, "About", "");
            let card = Rect::from_min_size(pos2(left, y), vec2(width, ROW));
            panel(ui, &theme, card, 1);
            labels(
                ui,
                &theme,
                pos2(card.left() + 16.0, card.center().y),
                &format!("Music {}", env!("CARGO_PKG_VERSION")),
                "GPL-3.0-or-later. Widevine and CENC code adapted from Kopuz (EUPL-1.2).",
                width - 32.0,
            );
        });
    output.state.clamped_offset().y
}

/// A group's title and its line of explanation; returns where its card goes.
fn group(ui: &Ui, theme: &Theme, left: f32, top: f32, width: f32, title: &str, blurb: &str) -> f32 {
    let g = text(ui, title, font(18.0, Weight::SemiBold), theme.text, width);
    paint_text(ui, g, left, top + 11.0, egui::Align::Min, theme.text);
    if blurb.is_empty() {
        return top + GROUP_HEAD - 18.0;
    }
    let muted = theme.muted();
    let g = text(ui, blurb, font(13.0, Weight::Regular), muted, width);
    paint_text(ui, g, left, top + 33.0, egui::Align::Min, muted);
    top + GROUP_HEAD
}

/// A card of `rows` rows, with hairlines between them.
fn panel(ui: &Ui, theme: &Theme, rect: Rect, rows: usize) {
    ui.painter().rect(
        rect,
        12.0,
        theme.text_a(0.04),
        Stroke::new(1.0, theme.line()),
        egui::StrokeKind::Inside,
    );
    for i in 1..rows {
        let y = rect.top() + i as f32 * ROW;
        ui.painter()
            .hline(rect.x_range(), y, Stroke::new(1.0, theme.line()));
    }
}

/// A setting's name and, under it, what it does.
fn labels(ui: &Ui, theme: &Theme, at: egui::Pos2, name: &str, detail: &str, width: f32) {
    let g = text(ui, name, font(13.5, Weight::Medium), theme.text, width);
    paint_text(ui, g, at.x, at.y - 9.0, egui::Align::Min, theme.text);
    let muted = theme.muted();
    let g = text(ui, detail, font(12.0, Weight::Regular), muted, width);
    paint_text(ui, g, at.x, at.y + 10.0, egui::Align::Min, muted);
}

/// An on/off switch ending at `right`, as the design draws it: the accent
/// when on, the knob sliding across. Returns whether it was clicked.
fn switch(ui: &mut Ui, theme: &Theme, right_center: egui::Pos2, on: bool, salt: &str) -> bool {
    let rect = Rect::from_min_max(
        pos2(right_center.x - 38.0, right_center.y - 11.0),
        pos2(right_center.x, right_center.y + 11.0),
    );
    let response = ui.interact(rect, Id::unique(("switch", salt)), Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(Id::unique(("switch-anim", salt)), on, 0.18);
    let off = theme.surface2;
    ui.painter()
        .rect_filled(rect, 11.0, crate::color::mix(theme.accent, off, t));
    let x = rect.left() + 11.0 + t * 16.0;
    ui.painter()
        .circle_filled(pos2(x, rect.center().y), 8.0, egui::Color32::WHITE);
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// A small button on a faint fill, ending at `right`. Returns whether it was
/// clicked.
fn button(ui: &mut Ui, theme: &Theme, right: f32, y: f32, label: &str, enabled: bool) -> bool {
    let g = text(ui, label, font(12.5, Weight::Medium), theme.text, 300.0);
    let rect = Rect::from_min_max(
        pos2(right - g.size().x - 28.0, y - 15.0),
        pos2(right, y + 15.0),
    );
    let response = ui.interact(
        rect,
        Id::unique(("settings-button", label, y as i32)),
        Sense::click(),
    );
    let hovered = enabled && response.hovered();
    let fill = theme.text_a(if hovered { 0.14 } else { 0.08 });
    ui.painter().rect_filled(rect, 8.0, fill);
    let color = if enabled { theme.text } else { theme.muted() };
    paint_text(ui, g, rect.left() + 14.0, y, egui::Align::Min, color);
    enabled
        && response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
}

/// The relay the devices meet through away from home, on the card's last
/// row; kept once left.
fn relay(app: &mut App, ui: &mut Ui, theme: &Theme, card: Rect) {
    let y = card.bottom() - ROW / 2.0;
    let width = 260.0;
    let rect = Rect::from_min_size(
        pos2(card.right() - 16.0 - width, y - 14.0),
        vec2(width, 28.0),
    );
    let field = ui.put(
        rect,
        egui::TextEdit::singleline(&mut app.relay_field)
            .hint_text("n0's relays")
            .font(font(12.5, Weight::Regular))
            .margin(egui::Margin::symmetric(8, 6))
            .desired_width(width),
    );
    if field.lost_focus() {
        app.keep_relay();
    }
    labels(
        ui,
        theme,
        pos2(card.left() + 16.0, y),
        "Relay",
        "Passes on what the devices say away from home, unable to read it. Used from the next start.",
        rect.left() - card.left() - 32.0,
    );
}

/// The devices paired, each with Forget; or how to pair one.
fn devices(app: &mut App, ui: &mut Ui, theme: &Theme, card: Rect) {
    let paired: Vec<connect::Peer> = app
        .devices
        .peers
        .iter()
        .filter(|p| p.paired)
        .cloned()
        .collect();
    if paired.is_empty() {
        let port = app.devices.node.port();
        labels(
            ui,
            theme,
            pos2(card.left() + 16.0, card.center().y),
            "No device paired yet",
            &format!(
                "On the phone, open Now Playing and tap the device under the controls. \
                 The firewall must let in TCP {port} and UDP 5353."
            ),
            card.width() - 32.0,
        );
        return;
    }
    for (i, peer) in paired.iter().enumerate() {
        let y = card.top() + i as f32 * ROW + ROW / 2.0;
        let state = match (&peer.playing, peer.connected) {
            (Some(p), true) if p.playing => match &p.track {
                Some(t) => format!("Playing “{}”", t.title),
                None => "Connected".to_owned(),
            },
            (_, true) => "Connected".to_owned(),
            _ => "Not on the network".to_owned(),
        };
        if button(ui, theme, card.right() - 16.0, y, "Forget", true) {
            app.devices.node.forget(&peer.device.id);
        }
        labels(
            ui,
            theme,
            pos2(card.left() + 16.0, y),
            &peer.device.name,
            &state,
            card.width() - 32.0 - 100.0,
        );
    }
}

fn services(app: &mut App, ui: &mut Ui, theme: &Theme, card: Rect) {
    let names = ["Apple Music", "SoundCloud", "Navidrome"];
    for (i, name) in names.into_iter().enumerate() {
        let y = card.top() + i as f32 * ROW + ROW / 2.0;
        let tile = Rect::from_center_size(pos2(card.left() + 16.0 + 20.0, y), vec2(40.0, 40.0));
        let x = tile.right() + 14.0;
        // Its switch at the end: off, the service is left signed in, unused.
        let provider = [
            ProviderKind::AppleMusic,
            ProviderKind::SoundCloud,
            ProviderKind::Navidrome,
        ][i];
        let on = app.service_on(provider);
        if switch(
            ui,
            theme,
            pos2(card.right() - 16.0, y),
            on,
            &format!("service-{i}"),
        ) {
            app.set_service(provider, !on);
        }
        let right = card.right() - 16.0 - 60.0;
        if !on {
            service_logo(ui, theme, provider, tile, false);
            labels(
                ui,
                theme,
                pos2(x, y),
                name,
                "Off: still signed in, and used again once turned on.",
                right - x - 12.0,
            );
            continue;
        }
        // What the row says, its button (label, enabled) and whether the
        // service is connected.
        let (detail, button_state, connected) = match i {
            0 => {
                let detail = match &app.connection {
                    Connection::Connected => format!(
                        "Storefront {} · AAC 256 kbps at best (ALAC is FairPlay-only)",
                        app.storefront.as_deref().unwrap_or("?").to_uppercase()
                    ),
                    Connection::Connecting => "Connecting…".to_owned(),
                    Connection::NotSignedIn => {
                        "Not signed in. Signing in opens Apple's page in Brave Origin.".to_owned()
                    }
                    Connection::Failed(e) => e.clone(),
                };
                let connected = app.connection == Connection::Connected;
                let button = if app.signing_in {
                    ("Waiting for you in the browser…", false)
                } else if connected {
                    ("Sign in again", true)
                } else {
                    ("Sign in", app.connection != Connection::Connecting)
                };
                (detail, button, connected)
            }
            1 => {
                let detail = match &app.soundcloud {
                    SoundCloudState::Connected(user) => format!(
                        "{user} · AAC 160 kbps (256 with Go+), the uploader's own file when they allow it"
                    ),
                    SoundCloudState::Connecting => "Connecting…".to_owned(),
                    SoundCloudState::NotSignedIn => "Not signed in. Signing in opens SoundCloud in Brave Origin: close it once you're in.".to_owned(),
                    SoundCloudState::Failed(e) => e.clone(),
                };
                let connected = matches!(app.soundcloud, SoundCloudState::Connected(_));
                let button = if app.soundcloud_signing_in {
                    ("Sign in, then close the browser…", false)
                } else if connected {
                    ("Sign in again", true)
                } else {
                    ("Sign in", app.soundcloud != SoundCloudState::Connecting)
                };
                (detail, button, connected)
            }
            _ => {
                let connected = matches!(app.navidrome, SoundCloudState::Connected(_));
                if !connected || app.navidrome_editing {
                    service_logo(ui, theme, provider, tile, false);
                    navidrome_form(app, ui, theme, x, right, y);
                    continue;
                }
                let user = match &app.navidrome {
                    SoundCloudState::Connected(user) => user.clone(),
                    _ => String::new(),
                };
                (
                    format!("{user} · the server's own files, played as they are"),
                    ("Sign in again", true),
                    true,
                )
            }
        };
        service_logo(ui, theme, provider, tile, connected);

        let (label, enabled) = button_state;
        // The buttons first, so the words know how much room is left: Sign
        // Out, while signed in, then the sign-in's.
        let g = text(ui, label, font(12.5, Weight::Medium), theme.text, 300.0);
        let mut button_left = right - g.size().x - 28.0;
        if button(ui, theme, right, y, label, enabled) {
            match i {
                0 => app.sign_in(),
                1 => app.sign_in_soundcloud(),
                _ => app.navidrome_editing = true,
            }
        }
        if connected {
            let now = ui.input(|i| i.time);
            let out = if app.sign_out_asked(provider, now) {
                // Asked once: shown till the moment passes.
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(500));
                "Sign Out, sure?"
            } else {
                "Sign Out"
            };
            let out_right = button_left - 8.0;
            let g = text(ui, out, font(12.5, Weight::Medium), theme.text, 300.0);
            if button(ui, theme, out_right, y, out, true) {
                app.sign_out(provider, now);
            }
            button_left = out_right - g.size().x - 28.0;
        }
        let mut state_right = button_left - 14.0;
        if connected {
            let g = text(
                ui,
                "Connected",
                font(12.0, Weight::Medium),
                theme.subtext0,
                100.0,
            );
            let w = g.size().x;
            paint_text(ui, g, state_right, y, egui::Align::Max, theme.subtext0);
            ui.painter()
                .circle_filled(pos2(state_right - w - 9.0, y), 3.5, theme.green);
            state_right -= w + 20.0;
        }
        labels(ui, theme, pos2(x, y), name, &detail, state_right - x - 12.0);
    }
}

/// A service's logo on its row, dimmed unless it is connected.
fn service_logo(ui: &Ui, theme: &Theme, provider: ProviderKind, tile: Rect, connected: bool) {
    paint_icon(
        ui,
        icons::service(provider),
        tile.center(),
        tile.width(),
        egui::Color32::WHITE,
    );
    if !connected {
        ui.painter().rect_filled(
            tile.expand(1.0),
            10.0,
            crate::color::alpha(theme.mantle, 0.55),
        );
    }
}

/// The server's sign-in, on its row: where it is, the user and the
/// password, which goes once to make the token kept.
fn navidrome_form(app: &mut App, ui: &mut Ui, theme: &Theme, x: f32, right: f32, y: f32) {
    let signing = app.navidrome_signing_in;
    let label = if signing { "Signing in…" } else { "Sign in" };
    let g = text(ui, label, font(12.5, Weight::Medium), theme.text, 300.0);
    let button_left = right - g.size().x - 28.0;
    let filled = !app.navidrome_form.url.trim().is_empty()
        && !app.navidrome_form.user.trim().is_empty()
        && !app.navidrome_form.password.is_empty();
    let mut submit = button(ui, theme, right, y, label, filled && !signing);
    let field =
        |ui: &mut Ui, left: f32, width: f32, value: &mut String, hint: &str, secret: bool| {
            let rect = Rect::from_min_size(pos2(left, y - 14.0), vec2(width, 28.0));
            ui.put(
                rect,
                egui::TextEdit::singleline(value)
                    .hint_text(hint)
                    .password(secret)
                    .font(font(12.5, Weight::Regular))
                    .margin(egui::Margin::symmetric(8, 6))
                    .desired_width(width),
            )
        };
    let gap = 8.0;
    let (url_w, user_w, pass_w) = (170.0, 100.0, 100.0);
    let pass_left = button_left - gap - pass_w;
    let user_left = pass_left - gap - user_w;
    let url_left = user_left - gap - url_w;
    let form = &mut app.navidrome_form;
    field(ui, url_left, url_w, &mut form.url, "0.0.0.0:4533", false);
    field(ui, user_left, user_w, &mut form.user, "User", false);
    let password = field(ui, pass_left, pass_w, &mut form.password, "Password", true);
    if password.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && filled {
        submit = true;
    }
    let detail = match &app.navidrome {
        SoundCloudState::Failed(e) => e.clone(),
        SoundCloudState::Connecting => "Connecting…".to_owned(),
        _ => "Your own server: its files play as they are, up to 24-bit.".to_owned(),
    };
    labels(
        ui,
        theme,
        pos2(x, y),
        "Navidrome",
        &detail,
        url_left - x - 12.0,
    );
    if submit && !signing {
        app.sign_in_navidrome();
    }
}

/// The lyrics sites, each with its switch and a way to ask it sooner, then
/// whether they come before the service's own.
fn lyrics_sites(app: &mut App, ui: &mut Ui, theme: &Theme, card: Rect) {
    let sites = app.lyrics_sites();
    for (i, (source, on)) in sites.iter().copied().enumerate() {
        let y = card.top() + i as f32 * ROW + ROW / 2.0;
        if switch(
            ui,
            theme,
            pos2(card.right() - 16.0, y),
            on,
            &format!("lyrics-site-{i}"),
        ) {
            app.set_lyrics_site(source, !on);
        }
        // The first is asked first already.
        let mut words_right = card.right() - 16.0 - 60.0;
        if i > 0 {
            let up = Rect::from_center_size(pos2(words_right - 14.0, y), vec2(28.0, 28.0));
            let mut up_ui = ui.new_child(egui::UiBuilder::new().max_rect(up));
            if super::icon_button(
                &mut up_ui,
                theme,
                icons::ARROW_UP,
                28.0,
                14.0,
                theme.subtext0,
                true,
            )
            .on_hover_text("Ask it sooner")
            .clicked()
            {
                app.raise_lyrics_site(source);
            }
            words_right = up.left() - 8.0;
        }
        let detail = match source {
            lyrics::Source::Lrclib => "Open and free, made for synced lyrics.",
            lyrics::Source::NetEase => "A large catalogue, with much from Asia.",
            lyrics::Source::KuGou => "Another large catalogue, from China.",
        };
        labels(
            ui,
            theme,
            pos2(card.left() + 16.0, y),
            &format!("{}. {}", i + 1, source.name()),
            detail,
            words_right - card.left() - 32.0,
        );
    }
    let y = card.top() + sites.len() as f32 * ROW + ROW / 2.0;
    let first = app.lyrics_sites_first();
    if switch(
        ui,
        theme,
        pos2(card.right() - 16.0, y),
        first,
        "lyrics-sites-first",
    ) {
        app.set_lyrics_sites_first(!first);
    }
    labels(
        ui,
        theme,
        pos2(card.left() + 16.0, y),
        "Before the service's own",
        "Look for the sites' lyrics first, even where Apple Music or the server has some.",
        card.width() - 32.0 - 60.0,
    );
}

/// The theme cards side by side.
const THEMES_PER_ROW: usize = 3;

/// How many rows of theme cards there are.
fn theme_rows() -> usize {
    Theme::catalogue().len().div_ceil(THEMES_PER_ROW)
}

/// The themes to choose from, three to a row.
fn themes(app: &mut App, ui: &mut Ui, theme: &Theme, card: Rect) {
    let chosen = app.theme_id().to_string();
    let inner = card.shrink(14.0);
    let gap = 12.0;
    let n = THEMES_PER_ROW as f32;
    let w = (inner.width() - (n - 1.0) * gap) / n;
    let h = THEMES - 28.0;
    for (i, (id, name, file)) in Theme::catalogue().into_iter().enumerate() {
        let (row, col) = (i / THEMES_PER_ROW, i % THEMES_PER_ROW);
        let rect = Rect::from_min_size(
            pos2(
                inner.left() + col as f32 * (w + gap),
                inner.top() + row as f32 * (h + 28.0),
            ),
            vec2(w, h),
        );
        // In the colours chosen, as it would show.
        let shown = file.as_ref().ok().map(|t| match app.appearance() {
            Appearance::Amane => Theme::amane().map_or(t.clone(), |c| t.recoloured(&c)),
            Appearance::Stock => t.clone(),
        });
        let detail = match &file {
            Ok(t) => layout_line(t),
            Err(why) => why.clone(),
        };
        if option_card(
            ui,
            theme,
            rect,
            &id,
            &name,
            &detail,
            shown.as_ref(),
            chosen == id,
        ) {
            app.set_theme(&id, ui.ctx());
        }
    }
}

/// What a theme's layout is like, in a few words.
fn layout_line(t: &Theme) -> String {
    use crate::theme::{PlayerBar, Sidebar, Window};
    let parts = [
        match t.layout.window {
            Window::Flat => "Edge to edge",
            Window::Cards => "Cards",
        },
        match t.layout.sidebar {
            Sidebar::Navigation => "pages on the left",
            Sidebar::Library => "library on the left",
        },
        match t.layout.player_bar {
            PlayerBar::Floating => "floating player",
            PlayerBar::Docked => "player along the bottom",
        },
    ];
    parts.join(", ")
}

fn appearance(app: &mut App, ui: &mut Ui, theme: &Theme, card: Rect) {
    let amane = Theme::amane();
    let chosen = app.appearance();
    let own = Theme::named(app.theme_id()).unwrap_or(Theme::STOCK);
    let options = [
        (
            Appearance::Stock,
            "The theme's",
            "The colours the theme comes with.",
            Some(own.clone()),
        ),
        (
            Appearance::Amane,
            "Amane",
            if amane.is_some() {
                "Follows switch-theme.sh."
            } else {
                "Amane's theme file isn't there yet."
            },
            amane.map(|c| own.recoloured(&c)),
        ),
    ];
    let inner = card.shrink(14.0);
    let w = (inner.width() - 12.0) / 2.0;
    for (i, (kind, name, detail, colors)) in options.into_iter().enumerate() {
        let rect = Rect::from_min_size(
            pos2(inner.left() + i as f32 * (w + 12.0), inner.top()),
            vec2(w, inner.height()),
        );
        if option_card(
            ui,
            theme,
            rect,
            name,
            name,
            detail,
            colors.as_ref(),
            chosen == kind,
        ) {
            app.set_appearance(kind, ui.ctx());
        }
    }
}

/// A choice drawn as a card: a small picture of the app, its name, a line,
/// its colours. Whether it was chosen just now.
#[allow(clippy::too_many_arguments)]
fn option_card(
    ui: &mut Ui,
    theme: &Theme,
    rect: Rect,
    salt: &str,
    name: &str,
    detail: &str,
    colors: Option<&Theme>,
    on: bool,
) -> bool {
    let w = rect.width();
    let enabled = colors.is_some();
    let response = ui.interact(rect, Id::unique(("theme-option", salt)), Sense::click());
    if enabled && response.hovered() && !on {
        ui.painter().rect_filled(rect, 12.0, theme.hover());
    }
    let border = if on {
        Stroke::new(2.0, theme.accent)
    } else {
        Stroke::new(1.0, theme.line())
    };
    ui.painter()
        .rect_stroke(rect, 12.0, border, egui::StrokeKind::Inside);
    let preview = Rect::from_min_size(rect.min + vec2(10.0, 10.0), vec2(w - 20.0, 74.0));
    let x = preview.left();
    let mut y = preview.bottom() + 10.0;
    match colors {
        Some(c) => paint_preview(ui, preview, c),
        None => {
            ui.painter().rect_filled(preview, 8.0, theme.text_a(0.04));
        }
    }
    let g = text(ui, name, font(13.5, Weight::SemiBold), theme.text, w - 20.0);
    paint_text(ui, g, x, y + 9.0, egui::Align::Min, theme.text);
    y += 18.0;
    let muted = theme.muted();
    let g = text(ui, detail, font(12.0, Weight::Regular), muted, w - 20.0);
    paint_text(ui, g, x, y + 9.0, egui::Align::Min, muted);
    y += 18.0 + 4.0;
    if let Some(c) = colors {
        for (k, color) in [c.base, c.surface1, c.text, c.accent, c.mauve]
            .into_iter()
            .enumerate()
        {
            let swatch = Rect::from_min_size(pos2(x + k as f32 * 18.0, y), vec2(14.0, 14.0));
            ui.painter().rect(
                swatch,
                4.0,
                color,
                Stroke::new(1.0, theme.line()),
                egui::StrokeKind::Inside,
            );
        }
    }
    let clicked = response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(detail)
        .clicked();
    enabled && clicked && !on
}

/// A small picture of the app in `c`'s colours and layout: the sidebar, a
/// title, a line, the player bar.
fn paint_preview(ui: &Ui, rect: Rect, c: &Theme) {
    use crate::theme::{PlayerBar, Window};
    let p = ui.painter();
    let at = |fx: f32, fy: f32| pos2(rect.left() + fx * rect.width(), rect.top() + fy);
    if c.layout.window == Window::Cards {
        // Cards on crust: the library, the page, the player under them.
        p.rect_filled(rect, 8.0, c.crust);
        let gap = 4.0;
        let bottom = rect.bottom() - 18.0;
        let side = Rect::from_min_max(
            rect.min + vec2(gap, gap + 8.0),
            pos2(rect.left() + 0.26 * rect.width(), bottom),
        );
        let page = Rect::from_min_max(
            pos2(side.right() + gap, side.top()),
            pos2(rect.right() - gap, bottom),
        );
        p.rect_filled(side, 4.0, c.base);
        p.rect_filled(page, 4.0, c.base);
        p.rect_filled(
            Rect::from_center_size(
                pos2(rect.center().x, rect.top() + 6.0),
                vec2(0.3 * rect.width(), 6.0),
            ),
            3.0,
            c.surface1,
        );
        p.rect_filled(
            Rect::from_min_size(page.min + vec2(8.0, 8.0), vec2(0.3 * page.width(), 7.0)),
            3.5,
            c.text,
        );
        p.circle_filled(pos2(page.left() + 14.0, page.top() + 30.0), 6.0, c.accent);
        p.circle_filled(pos2(rect.center().x, rect.bottom() - 9.0), 4.0, c.text);
        p.rect_filled(
            Rect::from_center_size(
                pos2(rect.center().x, rect.bottom() - 3.0),
                vec2(0.36 * rect.width(), 2.0),
            ),
            1.0,
            c.surface2,
        );
        return;
    }
    p.rect_filled(rect, 8.0, c.base);
    p.rect_filled(
        Rect::from_min_max(
            rect.min,
            pos2(rect.left() + 0.28 * rect.width(), rect.bottom()),
        ),
        egui::CornerRadius {
            nw: 8,
            sw: 8,
            ne: 0,
            se: 0,
        },
        c.crust,
    );
    p.rect_filled(
        Rect::from_min_size(at(0.36, 14.0), vec2(0.28 * rect.width(), 8.0)),
        4.0,
        c.text,
    );
    p.rect_filled(
        Rect::from_min_size(at(0.36, 30.0), vec2(0.18 * rect.width(), 6.0)),
        3.0,
        c.subtext0,
    );
    let bar = if c.layout.player_bar == PlayerBar::Docked {
        Rect::from_min_max(
            pos2(rect.left() + 0.28 * rect.width(), rect.bottom() - 14.0),
            pos2(rect.right(), rect.bottom()),
        )
    } else {
        Rect::from_min_max(
            pos2(rect.left() + 0.36 * rect.width(), rect.bottom() - 22.0),
            pos2(rect.right() - 0.10 * rect.width(), rect.bottom() - 10.0),
        )
    };
    p.rect_filled(bar, 6.0, c.mantle);
    p.rect_filled(
        Rect::from_min_size(bar.min, vec2(0.4 * bar.width(), bar.height())),
        6.0,
        c.accent,
    );
}

/// Automatic, Low and High, ending at `right`; returns where they begin.
fn motion_quality(app: &mut App, ui: &mut Ui, theme: &Theme, right: f32, y: f32) -> f32 {
    use crate::settings::MotionQuality;
    let chosen = app.motion_quality();
    let enabled = app.animated_artwork();
    let options = [
        (MotionQuality::Automatic, "Automatic"),
        (MotionQuality::Low, "Low"),
        (MotionQuality::High, "High"),
    ];
    let body = font(12.5, Weight::Medium);
    let widths: Vec<f32> = options
        .iter()
        .map(|(_, label)| text(ui, label, body.clone(), theme.text, 200.0).size().x + 24.0)
        .collect();
    let mut x = right - widths.iter().sum::<f32>() - 6.0 * (options.len() - 1) as f32;
    let left = x;
    for ((quality, label), w) in options.into_iter().zip(widths) {
        let rect = Rect::from_min_size(pos2(x, y - 14.0), vec2(w, 28.0));
        x += w + 6.0;
        let response = ui.interact(rect, Id::unique(("motion-quality", label)), Sense::click());
        let on = quality == chosen;
        let (fill, fg) = if on {
            (theme.text_a(if enabled { 0.9 } else { 0.4 }), theme.base)
        } else if enabled && response.hovered() {
            (theme.text_a(0.14), theme.text)
        } else {
            (
                theme.text_a(0.08),
                if enabled { theme.text } else { theme.muted() },
            )
        };
        ui.painter().rect_filled(rect, 14.0, fill);
        let g = text(ui, label, body.clone(), fg, w);
        paint_text(
            ui,
            g,
            rect.center().x,
            rect.center().y,
            egui::Align::Center,
            fg,
        );
        if enabled
            && response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
        {
            app.set_motion_quality(quality);
        }
    }
    left
}

fn scale(app: &mut App, ui: &mut Ui, theme: &Theme, card: Rect) {
    let y = card.center().y;
    let right = card.right() - 16.0;
    let current = app.scale();
    // From the right: Reset, +, the value, −.
    if button(ui, theme, right, y, "Reset", true) {
        app.set_scale(None);
    }
    let g = text(ui, "Reset", font(12.5, Weight::Medium), theme.text, 300.0);
    let mut x = right - g.size().x - 28.0 - 12.0;
    let step = |ui: &mut Ui, x: f32, icon: Icon, salt: &str| {
        let rect = Rect::from_center_size(pos2(x - 15.0, y), vec2(30.0, 30.0));
        let response = ui.interact(rect, Id::unique(("scale", salt)), Sense::click());
        let fill = theme.text_a(if response.hovered() { 0.14 } else { 0.08 });
        ui.painter().rect_filled(rect, 8.0, fill);
        paint_icon(ui, icon, rect.center(), 14.0, theme.text);
        response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
    };
    if step(ui, x, icons::PLUS, "bigger") {
        app.set_scale(Some(current + 0.1));
    }
    x -= 30.0 + 8.0;
    let value = format!("{:.0} %", current * 100.0);
    let g = text(ui, &value, font(13.0, Weight::Medium), theme.text, 80.0);
    paint_text(ui, g, x - 30.0, y, egui::Align::Center, theme.text);
    x -= 60.0 + 8.0;
    if step(ui, x, icons::MINUS, "smaller") {
        app.set_scale(Some(current - 0.1));
    }
    x -= 30.0;
    labels(
        ui,
        theme,
        pos2(card.left() + 16.0, y),
        "Interface scale",
        "Ctrl + and Ctrl − change it too; Ctrl 0 goes back to 100 %.",
        x - card.left() - 32.0,
    );
}
