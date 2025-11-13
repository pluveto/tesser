# tesser-indicators

`tesser-indicators` is the high-precision analytics engine that powers the Tesser strategy stack. Every indicator runs on `rust_decimal::Decimal` to eliminate floating-point drift, updates in **O(1)** time, and can be chained together through a lightweight `pipe()` API.

## Highlights

- **Decimal-first arithmetic** – all math is performed with `Decimal`, keeping indicator output stable even across millions of updates.
- **Generic inputs** – anything that implements the `Input` trait (`f64`, `Decimal`, `tesser_core::Candle`, etc.) can be fed into an indicator.
- **Composable by design** – the `Indicator` trait exposes a `pipe()` helper that connects two indicators without runtime allocation.
- **Battle-tested cores** – SMA, EMA, RSI, and Bollinger Bands ship with exhaustive unit tests covering warm-ups, resets, and steady-state calculations.

## Quick Start

```rust
use rust_decimal::Decimal;
use tesser_indicators::indicators::{Rsi, Sma};
use tesser_indicators::Indicator;

// Smoothing a momentum signal with a moving average
let mut rsi = Rsi::new(14).unwrap();
let mut smoothed = rsi.pipe(Sma::new(5).unwrap());

for price in [101.5, 102.0, 101.2, 103.3, 104.8] {
    if let Some(value) = smoothed.next(price) {
        println!("Smoothed RSI = {value}");
    }
}
```

## Available Indicators

- `Sma` – Simple moving average backed by a rolling accumulator.
- `Ema` – Wilder-style exponential moving average with constant-time updates.
- `Rsi` – Relative Strength Index that mirrors the default TradingView behaviour.
- `BollingerBands` – SMA + population standard deviation with configurable multipliers.

New indicators should live in the `src/indicators` module directory, implement the shared `Indicator` trait, and include exhaustive tests.

## Contributing

- Keep indicator updates strictly `O(1)` by using incremental statistics instead of rescanning history.
- Always prefer `Decimal` internals. Convert from `f64` **only** at the ingestion boundary using the provided `Input` trait.
- Add regression tests that cover warm-up behaviour, steady-state accuracy, and `reset()` semantics.
