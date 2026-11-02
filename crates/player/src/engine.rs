//! The engine thread: decodes ahead into the ring, follows what the output has
//! actually played, and turns commands into changes to both.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rtrb::{Producer, RingBuffer};

use crate::decoder::Decoder;
use crate::output::{Output, OutputState};
use crate::{Event, OutputInfo, Source, Status, Token};

pub(crate) enum Command {
    Play(Token, Source),
    /// A next track is being opened, on a thread of its own: opening one
    /// may wait on its download, which mustn't starve the track playing.
    Opening(u64),
    /// The next track, opened.
    Opened {
        token: Token,
        generation: u64,
        decoder: std::result::Result<Decoder, String>,
    },
    SkipToNext,
    /// The next track isn't wanted any more: the queue changed.
    ClearNext,
    Pause,
    Resume,
    Seek(Duration),
    Stop,
    Volume(f32),
    Quit,
}

struct Item {
    token: Token,
    decoder: Decoder,
}

/// What the player reaches without going through the engine's commands,
/// which wait while the engine waits on a read.
#[derive(Default)]
pub(crate) struct Shared {
    /// When the engine began waiting on a read, in ms from [`epoch`]; 0
    /// while it isn't.
    waiting_since: AtomicU64,
    /// Let go of: it ends at its next turn, saying nothing more.
    retired: AtomicBool,
    /// The output playing, to pause or set the volume of at once.
    output: Mutex<Option<Arc<OutputState>>>,
}

fn epoch_ms() -> u64 {
    static EPOCH: std::sync::LazyLock<Instant> = std::sync::LazyLock::new(Instant::now);
    EPOCH.elapsed().as_millis() as u64 + 1
}

impl Shared {
    /// How long the engine has been waiting on the read it is in.
    pub fn waiting_for(&self) -> Duration {
        match self.waiting_since.load(Ordering::Relaxed) {
            0 => Duration::ZERO,
            since => Duration::from_millis(epoch_ms().saturating_sub(since)),
        }
    }

    /// Run `read`, which may wait on the network, saying so meanwhile.
    fn waiting<T>(&self, read: impl FnOnce() -> T) -> T {
        self.waiting_since.store(epoch_ms(), Ordering::Relaxed);
        let result = read();
        self.waiting_since.store(0, Ordering::Relaxed);
        result
    }

    /// Silence it now; it ends once its read gives up.
    pub fn retire(&self) {
        self.retired.store(true, Ordering::Relaxed);
        self.output_paused(true);
    }

    fn retired(&self) -> bool {
        self.retired.load(Ordering::Relaxed)
    }

    fn output(&self) -> Option<Arc<OutputState>> {
        self.output
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn output_paused(&self, paused: bool) {
        if let Some(output) = self.output() {
            output.paused.store(paused, Ordering::Relaxed);
        }
    }

    pub fn output_volume(&self, volume: f32) {
        if let Some(output) = self.output() {
            output.volume.store(volume.to_bits(), Ordering::Relaxed);
        }
    }
}

/// A track with frames in the ring, in play order.
struct Segment {
    item: Item,
    /// The output frame at which this segment's `base_time` plays.
    start_frame: u64,
    base_time: Duration,
    decoded_all: bool,
}

struct Sink {
    _output: Output,
    info: OutputInfo,
    state: Arc<OutputState>,
    producer: Producer<f32>,
    channels: u64,
    layout: Option<u64>,
    sample_rate: u32,
}

impl Sink {
    fn played(&self) -> u64 {
        self.state.played.load(Ordering::Relaxed)
    }

    /// The frames heard so far: those played, less what the graph and the
    /// device still hold.
    fn heard(&self) -> u64 {
        self.played()
            .saturating_sub(self.state.latency.load(Ordering::Relaxed))
    }

