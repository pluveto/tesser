//! Shared SDK primitives for developing WebAssembly execution plugins.

mod types;
pub use types::*;

#[cfg(feature = "guest")]
pub mod guest;

#[cfg(feature = "guest")]
pub use guest::{ExecutionPlugin, PluginError};
