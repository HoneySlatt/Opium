//! What a device keeps: its own id, and the devices paired with it, each
//! with the key they share.

use std::path::{Path, PathBuf};

use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::Device;

#[derive(Serialize, Deserialize, Default)]
struct Saved {
    id: String,
    paired: Vec<Paired>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Paired {
    device: Device,
    /// The key shared at pairing, in hex.
    key: String,
}

pub(crate) struct Store {
    path: PathBuf,
    saved: Saved,
}

impl Store {
    /// The store in `dir`, made with a new id the first time.
    pub fn open(dir: &Path) -> Self {
        let path = dir.join("connect.json");
        let saved = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Saved>(&b).ok())
            .filter(|s| !s.id.is_empty());
        // There but unreadable: put aside rather than written over, its
        // pairings being worth more than a fresh start.
        if saved.is_none() && path.exists() {
            let _ = std::fs::rename(&path, path.with_extension("json.bad"));
        }
        let store = Self {
            saved: saved.unwrap_or_else(|| Saved {
                id: hex::encode(random::<8>()),
                paired: Vec::new(),
            }),
            path,
        };
        store.save();
        store
    }

    pub fn id(&self) -> &str {
        &self.saved.id
    }

    pub fn key(&self, id: &str) -> Option<[u8; 32]> {
        let paired = self.saved.paired.iter().find(|p| p.device.id == id)?;
        hex::decode(&paired.key).ok()?.try_into().ok()
    }

    pub fn paired(&self) -> impl Iterator<Item = &Device> {
        self.saved.paired.iter().map(|p| &p.device)
    }

    pub fn pair(&mut self, device: Device, key: [u8; 32]) {
        self.saved.paired.retain(|p| p.device.id != device.id);
        self.saved.paired.push(Paired {
            device,
            key: hex::encode(key),
        });
        self.save();
    }

    /// A paired device came back under another name, or as another kind.
    pub fn update(&mut self, device: &Device) {
        if let Some(p) = self
            .saved
            .paired
            .iter_mut()
            .find(|p| p.device.id == device.id)
            && p.device != *device
        {
            p.device = device.clone();
            self.save();
        }
    }

    pub fn forget(&mut self, id: &str) {
        self.saved.paired.retain(|p| p.device.id != id);
        self.save();
    }

    /// Written aside, readable by the user alone, then moved over.
    fn save(&self) {
        let Ok(bytes) = serde_json::to_vec_pretty(&self.saved) else {
            return;
        };
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let partial = self.path.with_extension("json.part");
        let written = {
            use std::io::Write;
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create(true).truncate(true);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
            // On disk before it replaces the last: a power cut midway leaves
            // one or the other whole.
            options
                .open(&partial)
                .and_then(|mut f| f.write_all(&bytes).and_then(|()| f.sync_all()))
        };
        if written.is_ok() {
            let _ = std::fs::rename(&partial, &self.path);
        }
    }
}

pub(crate) fn random<const N: usize>() -> [u8; N] {
    let mut bytes = [0; N];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes
}
