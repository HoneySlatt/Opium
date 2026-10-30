//! Now Playing's Lyrics tab: the lines in large type, the current one lit and
//! kept a third of the way down, a click on a line jumps to it.

use std::sync::Arc;
use std::time::Duration;

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Id, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{ItemId, LyricLine, Lyrics};

use crate::app::{App, Loaded};
use crate::fonts::{Weight, font};
use crate::theme::Theme;

use super::{paint_text, text};

const PAD: egui::Vec2 = vec2(12.0, 9.0);
/// The current line's scale.
const LIT: f32 = 1.025;
/// The fades over the top and bottom of the list, as fractions of its height.
const FADE_TOP: f32 = 0.16;
const FADE_BOTTOM: f32 = 0.26;

/// Where the list is scrolled, and to where it is heading.
#[derive(Clone, Default)]
struct Scroll {
    track: Option<ItemId>,
    current: Option<usize>,
    offset: f32,
    target: f32,
}

/// The lyrics once they have loaded, for the column to widen around them.
pub fn ready(app: &mut App) -> Option<Arc<Lyrics>> {
    let track = app.shown().track?;
    match app.lyrics(&track) {
        Loaded::Ready(lyrics) => lyrics.clone(),
        _ => None,
    }
}

/// The lyrics in `body` at `size` px. `reach` is the height the blank below the
/// last line scales with; `salt` tells two lists apart.
pub fn show(
    app: &mut App,
    ui: &mut Ui,
    theme: &Theme,
    body: Rect,
    size: f32,
    reach: f32,
    salt: &str,
) {
    let Some(track) = app.shown().track else {
        return;
    };
    let lyrics = match app.lyrics(&track) {
        Loaded::Loading => return,
        Loaded::Ready(Some(lyrics)) => lyrics.clone(),
        Loaded::Ready(None) => {
            return empty(
                ui,
                theme,
                body,
                "No lyrics for this track",
                "Neither its service nor the lyrics sites have them.",
            );
        }
        Loaded::Failed(e) => {
            let e = e.clone();
            return empty(ui, theme, body, "Couldn't load the lyrics", &e);
        }
    };

    let list = body;

    // Lay the lines out, each with its padding, the gaps as a row of dots.
    // A wide list keeps room on the right for the time of the line hovered.
    let gutter = if list.width() > 600.0 { 72.0 } else { 0.0 };
    let wrap = list.width() - 2.0 * PAD.x - gutter;
    let galleys: Vec<_> = lyrics
        .lines
        .iter()
        .map(|l| (!l.text.is_empty()).then(|| layout(ui, &l.text, size, wrap, 1.0, theme)))
        .collect();
    let heights: Vec<f32> = galleys
        .iter()
        .map(|g| g.as_ref().map_or(9.0 + 16.0, |g| g.size().y) + 2.0 * PAD.y)
        .collect();
    // The first line starts at the top; the blank below lets the last one
    // climb to a third of the way down.
    let pad_bottom = reach * 0.44;
    let mut tops = Vec::with_capacity(heights.len());
    let mut y = 0.0;
    for h in &heights {
        tops.push(y);
        y += h;
    }
    let max_offset = (y + pad_bottom - list.height()).max(0.0);

    let position = app.shown().position;
    let current = lyrics.current(position);
    let scroll_id = Id::unique((salt, "scroll"));
    let mut scroll: Scroll = ui.ctx().data(|d| d.get_temp(scroll_id)).unwrap_or_default();
    let new_track = scroll.track.as_ref() != Some(&track.id);
    if new_track || scroll.current != current {
        scroll.target = current.map_or(0.0, |i| tops[i] - list.height() * 0.34);
        if new_track {
            scroll.offset = scroll.target.clamp(0.0, max_offset);
        }
        scroll.track = Some(track.id.clone());
        scroll.current = current;
    }
    let list_response = ui.interact(list, Id::unique((salt, "list")), Sense::hover());
    if list_response.hovered() {
        scroll.target -= ui.input(|i| i.smooth_scroll_delta.y);
    }
    scroll.target = scroll.target.clamp(0.0, max_offset);
    let dt = ui.input(|i| i.stable_dt).min(0.1);
    let step = scroll.target - scroll.offset;
    scroll.offset += step * (1.0 - (-dt * 9.0).exp());
    if step.abs() < 0.5 {
        scroll.offset = scroll.target;
    } else {
        ui.ctx().request_repaint();
    }
    let offset = scroll.offset;
    ui.ctx().data_mut(|d| d.insert_temp(scroll_id, scroll));

    let painter = ui.painter_at(list);
    let time = ui.input(|i| i.time);
    for (i, line) in lyrics.lines.iter().enumerate() {
        let rect = Rect::from_min_size(
            pos2(list.left(), list.top() + tops[i] - offset),
            vec2(list.width(), heights[i]),
        );
        if rect.bottom() < list.top() || rect.top() > list.bottom() {
            continue;
        }
        let on = Some(i) == current;
        let lit = ui
            .ctx()
            .animate_bool_with_time(Id::unique((salt, "lit", i)), on, 0.35);
        // Unsynced lyrics have no current line: all of them read in full.
        let mut opacity = if lyrics.synced { 0.3 + 0.7 * lit } else { 1.0 };
        if lyrics.synced {
            let response = ui.interact(rect, Id::unique((salt, "hit", i)), Sense::click());
            if response.hovered() {
                painter.rect_filled(rect, 10.0, theme.hover());
                opacity = opacity.max(0.62);
                if gutter > 0.0 {
                    time_chip(ui, &painter, theme, rect, line.start);
                }
            }
            if response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                app.play_from(line.start);
            }
        }
        let fade = edge_fade(rect.center().y, list, offset);
        opacity *= fade;
        let color = theme.text_a(opacity);
        // On half-point sizes: each size is drawn into the font atlas, and
        // a zoom through every fraction of one would fill it.
        let grown = (size * (LIT - 1.0) * lit * 2.0).round() / 2.0;
        let scale = 1.0 + grown / size;
        match &galleys[i] {
            // The line being sung word by word: what is sung lights up as
            // it goes.
            Some(_) if on && !line.words.is_empty() => {
                let galley = sung(ui, line, position, size, wrap, scale, theme, fade);
                let top = rect.center().y - galley.size().y / 2.0;
                painter.galley(pos2(rect.left() + PAD.x, top), galley, color);
                // 60 frames a second light it smoothly: no need for the
                // screen's every refresh.
                if app.shown().playing {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(16));
                }
            }
            Some(galley) => {
                let galley = if scale > 1.0 {
                    layout(ui, &line.text, size, wrap, scale, theme)
                } else {
                    galley.clone()
                };
                let top = rect.center().y - galley.size().y / 2.0;
                painter.galley_with_override_text_color(
                    pos2(rect.left() + PAD.x, top),
                    galley,
                    color,
                );
            }
            None => {
                dots(&painter, rect, on, time, opacity, theme);
                if on && app.shown().playing {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(33));
                }
            }
        }
    }
}

