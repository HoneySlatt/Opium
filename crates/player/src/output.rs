//! Audio outputs, both fed from the same ring of interleaved `i32` frames.
//!
//! - Shared: the default device through cpal, which on Linux means PipeWire. The
//!   server may mix and resample, and volume is applied here in software.
//! - Exclusive: an ALSA `hw:` device opened directly, at the source's rate and
//!   channel count, with no volume. When the device takes 32-bit samples nothing
//!   between the decoder and the DAC changes them.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::Consumer;

use crate::{Error, OutputInfo, OutputMode, Result};

/// What the engine and the output thread share.
#[derive(Default)]
pub(crate) struct OutputState {
    /// Frames actually handed to the device since it was opened.
    pub played: AtomicU64,
    /// Set by the engine to drop everything buffered; cleared by the output.
    pub flush: AtomicBool,
    pub paused: AtomicBool,
    /// Shared mode only. `f32` bits.
    pub volume: AtomicU32,
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

pub(crate) enum Output {
    /// Kept alive for as long as it should play.
    Shared { _stream: cpal::Stream },
    Exclusive {
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
        /// Released after the thread has closed the device.
        _reservation: Option<crate::reserve::Reservation>,
    },
}

impl Drop for Output {
    fn drop(&mut self) {
        if let Output::Exclusive { stop, thread, .. } = self {
            stop.store(true, Ordering::Relaxed);
            if let Some(thread) = thread.take() {
                let _ = thread.join();
            }
        }
    }
}

impl Output {
    pub fn open(
        mode: &OutputMode,
        sample_rate: u32,
        channels: u16,
        source_bits: Option<u32>,
        consumer: Consumer<i32>,
        state: Arc<OutputState>,
    ) -> Result<(Self, OutputInfo)> {
        match mode {
            OutputMode::Shared => open_shared(sample_rate, channels, consumer, state),
            OutputMode::Exclusive { device } => {
                open_exclusive(device, sample_rate, channels, source_bits, consumer, state)
            }
        }
    }
}

fn open_shared(
    sample_rate: u32,
    channels: u16,
    mut consumer: Consumer<i32>,
    state: Arc<OutputState>,
) -> Result<(Output, OutputInfo)> {
    let output = |e: &dyn std::fmt::Display| Error::Output(e.to_string());
    let device = cpal::default_host()
        .default_output_device()
        .ok_or_else(|| Error::Output("no output device".into()))?;
    let configs: Vec<_> = device
        .supported_output_configs()
        .map_err(|e| output(&e))?
        .filter(|c| c.sample_format() == cpal::SampleFormat::F32 && c.channels() == channels)
        .collect();
    let config = configs
        .iter()
        .find_map(|c| c.try_with_sample_rate(sample_rate))
        .ok_or_else(|| {
            Error::Output(format!(
                "the output device can't play {channels} channels at {sample_rate} Hz"
            ))
        })?;

    let ch = usize::from(channels);
    let stream = device
        .build_output_stream(
            config.config(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let volume = f32::from_bits(state.volume.load(Ordering::Relaxed));
                let scale = volume / 2_147_483_648.0;
                fill(&mut consumer, &state, data, ch, 0.0, |s| s as f32 * scale);
            },
            |err| eprintln!("audio output error: {err}"),
            None,
        )
        .map_err(|e| output(&e))?;
    stream.play().map_err(|e| output(&e))?;
    let info = OutputInfo {
        device: device
            .description()
            .map(|d| d.name().to_owned())
            .unwrap_or_else(|_| "default".into()),
        sample_rate,
        channels,
        sample_format: "F32",
        bit_perfect: false,
    };
    Ok((Output::Shared { _stream: stream }, info))
}

/// Sample containers to try, best first, with how many bits of a sample survive.
#[derive(Clone, Copy)]
enum Container {
    S32,
    S16,
}

fn open_exclusive(
    device: &str,
    sample_rate: u32,
    channels: u16,
    source_bits: Option<u32>,
    consumer: Consumer<i32>,
    state: Arc<OutputState>,
) -> Result<(Output, OutputInfo)> {
    // Take the card from the sound server first, if it holds it.
    let reservation = crate::reserve::card_index(device)
        .map(crate::reserve::reserve)
        .transpose()?;
    let stop = Arc::new(AtomicBool::new(false));
    let (opened_tx, opened_rx) = std::sync::mpsc::sync_channel(1);
    let thread = {
        let device = device.to_owned();
        let stop = stop.clone();
        std::thread::Builder::new()
            .name("alsa-out".into())
            .spawn(move || {
                let pcm = match open_pcm(&device, sample_rate, channels) {
                    Ok(opened) => opened,
                    Err(e) => {
                        let _ = opened_tx.send(Err(e));
                        return;
                    }
                };
                let _ = opened_tx.send(Ok(pcm.1));
                write_loop(&pcm.0, pcm.1, channels, consumer, &state, &stop);
            })
            .map_err(|e| Error::Output(e.to_string()))?
    };
    let container = opened_rx
        .recv()
        .map_err(|_| Error::Output("the ALSA thread died".into()))??;

    // 32-bit containers carry any source untouched; 16-bit only 16-bit sources.
    let bit_perfect = match container {
        Container::S32 => true,
        Container::S16 => source_bits.is_some_and(|b| b <= 16),
    };
    let info = OutputInfo {
        device: device.to_owned(),
        sample_rate,
        channels,
        sample_format: match container {
            Container::S32 => "S32_LE",
            Container::S16 => "S16_LE",
        },
        bit_perfect,
    };
    Ok((
        Output::Exclusive {
            stop,
            thread: Some(thread),
            _reservation: reservation,
        },
        info,
    ))
}

fn alsa_error(device: &str, e: alsa::Error) -> Error {
    if e.errno() == EBUSY {
        Error::Output(format!(
            "{device} is busy: another program (usually PipeWire) is using it"
        ))
    } else {
        Error::Output(format!("{device}: {e}"))
    }
}

const EBUSY: i32 = 16;

/// Open `device` at exactly this rate and channel count; the hardware either
/// takes the stream as it is or the open fails.
fn open_pcm(device: &str, rate: u32, channels: u16) -> Result<(alsa::PCM, Container)> {
    use alsa::pcm::{Access, Format, HwParams};
    use alsa::{Direction, ValueOr};

    let err = |e| alsa_error(device, e);
    // A sound server that just released the card may still be closing it.
    let mut attempts = 0;
    let pcm = loop {
        match alsa::PCM::new(device, Direction::Playback, false) {
            Ok(pcm) => break pcm,
            Err(e) if e.errno() == EBUSY && attempts < 25 => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            Err(e) => return Err(err(e)),
        }
    };
    let container = {
        let hw = HwParams::any(&pcm).map_err(err)?;
        hw.set_access(Access::RWInterleaved).map_err(err)?;
        hw.set_rate_resample(false).map_err(err)?;
        hw.set_channels(u32::from(channels)).map_err(|_| {
            Error::Output(format!("{device} can't play {channels} channels"))
        })?;
        hw.set_rate(rate, ValueOr::Nearest)
            .ok()
            .filter(|()| hw.get_rate().ok() == Some(rate))
            .ok_or_else(|| Error::Output(format!("{device} can't run at {rate} Hz")))?;
        let container = if hw.set_format(Format::S32LE).is_ok() {
            Container::S32
        } else if hw.set_format(Format::S16LE).is_ok() {
            Container::S16
        } else {
            return Err(Error::Output(format!(
                "{device} takes neither 32- nor 16-bit samples"
            )));
        };
        // ~100 ms buffered in the device: pauses and seeks take effect that fast.
        hw.set_buffer_time_near(100_000, ValueOr::Nearest).map_err(err)?;
        hw.set_period_time_near(20_000, ValueOr::Nearest).map_err(err)?;
        pcm.hw_params(&hw).map_err(err)?;
        container
    };
    Ok((pcm, container))
}

fn write_loop(
    pcm: &alsa::PCM,
    container: Container,
    channels: u16,
    mut consumer: Consumer<i32>,
    state: &OutputState,
    stop: &AtomicBool,
) {
    let ch = usize::from(channels);
    let period = pcm
        .get_params()
        .map(|(_, period)| period as usize)
        .unwrap_or(1024);
    match container {
        Container::S32 => {
            let Ok(io) = pcm.io_i32() else { return };
            let mut buf = vec![0i32; period * ch];
            while !stop.load(Ordering::Relaxed) {
                fill(&mut consumer, state, &mut buf, ch, 0, |s| s);
                if let Err(e) = io.writei(&buf) {
                    let _ = pcm.try_recover(e, true);
                }
            }
        }
        Container::S16 => {
            let Ok(io) = pcm.io_i16() else { return };
            let mut buf = vec![0i16; period * ch];
            while !stop.load(Ordering::Relaxed) {
                fill(&mut consumer, state, &mut buf, ch, 0, |s| (s >> 16) as i16);
                if let Err(e) = io.writei(&buf) {
                    let _ = pcm.try_recover(e, true);
                }
            }
        }
    }
    let _ = pcm.drop();
}
