//! A catalog song's media playlist: no panic; a file read has a key id of 16 bytes, which
//! its key's `URI` holds in standard base64 after its first comma, a `URI` the playlist
//! holds as it is, and an absolute address.

#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

/// Where the playlist is, for its map to resolve against: Apple's form, with a query.
const PLAYLIST_URL: &str = "https://aod.example/a/b/p.m3u8?x=1";

fuzz_target!(|data: &[u8]| {
    let Some(file) = apple_music::fuzzing::catalog_file(PLAYLIST_URL, data) else {
        return;
    };
    let (_, encoded) = file
        .key_uri
        .split_once(',')
        .expect("a comma in the key's URI");
    assert_eq!(encoded, common::base64(&file.key_id), "{:?}", file.key_uri);
    assert!(
        data.windows(file.key_uri.len())
            .any(|w| w == file.key_uri.as_bytes()),
        "the key's URI as the playlist writes it"
    );
    let (scheme, _) = file.url.split_once(':').expect("an absolute address");
    assert!(
        scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.')),
        "{}",
        file.url
    );
});
