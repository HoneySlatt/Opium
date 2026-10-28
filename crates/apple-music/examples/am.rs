//! Manual check against a real account.
//!
//!   cargo run -p apple-music --example am -- signin
//!   cargo run -p apple-music --example am -- library
//!   cargo run -p apple-music --example am -- search "daft punk"
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
        _ => {
            eprintln!("usage: am signin | library | search <term>");
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
        println!("  {} — {}", t.title, t.artists.join(", "));
    }
    for a in r.albums.iter().take(3) {
        println!("  [album] {} — {} ({:?})", a.title, a.artists.join(", "), a.year);
    }
    Ok(())
}
