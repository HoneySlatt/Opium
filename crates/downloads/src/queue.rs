//! Songs downloaded one at a time, the next one fetched while one is read
//! into the store: what each app shares, its own [`Fetcher`] bringing the
//! songs as playing them would, its [`Listener`] told how they go.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use opium_core::model::{ItemId, ProviderKind, Track};
use tokio::runtime::Handle;

use crate::store::{Open, Pending, Song, Standing, Store};

/// How a song stands as a download, for its badge.
#[derive(Debug, Clone, PartialEq)]
pub enum Downloading {
    Queued,
    /// How much has come, from 0 to 1.
    Coming(f32),
    Kept(Standing),
    Failed(String),
    /// Not kept, or no longer.
    Gone,
}

/// A song kept, for the library's Downloaded.
#[derive(Debug, Clone)]
pub struct KeptSong {
    pub track: Track,
    pub standing: Standing,
    pub added: u64,
}

/// A song fetched, to read into the store.
pub struct Fetched {
    pub reader: Box<dyn Read + Send>,
    pub len: u64,
    pub song: Song,
}

/// Brings a song as playing it would: the app's services.
#[async_trait::async_trait]
pub trait Fetcher: Send + Sync + 'static {
    async fn fetch(&self, track: Track, original: bool) -> Result<Fetched, String>;
}

/// Told how the downloads go.
pub trait Listener: Send + Sync + 'static {
    fn state(&self, id: ItemId, state: Downloading);
    /// Every song kept, after one came or went, or the subscription said.
    fn kept(&self, kept: Vec<KeptSong>);
    fn failed(&self, message: String);
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[derive(Default)]
struct Queue {
    tracks: VecDeque<(Track, bool)>,
    running: bool,
    /// Those coming (the one read, the next one fetched meanwhile), and
    /// whether each was called off.
    active: Vec<(ItemId, bool)>,
}

impl Queue {
    /// Whether `id` was called off, or isn't coming at all.
    fn called_off(&self, id: &ItemId) -> bool {
        self.active
            .iter()
            .find(|(a, _)| a == id)
            .is_none_or(|(_, off)| *off)
    }
}

type Fetching = tokio::task::JoinHandle<Result<Fetched, String>>;

/// The store, as the downloads hold it.
enum StoreState {
    /// Still opening: what the services said of their subscriptions
    /// meanwhile, in order, each with whether it is active and when, in
    /// seconds since the epoch, told the store once it opens.
    Opening(Vec<(ProviderKind, bool, u64)>),
    Open(Store),
    /// It couldn't be opened: why.
    Failed(String),
}

