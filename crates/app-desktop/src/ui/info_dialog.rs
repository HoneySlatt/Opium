//! Get Info, as iTunes has it: a music file's tags, or an album's files'
//! at once, written back into the files.

use egui::{Color32, Frame, Id, Rect, Sense, Stroke, Ui, pos2, vec2};

use crate::app::{App, CoverChoice, InfoFields};
use crate::fonts::{Weight, font};
use crate::theme::Theme;

use super::playlist_dialog::pill;
use super::{artwork, paint_text, text};

const WIDTH: f32 = 460.0;
/// The labels' column.
const LABELS: f32 = 104.0;
const FIELD: f32 = 34.0;
const FIELD_GAP: f32 = 8.0;

pub fn show(app: &mut App, ctx: &egui::Context) {
    // A picture picked meanwhile, made a cover.
    if let Some(info) = &mut app.song_info
        && let Some(jpeg) = info.picked.lock().unwrap_or_else(|e| e.into_inner()).take()
    {
        info.cover = CoverChoice::Chosen(jpeg.into());
    }
    let Some(info) = &app.song_info else {
        return;
    };
    let theme = app.theme.clone();
    let several = info.tracks.len() > 1;
    let (title, sub) = match info.tracks.as_slice() {
        [one] => (
            one.title.clone(),
            std::path::Path::new(&one.id.id)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        ),
        many => (
            many[0].album.clone().unwrap_or_else(|| "Songs".into()),
            format!("{} songs: what is typed goes in each", many.len()),
        ),
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
    let (mut save, mut cancel) = (false, false);
    let response = egui::Modal::new(Id::unique("song-info"))
        .frame(frame)
        .backdrop_color(backdrop)
        .show(ctx, |ui| {
            ui.set_width(WIDTH);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let Some(info) = &mut app.song_info else {
                return;
            };

            // The cover, with its title and file beside it.
            let (head, _) = ui.allocate_exact_size(vec2(WIDTH, 152.0), Sense::hover());
            let art = Rect::from_min_size(head.min + vec2(24.0, 24.0), vec2(112.0, 112.0));
            let has_cover = match &info.cover {
                CoverChoice::Chosen(jpeg) => {
                    ui.painter().rect_filled(art, 10.0, theme.surface0);
                    egui::Image::from_bytes(
                        format!("bytes://info-cover-{}.jpg", jpeg.len()),
                        egui::load::Bytes::Shared(jpeg.clone()),
                    )
                    .corner_radius(10)
                    .paint_at(ui, art);
                    true
                }
                CoverChoice::Removed => {
                    ui.painter().rect_filled(art, 10.0, theme.surface0);
                    false
                }
                CoverChoice::Unchanged => {
                    artwork(ui, info.artwork.clone(), art, 10.0, &theme);
                    info.artwork.is_some()
                }
            };
            let x = art.right() + 16.0;
            let words = WIDTH - (x - head.left()) - 24.0;
            let g = text(ui, &title, font(17.0, Weight::Bold), theme.text, words);
            paint_text(ui, g, x, art.top() + 12.0, egui::Align::Min, theme.text);
            let g = text(ui, &sub, font(12.5, Weight::Regular), theme.subtext0, words);
            paint_text(ui, g, x, art.top() + 34.0, egui::Align::Min, theme.subtext0);
            let choose = Rect::from_min_size(pos2(x, art.bottom() - 34.0), vec2(124.0, 34.0));
            if pill(
                ui,
                &theme,
                choose,
                if has_cover {
                    "Change Cover…"
                } else {
                    "Add Cover…"
                },
                false,
                true,
            ) {
                info.pick_cover(ui.ctx());
            }
            if has_cover {
                let remove =
                    Rect::from_min_size(pos2(choose.right() + 8.0, choose.top()), vec2(84.0, 34.0));
                if pill(ui, &theme, remove, "Remove", false, true) {
                    info.cover = CoverChoice::Removed;
                }
            }

            // The tags: a song's title and number only for one song.
            let mixed = info.mixed.clone();
            let fields = &mut info.fields;
            let InfoFields {
                title,
                artist,
                album,
                album_artist,
                genre,
                track,
                disc,
                year,
            } = fields;
            let mut rows: Vec<(&str, &mut String, bool)> = Vec::new();
            if !several {
                rows.push(("Title", title, mixed.title));
            }
            rows.push(("Artist", artist, mixed.artist));
            rows.push(("Album", album, mixed.album));
            rows.push(("Album Artist", album_artist, mixed.album_artist));
            rows.push(("Genre", genre, mixed.genre));
            for (i, (label, value, mixed)) in rows.into_iter().enumerate() {
                field(ui, &theme, label, value, mixed, i == 0);
            }
            // The numbers, on one row.
            let (row, _) = ui.allocate_exact_size(vec2(WIDTH, FIELD + FIELD_GAP), Sense::hover());
            let mut numbers: Vec<(&str, &mut String, bool)> = Vec::new();
            if !several {
                numbers.push(("Track", track, mixed.track));
            }
            numbers.push(("Disc", disc, mixed.disc));
            numbers.push(("Year", year, mixed.year));
            let mut left = row.left() + 24.0;
            for (label, value, mixed) in numbers {
                let g = text(ui, label, font(13.0, Weight::Medium), theme.subtext0, 80.0);
                let w = g.size().x;
                paint_text(
                    ui,
                    g,
                    left,
                    row.top() + FIELD / 2.0,
                    egui::Align::Min,
                    theme.subtext0,
                );
                let rect = Rect::from_min_size(pos2(left + w + 10.0, row.top()), vec2(64.0, FIELD));
                edit(ui, &theme, rect, value, mixed, false);
                left = rect.right() + 20.0;
            }

            if let Some(error) = &info.error {
                ui.add_space(4.0);
                let (row, _) = ui.allocate_exact_size(vec2(WIDTH, 20.0), Sense::hover());
                let g = text(
                    ui,
                    error,
                    font(12.5, Weight::Regular),
                    theme.red,
                    WIDTH - 48.0,
                );
                paint_text(
                    ui,
                    g,
                    row.left() + 24.0,
                    row.center().y,
                    egui::Align::Min,
                    theme.red,
                );
            }

            ui.add_space(12.0);
            let (foot, _) = ui.allocate_exact_size(vec2(WIDTH, 72.0), Sense::hover());
            ui.painter()
                .hline(foot.x_range(), foot.top(), Stroke::new(1.0, theme.line()));
            let y = foot.center().y;
            let cancel_rect =
                Rect::from_center_size(pos2(foot.center().x - 70.0, y), vec2(124.0, 40.0));
            let save_rect =
                Rect::from_center_size(pos2(foot.center().x + 70.0, y), vec2(124.0, 40.0));
            if pill(ui, &theme, cancel_rect, "Cancel", false, true) {
                cancel = true;
            }
            let label = if info.saving { "Saving…" } else { "Save" };
            if pill(ui, &theme, save_rect, label, true, !info.saving) {
                save = true;
            }
            if !info.saving && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                save = true;
            }
        });
    if save {
        app.save_info();
    } else if cancel || response.should_close() {
        app.song_info = None;
    }
}

