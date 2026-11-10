// Written to fail at 46e7a5a, never compiled during the audit; its twin in ../edges/proofs or
// ../data/proofs ran and failed (see ../README.md).
// Belongs in: crates/synced/src/tests.rs (it uses that module's `device()`,
// `track()` and `history()`, and lib.rs's `now()`, `new_id()` and `Kept`).
// Run: cargo test -p synced a_copy_put_aside
// Fails at 46e7a5a: the phone's history holds songs 1000 to 1099 only. The
// same check, through public items, ran in
// .delegation/m18-data/scratch/tests/proofs.rs (proof 02.out, test p3).

/// ADR-0014: devices merge "without one device's copy overwriting another's
/// changes". A computer whose `shared.json` doesn't read starts again; the
/// history it was given from before the devices shared it (the apps'
/// `history.json`, never removed) must not come in as played now and push
/// the phone's own plays out of the 100 kept.
#[test]
fn a_copy_put_aside_doesnt_push_the_other_devices_history_out() {
    let hour_ago = now() - 3_600_000;
    let mut phone = device("phone");
    for n in 0..100 {
        phone.played(track(n), hour_ago + u64::from(n));
    }
    let dir = std::env::temp_dir().join(format!("synced-aside-{}", new_id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("shared.json"), b"{ damaged").unwrap();
    let computer = Kept::open(&dir, || (1000..1100).map(track).collect());
    phone.merge(computer.everything());
    computer.flush();
    drop(computer);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        history(&phone).first().map(String::as_str),
        Some("Song 99"),
        "the phone's last play is still its last"
    );
}

// Run: cargo test -p synced playlists_moving
// Fails at 46e7a5a (proof: scratch test p6): `keep` writes `shared.json`,
// now without the playlists, after `playlists.json` failed to be written.

/// "The playlists first: a power cut between the two leaves them in both
/// files." A write of `playlists.json` that fails (a full disk) must not
/// leave them in neither.
#[test]
fn playlists_moving_to_their_own_file_survive_a_write_that_fails() {
    let dir = std::env::temp_dir().join(format!("synced-move-fails-{}", new_id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut before = device("a");
    before.create_playlist("p", "Mix".into(), String::new(), vec![track(1)], 1000);
    let old = serde_json::json!({ "me": "a", "shared": before.shared });
    std::fs::write(dir.join("shared.json"), old.to_string()).unwrap();
    // As a full disk would: a folder stands where the new file's copy goes.
    std::fs::create_dir(dir.join("playlists.json.part")).unwrap();
    let kept = Kept::open(&dir, Vec::new);
    kept.flush();
    drop(kept);
    std::fs::remove_dir(dir.join("playlists.json.part")).unwrap();
    let again = Kept::open(&dir, Vec::new);
    let kept_on_disk = again.playlist("p").is_some();
    again.flush();
    drop(again);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(kept_on_disk, "the playlist is still in one of the files");
}
