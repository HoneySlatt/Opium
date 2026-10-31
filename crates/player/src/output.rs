//! Audio output: a PipeWire stream fed from the ring of interleaved `f32` frames.
//!
//! Other programs keep playing alongside. The stream asks the graph to run at the
//! source's rate (`node.force-rate`), which puts the resampling on whatever else
//! is playing rather than on the music, and sends 32-bit floats, as PipeWire
//! mixes them: a 24-bit source goes through exactly. Volume is applied here; at
//! 100 % the samples pass untouched. Pausing, resuming, seeking and a change of
//! volume ramp over a few milliseconds rather than step, which would click.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use pipewire as pw;
use pw::spa;
use rtrb::Consumer;

use crate::{Error, Mixing, OutputInfo, Result};

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
    /// Frames handed over but not heard yet: what PipeWire and the device
    /// still hold.
    pub latency: AtomicU64,
    /// Our stream's node in the graph, once it has one.
    node_id: AtomicU32,
    /// The device and what else plays into it, refreshed twice a second.
    pub mixing: Mutex<Mixing>,
    /// The stream is gone: PipeWire restarted, or the stream failed. The
    /// engine opens a new one.
    pub lost: AtomicBool,
}

impl OutputState {
    pub fn new(volume: f32) -> Self {
        Self {
            volume: AtomicU32::new(volume.to_bits()),
            node_id: AtomicU32::new(spa::sys::SPA_ID_INVALID),
            ..Self::default()
        }
    }
}

/// How long a change of gain takes: short enough to pass for at once, long
/// enough not to click.
const RAMP: f32 = 0.005;

