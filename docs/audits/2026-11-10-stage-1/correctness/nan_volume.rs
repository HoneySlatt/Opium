// Belongs in `crates/player/src/output.rs`, `mod tests` (it calls the private
// `fill` and `OutputState::new`, as the tests there do). Plays nothing.
//
//     cargo test -p player a_volume_that_is_not_a_number_never_amplifies
//
// The agent ran the same computation through a harness taking `fill` and its
// helpers from output.rs as they are (correctness/nan_volume_harness, proof 27.out):
// from a source at 0.5, the loudest sample after a second was 97.6.

    /// `settings.toml` may say `volume = nan`, which TOML reads; `App::new`
    /// clamps it (app.rs:1035) and `Player::set_volume` clamps it again
    /// (lib.rs:352), and a clamp keeps NaN. In `fill`, the gain then never
    /// equals its target and `max` ignores the NaN (output.rs:173): the gain
    /// walks down past zero by one each 5 ms, and the samples come out
    /// inverted and ever louder, past full scale within 10 ms.
    #[test]
    fn a_volume_that_is_not_a_number_never_amplifies() {
        let (mut producer, mut consumer) = rtrb::RingBuffer::new(4096);
        for _ in 0..4000 {
            producer.push(0.5).unwrap();
        }
        let state = OutputState::new(f32::NAN);
        let mut gain = 1.0;
        let mut out = [0.0; 4000];
        fill(&mut consumer, &state, &mut gain, &mut out, 1, 48_000);
        let loudest = out.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!(loudest <= 0.5, "loudest sample {loudest} from a source at 0.5; gain now {gain}");
    }
