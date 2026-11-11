//! What a device keeps: its own id, its key away from home, and the
//! devices paired with it, each with the key they share and where to reach
//! it away from home; the keys it gave two of them to know each other by,
//! and the devices the user forgot here.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use opium_core::files::{Mode, put_aside, write_whole};
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
    /// The keys made for two paired devices to know each other by, by
    /// their ids, the smaller first: given again alike each time.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    made: BTreeMap<String, String>,
    /// The devices unpaired here: not paired again but by hand.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    forgotten: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Paired {
    device: Device,
    /// The key shared at pairing, in hex.
    key: String,
    /// Its id on iroh, as it told once connected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    away: Option<String>,
    /// The device that introduced it, paired with both; none if paired
    /// here by hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    by: Option<String>,
}

pub(crate) struct Store {
    path: PathBuf,
    saved: Saved,
    /// Whether `path` may be written: not when a file there didn't read
    /// and couldn't be put aside.
    writable: bool,
}

impl Store {
    /// The store in `dir`, made with a new id the first time.
    ///
    /// A `connect.json` that doesn't read is put aside, as
    /// `connect.json.bad` or, when taken, `connect.json.bad.1`, `.2`…, and
    /// the store starts anew; one that can't be put aside is left as it is
    /// and never written over, the store then keeping its changes in memory
    /// only. Each is said on the standard error.
    pub fn open(dir: &Path) -> Self {
        let path = dir.join("connect.json");
        let read = std::fs::read(&path);
        let missing = read
            .as_ref()
            .is_err_and(|e| e.kind() == io::ErrorKind::NotFound);
        let saved = read
            .ok()
            .and_then(|b| serde_json::from_slice::<Saved>(&b).ok())
            .filter(|s| !s.id.is_empty());
        // There but unreadable: put aside rather than written over, its
        // pairings being worth more than a fresh start.
        let writable = saved.is_some() || missing || set_aside(&path);
        let mut store = Self {
            saved: saved.unwrap_or_else(|| Saved {
                id: hex::encode(random::<8>()),
                away: String::new(),
                paired: Vec::new(),
                made: BTreeMap::new(),
                forgotten: Vec::new(),
            }),
            path,
            writable,
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
        self.saved.forgotten.retain(|id| *id != device.id);
        self.saved.paired.push(Paired {
            device,
            key: hex::encode(key),
            away: None,
            by: None,
        });
        self.save();
    }

    /// The key for paired devices `a` and `b` to know each other by, made
    /// the first time.
    pub fn introduction(&mut self, a: &str, b: &str) -> String {
        let pair = if a < b {
            format!("{a} {b}")
        } else {
            format!("{b} {a}")
        };
        if let Some(key) = self.saved.made.get(&pair) {
            return key.clone();
        }
        let key = hex::encode(random::<32>());
        self.saved.made.insert(pair, key.clone());
        self.save();
        key
    }

    /// `by`, paired with this device and with `device`, introduced them
    /// with `key`: whether they are paired now as it says. Not a device
    /// forgotten here, nor one paired by hand; of two devices introducing
    /// the same one, the smaller id wins, on both sides alike.
    pub fn introduced(
        &mut self,
        device: Device,
        key: String,
        away: Option<String>,
        by: &str,
    ) -> bool {
        if self.saved.forgotten.contains(&device.id)
            || hex::decode(&key).map_or(true, |k| k.len() != 32)
        {
            return false;
        }
        match self
            .saved
            .paired
            .iter_mut()
            .find(|p| p.device.id == device.id)
        {
            Some(p) => {
                let Some(before) = &p.by else {
                    return false;
                };
                if p.key == key || by > before.as_str() {
                    return false;
                }
                p.key = key;
                p.by = Some(by.to_owned());
                if away.is_some() {
                    p.away = away;
                }
            }
            None => self.saved.paired.push(Paired {
                device,
                key,
                away,
                by: Some(by.to_owned()),
            }),
        }
        self.save();
        true
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
        if !self.saved.forgotten.iter().any(|f| f == id) {
            self.saved.forgotten.push(id.to_owned());
        }
        self.save();
    }

    /// Written whole or not at all, readable by the user alone
    /// ([`write_whole`]); never over a file that may not be written.
    fn save(&self) {
        if !self.writable {
            return;
        }
        let Ok(bytes) = serde_json::to_vec_pretty(&self.saved) else {
            return;
        };
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = write_whole(&self.path, &bytes, Mode::Private);
    }
}

/// `path`, which doesn't read, put aside: whether it was, and so whether
/// it may be written anew.
fn set_aside(path: &Path) -> bool {
    match put_aside(path, &path.with_extension("json.bad")) {
        Ok(aside) => {
            eprintln!(
                "opium connect: {} doesn't read: put aside as {}",
                path.display(),
                aside.display()
            );
            true
        }
        Err(e) => {
            eprintln!(
                "opium connect: {} doesn't read, and can't be put aside ({e}): left as it is, \
                 never written over; the pairings made are kept in memory only until Opium closes",
                path.display()
            );
            false
        }
    }
}

pub(crate) fn random<const N: usize>() -> [u8; N] {
    let mut bytes = [0; N];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DeviceKind;

    fn dir() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("connect-store-{}", hex::encode(random::<8>())));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn phone() -> Device {
        Device {
            id: "0123456789abcdef".into(),
            name: "Phone".into(),
            kind: DeviceKind::Phone,
        }
    }

    /// L5: each `connect.json` put aside keeps a name of its own, and the
    /// store starts anew beside them.
    #[test]
    fn a_second_copy_put_aside_keeps_the_first() {
        let dir = dir();
        for (copy, aside) in [
            ("{ first", "connect.json.bad"),
            ("{ second", "connect.json.bad.1"),
        ] {
            std::fs::write(dir.join("connect.json"), copy).unwrap();
            let id = Store::open(&dir).id().to_owned();
            assert_eq!(std::fs::read_to_string(dir.join(aside)).unwrap(), copy);
            assert_eq!(Store::open(&dir).id(), id, "written anew, and read again");
        }
        assert_eq!(
            std::fs::read_to_string(dir.join("connect.json.bad")).unwrap(),
            "{ first"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// L5: a `connect.json` that doesn't read and can't be put aside is
    /// never written over; its pairings go on in memory.
    #[cfg(unix)]
    #[test]
    fn a_copy_that_cant_be_put_aside_is_never_written_over() {
        use std::os::unix::fs::PermissionsExt;
        let dir = dir();
        std::fs::write(dir.join("connect.json"), b"{ damaged").unwrap();
        let mode = |m| std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(m)).unwrap();
        mode(0o500);
        assert!(
            std::fs::write(dir.join("probe"), b"").is_err(),
            "the folder refuses to be written (a test run as root can't check this)"
        );
        let mut store = Store::open(&dir);
        // Writable again: the store still mustn't replace what is there.
        mode(0o700);
        store.pair(phone(), [1; 32]);
        assert_eq!(store.key(&phone().id), Some([1; 32]), "paired in memory");
        let kept = std::fs::read_to_string(dir.join("connect.json"));
        let _ = std::fs::remove_dir_all(&dir);
        // Not printed: it would hold the keys.
        assert!(
            kept.unwrap() == "{ damaged",
            "connect.json was written over"
        );
    }
}
