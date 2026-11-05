//! Finding the CDM (§1) in a folder tree built in the test: each browser's layout, Flatpak's,
//! Firefox's, versions compared as numbers, a broken symbolic link, a folder in place of the
//! file, and malformed manifests.

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use super::{Rng, TestFolder};
use crate::Candidate;
use crate::find::{Places, candidates_in, choose, compare_versions, version_parts};
use crate::manifest::manifest_version;

const PLATFORM: &str = "linux_x64";

/// A machine's folders inside a test folder.
struct Machine {
    folder: TestFolder,
}

impl Machine {
    fn new(name: &str) -> Machine {
        Machine {
            folder: TestFolder::new(name),
        }
    }

    fn home(&self) -> PathBuf {
        self.folder.path().join("home")
    }

    fn config(&self) -> PathBuf {
        self.home().join(".config")
    }

    fn chrome_install(&self) -> PathBuf {
        self.folder.path().join("opt/google/chrome")
    }

    fn places(&self) -> Places {
        Places {
            explicit: None,
            config: Some(self.config()),
            home: Some(self.home()),
            chrome_install: self.chrome_install(),
            platform: Some(PLATFORM),
        }
    }

    /// A Chromium component under `user_data`, with a manifest naming `manifest_version` when
    /// given; the library's path.
    fn chromium(&self, user_data: &Path, folder: &str, manifest_version: Option<&str>) -> PathBuf {
        let component = user_data.join("WidevineCdm").join(folder);
        if let Some(version) = manifest_version {
            write(
                &component.join("manifest.json"),
                format!(r#"{{"name": "WidevineCdm", "version": "{version}"}}"#).as_bytes(),
            );
        }
        let library = component
            .join("_platform_specific")
            .join(PLATFORM)
            .join("libwidevinecdm.so");
        write(&library, b"not loaded by any test");
        library
    }

    fn firefox(&self, profiles: &Path, profile: &str, version: &str) -> PathBuf {
        let library = profiles
            .join(profile)
            .join("gmp-widevinecdm")
            .join(version)
            .join("libwidevinecdm.so");
        write(&library, b"not loaded by any test");
        library
    }
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn find<'a>(candidates: &'a [Candidate], path: &Path) -> &'a Candidate {
    candidates
        .iter()
        .find(|candidate| candidate.path == path)
        .unwrap_or_else(|| panic!("{} was not considered: {candidates:#?}", path.display()))
}

fn usable(candidates: &[Candidate], path: &Path, version: &str) {
    let candidate = find(candidates, path);
    assert_eq!(candidate.version.as_deref(), Some(version), "{candidate:?}");
    assert_eq!(candidate.set_aside, None, "{candidate:?}");
}

fn set_aside(candidates: &[Candidate], path: &Path, because: &str) {
    let candidate = find(candidates, path);
    let reason = candidate
        .set_aside
        .as_deref()
        .unwrap_or_else(|| panic!("{candidate:?} was not set aside"));
    assert!(reason.contains(because), "{candidate:?}");
}

#[test]
fn every_browsers_layout_is_found_and_the_highest_version_wins() {
    let machine = Machine::new("layouts");
    let config = machine.config();
    let home = machine.home();

    let brave = machine.chromium(
        &config.join("BraveSoftware/Brave-Browser"),
        "4.10.2830.0",
        Some("4.10.2830.0"),
    );
    let brave_beta = machine.chromium(
        &config.join("BraveSoftware/Brave-Browser-Beta"),
        "4.10.2891.0",
        None,
    );
    let chromium = machine.chromium(&config.join("chromium"), "4.9.99.0", None);
    let chrome = machine.chromium(
        &config.join("google-chrome"),
        "4.10.3112.0",
        Some("4.10.3112.0"),
    );
    let edge = machine.chromium(
        &config.join("microsoft-edge-dev"),
        "1.0.0.0",
        Some("4.10.100.0"),
    );
    let opera = machine.chromium(&config.join("opera-beta"), "4.10.5.0", None);
    let vivaldi = machine.chromium(&config.join("vivaldi"), "latest", None);
    let snapshot = machine.chromium(
        &config.join("vivaldi-snapshot"),
        "4.10.7.0",
        Some("not.a.version"),
    );
    let installed = machine.chromium(&machine.chrome_install(), "", None);
    write(
        &machine.chrome_install().join("WidevineCdm/manifest.json"),
        br#"{"version": "4.10.3000.0"}"#,
    );
    let flatpak = machine.chromium(
        &home.join(".var/app/com.google.Chrome/config/google-chrome"),
        "4.10.6.0",
        None,
    );
    let firefox = machine.firefox(
        &home.join(".mozilla/firefox"),
        "abcd.default-release",
        "4.10.2934.0",
    );
    let flatpak_firefox = machine.firefox(
        &home.join(".var/app/org.mozilla.firefox/.mozilla/firefox"),
        "efgh.default",
        "4.10.8.0",
    );
    // A browser that is not in the list, and another processor's folder: neither is looked at.
    machine.chromium(&config.join("some-other-browser"), "9.0.0.0", None);
    write(
        &config
            .join("chromium/WidevineCdm/9.0.0.0/_platform_specific/linux_arm64/libwidevinecdm.so"),
        b"",
    );

    let candidates = candidates_in(&machine.places());
    usable(&candidates, &brave, "4.10.2830.0");
    usable(&candidates, &brave_beta, "4.10.2891.0");
    usable(&candidates, &chromium, "4.9.99.0");
    usable(&candidates, &chrome, "4.10.3112.0");
    usable(&candidates, &edge, "4.10.100.0");
    usable(&candidates, &opera, "4.10.5.0");
    usable(&candidates, &installed, "4.10.3000.0");
    usable(&candidates, &flatpak, "4.10.6.0");
    usable(&candidates, &firefox, "4.10.2934.0");
    usable(&candidates, &flatpak_firefox, "4.10.8.0");
    set_aside(&candidates, &vivaldi, "does not read as numbers");
    set_aside(&candidates, &snapshot, "does not read as numbers");
    set_aside(
        &candidates,
        &config.join("chromium/WidevineCdm/9.0.0.0/_platform_specific/linux_x64/libwidevinecdm.so"),
        "cannot be read",
    );
    assert!(!candidates.iter().any(|candidate| {
        candidate
            .path
            .to_string_lossy()
            .contains("some-other-browser")
    }));
    assert!(
        !candidates
            .iter()
            .any(|candidate| candidate.path.to_string_lossy().contains("linux_arm64"))
    );
    assert_eq!(choose(&candidates), Some(chrome));
}

