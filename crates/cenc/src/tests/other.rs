//! What else the callers ask (§8) and the error's shape (§9).

use super::build::{self, Entry, KEY, OTHER_KEY, RunSpec, build, named, offset_of, plain};
use crate::{Error, Movie, index, last_sample_duration, write_key_ids};

#[test]
fn the_last_samples_duration_comes_from_the_trun_else_the_tfhd() {
    let built = build(&named("per_sample_fields"), false);
    let fragment = &built.bytes[built.fragments[0]..];
    assert_eq!(
        last_sample_duration(fragment),
        Some(300),
        "the trun's entry"
    );

    let built = build(&named("plain"), false);
    let fragment = &built.bytes[built.fragments[1]..];
    assert_eq!(
        last_sample_duration(fragment),
        Some(1024),
        "the tfhd's default"
    );

    let built = build(&named("trex_defaults"), false);
    let fragment = &built.bytes[built.fragments[0]..];
    assert_eq!(last_sample_duration(fragment), None, "only the trex knows");

    assert_eq!(last_sample_duration(&build::ftyp()), None, "no moof");
    assert_eq!(last_sample_duration(&[]), None);
}

/// The `tfhd`'s default duration is read past the fields that may come before it: the base data
/// offset (8 bytes, flag 0x1, the `base_offset` layout) and the sample description index
/// (4 bytes, flag 0x2, the `spatial` layout). Apple Music's end padding, for gapless playback,
/// is computed from it (`last_sample_duration`'s caller in `apple-music`'s `playback.rs`). The
/// test before reads only layouts whose `tfhd` has neither.
///
/// Shown to catch `+=` replaced by `-=` where `tfhd_default_duration` skips the sample
/// description index (a miss before: no layout tested had one).
#[test]
fn the_tfhds_default_duration_is_read_past_its_optional_fields() {
    for name in ["base_offset", "spatial"] {
        let built = build(&named(name), false);
        for &at in &built.fragments {
            assert_eq!(
                last_sample_duration(&built.bytes[at..]),
                Some(1024),
                "{name}, the fragment at {at}"
            );
        }
    }
}

#[test]
fn key_ids_are_written_into_zero_tencs_entry_by_entry() {
    let spec = named("spatial");
    let mut init = build(&spec, false).bytes;
    let ids = [KEY, OTHER_KEY];
    write_key_ids(&mut init, &ids);
    let movie = Movie::read(&init).unwrap().unwrap();
    let keys: Vec<[u8; 16]> = movie.tracks[0]
        .entries
        .iter()
        .map(|e| e.protection.as_ref().unwrap().key_id)
        .collect();
    assert_eq!(
        keys,
        [KEY, OTHER_KEY, [0; 16]],
        "the third entry has no id to take"
    );

    let mut spec = plain(0, 0, 0);
    spec.movie.tracks[0].entries = vec![Entry::cenc(16), Entry::cenc(16).with_key([0; 16])];
    let mut init = build(&spec, false).bytes;
    write_key_ids(&mut init, &[[0xAA; 16], [0xBB; 16]]);
    let movie = Movie::read(&init).unwrap().unwrap();
    let keys: Vec<[u8; 16]> = movie.tracks[0]
        .entries
        .iter()
        .map(|e| e.protection.as_ref().unwrap().key_id)
        .collect();
    assert_eq!(keys, [KEY, [0xBB; 16]], "an entry with a key keeps it");

    let mut junk = vec![0x55; 40];
    write_key_ids(&mut junk, &[KEY]);
    assert!(junk.iter().all(|b| *b == 0x55), "nothing to write into");
}

#[test]
fn the_movie_is_reported_only_once_whole() {
    let built = build(&named("plain"), false);
    let moov_end = build::top_level_boxes(&built.bytes)[1].1;
    for cut in [0, 4, 8, 20, moov_end - 1] {
        assert_eq!(
            Movie::read(&built.bytes[..cut]).unwrap(),
            None,
            "cut at {cut}"
        );
    }
    let movie = Movie::read(&built.bytes[..moov_end])
        .unwrap()
        .expect("whole");
    assert_eq!(movie.end, moov_end);
    assert_eq!(Movie::read(&built.bytes).unwrap(), Some(movie));
}

#[test]
fn an_error_names_the_box_its_offset_and_what_was_wrong() {
    let error = Error::new(0x1a2b, *b"trun", "runs past its `traf`");
    assert_eq!(
        error.to_string(),
        "the `trun` at 0x1a2b runs past its `traf`"
    );
    let odd = Error::new(8, [0xFF, b'a', 0, b'\n'], "is odd");
    assert_eq!(odd.to_string(), "the `\\xffa\\x00\\n` at 0x8 is odd");
    let source: Box<dyn std::error::Error> = Box::new(error.clone());
    assert_eq!(source.to_string(), error.to_string());
}

#[test]
fn an_empty_file_and_a_file_of_only_an_ftyp() {
    assert_eq!(Movie::read(&[]).unwrap(), None);
    assert_eq!(index(&[]).unwrap_err().kind, *b"moov");
    let ftyp = build::ftyp();
    assert_eq!(Movie::read(&ftyp).unwrap(), None);
    assert_eq!(index(&ftyp).unwrap_err().offset, 0);
}

