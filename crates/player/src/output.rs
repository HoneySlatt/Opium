//! Audio output: a PipeWire stream fed from the ring of interleaved `i32` frames.
//!
//! Other programs keep playing alongside. The stream asks the graph to run at the
//! source's rate (`node.force-rate`), which puts the resampling on whatever else
//! is playing rather than on the music, and sends 32-bit integers, which
//! PipeWire's float mixer carries exactly up to 24 bits. Volume is applied here,
//! and only below 100 %.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::thread::JoinHandle;

use pipewire as pw;
use pw::spa;
use rtrb::Consumer;

use crate::{Error, OutputInfo, Result};

/// What the engine and the output thread share.
#[derive(Default)]
pub(crate) struct OutputState {
    /// Frames actually handed to the device since it was opened.
    pub played: AtomicU64,
    /// Set by the engine to drop everything buffered; cleared by the output.
    pub flush: AtomicBool,
    pub paused: AtomicBool,
    /// `f32` bits.
    pub volume: AtomicU32,
    /// The rate the PipeWire graph actually runs at, once known.
    pub graph_rate: AtomicU32,
}

impl OutputState {
    pub fn new(volume: f32) -> Self {
        Self {
            volume: AtomicU32::new(volume.to_bits()),
            ..Self::default()
        }
    }
}

/// Move buffered frames into `out`, converting each sample. Silence while
/// paused or when the decoder falls behind.
fn fill<T: Copy>(
    consumer: &mut Consumer<i32>,
    state: &OutputState,
    out: &mut [T],
    channels: usize,
    silence: T,
    convert: impl Fn(i32) -> T,
) {
    if state.flush.load(Ordering::Acquire) {
        if let Ok(chunk) = consumer.read_chunk(consumer.slots()) {
            chunk.commit_all();
        }
        state.flush.store(false, Ordering::Release);
    }
    if state.paused.load(Ordering::Relaxed) {
        out.fill(silence);
        return;
    }
    let mut n = consumer.slots().min(out.len());
    n -= n % channels;
    if let Ok(chunk) = consumer.read_chunk(n) {
        let (a, b) = chunk.as_slices();
        for (dst, &src) in out.iter_mut().zip(a.iter().chain(b)) {
            *dst = convert(src);
        }
        chunk.commit_all();
    }
    out[n..].fill(silence);
    state
        .played
        .fetch_add((n / channels) as u64, Ordering::Relaxed);
}

