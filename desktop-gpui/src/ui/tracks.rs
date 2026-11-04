//! Track lists, as the egui window's `ui/tracks.rs`: the design's grid, its
//! header and its rows. Positions are in points from the list's own left,
//! as egui's are from the page's.

use gpui::prelude::FluentBuilder;
use gpui::{
    App, ClickEvent, Context, Div, InteractiveElement, IntoElement, ParentElement, Rgba,
    StatefulInteractiveElement, Styled, Window, div,
};
use music_core::model::Track;

use crate::app::{Music, SortKey};
use crate::color::alpha;
use crate::fonts::{Weight, line_height};
use crate::icons;
use crate::theme::{Favourite, Rows, Theme};

use super::{
    Align, artwork, disc, explicit_badge, format_time, hline, icon_at, label, label_cut, place,
    text_at, thumb_url, z,
};

pub const ROW: f32 = 52.0;
pub const HEAD: f32 = 32.0;
/// The header's height plus the gap under it.
pub const HEAD_SPACE: f32 = HEAD + 5.0;

/// Column x ranges inside a row, after the design's grid:
/// `36px 3fr 2fr 2fr 52px 36px`, gap 14, padding 10; without the album,
/// `36px 4fr 2fr 52px 36px`. No source column, as in Cider.
#[derive(Clone, Copy)]
pub struct Columns {
    number: (f32, f32),
    title: (f32, f32),
    artist: Option<(f32, f32)>,
    album: Option<(f32, f32)>,
    duration: (f32, f32),
    /// Small artwork before the title; album pages leave it out, as Cider does.
    thumbs: bool,
}

impl Columns {
    pub fn new(left: f32, width: f32, with_album: bool) -> Self {
        let gap = 14.0;
        let gaps = if with_album { 5.0 } else { 4.0 };
        let fixed = 36.0 + 52.0 + 36.0 + gap * gaps + 20.0;
        let fr = ((width - fixed) / if with_album { 7.0 } else { 6.0 }).max(0.0);
        let mut x = left + 10.0;
        let mut take = |w: f32| {
            let range = (x, x + w);
            x += w + gap;
            range
        };
        let number = take(36.0);
        let title = take(((if with_album { 3.0 } else { 4.0 }) * fr).max(200.0));
        let artist = take((2.0 * fr).max(120.0));
        let album = with_album.then(|| take((2.0 * fr).max(120.0)));
        Self {
            number,
            title,
            artist: Some(artist),
            album,
            duration: take(52.0),
            thumbs: true,
        }
    }

    /// The artist's column given to the title, whose row says them under
    /// it, in a theme of stacked rows.
    fn for_theme(self, theme: &Theme) -> Self {
        if theme.layout.rows != Rows::Stacked {
            return self;
        }
        let title = match self.artist {
            Some((_, a1)) => (self.title.0, a1),
            None => self.title,
        };
        Self {
            title,
            artist: None,
            ..self
        }
    }
}

/// The column titles, `width` points wide from `left` in a box whose top
/// is the list's. With `sort`, the titles sort the list.
pub fn head(
    theme: &Theme,
    columns: &Columns,
    left: f32,
    width: f32,
    sort: Option<Option<(SortKey, bool)>>,
    cx: &mut Context<Music>,
) -> Div {
    let columns = &columns.for_theme(theme);
    let y = HEAD / 2.0;
    let mut heads = vec![(columns.title, "Title", SortKey::Title)];
    if let Some(artist) = columns.artist {
        heads.push((artist, "Artist", SortKey::Artist));
    }
    if let Some(album) = columns.album {
        heads.push((album, "Album", SortKey::Album));
    }
    let (muted, text) = (theme.muted(), theme.text);
    let mut out = div().absolute().left_0().top_0().size_full();
    for ((x0, x1), name, key) in heads {
        let title = label_cut(name, 11.5, Weight::Medium, muted, x1 - x0);
        let Some(current) = sort else {
            out = out.child(text_at(x0, y, 11.5, Align::Min, title));
            continue;
        };
        // Sorting: the title and its arrow light up under the pointer.
        let arrow = current
            .filter(|(sorted, _)| *sorted == key)
            .map(|(_, ascending)| if ascending { "▲" } else { "▼" });
        let group = format!("sort-{name}");
        let lit = |d: Div| d.group_hover(group.clone(), move |s| s.text_color(text));
        let mut word = div().flex().items_center().gap(z(4.0)).child(lit(title));
        if let Some(arrow) = arrow {
            word = word.child(lit(label(arrow, 8.0, Weight::Regular, muted)));
        }
        out = out.child(
            text_at(x0, y, 11.5, Align::Min, word)
                .id(name)
                .group(group)
                .cursor_pointer()
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.sort_songs(key);
                    cx.notify();
                })),
        );
    }
    // The duration column is headed by a clock, which sorts by length.
    let (_, x1) = columns.duration;
    let clock = icon_at(icons::CLOCK, x1 - 8.0, y, 13.0, muted);
    out = out.child(match sort {
        Some(_) => clock
            .id("sort-duration")
            .cursor_pointer()
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.sort_songs(SortKey::Duration);
                cx.notify();
            }))
            .into_any_element(),
        None => clock.into_any_element(),
    });
    out.child(hline(left, HEAD - 0.5, width, theme.line()))
}

