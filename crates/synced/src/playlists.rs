//! The playlists made in Music itself, of songs from any service.
//!
//! A playlist's songs are each an entry of their own, so the same song can
//! be in it twice: added on two devices at once, both entries stay; taken
//! out, an entry stays out. Each entry has its own place, and a playlist its
//! name: of two changes to one, the latest wins. A playlist deleted stays
//! deleted, whatever another device did to it meanwhile.

use std::collections::BTreeMap;

use music_core::model::Track;
use serde::{Deserialize, Serialize};

use crate::{Latest, Stamp, latest, place};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Playlists {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    lists: BTreeMap<String, List>,
}

/// One playlist, or what changed in it: each part may be missing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct List {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<Latest<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deleted: Option<Stamp>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    songs: BTreeMap<String, Entry>,
}

/// A song in a playlist, or what changed about it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct Entry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    track: Option<Track>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    place: Option<Latest<String>>,
    /// Taken out: its song is no longer kept.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    removed: bool,
}

/// A playlist as shown.
#[derive(Debug, Clone, PartialEq)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub songs: Vec<PlaylistSong>,
}

/// A song in a playlist, with the entry that says which of its songs it is.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaylistSong {
    pub entry: String,
    pub track: Track,
}

impl Entry {
    fn merge(&mut self, other: Entry) {
        self.removed |= other.removed;
        if self.removed {
            *self = Entry {
                removed: true,
                ..Entry::default()
            };
            return;
        }
        if self.track.is_none() {
            self.track = other.track;
        }
        latest(&mut self.place, other.place);
    }

    fn shown(&self) -> Option<(&str, &Track)> {
        Some((self.place.as_ref()?.value.as_str(), self.track.as_ref()?))
    }
}

impl List {
    fn merge(&mut self, other: List) {
        if other.deleted > self.deleted {
            self.deleted = other.deleted;
        }
        if self.deleted.is_some() {
            self.name = None;
            self.songs.clear();
            return;
        }
        latest(&mut self.name, other.name);
        for (id, entry) in other.songs {
            self.songs.entry(id).or_default().merge(entry);
        }
    }

    /// Its songs in order, each with its entry and place.
    fn shown(&self) -> Vec<(&str, &str, &Track)> {
        let mut songs: Vec<(&str, &str, &Track)> = self
            .songs
            .iter()
            .filter_map(|(id, e)| e.shown().map(|(place, track)| (place, id.as_str(), track)))
            .collect();
        songs.sort_unstable_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
        songs
    }
}

impl Playlists {
    /// The playlists, by name.
    pub fn all(&self) -> Vec<Playlist> {
        let mut all: Vec<Playlist> = self.lists.keys().filter_map(|id| self.get(id)).collect();
        all.sort_by_cached_key(|p| (p.name.to_lowercase(), p.id.clone()));
        all
    }

    pub fn get(&self, id: &str) -> Option<Playlist> {
        let list = self.lists.get(id)?;
        let name = list.name.as_ref()?.value.clone();
        if list.deleted.is_some() {
            return None;
        }
        Some(Playlist {
            id: id.to_owned(),
            name,
            songs: list
                .shown()
                .into_iter()
                .map(|(_, entry, track)| PlaylistSong {
                    entry: entry.to_owned(),
                    track: track.clone(),
                })
                .collect(),
        })
    }

    /// The latest change it holds.
    pub(crate) fn newest(&self) -> Option<&Stamp> {
        self.lists
            .values()
            .flat_map(|l| {
                let songs = l
                    .songs
                    .values()
                    .filter_map(|e| e.place.as_ref().map(|p| &p.at));
                l.name
                    .as_ref()
                    .map(|n| &n.at)
                    .into_iter()
                    .chain(&l.deleted)
                    .chain(songs)
            })
            .max()
    }

