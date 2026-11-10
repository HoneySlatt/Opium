//! The m18-data findings, run against Opium's crates through their public
//! items. Each test states what the documents promise and fails today.
//! Every test writes only in its own folder under `std::env::temp_dir()`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use downloads::sealed;
use downloads::store::{Song, Store};
use opium_core::model::{ItemId, ProviderKind, Track};

/// A folder of the test's own, removed at the end.
struct Temp(PathBuf);

impl Temp {
    fn new(name: &str) -> Self {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "m18-data-{name}-{}-{}",
            std::process::id(),
            COUNT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn track(provider: ProviderKind, id: &str) -> Track {
    Track {
        id: ItemId::new(provider, id),
        title: format!("Song {id}"),
        artists: vec!["Someone".into()],
        album: None,
        album_id: None,
        duration: None,
        track_number: None,
        disc_number: None,
        artwork: None,
        explicit: false,
        quality: None,
        in_library: true,
        library_id: None,
    }
}

fn song(id: &str) -> Song {
    Song {
        track: track(ProviderKind::Navidrome, id),
        extension: "flac".into(),
        priming: 0,
        protected: false,
        kbps: None,
        spatial: false,
    }
}

fn keep(store: &mut Store, id: &str, data: &[u8], now: u64) {
    let mut pending = store.begin().unwrap();
    pending.write_all(data).unwrap();
    store.keep(pending, song(id), now).unwrap();
}

fn sealed_files(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".sealed"))
        .count()
}

/// ADR-0017: "A manifest that does not read is put aside, without sweeping
/// away the files it no longer names." The start that finds it does so;
/// the start after it sweeps them.
#[test]
fn p1a_songs_of_a_manifest_put_aside_stay_at_the_next_start() {
    let dir = Temp::new("p1a");
    let key = sealed::new_key();
    {
        let mut store = Store::open(&dir.0, key).unwrap();
        keep(&mut store, "1", b"music", 10);
    }
    // As a later version's manifest reads here (the store's own test).
    std::fs::write(
        dir.0.join("manifest.json"),
        br#"{"version":99,"items":[]}"#,
    )
    .unwrap();
    drop(Store::open(&dir.0, key).unwrap());
    assert!(dir.0.join("manifest.corrupt.json").exists());
    assert_eq!(sealed_files(&dir.0), 1, "left alone at the start that put it aside");
    // The app started again, nothing done in between.
    drop(Store::open(&dir.0, key).unwrap());
    assert_eq!(
        sealed_files(&dir.0),
        1,
        "the song's file must still be there at the next start"
    );
}

/// The same, with what every Apple Music connection does in between: the
/// subscription confirmed, which writes the (now empty) manifest.
#[test]
fn p1b_songs_of_a_manifest_put_aside_stay_after_a_subscription_is_confirmed() {
    let dir = Temp::new("p1b");
    let key = sealed::new_key();
    {
        let mut store = Store::open(&dir.0, key).unwrap();
        keep(&mut store, "1", b"music", 10);
    }
    std::fs::write(dir.0.join("manifest.json"), b"{ damaged").unwrap();
    {
        let mut store = Store::open(&dir.0, key).unwrap();
        store
            .subscription(ProviderKind::AppleMusic, true, 20)
            .unwrap();
    }
    drop(Store::open(&dir.0, key).unwrap());
    assert_eq!(
        sealed_files(&dir.0),
        1,
        "the song's file must still be there at the next start"
    );
}

/// `Store::keep` removes the copy kept before as the same track, then
/// writes the manifest: when that write fails, neither copy is left.
#[test]
fn p2_a_song_downloaded_again_is_not_lost_when_the_manifest_cannot_be_written() {
    let dir = Temp::new("p2");
    let key = sealed::new_key();
    let id = ItemId::new(ProviderKind::Navidrome, "1");
    {
        let mut store = Store::open(&dir.0, key).unwrap();
        keep(&mut store, "1", b"first copy", 10);
        // The manifest can't be written this time (a full disk, an I/O
        // error): here, a folder stands where its new copy goes.
        std::fs::create_dir(dir.0.join("manifest.json.part")).unwrap();
        let mut pending = store.begin().unwrap();
        pending.write_all(b"second copy").unwrap();
        assert!(store.keep(pending, song("1"), 20).is_err());
    }
    std::fs::remove_dir(dir.0.join("manifest.json.part")).unwrap();
    let store = Store::open(&dir.0, key).unwrap();
    let opened = store.open_song(&id, 30);
    assert!(
        matches!(opened, Some(Ok(_))),
        "one copy of the song must still open at the next start; {} sealed files left, open: {}",
        sealed_files(&dir.0),
        match &opened {
            None => "not kept".to_owned(),
            Some(Ok(_)) => "ok".to_owned(),
            Some(Err(e)) => e.to_string(),
        }
    );
}

