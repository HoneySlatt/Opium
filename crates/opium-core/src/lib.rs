pub mod color;
pub mod cover;
pub mod dedupe;
pub mod files;
pub mod growing;
pub mod model;
pub mod palette;
pub mod provider;
pub mod queue;
pub mod recent;
pub mod text;
pub mod theme;

pub use provider::{Error, Provider, Result};
pub use queue::{Handoff, Queue, Repeat};

#[cfg(feature = "ffi")]
uniffi::setup_scaffolding!();
