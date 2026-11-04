//! The sidebar: search, navigation, library, playlists and the account row.

use egui::{Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use opium_core::model::Collection;

use crate::app::{App, CollectionChoice, Page};
use crate::color::{alpha, mix};
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::{artwork, icon_button, paint_icon, paint_text, text};

pub fn show(app: &mut App, ui: &mut Ui, collapsed: bool) {
    let theme = app.theme.clone();
    let full = ui.max_rect();

    search(app, ui, &theme, full, collapsed);

    let account_height = 52.0 + 16.0;
    let nav = Rect::from_min_max(
        pos2(full.left(), full.top() + 50.0),
        pos2(full.right(), full.bottom() - account_height),
    );
    let mut list = ui.new_child(egui::UiBuilder::new().max_rect(nav));
    let mut new_playlist = false;
    let mut chosen: Option<(Collection, CollectionChoice)> = None;
    egui::ScrollArea::vertical()
        .id_salt("sidebar-scroll")
        .auto_shrink(false)
        .show_viewport(&mut list, |ui, viewport| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let scrolled = ui.cursor().top();
            ui.add_space(0.0);
            let mut row = |ui: &mut Ui, page: Page, icon: Icon, label: &str| {
                let active = app.page == page;
                if nav_item(ui, &theme, Lead::Icon(icon), label, active, collapsed, &NAV).clicked()
                {
                    app.go(page);
                }
            };
            row(ui, Page::Home, icons::HOME, "Home");
            row(ui, Page::New, icons::GRID, "New");
            row(ui, Page::Radio, icons::RADIO, "Radio");
            section(ui, &theme, "Library", None, collapsed);
            row(ui, Page::RecentlyAdded, icons::CLOCK, "Recently Added");
            row(ui, Page::Songs, icons::NOTE, "Songs");
            row(ui, Page::Albums, icons::DISC, "Albums");
            row(ui, Page::Artists, icons::MIC, "Artists");
            if section(
                ui,
                &theme,
                "Playlists",
                Some((icons::PLUS, "New Playlist")),
                collapsed,
            ) {
                new_playlist = true;
            }
            // Apple's own playlist of favourites, once the library says which,
            // drawn like the other playlists with Apple's cover for it.
            let favourites = match app.playlists.iter().find(|p| p.favourites) {
                Some(p) => Page::Playlist(p.id.clone()),
                None => Page::Favourites,
            };
            let active = app.page == favourites;
            if nav_item(
                ui,
                &theme,
                Lead::Favourites,
                "Favourite Songs",
                active,
                collapsed,
                &PLAYLIST,
            )
            .clicked()
            {
                app.go(favourites);
            }
            // The rows on screen only, the rest left as room above and below
            // them: what each row asks of the app — whether its playlist can
            // be deleted, what of it is downloaded, the menu it offers — is a
            // walk of the playlists and of the songs kept, and five hundred
            // playlists did all of that on every frame of every page.
            let shown: Vec<usize> = (0..app.playlists.len())
                .filter(|i| !app.playlists[*i].favourites)
                .collect();
            let rows = visible(viewport, ui.cursor().top() - scrolled, shown.len());
            ui.add_space(rows.start as f32 * PLAYLIST.height);
            for i in shown[rows.clone()].iter().copied() {
                let playlist = &app.playlists[i];
                let art = playlist.artwork.as_ref().map(|a| a.url(120));
                let id = playlist.id.clone();
                let active = app.page == Page::Playlist(id.clone());
                let name = playlist.name.clone();
                let response = nav_item(
                    ui,
                    &theme,
                    Lead::Cover(art),
                    &name,
                    active,
                    collapsed,
                    &PLAYLIST,
                );
                let playlist = &app.playlists[i];
                let facts = app.playlist_facts(playlist);
                if let Some(choice) = super::menu::collection_menu(&response, &theme, facts) {
                    chosen = Some((Collection::Playlist(playlist.clone()), choice));
                }
                if response.clicked() {
                    app.go(Page::Playlist(id));
                }
            }
            ui.add_space((shown.len() - rows.end) as f32 * PLAYLIST.height);
            ui.add_space(12.0);
        });

    if new_playlist {
        app.open_new_playlist(Vec::new());
    }
    if let Some((item, choice)) = chosen {
        app.collection_choice(&item, choice, ui.input(|i| i.time));
    }
    account(app, ui, &theme, full, collapsed);
}

