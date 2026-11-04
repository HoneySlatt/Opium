//! The services and the device's own files as one: an item goes to its own
//! service, a search to all of them, and what several have shows once, as
//! the best copy.

use std::sync::Arc;

use apple_music::AppleMusic;
use local::Local;
use music_core::Provider;
use music_core::model::{Album, ItemId, ProviderKind, SearchResults, Track};
use navidrome::Navidrome;
use soundcloud::SoundCloud;

/// Where a search looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Apple Music's and SoundCloud's catalogues, the server and the files.
    Catalog,
    /// The library: Apple Music's, the server and the files.
    Library,
}

/// The services connected, each `None` when it is off or signed out.
#[derive(Clone, Default)]
pub struct Services {
    pub apple: Option<Arc<AppleMusic>>,
    pub soundcloud: Option<Arc<SoundCloud>>,
    pub navidrome: Option<Arc<Navidrome>>,
    /// The music files of the folders chosen.
    pub local: Option<Arc<Local>>,
}

impl Services {
    /// The service `kind`, when connected.
    pub fn provider(&self, kind: ProviderKind) -> Option<Arc<dyn Provider>> {
        match kind {
            ProviderKind::AppleMusic => self.apple.clone().map(|p| p as Arc<dyn Provider>),
            ProviderKind::SoundCloud => self.soundcloud.clone().map(|p| p as Arc<dyn Provider>),
            ProviderKind::Navidrome => self.navidrome.clone().map(|p| p as Arc<dyn Provider>),
            ProviderKind::Local => self.local.clone().map(|p| p as Arc<dyn Provider>),
            // What Music keeps of its own is the app's, not a service's.
            ProviderKind::Music => None,
        }
    }

    /// The service an item is from, when connected.
    pub fn of(&self, id: &ItemId) -> Option<Arc<dyn Provider>> {
        self.provider(id.provider)
    }

    /// A search of every service: the catalog's in Apple Music and SoundCloud,
    /// the library's in Apple Music's; the server's and the files' either
    /// way, as they are the library. One that fails leaves the others' results. `only` asks that
    /// service alone, with all of its results, the copies others have too.
    pub async fn search(
        &self,
        scope: Scope,
        query: &str,
        only: Option<ProviderKind>,
    ) -> Result<SearchResults, String> {
        let asked = |kind| only.is_none_or(|k| k == kind);
        let files = self.local.clone().filter(|_| asked(ProviderKind::Local));
        let (apple, sc, nd) = (
            self.apple
                .clone()
                .filter(|_| asked(ProviderKind::AppleMusic)),
            self.soundcloud
                .clone()
                .filter(|_| asked(ProviderKind::SoundCloud)),
            self.navidrome
                .clone()
                .filter(|_| asked(ProviderKind::Navidrome)),
        );
        let apple_side = async {
            let am = apple?;
            Some(match scope {
                Scope::Catalog => am.search(query).await,
                Scope::Library => am.search_library(query).await,
            })
        };
        let sc_side = async {
            match (sc, scope) {
                (Some(sc), Scope::Catalog) => Some(sc.search(query).await),
                _ => None,
            }
        };
        let nd_side = async { Some(nd?.search(query).await) };
        let files_side = async { Some(files?.search_library(query).await) };
        let (a, b, n, f) = tokio::join!(apple_side, sc_side, nd_side, files_side);
        let mut error = None;
        let mut ok = |side: Option<music_core::Result<SearchResults>>| match side? {
            Ok(r) => Some(r),
            Err(e) => {
                error = Some(e.to_string());
                None
            }
        };
        let (apple, sc, nd, files) = (ok(a), ok(b), ok(n), ok(f));
        let any = apple.is_some() || sc.is_some() || nd.is_some() || files.is_some();
        // What several have is the best copy's, as in the library: the
        // device's own file, the server's, then Apple Music's AAC 256, then
        // SoundCloud's.
        let sides = [files, nd, apple, sc];
        let tracks: Vec<Track> = sides
            .iter()
            .flatten()
            .flat_map(|r| r.tracks.clone())
            .collect();
        let albums: Vec<Album> = sides
            .iter()
            .flatten()
            .flat_map(|r| r.albums.clone())
            .collect();
        let (worse_tracks, worse_albums) = (
            music_core::dedupe::songs(&tracks),
            music_core::dedupe::albums(&albums),
        );
        let (mut t, mut a) = (0, 0);
        let [files, nd, apple, sc] = sides.map(|side| {
            side.map(|mut r| {
                r.tracks.retain(|_| {
                    t += 1;
                    !worse_tracks.contains(&(t - 1))
                });
                r.albums.retain(|_| {
                    a += 1;
                    !worse_albums.contains(&(a - 1))
                });
                r
            })
        });
        // Taken in turns, Apple Music's first: the first rows show them all.
        let mut merged = SearchResults::default();
        for side in [apple, files, nd, sc].into_iter().flatten() {
            merged.tracks = in_turns(merged.tracks, side.tracks);
            merged.albums = in_turns(merged.albums, side.albums);
            merged.artists = in_turns(merged.artists, side.artists);
            merged.playlists = in_turns(merged.playlists, side.playlists);
        }
        match (any, error) {
            (false, Some(e)) => Err(e),
            // The one asked alone has nothing to search here.
            (false, None) if only.is_some() => Ok(merged),
            (false, None) => Err("No service is connected".into()),
            _ => Ok(merged),
        }
    }
}

/// `a`'s and `b`'s items in turns, starting with `a`'s; the longer one's
/// rest after.
pub fn in_turns<T>(a: Vec<T>, b: Vec<T>) -> Vec<T> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    let (mut a, mut b) = (a.into_iter(), b.into_iter());
    loop {
        match (a.next(), b.next()) {
            (None, None) => return out,
            (x, y) => out.extend(x.into_iter().chain(y)),
        }
    }
}

/// The first of a song's artists, as services join several in one name.
pub fn first_artist(artists: &[String]) -> String {
    let first = artists.first().map_or("", String::as_str);
    // ASCII lowercasing keeps the byte offsets of the name's own.
    let lower = first.to_ascii_lowercase();
    let cut = [" & ", ", ", " feat", " ft.", " x "]
        .iter()
        .filter_map(|sep| lower.find(sep))
        .min()
        .unwrap_or(first.len());
    first[..cut].trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_come_in_turns_then_the_rest() {
        assert_eq!(in_turns(vec![1, 3, 5, 7], vec![2, 4]), [1, 2, 3, 4, 5, 7]);
        assert_eq!(in_turns(Vec::new(), vec![2, 4]), [2, 4]);
    }

    #[test]
    fn the_first_artist_is_cut_where_services_join_them() {
        assert_eq!(first_artist(&["Future & Metro Boomin".into()]), "Future");
        assert_eq!(first_artist(&["Ωé, X".into()]), "Ωé");
    }
}
