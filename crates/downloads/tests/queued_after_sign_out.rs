//! `queue::Downloads::forget` (M1.15, L6): "its songs waiting or coming are called off". A song
//! coming is told `Gone` once its fetch ends (`work`); a song waiting is dropped from the queue,
//! and its listener is never told: the badge it showed, `Queued`, stays. `remove` tells `Gone` for
//! each song it calls off.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use downloads::queue::{Downloading, Downloads, Fetched, Fetcher, KeptSong, Listener};
use downloads::store::Store;
use opium_core::model::{ItemId, ProviderKind, Track};

/// A fetch that never ends: the first song stays coming, the others waiting behind it.
struct Stalled;

#[async_trait::async_trait]
impl Fetcher for Stalled {
    async fn fetch(&self, _: Track, _: bool) -> Result<Fetched, String> {
        std::future::pending().await
    }
}

#[derive(Default)]
struct Heard(Mutex<Vec<(ItemId, Downloading)>>);

impl Listener for Heard {
    fn state(&self, id: ItemId, state: Downloading) {
        self.0.lock().unwrap().push((id, state));
    }
    fn kept(&self, _: Vec<KeptSong>) {}
    fn failed(&self, _: String) {}
}

impl Heard {
    /// The last state told of `id`.
    fn last(&self, id: &ItemId) -> Option<Downloading> {
        let heard = self.0.lock().unwrap();
        heard
            .iter()
            .rev()
            .find(|(of, _)| of == id)
            .map(|(_, state)| state.clone())
    }
}

fn song(id: &str) -> Track {
    Track {
        id: ItemId::new(ProviderKind::SoundCloud, id),
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

struct Dir(std::path::PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn songs_waiting_are_told_gone_after_a_sign_out() {
    let dir =
        Dir(std::env::temp_dir().join(format!("downloads-queued-sign-out-{}", std::process::id())));
    let heard = Arc::new(Heard::default());
    let queue = Downloads::new(Arc::new(Stalled), heard.clone());
    queue.opened(Ok(
        Store::open(&dir.0, downloads::sealed::new_key()).expect("the store")
    ));

    let (coming, waiting) = (song("1"), [song("2"), song("3")]);
    queue.add(
        std::iter::once(coming.clone())
            .chain(waiting.clone())
            .collect(),
        false,
    );
    // The first is coming (its fetch never ends), the others wait.
    let deadline = Instant::now() + Duration::from_secs(5);
    while heard.last(&coming.id) != Some(Downloading::Coming(0.0)) {
        assert!(
            Instant::now() < deadline,
            "the first song never started coming"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Signed out of SoundCloud.
    queue.forget(ProviderKind::SoundCloud);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let left: Vec<(String, Option<Downloading>)> = waiting
        .iter()
        .map(|t| (t.id.id.clone(), heard.last(&t.id)))
        .filter(|(_, state)| *state != Some(Downloading::Gone))
        .collect();
    assert!(
        left.is_empty(),
        "songs called off by the sign-out still show as {left:?}: no download will come"
    );
}
