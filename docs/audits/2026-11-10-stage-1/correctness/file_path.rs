//! Belongs in `crates/desktop/src/lib.rs`, a `mod tests` of its own (there,
//! `file_path` is `super::file_path`).
//!
//! ```sh
//! cargo test -p desktop a_file_address_with_a_host_names_the_file_from_the_root
//! ```

use std::path::Path;

use desktop::file_path;

/// RFC 8089 lets a `file:` address name the host, `localhost` most often:
/// `file://localhost/music/a.flac` is `/music/a.flac`. `file_path`
/// (lib.rs:44-45) only strips `file://`, and gives the relative path
/// `localhost/music/a.flac`: `files_from_args` (mpris.rs:122) and `OpenUri`
/// (mpris.rs:568) then open a file relative to the running app's own
/// folder, which isn't there.
#[test]
fn a_file_address_with_a_host_names_the_file_from_the_root() {
    assert_eq!(
        file_path("file://localhost/music/a.flac").as_deref(),
        Some(Path::new("/music/a.flac"))
    );
    assert_eq!(
        file_path("file:///music/a.flac").as_deref(),
        Some(Path::new("/music/a.flac"))
    );
}
