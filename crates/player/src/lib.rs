//! Decoding and audio output.
//!
//! A [`Player`] owns an engine thread that decodes ahead into a ring of 32-bit
//! float frames, and a PipeWire stream that plays from it while other programs
//! keep playing. The stream pulls the graph to the source's rate, so the music is
//! not resampled. A track set with [`Player::set_next`] follows the current one
//! without a gap when both have the same rate and channel count.

mod decoder;
mod engine;
mod gapless;
mod graph;
mod output;

use std::io::{Read, Seek};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use symphonia::core::io::MediaSource;

use engine::{Command, Engine};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unreadable audio: {0}")]
    Decode(String),
    #[error("audio output: {0}")]
    Output(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// The caller's name for a source, echoed back in events and status.
pub type Token = u64;

/// Something to play: any seekable bytes of known length.
pub struct Source {
    reader: Box<dyn MediaSource>,
    extension: Option<String>,
    /// An AAC encode's priming frames, when its file doesn't say.
    priming: Option<u32>,
    seek_hint: Option<SeekHint>,
}

/// Told the target time just before the demuxer seeks.
pub(crate) type SeekHint = Box<dyn FnMut(Duration) + Send>;

impl Source {
    /// `extension` is a format hint (`"m4a"`, `"flac"`…).
    pub fn new<R>(reader: R, len: u64, extension: Option<&str>) -> Self
    where
        R: Read + Seek + Send + Sync + 'static,
    {
        Self {
            reader: Box::new(Bytes { inner: reader, len }),
            extension: extension.map(str::to_owned),
            priming: None,
            seek_hint: None,
        }
    }

    /// A stream that only goes forward and has no end in sight, as live radio:
    /// no seeking, no length.
    pub fn stream<R>(reader: R, extension: Option<&str>) -> Self
    where
        R: Read + Send + Sync + 'static,
    {
        Self {
            reader: Box::new(Forward(reader)),
            extension: extension.map(str::to_owned),
            priming: None,
            seek_hint: None,
        }
    }

    /// The frames an AAC encoder put before the music, for a file that
    /// doesn't say (Apple Music's: 2112), to leave them out.
    pub fn with_priming(mut self, frames: u32) -> Self {
        self.priming = Some(frames);
        self
    }

    /// Have `hint` told where a seek is headed before it happens, for sources
    /// that can prepare (decrypt ahead, fetch a range…).
    pub fn with_seek_hint(mut self, hint: impl FnMut(Duration) + Send + 'static) -> Self {
        self.seek_hint = Some(Box::new(hint));
        self
    }
}

struct Bytes<R> {
    inner: R,
    len: u64,
}

impl<R: Read> Read for Bytes<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R: Seek> Seek for Bytes<R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl<R: Read + Seek + Send + Sync> MediaSource for Bytes<R> {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}

struct Forward<R>(R);

impl<R: Read> Read for Forward<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl<R> Seek for Forward<R> {
    fn seek(&mut self, _: std::io::SeekFrom) -> std::io::Result<u64> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "a live stream can't seek",
        ))
    }
}

impl<R: Read + Send + Sync> MediaSource for Forward<R> {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

/// The source as decoded.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceFormat {
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u16,
    /// Which speaker each channel is for, as Symphonia's `Position` bits
    /// (one per channel, in order), when the file says.
    pub layout: Option<u64>,
    /// For lossless sources; lossy codecs have no inherent bit depth.
    pub bits_per_sample: Option<u32>,
    pub duration: Option<Duration>,
}

/// What shares the device with the music, from the PipeWire graph.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mixing {
    /// The device the stream plays into, as PipeWire describes it.
    pub device: Option<String>,
    /// Other programs playing into the same device right now: their sound is
    /// mixed with the music.
    pub others: Vec<String>,
    /// The device's software volume as a linear gain, when it isn't unity.
    pub device_volume: Option<f32>,
    pub device_muted: bool,
}

/// The output as opened.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputInfo {
    /// The device's name, or "PipeWire" until the graph says.
    pub device: String,
    /// The rate the PipeWire graph runs at, as of the last [`Player::status`].
    /// When it differs from the source's, the music is being resampled.
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: &'static str,
    pub mixing: Mixing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// `token` is now audible: after [`Player::play`], or when a queued track
    /// takes over.
    Started {
        token: Token,
        source: SourceFormat,
        output: OutputInfo,
    },
    /// The last track played to the end and nothing is queued.
    Finished,
    Error {
        token: Option<Token>,
        message: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    pub token: Option<Token>,
    pub position: Duration,
    pub paused: bool,
    pub source: Option<SourceFormat>,
    pub output: Option<OutputInfo>,
}

pub struct Player {
    engine: Mutex<Running>,
    /// Counts the next tracks asked for, so that only the last one opened
    /// is taken.
    next_asked: AtomicU64,
    events_tx: Sender<Event>,
    events: Receiver<Event>,
    /// As last set, for an engine started anew.
    volume: AtomicU32,
    paused: AtomicBool,
}

/// An engine thread, and what the player shares with it.
struct Running {
    commands: Sender<Command>,
    status: Arc<Mutex<Status>>,
    shared: Arc<engine::Shared>,
    thread: Option<JoinHandle<()>>,
}

/// How long the engine may wait on a read before it counts as stuck: a
/// download that stalled (the network gone) holds it until its timeout.
const STALLED: Duration = Duration::from_millis(400);

impl Player {
    pub fn new() -> Self {
        let (events_tx, events) = std::sync::mpsc::channel();
        Self {
            engine: Mutex::new(start(events_tx.clone())),
            next_asked: Default::default(),
            events_tx,
            events,
            volume: AtomicU32::new(1.0_f32.to_bits()),
            paused: AtomicBool::new(false),
        }
    }

