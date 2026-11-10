//! A harness for the agent's own run, not for the tree: `fill`, `Ramp`,
//! `read_into` and `drop_cut` are taken as they are from
//! `crates/player/src/output.rs` (lines 63-194, cut out by `build.sh` at each
//! run), beside an `OutputState` holding only the fields `fill` reads.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use rtrb::Consumer;

#[derive(Default)]
pub(crate) struct OutputState {
    pub played: AtomicU64,
    pub flush: AtomicBool,
    pub cut_at: AtomicU64,
    pub cut_len: AtomicU64,
    pub paused: AtomicBool,
    pub volume: AtomicU32,
}

impl OutputState {
    pub fn new(volume: f32) -> Self {
        Self {
            volume: AtomicU32::new(volume.to_bits()),
            cut_at: AtomicU64::new(u64::MAX),
            ..Self::default()
        }
    }
}

include!("output_fill.rs");

fn main() {
    for (name, volume) in [("0.5", 0.5_f32), ("NaN", f32::NAN)] {
        let (mut producer, mut consumer) = rtrb::RingBuffer::new(48_000);
        for _ in 0..48_000 {
            producer.push(0.5).unwrap();
        }
        let state = OutputState::new(volume);
        let mut gain = 1.0;
        // One second of mono at 48 kHz, in PipeWire-sized periods of 1024.
        let mut loudest = 0.0_f32;
        let mut last = 0.0;
        for _ in 0..46 {
            let mut out = [0.0; 1024];
            fill(&mut consumer, &state, &mut gain, &mut out, 1, 48_000);
            loudest = out.iter().fold(loudest, |m, s| m.max(s.abs()));
            last = out[1023];
        }
        println!("volume {name}: source 0.5, loudest sample out {loudest}, last {last}, gain {gain}");
    }
}
