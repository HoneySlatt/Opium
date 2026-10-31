pub mod color;
pub mod dedupe;
pub mod growing;
pub mod model;
pub mod palette;
pub mod provider;
pub mod queue;
pub mod text;
pub mod theme;

pub use provider::{Error, Provider, Result};
pub use queue::{Queue, Repeat};

#[cfg(feature = "ffi")]
uniffi::setup_scaffolding!();
