//! A development aid: `OPIUM_SCREENSHOT=out.png [OPIUM_SCREENSHOT_AFTER=seconds]
//! [OPIUM_SCREENSHOT_PLAY=row]` saves the window once it has had time to load,
//! then quits, optionally playing a Songs row first. `OPIUM_SCREENSHOT_PAGE`
//! opens a page first: `playlist:<index>`, `library-artist:<index>`, `album:<id>`, `artist:<id>` or
//! `search:<query>`, `shelf:home|new|radio:<title>`, or `page:home|new|radio|albums|artists|recent|songs|settings|favourites`; `OPIUM_SCREENSHOT_DRAWER`
//! opens the queue drawer; `OPIUM_SCREENSHOT_LYRICS` the lyrics page; `OPIUM_SCREENSHOT_STATION=ra.…` tunes in to a station.
//! `OPIUM_SCREENSHOT_DOWNLOAD` downloads the album or playlist opened, once
//! loaded; `OPIUM_SCREENSHOT_DOWNLOADED` shows Songs' downloads only.
//! Only the app's own window is captured.

use std::path::PathBuf;

pub struct Screenshot {
    path: PathBuf,
    after: f64,
    requested: bool,
    play: Option<usize>,
    page: Option<String>,
    /// For an artist of the library's: Artists opened, so that they load.
    artists_asked: bool,
    /// A station to tune in to: `OPIUM_SCREENSHOT_STATION=ra.…`.
    station: Option<String>,
    /// The page's songs to download, once loaded.
    download: bool,
}

impl Screenshot {
    pub fn from_env() -> Option<Self> {
        let path = std::env::var_os("OPIUM_SCREENSHOT")?.into();
        let after = std::env::var("OPIUM_SCREENSHOT_AFTER")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8.0);
        let play = std::env::var("OPIUM_SCREENSHOT_PLAY")
            .ok()
            .and_then(|s| s.parse().ok());
        Some(Self {
            path,
            after,
            requested: false,
            play,
            page: std::env::var("OPIUM_SCREENSHOT_PAGE").ok(),
            artists_asked: false,
            station: std::env::var("OPIUM_SCREENSHOT_STATION").ok(),
            download: std::env::var_os("OPIUM_SCREENSHOT_DOWNLOAD").is_some(),
        })
    }

    /// The Songs row to play, once, when the library has loaded.
    pub fn play_row(&mut self, loaded: bool) -> Option<usize> {
        if loaded { self.play.take() } else { None }
    }

    /// Whether to download the page's songs now: once, when they loaded.
    pub fn download(&mut self, loaded: bool) -> bool {
        loaded && std::mem::take(&mut self.download)
    }

    /// The station to tune in to, once, when connected.
    pub fn station(&mut self, connected: bool) -> Option<String> {
        if connected { self.station.take() } else { None }
    }

    /// The page to open, once, when the playlists have loaded.
    pub fn page(
        &mut self,
        playlists: &[opium_core::model::Playlist],
        artists: &[opium_core::model::Artist],
    ) -> Option<crate::app::Page> {
        if playlists.is_empty() {
            return None;
        }
        // An artist of the library's waits for them, Artists open meanwhile
        // so that they load.
        if artists.is_empty()
            && self
                .page
                .as_deref()
                .is_some_and(|p| p.starts_with("library-artist:"))
        {
            let first = !std::mem::replace(&mut self.artists_asked, true);
            return first.then_some(crate::app::Page::Artists);
        }
        let spec = self.page.take()?;
        let (kind, value) = spec.split_once(':')?;
        match kind {
            "library-artist" => value
                .parse::<usize>()
                .ok()
                .and_then(|i| artists.get(i))
                .map(|a| {
                    crate::app::Page::LibraryArtist(
                        a.library_id.clone().unwrap_or_else(|| a.id.clone()),
                    )
                }),
            "playlist" => value
                .parse::<usize>()
                .ok()
                .and_then(|i| playlists.get(i))
                .map(|p| crate::app::Page::Playlist(p.id.clone())),
            "album" => Some(crate::app::Page::Album(opium_core::model::ItemId::new(
                opium_core::model::ProviderKind::AppleMusic,
                value,
            ))),
            "search" => Some(crate::app::Page::Search(value.to_owned())),
            "page" => match value {
                "home" => Some(crate::app::Page::Home),
                "radio" => Some(crate::app::Page::Radio),
                "new" => Some(crate::app::Page::New),
                "settings" => Some(crate::app::Page::Settings),
                "favourites" => Some(crate::app::Page::Favourites),
                "albums" => Some(crate::app::Page::Albums),
                "artists" => Some(crate::app::Page::Artists),
                "recent" => Some(crate::app::Page::RecentlyAdded),
                "songs" => Some(crate::app::Page::Songs),
                _ => None,
            },
            // `shelf:home|new|radio:<title>`
            "shelf" => {
                let (which, title) = value.split_once(':')?;
                let which = match which {
                    "home" => crate::app::Shelves::Home,
                    "new" => crate::app::Shelves::New,
                    "radio" => crate::app::Shelves::Radio,
                    _ => return None,
                };
                Some(crate::app::Page::Shelf(which, title.to_owned()))
            }
            "artist" => Some(crate::app::Page::Artist(opium_core::model::ItemId::new(
                opium_core::model::ProviderKind::AppleMusic,
                value,
            ))),
            _ => None,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        if self.requested {
            return;
        }
        if ctx.input(|i| i.time) < self.after {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
            return;
        }
        self.requested = true;
        let (path, closer) = (self.path.clone(), ctx.clone());
        ctx.request_screenshot(move |image| {
            let [w, h] = image.size;
            let pixels: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            match image::save_buffer(&path, &pixels, w as u32, h as u32, image::ColorType::Rgba8) {
                Ok(()) => eprintln!("screenshot saved to {}", path.display()),
                Err(e) => eprintln!("screenshot failed: {e}"),
            }
            closer.send_viewport_cmd(egui::ViewportCommand::Close);
            closer.request_repaint();
        });
    }
}
