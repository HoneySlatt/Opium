//! Each box layout of §2 to §5, on a file built in the test, indexed whole and as a download.

use super::build::{self, Built, KEY, OTHER_KEY, apple_shape, build, corpus, named};
use super::{as_expected, index_both_ways};
use crate::{Movie, Pattern, SampleTable, Scheme};

/// Builds and indexes the layout `name`; the samples found must be those built.
fn check(name: &str) -> (Built, Movie, SampleTable) {
    let built = build(&named(name), false);
    let (movie, table) =
        index_both_ways(&built.bytes, 37).unwrap_or_else(|e| panic!("{name} indexes: {e}"));
    let found: Vec<_> = table.iter().map(as_expected).collect();
    assert_eq!(found, built.samples, "{name}: the samples");
    assert_eq!(table.len(), built.samples.len());
    assert_eq!(table.is_empty(), built.samples.is_empty());
    assert_eq!(table.get(table.len()), None, "past the end");
    let fragments: Vec<usize> = {
        let mut starts: Vec<usize> = table.iter().map(|s| s.fragment).collect();
        starts.dedup();
        starts
    };
    assert_eq!(fragments, built.fragments, "{name}: the fragments");
    assert_eq!(
        movie.end,
        build::top_level_boxes(&built.bytes)[1].1,
        "{name}: moov end"
    );
    (built, movie, table)
}

#[test]
fn every_layout_indexes_the_same_whole_and_in_pieces_of_any_size() {
    for (name, spec) in corpus() {
        let built = build(&spec, false);
        for chunk in [1, 7, 64, 1000, built.bytes.len()] {
            let (_, table) = index_both_ways(&built.bytes, chunk)
                .unwrap_or_else(|e| panic!("{name} in pieces of {chunk}: {e}"));
            assert_eq!(
                table.len(),
                built.samples.len(),
                "{name} in pieces of {chunk}"
            );
        }
    }
}

#[test]
fn plain_file() {
    let (_, movie, _) = check("plain");
    assert_eq!(movie.tracks.len(), 1);
    let track = &movie.tracks[0];
    assert_eq!((track.id, track.timescale), (1, 44_100));
    assert_eq!(track.defaults.sample_description_index, 1);
    assert_eq!(track.defaults.sample_duration, 1024);
    let entry = &track.entries[0];
    assert_eq!(entry.format, *b"mp4a");
    assert_eq!(entry.audio_config, Some(vec![0x12, 0x10]));
    let protection = entry.protection.as_ref().expect("encrypted");
    assert_eq!(protection.scheme, Scheme::Cenc);
    assert_eq!(protection.key_id, KEY);
    assert_eq!(protection.iv_size, 16);
    assert_eq!(protection.constant_iv, None);
    assert_eq!(protection.pattern, Pattern { crypt: 0, skip: 0 });
    assert!(movie.pssh.is_empty());
}

#[test]
fn sizes_of_64_bits_size_0_and_uuid_boxes() {
    let (built, movie, _) = check("sizes64");
    assert_eq!(&built.bytes[movie.at..movie.at + 4], &1u32.to_be_bytes());
    assert_eq!(movie.tracks.len(), 1);
}

#[test]
fn two_tracks_and_two_trafs_in_a_moof() {
    let (_, movie, table) = check("two_tracks");
    assert_eq!(movie.tracks.len(), 2);
    assert_eq!(movie.tracks[1].entries[0].format, *b"mp4a");
    assert!(movie.tracks[1].entries[0].protection.is_none());
    assert_eq!(table.iter().filter(|s| !s.protected).count(), 3);
}

#[test]
fn two_truns_in_a_traf() {
    let (_, _, table) = check("two_runs");
    assert_eq!(table.len(), 5);
    assert_eq!(table.get(4).unwrap().duration, 500);
}

#[test]
fn a_base_data_offset_in_the_tfhd() {
    check("base_offset");
}

#[test]
fn an_mdat_not_right_after_its_moof() {
    check("mdat_gap");
}

#[test]
fn an_mdat_before_its_moof_through_a_negative_data_offset() {
    check("mdat_first");
}

#[test]
fn eight_byte_ivs_with_subsamples() {
    let (_, _, table) = check("iv8_subsamples");
    let sample = table.get(1).unwrap();
    assert_eq!(sample.iv_size, 8);
    assert_eq!(&sample.iv[8..], &[0; 8], "padded with zeros");
    assert_eq!(sample.subsamples, &[(11, 60), (5, 24)]);
}

#[test]
fn cbcs_with_a_constant_iv_and_a_pattern_and_no_senc() {
    let (_, movie, table) = check("cbcs_constant_iv");
    let protection = movie.tracks[0].entries[0].protection.as_ref().unwrap();
    assert_eq!(protection.scheme, Scheme::Cbcs);
    assert_eq!(protection.iv_size, 0);
    assert_eq!(protection.constant_iv, Some(vec![0x44; 16]));
    assert_eq!(protection.pattern, Pattern { crypt: 1, skip: 9 });
    let sample = table.get(0).unwrap();
    assert_eq!(sample.iv, [0x44; 16]);
    assert_eq!(sample.iv_size, 16);
    assert!(sample.subsamples.is_empty(), "protected whole");
}

#[test]
fn cbcs_with_a_constant_iv_and_a_senc_of_subsamples_only() {
    check("cbcs_senc");
}

