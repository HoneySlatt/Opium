//! Album and playlist pages, as the egui window's `ui/detail.rs` lays them
//! out in Spotify's look: a band of the cover's colour with the cover, what
//! it is, the title and the details; Play, Shuffle and the other buttons
//! under it; then the tracks. The Songs page's banner is drawn here too.
//! Cider's hero and M3E's header come with their themes.

use std::sync::Arc;
use std::time::Duration;

use gpui::{
    ClickEvent, Context, Div, ElementId, InteractiveElement, ParentElement, Rgba,
    StatefulInteractiveElement, Styled, Window, div,
};
use music_core::model::{Album, FavouriteKind, ItemId, MusicVideo, Track};

use crate::app::{Loaded, Music, Page};
use crate::color::{BLACK, WHITE, black_alpha, mix, white_alpha};
use crate::fonts::{Weight, line_height};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::grid::{CARD_TEXT, CardClick, SHELF_HEAD, ShelfCard, cover_shelf, video_shelf};
use super::tracks::{self, Columns, PageDownload, RowAction, RowState};
use super::{
    Align, artwork, disc, icon_button, label_cut, loved_tile, place, shadow, text_at, text_width,
    vertical_gradient, z,
};

/// What the header shows.
pub struct Header {
    pub kind: &'static str,
    pub title: String,
    pub by: Option<String>,
    /// Where the `by` line leads.
    pub by_link: Option<ItemId>,
    /// The artist's photo, round before their name.
    pub by_art: Option<String>,
    pub meta: Vec<(Option<Icon>, String)>,
    pub description: Option<String>,
    pub art: Option<String>,
    /// The circle after Shuffle, when it can be a favourite: whether it is
    /// lit. (Changing it comes with the favourites' buttons.)
    pub favourite: Option<bool>,
    /// An Add button, when it isn't in the library yet. (Adding comes with
    /// the library's buttons.)
    pub add: bool,
    /// How long it all lasts, which Spotify's banner says.
    pub length: Option<Duration>,
    /// The library's songs: Spotify's Liked Songs tile for a cover.
    pub loved: bool,
    /// Its download button, when it has songs to download; set by the page.
    /// (Downloading from it comes with the downloads' buttons.)
    pub download: Option<PageDownload>,
}

/// The band behind Spotify's Liked Songs.
pub const SONGS_BAND: Rgba = crate::color::rgb(0x5038a0);

/// The banner's height, down to under the cover, in Spotify's layout.
pub const BANNER: f32 = 52.0 + 24.0 + 232.0 + 24.0;
/// The row of buttons under it.
pub const ACTIONS: f32 = 104.0;

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
    loading: bool,
    error: Option<String>,
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

