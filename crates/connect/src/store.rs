//! What a device keeps: its own id, its key away from home, and the
//! devices paired with it, each with the key they share and where to reach
//! it away from home.

use std::path::{Path, PathBuf};

use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::Device;

#[derive(Serialize, Deserialize, Default)]
struct Saved {
    id: String,
    /// This device's secret key on iroh, in hex: see `away`.
    #[serde(default)]
    away: String,
    paired: Vec<Paired>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Paired {
    device: Device,
    /// The key shared at pairing, in hex.
    key: String,
    /// Its id on iroh, as it told once connected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    away: Option<String>,
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
        let mut store = Self {
            saved: saved.unwrap_or_else(|| Saved {
                id: hex::encode(random::<8>()),
                away: String::new(),
                paired: Vec::new(),
            }),
            path,
        };
        if store.secret().is_none() {
            store.saved.away = hex::encode(random::<32>());
        }
        store.save();
        store
    }

    pub fn id(&self) -> &str {
        &self.saved.id
    }

    /// This device's secret key on iroh.
    pub fn secret(&self) -> Option<[u8; 32]> {
        hex::decode(&self.saved.away).ok()?.try_into().ok()
    }

    /// Where `id` is reached away from home, once it said.
    pub fn away(&self, id: &str) -> Option<&str> {
        self.saved
            .paired
            .iter()
            .find(|p| p.device.id == id)?
            .away
            .as_deref()
    }

    /// `id` said where it is reached away from home.
    pub fn set_away(&mut self, id: &str, away: String) {
        if let Some(p) = self.saved.paired.iter_mut().find(|p| p.device.id == id)
            && p.away.as_ref() != Some(&away)
        {
            p.away = Some(away);
            self.save();
        }
    }

    /// The paired device reached away from home as `away`.
    pub fn away_of(&self, away: &str) -> Option<&Device> {
        self.saved
            .paired
            .iter()
            .find(|p| p.away.as_deref() == Some(away))
            .map(|p| &p.device)
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
            away: None,
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
