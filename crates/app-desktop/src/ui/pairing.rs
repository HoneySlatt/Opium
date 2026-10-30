//! Pairing: the code to type into the device asking, shown large; or the
//! code the other device shows, typed in here.

use egui::{Color32, Frame, Id, Rect, Sense, Stroke, vec2};

use crate::app::App;
use crate::fonts::{Weight, font};

use super::playlist_dialog::pill;
use super::{paint_text, text};

pub fn show(app: &mut App, ctx: &egui::Context) {
    ask(app, ctx);
    let Some((device, code)) = app.devices.pairing.clone() else {
        return;
    };
    let theme = app.theme.clone();
    let frame = Frame::new()
        .fill(theme.mantle)
        .stroke(Stroke::new(1.0, theme.line()))
        .corner_radius(16)
        .inner_margin(0);
    let backdrop = if theme.light {
        Color32::from_black_alpha(72)
    } else {
        Color32::from_black_alpha(140)
    };
    let width = 400.0;
    let mut cancel = false;
    let response = egui::Modal::new(Id::unique("pairing"))
        .frame(frame)
        .backdrop_color(backdrop)
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

            let (head, _) = ui.allocate_exact_size(vec2(width, 200.0), Sense::hover());
            let g = text(
                ui,
                &format!("Pair with {}", device.name),
                font(18.0, Weight::Bold),
                theme.text,
                width - 48.0,
            );
            paint_text(
                ui,
                g,
                head.left() + 24.0,
                head.top() + 36.0,
                egui::Align::Min,
                theme.text,
            );
            let g = text(
                ui,
                &format!("Type this code on {}.", device.name),
                font(13.0, Weight::Regular),
                theme.subtext0,
                width - 48.0,
            );
            paint_text(
                ui,
                g,
                head.left() + 24.0,
                head.top() + 60.0,
                egui::Align::Min,
                theme.subtext0,
            );
            let shown = format!("{} {}", &code[..3], &code[3..]);
            let g = text(ui, &shown, font(40.0, Weight::Bold), theme.text, width);
            paint_text(
                ui,
                g,
                head.center().x,
                head.top() + 130.0,
                egui::Align::Center,
                theme.text,
            );

            let (foot, _) = ui.allocate_exact_size(vec2(width, 72.0), Sense::hover());
            ui.painter()
                .hline(foot.x_range(), foot.top(), Stroke::new(1.0, theme.line()));
            let rect = Rect::from_center_size(foot.center(), vec2(124.0, 40.0));
            if pill(ui, &theme, rect, "Cancel", false, true) {
                cancel = true;
            }
        });
    if cancel || response.should_close() {
        app.devices.node.cancel_pairing();
        app.devices.pairing = None;
    }
}

/// The other device shows a code: typed in here, it pairs them.
fn ask(app: &mut App, ctx: &egui::Context) {
    let Some((device, mut code)) = app.devices.asking.clone() else {
        return;
    };
    let theme = app.theme.clone();
    let frame = Frame::new()
        .fill(theme.mantle)
        .stroke(Stroke::new(1.0, theme.line()))
        .corner_radius(16)
        .inner_margin(0);
    let backdrop = if theme.light {
        Color32::from_black_alpha(72)
    } else {
        Color32::from_black_alpha(140)
    };
    let width = 400.0;
    let (mut cancel, mut pair) = (false, false);
    let response = egui::Modal::new(Id::unique("pairing-ask"))
        .frame(frame)
        .backdrop_color(backdrop)
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let (head, _) = ui.allocate_exact_size(vec2(width, 200.0), Sense::hover());
            let g = text(
                ui,
                &format!("Pair with {}", device.name),
                font(18.0, Weight::Bold),
                theme.text,
                width - 48.0,
            );
            paint_text(
                ui,
                g,
                head.left() + 24.0,
                head.top() + 36.0,
                egui::Align::Min,
                theme.text,
            );
            let g = text(
                ui,
                &format!("Type the code {} shows.", device.name),
                font(13.0, Weight::Regular),
                theme.subtext0,
                width - 48.0,
            );
            paint_text(
                ui,
                g,
                head.left() + 24.0,
                head.top() + 60.0,
                egui::Align::Min,
                theme.subtext0,
            );
            let field = Rect::from_center_size(
                egui::pos2(head.center().x, head.top() + 130.0),
                vec2(220.0, 56.0),
            );
            let edit = ui.put(
                field,
                egui::TextEdit::singleline(&mut code)
                    .font(font(36.0, Weight::Bold))
                    .align(egui::Align2::CENTER_CENTER)
                    .hint_text("000000")
                    .char_limit(6)
                    .frame(Frame::NONE),
            );
            if !edit.has_focus() && !edit.lost_focus() {
                edit.request_focus();
            }
            code.retain(|c| c.is_ascii_digit());
            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                pair = code.len() == 6;
            }

            let (foot, _) = ui.allocate_exact_size(vec2(width, 72.0), Sense::hover());
            ui.painter()
                .hline(foot.x_range(), foot.top(), Stroke::new(1.0, theme.line()));
            let y = foot.center().y;
            let cancel_rect =
                Rect::from_center_size(egui::pos2(foot.center().x - 70.0, y), vec2(124.0, 40.0));
            let pair_rect =
                Rect::from_center_size(egui::pos2(foot.center().x + 70.0, y), vec2(124.0, 40.0));
            if pill(ui, &theme, cancel_rect, "Cancel", false, true) {
                cancel = true;
            }
            if pill(ui, &theme, pair_rect, "Pair", true, code.len() == 6) {
                pair = true;
            }
        });
    if pair {
        app.devices.node.enter_code(code);
        app.devices.asking = None;
    } else if cancel || response.should_close() {
        app.devices.node.cancel_pairing();
        app.devices.asking = None;
    } else {
        app.devices.asking = Some((device, code));
    }
}
