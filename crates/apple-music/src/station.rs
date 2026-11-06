//! A station's next songs, gathered from calls that each bring a few.
//!
//! `next-tracks` answers about two songs a call, and concurrent calls share the station's
//! cursor, so calls run side by side: as many as the songs still missing need, at most
//! [`MOST_IN_FLIGHT`] at once and [`MOST_CALLS`] in all. A station that repeats itself, or
//! answers nothing new, [`FRUITLESS_TO_END`] times in a row ends the gathering early.
//!
//! [`Gathering`] holds the decisions, apart from the requests, so that tests can run it
//! against a simulated station.

use std::collections::HashSet;

use opium_core::model::Track;
use opium_core::{Error, Result};

use crate::{convert, types};

/// How many songs a call is counted on to bring: Apple's usual answer.
const SONGS_PER_CALL: usize = 2;
/// The most calls in flight at once.
pub(crate) const MOST_IN_FLIGHT: usize = 6;
/// The most calls a gathering sends.
pub(crate) const MOST_CALLS: usize = 32;
/// How many answers in a row that bring no new song, failures included, end the gathering.
pub(crate) const FRUITLESS_TO_END: usize = 3;

/// A gathering of up to `count` songs under way: what came, and what was sent.
///
/// Invariants: at most `count` songs, each once (by id), in the order they came; at most
/// [`MOST_IN_FLIGHT`] calls in flight and [`MOST_CALLS`] sent.
#[derive(Debug)]
pub(crate) struct Gathering {
    count: usize,
    songs: Vec<Track>,
    ids: HashSet<String>,
    sent: usize,
    in_flight: usize,
    /// Answers in a row that brought no new song.
    fruitless: usize,
    first_failure: Option<Error>,
}

impl Gathering {
    /// A gathering of up to `count` songs, nothing sent yet. With a `count` of 0, it sends
    /// nothing and ends with no song.
    pub fn new(count: usize) -> Self {
        Self {
            count,
            songs: Vec::new(),
            ids: HashSet::new(),
            sent: 0,
            in_flight: 0,
            fruitless: 0,
            first_failure: None,
        }
    }

    /// How many calls to send now: enough, with those in flight, to bring the songs still
    /// missing, within the bounds; none once the station has stopped bringing new ones.
    pub fn calls_to_send(&self) -> usize {
        if self.fruitless >= FRUITLESS_TO_END {
            return 0;
        }
        let missing = self.count.saturating_sub(self.songs.len());
        let wanted = missing.div_ceil(SONGS_PER_CALL).min(MOST_IN_FLIGHT);
        wanted
            .saturating_sub(self.in_flight)
            .min(MOST_CALLS - self.sent)
    }

    /// `calls` more were sent.
    pub fn sent(&mut self, calls: usize) {
        self.sent += calls;
        self.in_flight += calls;
    }

    /// A call answered `songs`, or failed.
    pub fn answered(&mut self, songs: Result<Vec<Track>>) {
        self.in_flight = self.in_flight.saturating_sub(1);
        let songs = match songs {
            Ok(songs) => songs,
            Err(e) => {
                self.first_failure.get_or_insert(e);
                self.fruitless += 1;
                return;
            }
        };
        let before = self.songs.len();
        for song in songs {
            if self.songs.len() == self.count {
                break;
            }
            if self.ids.insert(song.id.id.clone()) {
                self.songs.push(song);
            }
        }
        if self.songs.len() > before {
            self.fruitless = 0;
        } else {
            self.fruitless += 1;
        }
    }

    /// Whether nothing more is to come: every song wanted is in, or no call is in flight
    /// once those to send were sent.
    pub fn is_over(&self) -> bool {
        self.songs.len() >= self.count || self.in_flight == 0
    }

    /// The songs gathered.
    ///
    /// # Errors
    ///
    /// When none came: the first failure met, else [`Error::Protocol`] saying the station
    /// offered none.
    pub fn into_songs(self) -> Result<Vec<Track>> {
        if !self.songs.is_empty() || self.count == 0 {
            return Ok(self.songs);
        }
        Err(self
            .first_failure
            .unwrap_or_else(|| Error::Protocol("the station offered no songs".into())))
    }