#[test]
fn the_indexer_stays_put_after_an_error_and_after_the_end() {
    let built = build(&named("plain"), false);
    let mut file = built.bytes.clone();
    let trun = offset_of(&file, b"trun");
    build::put_u32(&mut file, trun + 12, 0x00FF_FFFF);
    let movie = Movie::read(&file).unwrap().unwrap();
    let mut indexer = crate::Indexer::new(&movie);
    let first = indexer.advance(&file).unwrap_err();
    assert_eq!(indexer.advance(&file).unwrap_err(), first);
    assert!(indexer.samples().is_empty());

    let (movie, table) = index(&built.bytes).unwrap();
    let mut indexer = crate::Indexer::new(&movie);
    indexer.finish(&built.bytes).unwrap();
    assert_eq!(indexer.samples(), &table);
    assert_eq!(indexer.advance(&built.bytes).unwrap(), built.bytes.len());
    assert_eq!(indexer.samples(), &table);
}

#[test]
fn every_public_type_is_send_sync_and_debug() {
    fn check<T: Send + Sync + std::fmt::Debug>() {}
    check::<Movie>();
    check::<crate::Track>();
    check::<crate::TrackDefaults>();
    check::<crate::SampleEntry>();
    check::<crate::Protection>();
    check::<crate::Scheme>();
    check::<crate::Pattern>();
    check::<crate::Pssh>();
    check::<crate::Indexer>();
    check::<crate::SampleTable>();
    check::<crate::Sample<'static>>();
    check::<Error>();
}

#[test]
fn a_zero_size_sample_stays_within_the_file() {
    // Zero-size samples need no mdat, but a run of them placed past the file is truncated.
    let mut spec = plain(1, 1, 40);
    spec.fragments[0].trafs[0].runs.push(RunSpec {
        data_offset: false,
        per_sample_size: false,
        ..RunSpec::of(2, 0, 16)
    });
    let built = build(&spec, false);
    let (_, table) = index(&built.bytes).expect("zero-size samples after the data");
    assert_eq!(table.len(), 3);
    assert!(table.iter().all(|s| s.offset + s.size <= built.bytes.len()));

    let mut file = built.bytes.clone();
    let trun = offset_of(&file, b"trun");
    build::put_u32(&mut file, trun + 16, 0x7FFF_0000);
    let error = index(&file).unwrap_err();
    assert_eq!(&error.kind, b"moof");
    assert!(error.what.starts_with("is truncated"), "{error}");

    // Zero-size samples with no fields of their own are justified by their moof's bytes: the
    // same answer whole and as a download, whatever has arrived.
    let mut file = built.bytes.clone();
    // The first trun: an 8-byte header, version and flags, the count, the data offset, one size.
    // The second has no field per sample: its count is bounded by its moof alone.
    let second_trun = trun + 8 + 4 + 4 + 4 + 4;
    assert_eq!(&file[second_trun + 4..second_trun + 8], b"trun");
    build::put_u32(&mut file, second_trun + 12, 1024);
    let error = index(&file).unwrap_err();
    assert_eq!(&error.kind, b"trun", "{error}");
    assert!(
        error
            .what
            .contains("zero-size samples where its `moof` has bytes for"),
        "{error}"
    );
    assert_eq!(super::stream_index(&file, 1).unwrap_err(), error);
}

#[test]
fn stray_bytes_at_a_containers_end_are_the_same_error_whatever_follows() {
    let built = build(&named("plain"), false);
    let moov = build::top_level_boxes(&built.bytes)[1];
    // One byte more in the moov's size, taken from the moof that follows it.
    let mut file = built.bytes.clone();
    build::put_u32(&mut file, moov.0, (moov.1 - moov.0 + 1) as u32);
    let whole = index(&file).unwrap_err();
    assert_eq!(whole.what, "ends in 1 stray bytes", "{whole}");
    assert_eq!(super::stream_index(&file, 1).unwrap_err(), whole);
    assert_eq!(Movie::read(&file[..moov.1 + 1]).unwrap_err(), whole);
}

#[test]
fn a_fragments_samples_in_two_mdats() {
    // Two runs, the second's data moved into an mdat of its own after the first's: the samples
    // of one fragment are found in two mdats, one after the other.
    let mut spec = plain(1, 2, 40);
    spec.fragments[0].trafs[0].runs.push(RunSpec::of(1, 40, 16));
    let built = build(&spec, false);
    let (mdat, mdat_end, _) = *build::top_level_boxes(&built.bytes)
        .iter()
        .find(|(_, _, kind)| kind == b"mdat")
        .expect("the fragment's mdat");
    let split = mdat + 8 + 2 * 40;
    let mut file = built.bytes[..split].to_vec();
    file.extend(build::boxed(b"mdat", &built.bytes[split..mdat_end]));
    file.extend(&built.bytes[mdat_end..]);
    build::put_u32(&mut file, mdat, 8 + 2 * 40);
    // The first trun: an 8-byte header, version and flags, the count, the data offset, two
    // sizes. The second's data offset moves past the new mdat's header.
    let second_trun = offset_of(&file, b"trun") + 8 + 4 + 4 + 4 + 2 * 4;
    assert_eq!(&file[second_trun + 4..second_trun + 8], b"trun");
    let data_offset = second_trun + 16;
    let moved = u32::from_be_bytes(file[data_offset..data_offset + 4].try_into().unwrap()) + 8;
    build::put_u32(&mut file, data_offset, moved);

    let mut expected = built.samples.clone();
    expected[2].offset += 8;
    let (_, table) = super::index_both_ways(&file, 7).expect("samples in two mdats");
    let found: Vec<_> = table.iter().map(super::as_expected).collect();
    assert_eq!(found, expected);

    // Half-way into the second mdat's header, the third sample lies in neither.
    build::put_u32(&mut file, data_offset, moved - 4);
    let error = index(&file).unwrap_err();
    assert_eq!(&error.kind, b"moof", "{error}");
    assert!(
        error.what.contains("sample 2 at") && error.what.contains("outside every `mdat`"),
        "{error}"
    );
}
