//! Track lists, shared by Songs, albums and playlists: the design's grid, its
//! header and its rows.

use egui::{Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use music_core::model::{ItemId, Track};

use crate::app::{SortKey, TrackChoice};
use crate::color::alpha;
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::menu::TrackFacts;
use super::{artwork, explicit_badge, format_time, paint_icon, paint_text, text, thumb_url};

pub const ROW: f32 = 52.0;
pub const HEAD: f32 = 32.0;
/// The header's height plus the gap under it.
pub const HEAD_SPACE: f32 = HEAD + 5.0;

/// Column x ranges inside a row, after the design's grid:
/// `36px 3fr 2fr 2fr 52px 36px`, gap 14, padding 10; without the album,
/// `36px 4fr 2fr 52px 36px`. No source column, as in Cider.
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

    /// An album's tracks, as Cider lists them: number, title, length; the
    /// artist only when it changes from track to track.
    pub fn album(left: f32, width: f32, with_artist: bool) -> Self {
        let gap = 14.0;
        let fixed = 36.0 + 52.0 + 36.0 + gap * if with_artist { 3.0 } else { 2.0 } + 20.0;
        let fr = ((width - fixed) / if with_artist { 6.0 } else { 4.0 }).max(0.0);
        let mut x = left + 10.0;
        let mut take = |w: f32| {
            let range = (x, x + w);
            x += w + gap;
            range
        };
        let number = take(36.0);
        let title = take((4.0 * fr).max(200.0));
        let artist = with_artist.then(|| take((2.0 * fr).max(120.0)));
        Self {
            number,
            title,
            artist,
            album: None,
            duration: take(52.0),
            thumbs: false,
        }
    }
}

/// The column titles. With `sort`, the titles sort the list; returns the one
/// clicked.
pub fn head(
    ui: &mut Ui,
    theme: &Theme,
    columns: &Columns,
    top: f32,
    left: f32,
    width: f32,
    sort: Option<Option<(SortKey, bool)>>,
) -> Option<SortKey> {
    let y = top + HEAD / 2.0;
    let f = font(11.5, Weight::Medium);
    let mut heads = vec![(columns.title, "Title", Some(SortKey::Title))];
    if let Some(artist) = columns.artist {
        heads.push((artist, "Artist", Some(SortKey::Artist)));
    }
    if let Some(album) = columns.album {
        heads.push((album, "Album", Some(SortKey::Album)));
    }
    let mut clicked = None;
    for ((x0, x1), label, key) in heads {
        let galley = text(ui, label, f.clone(), theme.muted(), x1 - x0);
        let rect = Rect::from_min_size(pos2(x0, y - 8.0), vec2(galley.size().x + 12.0, 16.0));
        let mut color = theme.muted();
        if let (Some(key), Some(current)) = (key, sort) {
            let response = ui.interact(rect, Id::unique(("sort", label)), Sense::click());
            if response.hovered() {
                color = theme.text;
            }
            if response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                clicked = Some(key);
            }
            if let Some((sorted, ascending)) = current
                && sorted == key
            {
                let arrow = if ascending { "▲" } else { "▼" };
                let g = text(ui, arrow, font(8.0, Weight::Regular), color, 20.0);
                paint_text(
                    ui,
                    g,
                    x0 + galley.size().x + 4.0,
                    y,
                    egui::Align::Min,
                    color,
                );
            }
        }
        paint_text(ui, galley, x0, y, egui::Align::Min, color);
    }
    // The duration column is headed by a clock, which sorts by length.
    let (_, x1) = columns.duration;
    let clock = Rect::from_center_size(pos2(x1 - 8.0, y), vec2(16.0, 16.0));
    let mut color = theme.muted();
    if sort.is_some() {
        let response = ui.interact(
            clock.expand(4.0),
            Id::unique(("sort", "duration")),
            Sense::click(),
        );
        if response.hovered() {
            color = theme.text;
        }
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            clicked = Some(SortKey::Duration);
        }
    }
    paint_icon(ui, icons::CLOCK, clock.center(), 13.0, color);
    ui.painter().hline(
        left..=left + width,
        top + HEAD - 0.5,
        Stroke::new(1.0, theme.line()),
    );
    clicked
}

/// The next sort after clicking `key`: ascending, then descending, then none.
pub fn cycle(current: Option<(SortKey, bool)>, key: SortKey) -> Option<(SortKey, bool)> {
    match current {
        Some((k, true)) if k == key => Some((key, false)),
        Some((k, false)) if k == key => None,
        _ => Some((key, true)),
    }
}

