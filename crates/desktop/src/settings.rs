//! What the app remembers between runs, in `settings.toml` in the config
//! folder (`~/.config/opium/`).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

/// Where the colours come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    /// The theme's own (Stock's: Carbonfox's black with Apple Music's pink).
    Stock,
    /// Whatever Amane's theme switch last wrote to `~/.config/opium/theme.toml`.
    Amane,
}

/// What the app remembers between runs, as `settings.toml` holds it: each
/// setting under its own key, unset ones left out.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The source each page was last narrowed to: Albums and Recently
    /// Added, Songs, Search.
    pub grid_source: Option<opium_core::model::ProviderKind>,
    pub songs_source: Option<opium_core::model::ProviderKind>,
    pub search_source: Option<opium_core::model::ProviderKind>,
    /// The service Home, New and Radio come from, when several are
    /// connected; unset: Apple Music, else the first connected.
    pub browse_source: Option<opium_core::model::ProviderKind>,
    /// Unset: Amane when its theme file exists, Stock otherwise.
    pub appearance: Option<Appearance>,
    /// The theme's id: a built-in one, or a file's stem in
    /// `~/.config/opium/themes/`. Unset or gone, Stock.
    pub theme: Option<String>,
    /// The interface scale, 1.0 for 100 % (which draws at [`SCALE_UNIT`]);
    /// Ctrl +, Ctrl − and Ctrl 0 change it. `None` is 100 %.
    pub ui_scale: Option<f32>,
    /// A song favourited joins the library if it isn't there (on unless
    /// turned off).
    pub favourites_to_library: Option<bool>,
    /// A song added to a playlist joins the library if it isn't there (on
    /// unless turned off), as Apple Music's own option does.
    pub playlist_songs_to_library: Option<bool>,
    /// A song taken out of the library leaves the user's playlists too:
    /// Opium's and SoundCloud's, Apple's doing so itself. Off unless set.
    pub removals_leave_playlists: Option<bool>,
    /// The playlists songs were last added to, most recent first, as kept
    /// here before the devices shared them: taken in once, then emptied.
    pub recent_playlists: Vec<String>,
    /// The volume, 0 to 1, as left last time; full unless set.
    pub volume: Option<f32>,
    /// Muted when last left.
    pub muted: Option<bool>,
    /// Shuffle was on when last left.
    pub shuffle: Option<bool>,
    /// Repeat as last left; off unless set.
    pub repeat: Option<Repeat>,
    /// Where Search looked last: Apple Music unless set.
    pub search_scope: Option<crate::SearchScope>,
    /// Covers move where Apple made them move (on unless turned off).
    pub animated_artwork: Option<bool>,
    /// How sharp the moving covers are; Automatic unless set.
    pub motion_quality: Option<MotionQuality>,
    /// How the Albums page is sorted; by name, A to Z, unless set.
    pub albums_sort: Option<AlbumSort>,
    pub albums_ascending: Option<bool>,
    /// A SoundCloud track plays from the uploader's original file when they
    /// let it be downloaded (on unless turned off).
    pub soundcloud_originals: Option<bool>,
    /// The lyrics sites, in the order they are asked; all of them unless set.
    pub lyrics_order: Vec<lyrics::Source>,
    /// The lyrics sites turned on: LRCLIB alone unless set, as the others
    /// are told, from China, of every song looked up.
    pub lyrics_on: Option<Vec<lyrics::Source>>,
    /// The sites' lyrics before the service's own (off unless set).
    pub lyrics_sites_first: Option<bool>,
    /// A song mixed in Dolby Atmos plays as Apple's Spatial Audio, its
    /// binaural rendering (on unless turned off).
    pub spatial_audio: Option<bool>,
    /// The services turned off: still signed in, but not used.
    pub services_off: Vec<opium_core::model::ProviderKind>,
    /// The folders whose music files are the library's own. In the file, a
    /// name that is not UTF-8 is written as a `file://` address
    /// ([`crate::file_uri`]), and read back to the same bytes.
    #[serde(with = "folder_names")]
    pub local_folders: Vec<std::path::PathBuf>,
    /// The sidebar was folded when the app last closed.
    pub sidebar_collapsed: Option<bool>,
    /// The column beside the page, as last left: closed, or what it showed.
    pub side_closed: Option<bool>,
    pub side: Option<crate::Side>,
    /// The relay the devices meet through away from home, as
    /// `https://relay.example.org`; n0's unless set.
    pub relay: Option<String>,
    /// Recently Added merges the services' by when each was added (off
    /// unless set); otherwise Apple Music's, SoundCloud's, then Navidrome's.
    pub recent_by_date: Option<bool>,
    /// The keys this build doesn't know, as they were read, written back
    /// unchanged after the others: a newer build's settings survive a step
    /// back to this one.
    #[serde(flatten)]
    pub unknown: toml::Table,
}

