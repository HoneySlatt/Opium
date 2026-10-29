//! Decoding and audio output.
//!
//! A [`Player`] owns an engine thread that decodes ahead into a ring of 32-bit
//! integer frames, and a PipeWire stream that plays from it while other programs
//! keep playing. The stream pulls the graph to the source's rate, so the music is
//! not resampled. A track set with [`Player::set_next`] follows the current one
//! without a gap when both have the same rate and channel count.

mod decoder;
mod engine;
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
            seek_hint: None,
        }
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

/// The source as decoded.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceFormat {
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u16,
    /// For lossless sources; lossy codecs have no inherent bit depth.
    pub bits_per_sample: Option<u32>,
    pub duration: Option<Duration>,
}

/// The output as opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputInfo {
    pub device: String,
    /// The rate the PipeWire graph runs at, as of the last [`Player::status`].
    /// When it differs from the source's, the music is being resampled.
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: &'static str,
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

    /// The track to follow the current one. Set it once per track: once it has
    /// started streaming in behind the current one it can't be replaced.
    pub fn set_next(&self, token: Token, source: Source) {
        self.send(Command::SetNext(token, source));
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
