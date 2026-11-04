//! Play On, from the player bar: this computer, then the devices paired and
//! on the network; a click moves the music there, as with Spotify's
//! devices. Those not paired yet are below, to pair.

use egui::{Frame, Rect, Sense, Stroke, Ui, pos2, vec2};

use connect::{DeviceKind, Peer};

use crate::app::{App, Shown};
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::{icon_button, paint_icon, paint_text, text};

const WIDTH: f32 = 300.0;
const ROW: f32 = 48.0;

/// What the user picked in the list.
enum Pick {
    PlayOn(Option<String>),
    Pair(String),
}

fn icon(peer: &Peer) -> Icon {
    match peer.device.kind {
        DeviceKind::Phone => icons::PHONE,
        DeviceKind::Computer => icons::MONITOR,
    }
}

/// What a device does, in a few words.
fn status(peer: &Peer) -> String {
    match peer.playing.as_ref() {
        _ if !peer.connected => "Not on the network".to_owned(),
        Some(p) => match &p.track {
            Some(t) if p.playing => format!("Playing “{}”", t.title),
            Some(t) => format!("Paused · “{}”", t.title),
            None => "Nothing playing".to_owned(),
        },
        None => "Nothing playing".to_owned(),
    }
}

/// The button, in the accent while another device plays, and its list.
pub fn button(app: &mut App, ui: &mut Ui, theme: &Theme, shown: &Shown) {
    let color = if shown.on.is_some() {
        theme.accent
    } else {
        theme.text_a(0.82)
    };
    let tip = match &shown.on {
        Some(device) => format!("Playing on {}", device.name),
        None => "Play on another device".to_owned(),
    };
    let response =
        icon_button(ui, theme, icons::DEVICES, 30.0, 16.0, color, true).on_hover_text(tip);
    let frame = Frame::new()
        .fill(theme.mantle)
        // M3E's popovers: radius 28, no outline.
        .stroke(if theme.expressive() {
            Stroke::NONE
        } else {
            Stroke::new(1.0, theme.line())
        })
        .corner_radius(if theme.expressive() { 28 } else { 14 })
        .inner_margin(if theme.expressive() { 12 } else { 6 })
        .shadow(egui::Shadow {
            offset: [0, 10],
            blur: 30,
            spread: 0,
            color: egui::Color32::from_black_alpha(110),
        });
    let peers = app.devices.peers.clone();
    let here = shown.on.is_none();
    let on = shown.on.as_ref().map(|d| d.id.clone());
    let mut pick = None;
    egui::Popup::menu(&response)
        .align(egui::RectAlign::TOP_END)
        .gap(12.0)
        .frame(frame)
        .width(WIDTH)
        .show(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            heading(ui, theme, "Play On");
            if row(ui, theme, icons::MONITOR, "This Computer", None, here, None) {
                pick = Some(Pick::PlayOn(None));
            }
            let paired: Vec<&Peer> = peers.iter().filter(|p| p.paired && p.connected).collect();
            for peer in &paired {
                let chosen = on.as_deref() == Some(peer.device.id.as_str());
                let detail = status(peer);
                if row(
                    ui,
                    theme,
                    icon(peer),
                    &peer.device.name,
                    Some(&detail),
                    chosen,
                    None,
                ) {
                    pick = Some(Pick::PlayOn(Some(peer.device.id.clone())));
                }
            }
            let nearby: Vec<&Peer> = peers.iter().filter(|p| !p.paired && p.nearby).collect();
            if !nearby.is_empty() {
                heading(ui, theme, "Nearby");
                for peer in nearby {
                    let detail = "Pair once: it shows a code to type in here";
                    if row(
                        ui,
                        theme,
                        icon(peer),
                        &peer.device.name,
                        Some(detail),
                        false,
                        Some("Pair"),
                    ) {
                        pick = Some(Pick::Pair(peer.device.id.clone()));
                    }
                }
            } else if paired.is_empty() {
                note(
                    ui,
                    theme,
                    "Your phone shows here once Opium is open on it, on the same Wi-Fi.",
                );
            }
            if pick.is_some() {
                ui.close();
            }
        });
    match pick {
        Some(Pick::PlayOn(id)) => app.play_on(id),
        Some(Pick::Pair(id)) => app.devices.node.pair(&id),
        None => {}
    }
}

fn heading(ui: &mut Ui, theme: &Theme, title: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::hover());
    let muted = theme.muted();
    let g = text(ui, title, font(12.0, Weight::SemiBold), muted, rect.width());
    paint_text(
        ui,
        g,
        rect.left() + 10.0,
        rect.center().y + 2.0,
        egui::Align::Min,
        muted,
    );
}

fn note(ui: &mut Ui, theme: &Theme, words: &str) {
    let muted = theme.muted();
    let g = text(
        ui,
        words,
        font(12.0, Weight::Regular),
        muted,
        ui.available_width() - 20.0,
    );
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), g.size().y + 16.0),
        Sense::hover(),
    );
    paint_text(
        ui,
        g,
        rect.left() + 10.0,
        rect.center().y,
        egui::Align::Min,
        muted,
    );
}

/// A device: its icon, name and what it does, a check when it plays the
/// music, or a button. Returns whether it was clicked.
fn row(
    ui: &mut Ui,
    theme: &Theme,
    icon: Icon,
    name: &str,
    detail: Option<&str>,
    chosen: bool,
    button: Option<&str>,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 8.0, theme.hover());
    }
    let tint = if chosen { theme.accent } else { theme.text };
    paint_icon(
        ui,
        icon,
        pos2(rect.left() + 10.0 + 10.0, rect.center().y),
        20.0,
        tint,
    );
    let left = rect.left() + 10.0 + 20.0 + 12.0;
    let right = rect.right() - if button.is_some() { 70.0 } else { 34.0 };
    let width = (right - left).max(0.0);
    let g = text(ui, name, font(13.5, Weight::Medium), tint, width);
    let y = if detail.is_some() {
        rect.center().y - 8.0
    } else {
        rect.center().y
    };
    paint_text(ui, g, left, y, egui::Align::Min, tint);
    if let Some(detail) = detail {
        let muted = theme.muted();
        let g = text(ui, detail, font(11.5, Weight::Regular), muted, width);
        paint_text(ui, g, left, rect.center().y + 9.0, egui::Align::Min, muted);
    }
    if chosen {
        paint_icon(
            ui,
            icons::CHECK,
            pos2(rect.right() - 10.0 - 8.0, rect.center().y),
            16.0,
            theme.accent,
        );
    }
    if let Some(label) = button {
        let g = text(
            ui,
            label,
            font(12.5, Weight::Medium),
            theme.on_accent(),
            100.0,
        );
        let pill = Rect::from_center_size(
            pos2(
                rect.right() - 10.0 - (g.size().x + 24.0) / 2.0,
                rect.center().y,
            ),
            vec2(g.size().x + 24.0, 28.0),
        );
        ui.painter().rect_filled(pill, 14.0, theme.accent);
        let on_accent = theme.on_accent();
        paint_text(
            ui,
            g,
            pill.center().x,
            pill.center().y,
            egui::Align::Center,
            on_accent,
        );
    }
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}
