//! How long Apple Music's steps take Opium on the owner's account, before and after
//! `crates/apple-music`'s rewrite (M1.12, `docs/specs/apple-music-playback.md`):
//!
//! - the developer token read from the web player, on a client of its own, then the whole
//!   connection (that token and the account's storefront), as Opium starts;
//! - the library's first page of songs, then every song of the library;
//! - a batch of 20 songs from the station of `song`;
//! - `song` ready to play: from asking the web player to the first samples decrypted, then a digest
//!   of its first mebibyte of cleartext, so that two builds can be shown to decrypt alike.
//!
//! It needs a signed-in account (the token `am signin` keeps) and asks Apple for one licence and
//! one station batch a run, which moves that station's cursor: only with the owner's go
//! (AGENTS.md, "Never on your own"). It changes nothing in the library; nothing is played or
//! written to disk.
//!
//! ```sh
//! cargo run --release -p apple-music --example am_cost -- <catalog song id>
//! ```
//!
//! `scripts/measure/cdm-cost.sh` runs two builds of it in turn, the same way. It calls only the
//! crate's public interface, which the rewrite keeps, so that the same file builds on both sides.

use std::hash::{DefaultHasher, Hasher};
use std::io::Read;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use apple_music::AppleMusic;
use opium_core::Provider;
use opium_core::model::{ItemId, ProviderKind};

/// The songs a station batch asks for, as the desktop does (`STATION_BATCH`).
const STATION_BATCH: usize = 20;
/// The cleartext read once the song is ready.
const DIGESTED: usize = 1 << 20;
/// Opium's own user agent, so that Apple serves the page it serves Opium.
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = run().await;
    widevine::shutdown();
    result
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let song = std::env::args()
        .nth(1)
        .ok_or("usage: am_cost <catalog song id>")?;
    let token =
        std::fs::read_to_string(token_path()).map_err(|_| "no token yet: run `am signin` first")?;

    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
    let started = Instant::now();
    apple_music::auth::developer_token(&client).await?;
    println!("developer token: {}", ms(started.elapsed()));
    drop(client);

    let started = Instant::now();
    let am = AppleMusic::connect(token.trim()).await?;
    println!("connected: {}", ms(started.elapsed()));

    let started = Instant::now();
    let page = am.library_tracks(None).await?;
    println!(
        "library, first page: {} songs in {}",
        page.items.len(),
        ms(started.elapsed())
    );

    let started = Instant::now();
    let mut songs = 0;
    am.each_library_track(|tracks, _| songs += tracks.len())
        .await?;
    println!("library, every song: {songs} in {}", ms(started.elapsed()));

    let track = ItemId::new(ProviderKind::AppleMusic, song.as_str());
    let station = am
        .song_links(&track)
        .await?
        .station
        .ok_or("the song has no station")?;
    let started = Instant::now();
    let batch = am.station_tracks(&station, STATION_BATCH).await?;
    let elapsed = started.elapsed();
    let mut ids: Vec<&str> = batch.iter().map(|t| t.id.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    println!(
        "station batch of {STATION_BATCH}: {} songs, {} distinct, in {}",
        batch.len(),
        ids.len(),
        ms(elapsed)
    );

    let cdm = widevine::Cdm::load(&widevine::find_cdm().ok_or("no Widevine CDM found")?)?;
    let started = Instant::now();
    let (mut stream, kbps, _) = am.fetch_track(&cdm, &track, false).await?;
    println!(
        "song {song} ready to play: {} ({} kbps)",
        ms(started.elapsed()),
        kbps.map_or("unknown".into(), |k| k.to_string())
    );
    // The stream reads as it downloads, blocking: off the runtime's threads.
    let (read, digest) = tokio::task::spawn_blocking(move || {
        let mut clear = vec![0; DIGESTED];
        let mut read = 0;
        while read < clear.len() {
            match stream.read(&mut clear[read..]) {
                Ok(0) | Err(_) => break,
                Ok(n) => read += n,
            }
        }
        let mut digest = DefaultHasher::new();
        digest.write(&clear[..read]);
        (read, digest.finish())
    })
    .await?;
    println!(
        "first {read} bytes of cleartext after {}, digest {digest:016x}",
        ms(started.elapsed())
    );
    Ok(())
}

fn token_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").expect("HOME")).join(".config"));
    base.join("opium/apple-music/media-user-token")
}

fn ms(d: Duration) -> String {
    format!("{:.1} ms", d.as_secs_f64() * 1e3)
}