/// What a row was asked to do.
pub enum RowAction {
    None,
    Select,
    Play,
    OpenAlbum(ItemId),
    /// From the right-click menu.
    Menu(TrackChoice),
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
    /// It is among the favourites: a star follows the title.
    pub favourite: bool,
    /// It is in the library already.
    pub in_library: bool,
    /// The list is a playlist of the user's, which the song can leave.
    pub in_playlist: bool,
    /// Rows are told apart by this and their position.
    pub list: &'static str,
    pub position: usize,
}

pub fn row(
    ui: &mut Ui,
    theme: &Theme,
    columns: &Columns,
    rect: Rect,
    track: &Track,
    state: RowState,
) -> RowAction {
    let response = ui.interact(
        rect,
        Id::unique((state.list, "row", state.position)),
        Sense::click(),
    );
    let hovered = response.contains_pointer();
    if hovered {
        super::pointer_on(ui, track);
    }
    let fill = if state.selected {
        Some(alpha(theme.accent, 0.16))
    } else if hovered {
        Some(theme.hover())
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, 8.0, fill);
    }
    let y = rect.center().y;
    let mut action = RowAction::None;

    // Number, or a play button on hover, or bars for the playing track.
    // `contains_pointer`, not `hovered`: over the play button the row is no
    // longer the hovered widget, and the button must not vanish under the click.
    let (n0, n1) = columns.number;
    let n_center = pos2((n0 + n1) / 2.0, y);
    if hovered {
        let button = Rect::from_center_size(n_center, vec2(28.0, 28.0));
        let hit = ui.interact(
            button,
            Id::unique((state.list, "play", state.position)),
            Sense::click(),
        );
        if hit.hovered() {
            ui.painter().circle_filled(n_center, 14.0, theme.hover());
        }
        paint_icon(ui, icons::PLAY_FILL, n_center, 13.0, theme.text);
        if hit.clicked() {
            action = RowAction::Play;
        }
    } else if state.now {
        bars(ui, theme, n_center, state.playing);
    } else if let Some(number) = state.number {
        let g = text(
            ui,
            &number.to_string(),
            font(13.0, Weight::Regular),
            theme.muted(),
            36.0,
        );
        paint_text(ui, g, n_center.x, y, egui::Align::Center, theme.muted());
    }

    // Artwork, title and the E badge.
    let (t0, t1) = columns.title;
    let tx = if columns.thumbs {
        let art = Rect::from_min_size(pos2(t0, y - 18.0), vec2(36.0, 36.0));
        artwork(ui, thumb_url(track), art, 5.0, theme);
        art.right() + 12.0
    } else {
        t0
    };
    let title_color = if state.now { theme.accent } else { theme.text };
    let max = t1
        - tx
        - if track.explicit { 18.0 } else { 0.0 }
        - if state.favourite { 17.0 } else { 0.0 };
    let title = text(
        ui,
        &track.title,
        font(13.0, Weight::Medium),
        title_color,
        max,
    );
    let title_end = tx + title.size().x;
    paint_text(ui, title, tx, y, egui::Align::Min, title_color);
    let mut end = title_end;
    if track.explicit {
        explicit_badge(ui, theme, pos2(title_end + 6.0 + 6.0, y));
        end += 6.0 + 12.0;
    }
    if state.favourite {
        favourite_star(ui, theme, pos2(end + 6.0 + 6.0, y));
    }

    // The artist and the album, each a link to its page.
    if let Some((a0, a1)) = columns.artist {
        let g = text(
            ui,
            &track.artists.join(", "),
            font(13.0, Weight::Regular),
            theme.subtext0,
            a1 - a0,
        );
        let link = Rect::from_min_size(pos2(a0, y - g.size().y / 2.0), g.size());
        let mut color = theme.subtext0;
        // Only a catalog song leads to its artist; an upload (`i.`) has none.
        if !track.id.id.starts_with("i.") {
            let hit = ui.interact(
                link,
                Id::unique((state.list, "artist", state.position)),
                Sense::click(),
            );
            if hit.hovered() {
                color = theme.text;
                ui.painter()
                    .hline(link.x_range(), link.bottom(), Stroke::new(1.0, color));
            }
            if hit
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                action = RowAction::Menu(TrackChoice::GoToArtist);
            }
        }
        paint_text(ui, g, a0, y, egui::Align::Min, color);
    }
    if let Some((c0, c1)) = columns.album {
        let name = track.album.clone().unwrap_or_default();
        let g = text(
            ui,
            &name,
            font(13.0, Weight::Regular),
            theme.subtext0,
            c1 - c0,
        );
        let link = Rect::from_min_size(pos2(c0, y - g.size().y / 2.0), g.size());
        let mut color = theme.subtext0;
        if let Some(album) = &track.album_id {
            let hit = ui.interact(
                link,
                Id::unique((state.list, "album", state.position)),
                Sense::click(),
            );
            if hit.hovered() {
                color = theme.text;
                ui.painter()
                    .hline(link.x_range(), link.bottom(), Stroke::new(1.0, color));
            }
            if hit
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                action = RowAction::OpenAlbum(album.clone());
            }
        }
        paint_text(ui, g, c0, y, egui::Align::Min, color);
    }

    // Duration.
    let (_, d1) = columns.duration;
    if let Some(duration) = track.duration {
        let g = text(
            ui,
            &format_time(duration),
            font(13.0, Weight::Regular),
            theme.muted(),
            52.0,
        );
        paint_text(ui, g, d1, y, egui::Align::Max, theme.muted());
    }

    let facts = TrackFacts {
        has_album: track.album_id.is_some(),
        favourite: state.favourite,
        in_library: state.in_library,
        removable: track.library_id.is_some(),
        in_playlist: state.in_playlist && track.library_id.is_some(),
    };
    if let Some(choice) = super::menu::track_menu(&response, theme, facts) {
        action = RowAction::Menu(choice);
    }
    if matches!(action, RowAction::None) {
        if response.double_clicked() {
            action = RowAction::Play;
        } else if response.clicked() {
            action = RowAction::Select;
        }
    }
    action
}

