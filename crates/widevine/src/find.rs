//! Finding the browser's CDM on this machine (§1): every candidate, and the one to use.

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use crate::Candidate;
use crate::manifest::manifest_version;

/// The library's file name in every browser's tree.
const LIBRARY: &str = "libwidevinecdm.so";

/// The user-data folders of Chromium-family browsers under a config folder, in alphabetical
/// order. A name ending in `*` stands for every folder whose name starts with what comes
/// before it: `BraveSoftware/*` is every folder under `BraveSoftware`, each one a Brave's
/// (`Brave-Browser`, `Brave-Browser-Beta`, `Brave-Origin`, …).
const CHROMIUM_FOLDERS: &[&str] = &[
    "BraveSoftware/*",
    "chromium",
    "google-chrome",
    "google-chrome-beta",
    "google-chrome-unstable",
    "microsoft-edge*",
    "opera*",
    "vivaldi",
    "vivaldi-snapshot",
];

/// The Flatpak applications whose config folder holds a Chromium-family browser's tree.
const FLATPAK_CHROMIUMS: &[&str] = &[
    "com.brave.Browser",
    "com.google.Chrome",
    "com.microsoft.Edge",
    "com.vivaldi.Vivaldi",
    "org.chromium.Chromium",
];

/// A manifest larger than this is not read: real ones are about a kilobyte.
const MANIFEST_LIMIT: u64 = 64 * 1024;

/// The CDM to use: among the usable candidates of [`cdm_candidates`], the highest version,
/// compared as numbers part by part; between equal versions, the first path in byte order.
///
/// When `WIDEVINE_CDM` is set, the file it names, if it is a regular file. `None` when no
/// candidate can be used.
pub fn find_cdm() -> Option<PathBuf> {
    choose(&cdm_candidates())
}

/// Every candidate considered, in the order they were looked for, with why each one that
/// cannot be used was set aside: for a diagnostic.
///
/// When `WIDEVINE_CDM` is set (and not empty), only the file it names. Otherwise each
/// Chromium-family browser's component under the config folder (`XDG_CONFIG_HOME`, else
/// `~/.config`), Google Chrome's own under `/opt/google/chrome`, the same browsers under
/// Flatpak, and Firefox's profiles. Reading the file system never fails the search: a folder
/// that cannot be read holds no candidate.
pub fn cdm_candidates() -> Vec<Candidate> {
    candidates_in(&Places::from_environment())
}

/// Where to look; the environment's, or a test's own folders.
pub(crate) struct Places {
    /// `WIDEVINE_CDM`, when set.
    pub explicit: Option<PathBuf>,
    /// The config folder.
    pub config: Option<PathBuf>,
    /// The home folder.
    pub home: Option<PathBuf>,
    /// Google Chrome's installation folder.
    pub chrome_install: PathBuf,
    /// The component's platform folder for this processor, when browsers ship one for it.
    pub platform: Option<&'static str>,
}

