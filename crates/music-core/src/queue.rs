use rand::seq::SliceRandom;

use crate::model::{ItemId, Track};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}

/// A queue to play from here on, where another device left it, or where
/// it was when the music last stopped.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Handoff {
    /// The queue's songs in their own order, the play order over them, and
    /// where in that the song playing is.
    pub tracks: Vec<Track>,
    pub order: Vec<u32>,
    pub at: u32,
    pub shuffle: bool,
    pub repeat: Repeat,
    pub position_ms: u64,
    pub playing: bool,
    /// The station of songs it went on with, if any.
    pub station: Option<ItemId>,
}

/// How many songs the queue holds at most, as the history: a list longer
/// than that plays the part around the song picked (a shuffle, that many
/// at random), and what is added past it pushes out what played first.
pub const LIMIT: usize = 100;

/// The play queue. Tracks keep their insertion order; `order` is the play order
/// over them, so turning shuffle off restores the original sequence.
#[derive(Debug, Default)]
pub struct Queue {
    tracks: Vec<Track>,
    order: Vec<usize>,
    /// Index into `order` of the current track.
    pos: usize,
    shuffle: bool,
    repeat: Repeat,
    /// Bumped by every change, so that a copy shown elsewhere is sent again
    /// only when it is out of date.
    revision: u64,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the whole queue, starting playback at `start` (an index into `tracks`).
    pub fn replace(&mut self, tracks: Vec<Track>, start: usize) {
        self.revision += 1;
        let (tracks, start) = self.within_limit(tracks, start);
        self.order = (0..tracks.len()).collect();
        self.pos = start.min(tracks.len().saturating_sub(1));
        self.tracks = tracks;
        if self.shuffle {
            self.shuffle_around_current();
        }
    }

    /// At most [`LIMIT`] of `tracks`, `start` among them, in their order:
    /// those from `start` on (and before it, to make up the number), or
    /// with shuffle on, others picked at random.
    fn within_limit(&self, mut tracks: Vec<Track>, start: usize) -> (Vec<Track>, usize) {
        if tracks.len() <= LIMIT {
            return (tracks, start);
        }
        let start = start.min(tracks.len() - 1);
        if self.shuffle {
            let mut others: Vec<usize> = (0..tracks.len()).filter(|&i| i != start).collect();
            others.shuffle(&mut rand::rng());
            others.truncate(LIMIT - 1);
            others.push(start);
            others.sort_unstable();
            let at = others.binary_search(&start).unwrap_or(0);
            let mut keep = vec![false; tracks.len()];
            for &i in &others {
                keep[i] = true;
            }
            let mut kept = keep.iter();
            tracks.retain(|_| *kept.next().unwrap_or(&false));
            (tracks, at)
        } else {
            let from = start.min(tracks.len() - LIMIT);
            tracks.truncate(from + LIMIT);
            tracks.drain(..from);
            (tracks, start - from)
        }
    }

    /// Back within [`LIMIT`]: what played goes first, then the far end.
    fn trim(&mut self) {
        let excess = self.order.len().saturating_sub(LIMIT);
        if excess == 0 {
            return;
        }
        let played = excess.min(self.pos);
        self.pos -= played;
        self.order.drain(..played);
        self.order.truncate(self.order.len() - (excess - played));
        self.keep_ordered();
    }

    pub fn current(&self) -> Option<&Track> {
        self.order.get(self.pos).map(|&i| &self.tracks[i])
    }

    /// Tracks after the current one, in play order.
    pub fn upcoming(&self) -> impl Iterator<Item = &Track> {
        self.order
            .iter()
            .skip(self.pos + 1)
            .map(|&i| &self.tracks[i])
    }

    /// The user pressed next. Wraps around unless repeat is off.
    pub fn skip_next(&mut self) -> Option<&Track> {
        self.revision += 1;
        if self.pos + 1 < self.order.len() {
            self.pos += 1;
        } else if self.repeat != Repeat::Off && !self.order.is_empty() {
            self.pos = 0;
        } else {
            return None;
        }
        self.current()
    }

    /// The user pressed previous. Stays on the first track rather than wrapping.
    pub fn skip_previous(&mut self) -> Option<&Track> {
        self.revision += 1;
        self.pos = self.pos.saturating_sub(1);
        self.current()
    }

    /// The current track played to the end: what plays now.
    pub fn track_finished(&mut self) -> Option<&Track> {
        if self.repeat == Repeat::One {
            return self.current();
        }
        self.skip_next()
    }

