//! Album and playlist pages, laid out as Cider does: a large cover with the
//! title, artist, details and notes beside it over the cover's colours, then
//! the tracks.

use std::sync::Arc;
use std::time::Duration;

use egui::{Color32, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{Album, Collection, FavouriteKind, ItemId, MusicVideo, Track};

use crate::app::{App, Loaded, Page, TrackChoice};
use crate::color::mix;
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::grid::{CARD_TEXT, CardClick, SHELF_HEAD, ShelfCard, cover_shelf, video_shelf};
use super::tracks::{self, Columns, RowAction, RowState};
use super::{
    Blob, PageButton, artwork, page_buttons, paint_icon, paint_text, quality_label, text, wash,
};

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
    /// What an Add button adds to the library, when it isn't there yet.
    add: Option<Collection>,
    /// The cover's animated version, when there is one.
    motion: Option<String>,
}

/// What a click in the header asked for.
enum HeaderAction {
    Play {
        shuffle: bool,
    },
    OpenArtist(ItemId),
    Favourite(FavouriteKind, ItemId),
    /// The header's `add`.
    AddToLibrary,
    /// The notes, cut to three lines, in full.
    ShowNotes,
}

const ART_TOP: f32 = 60.0;

/// An album's notes in full, in a window, once MORE was clicked.
pub fn notes_window(app: &mut App, ctx: &egui::Context) {
    let Some((title, notes)) = app.notes.clone() else {
        return;
    };
    let theme = app.theme.clone();
    let frame = egui::Frame::new()
        .fill(theme.mantle)
        .stroke(egui::Stroke::new(1.0, theme.line()))
        .corner_radius(16)
        .inner_margin(24);
    let backdrop = if theme.light {
        Color32::from_black_alpha(72)
    } else {
        Color32::from_black_alpha(140)
    };
    let width = 520.0;
    let response = egui::Modal::new(egui::Id::unique("notes-window"))
        .frame(frame)
        .backdrop_color(backdrop)
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let g = text(
                ui,
                &title,
                font(18.0, Weight::Bold),
                theme.text,
                width - 40.0,
            );
            let (head, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
            paint_text(
                ui,
                g,
                head.left(),
                head.center().y,
                egui::Align::Min,
                theme.text,
            );
            let close = Rect::from_center_size(
                pos2(head.right() - 12.0, head.center().y),
                vec2(28.0, 28.0),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(close));
            if super::icon_button(
                &mut child,
                &theme,
                icons::X,
                28.0,
                14.0,
                theme.muted(),
                true,
            )
            .on_hover_text("Close")
            .clicked()
            {
                app.notes = None;
            }
            ui.add_space(12.0);
            egui::ScrollArea::vertical()
                .id_salt("notes-window-text")
                .max_height(ctx.content_rect().height() * 0.6)
                .show(ui, |ui| {
                    let mut job = egui::text::LayoutJob::single_section(
                        notes,
                        egui::TextFormat {
                            font_id: font(13.5, Weight::Regular),
                            color: theme.subtext0,
                            line_height: Some(21.0),
                            ..Default::default()
                        },
                    );
                    job.wrap.max_width = width;
                    let galley = ui.painter().layout_job(job);
                    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                    ui.painter().galley(rect.min, galley, theme.subtext0);
                });
        });
    if response.should_close() {
        app.notes = None;
    }
}

/// Where the header's cover goes.
fn art_rect(content: Rect, pad: f32) -> Rect {
    let size = art_size(pad);
    Rect::from_min_size(
        pos2(content.left() + pad, content.top() + ART_TOP),
        vec2(size, size),
    )
}

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
    /// The user's own playlist shown, whose songs can be taken out.
    playlist: Option<ItemId>,
    /// A playlist: it can be searched.
    searchable: bool,
    /// What an album's page shows under its tracks, as Cider does.
    album: Option<AlbumFoot>,
}

/// Under an album's tracks: whether Apple mastered it, its date, count and
/// copyright, then its other editions, its songs' music videos and the
/// artist's other releases.
struct AlbumFoot {
    /// A library album with only some of its songs there: its catalog
    /// version, which Show Full Album opens.
    full_album: Option<ItemId>,
    mastered: bool,
    lines: Vec<String>,
    other_versions: Vec<Album>,
    videos: Vec<MusicVideo>,
    /// The artist, and what else they put out, newest first.
    more_by: Option<(String, ItemId, Vec<Album>)>,
}

