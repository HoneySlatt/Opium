//! One source, demuxed and decoded to interleaved 32-bit integer frames.
//!
//! Integers rather than floats so a 16- or 24-bit lossless source reaches the
//! output unchanged: symphonia widens them by shifting, which is exact.

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
    samples: Vec<i32>,
    seek_hint: Option<SeekHint>,
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
        let stream = MediaSourceStream::new(source.reader, Default::default());
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

        let info = SourceFormat {
            codec: decoder.codec_info().short_name.to_uppercase(),
            sample_rate,
            channels: params.channels.as_ref().map_or(2, |c| c.count() as u16),
            bits_per_sample: params.bits_per_sample,
            duration: num_frames.map(|n| Duration::from_secs_f64(n as f64 / f64::from(sample_rate))),
        };
        Ok(Self {
            format,
            decoder,
            track_id,
            info,
            samples: Vec::new(),
            seek_hint: source.seek_hint,
        })
    }

    /// The next packet's frames, interleaved. `None` at the end of the source.
    pub fn next(&mut self) -> Result<Option<&[i32]>> {
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