/// Two instances on one store (the known debt): a song the first keeps is
/// deleted once the second, opened before it, writes its manifest.
#[test]
fn p5_a_song_kept_by_one_instance_survives_another_writing_its_manifest() {
    let dir = Temp::new("p5");
    let key = sealed::new_key();
    let mut second = Store::open(&dir.0, key).unwrap();
    let mut first = Store::open(&dir.0, key).unwrap();
    keep(&mut first, "1", b"music", 10);
    // The second instance's Apple Music connects.
    second
        .subscription(ProviderKind::AppleMusic, true, 20)
        .unwrap();
    drop((first, second));
    let store = Store::open(&dir.0, key).unwrap();
    assert!(
        store.get(&ItemId::new(ProviderKind::Navidrome, "1")).is_some() || sealed_files(&dir.0) == 1,
        "the song the first instance kept must not be swept"
    );
}

/// ADR-0014: devices merge "without one device's copy overwriting another's
/// changes". A computer whose `shared.json` is put aside starts again from
/// the history kept before the devices shared it, stamped as played now:
/// meeting the phone, it pushes the phone's real history out of the 100
/// kept.
#[test]
fn p3_a_copy_put_aside_does_not_push_the_phones_history_out() {
    let phone_dir = Temp::new("p3-phone");
    let phone = synced::Kept::open(&phone_dir.0, Vec::new);
    for n in 0..100 {
        phone.played(track(ProviderKind::AppleMusic, &n.to_string()));
    }
    assert_eq!(phone.history()[0].title, "Song 99");
    // Later, the computer's `shared.json` damaged; its `history.json` from
    // before the devices shared their history still there (neither app
    // removes it), here songs 1000 to 1099.
    std::thread::sleep(std::time::Duration::from_millis(300));
    let computer_dir = Temp::new("p3-computer");
    std::fs::write(computer_dir.0.join("shared.json"), b"{ damaged").unwrap();
    let old: Vec<Track> = (1000..1100)
        .map(|n| track(ProviderKind::AppleMusic, &n.to_string()))
        .collect();
    let computer = synced::Kept::open(&computer_dir.0, move || old);
    assert!(computer_dir.0.join("shared.json.bad").exists());
    // They meet.
    phone.merge(computer.everything());
    let titles: Vec<String> = phone.history().into_iter().map(|t| t.title).collect();
    phone.flush();
    computer.flush();
    drop((phone, computer));
    assert!(
        titles.iter().any(|t| t == "Song 99"),
        "the phone's last play must stay in its history; it holds {} songs, the first {:?}",
        titles.len(),
        titles.first()
    );
}

/// The mechanism under `desktop::settings` (findings on `settings.toml`):
/// TOML can't write a path that isn't UTF-8, and `read_to_string` refuses
/// a file that isn't. `Settings` keeps its folders as `Vec<PathBuf>`.
#[test]
fn p4_settings_mechanism_toml_and_read_to_string() {
    use std::os::unix::ffi::OsStrExt;
    #[derive(serde::Serialize, Default)]
    #[serde(default)]
    struct LikeSettings {
        volume: Option<f32>,
        local_folders: Vec<PathBuf>,
    }
    let fine = LikeSettings {
        volume: Some(0.5),
        local_folders: vec![PathBuf::from("/music")],
    };
    assert!(toml::to_string(&fine).is_ok());
    let latin1 = LikeSettings {
        volume: Some(0.5),
        local_folders: vec![PathBuf::from(std::ffi::OsStr::from_bytes(b"/music/caf\xe9"))],
    };
    let written = toml::to_string(&latin1);
    let dir = Temp::new("p4");
    let file = dir.0.join("settings.toml");
    std::fs::write(&file, b"volume = 0.3\ntheme = \"caf\xe9\"\n").unwrap();
    let read = std::fs::read_to_string(&file);
    assert!(
        written.is_ok() && read.is_ok(),
        "toml::to_string with a non-UTF-8 folder: {:?}; read_to_string of a non-UTF-8 file: {:?}",
        written.as_ref().err().map(ToString::to_string),
        read.as_ref().err().map(|e| (e.kind(), e.to_string()))
    );
}

