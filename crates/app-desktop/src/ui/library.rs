//! Your Library, as Spotify's sidebar: the playlists, albums or artists of
//! the library with their covers, chosen by chips. Favourite Songs and Songs
//! are pinned over the playlists as Spotify pins Liked Songs, Recently Added
//! over the albums. Collapsed, only the covers.

use egui::{Color32, Id, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{Collection, ProviderKind};

use crate::app::{App, CollectionChoice, Page};
use crate::color::alpha;
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::{artwork, icon_button, paint_icon, paint_text, text};

pub const WIDTH: f32 = 300.0;
pub const COLLAPSED: f32 = 72.0;

/// What the chips narrow the list to.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Filter {
    #[default]
    Playlists,
    Albums,
    Artists,
}

/// A row's picture.
pub(super) enum Art {
    Cover(Option<String>),
    /// A pinned page's: its icon on a gradient.
    Tile(Icon, [Color32; 2]),
}

pub(super) struct Row {
    pub title: String,
    pub subtitle: String,
    pub art: Art,
    /// Artists are round.
    pub round: bool,
    pub page: Page,
    /// A library playlist or album, with its menu.
    pub menu: Option<Collection>,
}

pub fn show(app: &mut App, ui: &mut Ui, collapsed: bool) {
    let theme = app.theme.clone();
    let full = ui.max_rect();
    let filter_id = Id::unique("library-filter");
    let mut filter: Filter = ui.data(|d| d.get_temp(filter_id)).unwrap_or_default();

    // The heading. Collapsed, the library's icon opens it again; open, a
    // button to fold it shows left of the title while the pointer is over
    // the library, as Spotify's.
    let head = Rect::from_min_size(full.min, vec2(full.width(), 60.0));
    let fold_rect = if collapsed {
        Rect::from_center_size(pos2(full.center().x, head.center().y), vec2(40.0, 40.0))
    } else {
        Rect::from_center_size(
            pos2(head.left() + 12.0 + 16.0, head.center().y),
            vec2(32.0, 32.0),
        )
    };
    let hovering = ui.rect_contains_pointer(full);
    let mut title_left = head.left() + 16.0;
    let fold = if collapsed || hovering {
        let response = ui
            .interact(fold_rect, Id::unique("library-fold"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(if collapsed {
                "Expand Your Library"
            } else {
                "Collapse Your Library"
            });
        let color = if response.hovered() {
            theme.text
        } else {
            theme.subtext0
        };
        let icon = if collapsed {
            icons::LIBRARY
        } else {
            icons::PANEL
        };
        paint_icon(
            ui,
            icon,
            fold_rect.center(),
            if collapsed { 22.0 } else { 18.0 },
            color,
        );
        if !collapsed {
            title_left = fold_rect.right() + 6.0;
        }
        Some(response)
    } else {
        None
    };
    if !collapsed {
        let color = theme.text_a(0.92);
        let g = text(ui, "Your Library", font(16.0, Weight::Bold), color, 200.0);
        paint_text(ui, g, title_left, head.center().y, egui::Align::Min, color);
    }
    if fold.is_some_and(|f| f.clicked()) {
        app.sidebar_collapsed = !collapsed;
    }
    // New playlist, and the grid of what the chip shows.
    let mut new_playlist = false;
    let plus = if collapsed {
        Rect::from_center_size(
            pos2(full.center().x, head.bottom() + 16.0),
            vec2(32.0, 32.0),
        )
    } else {
        Rect::from_center_size(
            pos2(
                head.right()
                    - 16.0
                    - 16.0
                    - if filter == Filter::Playlists {
                        0.0
                    } else {
                        40.0
                    },
                head.center().y,
            ),
            vec2(32.0, 32.0),
        )
    };
    let hovered = ui.rect_contains_pointer(plus);
    ui.painter().circle_filled(
        plus.center(),
        16.0,
        if hovered {
            theme.surface2
        } else {
            theme.surface1
        },
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(plus));
    if icon_button(
        &mut child,
        &theme,
        icons::PLUS,
        32.0,
        16.0,
        theme.text,
        true,
    )
    .on_hover_text("Create playlist")
    .clicked()
    {
        new_playlist = true;
    }
    if !collapsed && filter != Filter::Playlists {
        let grid = Rect::from_center_size(
            pos2(head.right() - 16.0 - 16.0, head.center().y),
            vec2(32.0, 32.0),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(grid));
        if icon_button(
            &mut child,
            &theme,
            icons::GRID,
            32.0,
            16.0,
            theme.subtext0,
            true,
        )
        .on_hover_text("Show as covers")
        .clicked()
        {
            app.go(match filter {
                Filter::Albums => Page::Albums,
                _ => Page::Artists,
            });
        }
    }

    let mut top = head.bottom();
    if collapsed {
        top = plus.bottom() + 12.0;
    } else {
        let chips = [
            (Filter::Playlists, "Playlists"),
            (Filter::Albums, "Albums"),
            (Filter::Artists, "Artists"),
        ];
        let mut x = full.left() + 16.0;
        let f = font(14.0, Weight::Regular);
        for (kind, label) in chips {
            let w = text(ui, label, f.clone(), theme.text, 200.0).size().x;
            let rect = Rect::from_min_size(pos2(x, top), vec2(w + 24.0, 32.0));
            x = rect.right() + 8.0;
            let response = ui
                .interact(rect, Id::unique(("library-chip", label)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let on = filter == kind;
            let (fill, ink) = if on {
                (theme.text, theme.base)
            } else if response.hovered() {
                (theme.surface2, theme.text)
            } else {
                (theme.surface1, theme.text)
            };
            ui.painter().rect_filled(rect, 16.0, fill);
            let g = text(ui, label, f.clone(), ink, 200.0);
            paint_text(
                ui,
                g,
                rect.center().x,
                rect.center().y,
                egui::Align::Center,
                ink,
            );
            if response.clicked() {
                filter = kind;
            }
        }
        ui.data_mut(|d| d.insert_temp(filter_id, filter));
        top += 32.0 + 12.0;
    }

    // The rows, only those in sight laid out.
    let pinned = pinned(app, filter);
    let listed: Vec<usize> = match filter {
        Filter::Playlists => (0..app.playlists.len()).collect(),
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
    let row_height = if collapsed { 56.0 } else { 64.0 };
    let list = Rect::from_min_max(pos2(full.left(), top), full.max);
    let mut list_ui = ui.new_child(egui::UiBuilder::new().max_rect(list));
    let mut chosen: Option<(Collection, CollectionChoice)> = None;
    let mut go = None;
    egui::ScrollArea::vertical()
        .id_salt(("library-scroll", filter as u8))
        .auto_shrink(false)
        .show_rows(
            &mut list_ui,
            row_height,
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
                    let response = row_ui(app, ui, &theme, row, row_height, collapsed);
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
    // Their albums or artists on their way: rows in grey, or the spinner.
    let coming = match filter {
        Filter::Playlists => false,
        Filter::Albums => !app.library_albums.done,
        Filter::Artists => !app.library_artists.done,
    };
    if listed.is_empty() && coming && !collapsed {
        let first = list.top() + pinned.len() as f32 * row_height;
        for k in 0..6 {
            let y = first + k as f32 * row_height + (row_height - 48.0) / 2.0;
            let x = list.left() + 16.0;
            let round = if filter == Filter::Artists { 24.0 } else { 4.0 };
            super::loading::shape(
                ui,
                &theme,
                Rect::from_min_size(pos2(x, y), vec2(48.0, 48.0)),
                round,
            );
            super::loading::shape(
                ui,
                &theme,
                Rect::from_min_size(pos2(x + 60.0, y + 8.0), vec2(140.0, 12.0)),
                4.0,
            );
            super::loading::shape(
                ui,
                &theme,
                Rect::from_min_size(pos2(x + 60.0, y + 28.0), vec2(90.0, 10.0)),
                4.0,
            );
        }
        super::loading::spinner(ui, &theme, pos2(list.center().x, first + 40.0));
    }
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

/// The pages pinned over the chip's list.
fn pinned(app: &App, filter: Filter) -> Vec<Row> {
    let tile = |title: &str, subtitle: String, icon, colors, page| Row {
        title: title.into(),
        subtitle,
        art: Art::Tile(icon, colors),
        round: false,
        page,
        menu: None,
    };
    const PLAIN: [Color32; 2] = [
        Color32::from_rgb(0x2a, 0x2a, 0x2a),
        Color32::from_rgb(0x50, 0x50, 0x50),
    ];
    match filter {
        Filter::Playlists => {
            // The library's songs are Spotify's Liked Songs; Apple's own
            // playlist of favourites is listed with the others.
            vec![tile(
                "Songs",
                if app.songs.is_empty() && !app.songs_done {
                    "Playlist".to_owned()
                } else {
                    format!("Playlist · {} songs", app.songs.len())
                },
                icons::HEART,
                super::LOVED,
                Page::Songs,
            )]
        }
        Filter::Albums => vec![tile(
            "Recently Added",
            "Library".into(),
            icons::CLOCK,
            PLAIN,
            Page::RecentlyAdded,
        )],
        Filter::Artists => Vec::new(),
    }
}

/// The `i`th item of the library's list the chip shows.
pub(super) fn listed_row(app: &App, filter: Filter, i: usize) -> Row {
    match filter {
        Filter::Playlists => {
            let p = &app.playlists[i];
            let by = if p.editable {
                capitalised_user()
            } else {
                p.curator
                    .clone()
                    .unwrap_or_else(|| service(p.id.provider).into())
            };
            Row {
                title: p.name.clone(),
                subtitle: format!("Playlist · {by}"),
                art: Art::Cover(p.artwork.as_ref().map(|a| a.url(120))),
                round: false,
                page: Page::Playlist(p.id.clone()),
                menu: Some(Collection::Playlist(p.clone())),
            }
        }
        Filter::Albums => {
            let a = &app.library_albums.items[i];
            Row {
                title: a.title.clone(),
                subtitle: format!("Album · {}", a.artists.join(", ")),
                art: Art::Cover(a.artwork.as_ref().map(|art| art.url(120))),
                round: false,
                page: Page::Album(a.id.clone()),
                menu: Some(Collection::Album(a.clone())),
            }
        }
        Filter::Artists => {
            let a = &app.library_artists.items[i];
            Row {
                title: a.name.clone(),
                subtitle: "Artist".into(),
                art: Art::Cover(a.artwork.as_ref().map(|art| art.url(120))),
                round: true,
                // The library's own id, which its page asks by.
                page: Page::LibraryArtist(a.library_id.clone().unwrap_or_else(|| a.id.clone())),
                menu: None,
            }
        }
    }
}

fn row_ui(
    app: &App,
    ui: &mut Ui,
    theme: &Theme,
    row: &Row,
    height: f32,
    collapsed: bool,
) -> egui::Response {
    let (outer, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    let rect = outer.shrink2(vec2(8.0, 0.0));
    let active = app.page == row.page && !app.lyrics_page;
    let fill = if response.is_pointer_button_down_on() {
        Some(theme.press())
    } else if active {
        Some(alpha(theme.text, 0.1))
    } else if response.hovered() {
        Some(theme.hover())
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, 6.0, fill);
    }
    let size = 48.0;
    let lead = if collapsed {
        rect.center()
    } else {
        pos2(rect.left() + 8.0 + size / 2.0, rect.center().y)
    };
    let art = Rect::from_center_size(lead, vec2(size, size));
    let radius = if row.round { size / 2.0 } else { 4.0 };
    match &row.art {
        Art::Cover(url) => {
            // Scrolled out of sight, its cover isn't loaded, nor kept.
            if ui.is_rect_visible(rect) {
                artwork(ui, url.clone(), art, radius, theme);
            }
        }
        Art::Tile(icon, [from, to]) => {
            super::gradient(ui, art, radius, *from, *to);
            paint_icon(ui, *icon, art.center(), 20.0, Color32::WHITE);
        }
    }
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if collapsed {
        return response.on_hover_text(&row.title);
    }
    let left = art.right() + 12.0;
    let width = rect.right() - left - 8.0;
    // The page playing from shows in the accent, as Spotify's.
    let playing = app.is_playing() && app.playing_from() == Some(&row.page);
    let title_color = if playing { theme.accent } else { theme.text };
    let g = text(
        ui,
        &row.title,
        font(15.0, Weight::Regular),
        title_color,
        width,
    );
    paint_text(
        ui,
        g,
        left,
        rect.center().y - 10.0,
        egui::Align::Min,
        title_color,
    );
    let g = text(
        ui,
        &row.subtitle,
        font(13.0, Weight::Regular),
        theme.subtext0,
        width,
    );
    paint_text(
        ui,
        g,
        left,
        rect.center().y + 11.0,
        egui::Align::Min,
        theme.subtext0,
    );
    response
}

pub(super) fn service(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::AppleMusic => "Apple Music",
        ProviderKind::SoundCloud => "SoundCloud",
        ProviderKind::Navidrome => "Navidrome",
        ProviderKind::Local => "Files",
        ProviderKind::Music => "Music",
    }
}

fn capitalised_user() -> String {
    let name = std::env::var("USER").unwrap_or_default();
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
