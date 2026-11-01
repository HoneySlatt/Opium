//! Add to Playlists, as Cider asks it: the song at the top, a search, the
//! playlists used last, then every playlist of the user's with a box to tick,
//! each saying whether it has the song already.

use egui::{Color32, Frame, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use music_core::model::Playlist;

use crate::app::App;
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::{artwork, paint_icon, paint_text, text};

const WIDTH: f32 = 480.0;
const ROW: f32 = 60.0;
const ROW_GAP: f32 = 8.0;
const LIST_HEIGHT: f32 = 420.0;

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(dialog) = &app.playlist_dialog else {
        return;
    };
    let theme = app.theme.clone();
    let tracks = dialog.tracks.clone();
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
    let mut add = false;
    let mut cancel = false;
    let mut new = false;
    let response = egui::Modal::new(Id::unique("playlist-dialog"))
        .frame(frame)
        .backdrop_color(backdrop)
        .show(ctx, |ui| {
            ui.set_width(WIDTH);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

            // The song, or how many.
            let (head, _) = ui.allocate_exact_size(vec2(WIDTH, 88.0), Sense::hover());
            let art = Rect::from_min_size(head.min + vec2(24.0, 18.0), vec2(52.0, 52.0));
            artwork(
                ui,
                tracks
                    .first()
                    .and_then(|t| t.artwork.as_ref())
                    .map(|a| a.url(120)),
                art,
                8.0,
                &theme,
            );
            let x = art.right() + 16.0;
            let title = if tracks.len() > 1 {
                "Add to Playlists"
            } else {
                "Add to Playlist"
            };
            let g = text(ui, title, font(18.0, Weight::Bold), theme.text, WIDTH - x);
            paint_text(
                ui,
                g,
                x,
                art.center().y - 10.0,
                egui::Align::Min,
                theme.text,
            );
            let sub = match tracks.as_slice() {
                [one] => one.title.clone(),
                many => format!("{} songs", many.len()),
            };
            let g = text(
                ui,
                &sub,
                font(13.0, Weight::Regular),
                theme.subtext0,
                WIDTH - x - 24.0,
            );
            paint_text(
                ui,
                g,
                x,
                art.center().y + 12.0,
                egui::Align::Min,
                theme.subtext0,
            );
            ui.painter().hline(
                head.x_range(),
                head.bottom(),
                Stroke::new(1.0, theme.line()),
            );

            // The search.
            ui.add_space(18.0);
            let (search, _) = ui.allocate_exact_size(vec2(WIDTH, 36.0), Sense::hover());
            let field = Rect::from_min_max(
                pos2(search.left() + 24.0, search.top()),
                pos2(search.right() - 24.0, search.bottom()),
            );
            ui.painter().rect(
                field,
                9.0,
                theme.text_a(0.04),
                Stroke::new(1.0, theme.line()),
                egui::StrokeKind::Inside,
            );
            paint_icon(
                ui,
                icons::SEARCH,
                pos2(field.left() + 16.0, field.center().y),
                14.0,
                theme.muted(),
            );
            let body = font(13.5, Weight::Regular);
            let line = text(ui, "S", body.clone(), theme.text, 100.0).size().y;
            let edit = Rect::from_min_max(
                pos2(field.left() + 34.0, field.center().y - line / 2.0),
                pos2(field.right() - 12.0, field.center().y + line / 2.0),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(edit));
            if let Some(dialog) = &mut app.playlist_dialog {
                child.add(
                    egui::TextEdit::singleline(&mut dialog.filter)
                        .hint_text(egui::RichText::new("Search Playlists").color(theme.muted()))
                        .font(body)
                        .frame(Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .desired_width(edit.width()),
                );
            }
            ui.add_space(12.0);

            // Recent, then every playlist, filtered by the search.
            let filter = app
                .playlist_dialog
                .as_ref()
                .map(|d| d.filter.trim().to_lowercase())
                .unwrap_or_default();
            let matches =
                |p: &Playlist| filter.is_empty() || p.name.to_lowercase().contains(&filter);
            let tracks = app
                .playlist_dialog
                .as_ref()
                .map(|d| d.tracks.clone())
                .unwrap_or_default();
            let recent: Vec<Playlist> = if filter.is_empty() {
                app.recent_playlists(&tracks)
            } else {
                Vec::new()
            };
            let all: Vec<Playlist> = app
                .editable_playlists(&tracks)
                .into_iter()
                .filter(matches)
                .collect();
            egui::ScrollArea::vertical()
                .id_salt("playlist-dialog-list")
                .max_height(LIST_HEIGHT)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    if !recent.is_empty() {
                        heading(ui, &theme, "RECENT");
                        for playlist in &recent {
                            row(app, ui, &theme, playlist, "recent");
                        }
                        ui.add_space(8.0);
                        let (line, _) = ui.allocate_exact_size(vec2(WIDTH, 1.0), Sense::hover());
                        ui.painter().hline(
                            (line.left() + 24.0)..=(line.right() - 24.0),
                            line.center().y,
                            Stroke::new(1.0, theme.line()),
                        );
                    }
                    heading(ui, &theme, "PLAYLISTS");
                    if new_row(ui, &theme) {
                        new = true;
                    }
                    if all.is_empty() {
                        let (r, _) = ui.allocate_exact_size(vec2(WIDTH, 40.0), Sense::hover());
                        let muted = theme.muted();
                        let g = text(
                            ui,
                            "No playlist of yours matches.",
                            font(13.0, Weight::Regular),
                            muted,
                            WIDTH,
                        );
                        paint_text(
                            ui,
                            g,
                            r.left() + 24.0,
                            r.center().y,
                            egui::Align::Min,
                            muted,
                        );
                    }
                    for playlist in &all {
                        row(app, ui, &theme, playlist, "all");
                    }
                    ui.add_space(8.0);
                });

            // Cancel and Add.
            ui.add_space(8.0);
            let (foot, _) = ui.allocate_exact_size(vec2(WIDTH, 72.0), Sense::hover());
            ui.painter()
                .hline(foot.x_range(), foot.top(), Stroke::new(1.0, theme.line()));
            let chosen = app.playlist_dialog.as_ref().map_or(0, |d| d.chosen.len());
            let add_label = match chosen {
                0 | 1 => "Add".to_owned(),
                n => format!("Add to {n}"),
            };
            let y = foot.center().y;
            let cancel_rect =
                Rect::from_center_size(pos2(foot.center().x - 70.0, y), vec2(124.0, 40.0));
            let add_rect =
                Rect::from_center_size(pos2(foot.center().x + 70.0, y), vec2(124.0, 40.0));
            if pill(ui, &theme, cancel_rect, "Cancel", false, true) {
                cancel = true;
            }
            if pill(ui, &theme, add_rect, &add_label, true, chosen > 0) {
                add = true;
            }
            if chosen > 0 && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                add = true;
            }
        });
    if new {
        app.open_new_playlist(tracks);
    } else if add {
        app.add_to_chosen_playlists();
    } else if cancel || response.should_close() {
        app.playlist_dialog = None;
    }
}

