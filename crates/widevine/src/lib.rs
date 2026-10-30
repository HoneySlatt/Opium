//! A Widevine CDM borrowed from an installed browser, driven through its official
//! `cdm::ContentDecryptionModule_11` interface (see `shim/widevine_shim.cc`).
//!
//! The CDM is Google's proprietary library and is never shipped with this app.
//! Adapted from Kopuz (<https://github.com/Kopuz-org/kopuz>, revision a58c385,
//! `daemon/server/src/applemusic/widevine/`), Copyright (c) 2026 Temidaradev and
//! the Kopuz contributors, EUPL-1.2, redistributed under the GPL-3.0-or-later
//! through the EUPL's compatibility clause.

use std::os::raw::c_int;
use std::path::{Path, PathBuf};

/// Widevine's DRM system id, `edef8ba9-79d6-4ace-a3c8-27dcd51d21ed`.
pub const SYSTEM_ID: [u8; 16] = [
    0xed, 0xef, 0x8b, 0xa9, 0x79, 0xd6, 0x4a, 0xce, 0xa3, 0xc8, 0x27, 0xdc, 0xd5, 0x1d, 0x21, 0xed,
];

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("couldn't load the Widevine CDM at {0}")]
    Load(PathBuf),
    #[error("{0} isn't a Widevine CDM")]
    NotACdm(PathBuf),
    #[error("this Widevine CDM doesn't support the CDM-11 interface")]
    Interface,
    #[error("the Widevine CDM failed to initialize")]
    Init,
    #[error(
        "the Widevine CDM has no device certificate yet: play any DRM video once in the browser it came from"
    )]
    Unprovisioned,
    #[error("the CDM rejected the pssh box")]
    BadInitData,
    #[error("the CDM rejected the licence")]
    LicenseRejected,
    #[error("the licence carried no key for this content")]
    NoKey,
    #[error("Widevine {0} failed (code {1})")]
    Call(&'static str, c_int),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(not(target_os = "android"))]
mod native;
#[cfg(not(target_os = "android"))]
pub use native::{Cdm, Session, shutdown};

#[cfg(target_os = "android")]
mod stub;
#[cfg(target_os = "android")]
pub use stub::{Cdm, Session, shutdown};

/// The `WidevineCencHeader` protobuf for one key id, written by hand: field 1
/// `algorithm` = AESCTR, field 2 `key_id`, then empty `provider` (3) and
/// `policy` (6), in tag order as protobuf encoders write them.
fn cenc_header(key_id: &[u8]) -> Vec<u8> {
    let mut out = vec![0x08, 0x01, 0x12];
    let mut len = key_id.len();
    while len >= 0x80 {
        out.push(len as u8 | 0x80);
        len >>= 7;
    }
    out.push(len as u8);
    out.extend_from_slice(key_id);
    out.extend_from_slice(&[0x1a, 0x00, 0x32, 0x00]);
    out
}

/// A version-0 `pssh` box for `key_id`: the init data a CDM session starts from.
pub fn build_pssh(key_id: &[u8]) -> Vec<u8> {
    let payload = cenc_header(key_id);
    let total = 32 + payload.len();
    let mut pssh = Vec::with_capacity(total);
    pssh.extend_from_slice(&(total as u32).to_be_bytes());
    pssh.extend_from_slice(b"pssh");
    pssh.extend_from_slice(&[0, 0, 0, 0]);
    pssh.extend_from_slice(&SYSTEM_ID);
    pssh.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    pssh.extend_from_slice(&payload);
    pssh
}

const CDM_FILE: &str = "libwidevinecdm.so";

/// Find a CDM: `$WIDEVINE_CDM` if set, else one a Chromium-family browser or
/// Firefox has downloaded into the user's profile.
pub fn locate() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("WIDEVINE_CDM") {
        return Some(PathBuf::from(path));
    }
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));

    // Chromium keeps the component at `<user data>/WidevineCdm/<version>/
    // _platform_specific/linux_x64/`; Brave nests its user data one level down.
    let chromium = ["google-chrome", "chromium", "BraveSoftware/*", "vivaldi"]
        .iter()
        .flat_map(|root| {
            expand(
                &config,
                &format!("{root}/WidevineCdm/*/_platform_specific/linux_x64"),
            )
        });
    // Firefox: `<profile>/gmp-widevinecdm/<version>/`.
    let firefox = expand(&home, ".mozilla/firefox/*/gmp-widevinecdm/*");

    chromium
        .chain(firefox)
        .map(|dir| dir.join(CDM_FILE))
        .find(|path| path.is_file())
}

/// The directories matching `pattern` under `base`, where a `*` component matches
/// any entry. Newest-looking names first, so a later CDM version wins.
fn expand(base: &Path, pattern: &str) -> Vec<PathBuf> {
    let mut dirs = vec![base.to_owned()];
    for part in pattern.split('/') {
        dirs = dirs
            .into_iter()
            .flat_map(|dir| {
                if part != "*" {
                    return vec![dir.join(part)];
                }
                let mut children: Vec<PathBuf> = std::fs::read_dir(&dir)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .collect();
                children.sort_by(|a, b| b.cmp(a));
                children
            })
            .collect();
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pinned to the bytes prost emits for the same `WidevineCencHeader`.
    #[test]
    fn the_cenc_header_is_protobuf() {
        assert_eq!(
            cenc_header(&[0xAA; 4]),
            [
                0x08, 0x01, 0x12, 0x04, 0xAA, 0xAA, 0xAA, 0xAA, 0x1a, 0x00, 0x32, 0x00
            ]
        );
    }

    #[test]
    fn a_long_key_id_gets_a_multibyte_length() {
        assert_eq!(&cenc_header(&[0u8; 200])[2..5], &[0x12, 0xC8, 0x01]);
    }

    #[test]
    fn the_pssh_box_is_well_formed() {
        let pssh = build_pssh(&[0x11; 16]);
        assert_eq!(
            u32::from_be_bytes(pssh[..4].try_into().unwrap()) as usize,
            pssh.len()
        );
        assert_eq!(&pssh[4..8], b"pssh");
        assert_eq!(&pssh[12..28], &SYSTEM_ID);
        assert_eq!(
            u32::from_be_bytes(pssh[28..32].try_into().unwrap()) as usize,
            pssh.len() - 32
        );
    }

    #[test]
    fn expand_matches_wildcards_newest_first() {
        let base = std::env::temp_dir().join(format!("widevine-expand-{}", std::process::id()));
        for v in ["4.10.1", "4.10.2"] {
            std::fs::create_dir_all(base.join("Brave/WidevineCdm").join(v)).unwrap();
        }
        let found = expand(&base, "*/WidevineCdm/*");
        assert_eq!(found[0], base.join("Brave/WidevineCdm/4.10.2"));
        assert_eq!(found.len(), 2);
        std::fs::remove_dir_all(base).unwrap();
    }
}
