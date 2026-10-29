//! The same song, album or artist from several services shows once: the best
//! copy stays, the others are left out of the lists. Songs and albums keep
//! the server's own file first, then Apple Music's AAC 256, then
//! SoundCloud's; artists keep Apple Music's, which has their photos, their
//! page gathering their releases from everywhere.
//!
//! Copies are told apart by their title and first artist, folded (case,
//! accents, `- Single` and `- EP`, `&` or `,` between artists), and songs by
//! their length too, within a few seconds: a remix or a long version stays
//! apart. Copies within one service all stay: they are its own.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use music_core::model::{Album, Artist, ProviderKind, Track};

use crate::app::fold;

/// How much songs' lengths may differ and still be one song.
const SAME_LENGTH: Duration = Duration::from_secs(3);

/// Which copy of a song or album wins: the lower.
fn rank(provider: ProviderKind) -> u8 {
    match provider {
        ProviderKind::Navidrome => 0,
        ProviderKind::AppleMusic => 1,
        ProviderKind::SoundCloud => 2,
    }
}

/// Which copy of an artist wins: Apple Music's, for its photos.
fn artist_rank(provider: ProviderKind) -> u8 {
    match provider {
        ProviderKind::AppleMusic => 0,
        ProviderKind::Navidrome => 1,
        ProviderKind::SoundCloud => 2,
    }
}

/// A title as copies share it.
fn title_key(title: &str) -> String {
    let folded = fold(title.trim());
    let mut key = folded.as_str();
    for suffix in [" - single", " - ep"] {
        key = key.strip_suffix(suffix).unwrap_or(key);
    }
    key.trim().to_owned()
}

/// The first artist, as copies share them: services join several
/// differently.
pub fn artist_key(artists: &[String]) -> String {
    let first = artists.first().map_or("", String::as_str);
    let folded = fold(first);
    let mut cut = folded.len();
    for sep in [" & ", ", ", " feat", " ft.", " x ", " and "] {
        if let Some(at) = folded.find(sep) {
            cut = cut.min(at);
        }
    }
    folded[..cut].trim().to_owned()
}

/// The items to leave out of `items`: in each group sharing a key, those of
/// a service with a better copy, as `same` tells copies apart within it.
fn worse_copies<T>(
    items: &[T],
    key: impl Fn(&T) -> (String, String),
    provider: impl Fn(&T) -> ProviderKind,
    rank: impl Fn(ProviderKind) -> u8,
    same: impl Fn(&T, &T) -> bool,
) -> HashSet<usize> {
    let mut groups: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for (i, item) in items.iter().enumerate() {
        groups.entry(key(item)).or_default().push(i);
    }
    let mut out = HashSet::new();
    for mut group in groups.into_values().filter(|g| g.len() > 1) {
        group.sort_by_key(|&i| rank(provider(&items[i])));
        for (n, &i) in group.iter().enumerate() {
            let better = group[..n].iter().any(|&j| {
                provider(&items[j]) != provider(&items[i])
                    && !out.contains(&j)
                    && same(&items[j], &items[i])
            });
            if better {
                out.insert(i);
            }
        }
    }
    out
}

/// The songs to leave out of `songs`.
pub fn songs(songs: &[Track]) -> HashSet<usize> {
    worse_copies(
        songs,
        |t| (title_key(&t.title), artist_key(&t.artists)),
        |t| t.id.provider,
        rank,
        |a, b| match (a.duration, b.duration) {
            (Some(x), Some(y)) => x.abs_diff(y) <= SAME_LENGTH,
            _ => true,
        },
    )
}

/// The albums to leave out of `albums`.
pub fn albums(albums: &[Album]) -> HashSet<usize> {
    worse_copies(
        albums,
        |a| (title_key(&a.title), artist_key(&a.artists)),
        |a| a.id.provider,
        rank,
        |_, _| true,
    )
}

/// The artists to leave out of `artists`.
pub fn artists(artists: &[Artist]) -> HashSet<usize> {
    worse_copies(
        artists,
        |a| (fold(a.name.trim()), String::new()),
        |a| a.id.provider,
        artist_rank,
        |_, _| true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use music_core::model::ItemId;

    fn track(provider: ProviderKind, title: &str, artist: &str, secs: u64) -> Track {
        Track {
            id: ItemId::new(provider, title),
            title: title.into(),
            artists: vec![artist.into()],
            album: None,
            album_id: None,
            duration: Some(Duration::from_secs(secs)),
            track_number: None,
            disc_number: None,
            artwork: None,
            explicit: false,
            quality: None,
            in_library: true,
            library_id: None,
        }
    }

    #[test]
    fn the_servers_copy_of_a_song_wins_and_other_versions_stay() {
        let list = vec![
            track(
                ProviderKind::AppleMusic,
                "#1 (Intro)",
                "Future & Metro Boomin",
                41,
            ),
            track(ProviderKind::Navidrome, "#1 (Intro)", "Future", 42),
            track(ProviderKind::SoundCloud, "#1 (intro)", "future", 40),
            // A long version is another song.
            track(ProviderKind::AppleMusic, "#1 (Intro)", "Future", 200),
        ];
        let out = songs(&list);
        assert_eq!(out, HashSet::from([0, 2]));
    }

    #[test]
    fn a_services_own_copies_all_stay() {
        let list = vec![
            track(ProviderKind::AppleMusic, "Same", "One", 100),
            track(ProviderKind::AppleMusic, "Same", "One", 100),
        ];
        assert!(songs(&list).is_empty());
    }

    #[test]
    fn titles_and_artists_fold_as_services_write_them() {
        assert_eq!(title_key("#horsérie2 - Single"), title_key("#HORSERIE2"));
        assert_eq!(
            artist_key(&["Future, Metro Boomin".into()]),
            artist_key(&["Future & Metro Boomin".into()])
        );
    }
}
