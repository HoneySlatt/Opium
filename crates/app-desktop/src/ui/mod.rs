//! The window: sidebar, page, top bar and player bar, after `design/…/DESIGN.md`.

mod artist;
mod detail;
mod devices;
mod dock;
mod drawer;
mod grid;
mod home;
mod library;
mod library_artist;
mod lyrics;
mod menu;
mod now_playing;
mod pairing;
mod player_bar;
mod playlist_dialog;
mod search;
mod settings;
mod side;
mod sidebar;
mod signal;
mod songs;
mod tracks;
mod window_bar;

use std::sync::Arc;
use std::time::Duration;

use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Color32, CornerRadius, FontId, Galley, Mesh, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2,
    pos2, vec2,
};

use crate::app::{App, Page, SignalAnchor};
use crate::color::alpha;
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::{Panel, PlayerBar, Sidebar, Theme, TopBar, Window};

/// The gap between cards, in a theme of cards.
pub const GAP: f32 = 8.0;
/// The cards' corners.
pub const RADIUS: f32 = 8.0;

/// Below this window width the sidebar collapses and padding tightens.
pub const NARROW: f32 = 1100.0;
pub const TOP_BAR: f32 = 52.0;

pub fn show(app: &mut App, ui: &mut Ui) {
    // Before the pages, so that their arrow keys don't see these too.
    player_shortcuts(app, ui);
    mouse_navigation(app, ui);
    if app.now_playing {
        let layout = app.theme.layout;
        let cover = layout.now_playing == crate::theme::NowPlaying::Cover;
        // Spotify's full screen keeps the player along the bottom.
        if cover && layout.player_bar == PlayerBar::Docked {
            egui::Panel::bottom("dock")
                .exact_size(dock::height(app))
                .resizable(false)
                .show_separator_line(false)
                .frame(egui::Frame::new().fill(app.theme.crust))
                .show(ui, |ui| dock::show(app, ui));
        }
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let full = ui.max_rect();
            if cover {
                now_playing::show_cover(app, ui);
            } else {
                now_playing::show(app, ui);
            }
            signal::show(app, ui, full);
        });
        playlist_dialog::show(app, ui.ctx());
        playlist_dialog::show_new(app, ui.ctx());
        playlist_dialog::show_delete(app, ui.ctx());
        detail::notes_window(app, ui.ctx());
        pairing::show(app, ui.ctx());
        shortcuts(app, ui);
        return;
    }
    let narrow = ui.available_width() < NARROW;
    let collapsed = app.sidebar_collapsed || narrow;
    let theme = app.theme.clone();
    let layout = theme.layout;
    let cards = layout.window == Window::Cards;
    // Between the cards, and around them.
    let gap = if cards { GAP } else { 0.0 };
    let margin = |left: f32, right: f32, top: f32| {
        egui::Frame::new()
            .fill(theme.crust)
            .inner_margin(egui::Margin {
                left: left as i8,
                right: right as i8,
                top: top as i8,
                bottom: gap as i8,
            })
    };

    let window_bar = layout.top_bar == TopBar::Window;
    if window_bar {
        egui::Panel::top("window-bar")
            .exact_size(window_bar::HEIGHT)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(theme.crust))
            .show(ui, |ui| window_bar::show(app, ui));
    }
    // Under the window's bar, the cards keep only the gap.
    let top = if window_bar { 0.0 } else { gap };
    if layout.player_bar == PlayerBar::Docked {
        egui::Panel::bottom("dock")
            .exact_size(dock::height(app))
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(theme.crust))
            .show(ui, |ui| dock::show(app, ui));
    }

    let library = layout.sidebar == Sidebar::Library;
    let width = match (library, collapsed) {
        (true, true) => library::COLLAPSED,
        (true, false) => library::WIDTH,
        (false, true) => 68.0,
        (false, false) => 232.0,
    };
    egui::Panel::left("sidebar")
        .exact_size(width + gap)
        .resizable(false)
        .show_separator_line(false)
        .frame(margin(gap, 0.0, top))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            if cards {
                ui.painter().rect_filled(rect, RADIUS, theme.base);
            }
            if library {
                library::show(app, ui, collapsed);
            } else {
                sidebar::show(app, ui, collapsed);
            }
            if cards {
                round_off(ui, rect, theme.crust);
            }
        });

    // The column beside the page: the song playing, or the queue.
    let side = layout.panel == Panel::Side && !narrow;
    if side && let Some(shows) = app.side {
        egui::Panel::right("side")
            .exact_size(side::WIDTH + gap)
            .resizable(false)
            .show_separator_line(false)
            .frame(margin(0.0, gap, top))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                ui.painter().rect_filled(rect, RADIUS, theme.base);
                side::show(app, ui, shows);
                if cards {
                    round_off(ui, rect, theme.crust);
                }
            });
    }

    egui::CentralPanel::no_frame()
        .frame(if cards {
            margin(gap, gap, top)
        } else {
            egui::Frame::new().fill(theme.base)
        })
        .show(ui, |ui| {
            let main = ui.max_rect();
            if cards {
                ui.painter().rect_filled(main, RADIUS, theme.base);
            }
            let lyrics = app.lyrics_page && layout.lyrics == crate::theme::Lyrics::Page;
            let scrolled = if lyrics {
                lyrics_page(app, ui, main);
                0.0
            } else {
                page(app, ui, narrow)
            };
            if !cards && !library {
                // The sidebar's right hairline, over the page: a cover's
                // colours at the top of it would hide it otherwise.
                ui.painter()
                    .vline(main.left(), main.y_range(), Stroke::new(1.0, theme.line()));
            }
            if !lyrics {
                top_bar(app, ui, main, scrolled);
            }
            if layout.panel == Panel::Drawer || narrow {
                drawer::show(app, ui, main);
            }
            if layout.player_bar == PlayerBar::Floating {
                player_bar::show(app, ui, main);
            }
            signal::show(app, ui, main);
            toast(app, ui, main);
            if cards {
                round_off(ui, main, theme.crust);
            }
        });

    playlist_dialog::show(app, ui.ctx());
    playlist_dialog::show_new(app, ui.ctx());
    playlist_dialog::show_delete(app, ui.ctx());
    detail::notes_window(app, ui.ctx());
    pairing::show(app, ui.ctx());
    shortcuts(app, ui);
}