    /// How many calls were sent.
    #[cfg(test)]
    pub fn calls_sent(&self) -> usize {
        self.sent
    }

    /// How many calls are in flight.
    #[cfg(test)]
    pub fn calls_in_flight(&self) -> usize {
        self.in_flight
    }
}

/// `id`, when it is a station's id that may go into an address unescaped as one path
/// segment: one or more ASCII letters, digits, `-`, `.` or `_`, and neither `.` nor `..`,
/// which an address would read as the folder itself or its parent. Apple's ids take more
/// forms than [`crate::apple_id`]'s: a playlist's station is `ra.cp-pl.u-<letters and
/// digits>`.
///
/// # Errors
///
/// [`Error::Protocol`], naming `id`, for anything else: the station is asked nothing.
pub(crate) fn station_id(id: &str) -> Result<&str> {
    let one_segment = !id.is_empty()
        && id != "."
        && id != ".."
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_'));
    if one_segment {
        Ok(id)
    } else {
        Err(Error::Protocol(format!("not a station id: {id:?}")))
    }
}

/// The songs of a `next-tracks` answer `body`, to the request for `path`: its `data`, each
/// read as a song, those that don't read left out.
///
/// # Errors
///
/// [`Error::Protocol`], naming `path`, for an answer that does not read.
pub(crate) fn songs_in(path: &str, body: &str) -> Result<Vec<Track>> {
    let answer: types::Response<types::Song> =
        serde_json::from_str(body).map_err(|e| Error::Protocol(format!("{path}: {e}")))?;
    Ok(answer.data.into_iter().map(convert::track).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use opium_core::model::{ItemId, ProviderKind};

    pub(crate) fn song(id: &str) -> Track {
        Track {
            id: ItemId::new(ProviderKind::AppleMusic, id),
            title: format!("Made-up song {id}"),
            artists: vec!["Nobody Real".into()],
            album: None,
            album_id: None,
            duration: None,
            track_number: None,
            disc_number: None,
            artwork: None,
            explicit: false,
            quality: None,
            in_library: false,
            library_id: None,
        }
    }

    /// Ticks until `count` songs are gathered from a station whose every call answers in one
    /// tick, all at the same time, with `per_call` new songs; and the calls sent, and the most
    /// in flight.
    fn simulate(count: usize, per_call: usize) -> (usize, usize, usize) {
        let mut gathering = Gathering::new(count);
        let mut next_id = 0;
        let mut ticks = 0;
        let mut peak = 0;
        let to_send = gathering.calls_to_send();
        gathering.sent(to_send);
        let mut in_flight = to_send;
        while !gathering.is_over() {
            ticks += 1;
            peak = peak.max(gathering.calls_in_flight());
            // This tick's calls answer one after another; those they send answer next tick.
            let answering = in_flight;
            in_flight = 0;
            for _ in 0..answering {
                let songs = (0..per_call)
                    .map(|_| {
                        next_id += 1;
                        song(&next_id.to_string())
                    })
                    .collect();
                gathering.answered(Ok(songs));
                let more = gathering.calls_to_send();
                gathering.sent(more);
                in_flight += more;
                if gathering.is_over() {
                    break;
                }
            }
            assert!(ticks <= MOST_CALLS, "the gathering ends");
        }
        (ticks, gathering.calls_sent(), peak)
    }

    #[test]
    fn it_waits_for_no_more_calls_in_a_row_than_rounds_of_four_did() {
        for count in 1..=60 {
            let (ticks, sent, peak) = simulate(count, SONGS_PER_CALL);
            // Before: rounds of four calls at once, each waited for, until enough came.
            let rounds = count.div_ceil(SONGS_PER_CALL).div_ceil(4);
            assert!(
                ticks <= rounds,
                "{count} songs: {ticks} ticks, rounds of four took {rounds}"
            );
            assert!(
                peak <= MOST_IN_FLIGHT && sent <= MOST_CALLS,
                "{count}: {peak} {sent}"
            );
            // No call more than the songs need.
            if count <= 2 * MOST_CALLS {
                assert_eq!(sent, count.div_ceil(SONGS_PER_CALL), "{count} songs");
            }
        }
        // Calls of one song each still need no more than six at once.
        assert!(simulate(25, 1).2 <= MOST_IN_FLIGHT);
    }

    #[test]
    fn a_station_repeating_itself_ends_before_the_budget() {
        let mut gathering = Gathering::new(20);
        let mut sent = 0;
        loop {
            let more = gathering.calls_to_send();
            gathering.sent(more);
            sent += more;
            if gathering.is_over() {
                break;
            }
            gathering.answered(Ok(vec![song("1"), song("2")]));
        }
        assert!(sent < MOST_CALLS, "{sent} calls");
        let songs = gathering.into_songs().unwrap();
        assert_eq!(songs.len(), 2);
    }

    #[test]
    fn it_fails_only_when_no_song_came() {
        let mut failing = Gathering::new(10);
        failing.sent(3);
        failing.answered(Err(Error::NotSignedIn));
        failing.answered(Err(Error::Network("down".into())));
        failing.answered(Ok(Vec::new()));
        assert_eq!(failing.calls_to_send(), 0);
        assert!(matches!(failing.into_songs(), Err(Error::NotSignedIn)));

        let mut empty = Gathering::new(10);
        empty.sent(1);
        empty.answered(Ok(Vec::new()));
        assert!(
            empty
                .into_songs()
                .unwrap_err()
                .to_string()
                .contains("no songs")
        );

        let mut some = Gathering::new(10);
        some.sent(2);
        some.answered(Ok(vec![song("1")]));
        some.answered(Err(Error::NotSignedIn));
        assert_eq!(some.into_songs().unwrap().len(), 1);
    }

    #[test]
    fn songs_are_kept_once_in_order_and_no_more_than_asked() {
        let mut gathering = Gathering::new(3);
        gathering.sent(2);
        gathering.answered(Ok(vec![song("b"), song("a"), song("b")]));
        gathering.answered(Ok(vec![song("a"), song("c"), song("d")]));
        assert!(gathering.is_over());
        let ids: Vec<String> = gathering
            .into_songs()
            .unwrap()
            .into_iter()
            .map(|t| t.id.id)
            .collect();
        assert_eq!(ids, ["b", "a", "c"]);
    }

    #[test]
    fn nothing_is_sent_for_no_song() {
        let gathering = Gathering::new(0);
        assert_eq!(gathering.calls_to_send(), 0);
        assert!(gathering.is_over());
        assert!(gathering.into_songs().unwrap().is_empty());
    }

    #[test]
    fn a_station_id_is_one_path_segment() {
        for id in [
            "ra.cp-pl.u-AbC9",
            "ra.1234",
            "ra.u-a_b.c",
            "x",
            "..a",
            "a..",
            "...",
            "_",
        ] {
            assert_eq!(station_id(id).unwrap(), id);
        }
        for id in [
            "",
            ".",
            "..",
            "ra.1/../me",
            "ra 1",
            "ra.1?x=y",
            "ra.1#f",
            "ra%2E1",
            "ra.é",
            "ra\\1",
            "ra.1\n",
            "ra:1",
            "ra.1;x",
            "ra+1",
            "ra~1",
        ] {
            let error = station_id(id).unwrap_err();
            assert!(
                matches!(&error, Error::Protocol(m) if m.contains("not a station id")),
                "{id:?}: {error}"
            );
        }
    }

    #[test]
    fn an_answer_reads_as_songs_or_names_the_path() {
        let body = r#"{"data":[{"id":"101","type":"songs","attributes":{"name":"One"}},
            {"id":5},{"id":"102","type":"songs"}]}"#;
        let ids: Vec<String> = songs_in("/p", body)
            .unwrap()
            .into_iter()
            .map(|t| t.id.id)
            .collect();
        assert_eq!(ids, ["101", "102"]);
        for bad in ["", "[]", "{}", r#"{"data":{}}"#, r#"{"data":null}"#] {
            let error = songs_in("/v1/me/stations/next-tracks/ra.1", bad).unwrap_err();
            assert!(
                matches!(&error, Error::Protocol(m) if m.starts_with("/v1/me/stations/next-tracks/ra.1")),
                "{bad}: {error}"
            );
        }
    }
}
