//! Songs downloaded to play offline on the phone, as on the desktop (the
//! `downloads` crate's queue and store): kept sealed in the app's files with
//! a key the Keystore guards, played from there before any service is asked.
//! The phone's Widevine is given no licence it may keep, so an Apple Music
//! or SoundCloud song that is encrypted is borrowed from the paired
//! computer (see `connect::lend`), which downloads it as it would its own.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};

use downloads::queue::{self, Fetched, Fetcher, Listener};
use downloads::sealed;
use downloads::store::{Song, Standing, Store};
use opium_core::model::{ItemId, ProviderKind, Track};

use crate::{Connect, OpiumError, Session};

/// How a song stands as a download, for its badge.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum DownloadState {
    Queued,
    /// How much has come, from 0 to 1.
    Coming {
        part: f32,
    },
    /// Kept; until when it plays (seconds since the epoch) if it came through
    /// a service's DRM.
    Kept {
        until: Option<u64>,
    },
    /// Kept, but its subscription wasn't confirmed in the last 30 days.
    Expired,
    Failed {
        reason: String,
    },
    /// Not kept, or no longer.
    Gone,
}

impl From<queue::Downloading> for DownloadState {
    fn from(d: queue::Downloading) -> Self {
        match d {
            queue::Downloading::Queued => Self::Queued,
            queue::Downloading::Coming(part) => Self::Coming { part },
            queue::Downloading::Kept(standing) => standing.into(),
            queue::Downloading::Failed(reason) => Self::Failed { reason },
            queue::Downloading::Gone => Self::Gone,
        }
    }
}

impl From<Standing> for DownloadState {
    fn from(s: Standing) -> Self {
        match s {
            Standing::Plays => Self::Kept { until: None },
            Standing::PlaysUntil(at) => Self::Kept { until: Some(at) },
            Standing::Expired => Self::Expired,
        }
    }
}

/// A song kept, for the library's Downloaded.
#[derive(Debug, Clone, uniffi::Record)]
pub struct KeptDownload {
    pub track: Track,
    pub state: DownloadState,
    /// When it was downloaded, in seconds since the epoch.
    pub added: u64,
}

/// What the app is told of the downloads.
#[uniffi::export(with_foreign)]
pub trait DownloadListener: Send + Sync {
    fn state(&self, id: ItemId, state: DownloadState);
    /// Every song kept, after one came or went.
    fn kept(&self, kept: Vec<KeptDownload>);
    fn failed(&self, message: String);
}

struct Told(Arc<dyn DownloadListener>);

impl Listener for Told {
    fn state(&self, id: ItemId, state: queue::Downloading) {
        self.0.state(id, state.into());
    }

    fn kept(&self, kept: Vec<queue::KeptSong>) {
        self.0.kept(
            kept.into_iter()
                .map(|k| KeptDownload {
                    track: k.track,
                    state: k.standing.into(),
                    added: k.added,
                })
                .collect(),
        );
    }

    fn failed(&self, message: String) {
        self.0.failed(message);
    }
}

/// Fetches through the session's services, as they are when a song's turn
/// comes; what they can't give the phone, through the computer.
struct Phone {
    session: Arc<Session>,
    connect: Mutex<Option<Arc<Connect>>>,
}

impl Phone {
    /// `track` from the paired computer connected now.
    async fn borrow(&self, track: Track, original: bool) -> Result<Fetched, String> {
        let connect = self
            .connect
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let node = connect.as_ref().map(|c| &c.0);
        let Some(from) = node.and_then(|n| n.lenders().into_iter().next()) else {
            return Err("Comes through your computer: open Opium on it".into());
        };
        let (lent, borrowed) = node
            .expect("a lender is the node's")
            .borrow(&from, track.clone(), original)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Fetched {
            len: lent.len,
            song: Song {
                track,
                extension: lent.extension,
                priming: lent.priming,
                protected: lent.protected,
                kbps: lent.kbps,
                spatial: lent.spatial,
            },
            reader: Box::new(borrowed),
        })
    }
}

#[async_trait::async_trait]
impl Fetcher for Phone {
    async fn fetch(&self, track: Track, original: bool) -> Result<Fetched, String> {
        let services = self.session.services();
        let id = &track.id;
        match id.provider {
            ProviderKind::Navidrome => {
                let nd = services.navidrome.ok_or("Navidrome isn't connected")?;
                let audio = nd.audio(id).await.map_err(|e| e.to_string())?;
                Ok(Fetched {
                    len: audio.len,
                    song: Song {
                        track,
                        extension: audio.extension.to_owned(),
                        priming: 0,
                        protected: false,
                        kbps: None,
                        spatial: false,
                    },
                    reader: Box::new(audio.reader),
                })
            }
            ProviderKind::SoundCloud => {
                let sc = services.soundcloud.ok_or("SoundCloud isn't connected")?;
                // Without a CDM, an encrypted track can't come from here:
                // the computer brings it.
                let audio = match sc.audio(id, original, None).await {
                    Ok(audio) => audio,
                    Err(opium_core::Error::Playback(m)) if m.contains("encrypted") => {
                        return self.borrow(track, original).await;
                    }
                    Err(e) => return Err(e.to_string()),
                };
                Ok(Fetched {
                    len: audio.len,
                    song: Song {
                        track,
                        extension: audio.extension.to_owned(),
                        priming: 0,
                        protected: false,
                        kbps: audio.quality.bitrate_kbps,
                        spatial: false,
                    },
                    reader: Box::new(audio.reader),
                })
            }
            ProviderKind::AppleMusic => self.borrow(track, original).await,
            ProviderKind::Opium => Err("Opium has no songs of its own".into()),
            ProviderKind::Local => Err("this song's file is on this phone already".into()),
        }
    }
}

