//! Android has no browser CDM to borrow: the system's MediaDrm decrypts
//! there, through the player. These stand in so that the code using the
//! desktop's CDM still builds, and fail if called.

use std::path::Path;

use crate::{Error, Result};

pub fn shutdown() {}

#[derive(Debug, Clone, Copy)]
pub struct Cdm(());

impl Cdm {
    pub fn open(path: &Path) -> Result<Self> {
        Err(Error::Load(path.to_owned()))
    }

    pub fn start_session(&self, _pssh: &[u8]) -> Result<(Session, Vec<u8>)> {
        Err(Error::Init)
    }
}

#[derive(Debug)]
pub struct Session(());

impl Session {
    pub fn update(&self, _license: &[u8]) -> Result<()> {
        Err(Error::LicenseRejected)
    }

    pub fn decrypt(
        &self,
        _data: &[u8],
        _key_id: &[u8],
        _iv: &[u8],
        _subsamples: &[(u32, u32)],
    ) -> Result<Vec<u8>> {
        Err(Error::NoKey)
    }

    pub fn decrypt_cbcs(
        &self,
        _data: &[u8],
        _key_id: &[u8],
        _iv: &[u8],
        _pattern: (u32, u32),
        _subsamples: &[(u32, u32)],
    ) -> Result<Vec<u8>> {
        Err(Error::NoKey)
    }
}
