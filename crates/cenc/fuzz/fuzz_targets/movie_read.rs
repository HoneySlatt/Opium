//! `Movie::read` on any bytes: no panic, whatever they hold.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(Some(movie)) = cenc::Movie::read(data) {
        assert!(movie.end <= data.len());
        // The whole movie read again from a copy gives the same.
        let again = cenc::Movie::read(&data[..movie.end]).ok().flatten();
        assert_eq!(again.as_ref(), Some(&movie));
    }
});