    /// The output as it is now, with the rate the graph actually runs at.
    fn live_info(&self) -> OutputInfo {
        let mut info = self.info.clone();
        let graph_rate = self.state.graph_rate.load(Ordering::Relaxed);
        if graph_rate > 0 {
            info.sample_rate = graph_rate;
        }
        info.mixing = self
            .state
            .mixing
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if let Some(device) = &info.mixing.device {
            info.device = device.clone();
        }
        info
    }
}

/// How often, and how long, to try for a new output after losing one.
const RETRY_EVERY: Duration = Duration::from_millis(500);
const RETRY_FOR: Duration = Duration::from_secs(20);

struct Recovering {
    item: Item,
    at: Duration,
    next_try: Instant,
    give_up: Instant,
}

pub(crate) struct Engine {
    volume: f32,
    paused: bool,
    sink: Option<Sink>,
    segments: VecDeque<Segment>,
    /// The track to follow the last segment, not decoding yet.
    next: Option<Item>,
    /// The next track being opened, by the generation it was asked with.
    opening: Option<u64>,
    /// Skip to it as soon as it is open.
    skip_when_opened: bool,
    /// A next track whose format needs another output; waits for the ring to drain.
    switch: Option<Item>,
    /// The track playing when its output was lost, to go on with where it
    /// was once PipeWire is back.
    recovering: Option<Recovering>,
    /// Samples pushed into the ring since the output was opened.
    written: u64,
    /// A seek waits for the output to drop what it held.
    flushing: bool,
    /// Decoded samples that didn't fit in the ring yet.
    pending: Vec<f32>,
    pending_at: usize,
    status: Arc<Mutex<Status>>,
    events: Sender<Event>,
    shared: Arc<Shared>,
}

/// How much audio the ring holds between the decoder and the device.
const RING_SECONDS: usize = 2;
/// How long the engine sleeps when there is nothing to decode.
const IDLE: Duration = Duration::from_millis(10);

impl Engine {
    pub fn new(status: Arc<Mutex<Status>>, events: Sender<Event>, shared: Arc<Shared>) -> Self {
        Self {
            volume: 1.0,
            paused: false,
            sink: None,
            segments: VecDeque::new(),
            next: None,
            opening: None,
            skip_when_opened: false,
            switch: None,
            recovering: None,
            written: 0,
            flushing: false,
            pending: Vec::new(),
            pending_at: 0,
            status,
            events,
            shared,
        }
    }

    pub fn run(mut self, commands: Receiver<Command>) {
        loop {
            let command = if self.can_decode() {
                commands.try_recv().map_err(|e| match e {
                    std::sync::mpsc::TryRecvError::Empty => RecvTimeoutError::Timeout,
                    std::sync::mpsc::TryRecvError::Disconnected => RecvTimeoutError::Disconnected,
                })
            } else {
                commands.recv_timeout(self.idle())
            };
            match command {
                Ok(Command::Quit) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => {}
            }
            // Let go of while it waited on a read: another plays now.
            if self.shared.retired() {
                return;
            }
            self.advance();
            self.pump();
            if self.shared.retired() {
                return;
            }
            self.publish();
        }
    }

    /// How long to wait for a command when there is nothing to decode: a
    /// command wakes it at once, so with nothing playing it rests, while a
    /// track playing out of a full ring is watched closely for its end.
    fn idle(&self) -> Duration {
        if self.recovering.is_some()
            || self.opening.is_some()
            || self.switch.is_some()
            || self.flushing
        {
            IDLE
        } else if self.sink.is_none() {
            Duration::from_secs(1)
        } else if self.paused {
            Duration::from_millis(250)
        } else {
            IDLE
        }
    }

    fn emit(&self, event: Event) {
        if !self.shared.retired() {
            let _ = self.events.send(event);
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Play(token, source) => {
                self.next = None;
                self.switch = None;
                self.recovering = None;
                self.opening = None;
                self.skip_when_opened = false;
                if let Some(item) = self.open(token, source) {
                    self.start(item, Duration::ZERO);
                }
            }
            Command::Opening(generation) => self.opening = Some(generation),
            // Opened for a queue that has since changed: dropped.
            Command::Opened { generation, .. } if self.opening != Some(generation) => {}
            Command::Opened { token, decoder, .. } => {
                self.opening = None;
                self.switch = None;
                self.next = match decoder {
                    Ok(decoder) => Some(Item { token, decoder }),
                    Err(message) => {
                        self.emit(Event::Error {
                            token: Some(token),
                            message,
                        });
                        None
                    }
                };
                if std::mem::take(&mut self.skip_when_opened) {
                    if let Some(item) = self.next.take() {
                        self.start(item, Duration::ZERO);
                    }
                } else if self.segments.back().is_some_and(|s| s.decoded_all) {
                    self.queue_next();
                }
            }
            // The next track, already open, plays now: nothing to fetch again.
            Command::SkipToNext => {
                self.park_queued();
                if let Some(item) = self.next.take() {
                    self.start(item, Duration::ZERO);
                } else if self.opening.is_some() {
                    self.skip_when_opened = true;
                }
            }
            Command::ClearNext => {
                self.opening = None;
                self.skip_when_opened = false;
                self.switch = None;
                // Already in the ring behind the playing one: the output
                // drops it on reaching it, the one playing untouched.
                if self.segments.len() > 1
                    && let Some(sink) = &self.sink
                {
                    let boundary = self.segments[1].start_frame * sink.channels;
                    let len = self.written.saturating_sub(boundary);
                    if sink.state.cut_at.load(Ordering::Acquire) == u64::MAX {
                        sink.state.cut_len.store(len, Ordering::Release);
                        sink.state.cut_at.store(boundary, Ordering::Release);
                    } else {
                        // Another one taken out before it was reached: both go.
                        sink.state.cut_len.fetch_add(len, Ordering::AcqRel);
                    }
                    self.written = boundary;
                    self.segments.truncate(1);
                    self.pending.clear();
                    self.pending_at = 0;
                }
                self.next = None;
            }
            Command::Pause => self.set_paused(true),
            Command::Resume => self.set_paused(false),
            Command::Seek(to) => self.seek(to),
            Command::Stop => {
                self.recovering = None;
                self.segments.clear();
                self.opening = None;
                self.skip_when_opened = false;
                self.next = None;
                self.switch = None;
                self.fade_out();
                self.sink = None;
            }
            Command::Volume(volume) => {
                self.volume = volume;
                if let Some(sink) = &self.sink {
                    sink.state.volume.store(volume.to_bits(), Ordering::Relaxed);
                }
            }
            Command::Quit => {}
        }
    }

