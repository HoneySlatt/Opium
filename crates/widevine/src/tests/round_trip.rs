//! A decrypt's round trip through the host, as the caller sees it, measured with the fake CDM
//! (§11). It prints its figures and asserts no time; they mean something in a release build:
//!
//! ```sh
//! cargo test --release --lib round_trip -- --nocapture
//! ```

use std::ffi::c_void;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use super::fake::{Fake, unmask, well_behaved};
use super::{key, ready};
use crate::boundary::{DecryptInput, DecryptOutput, HOST_CALLS, SCHEME_CENC, Subsample};
use crate::{Protected, Scheme};

/// An AAC frame of Apple's shape: about 800 bytes, a clear header then the protected rest.
const SAMPLE_SIZE: usize = 800;
const RUNS: [(u32, u32); 1] = [(16, 784)];

fn iterations() -> usize {
    if cfg!(debug_assertions) {
        2_000
    } else {
        50_000
    }
}

/// The median and the 99th percentile.
fn summary(mut times: Vec<Duration>) -> (Duration, Duration) {
    times.sort_unstable();
    let at = |share: f64| times[((times.len() - 1) as f64 * share).round() as usize];
    (at(0.5), at(0.99))
}

#[test]
fn round_trip_of_a_decrypt_through_the_host() {
    let clear: Vec<u8> = (0..SAMPLE_SIZE).map(|index| index as u8).collect();
    let encrypted = unmask(&clear, &RUNS);
    let through_host = through_the_host(&encrypted, &clear);
    let direct = directly(&encrypted);
    let (host_median, host_p99) = summary(through_host);
    let (direct_median, direct_p99) = summary(direct);
    eprintln!(
        "round trip of an {SAMPLE_SIZE}-byte decrypt over {} runs ({} build):\n  \
         through the host: median {:.2} µs, p99 {:.2} µs\n  \
         the fake's decrypt called directly: median {:.2} µs, p99 {:.2} µs\n  \
         added by the host: median {:.2} µs",
        iterations(),
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        micros(host_median),
        micros(host_p99),
        micros(direct_median),
        micros(direct_p99),
        micros(host_median.saturating_sub(direct_median)),
    );
}

fn micros(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e6
}

/// Each decrypt timed from the caller's call to its return, through the CDM thread.
fn through_the_host(encrypted: &[u8], clear: &[u8]) -> Vec<Duration> {
    let fake = Fake::new(well_behaved());
    fake.quiet_decrypts.store(true, Ordering::Relaxed);
    let runtime = ready(&fake);
    let (session, _) = runtime.open_session(b"init").unwrap();
    runtime.give_licence(session, &key(1), &key(1)).unwrap();
    let sample = Protected {
        key_id: &key(1),
        iv: &[0; 16],
        subsamples: &RUNS,
        scheme: Scheme::Cenc,
    };
    let mut data = encrypted.to_vec();
    let mut times = Vec::with_capacity(iterations());
    for round in 0..iterations() + 500 {
        data.copy_from_slice(encrypted);
        let started = Instant::now();
        runtime.decrypt(session, &mut data, &sample).unwrap();
        let took = started.elapsed();
        // The first rounds warm the caches and the threads up.
        if round >= 500 {
            times.push(took);
        }
    }
    assert_eq!(data, clear);
    assert!(runtime.shutdown());
    times
}

/// Each decrypt of the fake called on this thread through its table: what the host adds is the
/// difference.
fn directly(encrypted: &[u8]) -> Vec<Duration> {
    let fake = Fake::new(well_behaved());
    fake.quiet_decrypts.store(true, Ordering::Relaxed);
    let library = fake.library();
    let calls = library.calls;
    // SAFETY: the fake's entry and table; the host pointer is never called back by a decrypt of
    // this script, and would be dropped unread if it were, as no inbox is this thread's.
    let instance = unsafe {
        (calls.create)(
            library.entry.0,
            c"x".as_ptr(),
            1,
            std::ptr::dangling_mut::<c_void>(),
            &HOST_CALLS,
        )
    };
    assert!(!instance.is_null());
    let runs: Vec<Subsample> = RUNS
        .iter()
        .map(|&(clear_bytes, cipher_bytes)| Subsample {
            clear_bytes,
            cipher_bytes,
        })
        .collect();
    let key_id = key(1);
    let iv = [0u8; 16];
    let mut data = encrypted.to_vec();
    let mut times = Vec::with_capacity(iterations());
    for round in 0..iterations() + 500 {
        data.copy_from_slice(encrypted);
        let started = Instant::now();
        let input = DecryptInput {
            data: data.as_ptr(),
            data_size: data.len() as u32,
            scheme: SCHEME_CENC,
            key_id: key_id.as_ptr(),
            key_id_size: 16,
            iv: iv.as_ptr(),
            iv_size: 16,
            subsamples: runs.as_ptr(),
            subsample_count: runs.len() as u32,
            crypt_byte_block: 0,
            skip_byte_block: 0,
            timestamp: 0,
        };
        let mut output = DecryptOutput {
            data: std::ptr::null(),
            size: 0,
            buffer: std::ptr::null_mut(),
        };
        // SAFETY: a live fake instance on this one thread; every pointer of the input is valid.
        let status = unsafe { (calls.decrypt)(instance, &input, &mut output) };
        assert_eq!(status, 0);
        // SAFETY: the output holds `size` bytes until its buffer is released, just after.
        unsafe {
            std::ptr::copy(output.data, data.as_mut_ptr(), output.size as usize);
            (calls.release_buffer)(output.buffer);
        }
        let took = started.elapsed();
        if round >= 500 {
            times.push(took);
        }
    }
    // SAFETY: the instance is destroyed once.
    unsafe { (calls.destroy)(instance) };
    times
}
