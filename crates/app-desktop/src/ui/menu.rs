//! Context menus, as the design draws them: a `mantle` card with a hairline,
//! 32 px items with an icon, the hovered one filled with the accent.

use egui::{Frame, Response, Sense, Stroke, Ui, pos2, vec2};

use crate::app::{CollectionChoice, Enqueue, TrackChoice};
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::settings::AlbumSort;
use crate::theme::Theme;

use super::{paint_icon, paint_text, text};

/// A menu line: an item with its icon, label and what it means, or a hairline.
pub(super) enum Entry<T> {
    Item(Icon, &'static str, T),
    /// An item with a check after it when it is the one chosen.
    Choice(Icon, &'static str, T, bool),
    Line,
}

/// Open a menu on a secondary click of `response`; returns the item chosen.
pub(super) fn context_menu<T: Clone>(
    response: &Response,
    theme: &Theme,
    entries: &[Entry<T>],
) -> Option<T> {
    menu(response, theme, entries, false)
}

/// The menu, opened by a secondary click on `response`, or by a primary one
/// (a "…" button).
fn menu<T: Clone>(
    response: &Response,
    theme: &Theme,
    entries: &[Entry<T>],
    on_click: bool,
) -> Option<T> {
    let mut chosen = None;
    let frame = Frame::new()
        .fill(theme.mantle)
        .stroke(Stroke::new(1.0, theme.line()))
        .corner_radius(14)
        .inner_margin(5)
        .shadow(egui::Shadow {
            offset: [0, 10],
            blur: 30,
            spread: 0,
            color: egui::Color32::from_black_alpha(110),
        });
    let popup = if on_click {
        egui::Popup::menu(response)
    } else {
        egui::Popup::context_menu(response)
    };
    popup.frame(frame).width(220.0).show(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        for entry in entries {
            match entry {
                Entry::Line => {
                    let (rect, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 9.0), Sense::hover());
                    ui.painter().hline(
                        (rect.left() + 6.0)..=(rect.right() - 6.0),
                        rect.center().y,
                        Stroke::new(1.0, theme.line()),
                    );
                }
                Entry::Item(icon, label, value) => {
                    if item(ui, theme, *icon, label, false) {
                        chosen = Some(value.clone());
                        ui.close();
                    }
                }
                Entry::Choice(icon, label, value, on) => {
                    if item(ui, theme, *icon, label, *on) {
                        chosen = Some(value.clone());
                        ui.close();
                    }
                }
            }
        }
    });
    chosen
}

fn item(ui: &mut Ui, theme: &Theme, icon: Icon, label: &str, checked: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
    let (fg, glyph) = if response.hovered() {
        ui.painter().rect_filled(rect, 7.0, theme.accent);
        (theme.on_accent(), theme.on_accent())
    } else {
        (theme.text, theme.subtext0)
    };
    paint_icon(
        ui,
        icon,
        pos2(rect.left() + 10.0 + 8.0, rect.center().y),
        16.0,
        glyph,
    );
    let g = text(
        ui,
        label,
        font(13.0, Weight::Regular),
        fg,
        rect.width() - 46.0,
    );
    paint_text(
        ui,
        g,
        rect.left() + 10.0 + 16.0 + 10.0,
        rect.center().y,
        egui::Align::Min,
        fg,
    );
    if checked {
        paint_icon(
            ui,
            icons::CHECK,
            pos2(rect.right() - 10.0 - 7.0, rect.center().y),
            14.0,
            glyph,
        );
    }
    response.clicked()
}

/// What the Albums page's sort menu asked for.
#[derive(Clone, Copy)]
pub(super) enum SortChoice {
    By(AlbumSort),
    Ascending(bool),
}

/// The Albums page's sort menu, opened by a click on `response`: what to sort
/// by, then which way, each with a check by the one in use.
/// The services to list the songs of, under a button: all, or one of
/// `services`. The one chosen.
pub(super) fn sources_menu(
    response: &Response,
    theme: &Theme,
    current: Option<music_core::model::ProviderKind>,
    services: &[music_core::model::ProviderKind],
) -> Option<Option<music_core::model::ProviderKind>> {
    use music_core::model::ProviderKind;
    let mut entries = vec![Entry::Choice(
        icons::LIST,
        "All services",
        None,
        current.is_none(),
    )];
    for (provider, icon, label) in [
        (ProviderKind::AppleMusic, icons::APPLE_MUSIC, "Apple Music"),
        (ProviderKind::SoundCloud, icons::SOUNDCLOUD, "SoundCloud"),
        (ProviderKind::Navidrome, icons::NAVIDROME, "Navidrome"),
    ] {
        if services.contains(&provider) {
            entries.push(Entry::Choice(
                icon,
                label,
                Some(provider),
                current == Some(provider),
            ));
        }
    }
    menu(response, theme, &entries, true)
}

