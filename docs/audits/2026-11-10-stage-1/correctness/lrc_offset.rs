//! Belongs in `crates/lyrics/src/lrc.rs`, `mod tests` (there, `parse` is
//! `super::parse`).
//!
//! ```sh
//! cargo test -p lyrics lrc::tests::an_offset_far_out_of_range_neither_panics_nor_wraps
//! ```

use lyrics::lrc::parse;

/// LRCLIB's lyrics are sent in by anyone. `[offset:…]` is read as any i64
/// (`offset`, lrc.rs:102-113) and taken from each time with a plain `-`
/// (lrc.rs:156, and lrc.rs:58 for a word's time): far out of range, it
/// overflows. Debug and test builds panic ("attempt to subtract with
/// overflow"); release builds, which keep no overflow checks
/// (Cargo.toml has none), wrap, and the line starts at 0 or far away.
#[test]
fn an_offset_far_out_of_range_neither_panics_nor_wraps() {
    let text = "[offset:-9223372036854775808]\n[00:01.00]one\n[00:02.00]two <00:02.50>words\n";
    let lyrics = parse(text, "").expect("two lines");
    assert_eq!(lyrics.lines.len(), 2);
}
