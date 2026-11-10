    // Belongs in: crates/opium-core/src/dedupe.rs, module `tests` (uses `use super::*`).
    //
    // Passes at 46e7a5a and kills the mutants named below (run in a copy: proofs named in the
    // report).

    fn album(provider: ProviderKind, title: &str, artist: &str) -> Album {
        Album {
            id: ItemId::new(provider, title),
            title: title.into(),
            artists: vec![artist.into()],
            artist_id: None,
            artist_artwork: None,
            released: None,
            genre: None,
            description: None,
            catalog_id: None,
            in_favourites: None,
            year: None,
            artwork: None,
            track_count: None,
            explicit: false,
            added: None,
            motion: None,
            copyright: None,
            mastered: false,
        }
    }

    fn artist(provider: ProviderKind, name: &str) -> Artist {
        Artist {
            id: ItemId::new(provider, name),
            name: name.into(),
            artwork: None,
            in_favourites: None,
            library_id: None,
        }
    }

    /// The library's Albums and Artists leave out the worse copies of one album or one artist,
    /// as Songs does, and nothing else: of an album, the copy of the better-ranked service stays
    /// (the server's before Apple Music's); of an artist, Apple Music's, for its photos. No test
    /// called `albums` or `artists` before, and both apps hide what they answer.
    ///
    /// Kills `dedupe.rs:112:5 replace albums -> HashSet<usize> with HashSet::from_iter([0])` and
    /// `dedupe.rs:206:5 replace artists -> HashSet<usize> with HashSet::from_iter([0])` (both
    /// clean misses), and their `HashSet::new()` and `HashSet::from_iter([1])`.
    #[test]
    fn albums_and_artists_keep_their_best_copy_and_the_rest() {
        let albums_here = [
            album(ProviderKind::Navidrome, "Album", "Artist"),
            album(ProviderKind::AppleMusic, "album", "artist"),
            album(ProviderKind::AppleMusic, "Another", "Artist"),
        ];
        assert_eq!(albums(&albums_here), HashSet::from([1]));
        let apart = [
            album(ProviderKind::AppleMusic, "Album", "Artist"),
            album(ProviderKind::Navidrome, "Other", "Artist"),
        ];
        assert!(albums(&apart).is_empty());

        let artists_here = [
            artist(ProviderKind::AppleMusic, "Artist"),
            artist(ProviderKind::Navidrome, "artist"),
            artist(ProviderKind::Local, "Other"),
        ];
        assert_eq!(artists(&artists_here), HashSet::from([1]));
        let apart = [
            artist(ProviderKind::Navidrome, "Artist"),
            artist(ProviderKind::AppleMusic, "Other"),
        ];
        assert!(artists(&apart).is_empty());
    }