/// The next sort after clicking `key`: ascending, then descending, then none.
pub fn cycle(current: Option<(SortKey, bool)>, key: SortKey) -> Option<(SortKey, bool)> {
    match current {
        Some((k, true)) if k == key => Some((key, false)),
        Some((k, false)) if k == key => None,
        _ => Some((key, true)),
    }
}

/// How a row shows.
pub struct RowState {
    /// What the number column shows: the position or the track number.
    pub number: Option<u32>,
    pub selected: bool,
    /// This is the track the player has.
    pub now: bool,
    /// …and it is playing.
    pub playing: bool,
    /// It is among the favourites: a mark follows the title.
    pub favourite: bool,
    /// How it stands as a download, if it is one.
    pub download: Option<Badge>,
    /// Its service is out of reach and it isn't downloaded: it shows faded.
    pub unplayable: bool,
    pub position: usize,
}

/// A download's mark on its row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Badge {
    Queued,
    /// How much has come, from 0 to 1.
    Coming(f32),
    Kept,
    /// Kept, but its subscription wasn't confirmed in time.
    Expired,
    Failed,
}

/// A download's mark, centred on `x`, `y`: an arrow in a disc once kept, a
/// ring as it comes, as Apple Music and Spotify mark them.
fn download_badge(theme: &Theme, x: f32, y: f32, badge: Badge) -> Div {
    let r = 7.0;
    let ring = |color: Rgba| {
        place(x - r, y - r, 2.0 * r, 2.0 * r)
            .rounded_full()
            .border(z(1.5))
            .border_color(color)
    };
    match badge {
        Badge::Queued => ring(theme.muted()),
        // The ring's arc as it fills is egui's polyline: here the ring in
        // the accent once half has come.
        Badge::Coming(p) => ring(if p >= 0.5 {
            theme.accent
        } else {
            alpha(theme.muted(), 0.5)
        }),
        Badge::Kept => disc(x, y, r, theme.accent).child(icon_at(
            icons::ARROW_DOWN,
            r,
            r,
            9.0,
            theme.on_accent(),
        )),
        Badge::Expired => {
            disc(x, y, r, theme.muted()).child(icon_at(icons::ARROW_DOWN, r, r, 9.0, theme.base))
        }
        Badge::Failed => {
            ring(theme.red).child(icon_at(icons::ARROW_DOWN, r - 1.5, r - 1.5, 9.0, theme.red))
        }
    }
}

