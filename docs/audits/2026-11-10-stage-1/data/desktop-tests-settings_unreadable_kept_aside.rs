// Belongs in: crates/desktop/tests/settings_unreadable_kept_aside.rs (a new
// integration test, alone in its binary as it sets XDG_CONFIG_HOME).
// Run: cargo test -p desktop --test settings_unreadable_kept_aside
// Fails at 46e7a5a: `Settings::load` takes any error of `read_to_string`
// (here `InvalidData`, proof 02.out, test p4) for a missing file, keeps no
// `.bad` copy, and the next save writes the defaults over the user's file.

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
    assert_eq!(loaded.volume, Some(0.3), "and every setting that reads is kept");
}
