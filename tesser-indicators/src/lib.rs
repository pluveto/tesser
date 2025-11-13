#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

//! High-performance, composable technical indicators built on decimal arithmetic.

/// Indicator composition helpers such as `PipedIndicator`.
pub mod combinators;
/// Foundational traits and shared abstractions.
pub mod core;
/// Built-in indicator implementations.
pub mod indicators;

/// Re-export of the piped indicator combinator for convenience.
pub use crate::combinators::PipedIndicator;
/// Re-export of the core traits and error type to make the crate easy to consume.
pub use crate::core::{Indicator, IndicatorError, Input};
