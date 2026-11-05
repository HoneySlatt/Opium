//! Thousands of small files made from the test files by damaging their `moov` and `moof` boxes
//! with a seeded generator: each indexed without a panic, the work bounded by the input's size.

use super::build::{Rng, broken, build, corpus, more_samples_than_bytes, mutate, top_level_boxes};
use super::stream_index;
use crate::index_counting;

/// How many boxes, samples and entries the parser may read per byte of input: every box header
/// takes 8 bytes of its container and nests at most 16 deep, every sample is justified by a byte,
/// every entry by two or more.
const STEPS_PER_BYTE: usize = 8;

fn seeds() -> Vec<Vec<u8>> {
    corpus()
        .iter()
        .map(|(_, spec)| build(spec, false).bytes)
        .chain(broken().into_iter().map(|(_, bytes)| bytes))
        .collect()
}

#[test]
fn thousands_of_damaged_files_index_or_fail_within_bounded_steps() {
    let mut rng = Rng::new(0x5EED_CAFE);
    let seeds = seeds();
    let mut indexed = 0;
    let mut total = 0;
    for round in 0..120 {
        for seed in &seeds {
            let mut file = mutate(&mut rng, seed);
            if round % 3 == 2 {
                file = mutate(&mut rng, &file);
            }
            total += 1;
            let (result, steps) = index_counting(&file);
            assert!(
                steps <= STEPS_PER_BYTE * file.len() + 64,
                "{steps} steps for {} bytes",
                file.len()
            );
            indexed += usize::from(result.is_ok());
        }
    }
    assert!(total >= 6000, "{total} files");
    assert!(indexed > 0, "some damage is harmless");
}

#[test]
fn damaged_files_index_the_same_whole_and_as_a_download() {
    let mut rng = Rng::new(0xD0_1D5);
    for seed in seeds() {
        for _ in 0..40 {
            let file = mutate(&mut rng, &seed);
            let whole = crate::index(&file);
            let streamed = stream_index(&file, 97);
            let moov_never_whole = whole.is_ok()
                && streamed
                    .as_ref()
                    .is_err_and(|e| e.kind == *b"moov" && e.what.starts_with("is missing"));
            // A top-level box of size 0 before or at the `moov` is whole only once the file is
            // (§2): a download never gets its movie, where the whole file does.
            if moov_never_whole {
                let open = top_level_boxes(&file)
                    .iter()
                    .take_while(|(start, _, kind)| {
                        kind != b"moov" || file[*start..*start + 4] == [0; 4]
                    })
                    .any(|(start, _, _)| file[*start..*start + 4] == [0; 4]);
                assert!(
                    open,
                    "a download misses the moov only when a box has size 0"
                );
                continue;
            }
            assert_eq!(whole, streamed);
        }
    }
}

#[test]
fn files_declaring_more_samples_than_their_bytes_fail_within_bounded_steps() {
    for (name, file) in more_samples_than_bytes() {
        let (result, steps) = index_counting(&file);
        assert!(result.is_err(), "{name} is an error");
        assert!(
            steps <= STEPS_PER_BYTE * file.len() + 64,
            "{name}: {steps} steps for {} bytes",
            file.len()
        );
    }
}
