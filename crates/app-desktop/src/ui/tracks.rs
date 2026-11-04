//! Track lists, shared by Songs, albums and playlists: the design's grid, its
//! header and its rows.

use egui::{Id, Rect, Response, Sense, Stroke, Ui, pos2, vec2};
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
        if theme.layout.rows != crate::theme::Rows::Stacked {
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

    /// With the small artwork before each title.
    pub fn with_thumbs(self) -> Self {
        Self {
            thumbs: true,
            ..self
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
    let columns = &columns.for_theme(theme);
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
    /// How it stands as a download, if it is one.
    pub download: Option<Badge>,
    /// It can be downloaded: a song of a service.
    pub downloadable: bool,
    /// Its service is out of reach and it isn't downloaded: it shows faded.
    pub unplayable: bool,
    /// Rows are told apart by this and their position.
    pub list: &'static str,
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

/// What a page's download button shows of its songs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageDownload {
    /// Not all kept: a click downloads the rest.
    Offer,
    /// On their way, this much of it: a click calls them off.
    Coming(f32),
    /// All kept: a click removes them.
    Kept,
}

/// A page's download button, an arrow in a ring `size` wide, as Spotify's:
/// filled in the accent once all is kept, the ring filling as it comes.
pub fn download_button(
    ui: &Ui,
    theme: &Theme,
    center: egui::Pos2,
    size: f32,
    salt: &str,
    state: PageDownload,
) -> Response {
    let rect = Rect::from_center_size(center, vec2(size, size));
    let response = ui
        .interact(rect, Id::unique(("page-download", salt)), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let r = size * 0.3;
    let glyph = size * 0.36;
    let hover = response.hovered();
    let ink = if hover { theme.text } else { theme.subtext0 };
    let ring = Stroke::new(1.8, ink);
    let tip = match state {
        PageDownload::Offer => {
            ui.painter().circle_stroke(center, r, ring);
            paint_icon(ui, icons::ARROW_DOWN, center, glyph, ink);
            "Download"
        }
        PageDownload::Coming(p) => {
            ui.painter()
                .circle_stroke(center, r, Stroke::new(1.8, alpha(ink, 0.35)));
            let p = p.clamp(0.02, 1.0);
            let steps = (p * 64.0).ceil() as usize;
            let points: Vec<egui::Pos2> = (0..=steps)
                .map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2
                        + std::f32::consts::TAU * p * i as f32 / steps as f32;
                    center + r * vec2(a.cos(), a.sin())
                })
                .collect();
            ui.painter()
                .add(egui::Shape::line(points, Stroke::new(1.8, theme.accent)));
            let stop = Rect::from_center_size(center, vec2(glyph * 0.45, glyph * 0.45));
            ui.painter().rect_filled(stop, 1.0, theme.accent);
            "Stop downloading"
        }
        PageDownload::Kept => {
            ui.painter().circle_filled(center, r + 0.9, theme.accent);
            paint_icon(ui, icons::ARROW_DOWN, center, glyph, theme.on_accent());
            "Remove Download"
        }
    };
    response.on_hover_text(tip)
}

/// A download's mark, `center`ed: an arrow in a disc once kept, a ring
/// filling as it comes, as Apple Music and Spotify mark them.
pub fn download_badge(ui: &Ui, theme: &Theme, center: egui::Pos2, badge: Badge) -> Response {
    let r = 7.0;
    let ring = |color| {
        ui.painter()
            .circle_stroke(center, r - 0.75, Stroke::new(1.5, color));
    };
    let tip = match badge {
        Badge::Queued => {
            ring(theme.muted());
            "Waiting to download"
        }
        Badge::Coming(p) => {
            ring(alpha(theme.muted(), 0.5));
            let p = p.clamp(0.02, 1.0);
            let steps = (p * 48.0).ceil() as usize;
            let points: Vec<egui::Pos2> = (0..=steps)
                .map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2
                        + std::f32::consts::TAU * p * i as f32 / steps as f32;
                    center + (r - 0.75) * vec2(a.cos(), a.sin())
                })
                .collect();
            ui.painter()
                .add(egui::Shape::line(points, Stroke::new(1.5, theme.accent)));
            "Downloading"
        }
        Badge::Kept => {
            ui.painter().circle_filled(center, r, theme.accent);
            paint_icon(ui, icons::ARROW_DOWN, center, 9.0, theme.on_accent());
            "Downloaded"
        }
        Badge::Expired => {
            ui.painter().circle_filled(center, r, theme.muted());
            paint_icon(ui, icons::ARROW_DOWN, center, 9.0, theme.base);
            "Downloaded, but the subscription wasn't confirmed: plays again once online"
        }
        Badge::Failed => {
            ring(theme.red);
            paint_icon(ui, icons::ARROW_DOWN, center, 9.0, theme.red);
            "Couldn't download"
        }
    };
    ui.interact(
        Rect::from_center_size(center, vec2(2.0 * r, 2.0 * r)),
        Id::unique(("badge", center.x as i32, center.y as i32)),
        Sense::hover(),
    )
    .on_hover_text(tip)
}