fn search(app: &mut App, ui: &mut Ui, theme: &Theme, full: Rect, collapsed: bool) {
    let rect = Rect::from_min_size(full.min + vec2(12.0, 12.0), vec2(full.width() - 24.0, 30.0));
    let painter = ui.painter();
    painter.rect(
        rect,
        8.0,
        mix(theme.text, theme.crust, 0.05),
        Stroke::new(1.0, theme.line()),
        egui::StrokeKind::Inside,
    );
    if collapsed {
        paint_icon(ui, icons::SEARCH, rect.center(), 14.0, theme.muted());
        // A click opens the sidebar on the field.
        if ui
            .interact(rect, Id::unique("search-collapsed"), Sense::click())
            .on_hover_text("Search")
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            app.sidebar_collapsed = false;
            app.focus_search = true;
        }
        return;
    }
    paint_icon(
        ui,
        icons::SEARCH,
        pos2(rect.right() - 16.0, rect.center().y),
        14.0,
        theme.muted(),
    );
    // The text edit is exactly one line tall, centred in the field: egui's own
    // layouts put its text at the top.
    let body = font(13.0, Weight::Regular);
    let line = text(ui, "Search", body.clone(), theme.text, f32::INFINITY)
        .size()
        .y;
    let field = Rect::from_min_max(
        pos2(rect.left() + 10.0, rect.center().y - line / 2.0),
        pos2(rect.right() - 30.0, rect.center().y + line / 2.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field));
    let response = child.add(
        egui::TextEdit::singleline(&mut app.search)
            .hint_text(egui::RichText::new("Search").color(theme.muted()))
            .font(body)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .desired_width(field.width()),
    );
    if std::mem::take(&mut app.focus_search) {
        response.request_focus();
    }
    if response.changed() {
        app.search_edited = Some(ui.input(|i| i.time));
    }
    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        app.submit_search();
    }
}

/// A section's title, with its button on the right; returns whether the
/// button was clicked.
/// A section's name, and the button beside it, if any: whether it was
/// clicked.
fn section(
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    button: Option<(Icon, &str)>,
    collapsed: bool,
) -> bool {
    ui.add_space(12.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
    let rect = rect.shrink2(vec2(8.0, 0.0));
    if collapsed {
        ui.painter().hline(
            (rect.center().x - 10.0)..=(rect.center().x + 10.0),
            rect.center().y,
            Stroke::new(1.0, theme.line()),
        );
        return false;
    }
    let galley = text(
        ui,
        label,
        font(11.5, Weight::Medium),
        theme.muted(),
        rect.width(),
    );
    paint_text(
        ui,
        galley,
        rect.left() + 9.0,
        rect.center().y,
        egui::Align::Min,
        theme.muted(),
    );
    let Some((icon, tip)) = button else {
        return false;
    };
    let button =
        Rect::from_center_size(pos2(rect.right() - 20.0, rect.center().y), vec2(22.0, 22.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(button));
    icon_button(&mut child, theme, icon, 22.0, 14.0, theme.muted(), true)
        .on_hover_text(tip)
        .clicked()
}

/// Which of `count` playlist rows the viewport shows, the first of them
/// `top` below the top of what is scrolled.
fn visible(viewport: Rect, top: f32, count: usize) -> std::ops::Range<usize> {
    let first = ((viewport.top() - top) / PLAYLIST.height).floor().max(0.0) as usize;
    let last = (((viewport.bottom() - top) / PLAYLIST.height)
        .ceil()
        .max(0.0) as usize)
        .min(count);
    first.min(last)..last
}

/// The sizes of a sidebar row.
struct RowSize {
    height: f32,
    /// The icon or artwork slot.
    lead: f32,
    font: f32,
}

/// Navigation rows, as designed.
const NAV: RowSize = RowSize {
    height: 30.0,
    lead: 20.0,
    font: 13.0,
};

/// Playlist rows: larger than the design's, so their artwork reads (the user's call).
const PLAYLIST: RowSize = RowSize {
    height: 40.0,
    lead: 30.0,
    font: 14.0,
};

/// What comes before a row's label.
enum Lead {
    Icon(Icon),
    /// A playlist's cover, from its URL if it has one.
    Cover(Option<String>),
    /// Favourite Songs' tile, as the phone draws it.
    Favourites,
}

/// A navigation row: an icon or an artwork, then the label.
#[allow(clippy::too_many_arguments)]
fn nav_item(
    ui: &mut Ui,
    theme: &Theme,
    lead_with: Lead,
    label: &str,
    active: bool,
    collapsed: bool,
    size: &RowSize,
) -> egui::Response {
    let (outer, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), size.height), Sense::click());
    let rect = outer.shrink2(vec2(8.0, 0.0));
    let fill = if active {
        Some(alpha(theme.text, 0.11))
    } else if response.is_pointer_button_down_on() {
        Some(theme.press())
    } else if response.hovered() {
        Some(theme.hover())
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, 7.0, fill);
    }
    let lead = if collapsed {
        rect.center()
    } else {
        pos2(rect.left() + 9.0 + size.lead / 2.0, rect.center().y)
    };
    let is_icon = matches!(lead_with, Lead::Icon(_));
    match lead_with {
        Lead::Icon(icon) => {
            let color = if active { theme.accent } else { theme.subtext0 };
            paint_icon(ui, icon, lead, 16.0, color);
        }
        Lead::Favourites => {
            super::favourites_tile(
                ui,
                theme,
                Rect::from_center_size(lead, vec2(size.lead, size.lead)),
            );
        }
        // Scrolled out of sight, its cover isn't loaded, nor kept.
        Lead::Cover(url) if ui.is_rect_visible(rect) => {
            artwork(
                ui,
                url,
                Rect::from_center_size(lead, vec2(size.lead, size.lead)),
                size.lead / 5.0,
                theme,
            );
        }
        Lead::Cover(_) => {}
    }
    if !collapsed {
        let left = rect.left() + 9.0 + size.lead + 10.0 + if is_icon { 0.0 } else { 4.0 };
        let galley = text(
            ui,
            label,
            font(size.font, Weight::Regular),
            theme.text,
            rect.right() - left - 9.0,
        );
        paint_text(
            ui,
            galley,
            left,
            rect.center().y,
            egui::Align::Min,
            theme.text,
        );
    }
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if collapsed {
        response.on_hover_text(label)
    } else {
        response
    }
}