/// The cover colours behind album and playlist pages, fitted to the theme.
pub fn backdrop(app: &mut App) -> Option<[Color32; 3]> {
    let url = match &app.page {
        Page::Album(id) => match app.albums.get(id) {
            Some(crate::app::Loaded::Ready((album, _))) => album.artwork.as_ref()?.url(96),
            _ => return None,
        },
        Page::Playlist(id) => app.playlist(id)?.artwork.as_ref()?.url(96),
        Page::Artist(id) => match app.artists.get(id) {
            Some(crate::app::Loaded::Ready(artist)) => artist.artist.artwork.as_ref()?.url(96),
            _ => return None,
        },
        _ => return None,
    };
    let light = app.theme.light;
    app.palette(&url)
        .map(|p| p.map(|c| crate::palette::fit_header(c, light)))
}

/// The page under the top bar; returns how far it is scrolled.
fn page(app: &mut App, ui: &mut Ui, narrow: bool) -> f32 {
    let pad = if narrow { 24.0 } else { 40.0 };
    match app.page.clone() {
        Page::Songs => songs::show(app, ui, pad),
        Page::Album(id) => detail::album(app, ui, pad, &id),
        Page::Playlist(id) => detail::playlist(app, ui, pad, &id),
        Page::Search(query) => search::show(app, ui, pad, &query),
        Page::Artist(id) => artist::show(app, ui, pad, &id),
        Page::LibraryArtist(id) => library_artist::show(app, ui, pad, &id),
        Page::Home => home::show(app, ui, pad),
        Page::Radio => home::radio(app, ui, pad),
        Page::New => home::new_music(app, ui, pad),
        Page::Settings => settings::show(app, ui, pad),
        Page::Albums => grid::albums(app, ui, pad),
        Page::Artists => grid::artists(app, ui, pad),
        Page::RecentlyAdded => grid::recently_added(app, ui, pad),
        Page::Shelf(which, title) => grid::shelf(app, ui, pad, which, &title),
        _ => placeholder(app, ui, pad),
    }
}

fn placeholder(app: &mut App, ui: &mut Ui, pad: f32) -> f32 {
    let theme = &app.theme;
    let rect = ui.max_rect();
    glow(
        ui,
        theme,
        Rect::from_min_size(rect.min, vec2(rect.width(), 460.0)),
    );
    let title = app.page_title();
    let left = rect.left() + pad;
    let galley = text(
        ui,
        &title,
        font(30.0, Weight::Bold),
        theme.text,
        rect.width(),
    );
    ui.painter()
        .galley(pos2(left, rect.top() + 70.0), galley, theme.text);
    let note = text(
        ui,
        "This page comes in a later step.",
        font(13.0, Weight::Regular),
        theme.subtext0,
        rect.width(),
    );
    ui.painter()
        .galley(pos2(left, rect.top() + 120.0), note, theme.subtext0);
    0.0
}

