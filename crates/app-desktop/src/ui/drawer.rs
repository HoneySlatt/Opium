//! The drawer: 360 px on the right between the top bar and the player bar,
//! with Playing Next, History and Lyrics tabs.

use egui::{Color32, Id, Order, Rect, Sense, Stroke, Ui, pos2, vec2};
use music_core::Repeat;
use music_core::model::Track;

use crate::app::{App, DrawerTab};
use crate::color::{alpha, mix};
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::{artwork, format_time, icon_button, paint_icon, paint_text, text, thumb_url};

const WIDTH: f32 = 360.0;

/// What a click in the list asked for.
enum Action {
    None,
    Jump(usize),
    Remove(usize),
    Clear,
    Shuffle,
    Repeat,
}

pub fn show(app: &mut App, ui: &mut Ui, main: Rect) {
    let open = ui
        .ctx()
        .animate_bool_with_time(Id::unique("drawer-open"), app.drawer_open, 0.24);
    if open == 0.0 {
        return;
    }
    // Ease out: it slides in fast and settles.
    let eased = 1.0 - (1.0 - open).powi(3);
    let theme = app.theme.clone();
    let rect = Rect::from_min_max(
        pos2(main.right() - 12.0 - WIDTH, main.top() + 52.0),
        pos2(main.right() - 12.0, main.bottom() - 70.0),
    )
    .translate(vec2((1.0 - eased) * (WIDTH + 24.0), 0.0));

    // An Area, not just a layer: egui then knows the pointer is over the
    // drawer, so the wheel scrolls its lists and not the page under it.
    egui::Area::new(Id::unique("drawer"))
        .order(Order::Foreground)
        .fixed_pos(rect.min)
        .constrain(false)
        .show(ui.ctx(), |ui| {
            ui.set_min_size(rect.size());
            ui.set_max_size(rect.size());
            contents(app, ui, &theme, rect, open);
        });
}

fn contents(app: &mut App, ui: &mut Ui, theme: &Theme, rect: Rect, open: f32) {
    let theme = theme.clone();
    ui.painter().add(
        egui::Shadow {
            offset: [0, 18],
            blur: 48,
            spread: 0,
            color: Color32::from_black_alpha(110),
        }
        .as_shape(rect.shrink(8.0), 16.0),
    );
    ui.painter().rect(
        rect,
        16.0,
        theme.mantle,
        Stroke::new(1.0, theme.line()),
        egui::StrokeKind::Inside,
    );
    // Clicks on the drawer must not reach the page underneath.
    ui.interact(rect, Id::unique("drawer-bg"), Sense::click());

    // Header: the tabs and a close button.
    let tabs_at = pos2(rect.left() + 16.0, rect.top() + 12.0);
    let labels = [
        (DrawerTab::PlayingNext, "Playing Next"),
        (DrawerTab::History, "History"),
        (DrawerTab::Lyrics, "Lyrics"),
    ];
    if let Some(tab) = tabs(ui, &theme, tabs_at, &labels, app.drawer_tab) {
        app.drawer_tab = tab;
    }
    let close = Rect::from_min_size(
        pos2(rect.right() - 12.0 - 32.0, rect.top() + 12.0),
        vec2(32.0, 32.0),
    );
    let mut close_ui = ui.new_child(egui::UiBuilder::new().max_rect(close));
    if icon_button(
        &mut close_ui,
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
        app.drawer_open = false;
    }

    // Lyrics lines carry their own padding: their text lines up with the tabs.
    let inset = if app.drawer_tab == DrawerTab::Lyrics {
        4.0
    } else {
        8.0
    };
    let body = Rect::from_min_max(
        pos2(rect.left() + inset, rect.top() + 12.0 + 32.0 + 8.0),
        pos2(rect.right() - 8.0, rect.bottom() - 12.0),
    );
    let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
    body_ui.set_clip_rect(body.intersect(ui.clip_rect()));
    match app.drawer_tab {
        DrawerTab::PlayingNext => queue(app, &mut body_ui, &theme, "drawer-body"),
        DrawerTab::History => {
            egui::ScrollArea::vertical()
                .id_salt("drawer-history")
                .auto_shrink(false)
                .show(&mut body_ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    history(app, ui, &theme);
                });
        }
        DrawerTab::Lyrics => {
            let reach = body.height();
            super::lyrics::show(
                app,
                &mut body_ui,
                &theme,
                body,
                21.0,
                reach,
                "drawer-lyrics",
            );
        }
    }
    if open < 1.0 {
        ui.ctx().request_repaint();
    }
}

