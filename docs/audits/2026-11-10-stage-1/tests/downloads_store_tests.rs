    // Belongs in: crates/downloads/src/store.rs, module `tests` (at its end). It uses that module's
    // helpers `dir`, `keep` and `song`, and `use super::*`.
    //
    // Each test passes at 46e7a5a and kills the mutants named above it.

    /// A manifest the system won't let the store read (a folder in its place here; a disk error, a
    /// permission) is neither missing nor damaged: the store fails to open, and sweeps nothing. Were
    /// it taken for a missing one, the store would start empty and sweep every song as nobody's
    /// (`sweep`, called when the manifest "says it all"): the user's downloads, protected songs
    /// included, gone for good (AGENTS.md: "An unreadable downloads manifest is put aside, never
    /// swept").
    ///
    /// Kills `store.rs:135:23 replace match guard e.kind() == io::ErrorKind::NotFound with true in
    /// Store::open`.
    #[test]
    fn a_manifest_that_cant_be_read_keeps_every_song() {
        let dir = dir();
        let key = sealed::new_key();
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            keep(
                &mut store,
                song(ProviderKind::Navidrome, "1", false),
                b"music",
                10,
            );
        }
        std::fs::remove_file(dir.0.join(MANIFEST)).unwrap();
        std::fs::create_dir(dir.0.join(MANIFEST)).unwrap();
        assert!(Store::open(&dir.0, key).is_err());
        let sealed = std::fs::read_dir(&dir.0)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".sealed"))
            .count();
        assert_eq!(sealed, 1, "the song's file is still there");
    }

    /// Thirty days from the last confirmation of the subscription, as Apple Music allows, and not
    /// a second more (ADR-0017). The test before reads the bound from `PROTECTED_FOR` itself, so
    /// any value of it passed.
    ///
    /// Kills the six `store.rs:22` mutants (`*` with `+` or `/`, at columns 35, 40 and 45).
    #[test]
    fn a_protected_song_plays_thirty_days_after_its_confirmation_and_no_more() {
        const DAY: u64 = 24 * 60 * 60;
        let dir = dir();
        let mut store = Store::open(&dir.0, sealed::new_key()).unwrap();
        let id = ItemId::new(ProviderKind::AppleMusic, "1");
        keep(
            &mut store,
            song(ProviderKind::AppleMusic, "1", true),
            b"x",
            0,
        );
        assert!(store.open_song(&id, 29 * DAY).is_some());
        assert!(store.open_song(&id, 30 * DAY - 1).is_some());
        assert!(store.open_song(&id, 30 * DAY).is_none());
    }