fn top_bar(app: &mut App, ui: &mut Ui, main: Rect, scrolled: f32) {
    let theme = app.theme.clone();
    // Under a banner, the page draws Spotify's bar itself, with its Play.
    if theme.layout.header == crate::theme::Header::Banner
        && matches!(
            app.page,
            Page::Album(_) | Page::Playlist(_) | Page::Artist(_)
        )
    {
        return;
    }
    let bar = Rect::from_min_size(main.min, vec2(main.width(), TOP_BAR));
    let solid =
        ui.ctx()
            .animate_bool_with_time(egui::Id::unique("top-bar-solid"), scrolled > 24.0, 0.2);
    if solid > 0.0 {
        let painter = ui.painter();
        painter.rect_filled(bar, 0.0, alpha(theme.base, 0.96 * solid));
        painter.hline(
            bar.x_range(),
            bar.bottom() - 0.5,
            Stroke::new(1.0, alpha(theme.line(), solid)),
        );
        let title = text(
            ui,
            &app.page_title(),
            font(13.0, Weight::SemiBold),
            theme.text,
            400.0,
        );
        let pos = bar.center() - title.size() / 2.0;
        ui.painter().galley(pos, title, alpha(theme.text, solid));
    }
    // Back, forward and the drawer's buttons are in the window's bar then.
    if theme.layout.top_bar == TopBar::Window {
        return;
    }

    let mut left = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_size(
                bar.min + vec2(18.0, 10.0),
                vec2(200.0, 32.0),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    left.spacing_mut().item_spacing.x = 4.0;
    if icon_button(
        &mut left,
        &theme,
        icons::LEFT,
        32.0,
        18.0,
        theme.subtext0,
        app.can_go_back(),
    )
    .on_hover_text("Back")
    .clicked()
    {
        app.go_back();
    }
    if icon_button(
        &mut left,
        &theme,
        icons::RIGHT,
        32.0,
        18.0,
        theme.subtext0,
        app.can_go_forward(),
    )
    .on_hover_text("Forward")
    .clicked()
    {
        app.go_forward();
    }
    let (sep, _) = left.allocate_exact_size(vec2(9.0, 16.0), Sense::hover());
    left.painter().vline(
        sep.center().x,
        sep.y_range(),
        Stroke::new(1.0, theme.line()),
    );
    if icon_button(
        &mut left,
        &theme,
        icons::PANEL,
        32.0,
        17.0,
        theme.subtext0,
        true,
    )
    .on_hover_text(if app.sidebar_collapsed {
        "Show Sidebar"
    } else {
        "Hide Sidebar"
    })
    .clicked()
    {
        app.sidebar_collapsed = !app.sidebar_collapsed;
    }

    let mut right = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(
                pos2(bar.right() - 18.0 - 104.0, bar.top() + 10.0),
                pos2(bar.right() - 18.0, bar.top() + 42.0),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    right.spacing_mut().item_spacing.x = 4.0;
    let lyrics_color = if app.drawer_shows(true) {
        theme.accent
    } else {
        theme.subtext0
    };
    if icon_button(
        &mut right,
        &theme,
        icons::LYRICS,
        32.0,
        17.0,
        lyrics_color,
        true,
    )
    .on_hover_text("Lyrics")
    .clicked()
    {
        app.toggle_drawer(true);
    }
    let queue_color = if app.drawer_shows(false) {
        theme.accent
    } else {
        theme.subtext0
    };
    if icon_button(
        &mut right,
        &theme,
        icons::QUEUE,
        32.0,
        17.0,
        queue_color,
        true,
    )
    .on_hover_text("Playing next")
    .clicked()
    {
        app.toggle_drawer(false);
    }
    let wave_color = if app.signal_open == Some(SignalAnchor::TopBar) {
        theme.accent
    } else {
        theme.subtext0
    };
    let wave = icon_button(
        &mut right,
        &theme,
        icons::WAVE,
        32.0,
        17.0,
        wave_color,
        true,
    )
    .on_hover_text("Signal path");
    signal::remember_anchor(&right, SignalAnchor::TopBar, wave.rect);
    if wave.clicked() {
        app.signal_open = match app.signal_open {
            Some(SignalAnchor::TopBar) => None,
            _ => app.now.is_some().then_some(SignalAnchor::TopBar),
        };
    }
}

/// How tall Spotify's bar over a scrolled page is.
pub const BANNER_BAR: f32 = 64.0;

/// Spotify's bar over a page scrolled past its banner, `shown` of the way
/// in: the page's colour, a round Play and the title, over the page. A
/// child of `ui` on the bar's layer, for what goes under it, and whether
/// Play was clicked.
pub fn banner_bar(
    ui: &mut Ui,
    theme: &Theme,
    main: Rect,
    color: Color32,
    title: &str,
    shown: f32,
) -> (Ui, bool) {
    let bar = Rect::from_min_size(main.min, vec2(main.width(), BANNER_BAR));
    let mut layer = ui.new_child(egui::UiBuilder::new().max_rect(main).layer_id(
        egui::LayerId::new(egui::Order::Middle, egui::Id::unique("banner-bar")),
    ));
    if shown <= 0.0 {
        return (layer, false);
    }
    layer.set_opacity(shown);
    let fill = crate::color::mix(Color32::BLACK, color, 0.6);
    layer.painter().rect_filled(bar, 0.0, fill);
    // The bar takes the clicks over it.
    layer.interact(bar, egui::Id::unique("banner-bar-bg"), Sense::click());
    let play = Rect::from_center_size(
        pos2(main.left() + 40.0 + 24.0, bar.center().y),
        vec2(48.0, 48.0),
    );
    let response = layer
        .interact(play, egui::Id::unique("banner-bar-play"), Sense::click())
        .on_hover_text("Play")
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let accent = if response.hovered() {
        crate::color::mix(theme.accent, Color32::WHITE, 0.85)
    } else {
        theme.accent
    };
    layer.painter().circle_filled(play.center(), 24.0, accent);
    paint_icon(
        &layer,
        icons::PLAY_FILL,
        play.center(),
        20.0,
        theme.on_accent(),
    );
    let g = text(
        &layer,
        title,
        font(24.0, Weight::Bold),
        Color32::WHITE,
        main.width() - 200.0,
    );
    paint_text(
        &layer,
        g,
        play.right() + 12.0,
        bar.center().y,
        egui::Align::Min,
        Color32::WHITE,
    );
    (layer, response.clicked())
}

/// Paint `rect`'s corners over in `color`, so that it reads as a card with
/// [`RADIUS`] corners whatever was drawn in it. Above the page, under the
/// player bar, menus and windows.
fn round_off(ui: &Ui, rect: Rect, color: Color32) {
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::unique(("round-off", rect.min.x as i32, rect.min.y as i32)),
    ));
    let r = RADIUS;
    let corners = [
        (rect.left_top(), vec2(1.0, 1.0)),
        (rect.right_top(), vec2(-1.0, 1.0)),
        (rect.left_bottom(), vec2(1.0, -1.0)),
        (rect.right_bottom(), vec2(-1.0, -1.0)),
    ];
    for (corner, inward) in corners {
        let center = corner + inward * r;
        let mut mesh = Mesh::default();
        mesh.colored_vertex(corner, color);
        let steps = 8;
        for i in 0..=steps {
            let a = i as f32 / steps as f32 * std::f32::consts::FRAC_PI_2;
            // From the corner's one side round to the other.
            let point = center - vec2(inward.x * a.cos(), inward.y * a.sin()) * r;
            mesh.colored_vertex(point, color);
            if i > 0 {
                mesh.add_triangle(0, i, i + 1);
            }
        }
        painter.add(mesh);
    }
}

