    // Two parts, for two files of crates/player. Built and run in a copy of the crate during the audit;
    // written against the code at 46e7a5a, its values explained in the comments.

    // ---------------------------------------------------------------------------------------------
    // Part 1. Belongs in: crates/player/src/output.rs, module `tests`. Passes at 46e7a5a.

    /// The fade when the ring runs dry, in stereo: the test before is mono, where `n / channels`
    /// and `n * channels` are the same number.
    ///
    /// 100 stereo frames of 1.0 into 200 frames' room, at 1 kHz: the fade is 5 frames long, frames
    /// 95 to 99 at 0.8, 0.6, 0.4, 0.2 and 0, both channels alike; the rest silent.
    ///
    /// Kills `output.rs:116:24 replace / with * in fill` (a clean miss): `frames` would be 400, the
    /// fade skip past every frame, and the music cut off where it ran dry, a click at each stall.
    #[test]
    fn a_ring_run_dry_fades_out_in_stereo_too() {
        let (mut producer, mut consumer) = rtrb::RingBuffer::new(1024);
        for _ in 0..200 {
            producer.push(1.0).unwrap();
        }
        let state = OutputState::new(1.0);
        let mut gain = 1.0;
        let mut out = [0.5; 400];
        fill(&mut consumer, &state, &mut gain, &mut out, 2, 1_000);
        assert_eq!(out[..190], [1.0; 190]);
        assert!(out[190] < 1.0 && out[190] > 0.0 && out[190] == out[191]);
        assert_eq!(out[198..], [0.0; 202]);
        assert_eq!(gain, 0.0);
    }

    /// A pause fades out, then reads nothing more: what is buffered waits in the ring, whole, and
    /// plays from where it stopped once the music goes on.
    ///
    /// 100 frames of 1.0, mono, 1 kHz: the fade to silence reads about 5 of them.
    ///
    /// Kills `output.rs:91:28 replace == with != in fill` (a clean miss: the pause would cut the
    /// sound at once rather than fade, and once silent the output would read on, the song moving on
    /// unheard) and `output.rs:133:14 replace && with || in fill` (a clean miss: everything
    /// buffered dropped at each pause, the music skipping ahead on resume).
    #[test]
    fn a_pause_fades_out_and_keeps_what_is_buffered() {
        let (mut producer, mut consumer) = rtrb::RingBuffer::new(1024);
        for _ in 0..100 {
            producer.push(1.0).unwrap();
        }
        let state = OutputState::new(1.0);
        state.paused.store(true, Ordering::Relaxed);
        let mut gain = 1.0;
        let mut out = [0.5; 20];
        fill(&mut consumer, &state, &mut gain, &mut out, 1, 1_000);
        assert!(out[0] > 0.0 && out[0] < 1.0, "a pause fades out");
        assert_eq!(gain, 0.0);
        let left = consumer.slots();
        assert!(left >= 90, "only the fade was read: {left} left");
        // Paused and silent: nothing read, nothing dropped.
        let mut out = [0.5; 20];
        fill(&mut consumer, &state, &mut gain, &mut out, 1, 1_000);
        assert_eq!(out, [0.0; 20]);
        assert_eq!(consumer.slots(), left);
        // Played again: from where it stopped, fading in.
        state.paused.store(false, Ordering::Relaxed);
        let mut out = [0.0; 10];
        fill(&mut consumer, &state, &mut gain, &mut out, 1, 1_000);
        assert!(out[0] > 0.0 && out[0] < 1.0);
        assert_eq!(consumer.slots(), left - 10);
    }

    // ---------------------------------------------------------------------------------------------
    // Part 2. Belongs in: crates/player/src/gapless.rs, module `tests` (uses `boxed`, `full` and
    // `header`).

    /// A file whose sound track has one edit, `duration` long at the movie's `movie_scale` and
    /// starting at `start`, in an edit list of `version`.
    fn edited(version: u8, duration: u64, start: u64, movie_scale: u32) -> Vec<u8> {
        let mut elst = 1u32.to_be_bytes().to_vec();
        if version == 1 {
            elst.extend_from_slice(&duration.to_be_bytes());
            elst.extend_from_slice(&start.to_be_bytes());
        } else {
            elst.extend_from_slice(&(duration as u32).to_be_bytes());
            elst.extend_from_slice(&(start as u32).to_be_bytes());
        }
        elst.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        let mut hdlr = vec![0u8; 4];
        hdlr.extend_from_slice(b"soun");
        let mdia = [header(b"mdhd", 44_100), boxed(b"hdlr", &full(0, &hdlr))].concat();
        let trak = [
            boxed(b"edts", &boxed(b"elst", &full(version, &elst))),
            boxed(b"mdia", &mdia),
        ]
        .concat();
        let moov = [header(b"mvhd", movie_scale), boxed(b"trak", &trak)].concat();
        [boxed(b"ftyp", b"iso6"), boxed(b"moov", &moov)].concat()
    }

    /// How long the music plays, from the edit's duration, in the media's frames: 10 s at the
    /// movie's 1,000 a second is 441,000 frames at 44.1 kHz. A movie that gives no timescale gives
    /// no length, rather than a division by zero. Passes at 46e7a5a: the test before has an edit of
    /// duration 0 only, so the length was never computed.
    ///
    /// Kills `gapless.rs:177:56 replace / with * in edit_list` (a clean miss: the length a thousand
    /// million frames, the padding played as a gap between the songs of an album) and
    /// `gapless.rs:176:51 replace > with >= in edit_list` (a clean miss: a panic, dividing by zero,
    /// on a file whose `mvhd` says a timescale of 0).
    #[test]
    fn the_edit_list_says_how_long_the_music_plays() {
        let trim = mp4_trim(&mut std::io::Cursor::new(edited(0, 10_000, 2112, 1000))).unwrap();
        assert_eq!(
            trim,
            Trim {
                priming: 2112,
                length: Some(441_000),
                timescale: Some(44_100)
            }
        );
        let trim = mp4_trim(&mut std::io::Cursor::new(edited(0, 10_000, 2112, 0))).unwrap();
        assert_eq!(trim.length, None);
    }

    /// FAILS at 46e7a5a, in the test build: `duration * u64::from(media_scale)` (gapless.rs:177)
    /// overflows on a version-1 edit list whose duration is near `u64::MAX`, which panics where
    /// overflow is checked (debug builds, `cargo test`) and wraps to a wrong length in a release
    /// build. Expected once fixed (`checked_mul`): no length. A malformed file is the only way in.
    #[test]
    fn an_edit_too_long_to_count_gives_no_length() {
        let file = edited(1, u64::MAX / 2, 2112, 1000);
        let trim = mp4_trim(&mut std::io::Cursor::new(file)).unwrap();
        assert_eq!(trim.length, None);
    }
