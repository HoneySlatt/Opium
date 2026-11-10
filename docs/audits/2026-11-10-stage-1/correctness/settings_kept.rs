//! Belongs in `crates/desktop/tests/settings_kept.rs` (an integration test of
//! `desktop`, its own process, so that setting `XDG_CONFIG_HOME` reaches no
//! other test). Writes only under its own folder in `std::env::temp_dir()`.
//!
//! ```sh
//! cargo test -p desktop --test settings_kept
//! ```

use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

use desktop::settings::Settings;

/// Settings kept in a folder of this test's own, read back.
fn profile(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("opium-settings-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("opium")).expect("the test's folder");
    // SAFETY: this binary's tests run one after another (`one_at_a_time`), and
    // nothing else in the process reads the environment meanwhile.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", &dir) };
    dir
}

#[test]
fn one_at_a_time() {
    let failed: Vec<&str> = [
        (
            "settings_with_a_folder_not_named_in_utf8_are_kept",
            settings_with_a_folder_not_named_in_utf8_are_kept as fn(),
        ),
        (
            "settings_that_are_not_utf8_are_kept_beside_before_being_replaced",
            settings_that_are_not_utf8_are_kept_beside_before_being_replaced,
        ),
    ]
    .into_iter()
    .filter(|(_, check)| std::panic::catch_unwind(check).is_err())
    .map(|(name, _)| name)
    .collect();
    assert!(failed.is_empty(), "failed: {failed:?}");
}

/// The portal hands over any folder, a Latin-1 name from an old disk
/// included (`app.rs` `pick_local_folders`, through `desktop::file_path`).
/// serde refuses to write a path that isn't UTF-8, and `Settings::save`
/// drops the error: nothing is written, that time and every time after,
/// while the folder stays listed.
fn settings_with_a_folder_not_named_in_utf8_are_kept() {
    let dir = profile("latin1");
    let folder = PathBuf::from(std::ffi::OsString::from_vec(b"/music/C\xe9line Dion".to_vec()));
    let settings = Settings {
        local_folders: vec![folder],
        volume: Some(0.3),
        ..Settings::default()
    };
    settings.save();
    let back = Settings::load();
    let written = dir.join("opium/settings.toml").exists();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        written && back.volume == Some(0.3),
        "save() wrote nothing (file written: {written}); the volume read back is {:?}",
        back.volume
    );
}

/// A file that doesn't parse is kept beside as `settings.toml.bad`
/// (`Settings::load`); one that isn't UTF-8 at all fails `read_to_string`
/// first, loads as the defaults, and the next save writes them over it,
/// with no copy kept.
fn settings_that_are_not_utf8_are_kept_beside_before_being_replaced() {
    let dir = profile("bytes");
    let file = dir.join("opium/settings.toml");
    std::fs::write(&file, b"volume = 0.3\ntheme = \"m\xe9\"\n").expect("the settings written");
    let loaded = Settings::load();
    let kept_beside = dir.join("opium/settings.toml.bad").exists();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        kept_beside,
        "loaded as the defaults (volume {:?}) and kept nowhere: the next save replaces the file",
        loaded.volume
    );
}
