//! A downloaded track, readable as a file, that decrypts itself as it is read.
//!
//! The browser CDM takes about 1.5 ms per sample whatever its size, and a
//! four-minute track has around 9 000 of them, so decrypting everything up front
//! would hold playback back for ~15 s. Instead a thread decrypts in file order,
//! about 15× faster than real time, and a read only waits for a sample it starts
//! inside of. It keeps only [`AHEAD`] in front of the reader: the one CDM is
//! shared, and a track decrypting to its end would keep the next one waiting
//! for its licence. A read that reaches an encrypted sample further on stops short of
//! it: the box headers between samples are clear, so a demuxer walking fragment
//! headers to seek never waits for audio it is about to skip. Either way the
//! thread moves to where the reader is. CENC keeps sizes and every sample
//! decrypts on its own, which is what makes any order work.
//!
//! Seeking still makes the demuxer read every byte up to the target, audio
//! included, because Apple's fragments are too small for it to skip them. A
//! [`seek hint`](AudioStream::seek_hint) names the target first: decryption jumps
//! there, and everything before the target's fragment (less a margin) is served
//! as it is, encrypted or not, since the decoder never sees it.

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

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
    /// How long each sample plays: they all hold the same number of frames.
    sample_duration: Duration,
}

struct State {
    data: Vec<u8>,
    decrypted: Vec<bool>,
    /// A sample a reader is blocked on, for the decrypt thread to jump to.
    wanted: Option<usize>,
    failed: Option<String>,
    /// Set when the reader is dropped, so the thread stops early.
    abandoned: bool,
    /// Samples ending before this offset may be read undecrypted: a seek is
    /// walking past them.
    skippable_before: usize,
    /// The first sample at or after where the reader is.
    reading: usize,
}

/// How far before a seek target bytes still have to be decrypted, to cover the
/// difference between the estimated and the actual landing point.
const SEEK_MARGIN: Duration = Duration::from_secs(2);

/// How much audio is decrypted ahead of the reader: at 15× real time, far more
/// than the player ever waits for.
const AHEAD: Duration = Duration::from_secs(30);

/// How long the decrypt thread waits for the reader before looking again.
const NAP: Duration = Duration::from_millis(250);

impl AudioStream {
    /// A track with nothing to decrypt: the user's own upload.
    pub(crate) fn plain(data: Vec<u8>) -> Self {
        Self::new(data, Vec::new(), Duration::ZERO)
    }

