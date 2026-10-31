//! An artist, laid out as Cider does: their video, or a wide picture of
//! them, across the top and fading into the page, their name and buttons
//! over its foot; the latest release beside the top songs in sideways
//! columns; then shelves of their albums, videos, playlists and the like.

use egui::{Color32, Id, Mesh, Rect, Sense, Ui, pos2, vec2};
use music_core::model::{Album, ArtistDetail, Collection, FavouriteKind, ItemId, Playlist};

use crate::app::{App, Loaded, Page};
use crate::fonts::{Weight, font};
use crate::icons;
use crate::theme::Theme;

use super::grid::{
    CARD_TEXT, CardClick, SHELF_HEAD, SONG_COLUMNS_HEIGHT, ShelfCard, cover_shelf, round_shelf,
    song_columns, video_shelf,
};
use super::search::{message, section_head};
use super::{artwork, format_date, paint_icon, paint_text, text};

const SECTION_GAP: f32 = 36.0;
/// The latest release beside them: its cover and the words next to it.
const LATEST_ART: f32 = 150.0;
const LATEST_WIDTH: f32 = LATEST_ART + 16.0 + 140.0;
const SHELF_CARD: f32 = 200.0;
/// The music videos' stills, wide.
const VIDEO_CARD: f32 = 240.0;

/// How tall the top of the page is, for a page `width` wide.
fn hero_height(width: f32) -> f32 {
    (width * 0.42).clamp(360.0, 760.0)
}

/// A shelf of the page, in the order Cider shows them.
enum Section<'a> {
    Albums(&'static str, &'a [Album]),
    Videos,
    Playlists(&'a [Playlist]),
    Similar,
}

