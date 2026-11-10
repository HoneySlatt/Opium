// For crates/player/src/lib.rs, inside `mod tests` (it uses that module's `Stalled` and the
// crate's `STALLED`). Run: `cargo test -p player dropping_a_player_stuck_on_a_read`.
//
// Expected today: PASSES. It shows that docs/debt.md:58, "a hang on closing during a stuck read"
// (left by the 2026-10-31 audit), no longer holds: since 6b82fe9 (2026-11-04) `Player`'s `Drop`
// waits 500 ms for its engine, then lets it go (crates/player/src/lib.rs:399-410).

#[test]
fn dropping_a_player_stuck_on_a_read_returns_at_once() {
    let (wake, stalled) = std::sync::mpsc::channel();
    let player = Player::new();
    player.play(
        1,
        Source::new(Stalled(Mutex::new(stalled)), 1_000_000, Some("flac")),
    );
    std::thread::sleep(STALLED + Duration::from_millis(200));
    assert!(player.stalled());
    let started = Instant::now();
    drop(player);
    // 500 ms of grace, then the engine is let go of: well under a second.
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "closing waited {:?} on a read that never ends",
        started.elapsed()
    );
    // The read given up at last: the engine, let go of, ends on its own.
    drop(wake);
}