#[test]
fn versions_compare_as_numbers_part_by_part() {
    let machine = Machine::new("numbers");
    let older = machine.chromium(&machine.config().join("chromium"), "4.9.12.0", None);
    let newer = machine.chromium(&machine.config().join("google-chrome"), "4.10.3.0", None);
    let candidates = candidates_in(&machine.places());
    assert_eq!(choose(&candidates), Some(newer));
    assert!(candidates.iter().any(|candidate| candidate.path == older));

    assert_eq!(version_parts("4.10.3112.0"), Some(vec![4, 10, 3112, 0]));
    for bad in [
        "",
        "4..1",
        "v4.1",
        "4.1a",
        "-1",
        "4.",
        ".4",
        "4 .1",
        "99999999999999999999",
    ] {
        assert_eq!(version_parts(bad), None, "{bad}");
    }
    assert_eq!(
        compare_versions(&[4, 9, 12, 0], &[4, 10, 3, 0]),
        Ordering::Less
    );
    assert_eq!(compare_versions(&[4, 10], &[4, 10, 0]), Ordering::Equal);
    assert_eq!(compare_versions(&[4, 10, 1], &[4, 10]), Ordering::Greater);
}

#[test]
fn between_equal_versions_the_first_path_in_byte_order_wins() {
    let machine = Machine::new("equal");
    let chrome = machine.chromium(&machine.config().join("google-chrome"), "4.10.0", None);
    let chromium = machine.chromium(&machine.config().join("chromium"), "4.10.0.0", None);
    let candidates = candidates_in(&machine.places());
    assert!(chromium < chrome);
    assert_eq!(choose(&candidates).as_ref(), Some(&chromium));
    let reversed: Vec<Candidate> = candidates.into_iter().rev().collect();
    assert_eq!(
        choose(&reversed),
        Some(chromium),
        "whatever the order found"
    );
}

