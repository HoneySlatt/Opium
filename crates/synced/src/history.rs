//! The songs played lately, on any of the devices: each song once, at its
//! last play.

use std::collections::BTreeMap;

use music_core::model::Track;
use serde::{Deserialize, Serialize};

use crate::Stamp;

/// How many songs the history keeps, the latest played.
pub const KEPT: usize = 100;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct History {
    /// Each song's last play, by the song's id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    played: BTreeMap<String, Play>,
    /// The history was cleared then: what was played until then is gone,
    /// on every device, however late they hear of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cleared: Option<Stamp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Play {
    at: Stamp,
    track: Track,
}

impl History {
    /// `track` played `at`, as a change to send.
    pub(crate) fn play(track: Track, at: Stamp) -> Self {
        Self {
            played: BTreeMap::from([(track.id.to_string(), Play { at, track })]),
            cleared: None,
        }
    }

    /// Cleared `at`, as a change to send.
    pub(crate) fn clear(at: Stamp) -> Self {
        Self {
            played: BTreeMap::new(),
            cleared: Some(at),
        }
    }

    /// The songs, the last played first.
    pub fn tracks(&self) -> Vec<Track> {
        let mut plays: Vec<&Play> = self.played.values().collect();
        plays.sort_by(|a, b| b.at.cmp(&a.at));
        plays.into_iter().map(|p| p.track.clone()).collect()
    }

    /// The latest change it holds.
    pub(crate) fn newest(&self) -> Option<&Stamp> {
        self.played
            .values()
            .map(|p| &p.at)
            .chain(&self.cleared)
            .max()
    }

    /// Take `other`'s plays in: whether anything changed.
    pub(crate) fn merge(&mut self, other: History) -> bool {
        let before = self.clone();
        if other.cleared > self.cleared {
            self.cleared = other.cleared;
        }
        for (id, play) in other.played {
            match self.played.get(&id) {
                Some(mine) if mine.at >= play.at => {}
                _ => {
                    self.played.insert(id, play);
                }
            }
        }
        self.settle();
        *self != before
    }

    /// Without what was cleared, and the latest [`KEPT`] only: the same
    /// whichever plays came in first.
    fn settle(&mut self) {
        if let Some(cleared) = &self.cleared {
            self.played.retain(|_, p| p.at > *cleared);
        }
        if self.played.len() > KEPT {
            let mut stamps: Vec<Stamp> = self.played.values().map(|p| p.at.clone()).collect();
            stamps.sort_unstable_by(|a, b| b.cmp(a));
            let oldest_kept = stamps[KEPT - 1].clone();
            self.played.retain(|_, p| p.at >= oldest_kept);
        }
    }
}
