//! The `Indexer` fed the same bytes in growing prefixes, cut where the input says, must agree
//! with the whole file indexed at once.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The first byte says how many cuts follow (up to 7), each a 16-bit offset into the file.
    let Some((&count, rest)) = data.split_first() else {
        return;
    };
    let count = usize::from(count & 7);
    if rest.len() < 2 * count {
        return;
    }
    let (cuts, file) = rest.split_at(2 * count);
    let mut cuts: Vec<usize> = cuts
        .chunks(2)
        .map(|c| usize::from(u16::from_be_bytes([c[0], c[1]])).min(file.len()))
        .collect();
    cuts.sort_unstable();
    cuts.push(file.len());

    let whole = cenc::index(file);
    let mut arrived: Option<(cenc::Movie, cenc::Indexer)> = None;
    let mut failed = None;
    for cut in cuts {
        let data = &file[..cut];
        if arrived.is_none() {
            match cenc::Movie::read(data) {
                Ok(Some(movie)) => {
                    let indexer = cenc::Indexer::new(&movie);
                    arrived = Some((movie, indexer));
                }
                Ok(None) => continue,
                Err(e) => {
                    failed = Some(e);
                    break;
                }
            }
        }
        let (_, indexer) = arrived.as_mut().unwrap();
        match indexer.advance(data) {
            Ok(ready) => assert!(ready <= data.len()),
            Err(e) => {
                failed = Some(e);
                break;
            }
        }
    }
    let streamed = match (failed, arrived) {
        (Some(e), _) => Err(e),
        (None, None) => return,
        (None, Some((movie, mut indexer))) => indexer.finish(file).map(|()| (movie, indexer)),
    };
    match (whole, streamed) {
        (Ok((movie, table)), Ok((streamed_movie, indexer))) => {
            assert_eq!(movie, streamed_movie);
            assert_eq!(table, *indexer.samples());
        }
        (Err(a), Err(b)) => assert_eq!(a, b),
        (Ok(_), Err(e)) => panic!("whole indexes, a download fails: {e}"),
        (Err(e), Ok(_)) => panic!("a download indexes, whole fails: {e}"),
    }
});
