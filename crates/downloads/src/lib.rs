//! Songs downloaded to play offline, kept sealed so that only Opium plays
//! them.

#[cfg(feature = "keyring")]
pub mod keyring;
pub mod queue;
pub mod sealed;
pub mod store;
