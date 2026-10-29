//! The engine thread: decodes ahead into the ring, follows what the output has
//! actually played, and turns commands into changes to both.

use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rtrb::{Producer, RingBuffer};

use crate::decoder::Decoder;
use crate::output::{Output, OutputState};
use crate::{Event, OutputInfo, Source, Status, Token};

pub(crate) enum Command {
    Play(Token, Source),
    SetNext(Token, Source),
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
    producer: Producer<i32>,
    channels: u64,
    sample_rate: u32,
}

impl Sink {
    fn played(&self) -> u64 {
        self.state.played.load(Ordering::Relaxed)
    }

    /// The output as it is now, with the rate the graph actually runs at.
    fn live_info(&self) -> OutputInfo {
        let mut info = self.info.clone();
        let graph_rate = self.state.graph_rate.load(Ordering::Relaxed);
        if graph_rate > 0 {
            info.sample_rate = graph_rate;
        }
        info
    }
}

pub(crate) struct Engine {
    volume: f32,
    paused: bool,
    sink: Option<Sink>,
    segments: VecDeque<Segment>,
    /// The track to follow the last segment, not decoding yet.
    next: Option<Item>,
    /// A next track whose format needs another output; waits for the ring to drain.
    switch: Option<Item>,
    /// Samples pushed into the ring since the output was opened.
    written: u64,
    /// Decoded samples that didn't fit in the ring yet.
    pending: Vec<i32>,
    pending_at: usize,
    status: Arc<Mutex<Status>>,
    events: Sender<Event>,
}

/// How much audio the ring holds between the decoder and the device.
const RING_SECONDS: usize = 2;
/// How long the engine sleeps when there is nothing to decode.
const IDLE: Duration = Duration::from_millis(10);

impl Engine {
    pub fn new(status: Arc<Mutex<Status>>, events: Sender<Event>) -> Self {
        Self {
            volume: 1.0,
            paused: false,
            sink: None,
            segments: VecDeque::new(),
            next: None,
            switch: None,
            written: 0,
            pending: Vec::new(),
            pending_at: 0,
            status,
            events,
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
                commands.recv_timeout(IDLE)
            };
            match command {
                Ok(Command::Quit) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => {}
            }
            self.advance();
            self.pump();
            self.publish();
        }
    }

    fn emit(&self, event: Event) {
        let _ = self.events.send(event);
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Play(token, source) => {
                self.next = None;
                self.switch = None;
                if let Some(item) = self.open(token, source) {
                    self.start(item, Duration::ZERO);
                }
            }
            Command::SetNext(token, source) => {
                self.switch = None;
                self.next = self.open(token, source);
                if self.segments.back().is_some_and(|s| s.decoded_all) {
                    self.queue_next();
                }
            }
            Command::Pause => self.set_paused(true),
            Command::Resume => self.set_paused(false),
            Command::Seek(to) => self.seek(to),
            Command::Stop => {
                self.segments.clear();
                self.next = None;
                self.switch = None;
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
        match Decoder::open(source) {
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
    fn start(&mut self, mut item: Item, at: Duration) {
        self.sink = None;
        self.segments.clear();
        self.pending.clear();
        self.pending_at = 0;
        self.written = 0;

        let format = item.decoder.info.clone();
        let base_time = if at.is_zero() {
            at
        } else {
            match item.decoder.seek(at) {
                Ok(actual) => actual,
                Err(_) => Duration::ZERO,
            }
        };
        let channels = format.channels;
        let capacity = format.sample_rate as usize * usize::from(channels) * RING_SECONDS;
        let (producer, consumer) = RingBuffer::new(capacity);
        let state = Arc::new(OutputState::new(self.volume));
        state.paused.store(self.paused, Ordering::Relaxed);

        match Output::open(format.sample_rate, channels, consumer, state.clone()) {
            Ok((output, info)) => {
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
                    sample_rate: format.sample_rate,
                });
                self.segments.push_back(Segment {
                    item,
                    start_frame: 0,
                    base_time,
                    decoded_all: false,
                });
            }
            Err(e) => self.emit(Event::Error {
                token: Some(item.token),
                message: e.to_string(),
            }),
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
        let queued = self.segments.drain(1..).map(|s| s.item);
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
        let actual = match front.item.decoder.seek(to) {
            Ok(actual) => actual,
            Err(e) => {
                let token = front.item.token;
                self.emit(Event::Error {
                    token: Some(token),
                    message: e.to_string(),
                });
                return;
            }
        };
        self.pending.clear();
        self.pending_at = 0;

        // The output drops what it holds the next time it runs.
        sink.state.flush.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_millis(500);
        while sink.state.flush.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        let played = sink.played();
        self.written = played * sink.channels;
        front.start_frame = played;
        front.base_time = actual;
        front.decoded_all = false;
    }

    /// The last segment finished decoding: the next track joins it in the ring if
    /// the output can play it as is, else waits for its own output.
    fn queue_next(&mut self) {
        let (Some(sink), Some(item)) = (&self.sink, self.next.take()) else {
            return;
        };
        let format = &item.decoder.info;
        if format.sample_rate == sink.sample_rate && u64::from(format.channels) == sink.channels {
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
        self.sink.as_ref().is_some_and(|s| s.producer.slots() > 0)
            && (self.pending_at < self.pending.len()
                || self.segments.back().is_some_and(|s| !s.decoded_all))
    }

    /// Follow the output: announce gapless track changes, and act once the ring
    /// has drained.
    fn advance(&mut self) {
        let Some(sink) = &self.sink else { return };
        let played = sink.played();

        while self.segments.len() > 1 && played >= self.segments[1].start_frame {
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
            match segment.item.decoder.next() {
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
                    segment.decoded_all = true;
                    let token = segment.item.token;
                    self.emit(Event::Error {
                        token: Some(token),
                        message: e.to_string(),
                    });
                }
            }
        }
    }

    fn position(&self) -> Duration {
        let (Some(sink), Some(front)) = (&self.sink, self.segments.front()) else {
            return Duration::ZERO;
        };
        let frames = sink.played().saturating_sub(front.start_frame);
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
            None => Status {
                paused: self.paused,
                ..Status::default()
            },
        };
        *self.status.lock().unwrap_or_else(|e| e.into_inner()) = status;
    }
}

/// Push as much of `samples` as fits; returns how many went in.
fn push(producer: &mut Producer<i32>, samples: &[i32]) -> usize {
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
