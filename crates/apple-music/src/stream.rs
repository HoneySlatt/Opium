//! A downloaded track, readable as a file, that decrypts itself as it is read.
//!
//! The browser CDM takes about 1.5 ms per sample whatever its size, and a
//! four-minute track has around 9 000 of them, so decrypting everything up front
//! would hold playback back for ~15 s. Instead a thread decrypts in file order,
//! about 15× faster than real time, and a read only waits for a sample it starts
//! inside of. A read that reaches an encrypted sample further on stops short of
//! it: the box headers between samples are clear, so a demuxer walking fragment
//! headers to seek never waits for audio it is about to skip. Either way the
//! thread moves to where the reader is. CENC keeps sizes and every sample
//! decrypts on its own, which is what makes any order work.

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use crate::cenc::EncryptedSample;

/// Turns one sample's ciphertext into cleartext of the same length.
pub(crate) type Decrypt = Box<dyn FnMut(&[u8], &EncryptedSample) -> Result<Vec<u8>, String> + Send>;

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
    pub(crate) fn decrypting(data: Vec<u8>, samples: Vec<EncryptedSample>, decrypt: Decrypt) -> Self {
        let stream = Self::new(data, samples);
        let worker = stream.shared.clone();
        std::thread::Builder::new()
            .name("am-decrypt".into())
            .spawn(move || worker.decrypt_all(decrypt))
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

    fn decrypt_all(&self, mut decrypt: Decrypt) {
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
            let result = decrypt(&ciphertext, sample);

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
                Err(e) => state.failed = Some(e),
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
        let mut end = (start + buf.len()).min(self.len as usize);

        let mut state = self.shared.lock();
        loop {
            if let Some(e) = &state.failed {
                return Err(io::Error::other(format!("decryption failed: {e}")));
            }
            match self.shared.pending_in(&state, start, end) {
                None => break,
                Some(index) => {
                    state.wanted = Some(index);
                    let sample_start = self.shared.samples[index].start;
                    if sample_start > start {
                        end = sample_start;
                        break;
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 100 clear bytes, then three 50-byte samples "encrypted" by XOR with 0xFF.
    fn track() -> (Vec<u8>, Vec<u8>, Vec<EncryptedSample>) {
        let clear: Vec<u8> = (0..250u32).map(|i| i as u8).collect();
        let mut data = clear.clone();
        let samples: Vec<EncryptedSample> = (0..3)
            .map(|i| EncryptedSample {
                start: 100 + i * 50,
                len: 50,
                iv: [0; 16],
                subs: vec![],
            })
            .collect();
        for s in &samples {
            data[s.start..s.end()].iter_mut().for_each(|b| *b ^= 0xFF);
        }
        (clear, data, samples)
    }

    fn xor() -> Decrypt {
        Box::new(|ciphertext, _| Ok(ciphertext.iter().map(|b| b ^ 0xFF).collect()))
    }

    #[test]
    fn reading_everything_yields_the_cleartext() {
        let (clear, data, samples) = track();
        let mut stream = AudioStream::decrypting(data, samples, xor());
        let mut out = Vec::new();
        stream.read_to_end(&mut out).unwrap();
        assert_eq!(out, clear);
    }

    #[test]
    fn a_read_stops_short_of_a_sample_not_yet_decrypted() {
        let (clear, data, samples) = track();
        // A decrypt that never runs: only the clear prefix is readable.
        let never: Decrypt = Box::new(|_, _| {
            std::thread::park();
            Err("parked".into())
        });
        let mut stream = AudioStream::decrypting(data, samples, never);
        let mut buf = [0u8; 200];
        assert_eq!(stream.read(&mut buf).unwrap(), 100);
        assert_eq!(&buf[..100], &clear[..100]);
    }

    #[test]
    fn seeking_into_a_sample_waits_for_it() {
        let (clear, data, samples) = track();
        let mut stream = AudioStream::decrypting(data, samples, xor());
        stream.seek(SeekFrom::Start(210)).unwrap();
        let mut buf = [0u8; 100];
        let n = stream.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], &clear[210..210 + n]);
    }

    #[test]
    fn a_failed_decrypt_is_a_read_error() {
        let (_, data, samples) = track();
        let failing: Decrypt = Box::new(|_, _| Err("no key".into()));
        let mut stream = AudioStream::decrypting(data, samples, failing);
        stream.seek(SeekFrom::Start(120)).unwrap();
        assert!(stream.read(&mut [0u8; 10]).is_err());
    }
}
