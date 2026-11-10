// Belongs in: crates/desktop/tests/settings_folder_not_utf8.rs (a new
// integration test, alone in its binary as it sets XDG_CONFIG_HOME).
// Run: cargo test -p desktop --test settings_folder_not_utf8
// Fails at 46e7a5a: `Settings::save` drops the error of `toml::to_string`,
// which refuses a path that isn't UTF-8 ("path contains invalid UTF-8
// characters", proof 02.out, test p4), so nothing is written again.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

use desktop::settings::Settings;

#[test]
fn settings_are_kept_with_a_music_folder_whose_name_isnt_utf8() {
    let dir = std::env::temp_dir().join(format!("settings-latin1-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // SAFETY: this binary holds this one test, so no other thread reads or
    // writes the environment meanwhile.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", &dir) };
    let mut settings = Settings {
        volume: Some(0.5),
        ..Settings::default()
    };
    settings.save();
    assert_eq!(Settings::load().volume, Some(0.5));
    // A folder from an old disk, named in Latin-1, as the folder chooser's
    // `file:///music/caf%E9` decodes (`desktop::file_path`).
    settings
        .local_folders
        .push(PathBuf::from(OsStr::from_bytes(b"/music/caf\xe9")));
    settings.volume = Some(0.25);
    settings.save();
    let back = Settings::load();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(back.volume, Some(0.25), "the change made after is kept");
    assert_eq!(back.local_folders, settings.local_folders, "and so is the folder");
}
