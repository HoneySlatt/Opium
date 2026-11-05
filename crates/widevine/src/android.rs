//! Android (§10): the system's MediaDrm decrypts inside the player, so the same types exist but
//! there is no CDM to load, and no C++ is built.

use std::path::{Path, PathBuf};

use crate::{Candidate, Error, Protected};

/// Finds no CDM: there is none on this platform.
pub fn find_cdm() -> Option<PathBuf> {
    None
}

/// Lists no candidate: there is none on this platform.
pub fn cdm_candidates() -> Vec<Candidate> {
    Vec::new()
}

/// Stands for the desktop's handle to a CDM instance; none can be made here.
#[derive(Debug, Clone, Copy)]
pub struct Cdm {
    _private: (),
}

impl Cdm {
    /// Always fails: no CDM on this platform.
    ///
    /// # Errors
    ///
    /// [`Error::NoCdmOnThisPlatform`], always.
    pub fn load(_path: &Path) -> Result<Cdm, Error> {
        Err(Error::NoCdmOnThisPlatform)
    }

    /// Always fails: no CDM on this platform.
    ///
    /// # Errors
    ///
    /// [`Error::NoCdmOnThisPlatform`], always.
    pub fn open_session(&self, _init_data: &[u8]) -> Result<(Session, Vec<u8>), Error> {
        Err(Error::NoCdmOnThisPlatform)
    }
}

/// Stands for the desktop's session; none can be made here.
#[derive(Debug)]
pub struct Session {
    _private: (),
}

impl Session {
    /// Always fails: no CDM on this platform.
    ///
    /// # Errors
    ///
    /// [`Error::NoCdmOnThisPlatform`], always.
    pub fn give_licence(&self, _licence: &[u8], _key_id: &[u8; 16]) -> Result<(), Error> {
        Err(Error::NoCdmOnThisPlatform)
    }

    /// Always fails: no CDM on this platform.
    ///
    /// # Errors
    ///
    /// [`Error::NoCdmOnThisPlatform`], always.
    pub fn decrypt(&self, _data: &mut [u8], _sample: &Protected<'_>) -> Result<(), Error> {
        Err(Error::NoCdmOnThisPlatform)
    }
}

/// Does nothing: there is no CDM to shut down.
pub fn shutdown() {}