/// The downloads on this phone.
#[derive(uniffi::Object)]
pub struct Downloads(queue::Downloads, Arc<Phone>);

#[uniffi::export(async_runtime = "tokio")]
impl Downloads {
    /// The store in `dir`, sealed with `key` (32 bytes), downloading through
    /// `session`'s services; what it keeps is told to `listener` at once.
    #[uniffi::constructor]
    pub async fn open(
        session: Arc<Session>,
        dir: String,
        key: Vec<u8>,
        listener: Arc<dyn DownloadListener>,
    ) -> Result<Arc<Self>, OpiumError> {
        let key: sealed::Key = key.try_into().map_err(|_| OpiumError::Failed {
            reason: "the downloads' key isn't 32 bytes".into(),
        })?;
        let store =
            tokio::task::spawn_blocking(move || Store::open(std::path::Path::new(&dir), key))
                .await
                .map_err(|e| OpiumError::Failed {
                    reason: e.to_string(),
                })?
                .map_err(|e| OpiumError::Failed {
                    reason: format!("Couldn't open the downloads: {e}"),
                })?;
        let phone = Arc::new(Phone {
            session,
            connect: Mutex::default(),
        });
        let queue = queue::Downloads::new(phone.clone(), Arc::new(Told(listener)));
        queue.opened(Ok(store));
        Ok(Arc::new(Self(queue, phone)))
    }

    /// Borrow from the paired computer, through `connect`, what can't be
    /// downloaded here.
    pub fn borrow_through(&self, connect: Arc<Connect>) {
        *self.1.connect.lock().unwrap_or_else(|e| e.into_inner()) = Some(connect);
    }

    /// Download `tracks`, after those waiting; `original`: a SoundCloud
    /// uploader's own file, where they allow it.
    pub fn add(&self, tracks: Vec<Track>, original: bool) {
        self.0.add(tracks, original);
    }

    /// No longer kept, nor to be downloaded.
    pub fn remove(&self, ids: Vec<ItemId>) {
        self.0.remove(ids);
    }

    /// Every download gone, and none to come.
    pub fn remove_all(&self) {
        self.0.remove_all();
    }

    /// Signed out of `provider`: its songs go.
    pub fn forget(&self, provider: ProviderKind) {
        self.0.forget(provider);
    }

    /// `provider` said whether its subscription is active.
    pub fn subscription(&self, provider: ProviderKind, active: bool) {
        self.0.subscription(provider, active);
    }

    /// Whether songs are waiting or coming: the phone keeps the app up
    /// meanwhile.
    pub fn busy(&self) -> bool {
        self.0.busy()
    }

    /// The song kept as `id`, to play, if it is kept and plays.
    pub fn song(&self, id: ItemId) -> Option<Arc<KeptAudio>> {
        let open = self.0.open(&id)?;
        Some(Arc::new(KeptAudio {
            len: open.len,
            extension: open.extension,
            priming: open.priming,
            reader: Mutex::new(open.reader),
        }))
    }
}

/// A kept song, opened: read where the player asks, unsealed as it goes.
#[derive(uniffi::Object)]
pub struct KeptAudio {
    len: u64,
    extension: String,
    priming: u32,
    reader: Mutex<sealed::Reader<File>>,
}

#[uniffi::export]
impl KeptAudio {
    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// `m4a`, `flac`…: what the file is.
    pub fn extension(&self) -> String {
        self.extension.clone()
    }

    /// Frames the encoder put before the music, which the file doesn't say.
    pub fn priming(&self) -> u32 {
        self.priming
    }

    /// Up to `length` bytes from `offset`; none at the end.
    pub fn read(&self, offset: u64, length: u32) -> Result<Vec<u8>, OpiumError> {
        let failed = |e: std::io::Error| OpiumError::Failed {
            reason: format!("the kept song: {e}"),
        };
        let mut reader = self.reader.lock().unwrap_or_else(|e| e.into_inner());
        reader.seek(SeekFrom::Start(offset)).map_err(failed)?;
        let want = (length as u64).min(self.len.saturating_sub(offset)) as usize;
        let mut buf = vec![0; want];
        let mut got = 0;
        while got < want {
            match reader.read(&mut buf[got..]).map_err(failed)? {
                0 => break,
                n => got += n,
            }
        }
        buf.truncate(got);
        Ok(buf)
    }
}