/// Move buffered frames into `out` at the volume. `gain` is the gain as
/// it stands, moved towards the volume, or towards silence to pause or
/// before dropping what is buffered (a seek), by [`RAMP`]. Once silent
/// that way, nothing more is read until it is to sound again. Silence too
/// when the decoder falls behind.
fn fill(
    consumer: &mut Consumer<f32>,
    state: &OutputState,
    gain: &mut f32,
    out: &mut [f32],
    channels: usize,
    sample_rate: u32,
) {
    let flush = state.flush.load(Ordering::Acquire);
    let volume = f32::from_bits(state.volume.load(Ordering::Relaxed));
    // Muted, it plays on unheard; paused, or to drop what is buffered, it
    // stops once silent.
    let stopping = flush || state.paused.load(Ordering::Relaxed);
    let target = if stopping { 0.0 } else { volume };
    let step = 1.0 / (RAMP * sample_rate as f32);
    let mut n = 0;
    if !(stopping && *gain == 0.0) {
        n = consumer.slots().min(out.len());
        n -= n % channels;
        if let Ok(chunk) = consumer.read_chunk(n) {
            let (a, b) = chunk.as_slices();
            let mut src = a.iter().chain(b);
            let mut read = 0;
            for frame in out[..n].chunks_exact_mut(channels) {
                // Faded out: the rest stays in the ring.
                if stopping && *gain == 0.0 {
                    break;
                }
                if *gain != target {
                    *gain = if *gain < target {
                        (*gain + step).min(target)
                    } else {
                        (*gain - step).max(target)
                    };
                }
                for dst in frame {
                    let s = *src.next().unwrap_or(&0.0);
                    // At full volume, and steady, untouched.
                    *dst = if *gain == 1.0 { s } else { s * *gain };
                }
                read += channels;
            }
            chunk.commit(read);
            n = read;
        }
    }
    out[n..].fill(0.0);
    state
        .played
        .fetch_add((n / channels) as u64, Ordering::Relaxed);
    // Silent now: what is buffered can go without a click.
    if flush && *gain == 0.0 {
        if let Ok(chunk) = consumer.read_chunk(consumer.slots()) {
            chunk.commit_all();
        }
        state.flush.store(false, Ordering::Release);
    }
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
        layout: Option<u64>,
        consumer: Consumer<f32>,
        state: Arc<OutputState>,
    ) -> Result<(Self, OutputInfo)> {
        let (quit, quit_rx) = pw::channel::channel::<()>();
        let (opened_tx, opened_rx) = std::sync::mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("pipewire-out".into())
            .spawn(move || {
                let format = (sample_rate, channels, layout);
                if let Err(e) = run(format, consumer, state, quit_rx, &opened_tx) {
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
            sample_format: "F32_LE",
            mixing: Mixing::default(),
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
    consumer: Consumer<f32>,
    state: Arc<OutputState>,
    channels: usize,
    sample_rate: u32,
    scratch: Vec<f32>,
    /// The gain applied as of the last frame.
    gain: f32,
}

/// Each channel's speaker: as the file lays them out, else mono, stereo, or
/// channels with no place.
fn positions(channels: u16, layout: Option<u64>) -> [u32; spa::param::audio::MAX_CHANNELS] {
    use spa::sys::*;
    // Symphonia's `Position` bits, in their order, as PipeWire names them.
    const SPEAKERS: [u32; 26] = [
        SPA_AUDIO_CHANNEL_FL,
        SPA_AUDIO_CHANNEL_FR,
        SPA_AUDIO_CHANNEL_FC,
        SPA_AUDIO_CHANNEL_LFE,
        SPA_AUDIO_CHANNEL_RL,
        SPA_AUDIO_CHANNEL_RR,
        SPA_AUDIO_CHANNEL_FLC,
        SPA_AUDIO_CHANNEL_FRC,
        SPA_AUDIO_CHANNEL_RC,
        SPA_AUDIO_CHANNEL_SL,
        SPA_AUDIO_CHANNEL_SR,
        SPA_AUDIO_CHANNEL_TC,
        SPA_AUDIO_CHANNEL_TFL,
        SPA_AUDIO_CHANNEL_TFC,
        SPA_AUDIO_CHANNEL_TFR,
        SPA_AUDIO_CHANNEL_TRL,
        SPA_AUDIO_CHANNEL_TRC,
        SPA_AUDIO_CHANNEL_TRR,
        SPA_AUDIO_CHANNEL_LFE2,
        SPA_AUDIO_CHANNEL_TSL,
        SPA_AUDIO_CHANNEL_TSR,
        SPA_AUDIO_CHANNEL_BC,
        SPA_AUDIO_CHANNEL_BLC,
        SPA_AUDIO_CHANNEL_BRC,
        SPA_AUDIO_CHANNEL_FLW,
        SPA_AUDIO_CHANNEL_FRW,
    ];
    let mut position = [0; spa::param::audio::MAX_CHANNELS];
    let n = usize::from(channels).min(position.len());
    let laid_out: Vec<u32> = layout
        .map(|bits| {
            (0..SPEAKERS.len())
                .filter(|&bit| bits & (1 << bit) != 0)
                .map(|bit| SPEAKERS[bit])
                .collect()
        })
        .unwrap_or_default();
    if laid_out.len() == n && n > 2 {
        position[..n].copy_from_slice(&laid_out);
        return position;
    }
    match n {
        1 => position[0] = SPA_AUDIO_CHANNEL_MONO,
        2 => {
            position[0] = SPA_AUDIO_CHANNEL_FL;
            position[1] = SPA_AUDIO_CHANNEL_FR;
        }
        _ => {
            for (c, p) in position.iter_mut().take(n).enumerate() {
                *p = SPA_AUDIO_CHANNEL_AUX0 + c as u32;
            }
        }
    }
    position
}

/// Runs on the output thread until `quit` fires.
fn run(
    (sample_rate, channels, layout): (u32, u16, Option<u64>),
    consumer: Consumer<f32>,
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
    // The connection to PipeWire broke (it restarted): this loop is done.
    let _core_listener = core
        .add_listener_local()
        .error({
            let mainloop = mainloop.clone();
            let state = state.clone();
            move |id, _seq, _res, _message| {
                if id == pw::core::PW_ID_CORE {
                    state.lost.store(true, Ordering::Release);
                    mainloop.quit();
                }
            }
        })
        .register();

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

    // Follow the rest of the graph, and summarise it for the engine.
    let registry = core.get_registry_rc()?;
    let (graph, _graph_listener) = crate::graph::watch(&registry);
    let summary_state = state.clone();
    let timer = mainloop.loop_().add_timer(move |_| {
        let ours = summary_state.node_id.load(Ordering::Relaxed);
        if ours != spa::sys::SPA_ID_INVALID {
            let mixing = graph.borrow().summary(ours);
            *summary_state
                .mixing
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = mixing;
        }
    });
    timer.update_timer(
        Some(Duration::from_millis(200)),
        Some(Duration::from_millis(500)),
    );

    // A track starts at the volume as it is, from its first sample.
    let gain = if state.paused.load(Ordering::Relaxed) {
        0.0
    } else {
        f32::from_bits(state.volume.load(Ordering::Relaxed))
    };
    let feed = Feed {
        consumer,
        state,
        channels: usize::from(channels),
        sample_rate,
        // Room for a large quantum, so the realtime callback rarely allocates.
        scratch: Vec::with_capacity(8192 * usize::from(channels)),
        gain,
    };
    let _listener = stream
        .add_local_listener_with_user_data(feed)
        .state_changed(|_, feed, _, new| {
            if matches!(
                new,
                pw::stream::StreamState::Error(_) | pw::stream::StreamState::Unconnected
            ) {
                feed.state.lost.store(true, Ordering::Release);
            }
        })
        .process(|stream, feed| {
            feed.state
                .node_id
                .store(stream.node_id(), Ordering::Relaxed);
            if let Ok(time) = stream.time() {
                let rate = time.rate();
                if rate.denom > 0 {
                    feed.state.graph_rate.store(rate.denom, Ordering::Relaxed);
                    // The delay is in graph ticks; the converter's in frames.
                    let delay = u64::try_from(time.delay()).unwrap_or(0)
                        * u64::from(rate.num)
                        * u64::from(feed.sample_rate)
                        / u64::from(rate.denom);
                    feed.state
                        .latency
                        .store(delay + time.buffered(), Ordering::Relaxed);
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
            feed.scratch.resize(frames * feed.channels, 0.0);
            fill(
                &mut feed.consumer,
                &feed.state,
                &mut feed.gain,
                &mut feed.scratch,
                feed.channels,
                feed.sample_rate,
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
    info.set_format(spa::param::audio::AudioFormat::F32LE);
    info.set_rate(sample_rate);
    info.set_channels(u32::from(channels));
    info.set_position(positions(channels, layout));
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

#[cfg(test)]
mod tests {
    use super::*;
    use spa::sys::*;

    #[test]
    fn a_five_one_file_keeps_its_speakers() {
        let p = positions(6, Some(0x3F));
        assert_eq!(
            p[..6],
            [
                SPA_AUDIO_CHANNEL_FL,
                SPA_AUDIO_CHANNEL_FR,
                SPA_AUDIO_CHANNEL_FC,
                SPA_AUDIO_CHANNEL_LFE,
                SPA_AUDIO_CHANNEL_RL,
                SPA_AUDIO_CHANNEL_RR
            ]
        );
        // A layout that doesn't fit the count leaves channels unplaced.
        assert_eq!(positions(3, Some(0x3F))[2], SPA_AUDIO_CHANNEL_AUX0 + 2);
        assert_eq!(positions(1, None)[0], SPA_AUDIO_CHANNEL_MONO);
    }
}
