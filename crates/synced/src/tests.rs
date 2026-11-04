//! Devices changing their copies apart, then meeting in every order.

use music_core::model::{ItemId, ProviderKind, Track};

use super::*;

fn device(name: &str) -> State {
    State::new(name.to_owned(), Shared::default())
}

fn track(n: u32) -> Track {
    Track {
        id: ItemId::new(ProviderKind::AppleMusic, n.to_string()),
        title: format!("Song {n}"),
        artists: vec!["Someone".into()],
        album: None,
        album_id: None,
        duration: None,
        track_number: None,
        disc_number: None,
        artwork: None,
        explicit: false,
        quality: None,
        in_library: false,
        library_id: None,
    }
}

fn titles(tracks: Vec<Track>) -> Vec<String> {
    tracks.into_iter().map(|t| t.title).collect()
}

fn history(s: &State) -> Vec<String> {
    titles(s.shared.history.tracks())
}

fn songs(s: &State, id: &str) -> Vec<String> {
    s.shared
        .playlists
        .get(id)
        .map(|p| p.songs.into_iter().map(|s| s.track.title).collect())
        .unwrap_or_default()
}

/// Two devices connect: each gets all the other has.
fn meet(a: &mut State, b: &mut State) {
    let (all_a, all_b) = (a.shared.clone(), b.shared.clone());
    a.merge(all_b);
    b.merge(all_a);
    assert_eq!(a.shared, b.shared, "met, they hold the same");
}

#[test]
fn songs_played_apart_are_in_both_histories() {
    let (mut a, mut b) = (device("a"), device("b"));
    a.played(track(1), 1000);
    b.played(track(2), 1500);
    a.played(track(3), 2000);
    meet(&mut a, &mut b);
    assert_eq!(history(&a), ["Song 3", "Song 2", "Song 1"]);
}

#[test]
fn a_song_played_on_both_is_there_once_at_its_last_play() {
    let (mut a, mut b) = (device("a"), device("b"));
    a.played(track(1), 1000);
    a.played(track(2), 2000);
    b.played(track(1), 3000);
    meet(&mut a, &mut b);
    assert_eq!(history(&b), ["Song 1", "Song 2"]);
}

#[test]
fn a_cleared_history_stays_cleared_however_late_it_is_heard_of() {
    let (mut a, mut b, mut c) = (device("a"), device("b"), device("c"));
    b.played(track(1), 1000);
    meet(&mut b, &mut c);
    // Cleared on `a`, which never heard of song 1: it goes everywhere all the same.
    a.clear_history(2000);
    b.played(track(2), 3000);
    meet(&mut a, &mut b);
    assert_eq!(history(&a), ["Song 2"]);
    // `c`, off all along, still has song 1: it doesn't come back.
    meet(&mut c, &mut a);
    assert_eq!(history(&c), ["Song 2"]);
}

#[test]
fn a_device_with_its_clock_behind_still_puts_its_last_song_first() {
    let (mut a, mut b) = (device("a"), device("b"));
    a.played(track(1), 10_000);
    meet(&mut a, &mut b);
    b.played(track(2), 1_000);
    assert_eq!(history(&b), ["Song 2", "Song 1"]);
    b.clear_history(1_100);
    meet(&mut a, &mut b);
    assert!(history(&a).is_empty());
}

#[test]
fn the_history_keeps_the_latest_songs_whatever_order_devices_meet_in() {
    let (mut a, mut b, mut c) = (device("a"), device("b"), device("c"));
    for n in 0..80 {
        a.played(track(n), 1000 + 2 * n as u64);
        b.played(track(100 + n), 1001 + 2 * n as u64);
    }
    c.played(track(500), 5000);
    let (mut a2, mut b2, mut c2) = (a.shared.clone(), b.shared.clone(), c.shared.clone());
    meet(&mut a, &mut b);
    meet(&mut b, &mut c);
    meet(&mut a, &mut c);
    // The other way round.
    c2.merge(b2.clone());
    c2.merge(a2.clone());
    b2.merge(c2.clone());
    a2.merge(b2.clone());
    assert_eq!(a.shared.history, a2.history);
    let kept = a.shared.history.tracks();
    assert_eq!(kept.len(), history::KEPT);
    assert_eq!(kept[0].title, "Song 500");
    // The oldest gone: 160 + 1 played, 100 kept.
    assert!(!titles(kept).contains(&"Song 0".to_owned()));
}