pub(crate) struct Output {
    quit: pw::channel::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Output {
    fn drop(&mut self) {
        let _ = self.quit.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Output {
    pub fn open(
        sample_rate: u32,
        channels: u16,
        consumer: Consumer<i32>,
        state: Arc<OutputState>,
    ) -> Result<(Self, OutputInfo)> {
        let (quit, quit_rx) = pw::channel::channel::<()>();
        let (opened_tx, opened_rx) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("pipewire-out".into())
            .spawn(move || {
                if let Err(e) = run(sample_rate, channels, consumer, state, quit_rx, &opened_tx) {
                    let _ = opened_tx.send(Err(Error::Output(format!("PipeWire: {e}"))));
                }
            })
            .map_err(|e| Error::Output(e.to_string()))?;
        opened_rx
            .recv()
            .map_err(|_| Error::Output("the PipeWire thread died".into()))??;
        let info = OutputInfo {
            device: "PipeWire".into(),
            sample_rate,
            channels,
            sample_format: "S32_LE",
        };
        Ok((
            Self {
                quit,
                thread: Some(thread),
            },
            info,
        ))
    }
}

/// What the realtime process callback works with.
struct Feed {
    consumer: Consumer<i32>,
    state: Arc<OutputState>,
    channels: usize,
    scratch: Vec<i32>,
}

/// Runs on the output thread until `quit` fires.
fn run(
    sample_rate: u32,
    channels: u16,
    consumer: Consumer<i32>,
    state: Arc<OutputState>,
    quit: pw::channel::Receiver<()>,
    opened: &std::sync::mpsc::SyncSender<Result<()>>,
) -> std::result::Result<(), pw::Error> {
    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;
    let _quit = quit.attach(mainloop.loop_(), {
        let mainloop = mainloop.clone();
        move |()| mainloop.quit()
    });

    let rate = format!("1/{sample_rate}");
    let stream = pw::stream::StreamBox::new(
        &core,
        "music",
        pw::properties::properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Playback",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::APP_NAME => "Music",
            *pw::keys::NODE_NAME => "music",
            *pw::keys::AUDIO_CHANNELS => channels.to_string(),
            *pw::keys::NODE_RATE => rate.as_str(),
            // 0: force the rate given in node.rate.
            *pw::keys::NODE_FORCE_RATE => "0",
            // If the graph still can't follow us, resample as well as possible.
            "resample.quality" => "14",
        },
    )?;

    let feed = Feed {
        consumer,
        state,
        channels: usize::from(channels),
        scratch: Vec::new(),
    };
    let _listener = stream
        .add_local_listener_with_user_data(feed)
        .process(|stream, feed| {
            if let Ok(time) = stream.time() {
                let denom = time.rate().denom;
                if denom > 0 {
                    feed.state.graph_rate.store(denom, Ordering::Relaxed);
                }
            }
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let requested = buffer.requested() as usize;
            let stride = 4 * feed.channels;
            let data = &mut buffer.datas_mut()[0];
            let Some(bytes) = data.data() else { return };
            let mut frames = bytes.len() / stride;
            if requested > 0 {
                frames = frames.min(requested);
            }
            feed.scratch.resize(frames * feed.channels, 0);
            let volume = f32::from_bits(feed.state.volume.load(Ordering::Relaxed));
            // At full volume the samples pass through untouched.
            let gain = (volume < 1.0).then_some(f64::from(volume));
            fill(
                &mut feed.consumer,
                &feed.state,
                &mut feed.scratch,
                feed.channels,
                0,
                |s| gain.map_or(s, |g| (f64::from(s) * g) as i32),
            );
            for (dst, s) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(&feed.scratch) {
                dst.copy_from_slice(&s.to_le_bytes());
            }
            let chunk = data.chunk_mut();
            *chunk.offset_mut() = 0;
            *chunk.stride_mut() = stride as i32;
            *chunk.size_mut() = (frames * stride) as u32;
        })
        .register()?;

    let mut info = spa::param::audio::AudioInfoRaw::new();
    info.set_format(spa::param::audio::AudioFormat::S32LE);
    info.set_rate(sample_rate);
    info.set_channels(u32::from(channels));
    let mut position = [0; spa::param::audio::MAX_CHANNELS];
    match channels {
        1 => position[0] = spa::sys::SPA_AUDIO_CHANNEL_MONO,
        2 => {
            position[0] = spa::sys::SPA_AUDIO_CHANNEL_FL;
            position[1] = spa::sys::SPA_AUDIO_CHANNEL_FR;
        }
        n => {
            for (c, p) in position.iter_mut().take(usize::from(n)).enumerate() {
                *p = spa::sys::SPA_AUDIO_CHANNEL_AUX0 + c as u32;
            }
        }
    }
    info.set_position(position);
    let format: Vec<u8> = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(spa::pod::Object {
            type_: spa::sys::SPA_TYPE_OBJECT_Format,
            id: spa::sys::SPA_PARAM_EnumFormat,
            properties: info.into(),
        }),
    )
    .expect("serialize an audio format")
    .0
    .into_inner();
    let mut params = [spa::pod::Pod::from_bytes(&format).expect("a valid pod")];
    stream.connect(
        spa::utils::Direction::Output,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;

    let _ = opened.send(Ok(()));
    mainloop.run();
    Ok(())
}
