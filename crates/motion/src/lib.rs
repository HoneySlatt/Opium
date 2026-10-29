//! Animated cover art: a short video file decoded in a loop on a thread of
//! its own, paced by its timestamps, each frame handed over as RGBA at the
//! size it is shown. FFmpeg's libraries do the decoding.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ffmpeg_next as ffmpeg;

/// One decoded picture, rows of pixels without padding: each pixel its R,
/// G, B and A bytes in memory order, in a `u32` so that it is aligned as a
/// GPU's colour type is and can become one without a copy.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

/// A video playing in a loop. Dropping it stops the decoding.
pub struct Motion {
    latest: Arc<Mutex<Option<Frame>>>,
    failed: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

impl Motion {
    /// Play the video at `path` over and over, scaled to fit within `size`
    /// pixels a side; `wake` is called once the first frame is there, or the
    /// video turns out unreadable. After that the frames come at
    /// [`frame_interval`](Self::frame_interval): look for them then.
    pub fn play(path: PathBuf, size: u32, wake: impl Fn() + Send + 'static) -> Self {
        let latest = Arc::new(Mutex::new(None));
        let failed = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = Decoding {
            latest: latest.clone(),
            stop: stop.clone(),
            wake: Box::new(wake),
        };
        let gave_up = failed.clone();
        let _ = std::thread::Builder::new()
            .name("motion".into())
            .spawn(move || {
                if thread.run(&path, size.max(16)).is_err() {
                    gave_up.store(true, Ordering::Relaxed);
                    (thread.wake)();
                }
            });
        Self {
            latest,
            failed,
            stop,
        }
    }

    /// The picture decoded since the last one asked for, if any: handed
    /// over, not copied.
    pub fn take_frame(&self) -> Option<Frame> {
        self.latest.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    /// How often a new frame comes: Apple's covers run at 25 a second.
    pub fn frame_interval(&self) -> Duration {
        Duration::from_millis(40)
    }

    /// The video couldn't be read: show the still cover instead.
    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }
}

impl Drop for Motion {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

struct Decoding {
    latest: Arc<Mutex<Option<Frame>>>,
    stop: Arc<AtomicBool>,
    wake: Box<dyn Fn() + Send>,
}

/// Fit `width` × `height` within `size` a side, keeping the shape; sizes even,
/// as the scaler prefers.
fn fit(width: u32, height: u32, size: u32) -> (u32, u32) {
    let scale = (f64::from(size) / f64::from(width.max(height))).min(1.0);
    let even = |v: f64| ((v as u32) & !1).max(2);
    (
        even(f64::from(width) * scale),
        even(f64::from(height) * scale),
    )
}

impl Decoding {
    fn run(&self, path: &PathBuf, size: u32) -> Result<(), ffmpeg::Error> {
        ffmpeg::init()?;
        let mut input = ffmpeg::format::input(path)?;
        let stream = input
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;
        let index = stream.index();
        let time_base = f64::from(stream.time_base());
        let context = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
        let mut decoder = context.decoder().video()?;
        let (width, height) = fit(decoder.width(), decoder.height(), size);
        let mut scaler = ffmpeg::software::scaling::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGBA,
            width,
            height,
            ffmpeg::software::scaling::Flags::BILINEAR,
        )?;

        let started = Instant::now();
        // Where the current pass starts on the clock, and how far it got.
        // Apple's videos don't begin at 0: each pass counts from its first
        // timestamp.
        let mut pass_start = 0.0;
        let mut origin: Option<f64> = None;
        let mut last = 0.0;
        let mut step = 1.0 / 25.0;
        let mut woken = false;
        let mut decoded = ffmpeg::util::frame::video::Video::empty();
        let mut scaled = ffmpeg::util::frame::video::Video::empty();
        loop {
            if self.stop.load(Ordering::Relaxed) {
                return Ok(());
            }
            let mut packet = ffmpeg::Packet::empty();
            let at_end = match packet.read(&mut input) {
                Ok(()) if packet.stream() == index => {
                    decoder.send_packet(&packet)?;
                    false
                }
                Ok(()) => continue,
                Err(ffmpeg::Error::Eof) => {
                    decoder.send_eof()?;
                    true
                }
                Err(e) => return Err(e),
            };
            while decoder.receive_frame(&mut decoded).is_ok() {
                let pts = match decoded.timestamp() {
                    Some(t) => {
                        let t = t as f64 * time_base;
                        t - *origin.get_or_insert(t)
                    }
                    None => last + step,
                };
                if pts > last {
                    step = (pts - last).clamp(1.0 / 120.0, 0.5);
                }
                last = pts;
                // Wait for the frame's time, waking now and then to see
                // whether it is still wanted.
                let due = started + Duration::from_secs_f64((pass_start + pts).max(0.0));
                loop {
                    if self.stop.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    let now = Instant::now();
                    if now >= due {
                        break;
                    }
                    std::thread::sleep((due - now).min(Duration::from_millis(50)));
                }
                scaler.run(&decoded, &mut scaled)?;
                let frame = rgba(&scaled, width, height);
                *self.latest.lock().unwrap_or_else(|e| e.into_inner()) = Some(frame);
                if !woken {
                    woken = true;
                    (self.wake)();
                }
            }
            if at_end {
                // Round again, from the start, right after the last frame.
                pass_start += last + step;
                last = 0.0;
                origin = None;
                input.seek(0, ..)?;
                decoder.flush();
            }
        }
    }
}

/// The scaled picture without the rows' padding.
fn rgba(frame: &ffmpeg::util::frame::video::Video, width: u32, height: u32) -> Frame {
    let stride = frame.stride(0);
    let row = width as usize * 4;
    let data = frame.data(0);
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height as usize {
        let line = &data[y * stride..y * stride + row];
        pixels.extend(
            line.as_chunks::<4>()
                .0
                .iter()
                .map(|p| u32::from_ne_bytes(*p)),
        );
    }
    Frame {
        width,
        height,
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::fit;

    #[test]
    fn a_video_fits_within_the_size_asked_keeping_its_shape() {
        assert_eq!(fit(1080, 1080, 600), (600, 600));
        assert_eq!(fit(1080, 1440, 600), (450, 600));
        assert_eq!(fit(486, 486, 1000), (486, 486));
    }
}
