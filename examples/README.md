# Examples

This directory contains ready-to-run assets for local experiments:

- `data/btcusdt_1m_sample.csv` — deterministic BTCUSDT 1-minute candles covering six hours. Each row matches the schema consumed by `tesser-cli backtest run --data`: `symbol,timestamp,open,high,low,close,volume`.
- `strategies/sma_cross.toml` — sample configuration for the `SmaCross` strategy tuned to the dataset.
- `strategies/rsi_reversion.toml` — sample configuration for the `RsiReversion` strategy.

To run a full backtest without downloading exchange data:

```sh
cargo run -p tesser-cli -- \
  backtest run \
  --strategy-config examples/strategies/sma_cross.toml \
  --data examples/data/btcusdt_1m_sample.csv \
  --quantity 0.01
```

Feel free to duplicate these files when crafting new experiments; the CLI only requires that each strategy config define a `strategy_name` plus a `[params]` table.*** End Patch
