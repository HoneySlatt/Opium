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

#[derive(Clone)]
pub struct Downloads {
    /// Until it is opened: `None`; why it couldn't be, after.
    store: Arc<Mutex<Option<Result<Store, String>>>>,
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
            store: Arc::default(),
            queue: Arc::default(),
            fetcher,
            listener,
            runtime: Handle::current(),
        }
    }

    /// The store opened, or why it couldn't be: what it keeps is told.
    pub fn opened(&self, store: Result<Store, String>) {
        *lock(&self.store) = Some(store);
        self.tell();
    }

    /// What is kept, to the listener.
    pub fn tell(&self) {
        let now = now();
        let kept = match &*lock(&self.store) {
            Some(Ok(store)) => store
                .items()
                .map(|item| KeptSong {
                    track: item.track.clone(),
                    standing: store.standing(item, now),
                    added: item.added,
                })
                .collect(),
            _ => Vec::new(),
        };
        self.listener.kept(kept);
    }

    /// The song kept as `id`, opened to play, if it is kept and plays.
    pub fn open(&self, id: &ItemId) -> Option<Open> {
        let store = lock(&self.store);
        let Some(Ok(store)) = &*store else {
            return None;
        };
        match store.open_song(id, now())? {
            Ok(open) => Some(open),
            // Damaged: it plays from the service, as if not kept.
            Err(e) => {
                eprintln!("downloads: {id}: {e}");
                None
            }
        }
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
            if let Some(Ok(store)) = &mut *lock(&store) {
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
            Some(Ok(store)) => store.items().map(|i| i.track.id.clone()).collect(),
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

    /// Signed out of `provider`: its songs go.
    pub fn forget(&self, provider: ProviderKind) {
        lock(&self.queue)
            .tracks
            .retain(|(t, _)| t.id.provider != provider);
        if let Some(Ok(store)) = &mut *lock(&self.store) {
            let _ = store.remove_service(provider);
        }
        self.tell();
    }

    /// `provider` said whether its subscription is active: its protected
    /// songs play for 30 days more, or no longer.
    pub fn subscription(&self, provider: ProviderKind, active: bool) {
        if let Some(Ok(store)) = &mut *lock(&self.store) {
            let _ = store.subscription(provider, active, now());
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
                    Some(Ok(store)) => store.begin().map_err(|e| e.to_string())?,
                    Some(Err(e)) => return Err(e.clone()),
                    None => return Err("the downloads aren't open yet".into()),
                };
                let id = song.track.id.clone();
                let called_off = || lock(&queue).called_off(&id);
                match copy(reader, pending, len, &called_off, |coming| {
                    listener.state(id.clone(), Downloading::Coming(coming))
                })? {
                    Some(pending) => {
                        let mut store = lock(&store);
                        let Some(Ok(store)) = &mut *store else {
                            pending.abandon();
                            return Err("the downloads closed".into());
                        };
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
}
