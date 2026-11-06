//! The next page's address, from Apple's `next` and the first page's request, and the page
//! at an offset: whatever the text, no panic, and a path that keeps the request on the API's
//! host when put after its address.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // `next`, a zero byte, then the first request; the offset from the input's length.
    let text = String::from_utf8_lossy(data);
    let (next, first) = text.split_once('\0').unwrap_or((&text, ""));
    let offset = data.len().saturating_mul(100);
    for path in [
        apple_music::fuzzing::next_page_path(next, first),
        apple_music::fuzzing::at_offset(next, offset, first),
    ] {
        // After `https://amp-api.music.apple.com`, a `/` ends the host: nothing after it can
        // name another one.
        assert!(path.starts_with('/'), "{path:?}");
    }
    let _ = apple_music::fuzzing::offset_in(next);
});