    /// What the file is, as an extension for the decoder: Apple's own
    /// tracks are MP4, a user's upload is whatever they put in.
    pub fn extension(&self) -> &'static str {
        let state = self.shared.lock();
        match state.data.get(..4) {
            Some([b'I', b'D', b'3', _]) => "mp3",
            Some([0xFF, b, _, _]) if b & 0xE0 == 0xE0 => "mp3",
            Some(b"fLaC") => "flac",
            _ => "m4a",
        }
    }

    fn new(data: Vec<u8>, samples: Vec<EncryptedSample>, sample_duration: Duration) -> Self {
        let len = data.len() as u64;
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                decrypted: vec![false; samples.len()],
                data,
                wanted: None,
                failed: None,
                abandoned: false,
                skippable_before: 0,
                reading: 0,
            }),
            changed: Condvar::new(),
            samples,
            sample_duration,
        });
        Self {
            shared,
            len,
            pos: 0,
        }
    }

    /// Start decrypting `data` in the background. Its init segment must already
    /// be patched; only `samples` are encrypted, each `sample_duration` long.
    pub(crate) fn decrypting(
        data: Vec<u8>,
        samples: Vec<EncryptedSample>,
        sample_duration: Duration,
        decrypt: Decrypt,
    ) -> Self {
        let stream = Self::new(data, samples, sample_duration);
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

    /// Call with the target time before seeking the demuxer reading this stream.
    pub fn seek_hint(&self) -> impl FnMut(Duration) + Send + 'static {
        let shared = self.shared.clone();
        move |to| shared.prepare_seek(to)
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn decrypt_all(&self, mut decrypt: Decrypt) {
        let ahead = if self.sample_duration.is_zero() {
            self.samples.len()
        } else {
            (AHEAD.as_secs_f64() / self.sample_duration.as_secs_f64()).ceil() as usize
        };
        let mut next = 0;
        loop {
            // Copy the ciphertext out so the lock isn't held across the CDM call.
            let (index, ciphertext) = {
                let mut state = self.lock();
                loop {
                    if state.abandoned {
                        return;
                    }
                    if let Some(wanted) = state.wanted.take() {
                        next = wanted;
                    }
                    // What a reader waits on comes first; then on from it, up
                    // to the window ahead of the reader.
                    let n = self.samples.len();
                    let found = (next..n).find(|&i| !state.decrypted[i]);
                    let limit = state.reading.saturating_add(ahead);
                    match found {
                        Some(index) if index <= limit => break,
                        _ if state.decrypted.iter().all(|&d| d) => return,
                        _ => {
                            // Ahead enough, or done from here: wait for the reader.
                            next = next.min(state.reading);
                            state = self
                                .changed
                                .wait_timeout(state, NAP)
                                .unwrap_or_else(|e| e.into_inner())
                                .0;
                        }
                    }
                }
                let n = self.samples.len();
                let index = (next..n)
                    .find(|&i| !state.decrypted[i])
                    .expect("found above");
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

    fn prepare_seek(&self, to: Duration) {
        if self.samples.is_empty() || self.sample_duration.is_zero() {
            return;
        }
        let safe_from = to.saturating_sub(SEEK_MARGIN);
        let index = ((safe_from.as_secs_f64() / self.sample_duration.as_secs_f64()) as usize)
            .min(self.samples.len() - 1);
        let mut state = self.lock();
        // The demuxer resumes at a fragment start; nothing before this one is decoded.
        state.skippable_before = self.samples[index].fragment;
        state.wanted = Some(index);
        state.reading = index;
        drop(state);
        self.changed.notify_all();
    }

    /// The first sample overlapping `start..end` that has to be decrypted before
    /// it is read and isn't yet.
    fn pending_in(&self, state: &State, start: usize, end: usize) -> Option<usize> {
        let first = self.samples.partition_point(|s| s.end() <= start);
        (first..self.samples.len())
            .take_while(|&i| self.samples[i].start < end)
            .find(|&i| !state.decrypted[i] && self.samples[i].end() > state.skippable_before)
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
        // The window ahead starts where this read does, even one far from the
        // last, so the sample it may wait on is inside it.
        let reading = self.shared.samples.partition_point(|s| s.end() <= start);
        if reading != state.reading {
            state.reading = reading;
            self.shared.changed.notify_all();
        }
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
        // Where the window ahead starts, for the decrypt thread.
        let reading = self
            .shared
            .samples
            .partition_point(|s| s.end() <= self.pos as usize);
        if reading != state.reading {
            state.reading = reading;
            drop(state);
            self.shared.changed.notify_all();
        }
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
        self.pos =
            pos.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek before start"))?;
        Ok(self.pos)
    }
}

impl Drop for AudioStream {
    fn drop(&mut self) {
        self.shared.lock().abandoned = true;
        self.shared.changed.notify_all();
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
                fragment: 100 + i * 50,
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
        let mut stream = AudioStream::decrypting(data, samples, Duration::from_secs(1), xor());
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
        let mut stream = AudioStream::decrypting(data, samples, Duration::from_secs(1), never);
        let mut buf = [0u8; 200];
        assert_eq!(stream.read(&mut buf).unwrap(), 100);
        assert_eq!(&buf[..100], &clear[..100]);
    }

    #[test]
    fn seeking_into_a_sample_waits_for_it() {
        let (clear, data, samples) = track();
        let mut stream = AudioStream::decrypting(data, samples, Duration::from_secs(1), xor());
        stream.seek(SeekFrom::Start(210)).unwrap();
        let mut buf = [0u8; 100];
        let n = stream.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], &clear[210..210 + n]);
    }

    #[test]
    fn decryption_keeps_to_a_window_ahead_of_the_reader() {
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = count.clone();
        let decrypt: Decrypt = Box::new(move |c, _| {
            counted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(c.to_vec())
        });
        // 100 one-second samples: the window is the first 30 s and the next.
        let samples: Vec<EncryptedSample> = (0..100)
            .map(|i| EncryptedSample {
                fragment: i * 10,
                start: i * 10,
                len: 10,
                iv: [0; 16],
                subs: vec![],
            })
            .collect();
        let stream =
            AudioStream::decrypting(vec![0; 1000], samples, Duration::from_secs(1), decrypt);
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 31);
        drop(stream);
    }

    #[test]
    fn a_read_far_past_the_window_still_gets_its_sample() {
        let samples: Vec<EncryptedSample> = (0..100)
            .map(|i| EncryptedSample {
                fragment: i * 10,
                start: i * 10,
                len: 10,
                iv: [0; 16],
                subs: vec![],
            })
            .collect();
        let data: Vec<u8> = (0..1000).map(|i| (i % 251) as u8 ^ 0xFF).collect();
        let mut stream = AudioStream::decrypting(data, samples, Duration::from_secs(1), xor());
        let (done, finished) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            stream.seek(SeekFrom::Start(905)).unwrap();
            let mut buf = [0u8; 5];
            let n = stream.read(&mut buf).unwrap();
            done.send((n, buf[0])).unwrap();
        });
        let (n, first) = finished
            .recv_timeout(Duration::from_secs(2))
            .expect("the read came back");
        assert_eq!((n, first), (5, (905 % 251) as u8));
    }

    #[test]
    fn a_failed_decrypt_is_a_read_error() {
        let (_, data, samples) = track();
        let failing: Decrypt = Box::new(|_, _| Err("no key".into()));
        let mut stream = AudioStream::decrypting(data, samples, Duration::from_secs(1), failing);
        stream.seek(SeekFrom::Start(120)).unwrap();
        assert!(stream.read(&mut [0u8; 10]).is_err());
    }

    #[test]
    fn a_seek_hint_lets_the_walk_read_past_samples_before_the_target() {
        let (_, data, samples) = track();
        let never: Decrypt = Box::new(|_, _| {
            std::thread::park();
            Err("parked".into())
        });
        let mut stream =
            AudioStream::decrypting(data.clone(), samples, Duration::from_secs(1), never);
        // Sample 2 starts 2 s in; with the margin, everything before it is skippable.
        stream.seek_hint()(Duration::from_secs(4));
        let mut buf = [0u8; 250];
        assert_eq!(
            stream.read(&mut buf).unwrap(),
            200,
            "stops at sample 2, which must decrypt"
        );
        assert_eq!(
            &buf[..200],
            &data[..200],
            "earlier samples come through as they are"
        );
    }
}