/// The current track, what plays next and the gapless note, scrolling in `ui`,
/// with its buttons acting on the queue. Shared with Now Playing.
pub(super) fn queue(app: &mut App, ui: &mut Ui, theme: &Theme, salt: &str) {
    let action = egui::ScrollArea::vertical()
        .id_salt(salt)
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let action = playing_next(app, ui, theme);
            // M3E's queue goes on with what has played.
            if theme.expressive() && !app.history.is_empty() {
                label(ui, theme, "HISTORY", &[]);
                history(app, ui, theme);
                ui.add_space(110.0);
            }
            action
        })
        .inner;
    match action {
        Action::None => {}
        Action::Jump(k) => app.jump_to(k),
        Action::Remove(k) => app.remove_upcoming(k),
        Action::Clear => app.clear_upcoming(),
        Action::Shuffle => app.toggle_shuffle(),
        Action::Repeat => app.cycle_repeat(),
    }
}

/// A segmented control. Returns the tab clicked.
pub(super) fn tabs<T: Copy + PartialEq>(
    ui: &mut Ui,
    theme: &Theme,
    at: egui::Pos2,
    labels: &[(T, &str)],
    current: T,
) -> Option<T> {
    let f = font(13.0, Weight::Medium);
    let widths: Vec<f32> = labels
        .iter()
        .map(|(_, l)| text(ui, l, f.clone(), theme.text, 200.0).size().x + 24.0)
        .collect();
    let outer = Rect::from_min_size(at, vec2(widths.iter().sum::<f32>() + 6.0, 32.0));
    ui.painter().rect_filled(outer, 9.0, theme.text_a(0.07));
    let mut x = outer.left() + 3.0;
    let mut clicked = None;
    for (&(tab, label), w) in labels.iter().zip(widths) {
        let rect = Rect::from_min_size(pos2(x, outer.top() + 3.0), vec2(w, 26.0));
        x += w;
        let response = ui.interact(rect, Id::unique(("tab", label)), Sense::click());
        let on = tab == current;
        if on {
            ui.painter().rect_filled(rect, 7.0, theme.surface2);
        }
        let color = if on || response.hovered() {
            theme.text
        } else {
            theme.subtext0
        };
        let g = text(ui, label, f.clone(), color, w);
        paint_text(
            ui,
            g,
            rect.center().x,
            rect.center().y,
            egui::Align::Center,
            color,
        );
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            clicked = Some(tab);
        }
    }
    clicked
}

/// A small caps label above a group of rows, with optional buttons on the right.
fn label(
    ui: &mut Ui,
    theme: &Theme,
    caption: &str,
    buttons: &[(Icon, bool, &str)],
) -> Option<usize> {
    // 12 px above, a 28 px line for the buttons, nothing below.
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::hover());
    let y = rect.top() + 12.0 + 14.0;
    let g = text(
        ui,
        caption,
        font(11.5, Weight::SemiBold),
        theme.muted(),
        rect.width() - 100.0,
    );
    paint_text(ui, g, rect.left() + 8.0, y, egui::Align::Min, theme.muted());
    let mut clicked = None;
    for (k, (icon, on, tip)) in buttons.iter().enumerate() {
        let n = buttons.len() - k;
        let at = Rect::from_center_size(
            pos2(rect.right() - 8.0 - 14.0 - (n - 1) as f32 * 30.0, y),
            vec2(28.0, 28.0),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(at));
        let color = if *on { theme.accent } else { theme.subtext0 };
        if icon_button(&mut child, theme, *icon, 28.0, 15.0, color, true)
            .on_hover_text(*tip)
            .clicked()
        {
            clicked = Some(k);
        }
    }
    clicked
}