/// The lyrics over the whole page, on the colour of the song's cover.
fn lyrics_page(app: &mut App, ui: &mut Ui, main: Rect) {
    let theme = app.theme.clone();
    let shown = app.shown();
    let color = shown
        .track
        .as_ref()
        .and_then(thumb_url)
        .and_then(|url| app.palette(&url))
        // Darker than a header's band, for white words.
        .map(|[c, _, _]| {
            crate::color::mix(crate::palette::fit_header(c, false), Color32::BLACK, 0.62)
        })
        .unwrap_or(theme.surface1);
    ui.painter().rect_filled(main, 0.0, color);
    // Over the cover's colour, the words are white.
    let on_cover = Theme {
        text: Color32::WHITE,
        subtext0: Color32::from_white_alpha(200),
        ..theme
    };
    let pad = if main.width() < 900.0 {
        32.0
    } else {
        0.18 * main.width()
    };
    let body = Rect::from_min_max(
        pos2(main.left() + pad, main.top() + 40.0),
        pos2(
            (main.right() - pad).min(main.left() + pad + 1000.0),
            main.bottom(),
        ),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
    child.set_clip_rect(main);
    lyrics::show(
        app,
        &mut child,
        &on_cover,
        body,
        34.0,
        body.height(),
        "page-lyrics",
    );
}

fn toast(app: &mut App, ui: &mut Ui, main: Rect) {
    let Some((message, since)) = app.toast.clone() else {
        return;
    };
    let now = ui.input(|i| i.time);
    if now - since > 5.0 {
        app.toast = None;
        return;
    }
    ui.ctx()
        .request_repaint_after(Duration::from_secs_f64(5.0 - (now - since)));
    let theme = &app.theme;
    let galley = text(ui, &message, font(13.0, Weight::Regular), theme.text, 560.0);
    let size = galley.size() + vec2(28.0, 18.0);
    let rect = Rect::from_center_size(
        pos2(
            main.center().x,
            main.bottom() - 14.0 - 44.0 - 16.0 - size.y / 2.0,
        ),
        size,
    );
    let painter = ui.painter();
    painter.add(
        egui::Shadow {
            offset: [0, 12],
            blur: 32,
            spread: 0,
            color: Color32::from_black_alpha(110),
        }
        .as_shape(rect, 10.0),
    );
    painter.rect(
        rect,
        10.0,
        theme.mantle,
        Stroke::new(1.0, theme.line()),
        egui::StrokeKind::Inside,
    );
    painter.galley(rect.min + vec2(14.0, 9.0), galley, theme.text);
}

/// Whether the keys are someone else's: a field being typed in, or a dialog
/// over the page.
pub fn keys_taken(ctx: &egui::Context) -> bool {
    ctx.egui_wants_keyboard_input() || ctx.memory(|m| m.top_modal_layer()).is_some()
}

fn shortcuts(app: &mut App, ui: &mut Ui) {
    if keys_taken(ui.ctx()) {
        return;
    }
    if ui.input(|i| i.key_pressed(egui::Key::Space)) {
        app.toggle_play();
    }
    // Escape closes the popover first, then Now Playing, then the drawer.
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        if app.signal_open.is_some() {
            app.signal_open = None;
        } else if app.now_playing {
            app.now_playing = false;
        } else {
            app.drawer_open = false;
        }
    }
}

/// The mouse's back and forward buttons go back and forward between pages,
/// as in a browser.
fn mouse_navigation(app: &mut App, ui: &Ui) {
    let (back, forward) = ui.input(|i| {
        (
            i.pointer.button_pressed(egui::PointerButton::Extra1),
            i.pointer.button_pressed(egui::PointerButton::Extra2),
        )
    });
    if back {
        app.go_back();
    }
    if forward {
        app.go_forward();
    }
}