    /// Take `other`'s changes in: whether anything changed.
    pub(crate) fn merge(&mut self, other: Playlists) -> bool {
        let mut changed = false;
        for (id, list) in other.lists {
            let mine = self.lists.entry(id).or_default();
            let before = mine.clone();
            mine.merge(list);
            changed |= *mine != before;
        }
        changed
    }

    fn change(id: &str, list: List) -> Self {
        Self {
            lists: BTreeMap::from([(id.to_owned(), list)]),
        }
    }

    /// A playlist `id` named `name`, of `tracks`, made `at`.
    pub(crate) fn create(
        id: &str,
        name: String,
        tracks: Vec<Track>,
        entries: impl FnMut() -> String,
        at: Stamp,
    ) -> Self {
        Self::change(
            id,
            List {
                name: Some(Latest {
                    at: at.clone(),
                    value: name,
                }),
                deleted: None,
                songs: Self::added(None, tracks, entries, at),
            },
        )
    }

    pub(crate) fn rename(id: &str, name: String, at: Stamp) -> Self {
        Self::change(
            id,
            List {
                name: Some(Latest { at, value: name }),
                ..List::default()
            },
        )
    }

    pub(crate) fn delete(id: &str, at: Stamp) -> Self {
        Self::change(
            id,
            List {
                deleted: Some(at),
                ..List::default()
            },
        )
    }

    /// `tracks` added at the end of `id`; nothing if it isn't there.
    pub(crate) fn add(
        &self,
        id: &str,
        tracks: Vec<Track>,
        entries: impl FnMut() -> String,
        at: Stamp,
    ) -> Self {
        let Some(list) = self
            .lists
            .get(id)
            .filter(|l| l.deleted.is_none() && l.name.is_some())
        else {
            return Self::default();
        };
        let last = list.shown().last().map(|(place, _, _)| *place);
        Self::change(
            id,
            List {
                songs: Self::added(last, tracks, entries, at),
                ..List::default()
            },
        )
    }

    fn added(
        after: Option<&str>,
        tracks: Vec<Track>,
        mut entries: impl FnMut() -> String,
        at: Stamp,
    ) -> BTreeMap<String, Entry> {
        let mut last = after.map(str::to_owned);
        tracks
            .into_iter()
            .map(|track| {
                let place = place::between(last.as_deref(), None);
                last = Some(place.clone());
                let entry = Entry {
                    track: Some(track),
                    place: Some(Latest {
                        at: at.clone(),
                        value: place,
                    }),
                    removed: false,
                };
                (entries(), entry)
            })
            .collect()
    }

    pub(crate) fn remove(id: &str, entries: &[String]) -> Self {
        Self::change(
            id,
            List {
                songs: entries
                    .iter()
                    .map(|e| {
                        (
                            e.clone(),
                            Entry {
                                removed: true,
                                ..Entry::default()
                            },
                        )
                    })
                    .collect(),
                ..List::default()
            },
        )
    }

    /// `entry` moved to be the `to`-th song of `id`, counted without it.
    pub(crate) fn move_to(&self, id: &str, entry: &str, to: usize, at: Stamp) -> Self {
        let Some(list) = self.lists.get(id).filter(|l| l.deleted.is_none()) else {
            return Self::default();
        };
        if list.songs.get(entry).and_then(Entry::shown).is_none() {
            return Self::default();
        }
        let others: Vec<&str> = list
            .shown()
            .into_iter()
            .filter(|(_, e, _)| *e != entry)
            .map(|(place, _, _)| place)
            .collect();
        let to = to.min(others.len());
        let before = to.checked_sub(1).map(|i| others[i]);
        let place = place::between(before, others.get(to).copied());
        Self::change(
            id,
            List {
                songs: BTreeMap::from([(
                    entry.to_owned(),
                    Entry {
                        place: Some(Latest { at, value: place }),
                        ..Entry::default()
                    },
                )]),
                ..List::default()
            },
        )
    }
}
