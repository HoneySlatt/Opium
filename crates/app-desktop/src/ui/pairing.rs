//! A device asks to pair: the code to type into it, shown large, until it
//! is typed in there or pairing is called off here.

use egui::{Color32, Frame, Id, Rect, Sense, Stroke, vec2};

use crate::app::App;
use crate::fonts::{Weight, font};

use super::playlist_dialog::pill;
use super::{paint_text, text};

pub fn show(app: &mut App, ctx: &egui::Context) {
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
