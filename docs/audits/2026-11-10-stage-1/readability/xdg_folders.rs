//! For `crates/desktop/tests/xdg_folders.rs` (a new integration test of the `desktop` crate).
//!
//! The XDG Base Directory specification: "If $XDG_CONFIG_HOME is either not set or empty, a
//! default equal to $HOME/.config should be used", and likewise $XDG_CACHE_HOME with
//! $HOME/.cache. `widevine::find` follows it (crates/widevine/src/find.rs:81-87); the seven
//! hand-written copies in `desktop` and `app-desktop` do not: an empty variable becomes the
//! relative folder `opium/…`, under whatever folder Opium was started from.
//!
//! Fails today: `settings::dir()` is `opium`, `apple_music_dir()` is `opium/apple-music`, and a
//! cover's file is `opium/covers/…`.
//!
//! This binary holds this one test, so that changing the environment races no other test.

use std::path::Path;

#[test]
fn an_empty_xdg_variable_counts_as_unset() {
    // SAFETY: this test binary holds this one test, and nothing it calls before these lines
    // starts a thread, so no other thread reads or writes the environment meanwhile.
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", "");
        std::env::set_var("XDG_CACHE_HOME", "");
        std::env::set_var("HOME", "/home/someone");
    }
    let home = Path::new("/home/someone");

    let settings = desktop::settings::dir();
    assert_eq!(settings, home.join(".config/opium"), "the settings' folder");

    let apple = desktop::backend::apple_music_dir();
    assert_eq!(apple, home.join(".config/opium/apple-music"), "Apple Music's sign-in folder");

    let cover = desktop::covers::file("https://example.invalid/cover.jpg")
        .expect("a cover has a file while HOME is set");
    assert!(
        cover.starts_with(home.join(".cache/opium/covers")),
        "a cover's file is {}",
        cover.display()
    );
}
