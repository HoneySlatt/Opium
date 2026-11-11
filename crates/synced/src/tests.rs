//! Devices changing their copies apart, then meeting in every order.

use opium_core::model::{ItemId, ProviderKind, Track};

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

/// Close `kept` with nothing left to write: a `Kept` dropped with a save
/// waiting writes it a moment later, from its own thread, and so makes its
/// folder again once the test has removed it.
fn close(kept: Kept) {
    kept.flush();
    drop(kept);
}

#[test]
fn kept_on_disk_from_one_launch_to_the_next() {
    let dir = std::env::temp_dir().join(format!("synced-test-{}", new_id()));
    let kept = Kept::open(&dir, || vec![track(2), track(1)]);
    assert_eq!(titles(kept.history()), ["Song 2", "Song 1"]);
    kept.played(track(3));
    let (id, _) = kept.create_playlist("Mix".into(), String::new(), vec![track(4)]);
    let me = kept.me();
    // On disk, both files, before the app closes.
    kept.flush();
    drop(kept);
    let path = dir.join("shared.json");
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
    assert_eq!(
        a.shared.playlists.get("p").unwrap().cover_data.as_deref(),
        Some("BBB")
    );
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
    assert_eq!(
        kept.playlist(&id)
            .unwrap()
            .model()
            .artwork
            .unwrap()
            .url_template,
        first
    );
    // Another one: the first file goes.
    kept.cover_playlist(&id, Some(b"jpeg two"));
    let second = kept.playlist(&id).unwrap().cover.unwrap();
    assert_ne!(first, second);
    assert!(!std::path::Path::new(path).exists());
    kept.cover_playlist(&id, None);
    assert_eq!(kept.playlist(&id).unwrap().cover, None);
    close(kept);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A chosen cover's file is a cache, written without a flush: an empty one,
/// which a power cut can leave, is written again from the playlist.
#[test]
fn a_chosen_covers_file_left_empty_is_written_again() {
    let dir = std::env::temp_dir().join(format!("synced-cover-empty-{}", new_id()));
    let kept = Kept::open(&dir, Vec::new);
    let (id, _) = kept.create_playlist("Mix".into(), String::new(), vec![track(1)]);
    kept.cover_playlist(&id, Some(b"jpeg one"));
    let cover = kept.playlist(&id).unwrap().cover.unwrap();
    let path = cover.strip_prefix("file://").unwrap().to_owned();
    std::fs::write(&path, b"").unwrap();
    let again = kept.playlist(&id).unwrap().cover.unwrap();
    let written = std::fs::read(&path).unwrap();
    close(kept);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(again, cover);
    assert_eq!(written, b"jpeg one");
}

fn session(songs: &[u32], at: u32, position_ms: u64) -> Handoff {
    Handoff {
        tracks: songs.iter().copied().map(track).collect(),
        order: (0..songs.len() as u32).collect(),
        at,
        shuffle: false,
        repeat: opium_core::Repeat::Off,
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
    close(a);
    close(b);
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
            repeat: opium_core::Repeat::Off,
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
        used: [("p".to_owned(), 1_700_000_000_000u64)]
            .into_iter()
            .collect(),
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

/// Stage 1 audit, L4: a copy put aside is no first run.
///
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

/// Stage 1 audit, L9: the playlists moving to their own file are kept
/// through a write of that file that fails.
///
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
    // As a full disk would: the new file's writes fail.
    let lists = dir.join("playlists.json");
    lock(&FAILING_WRITES).push(lists.clone());
    let kept = Kept::open(&dir, Vec::new);
    kept.flush();
    drop(kept);
    lock(&FAILING_WRITES).retain(|failing| *failing != lists);
    let failed = !lists.exists();
    let again = Kept::open(&dir, Vec::new);
    let kept_on_disk = again.playlist("p").is_some();
    again.flush();
    drop(again);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(failed, "the write of playlists.json failed");
    assert!(kept_on_disk, "the playlist is still in one of the files");
}

/// The files whose writes fail, as on a full disk: each test names its own,
/// in a folder of its own.
static FAILING_WRITES: Mutex<Vec<std::path::PathBuf>> = Mutex::new(Vec::new());

/// Whether a test made the writes of `path` fail.
pub(super) fn write_fails(path: &Path) -> bool {
    lock(&FAILING_WRITES).iter().any(|failing| failing == path)
}

/// Stage 1 audit, S4: a playlist id another device chose never names a
/// file outside the covers' folder.
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
    close(kept);
    let outside: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("escaped-"))
        .collect();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(outside.is_empty(), "written outside covers/: {outside:?}");
}

