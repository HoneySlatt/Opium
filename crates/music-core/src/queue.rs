use rand::seq::SliceRandom;

use crate::model::Track;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}

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
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the whole queue, starting playback at `start` (an index into `tracks`).
    pub fn replace(&mut self, tracks: Vec<Track>, start: usize) {
        self.order = (0..tracks.len()).collect();
        self.pos = start.min(tracks.len().saturating_sub(1));
        self.tracks = tracks;
        if self.shuffle {
            self.shuffle_around_current();
        }
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
        let at = (self.pos + 1).min(self.order.len());
        self.order.insert(at, self.tracks.len());
        self.tracks.push(track);
    }

    pub fn add_last(&mut self, track: Track) {
        self.order.push(self.tracks.len());
        self.tracks.push(track);
    }

    /// How many tracks follow the current one.
    pub fn upcoming_len(&self) -> usize {
        self.order.len().saturating_sub(self.pos + 1)
    }

    /// Make the `k`-th upcoming track current; the ones before it are skipped.
    pub fn jump(&mut self, k: usize) -> Option<&Track> {
        if k < self.upcoming_len() {
            self.pos += 1 + k;
        }
        self.current()
    }

    /// Take the `k`-th upcoming track out of the queue.
    pub fn remove_upcoming(&mut self, k: usize) {
        if k < self.upcoming_len() {
            let index = self.order.remove(self.pos + 1 + k);
            self.forget(index);
        }
    }

    /// Take every upcoming track out, keeping what has played.
    pub fn clear_upcoming(&mut self) {
        while self.upcoming_len() > 0 {
            self.remove_upcoming(self.upcoming_len() - 1);
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

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    /// Shuffling keeps the current track playing and shuffles everything else after it.
    pub fn set_shuffle(&mut self, on: bool) {
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
    fn adding_to_an_empty_queue_makes_it_current() {
        let mut q = Queue::new();
        assert!(q.current().is_none());
        q.add_next(track(7));
        assert_eq!(number(q.current()), Some(7));
    }
}