/// How [`Settings::local_folders`] is kept: each folder's name as itself when
/// it is UTF-8, else as a `file://` address ([`crate::file_uri`]), ASCII,
/// which TOML can hold and [`crate::file_path`] reads back to the same bytes.
/// A UTF-8 name that starts with `file://` is written as an address too, so
/// that no name reads back as another.
///
/// A build from before this form reads such an address as the name of a
/// folder, which doesn't exist: it lists it, finds no music in it, and writes
/// it back unchanged.
mod folder_names {
    use std::path::PathBuf;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(folders: &[PathBuf], to: S) -> Result<S::Ok, S::Error> {
        to.collect_seq(folders.iter().map(|folder| match folder.to_str() {
            Some(name) if !name.starts_with("file://") => name.to_owned(),
            _ => crate::file_uri(folder),
        }))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(from: D) -> Result<Vec<PathBuf>, D::Error> {
        let names = Vec::<String>::deserialize(from)?;
        Ok(names
            .into_iter()
            .map(|name| crate::file_path(&name).unwrap_or_else(|| name.into()))
            .collect())
    }
}

/// Repeat, as kept: the queue's own, by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
    Off,
    All,
    One,
}

impl From<Repeat> for opium_core::Repeat {
    fn from(repeat: Repeat) -> Self {
        match repeat {
            Repeat::Off => Self::Off,
            Repeat::All => Self::All,
            Repeat::One => Self::One,
        }
    }
}

impl From<opium_core::Repeat> for Repeat {
    fn from(repeat: opium_core::Repeat) -> Self {
        match repeat {
            opium_core::Repeat::Off => Self::Off,
            opium_core::Repeat::All => Self::All,
            opium_core::Repeat::One => Self::One,
        }
    }
}

/// How sharp moving covers are fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MotionQuality {
    /// As many pixels as the cover takes on screen.
    Automatic,
    Low,
    High,
}

/// What the Albums page sorts by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlbumSort {
    Added,
    Name,
    Artist,
    Released,
}

/// Opium's config folder ([`crate::folders::config`]), where it keeps the
/// settings, the paired devices, the sign-ins.
///
/// # Panics
///
/// When there is none. `main` refuses to start then, before anything asks
/// ([`crate::instance::take`]).
pub fn dir() -> PathBuf {
    crate::folders::config().expect("a config folder, which main makes sure of first")
}

fn path() -> PathBuf {
    dir().join("settings.toml")
}

/// The settings files that didn't read and couldn't be put aside: never
/// written over while this process runs, so that what they hold is kept.
static LEFT_IN_PLACE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

fn left_in_place() -> MutexGuard<'static, Vec<PathBuf>> {
    LEFT_IN_PLACE.lock().unwrap_or_else(|e| e.into_inner())
}

impl Settings {
    /// The settings kept in the config folder; missing ones are the defaults.
    ///
    /// A file that doesn't read whole (not UTF-8, a value this build doesn't
    /// know, a typo) is put aside as `settings.toml.bad`, or the first of
    /// `settings.toml.bad.1`, `.2`… free, and every setting of it that reads
    /// is written in its place at once, so that a run that changes nothing,
    /// or a crash, still leaves them there; a byte that isn't UTF-8 reads as
    /// U+FFFD. A file that can't be read at all (a permission) is put aside
    /// likewise, and the defaults are written in its place. Either is said on
    /// standard error, with the name used. A file that can't be put aside is
    /// left as it is, and never written over while the app runs.
    ///
    /// A volume that isn't finite is dropped (full, as unset), one outside
    /// `0..=1` held to it, so that none other reaches the player.
    pub fn load() -> Self {
        Self::load_from(&path())
    }

