//! Context menus, as the design draws them: a `mantle` card with a hairline,
//! 32 px items with an icon, the hovered one filled with the accent.

use egui::{Frame, Response, Sense, Stroke, Ui, pos2, vec2};

use crate::app::{Enqueue, PlaylistChoice, TrackChoice};
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

use super::{paint_icon, paint_text, text};

/// A menu line: an item with its icon, label and what it means, or a hairline.
pub(super) enum Entry<T> {
    Item(Icon, &'static str, T),
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
                    if item(ui, theme, *icon, label) {
                        chosen = Some(value.clone());
                        ui.close();
                    }
                }
            }
        }
    });
    chosen
}

fn item(ui: &mut Ui, theme: &Theme, icon: Icon, label: &str) -> bool {
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
    response.clicked()
}

/// What the track menu knows about its track.
pub(super) struct TrackFacts {
    pub has_album: bool,
    pub favourite: bool,
    pub in_library: bool,
    /// Its library id is known, so it can be taken out of the library.
    pub removable: bool,
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
    if facts.removable {
        entries.push(Entry::Line);
        entries.push(Entry::Item(
            icons::TRASH,
            "Remove from Library",
            TrackChoice::RemoveFromLibrary,
        ));
    }
    entries
}

/// A library playlist's menu in the sidebar: playing it, then renaming it if
/// it is the user's own, and deleting it unless it is Apple's favourites.
pub(super) fn playlist_menu(
    response: &Response,
    theme: &Theme,
    editable: bool,
    deletable: bool,
) -> Option<PlaylistChoice> {
    let mut entries = vec![
        Entry::Item(
            icons::PLAY_FILL,
            "Play",
            PlaylistChoice::Enqueue(Enqueue::Now),
        ),
        Entry::Line,
        Entry::Item(
            icons::NEXT,
            "Play Next",
            PlaylistChoice::Enqueue(Enqueue::Next),
        ),
        Entry::Item(
            icons::LATER,
            "Add to Queue",
            PlaylistChoice::Enqueue(Enqueue::Last),
        ),
    ];
    if editable || deletable {
        entries.push(Entry::Line);
    }
    if editable {
        entries.push(Entry::Item(icons::EDIT, "Rename…", PlaylistChoice::Rename));
    }
    if deletable {
        entries.push(Entry::Item(
            icons::TRASH,
            "Delete from Library…",
            PlaylistChoice::Delete,
        ));
    }
    context_menu(response, theme, &entries)
}

/// What an album's or a playlist's menu offers.
pub(super) fn collection_menu(response: &Response, theme: &Theme) -> Option<Enqueue> {
    let entries = [
        Entry::Item(icons::PLAY_FILL, "Play", Enqueue::Now),
        Entry::Line,
        Entry::Item(icons::NEXT, "Play Next", Enqueue::Next),
        Entry::Item(icons::LATER, "Add to Queue", Enqueue::Last),
    ];
    context_menu(response, theme, &entries)
}