/// L4, a real first run: the songs played before the devices shared their
/// history rank below every real play, so that meeting a paired device
/// they never push its plays out of the latest kept.
#[test]
fn songs_played_before_a_first_run_rank_below_every_real_play() {
    let hour_ago = now() - 3_600_000;
    let mut phone = device("phone");
    for n in 0..100 {
        phone.played(track(n), hour_ago + u64::from(n));
    }
    let dir = std::env::temp_dir().join(format!("synced-first-run-{}", new_id()));
    let computer = Kept::open(&dir, || (1000..1100).rev().map(track).collect());
    assert_eq!(
        history(&State::new("x".into(), computer.everything()))[0],
        "Song 1099"
    );
    phone.merge(computer.everything());
    // Played on the computer now: first, above the phone's.
    let played = computer.played(track(2000));
    phone.merge(played);
    computer.flush();
    drop(computer);
    let _ = std::fs::remove_dir_all(&dir);
    let kept = history(&phone);
    assert_eq!(kept.len(), history::KEPT);
    assert_eq!(kept[0], "Song 2000");
    assert_eq!(kept[1], "Song 99", "the phone's last play is next");
    assert!(
        !kept.iter().any(|t| t == "Song 1099"),
        "older than any real play"
    );
}

/// L5: each copy put aside keeps a name of its own, `shared.json`'s and
/// `playlists.json`'s alike.
#[test]
fn a_second_copy_put_aside_keeps_the_first() {
    let dir = std::env::temp_dir().join(format!("synced-aside-twice-{}", new_id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (copy, aside) in [("first", "bad"), ("second", "bad.1")] {
        std::fs::write(dir.join("shared.json"), format!("{{ {copy} shared")).unwrap();
        std::fs::write(dir.join("playlists.json"), format!("{{ {copy} lists")).unwrap();
        let kept = Kept::open(&dir, Vec::new);
        let (id, _) = kept.create_playlist("Mix".into(), String::new(), vec![track(1)]);
        kept.flush();
        drop(kept);
        for name in ["shared", "playlists"] {
            let kept_aside = dir.join(format!("{name}.json.{aside}"));
            let what = if name == "shared" { "shared" } else { "lists" };
            assert_eq!(
                std::fs::read_to_string(&kept_aside).unwrap(),
                format!("{{ {copy} {what}"),
                "{} keeps the {copy} copy",
                kept_aside.display()
            );
        }
        // Written again, readable, beside the copies put aside.
        let again = Kept::open(&dir, Vec::new);
        assert!(again.playlist(&id).is_some());
        close(again);
    }
    assert_eq!(
        std::fs::read_to_string(dir.join("shared.json.bad")).unwrap(),
        "{ first shared",
        "the first copy is still there"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// L5: a file that doesn't read and can't be put aside is never written
/// over; the copy goes on in memory.
#[cfg(unix)]
#[test]
fn a_copy_that_cant_be_put_aside_is_never_written_over() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("synced-stuck-{}", new_id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("shared.json"), b"{ damaged shared").unwrap();
    std::fs::write(dir.join("playlists.json"), b"{ damaged lists").unwrap();
    // A folder that can't be written: nothing in it can be put aside.
    let mode = |m| std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(m)).unwrap();
    mode(0o500);
    assert!(
        std::fs::write(dir.join("probe"), b"").is_err(),
        "the folder refuses to be written (a test run as root can't check this)"
    );
    let kept = Kept::open(&dir, Vec::new);
    let (id, _) = kept.create_playlist("Mix".into(), String::new(), vec![track(1)]);
    // Writable again: the copy still mustn't replace what is there.
    mode(0o700);
    kept.played(track(2));
    kept.flush();
    assert!(kept.playlist(&id).is_some(), "it goes on in memory");
    assert_eq!(titles(kept.history()), ["Song 2"]);
    drop(kept);
    let shared = std::fs::read_to_string(dir.join("shared.json"));
    let lists = std::fs::read_to_string(dir.join("playlists.json"));
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(shared.unwrap(), "{ damaged shared");
    assert_eq!(lists.unwrap(), "{ damaged lists");
}

/// A copy holding a little of everything, for cutting into parts: plays, a
/// clear, a session, settings, playlists used last, songs first seen, and
/// playlists long and short, renamed, covered, deleted, with songs taken
/// out and moved, and one holding nothing.
fn a_bit_of_everything() -> State {
    const WHEN: u64 = 1_700_000_000;
    let mut a = device("a");
    a.played(track(1), 1000);
    a.clear_history(1500);
    for n in 2..5 {
        a.played(track(n), 2000 + u64::from(n));
    }
    a.left(session(&[1, 2], 1, 30_000), 3000);
    a.create_playlist(
        "long",
        "Long".into(),
        "Many".into(),
        (0..300).map(track).collect(),
        4000,
    );
    a.create_playlist("short", "Short".into(), String::new(), vec![track(7)], 4100);
    a.cover_playlist("short", Some("AAAA".into()), 4200);
    a.create_playlist("gone", "Gone".into(), String::new(), vec![track(8)], 4300);
    a.delete_playlist("gone", 4400);
    let entries: Vec<String> = a.shared.playlists.get("long").unwrap().songs[..3]
        .iter()
        .map(|s| s.entry.clone())
        .collect();
    a.remove_from_playlist("long", &entries);
    let moved = a.shared.playlists.get("long").unwrap().songs[10]
        .entry
        .clone();
    a.move_in_playlist("long", &moved, 0, 4500);
    // A playlist holding nothing, as a change made of no songs leaves one.
    a.remove_from_playlist("empty", &[]);
    a.shared.seen = (0..200).map(|n| (format!("song {n}"), WHEN + n)).collect();
    a.shared.used = [("long".to_owned(), WHEN * 1000)].into_iter().collect();
    a.shared.settings.insert(
        "relay".into(),
        Latest {
            at: Stamp {
                at: 5000,
                device: "a".into(),
            },
            value: "https://relay.example".into(),
        },
    );
    a
}

#[test]
fn parts_taken_in_in_any_order_and_twice_make_the_whole() {
    let whole = a_bit_of_everything().shared;
    let max = 4096;
    let parts = whole.clone().into_parts(max);
    assert!(parts.len() > 5, "cut: {} parts", parts.len());
    for part in &parts {
        assert!(json_len(part) <= max, "{} bytes > {max}", json_len(part));
    }
    // Into a device of its own copy, in order, the other way round, and each
    // part twice: the same as the whole taken in at once.
    let mut b = device("b");
    b.played(track(50), 2500);
    b.create_playlist("long", "Mine".into(), String::new(), vec![track(51)], 3500);
    let mut at_once = b.shared.clone();
    at_once.merge(whole.clone());
    let mut in_order = b.shared.clone();
    for part in parts.iter().cloned() {
        in_order.merge(part);
    }
    let mut backwards = b.shared.clone();
    for part in parts.iter().rev().chain(&parts).cloned() {
        backwards.merge(part);
    }
    assert_eq!(in_order, at_once);
    assert_eq!(backwards, at_once);
    assert_eq!(backwards.digest(), at_once.digest());
    // Into an empty one: the whole again, the empty playlist with it.
    let mut empty = Shared::default();
    for part in parts {
        empty.merge(part);
    }
    assert_eq!(empty, whole);
}

#[test]
fn one_playlist_longer_than_a_part_spans_parts_in_its_order() {
    let mut a = device("a");
    a.create_playlist(
        "p",
        "Mix".into(),
        String::new(),
        (0..1000).map(track).collect(),
        1000,
    );
    let max = 16 * 1024;
    let parts = a.shared.clone().into_parts(max);
    assert!(parts.len() > 10, "cut: {} parts", parts.len());
    assert!(parts.iter().all(|p| json_len(p) <= max));
    let mut b = device("b");
    for part in parts.into_iter().rev() {
        b.merge(part);
    }
    assert_eq!(b.shared, a.shared);
    assert_eq!(songs(&b, "p"), songs(&a, "p"));
}

#[test]
fn an_empty_copy_has_no_part_and_a_small_one_one() {
    assert!(Shared::default().into_parts(1024).is_empty());
    let (change, _) = device("a").played(track(1), 1000);
    assert_eq!(change.clone().into_parts(1024), [change]);
}

#[test]
fn a_piece_longer_than_a_part_goes_alone() {
    let mut a = device("a");
    a.create_playlist(
        "p",
        "Mix".into(),
        String::new(),
        (0..20).map(track).collect(),
        1000,
    );
    a.cover_playlist("p", Some("A".repeat(10_000)), 2000);
    let max = 2048;
    let parts = a.shared.clone().into_parts(max);
    let over: Vec<&Shared> = parts.iter().filter(|p| json_len(*p) > max).collect();
    assert_eq!(over.len(), 1, "the cover's part alone is over");
    let alone = over[0].playlists.get("p").expect("the name and cover");
    assert!(alone.songs.is_empty(), "no song with it");
    let mut b = device("b");
    for part in parts {
        b.merge(part);
    }
    assert_eq!(b.shared, a.shared);
}

#[test]
fn playlist_ids_this_app_doesnt_make_are_left_out() {
    let lists: serde_json::Map<String, serde_json::Value> = [
        "../escaped",
        "a/b",
        "a\\b",
        "..",
        ".",
        "",
        "ABCDEF0123456789",
        "0123-4567",
        &"a".repeat(65),
        "0123456789abcdef",
    ]
    .into_iter()
    .map(|id| {
        let list = serde_json::json!({"name": {"at": {"at": 1, "device": "x"}, "value": id}});
        (id.to_owned(), list)
    })
    .collect();
    let mut theirs: Shared =
        serde_json::from_value(serde_json::json!({ "playlists": { "lists": lists } })).unwrap();
    assert_eq!(theirs.leave_out_bad_ids(), 9);
    assert_eq!(
        theirs.playlists.digests().keys().collect::<Vec<_>>(),
        ["0123456789abcdef"]
    );
    for _ in 0..1000 {
        assert!(is_playlist_id(&new_id()));
    }
}

/// What another device sends, in parts, is written once the parts stop
/// coming, with no flush asked.
#[test]
fn what_another_device_sends_is_written_once_its_parts_stop_coming() {
    let dir = std::env::temp_dir().join(format!("synced-soon-{}", new_id()));
    let kept = Kept::open(&dir, Vec::new);
    kept.flush();
    let mut a = device("a");
    let id = "0123456789abcdef";
    a.create_playlist(
        id,
        "Mix".into(),
        String::new(),
        (0..200).map(track).collect(),
        1000,
    );
    let parts = a.shared.clone().into_parts(4096);
    assert!(parts.len() > 3);
    for part in parts {
        assert!(kept.merge(part));
    }
    let lists = dir.join("playlists.json");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !std::fs::read_to_string(&lists).is_ok_and(|t| t.contains("Song 199")) {
        assert!(std::time::Instant::now() < deadline, "never written");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    drop(kept);
    assert!(Kept::open(&dir, Vec::new).playlist(id).is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The apps take a song out of one of Opium's playlists by the entries `entries_of` gives
/// (`desktop`'s backend and `opium-ffi`), and the removal is final on every
/// device: they must be the song's own entries, each time it is in, and no other.
///
/// Catches `==` replaced by `!=` in `Playlist::entries_of`, which would take out every other
/// song, for good.
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
/// Catches `playlists_without_unknown_services` replaced by `None` (a miss before: no test read
/// such a file).
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
/// (`connect`'s `lib.rs`), and every test so far asserts emptiness only.
///
/// Catches `Shared::is_empty` replaced by `true`.
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
/// Catches `|=` replaced by `&=` where `Shared::merge_parts` takes the settings in.
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
/// Catches the day's room in milliseconds (`AHEAD * 1000`) made `AHEAD + 1000`.
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
/// Catches `Kept::change` returning `Shared::default()`, which every change made on `Kept` goes
/// through.
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
    close(a);
    close(b);
    let _ = std::fs::remove_dir_all(&dir);
}