#[test]
fn taking_the_same_in_twice_changes_nothing() {
    let (mut a, mut b) = (device("a"), device("b"));
    a.played(track(1), 1000);
    let (made, _) = a.create_playlist("p", "Mix".into(), String::new(), vec![track(2)], 1100);
    assert!(b.merge(a.shared.clone()));
    assert!(!b.merge(a.shared.clone()));
    assert!(!b.merge(made));
}

#[test]
fn a_change_lost_on_the_way_comes_with_the_next_meeting() {
    let (mut a, mut b) = (device("a"), device("b"));
    let (_lost, _) = a.played(track(1), 1000);
    let (_lost, _) = a.create_playlist("p", "Mix".into(), String::new(), vec![track(2)], 1100);
    meet(&mut a, &mut b);
    assert_eq!(history(&b), ["Song 1"]);
    assert_eq!(songs(&b, "p"), ["Song 2"]);
}

#[test]
fn songs_added_apart_to_a_playlist_are_all_in_it_in_the_same_order() {
    let (mut a, mut b) = (device("a"), device("b"));
    a.create_playlist(
        "p",
        "Mix".into(),
        String::new(),
        vec![track(1), track(2)],
        1000,
    );
    meet(&mut a, &mut b);
    a.add_to_playlist("p", vec![track(3)], 2000);
    b.add_to_playlist("p", vec![track(4), track(5)], 2100);
    meet(&mut a, &mut b);
    let all = songs(&a, "p");
    assert_eq!(all.len(), 5);
    assert_eq!(all[..2], ["Song 1", "Song 2"]);
    assert_eq!(all, songs(&b, "p"));
    // Each device's songs stay together, in the order they were added.
    let at = |t: &str| all.iter().position(|s| s == t).unwrap();
    assert!(at("Song 4") < at("Song 5"));
}

#[test]
fn the_same_song_can_be_in_a_playlist_twice() {
    let (mut a, mut b) = (device("a"), device("b"));
    a.create_playlist("p", "Mix".into(), String::new(), vec![track(1)], 1000);
    meet(&mut a, &mut b);
    a.add_to_playlist("p", vec![track(2)], 2000);
    b.add_to_playlist("p", vec![track(2)], 2000);
    meet(&mut a, &mut b);
    assert_eq!(songs(&a, "p"), ["Song 1", "Song 2", "Song 2"]);
}

#[test]
fn of_two_names_given_apart_the_later_wins() {
    let (mut a, mut b) = (device("a"), device("b"));
    a.create_playlist("p", "Mix".into(), String::new(), vec![], 1000);
    meet(&mut a, &mut b);
    b.edit_playlist("p", "Later".into(), String::new(), 3000);
    a.edit_playlist("p", "Earlier".into(), String::new(), 2000);
    meet(&mut a, &mut b);
    assert_eq!(a.shared.playlists.get("p").unwrap().name, "Later");
}

#[test]
fn a_deleted_playlist_stays_deleted_whatever_was_done_to_it_meanwhile() {
    let (mut a, mut b, mut c) = (device("a"), device("b"), device("c"));
    a.create_playlist("p", "Mix".into(), String::new(), vec![track(1)], 1000);
    meet(&mut a, &mut b);
    meet(&mut b, &mut c);
    a.delete_playlist("p", 2000);
    b.add_to_playlist("p", vec![track(2)], 3000);
    b.edit_playlist("p", "Still here?".into(), String::new(), 3100);
    meet(&mut a, &mut b);
    assert!(a.shared.playlists.get("p").is_none());
    assert!(a.shared.playlists.all().is_empty());
    // `c` brings back its old copy: it stays gone.
    meet(&mut c, &mut b);
    assert!(c.shared.playlists.get("p").is_none());
    // And its songs aren't kept for nothing.
    assert!(!serde_json::to_string(&c.shared).unwrap().contains("Song 1"));
}