    fn load_from(path: &Path) -> Self {
        // What was read, and why the file is to be put aside, if it is.
        let (settings, unread) = match std::fs::read(path) {
            Ok(bytes) => match Self::read_whole(&bytes) {
                Some(settings) => (settings, None),
                None => {
                    let text = String::from_utf8_lossy(&bytes);
                    let what_reads = text
                        .parse::<toml::Table>()
                        .map(Self::what_reads)
                        .unwrap_or_default();
                    (what_reads, Some("doesn't read whole".to_owned()))
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Self::default(), None),
            Err(e) => (Self::default(), Some(format!("can't be read ({e})"))),
        };
        let settings = settings.with_a_usable_volume();
        if let Some(why) = unread
            && put_aside(path, &why)
        {
            settings.save_to(path);
        }
        settings
    }

    /// Every setting `bytes` hold, if they read whole: UTF-8 TOML whose keys
    /// all read, or are kept as [`Settings::unknown`].
    fn read_whole(bytes: &[u8]) -> Option<Self> {
        toml::from_str(std::str::from_utf8(bytes).ok()?).ok()
    }

    /// The volume as the player can take it: one that isn't finite (a
    /// damaged `nan`) is no volume, one outside `0..=1` is held to it.
    fn with_a_usable_volume(mut self) -> Self {
        self.volume = self
            .volume
            .filter(|volume| volume.is_finite())
            .map(|volume| volume.clamp(0.0, 1.0));
        self
    }

    /// The settings of `table` that read, each on its own.
    fn what_reads(table: toml::Table) -> Self {
        let readable: toml::Table = table
            .into_iter()
            .filter(|(key, value)| {
                let mut alone = toml::Table::new();
                alone.insert(key.clone(), value.clone());
                Self::deserialize(alone).is_ok()
            })
            .collect();
        Self::deserialize(readable).unwrap_or_default()
    }

    /// Write the settings in the config folder, readable by the user only:
    /// aside, then moved over the last, so that a crash midway leaves that
    /// whole. A save that fails is said on standard error, and so is one
    /// refused over a file [`load`](Self::load) left in place.
    pub fn save(&self) {
        self.save_to(&path());
    }

    fn save_to(&self, path: &Path) {
        if left_in_place().iter().any(|left| left == path) {
            eprintln!(
                "settings: not saved, so as not to write over {}, which didn't read and \
                 couldn't be put aside",
                path.display()
            );
            return;
        }
        let text = match toml::to_string(self) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("settings: not saved: {e}");
                return;
            }
        };
        if let Err(e) = crate::backend::write_private(path, text.as_bytes()) {
            eprintln!("settings: not saved in {}: {e}", path.display());
        }
    }
}

/// Move the settings file at `path` out of the way, saying why (`why`) and
/// where to; whether `path` is free now. One that can't be moved is left in
/// place, never to be written over.
fn put_aside(path: &Path, why: &str) -> bool {
    match opium_core::files::put_aside(path, &path.with_extension("toml.bad")) {
        Ok(aside) => {
            eprintln!(
                "settings: {} {why}; it is kept as {}",
                path.display(),
                aside.display()
            );
            true
        }
        // Gone meanwhile: nothing is left to keep.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(e) => {
            eprintln!(
                "settings: {} {why}, and couldn't be put aside ({e}): it is left as it is, and \
                 the settings aren't saved over it",
                path.display()
            );
            left_in_place().push(path.to_owned());
            false
        }
    }
}

/// What 100 % draws at: the design's sizes, 10 % larger (the user's call).
pub const SCALE_UNIT: f32 = 1.1;

#[cfg(test)]
mod tests {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use crate::SearchScope;

    /// A folder of the test's own under the temporary folder, gone after.
    struct Dir(PathBuf);

