//! Download a song sealed, as the downloads will be kept, and check it
//! reads back as the stream it came from and decodes to the end.
//!
//!   cargo run -p downloads --features keyring --example spike -- apple <song id> <out>
//!   cargo run -p downloads --features keyring --example spike -- navidrome <out>
//!
//! Uses the sign-ins the app keeps, never printed.

use std::io::{Read, Seek, SeekFrom, Write};
use std::time::Instant;

use downloads::sealed;
use opium_core::model::{ItemId, ProviderKind};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let key = downloads::keyring::key().await?;
    let again = downloads::keyring::key().await?;
    assert_eq!(key, again, "the keyring gave another key");
    println!("key: from the keyring");
    let started = Instant::now();
    let (stream, extension): (Box<dyn Read + Send>, &str) = match &args[..] {
        [service, id, _] if service == "apple" => {
            let token = std::fs::read_to_string(config().join("apple-music/media-user-token"))?;
            let am = apple_music::AppleMusic::connect(token.trim()).await?;
            println!("subscribed: {:?}", am.subscribed().await?);
            let cdm = widevine::Cdm::load(&widevine::find_cdm().ok_or("no Widevine CDM")?)?;
            let track = ItemId::new(ProviderKind::AppleMusic, id.as_str());
            let (stream, kbps, spatial) = am.fetch_track(&cdm, &track, false).await?;
            println!("{kbps:?} kbps, spatial {spatial}");
            let extension = stream.extension();
            (Box::new(stream), extension)
        }
        [service, _] if service == "navidrome" => {
            use opium_core::Provider;
            let json = std::fs::read_to_string(config().join("navidrome/credentials.json"))?;
            let nd = navidrome::Navidrome::connect(serde_json::from_str(&json)?).await?;
            let song = nd
                .library_tracks(None)
                .await?
                .items
                .into_iter()
                .next()
                .ok_or("no songs on the server")?;
            println!("the server's first song, {:?}", song.duration);
            let audio = nd.audio(&song.id).await?;
            (Box::new(audio.reader), audio.extension)
        }
        _ => return Err("usage: spike apple <song id> <out> | navidrome <out>".into()),
    };
    println!(".{extension}");
    let out = args.last().expect("an out path").clone();

    // Read on a thread: the stream waits on its download.
    let (clear, file) = {
        let out = out.clone();
        tokio::task::spawn_blocking(move || -> std::io::Result<_> {
            let mut stream = stream;
            let mut clear = Vec::new();
            stream.read_to_end(&mut clear)?;
            let mut writer = sealed::Writer::new(std::fs::File::create(&out)?, &key)?;
            writer.write_all(&clear)?;
            let file = writer.finish()?;
            file.sync_all()?;
            Ok((clear, file))
        })
        .await??
    };
    let on_disk = file.metadata()?.len();
    println!(
        "{} bytes, {on_disk} sealed, after {:.2?}",
        clear.len(),
        started.elapsed()
    );

    let mut reader = sealed::Reader::new(std::fs::File::open(&out)?, &key)?;
    let mut back = Vec::new();
    reader.read_to_end(&mut back)?;
    assert_eq!(back, clear, "the sealed file reads back otherwise");
    println!("reads back byte for byte");
    let bytes_on_disk = std::fs::read(&out)?;
    let shows = [b"ftyp", b"moov", b"fLaC", b"RIFF", b"OggS"];
    assert!(
        !shows
            .iter()
            .any(|magic| bytes_on_disk.windows(magic.len()).any(|w| w == *magic)),
        "the song shows through the seal"
    );
    println!("nothing of the song shows on disk");

    reader.seek(SeekFrom::Start(0))?;
    decode(reader, extension)?;
    Ok(())
}

fn config() -> std::path::PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").expect("HOME")).join(".config")
        })
        .join("opium")
}

/// Decode `reader` at 100 s, then from its start to its end.
fn decode(
    reader: sealed::Reader<std::fs::File>,
    extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::errors::Error;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::units::Time;

    struct Sealed(sealed::Reader<std::fs::File>);
    impl Read for Sealed {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(buf)
        }
    }
    impl Seek for Sealed {
        fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
            self.0.seek(to)
        }
    }
    impl symphonia::core::io::MediaSource for Sealed {
        fn is_seekable(&self) -> bool {
            true
        }
        fn byte_len(&self) -> Option<u64> {
            Some(self.0.len())
        }
    }

    let source = MediaSourceStream::new(Box::new(Sealed(reader)), Default::default());
    let mut hint = Hint::new();
    hint.with_extension(extension);
    let mut format = symphonia::default::get_probe().probe(
        &hint,
        source,
        FormatOptions::default(),
        Default::default(),
    )?;
    let track = format
        .first_track(TrackType::Audio)
        .ok_or("no audio")?
        .clone();
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or("not audio")?
        .clone();
    let rate = params.sample_rate.ok_or("no rate")?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())?;
    let middle = Time::try_from_secs_f64(100.0).unwrap_or_default();
    format.seek(
        SeekMode::Coarse,
        SeekTo::Time {
            time: middle,
            track_id: Some(track.id),
        },
    )?;
    let packet = format.next_packet()?.ok_or("nothing after the middle")?;
    decoder.reset();
    decoder.decode(&packet)?;
    println!("seeks to 100 s and decodes there");
    format.seek(
        SeekMode::Coarse,
        SeekTo::Time {
            time: Time::default(),
            track_id: Some(track.id),
        },
    )?;
    decoder.reset();
    let (mut frames, mut bad) = (0u64, 0);
    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        };
        match decoder.decode(&packet) {
            Ok(decoded) => frames += decoded.frames() as u64,
            Err(Error::DecodeError(_)) => bad += 1,
            Err(e) => return Err(e.into()),
        }
    }
    println!(
        "decoded to the end: {:.2} s at {rate} Hz, {bad} packets that didn't decode",
        frames as f64 / f64::from(rate)
    );
    Ok(())
}
