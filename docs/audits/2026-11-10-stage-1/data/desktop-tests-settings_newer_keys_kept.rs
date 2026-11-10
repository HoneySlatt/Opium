// Belongs in: crates/desktop/tests/settings_newer_keys_kept.rs (a new
// integration test, alone in its binary as it sets XDG_CONFIG_HOME).
// Run: cargo test -p desktop --test settings_newer_keys_kept
// Fails at 46e7a5a: `settings.toml` has no version, `Settings` ignores the
// keys it doesn't know, and `save` writes only those it knows, with no
// `.bad` copy since the file read: a build that goes back (a Nix or Flatpak
// rollback) silently drops what a newer one kept there.

use desktop::settings::Settings;

#[test]
fn settings_a_newer_build_kept_survive_an_older_one() {
    let dir = std::env::temp_dir().join(format!("settings-newer-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("opium")).unwrap();
    // SAFETY: this binary holds this one test, so no other thread reads or
    // writes the environment meanwhile.
    unsafe { std::env::set_var("XDG_CONFIG_HOME", &dir) };
    let file = dir.join("opium/settings.toml");
    std::fs::write(&file, "volume = 0.3\nsleep_timer_minutes = 30\n").unwrap();
    let mut settings = Settings::load();
    assert_eq!(settings.volume, Some(0.3));
    settings.shuffle = Some(true);
    settings.save();
    let text = std::fs::read_to_string(&file).unwrap();
    let aside = dir.join("opium/settings.toml.bad").exists();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        text.contains("sleep_timer_minutes = 30") || aside,
        "the newer build's setting is kept, or the file it was in; settings.toml now: {text:?}"
    );
}