/// A row, `width` points wide, its columns' places from its own left.
#[allow(clippy::too_many_arguments)]
pub fn row(
    window: &mut Window,
    cx: &mut Context<Music>,
    theme: &Theme,
    columns: &Columns,
    width: f32,
    track: &Track,
    state: RowState,
) -> impl IntoElement + use<> {
    let stacked = theme.layout.rows == Rows::Stacked;
    let columns = &columns.for_theme(theme);
    let position = state.position;
    let y = ROW / 2.0;
    let group = "song-row";
    let hover = alpha(theme.text, 0.07);
    let mut out = div()
        .id(("song", position))
        .group(group)
        .relative()
        .w(z(width))
        .h(z(ROW))
        .rounded(z(8.0))
        .when(state.selected, |d| d.bg(alpha(theme.accent, 0.16)))
        .when(!state.selected, |d| d.hover(move |s| s.bg(hover)))
        // A click chooses it, a double click plays the page from it.
        .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
            this.songs_view.selected = Some(position);
            if e.click_count() == 2 {
                this.play_list(super::songs::shown_tracks(this), position);
                this.acted(window, cx);
            }
            cx.notify();
        }));

    // Number, or a play button on hover, or bars for the playing track.
    let (n0, n1) = columns.number;
    let n_center = (n0 + n1) / 2.0;
    let (text, muted) = (theme.text, theme.muted());
    out = out.child(
        disc(n_center, y, 14.0, crate::color::TRANSPARENT)
            .id(("play", position))
            .invisible()
            .group_hover(group, |s| s.visible())
            .hover(move |s| s.bg(hover))
            .child(icon_at(icons::PLAY_FILL, 14.0, 14.0, 13.0, text))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                this.songs_view.selected = Some(position);
                this.play_list(super::songs::shown_tracks(this), position);
                this.acted(window, cx);
            })),
    );
    if state.now {
        out = out
            .child(bars(theme, n_center, y, state.playing).group_hover(group, |s| s.invisible()));
    } else if let Some(number) = state.number {
        out = out.child(
            text_at(
                n_center,
                y,
                13.0,
                Align::Center,
                label_cut(number.to_string(), 13.0, Weight::Regular, muted, 36.0),
            )
            .group_hover(group, |s| s.invisible()),
        );
    }

    // Artwork, title and the E badge.
    let (t0, t1) = columns.title;
    let tx = if columns.thumbs {
        out = out.child(div().absolute().left(z(t0)).top(z(y - 18.0)).child(artwork(
            window,
            cx,
            thumb_url(track).as_deref(),
            36.0,
            5.0,
            theme,
        )));
        t0 + 36.0 + 12.0
    } else {
        t0
    };
    let title_color = if state.now { theme.accent } else { theme.text };
    let artists = track.artists.join(", ");
    // Only a catalog song leads to its artist; an upload (`i.`) has none.
    let artist_link = !track.id.id.starts_with("i.");
    if stacked {
        // The title over the E badge and the artists, the favourite's mark
        // by the length, as Spotify's.
        out = out.child(text_at(
            tx,
            y - 9.0,
            14.5,
            Align::Min,
            label_cut(
                track.title.clone(),
                14.5,
                Weight::Regular,
                title_color,
                t1 - tx,
            ),
        ));
        let mut ax = tx;
        // Spotify's mark of a download, first on the artists' line.
        if let Some(badge) = state.download {
            out = out.child(download_badge(theme, ax + 7.0, y + 10.0, badge));
            ax += 14.0 + 5.0;
        }
        if track.explicit {
            out = out.child(explicit_badge(theme, ax + 6.0, y + 10.0));
            ax += 12.0 + 5.0;
        }
        out = out.child(link(
            ("artist", position),
            theme,
            ax,
            y + 10.0,
            12.5,
            artists,
            (t1 - ax).max(0.0),
            artist_link,
        ));
        if state.favourite {
            out = out.child(favourite_mark(theme, columns.duration.0 - 14.0, y));
        }
    } else {
        // One line: the title, its badge and mark after it.
        let title_w = super::text_width(window, &track.title, 13.0, Weight::Medium);
        let max = t1
            - tx
            - if track.explicit { 18.0 } else { 0.0 }
            - if state.favourite { 17.0 } else { 0.0 };
        out = out.child(text_at(
            tx,
            y,
            13.0,
            Align::Min,
            label_cut(track.title.clone(), 13.0, Weight::Medium, title_color, max),
        ));
        let title_end = tx + title_w.min(max);
        let mut end = title_end;
        if track.explicit {
            out = out.child(explicit_badge(theme, title_end + 12.0, y));
            end += 18.0;
        }
        if state.favourite {
            out = out.child(favourite_mark(theme, end + 12.0, y));
        }
    }

    // The artist and the album, each a link to its page.
    if let Some((a0, a1)) = columns.artist {
        out = out.child(link(
            ("artist", position),
            theme,
            a0,
            y,
            13.0,
            track.artists.join(", "),
            a1 - a0,
            artist_link,
        ));
    }
    if let Some((c0, c1)) = columns.album {
        out = out.child(link(
            ("album", position),
            theme,
            c0,
            y,
            13.0,
            track.album.clone().unwrap_or_default(),
            c1 - c0,
            track.album_id.is_some(),
        ));
    }

    // Apple Music's mark of a download, before the length.
    if !stacked && let Some(badge) = state.download {
        out = out.child(download_badge(theme, columns.duration.0 + 8.0, y, badge));
    }

    // Duration.
    let (_, d1) = columns.duration;
    if let Some(duration) = track.duration {
        out = out.child(text_at(
            d1,
            y,
            13.0,
            Align::Max,
            label_cut(format_time(duration), 13.0, Weight::Regular, muted, 52.0),
        ));
    }

    // Out of reach, as Spotify fades what it can't play offline.
    if state.unplayable && !state.now {
        out = out.child(place(0.0, 0.0, width, ROW).bg(alpha(theme.base, 0.6)));
    }
    out
}

