//! What the parser allocates, counted by a global allocator of this test's own: no input makes it
//! allocate more than the input justifies (§9), and the samples' table keeps at most 48 bytes a
//! sample on a file of Apple's shape (§10).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

#[path = "../src/tests/build.rs"]
mod build;

use build::{Rng, apple_shape, broken, build, corpus, more_samples_than_bytes, mutate};

/// The system allocator, counting the bytes live and the most live at once.
struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call is forwarded to the system allocator unchanged; the counters are plain
// atomics touched beside it, so the allocator's own contract (unique, correctly sized blocks)
// is the system's.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the layout is the caller's, forwarded as is.
        let block = unsafe { System.alloc(layout) };
        if !block.is_null() {
            grow(layout.size());
        }
        block
    }

    unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
        // SAFETY: the block came from `alloc` or `realloc` above with this layout.
        unsafe { System.dealloc(block, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the block came from this allocator with `layout`; `new_size` is the caller's.
        let moved = unsafe { System.realloc(block, layout, new_size) };
        if !moved.is_null() {
            LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
            grow(new_size);
        }
        moved
    }
}

fn grow(bytes: usize) {
    let live = LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes;
    PEAK.fetch_max(live, Ordering::Relaxed);
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn live() -> usize {
    LIVE.load(Ordering::Relaxed)
}

/// The counters are shared by every test of this binary: each measures alone, holding this.
static MEASURING: Mutex<()> = Mutex::new(());

fn measuring() -> MutexGuard<'static, ()> {
    MEASURING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Runs `work` and tells how many bytes it had live at most beyond what was live before it.
fn peak_during<T>(work: impl FnOnce() -> T) -> (T, usize) {
    let before = live();
    PEAK.store(before, Ordering::Relaxed);
    let result = work();
    (result, PEAK.load(Ordering::Relaxed) - before)
}

/// The budget of §10: at most 48 bytes a sample kept by the table, capacity included.
const BYTES_PER_SAMPLE: usize = 48;

/// What the parser may have live at most, per byte of input, while it works: the table at under
/// 48 bytes a sample where a sample is justified by one byte, its growth by doubling, and the
/// scratch of one fragment.
const PEAK_PER_INPUT_BYTE: usize = 160;

#[test]
fn the_table_keeps_at_most_48_bytes_a_sample_on_a_file_of_apples_shape() {
    let _alone = measuring();
    let built = build(&apple_shape(), false);
    let file = built.bytes;
    assert_eq!(file.len() / 1_000_000, 7, "about 7 MB");
    assert_eq!(built.fragments.len(), 100);
    let before = live();
    let (movie, table) = cenc::index(&file).expect("the file indexes");
    drop(movie);
    let kept = live() - before;
    assert_eq!(table.len(), 9_000);
    assert_eq!(table.len(), built.samples.len());
    let per_sample = kept as f64 / table.len() as f64;
    eprintln!(
        "the table keeps {kept} bytes for {} samples: {per_sample:.2} a sample",
        table.len()
    );
    assert!(
        kept <= BYTES_PER_SAMPLE * table.len(),
        "{per_sample:.2} bytes a sample, over the budget of {BYTES_PER_SAMPLE}"
    );
    assert!(
        table
            .iter()
            .all(|s| s.iv_size == 8 && s.subsamples.len() == 1)
    );
}

#[test]
fn the_table_of_a_download_keeps_the_same_once_finished() {
    let _alone = measuring();
    let file = build(&apple_shape(), false).bytes;
    let before = live();
    let movie = cenc::Movie::read(&file).unwrap().unwrap();
    let mut indexer = cenc::Indexer::new(&movie);
    for end in (0..file.len()).step_by(65_536).skip(1) {
        indexer.advance(&file[..end]).unwrap();
    }
    indexer.finish(&file).unwrap();
    let kept = live() - before;
    let samples = indexer.samples().len();
    assert_eq!(samples, 9_000);
    eprintln!(
        "after a download, the indexer holds {kept} bytes: {:.2} a sample",
        kept as f64 / samples as f64
    );
    // The indexer itself holds the movie's tracks and the mdat list beside the table.
    assert!(kept <= BYTES_PER_SAMPLE * samples + 4096);
}

#[test]
fn no_input_makes_the_parser_allocate_beyond_what_it_justifies() {
    let _alone = measuring();
    let mut rng = Rng::new(0xA110_CA7E);
    let mut seeds: Vec<Vec<u8>> = corpus()
        .iter()
        .map(|(_, spec)| build(spec, false).bytes)
        .collect();
    seeds.extend(broken().into_iter().map(|(_, bytes)| bytes));
    let mut worst = 0.0f64;
    for seed in &seeds {
        let (_, peak) = peak_during(|| cenc::index(seed));
        let ratio = peak as f64 / seed.len() as f64;
        worst = worst.max(ratio);
        assert!(
            peak <= PEAK_PER_INPUT_BYTE * seed.len() + 4096,
            "{peak} bytes for {} of input",
            seed.len()
        );
        for _ in 0..60 {
            let file = mutate(&mut rng, seed);
            let (_, peak) = peak_during(|| cenc::index(&file));
            worst = worst.max(peak as f64 / file.len().max(1) as f64);
            assert!(
                peak <= PEAK_PER_INPUT_BYTE * file.len() + 4096,
                "{peak} bytes live for {} bytes of input",
                file.len()
            );
        }
    }
    eprintln!("at most {worst:.1} bytes live per byte of input");
}

#[test]
fn a_count_is_checked_before_anything_is_reserved() {
    let _alone = measuring();
    // A trun of a billion samples in a few bytes, a stsd of four billion entries, a senc of
    // 65535 subsamples: none may reserve for them.
    let mut files = Vec::new();
    for name in [
        "trun_count_huge",
        "stsd_count_huge",
        "subsample_count_huge",
        "sgpd_count_huge",
        "pssh_kid_count_huge",
        "sbgp_count_huge",
    ] {
        let (_, bytes) = broken()
            .into_iter()
            .find(|(found, _)| *found == name)
            .unwrap();
        files.push((name, bytes));
    }
    for (name, bytes) in files {
        let (result, peak) = peak_during(|| cenc::index(&bytes));
        assert!(result.is_err(), "{name}");
        assert!(
            peak <= 8 * bytes.len() + 1024,
            "{name}: {peak} bytes for {} of input",
            bytes.len()
        );
    }
}

#[test]
fn a_dense_first_fragment_before_a_large_box_reserves_within_the_file_and_keeps_none_of_it() {
    let _alone = measuring();
    // One fragment of 200 one-byte samples, then a megabyte the indexer skips: at the first
    // fragment's density, the rest of the file would hold tens of thousands of samples more.
    let mut spec = build::plain(1, 200, 1);
    spec.trailing = build::boxed(b"free", &vec![0; 1 << 20]);
    let file = build(&spec, false).bytes;
    let before = live();
    let (indexed, peak) = peak_during(|| cenc::index(&file).expect("the file indexes"));
    let (movie, table) = indexed;
    drop(movie);
    let kept = live() - before;
    assert_eq!(table.len(), 200);
    assert!(
        peak <= BYTES_PER_SAMPLE * file.len(),
        "{peak} bytes live for {} of input: more than a sample in every byte",
        file.len()
    );
    // The table's descriptions and fragments besides its samples.
    assert!(
        kept <= BYTES_PER_SAMPLE * table.len() + 1024,
        "{kept} bytes kept for {} samples",
        table.len()
    );
}

#[test]
fn samples_declared_beyond_the_files_bytes_never_grow_the_table_beyond_the_file() {
    let _alone = measuring();
    // Runs of zero-size samples past their moof's bytes, runs and fragments whose samples share
    // the same bytes: each run alone looks justified, together they are not.
    let mut worst = 0.0f64;
    for (name, file) in more_samples_than_bytes() {
        let (result, peak) = peak_during(|| cenc::index(&file));
        assert!(result.is_err(), "{name} is an error");
        worst = worst.max(peak as f64 / file.len() as f64);
        assert!(
            peak <= PEAK_PER_INPUT_BYTE * file.len() + 4096,
            "{name}: {peak} bytes live for {} bytes of input",
            file.len()
        );
    }
    eprintln!("at most {worst:.1} bytes live per byte of input");
}
