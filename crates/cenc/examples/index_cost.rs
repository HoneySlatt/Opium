//! What indexing an Apple-shaped file costs: its time, the heap its result keeps, and the time a
//! download takes to index it as it comes.
//!
//! The file is synthetic, built here: AAC as Apple serves it, 90 samples a fragment, 8-byte IVs
//! and one subsample per sample. By default four minutes: 9 000 samples in 100 fragments, about
//! 7.4 MB; a number of fragments given makes it longer (6 000: four hours, 540 000 samples). Run it
//! in release:
//!
//! ```sh
//! cargo run --release -p cenc --example index_cost [fragments]
//! ```
//!
//! The figures go to `docs/measurements.md`, and the budget is in `docs/specs/fmp4-cenc.md` §10.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// The system allocator, counting the bytes and blocks alive.
struct Counting;

static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static LIVE_BLOCKS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call goes to the system allocator unchanged; the counters only observe it.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract is passed on as it is.
        let block = unsafe { System.alloc(layout) };
        if !block.is_null() {
            LIVE_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            LIVE_BLOCKS.fetch_add(1, Ordering::Relaxed);
        }
        block
    }

    unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract is passed on as it is.
        unsafe { System.dealloc(block, layout) };
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        LIVE_BLOCKS.fetch_sub(1, Ordering::Relaxed);
    }

    unsafe fn realloc(&self, block: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller's contract is passed on as it is.
        let moved = unsafe { System.realloc(block, layout, new_size) };
        if !moved.is_null() {
            LIVE_BYTES.fetch_add(new_size, Ordering::Relaxed);
            LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        }
        moved
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const FRAGMENTS: u32 = 100;
const SAMPLES_PER_FRAGMENT: u32 = 90;
/// How much a download brings at a time.
const STEP: usize = 64 * 1024;
const IV_SIZE: u8 = 8;
/// AAC's frame, in a 44.1 kHz timescale.
const FRAME: u32 = 1024;

/// The one call that depends on the crate's interface: index `file` whole, and say how many
/// samples were found. What it returns stays alive while the heap is counted.
fn index(file: &[u8]) -> (impl Sized, usize) {
    let indexed = cenc::index(file).expect("the synthetic file indexes");
    let count = indexed.1.len();
    (indexed, count)
}

/// The file fed to an `Indexer` [`STEP`] bytes at a time, made clear again after each step that
/// indexed a fragment, as `cenc::AudioStream` does while a track downloads: the time it all took,
/// and how many times the file was made clear.
fn download(file: &[u8]) -> (Duration, usize) {
    let movie = cenc::Movie::read(file)
        .expect("the synthetic file's movie")
        .expect("a whole movie box");
    let mut data = file.to_vec();
    let started = Instant::now();
    let mut indexer = cenc::Indexer::new(&movie);
    let (mut known, mut cleared) = (0, 0);
    for end in (STEP..data.len()).step_by(STEP).chain([data.len()]) {
        indexer
            .advance(&data[..end])
            .expect("the synthetic file indexes");
        if indexer.samples().len() > known {
            known = indexer.samples().len();
            cenc::make_clear(&mut data[..end], &movie, indexer.samples());
            cleared += 1;
        }
    }
    indexer
        .finish(&data)
        .expect("the synthetic file ends whole");
    (started.elapsed(), cleared)
}

fn main() {
    let fragments = std::env::args()
        .nth(1)
        .map_or(FRAGMENTS, |n| n.parse().expect("a number of fragments"));
    // Fewer runs of a long file: one takes a second's fraction already.
    let runs = if fragments > FRAGMENTS { 5 } else { 50 };
    let file = apple_shaped_file(fragments);
    let expected = (fragments * SAMPLES_PER_FRAGMENT) as usize;

    let before = (
        LIVE_BYTES.load(Ordering::Relaxed),
        LIVE_BLOCKS.load(Ordering::Relaxed),
    );
    let (kept, count) = index(&file);
    let bytes = LIVE_BYTES.load(Ordering::Relaxed) - before.0;
    let blocks = LIVE_BLOCKS.load(Ordering::Relaxed) - before.1;
    drop(kept);
    assert_eq!(count, expected, "every sample is indexed");

    let mut times: Vec<Duration> = (0..runs)
        .map(|_| {
            let started = Instant::now();
            black_box(index(black_box(&file)));
            started.elapsed()
        })
        .collect();
    times.sort();
    let (downloading, cleared) = download(&file);

    println!(
        "file: {:.2} MB, {count} samples in {fragments} fragments",
        file.len() as f64 / 1e6
    );
    println!(
        "index: best {:.3} ms, median {:.3} ms, of {runs} runs",
        times[0].as_secs_f64() * 1e3,
        times[runs / 2].as_secs_f64() * 1e3
    );
    println!(
        "kept: {:.1} KiB in {blocks} blocks, {:.1} bytes a sample",
        bytes as f64 / 1024.0,
        bytes as f64 / count as f64
    );
    println!(
        "download: {:.3} ms in all, {} KiB at a time, made clear {cleared} times",
        downloading.as_secs_f64() * 1e3,
        STEP / 1024
    );
}

/// A box: its 32-bit size, its type, its body.
fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let size = u32::try_from(8 + body.len()).expect("a small box");
    [&size.to_be_bytes()[..], kind, body].concat()
}

/// A full box: a box whose body starts with a version byte and 24 bits of flags.
fn full(kind: &[u8; 4], version: u8, flags: u32, body: &[u8]) -> Vec<u8> {
    let head = (u32::from(version) << 24 | flags).to_be_bytes();
    boxed(kind, &[&head[..], body].concat())
}