/// How tall the album's foot is.
fn foot_height(foot: &AlbumFoot) -> f32 {
    let mut h =
        24.0 + if foot.full_album.is_some() {
            FULL_ALBUM
        } else {
            0.0
        } + if foot.mastered { 30.0 } else { 0.0 }
            + foot.lines.len() as f32 * 18.0;
    if !foot.other_versions.is_empty() {
        h += shelf_height(MORE_BY_CARD);
    }
    if !foot.videos.is_empty() {
        h += shelf_height(VIDEO_CARD * 9.0 / 16.0);
    }
    if foot.more_by.is_some() {
        h += shelf_height(MORE_BY_CARD);
    }
    h
}

/// A shelf under the album's details, with the gap above it.
fn shelf_height(card: f32) -> f32 {
    40.0 + SHELF_HEAD + card + CARD_TEXT
}

/// The music videos' stills, wide.
const VIDEO_CARD: f32 = 240.0;

/// The row of Show Full Album.
const FULL_ALBUM: f32 = 40.0;

/// The covers of the artist's other releases.
const MORE_BY_CARD: f32 = 190.0;

/// The row above a playlist's songs that holds its search field.
const SEARCH_ROW: f32 = 46.0;

pub fn album(app: &mut App, ui: &mut Ui, pad: f32, id: &ItemId) -> f32 {
    // Whole from its copies on every service the library has it on.
    let whole = app.album_tracks(id);
    let (header, tracks, loading, error) = match app.albums.get(id) {
        Some(Loaded::Ready((album, _))) => {
            let tracks = whole.clone().unwrap_or_default();
            (
                Some(Header {
                    add: app
                        .can_add_album(&album.id)
                        .then(|| Collection::Album(album.clone())),
                    motion: album.motion.clone(),
                    ..album_header(
                        album,
                        &tracks,
                        app.is_favourite(FavouriteKind::Album, &App::album_favourite_id(album)),
                    )
                }),
                tracks,
                false,
                None,
            )
        }
        Some(Loaded::Failed(e)) => (None, Arc::default(), false, Some(e.clone())),
        _ => (None, Arc::default(), true, None),
    };
    // A compilation names each track's artist; an album by one artist need not.
    let with_artist = match (app.albums.get(id), &whole) {
        (Some(Loaded::Ready((album, _))), Some(tracks)) => {
            tracks.iter().any(|t| t.artists != album.artists)
        }
        _ => false,
    };
    let listing = Listing {
        columns: Kind::Album { with_artist },
        track_numbers: true,
        list: "album",
        loading,
        error,
        playlist: None,
        searchable: false,
        album: album_foot(app, id),
    };
    page(app, ui, pad, header, tracks, listing)
}

/// The foot of an album's page, once the album has come: its details, then
/// the artist's other releases once their page has.
fn album_foot(app: &mut App, id: &ItemId) -> Option<AlbumFoot> {
    let Some(Loaded::Ready((album, _))) = app.albums.get(id) else {
        return None;
    };
    let album = album.clone();
    let tracks = app.album_tracks(id).unwrap_or_default();
    let mut lines = Vec::new();
    if let Some(date) = album.released.as_deref().and_then(long_date) {
        lines.push(date);
    }
    let count = tracks.len();
    let mut count_line = format!("{count} track{}", if count == 1 { "" } else { "s" });
    if let Some(length) = total_length(&tracks) {
        let minutes = (length.as_secs() + 30) / 60;
        count_line += &format!(", {minutes} minute{}", if minutes == 1 { "" } else { "s" });
    }
    lines.push(count_line);
    if let Some(copyright) = &album.copyright {
        lines.push(copyright.clone());
    }
    // The lists under the songs are the catalog album's.
    let catalog = album
        .catalog_id
        .clone()
        .or_else(|| (!album.id.id.starts_with("l.")).then(|| album.id.clone()));
    let extras = catalog.as_ref().and_then(|c| app.album_extras(c));
    let (other_versions, videos) = match (&extras, &catalog) {
        (Some(extras), Some(catalog)) => (
            extras
                .other_versions
                .iter()
                .filter(|a| &a.id != catalog && a.id != album.id)
                .cloned()
                .collect(),
            extras.videos.clone(),
        ),
        _ => (Vec::new(), Vec::new()),
    };
    let more_by = album.artist_id.clone().and_then(|artist| {
        let detail = app.artist_detail(&artist)?;
        let mine = |a: &Album| {
            a.id == album.id || album.catalog_id.as_ref() == Some(&a.id) || a.title == album.title
        };
        let mut others: Vec<Album> = Vec::new();
        for a in detail
            .latest
            .iter()
            .chain(&detail.albums)
            .chain(&detail.singles)
        {
            if !mine(a) && !others.iter().any(|o| o.id == a.id) {
                others.push(a.clone());
            }
        }
        others.sort_by(|a, b| b.released.cmp(&a.released));
        others.truncate(20);
        (!others.is_empty()).then(|| (detail.artist.name.clone(), artist, others))
    });
    let full_album = album
        .catalog_id
        .clone()
        .filter(|_| album.id.id.starts_with("l."))
        .filter(|_| album.track_count.is_some_and(|n| n as usize > tracks.len()));
    Some(AlbumFoot {
        full_album,
        mastered: album.mastered,
        lines,
        other_versions,
        videos,
        more_by,
    })
}