/// The player's keys, as Apple Music's: Ctrl with the arrows for the next
/// or previous song and the volume, Shift with them to move in the song.
fn player_shortcuts(app: &mut App, ui: &mut Ui) {
    if keys_taken(ui.ctx()) {
        return;
    }
    use egui::{Key, Modifiers};
    let ctrl = Modifiers::COMMAND;
    let shift = Modifiers::SHIFT;
    let [
        next,
        previous,
        louder,
        quieter,
        mute,
        shuffle,
        repeat,
        lyrics,
        current,
        ahead,
        back,
    ] = ui.input_mut(|i| {
        [
            (ctrl, Key::ArrowRight),
            (ctrl, Key::ArrowLeft),
            (ctrl, Key::ArrowUp),
            (ctrl, Key::ArrowDown),
            (ctrl, Key::M),
            (ctrl, Key::S),
            (ctrl, Key::R),
            (ctrl, Key::L),
            (ctrl, Key::J),
            (shift, Key::ArrowRight),
            (shift, Key::ArrowLeft),
        ]
        .map(|(modifiers, key)| i.consume_key(modifiers, key))
    });
    if next {
        app.next();
    }
    if previous {
        app.previous();
    }
    if louder || quieter {
        let step = if louder { VOLUME_STEP } else { -VOLUME_STEP };
        let shown = app.shown();
        let from = if shown.muted {
            0.0
        } else {
            shown.volume.unwrap_or(app.volume)
        };
        app.set_volume(from + step);
    }
    if mute {
        app.toggle_mute();
    }
    if shuffle {
        app.toggle_shuffle();
    }
    if repeat {
        app.cycle_repeat();
    }
    if lyrics {
        app.toggle_drawer(true);
    }
    if current {
        app.go_to_playing();
    }
    // A live stream has no length, and doesn't move.
    if (ahead || back)
        && let Some(length) = app.shown().duration
    {
        let at = app.shown().position;
        let to = if ahead {
            at + SEEK_STEP
        } else {
            at.saturating_sub(SEEK_STEP)
        };
        if to < length {
            app.seek(to);
        } else {
            app.next();
        }
    }
}

const VOLUME_STEP: f32 = 0.05;
const SEEK_STEP: std::time::Duration = std::time::Duration::from_secs(10);

/// What a page's search did this frame.
pub(super) struct PageSearch {
    /// Where the magnifier or the field begins, for what sits left of it.
    pub left: f32,
    /// The words searched for changed.
    pub changed: bool,
}

/// A page's search, ending at `right`: a magnifier that opens into a field
/// when clicked, or with Ctrl+F. Left empty, or emptied with Escape, it folds
/// back. The words go in `App::page_search`.
pub(super) fn page_search(
    app: &mut App,
    ui: &mut Ui,
    theme: &Theme,
    right: f32,
    top: f32,
    hint: &str,
) -> PageSearch {
    let id = egui::Id::unique("page-search");
    let mut changed = false;
    let shortcut = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F));
    let mut focus = false;
    if shortcut {
        app.page_search_open = true;
        focus = true;
    }
    let open = ui
        .ctx()
        .animate_bool_with_time(id.with("open"), app.page_search_open, 0.18);
    let eased = 1.0 - (1.0 - open).powi(3);
    let height = 32.0;
    let width = height + (260.0 - height) * eased;
    let field = Rect::from_min_size(pos2(right - width, top), vec2(width, height));

    if open == 0.0 {
        // Closed: the magnifier alone.
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field));
        if icon_button(
            &mut child,
            theme,
            icons::SEARCH,
            height,
            15.0,
            theme.subtext0,
            true,
        )
        .on_hover_text(hint)
        .clicked()
        {
            app.page_search_open = true;
            focus = true;
        }
        if focus {
            // Taken by the field once it shows, from the next frame.
            ui.data_mut(|d| d.insert_temp(id.with("focus"), true));
        }
        return PageSearch {
            left: field.left(),
            changed,
        };
    }
    let focus = focus
        || ui
            .data_mut(|d| d.remove_temp::<bool>(id.with("focus")))
            .unwrap_or(false);

    ui.painter().rect(
        field,
        height / 2.0 * (1.0 - eased) + 9.0 * eased,
        theme.text_a(0.04 * eased),
        egui::Stroke::new(1.0, theme.line().gamma_multiply(eased)),
        egui::StrokeKind::Inside,
    );
    let icon_x = field.left() + height / 2.0 + (15.0 - height / 2.0) * eased;
    paint_icon(
        ui,
        icons::SEARCH,
        pos2(icon_x, field.center().y),
        14.0,
        theme.muted(),
    );
    let body = font(13.0, Weight::Regular);
    let line = text(ui, "S", body.clone(), theme.text, 100.0).size().y;
    let clear = !app.page_search.is_empty();
    let left = field.left() + 32.0;
    let edit = Rect::from_min_max(
        pos2(left, field.center().y - line / 2.0),
        pos2(
            (field.right() - if clear { 32.0 } else { 12.0 }).max(left + 1.0),
            field.center().y + line / 2.0,
        ),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(edit));
    // While it opens, the words show only as far as the field reaches.
    child.set_clip_rect(edit.intersect(field.shrink(1.0)).intersect(ui.clip_rect()));
    let response = child.add(
        egui::TextEdit::singleline(&mut app.page_search)
            .id(id)
            .hint_text(egui::RichText::new(hint).color(theme.muted()))
            .font(body)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .desired_width(edit.width()),
    );
    if focus {
        response.request_focus();
    }
    if response.has_focus()
        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        changed |= !app.page_search.is_empty();
        app.page_search.clear();
        app.page_search_open = false;
        response.surrender_focus();
    }
    changed |= response.changed();
    // Left empty, it folds back to the magnifier.
    if response.lost_focus() && app.page_search.trim().is_empty() {
        app.page_search.clear();
        app.page_search_open = false;
    }
    if clear {
        let button = Rect::from_center_size(
            pos2(field.right() - 16.0, field.center().y),
            vec2(24.0, 24.0),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(button));
        if icon_button(&mut child, theme, icons::X, 24.0, 12.0, theme.muted(), true)
            .on_hover_text("Clear")
            .clicked()
        {
            app.page_search.clear();
            changed = true;
            response.request_focus();
        }
    }
    PageSearch {
        left: field.left(),
        changed,
    }
}