/// The New Playlist window: a name, then Cancel and Create; Enter creates.
pub fn show_new(app: &mut App, ctx: &egui::Context) {
    let Some(new) = &app.new_playlist else {
        return;
    };
    let theme = app.theme.clone();
    let count = new.tracks.len();
    // Where a new playlist goes, asked when there is more than one choice.
    let services = match new.editing {
        Some(_) => Vec::new(),
        None => app.new_playlist_services(&new.tracks),
    };
    let (title, action) = if new.editing.is_some() {
        ("Edit Playlist", "Save")
    } else {
        ("New Playlist", "Create")
    };
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
    let mut create = false;
    let mut cancel = false;
    let response = egui::Modal::new(Id::unique("new-playlist"))
        .frame(frame)
        .backdrop_color(backdrop)
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

            let (head, _) = ui.allocate_exact_size(vec2(width, 72.0), Sense::hover());
            let g = text(
                ui,
                title,
                font(18.0, Weight::Bold),
                theme.text,
                width - 48.0,
            );
            paint_text(
                ui,
                g,
                head.left() + 24.0,
                head.center().y - if count > 0 { 9.0 } else { 0.0 },
                egui::Align::Min,
                theme.text,
            );
            if count > 0 {
                let sub = match count {
                    1 => "With 1 song".to_owned(),
                    n => format!("With {n} songs"),
                };
                let g = text(
                    ui,
                    &sub,
                    font(13.0, Weight::Regular),
                    theme.subtext0,
                    width - 48.0,
                );
                paint_text(
                    ui,
                    g,
                    head.left() + 24.0,
                    head.center().y + 12.0,
                    egui::Align::Min,
                    theme.subtext0,
                );
            }

            // The name.
            let (row, _) = ui.allocate_exact_size(vec2(width, 40.0), Sense::hover());
            let field = Rect::from_min_max(
                pos2(row.left() + 24.0, row.top()),
                pos2(row.right() - 24.0, row.bottom()),
            );
            ui.painter().rect(
                field,
                9.0,
                theme.text_a(0.04),
                Stroke::new(1.0, theme.accent),
                egui::StrokeKind::Inside,
            );
            let body = font(14.0, Weight::Regular);
            let line = text(ui, "S", body.clone(), theme.text, 100.0).size().y;
            let edit = Rect::from_min_max(
                pos2(field.left() + 14.0, field.center().y - line / 2.0),
                pos2(field.right() - 14.0, field.center().y + line / 2.0),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(edit));
            let mut writing_notes = false;
            if let Some(new) = &mut app.new_playlist {
                let response = child.add(
                    egui::TextEdit::singleline(&mut new.name)
                        .hint_text(egui::RichText::new("Playlist Name").color(theme.muted()))
                        .font(body.clone())
                        .frame(Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .desired_width(edit.width()),
                );
                // The name has the keyboard when the window opens.
                if ui.ctx().memory(|m| m.focused()).is_none() {
                    response.request_focus();
                }

                // The description, a few lines.
                ui.add_space(12.0);
                let (row, _) = ui.allocate_exact_size(vec2(width, 96.0), Sense::hover());
                let field = Rect::from_min_max(
                    pos2(row.left() + 24.0, row.top()),
                    pos2(row.right() - 24.0, row.bottom()),
                );
                ui.painter().rect(
                    field,
                    9.0,
                    theme.text_a(0.04),
                    Stroke::new(1.0, theme.line()),
                    egui::StrokeKind::Inside,
                );
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field.shrink(12.0)));
                let notes = egui::ScrollArea::vertical()
                    .id_salt("new-playlist-description")
                    .max_height(field.height() - 24.0)
                    .show(&mut child, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut new.description)
                                .hint_text(
                                    egui::RichText::new("Description (Optional)")
                                        .color(theme.muted()),
                                )
                                .font(body)
                                .frame(Frame::NONE)
                                .margin(egui::Margin::ZERO)
                                .desired_rows(4)
                                .desired_width(field.width() - 24.0),
                        )
                    });
                writing_notes = notes.inner.has_focus();
            }
            if services.len() > 1 {
                ui.add_space(12.0);
                let (row, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
                let gap = 8.0;
                let n = services.len() as f32;
                let w = (row.width() - 48.0 - gap * (n - 1.0)) / n;
                for (i, &service) in services.iter().enumerate() {
                    let x = row.left() + 24.0 + i as f32 * (w + gap);
                    let rect = Rect::from_min_size(pos2(x, row.top()), vec2(w, row.height()));
                    let chosen = app.new_playlist.as_ref().map(|n| n.service) == Some(service);
                    if pill(
                        ui,
                        &theme,
                        rect,
                        super::library::service(service),
                        chosen,
                        true,
                    ) && let Some(new) = &mut app.new_playlist
                    {
                        new.service = service;
                    }
                }
            }
            let named = app
                .new_playlist
                .as_ref()
                .is_some_and(|n| !n.name.trim().is_empty());

            ui.add_space(20.0);
            let (foot, _) = ui.allocate_exact_size(vec2(width, 72.0), Sense::hover());
            ui.painter()
                .hline(foot.x_range(), foot.top(), Stroke::new(1.0, theme.line()));
            let y = foot.center().y;
            let cancel_rect =
                Rect::from_center_size(pos2(foot.center().x - 70.0, y), vec2(124.0, 40.0));
            let create_rect =
                Rect::from_center_size(pos2(foot.center().x + 70.0, y), vec2(124.0, 40.0));
            if pill(ui, &theme, cancel_rect, "Cancel", false, true) {
                cancel = true;
            }
            if pill(ui, &theme, create_rect, action, true, named) {
                create = true;
            }
            // Enter makes a new line in the description, and saves elsewhere.
            if named && !writing_notes && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                create = true;
            }
        });
    if create {
        app.create_playlist();
    } else if cancel || response.should_close() {
        app.new_playlist = None;
    }
}