    /// What [`track_finished`](Self::track_finished) would move to, without
    /// moving: the track to have ready before the current one ends.
    pub fn peek_finished(&self) -> Option<&Track> {
        let next = match self.repeat {
            Repeat::One => self.pos,
            _ if self.pos + 1 < self.order.len() => self.pos + 1,
            Repeat::All => 0,
            Repeat::Off => return None,
        };
        self.order.get(next).map(|&i| &self.tracks[i])
    }

    /// Play `track` right after the current one.
    pub fn add_next(&mut self, track: Track) {
        self.revision += 1;
        // Next in the tracks' own order too, so turning shuffle off keeps it
        // there rather than at the very end.
        let index = self
            .order
            .get(self.pos)
            .map_or(self.tracks.len(), |&current| current + 1);
        for i in &mut self.order {
            if *i >= index {
                *i += 1;
            }
        }
        self.tracks.insert(index, track);
        let at = (self.pos + 1).min(self.order.len());
        self.order.insert(at, index);
        self.trim();
    }

    pub fn add_last(&mut self, track: Track) {
        self.revision += 1;
        self.order.push(self.tracks.len());
        self.tracks.push(track);
        self.trim();
    }

    /// How many tracks follow the current one.
    pub fn upcoming_len(&self) -> usize {
        self.order.len().saturating_sub(self.pos + 1)
    }

    /// Make the `k`-th upcoming track current; the ones before it are skipped.
    pub fn jump(&mut self, k: usize) -> Option<&Track> {
        self.revision += 1;
        if k < self.upcoming_len() {
            self.pos += 1 + k;
        }
        self.current()
    }

    /// Take the `k`-th upcoming track out of the queue.
    pub fn remove_upcoming(&mut self, k: usize) {
        self.revision += 1;
        if k < self.upcoming_len() {
            let index = self.order.remove(self.pos + 1 + k);
            self.forget(index);
        }
    }

    /// Take every upcoming track out, keeping what has played.
    pub fn clear_upcoming(&mut self) {
        self.revision += 1;
        self.order.truncate(self.pos + 1);
        self.keep_ordered();
    }

    /// Drop the tracks `order` no longer refers to.
    fn keep_ordered(&mut self) {
        let mut kept = vec![false; self.tracks.len()];
        for &i in &self.order {
            kept[i] = true;
        }
        // Each track's index once those not kept are gone.
        let new_index: Vec<usize> = kept
            .iter()
            .scan(0, |n, &k| {
                *n += usize::from(k);
                Some(*n - usize::from(k))
            })
            .collect();
        let mut keep = kept.iter();
        self.tracks.retain(|_| *keep.next().unwrap_or(&false));
        for i in &mut self.order {
            *i = new_index[*i];
        }
    }

