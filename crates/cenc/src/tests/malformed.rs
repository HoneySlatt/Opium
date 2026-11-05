//! Each malformed case of §9 and §13: an error naming the box, never a panic.

use super::build::{broken, fragments_sharing_bytes, offset_of, shared_byte_runs, zero_size_runs};
use super::index_both_ways;
use crate::Error;

/// The error the broken file `name` gives.
fn error(name: &str) -> Error {
    let (_, bytes) = broken()
        .into_iter()
        .find(|(found, _)| *found == name)
        .unwrap_or_else(|| panic!("a broken file named {name}"));
    let error = index_both_ways(&bytes, 53)
        .err()
        .unwrap_or_else(|| panic!("{name} is an error"));
    let shown = error.to_string();
    assert!(
        shown.starts_with(&format!(
            "the `{}` at {:#x} ",
            error.kind.escape_ascii(),
            error.offset
        )),
        "{name}: {shown}"
    );
    error
}

/// `name` fails in box `kind`, found at its first occurrence in the file.
fn fails_in(name: &str, kind: &[u8; 4]) -> Error {
    let error = error(name);
    assert_eq!(&error.kind, kind, "{name}: {error}");
    let (_, bytes) = broken()
        .into_iter()
        .find(|(found, _)| *found == name)
        .unwrap();
    assert_eq!(error.offset, offset_of(&bytes, kind), "{name}: {error}");
    error
}

#[test]
fn every_broken_file_is_an_error() {
    for (name, bytes) in broken() {
        let error = index_both_ways(&bytes, 53).err();
        assert!(error.is_some(), "{name} is an error");
    }
}

#[test]
fn a_size_below_the_header() {
    let error = fails_in("size_below_header", b"tkhd");
    assert!(
        error.what.contains("less than its 8-byte header"),
        "{error}"
    );
    fails_in("top_level_size_below_header", b"ftyp");
}

#[test]
fn a_size_past_the_container() {
    let error = fails_in("size_past_container", b"tkhd");
    assert_eq!(error.what, "runs past its `trak`");
}

#[test]
fn nesting_17_deep() {
    let error = error("nesting_17");
    assert_eq!(&error.kind, b"trak");
    assert!(error.what.contains("deeper than 16 levels"), "{error}");
}

#[test]
fn counts_larger_than_the_bytes_left() {
    for (name, kind) in [
        ("stsd_count_huge", b"stsd"),
        ("trun_count_huge", b"trun"),
        ("sgpd_count_huge", b"sgpd"),
        ("sbgp_count_huge", b"sbgp"),
        ("subsample_count_huge", b"senc"),
    ] {
        let error = fails_in(name, kind);
        assert!(error.what.starts_with("declares"), "{name}: {error}");
    }
    let error = error("pssh_kid_count_huge");
    assert_eq!(&error.kind, b"pssh");
    assert!(error.what.contains("key ids"), "{error}");
}

#[test]
fn subsamples_not_adding_up_to_the_sample() {
    let error = fails_in("subsamples_dont_add_up", b"senc");
    assert!(error.what.contains("subsamples of 101 bytes"), "{error}");
}

#[test]
fn a_senc_whose_count_differs_from_the_trafs_samples() {
    let error = fails_in("senc_count_mismatch", b"senc");
    assert_eq!(error.what, "counts 4 samples where its `traf` has 3");
}

#[test]
fn a_senc_whose_length_does_not_fit_its_iv_size() {
    let error = fails_in("senc_too_long", b"senc");
    assert!(error.what.contains("does not fit their IV size"), "{error}");
    let error = fails_in("senc_too_short", b"senc");
    assert_eq!(error.what, "ends before its IV");
}

#[test]
fn an_8_byte_iv_under_cbcs() {
    let error = fails_in("iv8_under_cbcs", b"tenc");
    assert!(
        error
            .what
            .contains("8-byte per-sample IV under a CBC scheme"),
        "{error}"
    );
    let error = fails_in("constant_iv8_under_cbcs", b"tenc");
    assert!(
        error.what.contains("8-byte constant IV under a CBC scheme"),
        "{error}"
    );
    let error = fails_in("seig_iv8_under_cbcs", b"sbgp");
    assert!(
        error.what.contains("8-byte IV under a CBC scheme"),
        "{error}"
    );
}

