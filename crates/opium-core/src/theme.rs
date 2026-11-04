//! Themes, shared by the desktop and the phone: a TOML file with a `name`,
//! a `[layout]` table of the choices both make, `[layout.desktop]` and
//! `[layout.mobile]` for each one's own, the colours in `[colors]` and, for
//! a theme that also has light ones, `[colors.light]`. Whatever a file
//! leaves out is Stock's. A malformed colour, an unknown key or an unknown
//! choice rejects the file, with why.
//!
//! The colour roles are named after Amane's `C[]` keys, as the desktop
//! draws with them; the phone's few roles of its own (`grouped`, `cell`,
//! `capsule`, `muted`, `tint`) may be given too, else it derives them.

use serde::Deserialize;

/// The themes that come with the apps, by id.
pub const BUILT_IN: [(&str, &str); 3] = [
    ("stock", include_str!("../assets/themes/stock.toml")),
    ("spotify", include_str!("../assets/themes/spotify.toml")),
    ("m3e", include_str!("../assets/themes/m3e.toml")),
];

/// A theme file, read.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct ThemeFile {
    pub name: String,
    pub layout: Layout,
    pub desktop: DesktopLayout,
    pub mobile: MobileLayout,
    /// The colours; dark unless they say `light = true`.
    pub colors: Palette,
    /// Light colours beside them, for the phone's light appearance.
    pub light_colors: Option<Palette>,
}

/// Colours as `0xRRGGBB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Palette {
    pub light: bool,
    pub base: u32,
    pub mantle: u32,
    pub crust: u32,
    pub surface0: u32,
    pub surface1: u32,
    pub surface2: u32,
    pub overlay0: u32,
    pub overlay1: u32,
    pub text: u32,
    pub subtext0: u32,
    pub accent: u32,
    pub mauve: u32,
    pub red: u32,
    pub green: u32,
    pub yellow: u32,
    /// The phone's own roles, when the theme gives them.
    pub grouped: Option<u32>,
    pub cell: Option<u32>,
    pub capsule: Option<u32>,
    pub muted: Option<u32>,
    pub tint: Option<u32>,
    /// Material's tonal roles, for a theme that draws with them; derived
    /// from the accent when left out.
    pub primary: Option<u32>,
    pub on_primary: Option<u32>,
    pub secondary_container: Option<u32>,
    pub on_secondary_container: Option<u32>,
    pub tertiary_container: Option<u32>,
    pub on_tertiary_container: Option<u32>,
}

/// Material 3's tonal roles, which M3E draws with, as `0xrrggbb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct Tonal {
    pub primary: u32,
    pub on_primary: u32,
    pub secondary_container: u32,
    pub on_secondary_container: u32,
    pub tertiary_container: u32,
    pub on_tertiary_container: u32,
}

impl Tonal {
    /// From an accent, in OKLCH: primary light and calm, the secondary a
    /// grey of its hue, the tertiary 60° round.
    pub fn from_accent(accent: u32) -> Tonal {
        let rgb = |v: u32| [(v >> 16) as u8, (v >> 8) as u8, v as u8];
        let (c, h) = crate::color::chroma_hue(rgb(accent));
        let oklch = |l, c, h| {
            let [r, g, b] = crate::color::oklch(l, c, h);
            u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
        };
        Tonal {
            primary: oklch(0.82, c.min(0.12), h),
            on_primary: oklch(0.25, c.min(0.12), h),
            secondary_container: oklch(0.36, 0.06, h),
            on_secondary_container: oklch(0.92, 0.04, h),
            tertiary_container: oklch(0.38, 0.07, h + 60.0),
            on_tertiary_container: oklch(0.92, 0.04, h + 60.0),
        }
    }
}

impl Palette {
    /// The tonal roles as the theme gives them, what it leaves out derived
    /// from its accent.
    pub fn tonal(&self) -> Tonal {
        let d = Tonal::from_accent(self.accent);
        Tonal {
            primary: self.primary.unwrap_or(d.primary),
            on_primary: self.on_primary.unwrap_or(d.on_primary),
            secondary_container: self.secondary_container.unwrap_or(d.secondary_container),
            on_secondary_container: self
                .on_secondary_container
                .unwrap_or(d.on_secondary_container),
            tertiary_container: self.tertiary_container.unwrap_or(d.tertiary_container),
            on_tertiary_container: self
                .on_tertiary_container
                .unwrap_or(d.on_tertiary_container),
        }
    }