#[test]
fn a_song_taken_out_stays_out_even_if_moved_meanwhile() {
    let (mut a, mut b) = (device("a"), device("b"));
    a.create_playlist(
        "p",
        "Mix".into(),
        String::new(),
        vec![track(1), track(2), track(3)],
        1000,
    );
    meet(&mut a, &mut b);
    let entry = a.shared.playlists.get("p").unwrap().songs[0].entry.clone();
    a.remove_from_playlist("p", std::slice::from_ref(&entry));
    b.move_in_playlist("p", &entry, 2, 3000);
    meet(&mut a, &mut b);
    assert_eq!(songs(&b, "p"), ["Song 2", "Song 3"]);
}

#[test]
fn a_song_moved_on_both_ends_up_in_one_place() {
    let (mut a, mut b) = (device("a"), device("b"));
    a.create_playlist(
        "p",
        "Mix".into(),
        String::new(),
        (1..=5).map(track).collect(),
        1000,
    );
    meet(&mut a, &mut b);
    let entry = a.shared.playlists.get("p").unwrap().songs[0].entry.clone();
    a.move_in_playlist("p", &entry, 4, 2000);
    b.move_in_playlist("p", &entry, 2, 2500);
    meet(&mut a, &mut b);
    assert_eq!(
        songs(&a, "p"),
        ["Song 2", "Song 3", "Song 1", "Song 4", "Song 5"]
    );
}

#[test]
fn moving_puts_a_song_where_asked() {
    let mut a = device("a");
    a.create_playlist(
        "p",
        "Mix".into(),
        String::new(),
        (1..=4).map(track).collect(),
        1000,
    );
    let entry = |a: &State, k: usize| a.shared.playlists.get("p").unwrap().songs[k].entry.clone();
    let e = entry(&a, 3);
    a.move_in_playlist("p", &e, 0, 2000);
    assert_eq!(songs(&a, "p"), ["Song 4", "Song 1", "Song 2", "Song 3"]);
    let e = entry(&a, 0);
    a.move_in_playlist("p", &e, 3, 3000);
    assert_eq!(songs(&a, "p"), ["Song 1", "Song 2", "Song 3", "Song 4"]);
    let e = entry(&a, 3);
    a.move_in_playlist("p", &e, 1, 4000);
    assert_eq!(songs(&a, "p"), ["Song 1", "Song 4", "Song 2", "Song 3"]);
}

#[test]
fn changes_heard_out_of_order_come_out_the_same() {
    let (mut a, mut b) = (device("a"), device("b"));
    let (made, _) = a.create_playlist("p", "Mix".into(), String::new(), vec![track(1)], 1000);
    let (added, _) = a.add_to_playlist("p", vec![track(2)], 2000);
    let entry = a.shared.playlists.get("p").unwrap().songs[1].entry.clone();
    let (moved, _) = a.move_in_playlist("p", &entry, 0, 3000);
    let (renamed, _) = a.edit_playlist("p", "Renamed".into(), String::new(), 4000);
    for change in [renamed, moved, added, made] {
        b.merge(change);
    }
    assert_eq!(a.shared, b.shared);
    assert_eq!(songs(&b, "p"), ["Song 2", "Song 1"]);
}

