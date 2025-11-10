# Research Environment

This directory hosts all Python-based research workflows:

- `notebooks/`: Jupyter or VS Code notebooks for exploratory data analysis.
- `scripts/`: Reusable Python scripts (feature generation, parameter sweeps, ML training).
- `strategies/`: Serialized outputs such as TOML parameter files that Rust consumers load (sample configs ship in this repo).
- `models/`: Artifacts produced by ML pipelines (e.g., the linear momentum model consumed by `MlClassifier`).

## Quick Start

```bash
cd research
uv venv
source .venv/bin/activate
uv pip install -e .
```

Once the environment is ready, you can open notebooks or run scripts:

```bash
uv run python scripts/find_optimal_sma.py --data ../data/btc.parquet
uv run python scripts/optimize_rsi.py --data ../data/btc.parquet --output strategies/rsi_from_python.toml
uv run python scripts/train_ml_classifier.py --data ../data/btc.parquet --output models/ml_linear.toml
uv run python scripts/walk_forward_sma.py --data ../data/btc.parquet --summary-csv summaries/sma_walk_forward.csv
```

Store generated strategy configs under `strategies/` (see `sma_cross.toml`, `rsi_reversion.toml`, etc.) and drop ML artifacts under `models/`. The Rust CLI consumes these files directly via `--strategy-config` (and the referenced `model_path`).

The walk-forward script evaluates SMA crossover parameters on rolling train/test windows, writes a per-window CSV summary, and emits a ready-to-use TOML config that reflects the best-performing parameters across the out-of-sample slices.

## Shipping Multiple Strategies

Each TOML file under `strategies/` can drive its own `tesser-cli live run` process. When you're ready to graduate a model from research to production:

1. Commit the generated config (e.g., `strategies/alpha_momo.toml`).
2. Launch a dedicated CLI instance with per-strategy overrides:

    ```sh
    tesser-cli --env prod live run \
      --strategy-config strategies/alpha_momo.toml \
      --state-path reports/alpha_momo.db \
      --metrics-addr 0.0.0.0:9300 \
      --log-path logs/alpha_momo.json \
      --initial-equity 25000 \
      --risk-max-order-qty 0.4 \
      --risk-max-position-qty 0.8
    ```

3. Repeat for each additional strategy (Docker Compose/systemd examples are documented in the top-level `README.md`).

This multi-process model keeps failure domains small—one misbehaving strategy can be restarted or rolled back without touching the others.
