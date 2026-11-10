// Belongs in: crates/cenc/src/tests/other.rs (module `cenc::tests::other`), beside
// `the_last_samples_duration_comes_from_the_trun_else_the_tfhd`. Uses that file's imports.
//
// Passes at 46e7a5a (run in a copy of the crate: proof `.delegation/m18-tests/` NN.out files
// named in the report).

/// The `tfhd`'s default duration is read past the fields that may come before it: the base data
/// offset (8 bytes, flag 0x1, the `base_offset` layout) and the sample description index
/// (4 bytes, flag 0x2, the `spatial` layout). Apple Music's end padding, for gapless playback,
/// is computed from it (`apple-music/src/playback.rs:665`). The test before reads only layouts
/// whose `tfhd` has neither.
///
/// Kills `clear.rs:303:12 replace += with -= in tfhd_default_duration` (a clean miss: the
/// description index's 4 bytes, the `spatial` layout) and `303:12 … with *=`; and pins the base
/// data offset's skip at 300:12, whose mutants the audit's run recorded caught from builds of
/// other mutants.
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