fn account(app: &mut App, ui: &mut Ui, theme: &Theme, full: Rect, collapsed: bool) {
    let rect = Rect::from_min_max(
        pos2(full.left() + 8.0, full.bottom() - 8.0 - 52.0),
        pos2(full.right() - 8.0, full.bottom() - 8.0),
    );
    let response = ui.interact(rect, egui::Id::unique("account"), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 10.0, theme.hover());
    }
    if response.clicked() {
        app.go(Page::Settings);
    }
    let name = std::env::var("USER").unwrap_or_default();
    let name = capitalise(&name);
    let avatar = if collapsed {
        rect.center()
    } else {
        pos2(rect.left() + 8.0 + 15.0, rect.center().y)
    };
    // The account's picture on Apple Music, else its initial.
    match &app.profile_picture {
        Some(picture) => super::artwork(
            ui,
            Some(picture.url(60)),
            Rect::from_center_size(avatar, vec2(30.0, 30.0)),
            15.0,
            theme,
        ),
        None => avatar_disc(ui, theme, avatar, &name),
    }
    if collapsed {
        return;
    }
    let left = rect.left() + 8.0 + 30.0 + 10.0;
    let width = rect.right() - left - 8.0;
    let galley = text(ui, &name, font(13.0, Weight::Medium), theme.text, width);
    paint_text(
        ui,
        galley,
        left,
        rect.center().y - 8.0,
        egui::Align::Min,
        theme.text,
    );
    let galley = text(
        ui,
        "Settings",
        font(11.5, Weight::Regular),
        theme.muted(),
        width,
    );
    paint_text(
        ui,
        galley,
        left,
        rect.center().y + 9.0,
        egui::Align::Min,
        theme.muted(),
    );
}

/// A 30 px disc with an accent-to-mauve gradient and the initial.
pub(super) fn avatar_disc(ui: &Ui, theme: &Theme, center: egui::Pos2, name: &str) {
    let mut mesh = egui::Mesh::default();
    let steps = 32;
    mesh.colored_vertex(center, mix(theme.accent, theme.mauve, 0.5));
    for i in 0..=steps {
        let angle = i as f32 / steps as f32 * std::f32::consts::TAU;
        let dir = vec2(angle.cos(), angle.sin());
        // 135°: accent at the top left, mauve at the bottom right.
        let t = ((dir.x + dir.y) / std::f32::consts::SQRT_2 + 1.0) / 2.0;
        mesh.colored_vertex(center + dir * 15.0, mix(theme.mauve, theme.accent, t));
        if i > 0 {
            mesh.add_triangle(0, i, i + 1);
        }
    }
    ui.painter().add(mesh);
    let initial: String = name
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default();
    let galley = text(
        ui,
        &initial,
        font(13.0, Weight::SemiBold),
        theme.on_accent(),
        30.0,
    );
    paint_text(
        ui,
        galley,
        center.x,
        center.y,
        egui::Align::Center,
        theme.on_accent(),
    );
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
