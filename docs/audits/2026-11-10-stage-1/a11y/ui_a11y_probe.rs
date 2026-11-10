//! What Orca and the keyboard get from Opium's window (stage audit M1.8,
//! accessibility). Proofs for findings F1-known and D1 of the m18-a11y report.
//!
//! Place at `crates/app-desktop/src/ui/a11y_probe.rs` and declare it in
//! `crates/app-desktop/src/ui/mod.rs` beside the other modules:
//!
//!     #[cfg(test)]
//!     mod a11y_probe;
//!
//! Run: `cargo test -p app-desktop a11y_probe`.
//!
//! Not compiled by the reviewer, who may not edit tracked files. It uses the
//! egui 0.37 forms the crate's own tests use (`Context::run_ui`,
//! `RawInput { time, screen_rect }`, measure.rs:497-519) and egui's AccessKit
//! re-export (`egui::accesskit`, enabled through eframe's default features;
//! `accesskit_winit` is in Cargo.lock). It paints text in egui's default
//! proportional font: `fonts::font()` names Geist families a bare context
//! lacks, and `fonts::install` would run fc-list.
//!
//! Expected today:
//! - `the_play_button_is_named_for_screen_readers`: FAILS (no node is a
//!   button named "Play": `icon_button`, ui/mod.rs:1235-1272, gives egui no
//!   widget info; no `widget_info` call exists in the crate).
//! - `a_songs_title_reaches_screen_readers`: FAILS (painted text makes no
//!   node: `text` + `paint_text`, ui/mod.rs:1204-1227, only add shapes).
//! - `a_control_that_senses_clicks_can_take_the_focus`: the premise of the
//!   next test; if it fails, Tab cannot reach Opium's controls at all.
//! - `a_focused_button_leaves_the_players_keys_to_opium`: FAILS if egui's
//!   `egui_wants_keyboard_input` reports any focused widget, not only a
//!   text field: then `keys_taken` (ui/mod.rs:914-916) turns off Space,
//!   Escape (ui/mod.rs:918-935), the player's keys (ui/mod.rs:956-959) and
//!   the song table's arrows (ui/tracks.rs:719-721) while a button has the
//!   focus.

use egui::accesskit::Role;
use egui::{Rect, pos2, vec2};

use crate::theme::Theme;

fn input(time: f64) -> egui::RawInput {
    egui::RawInput {
        time: Some(time),
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))),
        ..Default::default()
    }
}

/// The nodes egui hands AccessKit on the second frame of `draw`, as (role,
/// label, value): a widget's node may come only once it was seen a frame.
fn nodes(draw: impl Fn(&mut egui::Ui)) -> Vec<(Role, Option<String>, Option<String>)> {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut last = None;
    for time in [0.0, 0.1] {
        let mut output = ctx.run_ui(input(time), |ui| draw(ui));
        // No painter here takes the font atlas (as measure.rs:519).
        output.textures_delta.clear();
        last = output.platform_output.accesskit_update;
    }
    last.expect("AccessKit was enabled")
        .nodes
        .into_iter()
        .map(|(_, node)| {
            (
                node.role(),
                node.label().map(str::to_owned),
                node.value().map(str::to_owned),
            )
        })
        .collect()
}

#[test]
fn the_play_button_is_named_for_screen_readers() {
    let theme = Theme::STOCK;
    let nodes = nodes(|ui| {
        super::icon_button(ui, &theme, crate::icons::PLAY_FILL, 32.0, 16.0, theme.text, true)
            .on_hover_text("Play");
    });
    assert!(
        nodes
            .iter()
            .any(|(role, label, _)| *role == Role::Button && label.as_deref() == Some("Play")),
        "no button named Play among {nodes:?}"
    );
}

#[test]
fn a_songs_title_reaches_screen_readers() {
    let theme = Theme::STOCK;
    let nodes = nodes(|ui| {
        let galley = super::text(ui, "A Song Title", egui::FontId::proportional(14.0), theme.text, 400.0);
        super::paint_text(ui, galley, 10.0, 20.0, egui::Align::Min, theme.text);
    });
    assert!(
        nodes.iter().any(|(_, label, value)| {
            label.as_deref() == Some("A Song Title") || value.as_deref() == Some("A Song Title")
        }),
        "the title is in no node: {nodes:?}"
    );
}

/// A button-like area, as Opium draws its 231 controls (`ui.interact` or
/// `allocate_exact_size` with `Sense::click()`), focused as Tab would.
fn focused_button() -> (egui::Context, egui::Id) {
    let ctx = egui::Context::default();
    let id = egui::Id::new("a11y-probe-button");
    let draw = |ui: &mut egui::Ui| {
        ui.interact(
            Rect::from_min_size(pos2(10.0, 10.0), vec2(32.0, 32.0)),
            id,
            egui::Sense::click(),
        );
    };
    for time in [0.0, 0.1, 0.2] {
        let mut output = ctx.run_ui(input(time), |ui| draw(ui));
        output.textures_delta.clear();
        if time == 0.0 {
            ctx.memory_mut(|m| m.request_focus(id));
        }
    }
    (ctx, id)
}

#[test]
fn a_control_that_senses_clicks_can_take_the_focus() {
    let (ctx, id) = focused_button();
    assert!(ctx.memory(|m| m.has_focus(id)), "a click-sensing area kept no focus");
}

#[test]
fn a_focused_button_leaves_the_players_keys_to_opium() {
    let (ctx, id) = focused_button();
    assert!(ctx.memory(|m| m.has_focus(id)), "premise: the button holds the focus");
    assert!(
        !super::keys_taken(&ctx),
        "with a button focused, keys_taken() is true: Space, Escape, the player's keys and the table's arrows do nothing"
    );
}
