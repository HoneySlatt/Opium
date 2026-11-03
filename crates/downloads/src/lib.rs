//! Songs downloaded to play offline, kept sealed so that only Music plays
//! them.

#[cfg(feature = "keyring")]
pub mod keyring;
pub mod sealed;
