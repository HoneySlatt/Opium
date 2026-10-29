//! Album and playlist pages: a header over a gradient taken from the cover, then
//! the tracks. The artist page shares the header.

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
use super::{Blob, artwork, page_buttons, paint_icon, paint_text, quality_label, text, wash};

/// What the header shows.
pub(super) struct Header {
    pub kind: &'static str,
    pub title: String,
    pub by: Option<String>,
    /// Where the `by` line leads.
    pub by_link: Option<ItemId>,
    pub meta: Vec<(Option<Icon>, String)>,
    pub art: Option<String>,
    /// Round artwork, for an artist.
    pub round: bool,
}

/// What a click in the header asked for.
pub(super) enum HeaderAction {
    Play { shuffle: bool },
    OpenArtist(ItemId),
}

pub(super) const ART_TOP: f32 = 64.0;

pub(super) fn art_size(pad: f32) -> f32 {
    if pad < 40.0 { 200.0 } else { 260.0 }
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

pub fn playlist(app: &mut App, ui: &mut Ui, pad: f32, id: &ItemId) -> f32 {
    let Some(playlist) = app.playlist(id).cloned() else {
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
        by_link: None,
        meta,
        art: playlist.artwork.as_ref().map(|a| a.url(600)),
        round: false,
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
        by_link: album.artist_id.clone(),
        meta,
        art: album.artwork.as_ref().map(|a| a.url(600)),
        round: false,
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

fn page(
    app: &mut App,
    ui: &mut Ui,
    pad: f32,
    header: Option<Header>,
    tracks: Arc<Vec<Track>>,
    listing: Listing,
) -> f32 {
    let theme = app.theme.clone();
    let table_top = ART_TOP + art_size(pad) + 26.0;
    let rows = tracks.len();
    let foot = 18.0 + 16.0 + 18.0;
    let total = table_top + tracks::HEAD_SPACE + rows.max(1) as f32 * tracks::ROW + foot + 104.0;
    let colors = super::backdrop(app);
    let output = egui::ScrollArea::vertical()
        .id_salt(listing.list)
        .auto_shrink(false)
        .show_viewport(ui, |ui, viewport| {
            let (content, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
            let left = content.left() + pad;
            let width = content.width() - 2.0 * pad;
            match hero(ui, &theme, content, pad, header.as_ref(), colors) {
                Some(HeaderAction::Play { shuffle }) => {
                    app.play_shuffled_or_not(tracks.to_vec(), shuffle, ui.input(|i| i.time));
                }
                Some(HeaderAction::OpenArtist(id)) => app.go(Page::Artist(id)),
                None => {}
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

/// The cover's colours behind, the artwork, and the text beside it.
pub(super) fn hero(
    ui: &mut Ui,
    theme: &Theme,
    content: Rect,
    pad: f32,
    header: Option<&Header>,
    colors: Option<[Color32; 3]>,
) -> Option<HeaderAction> {
    if let Some(colors) = colors {
        cover_wash(
            ui,
            theme,
            Rect::from_min_size(content.min, vec2(content.width(), 620.0)),
            colors,
        );
    }
    let size = art_size(pad);
    let left = content.left() + pad;
    let art = Rect::from_min_size(pos2(left, content.top() + ART_TOP), vec2(size, size));
    let round = header.is_some_and(|h| h.round);
    let radius = if round { size / 2.0 } else { 12.0 };
    ui.painter().add(
        egui::Shadow {
            offset: [0, 18],
            blur: 48,
            spread: 0,
            color: Color32::from_black_alpha(110),
        }
        .as_shape(art.shrink(6.0), radius),
    );
    artwork(ui, header.and_then(|h| h.art.clone()), art, radius, theme);
    header_text(ui, theme, header?, art, content.right() - pad)
}

/// Three radial blobs of the cover's colours behind the header, fading into
/// `base`: colour 1 top left behind the artwork at 92 %, colour 2 top centre at
/// 72 %, colour 3 on the right at 62 %.
fn cover_wash(ui: &Ui, theme: &Theme, rect: Rect, [c1, c2, c3]: [Color32; 3]) {
    let blobs = [
        Blob {
            center: vec2(0.10, 0.04),
            radii: vec2(0.48, 0.78),
            color: c1,
            alpha: 0.92,
            fade: 0.72,
        },
        Blob {
            center: vec2(0.58, -0.06),
            radii: vec2(0.46, 0.70),
            color: c2,
            alpha: 0.72,
            fade: 0.74,
        },
        Blob {
            center: vec2(1.0, 0.26),
            radii: vec2(0.40, 0.70),
            color: c3,
            alpha: 0.62,
            fade: 0.72,
        },
    ];
    wash(ui, rect, theme.base, &blobs, true);
}

/// The text beside the artwork, bottom-aligned with it: kind, title, artist,
/// details, Play and Shuffle.
fn header_text(
    ui: &mut Ui,
    theme: &Theme,
    header: &Header,
    art: Rect,
    right: f32,
) -> Option<HeaderAction> {
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
    let mut action = None;
    if let Some(by) = by {
        let size = by.size();
        let mut color = by_color;
        if let Some(id) = &header.by_link {
            let link = Rect::from_min_size(pos2(x, y), size);
            let hit = ui.interact(link, egui::Id::unique("header-by"), Sense::click());
            if hit.hovered() {
                color = theme.text;
                ui.painter()
                    .hline(link.x_range(), link.bottom(), egui::Stroke::new(1.0, color));
            }
            if hit
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                action = Some(HeaderAction::OpenArtist(id.clone()));
            }
        }
        paint_text(ui, by, x, y + size.y / 2.0, egui::Align::Min, color);
        y += size.y + 8.0;
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
        .map(|shuffle| HeaderAction::Play { shuffle })
        .or(action)
}