/// Note the song the pointer is on this frame, for the app to fetch ahead.
pub(super) fn pointer_on(ui: &Ui, track: &music_core::model::Track) {
    ui.ctx().data_mut(|d| {
        d.insert_temp(egui::Id::unique("pointer-on-song"), Some(track.clone()));
    });
}

/// The song the pointer was on this frame, if any.
pub fn song_under_pointer(ctx: &egui::Context) -> Option<music_core::model::Track> {
    ctx.data_mut(|d| {
        d.remove_temp::<Option<music_core::model::Track>>(egui::Id::unique("pointer-on-song"))
    })
    .flatten()
}

// ---- Shared drawing helpers ----

/// One line of text, cut with an ellipsis past `max_width`.
pub fn text(ui: &Ui, s: &str, font: FontId, color: Color32, max_width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::simple_singleline(s.to_owned(), font, color);
    job.wrap = TextWrapping::truncate_at_width(max_width.max(0.0));
    ui.painter().layout_job(job)
}

/// Text vertically centred on `y`, from `x`, with `align` horizontally.
pub fn paint_text(
    ui: &Ui,
    galley: Arc<Galley>,
    x: f32,
    y: f32,
    align: egui::Align,
    color: Color32,
) {
    let size = galley.size();
    let left = match align {
        egui::Align::Min => x,
        egui::Align::Center => x - size.x / 2.0,
        egui::Align::Max => x - size.x,
    };
    ui.painter()
        .galley(pos2(left, y - size.y / 2.0), galley, color);
}

pub fn paint_icon(ui: &Ui, icon: Icon, center: Pos2, size: f32, color: Color32) {
    icon.image(size, color)
        .paint_at(ui, Rect::from_center_size(center, Vec2::splat(size)));
}

/// A square icon button: no background until hovered.
pub fn icon_button(
    ui: &mut Ui,
    theme: &Theme,
    icon: Icon,
    size: f32,
    glyph: f32,
    color: Color32,
    enabled: bool,
) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), sense);
    let hovered = enabled && response.hovered();
    if hovered {
        let fill = if response.is_pointer_button_down_on() {
            theme.press()
        } else {
            theme.hover()
        };
        ui.painter().rect_filled(rect, 8.0, fill);
    }
    let color = if !enabled {
        alpha(color, 0.35 * color.a() as f32 / 255.0)
    } else if hovered {
        theme.text
    } else {
        color
    };
    paint_icon(ui, icon, rect.center(), glyph, color);
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

/// The "E" badge after explicit titles.
pub fn explicit_badge(ui: &Ui, theme: &Theme, center: Pos2) {
    let rect = Rect::from_center_size(center, Vec2::splat(12.0));
    ui.painter().rect_filled(
        rect,
        2.5,
        crate::color::mix(theme.subtext0, theme.base, 0.55),
    );
    let e = text(ui, "E", font(8.5, Weight::Bold), theme.base, 12.0);
    paint_text(ui, e, center.x, center.y, egui::Align::Center, theme.base);
}

/// Whether `track` plays as Spatial Audio, its Dolby Atmos mix.
pub fn plays_in_atmos(track: &music_core::model::Track) -> bool {
    track
        .quality
        .as_ref()
        .is_some_and(|q| q.codec == music_core::model::Codec::Other("Dolby Atmos".into()))
}

/// The Dolby double D, centred on `center`, `size` high: two half discs
/// whose curves face each other, their flat sides outside.
pub fn dolby_logo(ui: &Ui, center: Pos2, size: f32, color: Color32) {
    let r = size / 2.0;
    let gap = size * 0.08;
    for side in [-1.0f32, 1.0] {
        let flat = center.x + side * (gap + r);
        let points: Vec<Pos2> = (0..=12)
            .map(|i| {
                let a = std::f32::consts::PI * (i as f32 / 12.0 - 0.5);
                pos2(flat - side * r * a.cos(), center.y + r * a.sin())
            })
            .collect();
        ui.painter()
            .add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
    }
}

