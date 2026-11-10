//! Stage 1 audit, L3: a settings file that is not UTF-8 is kept beside before anything replaces it.

use desktop::settings::Settings;

#[test]
fn a_settings_file_that_isnt_utf8_is_kept_beside() {
    let dir = std::env::temp_dir().join(format!("settings-bytes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("opium")).unwrap();
    // SAFETY: this binary holds this one test, so no other thread reads or
    // writes the environment meanwhile.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", &dir) };
    let file = dir.join("opium/settings.toml");
    // One byte that isn't UTF-8, in a value typed by hand or damaged.
    let text: &[u8] = b"volume = 0.3\nshuffle = true\ntheme = \"caf\xe9\"\n";
    std::fs::write(&file, text).unwrap();
    let loaded = Settings::load();
    loaded.save();
    let bad = std::fs::read(dir.join("opium/settings.toml.bad")).ok();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        bad.as_deref(),
        Some(text),
        "Settings::load and CONTRIBUTING.md (Settings care): a file that does not read is kept beside as settings.toml.bad"
    );
    assert_eq!(
        loaded.volume,
        Some(0.3),
        "and every setting that reads is kept"
    );
}
