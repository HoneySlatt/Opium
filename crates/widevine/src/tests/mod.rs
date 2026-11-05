//! Tests that need the host's internals: the fake CDM behind the C boundary, the host's state
//! machine driven by it, the seeded sequences, the search for candidates, and the round trip's
//! measurement.

mod cpp_layer;
pub(crate) mod fake;
mod find;
mod host;
mod loader;
mod round_trip;
mod sequence;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::runtime::{Deadlines, Runtime};
use fake::Fake;

/// A folder of the test's own under the system's temporary folder, removed when dropped.
pub(crate) struct TestFolder {
    path: PathBuf,
}

impl TestFolder {
    pub(crate) fn new(name: &str) -> TestFolder {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let number = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "widevine-test-{}-{name}-{number}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the test folder can be made");
        TestFolder { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestFolder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Deadlines short enough that a test never waits long, long enough that a fake answering at
/// once always meets them.
pub(crate) const QUICK: Deadlines = Deadlines {
    initialise: Duration::from_millis(2_000),
    session_request: Duration::from_millis(2_000),
    licence: Duration::from_millis(2_000),
    close: Duration::from_millis(500),
    shutdown: Duration::from_millis(3_000),
};

/// A runtime over the fake, not initialised yet.
pub(crate) fn start(fake: &Arc<Fake>, deadlines: Deadlines) -> Runtime {
    Runtime::start(fake.library(), deadlines).expect("the CDM thread starts")
}

/// A runtime over the fake, initialised.
pub(crate) fn ready(fake: &Arc<Fake>) -> Runtime {
    let runtime = start(fake, QUICK);
    runtime.initialise().expect("the fake initialises");
    runtime
}

/// A key id made of one byte.
pub(crate) fn key(byte: u8) -> [u8; 16] {
    [byte; 16]
}

/// A seeded generator (SplitMix64), so that every run of a randomised test is the same.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        mixed ^ (mixed >> 31)
    }

    /// A number in `0..bound`; `bound` is not 0.
    pub(crate) fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    /// `true` `percent` times in a hundred.
    pub(crate) fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    pub(crate) fn byte(&mut self) -> u8 {
        self.next() as u8
    }
}