    impl Dir {
        fn new(name: &str) -> Self {
            // Each test names its own; the process's id sets runs apart.
            let dir =
                std::env::temp_dir().join(format!("opium-settings-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self) -> PathBuf {
            self.0.join("settings.toml")
        }

        fn names(&self) -> Vec<String> {
            let mut names: Vec<String> = std::fs::read_dir(&self.0)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn folder(name: &[u8]) -> PathBuf {
        std::ffi::OsStr::from_bytes(name).into()
    }

    #[test]
    fn every_folder_name_and_setting_reads_back_the_same() {
        let dir = Dir::new("folders");
        let every_byte: Vec<u8> = (1..=255).collect();
        let kept = Settings {
            volume: Some(0.3),
            shuffle: Some(true),
            theme: Some("Stock".into()),
            local_folders: vec![
                folder(b"/music/caf\xe9"),
                folder(b"/music/C\xe9line Dion"),
                folder("/music/Céline".as_bytes()),
                folder(b"/music/100% live"),
                folder(b"file://x"),
                folder(&every_byte),
            ],
            ..Settings::default()
        };
        kept.save_to(&dir.file());
        let text = std::fs::read_to_string(dir.file()).unwrap();
        assert!(
            text.contains("\"/music/Céline\""),
            "a UTF-8 name as itself: {text}"
        );
        assert!(text.contains("\"file:///music/caf%E9\""), "{text}");
        assert_eq!(Settings::load_from(&dir.file()), kept);
        assert_eq!(dir.names(), ["settings.toml"], "nothing put aside");
    }

    /// The form of `local_folders` before a name not in UTF-8 could be kept.
    #[derive(Serialize, Deserialize)]
    struct OlderBuild {
        local_folders: Vec<PathBuf>,
    }

    #[test]
    fn an_older_build_writes_back_a_folder_kept_as_an_address() {
        let kept = Settings {
            local_folders: vec![folder(b"/music/a"), folder(b"/music/caf\xe9")],
            ..Settings::default()
        };
        let text = toml::to_string(&kept).unwrap();
        let older: OlderBuild = toml::from_str(&text).unwrap();
        assert_eq!(
            older.local_folders,
            [folder(b"/music/a"), folder(b"file:///music/caf%E9")]
        );
        let written_back = toml::to_string(&older).unwrap();
        assert_eq!(
            toml::from_str::<Settings>(&written_back)
                .unwrap()
                .local_folders,
            kept.local_folders
        );
    }

    #[test]
    fn keys_this_build_doesnt_know_are_written_back_unchanged() {
        let dir = Dir::new("unknown");
        let newer = "volume = 0.3\nsleep_timer_minutes = 30\nseen = 2026-11-10T10:00:00Z\n\
                     [equaliser]\nbands = [1.5, -2.0]\nname = \"Night\"\n\n\
                     [[stations]]\nname = \"a\"\n\n[[stations]]\nname = \"b\"\n";
        std::fs::write(dir.file(), newer).unwrap();
        let mut settings = Settings::load_from(&dir.file());
        assert_eq!(settings.volume, Some(0.3));
        assert_eq!(
            dir.names(),
            ["settings.toml"],
            "a file that reads isn't put aside"
        );
        settings.shuffle = Some(true);
        settings.save_to(&dir.file());
        let written: toml::Table = std::fs::read_to_string(dir.file())
            .unwrap()
            .parse()
            .unwrap();
        let newer: toml::Table = newer.parse().unwrap();
        for (key, value) in &newer {
            assert_eq!(written.get(key), Some(value), "{key}");
        }
        assert_eq!(
            written.get("shuffle"),
            Some(&true.into()),
            "the change is kept too"
        );
    }

    #[test]
    fn a_file_that_doesnt_read_whole_keeps_the_keys_this_build_doesnt_know() {
        let text = "volume = 0.3\nrepeat = \"sometimes\"\nsleep_timer_minutes = 30\n";
        let kept = Settings::what_reads(text.parse().unwrap());
        assert_eq!(kept.volume, Some(0.3));
        assert_eq!(kept.repeat, None);
        assert_eq!(kept.unknown.get("sleep_timer_minutes"), Some(&30.into()));
    }

    #[test]
    fn a_file_that_cant_be_read_is_put_aside_and_the_defaults_written_in_its_place() {
        let dir = Dir::new("unreadable");
        std::fs::write(dir.file(), "volume = 0.3\n").unwrap();
        std::fs::set_permissions(dir.file(), std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read(dir.file()).is_ok() {
            panic!("run as a user that permissions apply to, not root");
        }
        let loaded = Settings::load_from(&dir.file());
        assert_eq!(loaded, Settings::default());
        assert_eq!(dir.names(), ["settings.toml", "settings.toml.bad"]);
        let aside = dir.0.join("settings.toml.bad");
        std::fs::set_permissions(&aside, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(std::fs::read_to_string(&aside).unwrap(), "volume = 0.3\n");
        assert_eq!(Settings::load_from(&dir.file()), Settings::default());
    }

    // Stage 1 audit, L13: `write_private` wrote into a partial file left from before as it was.
    #[test]
    fn the_settings_are_the_users_alone_even_beside_a_partial_file_open_to_others() {
        let dir = Dir::new("private");
        // As an older build or a crash may have left it, readable by others.
        let left = dir.0.join("settings.part");
        std::fs::write(&left, "left").unwrap();
        std::fs::set_permissions(&left, std::fs::Permissions::from_mode(0o644)).unwrap();
        Settings {
            volume: Some(0.5),
            ..Settings::default()
        }
        .save_to(&dir.file());
        let mode = std::fs::metadata(dir.file()).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the settings are {mode:o}");
        assert_eq!(Settings::load_from(&dir.file()).volume, Some(0.5));
        assert_eq!(
            std::fs::read(&left).unwrap(),
            b"left",
            "a file it didn't make is left alone"
        );
    }

    #[test]
    fn a_file_put_aside_leaves_what_read_of_it_in_its_place() {
        let dir = Dir::new("in-place");
        std::fs::write(
            dir.file(),
            "volume = 0.3\ntheme = \"Spotify\"\nshuffle = true\nrepeat = \"sometimes\"\n",
        )
        .unwrap();
        let first = Settings::load_from(&dir.file());
        // Nothing saved in between: the app changed nothing, or crashed.
        let second = Settings::load_from(&dir.file());
        assert_eq!(second, first);
        assert_eq!(
            (second.volume, second.theme.as_deref(), second.shuffle),
            (Some(0.3), Some("Spotify"), Some(true))
        );
        assert_eq!(dir.names(), ["settings.toml", "settings.toml.bad"]);
    }

    #[test]
    fn a_file_that_cant_be_put_aside_is_never_written_over() {
        let dir = Dir::new("left");
        // A name whose `.bad` beside it is longer than a name can be, so
        // that it can't be put aside, though `write_private` could replace
        // it (its `.part` is no longer).
        let file = dir.0.join(format!("{}.toml", "s".repeat(250)));
        let damaged: &[u8] = b"volume = 0.3\ntheme = \"caf\xe9\"\n";
        std::fs::write(&file, damaged).unwrap();
        let loaded = Settings::load_from(&file);
        assert_eq!(loaded.volume, Some(0.3), "what reads is kept");
        loaded.save_to(&file);
        Settings {
            volume: Some(0.5),
            ..loaded
        }
        .save_to(&file);
        assert_eq!(std::fs::read(&file).unwrap(), damaged);
        assert_eq!(dir.names().len(), 1, "{:?}", dir.names());
    }

    #[test]
    fn each_file_put_aside_keeps_its_own_name() {
        let dir = Dir::new("aside");
        for damaged in [
            "volume = 0.3\nrepeat = \"sometimes\"\n",
            "shuffle = \"maybe\"\n",
        ] {
            std::fs::write(dir.file(), damaged).unwrap();
            Settings::load_from(&dir.file()).save_to(&dir.file());
        }
        assert_eq!(
            dir.names(),
            ["settings.toml", "settings.toml.bad", "settings.toml.bad.1"]
        );
        assert_eq!(
            std::fs::read_to_string(dir.0.join("settings.toml.bad")).unwrap(),
            "volume = 0.3\nrepeat = \"sometimes\"\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.0.join("settings.toml.bad.1")).unwrap(),
            "shuffle = \"maybe\"\n"
        );
    }

    #[test]
    fn a_volume_reaches_the_player_finite_and_within_full() {
        let dir = Dir::new("volume");
        for (text, volume) in [
            ("volume = nan\n", None),
            ("volume = -nan\n", None),
            ("volume = inf\n", None),
            ("volume = -inf\n", None),
            ("volume = 1.5\n", Some(1.0)),
            ("volume = -0.5\n", Some(0.0)),
            ("volume = 0.3\n", Some(0.3)),
            ("volume = 1\n", Some(1.0)),
            // In a file that doesn't read whole as well.
            ("volume = nan\nrepeat = \"sometimes\"\n", None),
            ("volume = 4.0\nrepeat = \"sometimes\"\n", Some(1.0)),
        ] {
            std::fs::write(dir.file(), text).unwrap();
            assert_eq!(Settings::load_from(&dir.file()).volume, volume, "{text}");
        }
    }

    #[test]
    fn a_setting_that_doesnt_read_loses_only_itself() {
        let text = "volume = 0.3\nrepeat = \"sometimes\"\nshuffle = true\n";
        assert!(toml::from_str::<Settings>(text).is_err());
        let kept = Settings::what_reads(text.parse().unwrap());
        assert_eq!(kept.volume, Some(0.3));
        assert_eq!(kept.shuffle, Some(true));
        assert_eq!(kept.repeat, None);
    }

    #[test]
    fn what_is_kept_reads_back_the_same() {
        let kept = Settings {
            search_scope: Some(SearchScope::Library),
            shuffle: Some(true),
            repeat: Some(Repeat::One),
            volume: Some(0.3),
            ..Settings::default()
        };
        let text = toml::to_string(&kept).unwrap();
        assert!(text.contains("search_scope = \"library\""), "{text}");
        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), kept);
    }

    /// Stage 1 audit, L3: a folder whose name is not UTF-8 stops no save.
    #[test]
    fn a_folder_whose_name_isnt_utf8_doesnt_stop_the_settings_being_kept() {
        use std::os::unix::ffi::OsStringExt;
        let kept = Settings {
            volume: Some(0.3),
            local_folders: vec![std::ffi::OsString::from_vec(b"/music/caf\xe9".to_vec()).into()],
            ..Settings::default()
        };
        assert!(toml::to_string(&kept).is_ok());
    }
}
