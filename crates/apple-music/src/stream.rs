//! A downloaded track, readable as a file, that decrypts itself as it is read.
//!
//! The browser CDM takes about 1.5 ms per sample whatever its size, and a
//! four-minute track has around 9 000 of them, so decrypting everything up front
//! would hold playback back for ~15 s. Instead a thread decrypts in file order,
//! about 15× faster than real time, and a read only waits for the samples it
//! covers. A read far ahead (a seek) moves the thread there first. CENC keeps
//! sizes and every sample decrypts on its own, which is what makes any order work.

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use widevine::Session;

use crate::cenc::EncryptedSample;

pub struct AudioStream {
    shared: Arc<Shared>,
    len: u64,
    pos: u64,
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    samples: Vec<EncryptedSample>,
}

struct State {
    data: Vec<u8>,
    decrypted: Vec<bool>,
    /// A sample a reader is blocked on, for the decrypt thread to jump to.
    wanted: Option<usize>,
    failed: Option<String>,
    /// Set when the reader is dropped, so the thread stops early.
    abandoned: bool,
}

impl AudioStream {
    /// A track with nothing to decrypt: the user's own upload.
    pub(crate) fn plain(data: Vec<u8>) -> Self {
        Self::new(data, Vec::new())
    }

    fn new(data: Vec<u8>, samples: Vec<EncryptedSample>) -> Self {
        let len = data.len() as u64;
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                decrypted: vec![false; samples.len()],
                data,
                wanted: None,
                failed: None,
                abandoned: false,
            }),
            changed: Condvar::new(),
            samples,
        });
        Self {
            shared,
            len,
            pos: 0,
        }
    }

    /// Start decrypting `data` in the background. Its init segment must already
    /// be patched; only `samples` are encrypted.
    pub(crate) fn decrypting(
        data: Vec<u8>,
        samples: Vec<EncryptedSample>,
        session: Session,
        key_id: Vec<u8>,
    ) -> Self {
        let stream = Self::new(data, samples);
        let worker = stream.shared.clone();
        std::thread::Builder::new()
            .name("am-decrypt".into())
            .spawn(move || worker.decrypt_all(&session, &key_id))
            .expect("spawn the decrypt thread");
        stream
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn decrypt_all(&self, session: &Session, key_id: &[u8]) {
        let mut next = 0;
        loop {
            // Copy the ciphertext out so the lock isn't held across the CDM call.
            let (index, ciphertext) = {
                let mut state = self.lock();
                if state.abandoned {
                    return;
                }
                if let Some(wanted) = state.wanted.take() {
                    next = wanted;
                }
                let n = self.samples.len();
                let Some(index) = (next..n).chain(0..next).find(|&i| !state.decrypted[i]) else {
                    return;
                };
                let s = &self.samples[index];
                (index, state.data[s.start..s.end()].to_vec())
            };

            let sample = &self.samples[index];
            let subsamples: Vec<(u32, u32)> =
                sample.subs.iter().map(|&(c, e)| (u32::from(c), e)).collect();
            let result = session.decrypt(&ciphertext, key_id, &sample.iv, &subsamples);

            let mut state = self.lock();
            match result {
                Ok(clear) if clear.len() == sample.len => {
                    state.data[sample.start..sample.end()].copy_from_slice(&clear);
                    state.decrypted[index] = true;
                }
                Ok(clear) => {
                    state.failed = Some(format!(
                        "the CDM returned {} bytes for a {}-byte sample",
                        clear.len(),
                        sample.len
                    ));
                }
                Err(e) => state.failed = Some(e.to_string()),
            }
            let failed = state.failed.is_some();
            drop(state);
            self.changed.notify_all();
            if failed {
                return;
            }
            next = index + 1;
        }
    }

    /// The first sample overlapping `start..end` that isn't decrypted yet.
    fn pending_in(&self, state: &State, start: usize, end: usize) -> Option<usize> {
        let first = self.samples.partition_point(|s| s.end() <= start);
        (first..self.samples.len())
            .take_while(|&i| self.samples[i].start < end)
            .find(|&i| !state.decrypted[i])
    }
}

impl Read for AudioStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.len || buf.is_empty() {
            return Ok(0);
        }
        let start = self.pos as usize;
        let end = (start + buf.len()).min(self.len as usize);

        let mut state = self.shared.lock();
        loop {
            if let Some(e) = &state.failed {
                return Err(io::Error::other(format!("decryption failed: {e}")));
            }
            match self.shared.pending_in(&state, start, end) {
                None => break,
                Some(index) => {
                    state.wanted = Some(index);
                    state = self
                        .shared
                        .changed
                        .wait(state)
                        .unwrap_or_else(|e| e.into_inner());
                }
            }
        }
        let n = end - start;
        buf[..n].copy_from_slice(&state.data[start..end]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for AudioStream {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let pos = match to {
            SeekFrom::Start(p) => Some(p),
            SeekFrom::End(d) => self.len.checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        };
        self.pos = pos.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek before start"))?;
        Ok(self.pos)
    }
}

impl Drop for AudioStream {
    fn drop(&mut self) {
        self.shared.lock().abandoned = true;
    }
}
