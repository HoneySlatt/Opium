//! Edge cases found by task m18-edges, at commit 46e7a5a. Each test states
//! what should hold and fails today. Those named `replica_` copy private
//! lines verbatim, cited; the others call the crates' public items.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use downloads::queue::{self, Downloading, Downloads, Fetched, Fetcher, KeptSong, Listener};
use downloads::sealed;
use downloads::store::{PROTECTED_FOR, Song, Standing, Store};
use opium_core::model::{Artwork, AudioQuality, Codec, ItemId, ProviderKind, Track};

/// A folder of the test's own under the temporary folder, gone afterwards.
struct Dir(PathBuf);
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn dir(name: &str) -> Dir {
    static N: AtomicU64 = AtomicU64::new(0);
    Dir(std::env::temp_dir().join(format!(
        "m18-edges-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    )))
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

fn song(provider: ProviderKind, id: &str, protected: bool) -> Song {
    Song {
        track: track(provider, id),
        extension: "m4a".into(),
        priming: 2112,
        protected,
        kbps: Some(256),
        spatial: false,
    }
}

fn keep(store: &mut Store, song: Song, data: &[u8], now: u64) {
    let mut pending = store.begin().unwrap();
    pending.write_all(data).unwrap();
    store.keep(pending, song, now).unwrap();
}

fn sealed_files(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".sealed"))
        .count()
}

/// crates/downloads/src/store.rs:116-150: a manifest that doesn't read is
/// put aside and the songs left alone, but the first save writes a manifest
/// naming none of them, and the next open sweeps them all.
#[test]
fn downloads_a_manifest_put_aside_keeps_its_songs_through_the_next_starts() {
    let d = dir("put-aside");
    let key = sealed::new_key();
    {
        let mut store = Store::open(&d.0, key).unwrap();
        keep(
            &mut store,
            song(ProviderKind::Navidrome, "1", false),
            b"music",
            10,
        );
    }
    std::fs::write(d.0.join("manifest.json"), b"{ damaged").unwrap();
    {
        // The next start: the manifest put aside, the song left alone...
        let mut store = Store::open(&d.0, key).unwrap();
        assert!(d.0.join("manifest.corrupt.json").exists());
        assert_eq!(sealed_files(&d.0), 1, "left alone at the first start");
        // ...then anything saved, as a confirmed subscription is: Apple's at each
        // connection (crates/desktop/src/backend.rs:2172-2173), or each service's on the phone.
        // A SoundCloud confirmation stands in for it here: the store does not tell them apart.
        store
            .subscription(ProviderKind::SoundCloud, true, 20)
            .unwrap();
    }
    // The start after.
    Store::open(&d.0, key).unwrap();
    assert_eq!(
        sealed_files(&d.0),
        1,
        "the song the damaged manifest named was swept at the start after"
    );
}

/// crates/downloads/src/store.rs:201-209: whether a protected song plays is
/// judged on the wall clock alone, so a clock set back makes an expired
/// song play again, offline, past the 30 days.
#[test]
fn downloads_a_clock_set_back_doesnt_bring_an_expired_song_back() {
    let d = dir("clock-back");
    let mut store = Store::open(&d.0, sealed::new_key()).unwrap();
    let confirmed = 1_800_000_000; // 2027-01-15
    keep(
        &mut store,
        song(ProviderKind::AppleMusic, "1", true),
        b"x",
        confirmed,
    );
    let id = ItemId::new(ProviderKind::AppleMusic, "1");
    let expired = confirmed + PROTECTED_FOR;
    assert!(
        store.open_song(&id, expired).is_none(),
        "30 days on, offline: it no longer plays"
    );
    let set_back = expired - 365 * 24 * 60 * 60;
    assert!(
        store.open_song(&id, set_back).is_none(),
        "the clock set back a year: it plays again"
    );
}

struct NoSongs;
#[async_trait::async_trait]
impl Fetcher for NoSongs {
    async fn fetch(&self, _: Track, _: bool) -> Result<Fetched, String> {
        Err("none".into())
    }
}

#[derive(Default)]
struct Told(Mutex<Vec<Vec<KeptSong>>>);
impl Listener for Told {
    fn state(&self, _: ItemId, _: Downloading) {}
    fn kept(&self, kept: Vec<KeptSong>) {
        self.0.lock().unwrap().push(kept);
    }
    fn failed(&self, _: String) {}
}

/// crates/downloads/src/queue.rs:234-239 drops a subscription said before
/// the store is opened; the desktop says SoundCloud's at every start right
/// after asking the keyring for the key (crates/desktop/src/backend.rs:1161-1169,
/// backend/downloading.rs:117-131).
#[test]
fn downloads_a_subscription_said_before_the_store_opens_still_counts() {
    let d = dir("before-open");
    let key = sealed::new_key();
    let long_ago = queue::now() - 40 * 24 * 60 * 60;
    {
        let mut store = Store::open(&d.0, key).unwrap();
        keep(
            &mut store,
            song(ProviderKind::SoundCloud, "1", true),
            b"x",
            long_ago,
        );
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _inside = runtime.enter();
    let told = Arc::new(Told::default());
    let downloads = Downloads::new(Arc::new(NoSongs), told.clone());
    // Signed in to SoundCloud, at start: said while the store still opens.
    downloads.subscription(ProviderKind::SoundCloud, true);
    downloads.opened(Ok(Store::open(&d.0, key).unwrap()));
    let kept = told.0.lock().unwrap().last().cloned().unwrap();
    assert!(
        matches!(kept[0].standing, Standing::PlaysUntil(_)),
        "standing {:?}: the start's confirmation was dropped",
        kept[0].standing
    );
}

/// crates/synced/src/lib.rs:631-655 makes a file name of a playlist's id,
/// which another device chooses: `../` takes it out of the covers' folder.
#[test]
fn synced_a_playlist_id_from_another_device_stays_in_the_covers_folder() {
    let d = dir("traversal");
    let kept = synced::Kept::open(&d.0, Vec::new);
    let theirs: synced::Shared = serde_json::from_value(serde_json::json!({
        "playlists": {"lists": {"../escaped": {
            "name": {"at": {"at": 1, "device": "x"}, "value": "P"},
            "cover": {"at": {"at": 1, "device": "x"}, "value": "AAAA"}
        }}}
    }))
    .unwrap();
    assert!(kept.merge(theirs));
    let _ = kept.playlists();
    let outside: Vec<String> = std::fs::read_dir(&d.0)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("escaped-"))
        .collect();
    assert!(outside.is_empty(), "written outside covers/: {outside:?}");
}

fn apple_track(n: u32) -> Track {
    Track {
        id: ItemId::new(ProviderKind::AppleMusic, format!("i.{n:014}")),
        title: format!("A song title of usual length {n}"),
        artists: vec!["An Artist".into()],
        album: Some("An Album Of Usual Length".into()),
        album_id: Some(ItemId::new(ProviderKind::AppleMusic, format!("l.{:014}", n / 12))),
        duration: Some(Duration::from_millis(215_000)),
        track_number: Some(n % 12 + 1),
        disc_number: Some(1),
        artwork: Some(Artwork {
            url_template: "https://is1-ssl.mzstatic.com/image/thumb/Music116/v4/3b/2a/1c/3b2a1c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d/196871234567.jpg/{size}x{size}bb.jpg".into(),
            background: Some(0x1a2b3c),
        }),
        explicit: false,
        quality: Some(AudioQuality {
            codec: Codec::Aac,
            sample_rate: Some(44_100),
            bit_depth: None,
            bitrate_kbps: Some(256),
        }),
        in_library: true,
        library_id: Some(ItemId::new(ProviderKind::AppleMusic, format!("i.{n:014}"))),
    }
}

/// crates/connect/src/wire.rs:18 caps a message at 32 MiB, and a device
/// meeting another sends every playlist that differs as one message
/// (crates/connect/src/lib.rs:1139-1143, 1003-1008): measured here.
#[test]
fn connect_the_playlists_of_a_large_library_fit_one_message() {
    const MAX_SEALED: usize = 32 * 1024 * 1024;
    let d = dir("size");
    let kept = synced::Kept::open(&d.0, Vec::new);
    let songs: u32 = 60_000;
    for p in 0..songs / 1000 {
        kept.create_playlist(
            format!("Playlist {p}"),
            String::new(),
            (p * 1000..(p + 1) * 1000).map(apple_track).collect(),
        );
    }
    // As `Connect::encode` makes it: `{"Shared":…}`.
    let bytes = serde_json::to_vec(&serde_json::json!({ "Shared": kept.everything() }))
        .unwrap()
        .len();
    println!(
        "{songs} songs in Opium's playlists: {bytes} bytes, {:.0} a song; the cap of {MAX_SEALED} reached at about {} songs",
        bytes as f64 / f64::from(songs),
        MAX_SEALED as u64 * u64::from(songs) / bytes as u64
    );
    assert!(bytes <= MAX_SEALED, "{bytes} bytes > {MAX_SEALED}");
}

/// The two fields of `desktop::settings::Settings` at stake, typed and
/// defaulted as there (crates/desktop/src/settings.rs:17-19, 48-49, 81).
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct LikeSettings {
    volume: Option<f32>,
    local_folders: Vec<PathBuf>,
}

/// `Settings::save` (crates/desktop/src/settings.rs:188-193) writes nothing
/// when `toml::to_string` fails, which it does for a folder whose name
/// isn't UTF-8 — and `desktop::file_path` keeps such names (lib.rs:44-67).
#[test]
fn settings_a_folder_whose_name_isnt_utf8_can_be_kept() {
    use std::os::unix::ffi::OsStringExt;
    let settings = LikeSettings {
        volume: Some(0.3),
        local_folders: vec![std::ffi::OsString::from_vec(b"/music/caf\xe9".to_vec()).into()],
    };
    let written = toml::to_string(&settings);
    assert!(
        written.is_ok(),
        "toml refuses the whole file: {:?}",
        written.err().map(|e| e.to_string())
    );
}

/// `volume = nan` reads as NaN, which `clamp` keeps: the start-up path
/// (crates/app-desktop/src/app.rs:1035, 1049-1050) and `Player::set_volume`
/// (crates/player/src/lib.rs:351-353) both pass it on to the output.
#[test]
fn settings_a_volume_that_isnt_a_number_reaches_the_player_as_one() {
    let read: LikeSettings = toml::from_str("volume = nan\n").unwrap();
    let held = read.volume.unwrap_or(1.0).clamp(0.0, 1.0);
    assert!(held.is_finite(), "the volume handed to the player is {held}");
}

/// The gain's step, verbatim from crates/player/src/output.rs:165-176
/// (`read_into`), with the target a NaN volume makes: one second at
/// 48 kHz, from a pause (gain 0, output.rs:392-396).
#[test]
fn replica_player_a_nan_volume_runs_the_gain_away() {
    let target = f32::NAN.clamp(0.0, 1.0);
    let step = 1.0 / (0.005 * 48_000.0); // output.rs:62, 84
    let mut gain = 0.0f32;
    for _ in 0..48_000 {
        if gain != target {
            gain = if gain < target {
                (gain + step).min(target)
            } else {
                (gain - step).max(target)
            };
        }
    }
    assert!(
        (0.0..=1.0).contains(&gain),
        "after one second the gain is {gain}: each sample is multiplied by it"
    );
}

/// `keep_cover`, verbatim from crates/local/src/scan.rs:320-330 (private):
/// the scan reads an album's tracks on up to eight threads at once
/// (scan.rs:201-221), each with the same picture, all writing one `.part`.
fn replica_keep_cover(bytes: &[u8], covers: &Path) -> Option<String> {
    let path = covers.join(format!("{:016x}.jpg", fnv(bytes)));
    if !path.exists() {
        let square = opium_core::cover::square(bytes)?;
        std::fs::create_dir_all(covers).ok()?;
        let partial = path.with_extension("part");
        std::fs::write(&partial, square).ok()?;
        std::fs::rename(&partial, &path).ok()?;
    }
    Some(path.to_string_lossy().into_owned())
}

/// The hash `keep_cover` names a picture by (crates/local/src/scan.rs), for
/// the replica: any stable hash shows the race alike.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[test]
fn replica_local_the_same_cover_kept_by_eight_threads_is_kept_for_each() {
    // A cover as albums carry them: larger than the 600 px kept.
    let png = image_png(640);
    // Control: alone, the cover is kept.
    let alone = dir("covers-alone");
    assert!(replica_keep_cover(&png, &alone.0).is_some(), "the replica keeps a cover alone");
    let mut without = 0;
    let rounds = 20;
    for _ in 0..rounds {
        let covers = dir("covers");
        let start = std::sync::Barrier::new(8);
        let kept: Vec<Option<String>> = std::thread::scope(|s| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    s.spawn(|| {
                        start.wait();
                        replica_keep_cover(&png, &covers.0)
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        without += kept.iter().filter(|k| k.is_none()).count();
    }
    assert_eq!(
        without, 0,
        "{without} of {} tracks left without their cover",
        rounds * 8
    );
}

/// A PNG `side` pixels square.
fn image_png(side: u32) -> Vec<u8> {
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_fn(side, side, |x, y| image::Rgb([(x % 256) as u8, (y % 256) as u8, 90]))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    png.into_inner()
}

/// `opium_core::text::fold` (crates/opium-core/src/text.rs:5-20) maps only
/// letters written whole: the same title written decomposed, as file names
/// from a Mac and some taggers have it, folds to another key, so search,
/// the best copy (dedupe.rs:45-52) and a file's album (local/src/lib.rs:278)
/// tell the two apart.
#[test]
fn text_a_title_written_decomposed_folds_as_the_composed_one() {
    let composed = "Caf\u{e9}";
    let decomposed = "Cafe\u{301}";
    assert_eq!(
        opium_core::text::fold(decomposed),
        opium_core::text::fold(composed),
        "\"cafe\" finds one and not the other"
    );
}

/// crates/downloads/src/store.rs:272-275 removes the song's old file before
/// the manifest naming the new one is saved: when that save fails (a full
/// disk; here its temporary name is taken by a folder), the next start has
/// a manifest naming the old file, gone, and sweeps the new one.
#[test]
fn downloads_a_song_downloaded_again_survives_a_manifest_that_cant_be_written() {
    let d = dir("keep-again");
    let key = sealed::new_key();
    let id = ItemId::new(ProviderKind::Navidrome, "1");
    let mut store = Store::open(&d.0, key).unwrap();
    keep(
        &mut store,
        song(ProviderKind::Navidrome, "1", false),
        b"first",
        10,
    );
    std::fs::create_dir(d.0.join("manifest.json.part")).unwrap();
    let mut pending = store.begin().unwrap();
    pending.write_all(b"again").unwrap();
    assert!(
        store
            .keep(pending, song(ProviderKind::Navidrome, "1", false), 20)
            .is_err(),
        "the save fails"
    );
    std::fs::remove_dir(d.0.join("manifest.json.part")).unwrap();
    drop(store);
    let store = Store::open(&d.0, key).unwrap();
    assert!(
        store.open_song(&id, 30).is_some_and(|r| r.is_ok()),
        "neither copy of the song is left ({} sealed files)",
        sealed_files(&d.0)
    );
}
