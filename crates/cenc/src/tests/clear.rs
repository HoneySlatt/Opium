//! Making a file clear (§6) changes only the types and counts named, byte for byte.

use super::build::{build, corpus, named};
use crate::{Indexer, Movie, index, make_clear};

#[test]
fn every_layout_made_clear_matches_the_file_built_clear() {
    for (name, spec) in corpus() {
        let built = build(&spec, false);
        let expected = build(&spec, true).bytes;
        let (movie, table) = index(&built.bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut cleared = built.bytes.clone();
        make_clear(&mut cleared, &movie, &table);
        assert!(
            cleared == expected,
            "{name}: only the named types and counts change"
        );
        make_clear(&mut cleared, &movie, &table);
        assert!(cleared == expected, "{name}: a second call changes nothing");
    }
}

#[test]
fn a_cleared_file_reads_as_a_plain_mp4_with_the_same_samples() {
    for (name, spec) in corpus() {
        let built = build(&spec, false);
        let (movie, table) = index(&built.bytes).unwrap();
        let mut cleared = built.bytes.clone();
        make_clear(&mut cleared, &movie, &table);
        let (clear_movie, clear_table) = index(&cleared).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(clear_movie.pssh.is_empty(), "{name}");
        assert!(
            clear_movie
                .tracks
                .iter()
                .all(|t| t.entries.iter().all(|e| e.protection.is_none())),
            "{name}"
        );
        let places: Vec<(usize, usize)> = table.iter().map(|s| (s.offset, s.size)).collect();
        let clear_places: Vec<(usize, usize)> =
            clear_table.iter().map(|s| (s.offset, s.size)).collect();
        assert_eq!(
            places, clear_places,
            "{name}: the samples stay where they were"
        );
        assert!(clear_table.iter().all(|s| !s.protected), "{name}");
    }
}

#[test]
fn clearing_piece_by_piece_never_touches_a_fragment_not_yet_indexed() {
    let spec = named("mdat_gap");
    let built = build(&spec, false);
    let expected = build(&spec, true).bytes;
    let second = built.fragments[1];
    let mut file = built.bytes.clone();
    let movie = Movie::read(&file)
        .unwrap()
        .expect("the whole file holds the moov");
    let mut indexer = Indexer::new(&movie);
    // The first fragment whole, the second's moof whole but its mdat cut.
    let ready = indexer.advance(&file[..file.len() - 10]).unwrap();
    assert_eq!(ready, second);
    make_clear(&mut file, &movie, indexer.samples());
    assert_eq!(
        &file[..second],
        &expected[..second],
        "the movie and the first fragment"
    );
    assert_eq!(
        &file[second..],
        &built.bytes[second..],
        "the second fragment untouched"
    );
    indexer.finish(&file).unwrap();
    make_clear(&mut file, &movie, indexer.samples());
    assert!(file == expected);
}

#[test]
fn a_buffer_that_does_not_match_the_movie_is_left_alone() {
    let built = build(&named("plain"), false);
    let (movie, table) = index(&built.bytes).unwrap();
    let mut other = vec![0xEE; built.bytes.len()];
    make_clear(&mut other, &movie, &table);
    assert!(other.iter().all(|b| *b == 0xEE));
    let mut short = built.bytes[..20].to_vec();
    make_clear(&mut short, &movie, &table);
    assert_eq!(short, &built.bytes[..20]);
}
