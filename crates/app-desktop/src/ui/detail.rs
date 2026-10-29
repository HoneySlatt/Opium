//! Album and playlist pages, laid out as Cider does: a large cover with the
//! title, artist, details and notes beside it over the cover's colours, then
//! the tracks.

use std::sync::Arc;
use std::time::Duration;

use egui::{Color32, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{Album, FavouriteKind, ItemId, Track};

use crate::app::{App, Loaded, Page};
use crate::color::mix;
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::tracks::{self, Columns, RowAction, RowState};
use super::{Blob, artwork, page_buttons, paint_icon, paint_text, quality_label, text, wash};

/// What the header shows.
struct Header {
    /// Tells the pages' buttons apart.
    kind: &'static str,
    title: String,
    by: Option<String>,
    /// Where the `by` line leads.
    by_link: Option<ItemId>,
    /// The artist's photo, round before their name.
    by_art: Option<String>,
    meta: Vec<(Option<Icon>, String)>,
    description: Option<String>,
    art: Option<String>,
    /// The star after the title: what it names, and whether it is lit.
    favourite: Option<(FavouriteKind, ItemId, bool)>,
}

/// What a click in the header asked for.
enum HeaderAction {
    Play { shuffle: bool },
    OpenArtist(ItemId),
    Favourite(FavouriteKind, ItemId),
}

const ART_TOP: f32 = 60.0;

fn art_size(pad: f32) -> f32 {
    if pad < 40.0 { 220.0 } else { 320.0 }
}

/// Which columns the list has.
enum Kind {
    Album { with_artist: bool },
    Playlist,
}

/// How the list under it reads.
struct Listing {
    columns: Kind,
    /// Album tracks show their track numbers; playlists count positions.
    track_numbers: bool,
    list: &'static str,
    loading: bool,
    error: Option<String>,
}

pub fn album(app: &mut App, ui: &mut Ui, pad: f32, id: &ItemId) -> f32 {
    let (header, tracks, loading, error) = match app.albums.get(id) {
        Some(Loaded::Ready((album, tracks))) => (
            Some(album_header(
                album,
                tracks,
                app.is_favourite(FavouriteKind::Album, &App::album_favourite_id(album)),
            )),
            tracks.clone(),
            false,
            None,
        ),
        Some(Loaded::Failed(e)) => (None, Arc::default(), false, Some(e.clone())),
        _ => (None, Arc::default(), true, None),
    };
    // A compilation names each track's artist; an album by one artist need not.
    let with_artist = match app.albums.get(id) {
        Some(Loaded::Ready((album, tracks))) => tracks.iter().any(|t| t.artists != album.artists),
        _ => false,
    };
    let listing = Listing {
        columns: Kind::Album { with_artist },
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
        by_art: None,
        meta,
        description: playlist.description.clone(),
        art: playlist.artwork.as_ref().map(|a| a.url(600)),
        // Any playlist with a catalog id can be a favourite, the user's own too.
        favourite: App::playlist_favourite_id(&playlist).map(|id| {
            let on = app.is_favourite(FavouriteKind::Playlist, &id);
            (FavouriteKind::Playlist, id, on)
        }),
    };
    let listing = Listing {
        columns: Kind::Playlist,
        track_numbers: false,
        list: "playlist",
        loading: tracks.is_empty() && !done && error.is_none(),
        error,
    };
    page(app, ui, pad, Some(header), tracks, listing)
}

/// Cider's details: genre, year, track count; then the quality, which is this
/// app's business.
fn album_header(album: &Album, tracks: &[Track], favourite: bool) -> Header {
    let mut meta = Vec::new();
    if let Some(genre) = &album.genre {
        meta.push((Some(icons::MIC), genre.to_uppercase()));
    }
    if let Some(year) = album.year {
        meta.push((Some(icons::CLOCK), year.to_string()));
    }
    // What the list holds: a library album lists only the tracks in the library.
    let count = if tracks.is_empty() {
        album.track_count.unwrap_or(0)
    } else {
        tracks.len() as u32
    };
    meta.push((
        Some(icons::NOTE),
        format!("{count} TRACK{}", if count == 1 { "" } else { "S" }),
    ));
    if let Some((label, _)) = tracks.iter().find_map(quality_label) {
        meta.push((Some(icons::AM), label));
    }
    Header {
        kind: "Album",
        title: album.title.clone(),
        by: Some(album.artists.join(", ")),
        by_link: album.artist_id.clone(),
        by_art: album.artist_artwork.as_ref().map(|a| a.url(64)),
        meta,
        description: album.description.clone(),
        art: album.artwork.as_ref().map(|a| a.url(640)),
        favourite: Some((
            FavouriteKind::Album,
            App::album_favourite_id(album),
            favourite,
        )),
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
    let table_top = ART_TOP + art_size(pad) + 34.0;
    // Albums have no column titles, as in Cider.
    let head = match listing.columns {
        Kind::Album { .. } => 0.0,
        Kind::Playlist => tracks::HEAD_SPACE,
    };
    let rows = tracks.len();
    let foot = 18.0 + 16.0 + 18.0;
    let total = table_top + head + rows.max(1) as f32 * tracks::ROW + foot + 104.0;
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
                Some(HeaderAction::Favourite(kind, id)) => app.toggle_favourite(kind, id),
                None => {}
            }

            let columns = match listing.columns {
                Kind::Album { with_artist } => Columns::album(left, width, with_artist),
                Kind::Playlist => Columns::new(left, width, true),
            };
            let table = content.top() + table_top;
            if head > 0.0 {
                tracks::head(ui, &theme, &columns, table, left, width, None);
            }
            let rows_top = table + head;
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
            for position in tracks::visible(viewport, table_top + head, rows) {
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
                    favourite: app.is_favourite(FavouriteKind::Song, &track.id),
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
                    RowAction::Favourite => {
                        let id = track.id.clone();
                        app.toggle_favourite(FavouriteKind::Song, id);
                    }
                    RowAction::Enqueue(how) => {
                        let track = track.clone();
                        app.enqueue(vec![track], how, ui.input(|i| i.time));
                    }
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
fn hero(
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
            Rect::from_min_size(content.min, vec2(content.width(), 680.0)),
            colors,
        );
    }
    let size = art_size(pad);
    let left = content.left() + pad;
    let art = Rect::from_min_size(pos2(left, content.top() + ART_TOP), vec2(size, size));
    ui.painter().add(
        egui::Shadow {
            offset: [0, 18],
            blur: 48,
            spread: 0,
            color: Color32::from_black_alpha(110),
        }
        .as_shape(art.shrink(6.0), 10.0),
    );
    artwork(ui, header.and_then(|h| h.art.clone()), art, 10.0, theme);
    header_text(ui, theme, header?, art, content.right() - pad)
}

/// Three radial blobs of the cover's colours behind the header, fading into
/// `base`: colour 1 top left behind the artwork at 92 %, colour 2 top centre at
/// 72 %, colour 3 on the right at 62 %.
pub(super) fn cover_wash(ui: &Ui, theme: &Theme, rect: Rect, [c1, c2, c3]: [Color32; 3]) {
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

/// The text beside the artwork, centred on it: title, the artist with their
/// photo, details, notes, Play and Shuffle.
fn header_text(
    ui: &mut Ui,
    theme: &Theme,
    header: &Header,
    art: Rect,
    right: f32,
) -> Option<HeaderAction> {
    let x = art.right() + 32.0;
    let width = (right - x).max(0.0);
    let room = width
        - if header.favourite.is_some() {
            48.0
        } else {
            0.0
        };
    let title = text(
        ui,
        &header.title,
        font(40.0, Weight::Bold),
        theme.text,
        room,
    );
    let by = header.by.as_ref().map(|by| {
        text(
            ui,
            by,
            font(16.0, Weight::Regular),
            theme.text,
            width - 34.0,
        )
    });
    let description = header
        .description
        .as_ref()
        .map(|d| notes(ui, d, theme, width.min(460.0)));
    let (meta_height, by_height, button_height) = (18.0, 28.0, 36.0);

    let gap = 12.0;
    let mut height = title.size().y + gap + meta_height + 18.0 + button_height;
    if by.is_some() {
        height += by_height + gap;
    }
    if let Some(d) = &description {
        height += d.size().y + gap;
    }
    let mut y = art.center().y - height / 2.0;

    let h = title.size().y;
    let title_end = x + title.size().x;
    paint_text(ui, title, x, y + h / 2.0, egui::Align::Min, theme.text);
    let mut action = None;
    if let Some((kind, id, on)) = &header.favourite {
        let star =
            Rect::from_center_size(pos2(title_end + 14.0 + 16.0, y + h / 2.0), vec2(32.0, 32.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(star));
        let (icon, color, hint) = if *on {
            (icons::STAR_FILL, theme.red, "Remove from Favourites")
        } else {
            (icons::STAR, theme.subtext0, "Favourite")
        };
        if super::icon_button(&mut child, theme, icon, 32.0, 18.0, color, true)
            .on_hover_text(hint)
            .clicked()
        {
            action = Some(HeaderAction::Favourite(*kind, id.clone()));
        }
    }
    y += h + gap;
    if let Some(by) = by {
        let cy = y + by_height / 2.0;
        let mut tx = x;
        if header.by_art.is_some() {
            let avatar = Rect::from_center_size(pos2(x + 13.0, cy), vec2(26.0, 26.0));
            artwork(ui, header.by_art.clone(), avatar, 13.0, theme);
            tx = avatar.right() + 9.0;
        }
        let size = by.size();
        let mut color = theme.text;
        if let Some(id) = &header.by_link {
            let link = Rect::from_min_max(
                pos2(x, cy - by_height / 2.0),
                pos2(tx + size.x, cy + by_height / 2.0),
            );
            let hit = ui.interact(link, egui::Id::unique("header-by"), Sense::click());
            if hit.hovered() {
                color = mix(theme.text, theme.accent, 0.35);
                ui.painter().hline(
                    tx..=tx + size.x,
                    cy + size.y / 2.0,
                    egui::Stroke::new(1.0, color),
                );
            }
            if hit
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                action = Some(HeaderAction::OpenArtist(id.clone()));
            }
        }
        paint_text(ui, by, tx, cy, egui::Align::Min, color);
        y += by_height + gap;
    }

    // The details, each with its icon.
    let meta_font = font(11.5, Weight::SemiBold);
    let mut mx = x;
    let my = y + meta_height / 2.0;
    for (icon, label) in &header.meta {
        if let Some(icon) = icon {
            paint_icon(ui, *icon, pos2(mx + 6.0, my), 12.0, theme.subtext0);
            mx += 12.0 + 6.0;
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
        mx += w + 16.0;
    }
    y += meta_height + gap;

    if let Some(d) = description {
        let h = d.size().y;
        ui.painter().galley(pos2(x, y), d, theme.subtext0);
        y += h + gap;
    }
    y += 6.0;

    page_buttons(ui, theme, pos2(x, y), header.kind)
        .map(|shuffle| HeaderAction::Play { shuffle })
        .or(action)
}

/// The editorial notes: three lines at most, then an ellipsis.
fn notes(ui: &Ui, s: &str, theme: &Theme, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::single_section(
        s.to_owned(),
        egui::TextFormat {
            font_id: font(12.5, Weight::Regular),
            color: theme.subtext0,
            line_height: Some(18.0),
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping {
        max_width: width,
        max_rows: 3,
        break_anywhere: false,
        overflow_character: Some('…'),
    };
    ui.painter().layout_job(job)
}
