// Written to fail at 46e7a5a, never compiled during the audit; its twin in ../edges/proofs or
// ../data/proofs ran and failed (see ../README.md).
// Belongs in crates/local/src/scan.rs, `mod tests` (needs the dev-dependency
// `image`). Fails at 46e7a5a, though not every run: `keep_cover`
// (scan.rs:320-330) writes one `<hash>.part` from every thread, so a thread
// whose `.part` another renamed fails its own rename and returns `None`. A
// verbatim replica lost 1 cover in 160 (run 01). Worth looping, e.g.
// `cargo test -p local the_same_cover -- --test-threads=1`, a few times.

    #[test]
    fn the_same_cover_kept_by_eight_threads_at_once_is_kept_for_each() {
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_fn(640, 640, |x, y| image::Rgb([x as u8, y as u8, 90]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let png = png.into_inner();
        let mut without = 0;
        for round in 0..20 {
            let covers = std::env::temp_dir()
                .join(format!("local-covers-race-{}-{round}", std::process::id()));
            let start = std::sync::Barrier::new(8);
            let kept: Vec<Option<String>> = std::thread::scope(|s| {
                let handles: Vec<_> = (0..8)
                    .map(|_| {
                        s.spawn(|| {
                            start.wait();
                            keep_cover(&png, &covers)
                        })
                    })
                    .collect();
                handles.into_iter().map(|h| h.join().unwrap()).collect()
            });
            without += kept.iter().filter(|k| k.is_none()).count();
            let _ = std::fs::remove_dir_all(&covers);
        }
        assert_eq!(without, 0, "tracks left without their cover");
    }
