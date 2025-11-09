"""Walk-forward optimization for the SMA crossover strategy."""

from __future__ import annotations

import argparse
from collections import defaultdict
from pathlib import Path
from typing import Iterable

import numpy as np
import pandas as pd
import toml


def load_close_series(path: Path) -> pd.Series:
    if path.suffix == ".parquet":
        df = pd.read_parquet(path)
    elif path.suffix == ".csv":
        df = pd.read_csv(path)
    else:
        raise ValueError(f"Unsupported extension: {path.suffix}")

    if "close" not in df:
        raise ValueError("Input data must contain a 'close' column")

    close = df["close"].astype(float)
    if "timestamp" in df:
        close.index = pd.to_datetime(df["timestamp"])
    return close


def sma_return(close: pd.Series, fast: int, slow: int) -> float:
    if fast >= slow:
        return float("-inf")
    fast_ma = close.rolling(fast).mean()
    slow_ma = close.rolling(slow).mean()
    signal = np.where(fast_ma > slow_ma, 1.0, -1.0)
    signal_series = pd.Series(signal, index=close.index).shift(1).fillna(0.0)
    returns = close.pct_change().fillna(0.0)
    pnl = (signal_series * returns).cumsum()
    return float(pnl.iloc[-1])


def parse_range(values: str) -> list[int]:
    parts = [int(value.strip()) for value in values.split(",") if value.strip()]
    if not parts:
        raise ValueError("At least one value must be provided")
    return parts


def walk_forward(
    close: pd.Series,
    train_window: int,
    test_window: int,
    fast_values: Iterable[int],
    slow_values: Iterable[int],
) -> list[dict[str, object]]:
    results: list[dict[str, object]] = []
    end = len(close)
    step = test_window

    if end < train_window + test_window:
        raise ValueError("Not enough data for the requested windows")

    window_index = 0
    for start in range(0, end - train_window - test_window + 1, step):
        train_slice = close.iloc[start : start + train_window]
        test_slice = close.iloc[start + train_window : start + train_window + test_window]
        best_score = float("-inf")
        best_params = None

        for fast in fast_values:
            for slow in slow_values:
                if slow <= fast:
                    continue
                score = sma_return(train_slice, fast, slow)
                if score > best_score:
                    best_score = score
                    best_params = (fast, slow)

        if best_params is None:
            continue
        fast, slow = best_params
        test_score = sma_return(test_slice, fast, slow)

        results.append(
            {
                "window": window_index,
                "train_start": train_slice.index[0],
                "train_end": train_slice.index[-1],
                "test_start": test_slice.index[0],
                "test_end": test_slice.index[-1],
                "fast": fast,
                "slow": slow,
                "train_score": best_score,
                "test_score": test_score,
            }
        )
        window_index += 1

    return results


def aggregate_best_params(results: list[dict[str, object]]) -> tuple[int, int]:
    scores: defaultdict[tuple[int, int], list[float]] = defaultdict(list)
    for row in results:
        key = (int(row["fast"]), int(row["slow"]))
        scores[key].append(float(row["test_score"]))
    best_pair = max(scores.items(), key=lambda item: np.mean(item[1]))[0]
    return best_pair


def main() -> None:
    parser = argparse.ArgumentParser(description="Walk-forward optimizer for SMA crossover")
    parser.add_argument("--data", type=Path, required=True, help="Path to CSV/Parquet data")
    parser.add_argument("--symbol", default="BTCUSDT")
    parser.add_argument("--train-window", type=int, default=200)
    parser.add_argument("--test-window", type=int, default=50)
    parser.add_argument("--fast-range", default="5,10,15,20")
    parser.add_argument("--slow-range", default="20,30,40,50")
    parser.add_argument("--summary-csv", type=Path, help="Optional output CSV for per-window stats")
    parser.add_argument(
        "--config-output",
        type=Path,
        default=Path("strategies/sma_walk_forward.toml"),
        help="Location to write the resulting strategy config",
    )
    args = parser.parse_args()

    close = load_close_series(args.data)
    fast_values = parse_range(args.fast_range)
    slow_values = parse_range(args.slow_range)

    results = walk_forward(close, args.train_window, args.test_window, fast_values, slow_values)
    if not results:
        raise RuntimeError("Walk-forward run produced no valid windows")

    if args.summary_csv:
        args.summary_csv.parent.mkdir(parents=True, exist_ok=True)
        pd.DataFrame(results).to_csv(args.summary_csv, index=False)

    best_fast, best_slow = aggregate_best_params(results)
    config = {
        "strategy_name": "SmaCross",
        "params": {
            "symbol": args.symbol,
            "fast_period": best_fast,
            "slow_period": best_slow,
            "min_samples": max(best_slow + 5, args.train_window // 10),
        },
    }
    if args.config_output:
        args.config_output.parent.mkdir(parents=True, exist_ok=True)
        args.config_output.write_text(toml.dumps(config))

    avg_test = np.mean([row["test_score"] for row in results])
    print(f"Completed {len(results)} walk-forward windows")
    print(f"Average test score: {avg_test:.4f}")
    print(f"Selected params -> fast={best_fast}, slow={best_slow}")
    if args.config_output:
        print(f"Config written to {args.config_output}")
    if args.summary_csv:
        print(f"Summary CSV written to {args.summary_csv}")


if __name__ == "__main__":
    main()