#[derive(Clone)]
pub struct Downloads {
    store: Arc<Mutex<StoreState>>,
    queue: Arc<Mutex<Queue>>,
    fetcher: Arc<dyn Fetcher>,
    listener: Arc<dyn Listener>,
    /// Where the work runs: the runtime made in.
    runtime: Handle,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Downloads {
    /// Made within a tokio runtime, where the downloads then run; the store
    /// is [`opened`](Self::opened) after.
    pub fn new(fetcher: Arc<dyn Fetcher>, listener: Arc<dyn Listener>) -> Self {
        Self {
            store: Arc::new(Mutex::new(StoreState::Opening(Vec::new()))),
            queue: Arc::default(),
            fetcher,
            listener,
            runtime: Handle::current(),
        }
    }

    /// The store opened, or why it couldn't be: what it keeps is told.
    /// Opened, it is told the subscriptions said meanwhile, in order, and
    /// the time, which its manifest keeps (a failed save is printed).
    pub fn opened(&self, store: Result<Store, String>) {
        {
            let mut state = lock(&self.store);
            let said = match &mut *state {
                StoreState::Opening(said) => std::mem::take(said),
                _ => Vec::new(),
            };
            *state = match store {
                Ok(mut store) => {
                    for (provider, active, at) in said {
                        store.note_subscription(provider, active, at);
                    }
                    // One write, on the caller's thread (an async worker),
                    // once per start: the exception to `keep_progress_soon`'s
                    // rule, which the debt records.
                    if let Err(e) = store.note_time(now()) {
                        eprintln!("downloads: couldn't save the manifest: {e}");
                    }
                    StoreState::Open(store)
                }
                Err(e) => StoreState::Failed(e),
            };
        }
        self.tell();
    }

    /// What is kept, to the listener. The time it counts, when the manifest
    /// should keep it, is written on a blocking thread ([`Store::standing`]
    /// says when).
    pub fn tell(&self) {
        let now = now();
        let kept = match &*lock(&self.store) {
            StoreState::Open(store) => {
                let kept: Vec<KeptSong> = store
                    .items()
                    .map(|item| KeptSong {
                        track: item.track.clone(),
                        standing: store.judge(item, now),
                        added: item.added,
                    })
                    .collect();
                if store.progress_unsaved() {
                    self.keep_progress_soon();
                }
                kept
            }
            _ => Vec::new(),
        };
        self.listener.kept(kept);
    }

    /// The song kept as `id`, opened to play, if it is kept and plays. The
    /// time it counts is kept as [`Downloads::tell`] keeps it.
    pub fn open(&self, id: &ItemId) -> Option<Open> {
        let store = lock(&self.store);
        let StoreState::Open(store) = &*store else {
            return None;
        };
        let item = store.get(id)?;
        let standing = store.judge(item, now());
        if store.progress_unsaved() {
            self.keep_progress_soon();
        }
        if standing == Standing::Expired {
            return None;
        }
        match store.open_item(item) {
            Ok(open) => Some(open),
            // Damaged: it plays from the service, as if not kept.
            Err(e) => {
                eprintln!("downloads: {id}: {e}");
                None
            }
        }
    }

    /// The time the store counts written into its manifest, on a blocking
    /// thread, never the caller's: the window's or an async worker's.
    fn keep_progress_soon(&self) {
        let store = self.store.clone();
        self.runtime.spawn_blocking(move || {
            if let StoreState::Open(store) = &*lock(&store) {
                store.keep_progress();
            }
        });
    }

    /// Download `tracks`, after those waiting; `original`: a SoundCloud
    /// uploader's own file, where they allow it.
    pub fn add(&self, tracks: Vec<Track>, original: bool) {
        let start = {
            let mut queue = lock(&self.queue);
            for track in tracks {
                let id = &track.id;
                let waiting = queue.tracks.iter().any(|(t, _)| &t.id == id)
                    || queue.active.iter().any(|(a, off)| a == id && !off);
                if waiting {
                    continue;
                }
                self.listener.state(id.clone(), Downloading::Queued);
                queue.tracks.push_back((track, original));
            }
            !std::mem::replace(&mut queue.running, true)
        };
        if start {
            self.runtime.spawn(self.clone().work());
        }
    }

    /// Whether songs are waiting or coming.
    pub fn busy(&self) -> bool {
        lock(&self.queue).running
    }

    /// No longer kept, nor to be downloaded.
    pub fn remove(&self, ids: Vec<ItemId>) {
        {
            let mut queue = lock(&self.queue);
            queue.tracks.retain(|(t, _)| !ids.contains(&t.id));
            for (id, off) in &mut queue.active {
                if ids.contains(id) {
                    *off = true;
                }
            }
        }
        let (store, listener) = (self.store.clone(), self.listener.clone());
        self.runtime.spawn_blocking(move || {
            if let StoreState::Open(store) = &mut *lock(&store) {
                for id in &ids {
                    if let Err(e) = store.remove(id) {
                        listener.failed(format!("Couldn't remove the download: {e}"));
                    }
                }
            }
            for id in ids {
                listener.state(id, Downloading::Gone);
            }
        });
    }

    /// Every song gone, and none to come.
    pub fn remove_all(&self) {
        let ids: Vec<ItemId> = match &*lock(&self.store) {
            StoreState::Open(store) => store.items().map(|i| i.track.id.clone()).collect(),
            _ => Vec::new(),
        };
        let waiting: Vec<ItemId> = {
            let queue = lock(&self.queue);
            queue
                .tracks
                .iter()
                .map(|(t, _)| t.id.clone())
                .chain(queue.active.iter().map(|(id, _)| id.clone()))
                .collect()
        };
        self.remove(ids.into_iter().chain(waiting).collect());
    }

    /// Signed out of `provider`: its songs waiting or coming are called off,
    /// and its protected songs are `Expired` until its subscription is
    /// confirmed again. Every song kept stays, and those that came without
    /// DRM play on (ADR-0017).
    pub fn forget(&self, provider: ProviderKind) {
        {
            let mut queue = lock(&self.queue);
            queue.tracks.retain(|(t, _)| t.id.provider != provider);
            // Kept, a protected one would confirm the subscription again.
            for (id, off) in &mut queue.active {
                if id.provider == provider {
                    *off = true;
                }
            }
        }
        self.subscription(provider, false);
    }

    /// `provider` said whether its subscription is active: its protected
    /// songs play for 30 days more, or no longer. Said before the store
    /// opens, it is kept and told the store once it does; a failed save is
    /// printed.
    pub fn subscription(&self, provider: ProviderKind, active: bool) {
        let now = now();
        match &mut *lock(&self.store) {
            StoreState::Open(store) => {
                if let Err(e) = store.subscription(provider, active, now) {
                    eprintln!("downloads: couldn't save {provider:?}'s subscription: {e}");
                }
            }
            StoreState::Opening(said) => said.push((provider, active, now)),
            StoreState::Failed(_) => {}
        }
        self.tell();
    }

    /// The next song waiting, now coming; with `stop`, none is: the worker
    /// stops, as one added later knows.
    fn next(&self, stop: bool) -> Option<(Track, bool)> {
        let mut queue = lock(&self.queue);
        let next = queue.tracks.pop_front();
        match &next {
            Some((track, _)) => queue.active.push((track.id.clone(), false)),
            None if stop => queue.running = false,
            None => {}
        }
        next
    }

    /// Start fetching `track`; Apple's licences a second apart at least,
    /// not in a burst.
    fn fetch(&self, track: Track, original: bool, last_apple: &mut Option<Instant>) -> Fetching {
        let wait = match track.id.provider {
            ProviderKind::AppleMusic => {
                let wait = last_apple.map_or(Duration::ZERO, |at| {
                    Duration::from_secs(1).saturating_sub(at.elapsed())
                });
                *last_apple = Some(Instant::now() + wait);
                wait
            }
            _ => Duration::ZERO,
        };
        let fetcher = self.fetcher.clone();
        self.runtime.spawn(async move {
            tokio::time::sleep(wait).await;
            fetcher.fetch(track, original).await
        })
    }

    /// Download one song at a time, the next one's fetch (its licence, its
    /// first bytes) under way while one is read: a CDM, which decrypts no
    /// faster than it does, isn't left waiting between songs.
    async fn work(self) {
        let mut ahead: Option<(ItemId, Fetching)> = None;
        let mut last_apple = None;
        loop {
            let (id, fetching) = match ahead.take() {
                Some(ahead) => ahead,
                None => match self.next(true) {
                    Some((track, original)) => {
                        let id = track.id.clone();
                        (id, self.fetch(track, original, &mut last_apple))
                    }
                    None => return,
                },
            };
            self.listener.state(id.clone(), Downloading::Coming(0.0));
            let fetched = fetching.await.map_err(|e| e.to_string()).and_then(|f| f);
            if let Some((track, original)) = self.next(false) {
                ahead = Some((
                    track.id.clone(),
                    self.fetch(track, original, &mut last_apple),
                ));
            }
            let state = match fetched {
                _ if lock(&self.queue).called_off(&id) => Ok(None),
                Ok(fetched) => self.keep(fetched).await,
                Err(e) => Err(e),
            };
            lock(&self.queue).active.retain(|(a, _)| a != &id);
            let state = match state {
                Ok(Some(standing)) => Downloading::Kept(standing),
                Ok(None) => Downloading::Gone,
                Err(e) => {
                    eprintln!("downloads: {id}: {e}");
                    Downloading::Failed(e)
                }
            };
            let kept = matches!(state, Downloading::Kept(_));
            self.listener.state(id, state);
            // The library's Downloaded has it.
            if kept {
                self.tell();
            }
        }
    }

    /// Read `fetched` into the store: how it then stands, or `None` if it
    /// was called off.
    async fn keep(&self, fetched: Fetched) -> Result<Option<Standing>, String> {
        let Fetched { reader, len, song } = fetched;
        let (store, queue, listener) = (
            self.store.clone(),
            self.queue.clone(),
            self.listener.clone(),
        );
        self.runtime
            .spawn_blocking(move || {
                let pending = match &*lock(&store) {
                    StoreState::Open(store) => store.begin().map_err(|e| e.to_string())?,
                    StoreState::Failed(e) => return Err(e.clone()),
                    StoreState::Opening(_) => return Err("the downloads aren't open yet".into()),
                };
                let id = song.track.id.clone();
                let called_off = || lock(&queue).called_off(&id);
                match copy(reader, pending, len, &called_off, |coming| {
                    listener.state(id.clone(), Downloading::Coming(coming))
                })? {
                    Some(pending) => {
                        let mut store = lock(&store);
                        let StoreState::Open(store) = &mut *store else {
                            pending.abandon();
                            return Err("the downloads closed".into());
                        };
                        // Asked again under the store's lock: a sign-out or a
                        // removal since `copy`'s last check is seen here, or
                        // waits for this keep and undoes it. Kept after a
                        // sign-out, a protected song would confirm the
                        // subscription again.
                        if called_off() {
                            pending.abandon();
                            return Ok(None);
                        }
                        let now = now();
                        let item = store
                            .keep(pending, song, now)
                            .map_err(|e| format!("Couldn't keep the download: {e}"))?
                            .clone();
                        Ok(Some(store.standing(&item, now)))
                    }
                    None => Ok(None),
                }
            })
            .await
            .map_err(|e| e.to_string())?
    }
}

/// Read `reader` to its end into `pending`, telling how far it came now
/// and then; `None` if it was called off meanwhile.
fn copy(
    mut reader: Box<dyn Read + Send>,
    mut pending: Pending,
    len: u64,
    called_off: &dyn Fn() -> bool,
    mut coming: impl FnMut(f32),
) -> Result<Option<Pending>, String> {
    let mut buf = vec![0; 256 * 1024];
    let mut received = 0u64;
    let mut told = Instant::now();
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                pending.abandon();
                return Err(e.to_string());
            }
        };
        if let Err(e) = pending.write_all(&buf[..n]) {
            pending.abandon();
            return Err(format!("Couldn't write the download: {e}"));
        }
        received += n as u64;
        if told.elapsed() >= Duration::from_millis(250) {
            if called_off() {
                pending.abandon();
                return Ok(None);
            }
            coming((received as f64 / len.max(1) as f64) as f32);
            told = Instant::now();
        }
    }
    if received != len {
        pending.abandon();
        return Err(format!("the download ended at {received} of {len} bytes"));
    }
    if called_off() {
        pending.abandon();
        return Ok(None);
    }
    Ok(Some(pending))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sealed;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn track(id: &str) -> Track {
        Track {
            id: ItemId::new(ProviderKind::Navidrome, id),
            title: id.into(),
            artists: vec![],
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

    /// A song of the id's length in bytes, or a failure for `bad`.
    struct Songs;
    #[async_trait::async_trait]
    impl Fetcher for Songs {
        async fn fetch(&self, track: Track, _: bool) -> Result<Fetched, String> {
            if track.id.id == "bad" {
                return Err("refused".into());
            }
            let len: usize = track.id.id.parse().unwrap();
            Ok(Fetched {
                reader: Box::new(std::io::Cursor::new(vec![7u8; len])),
                len: len as u64,
                song: Song {
                    track,
                    extension: "flac".into(),
                    priming: 0,
                    protected: false,
                    kbps: None,
                    spatial: false,
                },
            })
        }
    }

    #[derive(Default)]
    struct Told(Mutex<Vec<(ItemId, Downloading)>>);
    impl Listener for Told {
        fn state(&self, id: ItemId, state: Downloading) {
            lock(&self.0).push((id, state));
        }
        fn kept(&self, _: Vec<KeptSong>) {}
        fn failed(&self, _: String) {}
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn downloads_each_song_into_the_store_in_turn() {
        let dir = std::env::temp_dir().join(format!("queue-{:016x}", rand::random::<u64>()));
        let told = Arc::new(Told::default());
        let downloads = Downloads::new(Arc::new(Songs), told.clone());
        downloads.opened(Ok(Store::open(&dir, sealed::new_key()).unwrap()));
        downloads.add(vec![track("10"), track("bad"), track("300000")], false);
        while downloads.busy() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let ends: Vec<(String, bool)> = lock(&told.0)
            .iter()
            .filter(|(_, s)| matches!(s, Downloading::Kept(_) | Downloading::Failed(_)))
            .map(|(id, s)| (id.id.clone(), matches!(s, Downloading::Kept(_))))
            .collect();
        assert_eq!(
            ends,
            [
                ("10".into(), true),
                ("bad".into(), false),
                ("300000".into(), true)
            ]
        );
        let open = downloads.open(&track("300000").id).unwrap();
        assert_eq!(open.len, 300000);
        let _ = std::fs::remove_dir_all(dir);
    }

    // The stage 1 audit's W6: a confirmation said before the store opens.

    #[derive(Default)]
    struct KeptTold(Mutex<Vec<Vec<KeptSong>>>);
    impl Listener for KeptTold {
        fn state(&self, _: ItemId, _: Downloading) {}
        fn kept(&self, kept: Vec<KeptSong>) {
            lock(&self.0).push(kept);
        }
        fn failed(&self, _: String) {}
    }

    #[tokio::test]
    async fn a_subscription_said_before_the_store_opens_still_counts() {
        let dir = std::env::temp_dir().join(format!("queue-{:016x}", rand::random::<u64>()));
        let key = sealed::new_key();
        let long_ago = now() - 40 * 24 * 60 * 60;
        {
            let mut store = Store::open(&dir, key).unwrap();
            let mut pending = store.begin().unwrap();
            pending.write_all(b"x").unwrap();
            let track = Track {
                id: ItemId::new(ProviderKind::SoundCloud, "1"),
                ..track("1")
            };
            let song = Song {
                track,
                extension: "m4a".into(),
                priming: 0,
                protected: true,
                kbps: None,
                spatial: false,
            };
            store.keep(pending, song, long_ago).unwrap();
        }
        let told = Arc::new(KeptTold::default());
        let downloads = Downloads::new(Arc::new(Songs), told.clone());
        // Signed in to SoundCloud at start, the keyring still asked for the key.
        downloads.subscription(ProviderKind::SoundCloud, true);
        downloads.opened(Ok(Store::open(&dir, key).unwrap()));
        let kept = lock(&told.0).last().cloned().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            matches!(kept[0].standing, Standing::PlaysUntil(_)),
            "{:?}",
            kept[0].standing
        );
    }

    fn soundcloud_track(id: &str) -> Track {
        Track {
            id: ItemId::new(ProviderKind::SoundCloud, id),
            ..track(id)
        }
    }

    fn soundcloud_song(id: &str, protected: bool) -> Song {
        Song {
            track: soundcloud_track(id),
            extension: "m4a".into(),
            priming: 0,
            protected,
            kbps: None,
            spatial: false,
        }
    }

    /// Keep a SoundCloud song as `id` in the store in `dir`, now.
    fn keep_soundcloud(dir: &std::path::Path, key: sealed::Key, id: &str, protected: bool) {
        let mut store = Store::open(dir, key).unwrap();
        let mut pending = store.begin().unwrap();
        pending.write_all(b"x").unwrap();
        store
            .keep(pending, soundcloud_song(id, protected), now())
            .unwrap();
    }

    /// How the song kept as `id` stood when the listener was last told.
    fn standing(told: &KeptTold, id: &str) -> Option<Standing> {
        lock(&told.0)
            .last()?
            .iter()
            .find(|kept| kept.track.id.id == id)
            .map(|kept| kept.standing)
    }

    /// ADR-0017: "An ended subscription, or a sign-out, makes those songs
    /// `Expired`. Songs that came without DRM play always." The stage 1
    /// audit's L6: a sign-out deleted them all.
    #[tokio::test]
    async fn a_sign_out_keeps_the_songs_and_stops_only_the_protected_ones() {
        let dir = std::env::temp_dir().join(format!("queue-{:016x}", rand::random::<u64>()));
        let key = sealed::new_key();
        keep_soundcloud(&dir, key, "1", true);
        keep_soundcloud(&dir, key, "2", false);
        let told = Arc::new(KeptTold::default());
        let downloads = Downloads::new(Arc::new(Songs), told.clone());
        downloads.opened(Ok(Store::open(&dir, key).unwrap()));
        let protected = ItemId::new(ProviderKind::SoundCloud, "1");
        let without_drm = ItemId::new(ProviderKind::SoundCloud, "2");
        assert!(matches!(
            standing(&told, "1"),
            Some(Standing::PlaysUntil(_))
        ));

        downloads.forget(ProviderKind::SoundCloud);
        assert_eq!(standing(&told, "1"), Some(Standing::Expired));
        assert_eq!(standing(&told, "2"), Some(Standing::Plays));
        assert!(downloads.open(&protected).is_none());
        assert!(downloads.open(&without_drm).is_some());
        let sealed = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".sealed"))
            .count();
        assert_eq!(sealed, 2, "both songs' files are still there");

        // Signed in again: the protected song plays again.
        downloads.subscription(ProviderKind::SoundCloud, true);
        assert!(matches!(
            standing(&told, "1"),
            Some(Standing::PlaysUntil(_))
        ));
        assert!(downloads.open(&protected).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// What is said before the store opens is told it in order: a sign-out
    /// after a confirmation leaves the protected songs `Expired`.
    #[tokio::test]
    async fn a_sign_out_said_before_the_store_opens_still_counts() {
        let dir = std::env::temp_dir().join(format!("queue-{:016x}", rand::random::<u64>()));
        let key = sealed::new_key();
        keep_soundcloud(&dir, key, "1", true);
        let told = Arc::new(KeptTold::default());
        let downloads = Downloads::new(Arc::new(Songs), told.clone());
        downloads.subscription(ProviderKind::SoundCloud, true);
        downloads.forget(ProviderKind::SoundCloud);
        downloads.opened(Ok(Store::open(&dir, key).unwrap()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(standing(&told, "1"), Some(Standing::Expired));
    }

    /// A protected SoundCloud song of ten bytes, brought once `release` is
    /// set.
    #[derive(Default)]
    struct Held {
        fetching: AtomicBool,
        release: AtomicBool,
    }
    #[async_trait::async_trait]
    impl Fetcher for Held {
        async fn fetch(&self, track: Track, _: bool) -> Result<Fetched, String> {
            self.fetching.store(true, Ordering::SeqCst);
            while !self.release.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            Ok(Fetched {
                reader: Box::new(std::io::Cursor::new(vec![7u8; 10])),
                len: 10,
                song: soundcloud_song(&track.id.id, true),
            })
        }
    }

    /// A protected song still coming at a sign-out is called off: kept, it
    /// would confirm the subscription again, and every protected song of the
    /// service would play on.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_sign_out_calls_off_the_services_songs_coming() {
        let dir = std::env::temp_dir().join(format!("queue-{:016x}", rand::random::<u64>()));
        let key = sealed::new_key();
        keep_soundcloud(&dir, key, "1", true);
        let fetcher = Arc::new(Held::default());
        let told = Arc::new(KeptTold::default());
        let downloads = Downloads::new(fetcher.clone(), told.clone());
        downloads.opened(Ok(Store::open(&dir, key).unwrap()));
        downloads.add(vec![soundcloud_track("2")], false);
        while !fetcher.fetching.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        downloads.forget(ProviderKind::SoundCloud);
        fetcher.release.store(true, Ordering::SeqCst);
        while downloads.busy() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(standing(&told, "1"), Some(Standing::Expired));
        assert_eq!(standing(&told, "2"), None, "it isn't kept");
    }

    /// A protected SoundCloud song whose reader signs out of SoundCloud as
    /// it is dropped: `copy` drops it on returning, after its last check
    /// whether the song was called off, and before the store is locked to
    /// keep it.
    #[derive(Default)]
    struct SignsOutAtTheEnd(Mutex<Option<Downloads>>);
    #[async_trait::async_trait]
    impl Fetcher for SignsOutAtTheEnd {
        async fn fetch(&self, track: Track, _: bool) -> Result<Fetched, String> {
            let downloads = lock(&self.0).clone().ok_or("no downloads")?;
            Ok(Fetched {
                reader: Box::new(SignsOut {
                    bytes: std::io::Cursor::new(vec![7u8; 10]),
                    downloads,
                }),
                len: 10,
                song: soundcloud_song(&track.id.id, true),
            })
        }
    }
    struct SignsOut {
        bytes: std::io::Cursor<Vec<u8>>,
        downloads: Downloads,
    }
    impl Read for SignsOut {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.bytes.read(buf)
        }
    }
    impl Drop for SignsOut {
        fn drop(&mut self) {
            self.downloads.forget(ProviderKind::SoundCloud);
        }
    }

    /// A sign-out between the last check and the keep: the song isn't kept,
    /// so it doesn't confirm the subscription again.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_song_kept_as_its_service_is_signed_out_doesnt_confirm_it_again() {
        let dir = std::env::temp_dir().join(format!("queue-{:016x}", rand::random::<u64>()));
        let key = sealed::new_key();
        keep_soundcloud(&dir, key, "1", true);
        let fetcher = Arc::new(SignsOutAtTheEnd::default());
        let told = Arc::new(KeptTold::default());
        let downloads = Downloads::new(fetcher.clone(), told.clone());
        *lock(&fetcher.0) = Some(downloads.clone());
        downloads.opened(Ok(Store::open(&dir, key).unwrap()));
        downloads.add(vec![soundcloud_track("2")], false);
        while downloads.busy() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        lock(&fetcher.0).take();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(standing(&told, "1"), Some(Standing::Expired));
        assert_eq!(standing(&told, "2"), None, "it isn't kept");
    }

    /// An expiry the downloads find reaches the manifest, so that a restart
    /// with the clock set back finds it too: the time counted, crossing the
    /// 30 days, written from a blocking thread.
    #[tokio::test(flavor = "multi_thread")]
    async fn an_expiry_the_downloads_find_reaches_the_manifest() {
        let dir = std::env::temp_dir().join(format!("queue-{:016x}", rand::random::<u64>()));
        let key = sealed::new_key();
        keep_soundcloud(&dir, key, "1", true);
        keep_soundcloud(&dir, key, "2", false);
        let told = Arc::new(KeptTold::default());
        let downloads = Downloads::new(Arc::new(Songs), told.clone());
        downloads.opened(Ok(Store::open(&dir, key).unwrap()));
        let far = now() + 10 * crate::store::PROTECTED_FOR;
        {
            let state = lock(&downloads.store);
            let StoreState::Open(store) = &*state else {
                panic!("the store is open");
            };
            // The clock read far ahead, judging a song that always plays,
            // without the write `Store::standing` would make.
            let without_drm = store
                .get(&ItemId::new(ProviderKind::SoundCloud, "2"))
                .unwrap();
            assert_eq!(store.judge(without_drm, far), Standing::Plays);
        }
        downloads.tell();
        assert_eq!(standing(&told, "1"), Some(Standing::Expired));
        let kept_progress = || {
            let bytes = std::fs::read(dir.join("manifest.json")).unwrap();
            let manifest: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            manifest["elapsed"]["SoundCloud"]["progress"]
                .as_u64()
                .unwrap_or(0)
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        while kept_progress() < crate::store::PROTECTED_FOR && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let progress = kept_progress();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            progress >= crate::store::PROTECTED_FOR,
            "the manifest counts {progress} s"
        );
    }
}
