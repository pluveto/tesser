# Examples

This directory contains ready-to-run assets for local experiments:

- `data/btcusdt_1m_sample.parquet` — canonical BTCUSDT candles ready for `tesser-cli backtest run --data`.
- `data/btcusdt_1m_sample.csv` — raw deterministic candles used to regenerate the Parquet sample via `tesser-cli data normalize`.
- `strategies/sma_cross.toml` — sample configuration for the `SmaCross` strategy tuned to the dataset.
- `strategies/rsi_reversion.toml` — sample configuration for the `RsiReversion` strategy.

To run a full backtest without downloading exchange data:

```sh
cargo run -p tesser-cli -- \
  backtest run \
  --strategy-config examples/strategies/sma_cross.toml \
  --data examples/data/btcusdt_1m_sample.parquet \
  --quantity 0.01
```

To rebuild the sample dataset (or normalize your own CSVs) use:

```sh
cargo run -p tesser-cli -- data normalize \
  --source examples/data/btcusdt_1m_sample.csv \
  --output examples/data/lake_sample \
  --config configs/etl/sample_iso_csv.toml \
  --symbol BTCUSDT
```

Then pass any `.parquet` outputs to `--data`.

Feel free to duplicate these files when crafting new experiments; the CLI only requires that each strategy config define a `strategy_name` plus a `[params]` table.*** End Patch
