//! Songs downloaded to play offline, as the phone downloads them (the
//! `downloads` crate's queue): fetched as playing them would, with the
//! desktop's CDM. A song kept plays from the store before anything is asked
//! of its service.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use apple_music::AppleMusic;
pub use downloads::queue::{Downloading, KeptSong};
use downloads::queue::{Fetched, Fetcher, Listener};
use downloads::store::{Song, Store};
use navidrome::Navidrome;
use opium_core::model::{ItemId, ProviderKind, Track};
use soundcloud::SoundCloud;

use super::{Playable, Replies, Reply};

/// Where the downloads are kept: the user's data, not a cache that may be
/// cleared. `None` without a data folder.
fn dir() -> Option<PathBuf> {
    Some(crate::folders::data()?.join("downloads"))
}

/// The services as they are now, for the download that comes next.
#[derive(Clone, Default)]
pub(super) struct Services {
    pub apple: Option<Arc<AppleMusic>>,
    pub soundcloud: Option<Arc<SoundCloud>>,
    pub navidrome: Option<Arc<Navidrome>>,
    pub cdm: Option<widevine::Cdm>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Fetches with the services as they are when a song's turn comes.
struct Desktop(Mutex<Services>);

#[async_trait::async_trait]
impl Fetcher for Desktop {
    async fn fetch(&self, track: Track, original: bool) -> Result<Fetched, String> {
        let services = lock(&self.0).clone();
        fetch(services, track, original).await
    }
}

/// Tells the UI.
struct Told(Replies);

impl Listener for Told {
    fn state(&self, id: ItemId, state: Downloading) {
        self.0.send(Reply::Download { id, state });
    }

    fn kept(&self, kept: Vec<KeptSong>) {
        self.0.send(Reply::Downloads(kept));
    }

    fn failed(&self, message: String) {
        self.0.send(Reply::Failed(message));
    }
}

/// Lends the paired phone what it can't download itself (Apple Music's and
/// SoundCloud's encrypted songs), fetched as this computer downloads them,
/// in the clear only on the paired connection to the phone, which keeps
/// it sealed.
struct Lends {
    fetcher: Arc<Desktop>,
    runtime: tokio::runtime::Handle,
}

#[async_trait::async_trait]
impl connect::Lender for Lends {
    async fn lend(
        &self,
        track: Track,
        original: bool,
    ) -> Result<(connect::Lent, Box<dyn std::io::Read + Send>), String> {
        let services = lock(&self.fetcher.0).clone();
        let fetched = self
            .runtime
            .spawn(fetch(services, track, original))
            .await
            .map_err(|e| e.to_string())??;
        let song = fetched.song;
        let lent = connect::Lent {
            len: fetched.len,
            extension: song.extension,
            priming: song.priming,
            protected: song.protected,
            kbps: song.kbps,
            spatial: song.spatial,
        };
        Ok((lent, fetched.reader))
    }
}

#[derive(Clone)]
pub(super) struct Downloads {
    queue: downloads::queue::Downloads,
    fetcher: Arc<Desktop>,
}

impl Downloads {
    /// Open the store, its key from the keyring, and tell what it keeps.
    pub fn open(replies: Replies) -> Self {
        let fetcher = Arc::new(Desktop(Mutex::default()));
        let queue = downloads::queue::Downloads::new(fetcher.clone(), Arc::new(Told(replies)));
        let opening = queue.clone();
        tokio::spawn(async move {
            // The folder first: without one, the keyring isn't asked for a key.
            let opened = match dir() {
                None => Err(
                    "Couldn't open the downloads: there is no folder to keep them in, \
                             as neither XDG_DATA_HOME nor HOME names an absolute folder"
                        .to_owned(),
                ),
                Some(dir) => match downloads::keyring::key().await {
                    Ok(key) => tokio::task::spawn_blocking(move || {
                        Store::open(&dir, key)
                            .map_err(|e| format!("Couldn't open the downloads: {e}"))
                    })
                    .await
                    .unwrap_or_else(|e| Err(e.to_string())),
                    Err(e) => Err(e),
                },
            };
            if let Err(e) = &opened {
                eprintln!("downloads: {e}");
            }
            opening.opened(opened);
        });
        Self { queue, fetcher }
    }

    /// What lends the phone songs, with the services as they are.
    pub fn lender(&self) -> Arc<dyn connect::Lender> {
        Arc::new(Lends {
            fetcher: self.fetcher.clone(),
            runtime: tokio::runtime::Handle::current(),
        })
    }

    pub fn services(&self, services: Services) {
        *lock(&self.fetcher.0) = services;
    }

    /// The song kept as `id`, to play, if it is kept and plays.
    pub fn playable(&self, id: &ItemId) -> Option<Playable> {
        self.queue
            .open(id)
            .map(|open| Playable::Stored(Box::new(open)))
    }

    /// Download `tracks`, after those waiting; `original`: a SoundCloud
    /// uploader's own file, where they allow it.
    pub fn add(&self, tracks: Vec<Track>, original: bool) {
        self.queue.add(tracks, original);
    }

    /// No longer kept, nor to be downloaded.
    pub fn remove(&self, ids: Vec<ItemId>) {
        self.queue.remove(ids);
    }

    /// Signed out of `provider`: its downloads waiting or under way are
    /// called off, and its protected songs no longer play until its
    /// subscription is confirmed again; every song kept stays.
    pub fn forget(&self, provider: ProviderKind) {
        self.queue.forget(provider);
    }

    /// `ids`, asked for but never queued, will not come.
    pub fn called_off(&self, ids: Vec<ItemId>) {
        self.queue.called_off(ids);
    }

    /// `provider` said whether its subscription is active.
    pub fn subscription(&self, provider: ProviderKind, active: bool) {
        self.queue.subscription(provider, active);
    }
}

/// Fetch `track` as playing it would, in stereo, to read it into the store.
async fn fetch(services: Services, track: Track, original: bool) -> Result<Fetched, String> {
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
        ProviderKind::Opium => return Err("Opium has no songs of its own".into()),
        ProviderKind::Local => return Err("this song's file is on this computer already".into()),
    };
    Ok(match playable {
        Playable::Track {
            stream,
            kbps,
            spatial,
        } => Fetched {
            len: stream.len(),
            song: Song {
                extension: stream.extension().to_owned(),
                track,
                priming: apple_music::PRIMING,
                // Apple's songs, uploads too, are the subscription's.
                protected: true,
                kbps,
                spatial,
            },
            reader: Box::new(stream),
        },
        Playable::SoundCloud(audio) => Fetched {
            song: Song {
                track,
                extension: audio.extension.to_owned(),
                priming: 0,
                protected: matches!(audio.reader, soundcloud::Reader::Decrypting(_)),
                kbps: audio.quality.bitrate_kbps,
                spatial: false,
            },
            len: audio.len,
            reader: Box::new(audio.reader),
        },
        Playable::Navidrome(audio) => Fetched {
            song: Song {
                track,
                extension: audio.extension.to_owned(),
                priming: 0,
                protected: false,
                kbps: None,
                spatial: false,
            },
            len: audio.len,
            reader: Box::new(audio.reader),
        },
        Playable::Live(_) => return Err("a station can't be downloaded".into()),
        Playable::Stored(_) => return Err("already downloaded".into()),
        Playable::Local(_) => return Err("this song's file is on this computer already".into()),
    })
}