/// Whether to delete a playlist, asked once: Cancel, or Delete in red.
pub fn show_delete(app: &mut App, ctx: &egui::Context) {
    let Some(playlist) = &app.deleting else {
        return;
    };
    let theme = app.theme.clone();
    let name = playlist.name.clone();
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
    let mut delete = false;
    let mut cancel = false;
    let response = egui::Modal::new(Id::unique("delete-playlist"))
        .frame(frame)
        .backdrop_color(backdrop)
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);

            let (head, _) = ui.allocate_exact_size(vec2(width, 96.0), Sense::hover());
            let g = text(
                ui,
                &format!("Delete “{name}”?"),
                font(18.0, Weight::Bold),
                theme.text,
                width - 48.0,
            );
            paint_text(
                ui,
                g,
                head.left() + 24.0,
                head.center().y - 11.0,
                egui::Align::Min,
                theme.text,
            );
            let g = text(
                ui,
                "It goes from your library on every device.",
                font(13.0, Weight::Regular),
                theme.subtext0,
                width - 48.0,
            );
            paint_text(
                ui,
                g,
                head.left() + 24.0,
                head.center().y + 13.0,
                egui::Align::Min,
                theme.subtext0,
            );

            let (foot, _) = ui.allocate_exact_size(vec2(width, 72.0), Sense::hover());
            ui.painter()
                .hline(foot.x_range(), foot.top(), Stroke::new(1.0, theme.line()));
            let y = foot.center().y;
            let cancel_rect =
                Rect::from_center_size(pos2(foot.center().x - 70.0, y), vec2(124.0, 40.0));
            let delete_rect =
                Rect::from_center_size(pos2(foot.center().x + 70.0, y), vec2(124.0, 40.0));
            if pill(ui, &theme, cancel_rect, "Cancel", false, true) {
                cancel = true;
            }
            if danger_pill(ui, &theme, delete_rect, "Delete") {
                delete = true;
            }
        });
    if delete {
        app.delete_playlist();
    } else if cancel || response.should_close() {
        app.deleting = None;
    }
}