/// Text that leads to a page: in the text's colour and underlined under
/// the pointer, when it does. Its clicks go nowhere in this slice, but stop
/// at it, as egui's links take theirs.
#[allow(clippy::too_many_arguments)]
fn link(
    id: (&'static str, usize),
    theme: &Theme,
    x: f32,
    y: f32,
    size: f32,
    s: String,
    max: f32,
    leads: bool,
) -> impl IntoElement + use<> {
    let words = label_cut(s, size, Weight::Regular, theme.subtext0, max);
    if !leads {
        return text_at(x, y, size, Align::Min, words).into_any_element();
    }
    let text = theme.text;
    let group = format!("link-{}", id.0);
    text_at(
        x,
        y,
        size,
        Align::Min,
        div()
            .id(id)
            .group(group.clone())
            .relative()
            .cursor_pointer()
            .child(words.group_hover(group.clone(), move |s| s.text_color(text)))
            // egui's underline: a hairline along the galley's bottom.
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(z(line_height(size) - 0.5))
                    .h(z(1.0))
                    .invisible()
                    .bg(text)
                    .group_hover(group, |s| s.visible()),
            )
            .on_click(|_, _, cx: &mut App| cx.stop_propagation()),
    )
    .into_any_element()
}

/// The mark after a favourite's title: Cider's star, Spotify's check,
/// M3E's heart.
pub fn favourite_mark(theme: &Theme, x: f32, y: f32) -> Div {
    match theme.layout.favourite {
        Favourite::Star => icon_at(icons::STAR_FILL, x, y, 11.0, theme.red),
        Favourite::Check => disc(x, y, 6.5, theme.accent).child(icon_at(
            icons::CHECK,
            6.5,
            6.5,
            9.0,
            theme.on_accent(),
        )),
        Favourite::Heart => icon_at(icons::HEART_FILL, x, y, 12.0, theme.tonal.primary),
    }
}

/// Three accent bars for the playing row; still here (they move only while
/// it plays, which this slice doesn't).
fn bars(theme: &Theme, x: f32, y: f32, _moving: bool) -> Div {
    let color = if theme.expressive() {
        theme.tonal.primary
    } else {
        theme.accent
    };
    let mut out = div().absolute().left_0().top_0();
    for k in 0..3 {
        let h = 4.0;
        let bx = x - 5.0 + k as f32 * 5.0;
        out = out.child(
            place(bx - 1.5, y + 6.0 - h, 3.0, h)
                .rounded(z(1.0))
                .bg(color),
        );
    }
    out
}

/// A list on its way: its rows in grey.
pub fn skeleton(theme: &Theme, top: f32, left: f32, width: f32, rows: usize) -> Div {
    let mut out = div().absolute().left_0().top_0().size_full();
    for i in 0..rows {
        out = out.child(
            place(left, top + i as f32 * (ROW + 2.0), width, ROW)
                .rounded(z(8.0))
                .bg(theme.surface0),
        );
    }
    out
}

/// The rows of `count` that a viewport `from`..`to` shows (in points, the
/// list's own), for a list starting at `top`.
pub fn visible(from: f32, to: f32, top: f32, count: usize) -> std::ops::Range<usize> {
    let first = ((from - top) / ROW).floor().max(0.0) as usize;
    let last = (((to - top) / ROW).ceil().max(0.0) as usize).min(count);
    first.min(last)..last
}
