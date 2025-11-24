use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use assert_cmd::prelude::*;
use chrono::{Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use tempfile::tempdir;

use tesser_core::{Candle, Interval};
use tesser_data::io::{self, DatasetFormat};

#[test]
fn resamples_csv_into_parquet() -> Result<()> {
    let temp = tempdir()?;
    let input = temp.path().join("1m_BTCUSDT.csv");
    let output = temp.path().join("resampled.parquet");
    let candles = sample_candles(10);
    io::write_dataset(&input, DatasetFormat::Csv, &candles)?;

    run_resample(&input, &output, "5m", &[])?;

    let dataset = io::read_dataset(&output)?;
    assert_eq!(dataset.candles.len(), 2);
    let first = &dataset.candles[0];
    assert_eq!(first.timestamp, candles[0].timestamp);
    assert_eq!(first.close, candles[4].close);
    let expected_volume: Decimal = candles.iter().take(5).map(|c| c.volume).sum();
    assert_eq!(first.volume, expected_volume);
    Ok(())
}

#[test]
fn converts_parquet_to_csv() -> Result<()> {
    let temp = tempdir()?;
    let input = temp.path().join("candles.parquet");
    let output = temp.path().join("converted.csv");
    let candles = sample_candles(6);
    io::write_dataset(&input, DatasetFormat::Parquet, &candles)?;

    run_resample(&input, &output, "1m", &[])?;

    let dataset = io::read_dataset(&output)?;
    assert_eq!(dataset.candles.len(), candles.len());
    for (converted, expected) in dataset.candles.iter().zip(&candles) {
        assert_eq!(converted.symbol.code(), expected.symbol.code());
        assert_eq!(converted.interval, expected.interval);
        assert_eq!(converted.open, expected.open);
        assert_eq!(converted.high, expected.high);
        assert_eq!(converted.low, expected.low);
        assert_eq!(converted.close, expected.close);
        assert_eq!(converted.volume, expected.volume);
        assert_eq!(converted.timestamp, expected.timestamp);
    }
    Ok(())
}

fn run_resample(input: &Path, output: &Path, interval: &str, extra: &[&str]) -> Result<()> {
    let binary = assert_cmd::cargo::cargo_bin!("tesser-cli");
    let mut cmd = Command::new(binary);
    cmd.current_dir(workspace_root());
    let mut args = vec![
        "--env",
        "default",
        "data",
        "resample",
        "--input",
        input.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--interval",
        interval,
    ];
    args.extend_from_slice(extra);
    cmd.args(args);
    cmd.assert().success();
    Ok(())
}

fn sample_candles(count: usize) -> Vec<Candle> {
    let base = Utc
        .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
        .single()
        .expect("valid base timestamp");
    (0..count)
        .map(|idx| Candle {
            symbol: "BTCUSDT".into(),
            interval: Interval::OneMinute,
            open: Decimal::new(10 + idx as i64, 0),
            high: Decimal::new(11 + idx as i64, 0),
            low: Decimal::new(9 + idx as i64, 0),
            close: Decimal::new(10 + idx as i64, 0),
            volume: Decimal::new(1 + idx as i64, 0),
            timestamp: base + Duration::minutes(idx as i64),
        })
        .collect()
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}
