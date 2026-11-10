// For crates/local/src/tag_writes.rs, inside `mod tests` (it uses that module's `folder`,
// `cut_short`, `open_rw` and `id_of`). Run:
// `cargo test -p local a_write_a_power_cut_left_with_zeros_past_the_old_end_is_put_back`.
//
// Expected today: FAILS at the last assert_eq. It shows the data loss that docs/debt.md:33-34
// states in passing ("a song a power cut left with blocks of zeros is taken for one, and stays
// damaged"): `state_of` (tag_writes.rs:327-356) calls a byte that is neither copy's `Unknown`, so
// `undo_tag_write` (tag_writes.rs:211-230, its `State::Unknown` arm at :223-226) leaves the half-written song as it is and moves its
// original to `<id>.kept` in the app's private folder, which nothing shows the user.
//
// The module's own MP3 and WAV are silence, all zeros past their headers, so zeros left by a
// power cut match both copies there and the existing tests cannot see this: the file below
// carries non-zero audio, and no 0xFF byte that could pass for a frame's sync.

/// Ten frames of MPEG-1 Layer III, as `scan::tests::mp3`, but with audio that is not silence.
fn loud_mp3() -> Vec<u8> {
    let mut bytes = Vec::new();
    for frame in 0..10u32 {
        bytes.extend([0xFF, 0xFB, 0x90, 0x00]);
        bytes.extend((0..417u32 - 4).map(|i| 1 + ((i + frame) % 200) as u8));
    }
    bytes
}

#[test]
fn a_write_a_power_cut_left_with_zeros_past_the_old_end_is_put_back() {
    let dir = folder("tags-power-cut");
    let path = dir.join("song.mp3");
    let original = loud_mp3();
    fs::write(&path, &original).unwrap();
    let work = dir.join("work");
    // The phone's replace stopped halfway: the edited copy's first half, the original's rest.
    let copies = cut_short(&path, &work, "Cut", original.len() / 2);
    let edited = fs::read(&copies.edited).unwrap();
    // The ID3v2 tag goes first: the edited copy is longer and its audio sits further on.
    assert!(edited.len() > original.len());
    // The power cut: the file's new length reached the disk, the blocks past its old end did
    // not, and read back as zeros.
    let mut on_disk = fs::read(&path).unwrap();
    on_disk.resize(edited.len(), 0);
    fs::write(&path, &on_disk).unwrap();
    let id = id_of(&copies);
    undo_tag_write(&mut open_rw(&path), &work, &id).unwrap();
    assert_eq!(
        fs::read(&path).unwrap(),
        original,
        "the song is left half written; its original is only in {id}.kept"
    );
    fs::remove_dir_all(dir).unwrap();
}
