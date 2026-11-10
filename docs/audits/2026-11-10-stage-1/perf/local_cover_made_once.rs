// Written to fail at 46e7a5a, never compiled during the audit; a race, so it would fail very
// likely but not every run (see ../README.md and ../edges/local_scan.rs).
// Belongs in `crates/local/src/scan.rs`, inside `pub(crate) mod tests`: `keep_cover` is in the
// private module `scan`. Uses the crate's dev-dependency `image` (PNG).
//
// Finding: `scan` reads up to eight files at once, in folder order (`scan.rs:204-225`), so an
// album's first eight files reach `keep_cover` together, each with the same embedded picture.
// `keep_cover` (`scan.rs:320-330`) only checks that the cover's file is not there yet: all eight
// decode, resize and encode the same picture, then write the same `<hash>.part` and rename it.
// A copy of `keep_cover` with a counter, run 20 times by 8 threads at once
// (`.delegation/m18-perf/bench/src/bin/cover_race.rs`, proof `.delegation/m18-perf/06.out`):
// 152 covers made where 20 would do, and 25 of 160 calls gave `None` (the second rename finds
// the `.part` gone), the file then listed without its own cover.
//
// Written to fail on the lost covers (the work done eight times over is not visible from outside:
// see the copy's counter). Passes once one worker makes a given cover while the others wait for
// it, or each writes a `.part` of its own.

/// An album's cover, embedded in each of its files, reached by eight of the scan's workers at
/// once: each file is told where the cover is kept.
#[test]
fn an_albums_cover_reached_by_every_worker_at_once_is_kept_for_each_file() {
    let dir = std::env::temp_dir().join(format!("local-cover-once-{}", std::process::id()));
    // As covers come: 1200 px, so that making it square takes as long as it does for real.
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_fn(1200, 1200, |x, y| image::Rgb([x as u8, y as u8, 90]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let picture = png.into_inner();
    let mut without = 0;
    for round in 0..10 {
        let covers = dir.join(format!("covers-{round}"));
        let together = std::sync::Barrier::new(8);
        let kept: Vec<Option<String>> = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        together.wait();
                        keep_cover(&picture, &covers)
                    })
                })
                .collect();
            workers.into_iter().map(|w| w.join().unwrap()).collect()
        });
        without += kept.iter().filter(|k| k.is_none()).count();
    }
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(without, 0, "of 80 files, {without} told of no cover");
}