/// The star after a favourite's title, as Cider marks them.
pub fn favourite_star(ui: &Ui, theme: &Theme, center: egui::Pos2) {
    paint_icon(ui, icons::STAR_FILL, center, 11.0, theme.red);
}

/// Three accent bars for the playing row, moving while it plays.
fn bars(ui: &Ui, theme: &Theme, center: egui::Pos2, moving: bool) {
    let t = ui.input(|i| i.time) as f32;
    for (k, phase) in [0.0_f32, 1.7, 3.1].into_iter().enumerate() {
        let h = if moving {
            4.0 + 8.0 * (0.5 + 0.5 * (t * 6.0 + phase).sin())
        } else {
            4.0
        };
        let x = center.x - 5.0 + k as f32 * 5.0;
        let rect = Rect::from_min_max(
            pos2(x - 1.5, center.y + 6.0 - h),
            pos2(x + 1.5, center.y + 6.0),
        );
        ui.painter().rect_filled(rect, 1.0, theme.accent);
    }
    if moving {
        ui.ctx().request_repaint();
    }
}

/// Grey placeholder rows while a list loads.
pub fn skeleton(ui: &Ui, theme: &Theme, top: f32, left: f32, width: f32, rows: usize) {
    for i in 0..rows {
        let rect = Rect::from_min_size(pos2(left, top + i as f32 * (ROW + 2.0)), vec2(width, ROW));
        ui.painter().rect_filled(rect, 8.0, theme.hover());
    }
}

/// The rows of `count` that the viewport shows, for a list starting at `top`
/// (in the same coordinates as the viewport).
pub fn visible(viewport: Rect, top: f32, count: usize) -> std::ops::Range<usize> {
    let first = ((viewport.top() - top) / ROW).floor().max(0.0) as usize;
    let last = (((viewport.bottom() - top) / ROW).ceil().max(0.0) as usize).min(count);
    first.min(last)..last
}

/// Arrow keys move the selection, Enter plays it. Returns the row to play.
pub fn keyboard(ui: &Ui, rows: usize, selected: &mut Option<usize>) -> Option<usize> {
    if rows == 0 || ui.ctx().egui_wants_keyboard_input() {
        return None;
    }
    let (down, up, enter) = ui.input(|i| {
        (
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::Enter),
        )
    });
    if down {
        *selected = Some(selected.map_or(0, |s| (s + 1).min(rows - 1)));
    }
    if up {
        *selected = Some(selected.map_or(0, |s| s.saturating_sub(1)));
    }
    if enter { *selected } else { None }
}
