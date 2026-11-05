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

use crate::{Indexer, Movie, Sample, SampleTable};

/// Turns one sample's ciphertext into its cleartext, in place: CENC keeps sizes.
pub type Decrypt = Box<dyn FnMut(&mut [u8], &Sample<'_>) -> Result<(), String> + Send>;

/// A track readable as a file (`Read + Seek`) while a background thread decrypts its samples:
/// a read that reaches a sample not yet decrypted waits for it, or stops short of it.
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
    samples: Samples,
    /// Per sample of the table: whether it reads as clear, decrypted or never
    /// encrypted.
    decrypted: Vec<bool>,
    /// The bytes before this are whole and indexed: the movie box, then
    /// every fragment. A read waits for more past it.
    ready: usize,
    /// The file is still downloading.
    downloading: bool,
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

/// What is known of a file's samples.
enum Samples {
    /// A file with nothing to decrypt.
    None,
    /// A whole file, indexed at once.
    Whole(SampleTable),
    /// A file still downloading: nothing until its movie box has come, then
    /// its fragments as they come.
    Arriving(Option<Box<(Movie, Indexer)>>),
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
        Self::new(data, Samples::None, Duration::ZERO)
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

    fn new(data: Vec<u8>, samples: Samples, sample_duration: Duration) -> Self {
        let len = data.len() as u64;
        let decrypted = match samples.table() {
            Some(table) => table.iter().map(|s| !needs_decrypting(&s)).collect(),
            None => Vec::new(),
        };
        Self::with_state(
            len,
            sample_duration,
            State {
                decrypted,
                samples,
                ready: data.len(),
                data,
                downloading: false,
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

    /// Start decrypting `data` in the background. It must already read as a
    /// plain MP4 ([`crate::make_clear`]) but for the bytes of its `samples`,
    /// each `sample_duration` long.
    pub fn decrypting(
        data: Vec<u8>,
        samples: SampleTable,
        sample_duration: Duration,
        decrypt: Decrypt,
    ) -> Self {
        Self::new(data, Samples::Whole(samples), sample_duration).decrypt_in_background(decrypt)
    }

    /// A fragmented MP4 of `len` bytes still downloading, which reads as it
    /// comes: its `Feed` takes the bytes, each fragment is indexed once it
    /// is whole, and decrypted in the background as the others are.
    pub fn downloading(len: u64, sample_duration: Duration, decrypt: Decrypt) -> (Self, Feed) {
        let stream = Self::with_state(
            len,
            sample_duration,
            State {
                data: Vec::with_capacity(usize::try_from(len).unwrap_or(0)),
                samples: Samples::Arriving(None),
                decrypted: Vec::new(),
                ready: 0,
                downloading: true,
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

    /// The file's length in bytes: a download's announced length, before all of it has come.
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Whether the file has no byte at all.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// `f` of the samples known so far; `None` while a download has not
    /// brought its movie box yet, or for a file with nothing to decrypt.
    pub fn with_samples<T>(&self, f: impl FnOnce(&SampleTable) -> T) -> Option<T> {
        self.shared.lock().samples.table().map(f)
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
        if state.failed.is_none() {
            state.index_what_came();
        }
        drop(state);
        self.shared.changed.notify_all();
        true
    }

    /// How many samples are known so far.
    pub fn samples(&self) -> usize {
        self.shared.lock().count()
    }

    /// The download is over, cut short by `error` if any.
    pub fn end(&self, error: Option<String>) {
        let mut state = self.shared.lock();
        match error {
            Some(e) => state.failed = Some(e),
            None => state.finish_download(),
        }
        drop(state);
        self.shared.changed.notify_all();
    }
}

impl Drop for Feed {
    /// A download dropped before it said it ended was cut short.
    fn drop(&mut self) {
        let mut state = self.shared.lock();
        if state.downloading && state.failed.is_none() {
            state.failed = Some("the download stopped".into());
        }
        drop(state);
        self.shared.changed.notify_all();
    }
}

impl Samples {
    fn table(&self) -> Option<&SampleTable> {
        match self {
            Self::Whole(table) => Some(table),
            Self::Arriving(Some(arrived)) => Some(arrived.1.samples()),
            Self::None | Self::Arriving(None) => None,
        }
    }
}

/// Whether `sample`'s bytes must go through the CDM before they are read.
fn needs_decrypting(sample: &Sample<'_>) -> bool {
    sample.protected && sample.size > 0
}

/// Notes the samples `indexer` found since `decrypted` last grew, and makes what is indexed
/// read as a plain MP4 when anything was: new samples, or `indexed_more` (the movie box, or a
/// fragment without samples).
fn take_new_samples(
    data: &mut [u8],
    decrypted: &mut Vec<bool>,
    movie: &Movie,
    indexer: &Indexer,
    indexed_more: bool,
) {
    let table = indexer.samples();
    let known = decrypted.len();
    if table.len() > known {
        decrypted.extend(table.iter().skip(known).map(|s| !needs_decrypting(&s)));
    } else if !indexed_more {
        return;
    }
    // Its IVs read: what says the file is encrypted goes.
    crate::make_clear(data, movie, table);
}

/// `sample`, its subsample runs read from `runs`, so that it outlives the lock
/// on the table it came from.
fn detached<'a>(sample: &Sample<'_>, runs: &'a [(u32, u32)]) -> Sample<'a> {
    Sample {
        offset: sample.offset,
        size: sample.size,
        duration: sample.duration,
        fragment: sample.fragment,
        entry: sample.entry,
        key_id: sample.key_id,
        iv: sample.iv,
        iv_size: sample.iv_size,
        subsamples: runs,
        protected: sample.protected,
        scheme: sample.scheme,
        pattern: sample.pattern,
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
        // A sample's bytes and runs are copied out, so that the lock is not
        // held across the CDM call; both buffers serve every sample.
        let mut bytes = Vec::new();
        let mut runs = Vec::new();
        let mut next = 0;
        loop {
            let (index, sample) = {
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
                    let found = (next..state.count()).find(|&i| !state.decrypted[i]);
                    let limit = state.reading.saturating_add(ahead);
                    match found {
                        Some(index) if index <= limit => break index,
                        _ if !state.downloading && state.decrypted.iter().all(|&d| d) => {
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
                let sample = state.sample(index);
                bytes.clear();
                bytes.extend_from_slice(&state.data[sample.offset..sample.offset + sample.size]);
                runs.clear();
                runs.extend_from_slice(sample.subsamples);
                (index, detached(&sample, &runs))
            };

            let result = decrypt(&mut bytes, &sample);

            let mut state = self.lock();
            match result {
                Ok(()) => {
                    state.data[sample.offset..sample.offset + sample.size].copy_from_slice(&bytes);
                    state.decrypted[index] = true;
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
        let count = state.count();
        if count == 0 {
            return;
        }
        let safe_from = to.saturating_sub(SEEK_MARGIN);
        // A target past what has come lands on the last fragment so far.
        let index = ((safe_from.as_secs_f64() / self.sample_duration.as_secs_f64()) as usize)
            .min(count - 1);
        // The demuxer resumes at a fragment start; nothing before this one is decoded.
        state.skippable_before = state.sample(index).fragment;
        state.wanted = Some(index);
        state.reading = index;
        drop(state);
        self.changed.notify_all();
    }
}

impl State {
    /// How many samples are known.
    fn count(&self) -> usize {
        self.samples.table().map_or(0, SampleTable::len)
    }

    /// Sample `index`, which must be known.
    fn sample(&self, index: usize) -> Sample<'_> {
        self.samples
            .table()
            .and_then(|table| table.get(index))
            .expect("a sample the table holds")
    }

    /// How many samples end at or before `offset`: the samples are in file
    /// order, so this is also the first one that ends after it.
    fn ending_by(&self, offset: usize) -> usize {
        let (mut low, mut high) = (0, self.count());
        while low < high {
            let middle = low + (high - low) / 2;
            let sample = self.sample(middle);
            if sample.offset + sample.size <= offset {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        low
    }

    /// The first sample overlapping `start..end` that has to be decrypted before
    /// it is read and isn't yet.
    fn pending_in(&self, start: usize, end: usize) -> Option<usize> {
        (self.ending_by(start)..self.count())
            .take_while(|&i| self.sample(i).offset < end)
            .find(|&i| {
                let sample = self.sample(i);
                !self.decrypted[i] && sample.offset + sample.size > self.skippable_before
            })
    }

    /// Index what has come of a downloading file, and make what is indexed
    /// read as a plain MP4.
    fn index_what_came(&mut self) {
        let State {
            data,
            samples,
            decrypted,
            ready,
            failed,
            ..
        } = self;
        let Samples::Arriving(arrived) = samples else {
            return;
        };
        let mut movie_came = false;
        if arrived.is_none() {
            match Movie::read(data) {
                Ok(Some(movie)) => {
                    let indexer = Indexer::new(&movie);
                    *arrived = Some(Box::new((movie, indexer)));
                    movie_came = true;
                }
                Ok(None) => return,
                Err(e) => {
                    *failed = Some(e.to_string());
                    return;
                }
            }
        }
        let Some(arrived) = arrived else {
            return;
        };
        let (movie, indexer) = &mut **arrived;
        let step = indexer.advance(data);
        // A step that meets a malformed fragment may have indexed whole ones before it; and a
        // fragment may be whole without a sample.
        let indexed_more = movie_came || step.as_ref().is_ok_and(|&whole| whole > *ready);
        take_new_samples(data, decrypted, movie, indexer, indexed_more);
        match step {
            Ok(whole) => *ready = whole,
            Err(e) => *failed = Some(e.to_string()),
        }
    }

    /// The download ended whole: what follows its last fragment (an index,
    /// say) is clear, but a fragment it cut is an error.
    fn finish_download(&mut self) {
        let finished = match &mut self.samples {
            Samples::Arriving(Some(arrived)) => {
                let (movie, indexer) = &mut **arrived;
                let finished = indexer.finish(&self.data).map_err(|e| e.to_string());
                // The end makes whole what no step could: a last `moof` of size 0, which runs
                // to the end of the file.
                take_new_samples(&mut self.data, &mut self.decrypted, movie, indexer, true);
                finished
            }
            Samples::Arriving(None) => Err("the file ended before its movie box".into()),
            Samples::None | Samples::Whole(_) => Ok(()),
        };
        match finished {
            Ok(()) => {
                self.ready = self.data.len();
                self.downloading = false;
            }
            Err(e) => self.failed = Some(e),
        }
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
        let reading = state.ending_by(start);
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
                if !state.downloading {
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
                    let sample_start = state.sample(index).offset;
                    if sample_start > start {
                        end = sample_start;
                        break;
                    }
                    // A download that failed leaves what came to be decrypted and
                    // read: only a decrypt thread that failed ends the read here.
                    if state.undecryptable
                        && let Some(e) = &state.failed
                    {
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
        let reading = state.ending_by(self.pos as usize);
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

    /// A box: its 32-bit size, its type, its body.
    fn boxed(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let size = u32::try_from(8 + body.len()).expect("a small box");
        [&size.to_be_bytes()[..], kind, body].concat()
    }

    /// A full box of version 0: its body starts with four bytes of version and flags.
    fn full(kind: &[u8; 4], flags: u32, body: &[u8]) -> Vec<u8> {
        boxed(kind, &[&flags.to_be_bytes()[..], body].concat())
    }

    /// The movie box of one AAC track, scheme `cenc`, 16-byte IVs.
    fn movie_box() -> Vec<u8> {
        let mut tkhd = vec![0; 8];
        tkhd.extend(1u32.to_be_bytes());
        tkhd.extend([0; 68]);
        let mut mdhd = vec![0; 8];
        mdhd.extend(44_100u32.to_be_bytes());
        mdhd.extend([0; 8]);
        let mut tenc = vec![0, 0, 1, 16];
        tenc.extend([0x22; 16]);
        let sinf = boxed(
            b"sinf",
            &[
                boxed(b"frma", b"mp4a"),
                full(b"schm", 0, &[&b"cenc"[..], &[0, 1, 0, 0]].concat()),
                boxed(b"schi", &full(b"tenc", 0, &tenc)),
            ]
            .concat(),
        );
        let mut enca = vec![0; 6];
        enca.extend(1u16.to_be_bytes());
        enca.extend([0; 20]);
        enca.extend(sinf);
        let stsd = full(
            b"stsd",
            0,
            &[&1u32.to_be_bytes()[..], &boxed(b"enca", &enca)].concat(),
        );
        let minf = boxed(b"minf", &boxed(b"stbl", &stsd));
        let mdia = boxed(b"mdia", &[full(b"mdhd", 0, &mdhd), minf].concat());
        let trak = boxed(b"trak", &[full(b"tkhd", 7, &tkhd), mdia].concat());
        let mut trex = 1u32.to_be_bytes().to_vec();
        trex.extend(1u32.to_be_bytes());
        trex.extend([0; 12]);
        let mvex = boxed(b"mvex", &full(b"trex", 0, &trex));
        boxed(b"moov", &[trak, mvex].concat())
    }

    /// A fragment of `count` samples of `size` bytes, its data counted from
    /// its `moof`.
    fn fragment(count: u32, size: u32) -> Vec<u8> {
        let mut tfhd = 1u32.to_be_bytes().to_vec();
        tfhd.extend(1024u32.to_be_bytes());
        tfhd.extend(size.to_be_bytes());
        let tfhd = full(b"tfhd", 0x02_0018, &tfhd);
        let mut senc = count.to_be_bytes().to_vec();
        senc.extend((0..count).flat_map(|n| [n as u8; 16]));
        let senc = full(b"senc", 0, &senc);
        // The moof's size is known beforehand: a 16-byte mfhd, an 8-byte traf
        // header, then the boxes of the traf, the trun being 20 bytes long.
        let moof_size = 8 + 16 + 8 + tfhd.len() + 20 + senc.len();
        let data_offset = u32::try_from(moof_size + 8).expect("a small moof");
        let trun = full(
            b"trun",
            0x1,
            &[count.to_be_bytes(), data_offset.to_be_bytes()].concat(),
        );
        let traf = boxed(b"traf", &[tfhd, trun, senc].concat());
        let moof = boxed(
            b"moof",
            &[full(b"mfhd", 0, &1u32.to_be_bytes()), traf].concat(),
        );
        assert_eq!(moof.len(), moof_size);
        let mdat = boxed(b"mdat", &vec![0; (count * size) as usize]);
        [moof, mdat].concat()
    }

    /// A track of `fragments` fragments of `per_fragment` samples of `size`
    /// bytes: what the stream should read (the plain MP4, each byte of a
    /// sample its offset modulo 251), what it is given (the same, its samples
    /// "encrypted" by XOR with 0xFF), and its samples.
    fn track(fragments: u32, per_fragment: u32, size: u32) -> (Vec<u8>, Vec<u8>, SampleTable) {
        let (clear, mut data) = as_downloaded(fragments, per_fragment, size);
        let (movie, samples) = crate::index(&data).expect("the test file indexes");
        crate::make_clear(&mut data, &movie, &samples);
        (clear, data, samples)
    }

    fn xor() -> Decrypt {
        Box::new(|bytes, _| {
            bytes.iter_mut().for_each(|b| *b ^= 0xFF);
            Ok(())
        })
    }

    /// A decrypt that never comes back.
    fn never() -> Decrypt {
        Box::new(|_, _| {
            std::thread::park();
            Err("parked".into())
        })
    }

    fn offset_of(samples: &SampleTable, index: usize) -> usize {
        samples.get(index).expect("the sample").offset
    }

    #[test]
    fn reading_everything_yields_the_cleartext() {
        let (clear, data, samples) = track(1, 3, 50);
        let mut stream = AudioStream::decrypting(data, samples, Duration::from_secs(1), xor());
        let mut out = Vec::new();
        stream.read_to_end(&mut out).unwrap();
        assert_eq!(out, clear);
    }

    #[test]
    fn a_read_stops_short_of_a_sample_not_yet_decrypted() {
        let (clear, data, samples) = track(1, 3, 50);
        let first = offset_of(&samples, 0);
        let mut stream = AudioStream::decrypting(data, samples, Duration::from_secs(1), never());
        let mut buf = vec![0u8; first + 100];
        assert_eq!(stream.read(&mut buf).unwrap(), first);
        assert_eq!(&buf[..first], &clear[..first]);
    }

    #[test]
    fn seeking_into_a_sample_waits_for_it() {
        let (clear, data, samples) = track(1, 3, 50);
        let inside = offset_of(&samples, 2) + 10;
        let mut stream = AudioStream::decrypting(data, samples, Duration::from_secs(1), xor());
        stream.seek(SeekFrom::Start(inside as u64)).unwrap();
        let mut buf = [0u8; 100];
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0);
        assert_eq!(&buf[..n], &clear[inside..inside + n]);
    }

    #[test]
    fn decryption_keeps_to_a_window_ahead_of_the_reader() {
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = count.clone();
        let decrypt: Decrypt = Box::new(move |_, _| {
            counted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        });
        // 100 one-second samples: the window is the first 30 s and the next.
        let (_, data, samples) = track(1, 100, 10);
        let stream = AudioStream::decrypting(data, samples, Duration::from_secs(1), decrypt);
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 31);
        drop(stream);
    }

    #[test]
    fn a_read_far_past_the_window_still_gets_its_sample() {
        let (clear, data, samples) = track(1, 100, 10);
        let target = offset_of(&samples, 90) + 5;
        let mut stream = AudioStream::decrypting(data, samples, Duration::from_secs(1), xor());
        let (done, finished) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            stream.seek(SeekFrom::Start(target as u64)).unwrap();
            let mut buf = [0u8; 5];
            let n = stream.read(&mut buf).unwrap();
            done.send((n, buf[0])).unwrap();
        });
        let (n, first) = finished
            .recv_timeout(Duration::from_secs(2))
            .expect("the read came back");
        assert_eq!((n, first), (5, clear[target]));
    }

    #[test]
    fn a_failed_decrypt_is_a_read_error() {
        let (_, data, samples) = track(1, 3, 50);
        let inside = offset_of(&samples, 0) + 20;
        let failing: Decrypt = Box::new(|_, _| Err("no key".into()));
        let mut stream = AudioStream::decrypting(data, samples, Duration::from_secs(1), failing);
        stream.seek(SeekFrom::Start(inside as u64)).unwrap();
        assert!(stream.read(&mut [0u8; 10]).is_err());
    }

    #[test]
    fn a_seek_hint_lets_the_walk_read_past_samples_before_the_target() {
        // One sample a fragment, so that the target's fragment starts after
        // the samples before it.
        let (_, data, samples) = track(3, 1, 50);
        let last = offset_of(&samples, 2);
        let mut stream =
            AudioStream::decrypting(data.clone(), samples, Duration::from_secs(1), never());
        // Sample 2 starts 2 s in; with the margin, everything before it is skippable.
        stream.seek_hint()(Duration::from_secs(4));
        let mut buf = vec![0u8; data.len()];
        assert_eq!(
            stream.read(&mut buf).unwrap(),
            last,
            "stops at sample 2, which must decrypt"
        );
        assert_eq!(
            &buf[..last],
            &data[..last],
            "earlier samples come through as they are"
        );
    }

    /// What the stream should read of a track (as [`track`] says), and the
    /// file as a download brings it: its samples encrypted, its boxes saying
    /// so.
    fn as_downloaded(fragments: u32, per_fragment: u32, size: u32) -> (Vec<u8>, Vec<u8>) {
        let mut file = movie_box();
        for _ in 0..fragments {
            file.extend(fragment(per_fragment, size));
        }
        encrypted(file)
    }

    /// What the stream should read of `file`, and `file` as a download brings it: each byte of a
    /// sample its offset modulo 251, "encrypted" by XOR with 0xFF.
    fn encrypted(mut file: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
        let (movie, samples) = crate::index(&file).expect("the test file indexes");
        let mut clear = file.clone();
        crate::make_clear(&mut clear, &movie, &samples);
        for sample in samples.iter() {
            for at in sample.offset..sample.offset + sample.size {
                clear[at] = (at % 251) as u8;
                file[at] = clear[at] ^ 0xFF;
            }
        }
        (clear, file)
    }

    /// A file whose last fragment is a `moof` of size 0, which runs to the end of the file, its
    /// `count` samples of `size` bytes in an `mdat` before it.
    fn ending_in_an_open_moof(count: u32, size: u32) -> Vec<u8> {
        let movie = movie_box();
        let mdat = boxed(b"mdat", &vec![0; (count * size) as usize]);
        // Base data offset, default duration and default size.
        let mut tfhd = 1u32.to_be_bytes().to_vec();
        tfhd.extend((movie.len() as u64 + 8).to_be_bytes());
        tfhd.extend(1024u32.to_be_bytes());
        tfhd.extend(size.to_be_bytes());
        let tfhd = full(b"tfhd", 0x19, &tfhd);
        let trun = full(b"trun", 0, &count.to_be_bytes());
        let mut senc = count.to_be_bytes().to_vec();
        senc.extend((0..count).flat_map(|n| [n as u8; 16]));
        let senc = full(b"senc", 0, &senc);
        let traf = boxed(b"traf", &[tfhd, trun, senc].concat());
        let body = [full(b"mfhd", 0, &1u32.to_be_bytes()), traf].concat();
        let moof = [&0u32.to_be_bytes()[..], b"moof", &body].concat();
        [movie, mdat, moof].concat()
    }

    #[test]
    fn a_download_ending_in_an_open_moof_reads_its_samples_decrypted() {
        let (clear, file) = encrypted(ending_in_an_open_moof(3, 40));
        let (mut stream, feed) =
            AudioStream::downloading(file.len() as u64, Duration::from_secs(1), xor());
        feed.push(&file);
        // Only the end says the open moof is whole.
        assert_eq!(feed.samples(), 0);
        feed.end(None);
        let mut out = Vec::new();
        stream.read_to_end(&mut out).unwrap();
        assert_eq!(out, clear);
    }

    #[test]
    fn a_fragment_without_samples_is_made_clear_too() {
        let mut file = movie_box();
        file.extend(fragment(3, 40));
        let second = file.len();
        file.extend(fragment(0, 40));
        let (clear, file) = encrypted(file);
        let (mut stream, feed) =
            AudioStream::downloading(file.len() as u64, Duration::from_secs(1), xor());
        // The empty fragment comes alone: no sample comes with it.
        feed.push(&file[..second]);
        feed.push(&file[second..]);
        feed.end(None);
        let mut out = Vec::new();
        stream.read_to_end(&mut out).unwrap();
        assert_eq!(out, clear);
    }

    #[test]
    fn a_step_that_meets_a_malformed_fragment_keeps_the_whole_ones_before_it() {
        let (_, mut file) = as_downloaded(2, 3, 40);
        // The second fragment's tfhd says it holds a sample description index it has no room
        // for.
        let second = file.len() - fragment(3, 40).len();
        let flags = second + 8 + 16 + 8 + 8;
        assert_eq!(&file[flags - 4..flags], b"tfhd");
        file[flags + 3] |= 0x02;
        let decrypted = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = decrypted.clone();
        let decrypt: Decrypt = Box::new(move |_, _| {
            counted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        });
        let (mut stream, feed) =
            AudioStream::downloading(file.len() as u64, Duration::from_secs(1), decrypt);
        // One step brings the first fragment whole, and the malformed one.
        feed.push(&file);
        assert_eq!(feed.samples(), 3);
        // The decrypt thread goes on with the samples known, while the download still runs.
        let mut waited = 0;
        while decrypted.load(std::sync::atomic::Ordering::Relaxed) < 3 {
            assert!(
                waited < 200,
                "the first fragment's samples were never decrypted"
            );
            std::thread::sleep(Duration::from_millis(10));
            waited += 1;
        }
        feed.end(None);
        let mut out = Vec::new();
        assert!(stream.read_to_end(&mut out).is_err());
    }

    #[test]
    fn a_download_reads_as_it_comes() {
        let (clear, file) = as_downloaded(4, 5, 40);
        let (mut stream, feed) =
            AudioStream::downloading(file.len() as u64, Duration::from_secs(1), xor());
        let feeding = std::thread::spawn(move || {
            for chunk in file.chunks(37) {
                assert!(feed.push(chunk));
            }
            assert_eq!(feed.samples(), 20);
            feed.end(None);
        });
        let mut out = Vec::new();
        stream.read_to_end(&mut out).unwrap();
        feeding.join().unwrap();
        assert_eq!(out, clear);
    }

    #[test]
    fn a_download_cut_inside_a_fragment_fails_where_it_was_cut() {
        let (clear, file) = as_downloaded(2, 5, 40);
        // The second fragment's last sample never comes.
        let cut = file.len() - 20;
        let (mut stream, feed) =
            AudioStream::downloading(file.len() as u64, Duration::from_secs(1), xor());
        feed.push(&file[..cut]);
        feed.end(None);
        let second = clear.len() - fragment(5, 40).len();
        let mut out = Vec::new();
        assert!(stream.read_to_end(&mut out).is_err());
        assert_eq!(
            out,
            clear[..second],
            "the first fragment, nothing of the cut one"
        );
    }
}