/// The playlists, once kept in `shared.json`, move to `playlists.json`:
/// "a power cut between the two leaves them in both files". When the new
/// file can't be written (a full disk), `shared.json` is written anyway,
/// without them, and they are in neither file.
#[test]
fn p6_playlists_moving_to_their_own_file_survive_a_failed_write() {
    let src = Temp::new("p6-src");
    let kept = synced::Kept::open(&src.0, Vec::new);
    let (id, _) = kept.create_playlist(
        "Mix".into(),
        String::new(),
        vec![track(ProviderKind::AppleMusic, "1")],
    );
    let shared = kept.everything();
    kept.flush();
    drop(kept);
    // As kept before the playlists had a file of their own.
    let dir = Temp::new("p6");
    let old = serde_json::json!({ "me": "a", "shared": shared });
    std::fs::write(dir.0.join("shared.json"), old.to_string()).unwrap();
    // The new file can't be written: a folder stands where its copy goes.
    std::fs::create_dir(dir.0.join("playlists.json.part")).unwrap();
    let kept = synced::Kept::open(&dir.0, Vec::new);
    assert!(kept.playlist(&id).is_some());
    kept.flush();
    drop(kept);
    std::fs::remove_dir(dir.0.join("playlists.json.part")).unwrap();
    let again = synced::Kept::open(&dir.0, Vec::new);
    let found = again.playlist(&id).is_some();
    again.flush();
    drop(again);
    assert!(found, "the playlist is still kept on disk");
}

/// A file that doesn't read is "put aside rather than written over"
/// (`Kept::open`, `connect::Store::open`): put aside a second time, the
/// copy the first put aside is replaced, and with it the user's data.
#[test]
fn p7_a_second_put_aside_keeps_the_first() {
    let dir = Temp::new("p7");
    let first = b"{ the user's copy, damaged".to_vec();
    std::fs::write(dir.0.join("shared.json"), &first).unwrap();
    let kept = synced::Kept::open(&dir.0, Vec::new);
    kept.flush();
    drop(kept);
    assert_eq!(std::fs::read(dir.0.join("shared.json.bad")).unwrap(), first);
    // Damaged again later, by the same cause or another.
    std::fs::write(dir.0.join("shared.json"), b"{ damaged again").unwrap();
    let kept = synced::Kept::open(&dir.0, Vec::new);
    kept.flush();
    drop(kept);
    let files: Vec<Vec<u8>> = std::fs::read_dir(&dir.0)
        .unwrap()
        .flatten()
        .filter_map(|e| std::fs::read(e.path()).ok())
        .collect();
    assert!(
        files.iter().any(|bytes| *bytes == first),
        "the copy put aside first is still in the folder"
    );
}

/// As p1b, with a later version's manifest written whole, so that it
/// reaches the version check (`m.version <= VERSION`), not a missing field.
#[test]
fn p1c_a_later_versions_manifest_leaves_the_songs_at_the_next_start() {
    let dir = Temp::new("p1c");
    let key = sealed::new_key();
    {
        let mut store = Store::open(&dir.0, key).unwrap();
        keep(&mut store, "1", b"music", 10);
    }
    std::fs::write(
        dir.0.join("manifest.json"),
        br#"{"version":99,"items":[],"confirmed":{}}"#,
    )
    .unwrap();
    {
        let mut store = Store::open(&dir.0, key).unwrap();
        store
            .subscription(ProviderKind::AppleMusic, true, 20)
            .unwrap();
    }
    drop(Store::open(&dir.0, key).unwrap());
    assert_eq!(
        sealed_files(&dir.0),
        1,
        "the song's file must still be there at the next start"
    );
}