/// How long sample `n` of the file is: between 700 and 900 bytes, as AAC frames vary.
fn sample_size(n: u32) -> u32 {
    700 + n * 37 % 201
}

fn apple_shaped_file(fragments: u32) -> Vec<u8> {
    let mut file = boxed(b"ftyp", b"iso8\0\0\0\0iso8mp41");
    file.extend(movie());
    for fragment in 0..fragments {
        file.extend(fragment_bytes(fragment));
    }
    file
}

/// The movie box: one encrypted AAC track, scheme `cenc`, 8-byte IVs.
fn movie() -> Vec<u8> {
    let track_id = 1u32.to_be_bytes();
    let mut tkhd = vec![0; 8];
    tkhd.extend(track_id);
    tkhd.extend([0; 68]);
    let mut mdhd = vec![0; 8];
    mdhd.extend(44_100u32.to_be_bytes());
    mdhd.extend([0; 8]);

    let mut tenc = vec![0, 0, 1, IV_SIZE];
    tenc.extend([0x11; 16]);
    let sinf = boxed(
        b"sinf",
        &[
            boxed(b"frma", b"mp4a"),
            full(
                b"schm",
                0,
                0,
                &[b"cenc".as_slice(), &0x0001_0000u32.to_be_bytes()].concat(),
            ),
            boxed(b"schi", &full(b"tenc", 0, 0, &tenc)),
        ]
        .concat(),
    );
    // AAC-LC, 44.1 kHz, stereo.
    let audio_config = [0x12, 0x10];
    let mut decoder_config = vec![0x40, 0x15, 0, 0, 0, 0, 0x04, 0, 0, 0, 0x04, 0, 0];
    decoder_config.extend([0x05, audio_config.len() as u8]);
    decoder_config.extend(audio_config);
    let mut es = vec![0, 1, 0, 0x04, decoder_config.len() as u8];
    es.extend(decoder_config);
    es.extend([0x06, 1, 2]);
    let esds = full(b"esds", 0, 0, &[&[0x03, es.len() as u8][..], &es].concat());

    let mut enca = vec![0; 6];
    enca.extend(1u16.to_be_bytes());
    enca.extend([0; 8]);
    enca.extend(2u16.to_be_bytes());
    enca.extend(16u16.to_be_bytes());
    enca.extend([0; 4]);
    enca.extend((44_100u32 << 16).to_be_bytes());
    enca.extend(esds);
    enca.extend(sinf);
    let stsd = full(
        b"stsd",
        0,
        0,
        &[&1u32.to_be_bytes()[..], &boxed(b"enca", &enca)].concat(),
    );
    let stbl = boxed(b"stbl", &stsd);
    let mdia = boxed(
        b"mdia",
        &[full(b"mdhd", 0, 0, &mdhd), boxed(b"minf", &stbl)].concat(),
    );
    let trak = boxed(b"trak", &[full(b"tkhd", 0, 7, &tkhd), mdia].concat());

    let mut trex = track_id.to_vec();
    trex.extend(1u32.to_be_bytes());
    trex.extend(FRAME.to_be_bytes());
    trex.extend([0; 8]);
    let mvex = boxed(b"mvex", &full(b"trex", 0, 0, &trex));
    boxed(
        b"moov",
        &[full(b"mvhd", 0, 0, &[0; 96]), trak, mvex].concat(),
    )
}

/// Fragment `index`: a `moof` naming its samples' sizes and IVs, then their `mdat`.
fn fragment_bytes(index: u32) -> Vec<u8> {
    let first = index * SAMPLES_PER_FRAGMENT;
    let sizes: Vec<u32> = (first..first + SAMPLES_PER_FRAGMENT)
        .map(sample_size)
        .collect();

    // Track 1, default duration, its data counted from the `moof`.
    let mut tfhd = 1u32.to_be_bytes().to_vec();
    tfhd.extend(FRAME.to_be_bytes());
    let tfhd = full(b"tfhd", 0, 0x02_0008, &tfhd);
    let tfdt = full(b"tfdt", 1, 0, &(u64::from(first * FRAME)).to_be_bytes());

    let mut senc = SAMPLES_PER_FRAGMENT.to_be_bytes().to_vec();
    for (n, size) in (first..).zip(&sizes) {
        senc.extend(u64::from(n).to_be_bytes());
        senc.extend(1u16.to_be_bytes());
        senc.extend(16u16.to_be_bytes());
        senc.extend((size - 16).to_be_bytes());
    }
    let senc = full(b"senc", 0, 0x2, &senc);

    // The data offset is filled once the `moof`'s size is known.
    let mut trun = SAMPLES_PER_FRAGMENT.to_be_bytes().to_vec();
    trun.extend(0u32.to_be_bytes());
    for size in &sizes {
        trun.extend(size.to_be_bytes());
    }
    let trun = full(b"trun", 0, 0x0201, &trun);

    let mfhd = full(b"mfhd", 0, 0, &(index + 1).to_be_bytes());
    let traf = boxed(b"traf", &[tfhd, tfdt, trun, senc].concat());
    let mut moof = boxed(b"moof", &[mfhd, traf].concat());
    let data_offset = u32::try_from(moof.len() + 8).expect("a small moof");
    let at = moof
        .windows(4)
        .position(|w| w == b"trun")
        .expect("the trun")
        + 12;
    moof[at..at + 4].copy_from_slice(&data_offset.to_be_bytes());

    let audio: Vec<u8> = sizes
        .iter()
        .flat_map(|&size| (0..size).map(|b| b as u8))
        .collect();
    [moof, boxed(b"mdat", &audio)].concat()
}
