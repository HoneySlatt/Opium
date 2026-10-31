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
const LIT: f32 = 1.02;
/// How long a line takes to light, and to dim once sung, as on the phone.
const LIGHTING: f32 = 0.36;
const DIMMING: f32 = 0.52;
/// The list's glide to the next line, in seconds, and how much later each
/// line below it follows, for so many lines: as on the phone.
const GLIDE: f32 = 0.62;
const STAGGER: f32 = 0.045;
const STAGGERED: usize = 6;
/// The fades over the top and bottom of the list, as fractions of its height.
const FADE_TOP: f32 = 0.16;
const FADE_BOTTOM: f32 = 0.26;

/// Where the list is scrolled, or the glide taking it there.
#[derive(Clone, Default)]
struct Scroll {
    track: Option<ItemId>,
    current: Option<usize>,
    offset: f32,
    glide: Option<Glide>,
}

/// The list on its way to line `to`: it glides there, and the lines below
/// it follow one after the other, each a beat later.
#[derive(Clone, Copy)]
struct Glide {
    to: usize,
    from: f32,
    shift: f32,
    start: f64,
}

impl Glide {
    /// The offset `delay` seconds behind the list's own, at `now`.
    fn at(&self, now: f64, delay: f32) -> f32 {
        let t = ((now - self.start) as f32 - delay) / GLIDE;
        self.from + self.shift * bezier(0.3, 0.0, 0.1, 1.0, t)
    }

    /// Line `i`'s offset: those below the line glided to trail behind.
    fn line(&self, i: usize, now: f64) -> f32 {
        let rank = i.saturating_sub(self.to).min(STAGGERED);
        self.at(now, rank as f32 * STAGGER)
    }

    fn done(&self, now: f64) -> bool {
        (now - self.start) as f32 >= GLIDE + STAGGERED as f32 * STAGGER
    }
}

/// A CSS cubic Bézier easing, from `(x1, y1)` and `(x2, y2)`, at `t`.
fn bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let curve = |a: f32, b: f32, u: f32| {
        let v = 1.0 - u;
        3.0 * v * v * u * a + 3.0 * v * u * u * b + u * u * u
    };
    // The curve's parameter where it reaches `t` across: by halves.
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..20 {
        let mid = (lo + hi) / 2.0;
        if curve(x1, x2, mid) < t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    curve(y1, y2, (lo + hi) / 2.0)
}

