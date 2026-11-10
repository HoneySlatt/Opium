    // Two parts, for two files of crates/connect.
    //
    // Each test passes at 46e7a5a (built and run in a copy of the crate during the audit) and
    // kills the mutants named above it.

    // ---------------------------------------------------------------------------------------------
    // Part 1. Belongs in: crates/connect/src/pair.rs, module `tests`. Uses its helper `exchange`.

    /// The key two devices keep is drawn from their pairing's own: two pairings keep two keys,
    /// neither a fixed one, and the derivation is pinned, as two devices on different versions of
    /// Opium must derive alike. The test before checks only that both ends agree, which a constant
    /// passes: every pairing would then share one key, and any device could pass for a paired one.
    ///
    /// The answer: HMAC-SHA-256 under 32 bytes of 7 of "opium-connect psk", computed apart with
    /// Python's `hmac` (proof `.delegation/m18-tests/10.out`).
    ///
    /// Kills `pair.rs:66:5 replace kept -> [u8; 32] with [0; 32]` (a clean miss) and `… with [1; 32]`.
    #[test]
    fn each_pairing_keeps_a_key_of_its_own() {
        let (one, _) = exchange("123456", "123456");
        let (other, _) = exchange("123456", "123456");
        assert_ne!(one, other, "each exchange agrees on a key of its own");
        assert_ne!(kept(&one), kept(&other));
        let answer = "09882de3409ff79f29a76b4e4df4a060ce436a795cda80bf832deae082dae629";
        let kept_of_sevens: String = kept(&[7; 32]).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(kept_of_sevens, answer);
    }

    // ---------------------------------------------------------------------------------------------
    // Part 2. Belongs in: crates/connect/src/wire.rs, module `tests`.

    /// A sealed message of megabytes comes whole (a long library's playlists run to megabytes, says
    /// `receive`), and one byte more than 32 MiB is refused rather than held in memory. The long
    /// message of the test before is 200,000 bytes, which a limit cut to a megabyte passes, and
    /// nothing tested the limit itself. Each message goes on a connection of its own whose reader
    /// is dropped once it has read: a refusal then ends the writer's wait, and the test fails
    /// rather than hangs. The 32 MiB take about 10 s through the cipher in a debug build (proof
    /// `.delegation/m18-tests/27.out`); the first half alone kills 18:30.
    ///
    /// Kills `wire.rs:18:30 replace * with +` (`32 + 1024 * 1024`, a megabyte) and `… with /`,
    /// both recorded "caught" by the audit's run though the test before passes them, and
    /// `wire.rs:163:40 replace > with == in receive`, recorded "caught" as well.
    #[tokio::test]
    async fn a_message_of_megabytes_comes_whole_and_one_too_long_is_refused() {
        async fn through(message: &[u8]) -> io::Result<Vec<u8>> {
            let (mut a, mut b) = tokio::io::duplex(1 << 20);
            let key = [7; 32];
            let (ka, kb) = tokio::join!(
                handshake(&mut a, true, &key, b"x"),
                handshake(&mut b, false, &key, b"x"),
            );
            let (ka, kb) = (ka.unwrap(), kb.unwrap());
            let (mut na, mut nb) = (0, 0);
            let (_, got) = tokio::join!(send(&mut a, &ka, &mut na, message), async move {
                let got = receive(&mut b, &kb, &mut nb).await;
                drop(b);
                got
            });
            got
        }
        let long: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
        assert_eq!(through(&long).await.unwrap(), long);
        // 32 MiB and a byte, written out: a test reading the bound from `MAX_SEALED` would move
        // with it.
        assert!(through(&vec![0u8; 32 * 1024 * 1024 + 1]).await.is_err());
    }

    /// A frame in the clear (a greeting, a pairing step) may be 64 KiB long, and no longer,
    /// whether written or read. The length is written out: read from `MAX_PLAIN`, the test would
    /// move with it. A frame too long is refused as such, before it is read, not met as the end
    /// of the stream.
    ///
    /// Kills `wire.rs:15:29 replace * with +` (`64 + 1024`: a clean miss) and `… with /`,
    /// `wire.rs:31:20 replace > with == in write_plain` (a clean miss), `… with <` and `… with
    /// >=`, and `wire.rs:41:12 replace > with >= in read_plain` (a clean miss), `… with ==` and
    /// `… with <`.
    #[tokio::test]
    async fn a_frame_in_the_clear_may_take_its_whole_limit_and_no_more() {
        let (mut a, mut b) = tokio::io::duplex(1 << 18);
        const LIMIT: usize = 64 * 1024;
        let longest = vec![1u8; LIMIT];
        write_plain(&mut a, &longest).await.unwrap();
        assert_eq!(read_plain(&mut b).await.unwrap(), longest);
        assert!(write_plain(&mut a, &vec![1u8; LIMIT + 1]).await.is_err());
        // Told that a longer one comes, and nothing after: refused before it is read.
        a.write_all(&((LIMIT + 1) as u32).to_be_bytes())
            .await
            .unwrap();
        drop(a);
        let refused = read_plain(&mut b).await.unwrap_err();
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
    }

    // -----------------------------------------------------------------------------------------
    // Part 3. Belongs in: crates/connect/src/store.rs, in a new module at its end (the file has
    // none): `#[cfg(test)] mod tests { use super::*; … }`.

    /// A device forgotten here loses its key, and is kept in mind as forgotten, once; the others
    /// stay paired. The test before (`devices_paired_with_one_are_with_each_other`, lib.rs:1745)
    /// checks only that a forgotten device isn't introduced again, which a `forget` that unpaired
    /// every other device and kept the forgotten one passes: a lost phone would stay paired.
    ///
    /// Kills `store.rs:216:50 replace != with == in Store::forget`, `store.rs:216:9 replace
    /// Store::forget with ()`, `store.rs:217:12 delete ! in Store::forget` and `store.rs:217:51
    /// replace == with != in Store::forget` (a clean miss).
    #[test]
    fn a_device_forgotten_loses_its_key_and_the_others_keep_theirs() {
        let dir = std::env::temp_dir().join(format!("connect-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let device = |id: &str| Device {
            id: id.into(),
            name: id.into(),
            kind: crate::DeviceKind::Phone,
        };
        let mut store = Store::open(&dir);
        store.pair(device("lost"), [1; 32]);
        store.pair(device("kept"), [2; 32]);
        store.forget("lost");
        store.forget("lost");
        assert_eq!(store.key("lost"), None);
        assert_eq!(store.key("kept"), Some([2; 32]));
        let again = Store::open(&dir);
        assert_eq!(again.saved.forgotten, ["lost"]);
        assert_eq!(again.key("kept"), Some([2; 32]));
        let _ = std::fs::remove_dir_all(&dir);
    }
