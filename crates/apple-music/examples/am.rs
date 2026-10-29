//! Manual check against a real account.
//!
//!   cargo run -p apple-music --example am -- signin
//!   cargo run -p apple-music --example am -- library
//!   cargo run -p apple-music --example am -- search "daft punk"
//!   cargo run -p apple-music --example am -- fetch 1463409605 out.m4a
//!   cargo run -p apple-music --example am -- play 1463409605 1463409610
//!
//! The browser defaults to `brave-origin`; set `MUSIC_BROWSER` for another one.
//! The token is stored under `$XDG_CONFIG_HOME/music/apple-music/` and never printed.

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::time::Duration;

use apple_music::AppleMusic;
use music_core::Provider;

fn config_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").expect("HOME")).join(".config"));
    base.join("music/apple-music")
}

fn token_path() -> PathBuf {
    config_dir().join("media-user-token")
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("signin") => signin().await,
        Some("library") => library(&connect().await?).await,
        Some("search") => search(&connect().await?, &args[1..].join(" ")).await,
        Some("fetch") if args.len() == 3 => fetch(&connect().await?, &args[1], &args[2]).await,
        Some("play") if args.len() >= 2 => play(&connect().await?, &args[1..]).await,
        _ => {
            eprintln!("usage: am signin | library | search <term> | fetch <id> <out> | play <id>…");
            Ok(())
        }
    }
}

async fn signin() -> Result<(), Box<dyn std::error::Error>> {
    let browser = std::env::var_os("MUSIC_BROWSER").unwrap_or("brave-origin".into());
    let profile = config_dir().join("browser-profile");
    std::fs::create_dir_all(&profile)?;
    println!("Sign in to Apple Music in the window that opens…");
    let token =
        apple_music::signin::sign_in(browser.as_ref(), &profile, Duration::from_secs(600)).await?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(token_path())?;
    file.write_all(token.as_bytes())?;
    println!("Signed in, token saved to {}", token_path().display());
    Ok(())
}

async fn connect() -> Result<AppleMusic, Box<dyn std::error::Error>> {
    let token = std::fs::read_to_string(token_path())
        .map_err(|_| "no token yet: run `am signin` first")?;
    let am = AppleMusic::connect(token.trim()).await?;
    println!("storefront: {}", am.storefront());
    Ok(am)
}

async fn library(am: &AppleMusic) -> Result<(), Box<dyn std::error::Error>> {
    let songs = am.library_tracks(None).await?;
    println!("\nsongs (first page: {}, more: {})", songs.items.len(), songs.next.is_some());
    for t in songs.items.iter().take(5) {
        println!("  {} — {} [{}] {:?}", t.title, t.artists.join(", "), t.id.id, t.quality);
    }
    if let Some(next) = &songs.next {
        let page2 = am.library_tracks(Some(next)).await?;
        println!("  page 2: {} songs", page2.items.len());
    }

    let albums = am.library_albums(None).await?;
    println!("\nalbums (first page: {})", albums.items.len());
    for a in albums.items.iter().take(5) {
        println!("  {} — {} ({:?})", a.title, a.artists.join(", "), a.year);
    }
    if let Some(album) = albums.items.first() {
        let tracks = am.album_tracks(&album.id).await?;
        println!("  tracks of « {} »: {}", album.title, tracks.len());
    }

    let playlists = am.library_playlists(None).await?;
    println!("\nplaylists (first page: {})", playlists.items.len());
    for p in playlists.items.iter().take(5) {
        println!("  {}", p.name);
    }
    if let Some(playlist) = playlists.items.first() {
        let tracks = am.playlist_tracks(&playlist.id, None).await?;
        println!("  tracks of « {} »: {}", playlist.name, tracks.items.len());
    }
    Ok(())
}

async fn search(am: &AppleMusic, term: &str) -> Result<(), Box<dyn std::error::Error>> {
    let r = am.search(term).await?;
    println!(
        "\n{} songs, {} albums, {} artists, {} playlists",
        r.tracks.len(),
        r.albums.len(),
        r.artists.len(),
        r.playlists.len()
    );
    for t in r.tracks.iter().take(5) {
        println!("  {} — {} [{}] {:?}", t.title, t.artists.join(", "), t.id.id, t.track_number);
    }
    for a in r.albums.iter().take(3) {
        println!("  [album] {} — {} ({:?})", a.title, a.artists.join(", "), a.year);
    }
    Ok(())
}