pub(super) fn albums_sort_menu(
    response: &Response,
    theme: &Theme,
    by: AlbumSort,
    ascending: bool,
) -> Option<SortChoice> {
    let entries = [
        Entry::Choice(
            icons::HISTORY,
            "Date Added",
            SortChoice::By(AlbumSort::Added),
            by == AlbumSort::Added,
        ),
        Entry::Choice(
            icons::TYPE,
            "Name",
            SortChoice::By(AlbumSort::Name),
            by == AlbumSort::Name,
        ),
        Entry::Choice(
            icons::USER,
            "Artist",
            SortChoice::By(AlbumSort::Artist),
            by == AlbumSort::Artist,
        ),
        Entry::Choice(
            icons::CALENDAR,
            "Release Date",
            SortChoice::By(AlbumSort::Released),
            by == AlbumSort::Released,
        ),
        Entry::Line,
        Entry::Choice(
            icons::ARROW_UP,
            "Ascending",
            SortChoice::Ascending(true),
            ascending,
        ),
        Entry::Choice(
            icons::ARROW_DOWN,
            "Descending",
            SortChoice::Ascending(false),
            !ascending,
        ),
    ];
    menu(response, theme, &entries, true)
}

/// What the track menu knows about its track.
pub(super) struct TrackFacts {
    pub has_album: bool,
    pub favourite: bool,
    pub in_library: bool,
    /// Its library id is known, so it can be taken out of the library.
    pub removable: bool,
    /// It is shown in a playlist of the user's, which it can leave.
    pub in_playlist: bool,
}

/// The track menu; Add to Library only when the song isn't there yet, Go to
/// Album only when the album is known.
pub(super) fn track_menu(
    response: &Response,
    theme: &Theme,
    facts: TrackFacts,
) -> Option<TrackChoice> {
    menu(response, theme, &track_entries(facts), false)
}

/// The track menu, opened by a click on a "…" button.
pub(super) fn track_menu_button(
    response: &Response,
    theme: &Theme,
    facts: TrackFacts,
) -> Option<TrackChoice> {
    menu(response, theme, &track_entries(facts), true)
}

fn track_entries(facts: TrackFacts) -> Vec<Entry<TrackChoice>> {
    let star = if facts.favourite {
        Entry::Item(
            icons::STAR_FILL,
            "Remove from Favourites",
            TrackChoice::Favourite,
        )
    } else {
        Entry::Item(icons::STAR, "Favourite", TrackChoice::Favourite)
    };
    let mut entries = vec![
        Entry::Item(icons::NEXT, "Play Next", TrackChoice::PlayNext),
        Entry::Item(icons::LATER, "Add to Queue", TrackChoice::AddToQueue),
        Entry::Item(icons::RADIO, "Start Radio", TrackChoice::StartRadio),
        Entry::Line,
        star,
    ];
    if !facts.in_library {
        entries.push(Entry::Item(
            icons::PLUS,
            "Add to Library",
            TrackChoice::AddToLibrary,
        ));
    }
    entries.push(Entry::Item(
        icons::LIST,
        "Add to Playlist…",
        TrackChoice::AddToPlaylist,
    ));
    entries.push(Entry::Line);
    entries.push(Entry::Item(
        icons::MIC,
        "Go to Artist",
        TrackChoice::GoToArtist,
    ));
    if facts.has_album {
        entries.push(Entry::Item(
            icons::DISC,
            "Go to Album",
            TrackChoice::GoToAlbum,
        ));
    }
    if facts.in_playlist || facts.removable {
        entries.push(Entry::Line);
    }
    if facts.in_playlist {
        entries.push(Entry::Item(
            icons::MINUS,
            "Remove from Playlist",
            TrackChoice::RemoveFromPlaylist,
        ));
    }
    if facts.removable {
        entries.push(Entry::Item(
            icons::TRASH,
            "Remove from Library",
            TrackChoice::RemoveFromLibrary,
        ));
    }
    entries
}

