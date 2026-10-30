//! Decoding and audio output.
//!
//! A [`Player`] owns an engine thread that decodes ahead into a ring of 32-bit
//! integer frames, and a PipeWire stream that plays from it while other programs
//! keep playing. The stream pulls the graph to the source's rate, so the music is
//! not resampled. A track set with [`Player::set_next`] follows the current one
//! without a gap when both have the same rate and channel count.

mod decoder;
mod engine;
mod gapless;
mod graph;
mod output;

use std::io::{Read, Seek};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

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
    commands: Sender<Command>,
    /// Counts the next tracks asked for, so that only the last one opened
    /// is taken.
    next_asked: std::sync::atomic::AtomicU64,
    events: Receiver<Event>,
    status: Arc<Mutex<Status>>,
    thread: Option<JoinHandle<()>>,
}

impl Player {
    pub fn new() -> Self {
        let (commands, command_rx) = std::sync::mpsc::channel();
        let (event_tx, events) = std::sync::mpsc::channel();
        let status = Arc::new(Mutex::new(Status::default()));
        let engine = Engine::new(status.clone(), event_tx);
        let thread = std::thread::Builder::new()
            .name("player".into())
            .spawn(move || engine.run(command_rx))
            .expect("spawn the player thread");
        Self {
            commands,
            next_asked: Default::default(),
            events,
            status,
            thread: Some(thread),
        }
    }

    fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    /// Play `source` now, dropping whatever was playing or queued.
    pub fn play(&self, token: Token, source: Source) {
        self.send(Command::Play(token, source));
    }

    /// The track to follow the current one. Set it once per track: to change
    /// it, [`clear_next`](Self::clear_next) first.
    pub fn set_next(&self, token: Token, source: Source) {
        let generation = self
            .next_asked
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.send(Command::Opening(generation));
        let commands = self.commands.clone();
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

    pub fn pause(&self) {
        self.send(Command::Pause);
    }

    pub fn resume(&self) {
        self.send(Command::Resume);
    }

    pub fn seek(&self, to: Duration) {
        self.send(Command::Seek(to));
    }

    pub fn stop(&self) {
        self.send(Command::Stop);
    }

    /// Below 1.0 the samples are scaled; at 1.0 they pass untouched.
    pub fn set_volume(&self, volume: f32) {
        self.send(Command::Volume(volume.clamp(0.0, 1.0)));
    }

    pub fn status(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn events(&self) -> &Receiver<Event> {
        &self.events
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.send(Command::Quit);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
