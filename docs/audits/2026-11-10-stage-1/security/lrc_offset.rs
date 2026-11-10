// Belongs in: crates/lyrics/src/lrc.rs, inside its `mod tests` (it calls `parse` through
// `use super::*`).
//
// Run: cargo test -p lyrics an_offset_at_the_end
//
// What it shows: `[offset:-9223372036854775808]` in an LRC text (a file's tags, a `.lrc` beside
// it, or LRCLIB's answer, which its users write) makes `at - shift` (lrc.rs:156) and
// `ms - shift` (lrc.rs:58) overflow: a panic in a debug build, which `cargo test` and
// `cargo run` are, and a wrapped time in a release build. Today the test fails with "attempt to
// subtract with overflow"; with `saturating_sub` it passes.

#[test]
fn an_offset_at_the_end_of_its_range_does_not_overflow() {
    for shift in [i64::MIN, i64::MAX] {
        let text = format!("[offset:{shift}]\n[00:01.00]<00:01.00>One <00:02.00>two\n[00:03.00]Three\n");
        let lyrics = parse(&text, "Song").expect("two timed lines");
        assert!(lyrics.synced);
        let starts: Vec<u128> = lyrics.lines.iter().map(|l| l.start.as_millis()).collect();
        assert!(starts.windows(2).all(|w| w[0] <= w[1]), "{starts:?}");
    }
}