/// The badge a track's quality earns, as Apple Music gives them: Lossless or
/// Dolby Atmos, none for a lossy stream.
pub fn quality_badge(track: &music_core::model::Track) -> Option<&'static str> {
    if plays_in_atmos(track) {
        Some("Dolby Atmos")
    } else if track.quality.as_ref()?.codec.is_lossless() {
        Some("Lossless")
    } else {
        None
    }
}

/// `AAC 256`, `FLAC 24/96`… from what the provider says it delivers.
pub fn quality_label(track: &music_core::model::Track) -> Option<(String, bool)> {
    use music_core::model::Codec;
    let q = track.quality.as_ref()?;
    let codec = match &q.codec {
        Codec::Aac => "AAC",
        Codec::Alac => "ALAC",
        Codec::Flac => "FLAC",
        Codec::Mp3 => "MP3",
        Codec::Opus => "Opus",
        Codec::Vorbis => "Vorbis",
        Codec::Wav => "WAV",
        Codec::Other(name) => name.as_str(),
    };
    let detail = match (q.bit_depth, q.sample_rate, q.bitrate_kbps) {
        (Some(bits), Some(rate), _) if q.codec.is_lossless() => {
            format!(" {bits}/{}", trim_khz(rate))
        }
        (_, _, Some(kbps)) => format!(" {kbps}"),
        _ => String::new(),
    };
    Some((format!("{codec}{detail}"), q.codec.is_lossless()))
}

fn trim_khz(rate: u32) -> String {
    let khz = f64::from(rate) / 1000.0;
    if khz.fract() == 0.0 {
        format!("{khz:.0}")
    } else {
        format!("{khz:.1}")
    }
}

pub fn format_time(d: Duration) -> String {
    let s = d.as_secs();
    format!("{}:{:02}", s / 60, s % 60)
}

/// `2026-03-27` → `MAR 27, 2026`, as Cider dates releases.
pub fn format_date(iso: &str) -> Option<String> {
    const MONTHS: [&str; 12] = [
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ];
    let mut parts = iso.splitn(3, '-');
    let year = parts.next()?;
    let month: usize = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.get(..2)?.parse().ok()?;
    Some(format!(
        "{} {day}, {year}",
        MONTHS.get(month.checked_sub(1)?)?
    ))
}

/// Artwork from a URL template, as large as needed on this screen.
pub fn artwork(ui: &Ui, url: Option<String>, rect: Rect, radius: f32, theme: &Theme) {
    ui.painter().rect_filled(rect, radius, theme.surface0);
    if let Some(url) = url {
        crate::covers::seen(ui.ctx(), &url);
        egui::Image::new(url)
            .corner_radius(CornerRadius::same(radius as u8))
            .show_loading_spinner(false)
            .paint_at(ui, rect);
    }
}

/// The animated cover at `master` over the still one in `rect`, once it
/// plays. `points` is how wide it is meant to be, for the video's size: the
/// still cover may shrink and grow around it.
pub(super) fn motion_over(
    app: &mut App,
    ui: &Ui,
    slot: &'static str,
    master: Option<&str>,
    rect: Rect,
    radius: f32,
    points: f32,
) {
    let Some(master) = master else { return };
    let pixels = points * ui.ctx().pixels_per_point();
    if let Some(texture) = app.motion_frame(slot, master, pixels, ui.ctx()) {
        egui::Image::from_texture(texture)
            .corner_radius(CornerRadius::same(radius as u8))
            .paint_at(ui, rect);
    }
}

/// Artwork URLs share one size so the list, the sidebar and the player bar hit
/// the same cached image.
pub fn thumb_url(track: &music_core::model::Track) -> Option<String> {
    track.artwork.as_ref().map(|a| a.url(120))
}

/// A radial blob of colour: centre and radii as fractions of the painted rect,
/// opacity at the centre, and where (as a fraction of the radius) it fades out.
pub struct Blob {
    pub center: Vec2,
    pub radii: Vec2,
    pub color: Color32,
    pub alpha: f32,
    pub fade: f32,
}

/// Blobs over `base` as a coarse mesh, with per-vertex colours. With
/// `fade_to_base`, a vertical gradient then dissolves them into `base`: clear to
/// 30 %, `base` at 45 % opacity by 62 %, solid at the bottom.
pub fn wash(ui: &Ui, rect: Rect, base: Color32, blobs: &[Blob], fade_to_base: bool) {
    let veil = |f: Vec2| {
        if !fade_to_base || f.y < 0.30 {
            0.0
        } else if f.y < 0.62 {
            0.45 * (f.y - 0.30) / (0.62 - 0.30)
        } else {
            0.45 + 0.55 * (f.y - 0.62) / 0.38
        }
    };
    wash_with(ui, rect, base, blobs, veil);
}

