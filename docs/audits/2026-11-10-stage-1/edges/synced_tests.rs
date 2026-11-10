// Written to fail at 46e7a5a, never compiled during the audit; its twin in ../edges/proofs or
// ../data/proofs ran and failed (see ../README.md).
// Belongs in crates/synced/src/tests.rs. Fails at 46e7a5a: `Kept::with_cover`
// (lib.rs:631-655) joins a playlist id another device chose to the covers'
// folder. Proved in run 01: `escaped-87158c578f96a781.jpg` was written
// beside `covers/`, not in it.

#[test]
fn a_playlist_id_from_another_device_stays_in_the_covers_folder() {
    let dir = std::env::temp_dir().join(format!("synced-{:016x}", rand::random::<u64>()));
    let kept = Kept::open(&dir, Vec::new);
    let theirs: Shared = serde_json::from_value(serde_json::json!({
        "playlists": {"lists": {"../escaped": {
            "name": {"at": {"at": 1, "device": "x"}, "value": "P"},
            "cover": {"at": {"at": 1, "device": "x"}, "value": "AAAA"}
        }}}
    }))
    .unwrap();
    assert!(kept.merge(theirs));
    let _ = kept.playlists();
    let outside: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("escaped-"))
        .collect();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(outside.is_empty(), "written outside covers/: {outside:?}");
}
