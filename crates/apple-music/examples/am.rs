//! Manual check against a real account.
//!
//!   cargo run -p apple-music --example am -- signin
//!   cargo run -p apple-music --example am -- library
//!   cargo run -p apple-music --example am -- search "daft punk"
//!   cargo run -p apple-music --example am -- search-library "daft punk"
//!   cargo run -p apple-music --example am -- fetch 1463409605 out.m4a
//!   cargo run -p apple-music --example am -- play 1463409605 1463409610
//!   cargo run -p apple-music --example am -- lyrics 1463409605
//!   cargo run -p apple-music --example am -- subscribed
//!   cargo run -p apple-music --example am -- radio
//!   cargo run -p apple-music --example am -- links 617154366
//!   cargo run -p apple-music --example am -- new
//!   cargo run -p apple-music --example am -- add-library song 1247608150
//!   cargo run -p apple-music --example am -- station ra.u-…
//!   cargo run -p apple-music --example am -- live ra.978194965 out.aac [seconds]
//!   cargo run -p apple-music --example am -- artist 5468295
//!   cargo run -p apple-music --example am -- favourite song 1840800867 [on|off]
//!   cargo run -p apple-music --example am -- suggest-less album 1840800866 [on|off]
//!
//! The browser defaults to `brave-origin`; set `OPIUM_BROWSER` for another one.
//! The token is stored under `$XDG_CONFIG_HOME/opium/apple-music/` and never printed.

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::time::Duration;

use apple_music::AppleMusic;
use opium_core::Provider;

fn config_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").expect("HOME")).join(".config"));
    base.join("opium/apple-music")
}

fn token_path() -> PathBuf {
    config_dir().join("media-user-token")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = run().await;
    // The CDM, when one was loaded, is torn down before the process exits.
    widevine::shutdown();
    result
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("signin") => signin().await,
        Some("library") => library(&connect().await?).await,
        Some("search") => search(&connect().await?, &args[1..].join(" ")).await,
        Some("search-library") => search_library(&connect().await?, &args[1..].join(" ")).await,
        Some("fetch") if args.len() == 3 => fetch(&connect().await?, &args[1], &args[2]).await,
        Some("play") if args.len() >= 2 => play(&connect().await?, &args[1..]).await,
        Some("artist") if args.len() == 2 => artist(&connect().await?, &args[1]).await,
        Some("favourite") if args.len() >= 3 => {
            favourite(
                &connect().await?,
                &args[1],
                &args[2],
                args.get(3).map(String::as_str),
            )
            .await
        }
        Some("suggest-less") if args.len() >= 3 => {
            suggest_less(
                &connect().await?,
                &args[1],
                &args[2],
                args.get(3).map(String::as_str),
            )
            .await
        }
        Some("live") if args.len() >= 3 => {
            live(
                &connect().await?,
                &args[1],
                &args[2],
                args.get(3).and_then(|s| s.parse().ok()).unwrap_or(20),
            )
            .await
        }
        Some("radio") => radio(&connect().await?).await,
        Some("links") if args.len() == 2 => {
            let id = opium_core::model::ItemId::new(
                opium_core::model::ProviderKind::AppleMusic,
                &args[1],
            );
            println!("{:?}", connect().await?.song_links(&id).await?);
            Ok(())
        }
        Some("new") => new_music(&connect().await?).await,
        Some("station") if args.len() == 2 => station(&connect().await?, &args[1]).await,
        Some("add-library") if args.len() == 3 => {
            add_library(&connect().await?, &args[1], &args[2]).await
        }
        Some("subscribed") => {
            println!("{:?}", connect().await?.subscribed().await?);
            Ok(())
        }
        Some("lyrics") if args.len() == 2 => lyrics(&connect().await?, &args[1]).await,
        _ => {
            eprintln!(
                "usage: am signin | library | search <term> | search-library <term> | fetch <id> <out> | play <id>… | subscribed | lyrics <id> | radio | new | links <id> | station <id> | add-library <kind> <id> | live <station> <out.aac> [seconds]"
            );
            Ok(())
        }
    }
}

async fn signin() -> Result<(), Box<dyn std::error::Error>> {
    let browser = std::env::var_os("OPIUM_BROWSER").unwrap_or("brave-origin".into());
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
    let token =
        std::fs::read_to_string(token_path()).map_err(|_| "no token yet: run `am signin` first")?;
    let am = AppleMusic::connect(token.trim()).await?;
    println!("storefront: {}", am.storefront());
    Ok(am)
}

