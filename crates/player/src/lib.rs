//! Decoding and audio output.
//!
//! This first version plays one source to the end through the default output
//! device, at the source's own sample rate. It is the shared (PipeWire) path;
//! the bit-perfect `hw:` path and gapless queue playback come later.

use std::io::{Read, Seek};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unreadable audio: {0}")]
    Decode(String),
    #[error("audio output: {0}")]
    Output(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Any seekable byte source of known length, for symphonia.
struct Source<R> {
    inner: R,
    len: u64,
}

impl<R: Read> Read for Source<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R: Seek> Seek for Source<R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl<R: Read + Seek + Send + Sync> MediaSource for Source<R> {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}

/// What the output device was actually opened with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

/// Play `source` (`len` bytes, `extension` as a format hint) to the end. Blocks.
/// `on_start` gets the source and output formats once the device is open.
pub fn play<R>(
    source: R,
    len: u64,
    extension: Option<&str>,
    on_start: impl FnOnce(OutputFormat, OutputFormat),
) -> Result<()>
where
    R: Read + Seek + Send + Sync + 'static,
{
    let decode = |e: symphonia::core::errors::Error| Error::Decode(e.to_string());
    let output = |e: &dyn std::fmt::Display| Error::Output(e.to_string());

    let mut hint = Hint::new();
    if let Some(ext) = extension {
        hint.with_extension(ext);
    }
    let stream = MediaSourceStream::new(Box::new(Source { inner: source, len }), Default::default());
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(decode)?;
    let track = format
        .first_track(TrackType::Audio)
        .ok_or_else(|| Error::Decode("no audio track".into()))?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .cloned()
        .ok_or_else(|| Error::Decode("no audio parameters".into()))?;
    let source_format = OutputFormat {
        sample_rate: params
            .sample_rate
            .ok_or_else(|| Error::Decode("unknown sample rate".into()))?,
        channels: params.channels.as_ref().map_or(2, |c| c.count() as u16),
    };
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(decode)?;

    let device = cpal::default_host()
        .default_output_device()
        .ok_or_else(|| Error::Output("no output device".into()))?;
    // The source's own layout, else the device's usual one (stereo is then
    // spread or folded by the channel mapping below), never whatever comes first.
    let default_channels = device
        .default_output_config()
        .map_err(|e| output(&e))?
        .channels();
    let configs: Vec<_> = device
        .supported_output_configs()
        .map_err(|e| output(&e))?
        .filter(|c| c.sample_format() == cpal::SampleFormat::F32)
        .collect();
    let config = [source_format.channels, default_channels]
        .into_iter()
        .find_map(|channels| {
            configs
                .iter()
                .filter(|c| c.channels() == channels)
                .find_map(|c| c.try_with_sample_rate(source_format.sample_rate))
        })
        .ok_or_else(|| {
            Error::Output(format!(
                "the output device can't run at {} Hz",
                source_format.sample_rate
            ))
        })?;
    let out_format = OutputFormat {
        sample_rate: config.sample_rate(),
        channels: config.channels(),
    };

    // Two seconds of audio between the decoder and the device.
    let capacity = (out_format.sample_rate as usize) * usize::from(out_format.channels) * 2;
    let (mut producer, mut consumer) = rtrb::RingBuffer::<f32>::new(capacity);
    let stream = device
        .build_output_stream(
            config.config(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for sample in data.iter_mut() {
                    *sample = consumer.pop().unwrap_or(0.0);
                }
            },
            |err| eprintln!("audio output error: {err}"),
            None,
        )
        .map_err(|e| output(&e))?;
    stream.play().map_err(|e| output(&e))?;
    on_start(source_format, out_format);

    let in_channels = usize::from(source_format.channels);
    let out_channels = usize::from(out_format.channels);
    let mut interleaved: Vec<f32> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(decode(e)),
        };
        if packet.track_id != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(decode(e)),
        };
        interleaved.clear();
        decoded.copy_to_vec_interleaved(&mut interleaved);

        for frame in interleaved.chunks_exact(in_channels) {
            for c in 0..out_channels {
                let sample = frame[c % in_channels];
                while producer.push(sample).is_err() {
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
    }

    // Let the device drain what is still buffered.
    while producer.slots() < capacity {
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(200));
    Ok(())
}