#[test]
fn kept_on_disk_from_one_launch_to_the_next() {
    let dir = std::env::temp_dir().join(format!("synced-test-{}", new_id()));
    let kept = Kept::open(&dir, || vec![track(2), track(1)]);
    assert_eq!(titles(kept.history()), ["Song 2", "Song 1"]);
    kept.played(track(3));
    let (id, _) = kept.create_playlist("Mix".into(), String::new(), vec![track(4)]);
    let me = kept.me();
    drop(kept);
    // The writer goes on a moment after.
    let path = dir.join("shared.json");
    for _ in 0..100 {
        if std::fs::read_to_string(dir.join("playlists.json")).is_ok_and(|t| t.contains("Song 4")) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let again = Some(Kept::open(&dir, Vec::new)).filter(|k| k.playlist(&id).is_some());
    let again = again.expect("kept on disk");
    assert_eq!(again.me(), me);
    assert_eq!(titles(again.history()), ["Song 3", "Song 2", "Song 1"]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "only the user reads it");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_playlists_are_written_only_as_they_change() {
    let dir = std::env::temp_dir().join(format!("synced-lists-{}", new_id()));
    let kept = Kept::open(&dir, Vec::new);
    let (id, _) = kept.create_playlist("Mix".into(), String::new(), vec![track(1)]);
    kept.flush();
    let lists = dir.join("playlists.json");
    let written = std::fs::metadata(&lists).unwrap().modified().unwrap();
    assert!(
        !std::fs::read_to_string(dir.join("shared.json"))
            .unwrap()
            .contains("Song 1")
    );
    std::thread::sleep(std::time::Duration::from_millis(20));
    kept.played(track(2));
    kept.flush();
    assert_eq!(
        std::fs::metadata(&lists).unwrap().modified().unwrap(),
        written
    );
    kept.add_to_playlist(&id, vec![track(3)]);
    kept.flush();
    assert!(std::fs::read_to_string(&lists).unwrap().contains("Song 3"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn playlists_kept_with_the_rest_move_to_their_own_file() {
    let dir = std::env::temp_dir().join(format!("synced-move-{}", new_id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut before = device("a");
    before.create_playlist("p", "Mix".into(), String::new(), vec![track(1)], 1000);
    before.played(track(2), 2000);
    let old = serde_json::json!({ "me": "a", "shared": before.shared });
    std::fs::write(dir.join("shared.json"), old.to_string()).unwrap();
    let kept = Kept::open(&dir, Vec::new);
    assert_eq!(
        titles(
            kept.playlist("p")
                .unwrap()
                .songs
                .into_iter()
                .map(|s| s.track)
                .collect()
        ),
        ["Song 1"]
    );
    kept.flush();
    assert!(
        std::fs::read_to_string(dir.join("playlists.json"))
            .unwrap()
            .contains("Song 1")
    );
    assert!(
        !std::fs::read_to_string(dir.join("shared.json"))
            .unwrap()
            .contains("Song 1")
    );
    drop(kept);
    let again = Kept::open(&dir, Vec::new);
    assert!(again.playlist("p").is_some());
    assert_eq!(titles(again.history()), ["Song 2"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn devices_meeting_send_each_other_only_what_differs() {
    let (mut a, mut b) = (device("a"), device("b"));
    a.create_playlist("p", "Mix".into(), String::new(), vec![track(1)], 1000);
    a.create_playlist("q", "Other".into(), String::new(), vec![track(2)], 1100);
    meet(&mut a, &mut b);
    // Alike: nothing to send.
    assert!(a.shared.unlike(&b.shared.digest()).is_empty());
    // Apart: songs added to one playlist on each, one played on b.
    a.add_to_playlist("p", vec![track(3)], 2000);
    b.add_to_playlist("p", vec![track(4)], 2100);
    b.played(track(5), 2200);
    let to_b = a.shared.unlike(&b.shared.digest());
    let to_a = b.shared.unlike(&a.shared.digest());
    assert_eq!(
        to_b.playlists.digests().keys().collect::<Vec<_>>(),
        ["p"],
        "q is alike"
    );
    a.merge(to_a);
    b.merge(to_b);
    assert_eq!(a.shared, b.shared, "met, they hold the same");
    let mut p = songs(&a, "p");
    p.sort();
    assert_eq!(p, ["Song 1", "Song 3", "Song 4"]);
    assert_eq!(history(&a), ["Song 5"]);
}

#[test]
fn a_playlist_s_cover_is_the_last_chosen_on_any_device() {
    let (mut a, mut b) = (device("a"), device("b"));
    a.create_playlist("p", "Mix".into(), String::new(), vec![track(1)], 1000);
    meet(&mut a, &mut b);
    a.cover_playlist("p", Some("AAA".into()), 2000);
    b.cover_playlist("p", Some("BBB".into()), 3000);
    meet(&mut a, &mut b);
    assert_eq!(a.shared.playlists.get("p").unwrap().cover_data.as_deref(), Some("BBB"));
    // Taken off: the first song's again, on both.
    a.cover_playlist("p", None, 4000);
    meet(&mut a, &mut b);
    assert_eq!(b.shared.playlists.get("p").unwrap().cover_data, None);
}

#[test]
fn a_chosen_cover_shows_from_a_file_of_its_own() {
    let dir = std::env::temp_dir().join(format!("synced-cover-{}", new_id()));
    let kept = Kept::open(&dir, Vec::new);
    let (id, _) = kept.create_playlist("Mix".into(), String::new(), vec![track(1)]);
    assert_eq!(kept.playlist(&id).unwrap().cover, None);
    kept.cover_playlist(&id, Some(b"jpeg one"));
    let first = kept.playlist(&id).unwrap().cover.unwrap();
    let path = first.strip_prefix("file://").unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"jpeg one");
    assert_eq!(kept.playlist(&id).unwrap().model().artwork.unwrap().url_template, first);
    // Another one: the first file goes.
    kept.cover_playlist(&id, Some(b"jpeg two"));
    let second = kept.playlist(&id).unwrap().cover.unwrap();
    assert_ne!(first, second);
    assert!(!std::path::Path::new(path).exists());
    kept.cover_playlist(&id, None);
    assert_eq!(kept.playlist(&id).unwrap().cover, None);
    let _ = std::fs::remove_dir_all(&dir);
}

fn session(songs: &[u32], at: u32, position_ms: u64) -> Handoff {
    Handoff {
        tracks: songs.iter().copied().map(track).collect(),
        order: (0..songs.len() as u32).collect(),
        at,
        shuffle: false,
        repeat: music_core::Repeat::Off,
        position_ms,
        playing: true,
        station: None,
    }
}

fn last(s: &State) -> Option<(u64, String)> {
    s.shared
        .session
        .as_ref()
        .map(|l| (l.value.position_ms, l.at.device.clone()))
}

#[test]
fn where_the_music_last_stopped_is_the_device_that_played_last() {
    let (mut a, mut b) = (device("a"), device("b"));
    a.left(session(&[1, 2], 0, 30_000), 1000);
    meet(&mut a, &mut b);
    // `b`, alone, plays on and stops later.
    b.left(session(&[1, 2], 1, 42_000), 5000);
    meet(&mut a, &mut b);
    assert_eq!(last(&a), Some((42_000, "b".into())));
    let kept = &a.shared.session.as_ref().unwrap().value;
    assert_eq!(kept.at, 1);
    // Taken up again, it starts paused.
    assert!(!kept.playing);
}

#[test]
fn a_device_opened_and_closed_without_playing_doesnt_pass_for_the_last() {
    let (mut a, mut b) = (device("a"), device("b"));
    a.left(session(&[1], 0, 10_000), 1000);
    meet(&mut a, &mut b);
    b.left(session(&[3], 0, 20_000), 5000);
    // `a` opens and closes, still where it was, with `b`'s stop unheard of.
    let (change, changed) = a.left(session(&[1], 0, 10_000), 9000);
    assert!(!changed && change.is_empty());
    meet(&mut a, &mut b);
    assert_eq!(last(&a), Some((20_000, "b".into())));
}

#[test]
fn of_two_devices_times_for_a_song_the_earliest_wins_whoever_hears_first() {
    // Real times: a device's clock decides these two fields outright, so
    // one saying 1970 is not believed (see `plausible`).
    const WHEN: u64 = 1_700_000_000;
    let seen = |pairs: &[(&str, u64)]| Shared {
        seen: pairs
            .iter()
            .map(|(id, at)| ((*id).to_owned(), WHEN + *at))
            .collect(),
        ..Shared::default()
    };
    let phone = seen(&[("a", 100), ("b", 300)]);
    let desktop = seen(&[("a", 200), ("b", 250), ("c", 50)]);
    let mut one = phone.clone();
    assert!(one.merge(desktop.clone()));
    let mut other = desktop;
    assert!(other.merge(phone));
    assert_eq!(one, other);
    assert_eq!(one, seen(&[("a", 100), ("b", 250), ("c", 50)]));
    // Heard again: nothing changes.
    assert!(!one.merge(other));
}

#[test]
fn the_playlists_used_last_are_alike_everywhere_and_few() {
    const WHEN: u64 = 1_700_000_000_000;
    let used = |pairs: &[(&str, u64)]| Shared {
        used: pairs
            .iter()
            .map(|(id, at)| ((*id).to_owned(), WHEN + *at))
            .collect(),
        ..Shared::default()
    };
    let phone = used(&[("a", 10), ("b", 40), ("c", 30)]);
    let desktop = used(&[("a", 50), ("d", 20)]);
    let mut one = phone.clone();
    one.merge(desktop.clone());
    let mut other = desktop;
    other.merge(phone);
    assert_eq!(one, other);
    assert_eq!(one, used(&[("a", 50), ("b", 40), ("c", 30)]));
}

#[test]
fn a_setting_set_last_wins_and_one_from_before_gives_way() {
    let dir = std::env::temp_dir().join(format!("synced-settings-{}", std::process::id()));
    let (a, b) = (
        Kept::open(&dir.join("a"), Vec::new),
        Kept::open(&dir.join("b"), Vec::new),
    );
    a.offer_setting("relay", "old".into());
    let set = b.set_setting("relay", "new".into());
    a.merge(set);
    b.merge(a.everything());
    assert_eq!(a.settings(), b.settings());
    assert_eq!(a.settings()["relay"], "new");
    // Offered once there is one: nothing changes.
    assert!(a.offer_setting("relay", "older".into()).is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn songs_of_a_service_taken_out_of_the_app_are_dropped_not_everything() {
    let dir = std::env::temp_dir().join(format!("synced-unknown-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    {
        let kept = Kept::open(&dir, Vec::new);
        kept.played(track(1));
        kept.played(track(2));
        kept.left(Handoff {
            tracks: vec![track(2)],
            order: vec![0],
            at: 0,
            shuffle: false,
            repeat: music_core::Repeat::Off,
            position_ms: 0,
            playing: false,
            station: None,
        });
        kept.flush();
    }
    // Song 2 as of a service this build no longer knows.
    let path = dir.join("shared.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let gone = text.replace(
        r#""provider":"AppleMusic","id":"2""#,
        r#""provider":"Gone","id":"2""#,
    );
    assert_ne!(gone, text);
    std::fs::write(&path, gone).unwrap();
    let kept = Kept::open(&dir, Vec::new);
    assert_eq!(titles(kept.history()), ["Song 1"]);
    assert!(kept.session().is_none());
    assert!(!dir.join("shared.json.bad").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_device_whose_clock_is_wrong_doesnt_pin_the_times_everywhere() {
    // A phone booting with its clock in 1970, and one years ahead: neither
    // is believed, so Recently Added and Add to Playlist stay as they are.
    let mut here = Shared {
        seen: [("a".to_owned(), 1_700_000_000u64)].into_iter().collect(),
        used: [("p".to_owned(), 1_700_000_000_000u64)].into_iter().collect(),
        ..Shared::default()
    };
    let before = here.clone();
    let wrong = Shared {
        seen: [("a".to_owned(), 60u64)].into_iter().collect(),
        used: [("p".to_owned(), u64::MAX / 2)].into_iter().collect(),
        ..Shared::default()
    };
    assert!(!here.merge(wrong), "nothing of it is taken in");
    assert_eq!(here, before);
}
