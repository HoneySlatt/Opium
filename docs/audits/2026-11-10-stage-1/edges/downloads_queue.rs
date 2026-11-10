// Written to fail at 46e7a5a, never compiled during the audit; its twin in ../edges/proofs or
// ../data/proofs ran and failed (see ../README.md).
// Belongs in crates/downloads/src/queue.rs, `mod tests` (uses its `Songs`
// fetcher). Fails at 46e7a5a: the confirmation said before `opened` is
// dropped (queue.rs:234-239), which the desktop does at every start
// (crates/desktop/src/backend.rs:1161-1169). Proved in run 01.

    #[derive(Default)]
    struct KeptTold(Mutex<Vec<Vec<KeptSong>>>);
    impl Listener for KeptTold {
        fn state(&self, _: ItemId, _: Downloading) {}
        fn kept(&self, kept: Vec<KeptSong>) {
            lock(&self.0).push(kept);
        }
        fn failed(&self, _: String) {}
    }

    #[tokio::test]
    async fn a_subscription_said_before_the_store_opens_still_counts() {
        let dir = std::env::temp_dir().join(format!("queue-{:016x}", rand::random::<u64>()));
        let key = sealed::new_key();
        let long_ago = now() - 40 * 24 * 60 * 60;
        {
            let mut store = Store::open(&dir, key).unwrap();
            let mut pending = store.begin().unwrap();
            pending.write_all(b"x").unwrap();
            let track = Track {
                id: ItemId::new(ProviderKind::SoundCloud, "1"),
                ..track("1")
            };
            let song = Song {
                track,
                extension: "m4a".into(),
                priming: 0,
                protected: true,
                kbps: None,
                spatial: false,
            };
            store.keep(pending, song, long_ago).unwrap();
        }
        let told = Arc::new(KeptTold::default());
        let downloads = Downloads::new(Arc::new(Songs), told.clone());
        // Signed in to SoundCloud at start, the keyring still asked for the key.
        downloads.subscription(ProviderKind::SoundCloud, true);
        downloads.opened(Ok(Store::open(&dir, key).unwrap()));
        let kept = lock(&told.0).last().cloned().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            matches!(kept[0].standing, Standing::PlaysUntil(_)),
            "{:?}",
            kept[0].standing
        );
    }
