//! Album and playlist pages: a header over a gradient taken from the cover, then
//! the tracks.

use std::sync::Arc;
use std::time::Duration;

use egui::{Color32, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{Album, ItemId, Track};

use crate::app::{App, Loaded, Page};
use crate::color::mix;
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::tracks::{self, Columns, RowAction, RowState};
use super::{artwork, page_buttons, paint_icon, paint_text, quality_label, text};

/// What the header shows.
struct Header {
    kind: &'static str,
    title: String,
    by: Option<String>,
    meta: Vec<(Option<Icon>, String)>,
    art: Option<String>,
}

/// How the list under it reads.
struct Listing {
    with_album: bool,
    /// Album tracks show their track numbers; playlists count positions.
    track_numbers: bool,
    list: &'static str,
    loading: bool,
    error: Option<String>,
}

pub fn album(app: &mut App, ui: &mut Ui, pad: f32, id: &ItemId) -> f32 {
    let (header, tracks, loading, error) = match app.albums.get(id) {
        Some(Loaded::Ready((album, tracks))) => (
            Some(album_header(album, tracks)),
            tracks.clone(),
            false,
            None,
        ),
        Some(Loaded::Failed(e)) => (None, Arc::default(), false, Some(e.clone())),
        _ => (None, Arc::default(), true, None),
    };
    let listing = Listing {
        with_album: false,
        track_numbers: true,
        list: "album",
        loading,
        error,
    };
    page(app, ui, pad, header, tracks, listing)
}

pub fn playlist(app: &mut App, ui: &mut Ui, pad: f32, index: usize) -> f32 {
    let Some(playlist) = app.playlists.get(index).cloned() else {
        return 0.0;
    };
    let (tracks, done, error) = match app.playlist_tracks.get(&playlist.id) {
        Some(entry) => (entry.tracks.clone(), entry.done, entry.error.clone()),
        None => (Arc::default(), false, None),
    };
    let mut meta = vec![(
        Some(icons::NOTE),
        format!("{} TRACKS{}", tracks.len(), if done { "" } else { "…" }),
    )];
    if let Some(total) = total_length(&tracks) {
        meta.push((Some(icons::CLOCK), length_label(total)));
    }
    meta.push((Some(icons::AM), "APPLE MUSIC".into()));
    let header = Header {
        kind: "Playlist",
        title: playlist.name.clone(),
        by: None,
        meta,
        art: playlist.artwork.as_ref().map(|a| a.url(600)),
    };
    let listing = Listing {
        with_album: true,
        track_numbers: false,
        list: "playlist",
        loading: tracks.is_empty() && !done && error.is_none(),
        error,
    };
    page(app, ui, pad, Some(header), tracks, listing)
}

fn album_header(album: &Album, tracks: &[Track]) -> Header {
    let mut meta = Vec::new();
    if let Some(year) = album.year {
        meta.push((None, year.to_string()));
    }
    if let Some((label, _)) = tracks.iter().find_map(quality_label) {
        meta.push((None, label));
    }
    if let Some(total) = total_length(tracks) {
        meta.push((Some(icons::CLOCK), length_label(total)));
    }
    meta.push((Some(icons::AM), "APPLE MUSIC".into()));
    Header {
        kind: "Album",
        title: album.title.clone(),
        by: Some(album.artists.join(", ")),
        meta,
        art: album.artwork.as_ref().map(|a| a.url(600)),
    }
}

fn total_length(tracks: &[Track]) -> Option<Duration> {
    let total: Duration = tracks.iter().filter_map(|t| t.duration).sum();
    (!total.is_zero()).then_some(total)
}

/// `43 MIN`, `1 HR 12 MIN`.
fn length_label(total: Duration) -> String {
    let minutes = (total.as_secs() + 30) / 60;
    if minutes >= 60 {
        format!("{} HR {} MIN", minutes / 60, minutes % 60)
    } else {
        format!("{minutes} MIN")
    }
}

const ART_TOP: f32 = 64.0;

fn page(
    app: &mut App,
    ui: &mut Ui,
    pad: f32,
    header: Option<Header>,
    tracks: Arc<Vec<Track>>,
    listing: Listing,
) -> f32 {
    let theme = app.theme.clone();
    let art_size = if pad < 40.0 { 200.0 } else { 260.0 };
    let table_top = ART_TOP + art_size + 26.0;
    let rows = tracks.len();
    let foot = 18.0 + 16.0 + 18.0;
    let total = table_top + tracks::HEAD_SPACE + rows.max(1) as f32 * tracks::ROW + foot + 104.0;
    let output = egui::ScrollArea::vertical()
        .id_salt(listing.list)
        .auto_shrink(false)
        .show_viewport(ui, |ui, viewport| {
            let (content, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
            let left = content.left() + pad;
            let width = content.width() - 2.0 * pad;
            let art = Rect::from_min_size(
                pos2(left, content.top() + ART_TOP),
                vec2(art_size, art_size),
            );
            ui.painter().add(
                egui::Shadow {
                    offset: [0, 18],
                    blur: 48,
                    spread: 0,
                    color: Color32::from_black_alpha(110),
                }
                .as_shape(art.shrink(6.0), 12.0),
            );
            artwork(
                ui,
                header.as_ref().and_then(|h| h.art.clone()),
                art,
                12.0,
                &theme,
            );
            if let Some(header) = &header
                && let Some(shuffle) = header_text(ui, &theme, header, art, left + width)
            {
                app.play_shuffled_or_not(tracks.to_vec(), shuffle, ui.input(|i| i.time));
            }

            let columns = Columns::new(left, width, listing.with_album);
            let table = content.top() + table_top;
            tracks::head(ui, &theme, &columns, table, left, width, None);
            let rows_top = table + tracks::HEAD_SPACE;
            if let Some(error) = &listing.error {
                let g = text(
                    ui,
                    error,
                    font(13.0, Weight::Regular),
                    theme.subtext0,
                    width - 20.0,
                );
                paint_text(
                    ui,
                    g,
                    left + 10.0,
                    rows_top + 26.0,
                    egui::Align::Min,
                    theme.subtext0,
                );
                return;
            }
            if listing.loading {
                tracks::skeleton(ui, &theme, rows_top, left, width, 6);
                return;
            }
            for position in tracks::visible(viewport, table_top + tracks::HEAD_SPACE, rows) {
                let track = &tracks[position];
                let rect = Rect::from_min_size(
                    pos2(left, rows_top + position as f32 * tracks::ROW),
                    vec2(width, tracks::ROW),
                );
                let now = app.is_current(track);
                let number = if listing.track_numbers {
                    track.track_number
                } else {
                    Some(position as u32 + 1)
                };
                let state = RowState {
                    number,
                    selected: app.detail_selected == Some(position),
                    now,
                    playing: now && app.is_playing(),
                    list: listing.list,
                    position,
                };
                match tracks::row(ui, &theme, &columns, rect, track, state) {
                    RowAction::None => {}
                    RowAction::Select => app.detail_selected = Some(position),
                    RowAction::Play => {
                        app.detail_selected = Some(position);
                        app.play_list(tracks.to_vec(), position);
                    }
                    RowAction::OpenAlbum(id) => app.go(Page::Album(id)),
                }
            }
            if let Some(length) = total_length(&tracks) {
                let minutes = (length.as_secs() + 30) / 60;
                let foot_text = format!(
                    "{rows} song{}, {minutes} minute{}",
                    if rows == 1 { "" } else { "s" },
                    if minutes == 1 { "" } else { "s" }
                );
                let g = text(
                    ui,
                    &foot_text,
                    font(12.0, Weight::Regular),
                    theme.muted(),
                    width,
                );
                paint_text(
                    ui,
                    g,
                    left + 10.0,
                    rows_top + rows as f32 * tracks::ROW + 26.0,
                    egui::Align::Min,
                    theme.muted(),
                );
            }
        });
    if let Some(position) = tracks::keyboard(ui, tracks.len(), &mut app.detail_selected) {
        app.play_list(tracks.to_vec(), position);
    }
    output.state.clamped_offset().y
}

/// The text beside the artwork, bottom-aligned with it: kind, title, artist,
/// details, Play and Shuffle. Returns `Some(shuffle)` when a button was clicked.
fn header_text(ui: &mut Ui, theme: &Theme, header: &Header, art: Rect, right: f32) -> Option<bool> {
    let x = art.right() + 30.0;
    let width = (right - x).max(0.0);
    let eyebrow = text(
        ui,
        &header.kind.to_uppercase(),
        font(11.0, Weight::SemiBold),
        theme.muted(),
        width,
    );
    let title = text(
        ui,
        &header.title,
        font(44.0, Weight::Bold),
        theme.text,
        width,
    );
    let by_color = mix(theme.accent, theme.text, 0.75);
    let by = header
        .by
        .as_ref()
        .map(|by| text(ui, by, font(18.0, Weight::Medium), by_color, width));
    let meta_font = font(12.0, Weight::Medium);
    let meta_height = 17.0;

    let mut height = eyebrow.size().y + 4.0 + title.size().y + 8.0 + meta_height + 18.0 + 36.0;
    if let Some(by) = &by {
        height += by.size().y + 8.0;
    }
    let mut y = art.bottom() - height;

    let h = eyebrow.size().y;
    paint_text(ui, eyebrow, x, y + h / 2.0, egui::Align::Min, theme.muted());
    y += h + 4.0;
    let h = title.size().y;
    paint_text(ui, title, x, y + h / 2.0, egui::Align::Min, theme.text);
    y += h + 8.0;
    if let Some(by) = by {
        let h = by.size().y;
        paint_text(ui, by, x, y + h / 2.0, egui::Align::Min, by_color);
        y += h + 8.0;
    }

    let mut mx = x;
    let my = y + meta_height / 2.0;
    for (icon, label) in &header.meta {
        if let Some(icon) = icon {
            paint_icon(ui, *icon, pos2(mx + 6.0, my), 12.0, theme.subtext0);
            mx += 12.0 + 5.0;
        }
        let g = text(
            ui,
            label,
            meta_font.clone(),
            theme.subtext0,
            (right - mx).max(0.0),
        );
        let w = g.size().x;
        paint_text(ui, g, mx, my, egui::Align::Min, theme.subtext0);
        mx += w + 14.0;
    }
    y += meta_height + 18.0;

    page_buttons(ui, theme, pos2(x, y), header.kind)
}