/// A labelled tag on its row.
fn field(ui: &mut Ui, theme: &Theme, label: &str, value: &mut String, mixed: bool, first: bool) {
    let (row, _) = ui.allocate_exact_size(vec2(WIDTH, FIELD + FIELD_GAP), Sense::hover());
    let g = text(
        ui,
        label,
        font(13.0, Weight::Medium),
        theme.subtext0,
        LABELS,
    );
    paint_text(
        ui,
        g,
        row.left() + 24.0,
        row.top() + FIELD / 2.0,
        egui::Align::Min,
        theme.subtext0,
    );
    let rect = Rect::from_min_max(
        pos2(row.left() + 24.0 + LABELS, row.top()),
        pos2(row.right() - 24.0, row.top() + FIELD),
    );
    edit(ui, theme, rect, value, mixed, first);
}

/// A text field in `rect`; "Mixed" when the files differ and nothing is typed.
fn edit(ui: &mut Ui, theme: &Theme, rect: Rect, value: &mut String, mixed: bool, focus: bool) {
    ui.painter().rect(
        rect,
        8.0,
        theme.text_a(0.04),
        Stroke::new(1.0, theme.line()),
        egui::StrokeKind::Inside,
    );
    let body = font(13.5, Weight::Regular);
    let line = text(ui, "S", body.clone(), theme.text, 100.0).size().y;
    let inner = Rect::from_min_max(
        pos2(rect.left() + 10.0, rect.center().y - line / 2.0),
        pos2(rect.right() - 10.0, rect.center().y + line / 2.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    let hint = if mixed { "Mixed" } else { "" };
    let response = child.add(
        egui::TextEdit::singleline(value)
            .hint_text(egui::RichText::new(hint).color(theme.muted()))
            .font(body)
            .frame(Frame::NONE)
            .margin(egui::Margin::ZERO)
            .desired_width(inner.width()),
    );
    if focus && ui.ctx().memory(|m| m.focused()).is_none() {
        response.request_focus();
    }
}