/// A 40 px rounded button in red, for what can't be undone.
fn danger_pill(ui: &mut Ui, theme: &Theme, rect: Rect, label: &str) -> bool {
    let response = ui.interact(
        rect,
        Id::unique(("playlist-dialog-danger", label)),
        Sense::click(),
    );
    let fill = if response.hovered() {
        crate::color::mix(theme.red, theme.text, 0.85)
    } else {
        theme.red
    };
    ui.painter().rect_filled(rect, 20.0, fill);
    let g = text(
        ui,
        label,
        font(14.0, Weight::Medium),
        Color32::WHITE,
        rect.width(),
    );
    paint_text(
        ui,
        g,
        rect.center().x,
        rect.center().y,
        egui::Align::Center,
        Color32::WHITE,
    );
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// "New Playlist…", at the top of the list.
fn new_row(ui: &mut Ui, theme: &Theme) -> bool {
    let (outer, _) = ui.allocate_exact_size(vec2(WIDTH, ROW + ROW_GAP), Sense::hover());
    let rect = Rect::from_min_max(
        pos2(outer.left() + 24.0, outer.top()),
        pos2(outer.right() - 24.0, outer.top() + ROW),
    );
    let response = ui.interact(rect, Id::unique("playlist-dialog-new"), Sense::click());
    let fill = if response.hovered() {
        theme.text_a(0.08)
    } else {
        theme.text_a(0.04)
    };
    ui.painter().rect(
        rect,
        10.0,
        fill,
        Stroke::new(1.0, theme.line()),
        egui::StrokeKind::Inside,
    );
    let art = Rect::from_center_size(
        pos2(rect.left() + 24.0 + 9.0 + 14.0 + 20.0, rect.center().y),
        vec2(40.0, 40.0),
    );
    ui.painter().rect_filled(art, 6.0, theme.text_a(0.08));
    paint_icon(ui, icons::PLUS, art.center(), 18.0, theme.accent);
    let x = art.right() + 14.0;
    let g = text(
        ui,
        "New Playlist…",
        font(14.0, Weight::SemiBold),
        theme.text,
        rect.right() - 16.0 - x,
    );
    paint_text(ui, g, x, rect.center().y, egui::Align::Min, theme.text);
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn heading(ui: &mut Ui, theme: &Theme, label: &str) {
    let (r, _) = ui.allocate_exact_size(vec2(WIDTH, 36.0), Sense::hover());
    let muted = theme.muted();
    let g = text(ui, label, font(11.5, Weight::SemiBold), muted, WIDTH);
    paint_text(
        ui,
        g,
        r.left() + 26.0,
        r.center().y + 2.0,
        egui::Align::Min,
        muted,
    );
}

/// A playlist with its box: ticked if chosen; under the name, whether it has
/// the songs already.
fn row(app: &mut App, ui: &mut Ui, theme: &Theme, playlist: &Playlist, salt: &str) {
    let (outer, _) = ui.allocate_exact_size(vec2(WIDTH, ROW + ROW_GAP), Sense::hover());
    let rect = Rect::from_min_max(
        pos2(outer.left() + 24.0, outer.top()),
        pos2(outer.right() - 24.0, outer.top() + ROW),
    );
    let response = ui.interact(
        rect,
        Id::unique(("playlist-dialog-row", salt, &playlist.id.id)),
        Sense::click(),
    );
    let chosen = app
        .playlist_dialog
        .as_ref()
        .is_some_and(|d| d.chosen.contains(&playlist.id));
    let fill = if response.hovered() {
        theme.text_a(0.08)
    } else {
        theme.text_a(0.04)
    };
    let border = if chosen { theme.accent } else { theme.line() };
    ui.painter().rect(
        rect,
        10.0,
        fill,
        Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );

    let tick = Rect::from_center_size(pos2(rect.left() + 24.0, rect.center().y), vec2(18.0, 18.0));
    if chosen {
        ui.painter().rect_filled(tick, 5.0, theme.accent);
        paint_icon(ui, icons::CHECK, tick.center(), 12.0, theme.on_accent());
    } else {
        ui.painter().rect_stroke(
            tick,
            5.0,
            Stroke::new(1.5, theme.text_a(0.35)),
            egui::StrokeKind::Inside,
        );
    }
    let art = Rect::from_center_size(
        pos2(tick.right() + 14.0 + 20.0, rect.center().y),
        vec2(40.0, 40.0),
    );
    artwork(
        ui,
        playlist.artwork.as_ref().map(|a| a.url(120)),
        art,
        6.0,
        theme,
    );
    let x = art.right() + 14.0;
    let room = rect.right() - 16.0 - x;
    let tracks = app
        .playlist_dialog
        .as_ref()
        .map(|d| d.tracks.clone())
        .unwrap_or_default();
    let state = match app.playlist_has(&playlist.id, &tracks) {
        Some(true) => Some(if tracks.len() > 1 {
            "Has them already"
        } else {
            "Already in playlist"
        }),
        Some(false) => None,
        None => Some("Checking…"),
    };
    let name_y = if state.is_some() {
        rect.center().y - 9.0
    } else {
        rect.center().y
    };
    let g = text(
        ui,
        &playlist.name,
        font(14.0, Weight::SemiBold),
        theme.text,
        room,
    );
    paint_text(ui, g, x, name_y, egui::Align::Min, theme.text);
    if let Some(state) = state {
        let muted = theme.muted();
        let g = text(ui, state, font(12.0, Weight::Regular), muted, room);
        paint_text(ui, g, x, rect.center().y + 11.0, egui::Align::Min, muted);
    }
    if response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
        && let Some(dialog) = &mut app.playlist_dialog
    {
        if chosen {
            dialog.chosen.retain(|id| *id != playlist.id);
        } else {
            dialog.chosen.push(playlist.id.clone());
        }
    }
}

/// A 40 px rounded button: the accent for the action, a faint fill otherwise.
pub(super) fn pill(
    ui: &mut Ui,
    theme: &Theme,
    rect: Rect,
    label: &str,
    primary: bool,
    enabled: bool,
) -> bool {
    let response = ui.interact(
        rect,
        Id::unique(("playlist-dialog-pill", label)),
        Sense::click(),
    );
    let hovered = enabled && response.hovered();
    let (fill, fg) = match (primary, enabled) {
        (true, true) => (
            if hovered {
                crate::color::mix(theme.accent, theme.text, 0.85)
            } else {
                theme.accent
            },
            theme.on_accent(),
        ),
        (true, false) => (theme.text_a(0.06), theme.muted()),
        (false, _) => (theme.text_a(if hovered { 0.14 } else { 0.08 }), theme.text),
    };
    ui.painter().rect_filled(rect, 20.0, fill);
    let g = text(ui, label, font(14.0, Weight::Medium), fg, rect.width());
    paint_text(
        ui,
        g,
        rect.center().x,
        rect.center().y,
        egui::Align::Center,
        fg,
    );
    enabled
        && response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
}
