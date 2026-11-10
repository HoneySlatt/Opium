// Written to fail at 46e7a5a, never compiled during the audit; its twin in ../edges/proofs or
// ../data/proofs ran and failed (see ../README.md).
// Belongs in crates/player/src/output.rs, `mod tests`. Fails at 46e7a5a:
// a NaN target makes `read_into`'s `(gain - step).max(target)` return
// `gain - step` (output.rs:169-175), so from a pause the gain falls without
// bound; at a track's start (output.rs:392-396) it is NaN itself. A replica
// of the step failed in run 01 (gain -199.96 after one second at 48 kHz).

    #[test]
    fn a_volume_that_isnt_a_number_never_makes_it_louder() {
        for start in [0.0, f32::NAN] {
            let (mut producer, mut consumer) = rtrb::RingBuffer::new(4096);
            for _ in 0..4000 {
                producer.push(0.5).unwrap();
            }
            let state = OutputState::new(f32::NAN);
            let mut gain = start;
            let mut out = [0.0; 4000];
            fill(&mut consumer, &state, &mut gain, &mut out, 1, 48_000);
            assert!(
                out.iter().all(|s| s.is_finite() && s.abs() <= 0.5),
                "from {start}: gain {gain}, last sample {}",
                out[3999]
            );
        }
    }