/// `2026-10-02` → `2 October 2026`, as Cider writes it under the tracks.
fn long_date(iso: &str) -> Option<String> {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let mut parts = iso.splitn(3, '-');
    let year = parts.next()?;
    let month: usize = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.get(..2)?.parse().ok()?;
    Some(format!(
        "{day} {} {year}",
        MONTHS.get(month.checked_sub(1)?)?
    ))
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
    // The service by its logo alone.
    meta.push((Some(icons::service(playlist.id.provider)), String::new()));
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
        add: Some(Collection::Playlist(playlist.clone())).filter(|p| app.can_add_to_library(p)),
        motion: None,
    };
    let listing = Listing {
        columns: Kind::Playlist,
        track_numbers: false,
        list: "playlist",
        loading: tracks.is_empty() && !done && error.is_none(),
        error,
        playlist: playlist.editable.then(|| playlist.id.clone()),
        searchable: true,
        album: None,
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
    if let Some(((label, _), provider)) = tracks
        .iter()
        .find_map(|t| quality_label(t).map(|q| (q, t.id.provider)))
    {
        meta.push((Some(icons::service(provider)), label));
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
        add: None,
        motion: None,
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
    // What the search finds, numbered as in the playlist: Play, Shuffle and
    // the rows play what is shown.
    let query = app.page_search.clone();
    let searching = listing.searchable && !query.trim().is_empty();
    let (tracks, numbers) = if listing.searchable {
        app.search_in(tracks, &query)
    } else {
        (tracks, None)
    };
    let search_row = if listing.searchable { SEARCH_ROW } else { 0.0 };
    let table_top = ART_TOP + art_size(pad) + 34.0 + search_row;
    // Albums have no column titles, as in Cider.
    let head = match listing.columns {
        Kind::Album { .. } => 0.0,
        Kind::Playlist => tracks::HEAD_SPACE,
    };
    let rows = tracks.len();
    let foot = match &listing.album {
        Some(album) => foot_height(album),
        None => 18.0 + 16.0 + 18.0,
    };
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
            let action = hero(ui, &theme, content, pad, header.as_ref(), colors);
            // The cover moves, where Apple made it move.
            let art = art_rect(content, pad);
            let motion = header.as_ref().and_then(|h| h.motion.as_deref());
            super::motion_over(app, ui, "album", motion, art, 10.0, art.width());
            match action {
                Some(HeaderAction::Play { shuffle }) => {
                    app.play_shuffled_or_not(tracks.to_vec(), shuffle, ui.input(|i| i.time));
                }
                Some(HeaderAction::OpenArtist(id)) => app.go(Page::Artist(id)),
                Some(HeaderAction::Favourite(kind, id)) => app.toggle_favourite(kind, id),
                Some(HeaderAction::ShowNotes) => {
                    if let Some(h) = &header
                        && let Some(notes) = &h.description
                    {
                        app.notes = Some((h.title.clone(), notes.clone()));
                    }
                }
                Some(HeaderAction::AddToLibrary) => {
                    if let Some(item) = header.as_ref().and_then(|h| h.add.as_ref()) {
                        app.add_collection_to_library(item, ui.input(|i| i.time));
                    }
                }
                None => {}
            }

            let columns = match listing.columns {
                Kind::Album { with_artist } => Columns::album(left, width, with_artist),
                Kind::Playlist => Columns::new(left, width, true),
            };
            let table = content.top() + table_top;
            if listing.searchable {
                let search = super::page_search(
                    app,
                    ui,
                    &theme,
                    left + width,
                    table - search_row + 4.0,
                    "Search in Playlist",
                );
                if search.changed {
                    app.detail_selected = None;
                }
            }
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
            if searching && tracks.is_empty() {
                let g = text(
                    ui,
                    &format!("No songs match “{}”.", query.trim()),
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
            for position in tracks::visible(viewport, table_top + head, rows) {
                let track = &tracks[position];
                let rect = Rect::from_min_size(
                    pos2(left, rows_top + position as f32 * tracks::ROW),
                    vec2(width, tracks::ROW),
                );
                let now = app.is_current(track);
                let number = if listing.track_numbers {
                    track.track_number
                } else if let Some(numbers) = &numbers {
                    numbers.get(position).copied()
                } else {
                    Some(position as u32 + 1)
                };
                let state = RowState {
                    number,
                    selected: app.detail_selected == Some(position),
                    now,
                    playing: now && app.is_playing(),
                    favourite: app.is_favourite(FavouriteKind::Song, &track.id),
                    in_library: app.track_in_library(track),
                    in_playlist: listing.playlist.is_some(),
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
                    RowAction::Menu(TrackChoice::RemoveFromPlaylist) => {
                        if let Some(playlist) = &listing.playlist {
                            app.remove_from_playlist(playlist, track);
                        }
                    }
                    RowAction::Menu(choice) => {
                        let track = track.clone();
                        app.track_choice(track, choice, ui.input(|i| i.time));
                    }
                }
            }
            let foot_top = rows_top + rows as f32 * tracks::ROW;
            if let Some(foot) = &listing.album {
                album_foot_ui(app, ui, &theme, foot, left, width, foot_top);
            } else if let Some(length) = total_length(&tracks) {
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
    let art = art_rect(content, pad);
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
        let area = Rect::from_min_size(pos2(x, y), d.size());
        // Cut short, they end in MORE, and open whole on a click.
        let more = d.elided.then(|| {
            let end = d.rows.last().map_or(0.0, |r| r.rect().right());
            pos2(x + end + 6.0, y + h - 9.0)
        });
        ui.painter().galley(pos2(x, y), d, theme.subtext0);
        if let Some(at) = more {
            let response = ui.interact(
                area.union(Rect::from_min_size(
                    at - vec2(0.0, 9.0),
                    vec2(MORE_WIDTH, 18.0),
                )),
                egui::Id::unique("header-notes"),
                Sense::click(),
            );
            let color = if response.hovered() {
                theme.text
            } else {
                theme.subtext0
            };
            let g = text(ui, "MORE", font(11.5, Weight::Bold), color, MORE_WIDTH);
            paint_text(ui, g, at.x, at.y, egui::Align::Min, color);
            if response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                action = Some(HeaderAction::ShowNotes);
            }
        }
        y += h + gap;
    }
    y += 6.0;

    match page_buttons(ui, theme, pos2(x, y), header.kind, header.add.is_some()) {
        Some(PageButton::Play) => Some(HeaderAction::Play { shuffle: false }),
        Some(PageButton::Shuffle) => Some(HeaderAction::Play { shuffle: true }),
        Some(PageButton::Add) => Some(HeaderAction::AddToLibrary),
        None => action,
    }
}

/// The album's details under its tracks, then More by its artist.
#[allow(clippy::too_many_arguments)]
fn album_foot_ui(
    app: &mut App,
    ui: &mut Ui,
    theme: &Theme,
    foot: &AlbumFoot,
    left: f32,
    width: f32,
    top: f32,
) {
    let muted = theme.muted();
    let mut y = top + 24.0;
    // Right under the last track, in the accent, as Apple Music's app has it.
    if let Some(catalog) = &foot.full_album {
        let g = text(
            ui,
            "Show Full Album",
            font(13.5, Weight::SemiBold),
            theme.accent,
            width,
        );
        let link = Rect::from_min_size(pos2(left, y - g.size().y / 2.0), g.size());
        let hit = ui.interact(
            link,
            egui::Id::unique(("full-album", &catalog.id)),
            Sense::click(),
        );
        if hit.hovered() {
            ui.painter().hline(
                link.x_range(),
                link.bottom(),
                egui::Stroke::new(1.0, theme.accent),
            );
        }
        paint_text(ui, g, left, y, egui::Align::Min, theme.accent);
        if hit
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            app.go(Page::Album(catalog.clone()));
        }
        y += FULL_ALBUM;
    }
    if foot.mastered {
        let g = text(
            ui,
            "Apple Digital Master",
            font(12.5, Weight::Regular),
            muted,
            width,
        );
        paint_text(ui, g, left, y, egui::Align::Min, muted);
        y += 30.0;
    }
    for line in &foot.lines {
        let g = text(ui, line, font(12.5, Weight::Regular), muted, width);
        paint_text(ui, g, left, y, egui::Align::Min, muted);
        y += 18.0;
    }
    let now = ui.input(|i| i.time);
    if !foot.other_versions.is_empty() {
        let at = pos2(left, y + 40.0 - 18.0);
        let versions = &foot.other_versions;
        let cards: Vec<ShelfCard> = versions
            .iter()
            .map(|a| ShelfCard {
                title: a.title.clone(),
                sub: match a.track_count {
                    Some(n) => format!("{} · {n} Songs", a.artists.join(", ")),
                    None => a.artists.join(", "),
                },
                url: a.artwork.as_ref().map(|art| art.url(400)),
                explicit: a.explicit,
                addable: app.can_add_album(&a.id),
            })
            .collect();
        let id = egui::Id::unique("other-versions");
        let title = "Other Versions";
        match cover_shelf(ui, theme, at, width, title, &cards, MORE_BY_CARD, id) {
            Some((i, CardClick::Open)) => app.go(Page::Album(versions[i].id.clone())),
            Some((i, CardClick::Enqueue(how))) => {
                app.enqueue_album(versions[i].id.clone(), how, now);
            }
            Some((i, CardClick::AddToLibrary)) => {
                app.add_collection_to_library(&Collection::Album(versions[i].clone()), now);
            }
            None => {}
        }
        y += shelf_height(MORE_BY_CARD);
    }
    if !foot.videos.is_empty() {
        let at = pos2(left, y + 40.0 - 18.0);
        let cards: Vec<ShelfCard> = foot
            .videos
            .iter()
            .map(|v| ShelfCard {
                title: v.title.clone(),
                sub: v.artists.join(", "),
                url: v.thumbnail.clone(),
                explicit: v.explicit,
                addable: false,
            })
            .collect();
        let id = egui::Id::unique("music-videos");
        // There is no video player here: they play on Apple Music's website.
        if let Some(i) = video_shelf(ui, theme, at, width, "Music Videos", &cards, VIDEO_CARD, id)
            && let Some(url) = &foot.videos[i].url
        {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
        }
        y += shelf_height(VIDEO_CARD * 9.0 / 16.0);
    }
    let Some((name, artist, albums)) = &foot.more_by else {
        return;
    };
    let at = pos2(left, y + 40.0 - 18.0);
    let title = format!("More by {name}");
    let cards: Vec<ShelfCard> = albums
        .iter()
        .map(|a| ShelfCard {
            title: a.title.clone(),
            sub: a.artists.join(", "),
            url: a.artwork.as_ref().map(|art| art.url(400)),
            explicit: a.explicit,
            addable: app.can_add_album(&a.id),
        })
        .collect();
    let id = egui::Id::unique(("more-by", &artist.id));
    // The title, and a chevron after it, lead to the artist.
    let g = text(ui, &title, font(18.0, Weight::SemiBold), theme.text, 400.0);
    let chevron = pos2(at.x + g.size().x + 12.0, at.y + 12.0);
    let link = Rect::from_min_max(pos2(at.x, at.y - 3.0), pos2(chevron.x + 10.0, at.y + 25.0));
    let hit = ui.interact(link, id.with("title"), Sense::click());
    let color = if hit.hovered() { theme.text } else { muted };
    paint_icon(ui, icons::RIGHT, chevron, 16.0, color);
    if hit
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        app.go(Page::Artist(artist.clone()));
    }
    match cover_shelf(ui, theme, at, width, &title, &cards, MORE_BY_CARD, id) {
        Some((i, CardClick::Open)) => app.go(Page::Album(albums[i].id.clone())),
        Some((i, CardClick::Enqueue(how))) => app.enqueue_album(albums[i].id.clone(), how, now),
        Some((i, CardClick::AddToLibrary)) => {
            app.add_collection_to_library(&Collection::Album(albums[i].clone()), now);
        }
        None => {}
    }
}

/// The room MORE takes after notes cut short.
const MORE_WIDTH: f32 = 40.0;

/// The editorial notes: three lines at most, then an ellipsis; cut short,
/// narrower by MORE, which follows them.
fn notes(ui: &Ui, s: &str, theme: &Theme, width: f32) -> std::sync::Arc<egui::Galley> {
    let full = notes_at(ui, s, theme, width);
    if full.elided {
        notes_at(ui, s, theme, width - MORE_WIDTH - 6.0)
    } else {
        full
    }
}

fn notes_at(ui: &Ui, s: &str, theme: &Theme, width: f32) -> std::sync::Arc<egui::Galley> {
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