    fn open(&self, token: Token, source: Source) -> Option<Item> {
        match self.shared.waiting(|| Decoder::open(source)) {
            Ok(decoder) => Some(Item { token, decoder }),
            Err(e) => {
                self.emit(Event::Error {
                    token: Some(token),
                    message: e.to_string(),
                });
                None
            }
        }
    }

    /// Open an output for `item`'s format and play it from `at`.
    fn start(&mut self, item: Item, at: Duration) {
        // Another track asked for: the one waiting for an output to come
        // back is no longer wanted.
        self.recovering = None;
        if let Err(failed) = self.try_start(item, at) {
            let (item, e) = *failed;
            self.emit(Event::Error {
                token: Some(item.token),
                message: e.to_string(),
            });
        }
    }

    /// Open an output for `item`'s format and play it from `at`; the item
    /// back if no output would open.
    fn try_start(&mut self, mut item: Item, at: Duration) -> Result<(), Box<(Item, crate::Error)>> {
        self.fade_out();
        self.sink = None;
        self.flushing = false;
        self.segments.clear();
        self.pending.clear();
        self.pending_at = 0;
        self.written = 0;

        let format = item.decoder.info.clone();
        let base_time = if at.is_zero() {
            at
        } else {
            match self.shared.waiting(|| item.decoder.seek(at)) {
                Ok(actual) => actual,
                Err(_) => Duration::ZERO,
            }
        };
        let channels = format.channels;
        let capacity = format.sample_rate as usize * usize::from(channels) * RING_SECONDS;
        let (producer, consumer) = RingBuffer::new(capacity);
        let state = Arc::new(OutputState::new(self.volume));
        state.paused.store(self.paused, Ordering::Relaxed);

        match Output::open(
            format.sample_rate,
            channels,
            format.layout,
            consumer,
            state.clone(),
        ) {
            Ok((output, info)) => {
                *self.shared.output.lock().unwrap_or_else(|e| e.into_inner()) = Some(state.clone());
                self.emit(Event::Started {
                    token: item.token,
                    source: format.clone(),
                    output: info.clone(),
                });
                self.sink = Some(Sink {
                    _output: output,
                    info,
                    state,
                    producer,
                    channels: u64::from(channels),
                    layout: format.layout,
                    sample_rate: format.sample_rate,
                });
                self.segments.push_back(Segment {
                    item,
                    start_frame: 0,
                    base_time,
                    decoded_all: false,
                });
                Ok(())
            }
            Err(e) => Err(Box::new((item, e))),
        }
    }

    /// Bring what sounds down to silence before its output closes, as a
    /// seek or a pause does: cut at full level, it would click. A stream
    /// not running (paused, or still starting) is closed as it is.
    fn fade_out(&self) {
        let Some(sink) = self.sink.as_ref().filter(|_| !self.paused) else {
            return;
        };
        sink.state.flush.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_millis(100);
        while sink.state.flush.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        if let Some(sink) = &self.sink {
            sink.state.paused.store(paused, Ordering::Relaxed);
        }
    }