fn playing_next(app: &App, ui: &mut Ui, theme: &Theme) -> Action {
    let mut action = Action::None;
    let shown_now = app.shown();
    if let Some(now) = &shown_now.track {
        label(ui, theme, "NOW PLAYING", &[]);
        row(ui, theme, now, RowKind::Current, "now");
    }

    let upcoming_len = app.upcoming_len_shown();
    let repeat_icon = if shown_now.repeat == Repeat::One {
        icons::REPEAT_ONE
    } else {
        icons::REPEAT
    };
    let caption = format!("PLAYING NEXT · {upcoming_len}");
    match label(
        ui,
        theme,
        &caption,
        &[
            (icons::SHUFFLE, shown_now.shuffle, "Shuffle"),
            (repeat_icon, shown_now.repeat != Repeat::Off, "Repeat"),
            (icons::TRASH, false, "Clear queue"),
        ],
    ) {
        Some(0) => action = Action::Shuffle,
        Some(1) => action = Action::Repeat,
        Some(2) => action = Action::Clear,
        _ => {}
    }
    if upcoming_len == 0 {
        note(ui, theme, "The queue ends here.");
    }
    // Only the rows in view are drawn, and their covers loaded; the others
    // just take their room.
    let visible = ui.clip_rect().y_range();
    let first =
        (((visible.min - ui.cursor().top()) / ROW).floor().max(0.0) as usize).min(upcoming_len);
    let shown = ((visible.span() / ROW).ceil() as usize + 1).min(upcoming_len - first);
    ui.allocate_space(vec2(ui.available_width(), first as f32 * ROW));
    for (k, track) in app.upcoming_shown().enumerate().skip(first).take(shown) {
        match row(ui, theme, track, RowKind::Upcoming, &format!("next-{k}")) {
            RowAction::Open => action = Action::Jump(k),
            RowAction::Remove => action = Action::Remove(k),
            RowAction::None => {}
        }
    }
    let after = upcoming_len - first - shown;
    ui.allocate_space(vec2(ui.available_width(), after as f32 * ROW));

    // Whether the next track will join without a gap.
    if let (Some(next), Some(source), None) = (
        app.queue.upcoming().next(),
        &app.status.source,
        &shown_now.on,
    ) {
        let next_rate = next.quality.as_ref().and_then(|q| q.sample_rate);
        if let Some(next_rate) = next_rate {
            let (icon, message) = if next_rate == source.sample_rate {
                (icons::CHECK, format!("Gapless into “{}”", next.title))
            } else {
                (
                    icons::PW,
                    format!(
                        "Next track changes rate ({} → {} kHz): a short gap while the card switches",
                        khz(source.sample_rate),
                        khz(next_rate)
                    ),
                )
            };
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
            let y = rect.top() + 8.0 + 8.0;
            paint_icon(
                ui,
                icon,
                pos2(rect.left() + 8.0 + 6.5, y),
                13.0,
                theme.muted(),
            );
            let g = text(
                ui,
                &message,
                font(11.5, Weight::Regular),
                theme.muted(),
                rect.width() - 36.0,
            );
            paint_text(
                ui,
                g,
                rect.left() + 8.0 + 13.0 + 6.0,
                y,
                egui::Align::Min,
                theme.muted(),
            );
        }
    }
    action
}

pub(super) fn history(app: &App, ui: &mut Ui, theme: &Theme) {
    if app.history.is_empty() {
        note(ui, theme, "Tracks you finish or skip show up here.");
        return;
    }
    ui.add_space(4.0);
    for (k, track) in app.history.iter().enumerate() {
        row(ui, theme, track, RowKind::Past, &format!("hist-{k}"));
    }
}

fn note(ui: &mut Ui, theme: &Theme, message: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::hover());
    let g = text(
        ui,
        message,
        font(13.0, Weight::Regular),
        theme.subtext0,
        rect.width() - 16.0,
    );
    paint_text(
        ui,
        g,
        rect.left() + 8.0,
        rect.center().y,
        egui::Align::Min,
        theme.subtext0,
    );
}

