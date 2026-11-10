//! The playlists made in Opium itself, of songs from any service.
//!
//! A playlist's songs are each an entry of their own, so the same song can
//! be in it twice: added on two devices at once, both entries stay; taken
//! out, an entry stays out. Each entry has its own place, and a playlist its
//! name: of two changes to one, the latest wins. A playlist deleted stays
//! deleted, whatever another device did to it meanwhile.

use std::collections::BTreeMap;

use opium_core::model::{ItemId, ProviderKind, Track};
use serde::{Deserialize, Serialize};

use crate::{Latest, PIECE_SLACK, Parts, Stamp, is_playlist_id, json_len, latest, place};

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
    description: Option<Latest<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deleted: Option<Stamp>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    songs: BTreeMap<String, Entry>,
    /// The cover chosen for it, a JPEG in base64; none given back to the
    /// first song's. The latest wins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cover: Option<Latest<Option<String>>>,
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
    pub description: String,
    pub songs: Vec<PlaylistSong>,
    /// Where its chosen cover is kept on this device (see `Kept`), as a
    /// `file://` address; none: its first song's.
    pub cover: Option<String>,
    /// That cover, a JPEG in base64.
    pub(crate) cover_data: Option<String>,
}

/// A song in a playlist, with the entry that says which of its songs it is.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaylistSong {
    pub entry: String,
    pub track: Track,
}

impl Playlist {
    /// As the apps show a service's playlist: its cover its first song's.
    pub fn model(&self) -> opium_core::model::Playlist {
        opium_core::model::Playlist {
            id: ItemId::new(ProviderKind::Opium, self.id.clone()),
            name: self.name.clone(),
            description: Some(self.description.clone()).filter(|d| !d.is_empty()),
            curator: Some("Opium".to_owned()),
            favourites: false,
            artwork: self
                .cover
                .as_ref()
                .map(|url| opium_core::model::Artwork {
                    url_template: url.clone(),
                    background: None,
                })
                .or_else(|| self.songs.iter().find_map(|s| s.track.artwork.clone())),
            track_count: Some(self.songs.len() as u32),
            catalog_id: None,
            in_favourites: None,
            editable: true,
            added: None,
        }
    }

    /// The entries of `track`, every time it is in.
    pub fn entries_of(&self, track: &ItemId) -> Vec<String> {
        self.songs
            .iter()
            .filter(|s| s.track.id == *track)
            .map(|s| s.entry.clone())
            .collect()
    }

    pub fn tracks(&self) -> Vec<Track> {
        self.songs.iter().map(|s| s.track.clone()).collect()
    }
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
            self.description = None;
            self.songs.clear();
            self.cover = None;
            return;
        }
        latest(&mut self.name, other.name);
        latest(&mut self.description, other.description);
        latest(&mut self.cover, other.cover);
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
            description: list
                .description
                .as_ref()
                .map(|d| d.value.clone())
                .unwrap_or_default(),
            songs: list
                .shown()
                .into_iter()
                .map(|(_, entry, track)| PlaylistSong {
                    entry: entry.to_owned(),
                    track: track.clone(),
                })
                .collect(),
            cover: None,
            cover_data: list.cover.as_ref().and_then(|c| c.value.clone()),
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
                    .chain(l.description.as_ref().map(|d| &d.at))
                    .chain(l.cover.as_ref().map(|c| &c.at))
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

    pub(crate) fn is_empty(&self) -> bool {
        self.lists.is_empty()
    }

    /// The playlists put in `parts`, as [`crate::Shared::into_parts`] says:
    /// each one's name, description, cover and deletion as one piece, then
    /// each of its songs as a piece of its own. A playlist holding nothing
    /// goes too, as one piece, so that the parts make all of it again.
    pub(crate) fn cut(self, parts: &mut Parts) {
        for (id, list) in self.lists {
            let List {
                name,
                description,
                deleted,
                songs,
                cover,
            } = list;
            let header = List {
                name,
                description,
                deleted,
                songs: BTreeMap::new(),
                cover,
            };
            let id_len = json_len(&id);
            if header != List::default() || songs.is_empty() {
                let len = id_len + json_len(&header) + PIECE_SLACK;
                parts.room(len).playlists.lists.insert(id.clone(), header);
            }
            for (entry, song) in songs {
                let len = id_len + json_len(&entry) + json_len(&song) + PIECE_SLACK;
                let part = parts.room(len);
                let list = part.playlists.lists.entry(id.clone()).or_default();
                list.songs.insert(entry, song);
            }
        }
    }

    /// Leaves out the playlists whose id this app doesn't make
    /// ([`is_playlist_id`]): how many.
    pub(crate) fn leave_out_bad_ids(&mut self) -> usize {
        let before = self.lists.len();
        self.lists.retain(|id, _| is_playlist_id(id));
        before - self.lists.len()
    }

    /// Each playlist's digest, by id: alike on two devices that have it alike.
    pub(crate) fn digests(&self) -> BTreeMap<String, u64> {
        self.lists
            .iter()
            .map(|(id, l)| (id.clone(), crate::digest(l)))
            .collect()
    }

    /// The playlists that aren't as `theirs` says another device has them.
    pub(crate) fn unlike(&self, theirs: &BTreeMap<String, u64>) -> Playlists {
        Playlists {
            lists: self
                .lists
                .iter()
                .filter(|(id, l)| theirs.get(*id) != Some(&crate::digest(l)))
                .map(|(id, l)| (id.clone(), l.clone()))
                .collect(),
        }
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
        description: String,
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
                description: Some(Latest {
                    at: at.clone(),
                    value: description,
                }),
                deleted: None,
                songs: Self::added(None, tracks, entries, at),
                cover: None,
            },
        )
    }

    /// `id` named `name`, described by `description`.
    pub(crate) fn edit(id: &str, name: String, description: String, at: Stamp) -> Self {
        Self::change(
            id,
            List {
                name: Some(Latest {
                    at: at.clone(),
                    value: name,
                }),
                description: Some(Latest {
                    at,
                    value: description,
                }),
                ..List::default()
            },
        )
    }

    /// `id`'s cover `cover`, a JPEG in base64; none: its first song's.
    pub(crate) fn cover(id: &str, cover: Option<String>, at: Stamp) -> Self {
        Self::change(
            id,
            List {
                cover: Some(Latest { at, value: cover }),
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