    /// Take every track after the playing one back out of the ring's future, so
    /// the playing one can be moved. The first goes back to `next`.
    fn park_queued(&mut self) {
        let start = self.segments.len().min(1);
        let queued = self.segments.drain(start..).map(|s| s.item);
        let parked: Vec<Item> = queued.chain(self.switch.take()).collect();
        for mut item in parked {
            if self.next.is_none() && item.decoder.seek(Duration::ZERO).is_ok() {
                self.next = Some(item);
            }
        }
    }

    fn seek(&mut self, to: Duration) {
        self.park_queued();
        let (Some(sink), Some(front)) = (&self.sink, self.segments.front_mut()) else {
            return;
        };
        let shared = self.shared.clone();
        let actual = match shared.waiting(|| front.item.decoder.seek(to)) {
            Ok(actual) => actual,
            Err(e) => {
                let token = front.item.token;
                let decoded_all = front.decoded_all;
                self.emit(Event::Error {
                    token: Some(token),
                    message: e.to_string(),
                });
                // The track decoded to its end: what was parked follows it again.
                if decoded_all {
                    self.queue_next();
                }
                return;
            }
        };
        self.pending.clear();
        self.pending_at = 0;

        // The output drops what it holds the next time it runs, a cut
        // waiting in it included.
        sink.state.cut_at.store(u64::MAX, Ordering::Release);
        sink.state.cut_len.store(0, Ordering::Release);
        sink.state.flush.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_millis(50);
        while sink.state.flush.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        front.base_time = actual;
        front.decoded_all = false;
        // Counted from here meanwhile, for the position shown.
        front.start_frame = sink.played();
        // An output not running yet (starting, its device gone a moment)
        // drops it later: nothing is pushed until it has, or it would be
        // dropped too, and the track never be heard to its end.
        self.flushing = true;
        self.flushed();
    }

    /// Once the output dropped what it held: the track goes on from what
    /// it has played.
    fn flushed(&mut self) {
        let (Some(sink), Some(front)) = (&self.sink, self.segments.front_mut()) else {
            self.flushing = false;
            return;
        };
        if !self.flushing || sink.state.flush.load(Ordering::Acquire) {
            return;
        }
        self.flushing = false;
        let played = sink.played();
        self.written = played * sink.channels;
        front.start_frame = played;
    }

    /// The last segment finished decoding: the next track joins it in the ring if
    /// the output can play it as is, else waits for its own output.
    fn queue_next(&mut self) {
        let (Some(sink), Some(item)) = (&self.sink, self.next.take()) else {
            return;
        };
        let format = &item.decoder.info;
        if format.sample_rate == sink.sample_rate
            && u64::from(format.channels) == sink.channels
            && format.layout == sink.layout
        {
            self.segments.push_back(Segment {
                item,
                start_frame: self.written / sink.channels,
                base_time: Duration::ZERO,
                decoded_all: false,
            });
        } else {
            self.switch = Some(item);
        }
    }

    fn can_decode(&self) -> bool {
        !self.flushing
            && self.sink.as_ref().is_some_and(|s| s.producer.slots() > 0)
            && (self.pending_at < self.pending.len()
                || self.segments.back().is_some_and(|s| !s.decoded_all))
    }

    /// Follow the output: announce gapless track changes, and act once the ring
    /// has drained.
    fn advance(&mut self) {
        self.recover();
        self.flushed();
        if self.flushing {
            return;
        }
        let Some(sink) = &self.sink else { return };
        let played = sink.played();

        // A track that follows another starts when it is heard.
        while self.segments.len() > 1 && sink.heard() >= self.segments[1].start_frame {
            self.segments.pop_front();
            let front = &self.segments[0];
            self.emit(Event::Started {
                token: front.item.token,
                source: front.item.decoder.info.clone(),
                output: sink.info.clone(),
            });
        }

        let drained =
            played * sink.channels >= self.written && self.pending_at >= self.pending.len();
        let last_done = self.segments.len() == 1 && self.segments[0].decoded_all;
        if !(drained && last_done) {
            return;
        }
        if let Some(item) = self.switch.take() {
            self.start(item, Duration::ZERO);
        } else if self.next.is_none() {
            self.segments.clear();
            // Close the stream, so it stops holding the graph at its rate.
            self.sink = None;
            self.emit(Event::Finished);
        }
    }

