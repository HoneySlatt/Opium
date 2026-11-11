//! When each of the newest library songs was first seen here. Apple says
//! when an album or a playlist joined the library, not a song: an album a
//! song was just added to comes first in its Recently Added, but with the
//! album's own date, which the other services' items can't be set beside.
//! The first time a song is seen among the newest stands for when it was
//! added: at once for a song added in the app, which reads the list again
//! then; for one added elsewhere, the next time the list is read. With the
//! user's other devices, the earliest any of them saw it.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use opium_core::files::{Mode, write_whole};

/// The times the user's other devices saw songs first, kept in common.
pub trait SeenElsewhere: Send + Sync {
    /// When each of `ids` was first seen on any device, of those known, in
    /// seconds since 1970.
    fn known(&self, ids: &[String]) -> HashMap<String, u64>;
    /// Songs first seen here at those times.
    fn seen_here(&self, times: Vec<(String, u64)>);
}

/// The songs' times, by id, and where they are kept.
pub(crate) struct Seen {
    path: PathBuf,
    /// RFC 3339; empty for a song already there the first time, of which
    /// nothing is known.
    times: HashMap<String, String>,
    /// Nothing was kept yet: what is there now is no newer than anything.
    first: bool,
    elsewhere: Option<Arc<dyn SeenElsewhere>>,
}

impl Seen {
    pub fn open(path: PathBuf, elsewhere: Option<Arc<dyn SeenElsewhere>>) -> Self {
        let times: Option<HashMap<String, String>> = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        Self {
            path,
            first: times.is_none(),
            times: times.unwrap_or_default(),
            elsewhere,
        }
    }

    /// When each of `ids` (the newest songs) was first seen: now for one
    /// not seen before. Only theirs are kept.
    pub fn when(&mut self, ids: &[String]) -> Vec<Option<String>> {
        // No songs at all means the newest couldn't be read, not that there
        // are none: what was seen stays, so that a request that failed
        // doesn't have every song stamped as seen now next time.
        if ids.is_empty() {
            return Vec::new();
        }
        let now = opium_core::recent::rfc3339(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64),
        );
        let new = if self.first { String::new() } else { now };
        let mut times = HashMap::with_capacity(ids.len());
        let here: Vec<Option<u64>> = ids
            .iter()
            .map(|id| {
                let at = self.times.get(id).cloned().unwrap_or_else(|| new.clone());
                times.insert(id.clone(), at.clone());
                opium_core::recent::seconds(&at).and_then(|s| u64::try_from(s).ok())
            })
            .collect();
        if times != self.times || self.first {
            self.times = times;
            self.first = false;
            self.save();
        }
        let Some(elsewhere) = &self.elsewhere else {
            return here.into_iter().map(|at| at.map(to_text)).collect();
        };
        // Of here and elsewhere, the earliest; what is earlier here, told.
        let known = elsewhere.known(ids);
        let mut earlier = Vec::new();
        let out = ids
            .iter()
            .zip(here)
            .map(|(id, here)| {
                let there = known.get(id).copied();
                if let Some(at) = here
                    && there.is_none_or(|there| at < there)
                {
                    earlier.push((id.clone(), at));
                }
                here.into_iter().chain(there).min().map(to_text)
            })
            .collect();
        if !earlier.is_empty() {
            elsewhere.seen_here(earlier);
        }
        out
    }

    /// Written whole or not at all ([`write_whole`]).
    fn save(&self) {
        let Ok(bytes) = serde_json::to_vec(&self.times) else {
            return;
        };
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = write_whole(&self.path, &bytes, Mode::Default);
    }
}

fn to_text(secs: u64) -> String {
    opium_core::recent::rfc3339(secs as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn songs_there_at_first_have_no_time_and_those_after_have_theirs() {
        let path = std::env::temp_dir().join(format!("seen-test-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let ids = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let mut seen = Seen::open(path.clone(), None);
        assert_eq!(seen.when(&ids(&["a", "b"])), [None, None]);
        let got = seen.when(&ids(&["c", "a", "b"]));
        assert!(got[0].is_some() && got[1].is_none() && got[2].is_none());
        // Kept: read again, the same.
        let mut again = Seen::open(path.clone(), None);
        assert_eq!(again.when(&ids(&["c", "a"])), [got[0].clone(), None]);
        let _ = std::fs::remove_file(&path);
    }
}
