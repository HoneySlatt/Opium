//! Add to Playlists, as Cider asks it: the song at the top, a search, the
//! playlists used last, then every playlist of the user's with a box to tick,
//! each saying whether it has the song already.

use std::collections::HashSet;

use egui::{Color32, Frame, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use opium_core::model::{Collection, ItemId, Playlist};

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
    // Only what the top of the window shows: a copy of the staged songs
    // themselves, a thousand of them when a whole album list is added, cost
    // more every frame than the window had to spare.
    let count = dialog.tracks.len();
    let cover = dialog
        .tracks
        .first()
        .and_then(|t| t.artwork.as_ref())
        .map(|a| a.url(120));
    let sub = match dialog.tracks.as_slice() {
        [one] => one.title.clone(),
        many => format!("{} songs", many.len()),
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
            artwork(ui, cover.clone(), art, 8.0, &theme);
            let x = art.right() + 16.0;
            let title = if count > 1 {
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
            let staged: &[opium_core::model::Track] = app
                .playlist_dialog
                .as_ref()
                .map_or(&[], |d| d.tracks.as_slice());
            // The staged songs by id, made once: a playlist then says whether
            // it has them all in one walk of its own songs, rather than one
            // walk of the staged list for each song it holds.
            let ids: HashSet<&ItemId> = staged.iter().map(|t| &t.id).collect();
            let recent: Vec<Playlist> = if filter.is_empty() {
                app.recent_playlists(staged)
            } else {
                Vec::new()
            };
            let all: Vec<Playlist> = app
                .editable_playlists(staged)
                .into_iter()
                .filter(matches)
                .collect();
            let mut ticked: Option<ItemId> = None;
            egui::ScrollArea::vertical()
                .id_salt("playlist-dialog-list")
                .max_height(LIST_HEIGHT)
                .auto_shrink([false, true])
                .show_viewport(ui, |ui, viewport| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    let top = ui.cursor().top();
                    if !recent.is_empty() {
                        heading(ui, &theme, "RECENT");
                        for playlist in &recent {
                            if row(app, ui, &theme, playlist, "recent", &ids, count) {
                                ticked = Some(playlist.id.clone());
                            }
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
                    // The rows on screen only, the rest left as room above and
                    // below them: every editable playlist was asked whether it
                    // had the songs, however far down the list it was.
                    let shown = visible(viewport, ui.cursor().top() - top, all.len());
                    ui.add_space(shown.start as f32 * (ROW + ROW_GAP));
                    for playlist in &all[shown.clone()] {
                        if row(app, ui, &theme, playlist, "all", &ids, count) {
                            ticked = Some(playlist.id.clone());
                        }
                    }
                    ui.add_space((all.len() - shown.end) as f32 * (ROW + ROW_GAP));
                    ui.add_space(8.0);
                });
            // Once the rows that read the staged songs are drawn.
            if let Some(id) = ticked
                && let Some(dialog) = &mut app.playlist_dialog
            {
                if dialog.chosen.contains(&id) {
                    dialog.chosen.retain(|chosen| *chosen != id);
                } else {
                    dialog.chosen.push(id);
                }
            }

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
        let tracks = app
            .playlist_dialog
            .as_ref()
            .map(|d| d.tracks.clone())
            .unwrap_or_default();
        app.open_new_playlist(tracks);
    } else if add {
        app.add_to_chosen_playlists();
    } else if cancel || response.should_close() {
        app.playlist_dialog = None;
    }
}

/// The New Playlist window: a name, then Cancel and Create; Enter creates.
pub fn show_new(app: &mut App, ctx: &egui::Context) {
    // A picture picked meanwhile, made a cover.
    if let Some(new) = &mut app.new_playlist
        && let Some(jpeg) = new.picked.lock().unwrap_or_else(|e| e.into_inner()).take()
    {
        new.cover = crate::app::CoverChoice::Chosen(jpeg.into());
    }
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

            // Its cover, for Opium's own and Navidrome's: a picture picked,
            // the one it has, or none yet.
            let provider = app
                .new_playlist
                .as_ref()
                .map(|n| n.editing.as_ref().map_or(n.service, |id| id.provider));
            if let Some(new) = app.new_playlist.as_ref()
                && provider.is_some_and(crate::app::coverable)
            {
                let (row, _) = ui.allocate_exact_size(vec2(width, 128.0), Sense::hover());
                let art = Rect::from_min_size(row.min + vec2(24.0, 0.0), vec2(112.0, 112.0));
                let custom = match &new.cover {
                    crate::app::CoverChoice::Chosen(jpeg) => {
                        ui.painter().rect_filled(art, 10.0, theme.surface0);
                        egui::Image::from_bytes(format!("bytes://new-cover-{}.jpg", jpeg.len()), egui::load::Bytes::Shared(jpeg.clone()))
                            .corner_radius(10)
                            .paint_at(ui, art);
                        true
                    }
                    crate::app::CoverChoice::Removed => {
                        ui.painter().rect_filled(art, 10.0, theme.surface0);
                        false
                    }
                    crate::app::CoverChoice::Unchanged => {
                        artwork(ui, new.artwork.clone(), art, 10.0, &theme);
                        new.editing.is_some()
                            && (provider == Some(opium_core::model::ProviderKind::Navidrome)
                                || new.artwork.as_deref().is_some_and(|a| a.starts_with("file://")))
                    }
                };
                let x = art.right() + 16.0;
                let choose = Rect::from_min_size(pos2(x, art.center().y - 38.0), vec2(150.0, 34.0));
                let label = if custom { "Change Cover…" } else { "Choose Cover…" };
                if pill(ui, &theme, choose, label, false, true) {
                    new.pick_cover(ui.ctx());
                }
                if custom {
                    let remove = Rect::from_min_size(pos2(x, art.center().y + 4.0), vec2(150.0, 34.0));
                    if pill(ui, &theme, remove, "Remove Cover", false, true)
                        && let Some(new) = &mut app.new_playlist
                    {
                        new.cover = crate::app::CoverChoice::Removed;
                    }
                }
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

/// Whether to delete a playlist, or an album from the library, asked once:
/// Cancel, or Delete in red.
pub fn show_delete(app: &mut App, ctx: &egui::Context) {
    let (name, gone) = match &app.deleting {
        Some(Collection::Playlist(p)) => {
            (p.name.clone(), "It goes from your library on every device.")
        }
        Some(Collection::Album(a)) => (
            a.title.clone(),
            "It and its songs go from your library on every device.",
        ),
        _ => return,
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
                gone,
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
        app.delete_collection(ctx.input(|i| i.time));
    } else if cancel || response.should_close() {
        app.deleting = None;
    }
}

/// Navidrome's own API wants the password: given once, it opens a session
/// the app keeps (never the password), renewed while the app is used.
pub fn show_navidrome_password(app: &mut App, ctx: &egui::Context) {
    let Some((_, _, refused)) = &app.navidrome_password else {
        return;
    };
    let refused = *refused;
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
    let (mut go, mut cancel) = (false, false);
    let response = egui::Modal::new(Id::unique("navidrome-password"))
        .frame(frame)
        .backdrop_color(backdrop)
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let (head, _) = ui.allocate_exact_size(vec2(width, 96.0), Sense::hover());
            let g = text(ui, "Navidrome’s Password", font(18.0, Weight::Bold), theme.text, width - 48.0);
            paint_text(ui, g, head.left() + 24.0, head.center().y - 11.0, egui::Align::Min, theme.text);
            let said = if refused {
                "That password isn’t the one Navidrome knows."
            } else {
                "Changing a cover needs a session on your server. The password opens it and isn’t kept."
            };
            let colour = if refused { theme.red } else { theme.subtext0 };
            let g = text(ui, said, font(13.0, Weight::Regular), colour, width - 48.0);
            paint_text(ui, g, head.left() + 24.0, head.center().y + 13.0, egui::Align::Min, colour);

            let (row, _) = ui.allocate_exact_size(vec2(width, 40.0), Sense::hover());
            let field = Rect::from_min_max(pos2(row.left() + 24.0, row.top()), pos2(row.right() - 24.0, row.bottom()));
            ui.painter().rect(field, 9.0, theme.text_a(0.04), Stroke::new(1.0, theme.accent), egui::StrokeKind::Inside);
            let body = font(14.0, Weight::Regular);
            let line = text(ui, "S", body.clone(), theme.text, 100.0).size().y;
            let edit = Rect::from_min_max(
                pos2(field.left() + 14.0, field.center().y - line / 2.0),
                pos2(field.right() - 14.0, field.center().y + line / 2.0),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(edit));
            if let Some((_, password, _)) = &mut app.navidrome_password {
                let typed = child.add(
                    egui::TextEdit::singleline(password)
                        .password(true)
                        .hint_text(egui::RichText::new("Password").color(theme.muted()))
                        .font(body)
                        .frame(Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .desired_width(edit.width()),
                );
                if ui.ctx().memory(|m| m.focused()).is_none() {
                    typed.request_focus();
                }
                go = typed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            }
            ui.add_space(16.0);
            let (foot, _) = ui.allocate_exact_size(vec2(width, 72.0), Sense::hover());
            ui.painter().hline(foot.x_range(), foot.top(), Stroke::new(1.0, theme.line()));
            let y = foot.center().y;
            let cancel_rect = Rect::from_center_size(pos2(foot.center().x - 70.0, y), vec2(124.0, 40.0));
            let go_rect = Rect::from_center_size(pos2(foot.center().x + 70.0, y), vec2(124.0, 40.0));
            if pill(ui, &theme, cancel_rect, "Cancel", false, true) {
                cancel = true;
            }
            if pill(ui, &theme, go_rect, "Continue", true, true) {
                go = true;
            }
        });
    if go {
        if let Some((_, password, _)) = &mut app.navidrome_password
            && !password.is_empty()
        {
            let typed = std::mem::take(password);
            app.send_navidrome_password(typed);
        }
    } else if cancel || response.should_close() {
        app.navidrome_password = None;
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

/// Which of `count` rows the viewport shows, the first of them `top` below
/// the top of the list.
fn visible(viewport: Rect, top: f32, count: usize) -> std::ops::Range<usize> {
    let row = ROW + ROW_GAP;
    let first = ((viewport.top() - top) / row).floor().max(0.0) as usize;
    let last = (((viewport.bottom() - top) / row).ceil().max(0.0) as usize).min(count);
    first.min(last)..last
}

/// Whether `playlist` has every song staged, `None` while its songs are
/// still loading: its own songs are looked up in `ids` rather than the
/// other way about, so the cost is the playlist's, not the staging's.
fn has_staged(app: &App, playlist: &ItemId, ids: &HashSet<&ItemId>) -> Option<bool> {
    let entry = app.playlist_tracks.get(playlist)?;
    let mut found: HashSet<&ItemId> = HashSet::new();
    for track in entry.tracks.iter() {
        if ids.contains(&track.id) {
            found.insert(&track.id);
        }
    }
    let has = found.len() == ids.len();
    (has || entry.done).then_some(has)
}

/// A playlist with its box: ticked if chosen; under the name, whether it has
/// the songs already. Whether it was clicked, to tick or untick.
#[allow(clippy::too_many_arguments)]
fn row(
    app: &App,
    ui: &mut Ui,
    theme: &Theme,
    playlist: &Playlist,
    salt: &str,
    ids: &HashSet<&ItemId>,
    count: usize,
) -> bool {
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
    let state = match has_staged(app, &playlist.id, ids) {
        Some(true) => Some(if count > 1 {
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
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
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
