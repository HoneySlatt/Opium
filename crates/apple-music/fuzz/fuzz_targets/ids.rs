//! The ids a song, a change or a station may carry, and the `webPlayback` body: whatever the
//! text, no panic; an id is of a kind only in that kind's form; the body names the id by its
//! kind and nothing else; an id that goes into an address unescaped holds nothing that could
//! leave its path segment.

#![no_main]

use libfuzzer_sys::fuzz_target;

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// A dot's left side `prefix_ok`, its right side one or more ASCII letters, digits or `-`.
fn is_prefixed(id: &str, prefix_ok: impl Fn(&str) -> bool) -> bool {
    id.split_once('.').is_some_and(|(prefix, name)| {
        prefix_ok(prefix)
            && !name.is_empty()
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    })
}

fuzz_target!(|data: &[u8]| {
    let Ok(id) = std::str::from_utf8(data) else {
        return;
    };
    let kind = apple_music::fuzzing::id_kind(id);
    match kind {
        Some("catalog") => assert!(is_digits(id), "{id:?}"),
        Some("library") => assert!(
            is_prefixed(id, |p| matches!(p, "i" | "l" | "p" | "a")),
            "{id:?}"
        ),
        Some(other) => panic!("no such kind: {other}"),
        None => assert!(!is_digits(id)),
    }
    match apple_music::fuzzing::playback_request(id) {
        Some(body) => {
            let key = if kind == Some("catalog") {
                "salableAdamId"
            } else {
                "universalLibraryId"
            };
            assert!(kind.is_some(), "{id:?}");
            // The id needs no escaping in JSON: it is digits, letters, dots and dashes.
            assert_eq!(body, format!("{{\"{key}\":\"{id}\"}}"));
        }
        None => assert_eq!(kind, None, "{id:?}"),
    }
    if apple_music::fuzzing::apple_id(id) {
        let lowercase =
            |p: &str| (1..=3).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_lowercase());
        assert!(is_digits(id) || is_prefixed(id, lowercase), "{id:?}");
    }
    if let Some(station) = apple_music::fuzzing::station_id(id) {
        assert_eq!(station, id);
        assert!(!id.is_empty() && id != "." && id != "..", "{id:?}");
        assert!(
            id.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_')),
            "{id:?}"
        );
    }
});