/// Where a click on the line hovered goes, at the right of the list.
fn time_chip(ui: &Ui, painter: &egui::Painter, theme: &Theme, line: Rect, at: std::time::Duration) {
    let g = text(
        ui,
        &super::format_time(at),
        font(12.5, Weight::SemiBold),
        theme.text_a(0.6),
        60.0,
    );
    let chip = Rect::from_center_size(
        pos2(line.right() - 36.0, line.center().y),
        g.size() + vec2(18.0, 10.0),
    );
    painter.rect(
        chip,
        chip.height() / 2.0,
        crate::color::alpha(theme.crust, 0.5),
        egui::Stroke::new(1.0, theme.text_a(0.08)),
        egui::StrokeKind::Inside,
    );
    let size = g.size();
    painter.galley(chip.center() - size / 2.0, g, theme.text_a(0.6));
}

/// How bright what is still to be sung in the line playing is, to what is
/// sung: as Apple Music shows it.
const UNSUNG: f32 = 0.35;
/// How many letters the edge of the light spreads over, as it moves along
/// a word: a soft wipe rather than letter by letter.
const EDGE: f32 = 1.8;

/// The line playing, word by word: each letter lit as its word goes by, the
/// word being sung filling from its first letter to its last.
#[allow(clippy::too_many_arguments)]
fn sung(
    ui: &Ui,
    line: &LyricLine,
    position: Duration,
    size: f32,
    wrap: f32,
    scale: f32,
    theme: &Theme,
    fade: f32,
) -> Arc<egui::Galley> {
    let size = size * scale;
    let format = |opacity: f32| TextFormat {
        font_id: font(size, Weight::Bold),
        color: theme.text_a(opacity * fade),
        extra_letter_spacing: -0.015 * size,
        line_height: Some(size * 1.22),
        ..TextFormat::default()
    };
    let mut job = LayoutJob::default();
    let count = line.words.len();
    for (k, word) in line.words.iter().enumerate() {
        let span = word.end.saturating_sub(word.start).as_secs_f32().max(0.001);
        let progress = (position.saturating_sub(word.start).as_secs_f32() / span).clamp(0.0, 1.0);
        let text = if k + 1 == count {
            word.text.trim_end()
        } else {
            word.text.as_str()
        };
        let letters = text.chars().filter(|c| !c.is_whitespace()).count().max(1);
        // Where the light's edge is, in letters: past the last one by the
        // edge's width once the word is sung, so that it ends wholly lit.
        let edge = progress * (letters as f32 - 1.0 + EDGE);
        let mut n = 0usize;
        let mut last = 0.0;
        for c in text.chars() {
            let lit = if c.is_whitespace() {
                last
            } else {
                let t = ((edge - n as f32) / EDGE).clamp(0.0, 1.0);
                n += 1;
                // Eased, as the light comes on and settles.
                t * t * (3.0 - 2.0 * t)
            };
            last = lit;
            job.append(&c.to_string(), 0.0, format(UNSUNG + (1.0 - UNSUNG) * lit));
        }
    }
    job.wrap = TextWrapping::wrap_at_width(wrap * scale);
    ui.painter().layout_job(job)
}