/// An album's page, `width` by `height` points in the main card.
pub fn album(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    width: f32,
    height: f32,
    pad: f32,
    id: &ItemId,
) -> Div {
    let whole = app.album_tracks(id);
    let (header, tracks, loading, error) = match app.albums.get(id) {
        Some(Loaded::Ready((album, _))) => {
            let tracks = whole.clone().unwrap_or_default();
            (
                Some(Header {
                    add: app.can_add_album(&album.id),
                    length: total_length(&tracks),
                    ..album_header(
                        album,
                        &tracks,
                        app.is_favourite(FavouriteKind::Album, &Music::album_favourite_id(album)),
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
        loading,
        error,
        searchable: false,
        album: album_foot(app, id),
    };
    page(app, window, cx, width, height, pad, header, tracks, listing)
}

/// The foot of an album's page, once the album has come: its details, then
/// the artist's other releases once their page has.
fn album_foot(app: &mut Music, id: &ItemId) -> Option<AlbumFoot> {
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
    let catalog = Music::album_catalog_id(&album);
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

/// A playlist's page, `width` by `height` points in the main card.
pub fn playlist(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    width: f32,
    height: f32,
    pad: f32,
    id: &ItemId,
) -> Div {
    let Some(playlist) = app.playlist(id).cloned() else {
        return div();
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
        favourite: Music::playlist_favourite_id(&playlist)
            .map(|id| app.is_favourite(FavouriteKind::Playlist, &id)),
        add: app.can_add_playlist(&playlist.id),
        length: total_length(&tracks),
        loved: false,
        download: None,
    };
    let listing = Listing {
        columns: Kind::Playlist,
        track_numbers: false,
        loading: tracks.is_empty() && !done && error.is_none(),
        error,
        searchable: true,
        album: None,
    };
    page(
        app,
        window,
        cx,
        width,
        height,
        pad,
        Some(header),
        tracks,
        listing,
    )
}

/// The Songs page's banner, as Spotify's Liked Songs: its tile, Playlist,
/// the title, the user and the count; Play and Shuffle under it. In a box
/// `width` points wide, its top the page's.
#[allow(clippy::too_many_arguments)]
pub fn songs_banner(
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    width: f32,
    pad: f32,
    count: usize,
    shuffle: bool,
    here: bool,
) -> Div {
    let user = std::env::var("USER").unwrap_or_default();
    let mut chars = user.chars();
    let user: String = chars
        .next()
        .map(|f| f.to_uppercase().chain(chars).collect())
        .unwrap_or_default();
    let header = Header {
        kind: "Playlist",
        title: "Songs".into(),
        by: Some(user),
        by_link: None,
        by_art: None,
        meta: vec![(
            Some(icons::NOTE),
            format!("{count} TRACK{}", if count == 1 { "" } else { "S" }),
        )],
        description: None,
        art: None,
        favourite: None,
        add: false,
        length: None,
        loved: true,
        download: None,
    };
    banner_header(
        window,
        cx,
        theme,
        width,
        pad,
        Some(&header),
        SONGS_BAND,
        shuffle,
        here,
    )
}

/// Spotify's header: a band of the cover's colour darkening downwards, the
/// cover at its bottom left, beside it what it is, the title in large type
/// and a line of details; under the band, a large round Play and the other
/// buttons on the colour fading into the page. Without a header yet, the
/// band and the cover's place.
#[allow(clippy::too_many_arguments)]
fn banner_header(
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    width: f32,
    pad: f32,
    header: Option<&Header>,
    color: Rgba,
    shuffle: bool,
    here: bool,
) -> Div {
    let shade = |c: Rgba, p: f32| mix(BLACK, c, p);
    let mut out = div()
        .absolute()
        .left_0()
        .top_0()
        .w(z(width))
        .h(z(BANNER + ACTIONS));
    // The band, rounded at the card's top as egui's corners painted over it.
    out = out.child(place(0.0, 0.0, width, BANNER).child(vertical_gradient(
        Some(width),
        BANNER,
        color,
        shade(color, 0.5),
    )));
    out = out.child(place(0.0, BANNER, width, 232.0).child(vertical_gradient(
        Some(width),
        232.0,
        mix(shade(color, 0.4), theme.base, 0.8),
        theme.base,
    )));

    let (ax, ay, side) = (pad, BANNER - 24.0 - 232.0, 232.0);
    let art = place(ax, ay, side, side)
        .rounded(z(4.0))
        .shadow(shadow(8.0, 24.0, black_alpha(130)));
    out = out.child(if header.is_some_and(|h| h.loved) {
        art.child(loved_tile(window, side))
    } else {
        art.child(artwork(
            window,
            cx,
            header.and_then(|h| h.art.as_deref()),
            side,
            4.0,
            theme,
        ))
    });
    let Some(header) = header else {
        return out;
    };

    // What it is, the title as large as it fits, the details.
    let x = ax + side + 24.0;
    let room = (width - pad - x).max(0.0);
    let title_size = [96.0, 72.0, 56.0, 40.0, 32.0]
        .into_iter()
        .find(|&size| text_width(window, &header.title, size, Weight::Bold) <= room)
        .unwrap_or(32.0);
    let details = banner_details(header);
    let bottom = ay + side;
    let line_y = bottom - 10.0;
    // The artist (or the playlist's maker) first, linked, with their photo.
    let mut lx = x;
    if let Some(by) = &header.by {
        if let Some(by_art) = &header.by_art {
            out = out.child(place(lx, line_y - 12.0, 24.0, 24.0).child(artwork(
                window,
                cx,
                Some(by_art),
                24.0,
                12.0,
                theme,
            )));
            lx += 24.0 + 6.0;
        }
        let max = room - (lx - x);
        let w = text_width(window, by, 14.0, Weight::Bold).min(max);
        let name = label_cut(by.clone(), 14.0, Weight::Bold, WHITE, max);
        out = out.child(match &header.by_link {
            // The artist's page is still to come: it opens empty.
            Some(id) => {
                let id = id.clone();
                text_at(
                    lx,
                    line_y,
                    14.0,
                    Align::Min,
                    div()
                        .id("banner-by")
                        .group("banner-by")
                        .relative()
                        .cursor_pointer()
                        .child(name)
                        // egui's underline: a hairline along the galley's bottom.
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .top(z(line_height(14.0) - 0.5))
                                .h(z(1.0))
                                .invisible()
                                .bg(WHITE)
                                .group_hover("banner-by", |s| s.visible()),
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.go(Page::Artist(id.clone()));
                            cx.notify();
                        })),
                )
            }
            None => text_at(lx, line_y, 14.0, Align::Min, name),
        });
        lx += w;
    }
    if !details.is_empty() {
        let line = if header.by.is_some() {
            format!(" • {details}")
        } else {
            details
        };
        let soft = white_alpha(200);
        out = out.child(text_at(
            lx,
            line_y,
            14.0,
            Align::Min,
            label_cut(line, 14.0, Weight::Regular, soft, (x + room - lx).max(0.0)),
        ));
    }
    let mut y = line_y - 12.0 - 8.0;
    // A playlist's description; an album's notes stay behind "…", as
    // Spotify shows none.
    if let Some(d) = header
        .description
        .as_ref()
        .filter(|_| header.kind == "Playlist")
    {
        let soft = white_alpha(170);
        let gh = line_height(14.0);
        out = out.child(text_at(
            x,
            y - gh / 2.0,
            14.0,
            Align::Min,
            label_cut(
                d.lines().next().unwrap_or_default().to_owned(),
                14.0,
                Weight::Regular,
                soft,
                room,
            ),
        ));
        y -= gh + 8.0;
    }
    let h = line_height(title_size);
    out = out.child(text_at(
        x,
        y - h / 2.0,
        title_size,
        Align::Min,
        label_cut(header.title.clone(), title_size, Weight::Bold, WHITE, room),
    ));
    y -= h + 2.0;
    out = out.child(text_at(
        x,
        y - 8.0,
        14.0,
        Align::Min,
        label_cut(header.kind, 14.0, Weight::Medium, WHITE, room),
    ));

    // The buttons: Play, large and round in the accent, then the others.
    let mid = BANNER + ACTIONS / 2.0;
    let (px, py) = (pad + 28.0, mid);
    let lit = mix(theme.accent, WHITE, 0.85);
    // Play grows by two points and lightens under the pointer; it plays
    // the page, or pauses it.
    out = out.child(
        disc(px, py, 28.0, theme.accent)
            .id("banner-play")
            .cursor_pointer()
            .hover(move |s| s.bg(lit).left(z(px - 30.0)).top(z(py - 30.0)).size(z(60.0)))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                let tracks = super::page_tracks(this);
                this.play_page(tracks);
                this.acted(window, cx);
            }))
            .child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        (if here {
                            icons::PAUSE_FILL
                        } else {
                            icons::PLAY_FILL
                        })
                        .element(24.0, theme.on_accent()),
                    ),
            ),
    );
    // Where the next button goes.
    let mut bx = pad + 56.0 + 16.0;
    let button = |bx: &mut f32, id: &'static str, icon: Icon, size: f32, color: Rgba| {
        let at = place(*bx, mid - 20.0, 40.0, 40.0)
            .child(icon_button(id, theme, icon, 40.0, size, color, true));
        *bx += 40.0 + 8.0;
        at
    };
    // Shuffle is a switch, green with a dot while on, as Spotify's.
    // The first of the buttons after Play.
    let color = if shuffle {
        theme.accent
    } else {
        theme.subtext0
    };
    let shuffle_at = (bx + 20.0, mid + 17.0);
    out = out.child(
        place(bx, mid - 20.0, 40.0, 40.0).child(
            icon_button(
                "banner-shuffle",
                theme,
                icons::SHUFFLE,
                40.0,
                26.0,
                color,
                true,
            )
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.toggle_shuffle();
                this.acted(window, cx);
            })),
        ),
    );
    bx += 40.0 + 8.0;
    if shuffle {
        out = out.child(disc(shuffle_at.0, shuffle_at.1, 2.0, theme.accent));
    }
    // Favourite, Add, the download and the notes are drawn as egui's; what
    // they do comes with the favourites', the library's and the downloads'
    // buttons, and the notes' window.
    if let Some(on) = header.favourite {
        let (icon, color) = if on {
            (icons::CIRCLE_CHECK, theme.accent)
        } else {
            (icons::CIRCLE_PLUS, theme.subtext0)
        };
        out = out.child(button(&mut bx, "banner-favourite", icon, 28.0, color));
    }
    if header.add {
        out = out.child(button(
            &mut bx,
            "banner-add",
            icons::PLUS,
            26.0,
            theme.subtext0,
        ));
    }
    if let Some(state) = header.download {
        out = out.child(
            place(bx, mid - 20.0, 40.0, 40.0).child(tracks::download_button(theme, 40.0, state)),
        );
        bx += 40.0 + 8.0;
    }
    if header.description.is_some() {
        out = out.child(button(
            &mut bx,
            "banner-notes",
            icons::MORE,
            26.0,
            theme.subtext0,
        ));
    }
    out
}