#[test]
fn cbcs_with_a_constant_iv_and_an_empty_senc_as_apples_live_radio_sends() {
    let (_, _, table) = check("radio_empty_senc");
    assert_eq!(table.len(), 10);
    for sample in table.iter() {
        assert!(sample.protected);
        assert_eq!(sample.scheme, Scheme::Cbcs);
        assert_eq!(sample.pattern, Pattern { crypt: 1, skip: 9 });
        assert_eq!(sample.iv, [0x44; 16], "the constant IV");
        assert_eq!(sample.iv_size, 16);
        assert!(sample.subsamples.is_empty(), "protected whole");
    }
}

#[test]
fn saiz_and_saio_without_senc() {
    let (_, _, table) = check("saiz_saio");
    assert_eq!(table.get(0).unwrap().subsamples, &[(2, 48)]);
    assert!(table.get(1).unwrap().subsamples.is_empty());
}

#[test]
fn seig_groups_of_the_movie_and_of_the_fragment() {
    let (_, _, table) = check("seig");
    let samples: Vec<_> = table.iter().collect();
    assert!(samples[0].protected && samples[0].key_id == KEY);
    assert!(!samples[1].protected, "group 1 says clear");
    assert!(samples[2].protected && samples[2].key_id == OTHER_KEY && samples[2].iv_size == 8);
    assert_eq!(samples[3].key_id, [0x55; 16], "the fragment's own group");
    assert_eq!(samples[3].pattern, Pattern { crypt: 3, skip: 7 });
    assert_eq!(samples[4].key_id, KEY);
}

#[test]
fn seig_groups_of_a_version_0_sgpd() {
    let (_, _, table) = check("seig_version0");
    assert!(table.iter().all(|s| s.key_id == OTHER_KEY));
}

#[test]
fn several_sample_entries_for_spatial_audio() {
    let (_, movie, table) = check("spatial");
    assert_eq!(movie.tracks[0].entries.len(), 3);
    let entries: Vec<u32> = table.iter().map(|s| s.entry).collect();
    assert_eq!(entries, [0, 0, 1, 1, 2, 2]);
    assert_eq!(movie.pssh.len(), 1);
    assert_eq!(movie.pssh[0].key_ids, vec![KEY, OTHER_KEY]);
    assert_eq!(movie.pssh[0].data, vec![1, 2, 3]);
}

#[test]
fn an_unencrypted_mp4a_track() {
    let (_, movie, table) = check("clear");
    assert!(movie.tracks[0].entries[0].protection.is_none());
    assert!(table.iter().all(|s| !s.protected && s.iv_size == 0));
}

#[test]
fn trex_defaults_give_sizes_and_durations() {
    let (_, _, table) = check("trex_defaults");
    assert!(table.iter().all(|s| s.size == 48 && s.duration == 960));
}

#[test]
fn every_per_sample_field_in_a_trun() {
    let (_, _, table) = check("per_sample_fields");
    let durations: Vec<u32> = table.iter().map(|s| s.duration).collect();
    assert_eq!(durations, [1000, 1024, 300]);
}

#[test]
fn pssh_boxes_of_both_versions() {
    let (_, movie, _) = check("pssh");
    assert_eq!(movie.pssh.len(), 2);
    assert!(movie.pssh[0].key_ids.is_empty());
    assert_eq!(movie.pssh[0].data, vec![0xAB; 6]);
    assert_eq!(movie.pssh[1].key_ids, vec![KEY]);
}

#[test]
fn quicktime_sound_descriptions_of_versions_1_and_2() {
    let (_, movie, _) = check("quicktime_versions");
    assert!(movie.tracks[0].entries[0].protection.is_some());
    assert!(movie.tracks[1].entries[0].protection.is_some());
    assert_eq!(movie.tracks[1].entries[0].audio_config, None);
}

#[test]
fn track_headers_of_version_1() {
    let (_, movie, _) = check("version1_headers");
    assert_eq!((movie.tracks[0].id, movie.tracks[0].timescale), (1, 48_000));
}

#[test]
fn an_encrypted_video_entry_is_read_for_its_protection() {
    let (_, movie, table) = check("encv");
    let video = &movie.tracks[1].entries[0];
    assert_eq!(video.format, *b"avc1");
    assert!(video.protection.is_some());
    assert!(table.iter().all(|s| s.protected));
}

#[test]
fn an_entry_of_another_format_is_reported_by_its_type() {
    let (_, movie, _) = check("opaque_entry");
    let opus = &movie.tracks[1].entries[0];
    assert_eq!(opus.format, *b"Opus");
    assert!(opus.protection.is_none() && opus.audio_config.is_none());
}

#[test]
fn a_file_of_apples_shape_indexes_its_9000_samples_in_bounded_steps() {
    let built = build(&apple_shape(), false);
    let (result, steps) = crate::index_counting(&built.bytes);
    let (_, table) = result.expect("the file indexes");
    assert_eq!(table.len(), 9_000);
    let found: Vec<_> = table.iter().map(as_expected).collect();
    assert_eq!(found, built.samples);
    // Boxes, samples and subsample runs: a few per sample, far from one per byte.
    assert!(steps < 4 * 9_000 + 1_000, "{steps} steps");
    let (_, streamed) = super::stream_index(&built.bytes, 100_000).unwrap();
    assert_eq!(streamed, table);
}