pub fn show(app: &mut App, ui: &mut Ui, pad: f32, id: &ItemId) -> f32 {
    let theme = app.theme.clone();
    if theme.layout.header == crate::theme::Header::Banner {
        return show_banner(app, ui, pad, id);
    }
    let (artist, error) = match app.artists.get(id) {
        Some(Loaded::Ready(a)) => (Some(a.clone()), None),
        Some(Loaded::Failed(e)) => (None, Some(e.clone())),
        _ => (None, None),
    };
    let width = ui.available_width() - 2.0 * pad;
    let hero = hero_height(ui.available_width());
    let top_songs = artist
        .as_ref()
        .is_some_and(|a| !a.top_songs.is_empty() || a.latest.is_some());
    let sections: Vec<Section> = artist
        .as_ref()
        .map(|a| {
            let mut s = vec![
                Section::Albums("Essential Albums", a.essentials.as_slice()),
                Section::Albums("Albums", a.albums.as_slice()),
                Section::Albums("Singles & EPs", a.singles.as_slice()),
            ];
            if !a.videos.is_empty() {
                s.push(Section::Videos);
            }
            if !a.playlists.is_empty() {
                s.push(Section::Playlists(a.playlists.as_slice()));
            }
            s.push(Section::Albums("Compilations", a.compilations.as_slice()));
            s.push(Section::Albums("Appears On", a.appears_on.as_slice()));
            if !a.similar.is_empty() {
                s.push(Section::Similar);
            }
            s.retain(|s| !matches!(s, Section::Albums(_, a) if a.is_empty()));
            s
        })
        .unwrap_or_default();
    let shelf_height = SHELF_HEAD + SHELF_CARD + CARD_TEXT + SECTION_GAP;
    let video_height = SHELF_HEAD + VIDEO_CARD * 9.0 / 16.0 + CARD_TEXT + SECTION_GAP;
    let mut total = hero + 16.0;
    if top_songs {
        total += SHELF_HEAD + SONG_COLUMNS_HEIGHT + SECTION_GAP;
    }
    for section in &sections {
        total += match section {
            Section::Videos => video_height,
            _ => shelf_height,
        };
    }
    total += 104.0;

    let output = egui::ScrollArea::vertical()
        .id_salt(("artist", &id.id))
        .auto_shrink(false)
        .show(ui, |ui| {
            let (content, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
            let left = content.left() + pad;
            let top = Rect::from_min_size(content.min, vec2(content.width(), hero));
            if let Some(artist) = &artist {
                banner(app, ui, &theme, artist, top);
            }

            let mut y = content.top() + hero + 16.0;
            if let Some(e) = &error {
                message(ui, &theme, left, y, width, "Couldn't load this artist", e);
                return;
            }
            let Some(artist) = &artist else {
                return;
            };
            // The name and the buttons over the foot of the picture, white
            // whatever the theme, as the picture is under them.
            let name = text(
                ui,
                &artist.artist.name,
                font(44.0, Weight::Bold),
                Color32::WHITE,
                width,
            );
            let name_y = top.bottom() - 132.0;
            ui.painter().galley_with_override_text_color(
                pos2(left, name_y - name.size().y / 2.0 + 2.0),
                name.clone(),
                Color32::from_black_alpha(90),
            );
            paint_text(ui, name, left, name_y, egui::Align::Min, Color32::WHITE);
            let favourite = app.is_favourite(FavouriteKind::Artist, &artist.artist.id);
            let now = ui.input(|i| i.time);
            match hero_buttons(ui, &theme, pos2(left, top.bottom() - 70.0), favourite) {
                Some(HeroButton::Play) => {
                    app.play_shuffled_or_not(artist.top_songs.clone(), false, now);
                }
                Some(HeroButton::Shuffle) => {
                    app.play_shuffled_or_not(artist.top_songs.clone(), true, now);
                }
                Some(HeroButton::Favourite) => {
                    app.toggle_favourite(FavouriteKind::Artist, artist.artist.id.clone());
                }
                None => {}
            }

            if top_songs {
                songs_section(app, ui, &theme, artist, pos2(left, y), width);
                y += SHELF_HEAD + SONG_COLUMNS_HEIGHT + SECTION_GAP;
            }
            for section in &sections {
                match section {
                    Section::Albums(title, albums) => {
                        albums_shelf(
                            app,
                            ui,
                            &theme,
                            artist,
                            title,
                            albums,
                            pos2(left, y),
                            width,
                            false,
                        );
                        y += shelf_height;
                    }
                    Section::Videos => {
                        let cards: Vec<ShelfCard> = artist
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
                        let shelf_id = Id::unique(("artist-videos", &artist.artist.id.id));
                        // There is no video player here: they play on Apple
                        // Music's website.
                        if let Some(i) = video_shelf(
                            ui,
                            &theme,
                            pos2(left, y),
                            width,
                            "Music Videos",
                            &cards,
                            VIDEO_CARD,
                            shelf_id,
                        ) && let Some(url) = &artist.videos[i].url
                        {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                        }
                        y += video_height;
                    }
                    Section::Playlists(playlists) => {
                        let cards: Vec<ShelfCard> = playlists
                            .iter()
                            .map(|p| ShelfCard {
                                title: p.name.clone(),
                                sub: p.curator.clone().unwrap_or_default(),
                                url: p.artwork.as_ref().map(|art| art.url(400)),
                                explicit: false,
                                addable: app.can_add_to_library(&Collection::Playlist(p.clone())),
                            })
                            .collect();
                        let shelf_id = Id::unique(("artist-playlists", &artist.artist.id.id));
                        if let Some((i, click)) = cover_shelf(
                            ui,
                            &theme,
                            pos2(left, y),
                            width,
                            "Artist Playlists",
                            &cards,
                            SHELF_CARD,
                            shelf_id,
                        ) {
                            let playlist = &playlists[i];
                            match click {
                                CardClick::Open => app.open_playlist(playlist),
                                CardClick::Enqueue(how) => app.enqueue_playlist(playlist, how, now),
                                CardClick::AddToLibrary => app.add_collection_to_library(
                                    &Collection::Playlist(playlist.clone()),
                                    now,
                                ),
                            }
                        }
                        y += shelf_height;
                    }
                    // Artists like them, in round photos, each leading to
                    // their page.
                    Section::Similar => {
                        let cards: Vec<ShelfCard> = artist
                            .similar
                            .iter()
                            .map(|a| ShelfCard {
                                title: a.name.clone(),
                                sub: String::new(),
                                url: a.artwork.as_ref().map(|art| art.url(400)),
                                explicit: false,
                                addable: false,
                            })
                            .collect();
                        let shelf_id = Id::unique(("artist-similar", &artist.artist.id.id));
                        if let Some(i) = round_shelf(
                            ui,
                            &theme,
                            pos2(left, y),
                            width,
                            "Similar Artists",
                            &cards,
                            SHELF_CARD,
                            shelf_id,
                        ) {
                            app.go(Page::Artist(artist.similar[i].id.clone()));
                        }
                        y += shelf_height;
                    }
                }
            }
        });
    output.state.clamped_offset().y
}

/// Spotify's artist page: their picture across the top with their name in
/// large type over its foot; Play, Shuffle and Follow; their popular songs
/// as a list beside their latest release; their discography by chips; then
/// the other shelves.
fn show_banner(app: &mut App, ui: &mut Ui, pad: f32, id: &ItemId) -> f32 {
    let theme = app.theme.clone();
    let (artist, error) = match app.artists.get(id) {
        Some(Loaded::Ready(a)) => (Some(a.clone()), None),
        Some(Loaded::Failed(e)) => (None, Some(e.clone())),
        _ => (None, None),
    };
    let width = ui.available_width() - 2.0 * pad;
    let hero = (ui.available_width() * 0.3).clamp(340.0, 500.0);
    let more_id = Id::unique(("artist-popular-more", &id.id));
    let more: bool = ui.data(|d| d.get_temp(more_id)).unwrap_or(false);
    let popular = artist
        .as_ref()
        .map_or(0, |a| a.top_songs.len().min(if more { 10 } else { 5 }));
    let has_more = artist.as_ref().is_some_and(|a| a.top_songs.len() > 5);
    // The discography's chips, those with releases.
    let releases: Vec<(&'static str, Vec<Album>)> = artist
        .as_ref()
        .map(|a| {
            let mut popular: Vec<Album> = a.latest.iter().cloned().collect();
            popular.extend(
                a.essentials
                    .iter()
                    .filter(|e| Some(&e.id) != a.latest.as_ref().map(|l| &l.id))
                    .cloned(),
            );
            vec![
                ("Popular releases", popular),
                ("Albums", a.albums.clone()),
                ("Singles and EPs", a.singles.clone()),
                ("Compilations", a.compilations.clone()),
            ]
            .into_iter()
            .filter(|(_, albums)| !albums.is_empty())
            .collect()
        })
        .unwrap_or_default();
    let shelf_height = SHELF_HEAD + SHELF_CARD + CARD_TEXT + SECTION_GAP;
    let video_height = SHELF_HEAD + VIDEO_CARD * 9.0 / 16.0 + CARD_TEXT + SECTION_GAP;
    let popular_height = if popular > 0 {
        SHELF_HEAD
            + 12.0
            + popular as f32 * POPULAR_ROW
            + if has_more { 44.0 } else { 0.0 }
            + SECTION_GAP
    } else {
        0.0
    };
    let discography_height = if releases.is_empty() {
        0.0
    } else {
        shelf_height + 44.0
    };
    let others = artist.as_ref().map_or(0.0, |a| {
        let mut h = 0.0;
        if !a.videos.is_empty() {
            h += video_height;
        }
        if !a.playlists.is_empty() {
            h += shelf_height;
        }
        if !a.appears_on.is_empty() {
            h += shelf_height;
        }
        if !a.similar.is_empty() {
            h += shelf_height;
        }
        h
    });
    let total = hero + BANNER_ACTIONS + popular_height + discography_height + others + 104.0;
    let colors = super::backdrop(app);

    let output = egui::ScrollArea::vertical()
        .id_salt(("artist", &id.id))
        .auto_shrink(false)
        .show(ui, |ui| {
            let (content, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), total), Sense::hover());
            let left = content.left() + pad;
            let top = Rect::from_min_size(content.min, vec2(content.width(), hero));
            // Under the picture, its colour fading into the page.
            let color = colors.map_or(theme.surface1, |[c, _, _]| c);
            let fade = Rect::from_min_size(top.left_bottom(), vec2(content.width(), 260.0));
            let mut mesh = Mesh::default();
            let shade = crate::color::mix(
                crate::color::mix(Color32::BLACK, color, 0.4),
                theme.base,
                0.8,
            );
            mesh.colored_vertex(fade.left_top(), shade);
            mesh.colored_vertex(fade.right_top(), shade);
            mesh.colored_vertex(fade.right_bottom(), theme.base);
            mesh.colored_vertex(fade.left_bottom(), theme.base);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            ui.painter().add(mesh);
            if let Some(artist) = &artist {
                picture(app, ui, &theme, artist, top);
            }
            if let Some(e) = &error {
                message(
                    ui,
                    &theme,
                    left,
                    top.bottom() + 16.0,
                    width,
                    "Couldn't load this artist",
                    e,
                );
                return;
            }
            let Some(artist) = &artist else {
                return;
            };
            // The name, as large as it fits.
            let white = Color32::WHITE;
            let name_font = [96.0, 72.0, 56.0, 40.0]
                .into_iter()
                .map(|size| font(size, Weight::Bold))
                .find(|f| {
                    text(ui, &artist.artist.name, f.clone(), white, f32::INFINITY)
                        .size()
                        .x
                        <= width
                })
                .unwrap_or(font(40.0, Weight::Bold));
            let name = text(ui, &artist.artist.name, name_font, white, width);
            let name_y = top.bottom() - 28.0 - name.size().y / 2.0;
            paint_text(ui, name, left, name_y, egui::Align::Min, white);

            // Play, Shuffle and Follow.
            let now = ui.input(|i| i.time);
            let mid = top.bottom() + BANNER_ACTIONS / 2.0;
            let play = Rect::from_center_size(pos2(left + 28.0, mid), vec2(56.0, 56.0));
            let response = ui
                .interact(play, Id::unique("artist-banner-play"), Sense::click())
                .on_hover_text("Play")
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let fill = if response.hovered() {
                crate::color::mix(theme.accent, Color32::WHITE, 0.85)
            } else {
                theme.accent
            };
            ui.painter().circle_filled(
                play.center(),
                28.0 + if response.hovered() { 2.0 } else { 0.0 },
                fill,
            );
            let icon = if app.page_playing() {
                icons::PAUSE_FILL
            } else {
                icons::PLAY_FILL
            };
            paint_icon(ui, icon, play.center(), 24.0, theme.on_accent());
            if response.clicked() {
                app.play_page(artist.top_songs.clone(), now);
            }
            let shuffle =
                Rect::from_center_size(pos2(play.right() + 16.0 + 20.0, mid), vec2(40.0, 40.0));
            // Shuffle is a switch, green with a dot while on, as Spotify's.
            let shuffling = app.shown().shuffle;
            let (color, tip) = if shuffling {
                (theme.accent, "Turn shuffle off")
            } else {
                (theme.subtext0, "Turn shuffle on")
            };
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(shuffle));
            if super::icon_button(&mut child, &theme, icons::SHUFFLE, 40.0, 26.0, color, true)
                .on_hover_text(tip)
                .clicked()
            {
                app.toggle_shuffle();
            }
            if shuffling {
                ui.painter()
                    .circle_filled(shuffle.center() + vec2(0.0, 17.0), 2.0, theme.accent);
            }
            let following = app.is_favourite(FavouriteKind::Artist, &artist.artist.id);
            let label = if following { "Following" } else { "Follow" };
            let f = font(14.0, Weight::Bold);
            let w = text(ui, label, f.clone(), theme.text, 200.0).size().x + 32.0;
            let follow =
                Rect::from_min_size(pos2(shuffle.right() + 16.0, mid - 16.0), vec2(w, 32.0));
            let hit = ui
                .interact(follow, Id::unique("artist-follow"), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let border = if hit.hovered() {
                theme.text
            } else {
                theme.text_a(0.5)
            };
            ui.painter().rect_stroke(
                follow,
                16.0,
                egui::Stroke::new(1.0, border),
                egui::StrokeKind::Inside,
            );
            let g = text(ui, label, f, theme.text, 200.0);
            paint_text(
                ui,
                g,
                follow.center().x,
                follow.center().y,
                egui::Align::Center,
                theme.text,
            );
            if hit.clicked() {
                app.toggle_favourite(FavouriteKind::Artist, artist.artist.id.clone());
            }

            let mut y = top.bottom() + BANNER_ACTIONS;
            if popular > 0 {
                // The latest release on the right, where there is room.
                let side = width > 900.0 && artist.latest.is_some();
                let list_width = if side { width * 0.62 } else { width };
                section_head(ui, &theme, left, y, "Popular");
                let rows_top = y + SHELF_HEAD + 12.0;
                let columns = super::tracks::Columns::album(left, list_width, false).with_thumbs();
                for (i, track) in artist.top_songs.iter().take(popular).enumerate() {
                    let rect = Rect::from_min_size(
                        pos2(left, rows_top + i as f32 * POPULAR_ROW),
                        vec2(list_width, POPULAR_ROW),
                    );
                    let current = app.is_current(track);
                    let state = super::tracks::RowState {
                        number: Some(i as u32 + 1),
                        selected: false,
                        now: current,
                        playing: current && app.is_playing(),
                        favourite: app.is_favourite(FavouriteKind::Song, &track.id),
                        in_library: app.track_in_library(track),
                        in_playlist: false,
                        list: "artist-popular",
                        position: i,
                    };
                    match super::tracks::row(ui, &theme, &columns, rect, track, state) {
                        super::tracks::RowAction::Play => {
                            app.play_list(artist.top_songs.clone(), i);
                        }
                        super::tracks::RowAction::OpenAlbum(id) => app.go(Page::Album(id)),
                        super::tracks::RowAction::Menu(choice) => {
                            app.track_choice(track.clone(), choice, now);
                        }
                        _ => {}
                    }
                }
                if has_more {
                    let label = if more { "Show less" } else { "See more" };
                    let g = text(ui, label, font(14.0, Weight::Bold), theme.subtext0, 200.0);
                    let at = pos2(left + 16.0, rows_top + popular as f32 * POPULAR_ROW + 22.0);
                    let rect = Rect::from_min_size(at - vec2(0.0, g.size().y / 2.0), g.size());
                    let hit = ui
                        .interact(rect, Id::unique("artist-see-more"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    let color = if hit.hovered() {
                        theme.text
                    } else {
                        theme.subtext0
                    };
                    let g = text(ui, label, font(14.0, Weight::Bold), color, 200.0);
                    paint_text(ui, g, at.x, at.y, egui::Align::Min, color);
                    if hit.clicked() {
                        ui.data_mut(|d| d.insert_temp(more_id, !more));
                    }
                }
                if side && let Some(latest) = &artist.latest {
                    let x = left + list_width + 48.0;
                    section_head(ui, &theme, x, y, "Latest release");
                    let art = Rect::from_min_size(pos2(x, rows_top), vec2(96.0, 96.0));
                    let tile = Rect::from_min_max(
                        art.min - vec2(8.0, 8.0),
                        pos2(left + width, art.bottom() + 8.0),
                    );
                    let hit = ui
                        .interact(tile, Id::unique("artist-latest-tile"), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if hit.hovered() {
                        ui.painter().rect_filled(tile, 8.0, theme.surface0);
                    }
                    artwork(
                        ui,
                        latest.artwork.as_ref().map(|a| a.url(240)),
                        art,
                        4.0,
                        &theme,
                    );
                    let tx = art.right() + 16.0;
                    let room = left + width - tx;
                    let muted = theme.subtext0;
                    let g = text(
                        ui,
                        &date_line(latest),
                        font(12.5, Weight::Regular),
                        muted,
                        room,
                    );
                    paint_text(ui, g, tx, art.top() + 22.0, egui::Align::Min, muted);
                    let g = text(
                        ui,
                        &latest.title,
                        font(16.0, Weight::Bold),
                        theme.text,
                        room,
                    );
                    paint_text(ui, g, tx, art.top() + 46.0, egui::Align::Min, theme.text);
                    let kind = match latest.track_count {
                        Some(1) => "Single".to_string(),
                        Some(n) => format!("Album · {n} songs"),
                        None => "Album".into(),
                    };
                    let g = text(ui, &kind, font(12.5, Weight::Regular), muted, room);
                    paint_text(ui, g, tx, art.top() + 70.0, egui::Align::Min, muted);
                    if hit.clicked() {
                        app.go(Page::Album(latest.id.clone()));
                    }
                }
                y += popular_height;
            }

            // The discography, by chips.
            if !releases.is_empty() {
                let chosen_id = Id::unique(("artist-discography", &id.id));
                let mut chosen: usize = ui.data(|d| d.get_temp(chosen_id)).unwrap_or(0);
                chosen = chosen.min(releases.len() - 1);
                section_head(ui, &theme, left, y, "Discography");
                let mut x = left;
                let f = font(14.0, Weight::Regular);
                for (k, (label, _)) in releases.iter().enumerate() {
                    let w = text(ui, label, f.clone(), theme.text, 200.0).size().x;
                    let rect =
                        Rect::from_min_size(pos2(x, y + SHELF_HEAD + 4.0), vec2(w + 24.0, 32.0));
                    x = rect.right() + 8.0;
                    let hit = ui
                        .interact(rect, Id::unique(("artist-chip", label)), Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    let on = k == chosen;
                    let (fill, ink) = if on {
                        (theme.text, theme.base)
                    } else if hit.hovered() {
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
                    if hit.clicked() {
                        chosen = k;
                    }
                }
                ui.data_mut(|d| d.insert_temp(chosen_id, chosen));
                let (label, albums) = &releases[chosen];
                albums_shelf(
                    app,
                    ui,
                    &theme,
                    artist,
                    label,
                    albums,
                    pos2(left, y + 44.0),
                    width,
                    false,
                );
                y += discography_height;
            }
            if !artist.videos.is_empty() {
                let cards: Vec<ShelfCard> = artist
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
                let shelf_id = Id::unique(("artist-videos", &artist.artist.id.id));
                if let Some(i) = video_shelf(
                    ui,
                    &theme,
                    pos2(left, y),
                    width,
                    "Music videos",
                    &cards,
                    VIDEO_CARD,
                    shelf_id,
                ) && let Some(url) = &artist.videos[i].url
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                }
                y += video_height;
            }
            if !artist.playlists.is_empty() {
                let cards: Vec<ShelfCard> = artist
                    .playlists
                    .iter()
                    .map(|p| ShelfCard {
                        title: p.name.clone(),
                        sub: p.curator.clone().unwrap_or_default(),
                        url: p.artwork.as_ref().map(|art| art.url(400)),
                        explicit: false,
                        addable: app.can_add_to_library(&Collection::Playlist(p.clone())),
                    })
                    .collect();
                let shelf_id = Id::unique(("artist-playlists", &artist.artist.id.id));
                let title = format!("Featuring {}", artist.artist.name);
                if let Some((i, click)) = cover_shelf(
                    ui,
                    &theme,
                    pos2(left, y),
                    width,
                    &title,
                    &cards,
                    SHELF_CARD,
                    shelf_id,
                ) {
                    let playlist = &artist.playlists[i];
                    match click {
                        CardClick::Open => app.open_playlist(playlist),
                        CardClick::Enqueue(how) => app.enqueue_playlist(playlist, how, now),
                        CardClick::AddToLibrary => app.add_collection_to_library(
                            &Collection::Playlist(playlist.clone()),
                            now,
                        ),
                    }
                }
                y += shelf_height;
            }
            if !artist.similar.is_empty() {
                let cards: Vec<ShelfCard> = artist
                    .similar
                    .iter()
                    .map(|a| ShelfCard {
                        title: a.name.clone(),
                        sub: "Artist".into(),
                        url: a.artwork.as_ref().map(|art| art.url(400)),
                        explicit: false,
                        addable: false,
                    })
                    .collect();
                let shelf_id = Id::unique(("artist-similar", &artist.artist.id.id));
                if let Some(i) = round_shelf(
                    ui,
                    &theme,
                    pos2(left, y),
                    width,
                    "Fans also like",
                    &cards,
                    SHELF_CARD,
                    shelf_id,
                ) {
                    app.go(Page::Artist(artist.similar[i].id.clone()));
                }
                y += shelf_height;
            }
            if !artist.appears_on.is_empty() {
                let albums = artist.appears_on.clone();
                albums_shelf(
                    app,
                    ui,
                    &theme,
                    artist,
                    "Appears On",
                    &albums,
                    pos2(left, y),
                    width,
                    true,
                );
            }
        });
    let offset = output.state.clamped_offset().y;
    if let Some(artist) = &artist {
        let shown = ((offset - (hero - 2.0 * super::BANNER_BAR)) / 48.0).clamp(0.0, 1.0);
        let color = colors.map_or(theme.surface1, |[c, _, _]| c);
        let main = output.inner_rect;
        let here = app.page_playing();
        let (_, play) =
            super::banner_bar(ui, &theme, main, color, &artist.artist.name, shown, here);
        if play {
            app.play_page(artist.top_songs.clone(), ui.input(|i| i.time));
        }
    }
    offset
}

/// The height of a popular song's row.
const POPULAR_ROW: f32 = 56.0;
/// The row of Play, Shuffle and Follow under the picture.
const BANNER_ACTIONS: f32 = 104.0;

/// Their wide picture, else their photo, filling `rect` and darkening at its
/// foot under their name.
fn picture(app: &mut App, ui: &Ui, theme: &Theme, artist: &ArtistDetail, rect: Rect) {
    let pixels = rect.width() * ui.ctx().pixels_per_point();
    let motion = artist
        .motion
        .as_deref()
        .and_then(|master| app.motion_frame("artist-hero", master, pixels, ui.ctx()));
    let (texture, photo) = match motion {
        Some(t) => (Some(t), false),
        None => {
            let (url, photo) = match &artist.banner {
                Some(url) => (Some(url.clone()), false),
                None => (artist.artist.artwork.as_ref().map(|a| a.url(1600)), true),
            };
            let texture = url.and_then(|url| {
                crate::covers::seen(ui.ctx(), &url);
                match egui::Image::new(url).load_for_size(ui.ctx(), rect.size()) {
                    Ok(egui::load::TexturePoll::Ready { texture }) => Some(texture),
                    _ => None,
                }
            });
            (texture, photo)
        }
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme.surface0);
    if let Some(texture) = texture {
        let scale = (rect.width() / texture.size.x).max(rect.height() / texture.size.y);
        let seen = rect.size() / (texture.size * scale);
        let u = (1.0 - seen.x) / 2.0;
        // A photo a little above its middle, where faces are.
        let v = (1.0 - seen.y) * if photo { 0.3 } else { 0.25 };
        let uv = Rect::from_min_size(pos2(u, v), seen);
        painter.image(texture.id, rect, uv, Color32::WHITE);
    }
    let mut mesh = Mesh::default();
    let stops = [
        (0.0, Color32::from_black_alpha(60)),
        (0.4, Color32::TRANSPARENT),
        (1.0, Color32::from_black_alpha(140)),
    ];
    for (k, (f, color)) in stops.iter().enumerate() {
        let y = rect.top() + rect.height() * f;
        mesh.colored_vertex(pos2(rect.left(), y), *color);
        mesh.colored_vertex(pos2(rect.right(), y), *color);
        if k > 0 {
            let i = (k * 2) as u32;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    painter.add(mesh);
}

/// A shelf of `albums` under `title`.
#[allow(clippy::too_many_arguments)]
fn albums_shelf(
    app: &mut App,
    ui: &mut Ui,
    theme: &Theme,
    artist: &ArtistDetail,
    title: &str,
    albums: &[Album],
    at: egui::Pos2,
    width: f32,
    by_artist: bool,
) {
    let spotify = theme.layout.header == crate::theme::Header::Banner;
    let cards: Vec<ShelfCard> = albums
        .iter()
        .map(|a| ShelfCard {
            title: a.title.clone(),
            // Spotify's `2024 • Album`, or who made it.
            sub: match (spotify, by_artist) {
                (true, true) => a.artists.join(", "),
                (true, false) => {
                    let kind = if a.track_count.is_some_and(|n| n <= 3) {
                        "Single"
                    } else {
                        "Album"
                    };
                    a.year.map_or(kind.to_string(), |y| format!("{y} • {kind}"))
                }
                _ => date_line(a),
            },
            url: a.artwork.as_ref().map(|art| art.url(400)),
            explicit: a.explicit,
            addable: app.can_add_album(&a.id),
        })
        .collect();
    let shelf_id = Id::unique(("artist-shelf", title, &artist.artist.id.id));
    // The discography's chips name the shelf above it.
    let shown = if spotify && !by_artist { "" } else { title };
    if let Some((i, click)) = cover_shelf(ui, theme, at, width, shown, &cards, SHELF_CARD, shelf_id)
    {
        let id = albums[i].id.clone();
        let now = ui.input(|i| i.time);
        match click {
            CardClick::Open => app.go(Page::Album(id)),
            CardClick::AddToLibrary => {
                app.add_collection_to_library(&Collection::Album(albums[i].clone()), now);
            }
            CardClick::Enqueue(how) => app.enqueue_album(id, how, now),
        }
    }
}

/// The artist's video, else their wide picture, else their photo, filling
/// the width from the top and cut at the foot of `rect`, where it fades into
/// the page. A shade at the top keeps the top bar's buttons legible.
fn banner(app: &mut App, ui: &Ui, theme: &Theme, artist: &ArtistDetail, rect: Rect) {
    let area = Rect::from_min_size(rect.min, vec2(rect.width(), rect.width() * 9.0 / 16.0));
    let pixels = rect.width() * ui.ctx().pixels_per_point();
    let motion = artist
        .motion
        .as_deref()
        .and_then(|master| app.motion_frame("artist-hero", master, pixels, ui.ctx()));
    let (texture, photo) = match motion {
        Some(t) => (Some(t), false),
        None => {
            let (url, photo) = match &artist.banner {
                Some(url) => (Some(url.clone()), false),
                None => (artist.artist.artwork.as_ref().map(|a| a.url(1600)), true),
            };
            let texture = url.and_then(|url| {
                crate::covers::seen(ui.ctx(), &url);
                match egui::Image::new(url).load_for_size(ui.ctx(), area.size()) {
                    Ok(egui::load::TexturePoll::Ready { texture }) => Some(texture),
                    _ => None,
                }
            });
            (texture, photo)
        }
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme.crust);
    if let Some(texture) = texture {
        // Filling the area: what overflows is cut, a photo's a little above
        // its middle, where faces are.
        let scale = (area.width() / texture.size.x).max(area.height() / texture.size.y);
        let seen = area.size() / (texture.size * scale);
        let u = (1.0 - seen.x) / 2.0;
        let v = (1.0 - seen.y) * if photo { 0.3 } else { 0.0 };
        let uv = Rect::from_min_size(pos2(u, v), seen);
        painter.image(texture.id, area, uv, Color32::WHITE);
    }
    // Into the page over the lower half; a shade under the top bar.
    let mut mesh = Mesh::default();
    let stops = [
        (0.0, Color32::from_black_alpha(150)),
        (0.22, Color32::TRANSPARENT),
        (0.45, Color32::TRANSPARENT),
        (0.75, crate::color::alpha(theme.base, 0.55)),
        (1.0, theme.base),
    ];
    for (k, (f, color)) in stops.iter().enumerate() {
        let y = rect.top() + rect.height() * f;
        mesh.colored_vertex(pos2(rect.left(), y), *color);
        mesh.colored_vertex(pos2(rect.right(), y), *color);
        if k > 0 {
            let i = (k * 2) as u32;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    painter.add(mesh);
}

/// `MAR 27, 2026`, or the year alone.
fn date_line(album: &Album) -> String {
    album
        .released
        .as_deref()
        .and_then(format_date)
        .or_else(|| album.year.map(|y| y.to_string()))
        .unwrap_or_default()
}

/// What a round button under the name asked for.
#[derive(Clone, Copy, PartialEq)]
enum HeroButton {
    Play,
    Shuffle,
    Favourite,
}

/// Shuffle, the big round Play and the favourite star, from `start` (the
/// left end of their row, at its middle), dark over the picture.
fn hero_buttons(
    ui: &mut Ui,
    theme: &Theme,
    start: egui::Pos2,
    favourite: bool,
) -> Option<HeroButton> {
    let (big, small, gap) = (26.0, 19.0, 12.0);
    let side = big + gap + small;
    let center = start + vec2(small + gap + big, 0.0);
    let star = if favourite {
        icons::STAR_FILL
    } else {
        icons::STAR
    };
    let buttons = [
        (
            center - vec2(side, 0.0),
            small,
            icons::SHUFFLE,
            17.0,
            HeroButton::Shuffle,
            "Shuffle",
        ),
        (
            center,
            big,
            icons::PLAY_FILL,
            22.0,
            HeroButton::Play,
            "Play",
        ),
        (
            center + vec2(side, 0.0),
            small,
            star,
            17.0,
            HeroButton::Favourite,
            if favourite {
                "Remove from Favourites"
            } else {
                "Favourite"
            },
        ),
    ];
    let mut clicked = None;
    for (c, radius, icon, size, button, label) in buttons {
        let rect = Rect::from_center_size(c, vec2(radius * 2.0, radius * 2.0));
        let response = ui.interact(rect, Id::unique(("artist-hero", label)), Sense::click());
        let fill = Color32::from_black_alpha(if response.hovered() { 150 } else { 105 });
        ui.painter().circle_filled(c, radius, fill);
        let color = if button == HeroButton::Favourite && favourite {
            theme.red
        } else {
            Color32::WHITE
        };
        // The triangle is drawn off-centre in its icon already, so that it
        // looks centred.
        paint_icon(ui, icon, c, size, color);
        if response
            .on_hover_text(label)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            clicked = Some(button);
        }
    }
    clicked
}

/// The latest release on the left, then the top songs three to a column,
/// scrolling sideways.
fn songs_section(
    app: &mut App,
    ui: &mut Ui,
    theme: &Theme,
    artist: &ArtistDetail,
    at: egui::Pos2,
    width: f32,
) {
    let mut x = at.x;
    let rows_top = at.y + SHELF_HEAD;
    if let Some(latest) = &artist.latest {
        let art = Rect::from_min_size(
            pos2(x, rows_top + (SONG_COLUMNS_HEIGHT - LATEST_ART) / 2.0),
            vec2(LATEST_ART, LATEST_ART),
        );
        let response = ui.interact(art, Id::unique("artist-latest"), Sense::click());
        artwork(
            ui,
            latest.artwork.as_ref().map(|a| a.url(320)),
            art,
            8.0,
            theme,
        );
        if response.hovered() {
            ui.painter()
                .rect_filled(art, 8.0, egui::Color32::from_black_alpha(40));
        }
        let tx = art.right() + 16.0;
        let room = LATEST_WIDTH - LATEST_ART - 16.0;
        let muted = theme.muted();
        let mut ty = art.center().y - 30.0;
        let g = text(
            ui,
            &date_line(latest),
            font(11.0, Weight::SemiBold),
            muted,
            room,
        );
        paint_text(ui, g, tx, ty, egui::Align::Min, muted);
        ty += 20.0;
        let g = text(
            ui,
            &latest.title,
            font(14.0, Weight::Medium),
            theme.text,
            room,
        );
        paint_text(ui, g, tx, ty, egui::Align::Min, theme.text);
        ty += 20.0;
        if let Some(count) = latest.track_count {
            let songs = format!("{count} Song{}", if count == 1 { "" } else { "s" });
            let g = text(ui, &songs, font(12.0, Weight::Regular), muted, room);
            paint_text(ui, g, tx, ty, egui::Align::Min, muted);
        }
        // Add, while it isn't in the library.
        if app.can_add_album(&latest.id) {
            let add = Rect::from_min_size(pos2(tx, ty + 18.0), vec2(64.0, 28.0));
            let hit = ui.interact(add, Id::unique("artist-latest-add"), Sense::click());
            let fill = theme.text_a(if hit.hovered() { 0.14 } else { 0.08 });
            ui.painter().rect_filled(add, 8.0, fill);
            paint_icon(
                ui,
                icons::PLUS,
                add.left_center() + vec2(15.0, 0.0),
                12.0,
                theme.text,
            );
            let g = text(ui, "Add", font(12.5, Weight::Medium), theme.text, 40.0);
            paint_text(
                ui,
                g,
                add.left() + 26.0,
                add.center().y,
                egui::Align::Min,
                theme.text,
            );
            if hit
                .on_hover_text("Add to Library")
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                let album = Collection::Album(latest.clone());
                app.add_collection_to_library(&album, ui.input(|i| i.time));
            }
        }
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            app.go(Page::Album(latest.id.clone()));
        }
        x += LATEST_WIDTH + 28.0;
    }
    if artist.top_songs.is_empty() {
        return;
    }

    let room = at.x + width - x;
    section_head(ui, theme, x, at.y, "Top Songs");
    let row = Rect::from_min_size(pos2(x, rows_top), vec2(room, SONG_COLUMNS_HEIGHT));
    let id = Id::unique(("artist-songs", &artist.artist.id.id));
    song_columns(
        app,
        ui,
        theme,
        row,
        pos2(at.x + width, at.y + 11.0),
        &artist.top_songs,
        id,
        true,
        3,
    );
}
