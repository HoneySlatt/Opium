//! Stage 1 audit, W8: an empty XDG variable counts as unset, never as a relative folder.

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
    assert_eq!(
        apple,
        home.join(".config/opium/apple-music"),
        "Apple Music's sign-in folder"
    );

    let cover = desktop::covers::file("https://example.invalid/cover.jpg")
        .expect("a cover has a file while HOME is set");
    assert!(
        cover.starts_with(home.join(".cache/opium/covers")),
        "a cover's file is {}",
        cover.display()
    );
}
