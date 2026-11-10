// Belongs in: crates/synced/src/tests.rs (module `synced::tests`), appended at its end. It uses
// that file's helpers (`device`, `track`, `titles`, `history`, `songs`) and `use super::*`.
//
// Each test passes at 46e7a5a and kills the mutants named above it (cargo-mutants' names).

/// The apps take a song out of one of Opium's playlists by the entries `entries_of` gives
/// (`desktop/src/backend.rs:586`, `opium-ffi/src/lib.rs:898`), and the removal is final on every
/// device: they must be the song's own entries, each time it is in, and no other.
///
/// Kills `playlists.rs:101:36 replace == with != in Playlist::entries_of` (which would take out
/// every other song, for good), `playlists.rs:99:9 … with vec!["xyzzy".into()]` and
/// `… with vec![String::new()]`.
#[test]
fn taking_a_song_out_takes_its_entries_and_no_others() {
    let mut a = device("a");
    a.create_playlist(
        "p",
        "Mix".into(),
        String::new(),
        vec![track(1), track(2), track(1)],
        1000,
    );
    let playlist = a.shared.playlists.get("p").unwrap();
    let entries = playlist.entries_of(&track(1).id);
    let its_own: Vec<String> = playlist
        .songs
        .iter()
        .filter(|s| s.track.id == track(1).id)
        .map(|s| s.entry.clone())
        .collect();
    assert_eq!(its_own.len(), 2, "the song is in twice");
    assert_eq!(entries, its_own);
    a.remove_from_playlist("p", &entries);
    assert_eq!(songs(&a, "p"), ["Song 2"]);
}