/// Blobs over `base`, then `base` again at the opacity `veil` gives for each
/// point (as fractions of the rect).
pub fn wash_with(ui: &Ui, rect: Rect, base: Color32, blobs: &[Blob], veil: impl Fn(Vec2) -> f32) {
    let (cols, rows) = (24, 16);
    let mut mesh = Mesh::default();
    for row in 0..=rows {
        for col in 0..=cols {
            let f = vec2(col as f32 / cols as f32, row as f32 / rows as f32);
            let mut color = base;
            for blob in blobs {
                let d = ((f - blob.center) / blob.radii).length();
                let t = (1.0 - d / blob.fade).clamp(0.0, 1.0);
                color = over(alpha(blob.color, blob.alpha * t), color);
            }
            let a = veil(f);
            if a > 0.0 {
                color = over(alpha(base, a), color);
            }
            mesh.colored_vertex(rect.min + f * rect.size(), color);
        }
    }
    for row in 0..rows {
        for col in 0..cols {
            let i = (row * (cols + 1) + col) as u32;
            let w = (cols + 1) as u32;
            mesh.add_triangle(i, i + 1, i + w);
            mesh.add_triangle(i + 1, i + w + 1, i + w);
        }
    }
    ui.painter().add(mesh);
}

/// The accent's glow at the top of pages without a cover: two radial gradients
/// (accent at 26 % from the top left, mauve at 20 % from the top centre).
pub fn glow(ui: &Ui, theme: &Theme, rect: Rect) {
    if theme.layout.glow == crate::theme::Glow::Grey {
        let blobs = [Blob {
            center: vec2(0.5, -0.6),
            radii: vec2(1.4, 1.3),
            color: theme.surface2,
            alpha: 0.55,
            fade: 0.9,
        }];
        wash(ui, rect, theme.base, &blobs, false);
        return;
    }
    let blobs = [
        Blob {
            center: vec2(0.08, -0.10),
            radii: vec2(0.55, 0.90),
            color: theme.accent,
            alpha: 0.26,
            fade: 0.70,
        },
        Blob {
            center: vec2(0.62, -0.25),
            radii: vec2(0.45, 0.80),
            color: theme.mauve,
            alpha: 0.20,
            fade: 0.72,
        },
    ];
    wash(ui, rect, theme.base, &blobs, false);
}

/// The Play and Shuffle buttons of a page, from `at`. Returns `Some(shuffle)`
/// when one was clicked.
pub fn page_buttons(
    ui: &mut Ui,
    theme: &Theme,
    at: Pos2,
    salt: &str,
    add: bool,
) -> Option<PageButton> {
    let play = page_button(ui, theme, at, icons::PLAY_FILL, "Play", salt);
    let shuffle_at = pos2(play.rect.right() + 10.0, at.y);
    let shuffle = page_button(ui, theme, shuffle_at, icons::SHUFFLE, "Shuffle", salt);
    let add = add.then(|| {
        let add_at = pos2(shuffle.rect.right() + 10.0, at.y);
        page_button(ui, theme, add_at, icons::PLUS, "Add", salt)
    });
    if play.clicked() {
        Some(PageButton::Play)
    } else if shuffle.clicked() {
        Some(PageButton::Shuffle)
    } else if add.is_some_and(|a| a.clicked()) {
        Some(PageButton::Add)
    } else {
        None
    }
}

/// A button under a page's title.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PageButton {
    Play,
    Shuffle,
    /// Add to Library.
    Add,
}

/// A 36 px button on a faint fill, as Cider draws Play and Shuffle: as wide as
/// its icon and label, with both centred in it.
fn page_button(
    ui: &mut Ui,
    theme: &Theme,
    at: Pos2,
    icon: Icon,
    label: &str,
    salt: &str,
) -> egui::Response {
    let (icon_size, gap, padding) = (14.0, 8.0, 24.0);
    let galley = text(ui, label, font(13.0, Weight::Medium), theme.text, 300.0);
    let content = icon_size + gap + galley.size().x;
    let rect = Rect::from_min_size(at, vec2(content + 2.0 * padding, 36.0));
    let response = ui.interact(
        rect,
        egui::Id::unique(("page-button", salt, label)),
        Sense::click(),
    );
    let fill = theme.text_a(if response.hovered() { 0.14 } else { 0.09 });
    let shown = if response.is_pointer_button_down_on() {
        rect.shrink(0.4)
    } else {
        rect
    };
    ui.painter().rect_filled(shown, 9.0, fill);
    let x = rect.center().x - content / 2.0;
    paint_icon(
        ui,
        icon,
        pos2(x + icon_size / 2.0, rect.center().y),
        icon_size,
        theme.text,
    );
    paint_text(
        ui,
        galley,
        x + icon_size + gap,
        rect.center().y,
        egui::Align::Min,
        theme.text,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// `top` composited over `bottom`, both premultiplied.
fn over(top: Color32, bottom: Color32) -> Color32 {
    let a = top.a() as f32 / 255.0;
    let mix = |t: u8, b: u8| (f32::from(t) + f32::from(b) * (1.0 - a)).round().min(255.0) as u8;
    Color32::from_rgba_premultiplied(
        mix(top.r(), bottom.r()),
        mix(top.g(), bottom.g()),
        mix(top.b(), bottom.b()),
        mix(top.a(), bottom.a()),
    )
}