#[test]
fn a_broken_link_and_a_folder_are_set_aside_and_a_good_link_is_followed() {
    let machine = Machine::new("links");
    let config = machine.config();
    let broken =
        config.join("chromium/WidevineCdm/5.0.0.0/_platform_specific/linux_x64/libwidevinecdm.so");
    fs::create_dir_all(broken.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(machine.folder.path().join("nowhere"), &broken).unwrap();
    let folder = config
        .join("google-chrome/WidevineCdm/6.0.0.0/_platform_specific/linux_x64/libwidevinecdm.so");
    fs::create_dir_all(&folder).unwrap();
    let elsewhere = machine.folder.path().join("elsewhere/libwidevinecdm.so");
    write(&elsewhere, b"");
    let linked =
        config.join("vivaldi/WidevineCdm/4.0.0.0/_platform_specific/linux_x64/libwidevinecdm.so");
    fs::create_dir_all(linked.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &linked).unwrap();

    let candidates = candidates_in(&machine.places());
    set_aside(&candidates, &broken, "symbolic link to nothing");
    set_aside(&candidates, &folder, "a folder");
    usable(&candidates, &linked, "4.0.0.0");
    assert_eq!(choose(&candidates), Some(linked));
}

#[test]
fn the_variable_names_the_one_file_to_use() {
    let machine = Machine::new("explicit");
    machine.chromium(&machine.config().join("google-chrome"), "4.10.3112.0", None);
    let chosen = machine.folder.path().join("my/libwidevinecdm.so");
    write(&chosen, b"");
    let places = Places {
        explicit: Some(chosen.clone()),
        ..machine.places()
    };
    let candidates = candidates_in(&places);
    assert_eq!(
        candidates,
        vec![Candidate {
            path: chosen.clone(),
            version: None,
            set_aside: None
        }]
    );
    assert_eq!(choose(&candidates), Some(chosen));

    let missing = Places {
        explicit: Some(machine.folder.path().join("missing.so")),
        ..machine.places()
    };
    let candidates = candidates_in(&missing);
    assert_eq!(candidates.len(), 1);
    assert!(candidates[0].set_aside.is_some());
    assert_eq!(choose(&candidates), None);
}

#[test]
fn nothing_found_without_folders() {
    let machine = Machine::new("empty");
    assert!(candidates_in(&machine.places()).is_empty());
    let nowhere = Places {
        explicit: None,
        config: None,
        home: None,
        chrome_install: machine.chrome_install(),
        platform: None,
    };
    assert!(candidates_in(&nowhere).is_empty());
    assert_eq!(choose(&[]), None);
}

/// A manifest made worse: bytes flipped, cut, repeated or inserted.
fn mutate(rng: &mut Rng, text: &[u8]) -> Vec<u8> {
    let mut text = text.to_vec();
    for _ in 0..=rng.below(4) {
        let at = rng.below(text.len() as u64 + 1) as usize;
        match rng.below(5) {
            0 if at < text.len() => text[at] ^= 1 << rng.below(8),
            1 => text.truncate(at),
            2 => {
                let end = (at + rng.below(16) as usize).min(text.len());
                let piece = text[at..end].to_vec();
                for _ in 0..rng.below(64) {
                    text.splice(at..at, piece.iter().copied());
                }
            }
            3 => text.insert(
                at,
                *b"{}[]\":,\\ 9."
                    .get(rng.below(12) as usize)
                    .unwrap_or(&b'"'),
            ),
            _ => text.insert(at, rng.byte()),
        }
    }
    text
}

const MANIFEST: &[u8] = br#"{
  "manifest_version": 2,
  "update_url": "https://example.invalid/update",
  "name": "WidevineCdm",
  "description": "Widevine Content Decryption Module",
  "offline_enabled": false,
  "x-cdm-module-versions": "4",
  "x-cdm-interface-versions": "10,11",
  "x-cdm-codecs": "vp8,vp09,avc1,av01",
  "platforms": [{"os": "linux", "arch": "x64", "sub_package_path": "_platform_specific/linux_x64/"}],
  "nested": {"version": "0.0.0.1", "list": [1, 2.5e3, true, null, "a\"b"]},
  "version": "4.10.2830.0"
}"#;

#[test]
fn malformed_manifests_never_panic_the_reader() {
    assert_eq!(manifest_version(MANIFEST), Some("4.10.2830.0"));
    let mut rng = Rng::new(0x5eed_0001);
    for _ in 0..50_000 {
        let text = mutate(&mut rng, MANIFEST);
        if let Some(version) = manifest_version(&text).filter(|version| !version.is_empty()) {
            assert!(
                text.windows(version.len())
                    .any(|window| window == version.as_bytes())
            );
        }
    }
    for _ in 0..5_000 {
        let length = rng.below(64) as usize;
        let noise: Vec<u8> = (0..length)
            .map(|_| *b"{}[]\":,\\ v9.".get(rng.below(12) as usize).unwrap_or(&0))
            .collect();
        let _ = manifest_version(&noise);
    }
}

#[test]
fn malformed_manifests_in_a_tree_never_panic_the_search() {
    let machine = Machine::new("mutations");
    let user_data = machine.config().join("google-chrome");
    let library = machine.chromium(&user_data, "4.10.1.0", Some("4.10.1.0"));
    let manifest = library
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("manifest.json");
    let mut rng = Rng::new(0x5eed_0002);
    for _ in 0..2_000 {
        let text = mutate(&mut rng, MANIFEST);
        fs::write(&manifest, &text).unwrap();
        let candidates = candidates_in(&machine.places());
        let candidate = find(&candidates, &library);
        // The manifest's version when it gives one, else the folder's name.
        let expected = manifest_version(&text).unwrap_or("4.10.1.0");
        assert_eq!(candidate.version.as_deref(), Some(expected));
    }
    fs::write(&manifest, vec![b'['; 100_000]).unwrap();
    assert_eq!(
        find(&candidates_in(&machine.places()), &library)
            .version
            .as_deref(),
        Some("4.10.1.0")
    );
}
