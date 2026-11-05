//! Driving the browser's Widevine CDM on the desktop, as Chromium's CDM interface describes it.
//!
//! The CDM is Google's `libwidevinecdm.so`, which a browser downloaded: [`find_cdm`] finds it,
//! [`Cdm::load`] loads it and initialises the process's one instance, [`Cdm::open_session`]
//! opens a session and gives its licence request, [`Session::give_licence`] gives it the
//! licence server's answer, and [`Session::decrypt`] decrypts one sample in place. Before the
//! process exits, [`shutdown`] ends it all.
//!
//! One thread, named `cdm`, owns the instance: it makes every call into it, receives every
//! callback and fires the CDM's timers; callers on any thread send it requests and wait. At
//! rest, with no request and no timer due, it sleeps. A thin C++ layer (`shim/`) implements the
//! interface's classes for one version and only passes calls across a C boundary; every
//! decision is made here, in Rust, and knows no interface version.
//!
//! On Android the types exist, but there is no CDM to load: the system's MediaDrm decrypts.

#![warn(missing_docs)]

mod error;
mod pssh;

#[cfg(target_os = "android")]
mod android;
#[cfg(not(target_os = "android"))]
mod boundary;
#[cfg(not(target_os = "android"))]
mod desktop;
#[cfg(not(target_os = "android"))]
mod find;
#[cfg(not(target_os = "android"))]
mod host;
#[cfg(not(target_os = "android"))]
mod library;
#[cfg(not(target_os = "android"))]
mod log;
#[cfg(not(target_os = "android"))]
mod manifest;
#[cfg(not(target_os = "android"))]
mod runtime;
#[cfg(not(target_os = "android"))]
mod timers;

#[cfg(all(test, not(target_os = "android")))]
mod tests;

#[cfg(target_os = "android")]
pub use android::{Cdm, Session, cdm_candidates, find_cdm, shutdown};
#[cfg(not(target_os = "android"))]
pub use desktop::{Cdm, Session, shutdown};
pub use error::Error;
#[cfg(not(target_os = "android"))]
pub use find::{cdm_candidates, find_cdm};
pub use pssh::{SYSTEM_ID, pssh_for_key};

/// A CDM library found on this machine, chosen or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// The library's path, as found (symbolic links not resolved).
    pub path: std::path::PathBuf,
    /// Its version, as read from its manifest or its folder's name, when there is one.
    pub version: Option<String>,
    /// Why it cannot be used, when it cannot: then [`find_cdm`] never returns it.
    pub set_aside: Option<String>,
}

/// One protected sample's decryption inputs, as the file gives them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Protected<'a> {
    /// The key the sample is encrypted with.
    pub key_id: &'a [u8; 16],
    /// The IV, always 16 bytes: an 8-byte IV is followed by eight zero bytes.
    pub iv: &'a [u8; 16],
    /// The subsample runs as (clear, protected) bytes; empty when the whole sample is
    /// protected.
    pub subsamples: &'a [(u32, u32)],
    /// How the protected bytes are encrypted.
    pub scheme: Scheme,
}

/// The encryption schemes the CDM decrypts (ISO/IEC 23001-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scheme {
    /// `cenc`: AES-CTR, no pattern.
    Cenc,
    /// `cbcs`: AES-CBC with a pattern of 16-byte blocks, `crypt` encrypted then `skip` clear.
    Cbcs {
        /// Encrypted blocks in each pattern.
        crypt: u8,
        /// Clear blocks in each pattern.
        skip: u8,
    },
}
