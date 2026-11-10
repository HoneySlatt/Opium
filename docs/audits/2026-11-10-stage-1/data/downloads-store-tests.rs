// Written to fail at 46e7a5a, never compiled during the audit; its twin in ../edges/proofs or
// ../data/proofs ran and failed (see ../README.md).
// Belongs in: crates/downloads/src/store.rs, at the end of `mod tests`
// (it uses that module's `dir()`, `keep()`, `song()` and `MANIFEST`).
// Run: cargo test -p downloads store::tests
// All three fail at 46e7a5a; the same checks, through public items, ran in
// .delegation/m18-data/scratch/tests/proofs.rs (proof 02.out).

    fn sealed_files(dir: &Path) -> usize {
        std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".sealed"))
            .count()
    }

    /// ADR-0017: a manifest that doesn't read is put aside "without
    /// sweeping away the files it no longer names", at the start that finds
    /// it and at every start after.
    #[test]
    fn songs_of_a_manifest_put_aside_are_still_there_at_the_next_start() {
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
        drop(Store::open(&dir.0, key).unwrap());
        drop(Store::open(&dir.0, key).unwrap());
        assert_eq!(sealed_files(&dir.0), 1, "the song's file is still there");
    }

    /// The same when, in between, a service confirmed its subscription
    /// (each Apple Music connection does), which writes the manifest anew.
    #[test]
    fn songs_of_a_manifest_put_aside_stay_once_a_subscription_is_confirmed() {
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
        // A later version's, whole: it reaches the version check.
        std::fs::write(
            dir.0.join(MANIFEST),
            br#"{"version":99,"items":[],"confirmed":{}}"#,
        )
        .unwrap();
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            store
                .subscription(ProviderKind::AppleMusic, true, 20)
                .unwrap();
        }
        drop(Store::open(&dir.0, key).unwrap());
        assert_eq!(sealed_files(&dir.0), 1, "the song's file is still there");
    }

    /// A song kept again as the same track: when the manifest can't be
    /// written, the copy kept before still plays at the next start.
    #[test]
    fn a_song_kept_again_still_plays_when_the_manifest_cant_be_written() {
        let dir = dir();
        let key = sealed::new_key();
        let id = ItemId::new(ProviderKind::Navidrome, "1");
        {
            let mut store = Store::open(&dir.0, key).unwrap();
            keep(
                &mut store,
                song(ProviderKind::Navidrome, "1", false),
                b"first copy",
                10,
            );
            // As a full disk or an I/O error would: a folder stands where
            // the manifest's new copy goes.
            std::fs::create_dir(dir.0.join("manifest.json.part")).unwrap();
            let mut pending = store.begin().unwrap();
            pending.write_all(b"second copy").unwrap();
            assert!(
                store
                    .keep(pending, song(ProviderKind::Navidrome, "1", false), 20)
                    .is_err()
            );
        }
        std::fs::remove_dir(dir.0.join("manifest.json.part")).unwrap();
        let store = Store::open(&dir.0, key).unwrap();
        assert!(
            store.open_song(&id, 30).is_some_and(|r| r.is_ok()),
            "one copy of the song still opens"
        );
    }