    /// Decode into the ring until it is full or there is nothing left to decode.
    fn pump(&mut self) {
        loop {
            if self.shared.retired() || self.flushing {
                return;
            }
            let Some(sink) = &mut self.sink else { return };
            if self.pending_at < self.pending.len() {
                let pushed = push(&mut sink.producer, &self.pending[self.pending_at..]);
                self.pending_at += pushed;
                self.written += pushed as u64;
                if self.pending_at < self.pending.len() {
                    return;
                }
            }
            let Some(segment) = self.segments.back_mut() else {
                return;
            };
            if segment.decoded_all {
                return;
            }
            let shared = self.shared.clone();
            match shared.waiting(|| segment.item.decoder.next()) {
                Ok(Some(samples)) => {
                    self.pending.clear();
                    self.pending.extend_from_slice(samples);
                    self.pending_at = 0;
                }
                Ok(None) => {
                    segment.decoded_all = true;
                    self.queue_next();
                }
                Err(e) => {
                    // The track ends where it broke, and the next one follows.
                    segment.decoded_all = true;
                    let token = segment.item.token;
                    self.emit(Event::Error {
                        token: Some(token),
                        message: e.to_string(),
                    });
                    self.queue_next();
                }
            }
        }
    }

    /// An output lost (PipeWire restarted): the track playing waits where
    /// it was, its followers parked, and goes on in a new output as soon as
    /// one opens.
    fn recover(&mut self) {
        if self
            .sink
            .as_ref()
            .is_some_and(|s| s.state.lost.load(Ordering::Acquire))
        {
            let at = self.position();
            self.park_queued();
            let front = self.segments.pop_front();
            self.segments.clear();
            self.sink = None;
            self.pending.clear();
            self.pending_at = 0;
            let now = Instant::now();
            self.recovering = front.map(|segment| Recovering {
                item: segment.item,
                at,
                next_try: now + RETRY_EVERY,
                give_up: now + RETRY_FOR,
            });
        }
        let Some(waiting) = self.recovering.take_if(|r| Instant::now() >= r.next_try) else {
            return;
        };
        let Recovering {
            item, at, give_up, ..
        } = waiting;
        if let Err(failed) = self.try_start(item, at) {
            let (item, e) = *failed;
            if Instant::now() >= give_up {
                self.emit(Event::Error {
                    token: Some(item.token),
                    message: e.to_string(),
                });
            } else {
                self.recovering = Some(Recovering {
                    item,
                    at,
                    next_try: Instant::now() + RETRY_EVERY,
                    give_up,
                });
            }
        }
    }

    fn position(&self) -> Duration {
        let (Some(sink), Some(front)) = (&self.sink, self.segments.front()) else {
            return Duration::ZERO;
        };
        let frames = sink.heard().saturating_sub(front.start_frame);
        front.base_time + Duration::from_secs_f64(frames as f64 / f64::from(sink.sample_rate))
    }

    fn publish(&self) {
        let status = match self.segments.front() {
            Some(front) => Status {
                token: Some(front.item.token),
                position: self.position(),
                paused: self.paused,
                source: Some(front.item.decoder.info.clone()),
                output: self.sink.as_ref().map(Sink::live_info),
            },
            // Still the track playing, while its output comes back.
            None => match &self.recovering {
                Some(waiting) => Status {
                    token: Some(waiting.item.token),
                    position: waiting.at,
                    paused: self.paused,
                    source: Some(waiting.item.decoder.info.clone()),
                    output: None,
                },
                None => Status {
                    paused: self.paused,
                    ..Status::default()
                },
            },
        };
        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = status;
    }
}

/// Push as much of `samples` as fits; returns how many went in.
fn push(producer: &mut Producer<f32>, samples: &[f32]) -> usize {
    let n = producer.slots().min(samples.len());
    if n == 0 {
        return 0;
    }
    let Ok(mut chunk) = producer.write_chunk(n) else {
        return 0;
    };
    let (a, b) = chunk.as_mut_slices();
    let (first, rest) = samples[..n].split_at(a.len());
    a.copy_from_slice(first);
    b.copy_from_slice(rest);
    chunk.commit_all();
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_commands_are_safe_without_a_playing_segment() {
        let (events, _) = std::sync::mpsc::channel();
        let mut engine = Engine::new(
            Arc::new(Mutex::new(Status::default())),
            events,
            Arc::new(Shared::default()),
        );
        engine.handle(Command::Seek(Duration::from_secs(60)));
        engine.handle(Command::SkipToNext);
        assert!(engine.segments.is_empty());
        assert!(engine.next.is_none());
    }
}
