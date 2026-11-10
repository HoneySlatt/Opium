// Written to fail at 46e7a5a, never compiled during the audit; its twin in ../edges/proofs or
// ../data/proofs ran and failed (see ../README.md).
// Belongs in crates/downloads/src/store.rs, `mod tests` (uses its `dir`,
// `keep`, `song` helpers). Both fail at 46e7a5a; proved through the public
// API in .delegation/m18-edges/repro/tests/edges.rs (run 01).

    fn sealed_files(dir: &Path) -> usize {
        std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".sealed"))
            .count()
    }

    #[test]
    fn a_manifest_put_aside_keeps_its_songs_through_the_next_starts() {
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
        std::fs::write(dir.0.join(MANIFEST), b"{ damaged").unwrap();
        {
            // The next start: put aside, then anything saved, as a confirmed subscription is
            // (Apple's at each connection on the desktop, each service's on the phone); a
            // SoundCloud confirmation stands in for it here.
            let mut store = Store::open(&dir.0, key).unwrap();
            store
                .subscription(ProviderKind::SoundCloud, true, 20)
                .unwrap();
        }
        // The start after.
        Store::open(&dir.0, key).unwrap();
        assert_eq!(sealed_files(&dir.0), 1, "the song was swept");
    }

    #[test]
    fn a_clock_set_back_doesnt_bring_an_expired_song_back() {
        let dir = dir();
        let mut store = Store::open(&dir.0, sealed::new_key()).unwrap();
        let confirmed = 1_800_000_000;
        keep(
            &mut store,
            song(ProviderKind::AppleMusic, "1", true),
            b"x",
            confirmed,
        );
        let id = ItemId::new(ProviderKind::AppleMusic, "1");
        let expired = confirmed + PROTECTED_FOR;
        assert!(store.open_song(&id, expired).is_none());
        // Offline, the clock set back a year.
        assert!(store.open_song(&id, expired - 365 * 24 * 60 * 60).is_none());
    }
