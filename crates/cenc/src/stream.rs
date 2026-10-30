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
//! included, because the fragments are too small for it to skip them. A
//! [`seek hint`](AudioStream::seek_hint) names the target first: decryption jumps
//! there, and everything before the target's fragment (less a margin) is served
//! as it is, encrypted or not, since the decoder never sees it.

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

use crate::fmp4::{EncryptedSample, Init};

/// Turns one sample's ciphertext into cleartext of the same length.
pub type Decrypt = Box<dyn FnMut(&[u8], &EncryptedSample) -> Result<Vec<u8>, String> + Send>;

pub struct AudioStream {
    shared: Arc<Shared>,
    len: u64,
    pos: u64,
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    /// How long each sample plays: they all hold the same number of frames.
    sample_duration: Duration,
}

struct State {
    data: Vec<u8>,
    /// Every encrypted sample known so far, in file order.
    samples: Vec<EncryptedSample>,
    decrypted: Vec<bool>,
    /// The bytes before this are whole and indexed: the movie box, then
    /// every fragment. A read waits for more past it.
    ready: usize,
    /// While the file downloads: what its movie box said, once it came.
    downloading: Option<Option<Init>>,
    /// A sample a reader is blocked on, for the decrypt thread to jump to.
    wanted: Option<usize>,
    failed: Option<String>,
    /// The CDM itself failed: nothing more is decrypted. A download that
    /// failed leaves what came to be decrypted and played.
    undecryptable: bool,
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

/// Hands a downloading track's bytes to its [`AudioStream`] as they come.
pub struct Feed {
    shared: Arc<Shared>,
}

impl AudioStream {
    /// A track with nothing to decrypt: the user's own upload.
    pub fn plain(data: Vec<u8>) -> Self {
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
        Self::with_state(
            len,
            sample_duration,
            State {
                decrypted: vec![false; samples.len()],
                samples,
                ready: data.len(),
                data,
                downloading: None,
                wanted: None,
                failed: None,
                undecryptable: false,
                abandoned: false,
                skippable_before: 0,
                reading: 0,
            },
        )
    }

    fn with_state(len: u64, sample_duration: Duration, state: State) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(state),
                changed: Condvar::new(),
                sample_duration,
            }),
            len,
            pos: 0,
        }
    }

    fn decrypt_in_background(self, decrypt: Decrypt) -> Self {
        let worker = self.shared.clone();
        std::thread::Builder::new()
            .name("decrypt".into())
            .spawn(move || worker.decrypt_all(decrypt))
            .expect("spawn the decrypt thread");
        self
    }

    /// Start decrypting `data` in the background. Its init segment must already
    /// be patched; only `samples` are encrypted, each `sample_duration` long.
    pub fn decrypting(
        data: Vec<u8>,
        samples: Vec<EncryptedSample>,
        sample_duration: Duration,
        decrypt: Decrypt,
    ) -> Self {
        Self::new(data, samples, sample_duration).decrypt_in_background(decrypt)
    }

    /// A fragmented MP4 of `len` bytes still downloading, which reads as it
    /// comes: its [`Feed`] takes the bytes, each fragment is indexed once it
    /// is whole, and decrypted in the background as the others are.
    pub fn downloading(len: u64, sample_duration: Duration, decrypt: Decrypt) -> (Self, Feed) {
        let stream = Self::with_state(
            len,
            sample_duration,
            State {
                data: Vec::with_capacity(usize::try_from(len).unwrap_or(0)),
                samples: Vec::new(),
                decrypted: Vec::new(),
                ready: 0,
                downloading: Some(None),
                wanted: None,
                failed: None,
                undecryptable: false,
                abandoned: false,
                skippable_before: 0,
                reading: 0,
            },
        )
        .decrypt_in_background(decrypt);
        let feed = Feed {
            shared: stream.shared.clone(),
        };
        (stream, feed)
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// `f` of the encrypted samples known so far.
    pub fn with_samples<T>(&self, f: impl FnOnce(&[EncryptedSample]) -> T) -> T {
        f(&self.shared.lock().samples)
    }

    /// Call with the target time before seeking the demuxer reading this stream.
    pub fn seek_hint(&self) -> impl FnMut(Duration) + Send + 'static {
        let shared = self.shared.clone();
        move |to| shared.prepare_seek(to)
    }
}

impl Feed {
    /// Add the next bytes; `false` once nobody reads the stream any more.
    pub fn push(&self, bytes: &[u8]) -> bool {
        let mut state = self.shared.lock();
        if state.abandoned {
            return false;
        }
        state.data.extend_from_slice(bytes);
        let state = &mut *state;
        if let Some(init) = &mut state.downloading {
            if init.is_none() {
                match Init::read(&state.data) {
                    Ok(Some(found)) => {
                        // Relabelled, so decoders take the track as plain AAC.
                        for &at in &found.enca_positions {
                            if let Some(kind) = state.data.get_mut(at + 4..at + 8) {
                                kind.copy_from_slice(b"mp4a");
                            }
                        }
                        state.ready = found.end;
                        *init = Some(found);
                    }
                    Ok(None) => {}
                    Err(e) => state.failed = Some(e),
                }
            }
            if let Some(init) = init {
                let before = state.samples.len();
                state.ready = init.fragments(&state.data, state.ready, &mut state.samples);
                let added = state.samples.len() - before;
                state.decrypted.extend(std::iter::repeat_n(false, added));
            }
        }
        self.shared.changed.notify_all();
        true
    }

