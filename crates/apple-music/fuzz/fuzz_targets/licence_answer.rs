//! The licence server's answer: no panic on any body; a licence given is never empty; and an
//! answer built from the input, with an `errorCode` of some form and a licence of its bytes,
//! is read as §5 says, without any error carrying the licence.

#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

/// The forms of `errorCode` an answer is built with, and whether each refuses the licence.
const ERROR_CODES: [(Option<&str>, bool); 10] = [
    (None, false),
    (Some("0"), false),
    (Some("0.0"), false),
    (Some("-0"), false),
    (Some("3"), true),
    (Some("-1"), true),
    (Some("1.5"), true),
    (Some("\"3\""), false),
    (Some("null"), false),
    (Some("true"), false),
];

fuzz_target!(|data: &[u8]| {
    if let Ok(licence) = apple_music::fuzzing::licence(data) {
        assert!(!licence.is_empty());
    }
    let Some((&form, licence)) = data.split_first() else {
        return;
    };
    let (code, refused) = ERROR_CODES[usize::from(form) % ERROR_CODES.len()];
    let encoded = common::base64(licence);
    let answer = match code {
        Some(code) => format!("{{\"errorCode\":{code},\"license\":\"{encoded}\"}}"),
        None => format!("{{\"license\":\"{encoded}\"}}"),
    };
    match apple_music::fuzzing::licence(answer.as_bytes()) {
        Ok(read) => {
            assert!(!refused, "{answer}");
            assert_eq!(read, licence);
        }
        Err(e) => {
            assert!(refused || licence.is_empty(), "{answer}: {e}");
            if refused {
                assert!(e.contains("errorCode"), "{e}");
            }
            if encoded.len() >= 8 {
                assert!(!e.contains(&encoded), "no licence in an error");
            }
        }
    }
});