/// What an album's or a playlist's menu offers besides playing it.
#[derive(Clone, Copy, Default)]
pub(crate) struct CollectionFacts {
    /// An album or a playlist: songs to shuffle and add to playlists. A
    /// station's menu only plays it.
    pub songs: bool,
    /// A playlist, not an album: its Share and Delete say so.
    pub playlist: bool,
    /// Not wholly in the library: Add to Library.
    pub addable: bool,
    /// In the user's library, to delete from it.
    pub deletable: bool,
    /// The user's own playlist, to rename.
    pub editable: bool,
    /// Whether it is a favourite, when it can be one.
    pub favourite: Option<bool>,
    /// Whether less like it was asked for, when that can be.
    pub less: Option<bool>,
    /// Its artist's page is known.
    pub artist: bool,
    /// It has a station of songs like it.
    pub radio: bool,
    /// It has a page on the service's website.
    pub shareable: bool,
}

/// An album's or a playlist's menu, as the phone's: playing it, the library
/// and the favourites, where it leads, and deleting it.
pub(super) fn collection_menu(
    response: &Response,
    theme: &Theme,
    facts: CollectionFacts,
) -> Option<CollectionChoice> {
    let mut entries = vec![Entry::Item(
        icons::PLAY_FILL,
        "Play",
        CollectionChoice::Enqueue(Enqueue::Now),
    )];
    if facts.songs {
        entries.push(Entry::Item(
            icons::SHUFFLE,
            "Shuffle",
            CollectionChoice::Shuffle,
        ));
    }
    entries.extend([
        Entry::Line,
        Entry::Item(
            icons::NEXT,
            "Play Next",
            CollectionChoice::Enqueue(Enqueue::Next),
        ),
        Entry::Item(
            icons::LATER,
            "Add to Queue",
            CollectionChoice::Enqueue(Enqueue::Last),
        ),
    ]);
    if !facts.songs {
        return finish(response, theme, &entries);
    }
    entries.push(Entry::Line);
    let delete = || {
        Entry::Item(
            icons::TRASH,
            "Delete from Library…",
            CollectionChoice::Delete,
        )
    };
    if facts.addable {
        entries.push(Entry::Item(
            icons::PLUS,
            "Add to Library",
            CollectionChoice::AddToLibrary,
        ));
    } else if facts.deletable && !facts.playlist {
        entries.push(delete());
    }
    entries.push(Entry::Item(
        icons::LIST,
        "Add to Playlist…",
        CollectionChoice::AddToPlaylist,
    ));
    if facts.editable {
        entries.push(Entry::Item(icons::EDIT, "Edit…", CollectionChoice::Edit));
    }
    match facts.favourite {
        Some(true) => entries.push(Entry::Item(
            icons::STAR_FILL,
            "Remove from Favourites",
            CollectionChoice::Favourite,
        )),
        Some(false) => entries.push(Entry::Item(
            icons::STAR,
            "Favourite",
            CollectionChoice::Favourite,
        )),
        None => {}
    }
    match facts.less {
        Some(true) => entries.push(Entry::Item(
            icons::THUMBS_DOWN,
            "Undo Suggest Less",
            CollectionChoice::SuggestLess,
        )),
        Some(false) => entries.push(Entry::Item(
            icons::THUMBS_DOWN,
            "Suggest Less",
            CollectionChoice::SuggestLess,
        )),
        None => {}
    }
    if facts.artist || facts.radio || facts.shareable {
        entries.push(Entry::Line);
    }
    if facts.artist {
        entries.push(Entry::Item(
            icons::MIC,
            "Go to Artist",
            CollectionChoice::GoToArtist,
        ));
    }
    if facts.radio {
        entries.push(Entry::Item(
            icons::RADIO,
            "Start Radio",
            CollectionChoice::StartRadio,
        ));
    }
    if facts.shareable {
        let label = if facts.playlist {
            "Share Playlist"
        } else {
            "Share Album"
        };
        entries.push(Entry::Item(icons::SHARE, label, CollectionChoice::Share));
    }
    if facts.deletable && facts.playlist {
        entries.push(Entry::Line);
        entries.push(delete());
    }
    finish(response, theme, &entries)
}

fn finish(
    response: &Response,
    theme: &Theme,
    entries: &[Entry<CollectionChoice>],
) -> Option<CollectionChoice> {
    let chosen = context_menu(response, theme, entries);
    // Opened now: what it says of the favourites is asked of the service.
    if chosen.is_none() && response.secondary_clicked() {
        return Some(CollectionChoice::Opened);
    }
    chosen
}