/// Compose's standard easing, which the phone's lines light and dim with.
fn standard(t: f32) -> f32 {
    bezier(0.4, 0.0, 0.2, 1.0, t)
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
        .map(|l| (!l.text.is_empty()).then(|| layout(ui, &l.text, size, wrap, theme)))
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
    let now = ui.input(|i| i.time);
    let scroll_id = Id::unique((salt, "scroll"));
    let mut scroll: Scroll = ui.ctx().data(|d| d.get_temp(scroll_id)).unwrap_or_default();
    // Where the list is now, gliding or not.
    let base = |scroll: &Scroll| scroll.glide.map_or(scroll.offset, |g| g.at(now, 0.0));
    let target = current
        .map_or(0.0, |i| tops[i] - list.height() * 0.34)
        .clamp(0.0, max_offset);
    if scroll.track.as_ref() != Some(&track.id) {
        scroll.offset = target;
        scroll.glide = None;
    } else if scroll.current != current {
        // A glide to the line if it is in sight, a cut if not.
        let from = base(&scroll);
        scroll.glide = current
            .filter(|&i| tops[i] + heights[i] > from && tops[i] < from + list.height())
            .filter(|_| (target - from).abs() > 0.5)
            .map(|to| Glide {
                to,
                from,
                shift: target - from,
                start: now,
            });
        scroll.offset = target;
    }
    scroll.track = Some(track.id.clone());
    scroll.current = current;
    let list_response = ui.interact(list, Id::unique((salt, "list")), Sense::hover());
    let wheel = ui.input(|i| i.smooth_scroll_delta.y);
    if list_response.hovered() && wheel != 0.0 {
        scroll.offset = (base(&scroll) - wheel).clamp(0.0, max_offset);
        scroll.glide = None;
    }
    if scroll.glide.is_some_and(|g| g.done(now)) {
        scroll.glide = None;
    }
    if scroll.glide.is_some() {
        ui.ctx().request_repaint();
    }
    let offset = base(&scroll);
    let glide = scroll.glide;
    ui.ctx().data_mut(|d| d.insert_temp(scroll_id, scroll));

    let painter = ui.painter_at(list);
    let time = ui.input(|i| i.time);
    for (i, line) in lyrics.lines.iter().enumerate() {
        let line_offset = glide.map_or(offset, |g| g.line(i, now));
        let rect = Rect::from_min_size(
            pos2(list.left(), list.top() + tops[i] - line_offset),
            vec2(list.width(), heights[i]),
        );
        if rect.bottom() < list.top() || rect.top() > list.bottom() {
            continue;
        }
        let on = Some(i) == current;
        // A line that was sung dims slowly; the next one grows as it lights.
        let lit = ui.ctx().animate_bool_with_time_and_easing(
            Id::unique((salt, "lit", i)),
            on,
            if on { LIGHTING } else { DIMMING },
            standard,
        );
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
        // Grown from its left edge: while it grows, the text as laid out
        // drawn larger; once grown, laid out at its size, to stay sharp.
        let grown = lit >= 0.999;
        let scale = if grown { 1.0 } else { 1.0 + (LIT - 1.0) * lit };
        let grow = |shape: &mut egui::Shape| {
            let pivot = pos2(rect.left() + PAD.x, rect.center().y);
            shape.transform(egui::emath::TSTransform::new(
                pivot.to_vec2() * (1.0 - scale),
                scale,
            ));
        };
        match &galleys[i] {
            // The line being sung: what is sung lights up as it goes, word
            // by word as timed, else each word in its share of the line.
            Some(_) if on => {
                let next = lyrics.lines.get(i + 1);
                let (size, wrap) = if grown {
                    (size * LIT, wrap * LIT)
                } else {
                    (size, wrap)
                };
                let galley = sung(ui, &words(line, next, position), size, wrap, theme, fade);
                let top = rect.center().y - galley.size().y / 2.0;
                let mut shape = egui::Shape::galley(pos2(rect.left() + PAD.x, top), galley, color);
                grow(&mut shape);
                painter.add(shape);
                // 60 frames a second light it smoothly: no need for the
                // screen's every refresh.
                if app.shown().playing {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(16));
                }
            }
            Some(galley) => {
                let top = rect.center().y - galley.size().y / 2.0;
                let mut shape = egui::Shape::galley_with_override_text_color(
                    pos2(rect.left() + PAD.x, top),
                    galley.clone(),
                    color,
                );
                grow(&mut shape);
                painter.add(shape);
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

/// The words of the line playing, each with how far it has been sung: as
/// timed word by word, else each its share of the line by its length, as
/// on the phone. Their text keeps the space after them.
fn words(line: &LyricLine, next: Option<&LyricLine>, position: Duration) -> Vec<(String, f32)> {
    if !line.words.is_empty() {
        return line
            .words
            .iter()
            .map(|w| {
                let span = w.end.saturating_sub(w.start).as_secs_f32().max(0.001);
                let sung = position.saturating_sub(w.start).as_secs_f32() / span;
                (w.text.clone(), sung.clamp(0.0, 1.0))
            })
            .collect();
    }
    let end = line
        .end
        .or(next.map(|n| n.start))
        .unwrap_or(line.start + Duration::from_secs(4));
    // The line is sung a little ahead of its end.
    let span = (end.saturating_sub(line.start).as_secs_f32() * 0.82).max(0.5);
    let sung = (position.saturating_sub(line.start).as_secs_f32() / span).clamp(0.0, 1.0);
    let parts: Vec<&str> = line.text.split(' ').filter(|w| !w.is_empty()).collect();
    let total: f32 = parts.iter().map(|w| w.chars().count() as f32 + 1.0).sum();
    let mut done = 0.0;
    parts
        .iter()
        .enumerate()
        .map(|(k, w)| {
            let from = done / total;
            done += w.chars().count() as f32 + 1.0;
            let to = done / total;
            let text = if k + 1 < parts.len() {
                format!("{w} ")
            } else {
                (*w).to_owned()
            };
            (text, ((sung - from) / (to - from)).clamp(0.0, 1.0))
        })
        .collect()
}

/// The line playing, word by word: each letter lit as its word goes by, the
/// word being sung filling from its first letter to its last.
fn sung(
    ui: &Ui,
    words: &[(String, f32)],
    size: f32,
    wrap: f32,
    theme: &Theme,
    fade: f32,
) -> Arc<egui::Galley> {
    let format = |opacity: f32| TextFormat {
        font_id: font(size, Weight::Bold),
        color: theme.text_a(opacity * fade),
        extra_letter_spacing: -0.015 * size,
        line_height: Some(size * 1.22),
        ..TextFormat::default()
    };
    let mut job = LayoutJob::default();
    let count = words.len();
    for (k, (word, progress)) in words.iter().enumerate() {
        let text = if k + 1 == count {
            word.trim_end()
        } else {
            word.as_str()
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
    job.wrap = TextWrapping::wrap_at_width(wrap);
    ui.painter().layout_job(job)
}

fn layout(ui: &Ui, s: &str, size: f32, wrap: f32, theme: &Theme) -> Arc<egui::Galley> {
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
    job.wrap = TextWrapping::wrap_at_width(wrap);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_glide_eases_from_start_to_end() {
        assert!(bezier(0.3, 0.0, 0.1, 1.0, 0.0).abs() < 1e-3);
        assert!((bezier(0.3, 0.0, 0.1, 1.0, 1.0) - 1.0).abs() < 1e-3);
        // Quick to leave, slow to settle.
        assert!(bezier(0.3, 0.0, 0.1, 1.0, 0.5) > 0.8);
        let steps: Vec<f32> = (0..=10).map(|k| standard(k as f32 / 10.0)).collect();
        assert!(steps.windows(2).all(|w| w[1] >= w[0]));
    }

    #[test]
    fn an_untimed_line_is_shared_among_its_words_by_length() {
        let line = LyricLine {
            start: Duration::from_secs(10),
            end: Some(Duration::from_secs(20)),
            text: "a longer line".into(),
            words: Vec::new(),
        };
        let at = |s: f32| words(&line, None, Duration::from_secs_f32(s));
        assert!(at(10.0).iter().all(|(_, p)| *p == 0.0));
        let half = at(13.0);
        assert_eq!(half[0], ("a ".to_owned(), 1.0));
        assert!(half[1].1 > 0.0 && half[1].1 < 1.0);
        assert_eq!(half[2].1, 0.0);
        // Sung by 82 % of its span.
        assert!(at(18.3).iter().all(|(_, p)| *p == 1.0));
        assert_eq!(at(18.3).last().unwrap().0, "line");
    }
}