#[derive(Clone, Copy, PartialEq)]
enum RowKind {
    Current,
    Upcoming,
    Past,
}

enum RowAction {
    None,
    Open,
    Remove,
}

/// A 52 px row: 40 px artwork, title, source and artist; a remove button on
/// hover for upcoming tracks, the length for the current one.
/// A row's height.
const ROW: f32 = 52.0;

fn row(ui: &mut Ui, theme: &Theme, track: &Track, kind: RowKind, salt: &str) -> RowAction {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::click());
    let hovered = response.contains_pointer();
    let fade = if kind == RowKind::Past { 0.6 } else { 1.0 };
    // M3E's: the song playing a card in the secondary container.
    let expressive = theme.expressive();
    let fill = match kind {
        RowKind::Current if expressive => Some(theme.tonal.secondary_container),
        RowKind::Current => Some(alpha(theme.accent, 0.14)),
        _ if hovered && kind == RowKind::Upcoming => Some(theme.hover()),
        _ => None,
    };
    let (radius, art_radius) = match kind {
        _ if !expressive => (9.0, 5.0),
        RowKind::Current => (20.0, 12.0),
        _ => (16.0, 10.0),
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, radius, fill);
    }
    let art = Rect::from_min_size(
        pos2(rect.left() + 8.0, rect.center().y - 20.0),
        vec2(40.0, 40.0),
    );
    artwork(ui, thumb_url(track), art, art_radius, theme);
    if fade < 1.0 {
        ui.painter()
            .rect_filled(art, art_radius, alpha(theme.mantle, 1.0 - fade));
    }

    let left = art.right() + 10.0;
    let right_reserve = 44.0;
    let width = rect.right() - 8.0 - right_reserve - left;
    let title_color = match kind {
        RowKind::Current if expressive => theme.tonal.on_secondary_container,
        RowKind::Current => mix(theme.accent, theme.text, 0.8),
        _ => alpha(theme.text, fade),
    };
    let g = text(
        ui,
        &track.title,
        font(13.0, Weight::Medium),
        title_color,
        width,
    );
    paint_text(
        ui,
        g,
        left,
        rect.center().y - 8.0,
        egui::Align::Min,
        title_color,
    );
    let source = icons::service(track.id.provider);
    let muted = alpha(theme.muted(), fade);
    paint_icon(
        ui,
        source,
        pos2(left + 5.5, rect.center().y + 9.0),
        11.0,
        muted,
    );
    let g = text(
        ui,
        &track.artists.join(", "),
        font(12.0, Weight::Regular),
        muted,
        width - 16.0,
    );
    paint_text(
        ui,
        g,
        left + 11.0 + 5.0,
        rect.center().y + 9.0,
        egui::Align::Min,
        muted,
    );

    let mut action = RowAction::None;
    match kind {
        RowKind::Upcoming if hovered => {
            let x = Rect::from_center_size(
                pos2(rect.right() - 8.0 - 16.0, rect.center().y),
                vec2(32.0, 32.0),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(x).id_salt(salt));
            if icon_button(
                &mut child,
                theme,
                icons::X,
                32.0,
                14.0,
                theme.subtext0,
                true,
            )
            .on_hover_text("Remove from queue")
            .clicked()
            {
                action = RowAction::Remove;
            }
        }
        RowKind::Current => {
            if let Some(duration) = track.duration {
                let g = text(
                    ui,
                    &format_time(duration),
                    font(12.0, Weight::Regular),
                    theme.subtext0,
                    52.0,
                );
                paint_text(
                    ui,
                    g,
                    rect.right() - 8.0,
                    rect.center().y,
                    egui::Align::Max,
                    theme.subtext0,
                );
            }
        }
        _ => {}
    }
    if kind == RowKind::Upcoming
        && matches!(action, RowAction::None)
        && response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
    {
        action = RowAction::Open;
    }
    action
}

/// `44.1`, `96`.
fn khz(rate: u32) -> String {
    let khz = f64::from(rate) / 1000.0;
    if khz.fract() == 0.0 {
        format!("{khz:.0}")
    } else {
        format!("{khz:.1}")
    }
}