    fn engine(&self) -> std::sync::MutexGuard<'_, Running> {
        self.engine.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn send(&self, command: Command) {
        let _ = self.engine().commands.send(command);
    }

    /// Whether the engine has been waiting on a read for a while: what it
    /// is asked then waits too, unless the player starts afresh.
    pub fn stalled(&self) -> bool {
        self.engine().shared.waiting_for() > STALLED
    }

    /// The engine stuck on a read is let go of: silenced now, it ends once
    /// the read gives up, and a new one takes over at once, at the volume
    /// and pause as set.
    fn start_afresh_if_stalled(&self) {
        let mut engine = self.engine();
        if engine.shared.waiting_for() <= STALLED {
            return;
        }
        engine.shared.retire();
        let fresh = start(self.events_tx.clone());
        let _ = fresh.commands.send(Command::Volume(f32::from_bits(
            self.volume.load(Ordering::Relaxed),
        )));
        if self.paused.load(Ordering::Relaxed) {
            let _ = fresh.commands.send(Command::Pause);
        }
        // Not joined: it may not wake for a while.
        let _stuck = std::mem::replace(&mut *engine, fresh);
    }

    /// Play `source` now, dropping whatever was playing or queued.
    pub fn play(&self, token: Token, source: Source) {
        self.start_afresh_if_stalled();
        self.send(Command::Play(token, source));
    }

    /// The track to follow the current one. Set it once per track: to change
    /// it, [`clear_next`](Self::clear_next) first.
    pub fn set_next(&self, token: Token, source: Source) {
        let generation = self.next_asked.fetch_add(1, Ordering::Relaxed);
        self.send(Command::Opening(generation));
        let commands = self.engine().commands.clone();
        let opened = std::thread::Builder::new()
            .name("player-open".into())
            .spawn(move || {
                let decoder = decoder::Decoder::open(source).map_err(|e| e.to_string());
                let _ = commands.send(Command::Opened {
                    token,
                    generation,
                    decoder,
                });
            });
        if let Err(e) = opened {
            self.send(Command::Opened {
                token,
                generation,
                decoder: Err(e.to_string()),
            });
        }
    }

    /// Drop the track set with [`set_next`](Self::set_next), even once it has
    /// started streaming in behind the current one.
    pub fn clear_next(&self) {
        self.send(Command::ClearNext);
    }

    /// Play the track set with [`set_next`](Self::set_next) now.
    pub fn skip_to_next(&self) {
        self.send(Command::SkipToNext);
    }

    /// Heard at once, even with the engine waiting on a read.
    pub fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
        self.engine().shared.output_paused(true);
        self.send(Command::Pause);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
        self.engine().shared.output_paused(false);
        self.send(Command::Resume);
    }

    pub fn seek(&self, to: Duration) {
        self.send(Command::Seek(to));
    }

    pub fn stop(&self) {
        self.start_afresh_if_stalled();
        self.send(Command::Stop);
    }

    /// Below 1.0 the samples are scaled; at 1.0 they pass untouched. Heard
    /// at once, even with the engine waiting on a read.
    pub fn set_volume(&self, volume: f32) {
        let volume = volume.clamp(0.0, 1.0);
        self.volume.store(volume.to_bits(), Ordering::Relaxed);
        self.engine().shared.output_volume(volume);
        self.send(Command::Volume(volume));
    }

    pub fn status(&self) -> Status {
        let status = self.engine().status.clone();
        status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn events(&self) -> &Receiver<Event> {
        &self.events
    }
}

/// An engine on a thread of its own.
fn start(events: Sender<Event>) -> Running {
    let (commands, command_rx) = std::sync::mpsc::channel();
    let status = Arc::new(Mutex::new(Status::default()));
    let shared = Arc::new(engine::Shared::default());
    let engine = Engine::new(status.clone(), events, shared.clone());
    let thread = std::thread::Builder::new()
        .name("player".into())
        .spawn(move || engine.run(command_rx))
        .expect("spawn the player thread");
    Running {
        commands,
        status,
        shared,
        thread: Some(thread),
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let mut engine = self.engine();
        let _ = engine.commands.send(Command::Quit);
        let Some(thread) = engine.thread.take() else {
            return;
        };
        // Waited for, but not for long: an engine parked on a read that the
        // network dropped would hold the window open until the read gives
        // up, a minute later. It is let go of instead, silenced, and ends
        // on its own once the read does.
        let until = Instant::now() + Duration::from_millis(500);
        while !thread.is_finished() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
        if thread.is_finished() {
            let _ = thread.join();
        } else {
            engine.shared.retire();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A download that never comes, as with the network gone.
    struct Stalled(Mutex<std::sync::mpsc::Receiver<()>>);

    impl Read for Stalled {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            let _ = self.0.lock().unwrap().recv();
            Ok(0)
        }
    }

    impl Seek for Stalled {
        fn seek(&mut self, _: std::io::SeekFrom) -> std::io::Result<u64> {
            Ok(0)
        }
    }

    #[test]
    fn a_player_stuck_on_a_read_starts_afresh() {
        let (wake, stalled) = std::sync::mpsc::channel();
        let player = Player::new();
        player.play(
            1,
            Source::new(Stalled(Mutex::new(stalled)), 1_000_000, Some("flac")),
        );
        std::thread::sleep(STALLED + Duration::from_millis(200));
        assert!(player.stalled());
        // Stop doesn't wait on it: a fresh engine takes over.
        player.stop();
        assert!(!player.stalled());
        assert_eq!(player.status().token, None);
        // The stuck one, woken, ends without a word.
        drop(wake);
        std::thread::sleep(Duration::from_millis(100));
        assert!(player.events().try_recv().is_err());
    }
}