/// The header's details as Spotify writes them: `2026 • 12 songs, 43 min
/// 7 sec`, then the quality's badge. No genre, as Spotify.
fn banner_details(header: &Header) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (icon, label) in &header.meta {
        if label.is_empty() || *icon == Some(icons::MIC) {
            continue;
        }
        let digits = label.starts_with(|c: char| c.is_ascii_digit());
        // A length of the details': said whole after the count instead.
        if *icon == Some(icons::CLOCK) && !label.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if digits && label.contains("TRACK") {
            let count = label
                .to_lowercase()
                .replace(" tracks", " songs")
                .replace(" track", " song");
            parts.push(match header.length {
                Some(length) => format!("{count}, {}", spoken_length(length)),
                None => count,
            });
        } else if digits {
            parts.push(label.clone());
        } else {
            // LOSSLESS, DOLBY ATMOS: each word capitalised.
            let mut after_letter = false;
            parts.push(
                label
                    .chars()
                    .map(|c| {
                        let out = if after_letter {
                            c.to_lowercase().next().unwrap_or(c)
                        } else {
                            c
                        };
                        after_letter = c.is_alphabetic();
                        out
                    })
                    .collect(),
            );
        }
    }
    parts.join(" • ")
}

/// `37 min 3 sec`, `1 hr 12 min`.
fn spoken_length(total: Duration) -> String {
    let secs = total.as_secs();
    let (hours, minutes, seconds) = (secs / 3600, secs / 60 % 60, secs % 60);
    if hours > 0 {
        format!("{hours} hr {minutes} min")
    } else {
        format!("{minutes} min {seconds} sec")
    }
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
    // The service, and the badge its quality earns: none for a lossy
    // stream, as in Apple Music.
    if let Some(track) = tracks.first() {
        let badge = tracks.iter().find_map(super::quality_badge);
        meta.push((
            Some(icons::service(track.id.provider)),
            badge.map(str::to_uppercase).unwrap_or_default(),
        ));
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
        favourite: Some(favourite),
        add: false,
        length: None,
        loved: false,
        download: None,
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

/// The page: the banner, the column titles and the tracks, only the rows in
/// sight laid out, the foot; once scrolled past the banner, Spotify's bar.
#[allow(clippy::too_many_arguments)]
fn page(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    width: f32,
    height: f32,
    pad: f32,
    header: Option<Header>,
    tracks: Arc<Vec<Track>>,
    listing: Listing,
) -> Div {
    let theme = app.theme.clone();
    let mut header = header;
    if let Some(h) = &mut header {
        h.download = app.page_download(&tracks);
    }
    // What the page's search finds comes with the text field: all is shown.
    let table_top = BANNER + ACTIONS;
    let head = tracks::HEAD_SPACE;
    let rows = tracks.len();
    let foot = match &listing.album {
        Some(album) => foot_height(album),
        None => 18.0 + 16.0 + 18.0,
    };
    let total = table_top + head + rows.max(1) as f32 * tracks::ROW + foot + 104.0;
    let colors = super::backdrop(app);
    let color = colors.map_or(theme.surface1, |[c, _, _]| c);
    let (shuffle_on, here) = (app.shown().shuffle, app.page_playing());
    // Each album and playlist keeps its own place.
    let shown = app.page.clone();
    let scroll = app.page_scroll(&shown);
    let offset = (-f32::from(scroll.offset().y) / super::zoom()).max(0.0);

    let left = pad;
    let list_width = width - 2.0 * pad;
    let mut content = div().relative().w(z(width)).h(z(total));
    content = content.child(banner_header(
        window,
        cx,
        &theme,
        width,
        pad,
        header.as_ref(),
        color,
        shuffle_on,
        here,
    ));
    let columns = match listing.columns {
        Kind::Album { with_artist } => Columns::album(0.0, list_width, with_artist),
        Kind::Playlist => Columns::new(0.0, list_width, true),
    };
    if listing.searchable {
        // In Spotify's layout, on the right of the buttons' row: its
        // magnifier, closed. Typing in it comes with the text field.
        let top = BANNER + (ACTIONS - 32.0) / 2.0;
        content = content.child(place(left + list_width - 32.0, top, 32.0, 32.0).child(
            icon_button(
                "page-search",
                &theme,
                icons::SEARCH,
                32.0,
                15.0,
                theme.subtext0,
                true,
            ),
        ));
    }
    content = content.child(
        place(left, table_top, list_width, tracks::HEAD)
            .child(tracks::head(&theme, &columns, 0.0, list_width, None, cx)),
    );
    let rows_top = table_top + head;
    if let Some(error) = &listing.error {
        content = content.child(text_at(
            left + 10.0,
            rows_top + 26.0,
            13.0,
            Align::Min,
            label_cut(
                error.clone(),
                13.0,
                Weight::Regular,
                theme.subtext0,
                list_width - 20.0,
            ),
        ));
    } else if listing.loading {
        content = content.child(tracks::skeleton(&theme, rows_top, left, list_width, 6));
    } else {
        for position in tracks::visible(offset, offset + height, rows_top, rows) {
            let track = &tracks[position];
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
                download: app.badge(&track.id),
                unplayable: app.unplayable(track),
                position,
            };
            content = content.child(
                div()
                    .absolute()
                    .left(z(left))
                    .top(z(rows_top + position as f32 * tracks::ROW))
                    .child(tracks::row(
                        window, cx, &theme, &columns, list_width, track, state,
                    )),
            );
        }
        let foot_top = rows_top + rows as f32 * tracks::ROW;
        if let Some(foot) = &listing.album {
            content = content.child(album_foot_ui(
                app,
                window,
                cx,
                &theme,
                foot,
                (left, list_width),
                foot_top,
                (offset, offset + height),
            ));
        } else if let Some(length) = total_length(&tracks) {
            let minutes = (length.as_secs() + 30) / 60;
            let foot_text = format!(
                "{rows} song{}, {minutes} minute{}",
                if rows == 1 { "" } else { "s" },
                if minutes == 1 { "" } else { "s" }
            );
            content = content.child(text_at(
                left + 10.0,
                foot_top + 26.0,
                12.0,
                Align::Min,
                label_cut(foot_text, 12.0, Weight::Regular, theme.muted(), list_width),
            ));
        }
    }

    let mut page = div().absolute().left_0().top_0().size_full().child(
        div()
            .id(ElementId::Name(format!("{shown:?}").into()))
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&scroll)
            .child(content),
    );
    if let Some(header) = &header {
        // Past the banner, Spotify's bar with its Play, and the column
        // titles kept under it.
        let bar = super::BANNER_BAR;
        let shown = ((offset - (BANNER - 2.0 * bar)) / 48.0).clamp(0.0, 1.0);
        if shown > 0.0 {
            let mut layer = super::banner_bar(cx, &theme, width, color, &header.title, shown, here);
            if offset > table_top - bar {
                // Spotify's: a shade lighter than the page, across the table.
                layer = layer.child(
                    place(left - 16.0, bar, list_width + 32.0, tracks::HEAD).bg(mix(
                        theme.surface0,
                        theme.base,
                        0.5,
                    )),
                );
                layer = layer.child(
                    place(left, bar, list_width, tracks::HEAD)
                        .child(tracks::head(&theme, &columns, 0.0, list_width, None, cx)),
                );
            }
            page = page.child(layer);
        }
    }
    page
}