    /// Drop `tracks[index]`, which `order` no longer refers to, and shift the
    /// indices after it, so turning shuffle off can't bring it back.
    fn forget(&mut self, index: usize) {
        self.tracks.remove(index);
        for i in &mut self.order {
            if *i > index {
                *i -= 1;
            }
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The whole queue: its tracks in their order, the play order over them,
    /// and where in it the current track is.
    pub fn snapshot(&self) -> (Vec<Track>, Vec<usize>, usize) {
        (self.tracks.clone(), self.order.clone(), self.pos)
    }

    /// Take over a queue as [`snapshot`](Self::snapshot) gave it. An order
    /// that doesn't fit the tracks plays them in their own.
    pub fn restore(&mut self, tracks: Vec<Track>, order: Vec<usize>, pos: usize, shuffle: bool) {
        self.revision += 1;
        let mut seen = vec![false; tracks.len()];
        let fits = order.len() <= tracks.len()
            && order
                .iter()
                .all(|&i| i < tracks.len() && !std::mem::replace(&mut seen[i], true));
        self.order = if fits {
            order
        } else {
            (0..tracks.len()).collect()
        };
        self.pos = pos.min(self.order.len().saturating_sub(1));
        self.tracks = tracks;
        self.shuffle = shuffle;
        // A queue kept, or handed over, from before there was a limit.
        if self.order.len() < self.tracks.len() {
            self.keep_ordered();
        }
        self.trim();
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    /// Shuffling keeps the current track playing and shuffles everything else after it.
    pub fn set_shuffle(&mut self, on: bool) {
        self.revision += 1;
        if on == self.shuffle {
            return;
        }
        self.shuffle = on;
        if on {
            self.shuffle_around_current();
        } else if let Some(&current) = self.order.get(self.pos) {
            self.order = (0..self.tracks.len()).collect();
            self.pos = current;
        }
    }

    pub fn repeat(&self) -> Repeat {
        self.repeat
    }

    pub fn set_repeat(&mut self, repeat: Repeat) {
        self.revision += 1;
        self.repeat = repeat;
    }

    fn shuffle_around_current(&mut self) {
        let Some(&current) = self.order.get(self.pos) else {
            return;
        };
        let mut rest: Vec<usize> = (0..self.tracks.len()).filter(|&i| i != current).collect();
        rest.shuffle(&mut rand::rng());
        self.order = std::iter::once(current).chain(rest).collect();
        self.pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ItemId, ProviderKind};

    fn track(n: u32) -> Track {
        Track {
            id: ItemId::new(ProviderKind::Navidrome, n.to_string()),
            title: format!("track {n}"),
            artists: vec![],
            album: None,
            album_id: None,
            duration: None,
            track_number: Some(n),
            disc_number: None,
            artwork: None,
            explicit: false,
            quality: None,
            in_library: false,
            library_id: None,
        }
    }

    fn queue_of(n: u32, start: usize) -> Queue {
        let mut q = Queue::new();
        q.replace((0..n).map(track).collect(), start);
        q
    }

    fn number(t: Option<&Track>) -> Option<u32> {
        t.and_then(|t| t.track_number)
    }

    #[test]
    fn plays_in_order_and_stops_at_the_end() {
        let mut q = queue_of(3, 0);
        assert_eq!(number(q.current()), Some(0));
        assert_eq!(number(q.track_finished()), Some(1));
        assert_eq!(number(q.track_finished()), Some(2));
        assert_eq!(number(q.track_finished()), None);
        assert_eq!(number(q.current()), Some(2));
    }

    #[test]
    fn repeat_all_wraps_and_repeat_one_only_holds_on_track_end() {
        let mut q = queue_of(2, 1);
        q.set_repeat(Repeat::All);
        assert_eq!(number(q.track_finished()), Some(0));

        q.set_repeat(Repeat::One);
        assert_eq!(number(q.track_finished()), Some(0));
        assert_eq!(number(q.skip_next()), Some(1));
    }

    #[test]
    fn peeking_agrees_with_what_track_finished_does() {
        for repeat in [Repeat::Off, Repeat::All, Repeat::One] {
            for start in 0..3 {
                let mut q = queue_of(3, start);
                q.set_repeat(repeat);
                let peeked = number(q.peek_finished());
                assert_eq!(
                    peeked,
                    number(q.track_finished()),
                    "{repeat:?} from {start}"
                );
            }
        }
    }

    #[test]
    fn jumping_skips_to_an_upcoming_track() {
        let mut q = queue_of(5, 1);
        assert_eq!(number(q.jump(2)), Some(4));
        assert_eq!(q.upcoming_len(), 0);
        assert_eq!(number(q.jump(5)), Some(4), "out of range: stays");
    }

    #[test]
    fn removed_tracks_stay_gone_even_after_unshuffling() {
        let mut q = queue_of(5, 0);
        q.remove_upcoming(1);
        let upcoming: Vec<u32> = q.upcoming().filter_map(|t| t.track_number).collect();
        assert_eq!(upcoming, vec![1, 3, 4]);
        q.set_shuffle(true);
        q.set_shuffle(false);
        let upcoming: Vec<u32> = q.upcoming().filter_map(|t| t.track_number).collect();
        assert_eq!(upcoming, vec![1, 3, 4]);
        assert_eq!(number(q.current()), Some(0));
    }

    #[test]
    fn clearing_keeps_the_current_track() {
        let mut q = queue_of(4, 1);
        q.clear_upcoming();
        assert_eq!(q.upcoming_len(), 0);
        assert_eq!(number(q.current()), Some(1));
        assert_eq!(number(q.skip_previous()), Some(0));
    }

    #[test]
    fn previous_stops_at_the_first_track() {
        let mut q = queue_of(2, 1);
        assert_eq!(number(q.skip_previous()), Some(0));
        assert_eq!(number(q.skip_previous()), Some(0));
    }

    #[test]
    fn shuffle_keeps_the_current_track_and_unshuffle_restores_order() {
        let mut q = queue_of(50, 10);
        q.set_shuffle(true);
        assert_eq!(number(q.current()), Some(10));
        let mut upcoming: Vec<u32> = q.upcoming().filter_map(|t| t.track_number).collect();
        upcoming.sort();
        assert_eq!(upcoming, (0..50).filter(|&n| n != 10).collect::<Vec<_>>());

        q.skip_next();
        let playing = number(q.current());
        q.set_shuffle(false);
        assert_eq!(number(q.current()), playing);
        let after: Vec<u32> = q.upcoming().filter_map(|t| t.track_number).collect();
        assert_eq!(after, (playing.unwrap() + 1..50).collect::<Vec<_>>());
    }

    #[test]
    fn add_next_plays_after_the_current_track() {
        let mut q = queue_of(3, 0);
        q.add_next(track(99));
        q.add_last(track(100));
        let upcoming: Vec<u32> = q.upcoming().filter_map(|t| t.track_number).collect();
        assert_eq!(upcoming, vec![99, 1, 2, 100]);
    }

    #[test]
    fn a_song_added_next_stays_next_when_shuffle_goes_off() {
        let mut q = queue_of(20, 4);
        q.set_shuffle(true);
        q.add_next(track(99));
        assert_eq!(number(q.upcoming().next()), Some(99));
        let playing = number(q.current()).unwrap();
        q.set_shuffle(false);
        assert_eq!(number(q.current()), Some(playing));
        assert_eq!(number(q.upcoming().next()), Some(99));
        assert_eq!(q.upcoming_len() + 1 + playing as usize, 21);
    }

    #[test]
    fn a_restored_snapshot_plays_on_and_unshuffles_the_same() {
        let mut q = queue_of(6, 2);
        q.set_shuffle(true);
        q.skip_next();
        let (tracks, order, pos) = q.snapshot();
        let mut other = Queue::new();
        other.restore(tracks, order, pos, true);
        assert_eq!(number(other.current()), number(q.current()));
        assert!(other.upcoming().eq(q.upcoming()));
        q.set_shuffle(false);
        other.set_shuffle(false);
        assert!(other.upcoming().eq(q.upcoming()));
    }

    #[test]
    fn an_order_that_does_not_fit_is_dropped() {
        let mut q = Queue::new();
        q.restore((0..3).map(track).collect(), vec![0, 0, 7], 1, false);
        assert_eq!(number(q.current()), Some(1));
        assert_eq!(q.upcoming_len(), 1);
    }

    #[test]
    fn a_long_list_keeps_the_hundred_from_the_song_picked() {
        let mut q = queue_of(300, 20);
        assert_eq!(number(q.current()), Some(20));
        assert_eq!(q.upcoming_len(), LIMIT - 1);
        assert_eq!(number(q.upcoming().last()), Some(119));
        // Near the end, the songs before it make up the number.
        let q = queue_of(300, 290);
        assert_eq!(number(q.current()), Some(290));
        assert_eq!(q.snapshot().0.len(), LIMIT);
        assert_eq!(q.upcoming_len(), 9);
    }

    #[test]
    fn a_long_list_shuffled_plays_a_hundred_of_it() {
        let mut q = Queue::new();
        q.set_shuffle(true);
        q.replace((0..1000).map(track).collect(), 500);
        assert_eq!(number(q.current()), Some(500));
        assert_eq!(q.upcoming_len(), LIMIT - 1);
        q.set_shuffle(false);
        assert_eq!(number(q.current()), Some(500));
        let all: Vec<u32> = q
            .snapshot()
            .0
            .iter()
            .filter_map(|t| t.track_number)
            .collect();
        assert!(all.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn what_is_added_past_the_limit_pushes_out_what_played() {
        let mut q = queue_of(100, 0);
        for _ in 0..10 {
            q.skip_next();
        }
        q.add_last(track(500));
        assert_eq!(q.snapshot().0.len(), LIMIT);
        assert_eq!(number(q.current()), Some(10));
        assert_eq!(number(q.upcoming().last()), Some(500));
        // Nothing played left to push out: the far end goes.
        let mut q = queue_of(100, 0);
        q.add_next(track(7));
        assert_eq!(q.snapshot().0.len(), LIMIT);
        assert_eq!(number(q.upcoming().next()), Some(7));
        assert_eq!(number(q.upcoming().last()), Some(98));
    }

    #[test]
    fn a_long_queue_restored_is_cut_down() {
        let mut q = Queue::new();
        q.restore(
            (0..300).map(track).collect(),
            (0..300).collect(),
            150,
            false,
        );
        assert_eq!(number(q.current()), Some(150));
        assert_eq!(q.snapshot().0.len(), LIMIT);
        assert_eq!(q.upcoming_len(), 99);
    }

    #[test]
    fn adding_to_an_empty_queue_makes_it_current() {
        let mut q = Queue::new();
        assert!(q.current().is_none());
        q.add_next(track(7));
        assert_eq!(number(q.current()), Some(7));
    }
}