#[test]
fn a_sample_outside_every_mdat() {
    let error = fails_in("sample_outside_mdat", b"moof");
    assert!(error.what.contains("outside every `mdat`"), "{error}");
    let error = fails_in("sample_past_file", b"moof");
    assert!(error.what.contains("is truncated"), "{error}");
    let error = fails_in("sample_before_file", b"trun");
    assert!(error.what.contains("before the file's start"), "{error}");
}

#[test]
fn a_fragment_naming_what_the_movie_lacks() {
    let error = fails_in("unknown_track", b"tfhd");
    assert_eq!(error.what, "names track 9, which the movie does not have");
    let error = fails_in("description_index_0", b"tfhd");
    assert_eq!(error.what, "names sample entry 0 of a track with 1 entries");
}

#[test]
fn an_entry_whose_protection_cannot_be_read() {
    let error = fails_in("unknown_scheme", b"schm");
    assert!(error.what.contains("`abcd`"), "{error}");
    let error = fails_in("enca_without_sinf", b"enca");
    assert_eq!(error.what, "has no `sinf`");
    let error = fails_in("tenc_unprotected", b"tenc");
    assert!(error.what.contains("not protected"), "{error}");
    let error = fails_in("tenc_iv_size_7", b"tenc");
    assert!(error.what.contains("IV size of 7"), "{error}");
}

#[test]
fn a_file_without_a_movie_box_or_cut_short() {
    let error = error("no_moov");
    assert_eq!((error.offset, error.kind), (0, *b"moov"));
    let error = fails_in("cut_moof", b"moof");
    assert!(error.what.starts_with("is cut short"), "{error}");
    let error = self::error("cut_mdat");
    assert_eq!(&error.kind, b"moof", "{error}");
    assert!(error.what.starts_with("is truncated"), "{error}");
    let error = self::error("open_moof_cut_mdat");
    assert_eq!(error.what, "runs past its `moof`", "{error}");
    let cut = self::error("largesize_cut");
    assert_eq!(&cut.kind, b"moov");
}

#[test]
fn ivs_missing_or_half_located() {
    let error = fails_in("missing_ivs", b"traf");
    assert!(error.what.contains("no `senc`"), "{error}");
    let error = fails_in("saiz_without_saio", b"saiz");
    assert_eq!(error.what, "has no `saio` beside it");
}

#[test]
fn an_sbgp_mapping_wrongly() {
    let error = fails_in("sbgp_maps_too_few", b"sbgp");
    assert!(error.what.starts_with("maps"), "{error}");
    let error = fails_in("group_index_0x10000", b"sbgp");
    assert!(error.what.contains("0x10000"), "{error}");
}

#[test]
fn samples_beyond_what_the_files_bytes_justify() {
    // Every run alone is within its bounds: the second one is where the samples together, of
    // one fragment or of two, are more than their bytes.
    for (name, what) in [
        (
            "zero_size_runs",
            "declares 3252 zero-size samples where its `moof` has bytes for 0 more",
        ),
        ("shared_byte_runs", "some of which another sample owns"),
        (
            "shared_byte_sized_runs",
            "some of which another sample owns",
        ),
        (
            "fragments_sharing_bytes",
            "some of which another sample owns",
        ),
    ] {
        let error = error(name);
        let (_, bytes) = broken()
            .into_iter()
            .find(|(found, _)| *found == name)
            .unwrap();
        let second_trun = bytes
            .windows(4)
            .enumerate()
            .filter(|(_, window)| *window == b"trun")
            .nth(1)
            .expect("a second trun")
            .0
            - 4;
        assert_eq!(
            (error.offset, &error.kind),
            (second_trun, b"trun"),
            "{name}: {error}"
        );
        assert!(error.what.contains(what), "{name}: {error}");
    }
}

#[test]
fn samples_that_their_bytes_justify_exactly_are_indexed() {
    // As many zero-size samples as the moof has bytes; samples owning every byte of their mdat.
    let file = zero_size_runs(1);
    let (_, table) = index_both_ways(&file, 53).expect("one run within its moof");
    assert_eq!(table.len(), 68);
    let (_, table) = index_both_ways(&shared_byte_runs(1, 1000), 53).expect("one run");
    assert_eq!(table.len(), 1000);
    let (_, table) = index_both_ways(&fragments_sharing_bytes(1, 1000), 53).expect("one");
    assert_eq!(table.len(), 1000);
}