/// Under an album's tracks, from `top`: Show Full Album, Apple's mastering,
/// the date, the count and the copyright; then its other versions, its
/// music videos and More by its artist, those in `seen` (the page's part in
/// sight) laid out.
#[allow(clippy::too_many_arguments)]
fn album_foot_ui(
    app: &mut Music,
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    foot: &AlbumFoot,
    (left, width): (f32, f32),
    top: f32,
    seen: (f32, f32),
) -> Div {
    let muted = theme.muted();
    let mut out = div().absolute().left_0().top_0().size_full();
    let mut y = top + 24.0;
    // Right under the last track, in the accent, as Apple Music's app has it.
    if let Some(catalog) = &foot.full_album {
        let catalog = catalog.clone();
        let accent = theme.accent;
        out = out.child(text_at(
            left,
            y,
            13.5,
            Align::Min,
            div()
                .id("full-album")
                .group("full-album")
                .relative()
                .cursor_pointer()
                .child(label_cut(
                    "Show Full Album",
                    13.5,
                    Weight::SemiBold,
                    accent,
                    width,
                ))
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(z(line_height(13.5) - 0.5))
                        .h(z(1.0))
                        .invisible()
                        .bg(accent)
                        .group_hover("full-album", |s| s.visible()),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.go(Page::Album(catalog.clone()));
                    cx.notify();
                })),
        ));
        y += FULL_ALBUM;
    }
    if foot.mastered {
        out = out.child(text_at(
            left,
            y,
            12.5,
            Align::Min,
            label_cut("Apple Digital Master", 12.5, Weight::Regular, muted, width),
        ));
        y += 30.0;
    }
    for line in &foot.lines {
        out = out.child(text_at(
            left,
            y,
            12.5,
            Align::Min,
            label_cut(line.clone(), 12.5, Weight::Regular, muted, width),
        ));
        y += 18.0;
    }
    // A shelf `card` points tall from `y` on, in sight.
    let in_sight = |y: f32, card: f32| y + shelf_height(card) > seen.0 && y < seen.1;
    if !foot.other_versions.is_empty() {
        if in_sight(y, MORE_BY_CARD) {
            let at = (left, y + 40.0 - 18.0);
            let versions = foot.other_versions.clone();
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
                })
                .collect();
            out = out.child(cover_shelf(
                app,
                window,
                cx,
                theme,
                at,
                width,
                "Other Versions",
                &cards,
                MORE_BY_CARD,
                "other-versions",
                move |app, i, click, _| match click {
                    CardClick::Open => app.go(Page::Album(versions[i].id.clone())),
                    CardClick::Enqueue => app.enqueue_album(versions[i].id.clone()),
                },
            ));
        }
        y += shelf_height(MORE_BY_CARD);
    }
    if !foot.videos.is_empty() {
        if in_sight(y, VIDEO_CARD * 9.0 / 16.0) {
            let at = (left, y + 40.0 - 18.0);
            let cards: Vec<ShelfCard> = foot
                .videos
                .iter()
                .map(|v| ShelfCard {
                    title: v.title.clone(),
                    sub: v.artists.join(", "),
                    url: v.thumbnail.clone(),
                    explicit: v.explicit,
                })
                .collect();
            let urls: Vec<Option<String>> = foot.videos.iter().map(|v| v.url.clone()).collect();
            // There is no video player here: they play on Apple Music's website.
            out = out.child(video_shelf(
                app,
                window,
                cx,
                theme,
                at,
                width,
                "Music Videos",
                &cards,
                VIDEO_CARD,
                "music-videos",
                move |_, i, cx| {
                    if let Some(url) = &urls[i] {
                        cx.open_url(url);
                    }
                },
            ));
        }
        y += shelf_height(VIDEO_CARD * 9.0 / 16.0);
    }
    let Some((name, artist, albums)) = &foot.more_by else {
        return out;
    };
    if !in_sight(y, MORE_BY_CARD) {
        return out;
    }
    let at = (left, y + 40.0 - 18.0);
    let title = format!("More by {name}");
    let cards: Vec<ShelfCard> = albums
        .iter()
        .map(|a| ShelfCard {
            title: a.title.clone(),
            sub: a.artists.join(", "),
            url: a.artwork.as_ref().map(|art| art.url(400)),
            explicit: a.explicit,
        })
        .collect();
    let id = format!("more-by-{}", artist.id);
    // The title, and a chevron after it, lead to the artist. (Their page
    // comes later.)
    let (size, weight) = super::heading(theme);
    let chevron = (
        at.0 + text_width(window, &title, size, weight).min(600.0) + 12.0,
        at.1 + 12.0,
    );
    let (text, artist) = (theme.text, artist.clone());
    out = out.child(
        place(at.0, at.1 - 3.0, chevron.0 + 10.0 - at.0, 28.0)
            .id("more-by-title")
            .group("more-by-title")
            .cursor_pointer()
            .child(
                place(
                    chevron.0 - 8.0 - at.0,
                    chevron.1 - 8.0 - (at.1 - 3.0),
                    16.0,
                    16.0,
                )
                .child(
                    gpui::svg()
                        .data(icons::RIGHT.bytes())
                        .size(z(16.0))
                        .text_color(muted)
                        .group_hover("more-by-title", move |s| s.text_color(text)),
                ),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.go(Page::Artist(artist.clone()));
                cx.notify();
            })),
    );
    let albums = albums.clone();
    out.child(cover_shelf(
        app,
        window,
        cx,
        theme,
        at,
        width,
        &title,
        &cards,
        MORE_BY_CARD,
        &id,
        move |app, i, click, _| match click {
            CardClick::Open => app.go(Page::Album(albums[i].id.clone())),
            CardClick::Enqueue => app.enqueue_album(albums[i].id.clone()),
        },
    ))
}

