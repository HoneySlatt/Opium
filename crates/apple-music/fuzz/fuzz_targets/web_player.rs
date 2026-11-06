//! The web player's page and main script: whatever the text, no panic; the script named is
//! a path of the main script's form, with a query that holds no quote, space, `<` or `>`, as
//! the page writes it, and is asked of the web player's own host, at `/assets/` and one
//! segment, whatever the page writes before it; the token found is three base64url segments
//! whose header names ES256, as the script writes it, not inside a longer run.

#![no_main]

use libfuzzer_sys::fuzz_target;

fn is_base64url(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

/// Whether `path` is `/assets/index~<name>.js` or `/assets/index-<name>.js`, the name one or
/// more characters but `/`, `?`, `#`, a quote, `<`, `>` or an ASCII space.
fn is_main_script(path: &str) -> bool {
    path.strip_prefix("/assets/index")
        .and_then(|rest| rest.strip_prefix(['~', '-']))
        .and_then(|rest| rest.strip_suffix(".js"))
        .is_some_and(|name| {
            !name.is_empty()
                && !name.contains(['/', '?', '#', '"', '\'', '<', '>'])
                && !name.contains(|c: char| c.is_ascii_whitespace())
        })
}

fuzz_target!(|data: &[u8]| {
    // As the web player's answers are read: invalid UTF-8 replaced.
    let text = String::from_utf8_lossy(data);
    if let Some(script) = apple_music::fuzzing::main_script(&text) {
        assert!(text.contains(script));
        let (path, query) = script.split_once('?').unwrap_or((script, ""));
        assert!(is_main_script(path), "{script:?}");
        assert!(
            !query.contains(['"', '\'', '<', '>'])
                && !query.contains(|c: char| c.is_ascii_whitespace()),
            "{script:?}"
        );
        let (host, path, _) =
            apple_music::fuzzing::script_url(script).expect("an address that parses");
        assert_eq!(host, "music.apple.com", "{script:?}");
        let file = path.strip_prefix("/assets/").expect("in /assets/");
        assert!(file.starts_with("index") && !file.contains('/'), "{script:?}");
    }
    if let Some(token) = apple_music::fuzzing::first_token(&text) {
        let segments: Vec<&str> = token.split('.').collect();
        assert_eq!(segments.len(), 3, "{token:?}");
        assert!(
            segments
                .iter()
                .all(|s| !s.is_empty() && s.bytes().all(is_base64url)),
            "{token:?}"
        );
        assert!(apple_music::fuzzing::names_es256(segments[0]));
        let at = text.find(token).expect("the token as the script writes it");
        // The first place it is written may be inside a run: then a later one is not.
        let alone = text
            .match_indices(token)
            .any(|(at, _)| at == 0 || !is_base64url(text.as_bytes()[at - 1]));
        assert!(alone, "{token:?} at {at}");
        let _ = apple_music::auth::expires_at(token);
    }
    let _ = apple_music::auth::expires_at(&text);
});