fn layout(ui: &Ui, s: &str, size: f32, wrap: f32, scale: f32, theme: &Theme) -> Arc<egui::Galley> {
    let size = size * scale;
    let mut job = LayoutJob::single_section(
        s.to_owned(),
        TextFormat {
            font_id: font(size, Weight::Bold),
            color: theme.text,
            extra_letter_spacing: -0.015 * size,
            line_height: Some(size * 1.22),
            ..TextFormat::default()
        },
    );
    job.wrap = TextWrapping::wrap_at_width(wrap * scale);
    ui.painter().layout_job(job)
}

/// The list's top 16 % and bottom 26 % fade out.
fn edge_fade(y: f32, list: Rect, offset: f32) -> f32 {
    let f = ((y - list.top()) / list.height()).clamp(0.0, 1.0);
    // Nothing above to fade until the list has scrolled.
    let above = (offset / (list.height() * FADE_TOP)).clamp(0.0, 1.0);
    let top = 1.0 - above * (1.0 - (f / FADE_TOP).min(1.0));
    let bottom = ((1.0 - f) / FADE_BOTTOM).min(1.0);
    top.min(bottom)
}

/// An instrumental gap: three dots, breathing while it lasts.
fn dots(painter: &egui::Painter, rect: Rect, on: bool, time: f64, opacity: f32, theme: &Theme) {
    let y = rect.center().y;
    for k in 0..3 {
        let x = rect.left() + PAD.x + 4.5 + k as f32 * 16.0;
        // 1.6 s a breath, each dot a quarter of a second behind the last.
        let breath = if on {
            let phase = ((time - 0.25 * k as f64) / 1.6).rem_euclid(1.0) as f32;
            (1.0 - (phase * std::f32::consts::TAU).cos()) / 2.0
        } else {
            0.0
        };
        let color = theme.text_a(opacity * (0.5 + 0.5 * breath));
        painter.circle_filled(pos2(x, y), 4.5 * (1.0 + 0.25 * breath), color);
    }
}

fn empty(ui: &Ui, theme: &Theme, body: Rect, title: &str, detail: &str) {
    let center = body.center();
    let g = text(
        ui,
        title,
        font(18.0, Weight::SemiBold),
        theme.text,
        body.width(),
    );
    paint_text(
        ui,
        g,
        center.x,
        center.y - 12.0,
        egui::Align::Center,
        theme.text,
    );
    let g = text(
        ui,
        detail,
        font(13.0, Weight::Regular),
        theme.subtext0,
        body.width() - 40.0,
    );
    paint_text(
        ui,
        g,
        center.x,
        center.y + 14.0,
        egui::Align::Center,
        theme.subtext0,
    );
}
