//! Built-in indicator implementations provided by the crate.

/// Average True Range indicator module.
pub mod atr;
pub mod bollinger;
pub mod ema;
/// Ichimoku Cloud indicator module.
pub mod ichimoku;
/// Moving Average Convergence Divergence module.
pub mod macd;
pub mod rsi;
pub mod sma;

pub use atr::Atr;
pub use bollinger::{BollingerBands, BollingerBandsOutput};
pub use ema::Ema;
pub use ichimoku::{Ichimoku, IchimokuOutput};
pub use macd::{Macd, MacdOutput};
pub use rsi::Rsi;
pub use sma::Sma;
