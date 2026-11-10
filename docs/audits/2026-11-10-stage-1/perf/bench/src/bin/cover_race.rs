//! M1.8 performance audit: the scan's workers making one album's embedded cover at once.
//!
//! `keep_cover` below is `crates/local/src/scan.rs:319-330` as it is, with one counter added
//! just before `square`, to count the covers made. `scan` (`scan.rs:204-225`) reads up to 8
//! files at once, in folder order: the first eight files of an album, each with the same
//! embedded picture, reach `keep_cover` within a millisecond of each other, as the barrier
//! here makes them.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::time::Instant;

static MADE: AtomicUsize = AtomicUsize::new(0);

/// `crates/local/src/scan.rs:319-330`, plus the counter.
pub fn keep_cover(bytes: &[u8], covers: &Path) -> Option<String> {
    let path = covers.join(format!("{:016x}.jpg", fnv(bytes)));
    if !path.exists() {
        MADE.fetch_add(1, Ordering::Relaxed);
        let square = opium_core::cover::square(bytes)?;
        std::fs::create_dir_all(covers).ok()?;
        let partial = path.with_extension("part");
        std::fs::write(&partial, square).ok()?;
        std::fs::rename(&partial, &path).ok()?;
    }
    Some(path.to_string_lossy().into_owned())
}

/// `crates/local/src/scan.rs:366-371`.
pub fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ *b as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// A `side` by `side` RGB PNG, its pixels a gradient set by `seed`, stored without
/// compression (deflate's stored blocks), so that no image library is needed to make it.
fn png(side: u32, seed: u8) -> Vec<u8> {
    fn crc(bytes: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for &b in bytes {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
        }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend(crc(&body).to_be_bytes());
    }
    let mut raw = Vec::with_capacity((side * (side * 3 + 1)) as usize);
    for y in 0..side {
        raw.push(0); // no filter
        for x in 0..side {
            raw.extend([
                (x * 255 / side) as u8 ^ seed,
                (y * 255 / side) as u8,
                seed.wrapping_mul(37),
            ]);
        }
    }
    let mut zlib = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65_535).enumerate() {
        let last = (i + 1) * 65_535 >= raw.len();
        zlib.push(u8::from(last));
        zlib.extend((block.len() as u16).to_le_bytes());
        zlib.extend((!(block.len() as u16)).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    zlib.extend(((b << 16) | a).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = side.to_be_bytes().to_vec();
    header.extend(side.to_be_bytes());
    header.extend([8, 2, 0, 0, 0]); // 8 bits, RGB
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &zlib);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn main() {
    let root = std::env::args().nth(1).expect("a folder to make covers in");
    let rounds = 20;
    let workers = 8;
    let mut lost = 0;
    let mut made = 0;
    let mut alone_ms = Vec::new();
    let mut together_ms = Vec::new();
    for round in 0..rounds {
        let picture = Arc::new(png(1200, round as u8));
        // Once alone, to time one cover made.
        let alone = Path::new(&root).join(format!("alone-{round}"));
        let start = Instant::now();
        assert!(keep_cover(&picture, &alone).is_some(), "a cover made alone");
        alone_ms.push(start.elapsed().as_secs_f64() * 1e3);
        MADE.store(0, Ordering::Relaxed);

        // Then by eight workers at once, as the scan does with an album's first eight files.
        let covers = Arc::new(Path::new(&root).join(format!("round-{round}")));
        let barrier = Arc::new(Barrier::new(workers));
        let start = Instant::now();
        let results: Vec<Option<String>> = (0..workers)
            .map(|_| {
                let (picture, covers, barrier) = (picture.clone(), covers.clone(), barrier.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    keep_cover(&picture, &covers)
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|t| t.join().expect("a worker"))
            .collect();
        together_ms.push(start.elapsed().as_secs_f64() * 1e3);
        made += MADE.load(Ordering::Relaxed);
        lost += results.iter().filter(|r| r.is_none()).count();
    }
    alone_ms.sort_by(f64::total_cmp);
    together_ms.sort_by(f64::total_cmp);
    println!(
        "{rounds} albums, {workers} files each at once: {made} covers made where {rounds} would do ({:.1} an album); {lost} of {} files left with no cover (keep_cover gave None)",
        made as f64 / rounds as f64,
        rounds * workers
    );
    println!(
        "one cover made alone: median {:.1} ms; eight at once, wall time: median {:.1} ms",
        alone_ms[rounds / 2],
        together_ms[rounds / 2]
    );
}
