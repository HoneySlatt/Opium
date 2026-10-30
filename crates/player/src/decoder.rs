//! One source, demuxed and decoded to interleaved 32-bit float frames.
//!
//! A 16- or 24-bit lossless source reaches the output unchanged: a float
//! holds 24 bits exactly, and PipeWire mixes in floats. A lossy one keeps
//! what its decoder gives above full scale (loud masters' AAC goes over),
//! so the volume brings it down rather than it being clipped first.

use std::time::Duration;

use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;

use crate::{Error, Result, SeekHint, Source, SourceFormat};

pub(crate) struct Decoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    pub info: SourceFormat,
    samples: Vec<f32>,
    seek_hint: Option<SeekHint>,
    /// Where the music is among the frames of an AAC encode in MP4.
    music: Option<Music>,
}

/// An AAC encode's music among its frames: after the encoder's priming, up
/// to its length when known, and within each packet's duration (the last
/// one ends early). Frames outside it would be a gap between songs.
struct Music {
    /// In frames.
    start: u64,
    end: Option<u64>,
    /// Frames per timestamp tick, as a fraction.
    ticks: (u64, u64),
}

impl Music {
    fn frames(&self, ticks: u64) -> u64 {
        ticks * self.ticks.0 / self.ticks.1
    }
}

fn decode_error(e: symphonia::core::errors::Error) -> Error {
    Error::Decode(e.to_string())
}

impl Decoder {
    pub fn open(source: Source) -> Result<Self> {
        let mut hint = Hint::new();
        if let Some(ext) = &source.extension {
            hint.with_extension(ext);
        }
        let mut reader = source.reader;
        let mp4 = matches!(
            source.extension.as_deref(),
            Some("m4a" | "mp4" | "m4b" | "aac" | "mp4a")
        );
        let trim = (mp4 && reader.is_seekable())
            .then(|| crate::gapless::mp4_trim(&mut reader))
            .flatten();
        let stream = MediaSourceStream::new(reader, Default::default());
        let format = symphonia::default::get_probe()
            .probe(
                &hint,
                stream,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(decode_error)?;
        let track = format
            .first_track(TrackType::Audio)
            .ok_or_else(|| Error::Decode("no audio track".into()))?;
        let track_id = track.id;
        let num_frames = track.num_frames;
        let time_base = track.time_base;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .cloned()
            .ok_or_else(|| Error::Decode("no audio parameters".into()))?;
        let sample_rate = params
            .sample_rate
            .ok_or_else(|| Error::Decode("unknown sample rate".into()))?;
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&params, &AudioDecoderOptions::default())
            .map_err(decode_error)?;

        let aac = decoder.codec_info().short_name.eq_ignore_ascii_case("aac");
        // Ticks to frames: a tick is `numer / denom` seconds.
        let ticks = time_base.map(|tb| {
            (
                u64::from(tb.numer.get()) * u64::from(sample_rate),
                u64::from(tb.denom.get()),
            )
        });
        let music = match (aac && mp4, ticks) {
            (true, Some(ticks)) => {
                let to_frames = |v: u64, scale: Option<u32>| match scale {
                    Some(scale) if scale > 0 => v * u64::from(sample_rate) / u64::from(scale),
                    _ => v,
                };
                let (start, end) = match trim {
                    Some(t) => {
                        let start = to_frames(t.priming, t.timescale);
                        (start, t.length.map(|l| start + to_frames(l, t.timescale)))
                    }
                    None => (u64::from(source.priming.unwrap_or(0)), None),
                };
                Some(Music { start, end, ticks })
            }
            _ => None,
        };
        let info = SourceFormat {
            codec: decoder.codec_info().short_name.to_uppercase(),
            sample_rate,
            channels: params.channels.as_ref().map_or(2, |c| c.count() as u16),
            layout: match &params.channels {
                Some(symphonia::core::audio::Channels::Positioned(p)) => Some(p.bits()),
                _ => None,
            },
            bits_per_sample: params.bits_per_sample,
            duration: num_frames
                .map(|n| Duration::from_secs_f64(n as f64 / f64::from(sample_rate))),
        };
        Ok(Self {
            format,
            decoder,
            track_id,
            info,
            samples: Vec::new(),
            seek_hint: source.seek_hint,
            music,
        })
    }

    /// The next packet's frames, interleaved. `None` at the end of the source.
    pub fn next(&mut self) -> Result<Option<&[f32]>> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => return Ok(None),
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None);
                }
                Err(symphonia::core::errors::Error::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(e) => return Err(decode_error(e)),
            };
            if packet.track_id != self.track_id {
                continue;
            }
            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    self.samples.clear();
                    decoded.copy_to_vec_interleaved(&mut self.samples);
                    if let Some(music) = &self.music {
                        // The packet's frames that are music, if any.
                        let channels = usize::from(self.info.channels.max(1));
                        let frames = (self.samples.len() / channels) as u64;
                        let at = music.frames(u64::try_from(packet.pts.get()).unwrap_or(0));
                        let from = music.start.saturating_sub(at).min(frames);
                        let mut to = frames.min(music.frames(packet.dur.get()).max(from));
                        if let Some(end) = music.end {
                            to = to.min(end.saturating_sub(at));
                        }
                        if to <= from {
                            continue;
                        }
                        self.samples.truncate(to as usize * channels);
                        self.samples.drain(..from as usize * channels);
                    }
                    return Ok(Some(&self.samples));
                }
                // A corrupt packet costs its few milliseconds, not the track.
                Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
                Err(e) => return Err(decode_error(e)),
            }
        }
    }

    /// Seek near `to`; returns where decoding actually resumes.
    pub fn seek(&mut self, to: Duration) -> Result<Duration> {
        if let Some(hint) = &mut self.seek_hint {
            hint(to);
        }
        let time = Time::try_from_secs_f64(to.as_secs_f64()).unwrap_or_default();
        let seeked = self
            .format
            .seek(
                SeekMode::Coarse,
                SeekTo::Time {
                    time,
                    track_id: Some(self.track_id),
                },
            )
            .map_err(decode_error)?;
        self.decoder.reset();
        let actual = self
            .format
            .tracks()
            .iter()
            .find(|t| t.id == self.track_id)
            .and_then(|t| t.time_base)
            .and_then(|tb| tb.calc_time(seeked.actual_ts))
            .map(|t| Duration::from_secs_f64(t.as_secs_f64().max(0.0)));
        Ok(actual.unwrap_or(to))
    }
}
