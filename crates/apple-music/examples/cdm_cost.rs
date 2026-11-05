//! What the browser's CDM costs Opium, on one Apple Music song (`docs/specs/widevine-host.md` §11):
//!
//! - loading the CDM, and the time from opening a session to a usable key, the CDM's share and
//!   the licence server's apart;
//! - the CDM's time for each sample of the whole song, decrypted in place as the player needs it;
//! - the process's wake-ups while it waits, a session open, nothing asked of the CDM;
//! - closing the session and shutting the CDM down, as Opium does when it quits.
//!
//! It needs a signed-in account (the token `am signin` keeps) and asks Apple for one licence a
//! run: only with the owner's go (AGENTS.md, "Never on your own"). The encrypted file is held in
//! memory, its cleartext too, and nothing is written to disk; only a digest of the cleartext is
//! printed, so that two builds can be shown to decrypt alike.
//!
//! ```sh
//! cargo run --release -p apple-music --example cdm_cost -- <catalog song id> [seconds at rest]
//! ```
//!
//! `scripts/measure/cdm-cost.sh` runs two builds of it in turn, the same way. Every CDM call is in
//! the module `cdm` below, the only part that follows `widevine`'s interface.

use std::hash::{DefaultHasher, Hasher};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use apple_music::AppleMusic;
use opium_core::model::{ItemId, ProviderKind};

/// The four calls measured, on `widevine`'s interface.
mod cdm {
    pub use widevine::{Cdm, Session};

    pub fn load() -> Result<Cdm, Box<dyn std::error::Error>> {
        let path = widevine::find_cdm().ok_or("no Widevine CDM found")?;
        Ok(Cdm::load(&path)?)
    }

    pub fn open_session(
        cdm: &Cdm,
        pssh: &[u8],
    ) -> Result<(Session, Vec<u8>), Box<dyn std::error::Error>> {
        Ok(cdm.open_session(pssh)?)
    }

    pub fn give_licence(
        session: &Session,
        licence: &[u8],
        key_id: &[u8; 16],
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(session.give_licence(licence, key_id)?)
    }

    /// One sample decrypted over `bytes` with the playlist's key, as the player's decrypt closure
    /// does it.
    pub fn decrypt(
        session: &Session,
        bytes: &mut [u8],
        key_id: &[u8; 16],
        sample: &cenc::Sample<'_>,
    ) -> Result<(), String> {
        let protected = widevine::Protected {
            key_id,
            iv: &sample.iv,
            subsamples: sample.subsamples,
            scheme: widevine::Scheme::Cenc,
        };
        session
            .decrypt(bytes, &protected)
            .map_err(|e| e.to_string())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let song = args
        .next()
        .ok_or("usage: cdm_cost <catalog song id> [seconds at rest]")?;
    let rest_seconds: u64 = args.next().map_or(Ok(30), |s| s.parse())?;

    let token =
        std::fs::read_to_string(token_path()).map_err(|_| "no token yet: run `am signin` first")?;
    let token = token.trim().to_owned();
    let am = AppleMusic::connect(token.clone()).await?;
    let track = ItemId::new(ProviderKind::AppleMusic, song.as_str());
    let protected = am
        .protected(&track, false)
        .await?
        .ok_or("not a catalog song: it has no licence")?;
    let key_id: [u8; 16] = protected
        .key_id
        .as_slice()
        .try_into()
        .map_err(|_| "a key id that is not 16 bytes")?;
    let file = download(&protected.url, &token).await?;
    let (movie, samples) = cenc::index(&file)?;
    let timescale = movie.tracks.first().map_or(0, |t| t.timescale);
    // The connections to Apple's hosts are opened before the licence is timed.
    am.warm().await;

    let started = Instant::now();
    let loaded = cdm::load()?;
    let load = started.elapsed();

    let started = Instant::now();
    let (session, challenge) = cdm::open_session(&loaded, &protected.pssh)?;
    let open = started.elapsed();
    let asked = Instant::now();
    let licence = am.license(&protected, &challenge).await?;
    let server = asked.elapsed();
    let given = Instant::now();
    cdm::give_licence(&session, &licence, &key_id)?;
    let give = given.elapsed();
    let usable = started.elapsed();

    let mut clear = file.clone();
    let mut times = Vec::with_capacity(samples.len());
    let mut audio_units = 0u64;
    for sample in samples.iter() {
        audio_units += u64::from(sample.duration);
        if !sample.protected {
            continue;
        }
        let bytes = &mut clear[sample.offset..sample.offset + sample.size];
        let started = Instant::now();
        cdm::decrypt(&session, bytes, &key_id, &sample)?;
        times.push(started.elapsed());
    }
    let mut digest = DefaultHasher::new();
    for sample in samples.iter() {
        digest.write(&clear[sample.offset..sample.offset + sample.size]);
    }
    drop(clear);

    // The process at rest, a session open: every thread's wake-ups over the period.
    std::thread::sleep(Duration::from_secs(2));
    let before = thread_switches();
    std::thread::sleep(Duration::from_secs(rest_seconds));
    let after = thread_switches();

    println!(
        "song {song}: {} samples, {} bytes",
        samples.len(),
        file.len()
    );
    println!("CDM loaded and initialised: {}", ms(load));
    println!(
        "session to usable key: {} (open {}, licence server {}, licence given {}; CDM {})",
        ms(usable),
        ms(open),
        ms(server),
        ms(give),
        ms(open + give)
    );
    report_decrypts(&mut times, audio_units, timescale);
    println!("cleartext digest: {:016x}", digest.finish());
    report_rest(&before, &after, rest_seconds);

    // What quitting costs once the window is gone: the session closed, the CDM torn down.
    let started = Instant::now();
    drop(session);
    widevine::shutdown();
    println!("session closed and CDM shut down: {}", ms(started.elapsed()));
    Ok(())
}

fn token_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").expect("HOME")).join(".config"));
    base.join("opium/apple-music/media-user-token")
}