async fn open_track(
    am: &AppleMusic,
    id: &str,
) -> Result<apple_music::AudioStream, Box<dyn std::error::Error>> {
    let cdm_path = widevine::locate().ok_or("no Widevine CDM found")?;
    let cdm = widevine::Cdm::open(&cdm_path)?;
    let track = music_core::model::ItemId::new(music_core::model::ProviderKind::AppleMusic, id);
    Ok(am.fetch_track(&cdm, &track).await?)
}

async fn fetch(am: &AppleMusic, id: &str, out: &str) -> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let mut stream = open_track(am, id).await?;
    println!("ready to read after {:.2?}", started.elapsed());
    let mut file = std::fs::File::create(out)?;
    let written = std::io::copy(&mut stream, &mut file)?;
    println!("{written} bytes decrypted after {:.2?}", started.elapsed());
    Ok(())
}

/// Play `ids` in order, gapless where the formats allow. On stdin: `p`
/// pause/resume, `s <seconds>` seek, `n` next, `q` quit.
async fn play(am: &AppleMusic, ids: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let player = player::Player::new();
    let started = std::time::Instant::now();

    player.play(0, make_source(open_track(am, &ids[0]).await?));
    let mut queued = 0;

    let (lines_tx, lines) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        for line in std::io::stdin().lines().map_while(Result::ok) {
            if lines_tx.send(line).is_err() {
                break;
            }
        }
    });

    let mut paused = false;
    let mut last_report = std::time::Instant::now();
    loop {
        while let Ok(event) = player.events().try_recv() {
            match event {
                player::Event::Started { token, source, output } => {
                    println!(
                        "[{:>6.2?}] ▶ {} — {} {} Hz × {} → {} {} Hz × {} {}",
                        started.elapsed(),
                        ids[token as usize],
                        source.codec,
                        source.sample_rate,
                        source.channels,
                        output.device,
                        output.sample_rate,
                        output.channels,
                        output.sample_format,
                    );
                    // Queue the following track as soon as this one is audible.
                    let following = token as usize + 1;
                    if following < ids.len() && queued < following {
                        queued = following;
                        player.set_next(following as u64, make_source(open_track(am, &ids[following]).await?));
                    }
                }
                player::Event::Finished => {
                    println!("[{:>6.2?}] finished", started.elapsed());
                    return Ok(());
                }
                player::Event::Error { token, message } => println!("error ({token:?}): {message}"),
            }
        }
        while let Ok(line) = lines.try_recv() {
            println!("[{:>6.2?}] < {line}", started.elapsed());
            let mut words = line.split_whitespace();
            match words.next() {
                Some("p") => {
                    paused = !paused;
                    if paused { player.pause() } else { player.resume() }
                }
                Some("s") => {
                    if let Some(secs) = words.next().and_then(|w| w.parse::<f64>().ok()) {
                        player.seek(std::time::Duration::from_secs_f64(secs));
                    }
                }
                Some("n") => {
                    let status = player.status();
                    let following = status.token.map_or(0, |t| t as usize + 1);
                    if following < ids.len() {
                        queued = following;
                        player.play(following as u64, make_source(open_track(am, &ids[following]).await?));
                    }
                }
                Some("q") => return Ok(()),
                _ => {}
            }
        }
        if last_report.elapsed() >= std::time::Duration::from_secs(1) {
            last_report = std::time::Instant::now();
            let status = player.status();
            println!(
                "[{:>6.2?}] {:?} at {:.1?}{}{}",
                started.elapsed(),
                status.token.map(|t| &ids[t as usize]),
                status.position,
                if status.paused { " (paused)" } else { "" },
                status
                    .output
                    .map(|o| format!(" — device at {} Hz", o.sample_rate))
                    .unwrap_or_default(),
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

fn make_source(stream: apple_music::AudioStream) -> player::Source {
    let len = stream.len();
    let hint = stream.seek_hint();
    player::Source::new(stream, len, Some("m4a")).with_seek_hint(hint)
}
