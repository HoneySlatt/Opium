//! Entry points to the readers of what Apple sends, for the `cargo fuzz` targets of
//! `crates/apple-music/fuzz`: built only with the crate's `fuzzing` feature, hidden from its
//! documentation, and nothing else. The readers stay private; each entry point takes what
//! Apple sends as it comes (bytes or text) and answers what the reader made of it in plain
//! types, so that a target can check the invariants the mutation tests check.
//!
//! Here: the ids, the next page's address and a station's gathering, which live at the
//! crate's root and in `station`; the readers of `playback` and `auth` are their files' own
//! `fuzzing` modules, re-exported.

pub use crate::auth::fuzzing::{first_token, main_script, names_es256, script_url};
pub use crate::playback::fuzzing::{
    Asset, CatalogFile, Chosen, Entry, aac_rate, catalog_file, enhanced_file, exoplayer_playlist,
    id_kind, licence, map_range, playback_request, song_id, variant, web_playback_entry,
    widevine_playlist,
};

/// The most calls a station's gathering sends.
pub const MOST_CALLS: usize = crate::station::MOST_CALLS;

/// The most calls a station's gathering has in flight at once.
pub const MOST_IN_FLIGHT: usize = crate::station::MOST_IN_FLIGHT;

/// Whether `id` may go unescaped into a request that changes the account.
pub fn apple_id(id: &str) -> bool {
    crate::apple_id(id).is_ok()
}

/// `id`, when it may go unescaped into a station's address.
pub fn station_id(id: &str) -> Option<&str> {
    crate::station::station_id(id).ok()
}

/// The address of the page after `first`'s, whose answer said `next`.
pub fn next_page_path(next: &str, first: &str) -> String {
    crate::next_page_path(next, first)
}

/// The address of the page of `next`'s list at `offset`, with `first`'s parameters.
pub fn at_offset(next: &str, offset: usize, first: &str) -> String {
    crate::at_offset(next, offset, first)
}

/// Where the page `next` names starts.
pub fn offset_in(next: &str) -> Option<usize> {
    crate::offset_in(next)
}

/// A station's gathering run to its end.
#[derive(Debug)]
pub struct Gathered {
    /// The songs' ids, in the order they were kept, or the error's message.
    pub songs: Result<Vec<String>, String>,
    /// How many calls were sent.
    pub calls_sent: usize,
    /// The most calls that were in flight at once.
    pub most_in_flight: usize,
}

/// A gathering of `count` songs from a station whose calls answer `next_answer()` in turn,
/// each body read as a `next-tracks` answer: the steps `station_tracks` takes, the calls
/// answering one at a time, as its stream of calls hands them over.
pub fn gather_station(count: usize, mut next_answer: impl FnMut() -> Vec<u8>) -> Gathered {
    let mut gathering = crate::station::Gathering::new(count);
    let mut in_flight = 0;
    let mut most_in_flight = 0;
    let mut calls_sent = 0;
    loop {
        let more = gathering.calls_to_send();
        gathering.sent(more);
        calls_sent += more;
        in_flight += more;
        most_in_flight = usize::max(most_in_flight, in_flight);
        if gathering.is_over() {
            break;
        }
        let body = next_answer();
        in_flight -= 1;
        gathering.answered(crate::station::songs_in(
            "/v1/me/stations/next-tracks/ra.fuzz",
            &String::from_utf8_lossy(&body),
        ));
    }
    Gathered {
        songs: gathering
            .into_songs()
            .map(|songs| songs.into_iter().map(|song| song.id.id).collect())
            .map_err(|e| e.to_string()),
        calls_sent,
        most_in_flight,
    }
}
