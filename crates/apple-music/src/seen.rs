//! When each of the newest library songs was first seen here. Apple says
//! when an album or a playlist joined the library, not a song: an album a
//! song was just added to comes first in its Recently Added, but with the
//! album's own date, which the other services' items can't be set beside.
//! The first time a song is seen among the newest stands for when it was
//! added: at once for a song added in the app, which reads the list again
//! then; for one added elsewhere, the next time the list is read.

use std::collections::HashMap;
use std::path::PathBuf;

/// The songs' times, by id, and where they are kept.
pub(crate) struct Seen {
    path: PathBuf,
    /// RFC 3339; empty for a song already there the first time, of which
    /// nothing is known.
    times: HashMap<String, String>,
    /// Nothing was kept yet: what is there now is no newer than anything.
    first: bool,
}

impl Seen {
    pub fn open(path: PathBuf) -> Self {
        let times: Option<HashMap<String, String>> = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        Self {
            path,
            first: times.is_none(),
            times: times.unwrap_or_default(),
        }
    }

    /// When each of `ids` (the newest songs) was first seen: now for one
    /// not seen before. Only theirs are kept.
    pub fn when(&mut self, ids: &[String]) -> Vec<Option<String>> {
        let now = music_core::recent::rfc3339(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64),
        );
        let new = if self.first { String::new() } else { now };
        let mut times = HashMap::with_capacity(ids.len());
        let out = ids
            .iter()
            .map(|id| {
                let at = self.times.get(id).cloned().unwrap_or_else(|| new.clone());
                times.insert(id.clone(), at.clone());
                Some(at).filter(|a| !a.is_empty())
            })
            .collect();
        if times != self.times || self.first {
            self.times = times;
            self.first = false;
            self.save();
        }
        out
    }

    fn save(&self) {
        let Ok(bytes) = serde_json::to_vec(&self.times) else {
            return;
        };
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let partial = self.path.with_extension("part");
        if std::fs::write(&partial, bytes).is_ok() {
            let _ = std::fs::rename(&partial, &self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn songs_there_at_first_have_no_time_and_those_after_have_theirs() {
        let path = std::env::temp_dir().join(format!("seen-test-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let ids = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let mut seen = Seen::open(path.clone());
        assert_eq!(seen.when(&ids(&["a", "b"])), [None, None]);
        let got = seen.when(&ids(&["c", "a", "b"]));
        assert!(got[0].is_some() && got[1].is_none() && got[2].is_none());
        // Kept: read again, the same.
        let mut again = Seen::open(path.clone());
        assert_eq!(again.when(&ids(&["c", "a"])), [got[0].clone(), None]);
        let _ = std::fs::remove_file(&path);
    }
}
