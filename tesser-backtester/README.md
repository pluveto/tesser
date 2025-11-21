# tesser-backtester

Event-driven simulation engine that replays historical data through strategies and the execution stack.

## Overview
- Wires `Strategy`, `ExecutionEngine`, `Portfolio`, and `tesser-paper` into a deterministic loop.
- Supports configurable history length, latency (in candles), slippage, and fees via `BacktestConfig`.
- Produces `BacktestReport` summaries (signals emitted, orders sent, equity, dropped orders).

## Usage
Most users drive the backtester through `tesser-cli backtest run`, but you can embed it yourself:
```rust
let cfg = BacktestConfig::new(symbol.clone());
let market_registry = Arc::new(MarketRegistry::load_from_file("config/markets.toml")?);
let stream: BacktestStream = Box::new(PaperMarketStream::from_data(symbol.clone(), Vec::new(), candles));
let report = Backtester::new(cfg, strategy, execution, None, market_registry, Some(stream), None)
    .run()
    .await?;
```

## Tests
```sh
cargo test -p tesser-backtester
```
