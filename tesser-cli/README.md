# tesser-cli

Unified command-line interface for data engineering, backtesting, and live trading.

## Commands
- `data download|validate|resample|normalize`: manage historical datasets (Bybit/Binance klines) and convert them into the canonical Parquet schema.
- `backtest run|batch`: execute one or many strategy configs against canonical Parquet candle files.
- `live run`: connect to the configured exchange public stream, drive strategies in real time, and route signals through the paper execution engine.
- `strategies`: list compiled built-in strategies.

Run `cargo run -p tesser-cli -- --help` for global options and `-- <command> --help` for subcommands.

## Configuration
The CLI loads layered config files from `config/` (default, env-specific, local override) plus environment variables (`TESSER_*`). The `[live]` section controls telemetry (state path, metrics bind, structured log file, alerting thresholds).

## Development
```sh
cargo fmt -p tesser-cli
cargo clippy -p tesser-cli -- -D warnings
cargo test -p tesser-cli
```