async fn library(am: &AppleMusic) -> Result<(), Box<dyn std::error::Error>> {
    let songs = am.library_tracks(None).await?;
    println!(
        "\nsongs (first page: {}, more: {})",
        songs.items.len(),
        songs.next.is_some()
    );
    for t in songs.items.iter().take(5) {
        println!(
            "  {} — {} [{}] {:?}",
            t.title,
            t.artists.join(", "),
            t.id.id,
            t.quality
        );
    }
    if let Some(next) = &songs.next {
        let page2 = am.library_tracks(Some(next)).await?;
        println!("  page 2: {} songs", page2.items.len());
    }

    let albums = am.library_albums(None).await?;
    println!("\nalbums (first page: {})", albums.items.len());
    for a in albums.items.iter().take(5) {
        println!(
            "  {} — {} ({:?}) [{}]",
            a.title,
            a.artists.join(", "),
            a.year,
            a.id.id
        );
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

    let artists = am.library_artists(None).await?;
    println!("\nartists (first page: {})", artists.items.len());
    for a in artists.items.iter().take(5) {
        println!("  {} [{}] photo: {}", a.name, a.id.id, a.artwork.is_some());
    }

    let recent = am.recently_added(None).await?;
    println!("\nrecently added (first page: {})", recent.items.len());
    for item in recent.items.iter().take(5) {
        match item {
            opium_core::model::Collection::Album(a) => println!("  [album] {}", a.title),
            opium_core::model::Collection::Playlist(p) => println!("  [playlist] {}", p.name),
            opium_core::model::Collection::Station(s) => println!("  [station] {}", s.name),
        }
    }
    Ok(())
}

async fn artist(am: &AppleMusic, id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let id = opium_core::model::ItemId::new(opium_core::model::ProviderKind::AppleMusic, id);
    let a = am.artist(&id).await?;
    println!("{} [{}]", a.artist.name, a.artist.id.id);
    for t in &a.top_songs {
        println!("  {} ({:?})", t.title, t.album_id.as_ref().map(|i| &i.id));
    }
    for al in a.albums.iter().chain(&a.singles) {
        println!("  [album] {} ({:?})", al.title, al.year);
    }
    Ok(())
}

/// Read whether an item is a favourite; with `on` or `off`, set it first.
async fn favourite(
    am: &AppleMusic,
    kind: &str,
    id: &str,
    set: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use opium_core::model::FavouriteKind;
    let kind = match kind {
        "song" => FavouriteKind::Song,
        "album" => FavouriteKind::Album,
        "artist" => FavouriteKind::Artist,
        "playlist" => FavouriteKind::Playlist,
        _ => return Err("kind is song, album, artist or playlist".into()),
    };
    let id = opium_core::model::ItemId::new(opium_core::model::ProviderKind::AppleMusic, id);
    if let Some(set) = set {
        am.set_favourite(kind, &id, set == "on").await?;
        println!("set {set}");
    }
    println!("in favourites: {:?}", am.is_favourite(kind, &id).await?);
    Ok(())
}

async fn suggest_less(
    am: &AppleMusic,
    kind: &str,
    id: &str,
    set: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use opium_core::model::FavouriteKind;
    let kind = match kind {
        "song" => FavouriteKind::Song,
        "album" => FavouriteKind::Album,
        "playlist" => FavouriteKind::Playlist,
        _ => return Err("kind is song, album or playlist".into()),
    };
    let id = opium_core::model::ItemId::new(opium_core::model::ProviderKind::AppleMusic, id);
    if let Some(set) = set {
        am.suggest_less(kind, &id, set == "on").await?;
        println!("set {set}");
    }
    println!("suggests less: {:?}", am.suggests_less(kind, &id).await?);
    Ok(())
}

/// Record a live station's decrypted stream as ADTS, about `seconds` long.
async fn live(
    am: &AppleMusic,
    station: &str,
    out: &str,
    seconds: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let cdm = widevine::Cdm::load(&widevine::find_cdm().ok_or("no Widevine module")?)?;
    let id = opium_core::model::ItemId::new(opium_core::model::ProviderKind::AppleMusic, station);
    let stream = am.live_station(&cdm, &id).await?;
    // 256 kbps: about 32 kB a second.
    let want = seconds as usize * 32_000;
    let bytes = tokio::task::spawn_blocking(move || {
        let mut stream = stream;
        let mut buf = vec![0u8; want];
        let mut got = 0;
        while got < want {
            match stream.read(&mut buf[got..]) {
                Ok(0) => break,
                Err(e) => {
                    eprintln!("stream error: {e}");
                    break;
                }
                Ok(n) => got += n,
            }
        }
        buf.truncate(got);
        buf
    })
    .await?;
    std::fs::write(out, &bytes)?;
    println!("wrote {} bytes to {out}", bytes.len());
    Ok(())
}

async fn add_library(
    am: &AppleMusic,
    kind: &str,
    id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use opium_core::model::FavouriteKind;
    let kind = match kind {
        "song" => FavouriteKind::Song,
        "album" => FavouriteKind::Album,
        "playlist" => FavouriteKind::Playlist,
        _ => return Err("kind is song, album or playlist".into()),
    };
    let id = opium_core::model::ItemId::new(opium_core::model::ProviderKind::AppleMusic, id);
    am.add_to_library(kind, &id).await?;
    println!("added");
    Ok(())
}

async fn new_music(am: &AppleMusic) -> Result<(), Box<dyn std::error::Error>> {
    for shelf in am.new_music().await? {
        println!(
            "{} ({} items, {} songs{})",
            shelf.title,
            shelf.items.len(),
            shelf.songs.len(),
            if shelf.hero { ", hero" } else { "" }
        );
        for pick in shelf.picks.iter().take(3) {
            println!(
                "  [{}] {:?} wide: {}",
                pick.badge,
                pick.blurb,
                pick.wide.is_some()
            );
        }
    }
    Ok(())
}

async fn radio(am: &AppleMusic) -> Result<(), Box<dyn std::error::Error>> {
    for shelf in am.radio().await? {
        println!(
            "{} ({} items{})",
            shelf.title,
            shelf.items.len(),
            if shelf.hero { ", on air" } else { "" }
        );
        for item in shelf.items.iter().take(3) {
            if let opium_core::model::Collection::Station(s) = item {
                println!("  {:?} {} [{}]", s.kind, s.name, s.id.id);
            }
        }
    }
    Ok(())
}

async fn station(am: &AppleMusic, id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let id = opium_core::model::ItemId::new(opium_core::model::ProviderKind::AppleMusic, id);
    for t in am.station_tracks(&id, 12).await? {
        println!("  {} — {} [{}]", t.title, t.artists.join(", "), t.id.id);
    }
    Ok(())
}

async fn lyrics(am: &AppleMusic, id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let id = opium_core::model::ItemId::new(opium_core::model::ProviderKind::AppleMusic, id);
    let Some(lyrics) = am.lyrics(&id).await? else {
        println!("no lyrics");
        return Ok(());
    };
    println!("{} lines, synced: {}", lyrics.lines.len(), lyrics.synced);
    for line in &lyrics.lines {
        println!("  {:>7.2}  {}", line.start.as_secs_f64(), line.text);
        // Word-timed lyrics: each word's start and text.
        if !line.words.is_empty() {
            let words: Vec<String> = line
                .words
                .iter()
                .map(|w| format!("{:.2}:{:?}", w.start.as_secs_f64(), w.text))
                .collect();
            println!("           {}", words.join(" "));
        }
    }
    Ok(())
}

async fn search_library(am: &AppleMusic, term: &str) -> Result<(), Box<dyn std::error::Error>> {
    let r = am.search_library(term).await?;
    println!(
        "\n{} songs, {} albums, {} artists, {} playlists",
        r.tracks.len(),
        r.albums.len(),
        r.artists.len(),
        r.playlists.len()
    );
    for t in r.tracks.iter().take(5) {
        println!("  {} — {} [{}]", t.title, t.artists.join(", "), t.id.id);
    }
    for a in r.albums.iter().take(3) {
        println!(
            "  [album] {} — {} [{}]",
            a.title,
            a.artists.join(", "),
            a.id.id
        );
    }
    for p in r.playlists.iter().take(3) {
        println!("  [playlist] {} [{}]", p.name, p.id.id);
    }
    for a in r.artists.iter().take(3) {
        println!("  [artist] {} [{}]", a.name, a.id.id);
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
        println!(
            "  {} — {} [{}] {:?}",
            t.title,
            t.artists.join(", "),
            t.id.id,
            t.track_number
        );
    }
    for a in r.albums.iter().take(3) {
        println!(
            "  [album] {} — {} ({:?})",
            a.title,
            a.artists.join(", "),
            a.year
        );
    }
    Ok(())
}

async fn open_track(
    am: &AppleMusic,
    id: &str,
) -> Result<apple_music::AudioStream, Box<dyn std::error::Error>> {
    let cdm_path = widevine::find_cdm().ok_or("no Widevine CDM found")?;
    let cdm = widevine::Cdm::load(&cdm_path)?;
    let track = opium_core::model::ItemId::new(opium_core::model::ProviderKind::AppleMusic, id);
    // `OPIUM_SPATIAL=1`: Spatial Audio, for a song with a Dolby Atmos mix.
    let spatial = std::env::var_os("OPIUM_SPATIAL").is_some();
    let (stream, _, spatial) = am.fetch_track(&cdm, &track, spatial).await?;
    if spatial {
        println!("Spatial Audio (binaural)");
    }
    Ok(stream)
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
                player::Event::Started {
                    token,
                    source,
                    output,
                } => {
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
                        player.set_next(
                            following as u64,
                            make_source(open_track(am, &ids[following]).await?),
                        );
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
                    if paused {
                        player.pause()
                    } else {
                        player.resume()
                    }
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
                        player.play(
                            following as u64,
                            make_source(open_track(am, &ids[following]).await?),
                        );
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
    let extension = stream.extension();
    player::Source::new(stream, len, Some(extension)).with_seek_hint(hint)
}
