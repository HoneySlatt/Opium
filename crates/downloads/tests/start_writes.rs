//! The phone's start, as `android/…/data/Downloads.kt` makes it: the core opens the store
//! (`opium-ffi`'s `Downloads::open`, which calls `queue::Downloads::opened`), then Kotlin's
//! `StoreOrders::opened` tells the store a sign-out of each service with a subscription that is
//! not signed in (`WITH_SUBSCRIPTION.filterNot(core::signedIn)`), through `Downloads::forget`.
//!
//! `docs/debt.md` says the manifest is written "on an async worker once a start". Each write of
//! the manifest goes through `write_whole`, which renames a new file over the old one, so the
//! manifest's inode changes at each write: this test counts them.

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use downloads::queue::{Downloading, Downloads, Fetched, Fetcher, KeptSong, Listener};
use downloads::store::Store;
use opium_core::model::{ItemId, ProviderKind, Track};

struct NoFetch;

#[async_trait::async_trait]
impl Fetcher for NoFetch {
    async fn fetch(&self, _: Track, _: bool) -> Result<Fetched, String> {
        Err("no fetch in this test".into())
    }
}

struct Deaf;

impl Listener for Deaf {
    fn state(&self, _: ItemId, _: Downloading) {}
    fn kept(&self, _: Vec<KeptSong>) {}
    fn failed(&self, _: String) {}
}

struct Dir(PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn inode(path: &Path) -> u64 {
    std::fs::metadata(path).expect("the manifest").ino()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_phone_start_writes_the_manifest_once() {
    let dir =
        Dir(std::env::temp_dir().join(format!("downloads-start-writes-{}", std::process::id())));
    let manifest = dir.0.join("manifest.json");
    let store = Store::open(&dir.0, downloads::sealed::new_key()).expect("the store");
    let queue = Downloads::new(Arc::new(NoFetch), Arc::new(Deaf));

    // The core opens the store: its one write of the start (`note_time`).
    queue.opened(Ok(store));
    let after_open = inode(&manifest);

    // Kotlin then tells it that neither Apple Music nor SoundCloud is signed in, as for a user of
    // Navidrome alone: no subscription was ever confirmed, so nothing changes in the store.
    queue.forget(ProviderKind::AppleMusic);
    let after_apple = inode(&manifest);
    queue.forget(ProviderKind::SoundCloud);
    let after_soundcloud = inode(&manifest);

    let writes_after_open =
        usize::from(after_apple != after_open) + usize::from(after_soundcloud != after_apple);
    assert_eq!(
        writes_after_open, 0,
        "the manifest was written {writes_after_open} more times, each with a flush of the file and \
         of its folder, to note sign-outs of services the store never confirmed"
    );
}
