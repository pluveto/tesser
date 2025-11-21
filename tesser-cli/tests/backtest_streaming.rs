use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use assert_cmd::prelude::*;
use chrono::{TimeZone, Utc};
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use rust_decimal::Decimal;
use tempfile::tempdir;

use tesser_core::{Side, Tick};
use tesser_data::encoding::ticks_to_batch;

const STRATEGY_CONFIG: &str = r#"
strategy_name = "SmaCross"

[params]
symbol = "BTCUSDT"
fast_period = 3
slow_period = 5
min_samples = 5
"#;

#[test]
fn backtest_ticks_stream_from_parquet() -> Result<()> {
    let temp = tempdir()?;
    let strategy_path = temp.path().join("strategy.toml");
    fs::write(&strategy_path, STRATEGY_CONFIG)?;

    let flight_dir = temp.path().join("flight");
    write_ticks_dataset(&flight_dir)?;

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let markets_file = workspace_root.join("config/markets.toml");
    let binary = assert_cmd::cargo::cargo_bin!("tesser-cli");
    let mut cmd = Command::new(binary);
    cmd.current_dir(&workspace_root);
    cmd.args([
        "--env",
        "default",
        "backtest",
        "run",
        "--strategy-config",
        strategy_path.to_str().unwrap(),
        "--mode",
        "tick",
        "--lob-data",
        flight_dir.to_str().unwrap(),
        "--markets-file",
        markets_file.to_str().unwrap(),
        "--quantity",
        "0.01",
    ]);
    cmd.assert().success();
    Ok(())
}

fn write_ticks_dataset(root: &Path) -> Result<()> {
    let day_dir = root.join("ticks/2024-01-01");
    fs::create_dir_all(&day_dir)?;
    let ticks = sample_ticks();
    let batch = ticks_to_batch(&ticks)?;
    write_parquet(day_dir.join("ticks.parquet"), &batch)?;
    Ok(())
}

fn sample_ticks() -> Vec<Tick> {
    let base = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    (0..3)
        .map(|idx| Tick {
            symbol: "BTCUSDT".into(),
            price: Decimal::new(20_000 + idx as i64, 0),
            size: Decimal::new(1, 0),
            side: Side::Buy,
            exchange_timestamp: base + chrono::Duration::seconds(idx as i64),
            received_at: base + chrono::Duration::seconds(idx as i64),
        })
        .collect()
}

fn write_parquet(path: PathBuf, batch: &arrow::record_batch::RecordBatch) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))?;
    writer.write(batch)?;
    writer.close()?;
    Ok(())
}