    /// Carbonfox's black and neutral greys, Apple Music's pink.
    pub const STOCK: Palette = Palette {
        light: false,
        base: 0x161616,
        mantle: 0x1e1e1e,
        crust: 0x101010,
        surface0: 0x242424,
        surface1: 0x2e2e2e,
        surface2: 0x3a3a3a,
        overlay0: 0x6e7074,
        overlay1: 0x9a9ca0,
        text: 0xf2f4f8,
        subtext0: 0xb6b8bb,
        accent: 0xfa2d48,
        mauve: 0xfb5c74,
        red: 0xff453a,
        green: 0x4ade80,
        yellow: 0xfbbf24,
        grouped: None,
        cell: None,
        capsule: None,
        muted: None,
        tint: None,
        primary: None,
        on_primary: None,
        secondary_container: None,
        on_secondary_container: None,
        tertiary_container: None,
        on_tertiary_container: None,
    };

    /// Stock's light colours, iOS's whites and greys.
    pub const STOCK_LIGHT: Palette = Palette {
        light: true,
        base: 0xffffff,
        mantle: 0xffffff,
        crust: 0xf2f2f7,
        surface0: 0xf0f0f2,
        surface1: 0xe6e6ea,
        surface2: 0xd8d8dd,
        overlay0: 0x8a8a8e,
        overlay1: 0x6b6b70,
        text: 0x161616,
        subtext0: 0x6b6b70,
        accent: 0xfa2d48,
        mauve: 0xfb5c74,
        red: 0xff3b30,
        green: 0x15803d,
        yellow: 0xb45309,
        grouped: None,
        cell: None,
        capsule: None,
        muted: None,
        tint: None,
        primary: None,
        on_primary: None,
        secondary_container: None,
        on_secondary_container: None,
        tertiary_container: None,
        on_tertiary_container: None,
    };

    /// `table`'s colours over `self`'s.
    fn over(mut self, table: &toml::Table) -> Result<Palette, String> {
        for (key, value) in table {
            if key == "light" {
                self.light = value.as_bool().ok_or("light is true or false")?;
                continue;
            }
            let color = value
                .as_str()
                .and_then(hex)
                .ok_or_else(|| format!("{key} is not a #rrggbb colour"))?;
            match key.as_str() {
                "base" => self.base = color,
                "mantle" => self.mantle = color,
                "crust" => self.crust = color,
                "surface0" => self.surface0 = color,
                "surface1" => self.surface1 = color,
                "surface2" => self.surface2 = color,
                "overlay0" => self.overlay0 = color,
                "overlay1" => self.overlay1 = color,
                "text" => self.text = color,
                "subtext0" => self.subtext0 = color,
                "accent" => self.accent = color,
                "mauve" => self.mauve = color,
                "red" => self.red = color,
                "green" => self.green = color,
                "yellow" => self.yellow = color,
                "grouped" => self.grouped = Some(color),
                "cell" => self.cell = Some(color),
                "capsule" => self.capsule = Some(color),
                "muted" => self.muted = Some(color),
                "tint" => self.tint = Some(color),
                "primary" => self.primary = Some(color),
                "on_primary" => self.on_primary = Some(color),
                "secondary_container" => self.secondary_container = Some(color),
                "on_secondary_container" => self.on_secondary_container = Some(color),
                "tertiary_container" => self.tertiary_container = Some(color),
                "on_tertiary_container" => self.on_tertiary_container = Some(color),
                _ => return Err(format!("{key} is not a colour role")),
            }
        }
        Ok(self)
    }
}

/// `#rrggbb`.
pub fn hex(s: &str) -> Option<u32> {
    let s = s.trim().strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    u32::from_str_radix(s, 16).ok()
}