/// `playlists.json` with a song of a service this build no longer knows: that song goes, the
/// playlist and its other songs stay, and the file is not put aside as unreadable. Without the
/// second reading, every Opium playlist would be gone from the app, and the next change to any
/// playlist would write the file over without them.
///
/// Kills `lib.rs:329:5 replace playlists_without_unknown_services -> Option<Playlists> with None`
/// and `… with Some(Default::default())` (both clean misses: no test reads such a file).
#[test]
fn a_playlist_with_a_song_of_a_service_taken_out_keeps_the_rest() {
    let dir = std::env::temp_dir().join(format!("synced-unknown-lists-{}", new_id()));
    let id = {
        let kept = Kept::open(&dir, Vec::new);
        let (id, _) = kept.create_playlist("Mix".into(), String::new(), vec![track(1), track(2)]);
        kept.flush();
        id
    };
    let path = dir.join("playlists.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let gone = text.replace(
        r#""provider":"AppleMusic","id":"2""#,
        r#""provider":"Gone","id":"2""#,
    );
    assert_ne!(gone, text);
    std::fs::write(&path, gone).unwrap();
    let kept = Kept::open(&dir, Vec::new);
    let playlist = kept.playlist(&id).expect("the playlist is still there");
    let titles: Vec<String> = playlist.songs.into_iter().map(|s| s.track.title).collect();
    assert_eq!(titles, ["Song 1"]);
    assert!(!dir.join("playlists.json.bad").exists());
    drop(kept);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A change made here is not empty: Opium Connect sends a change only when it isn't
/// (`connect/src/lib.rs:683`), and every test so far asserts emptiness only.
///
/// Kills `lib.rs:181:9 replace Shared::is_empty -> bool with true`.
#[test]
fn a_change_made_here_is_something_to_send() {
    let mut a = device("a");
    let (change, changed) = a.played(track(1), 1000);
    assert!(changed);
    assert!(!change.is_empty());
    assert!(Shared::default().is_empty());
}

/// A setting heard of from another device is a change: merged, it is written to disk
/// (`Kept::merge` saves only on a change), else it is lost at the next start.
///
/// Kills `lib.rs:200:22 replace |= with &= in Shared::merge_parts`.
#[test]
fn a_setting_heard_of_counts_as_a_change_and_once_only() {
    let mut here = Shared::default();
    let theirs = Shared {
        settings: BTreeMap::from([(
            "relay".to_owned(),
            Latest {
                at: Stamp {
                    at: 1000,
                    device: "b".into(),
                },
                value: "new".to_owned(),
            },
        )]),
        ..Shared::default()
    };
    assert!(here.merge(theirs.clone()), "a setting heard of is a change");
    assert!(!here.merge(theirs), "heard again, it is none");
}

/// Times another device says, believed a few hours ahead of this clock (a clock a little fast,
/// a timezone's worth) and not two days ahead, in both units the shared state keeps. The test
/// before (`a_device_whose_clock_is_wrong_doesnt_pin_the_times_everywhere`) checks 1970 and
/// years ahead only, so the day's room itself was never checked.
///
/// Kills the eight `plausible` mutants: `lib.rs:66:27` and `66:32` (`*` with `+` or `/`, the
/// day's room), `68:26` (`/` with `%` or `*`, now in seconds) and `70:19` (`*` with `+` or `/`,
/// the year 2000 in milliseconds).
#[test]
fn a_time_hours_ahead_is_believed_and_one_days_ahead_is_not() {
    let now_ms = now();
    let now_s = now_ms / 1000;
    let hours = 6 * 60 * 60;
    let days = 2 * 24 * 60 * 60;
    assert!(plausible(now_s + hours, true));
    assert!(plausible(now_ms + hours * 1000, false));
    assert!(!plausible(now_s + days, true));
    assert!(!plausible(now_ms + days * 1000, false));
    // 2000-01-01, the earliest believed, in seconds and in milliseconds.
    assert!(plausible(946_684_800, true));
    assert!(!plausible(946_684_799, true));
    assert!(plausible(946_684_800_000, false));
    assert!(!plausible(946_684_799_999, false));
}

/// What each of `Kept`'s changes returns is what the other devices are sent
/// (`desktop/src/backend.rs` and `opium-ffi` share it as it comes): taken in elsewhere, it
/// brings the change there. The tests so far check `Kept`'s own copy, never what it returns.
///
/// Kills `lib.rs:700:9 replace Kept::change -> Shared with Default::default()`, and the same
/// mutant of `Kept::played` (710:9), `clear_history` (714:9), `add_to_playlist` (887:9),
/// `remove_from_playlist` (895:9), `move_in_playlist` (904:9) and `delete_playlist` (879:9).
#[test]
fn what_a_change_here_returns_brings_it_to_another_device() {
    let dir = std::env::temp_dir().join(format!("synced-changes-{}", new_id()));
    let (a, b) = (
        Kept::open(&dir.join("a"), Vec::new),
        Kept::open(&dir.join("b"), Vec::new),
    );
    let (id, made) = a.create_playlist("Mix".into(), String::new(), vec![track(1), track(2)]);
    assert!(b.merge(made));
    assert!(b.merge(a.played(track(3))));
    assert_eq!(titles(b.history()), ["Song 3"]);
    assert!(b.merge(a.add_to_playlist(&id, vec![track(4)])));
    let entries = a.playlist(&id).unwrap().entries_of(&track(2).id);
    assert!(b.merge(a.remove_from_playlist(&id, &entries)));
    let four = a.playlist(&id).unwrap().entries_of(&track(4).id);
    assert!(b.merge(a.move_in_playlist(&id, &four[0], 0)));
    let order = |k: &Kept| -> Vec<String> {
        k.playlist(&id)
            .unwrap()
            .songs
            .into_iter()
            .map(|s| s.track.title)
            .collect()
    };
    assert_eq!(order(&a), ["Song 4", "Song 1"]);
    assert_eq!(order(&b), order(&a));
    assert!(b.merge(a.clear_history()));
    assert!(b.history().is_empty());
    assert!(b.merge(a.delete_playlist(&id)));
    assert!(b.playlist(&id).is_none());
    drop((a, b));
    let _ = std::fs::remove_dir_all(&dir);
}
