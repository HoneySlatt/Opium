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
    /// Where, in samples played, a track taken back out of what follows
    /// begins (`u64::MAX`: none), and how long it is: the output drops it
    /// on reaching it, the track playing untouched until then.
    pub cut_at: AtomicU64,
    pub cut_len: AtomicU64,
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
            cut_at: AtomicU64::new(u64::MAX),
            ..Self::default()
        }
    }
}

/// How long a change of gain takes: short enough to pass for at once, long
/// enough not to click.
const RAMP: f32 = 0.005;

/// A gain read from where it was left: within `0.0..=1.0`, and silent, to
/// fade in again, when it is not a number.
fn settled(gain: f32) -> f32 {
    if gain.is_finite() {
        gain.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// A volume as the output may use it, if it is one: within `0.0..=1.0`.
/// `None` for a value that is not finite.
fn audible(volume: f32) -> Option<f32> {
    volume.is_finite().then(|| volume.clamp(0.0, 1.0))
}

/// Move buffered frames into `out` at the volume. `gain` is the gain as
/// it stands, moved towards the volume, or towards silence to pause or
/// before dropping what is buffered (a seek), by [`RAMP`]. Once silent
/// that way, nothing more is read until it is to sound again. Silence too
/// when the decoder falls behind.
///
/// Whatever it is given, the gain it leaves is within `0.0..=1.0`: a gain
/// that is not a number starts again from silence, a volume outside the
/// range is held to it, and one that is not a number holds the gain as it
/// stands.
fn fill(
    consumer: &mut Consumer<f32>,
    state: &OutputState,
    gain: &mut f32,
    out: &mut [f32],
    channels: usize,
    sample_rate: u32,
) {
    let flush = state.flush.load(Ordering::Acquire);
    *gain = settled(*gain);
    let volume = audible(f32::from_bits(state.volume.load(Ordering::Relaxed))).unwrap_or(*gain);
    // Muted, it plays on unheard; paused, or to drop what is buffered, it
    // stops once silent.
    let stopping = flush || state.paused.load(Ordering::Relaxed);
    let target = if stopping { 0.0 } else { volume };
    let step = 1.0 / (RAMP * sample_rate as f32);
    let ramp = Ramp {
        target,
        step,
        stopping,
    };
    let mut n = 0;
    if !(stopping && *gain == 0.0) {
        // Up to where a track taken out of what follows begins, if it is in
        // this stretch: there the rest of it goes, and what follows it now
        // comes straight after, without a gap.
        let at = state.played.load(Ordering::Relaxed) * channels as u64;
        let cut = state.cut_at.load(Ordering::Acquire);
        if cut != u64::MAX && at >= cut {
            drop_cut(consumer, state);
        }
        let room = match state.cut_at.load(Ordering::Acquire) {
            u64::MAX => out.len(),
            cut => usize::try_from(cut - at)
                .unwrap_or(usize::MAX)
                .min(out.len()),
        };
        n = read_into(consumer, &mut out[..room], gain, &ramp, channels);
        if n == room && room < out.len() {
            drop_cut(consumer, state);
            n += read_into(consumer, &mut out[n..], gain, &ramp, channels);
        }
    }
    // The ring ran dry while sounding (the network fell behind): what it had
    // fades out rather than cut, and what comes once it fills again fades
    // in, as after a pause.
    if n < out.len() && !stopping && *gain > 0.0 {
        let frames = n / channels;
        let fade = frames.min((RAMP * sample_rate as f32) as usize).max(1);
        for (k, frame) in out[..n]
            .chunks_exact_mut(channels)
            .skip(frames.saturating_sub(fade))
            .enumerate()
        {
            let left = 1.0 - (k + 1) as f32 / fade as f32;
            frame.iter_mut().for_each(|s| *s *= left);
        }
        *gain = 0.0;
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

/// Where the gain goes, and how fast.
struct Ramp {
    target: f32,
    step: f32,
    stopping: bool,
}

/// Read what the ring holds into `out`, through the gain; how much.
fn read_into(
    consumer: &mut Consumer<f32>,
    out: &mut [f32],
    gain: &mut f32,
    ramp: &Ramp,
    channels: usize,
) -> usize {
    let mut n = consumer.slots().min(out.len());
    n -= n % channels;
    let Ok(chunk) = consumer.read_chunk(n) else {
        return 0;
    };
    let (a, b) = chunk.as_slices();
    let mut src = a.iter().chain(b);
    let mut read = 0;
    for frame in out[..n].chunks_exact_mut(channels) {
        // Faded out: the rest stays in the ring.
        if ramp.stopping && *gain == 0.0 {
            break;
        }
        if *gain != ramp.target {
            *gain = if *gain < ramp.target {
                (*gain + ramp.step).min(ramp.target)
            } else {
                (*gain - ramp.step).max(ramp.target)
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
    read
}

/// At the cut: what was taken out goes from the ring.
fn drop_cut(consumer: &mut Consumer<f32>, state: &OutputState) {
    let len = usize::try_from(state.cut_len.swap(0, Ordering::AcqRel)).unwrap_or(usize::MAX);
    if let Ok(chunk) = consumer.read_chunk(len.min(consumer.slots())) {
        chunk.commit_all();
    }
    state.cut_at.store(u64::MAX, Ordering::Release);
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
        "opium",
        pw::properties::properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Playback",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::APP_NAME => "Opium",
            *pw::keys::NODE_NAME => "opium",
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
        audible(f32::from_bits(state.volume.load(Ordering::Relaxed))).unwrap_or(0.0)
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
    fn the_gain_stays_between_silence_and_full_whatever_it_is_given() {
        let bad = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 2.0, -1.0, 1e30];
        for volume in bad.into_iter().chain([0.0, 0.5, 1.0]) {
            for start in bad.into_iter().chain([0.0, 0.5, 1.0]) {
                for sample_rate in [48_000, 1_000, 0] {
                    let (mut producer, mut consumer) = rtrb::RingBuffer::new(512);
                    for _ in 0..400 {
                        producer.push(0.5).unwrap();
                    }
                    let state = OutputState::new(volume);
                    let mut gain = start;
                    let mut out = [0.0; 400];
                    fill(&mut consumer, &state, &mut gain, &mut out, 1, sample_rate);
                    let context = format!("volume {volume}, gain {start}, {sample_rate} Hz");
                    assert!((0.0..=1.0).contains(&gain), "{context}: gain {gain}");
                    assert!(
                        out.iter().all(|s| s.is_finite() && s.abs() <= 0.5),
                        "{context}: a sample past the source"
                    );
                }
            }
        }
    }

    #[test]
    fn a_ring_run_dry_fades_out_then_back_in() {
        let (mut producer, mut consumer) = rtrb::RingBuffer::new(1024);
        for _ in 0..100 {
            producer.push(1.0).unwrap();
        }
        let state = OutputState::new(1.0);
        let mut gain = 1.0;
        let mut out = [0.5; 200];
        // At 1 kHz the fade is 5 frames long.
        fill(&mut consumer, &state, &mut gain, &mut out, 1, 1_000);
        assert_eq!(out[94], 1.0);
        assert!(out[95] < 1.0 && out[99] == 0.0 && out[150] == 0.0);
        assert_eq!(gain, 0.0);
        // Back: it fades in.
        producer.push(1.0).unwrap();
        producer.push(1.0).unwrap();
        let mut out = [0.0; 2];
        fill(&mut consumer, &state, &mut gain, &mut out, 1, 1_000);
        assert!(out[0] > 0.0 && out[0] < 1.0);
    }

    #[test]
    fn a_track_taken_out_of_what_follows_is_skipped_without_a_gap() {
        let (mut producer, mut consumer) = rtrb::RingBuffer::new(32);
        // Mono: the end of the one playing, the one taken out, the new next.
        for s in [1.0, 2.0, 3.0, 4.0, 10.0, 11.0, 12.0, 13.0, 100.0, 101.0] {
            producer.push(s).unwrap();
        }
        let state = OutputState::new(1.0);
        state.cut_len.store(4, Ordering::Relaxed);
        state.cut_at.store(4, Ordering::Relaxed);
        let mut gain = 1.0;
        let mut out = [0.0; 6];
        fill(&mut consumer, &state, &mut gain, &mut out, 1, 48_000);
        assert_eq!(out, [1.0, 2.0, 3.0, 4.0, 100.0, 101.0]);
        assert_eq!(state.cut_at.load(Ordering::Relaxed), u64::MAX);
        assert_eq!(state.played.load(Ordering::Relaxed), 6);
    }

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

    // W1 of the stage 1 audit: a volume that is not a number.
    #[test]
    fn a_volume_that_isnt_a_number_never_makes_it_louder() {
        for start in [0.0, f32::NAN] {
            let (mut producer, mut consumer) = rtrb::RingBuffer::new(4096);
            for _ in 0..4000 {
                producer.push(0.5).unwrap();
            }
            let state = OutputState::new(f32::NAN);
            let mut gain = start;
            let mut out = [0.0; 4000];
            fill(&mut consumer, &state, &mut gain, &mut out, 1, 48_000);
            assert!(
                out.iter().all(|s| s.is_finite() && s.abs() <= 0.5),
                "from {start}: gain {gain}, last sample {}",
                out[3999]
            );
        }
    }

    /// `settings.toml` may say `volume = nan`, which TOML reads, and a clamp keeps NaN. The
    /// gain used to walk down past zero by one each 5 ms, the samples coming out inverted and
    /// ever louder, past full scale within 10 ms.
    #[test]
    fn a_volume_that_is_not_a_number_never_amplifies() {
        let (mut producer, mut consumer) = rtrb::RingBuffer::new(4096);
        for _ in 0..4000 {
            producer.push(0.5).unwrap();
        }
        let state = OutputState::new(f32::NAN);
        let mut gain = 1.0;
        let mut out = [0.0; 4000];
        fill(&mut consumer, &state, &mut gain, &mut out, 1, 48_000);
        let loudest = out.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!(
            loudest <= 0.5,
            "loudest sample {loudest} from a source at 0.5; gain now {gain}"
        );
    }
}