pub fn row(
    ui: &mut Ui,
    theme: &Theme,
    columns: &Columns,
    rect: Rect,
    track: &Track,
    state: RowState,
) -> RowAction {
    let stacked = theme.layout.rows == crate::theme::Rows::Stacked;
    let columns = &columns.for_theme(theme);
    let response = ui.interact(
        rect,
        Id::unique((state.list, "row", state.position)),
        Sense::click(),
    );
    let hovered = response.contains_pointer();
    if hovered {
        super::pointer_on(ui, track);
    }
    // The hover comes and goes in a tenth of a second.
    let lit = ui
        .ctx()
        .animate_bool_with_time(response.id.with("lit"), hovered, 0.1);
    // M3E's: radius 16, and the song playing a pill in the secondary
    // container.
    let expressive = theme.expressive();
    let playing_pill = expressive && state.now;
    let fill = if playing_pill {
        Some(theme.tonal.secondary_container)
    } else if state.selected {
        Some(alpha(theme.accent, 0.16))
    } else if lit > 0.0 {
        Some(alpha(
            theme.text,
            if expressive { 0.05 } else { 0.07 } * lit,
        ))
    } else {
        None
    };
    if let Some(fill) = fill {
        let radius = match () {
            _ if playing_pill => rect.height() / 2.0,
            _ if expressive => 16.0,
            _ => 8.0,
        };
        ui.painter().rect_filled(rect, radius, fill);
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
        let radius = if expressive { 10.0 } else { 5.0 };
        artwork(ui, thumb_url(track), art, radius, theme);
        art.right() + 12.0
    } else {
        t0
    };
    let title_color = match () {
        _ if playing_pill => theme.tonal.on_secondary_container,
        _ if state.now => theme.accent,
        _ => theme.text,
    };
    let max = if stacked {
        t1 - tx
    } else {
        t1 - tx - if track.explicit { 18.0 } else { 0.0 } - if state.favourite { 17.0 } else { 0.0 }
    };
    // Spotify's stacked rows read the title larger, in regular weight.
    let title_font = if stacked {
        font(14.5, Weight::Regular)
    } else {
        font(13.0, Weight::Medium)
    };
    let title = text(ui, &track.title, title_font, title_color, max);
    let title_end = tx + title.size().x;
    if stacked {
        // The title over the E badge and the artists, the favourite's mark
        // by the length, as Spotify's.
        paint_text(ui, title, tx, y - 9.0, egui::Align::Min, title_color);
        let mut ax = tx;
        // Spotify's mark of a download, first on the artists' line.
        if let Some(badge) = state.download {
            download_badge(ui, theme, pos2(ax + 7.0, y + 10.0), badge);
            ax += 14.0 + 5.0;
        }
        if track.explicit {
            explicit_badge(ui, theme, pos2(ax + 6.0, y + 10.0));
            ax += 12.0 + 5.0;
        }
        let g = text(
            ui,
            &track.artists.join(", "),
            font(12.5, Weight::Regular),
            theme.subtext0,
            (t1 - ax).max(0.0),
        );
        let link = Rect::from_min_size(pos2(ax, y + 10.0 - g.size().y / 2.0), g.size());
        let mut color = theme.subtext0;
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
        let g = text(
            ui,
            &track.artists.join(", "),
            font(12.5, Weight::Regular),
            color,
            (t1 - ax).max(0.0),
        );
        paint_text(ui, g, ax, y + 10.0, egui::Align::Min, color);
        if state.favourite {
            favourite_star(ui, theme, pos2(columns.duration.0 - 14.0, y));
        }
    } else {
        paint_text(ui, title, tx, y, egui::Align::Min, title_color);
        let mut end = title_end;
        if track.explicit {
            explicit_badge(ui, theme, pos2(title_end + 6.0 + 6.0, y));
            end += 6.0 + 12.0;
        }
        if state.favourite {
            favourite_star(ui, theme, pos2(end + 6.0 + 6.0, y));
        }
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

    // Apple Music's mark of a download, before the length.
    if !stacked && let Some(badge) = state.download {
        download_badge(ui, theme, pos2(columns.duration.0 + 8.0, y), badge);
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

    // Out of reach, as Spotify fades what it can't play offline.
    if state.unplayable && !state.now {
        ui.painter().rect_filled(rect, 0.0, alpha(theme.base, 0.6));
    }

    let facts = TrackFacts {
        file: track.id.provider == music_core::model::ProviderKind::Local,
        has_album: track.album_id.is_some(),
        favourite: state.favourite,
        in_library: state.in_library,
        removable: track.library_id.is_some(),
        // Apple names a playlist's row by its library id; SoundCloud by the
        // track itself.
        in_playlist: state.in_playlist
            && (track.library_id.is_some()
                || track.id.provider == music_core::model::ProviderKind::SoundCloud),
        download: state
            .downloadable
            .then_some(state.download.is_some_and(|b| b != Badge::Failed)),
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
    match theme.layout.favourite {
        crate::theme::Favourite::Star => paint_icon(ui, icons::STAR_FILL, center, 11.0, theme.red),
        crate::theme::Favourite::Check => {
            ui.painter().circle_filled(center, 6.5, theme.accent);
            paint_icon(ui, icons::CHECK, center, 9.0, theme.on_accent());
        }
        crate::theme::Favourite::Heart => {
            paint_icon(ui, icons::HEART_FILL, center, 12.0, theme.tonal.primary)
        }
    }
}

/// Three accent bars for the playing row, moving while it plays.
fn bars(ui: &Ui, theme: &Theme, center: egui::Pos2, moving: bool) {
    // Still while the window is in the background: the whole window would
    // be drawn again for them alone.
    let moving = moving && ui.input(|i| i.focused);
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
        let color = if theme.expressive() {
            theme.tonal.primary
        } else {
            theme.accent
        };
        ui.painter().rect_filled(rect, 1.0, color);
    }
    // 20 frames a second are plenty for them.
    if moving {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(50));
    }
}

/// A list on its way: its rows in grey, or the spinner over them.
pub fn skeleton(ui: &Ui, theme: &Theme, top: f32, left: f32, width: f32, rows: usize) {
    for i in 0..rows {
        let rect = Rect::from_min_size(pos2(left, top + i as f32 * (ROW + 2.0)), vec2(width, ROW));
        super::loading::shape(ui, theme, rect, 8.0);
    }
    super::loading::spinner(ui, theme, pos2(left + width / 2.0, top + ROW));
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
    if rows == 0 || super::keys_taken(ui.ctx()) {
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
    if let Some(row) = selected.filter(|_| down || up) {
        ui.ctx().data_mut(|d| d.insert_temp(scroll_id(), row));
        ui.ctx().request_repaint();
    }
    if enter { *selected } else { None }
}

fn scroll_id() -> Id {
    Id::unique("track-row-scroll")
}

/// Bring the row the arrow keys moved to into view, in the list whose rows
/// start at `rows_top`.
pub fn scroll_to_selected(ui: &Ui, rows_top: f32, left: f32, width: f32) {
    if let Some(row) = ui.ctx().data_mut(|d| d.remove_temp::<usize>(scroll_id())) {
        let rect = Rect::from_min_size(pos2(left, rows_top + row as f32 * ROW), vec2(width, ROW));
        ui.scroll_to_rect(rect, None);
    }
}
