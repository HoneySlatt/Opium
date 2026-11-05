//! Try the SoundCloud client from the command line.
//!
//! `sc signin` opens a browser on soundcloud.com and keeps the tokens under
//! `$XDG_CONFIG_HOME/opium/soundcloud/`, readable by the user only; they are
//! never printed. Then: `sc me`, `sc likes`, `sc playlists`, `sc playlist <id>`,
//! `sc search <words>`, `sc radio <track id>`, `sc fetch <track id> <out>
//! [--stream]`. `sc public
//! <track id> <out>` fetches a public track without signing in.

use std::path::PathBuf;
use std::time::Duration;

use opium_core::Provider;
use opium_core::model::{ItemId, ProviderKind};
use soundcloud::{SoundCloud, Tokens};

fn dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").expect("HOME")).join(".config"));
    base.join("opium/soundcloud")
}

fn keep(tokens: &Tokens) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let _ = std::fs::create_dir_all(dir());
    let json = serde_json::json!({ "oauth": tokens.oauth, "refresh": tokens.refresh });
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(dir().join("tokens.json"))
    {
        let _ = file.write_all(json.to_string().as_bytes());
    }
}

fn kept() -> Option<Tokens> {
    let text = std::fs::read_to_string(dir().join("tokens.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    Some(Tokens {
        oauth: json["oauth"].as_str()?.to_owned(),
        refresh: json["refresh"].as_str().map(str::to_owned),
    })
}

async fn connect() -> Result<SoundCloud, Box<dyn std::error::Error>> {
    let tokens = kept().ok_or("no tokens yet: run `sc signin` first")?;
    let sc = SoundCloud::connect(tokens, Box::new(keep), None).await?;
    println!("signed in as {}", sc.username());
    Ok(sc)
}

fn track_id(id: &str) -> ItemId {
    ItemId::new(ProviderKind::SoundCloud, id)
}

async fn fetch(
    sc: &SoundCloud,
    id: &str,
    out: &str,
    original: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    // The browser's CDM, for an encrypted track.
    let cdm = widevine::find_cdm().and_then(|path| widevine::Cdm::load(&path).ok());
    let mut audio = sc.audio(&track_id(id), original, cdm.as_ref()).await?;
    let ready = started.elapsed();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut audio.reader, &mut bytes)?;
    std::fs::write(out, &bytes)?;
    println!(
        "{} bytes, playable after {ready:?}, all in {:?}: {:?} ({}{})",
        bytes.len(),
        started.elapsed(),
        audio.quality,
        audio.extension,
        if audio.original { ", the original" } else { "" }
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or_default();
    match arg(0) {
        "signin" => {
            let profile = dir().join("browser-profile");
            std::fs::create_dir_all(&profile)?;
            let browser =
                std::env::var_os("OPIUM_BROWSER").unwrap_or_else(|| "brave-origin".into());
            let tokens =
                soundcloud::signin::sign_in(browser.as_ref(), &profile, Duration::from_secs(600))
                    .await?;
            println!(
                "signed in (refresh token: {})",
                if tokens.refresh.is_some() {
                    "yes"
                } else {
                    "no"
                }
            );
            keep(&tokens);
        }
        "me" => {
            connect().await?;
        }
        "likes" => {
            let sc = connect().await?;
            let page = sc.library_tracks(None).await?;
            for t in page.items.iter().take(20) {
                println!(
                    "{:>12}  {} — {}  {:?}",
                    t.id.id,
                    t.artists.join(", "),
                    t.title,
                    t.quality
                );
            }
            println!(
                "{} on the first page, more: {}",
                page.items.len(),
                page.next.is_some()
            );
        }
        "playlists" => {
            let sc = connect().await?;
            let mut cursor = None;
            loop {
                let page = sc.library_playlists(cursor.as_ref()).await?;
                for p in &page.items {
                    println!(
                        "{:>12}  {}  ({:?} tracks, by {:?}, editable {})",
                        p.id.id, p.name, p.track_count, p.curator, p.editable
                    );
                }
                match page.next {
                    Some(next) => cursor = Some(next),
                    None => break,
                }
            }
        }
        "playlist" => {
            let sc = connect().await?;
            let page = sc.playlist_tracks(&track_id(arg(1)), None).await?;
            for t in &page.items {
                println!("{:>12}  {} — {}", t.id.id, t.artists.join(", "), t.title);
            }
            println!("{} tracks", page.items.len());
        }
        "search" => {
            let sc = connect().await?;
            let results = sc.search(&args[1..].join(" ")).await?;
            for t in results.tracks.iter().take(10) {
                println!(
                    "track {:>12}  {} — {}  {:?}",
                    t.id.id,
                    t.artists.join(", "),
                    t.title,
                    t.quality
                );
            }
            for a in results.artists.iter().take(5) {
                println!("user  {:>12}  {}", a.id.id, a.name);
            }
            for p in results.playlists.iter().take(5) {
                println!("set   {:>12}  {}", p.id.id, p.name);
            }
            for a in results.albums.iter().take(5) {
                println!(
                    "album {:>12}  {} — {}",
                    a.id.id,
                    a.artists.join(", "),
                    a.title
                );
            }
        }
        "radio" if args.len() >= 2 => {
            let sc = connect().await?;
            let links = sc.song_links(&track_id(arg(1))).await?;
            let station = links.station.ok_or("no station")?;
            for round in 0..2 {
                let tracks = sc.station_tracks(&station, 20).await?;
                println!("round {round}: {} tracks", tracks.len());
                for t in tracks.iter().take(4) {
                    println!("  {:>12}  {} — {}", t.id.id, t.artists.join(", "), t.title);
                }
            }
        }
        "fetch" if args.len() >= 3 => {
            let sc = connect().await?;
            fetch(&sc, arg(1), arg(2), arg(3) != "--stream").await?;
        }
        "public" if args.len() >= 3 => {
            let sc = SoundCloud::connect(Tokens::default(), Box::new(|_| {}), None).await?;
            fetch(&sc, arg(1), arg(2), false).await?;
        }
        _ => eprintln!(
            "usage: sc signin | me | likes | playlists | playlist <id> | search <words> | radio <track id> | fetch <track id> <out> [--stream] | public <track id> <out>"
        ),
    }
    Ok(())
}