/// Read a theme file.
pub fn parse(source: &str) -> Result<ThemeFile, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct File {
        name: Option<String>,
        #[serde(default)]
        layout: toml::Table,
        colors: Option<toml::Table>,
    }
    let file: File = toml::from_str(source).map_err(|e| e.message().to_string())?;

    // The layout: the shared choices, then each platform's table.
    let mut shared = file.layout;
    let desktop: DesktopLayout = match shared.remove("desktop") {
        Some(table) => table
            .try_into()
            .map_err(|e: toml::de::Error| format!("layout.desktop: {}", e.message()))?,
        None => DesktopLayout::default(),
    };
    let mobile: MobileLayout = match shared.remove("mobile") {
        Some(table) => table
            .try_into()
            .map_err(|e: toml::de::Error| format!("layout.mobile: {}", e.message()))?,
        None => MobileLayout::default(),
    };
    let layout: Layout = toml::Value::Table(shared)
        .try_into()
        .map_err(|e: toml::de::Error| format!("layout: {}", e.message()))?;

    // A file with no colours is Stock's, light ones too; a file with dark
    // ones only stays dark.
    let (colors, light_colors) = match file.colors {
        None => (Palette::STOCK, Some(Palette::STOCK_LIGHT)),
        Some(mut table) => {
            let light = match table.remove("light") {
                Some(toml::Value::Table(light)) => Some(
                    Palette::STOCK_LIGHT
                        .over(&light)
                        .map_err(|e| format!("colors.light: {e}"))?,
                ),
                Some(flag) => {
                    table.insert("light".into(), flag);
                    None
                }
                None => None,
            };
            (Palette::STOCK.over(&table)?, light)
        }
    };
    Ok(ThemeFile {
        name: file.name.unwrap_or_default(),
        layout,
        desktop,
        mobile,
        colors,
        light_colors,
    })
}

/// A built-in theme by its id.
pub fn built_in(id: &str) -> Option<ThemeFile> {
    BUILT_IN
        .iter()
        .find(|(built, _)| *built == id)
        .map(|(_, source)| parse(source).expect("the built-in themes are valid"))
}

/// The choices both apps make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[serde(default, deny_unknown_fields)]
pub struct Layout {
    pub header: Header,
    pub rows: Rows,
    pub cards: Cards,
    pub favourite: Favourite,
    pub headings: Headings,
    pub motion: Motion,
}

/// The desktop's own choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct DesktopLayout {
    pub window: Window,
    pub sidebar: Sidebar,
    pub top_bar: TopBar,
    pub player_bar: PlayerBar,
    pub panel: Panel,
    pub lyrics: LyricsPlace,
    pub glow: Glow,
    pub now_playing: NowPlaying,
}

/// The phone's own choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct MobileLayout {
    pub tabs: Tabs,
    pub mini_player: MiniPlayer,
    pub now_playing: MobileNowPlaying,
    pub search: SearchView,
    pub library: LibraryView,
}

