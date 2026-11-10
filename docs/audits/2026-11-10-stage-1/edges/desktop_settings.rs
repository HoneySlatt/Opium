// Written to fail at 46e7a5a, never compiled during the audit; its twin in ../edges/proofs or
// ../data/proofs ran and failed (see ../README.md).
// Belongs in crates/desktop/src/settings.rs, `mod tests`. Fails at 46e7a5a:
// `toml::to_string` refuses a path that isn't UTF-8 ("path contains invalid
// UTF-8 characters", run 01), and `Settings::save` (settings.rs:188-193)
// then writes nothing, for that change and every one after it.

    #[test]
    fn a_folder_whose_name_isnt_utf8_doesnt_stop_the_settings_being_kept() {
        use std::os::unix::ffi::OsStringExt;
        let kept = Settings {
            volume: Some(0.3),
            local_folders: vec![std::ffi::OsString::from_vec(b"/music/caf\xe9".to_vec()).into()],
            ..Settings::default()
        };
        assert!(toml::to_string(&kept).is_ok());
    }
