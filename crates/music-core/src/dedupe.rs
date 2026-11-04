//! The same song, album or artist from several services shows once: the best
//! copy stays, the others are left out of the lists. Songs and albums keep
//! the device's own file first, then the server's, then Apple Music's AAC
//! 256, then SoundCloud's; artists keep Apple Music's, which has their
//! photos, their page gathering their releases from everywhere.
//!
//! Copies are told apart by their title and first artist, folded (case,
//! accents, `- Single` and `- EP`, `&` or `,` between artists), and songs by
//! their length too, within a few seconds: a remix or a long version stays
//! apart. Copies within one service all stay: they are its own.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use crate::model::{Album, Artist, ProviderKind, Track};
use crate::text::fold;

/// How much songs' lengths may differ and still be one song.
const SAME_LENGTH: Duration = Duration::from_secs(3);

/// Which copy of a song or album wins: the lower.
fn rank(provider: ProviderKind) -> u8 {
    match provider {
        ProviderKind::Local => 0,
        ProviderKind::Navidrome => 1,
        ProviderKind::AppleMusic => 2,
        ProviderKind::SoundCloud => 3,
        // Music has no songs of its own.
        ProviderKind::Music => 4,
    }
}

/// Which copy of an artist wins: Apple Music's, for its photos.
fn artist_rank(provider: ProviderKind) -> u8 {
    match provider {
        ProviderKind::AppleMusic => 0,
        ProviderKind::Navidrome => 1,
        ProviderKind::Local => 2,
        ProviderKind::SoundCloud => 3,
        ProviderKind::Music => 4,
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

/// What copies of an album share.
pub fn album_key(album: &Album) -> (String, String) {
    (title_key(&album.title), artist_key(&album.artists))
}

/// What copies of a song share.
pub fn song_key(track: &Track) -> (String, String) {
    (title_key(&track.title), artist_key(&track.artists))
}

/// Whether two tracks are one song.
pub fn same_song(a: &Track, b: &Track) -> bool {
    title_key(&a.title) == title_key(&b.title)
        && artist_key(&a.artists) == artist_key(&b.artists)
        && match (a.duration, b.duration) {
            (Some(x), Some(y)) => x.abs_diff(y) <= SAME_LENGTH,
            _ => true,
        }
}

/// An album whole from its copies on several services: each song from the
/// best copy that has it, the others filling in what it lacks, in disc and
/// track order where the copies number them.
pub fn album_tracks(copies: &[&[Track]]) -> Vec<Track> {
    let mut copies: Vec<&[Track]> = copies.to_vec();
    copies.sort_by_key(|c| c.first().map_or(u8::MAX, |t| rank(t.id.provider)));
    let mut out: Vec<Track> = Vec::new();
    for copy in copies {
        // A copy's own songs all stay: two of its interludes are two songs.
        let before = out.len();
        for track in copy {
            if !out[..before].iter().any(|t| same_song(t, track)) {
                out.push(track.clone());
            }
        }
    }
    if out.iter().all(|t| t.track_number.is_some()) {
        out.sort_by_key(|t| (t.disc_number.unwrap_or(1), t.track_number));
    }
    out
}

/// One service's songs by what copies share, to find a song's copy there:
/// the server's file to play in place of a stream.
#[derive(Debug, Default, Clone)]
pub struct Copies(HashMap<(String, String), Vec<Track>>);

impl Copies {
    pub fn of<'a>(songs: impl IntoIterator<Item = &'a Track>) -> Self {
        let mut index: HashMap<(String, String), Vec<Track>> = HashMap::new();
        for song in songs {
            index.entry(song_key(song)).or_default().push(song.clone());
        }
        Self(index)
    }

    /// The copy of `track` among them, if there is one.
    pub fn find(&self, track: &Track) -> Option<&Track> {
        self.0
            .get(&song_key(track))?
            .iter()
            .find(|copy| same_song(copy, track))
    }
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
    use crate::model::ItemId;

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
    fn an_album_is_whole_from_its_copies_the_best_one_first() {
        let numbered = |mut t: Track, n| {
            t.track_number = Some(n);
            t
        };
        let server = vec![
            numbered(track(ProviderKind::Navidrome, "One", "Artist", 100), 1),
            numbered(track(ProviderKind::Navidrome, "Three", "Artist", 120), 3),
        ];
        let apple = vec![
            numbered(track(ProviderKind::AppleMusic, "One", "Artist", 101), 1),
            numbered(track(ProviderKind::AppleMusic, "Two", "Artist", 90), 2),
            numbered(track(ProviderKind::AppleMusic, "Three", "Artist", 120), 3),
        ];
        let whole = album_tracks(&[&apple, &server]);
        let got: Vec<(&str, ProviderKind)> = whole
            .iter()
            .map(|t| (t.title.as_str(), t.id.provider))
            .collect();
        assert_eq!(
            got,
            [
                ("One", ProviderKind::Navidrome),
                ("Two", ProviderKind::AppleMusic),
                ("Three", ProviderKind::Navidrome),
            ]
        );
    }

    #[test]
    fn an_albums_own_namesakes_stay_when_merged() {
        let server = vec![track(ProviderKind::Navidrome, "One", "Artist", 100)];
        let apple = vec![
            track(ProviderKind::AppleMusic, "Interlude", "Artist", 60),
            track(ProviderKind::AppleMusic, "Interlude", "Artist", 61),
        ];
        assert_eq!(album_tracks(&[&apple, &server]).len(), 3);
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
