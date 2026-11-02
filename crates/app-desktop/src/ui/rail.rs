//! M3E's navigation rail: Home, New and Radio as pills, then Your Library
//! (chips, pinned pages, the list) and Settings at the bottom. Folded, the
//! pages are icons with their labels under them, the library covers only.
//! It sits on the window's `crust`, not in a pane.

use egui::{Color32, Id, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{Collection, ProviderKind};

use crate::app::{App, CollectionChoice, Page};
use crate::color::alpha;
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::library::{Art, Filter, Row, listed_row};
use super::{artwork, paint_icon, paint_text, text};

pub const WIDTH: f32 = 272.0;
pub const COLLAPSED: f32 = 96.0;

pub fn show(app: &mut App, ui: &mut Ui, collapsed: bool) {
    let theme = app.theme.clone();
    let full = ui.max_rect();
    let mut go = None;

    // The pages, then Settings at the bottom.
    let mut y = full.top() + 4.0;
    for (page, icon, label) in [
        (Page::Home, icons::HOME, "Home"),
        (Page::New, icons::GRID, "New"),
        (Page::Radio, icons::RADIO, "Radio"),
    ] {
        if destination(app, ui, &theme, full, y, collapsed, &page, icon, label) {
            go = Some(page);
        }
        y += if collapsed { 64.0 } else { 60.0 };
    }
    let settings_y = full.bottom() - if collapsed { 68.0 } else { 68.0 };
    if destination(
        app,
        ui,
        &theme,
        full,
        settings_y,
        collapsed,
        &Page::Settings,
        icons::SLIDERS,
        "Settings",
    ) {
        go = Some(Page::Settings);
    }

    // Your Library.
    let filter_id = Id::unique("rail-filter");
    let mut filter: Filter = ui.data(|d| d.get_temp(filter_id)).unwrap_or_default();
    let mut new_playlist = false;
    y += 16.0;
    if collapsed {
        let plus = Rect::from_center_size(pos2(full.center().x, y + 20.0), vec2(56.0, 40.0));
        if tonal_button(ui, &theme, plus, icons::PLUS, "rail-new")
            .on_hover_text("New playlist")
            .clicked()
        {
            new_playlist = true;
        }
        y = plus.bottom() + 8.0;
    } else {
        let g = text(
            ui,
            "Your Library",
            font(15.0, Weight::SemiBold),
            theme.text,
            200.0,
        );
        paint_text(
            ui,
            g,
            full.left() + 30.0,
            y + 20.0,
            egui::Align::Min,
            theme.text,
        );
        let plus = Rect::from_center_size(pos2(full.right() - 36.0, y + 20.0), vec2(40.0, 40.0));
        if round_icon(ui, &theme, plus, icons::PLUS, "rail-new")
            .on_hover_text("New playlist")
            .clicked()
        {
            new_playlist = true;
        }
        y += 50.0;
        // Material's filter chips: the one on in the secondary container,
        // with a check.
        let mut x = full.left() + 18.0;
        let f = font(13.0, Weight::Medium);
        for (kind, label) in [
            (Filter::Playlists, "Playlists"),
            (Filter::Albums, "Albums"),
            (Filter::Artists, "Artists"),
        ] {
            let on = filter == kind;
            let w = text(ui, label, f.clone(), theme.text, 200.0).size().x;
            let width = w + if on { 16.0 + 24.0 } else { 24.0 };
            let rect = Rect::from_min_size(pos2(x, y), vec2(width, 32.0));
            x = rect.right() + 6.0;
            let response = ui
                .interact(rect, Id::unique(("rail-chip", label)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let ink = if on {
                theme.tonal.on_secondary_container
            } else {
                theme.subtext0
            };
            if on {
                ui.painter()
                    .rect_filled(rect, 8.0, theme.tonal.secondary_container);
                paint_icon(
                    ui,
                    icons::CHECK,
                    pos2(rect.left() + 16.0, rect.center().y),
                    16.0,
                    ink,
                );
            } else {
                ui.painter().rect_stroke(
                    rect,
                    8.0,
                    egui::Stroke::new(1.0, theme.surface2),
                    egui::StrokeKind::Inside,
                );
            }
            if response.hovered() {
                ui.painter().rect_filled(rect, 8.0, alpha(theme.text, 0.08));
            }
            let g = text(ui, label, f.clone(), ink, 200.0);
            let tx = if on {
                rect.left() + 32.0
            } else {
                rect.left() + 12.0
            };
            paint_text(ui, g, tx, rect.center().y, egui::Align::Min, ink);
            if response.clicked() {
                filter = kind;
            }
        }
        ui.data_mut(|d| d.insert_temp(filter_id, filter));
        y += 32.0 + 10.0;
    }

    // The list, pinned pages first, only the rows in sight laid out.
    let pinned = if filter == Filter::Playlists {
        pinned(app)
    } else {
        Vec::new()
    };
    let listed: Vec<usize> = match filter {
        Filter::Playlists => (0..app.playlists.len())
            .filter(|&i| !app.playlists[i].favourites)
            .collect(),
        Filter::Albums => {
            app.want_library_albums();
            let copies = app.copies(0);
            app.albums_in_order()
                .iter()
                .copied()
                .filter(|i| !copies.contains(i))
                .collect()
        }
        Filter::Artists => {
            app.want_library_artists();
            let copies = app.copies(1);
            (0..app.library_artists.items.len())
                .filter(|i| !copies.contains(i))
                .collect()
        }
    };
    let list = Rect::from_min_max(pos2(full.left(), y), pos2(full.right(), settings_y - 8.0));
    let mut list_ui = ui.new_child(egui::UiBuilder::new().max_rect(list));
    list_ui.set_clip_rect(list.intersect(ui.clip_rect()));
    let mut chosen: Option<(Collection, CollectionChoice)> = None;
    egui::ScrollArea::vertical()
        .id_salt(("rail-scroll", filter as u8))
        .auto_shrink(false)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
        .show_rows(
            &mut list_ui,
            60.0,
            pinned.len() + listed.len(),
            |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for i in range {
                    let owned;
                    let row = match i.checked_sub(pinned.len()) {
                        None => &pinned[i],
                        Some(k) => {
                            owned = listed_row(app, filter, listed[k]);
                            &owned
                        }
                    };
                    let response = row_ui(app, ui, &theme, row, collapsed);
                    if let Some(item) = &row.menu
                        && let Some(choice) = super::menu::collection_menu(
                            &response,
                            &theme,
                            app.collection_facts(item),
                        )
                    {
                        chosen = Some((item.clone(), choice));
                    }
                    if response.clicked() {
                        go = Some(row.page.clone());
                    }
                }
            },
        );

    if let Some(page) = go {
        app.go(page);
    }
    if new_playlist {
        app.open_new_playlist(Vec::new());
    }
    if let Some((item, choice)) = chosen {
        app.collection_choice(&item, choice, ui.input(|i| i.time));
    }
}

/// A page of the rail: a 56px pill, or folded an icon in a 56 × 32
/// indicator with its label under it. The page shown in the secondary
/// container. Whether it was clicked.
#[allow(clippy::too_many_arguments)]
fn destination(
    app: &App,
    ui: &Ui,
    theme: &Theme,
    full: Rect,
    y: f32,
    collapsed: bool,
    page: &Page,
    icon: Icon,
    label: &str,
) -> bool {
    let active = app.page == *page && !app.lyrics_page;
    let (hit, indicator) = if collapsed {
        let hit = Rect::from_min_size(pos2(full.left(), y), vec2(full.width(), 56.0));
        let indicator = Rect::from_center_size(pos2(full.center().x, y + 16.0), vec2(56.0, 32.0));
        (hit, indicator)
    } else {
        let pill =
            Rect::from_min_size(pos2(full.left() + 12.0, y), vec2(full.width() - 24.0, 56.0));
        (pill, pill)
    };
    let response = ui
        .interact(hit, Id::unique(("rail-page", label)), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let radius = indicator.height() / 2.0;
    if active {
        ui.painter()
            .rect_filled(indicator, radius, theme.tonal.secondary_container);
    }
    if response.hovered() {
        let p = if response.is_pointer_button_down_on() {
            0.12
        } else {
            0.08
        };
        ui.painter()
            .rect_filled(indicator, radius, alpha(theme.text, p));
    }
    let ink = if active {
        theme.tonal.on_secondary_container
    } else {
        theme.subtext0
    };
    let f = font(
        if collapsed { 12.0 } else { 15.0 },
        if active {
            Weight::SemiBold
        } else {
            Weight::Medium
        },
    );
    if collapsed {
        paint_icon(ui, icon, indicator.center(), 22.0, ink);
        let ink = if active { theme.text } else { theme.subtext0 };
        let g = text(ui, label, f, ink, full.width());
        paint_text(
            ui,
            g,
            full.center().x,
            indicator.bottom() + 12.0,
            egui::Align::Center,
            ink,
        );
    } else {
        paint_icon(
            ui,
            icon,
            pos2(indicator.left() + 30.0, indicator.center().y),
            22.0,
            ink,
        );
        let g = text(ui, label, f, ink, indicator.width() - 70.0);
        paint_text(
            ui,
            g,
            indicator.left() + 56.0,
            indicator.center().y,
            egui::Align::Min,
            ink,
        );
    }
    response.clicked()
}

/// A round icon button, hover in a tint of the text.
fn round_icon(ui: &Ui, theme: &Theme, rect: Rect, icon: Icon, salt: &str) -> egui::Response {
    let response = ui
        .interact(rect, Id::unique(salt), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.hovered() {
        ui.painter()
            .circle_filled(rect.center(), rect.width() / 2.0, alpha(theme.text, 0.08));
    }
    paint_icon(ui, icon, rect.center(), 20.0, theme.subtext0);
    response
}

/// A tonal button: the secondary container, its icon on it.
fn tonal_button(ui: &Ui, theme: &Theme, rect: Rect, icon: Icon, salt: &str) -> egui::Response {
    let response = ui
        .interact(rect, Id::unique(salt), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let radius = if response.is_pointer_button_down_on() {
        12.0
    } else {
        rect.height() / 2.0
    };
    ui.painter()
        .rect_filled(rect, radius, theme.tonal.secondary_container);
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, radius, alpha(theme.text, 0.08));
    }
    paint_icon(
        ui,
        icon,
        rect.center(),
        20.0,
        theme.tonal.on_secondary_container,
    );
    response
}

/// Songs, Recently Added and Favourite Songs, over the playlists.
fn pinned(app: &App) -> Vec<Row> {
    let tile = |title: &str, subtitle: String, icon, page| Row {
        title: title.into(),
        subtitle,
        art: Art::Tile(icon, [Color32::TRANSPARENT; 2]),
        round: false,
        page,
        menu: None,
    };
    let songs = if app.songs.is_empty() && !app.songs_done {
        "Every service".to_owned()
    } else {
        format!("{} songs · every service", app.songs.len())
    };
    let mut rows = vec![
        tile("Songs", songs, icons::NOTE, Page::Songs),
        tile(
            "Recently Added",
            "Albums and playlists".into(),
            icons::CLOCK,
            Page::RecentlyAdded,
        ),
    ];
    if let Some(p) = app.playlists.iter().find(|p| p.favourites) {
        let count = p.track_count.map_or_else(
            || "Playlist".to_owned(),
            |n| format!("Playlist · {n} songs"),
        );
        rows.push(tile(
            "Favourite Songs",
            count,
            icons::HEART_FILL,
            Page::Playlist(p.id.clone()),
        ));
    }
    rows
}

fn row_ui(app: &App, ui: &mut Ui, theme: &Theme, row: &Row, collapsed: bool) -> egui::Response {
    let (outer, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 60.0), Sense::click());
    let rect = outer.shrink2(vec2(if collapsed { 12.0 } else { 12.0 }, 2.0));
    let active = app.page == row.page && !app.lyrics_page;
    if active {
        ui.painter()
            .rect_filled(rect, 16.0, theme.tonal.secondary_container);
    }
    if response.hovered() {
        let p = if response.is_pointer_button_down_on() {
            0.12
        } else {
            0.08
        };
        ui.painter().rect_filled(rect, 16.0, alpha(theme.text, p));
    }
    let size = 48.0;
    let lead = if collapsed {
        rect.center()
    } else {
        pos2(rect.left() + 4.0 + size / 2.0, rect.center().y)
    };
    let art = Rect::from_center_size(lead, vec2(size, size));
    let radius = if row.round { size / 2.0 } else { 12.0 };
    match &row.art {
        Art::Cover(url) => {
            if ui.is_rect_visible(rect) {
                artwork(ui, url.clone(), art, radius, theme);
            }
        }
        // The pinned pages, each in a tonal colour of its own.
        Art::Tile(icon, _) => {
            let (fill, ink) = match row.page {
                Page::Songs => (
                    theme.tonal.tertiary_container,
                    theme.tonal.on_tertiary_container,
                ),
                Page::RecentlyAdded => (
                    theme.tonal.secondary_container,
                    theme.tonal.on_secondary_container,
                ),
                _ => (theme.accent, theme.on_accent()),
            };
            ui.painter().rect_filled(art, radius, fill);
            paint_icon(ui, *icon, art.center(), 22.0, ink);
        }
    }
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if collapsed {
        return response.on_hover_text(&row.title);
    }
    let left = art.right() + 14.0;
    let width = rect.right() - left - 6.0;
    // The list playing, its title in the primary.
    let playing = app.is_playing() && app.playing_from() == Some(&row.page);
    let ink = if active {
        theme.tonal.on_secondary_container
    } else if playing {
        theme.tonal.primary
    } else {
        theme.text
    };
    let g = text(ui, &row.title, font(14.0, Weight::Medium), ink, width);
    paint_text(ui, g, left, rect.center().y - 9.0, egui::Align::Min, ink);
    // The service's logo, then what it is.
    let mut x = left;
    let subtitle = match &row.menu {
        Some(Collection::Playlist(p)) => {
            if let Some(logo) = logo(p.id.provider) {
                paint_icon(
                    ui,
                    logo,
                    pos2(x + 6.0, rect.center().y + 11.0),
                    12.0,
                    Color32::WHITE,
                );
                x += 16.0;
            }
            p.track_count.map_or_else(
                || "Playlist".to_owned(),
                |n| format!("Playlist · {n} songs"),
            )
        }
        Some(Collection::Album(a)) => {
            if let Some(logo) = logo(a.id.provider) {
                paint_icon(
                    ui,
                    logo,
                    pos2(x + 6.0, rect.center().y + 11.0),
                    12.0,
                    Color32::WHITE,
                );
                x += 16.0;
            }
            row.subtitle.clone()
        }
        _ => row.subtitle.clone(),
    };
    let sub = if active {
        alpha(theme.tonal.on_secondary_container, 0.8)
    } else {
        theme.subtext0
    };
    let g = text(
        ui,
        &subtitle,
        font(12.5, Weight::Regular),
        sub,
        width - (x - left),
    );
    paint_text(ui, g, x, rect.center().y + 11.0, egui::Align::Min, sub);
    response
}

fn logo(provider: ProviderKind) -> Option<Icon> {
    match provider {
        ProviderKind::AppleMusic => Some(icons::APPLE_MUSIC),
        ProviderKind::SoundCloud => Some(icons::SOUNDCLOUD),
        ProviderKind::Navidrome => Some(icons::NAVIDROME),
        ProviderKind::Music => None,
    }
}
