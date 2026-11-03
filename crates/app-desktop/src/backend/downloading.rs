//! Songs downloaded to play offline: one at a time, through the same
//! fetch as playing them, read to the end into the store sealed. A song
//! kept plays from there before anything is asked of its service.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use apple_music::AppleMusic;
use downloads::store::{Song, Standing, Store};
use music_core::model::{ItemId, ProviderKind, Track};
use navidrome::Navidrome;
use soundcloud::SoundCloud;

use super::{Playable, Replies, Reply};

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

/// Where the downloads are kept: the user's data, not a cache that may be
/// cleared.
fn dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_default()
        .join("music/downloads")
}

pub(super) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The services as they are now, for the download that comes next.
#[derive(Clone, Default)]
pub(super) struct Services {
    pub apple: Option<Arc<AppleMusic>>,
    pub soundcloud: Option<Arc<SoundCloud>>,
    pub navidrome: Option<Arc<Navidrome>>,
    pub cdm: Option<widevine::Cdm>,
}

#[derive(Default)]
struct Queue {
    tracks: VecDeque<(Track, bool)>,
    running: bool,
    /// The one coming, and whether it was called off.
    current: Option<(ItemId, bool)>,
}

#[derive(Clone)]
pub(super) struct Downloads {
    /// Until the keyring gave the key: `None`; why it didn't, after.
    store: Arc<Mutex<Option<Result<Store, String>>>>,
    queue: Arc<Mutex<Queue>>,
    services: Arc<Mutex<Services>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Downloads {
    /// Open the store, its key from the keyring, and tell what it keeps.
    pub fn open(replies: Replies) -> Self {
        let downloads = Self {
            store: Arc::default(),
            queue: Arc::default(),
            services: Arc::default(),
        };
        let store = downloads.store.clone();
        let told = downloads.clone();
        tokio::spawn(async move {
            let opened = match downloads::keyring::key().await {
                Ok(key) => tokio::task::spawn_blocking(move || {
                    Store::open(&dir(), key)
                        .map_err(|e| format!("Couldn't open the downloads: {e}"))
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string())),
                Err(e) => Err(e),
            };
            if let Err(e) = &opened {
                eprintln!("downloads: {e}");
            }
            *lock(&store) = Some(opened);
            told.tell(&replies);
        });
        downloads
    }

    /// What is kept, to the UI.
    pub fn tell(&self, replies: &Replies) {
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
        replies.send(Reply::Downloads(kept));
    }

    pub fn services(&self, services: Services) {
        *lock(&self.services) = services;
    }

    /// The song kept as `id`, to play, if it is kept and plays.
    pub fn playable(&self, id: &ItemId) -> Option<Playable> {
        let store = lock(&self.store);
        let Some(Ok(store)) = &*store else {
            return None;
        };
        match store.open_song(id, now())? {
            Ok(open) => Some(Playable::Stored(Box::new(open))),
            // Damaged: it plays from the service, as if not kept.
            Err(e) => {
                eprintln!("downloads: {id}: {e}");
                None
            }
        }
    }

    /// Download `tracks`, after those waiting; `original`: a SoundCloud
    /// uploader's own file, where they allow it.
    pub fn add(&self, tracks: Vec<Track>, original: bool, replies: &Replies) {
        let start = {
            let mut queue = lock(&self.queue);
            for track in tracks {
                let id = &track.id;
                let waiting = queue.tracks.iter().any(|(t, _)| &t.id == id)
                    || queue
                        .current
                        .as_ref()
                        .is_some_and(|(c, off)| c == id && !off);
                if waiting {
                    continue;
                }
                replies.send(Reply::Download {
                    id: id.clone(),
                    state: Downloading::Queued,
                });
                queue.tracks.push_back((track, original));
            }
            !std::mem::replace(&mut queue.running, true)
        };
        if start {
            tokio::spawn(self.clone().work(replies.clone()));
        }
    }

    /// No longer kept, nor to be downloaded.
    pub fn remove(&self, ids: Vec<ItemId>, replies: &Replies) {
        {
            let mut queue = lock(&self.queue);
            queue.tracks.retain(|(t, _)| !ids.contains(&t.id));
            if let Some((current, off)) = &mut queue.current
                && ids.contains(current)
            {
                *off = true;
            }
        }
        let store = self.store.clone();
        let replies = replies.clone();
        tokio::task::spawn_blocking(move || {
            if let Some(Ok(store)) = &mut *lock(&store) {
                for id in &ids {
                    if let Err(e) = store.remove(id) {
                        replies.send(Reply::Failed(format!("Couldn't remove the download: {e}")));
                    }
                }
            }
            for id in ids {
                replies.send(Reply::Download {
                    id,
                    state: Downloading::Gone,
                });
            }
        });
    }

    /// Signed out of `provider`: its songs go.
    pub fn forget(&self, provider: ProviderKind, replies: &Replies) {
        lock(&self.queue)
            .tracks
            .retain(|(t, _)| t.id.provider != provider);
        if let Some(Ok(store)) = &mut *lock(&self.store) {
            let _ = store.remove_service(provider);
        }
        self.tell(replies);
    }

    /// `provider` said whether its subscription is active: its protected
    /// songs play for 30 days more, or no longer.
    pub fn subscription(&self, provider: ProviderKind, active: bool, replies: &Replies) {
        if let Some(Ok(store)) = &mut *lock(&self.store) {
            let _ = store.subscription(provider, active, now());
        }
        self.tell(replies);
    }

    async fn work(self, replies: Replies) {
        loop {
            let next = {
                let mut queue = lock(&self.queue);
                let next = queue.tracks.pop_front();
                queue.current = next.as_ref().map(|(t, _)| (t.id.clone(), false));
                if next.is_none() {
                    queue.running = false;
                }
                next
            };
            let Some((track, original)) = next else {
                return;
            };
            let id = track.id.clone();
            let apple = id.provider == ProviderKind::AppleMusic;
            replies.send(Reply::Download {
                id: id.clone(),
                state: Downloading::Coming(0.0),
            });
            let state = match self.download(track, original, &replies).await {
                Ok(Some(standing)) => Downloading::Kept(standing),
                Ok(None) => Downloading::Gone,
                Err(e) => Downloading::Failed(e),
            };
            let kept = matches!(state, Downloading::Kept(_));
            replies.send(Reply::Download { id, state });
            // The library's Downloaded has it.
            if kept {
                self.tell(&replies);
            }
            // Apple's licences one after the other, not in a burst.
            if apple {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    /// Download `track` into the store: how it then stands, or `None` if it
    /// was called off.
    async fn download(
        &self,
        track: Track,
        original: bool,
        replies: &Replies,
    ) -> Result<Option<Standing>, String> {
        let services = lock(&self.services).clone();
        let id = &track.id;
        let playable = match id.provider {
            ProviderKind::AppleMusic => {
                let am = services.apple.ok_or("Apple Music isn't connected")?;
                let cdm = services.cdm.ok_or("no Widevine module found");
                // In stereo: Spatial Audio isn't downloaded.
                super::playable_as(&am, cdm.map_err(str::to_owned), id, false).await?
            }
            ProviderKind::SoundCloud => {
                let sc = services.soundcloud.ok_or("SoundCloud isn't connected")?;
                super::soundcloud_playable(&sc, id, original, services.cdm.as_ref()).await?
            }
            ProviderKind::Navidrome => {
                let nd = services.navidrome.ok_or("Navidrome isn't connected")?;
                Playable::Navidrome(nd.audio(id).await.map_err(|e| e.to_string())?)
            }
            ProviderKind::Music => return Err("Music has no songs of its own".into()),
        };
        let (reader, len, song): (Box<dyn Read + Send>, u64, Song) = match playable {
            Playable::Track {
                stream,
                kbps,
                spatial,
            } => {
                let len = stream.len();
                let song = Song {
                    track,
                    extension: stream.extension().to_owned(),
                    priming: apple_music::PRIMING,
                    // Apple's songs, uploads too, are the subscription's.
                    protected: true,
                    kbps,
                    spatial,
                };
                (Box::new(stream), len, song)
            }
            Playable::SoundCloud(audio) => {
                let song = Song {
                    track,
                    extension: audio.extension.to_owned(),
                    priming: 0,
                    protected: matches!(audio.reader, soundcloud::Reader::Decrypting(_)),
                    kbps: audio.quality.bitrate_kbps,
                    spatial: false,
                };
                (Box::new(audio.reader), audio.len, song)
            }
            Playable::Navidrome(audio) => {
                let song = Song {
                    track,
                    extension: audio.extension.to_owned(),
                    priming: 0,
                    protected: false,
                    kbps: None,
                    spatial: false,
                };
                (Box::new(audio.reader), audio.len, song)
            }
            Playable::Live(_) => return Err("a station can't be downloaded".into()),
            Playable::Stored(_) => return Err("already downloaded".into()),
        };
        let (store, queue, replies) = (self.store.clone(), self.queue.clone(), replies.clone());
        tokio::task::spawn_blocking(move || {
            let pending = match &*lock(&store) {
                Some(Ok(store)) => store.begin().map_err(|e| e.to_string())?,
                Some(Err(e)) => return Err(e.clone()),
                None => return Err("the downloads aren't open yet".into()),
            };
            let id = song.track.id.clone();
            let called_off = || {
                lock(&queue)
                    .current
                    .as_ref()
                    .is_none_or(|(c, off)| c != &id || *off)
            };
            match copy(reader, pending, len, &called_off, |coming| {
                replies.send(Reply::Download {
                    id: id.clone(),
                    state: Downloading::Coming(coming),
                })
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
    mut pending: downloads::store::Pending,
    len: u64,
    called_off: &dyn Fn() -> bool,
    mut coming: impl FnMut(f32),
) -> Result<Option<downloads::store::Pending>, String> {
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