    /// How many encrypted samples are known so far.
    pub fn samples(&self) -> usize {
        self.shared.lock().samples.len()
    }

    /// The download is over, cut short by `error` if any.
    pub fn end(&self, error: Option<String>) {
        let mut state = self.shared.lock();
        match error {
            Some(e) => state.failed = Some(e),
            None => {
                // What follows the last fragment (an index, say) is clear.
                state.ready = state.data.len();
                state.downloading = None;
            }
        }
        drop(state);
        self.shared.changed.notify_all();
    }
}

impl Drop for Feed {
    /// A download dropped before it said it ended was cut short.
    fn drop(&mut self) {
        let mut state = self.shared.lock();
        if state.downloading.is_some() && state.failed.is_none() {
            state.failed = Some("the download stopped".into());
        }
        drop(state);
        self.shared.changed.notify_all();
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn decrypt_all(&self, mut decrypt: Decrypt) {
        let ahead = if self.sample_duration.is_zero() {
            usize::MAX
        } else {
            (AHEAD.as_secs_f64() / self.sample_duration.as_secs_f64()).ceil() as usize
        };
        let mut next = 0;
        loop {
            // Copy the ciphertext out so the lock isn't held across the CDM call.
            let (index, ciphertext, sample) = {
                let mut state = self.lock();
                let index = loop {
                    if state.abandoned || state.undecryptable {
                        return;
                    }
                    if let Some(wanted) = state.wanted.take() {
                        next = wanted;
                    }
                    // What a reader waits on comes first; then on from it, up
                    // to the window ahead of the reader.
                    let n = state.samples.len();
                    let found = (next..n).find(|&i| !state.decrypted[i]);
                    let limit = state.reading.saturating_add(ahead);
                    match found {
                        Some(index) if index <= limit => break index,
                        _ if state.downloading.is_none() && state.decrypted.iter().all(|&d| d) => {
                            return;
                        }
                        _ => {
                            // Ahead enough, done from here, or waiting for
                            // the download: wait for the reader or the bytes.
                            next = next.min(state.reading);
                            state = self
                                .changed
                                .wait_timeout(state, NAP)
                                .unwrap_or_else(|e| e.into_inner())
                                .0;
                        }
                    }
                };
                let sample = state.samples[index].clone();
                (
                    index,
                    state.data[sample.start..sample.end()].to_vec(),
                    sample,
                )
            };

            let result = decrypt(&ciphertext, &sample);

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
            let failed = !state.decrypted[index];
            state.undecryptable = failed;
            drop(state);
            self.changed.notify_all();
            if failed {
                return;
            }
            next = index + 1;
        }
    }

    fn prepare_seek(&self, to: Duration) {
        if self.sample_duration.is_zero() {
            return;
        }
        let mut state = self.lock();
        if state.samples.is_empty() {
            return;
        }
        let safe_from = to.saturating_sub(SEEK_MARGIN);
        // A target past what has come lands on the last fragment so far.
        let index = ((safe_from.as_secs_f64() / self.sample_duration.as_secs_f64()) as usize)
            .min(state.samples.len() - 1);
        // The demuxer resumes at a fragment start; nothing before this one is decoded.
        state.skippable_before = state.samples[index].fragment;
        state.wanted = Some(index);
        state.reading = index;
        drop(state);
        self.changed.notify_all();
    }
}

impl State {
    /// The first sample overlapping `start..end` that has to be decrypted before
    /// it is read and isn't yet.
    fn pending_in(&self, start: usize, end: usize) -> Option<usize> {
        let first = self.samples.partition_point(|s| s.end() <= start);
        (first..self.samples.len())
            .take_while(|&i| self.samples[i].start < end)
            .find(|&i| !self.decrypted[i] && self.samples[i].end() > self.skippable_before)
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
        let reading = state.samples.partition_point(|s| s.end() <= start);
        if reading != state.reading {
            state.reading = reading;
            self.shared.changed.notify_all();
        }
        // A failure ends the track where it struck, not where it is read:
        // what came and was decrypted before still plays.
        let failed = |e: &String| io::Error::other(format!("decryption failed: {e}"));
        loop {
            // Only what has come, and been indexed, can be read.
            if start >= state.ready {
                if let Some(e) = &state.failed {
                    return Err(failed(e));
                }
                if state.downloading.is_none() {
                    return Ok(0);
                }
                state = self
                    .shared
                    .changed
                    .wait(state)
                    .unwrap_or_else(|e| e.into_inner());
                continue;
            }
            end = end.min(state.ready);
            match state.pending_in(start, end) {
                None => break,
                Some(index) => {
                    state.wanted = Some(index);
                    let sample_start = state.samples[index].start;
                    if sample_start > start {
                        end = sample_start;
                        break;
                    }
                    if let Some(e) = &state.failed {
                        return Err(failed(e));
                    }
                    self.shared.changed.notify_all();
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
        let reading = state
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