/// The album's or playlist's songs the page shows, in its order.
pub fn shown_tracks(app: &mut Music) -> Arc<Vec<Track>> {
    match app.page.clone() {
        Page::Album(id) => app.album_tracks(&id).unwrap_or_default(),
        Page::Playlist(id) => app
            .playlist_tracks
            .get(&id)
            .map(|p| p.tracks.clone())
            .unwrap_or_default(),
        _ => Arc::default(),
    }
}

/// What a row of the page asked for, as egui's page does it.
pub fn row_action(app: &mut Music, position: usize, action: RowAction) {
    match action {
        RowAction::Select => app.detail_selected = Some(position),
        RowAction::Play => {
            app.detail_selected = Some(position);
            let tracks = shown_tracks(app).to_vec();
            app.play_list(tracks, position);
        }
        RowAction::OpenAlbum(id) => app.go(Page::Album(id)),
    }
}

/// Arrow keys move the selection, Enter plays it, as egui's
/// `tracks::keyboard`: whether `key` was one of them.
pub fn keyboard(app: &mut Music, key: &str) -> bool {
    let tracks = shown_tracks(app);
    let (taken, play) = tracks::keyboard(tracks.len(), &mut app.detail_selected, key);
    if let Some(position) = play {
        app.play_list(tracks.to_vec(), position);
    } else if let Some(row) = app.detail_selected.filter(|_| taken) {
        let shown = app.page.clone();
        let scroll = app.page_scroll(&shown);
        tracks::scroll_to_row(&scroll, BANNER + ACTIONS + tracks::HEAD_SPACE, row);
    }
    taken
}