/// The whole encrypted file, with the user's token as Apple's asset hosts want it.
async fn download(url: &str, token: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let parsed = reqwest::Url::parse(url)?;
    let apple = parsed.scheme() == "https"
        && parsed
            .host_str()
            .is_some_and(|h| h == "apple.com" || h.ends_with(".apple.com"));
    if !apple {
        return Err("the file is not on Apple's hosts".into());
    }
    let resp = reqwest::Client::new()
        .get(url)
        .header("x-apple-music-user-token", token)
        .header("Cookie", format!("media-user-token={token}"))
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.bytes().await?.to_vec())
}

fn report_decrypts(times: &mut [Duration], audio_units: u64, timescale: u32) {
    if times.is_empty() {
        println!("no protected sample");
        return;
    }
    times.sort_unstable();
    let total: Duration = times.iter().sum();
    let at = |q: f64| times[((times.len() - 1) as f64 * q).round() as usize];
    println!(
        "decrypts: {} samples, {} in all, mean {:.1} µs, median {:.1} µs, p95 {:.1} µs, p99 {:.1} µs, max {:.1} µs",
        times.len(),
        ms(total),
        total.as_secs_f64() * 1e6 / times.len() as f64,
        us(at(0.5)),
        us(at(0.95)),
        us(at(0.99)),
        us(times[times.len() - 1]),
    );
    // The CDM paces its output: a few calls wait inside it far longer than the rest.
    for floor_ms in [1, 10, 100] {
        let floor = Duration::from_millis(floor_ms);
        let slow: Vec<_> = times.iter().filter(|&&t| t >= floor).collect();
        println!(
            "  {} decrypts of {floor_ms} ms or more, {} in all",
            slow.len(),
            ms(slow.into_iter().sum())
        );
    }
    if timescale > 0 {
        let audio = audio_units as f64 / f64::from(timescale);
        println!(
            "audio {audio:.1} s, decrypted at {:.0} times real time",
            audio / total.as_secs_f64()
        );
    }
}

fn report_rest(before: &[(String, String, u64)], after: &[(String, String, u64)], seconds: u64) {
    println!("at rest for {seconds} s, a session open: wake-ups a second by thread");
    let mut others = 0u64;
    for (tid, name, count) in after {
        let earlier = before
            .iter()
            .find(|(t, ..)| t == tid)
            .map_or(0, |(.., c)| *c);
        let woken = count - earlier.min(*count);
        let main = *tid == std::process::id().to_string();
        if !main {
            others += woken;
        }
        println!(
            "  {tid} {name}{}: {:.2}/s",
            if main { " (the measuring thread)" } else { "" },
            woken as f64 / seconds as f64
        );
    }
    println!(
        "  every thread but the measuring one: {:.2}/s",
        others as f64 / seconds as f64
    );
}

/// Each thread of this process: its id, its name and its context switches so far, voluntary and
/// not, which count its wake-ups.
fn thread_switches() -> Vec<(String, String, u64)> {
    let mut threads = Vec::new();
    let Ok(tasks) = std::fs::read_dir("/proc/self/task") else {
        return threads;
    };
    for task in tasks.flatten() {
        let path = task.path();
        let tid = task.file_name().to_string_lossy().into_owned();
        let name = std::fs::read_to_string(path.join("comm")).unwrap_or_default();
        let status = std::fs::read_to_string(path.join("status")).unwrap_or_default();
        let switches = status
            .lines()
            .filter(|l| l.contains("ctxt_switches:"))
            .filter_map(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
            .sum();
        threads.push((tid, name.trim().to_owned(), switches));
    }
    threads.sort();
    threads
}

fn ms(d: Duration) -> String {
    format!("{:.2} ms", d.as_secs_f64() * 1e3)
}

fn us(d: Duration) -> f64 {
    d.as_secs_f64() * 1e6
}