impl Places {
    fn from_environment() -> Places {
        let non_empty = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
        let home = non_empty("HOME").map(PathBuf::from);
        let config = non_empty("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            // The XDG specification ignores a relative path.
            .filter(|path| path.is_absolute())
            .or_else(|| home.as_ref().map(|home| home.join(".config")));
        Places {
            explicit: non_empty("WIDEVINE_CDM").map(PathBuf::from),
            config,
            home,
            chrome_install: PathBuf::from("/opt/google/chrome"),
            platform: match std::env::consts::ARCH {
                "x86_64" => Some("linux_x64"),
                "aarch64" => Some("linux_arm64"),
                _ => None,
            },
        }
    }
}

pub(crate) fn candidates_in(places: &Places) -> Vec<Candidate> {
    if let Some(path) = &places.explicit {
        return vec![check_file(path.clone(), None)];
    }
    let mut found = Vec::new();
    if let Some(platform) = places.platform {
        if let Some(config) = &places.config {
            chromium_trees(config, platform, &mut found);
        }
        let install = places.chrome_install.join("WidevineCdm");
        if install.is_dir() {
            let library = install
                .join("_platform_specific")
                .join(platform)
                .join(LIBRARY);
            let version = chromium_version(&install);
            found.push(check_file(library, Some(version)));
        }
        if let Some(home) = &places.home {
            for app in FLATPAK_CHROMIUMS {
                let config = home.join(".var/app").join(app).join("config");
                chromium_trees(&config, platform, &mut found);
            }
        }
    }
    if let Some(home) = &places.home {
        firefox_profiles(&home.join(".mozilla/firefox"), &mut found);
        firefox_profiles(
            &home.join(".var/app/org.mozilla.firefox/.mozilla/firefox"),
            &mut found,
        );
    }
    found
}

/// The usable candidate with the highest version; between equal versions, the first path in
/// byte order. A candidate with no version (`WIDEVINE_CDM`'s) is usable unless set aside.
pub(crate) fn choose(candidates: &[Candidate]) -> Option<PathBuf> {
    candidates
        .iter()
        .filter(|candidate| candidate.set_aside.is_none())
        .map(|candidate| {
            let parts = candidate
                .version
                .as_deref()
                .and_then(version_parts)
                .unwrap_or_default();
            (parts, candidate)
        })
        .max_by(|(left_parts, left), (right_parts, right)| {
            compare_versions(left_parts, right_parts)
                // The first path in byte order wins, so it must compare as the greatest.
                .then_with(|| {
                    right
                        .path
                        .as_os_str()
                        .as_bytes()
                        .cmp(left.path.as_os_str().as_bytes())
                })
        })
        .map(|(_, candidate)| candidate.path.clone())
}

/// A version's parts as numbers, or `None` when it does not read as dot-separated numbers.
pub(crate) fn version_parts(version: &str) -> Option<Vec<u64>> {
    version
        .split('.')
        .map(|part| {
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            part.parse().ok()
        })
        .collect()
}

/// Compares versions part by part as numbers; a missing part counts as 0, so that `4.10` and
/// `4.10.0` are equal.
pub(crate) fn compare_versions(left: &[u64], right: &[u64]) -> Ordering {
    let length = left.len().max(right.len());
    (0..length)
        .map(|index| {
            let left = left.get(index).copied().unwrap_or(0);
            let right = right.get(index).copied().unwrap_or(0);
            left.cmp(&right)
        })
        .find(|ordering| ordering.is_ne())
        .unwrap_or(Ordering::Equal)
}

/// Every Chromium-family browser's components under one config folder.
fn chromium_trees(config: &Path, platform: &str, found: &mut Vec<Candidate>) {
    for folder in CHROMIUM_FOLDERS {
        for user_data in matching_folders(config, folder) {
            let components = user_data.join("WidevineCdm");
            for component in subfolders(&components) {
                let library = component
                    .join("_platform_specific")
                    .join(platform)
                    .join(LIBRARY);
                let version = chromium_version(&component);
                found.push(check_file(library, Some(version)));
            }
        }
    }
}

/// Firefox's components: `<profile>/gmp-widevinecdm/<version>/`.
fn firefox_profiles(profiles: &Path, found: &mut Vec<Candidate>) {
    for profile in subfolders(profiles) {
        for component in subfolders(&profile.join("gmp-widevinecdm")) {
            let version = folder_name(&component);
            found.push(check_file(component.join(LIBRARY), Some(version)));
        }
    }
}

/// The folders a name of [`CHROMIUM_FOLDERS`] stands for, under `config`.
fn matching_folders(config: &Path, pattern: &str) -> Vec<PathBuf> {
    let Some(start) = pattern.strip_suffix('*') else {
        let path = config.join(pattern);
        return if path.is_dir() {
            vec![path]
        } else {
            Vec::new()
        };
    };
    // The pattern's last part is a name's start, possibly empty; what comes before it, the
    // folder to look in.
    let (parent, prefix) = match start.rsplit_once('/') {
        Some((parent, prefix)) => (config.join(parent), prefix),
        None => (config.to_path_buf(), start),
    };
    subfolders(&parent)
        .into_iter()
        .filter(|folder| {
            folder
                .file_name()
                .is_some_and(|name| name.as_bytes().starts_with(prefix.as_bytes()))
        })
        .collect()
}

/// The folders directly in `folder`, symbolic links followed, sorted by name in byte order;
/// none when it cannot be read.
fn subfolders(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut folders: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    folders.sort_by(|left, right| {
        left.as_os_str()
            .as_bytes()
            .cmp(right.as_os_str().as_bytes())
    });
    folders
}

/// A Chromium component's version: its manifest's when it has one that names it, else its
/// folder's name.
fn chromium_version(component: &Path) -> String {
    read_manifest(&component.join("manifest.json"))
        .and_then(|text| manifest_version(&text).map(str::to_owned))
        .unwrap_or_else(|| folder_name(component))
}

fn read_manifest(path: &Path) -> Option<Vec<u8>> {
    let file = fs::File::open(path).ok()?;
    let mut text = Vec::new();
    file.take(MANIFEST_LIMIT + 1).read_to_end(&mut text).ok()?;
    (text.len() as u64 <= MANIFEST_LIMIT).then_some(text)
}

fn folder_name(folder: &Path) -> String {
    folder
        .file_name()
        .map(OsStr::to_string_lossy)
        .unwrap_or_default()
        .into_owned()
}

/// A candidate, set aside when it is not a regular file once symbolic links are resolved, or
/// when its version does not read as numbers.
fn check_file(path: PathBuf, version: Option<String>) -> Candidate {
    let set_aside = match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => match &version {
            Some(version) if version_parts(version).is_none() => Some(format!(
                "its version \"{version}\" does not read as numbers"
            )),
            _ => None,
        },
        Ok(metadata) if metadata.is_dir() => Some("a folder, not a file".to_owned()),
        Ok(_) => Some("not a regular file".to_owned()),
        Err(_) if fs::symlink_metadata(&path).is_ok() => {
            Some("a symbolic link to nothing".to_owned())
        }
        Err(error) => Some(format!("cannot be read: {error}")),
    };
    Candidate {
        path,
        version,
        set_aside,
    }
}