macro_rules! choice {
    ($(#[$doc:meta])* $name:ident { $(#[$first_doc:meta])* $first:ident, $($(#[$rest_doc:meta])* $rest:ident),* $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
        #[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $(#[$first_doc])*
            #[default]
            $first,
            $($(#[$rest_doc])* $rest),*
        }
    };
}

choice!(
    /// Album and playlist pages' tops.
    Header {
        /// A large cover with the details beside it, as Cider and Apple Music.
        Cover,
        /// The cover on a band of its colour, the title in large type, Play
        /// in the accent and Shuffle a switch, as Spotify.
        Banner,
        /// Material 3 Expressive's: the cover (an artist in the Sunny shape),
        /// a display title, a shaped Play and a tonal Shuffle.
        Expressive,
    }
);
choice!(
    /// How a song's row in a list reads.
    Rows {
        /// On one line, the artists in a column of their own.
        Single,
        /// The artists under the title, as Spotify's.
        Stacked,
    }
);
choice!(
    /// How covers in grids and shelves are drawn.
    Cards {
        /// The cover alone, darkened on hover.
        Covers,
        /// On hover, a tile behind the cover and its words, and a round Play
        /// on the cover, as Spotify's.
        Tiles,
        /// Corners that grow on hover, a tonal tile behind, a Play that rises
        /// and changes shape; artists in the Sunny shape.
        Shapes,
    }
);
choice!(
    /// How a favourite song is marked in the lists.
    Favourite {
        /// A red star, as Apple Music's.
        Star,
        /// A check in the accent, as Spotify's saved songs.
        Check,
        /// A filled heart in the primary colour.
        Heart,
    }
);
choice!(
    /// How large the shelves' titles are.
    Headings {
        /// Semibold, as Cider's.
        Small,
        /// Larger and bold, as Spotify's.
        Large,
    }
);
choice!(
    /// How the apps move and wait.
    Motion {
        /// Apple Music's: pages slide over from the right on springs, the
        /// cover grows from the player into Now Playing, a spinner while
        /// something loads.
        Apple,
        /// Spotify's: pages fade in, what's pressed gives a little, songs
        /// change by swiping the cover, grey shapes pulse while something
        /// loads.
        Spotify,
        /// Material 3 Expressive's: springs that overshoot a little, shapes
        /// that morph, a shape-shifting indicator while something loads.
        Expressive,
    }
);
choice!(
    /// The desktop window's parts.
    Window {
        /// Side by side, edge to edge.
        Flat,
        /// Rounded cards with gaps between them on `crust`.
        Cards,
        /// Panes with large rounded corners on `crust`, the rail on `crust`
        /// itself, the player floating over them.
        Panes,
    }
);
choice!(
    /// The desktop's sidebar.
    Sidebar {
        /// Search, then the pages, the library and the playlists, as Cider.
        Navigation,
        /// Your Library, as Spotify: the playlists, albums or artists with
        /// their covers, Search and Home being in the top bar.
        Library,
        /// A navigation rail: the pages as pills, then Your Library; folds
        /// to icons with their labels under them.
        Rail,
    }
);
choice!(
    /// The desktop's top bar.
    TopBar {
        /// Over the page: back, forward and the drawer's buttons.
        Page,
        /// Across the window: back and forward, Home and Search in the
        /// middle, the account on the right.
        Window,
        /// Across the window: the rail's toggle and the brand, back and
        /// forward, a large search bar in the middle, the account.
        Search,
    }
);
choice!(
    /// The desktop's player bar.
    PlayerBar {
        /// A floating bar over the page, as Cider's.
        Floating,
        /// Along the window's bottom: the song on the left, the controls and
        /// the position in the middle, the volume on the right.
        Docked,
        /// A floating toolbar with a shaped Play and wavy progress, as
        /// Material 3 Expressive's.
        Toolbar,
    }
);
choice!(
    /// The desktop's panel on the right.
    Panel {
        /// A drawer sliding over the page with Playing Next, History and Lyrics.
        Drawer,
        /// A column beside the page with the song playing or the queue.
        Side,
    }
);
choice!(
    /// Where the desktop shows the lyrics (`Lyrics` is the model's).
    LyricsPlace {
        /// In the drawer.
        Drawer,
        /// Over the whole page, on the cover's colour.
        Page,
        /// A tab of the side panel, and of Now Playing.
        Panel,
    }
);
choice!(
    /// What lights the top of the desktop's pages without a cover.
    Glow {
        /// The accent and its glow, blurred.
        Accent,
        /// A grey fading into the page, as Spotify's.
        Grey,
        /// Blobs of the tonal containers, blurred.
        Tonal,
    }
);
choice!(
    /// The desktop's full screen player.
    NowPlaying {
        /// Cider's: the cover down the left fading into its colours, the
        /// lyrics on the right, the controls in a card.
        Lyrics,
        /// Spotify's: the cover large in the middle on its colour, cards
        /// under it, the player bar kept along the bottom.
        Cover,
        /// Material 3 Expressive's: the cover and the controls on the left,
        /// the lyrics or what's next on the right.
        Expressive,
    }
);
choice!(
    /// The phone's tabs.
    Tabs {
        /// Home, New, Radio, Library and Search, as Apple Music.
        Five,
        /// Home (New and Radio as its chips), Search and Your Library, as
        /// Spotify.
        Three,
        /// Home, New, Radio and Library, as M3E's navigation bar; search is
        /// the bar over the root pages.
        Four,
    }
);
choice!(
    /// The phone's player over the tabs.
    MiniPlayer {
        /// A capsule, as Apple Music's.
        Capsule,
        /// A card in the cover's colour with the position along its foot,
        /// as Spotify's.
        Card,
        /// M3E's floating toolbar: the turning cover in a wavy ring that is
        /// the position, the shaped Play.
        Toolbar,
    }
);
choice!(
    /// The phone's Now Playing.
    MobileNowPlaying {
        /// Apple Music's.
        Apple,
        /// Spotify's: where it plays from at the top, the cover, the song
        /// and its saved mark, the position, the controls, the device and
        /// quality, then the lyrics in a card.
        Spotify,
        /// M3E's: Playing, Lyrics and Queue tabs, the wavy position, the
        /// shaped Play in a button group, cards under it.
        Expressive,
    }
);
choice!(
    /// The phone's Search tab before typing.
    SearchView {
        /// The field and the recent searches, as Apple Music.
        Field,
        /// A white field over tiles of the catalogue's genres, as Spotify.
        Tiles,
        /// M3E's search bar over the root pages, opening the search view:
        /// recent searches as chips, genres in shapes.
        Bar,
    }
);
choice!(
    /// The phone's library tab.
    LibraryView {
        /// Apple Music's: its sections, then the recently added.
        Sections,
        /// Spotify's Your Library: chips, the songs pinned as Liked Songs,
        /// then the playlists, albums or artists.
        List,
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_themes_read() {
        let stock = built_in("stock").unwrap();
        // Stock's file says the phone's own roles; the defaults derive them.
        let own = Palette {
            grouped: None,
            cell: None,
            capsule: None,
            muted: None,
            ..stock.colors
        };
        assert_eq!(own, Palette::STOCK);
        assert_eq!(stock.layout, Layout::default());
        assert!(stock.light_colors.is_some());
        let spotify = built_in("spotify").unwrap();
        assert_eq!(spotify.layout.header, Header::Banner);
        assert_eq!(spotify.desktop.sidebar, Sidebar::Library);
        assert_eq!(spotify.mobile.tabs, Tabs::Three);
        assert_eq!(spotify.colors.accent, 0x1ed760);
        // Spotify is dark only.
        assert!(spotify.light_colors.is_none());
    }

    #[test]
    fn what_a_file_leaves_out_is_stocks() {
        let theme =
            parse("[layout]\nrows = \"stacked\"\n[layout.desktop]\nplayer_bar = \"docked\"\n[colors]\naccent = \"#00ff00\"")
                .unwrap();
        assert_eq!(theme.name, "");
        assert_eq!(theme.layout.rows, Rows::Stacked);
        assert_eq!(theme.desktop.player_bar, PlayerBar::Docked);
        assert_eq!(theme.desktop.sidebar, Sidebar::Navigation);
        assert_eq!(theme.mobile, MobileLayout::default());
        assert_eq!(theme.colors.accent, 0x00ff00);
        assert_eq!(theme.colors.base, Palette::STOCK.base);
        assert!(theme.light_colors.is_none());
        // No colours at all: Stock's, light ones too.
        assert!(parse("name = \"x\"").unwrap().light_colors.is_some());
    }

    #[test]
    fn light_colours_go_over_stocks_light_ones() {
        let theme =
            parse("[colors]\nbase = \"#000000\"\n[colors.light]\naccent = \"#123456\"").unwrap();
        let light = theme.light_colors.unwrap();
        assert!(light.light);
        assert_eq!(light.accent, 0x123456);
        assert_eq!(light.base, 0xffffff);
        // A light-only theme says so among its colours.
        assert!(parse("[colors]\nlight = true").unwrap().colors.light);
    }

    #[test]
    fn mistakes_reject_the_file() {
        assert!(parse("[layout]\nheader = \"nope\"").is_err());
        assert!(parse("[layout]\nsidebar = \"library\"").is_err());
        assert!(parse("[layout.desktop]\nsidebar = \"nope\"").is_err());
        assert!(parse("[layout.mobile]\ntab = \"three\"").is_err());
        assert!(parse("[colors]\nbase = \"blue\"").is_err());
        assert!(parse("[colors]\nnot_a_role = \"#000000\"").is_err());
        assert!(parse("not toml [").is_err());
    }
}
